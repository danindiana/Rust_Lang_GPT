use std::{
    num::NonZeroU32,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
    collections::HashSet,
};

use dashmap::DashMap;
use anyhow::{Context, Result};
use dashmap::DashSet;
use futures::{StreamExt, TryStreamExt};
use governor::{Quota, RateLimiter, state::{InMemoryState, NotKeyed}, clock::DefaultClock};
use mime::Mime;
use moka::future::Cache;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::{
    header::{HeaderMap, CONTENT_DISPOSITION, CONTENT_TYPE, RANGE},
    redirect::Policy,
    Client,
};
use rustc_hash::FxHashMap;
use scraper::{Html, Selector};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom},
    sync::{mpsc, Semaphore},
    task::{JoinHandle, JoinSet},
    time::{sleep, timeout},
};
use tokio_util::io::StreamReader;
use url::Url;
use urlencoding;

// Use faster allocator
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// --- Static Regular Expressions & MIME Types (using once_cell) ---

static PDF_MIME_TYPES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "application/pdf",
        "application/x-pdf", 
        "application/acrobat",
        "applications/vnd.pdf",
        "text/pdf",
        "text/x-pdf"
    ].into_iter().collect()
});

// --- Enhanced Detection Heuristics ---
static PDF_CLUE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(?:pdf|portable document|\.pdf[)"'\s])"#).unwrap()
});

static ANCHOR_TEXT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(?:download|pdf|document|paper|report|slides)"#).unwrap()
});

static EXTRA_PDF_MIME: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "application/pdf",
        "application/x-pdf",
        "application/acrobat",
        "applications/vnd.pdf",
        "text/pdf",
        "text/x-pdf",
        "binary/pdf",                // seen on old servers
        "application/octet-stream",  // often mis-configured
    ].into_iter().collect()
});

// Corrected Magic Byte Detection
static MAGIC_PDF: Lazy<Vec<&'static [u8]>> = Lazy::new(|| {
    vec![
        b"%PDF-1.0",
        b"%PDF-1.1",
        b"%PDF-1.2",
        b"%PDF-1.3",
        b"%PDF-1.4",
        b"%PDF-1.5",
        b"%PDF-1.6",
        b"%PDF-1.7",
        b"%PDF-2.0",
        b"%PDF",  // Generic fallback
    ]
});


static FILENAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"filename\*?=['"]?(?:UTF-\d['"]*)?([^"'\s;]+)"#).unwrap()
});

static DANGEROUS_CHARS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"[\\/*?:"<>|\x00-\x1f]"#).unwrap()
});

static PDF_URL_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)\.pdf$").unwrap(),
        Regex::new(r"(?i)\.pdf\?").unwrap(),
        Regex::new(r"(?i)\.pdf#").unwrap(),
        Regex::new(r"(?i)/pdf/").unwrap(),
    ]
});

static LINK_SELECTOR: Lazy<Selector> = Lazy::new(|| {
    Selector::parse("a").unwrap()
});

// Unified PDF detection result
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum PdfKind {
    Yes,
    No,
}

// Rate limiter type
type Limiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;
type RateIndex = Mutex<FxHashMap<String, Limiter>>;

// Packed counters in a single cache line
#[repr(align(64))]
struct PackedCounters(AtomicU64);

impl PackedCounters {
    const DOWNLOADED_MASK: u64 = 0x0000_0000_0000_FFFF;
    const FAILED_MASK: u64 = 0x0000_0000_FFFF_0000;
    const QUEUED_MASK: u64 = 0x0000_FFFF_0000_0000;
    const CACHE_HITS_MASK: u64 = 0xFFFF_0000_0000_0000;
    
    const DOWNLOADED_SHIFT: u64 = 0;
    const FAILED_SHIFT: u64 = 16;
    const QUEUED_SHIFT: u64 = 32;
    const CACHE_HITS_SHIFT: u64 = 48;

    fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    fn inc_downloaded(&self) {
        self.0.fetch_add(1 << Self::DOWNLOADED_SHIFT, Ordering::Relaxed);
    }

    fn inc_failed(&self) {
        self.0.fetch_add(1 << Self::FAILED_SHIFT, Ordering::Relaxed);
    }

    fn inc_queued(&self) {
        self.0.fetch_add(1 << Self::QUEUED_SHIFT, Ordering::Relaxed);
    }

    fn dec_queued(&self) {
        self.0.fetch_sub(1 << Self::QUEUED_SHIFT, Ordering::Relaxed);
    }

    fn inc_cache_hits(&self) {
        self.0.fetch_add(1 << Self::CACHE_HITS_SHIFT, Ordering::Relaxed);
    }

    fn get_counts(&self) -> (u64, u64, u64, u64) {
        let val = self.0.load(Ordering::Relaxed);
        (
            (val & Self::DOWNLOADED_MASK) >> Self::DOWNLOADED_SHIFT,
            (val & Self::FAILED_MASK) >> Self::FAILED_SHIFT,
            (val & Self::QUEUED_MASK) >> Self::QUEUED_SHIFT,
            (val & Self::CACHE_HITS_MASK) >> Self::CACHE_HITS_SHIFT,
        )
    }
}

// --- Configuration ---

#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    pub download_dir: PathBuf,
    pub concurrent_crawlers: usize,
    pub concurrent_checks: usize,
    pub concurrent_downloads: usize,
    pub pdf_header_check_size: usize,
    pub cache_capacity: u64,
    pub cache_ttl: Duration,
    pub negative_cache_ttl: Duration,
    pub page_request_timeout: Duration,
    pub pdf_queue_buffer: usize,
    pub crawl_queue_buffer: usize,
    pub host_rate_limit: u32,
    pub global_socket_limit: usize,
    pub download_timeout: Duration,
    pub max_retries: usize,
    pub retry_delay: Duration,
    pub connection_pool_size: usize,
    pub resume: bool,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::from("downloaded_pdfs"),
            concurrent_crawlers: 30, // Increased from 20
            concurrent_checks: 10,
            concurrent_downloads: 4,
            pdf_header_check_size: 1024,
            cache_capacity: 120_000,
            cache_ttl: Duration::from_secs(3600),
            negative_cache_ttl: Duration::from_secs(30),
            page_request_timeout: Duration::from_secs(12), // Increased from 8
            pdf_queue_buffer: 2500,
            crawl_queue_buffer: 10000,
            host_rate_limit: 20, // Increased from 10
            global_socket_limit: 5000, // Increased from 3000
            download_timeout: Duration::from_secs(120),
            max_retries: 2,
            retry_delay: Duration::from_millis(500),
            connection_pool_size: 300,
            resume: true, 
        }
    }
}

// --- Core Crawler Structs ---

#[derive(Debug, Clone)]
struct CrawlTask {
    url: Url,
    depth: usize,
    retry_count: usize,
}

#[derive(Clone)]
struct SharedState {
    visited_urls: Arc<DashSet<u64>>,
    pdf_cache: Cache<Url, PdfKind>,
    rate_limiters: Arc<RateIndex>,
    // Anchor text map for zero-cost context
    anchor_text_map: Arc<DashMap<Url, String>>,
}

pub struct ParallelPDFCrawler {
    config: CrawlerConfig,
    client: Arc<Client>,
    state: SharedState,
    counters: Arc<PackedCounters>,
    global_semaphore: Arc<Semaphore>,
}

// Enhanced PDF validation
struct PDFValidator;

impl PDFValidator {
    // More robust header validation
    fn validate_header(data: &[u8]) -> bool {
        if data.len() < 8 {
            return false;
        }
        
        // Check for PDF magic bytes with more flexibility
        if data.starts_with(b"%PDF-") && data.len() >= 8 {
            // Verify version format (should be %PDF-X.Y)
            let version_part = &data[5..];
            if version_part.len() >= 3 {
                return version_part[0].is_ascii_digit() 
                    && version_part[1] == b'.' 
                    && version_part[2].is_ascii_digit();
            }
        }
        
        // Also check for linearized PDFs which might have different headers
        if data.starts_with(b"%PDF") {
            // Some PDFs might have %PDF without version immediately following
            return true;
        }
        
        false
    }
    
    // Comprehensive PDF structure validation
    async fn validate_complete_pdf(file_path: &Path) -> Result<bool> {
        let mut file = File::open(file_path).await
            .context("Failed to open file for validation")?;
        
        let metadata = file.metadata().await?;
        let file_size = metadata.len();
        
        // Minimum viable PDF size
        if file_size < 100 {
            return Ok(false);
        }
        
        // Maximum reasonable PDF size (adjust as needed)
        if file_size > 500_000_000 { // 500MB
            return Ok(false);
        }
        
        // Read header
        let mut header = [0u8; 16];
        if file.read_exact(&mut header).await.is_err() {
            return Ok(false);
        }
        
        if !Self::validate_header(&header) {
            return Ok(false);
        }
        
        // Check for EOF marker - try multiple approaches
        let has_eof = Self::check_eof_marker(&mut file, file_size).await?;
        if !has_eof {
            return Ok(false);
        }
        
        // Read more content for structure validation
        file.seek(SeekFrom::Start(0)).await?;
        let read_size = std::cmp::min(file_size, 1_000_000) as usize; // Read up to 1MB
        let mut content = vec![0u8; read_size];
        file.read_exact(&mut content).await?;
        
        let content_str = String::from_utf8_lossy(&content);
        
        // Check for essential PDF elements
        let structure_checks = Self::validate_pdf_structure(&content_str);
        
        Ok(structure_checks)
    }
    
    // More thorough EOF marker checking
    async fn check_eof_marker(file: &mut File, file_size: u64) -> Result<bool> {
        // Try reading from different positions near the end
        let search_sizes = [32, 64, 128, 256];
        
        for &size in &search_sizes {
            if file_size >= size {
                file.seek(SeekFrom::End(-(size as i64))).await?;
                let mut buffer = vec![0u8; size as usize];
                if file.read_exact(&mut buffer).await.is_ok() {
                    let buffer_str = String::from_utf8_lossy(&buffer);
                    if buffer_str.contains("%%EOF") || buffer_str.contains("startxref") {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    // Enhanced structure validation
    fn validate_pdf_structure(content: &str) -> bool {
        // Convert to lowercase for case-insensitive matching
        let content_lower = content.to_lowercase();
        
        // Essential PDF elements (more flexible matching)
        let has_version = content.starts_with("%PDF");
        let has_objects = content_lower.contains("obj") && content_lower.contains("endobj");
        let has_xref = content_lower.contains("xref") || content_lower.contains("xrefstm");
        let has_trailer = content_lower.contains("trailer") || content_lower.contains("/root");
        let has_startxref = content_lower.contains("startxref");
        
        // Check for common PDF keywords
        let has_pdf_keywords = content_lower.contains("/catalog") || 
                                 content_lower.contains("/pages") ||
                                 content_lower.contains("/type") ||
                                 content_lower.contains("/font");
        
        // Must have version and at least some PDF structure
        has_version && (
            (has_objects && (has_xref || has_trailer)) ||
            (has_startxref && has_pdf_keywords) ||
            (has_xref && has_trailer)
        )
    }
    
    // Enhanced quick validation for streaming
    fn is_likely_pdf_start(data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }
        
        // Primary check: starts with %PDF
        if data.starts_with(b"%PDF") {
            return true;
        }
        
        // Secondary check: hex encoded %PDF
        if data.starts_with(b"\x25\x50\x44\x46") {
            return true;
        }
        
        // Tertiary check: scan first few bytes for %PDF (some files have prefix)
        if data.len() >= 10 {
            for window in data.windows(4) {
                if window == b"%PDF" {
                    return true;
                }
            }
        }
        
        // Check for common PDF object indicators in first chunk
        let data_str = String::from_utf8_lossy(data).to_lowercase();
        if data_str.contains("pdf") && (
            data_str.contains("obj") || 
            data_str.contains("stream") ||
            data_str.contains("catalog")
        ) {
            return true;
        }
        
        false
    }
    
    // Additional validation: check if file is actually HTML/XML disguised as PDF
    fn is_likely_html_or_xml(data: &[u8]) -> bool {
        let data_str = String::from_utf8_lossy(data).to_lowercase();
        data_str.trim_start().starts_with("<!doctype") ||
        data_str.trim_start().starts_with("<html") ||
        data_str.trim_start().starts_with("<?xml") ||
        data_str.contains("<head>") ||
        data_str.contains("<body>") ||
        data_str.contains("<title>")
    }
}


impl ParallelPDFCrawler {
    pub async fn new(config: CrawlerConfig) -> Result<Self> {
        Self::ensure_fd_limit()?;
        
        tokio::fs::create_dir_all(&config.download_dir)
            .await
            .context("Failed to create download directory")?;

        let client = Arc::new(Self::create_optimized_client(&config)?);

        // Single unified cache with conditional TTL
        let pdf_cache = Cache::builder()
            .max_capacity(config.cache_capacity)
            .time_to_live(config.cache_ttl)
            .build();

        let state = SharedState {
            visited_urls: Arc::new(DashSet::new()),
            pdf_cache,
            rate_limiters: Arc::new(Mutex::new(FxHashMap::default())),
            // Initialize anchor text map
            anchor_text_map: Arc::new(DashMap::new()),
        };

        Ok(Self {
            global_semaphore: Arc::new(Semaphore::new(config.global_socket_limit)),
            counters: Arc::new(PackedCounters::new()),
            config,
            client,
            state,
        })
    }

    fn ensure_fd_limit() -> Result<()> {
        #[cfg(feature = "fast")]
        {
            use rlimit::{Resource, setrlimit};
            if let Err(e) = setrlimit(Resource::NOFILE, 65536, 65536) {
                return Err(anyhow::anyhow!("Failed to raise file descriptor limit: {}", e));
            }
        }
        #[cfg(not(feature = "fast"))]
        {
            println!("Note: Run 'ulimit -n 65536' before starting for optimal performance");
        }
        Ok(())
    }

    fn create_optimized_client(config: &CrawlerConfig) -> Result<Client> {
        Client::builder()
            .pool_max_idle_per_host(config.connection_pool_size)
            .pool_idle_timeout(Duration::from_secs(90))
            .redirect(Policy::limited(5))
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .tcp_nodelay(true)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .http2_keep_alive_interval(Some(Duration::from_secs(30)))
            .use_rustls_tls()
            .tls_built_in_root_certs(true)
            .gzip(true)
            .brotli(true)
            .http1_title_case_headers()
            .user_agent("Mozilla/5.0 (compatible; Rust-PDF-Crawler/4.4)")
            .build()
            .context("Failed to build reqwest client")
    }

    fn hash_url(url: &Url) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        let normalized = url.as_str().trim_end_matches('/').to_lowercase();
        normalized.hash(&mut hasher);
        hasher.finish()
    }

    fn get_rate_limiter(&self, host: &str) -> Limiter {
        let mut limiters = self.state.rate_limiters.lock().unwrap();
        limiters.entry(host.to_string())
            .or_insert_with(|| {
                Arc::new(RateLimiter::direct(
                    Quota::per_second(NonZeroU32::new(self.config.host_rate_limit).unwrap())
                ))
            })
            .clone()
    }

    pub async fn crawl_and_download(
        &self,
        start_url_str: &str,
        recursive: bool,
        max_depth: Option<usize>,
    ) -> Result<usize> {
        let start_url = Url::parse(start_url_str).context("Invalid start URL")?;
        let base_host = start_url.host_str().unwrap_or("unknown").to_string();

        println!("Starting optimized crawl of {}", start_url);
        println!("Mode: {}, Max depth: {:?}", if recursive { "Recursive" } else { "Single page" }, max_depth);

        let monitor_handle = self.spawn_monitoring_task();
        let (pdf_tx, pdf_rx) = mpsc::channel::<Url>(self.config.pdf_queue_buffer);
        
        // Add shutdown detection
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        let shutdown_signal_clone = Arc::clone(&shutdown_signal);
        
        let download_handle = {
            let client = Arc::clone(&self.client);
            let config = self.config.clone();
            let counters = Arc::clone(&self.counters);
            let shutdown = Arc::clone(&shutdown_signal);
            
            tokio::spawn(async move {
                let mut stream = tokio_stream::wrappers::ReceiverStream::new(pdf_rx);
                let mut downloaded_count = 0;
                
                while let Some(url) = stream.next().await {
                    if shutdown.load(Ordering::Relaxed) {
                        println!("[DEBUG] Download task received shutdown signal");
                        break;
                    }
                    
                    counters.dec_queued();
                    let result = timeout(
                        config.download_timeout,
                        Self::download_pdf_with_validation(config.download_dir.clone(), Arc::clone(&client), url.clone(), config.resume)
                    ).await;
                    
                    match result {
                        Ok(Ok(())) => {
                            counters.inc_downloaded();
                            downloaded_count += 1;
                        }
                        Ok(Err(e)) => {
                            eprintln!("Download failed for {}: {}", url, e);
                            counters.inc_failed();
                        }
                        Err(_) => {
                            eprintln!("Download timed out for {}", url);
                            counters.inc_failed();
                        }
                    }
                }
                
                downloaded_count
            })
        };

        if recursive {
            let (crawl_tx, crawl_rx) = mpsc::channel::<CrawlTask>(self.config.crawl_queue_buffer);
            
            let url_hash = Self::hash_url(&start_url);
            self.state.visited_urls.insert(url_hash);
            
            let initial_task = CrawlTask { 
                url: start_url, 
                depth: 0, 
                retry_count: 0 
            };
            
            if let Err(e) = crawl_tx.send(initial_task).await {
                eprintln!("Failed to send initial crawl task: {}", e);
                return Ok(0);
            }

            // Run with timeout and shutdown detection
            let crawl_result = timeout(
                Duration::from_secs(300), // 5 minute overall timeout
                self.run_recursive_crawl_with_shutdown(crawl_rx, crawl_tx.clone(), pdf_tx.clone(), base_host, max_depth, Arc::clone(&shutdown_signal))
            ).await;
            
            match crawl_result {
                Ok(_) => println!("Crawl completed normally"),
                Err(_) => {
                    println!("Crawl timed out - initiating shutdown");
                    shutdown_signal_clone.store(true, Ordering::Relaxed);
                }
            }
            
            drop(crawl_tx);
        } else {
            self.run_single_page_crawl_optimized(start_url, pdf_tx.clone()).await?;
        }

        drop(pdf_tx);
        
        // Wait for downloads with timeout
        let total_downloaded = match timeout(Duration::from_secs(60), download_handle).await {
            Ok(Ok(count)) => count,
            Ok(Err(_)) => {
                println!("Download task failed");
                0
            }
            Err(_) => {
                println!("Download timeout - some downloads may still be in progress");
                shutdown_signal_clone.store(true, Ordering::Relaxed);
                0
            }
        };
        
        monitor_handle.abort();

        let visited_count = self.state.visited_urls.len();
        let (downloaded, failed, _queued, cache_hits) = self.counters.get_counts();

        println!("\nCRAWL COMPLETE!");
        println!("URLs visited: {}", visited_count);
        println!("PDFs downloaded: {}", downloaded);
        println!("Failed downloads: {}", failed);
        println!("Cache hits: {}", cache_hits);
        println!("Download directory: {}", self.config.download_dir.display());

        Ok(total_downloaded)
    }

    async fn run_recursive_crawl_with_shutdown(
        &self,
        mut crawl_rx: mpsc::Receiver<CrawlTask>,
        crawl_tx: mpsc::Sender<CrawlTask>,
        pdf_tx: mpsc::Sender<Url>,
        base_host: String,
        max_depth: Option<usize>,
        shutdown_signal: Arc<AtomicBool>,
    ) {
        let mut tasks = JoinSet::new();
        let mut no_work_counter = 0;
        
        loop {
            if shutdown_signal.load(Ordering::Relaxed) {
                println!("[DEBUG] Crawl received shutdown signal");
                break;
            }
            
            tokio::select! {
                Some(task) = crawl_rx.recv() => {
                    no_work_counter = 0; // Reset counter when we get work
                    
                    // Manage task capacity
                    while tasks.len() >= self.config.concurrent_crawlers {
                        if let Some(res) = tasks.join_next().await {
                            if let Err(e) = res {
                                eprintln!("Crawler task error: {:?}", e);
                            }
                        }
                    }

                    let permit = match self.global_semaphore.clone().try_acquire_owned() {
                        Ok(permit) => permit,
                        Err(_) => {
                            // Put task back if no permits
                            let _ = crawl_tx.try_send(task);
                            continue;
                        }
                    };

                    let this = self.clone();
                    let crawl_tx_clone = crawl_tx.clone();
                    let pdf_tx_clone = pdf_tx.clone();
                    let base_host_clone = base_host.clone();

                    tasks.spawn(async move {
                        let _permit = permit;
                        if let Err(e) = this.process_crawl_task_optimized(task, crawl_tx_clone, pdf_tx_clone, base_host_clone, max_depth).await {
                            eprintln!("Error processing task: {}", e);
                        }
                    });
                },
                Some(res) = tasks.join_next(), if !tasks.is_empty() => {
                    if let Err(e) = res {
                        eprintln!("Crawler task panicked: {:?}", e);
                    }
                },
                _ = sleep(Duration::from_millis(500)) => {
                    no_work_counter += 1;
                    
                    if no_work_counter >= 20 { // 10 seconds of no work
                        println!("[DEBUG] No work available for 10 seconds - checking for completion");
                        
                        if tasks.is_empty() && crawl_rx.is_empty() {
                            println!("[DEBUG] All tasks completed - shutting down crawl");
                            break;
                        }
                    }
                    
                    if no_work_counter >= 60 { // 30 seconds
                        println!("[DEBUG] Extended period with no work - forcing shutdown");
                        break;
                    }
                }
            }
        }
        
        // Wait for remaining tasks
        while let Some(res) = tasks.join_next().await {
            if let Err(e) = res {
                eprintln!("Final crawler task error: {:?}", e);
            }
        }
        
        println!("All crawl tasks completed");
    }

    async fn run_single_page_crawl_optimized(&self, start_url: Url, pdf_tx: mpsc::Sender<Url>) -> Result<()> {
        println!("Processing single page: {}", start_url);
        
        let links = match self.extract_links_optimized(&start_url).await {
            Ok(links) => links,
            Err(e) => {
                eprintln!("Failed to extract links from {}: {}", start_url, e);
                return Ok(());
            }
        };
        
        println!("Found {} potential links on the page.", links.len());

        let pdf_checks = self.batch_check_pdf_links_optimized(links).await;
        let pdf_count = pdf_checks.iter().filter(|(_, kind)| *kind == PdfKind::Yes).count();
        
        println!("Identified {} PDFs for download", pdf_count);
        
        for (url, _) in pdf_checks.into_iter().filter(|(_, kind)| *kind == PdfKind::Yes) {
            if let Err(e) = pdf_tx.send(url.clone()).await {
                eprintln!("Failed to send PDF to download queue: {}", e);
            } else {
                self.counters.inc_queued();
            }
        }
        Ok(())
    }

    async fn process_crawl_task_optimized(
        &self,
        task: CrawlTask,
        crawl_tx: mpsc::Sender<CrawlTask>,
        pdf_tx: mpsc::Sender<Url>,
        base_host: String,
        max_depth: Option<usize>,
    ) -> Result<()> {
        if let Some(host) = task.url.host_str() {
            let limiter = self.get_rate_limiter(host);
            limiter.until_ready().await;
        }
        
        let links = match self.extract_links_optimized(&task.url).await {
            Ok(links) => links,
            Err(_e) => {
                if task.retry_count < self.config.max_retries {
                    let retry_task = CrawlTask {
                        retry_count: task.retry_count + 1,
                        ..task
                    };
                    
                    let crawl_tx_for_retry = crawl_tx.clone();
                    tokio::spawn(async move {
                        sleep(Duration::from_millis(500)).await;
                        let _ = timeout(Duration::from_secs(1), crawl_tx_for_retry.send(retry_task)).await;
                    });
                }
                return Ok(());
            }
        };

        let unique_links: Vec<_> = links.into_iter()
            .filter(|link| {
                let hash = Self::hash_url(link);
                self.state.visited_urls.insert(hash)
            })
            .collect();

        if unique_links.is_empty() {
            return Ok(());
        }

        let pdf_checks = self.batch_check_pdf_links_optimized(unique_links).await;

        for (url, kind) in pdf_checks {
            if kind == PdfKind::Yes {
                if pdf_tx.try_send(url.clone()).is_err() {
                    let pdf_tx_clone = pdf_tx.clone();
                    tokio::spawn(async move {
                        if pdf_tx_clone.send(url).await.is_ok() {
                            // This is tricky, the counter should be handled by the receiver ideally
                        }
                    });
                } else {
                    self.counters.inc_queued();
                }
            } else if url.host_str() == Some(&base_host) {
                let should_crawl = max_depth.map_or(true, |max| task.depth + 1 < max);
                if should_crawl {
                    let new_task = CrawlTask { 
                        url, 
                        depth: task.depth + 1, 
                        retry_count: 0 
                    };
                    
                    let _ = crawl_tx.try_send(new_task);
                }
            }
        }
        Ok(())
    }

    async fn extract_links_optimized(&self, url: &Url) -> Result<Vec<Url>> {
        let response = timeout(
            self.config.page_request_timeout,
            self.client.get(url.clone()).send()
        ).await??;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }

        let content_type = response.headers().get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
            
        if !content_type.contains("text/html") {
            return Ok(Vec::new());
        }

        let text = response.text().await?;
        let base_url = url.clone();

        // Capture anchor text while parsing
        let anchor_map: Arc<DashMap<Url, String>> = Arc::clone(&self.state.anchor_text_map);
        let links = tokio::task::spawn_blocking(move || {
            let document = Html::parse_document(&text);
            let mut links = Vec::new();
            
            for element in document.select(&LINK_SELECTOR) {
                if let Some(href) = element.value().attr("href") {
                    if let Ok(link_url) = base_url.join(href) {
                        if matches!(link_url.scheme(), "http" | "https") {
                            links.push(link_url.clone());
                            // Store anchor text for later PDF detection
                            let anchor_text = element.text().collect::<String>();
                            if !anchor_text.trim().is_empty() {
                                anchor_map.insert(link_url, anchor_text);
                            }
                        }
                    }
                }
            }
            
            links
        }).await?;

        Ok(links)
    }

    async fn batch_check_pdf_links_optimized(&self, urls: Vec<Url>) -> Vec<(Url, PdfKind)> {
        let mut join_set = JoinSet::new();
        
        for url in urls {
            let this = self.clone();
            join_set.spawn(async move {
                let result = this.is_pdf_link_optimized(&url).await.unwrap_or(PdfKind::No);
                (url, result)
            });
        }
        
        let mut results = Vec::new();
        while let Some(res) = join_set.join_next().await {
            if let Ok(result) = res {
                results.push(result);
            }
        }
        
        results
    }

    // Upgraded PDF detection pipeline
    async fn is_pdf_link_optimized(&self, url: &Url) -> Result<PdfKind> {
        // 0. Check cache first
        if let Some(cached) = self.state.pdf_cache.get(url).await {
            self.counters.inc_cache_hits();
            return Ok(cached);
        }

        let mut kind = PdfKind::No;

        // 1. URL heuristic (expanded)
        if PDF_URL_PATTERNS.iter().any(|re| re.is_match(url.as_str())) {
            kind = PdfKind::Yes;
        }

        // 2. Inline JS clue (check path/fragment for PDF hints)
        if kind == PdfKind::No {
            let full_url_str = url.as_str();
            if PDF_CLUE_REGEX.is_match(full_url_str) {
                kind = PdfKind::Yes;
            }
        }

        // 3. Parent page anchor text (zero-cost, already captured)
        if kind == PdfKind::No {
            if let Some(text_ref) = self.state.anchor_text_map.get(url) {
                let text = text_ref.value().clone();
                if ANCHOR_TEXT_REGEX.is_match(&text) {
                    kind = PdfKind::Yes;
                }
            }
        }

        // 4. HEAD request with broader MIME types
        if kind == PdfKind::No {
            let resp = timeout(
                Duration::from_secs(3),
                self.client.head(url.as_str()).send()
            ).await;

            if let Ok(Ok(r)) = resp {
                if let Some(ct) = r.headers().get(CONTENT_TYPE)
                                  .and_then(|v| v.to_str().ok())
                                  .and_then(|s| s.parse::<Mime>().ok()) {
                    if EXTRA_PDF_MIME.contains(ct.essence_str()) {
                        kind = PdfKind::Yes;
                    }
                }
            }
        }

        // 5. Range request (fallback with enhanced magic bytes)
        if kind == PdfKind::No {
            let range = format!("bytes=0-{}", self.config.pdf_header_check_size - 1);
            if let Ok(Ok(resp)) = timeout(
                Duration::from_secs(3),
                self.client.get(url.as_str()).header(RANGE, range).send()
            ).await {
                if resp.status().is_success() || resp.status() == reqwest::StatusCode::PARTIAL_CONTENT {
                    if let Ok(chunk) = resp.bytes().await {
                        for magic in MAGIC_PDF.iter() {
                            if chunk.starts_with(magic) {
                                kind = PdfKind::Yes;
                                break;
                            }
                        }
                    }
                }
            }
        }

        self.state.pdf_cache.insert(url.clone(), kind).await;
        Ok(kind)
    }

    fn spawn_monitoring_task(&self) -> JoinHandle<()> {
        let counters = Arc::clone(&self.counters);
        let state = self.state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                let visited = state.visited_urls.len();
                let (downloaded, failed, queued, cache_hits) = counters.get_counts();

                println!(
                    "[STATUS] Visited: {} | Downloaded: {} | Failed: {} | Queued: {} | Cache hits: {}",
                    visited, downloaded, failed, queued, cache_hits
                );
            }
        })
    }

    // Simplified and more robust download function
    async fn download_pdf_with_validation(
        download_dir: PathBuf,
        client: Arc<Client>,
        url: Url,
        resume: bool,
    ) -> Result<()> {
        // Start the request to get headers
        let response = client.get(url.clone()).send().await?.error_for_status()?;
        
        // Get final filename from headers if available
        let final_filename = get_safe_filename(&url, response.headers());
        let final_filepath = download_dir.join(&final_filename);
        let temp_filepath = download_dir.join(format!("{}.tmp", final_filename));
    
        // Check if file already exists and is valid
        if final_filepath.exists() && !resume {
            if PDFValidator::validate_complete_pdf(&final_filepath).await.unwrap_or(false) {
                println!("File already exists and is valid: {}", final_filename);
                return Ok(());
            } else {
                println!("Existing file is invalid, re-downloading: {}", final_filename);
                let _ = tokio::fs::remove_file(&final_filepath).await;
            }
        }
    
        // Validate content-type if available
        if let Some(content_type) = response.headers().get(CONTENT_TYPE) {
            let ct_str = content_type.to_str().unwrap_or("");
            // Only reject obviously wrong types
            if ct_str.contains("text/html") && !ct_str.contains("pdf") {
                println!("[DEBUG] Skipping HTML content: {}", url);
                return Err(anyhow::anyhow!("Content is HTML, not PDF"));
            }
            // Allow everything else to proceed to content validation
        }
    
        // Get expected file size for validation
        let expected_size = response.content_length();
    
        // Create temporary file
        let mut file = File::create(&temp_filepath).await
            .context("Failed to create temporary file")?;
        
        // Pre-allocate space if we know the size
        if let Some(size) = expected_size {
            let _ = file.set_len(size).await;
        }
    
        // Download with streaming validation
        let mut stream = response.bytes_stream();
        let mut total_bytes = 0u64;
        let mut first_chunk = true;
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read chunk")?;
            
            // Validate first chunk contains PDF header
            if first_chunk {
                if PDFValidator::is_likely_html_or_xml(&chunk) {
                    tokio::fs::remove_file(&temp_filepath).await.ok();
                    return Err(anyhow::anyhow!("File appears to be HTML/XML, not PDF"));
                }

                if !PDFValidator::is_likely_pdf_start(&chunk) {
                    tokio::fs::remove_file(&temp_filepath).await.ok();
                    return Err(anyhow::anyhow!("File does not appear to be a PDF (invalid header)"));
                }
                first_chunk = false;
            }
            
            // Write chunk to file
            file.write_all(&chunk).await.context("Failed to write chunk")?;
            total_bytes += chunk.len() as u64;
            
            // Optional: Check if we're downloading too much (potential attack)
            if total_bytes > 100_000_000 { // 100MB limit
                tokio::fs::remove_file(&temp_filepath).await.ok();
                return Err(anyhow::anyhow!("File too large (>100MB), aborting download"));
            }
        }
        
        // Ensure data is written to disk
        file.flush().await.context("Failed to flush file")?;
        file.sync_all().await.context("Failed to sync file")?;
        drop(file);
    
        // Validate expected size if known
        if let Some(expected) = expected_size {
            if total_bytes != expected {
                tokio::fs::remove_file(&temp_filepath).await.ok();
                return Err(anyhow::anyhow!(
                    "File size mismatch: expected {} bytes, got {} bytes", 
                    expected, total_bytes
                ));
            }
        }
        
        // Validate complete PDF structure
        if !PDFValidator::validate_complete_pdf(&temp_filepath).await.unwrap_or(false) {
            tokio::fs::remove_file(&temp_filepath).await.ok();
            return Err(anyhow::anyhow!("Downloaded file failed PDF validation"));
        }
        
        // Move temp file to final location
        tokio::fs::rename(&temp_filepath, &final_filepath).await
            .context("Failed to move temporary file")?;
        
        println!("Downloaded and validated: {} ({} bytes)", final_filename, total_bytes);
        Ok(())
    }
}

// Standalone filename utilities
fn get_safe_filename(url: &Url, headers: &HeaderMap) -> String {
    if let Some(disposition) = headers.get(CONTENT_DISPOSITION).and_then(|v| v.to_str().ok()) {
        if let Some(caps) = FILENAME_REGEX.captures(disposition) {
            if let Some(name) = caps.get(1) {
                return sanitize_filename(name.as_str());
            }
        }
    }

    let filename = Path::new(url.path())
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.pdf");

    sanitize_filename(filename)
}

fn sanitize_filename(filename: &str) -> String {
    let decoded = urlencoding::decode(filename).unwrap_or_else(|_| filename.into());
    let sanitized = DANGEROUS_CHARS.replace_all(&decoded, "_");

    let mut result: String = sanitized
        .trim_matches(|c: char| c == '.' || c.is_whitespace())
        .chars()
        .take(200)
        .collect();

    if result.ends_with('.') {
        result.pop();
    }

    if result.is_empty() {
        "document.pdf".to_string()
    } else if !result.to_lowercase().ends_with(".pdf") {
        format!("{}.pdf", result)
    } else {
        result
    }
}

impl Clone for ParallelPDFCrawler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            client: Arc::clone(&self.client),
            state: self.state.clone(),
            counters: Arc::clone(&self.counters),
            global_semaphore: Arc::clone(&self.global_semaphore),
        }
    }
}

async fn read_user_input(prompt: &str) -> Result<String> {
    use std::io::Write;
    print!("{}", prompt);
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== OPTIMIZED PDF CRAWLER V4.4 (ANTI-STALL) ===");

    let target_url = read_user_input("Enter target URL: ").await?;
    let recursive_input = read_user_input("Recursive search? (y/n): ").await?;
    let recursive = recursive_input.eq_ignore_ascii_case("y");

    let max_depth = if recursive {
        read_user_input("Max depth (or Enter for default): ").await?.parse().ok()
    } else {
        None
    };

    let config = CrawlerConfig::default();
    let crawler = ParallelPDFCrawler::new(config).await?;
    let start_time = Instant::now();

    match crawler.crawl_and_download(&target_url, recursive, max_depth).await {
        Ok(count) => {
            let elapsed = start_time.elapsed();
            if elapsed.as_secs_f64() > 0.0 {
                let rate = count as f64 / elapsed.as_secs_f64();
                println!("\nSuccess! Downloaded {} PDFs in {:.2?} ({:.2} PDFs/sec)", count, elapsed, rate);
            } else {
                println!("\nSuccess! Downloaded {} PDFs in {:.2?}", count, elapsed);
            }
        }
        Err(e) => {
            eprintln!("\nAn error occurred: {}", e);
            for cause in e.chain().skip(1) {
                eprintln!("  Caused by: {}", cause);
            }
        }
    }

    Ok(())
}


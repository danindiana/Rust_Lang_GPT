use clap::Parser;
use hyper::{Body, Client, Request, Method, header, Uri};
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use url::Url;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio::fs::File;
use tokio::sync::mpsc;
use chrono;

#[derive(Parser, Debug)]
#[command(name = "pdf-crawler")]
#[command(about = "A web crawler that discovers and verifies PDF files")]
struct Args {
    /// Starting URL to crawl
    #[arg(short, long)]
    url: String,
    /// Maximum crawl depth
    #[arg(short, long, default_value = "5")]
    depth: usize,
    /// Maximum concurrent requests
    #[arg(short, long, default_value = "12")]
    concurrency: usize,
    /// Delay between requests in milliseconds
    #[arg(long, default_value = "1000")]
    delay: u64,
    /// Output JSON file path
    #[arg(short, long, default_value = "pdfs.json")]
    output: String,
    /// Respect robots.txt
    #[arg(long)]
    respect_robots: bool,
    /// Verify PDF content with HTTP checks
    #[arg(long, default_value = "true")]
    verify_pdfs: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PdfInfo {
    url: String,
    source_page: String,
    depth: usize,
    title: Option<String>,
    size_hint: Option<String>,
    content_type: Option<String>,
    content_length: Option<u64>,
    discovered_at: String,
    verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CrawlMetadata {
    start_url: String,
    max_depth: usize,
    total_pages_crawled: usize,
    total_pdfs_found: usize,
    verified_pdfs: usize,
    failed_verifications: usize,
    crawl_timestamp: String,
    status: String,
    verification_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CrawlResults {
    metadata: CrawlMetadata,
    pdfs: Vec<PdfInfo>,
}

enum WriterMessage {
    AddPdf(PdfInfo),
    UpdateMetadata { pages_crawled: usize, status: String },
}

struct Crawler {
    client: Client<HttpsConnector<HttpConnector>>,
    robots_cache: HashMap<String, bool>,
    visited: HashSet<String>,
    discovered_pdfs: HashSet<String>,
    semaphore: Arc<Semaphore>,
    delay: Duration,
    respect_robots: bool,
    verify_pdfs: bool,
    pdf_sender: mpsc::Sender<WriterMessage>,
}

impl Crawler {
    async fn new(
        concurrency: usize,
        delay: Duration,
        respect_robots: bool,
        verify_pdfs: bool,
        output_file: String,
    ) -> Result<(Self, mpsc::Sender<WriterMessage>), Box<dyn std::error::Error + Send + Sync>> {
        let https = HttpsConnector::new();
        let client = Client::builder()
            .pool_max_idle_per_host(5)
            .pool_idle_timeout(Duration::from_secs(30))
            .build(https);
        
        let (pdf_sender, pdf_receiver) = mpsc::channel(100);
        
        // Initialize the output file with empty structure
        let initial_data = CrawlResults {
            metadata: CrawlMetadata {
                start_url: String::new(),
                max_depth: 0,
                total_pages_crawled: 0,
                total_pdfs_found: 0,
                verified_pdfs: 0,
                failed_verifications: 0,
                crawl_timestamp: chrono::Utc::now().to_rfc3339(),
                status: "in_progress".to_string(),
                verification_enabled: verify_pdfs,
            },
            pdfs: Vec::new(),
        };
        
        // Spawn writer task
        let sender_clone = pdf_sender.clone();
        tokio::spawn(async move {
            if let Err(e) = writer_task(initial_data, pdf_receiver, output_file).await {
                eprintln!("Writer task error: {}", e);
            }
        });
        
        Ok((Self {
            client,
            robots_cache: HashMap::new(),
            visited: HashSet::new(),
            discovered_pdfs: HashSet::new(),
            semaphore: Arc::new(Semaphore::new(concurrency)),
            delay,
            respect_robots,
            verify_pdfs,
            pdf_sender: sender_clone,
        }, pdf_sender))
    }

    fn is_likely_pdf_url(&self, url: &str) -> bool {
        let url_lower = url.to_lowercase();
        
        // Direct PDF extension
        if url_lower.ends_with(".pdf") {
            return true;
        }
        
        // Common PDF-serving patterns
        if url_lower.contains("pdf") &&
            (url_lower.contains("download")
                || url_lower.contains("file")
                || url_lower.contains("doc")
                || url_lower.contains("paper")
                || url_lower.contains("publication")
                || url_lower.contains("proceedings"))
        {
            return true;
        }
        
        // Content disposition or view patterns
        if url_lower.contains("view") && url_lower.contains("pdf") {
            return true;
        }
        
        // Query parameter patterns
        if url_lower.contains("format=pdf")
            || url_lower.contains("type=pdf")
            || url_lower.contains("export=pdf")
            || url_lower.contains(".pdf?")
        {
            return true;
        }
        
        false
    }

    async fn verify_pdf_shared(
        client: &Client<HttpsConnector<HttpConnector>>,
        semaphore: &Semaphore,
        url: &str,
    ) -> Result<(Option<String>, Option<u64>, bool), String> {
        // Acquire a permit for network I/O
        let _permit = semaphore.acquire().await.map_err(|e| e.to_string())?;
        
        // Gentle pacing across hosts
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Parse URL
        let uri = url.parse::<Uri>().map_err(|e| format!("Invalid URL: {}", e))?;
        
        // Try HEAD request first
        let head_request = Request::builder()
            .method(Method::HEAD)
            .uri(uri.clone())
            .header(header::USER_AGENT, "PDF-Crawler/1.0 (Educational Purpose)")
            .body(Body::empty())
            .map_err(|e| format!("Failed to build HEAD request: {}", e))?;
        
        let head_response = client.request(head_request).await;
        
        let (mut content_type, mut content_length, head_ok) = match head_response {
            Ok(rsp) => {
                if !rsp.status().is_success() {
                    (None, None, false)
                } else {
                    (
                        rsp.headers()
                            .get(header::CONTENT_TYPE)
                            .and_then(|h| h.to_str().ok())
                            .map(|s| s.to_lowercase()),
                        rsp.headers()
                            .get(header::CONTENT_LENGTH)
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse().ok()),
                        true,
                    )
                }
            }
            Err(_) => (None, None, false),
        };
        
        // If HEAD is insufficient, fallback to partial GET with magic-number scan
        if !head_ok || content_type.is_none() || content_length.unwrap_or(0) == 0 {
            // Release previous permit scope and reacquire a new one for GET
            drop(_permit);
            let _permit2 = semaphore.acquire().await.map_err(|e| e.to_string())?;
            tokio::time::sleep(Duration::from_millis(50)).await;
            
            let get_request = Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header(header::USER_AGENT, "PDF-Crawler/1.0 (Educational Purpose)")
                .header(header::RANGE, "bytes=0-8191") // Scan first 8KB
                .body(Body::empty())
                .map_err(|e| format!("Failed to build GET request: {}", e))?;
            
            let get_response = client.request(get_request).await
                .map_err(|e| format!("Partial GET failed: {}", e))?;
            
            if !(get_response.status().is_success() || get_response.status().as_u16() == 206) {
                return Err(format!("HTTP {} - partial GET not ok", get_response.status()));
            }
            
            content_type = get_response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_lowercase());
            
            content_length = get_response
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|h| h.to_str().ok())
                .and_then(|range| range.split('/').nth(1).and_then(|s| s.parse().ok()))
                .or_else(|| {
                    get_response
                        .headers()
                        .get(header::CONTENT_LENGTH)
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.parse().ok())
                });
            
            let bytes = hyper::body::to_bytes(get_response.into_body()).await
                .map_err(|e| format!("Read bytes failed: {}", e))?;
            
            // Scan for "%PDF" anywhere within 8KB window
            let has_pdf_magic = bytes.windows(4).any(|w| w == b"%PDF");
            let is_pdf_ct = content_type
                .as_ref()
                .map_or(false, |ct| ct.contains("application/pdf") || ct.contains("application/x-pdf"));
            
            let is_valid = has_pdf_magic
                && content_length.unwrap_or(0) > 1024
                && (is_pdf_ct || has_pdf_magic);
            
            return Ok((content_type, content_length, is_valid));
        }
        
        // HEAD-only acceptance (conservative)
        let is_pdf_ct = content_type
            .as_ref()
            .map_or(false, |ct| ct.contains("application/pdf") || ct.contains("application/x-pdf"));
        let is_valid = is_pdf_ct && content_length.unwrap_or(0) > 1024;
        
        Ok((content_type, content_length, is_valid))
    }

    async fn can_fetch(&mut self, url: &Url) -> bool {
        if !self.respect_robots {
            return true;
        }
        
        let base_url = format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""));
        if let Some(&allowed) = self.robots_cache.get(&base_url) {
            return allowed;
        }
        
        // Simplified robots.txt check
        let robots_url = format!("{}/robots.txt", base_url);
        let uri = match robots_url.parse::<Uri>() {
            Ok(uri) => uri,
            Err(_) => {
                self.robots_cache.insert(base_url, true);
                return true;
            }
        };
        
        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header(header::USER_AGENT, "PDF-Crawler/1.0 (Educational Purpose)")
            .body(Body::empty());
        
        let request = match request {
            Ok(req) => req,
            Err(_) => {
                self.robots_cache.insert(base_url, true);
                return true;
            }
        };
        
        match self.client.request(request).await {
            Ok(response) if response.status().is_success() => {
                let bytes = match hyper::body::to_bytes(response.into_body()).await {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        self.robots_cache.insert(base_url, true);
                        return true;
                    }
                };
                
                let content = match std::str::from_utf8(&bytes) {
                    Ok(content) => content,
                    Err(_) => {
                        self.robots_cache.insert(base_url, true);
                        return true;
                    }
                };
                
                let allowed = !content.contains("Disallow: /");
                self.robots_cache.insert(base_url, allowed);
                allowed
            }
            _ => {
                self.robots_cache.insert(base_url, true);
                true
            }
        }
    }

    async fn fetch_page(&self, url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let _permit = self.semaphore.acquire().await?;
        tokio::time::sleep(self.delay).await;
        println!("🌐 Fetching: {}", url);
        
        let uri = url.parse::<Uri>()?;
        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header(header::USER_AGENT, "PDF-Crawler/1.0 (Educational Purpose)")
            .body(Body::empty())?;
        
        let response = self.client.request(request).await?;
        let bytes = hyper::body::to_bytes(response.into_body()).await?;
        Ok(String::from_utf8(bytes.to_vec())?)
    }

    async fn extract_and_verify_pdfs(
        &mut self,
        html: &str,
        base_url: &Url,
        current_depth: usize,
    ) -> (Vec<String>, usize) {
        let document = Html::parse_document(html);
        let link_selector = Selector::parse("a[href]").unwrap();
        let mut links = Vec::new();
        let mut potential_pdfs = Vec::new();
        
        println!("🔍 Extracting links from: {}", base_url);
        
        // Collect same-site links + potential PDFs
        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                // Skip empty, javascript, and mailto links
                if href.is_empty() || href.starts_with("javascript:") || href.starts_with("mailto:") {
                    continue;
                }
                
                if let Ok(absolute_url) = base_url.join(href) {
                    let url_str = absolute_url.to_string();
                    
                    // Skip URLs with fragments
                    if let Some(fragment_pos) = url_str.find('#') {
                        let base_url_str = &url_str[..fragment_pos];
                        if self.visited.contains(base_url_str) {
                            continue;
                        }
                    }
                    
                    if self.is_likely_pdf_url(&url_str) {
                        if !self.discovered_pdfs.contains(&url_str) {
                            let title = element.text().collect::<String>().trim().to_string();
                            let title = if title.is_empty() { None } else { Some(title) };
                            let pdf_info = PdfInfo {
                                url: url_str.clone(),
                                source_page: base_url.to_string(),
                                depth: current_depth,
                                title,
                                size_hint: None,
                                content_type: None,
                                content_length: None,
                                discovered_at: chrono::Utc::now().to_rfc3339(),
                                verified: false,
                            };
                            potential_pdfs.push(pdf_info);
                            self.discovered_pdfs.insert(url_str);
                        }
                    } else if absolute_url.host() == base_url.host() {
                        // Only crawl same-domain links
                        println!("  📎 Found link: {}", url_str);
                        links.push(url_str);
                    }
                }
            }
        }
        
        println!("  📊 Found {} PDFs and {} links", potential_pdfs.len(), links.len());
        
        if potential_pdfs.is_empty() {
            return (links, 0);
        }
        
        println!(
            "🔍 Verifying {} potential PDFs concurrently...",
            potential_pdfs.len()
        );
        
        let mut futs = FuturesUnordered::new();
        for p in potential_pdfs {
            let client = self.client.clone();
            let semaphore = Arc::clone(&self.semaphore);
            let verifying_enabled = self.verify_pdfs;
            futs.push(async move {
                if verifying_enabled {
                    let r = Crawler::verify_pdf_shared(&client, &semaphore, &p.url).await;
                    (p, r)
                } else {
                    (p, Ok((None, None, false)))
                }
            });
        }
        
        let mut verified_count = 0usize;
        while let Some((mut pdf_info, verification_result)) = futs.next().await {
            match verification_result {
                Ok((content_type, content_length, verified)) => {
                    if self.verify_pdfs && !verified {
                        println!("❌ INVALID: {} - SKIPPING", pdf_info.url);
                        if let Err(e) = self.pdf_sender.send(WriterMessage::UpdateMetadata {
                            pages_crawled: 0,
                            status: "in_progress".to_string(),
                        }).await {
                            eprintln!("Failed to send metadata update: {}", e);
                        }
                        continue;
                    }
                    
                    pdf_info.content_type = content_type;
                    pdf_info.content_length = content_length;
                    pdf_info.verified = verified;
                    
                    if verified {
                        let size_str = pdf_info.content_length.map(|size| {
                            if size > 1024 * 1024 {
                                format!(" ({:.1} MB)", size as f64 / (1024.0 * 1024.0))
                            } else if size > 1024 {
                                format!(" ({:.1} KB)", size as f64 / 1024.0)
                            } else {
                                format!(" ({} bytes)", size)
                            }
                        }).unwrap_or_default();
                        println!("✅ VERIFIED: {}{}", pdf_info.url, size_str);
                    } else {
                        println!("➕ ADDED: {} (verification disabled)", pdf_info.url);
                    }
                    
                    if let Err(e) = self.pdf_sender.send(WriterMessage::AddPdf(pdf_info)).await {
                        eprintln!("Failed to send PDF info: {}", e);
                    } else {
                        verified_count += 1;
                    }
                }
                Err(e) => {
                    println!("❌ FAILED: {} - {} - SKIPPING", pdf_info.url, e);
                    if let Err(e) = self.pdf_sender.send(WriterMessage::UpdateMetadata {
                        pages_crawled: 0,
                        status: "in_progress".to_string(),
                    }).await {
                        eprintln!("Failed to send metadata update: {}", e);
                    }
                }
            }
        }
        
        (links, verified_count)
    }

    async fn crawl(&mut self, start_url: String, max_depth: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut queue = VecDeque::new();
        let mut pages_crawled = 0;
        
        // Initialize metadata
        if let Err(e) = self.pdf_sender.send(WriterMessage::UpdateMetadata {
            pages_crawled: 0,
            status: "in_progress".to_string(),
        }).await {
            return Err(format!("Failed to send initial metadata: {}", e).into());
        }
        
        // Parse start URL
        let _base_url = Url::parse(&start_url)?;
        queue.push_back((start_url.clone(), 0));
        
        println!("🚀 Starting crawl with max depth: {}", max_depth);
        
        while let Some((current_url, depth)) = queue.pop_front() {
            println!("📋 Processing queue item: {} (depth: {})", current_url, depth);
            
            if depth > max_depth {
                println!("  ⏭️  Skipping - exceeds max depth");
                continue;
            }
            
            if self.visited.contains(&current_url) {
                println!("  ⏭️  Skipping - already visited");
                continue;
            }
            
            let url = Url::parse(&current_url)?;
            if !self.can_fetch(&url).await {
                println!("  ⏭️  Skipping - robots.txt disallows");
                continue;
            }
            
            self.visited.insert(current_url.clone());
            
            match self.fetch_page(&current_url).await {
                Ok(html) => {
                    pages_crawled += 1;
                    println!("  ✅ Successfully fetched page ({} bytes)", html.len());
                    
                    let (links, new_pdfs_found) = self.extract_and_verify_pdfs(&html, &url, depth).await;
                    
                    if new_pdfs_found > 0 {
                        println!(
                            "🚀 Verified {} PDFs from page: {} (depth: {})",
                            new_pdfs_found, current_url, depth
                        );
                    }
                    
                    // Update metadata periodically
                    if pages_crawled % 5 == 0 || new_pdfs_found > 0 {
                        if let Err(e) = self.pdf_sender.send(WriterMessage::UpdateMetadata {
                            pages_crawled,
                            status: "in_progress".to_string(),
                        }).await {
                            eprintln!("Failed to send metadata update: {}", e);
                        }
                    }
                    
                    // Add new links to queue if we haven't reached max depth
                    if depth < max_depth {
                        println!("🔗 Adding {} links to queue from {} (depth: {})", links.len(), current_url, depth);
                        for link in links {
                            if !self.visited.contains(&link) {
                                println!("  ➡️  Adding to queue: {}", link);
                                queue.push_back((link, depth + 1));
                            } else {
                                println!("  ⏭️  Skipping already visited: {}", link);
                            }
                        }
                    } else {
                        println!("⏭️  Not adding links - reached max depth {}", depth);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to fetch {}: {}", current_url, e);
                }
            }
            
            println!("📊 Queue size: {}", queue.len());
        }
        
        // Final metadata update
        if let Err(e) = self.pdf_sender.send(WriterMessage::UpdateMetadata {
            pages_crawled,
            status: "completed".to_string(),
        }).await {
            eprintln!("Failed to send final metadata: {}", e);
        }
        
        Ok(())
    }
}

async fn writer_task(
    mut results: CrawlResults,
    mut receiver: mpsc::Receiver<WriterMessage>,
    output_file: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut discovered_urls = HashSet::new();
    
    // Initialize discovered_urls with the URLs already in results.pdfs
    for pdf in &results.pdfs {
        discovered_urls.insert(pdf.url.clone());
    }
    
    // Write initial file
    write_results(&results, &output_file).await?;
    
    while let Some(msg) = receiver.recv().await {
        match msg {
            WriterMessage::AddPdf(pdf) => {
                if !discovered_urls.contains(&pdf.url) {
                    discovered_urls.insert(pdf.url.clone());
                    results.pdfs.push(pdf);
                    results.metadata.total_pdfs_found = results.pdfs.len();
                    
                    // Update verified_pdfs and failed_verifications
                    let last_pdf = &results.pdfs.last().unwrap();
                    if last_pdf.verified {
                        results.metadata.verified_pdfs += 1;
                    } else {
                        results.metadata.failed_verifications += 1;
                    }
                    
                    write_results(&results, &output_file).await?;
                }
            }
            WriterMessage::UpdateMetadata { pages_crawled, status } => {
                results.metadata.total_pages_crawled = pages_crawled;
                results.metadata.status = status;
                write_results(&results, &output_file).await?;
            }
        }
    }
    
    Ok(())
}

async fn write_results(results: &CrawlResults, output_file: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string_pretty(results)?;
    let temp_file = format!("{}.tmp", output_file);
    
    let mut file = File::create(&temp_file).await?;
    file.write_all(json.as_bytes()).await?;
    file.sync_all().await?;
    drop(file);
    
    tokio::fs::rename(&temp_file, output_file).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();
    
    println!("Starting Smart PDF Crawler...");
    println!("URL: {}", args.url);
    println!("Max depth: {}", args.depth);
    println!("Concurrency: {}", args.concurrency);
    println!(
        "PDF Verification: {}",
        if args.verify_pdfs { "enabled" } else { "disabled" }
    );
    println!("Output: {}", args.output);
    
    let (mut crawler, _sender) = Crawler::new(
        args.concurrency,
        Duration::from_millis(args.delay),
        args.respect_robots,
        args.verify_pdfs,
        args.output.clone(),
    ).await?;
    
    // Initialize metadata with start URL and max depth
    if let Err(e) = crawler.pdf_sender.send(WriterMessage::UpdateMetadata {
        pages_crawled: 0,
        status: "initializing".to_string(),
    }).await {
        return Err(format!("Failed to initialize metadata: {}", e).into());
    }
    
    match crawler.crawl(args.url, args.depth).await {
        Ok(()) => {
            // Read final results to display summary
            let content = tokio::fs::read_to_string(&args.output).await?;
            let results: CrawlResults = serde_json::from_str(&content)?;
            
            println!("\n🎉 Crawl completed!");
            println!("Pages crawled: {}", results.metadata.total_pages_crawled);
            println!("Potential/recorded PDFs: {}", results.metadata.total_pdfs_found);
            
            if results.metadata.verification_enabled {
                println!("Verified PDFs: {}", results.metadata.verified_pdfs);
                println!(
                    "Failed verifications: {}",
                    results.metadata.failed_verifications
                );
                
                let denom = (results.metadata.verified_pdfs
                    + results.metadata.failed_verifications) as f64;
                let success = if denom > 0.0 {
                    (results.metadata.verified_pdfs as f64 / denom) * 100.0
                } else {
                    0.0
                };
                println!("Success rate: {:.1}%", success);
            }
            
            println!("Results saved to: {}", args.output);
        }
        Err(e) => {
            // Mark as failed in metadata
            if let Err(_) = crawler.pdf_sender.send(WriterMessage::UpdateMetadata {
                pages_crawled: 0,
                status: "failed".to_string(),
            }).await {
                // Ignore error if we can't update metadata
            }
            eprintln!("Crawl failed: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

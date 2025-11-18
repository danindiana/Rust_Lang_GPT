//! Advanced Asynchronous Web Crawler
//!
//! This is a production-grade web crawler using Tokio for asynchronous operations.
//! It features:
//! - Fully asynchronous using Tokio runtime
//! - Dynamic worker pool management
//! - Error-based throttling and retry mechanisms
//! - RwLock for efficient concurrent URL access
//! - Configurable timeouts and worker counts
//! - Comprehensive logging
//! - Domain filtering and URL normalization

use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{self, Duration};
use url::Url;

/// Configuration constants
const USER_AGENT: &str = "HydraBot-Advanced/1.0 (+https://github.com/yourusername/hydra_mt)";
const MAX_PAGES_PER_DOMAIN: usize = 10000;
const MIN_WORKERS: usize = 5;
const MAX_WORKERS: usize = 20;
const ERROR_THRESHOLD: usize = 10;
const REQUEST_TIMEOUT_SECS: u64 = 10;
const DEFAULT_WORKERS: usize = 10;

/// Custom error types
#[derive(Debug, thiserror::Error)]
enum CrawlerError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Timeout error")]
    Timeout,

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

/// Statistics for the crawler
#[derive(Debug, Default)]
struct CrawlStats {
    pages_crawled: AtomicUsize,
    errors: AtomicUsize,
    active_workers: AtomicUsize,
}

impl CrawlStats {
    fn new(initial_workers: usize) -> Self {
        Self {
            pages_crawled: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            active_workers: AtomicUsize::new(initial_workers),
        }
    }

    fn increment_pages(&self) {
        self.pages_crawled.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    fn decrement_errors(&self) {
        self.errors.fetch_sub(1, Ordering::Relaxed);
    }

    fn pages(&self) -> usize {
        self.pages_crawled.load(Ordering::Relaxed)
    }

    fn errors(&self) -> usize {
        self.errors.load(Ordering::Relaxed)
    }

    fn workers(&self) -> usize {
        self.active_workers.load(Ordering::Relaxed)
    }

    fn adjust_workers(&self, delta: isize) {
        let current = self.active_workers.load(Ordering::Relaxed);
        let new_value = (current as isize + delta).clamp(MIN_WORKERS as isize, MAX_WORKERS as isize) as usize;
        self.active_workers.store(new_value, Ordering::Relaxed);
        info!("Adjusted worker count: {} -> {}", current, new_value);
    }
}

/// Crawler configuration
#[derive(Debug, Clone)]
struct CrawlConfig {
    output_file: String,
    excluded_domains: HashSet<String>,
    max_pages: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_millis()
        .init();

    info!("=== Advanced Async Web Crawler ===\n");

    // Get user input
    let domain = get_user_input("Please enter the domain to begin crawling: ").await?;
    let output_file = get_user_input("Please enter the file name to write the crawled results: ").await?;
    let excluded_input = get_user_input("Please enter the domains to be excluded (comma-separated): ").await?;

    let excluded_domains: HashSet<String> = excluded_input
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    // Parse starting URL
    let starting_url = parse_url(&domain)?;

    let config = CrawlConfig {
        output_file: output_file.clone(),
        excluded_domains,
        max_pages: MAX_PAGES_PER_DOMAIN,
    };

    info!("Configuration:");
    info!("  Starting URL: {}", starting_url);
    info!("  Output file: {}", output_file);
    info!("  Max pages: {}", config.max_pages);
    info!("  Excluded domains: {:?}", config.excluded_domains);
    info!("  Workers: {}-{} (dynamic)", MIN_WORKERS, MAX_WORKERS);
    info!("");

    // Run the crawler
    crawl(starting_url, config).await?;

    info!("\n=== Crawling Complete ===");

    Ok(())
}

/// Main crawling function
async fn crawl(starting_url: Url, config: CrawlConfig) -> Result<()> {
    // Initialize shared state
    let crawled_urls = Arc::new(RwLock::new(HashSet::new()));
    let urls_to_crawl = Arc::new(Mutex::new(VecDeque::new()));
    let stats = Arc::new(CrawlStats::new(DEFAULT_WORKERS));

    // Add starting URL
    urls_to_crawl.lock().await.push_back(starting_url.clone());

    // Create HTTP client
    let client = Arc::new(
        Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .pool_max_idle_per_host(10)
            .build()
            .context("Failed to build HTTP client")?,
    );

    // Open output file
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.output_file)
        .await
        .context("Failed to create output file")?;
    let file = Arc::new(Mutex::new(file));

    info!("Starting crawl with {} workers...\n", stats.workers());

    // Main crawling loop
    loop {
        // Check if we've reached the limit
        if stats.pages() >= config.max_pages {
            info!("Reached maximum pages limit: {}", config.max_pages);
            break;
        }

        // Check if there are URLs to crawl
        let queue_size = urls_to_crawl.lock().await.len();
        if queue_size == 0 && stats.pages() > 0 {
            info!("No more URLs to crawl. Queue empty.");
            break;
        }

        let num_workers = stats.workers();

        // Spawn worker tasks
        let mut handles = vec![];

        for worker_id in 0..num_workers {
            let urls_to_crawl = Arc::clone(&urls_to_crawl);
            let crawled_urls = Arc::clone(&crawled_urls);
            let client = Arc::clone(&client);
            let file = Arc::clone(&file);
            let stats = Arc::clone(&stats);
            let config = config.clone();

            let handle = tokio::spawn(async move {
                crawl_worker(
                    worker_id,
                    urls_to_crawl,
                    crawled_urls,
                    client,
                    file,
                    stats,
                    config,
                )
                .await
            });

            handles.push(handle);
        }

        // Wait for all workers to complete
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Worker task failed: {}", e);
            }
        }

        // Adjust worker count based on error rate
        adjust_workers(&stats);

        // Small delay before next iteration
        time::sleep(Duration::from_millis(500)).await;

        // Log progress
        info!(
            "Progress: {} pages crawled, {} errors, {} URLs in queue",
            stats.pages(),
            stats.errors(),
            urls_to_crawl.lock().await.len()
        );
    }

    info!(
        "\nFinal stats: {} pages crawled, {} errors encountered",
        stats.pages(),
        stats.errors()
    );

    Ok(())
}

/// Individual worker task
async fn crawl_worker(
    worker_id: usize,
    urls_to_crawl: Arc<Mutex<VecDeque<Url>>>,
    crawled_urls: Arc<RwLock<HashSet<String>>>,
    client: Arc<Client>,
    file: Arc<Mutex<tokio::fs::File>>,
    stats: Arc<CrawlStats>,
    config: CrawlConfig,
) {
    debug!("[Worker {}] Started", worker_id);

    loop {
        // Check if we've reached the limit
        if stats.pages() >= config.max_pages {
            break;
        }

        // Get next URL
        let url = {
            let mut queue = urls_to_crawl.lock().await;
            queue.pop_front()
        };

        let url = match url {
            Some(u) => u,
            None => {
                // No URLs available, sleep and try again
                time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };

        let url_str = url.as_str().to_string();

        // Check if already crawled
        {
            let crawled = crawled_urls.read().await;
            if crawled.contains(&url_str) {
                debug!("[Worker {}] Already crawled: {}", worker_id, url_str);
                continue;
            }
        }

        // Parse domain
        let domain = match url.domain() {
            Some(d) => d.to_lowercase(),
            None => {
                warn!("[Worker {}] No domain for URL: {}", worker_id, url_str);
                continue;
            }
        };

        // Check if domain is excluded
        if config.excluded_domains.contains(&domain) {
            debug!("[Worker {}] Excluded domain: {}", worker_id, domain);
            continue;
        }

        // Mark as crawled before fetching to avoid duplicates
        {
            let mut crawled = crawled_urls.write().await;
            crawled.insert(url_str.clone());
        }

        // Fetch and process URL
        match fetch_and_process_url(worker_id, &url, &client, &stats, &config).await {
            Ok(new_urls) => {
                // Add new URLs to queue
                let mut queue = urls_to_crawl.lock().await;
                for new_url in new_urls {
                    queue.push_back(new_url);
                }
                drop(queue);

                // Write to file
                let mut f = file.lock().await;
                let _ = f.write_all(format!("{}\n", url_str).as_bytes()).await;
                drop(f);

                // Update stats
                stats.increment_pages();
                if stats.errors() > 0 {
                    stats.decrement_errors();
                }

                info!("[Worker {}] Crawled: {} ({} total)", worker_id, url_str, stats.pages());
            }
            Err(e) => {
                error!("[Worker {}] Error crawling {}: {}", worker_id, url_str, e);
                stats.increment_errors();

                // Re-queue URL for retry
                urls_to_crawl.lock().await.push_back(url);
            }
        }
    }

    debug!("[Worker {}] Finished", worker_id);
}

/// Fetch and process a single URL
async fn fetch_and_process_url(
    worker_id: usize,
    url: &Url,
    client: &Client,
    stats: &Arc<CrawlStats>,
    config: &CrawlConfig,
) -> Result<Vec<Url>, CrawlerError> {
    debug!("[Worker {}] Fetching: {}", worker_id, url);

    // Send request with timeout
    let response = time::timeout(
        Duration::from_secs(REQUEST_TIMEOUT_SECS),
        client.get(url.as_str()).send(),
    )
    .await
    .map_err(|_| CrawlerError::Timeout)?
    .map_err(CrawlerError::HttpError)?;

    // Check status
    if !response.status().is_success() {
        return Err(CrawlerError::ParseError(format!(
            "HTTP status: {}",
            response.status()
        )));
    }

    // Get response text
    let content = response
        .text()
        .await
        .map_err(CrawlerError::HttpError)?;

    // Parse HTML and extract URLs
    let new_urls = extract_urls(&content, url, config)?;

    debug!("[Worker {}] Extracted {} URLs from {}", worker_id, new_urls.len(), url);

    Ok(new_urls)
}

/// Extract URLs from HTML content
fn extract_urls(content: &str, base_url: &Url, config: &CrawlConfig) -> Result<Vec<Url>, CrawlerError> {
    let document = Html::parse_document(content);
    let selector = Selector::parse("a[href]")
        .map_err(|e| CrawlerError::ParseError(format!("Selector error: {:?}", e)))?;

    let mut urls = Vec::new();

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            // Parse and normalize URL
            if let Ok(url) = base_url.join(href) {
                // Filter by scheme
                if url.scheme() == "http" || url.scheme() == "https" {
                    // Check if domain is excluded
                    if let Some(domain) = url.domain() {
                        if !config.excluded_domains.contains(&domain.to_lowercase()) {
                            urls.push(url);
                        }
                    }
                }
            }
        }
    }

    Ok(urls)
}

/// Adjust worker count based on error rate
fn adjust_workers(stats: &Arc<CrawlStats>) {
    let error_count = stats.errors();
    let current_workers = stats.workers();

    if error_count >= ERROR_THRESHOLD && current_workers > MIN_WORKERS {
        stats.adjust_workers(-1);
        warn!(
            "High error rate ({}). Decreasing workers to {}",
            error_count,
            stats.workers()
        );
    } else if error_count < ERROR_THRESHOLD / 2 && current_workers < MAX_WORKERS {
        stats.adjust_workers(1);
        info!(
            "Low error rate ({}). Increasing workers to {}",
            error_count,
            stats.workers()
        );
    }
}

/// Get user input asynchronously
async fn get_user_input(prompt: &str) -> Result<String> {
    use std::io::{self, Write};

    print!("{}", prompt);
    io::stdout().flush()?;

    // Read input in blocking task to avoid blocking the async runtime
    let input = tokio::task::spawn_blocking(|| {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok::<_, io::Error>(input)
    })
    .await??;

    Ok(input.trim().to_string())
}

/// Parse URL with fallback to prepending http://
fn parse_url(input: &str) -> Result<Url> {
    Url::parse(input)
        .or_else(|_| Url::parse(&format!("http://{}", input)))
        .context("Invalid URL format")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url() {
        let url1 = parse_url("https://example.com").unwrap();
        assert_eq!(url1.scheme(), "https");

        let url2 = parse_url("example.com").unwrap();
        assert_eq!(url2.scheme(), "http");
    }

    #[test]
    fn test_stats() {
        let stats = CrawlStats::new(10);
        assert_eq!(stats.pages(), 0);
        assert_eq!(stats.workers(), 10);

        stats.increment_pages();
        assert_eq!(stats.pages(), 1);

        stats.increment_errors();
        assert_eq!(stats.errors(), 1);

        stats.decrement_errors();
        assert_eq!(stats.errors(), 0);
    }

    #[tokio::test]
    async fn test_url_extraction() {
        let html = r#"
            <html>
                <body>
                    <a href="https://example.com/page1">Link 1</a>
                    <a href="/page2">Link 2</a>
                    <a href="mailto:test@example.com">Email</a>
                </body>
            </html>
        "#;

        let base_url = Url::parse("https://example.com").unwrap();
        let config = CrawlConfig {
            output_file: "test.txt".to_string(),
            excluded_domains: HashSet::new(),
            max_pages: 100,
        };

        let urls = extract_urls(html, &base_url, &config).unwrap();

        assert_eq!(urls.len(), 2); // mailto should be filtered out
        assert!(urls.iter().any(|u| u.as_str() == "https://example.com/page1"));
        assert!(urls.iter().any(|u| u.as_str() == "https://example.com/page2"));
    }
}

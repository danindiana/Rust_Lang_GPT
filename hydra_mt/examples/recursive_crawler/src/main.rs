//! Recursive Web Crawler with Depth Control
//!
//! This example demonstrates a recursive web crawler that:
//! - Crawls web pages up to a configurable depth
//! - Supports internal-only or external domain crawling
//! - Provides comprehensive logging to both console and file
//! - Uses parallel processing for link extraction
//!
//! # Features
//! - Depth-limited recursive crawling
//! - Domain filtering (internal vs external links)
//! - Comprehensive error logging
//! - Thread-safe URL tracking
//! - Parallel link processing with Rayon

use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use rayon::prelude::*;
use reqwest::blocking::{Client, ClientBuilder};
use select::document::Document;
use select::predicate::Name;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use url::Url;

/// Configuration constants
const USER_AGENT: &str = "HydraBot-Recursive/1.0";
const TIMEOUT_SECS: u64 = 10;
const DEFAULT_MAX_DEPTH: usize = 3;

/// Configuration for the crawler
#[derive(Debug, Clone)]
struct CrawlConfig {
    max_depth: usize,
    allow_external: bool,
    base_domain: String,
}

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("=== Recursive Web Crawler ===\n");

    // Get user configuration
    let base_url = get_user_input("Enter the URL to crawl: ")?;
    let max_depth = get_max_depth()?;
    let allow_external = get_allow_external()?;

    let config = CrawlConfig {
        max_depth,
        allow_external,
        base_domain: base_url
            .domain()
            .unwrap_or("unknown")
            .to_string(),
    };

    info!("Configuration:");
    info!("  Base URL: {}", base_url);
    info!("  Max Depth: {}", config.max_depth);
    info!("  Allow External: {}", config.allow_external);
    info!("");

    // Initialize thread-safe data structures
    let crawled_urls = Arc::new(Mutex::new(HashSet::new()));
    crawled_urls.lock().unwrap().insert(base_url.clone());

    // Create output files
    let output_filename = format!(
        "crawled_urls_{}.txt",
        base_url.host_str().unwrap_or("output")
    );
    let log_filename = format!(
        "crawl_log_{}.txt",
        base_url.host_str().unwrap_or("output")
    );

    let output_file = File::create(&output_filename)
        .context("Failed to create output file")?;
    let output_file = Arc::new(Mutex::new(BufWriter::new(output_file)));

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_filename)
        .context("Failed to create log file")?;
    let log_file = Arc::new(Mutex::new(BufWriter::new(log_file)));

    info!("Output file: {}", output_filename);
    info!("Log file: {}\n", log_filename);

    // Create HTTP client
    let client = Arc::new(
        ClientBuilder::new()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()
            .context("Failed to build HTTP client")?,
    );

    // Start recursive crawling
    info!("Starting recursive crawl...\n");
    recursive_crawl(
        base_url.clone(),
        crawled_urls.clone(),
        output_file.clone(),
        log_file.clone(),
        config,
        client,
        0,
    )?;

    // Final output
    let final_urls = crawled_urls.lock().unwrap();
    let url_count = final_urls.len();

    info!("\n=== Crawling Summary ===");
    info!("Total URLs crawled: {}", url_count);

    // Write all URLs to output file
    final_urls.par_iter().for_each(|url| {
        if let Ok(mut file) = output_file.lock() {
            let _ = writeln!(file, "{}", url);
        }
    });

    // Flush files
    if let Ok(mut file) = output_file.lock() {
        file.flush().ok();
    }
    if let Ok(mut file) = log_file.lock() {
        file.flush().ok();
    }

    info!("Results saved to: {}", output_filename);
    info!("Logs saved to: {}", log_filename);

    Ok(())
}

/// Recursive crawl function with depth control
fn recursive_crawl(
    url: Url,
    crawled_urls: Arc<Mutex<HashSet<Url>>>,
    output_file: Arc<Mutex<BufWriter<File>>>,
    log_file: Arc<Mutex<BufWriter<File>>>,
    config: CrawlConfig,
    client: Arc<Client>,
    current_depth: usize,
) -> Result<()> {
    // Check depth limit
    if current_depth > config.max_depth {
        debug!("Reached max depth at: {}", url);
        return Ok(());
    }

    info!(
        "[Depth {}/{}] Crawling: {}",
        current_depth, config.max_depth, url
    );

    // Log the crawl attempt
    log_to_file(&log_file, &format!("CRAWLING [depth={}]: {}", current_depth, url))?;

    // Fetch the page
    let response = match client.get(url.as_str()).send() {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("Non-success status {} for: {}", resp.status(), url);
                log_to_file(
                    &log_file,
                    &format!("ERROR: HTTP {} for {}", resp.status(), url),
                )?;
                return Ok(());
            }
            resp
        }
        Err(e) => {
            error!("Failed to fetch {}: {}", url, e);
            log_to_file(&log_file, &format!("ERROR: {} - {}", url, e))?;
            return Ok(());
        }
    };

    // Parse the HTML
    let document = match Document::from_read(response) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to parse HTML from {}: {}", url, e);
            log_to_file(&log_file, &format!("PARSE_ERROR: {} - {}", url, e))?;
            return Ok(());
        }
    };

    log_to_file(&log_file, &format!("SUCCESS: {}", url))?;

    // Extract all links
    let links: Vec<_> = document
        .find(Name("a"))
        .filter_map(|node| node.attr("href"))
        .collect();

    debug!("  Found {} links", links.len());

    // Process links in parallel
    links.into_par_iter().for_each(|link| {
        // Parse the URL
        if let Ok(new_url) = Url::parse(link).or_else(|_| url.join(link)) {
            // Check if we should crawl this URL
            if !should_crawl(&url, &new_url, &config) {
                debug!("  Skipping (domain filter): {}", new_url);
                return;
            }

            // Check if URL is new
            let is_new = {
                let mut urls = crawled_urls.lock().unwrap();
                urls.insert(new_url.clone())
            };

            if is_new {
                info!("  [+] Found: {}", new_url);

                // Recursively crawl (note: this can create many parallel branches)
                let _ = recursive_crawl(
                    new_url,
                    crawled_urls.clone(),
                    output_file.clone(),
                    log_file.clone(),
                    config.clone(),
                    client.clone(),
                    current_depth + 1,
                );
            } else {
                debug!("  [-] Already crawled: {}", new_url);
            }
        }
    });

    Ok(())
}

/// Prompts the user for input and parses it as a URL
fn get_user_input(prompt: &str) -> Result<Url> {
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read user input")?;

    let input = input.trim();

    Url::parse(input)
        .or_else(|_| Url::parse(&format!("http://{}", input)))
        .context("Invalid URL format")
}

/// Gets the maximum crawl depth from user
fn get_max_depth() -> Result<usize> {
    print!("Enter the max depth for recursive crawling [default: {}]: ", DEFAULT_MAX_DEPTH);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim();
    if input.is_empty() {
        return Ok(DEFAULT_MAX_DEPTH);
    }

    input
        .parse()
        .context("Invalid depth value, must be a number")
}

/// Asks if external domains should be allowed
fn get_allow_external() -> Result<bool> {
    print!("Allow external domains? (yes/no) [default: no]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("yes"))
}

/// Determines if a URL should be crawled based on configuration
fn should_crawl(base_url: &Url, target_url: &Url, config: &CrawlConfig) -> bool {
    // Skip non-HTTP(S) schemes
    if target_url.scheme() != "http" && target_url.scheme() != "https" {
        return false;
    }

    // Check domain restriction
    if !config.allow_external {
        if let (Some(base_domain), Some(target_domain)) = (base_url.domain(), target_url.domain())
        {
            if base_domain != target_domain {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

/// Writes a log message to the log file
fn log_to_file(log_file: &Arc<Mutex<BufWriter<File>>>, message: &str) -> Result<()> {
    if let Ok(mut file) = log_file.lock() {
        writeln!(file, "{}", message).context("Failed to write to log file")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_crawl_same_domain() {
        let base = Url::parse("https://example.com/page1").unwrap();
        let target = Url::parse("https://example.com/page2").unwrap();
        let config = CrawlConfig {
            max_depth: 3,
            allow_external: false,
            base_domain: "example.com".to_string(),
        };

        assert!(should_crawl(&base, &target, &config));
    }

    #[test]
    fn test_should_crawl_external_blocked() {
        let base = Url::parse("https://example.com/page1").unwrap();
        let target = Url::parse("https://external.com/page2").unwrap();
        let config = CrawlConfig {
            max_depth: 3,
            allow_external: false,
            base_domain: "example.com".to_string(),
        };

        assert!(!should_crawl(&base, &target, &config));
    }

    #[test]
    fn test_should_crawl_external_allowed() {
        let base = Url::parse("https://example.com/page1").unwrap();
        let target = Url::parse("https://external.com/page2").unwrap();
        let config = CrawlConfig {
            max_depth: 3,
            allow_external: true,
            base_domain: "example.com".to_string(),
        };

        assert!(should_crawl(&base, &target, &config));
    }
}

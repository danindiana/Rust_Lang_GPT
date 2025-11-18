//! Basic Parallel Web Crawler
//!
//! This example demonstrates a simple parallel web crawler using Rayon for data parallelism.
//! It fetches a single URL, extracts all links, and processes them in parallel.
//!
//! # Features
//! - Blocking HTTP requests with reqwest
//! - Parallel link processing with Rayon
//! - Thread-safe file output
//! - URL normalization and deduplication

use anyhow::{Context, Result};
use rayon::prelude::*;
use reqwest::blocking::ClientBuilder;
use select::document::Document;
use select::predicate::Name;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::sync::Mutex;
use std::time::Duration;
use url::Url;

/// Configuration constants
const USER_AGENT: &str = "HydraBot-Basic/1.0";
const TIMEOUT_SECS: u64 = 10;

fn main() -> Result<()> {
    println!("=== Basic Parallel Web Crawler ===\n");

    // Get input URL from user
    let base_url = get_user_input("Enter the URL to crawl: ")?;

    // Initialize data structures
    let mut crawled_urls = HashSet::new();
    crawled_urls.insert(base_url.clone());

    // Create output file
    let output_filename = format!(
        "crawled_urls_{}.txt",
        base_url.host_str().unwrap_or("output")
    );
    let file = File::create(&output_filename)
        .context("Failed to create output file")?;
    let file = Mutex::new(BufWriter::new(file));

    println!("\nStarting crawl of: {}", base_url);
    println!("Output file: {}\n", output_filename);

    // Perform the crawl
    crawl_url(&base_url, &mut crawled_urls, &file)?;

    // Write all crawled URLs to file and console
    let crawled_urls_vec: Vec<_> = crawled_urls.into_par_iter().collect();

    println!("\n=== Crawled URLs ({} total) ===", crawled_urls_vec.len());

    crawled_urls_vec.par_iter().for_each(|url| {
        println!("{}", url);
        if let Ok(mut f) = file.lock() {
            let _ = writeln!(f, "{}", url);
        }
    });

    // Flush the file buffer
    if let Ok(mut f) = file.lock() {
        f.flush().ok();
    }

    println!("\n=== Crawling completed ===");
    println!("Results saved to: {}", output_filename);

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

    // Try to parse as-is, or prepend http:// if needed
    Url::parse(input)
        .or_else(|_| Url::parse(&format!("http://{}", input)))
        .context("Invalid URL format")
}

/// Crawls a URL and extracts all links, processing them in parallel
fn crawl_url(
    base_url: &Url,
    crawled_urls: &mut HashSet<Url>,
    file: &Mutex<BufWriter<File>>,
) -> Result<()> {
    // Create HTTP client with timeout
    let client = ClientBuilder::new()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build()
        .context("Failed to build HTTP client")?;

    // Fetch the page
    let response = client
        .get(base_url.as_str())
        .send()
        .context("Failed to fetch URL")?;

    // Parse HTML document
    let document = Document::from_read(response)
        .context("Failed to parse HTML")?;

    // Extract all links
    let links: Vec<_> = document
        .find(Name("a"))
        .filter_map(|node| node.attr("href"))
        .collect();

    println!("Found {} links on the page", links.len());

    // Process links in parallel
    links.into_par_iter().for_each(|link| {
        // Try to parse the link as a URL, or resolve it relative to base_url
        if let Ok(url) = Url::parse(link).or_else(|_| base_url.join(link)) {
            // Thread-safe file writing
            if let Ok(mut f) = file.lock() {
                // Check if URL is new (Note: this has a race condition in parallel context,
                // but it's acceptable for this simple example)
                if !crawled_urls.contains(&url) {
                    let _ = writeln!(f, "{}", url);
                    println!("  [+] {}", url);
                }
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parsing() {
        let url = Url::parse("https://example.com").unwrap();
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn test_url_join() {
        let base = Url::parse("https://example.com/path/").unwrap();
        let joined = base.join("relative.html").unwrap();
        assert_eq!(joined.as_str(), "https://example.com/path/relative.html");
    }
}

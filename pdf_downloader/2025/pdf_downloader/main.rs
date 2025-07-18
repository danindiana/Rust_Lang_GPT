use std::{
    collections::{HashSet, VecDeque},
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::{
    header::{HeaderMap, CONTENT_DISPOSITION, CONTENT_TYPE},
    Client, Url,
};
use select::{document::Document, predicate::Name};
use thirtyfour::{DesiredCapabilities, WebDriver};
use url::Host;

// --- Constants ---
const DEFAULT_DOWNLOAD_DIR: &str = "downloaded_pdfs";
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50MB
const REQUEST_DELAY: Duration = Duration::from_secs(1);
const REQUEST_JITTER: Duration = Duration::from_millis(500);
const RENDER_TIMEOUT: Duration = Duration::from_secs(10);

// --- Static Regex ---
lazy_static! {
    // Regex to extract filename from Content-Disposition header.
    static ref FILENAME_REGEX: Regex = Regex::new(r#"filename\*?=['"]?(?:UTF-\d['"]*)?([^"'\s;]+)"#).unwrap();
    // Regex to find characters that are invalid in filenames on most OSes.
    static ref DANGEROUS_CHARS: Regex = Regex::new(r#"[\\/*?:"<>|\x00-\x1f]"#).unwrap();
}

/// A secure PDF downloader that crawls web pages, finds PDF links, and downloads them.
struct SecurePDFDownloader {
    download_dir: PathBuf,
    visited_urls: HashSet<Url>,
    client: Client,
    driver: WebDriver,
}

impl SecurePDFDownloader {
    /// Creates a new instance of the downloader.
    /// Initializes the HTTP client, WebDriver, and creates the download directory.
    async fn new() -> Result<Self> {
        let download_dir = Path::new(DEFAULT_DOWNLOAD_DIR).to_path_buf();
        fs::create_dir_all(&download_dir)
            .context("Failed to create download directory")?;

        // Use an asynchronous reqwest client for all HTTP requests.
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let driver = Self::init_selenium().await?;

        Ok(Self {
            download_dir,
            visited_urls: HashSet::new(),
            client,
            driver,
        })
    }

    /// Initializes and returns a Selenium WebDriver instance.
    async fn init_selenium() -> Result<WebDriver> {
        let caps = DesiredCapabilities::chrome();
        // NOTE: This requires a running chromedriver instance on localhost:9515.
        let driver = WebDriver::new("http://localhost:9515", caps)
            .await
            .context("Failed to connect to WebDriver. Is chromedriver running?")?;
        Ok(driver)
    }

    /// Sanitizes a string to be a valid filename.
    fn sanitize_filename(&self, filename: &str) -> String {
        // Decode URL-encoded characters.
        let decoded = urlencoding::decode(filename).unwrap_or_else(|_| filename.into());
        // Replace characters that are invalid in filenames.
        let sanitized = DANGEROUS_CHARS.replace_all(&decoded, "_");
        // Trim leading/trailing dots and whitespace which can cause issues.
        let trimmed = sanitized.trim_matches(|c: char| c == '.' || c.is_whitespace());
        // Limit filename length to prevent issues with filesystems.
        trimmed.chars().take(255).collect()
    }

    /// Extracts a safe filename from response headers or the URL.
    fn get_safe_filename(&self, url: &Url, headers: &HeaderMap) -> String {
        // Prioritize the Content-Disposition header, as it's the most reliable source.
        if let Some(disposition) = headers.get(CONTENT_DISPOSITION) {
            if let Ok(disposition_str) = disposition.to_str() {
                if let Some(captures) = FILENAME_REGEX.captures(disposition_str) {
                    if let Some(filename) = captures.get(1) {
                        return self.sanitize_filename(filename.as_str());
                    }
                }
            }
        }

        // As a fallback, use the last segment of the URL path.
        let filename = Path::new(url.path())
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document.pdf");

        self.sanitize_filename(filename)
    }

    /// Asynchronously checks if a URL points to a PDF file.
    async fn is_pdf_link(&self, url: &Url) -> bool {
        // Fast check for the .pdf extension.
        if url.path().to_lowercase().ends_with(".pdf") {
            return true;
        }

        // For links without the extension, send a HEAD request to check the Content-Type.
        match self.client.head(url.clone()).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    return false;
                }
                let content_type = response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("")
                    .to_lowercase();

                content_type.contains("application/pdf")
            }
            Err(_) => false,
        }
    }

    /// Uses Selenium to render a JavaScript-heavy page and get its final HTML source.
    async fn get_rendered_page(&self, url: &Url) -> Result<String> {
        self.driver.goto(url.as_str()).await?;

        let start = Instant::now();
        loop {
            // Check for timeout.
            if start.elapsed() > RENDER_TIMEOUT {
                return Err(anyhow::anyhow!("Timeout waiting for page to render"));
            }

            // A simple heuristic to wait for the page to load: check if the readyState is 'complete'.
            if let Ok(ready_state) = self.driver.execute("return document.readyState", vec![]).await {
                if let Ok(state_str) = ready_state.convert::<String>() {
                    if state_str == "complete" {
                        break;
                    }
                }
            }
            
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        self.driver.source().await.map_err(|e| e.into())
    }

    /// Asynchronously downloads a PDF from a URL and saves it to the download directory.
    async fn download_pdf(&self, url: &Url) -> Result<String> {
        // Polite delay between requests to avoid overwhelming the server.
        let jitter = rand::random::<u64>() % REQUEST_JITTER.as_millis() as u64;
        let delay = REQUEST_DELAY + Duration::from_millis(jitter);
        tokio::time::sleep(delay).await;

        let mut response = self.client.get(url.clone()).send().await?;
        response.error_for_status_ref()?;

        // Verify content type from the response headers.
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        if !content_type.contains("application/pdf") {
            return Err(anyhow::anyhow!("Not a PDF file (Content-Type: {})", content_type));
        }

        // Check file size from Content-Length header before downloading.
        let size = response
            .content_length()
            .unwrap_or(0);

        if size > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!("File too large ({} bytes)", size));
        }

        // Generate a safe filename and construct the full path.
        let filename = self.get_safe_filename(url, response.headers());
        let filepath = self.download_dir.join(&filename);

        // Security check: ensure the final path is within the download directory.
        if !filepath.starts_with(&self.download_dir) {
            return Err(anyhow::anyhow!("Path traversal attempt detected"));
        }

        // Create the file. Note: std::fs is blocking. For extreme performance,
        // one might use tokio::fs, but for this tool, it's an acceptable trade-off.
        let mut file = File::create(&filepath)?;
        let mut downloaded: u64 = 0;

        // Stream the response body chunk by chunk.
        while let Some(chunk) = response.chunk().await? {
            downloaded += chunk.len() as u64;
            if downloaded > MAX_FILE_SIZE {
                // Clean up the partial file if the size limit is exceeded during download.
                drop(file);
                fs::remove_file(&filepath)?;
                return Err(anyhow::anyhow!("File size exceeded limit during download"));
            }
            file.write_all(&chunk)?;
        }

        Ok(filename)
    }

    /// Checks if a target URL is on the same domain as the base URL.
    fn is_same_domain(&self, base_url: &Url, target_url: &Url) -> bool {
        matches!((base_url.host(), target_url.host()), (Some(Host::Domain(base)), Some(Host::Domain(target))) if base == target)
    }

    /// The main crawling and downloading logic.
    pub async fn scan_and_download(
        &mut self,
        start_url_str: &str,
        recursive: bool,
        max_depth: Option<usize>,
    ) -> Result<()> {
        let start_url = Url::parse(start_url_str).context("Invalid start URL")?;
        let mut queue = VecDeque::new();
        queue.push_back((start_url.clone(), 0));
        let mut download_count = 0;

        while let Some((current_url, depth)) = queue.pop_front() {
            if self.visited_urls.contains(&current_url) {
                continue;
            }
            self.visited_urls.insert(current_url.clone());

            if let Some(max) = max_depth {
                if depth >= max {
                    println!("-> Reached max depth at: {}", current_url);
                    continue;
                }
            }

            println!("\nScanning [Depth {}]: {}", depth, current_url);

            // Get rendered page content using Selenium.
            let page_source = match self.get_rendered_page(&current_url).await {
                Ok(source) => source,
                Err(e) => {
                    eprintln!("  ! Warning: Could not render page: {}", e);
                    continue;
                }
            };

            // Parse HTML and find all links.
            let document = Document::from(page_source.as_str());
            for link in document.find(Name("a")) {
                if let Some(href) = link.attr("href") {
                    let href = href.trim();
                    if href.is_empty() || href.starts_with("mailto:") || href.starts_with("javascript:") {
                        continue;
                    }

                    // Resolve relative URLs into absolute URLs.
                    let full_url = match current_url.join(href) {
                        Ok(url) => url,
                        Err(_) => {
                            eprintln!("  ! Warning: Could not parse link: {}", href);
                            continue;
                        }
                    };

                    // Check if the link is a PDF and download it.
                    if self.is_pdf_link(&full_url).await {
                        match self.download_pdf(&full_url).await {
                            Ok(filename) => {
                                download_count += 1;
                                println!("  ✓ Downloaded: {}", filename);
                            }
                            Err(e) => {
                                eprintln!("  ✗ Failed to download {}: {}", full_url, e);
                            }
                        }
                    }
                    // If recursive, queue other links on the same domain.
                    else if recursive && self.is_same_domain(&start_url, &full_url) {
                        queue.push_back((full_url, depth + 1));
                    }
                }
            }
        }

        println!("\nScan complete. Total PDFs downloaded: {}", download_count);
        Ok(())
    }

    /// Quits the WebDriver to clean up the browser session.
    pub async fn cleanup(self) -> Result<()> {
        self.driver.quit().await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Secure PDF Downloader ===");
    println!("NOTE: Ensure chromedriver is running before starting.");

    let target_url = {
        print!("Enter target URL: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    let recursive = {
        print!("Recursive search on the same domain? (y/n): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_lowercase() == "y"
    };

    let max_depth = if recursive {
        print!("Max depth (e.g., 2, or Enter for unlimited): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().parse().ok()
    } else {
        None
    };

    let mut downloader = SecurePDFDownloader::new().await?;

    if let Err(e) = downloader
        .scan_and_download(&target_url, recursive, max_depth)
        .await
    {
        eprintln!("\nAn error occurred during scanning: {}", e);
    }

    downloader.cleanup().await?;
    Ok(())
}

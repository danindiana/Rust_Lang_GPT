// ---------------------------------------------------------------
//  PDF Crawler – incremental JSON with timestamp
// ---------------------------------------------------------------
use clap::Parser;
use futures::stream::{FuturesUnordered, StreamExt};
use hyper::{Body, Client, Method, Request, Uri};
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use url::Url;

// ---------- CLI --------------------------------------------------------------
#[derive(Parser, Debug)]
#[command(name = "pdf-crawler")]
struct Args {
    #[arg(short, long)]
    url: String,
    #[arg(short, long, default_value = "5")]
    depth: usize,
    #[arg(short, long, default_value = "12")]
    concurrency: usize,
    #[arg(short, long)]
    output: Option<String>, // optional
    #[arg(long, default_value = "true")]
    verify_pdfs: bool,
}

// ---------- Data -------------------------------------------------------------
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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

// ---------- Telemetry --------------------------------------------------------
fn log(msg: &str) {
    println!(
        "[{}] {}",
        chrono::Local::now().format("%H:%M:%S%.3f"),
        msg
    );
}

// ---------- Incremental writer ----------------------------------------------
struct IncrementalWriter {
    path: PathBuf,
    metadata: CrawlMetadata,
    pdfs: Vec<PdfInfo>,
    seen: HashSet<String>,
}

impl IncrementalWriter {
    fn new(start_url: String, max_depth: usize, verification_enabled: bool) -> Self {
        let file_name = format!(
            "pdfs_{}.json",
            chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S")
        );
        let meta = CrawlMetadata {
            start_url,
            max_depth,
            total_pages_crawled: 0,
            total_pdfs_found: 0,
            verified_pdfs: 0,
            failed_verifications: 0,
            crawl_timestamp: chrono::Utc::now().to_rfc3339(),
            status: "running".to_string(),
            verification_enabled,
        };
        Self {
            path: PathBuf::from(file_name),
            metadata: meta,
            pdfs: Vec::new(),
            seen: HashSet::new(),
        }
    }

    async fn add_pdf(&mut self, pdf: PdfInfo) {
        if self.seen.contains(&pdf.url) {
            return;
        }
        self.seen.insert(pdf.url.clone());
        self.pdfs.push(pdf.clone());
        self.metadata.total_pdfs_found += 1;
        if pdf.verified {
            self.metadata.verified_pdfs += 1;
        } else {
            self.metadata.failed_verifications += 1;
        }
        log(&format!(
            "📎 {} PDF ({}) – {}",
            if pdf.verified { "✅" } else { "➕" },
            human_bytes(pdf.content_length.unwrap_or(0)),
            pdf.url
        ));
        self.flush().await;
    }

    async fn inc_pages(&mut self) {
        self.metadata.total_pages_crawled += 1;
        self.flush().await;
    }

    async fn finish(&mut self) {
        self.metadata.status = "completed".to_string();
        self.flush().await;
    }

    async fn flush(&self) {
        let results = CrawlResults {
            metadata: self.metadata.clone(),
            pdfs: self.pdfs.clone(),
        };
        let json = serde_json::to_string_pretty(&results).unwrap();
        let tmp = self.path.with_extension("json.tmp");
        tokio::fs::write(&tmp, json).await.unwrap();
        tokio::fs::rename(tmp, &self.path).await.unwrap();
    }
}

// ---------- Crawler ----------------------------------------------------------
struct Crawler {
    client: Client<HttpsConnector<HttpConnector>>,
    semaphore: Arc<Semaphore>,
    writer: Arc<tokio::sync::Mutex<IncrementalWriter>>,
}

impl Crawler {
    fn new(
        concurrency: usize,
        start_url: String,
        max_depth: usize,
        verify_pdfs: bool,
    ) -> Self {
        let https = HttpsConnector::new();
        let client = Client::builder()
            .pool_max_idle_per_host(5)
            .pool_idle_timeout(Duration::from_secs(30))
            .build::<_, Body>(https);

        let writer = Arc::new(tokio::sync::Mutex::new(IncrementalWriter::new(
            start_url.clone(),
            max_depth,
            verify_pdfs,
        )));
        Self {
            client,
            semaphore: Arc::new(Semaphore::new(concurrency)),
            writer,
        }
    }

    // ---------- PDF heuristics
    fn is_likely_pdf_url(&self, url: &str) -> bool {
        let url_lower = url.to_lowercase();
        url_lower.ends_with(".pdf")
            || (url_lower.contains("pdf")
                && (url_lower.contains("download")
                    || url_lower.contains("file")
                    || url_lower.contains("doc")
                    || url_lower.contains("paper")
                    || url_lower.contains("publication")
                    || url_lower.contains("proceedings")))
            || (url_lower.contains("view") && url_lower.contains("pdf"))
            || url_lower.contains("format=pdf")
            || url_lower.contains("type=pdf")
            || url_lower.contains("export=pdf")
            || url_lower.contains(".pdf?")
    }

    // ---------- PDF verification
    async fn verify_pdf(
        &self,
        url: &str,
    ) -> Result<(Option<String>, Option<u64>, bool), String> {
        let _permit = self.semaphore.acquire().await.map_err(|e| e.to_string())?;

        let uri: Uri = url.parse().map_err(|_| "Bad URL".to_string())?;

        // HEAD
        let req = Request::builder()
            .method(Method::HEAD)
            .uri(uri.clone())
            .header("User-Agent", "pdf-crawler/1.0")
            .body(Body::empty())
            .unwrap();
        let resp = self.client.request(req).await;
        let (mut ct, mut len, ok) = match resp {
            Ok(r) if r.status().is_success() => (
                r.headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_lowercase()),
                r.headers()
                    .get("content-length")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse().ok()),
                true,
            ),
            _ => (None, None, false),
        };

        if !ok || ct.is_none() || len.unwrap_or(0) == 0 {
            // Partial GET fallback
            let req = Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header("User-Agent", "pdf-crawler/1.0")
                .header("Range", "bytes=0-8191")
                .body(Body::empty())
                .unwrap();
            let resp = self.client.request(req).await.map_err(|e| e.to_string())?;
            if !(resp.status().is_success() || resp.status() == 206) {
                return Err("GET failed".into());
            }
            ct = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_lowercase());
            len = resp
                .headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(|r| r.split('/').nth(1))
                .and_then(|s| s.parse().ok())
                .or_else(|| {
                    resp.headers()
                        .get("content-length")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse().ok())
                });

            let bytes = hyper::body::to_bytes(resp.into_body())
                .await
                .map_err(|e| e.to_string())?;
            let has_magic = bytes.windows(4).any(|w| w == b"%PDF");
            let is_pdf_ct = ct
                .as_ref()
                .map_or(false, |c| c.contains("application/pdf"));
            let valid = has_magic && len.unwrap_or(0) > 1024 && (is_pdf_ct || has_magic);
            return Ok((ct, len, valid));
        }

        let is_pdf_ct = ct
            .as_ref()
            .map_or(false, |c| c.contains("application/pdf"));
        let valid = is_pdf_ct && len.unwrap_or(0) > 1024;
        Ok((ct, len, valid))
    }

    // ---------- Fetch page
    async fn fetch_page(&self, url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let _permit = self.semaphore.acquire().await?;
        let uri: Uri = url.parse()?;
        let req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("User-Agent", "pdf-crawler/1.0")
            .body(Body::empty())?;
        let resp = self.client.request(req).await?;
        log(&format!("📡 HTTP {} – {}", resp.status(), url));
        let bytes = hyper::body::to_bytes(resp.into_body()).await?;
        Ok(String::from_utf8(bytes.to_vec())?)
    }

    // ---------- Main crawl loop
    pub async fn crawl(&self, start_url: String, max_depth: usize, verify_pdfs: bool) {
        let mut seen: HashSet<String> = HashSet::new();
        let mut pending = FuturesUnordered::new();

        seen.insert(start_url.clone());
        pending.push(self.crawl_one(
            start_url.clone(),
            0,
            start_url.clone(),
            verify_pdfs,
        ));

        while let Some((url, depth, links, pdfs)) = pending.next().await {
            {
                let mut w = self.writer.lock().await;
                w.inc_pages().await;
            }

            // enqueue deeper links
            if depth < max_depth {
                for l in links {
                    if !seen.contains(&l) {
                        seen.insert(l.clone());
                        log(&format!("📥 QUEUED    {} (depth {})", l, depth + 1));
                        pending.push(self.crawl_one(l, depth + 1, url.clone(), verify_pdfs));
                    }
                }
            }

            // store PDFs
            for p in pdfs {
                let mut w = self.writer.lock().await;
                w.add_pdf(p).await;
            }
        }

        let mut w = self.writer.lock().await;
        w.finish().await;
    }

    // ---------- Task executed for every URL
    async fn crawl_one(
        &self,
        url: String,
        depth: usize,
        _source_page: String,
        verify_pdfs: bool,
    ) -> (String, usize, Vec<String>, Vec<PdfInfo>) {
        let mut links = Vec::new();
        let mut pdfs = Vec::new();

        let base_url = match Url::parse(&url) {
            Ok(u) => u,
            Err(_) => return (url, depth, links, pdfs),
        };

        let html = match self.fetch_page(&url).await {
            Ok(h) => {
                log(&format!("📄 FETCHED   {} ({} bytes)", url, h.len()));
                h
            }
            Err(e) => {
                log(&format!("❌ ERROR     {} – {}", url, e));
                return (url, depth, links, pdfs);
            }
        };

        let doc = Html::parse_document(&html);
        let selector = Selector::parse("a[href]").unwrap();

        for a in doc.select(&selector) {
            let href = match a.value().attr("href") {
                Some(h) => h.trim(),
                None => continue,
            };
            if href.is_empty() || href.starts_with("javascript:") || href.starts_with("mailto:") {
                continue;
            }

            let absolute = match base_url.join(href) {
                Ok(abs) => abs,
                Err(_) => continue,
            };

            let url_str = absolute.to_string();
            if absolute.host() != base_url.host() {
                continue;
            }

            if self.is_likely_pdf_url(&url_str) {
                let title = a.text().collect::<String>().trim().to_string();
                let title = if title.is_empty() { None } else { Some(title) };

                let (ct, len, verified) = if verify_pdfs {
                    match self.verify_pdf(&url_str).await {
                        Ok(v) => v,
                        Err(_) => continue,
                    }
                } else {
                    (None, None, false)
                };

                pdfs.push(PdfInfo {
                    url: url_str.clone(),
                    source_page: url.clone(),
                    depth,
                    title,
                    size_hint: len.map(human_bytes),
                    content_type: ct,
                    content_length: len,
                    discovered_at: chrono::Utc::now().to_rfc3339(),
                    verified,
                });
            } else {
                links.push(url_str);
            }
        }
        (url, depth, links, pdfs)
    }
}

// ---------- Utility
fn human_bytes(bytes: u64) -> String {
    if bytes > 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes > 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

// ---------- Entry point
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    log("🚀 PDF crawler – incremental JSON output");
    log(&format!("   start   : {}", args.url));
    log(&format!("   depth   : {}", args.depth));
    log(&format!("   workers : {}", args.concurrency));
    log(&format!("   verify  : {}", args.verify_pdfs));

    let crawler = Crawler::new(
        args.concurrency,
        args.url.clone(),
        args.depth,
        args.verify_pdfs,
    );
    crawler.crawl(args.url, args.depth, args.verify_pdfs).await;

    Ok(())
}

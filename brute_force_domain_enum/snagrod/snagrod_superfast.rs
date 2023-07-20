#[macro_use(defer)]
extern crate scopeguard;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::Client;
use scraper::{Html, Selector};
use url::Url;
use std::collections::HashSet;
use std::io::{self, Write, BufWriter};
use std::fs::File;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore, Mutex};
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use scopeguard::defer;

const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.9999.99 Safari/537.36";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("Please enter the filename for the output file:");
    let output_file_path = prompt_user_input();

    println!("Please enter the URL of the website to crawl:");
    let starting_url = prompt_user_input();

    let max_concurrency: usize = 5; // Change this to your desired level of concurrency

    let file = File::create(&output_file_path)?;
    let writer = Arc::new(Mutex::new(BufWriter::new(file)));
    let crawled_urls = Arc::new(Mutex::new(HashSet::new()));
    let urls_to_crawl = Arc::new(Mutex::new(HashSet::new()));
    urls_to_crawl.lock().await.insert(starting_url);
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let active_tasks = Arc::new(AtomicUsize::new(0));

    let (tx, mut rx) = mpsc::channel::<String>(100);

    let writer_handle = tokio::spawn({
        let writer = Arc::clone(&writer);
        async move {
            while let Some(url) = rx.recv().await {
                let mut writer_guard = writer.lock().await;
                writeln!(*writer_guard, "{}", url).unwrap();
                println!("Crawled URL: {}", url);
            }
        }
    });

    loop {
        let url_option = urls_to_crawl.lock().await.iter().next().cloned();
        match url_option {
            Some(url) => {
                urls_to_crawl.lock().await.remove(&url);
                let mut crawled_urls_guard = crawled_urls.lock().await;
                if crawled_urls_guard.contains(&url) {
                    continue;
                }

                let permit = semaphore.clone().acquire_owned().await.unwrap();
                crawled_urls_guard.insert(url.clone());
                active_tasks.fetch_add(1, Ordering::Relaxed);

                let active_tasks = Arc::clone(&active_tasks);
                let urls_to_crawl = Arc::clone(&urls_to_crawl);
                let tx = tx.clone();

                tokio::spawn(async move {
                    defer! {
                        active_tasks.fetch_sub(1, Ordering::Relaxed);
                    }

                    permit.forget();
                    if let Some(links) = crawl(&url).await {
                        for link in links {
                            tx.send(link.clone()).await.unwrap();
                            urls_to_crawl.lock().await.insert(link);
                        }
                    }
                });
            }
            None => {
                if active_tasks.load(Ordering::Relaxed) == 0 {
                    break;
                }
                // Sleep before checking again to prevent busy waiting
                tokio::time::sleep(tokio::time::Duration::from_millis(12000)).await;
            }
        }
    }
    drop(tx); // close the channel when crawling is finished
    writer_handle.await.unwrap();

    Ok(())
}

async fn crawl(url: &str) -> Option<Vec<String>> {
    println!("Crawling: {}", url);

    match get_links(&url).await {
        Ok(links) => Some(links),
        Err(err) => {
            println!("Failed to crawl {}: {}", url, err);
            None
        }
    }
}

fn get_hostname(url: &str) -> Option<String> {
    match Url::parse(url) {
        Ok(parsed_url) => parsed_url.host_str().map(|s| s.to_owned()),
        Err(_) => {
            println!("Failed to parse URL: {}", url);
            None
        }
    }
}

fn resolve_absolute_url(base_url: &str, relative_url: &str) -> String {
    if let Ok(base) = Url::parse(base_url) {
        if let Ok(absolute) = base.join(relative_url) {
            return absolute.to_string();
        }
    }
    relative_url.to_string()
}

async fn get_links(url: &str) -> Result<Vec<String>, reqwest::Error> {
    let client = Client::builder()
        .default_headers(default_headers())
        .timeout(Duration::from_secs(5))  // 10 seconds timeout
        .build()?;

    let response = client.get(url).send().await?;
    let content_type = response.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|ct| ct.to_str().ok());

    if let Some(content_type) = content_type {
        if content_type.starts_with("text/html") {
            let body = response.text().await?;
            let document = Html::parse_document(&body);
            let selector = Selector::parse("a").unwrap();

            let links: Vec<String> = document
                .select(&selector)
                .filter_map(|n| n.value().attr("href"))
                .map(|link| resolve_absolute_url(url, link))
                .collect();

            Ok(links)
        } else {
            println!("Skipping non-HTML content: {}", content_type);
            Ok(vec![])
        }
    } else {
        println!("No Content-Type header found");
        Ok(vec![])
    }
}

fn prompt_user_input() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STRING));
    headers
}

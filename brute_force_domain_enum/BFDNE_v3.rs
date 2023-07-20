use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::Client;
use scraper::{Html, Selector};
use url::Url;
use std::collections::HashSet;
use std::io::{self, Write, BufWriter};
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal::ctrl_c;

const MAX_HOSTNAMES_PER_DOMAIN: usize = 110;
const MAX_LINKS_PER_HOSTNAME: usize = 50;
const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.9999.99 Safari/537.36";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("Welcome to the dcrawl program!");
    println!("You can stop the process anytime by pressing Ctrl-X.");

    println!("Please enter the filename to save the crawled websites:");
    let filename = prompt_user_input();

    // Open the file for writing
    let file = File::create(&filename)?;
    let mut writer = BufWriter::new(file);

    println!("Please enter the URL to begin the crawl process: ");
    let starting_url = prompt_user_input();
    let mut url_queue = vec![starting_url.clone()];
    let mut crawled_urls = HashSet::new();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    tokio::spawn(async move {
        ctrl_c().await.expect("failed to listen for Ctrl+C");
        r.store(false, Ordering::SeqCst);
    });

    println!("Starting dcrawl...");

    while let Some(url) = url_queue.pop() {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        if crawled_urls.contains(&url) {
            continue;
        }

        println!("Crawling: {}", url);
        crawled_urls.insert(url.clone());

        writeln!(writer, "{}", url)?; // write to file

        if let Some(hostname) = get_hostname(&url) {
            let links = match get_links(&url).await {
                Ok(links) => links,
                Err(err) => {
                    println!("Failed to crawl {}: {}", url, err);
                    continue;
                }
            };

            let mut hostname_count = 0;
            let mut link_count = 0;

            for link in links {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                if let Some(link_hostname) = get_hostname(&link) {
                    if hostname_count >= MAX_HOSTNAMES_PER_DOMAIN || link_count >= MAX_LINKS_PER_HOSTNAME {
                        break;
                    }

                    if link_hostname == hostname {
                        let absolute_url = resolve_absolute_url(&url, &link);
                        if !crawled_urls.contains(&absolute_url) {
                            url_queue.push(absolute_url.clone());
                            link_count += 1;
                        }
                    } else {
                        hostname_count += 1;
                    }
                }
            }
        }
    }

    println!("dcrawl process completed. You stopped the process.");

    Ok(())
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

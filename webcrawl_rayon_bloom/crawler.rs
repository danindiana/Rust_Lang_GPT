use rayon::prelude::*;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};
use std::time::Duration;
use std::fs::OpenOptions;
use url::Url;
use log::{error, info};
use structopt::StructOpt;
use bloom::{BloomFilter, ASMS};

const MAX_PAGES_PER_DOMAIN: usize = 568210;
const RETRY_DELAY: Duration = Duration::from_secs(2);

#[derive(StructOpt, Debug)]
pub struct Config {
    #[structopt(long)]
    pub domain: String,

    #[structopt(long)]
    pub output: String,

    #[structopt(long)]
    pub timeout: u64,

    #[structopt(long)]
    pub retries: usize,

    #[structopt(long)]
    pub threads: usize,

    #[structopt(long)]
    pub error_threshold: usize,

    #[structopt(long)]
    pub user_agent: String,
}

pub fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let error_count = Arc::new(AtomicUsize::new(0));

    let starting_url = config.domain.clone();
    let crawled_urls = Arc::new(Mutex::new(BloomFilter::with_rate(0.01, 100_000_000)));
    let num_pages_crawled = Arc::new(AtomicUsize::new(0));
    let urls_to_crawl = Arc::new(Mutex::new(HashMap::new()));

    urls_to_crawl.lock().unwrap().entry(config.domain.clone()).or_insert_with(VecDeque::new).push_back(starting_url.clone());

    let client = Client::builder()
        .user_agent(&config.user_agent)
        .timeout(Duration::from_secs(config.timeout))
        .build()?;

    rayon::ThreadPoolBuilder::new().num_threads(config.threads).build_global()?;

    loop {
        let urls: Vec<(String, String)> = {
            let mut urls_to_crawl = urls_to_crawl.lock().unwrap();
            urls_to_crawl.iter_mut()
                .flat_map(|(domain, urls)| urls.drain(..).map(move |url| (domain.clone(), url)))
                .collect()
        };

        if urls.is_empty() {
            break;
        }

        urls.par_iter().for_each(|(domain, url)| {
            let mut crawled_urls = crawled_urls.lock().unwrap();
            if crawled_urls.contains(url) {
                return;
            }

            for attempt in 1..=config.retries {
                match client.get(url).send() {
                    Ok(res) => {
                        if res.status().is_success() {
                            let content = match res.text() {
                                Ok(text) => text,
                                Err(err) => {
                                    error!("Error reading response text for URL {}: {}", url, err);
                                    return;
                                }
                            };

                            let mut new_urls = Vec::new();
                            let fragment = Html::parse_document(&content);
                            let selector = Selector::parse("a").unwrap();

                            for element in fragment.select(&selector) {
                                if let Some(new_url) = element.value().attr("href") {
                                    let resolved_url = if new_url.starts_with("http") || new_url.starts_with("https") {
                                        new_url.to_string()
                                    } else {
                                        match Url::parse(url) {
                                            Ok(base) => match base.join(new_url) {
                                                Ok(resolved) => resolved.to_string(),
                                                Err(err) => {
                                                    error!("Error resolving URL {}: {}", new_url, err);
                                                    continue;
                                                }
                                            },
                                            Err(err) => {
                                                error!("Error parsing base URL {}: {}", url, err);
                                                continue;
                                            }
                                        }
                                    };
                                    new_urls.push(resolved_url);
                                }
                            }

                            {
                                let mut urls_to_crawl = urls_to_crawl.lock().unwrap();
                                for new_url in new_urls {
                                    urls_to_crawl.entry(domain.clone()).or_default().push_back(new_url);
                                }
                            }

                            {
                                crawled_urls.insert(&url);
                            }

                            // Ensure the file is created and written to
                            if let Err(err) = (|| -> Result<(), std::io::Error> {
                                let mut file = OpenOptions::new()
                                    .write(true)
                                    .create(true)
                                    .append(true)
                                    .open(&config.output)?;
                                writeln!(file, "{}", url)
                            })() {
                                error!("Failed to write URL to file {}: {}", config.output, err);
                            }

                            // Output the crawled URL to the console
                            println!("Crawled URL: {}", url);
                            info!("Crawled URL: {}", url);

                            num_pages_crawled.fetch_add(1, Ordering::Relaxed);
                            break; // Exit retry loop on success
                        }
                    }
                    Err(err) => {
                        error!("Request error for URL {}: {} (Attempt {}/{})", url, err, attempt, config.retries);
                        if attempt == config.retries {
                            error!("Giving up on URL {} after {} attempts", url, config.retries);
                        } else {
                            std::thread::sleep(RETRY_DELAY);
                        }
                    }
                }
            }
        });

        if num_pages_crawled.load(Ordering::Relaxed) >= MAX_PAGES_PER_DOMAIN {
            break;
        }

        let curr_error_count = error_count.load(Ordering::Relaxed);
        if curr_error_count >= config.error_threshold {
            break;
        }
    }

    Ok(())
}

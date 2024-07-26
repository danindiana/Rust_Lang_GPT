use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, atomic::AtomicUsize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::task;
use std::io::{self, Write};
use tokio::time::{self, Duration};
use url::Url;

const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.9999.99 Safari/537.36";
const MAX_PAGES_PER_DOMAIN: usize = 568210;
const MIN_THREADS: usize = 30;
const MAX_THREADS: usize = 60;
const ERROR_THRESHOLD: usize = 20;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let active_threads = Arc::new(AtomicUsize::new(MIN_THREADS));
    let error_count = Arc::new(AtomicUsize::new(0));

    print!("Please enter the domain to begin crawling: ");
    io::stdout().flush()?; 
    let mut domain = String::new();
    io::stdin().read_line(&mut domain)?;
    domain = domain.trim().to_string();

    print!("Please enter the file name to write the crawled results: ");
    io::stdout().flush()?; 
    let mut output_file = String::new();
    io::stdin().read_line(&mut output_file)?;
    output_file = output_file.trim().to_string();

    print!("Please enter the domains to be excluded (comma-separated): ");
    io::stdout().flush()?; 
    let mut excluded_domains = String::new();
    io::stdin().read_line(&mut excluded_domains)?;
    let excluded_domains: HashSet<_> = excluded_domains.trim().split(",").map(|s| s.to_string()).collect();

    let starting_url = format!("https://{}", domain);
    let crawled_urls = Arc::new(Mutex::new(HashSet::new()));
    let num_pages_crawled = Arc::new(AtomicUsize::new(0));
    let urls_to_crawl = Arc::new(Mutex::new(HashMap::new()));

    urls_to_crawl.lock().unwrap().entry(domain.clone()).or_insert_with(VecDeque::new).push_back(starting_url.clone());

    while num_pages_crawled.load(std::sync::atomic::Ordering::Relaxed) < MAX_PAGES_PER_DOMAIN {
        let num_threads = active_threads.load(std::sync::atomic::Ordering::Relaxed);
        let handles = (0..num_threads).map(|_| {
            let crawled_urls = Arc::clone(&crawled_urls);
            let urls_to_crawl = Arc::clone(&urls_to_crawl);
            let num_pages_crawled = Arc::clone(&num_pages_crawled);
            let output_file = output_file.clone();
            let active_threads = Arc::clone(&active_threads);
            let error_count = Arc::clone(&error_count);
            let excluded_domains = excluded_domains.clone();

            task::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent(USER_AGENT_STRING)
                    .build()
                    .unwrap();

                while num_pages_crawled.load(std::sync::atomic::Ordering::Relaxed) < MAX_PAGES_PER_DOMAIN {
                    let mut domain_to_crawl = None;
                    {
                        let mut urls_to_crawl = urls_to_crawl.lock().unwrap();
                        for (domain, urls) in urls_to_crawl.iter_mut() {
                            if let Some(url) = urls.pop_front() {
                                domain_to_crawl = Some((domain.clone(), url));
                                break;
                            }
                        }
                    }

                    if let Some((domain, url)) = domain_to_crawl {
                        if crawled_urls.lock().unwrap().contains(&url) || excluded_domains.contains(&domain) {
                            continue;
                        }

                        let response = time::timeout(REQUEST_TIMEOUT, client.get(&url).send()).await;

                        match response {
                            Ok(Ok(res)) => {
                                let content = res.text().await.unwrap();
                                let mut new_urls = Vec::new();
                                {
                                    let fragment = Html::parse_document(&content);
                                    let selector = Selector::parse("a").unwrap();

                                    for element in fragment.select(&selector) {
                                        if let Some(new_url) = element.value().attr("href") {
                                            let resolved_url = if new_url.starts_with("http") || new_url.starts_with("https") {
                                                new_url.to_string()
                                            } else {
                                                match Url::parse(&url) {
                                                    Ok(base) => match base.join(new_url) {
                                                        Ok(resolved) => resolved.to_string(),
                                                        Err(err) => {
                                                            eprintln!("Error resolving URL {}: {}", new_url, err);
                                                            continue;
                                                        }
                                                    },
                                                    Err(err) => {
                                                        eprintln!("Error parsing base URL {}: {}", url, err);
                                                        continue;
                                                    }
                                                }
                                            };
                                            new_urls.push(resolved_url);
                                        }
                                    }
                                }

                                for new_url in new_urls {
                                    urls_to_crawl.lock().unwrap().entry(domain.clone()).or_default().push_back(new_url);
                                }

                                crawled_urls.lock().unwrap().insert(url.clone());

                                println!("Crawled URL: {}", &url);

                                let mut file = OpenOptions::new()
                                    .write(true)
                                    .create(true)
                                    .append(true)
                                    .open(&output_file)
                                    .await.unwrap();

                                file.write_all(format!("{}\n", &url).as_bytes()).await.unwrap();

                                num_pages_crawled.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                                if error_count.load(std::sync::atomic::Ordering::Relaxed) > 0 {
                                    error_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                                }
                            },
                            Ok(Err(err)) => {
                                eprintln!("Request error: {}", err); 
                                urls_to_crawl.lock().unwrap().entry(domain).or_default().push_back(url);
                                error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed); 
                            },
                            Err(_) => {
                                eprintln!("Timeout occurred while sending request.");
                                urls_to_crawl.lock().unwrap().entry(domain).or_default().push_back(url);
                                error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed); 
                            }
                        }

                    }
                }

                active_threads.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            })
        })
        .collect::<Vec<_>>();

        for handle in handles {
            handle.await.unwrap();
        }

        let curr_error_count = error_count.load(std::sync::atomic::Ordering::Relaxed);
        let curr_active_threads = active_threads.load(std::sync::atomic::Ordering::Relaxed);

        if curr_error_count >= ERROR_THRESHOLD && curr_active_threads > MIN_THREADS {
            active_threads.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        } else if curr_error_count < ERROR_THRESHOLD && curr_active_threads < MAX_THREADS {
            active_threads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    Ok(())
}

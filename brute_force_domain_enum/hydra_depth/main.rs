use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, atomic::AtomicUsize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::task;
use std::io::{self, Write};

const MAX_PAGES_PER_DOMAIN: usize = 1000; // Set your desired max pages per domain here
const MAX_DEPTH: usize = 7; // Set your desired maximum depth here
const MIN_THREADS: usize = 7;
const MAX_THREADS: usize = 50;
const ERROR_THRESHOLD: usize = 10;

fn should_exclude_domain(url: &str) -> bool {
    // Add the domains you want to exclude to this array.
    const EXCLUDED_DOMAINS: [&str; 8] = ["facebook", "youtube", "linkedin", "instagram", "flickr", "bloomberg", "pintrest", "amazonaws"];
    EXCLUDED_DOMAINS.iter().any(|&excluded| url.contains(excluded))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize atomic counters
    let active_threads = Arc::new(AtomicUsize::new(MIN_THREADS));
    let error_count = Arc::new(AtomicUsize::new(0));

    // Prompt the user for the domain
    print!("Please enter the domain to begin crawling: ");
    io::stdout().flush()?;
    let mut domain = String::new();
    io::stdin().read_line(&mut domain)?;
    domain = domain.trim().to_string();

    // Prompt the user for the output file name
    print!("Please enter the file name to write the crawled results: ");
    io::stdout().flush()?;
    let mut output_file = String::new();
    io::stdin().read_line(&mut output_file)?;
    output_file = output_file.trim().to_string();

    let starting_url = domain.clone();
    let crawled_urls = Arc::new(Mutex::new(HashSet::new()));
    let num_pages_crawled = Arc::new(AtomicUsize::new(0));
    let urls_to_crawl = Arc::new(Mutex::new(HashMap::new()));

    urls_to_crawl.lock().unwrap().entry(domain.clone()).or_insert_with(VecDeque::new).push_back((starting_url.clone(), 0));

    // Main crawling loop
    while num_pages_crawled.load(std::sync::atomic::Ordering::Relaxed) < MAX_PAGES_PER_DOMAIN {
        let num_threads = active_threads.load(std::sync::atomic::Ordering::Relaxed);

        let handles = (0..num_threads).map(|_| {
            let crawled_urls = Arc::clone(&crawled_urls);
            let urls_to_crawl = Arc::clone(&urls_to_crawl);
            let num_pages_crawled = Arc::clone(&num_pages_crawled);
            let output_file = output_file.clone();
            let active_threads = Arc::clone(&active_threads);
            let error_count = Arc::clone(&error_count);

            task::spawn(async move {
                let client = reqwest::Client::builder()
                    .build()
                    .unwrap();

                while num_pages_crawled.load(std::sync::atomic::Ordering::Relaxed) < MAX_PAGES_PER_DOMAIN {
                    let mut domain_to_crawl = None;

                    {
                        let mut urls_to_crawl = urls_to_crawl.lock().unwrap();
                        for (domain, urls) in urls_to_crawl.iter_mut() {
                            while let Some((url, current_depth)) = urls.pop_front() {
                                if crawled_urls.lock().unwrap().insert(url.clone()) {
                                    if current_depth < MAX_DEPTH && !should_exclude_domain(&url) {
                                        domain_to_crawl = Some((domain.clone(), url.clone(), current_depth));
                                    }
                                    break;
                                }
                            }

                            if domain_to_crawl.is_some() {
                                break;
                            }
                        }
                    }

                    if let Some((domain, url, depth)) = domain_to_crawl {
                        let response = client.get(&url).send().await;

                        match response {
                            Ok(res) => {
                                let content = res.text().await.unwrap();
                                let mut new_urls = Vec::new();
                                {
                                    let fragment = Html::parse_document(&content);
                                    let selector = Selector::parse("a").unwrap();

                                    for element in fragment.select(&selector) {
                                        if let Some(new_url) = element.value().attr("href") {
                                            if (depth + 1) <= MAX_DEPTH && !should_exclude_domain(new_url) && (new_url.starts_with("http") || new_url.starts_with("https")) {
                                                new_urls.push(new_url.to_string());
                                            }
                                        }
                                    }
                                }

                                for new_url in new_urls {
                                    urls_to_crawl.lock().unwrap().entry(domain.clone()).or_default().push_back((new_url, depth + 1));
                                }

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
                            }
                            Err(_) => {
                                urls_to_crawl.lock().unwrap().entry(domain).or_default().push_back((url, depth));
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

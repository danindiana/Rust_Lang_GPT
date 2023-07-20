use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, atomic::AtomicUsize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::task;
use std::io::{self, Write}; // For flushing the output before reading input

const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.9999.99 Safari/537.36";
const MAX_PAGES_PER_DOMAIN: usize = 568210;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let num_threads = 10;

    // Prompt the user for the domain
    print!("Please enter the domain to begin crawling: ");
    io::stdout().flush()?; // flush it to the stdout immediately
    let mut domain = String::new();
    io::stdin().read_line(&mut domain)?;
    domain = domain.trim().to_string();

    // Prompt the user for the output file name
    print!("Please enter the file name to write the crawled results: ");
    io::stdout().flush()?; // flush it to the stdout immediately
    let mut output_file = String::new();
    io::stdin().read_line(&mut output_file)?;
    output_file = output_file.trim().to_string();

    let starting_url = domain.clone();
    let crawled_urls = Arc::new(Mutex::new(HashSet::new()));
    let num_pages_crawled = Arc::new(AtomicUsize::new(0));
    let urls_to_crawl = Arc::new(Mutex::new(HashMap::new()));

    urls_to_crawl.lock().unwrap().entry(domain.clone()).or_insert_with(VecDeque::new).push_back(starting_url.clone());

    let handles = (0..num_threads).map(|_| {
        let crawled_urls = Arc::clone(&crawled_urls);
        let urls_to_crawl = Arc::clone(&urls_to_crawl);
        let num_pages_crawled = Arc::clone(&num_pages_crawled);
        let output_file = output_file.clone();

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
                    let response = client.get(&url).send().await;

                    match response {
                        Ok(res) => {
                            let content = res.text().await.unwrap();
                            {
                                let fragment = Html::parse_document(&content);
                                let selector = Selector::parse("a").unwrap();
    
                                for element in fragment.select(&selector) {
                                    let new_url = element.value().attr("href").unwrap_or("");
                                    if new_url.starts_with("http") || new_url.starts_with("https") {
                                        urls_to_crawl.lock().unwrap().entry(domain.clone()).or_default().push_back(new_url.to_string());
                                    }
                                }
                            }

                            crawled_urls.lock().unwrap().insert(url.clone());

                            println!("Crawled URL: {}", &url);

                            // Open the file in append mode and write the URL
                            let mut file = OpenOptions::new()
                                .write(true)
                                .create(true)
                                .append(true)
                                .open(&output_file)
                                .await.unwrap();
                            
                            file.write_all(format!("{}\n", &url).as_bytes()).await.unwrap();

                            num_pages_crawled.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                        Err(_) => {
                            urls_to_crawl.lock().unwrap().entry(domain).or_default().push_back(url);
                        }
                    }
                }
            }
        })
    })
    .collect::<Vec<_>>();

    for handle in handles {
        handle.await.unwrap();
    }

    Ok(())
}

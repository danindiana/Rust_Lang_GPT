### Prompt for Code Completion Language Model

**Objective:**
Generate a web crawler in Rust using the Tokio asynchronous runtime, adhering to best practices in Rust programming, concurrency, and error handling. The crawler should be efficient, correct, and scalable. It should dynamically adjust the number of active threads based on the error rate and ensure thread safety using appropriate synchronization primitives.

**Requirements:**
1. **Asynchronous Execution:** Utilize the Tokio runtime for asynchronous I/O operations.
2. **Thread Management:** Dynamically adjust the number of active threads based on the error rate.
3. **Error Handling:** Implement robust error handling for HTTP requests and file I/O operations.
4. **Concurrency Safety:** Use `Arc` and `Mutex` for shared state management to ensure thread safety.
5. **Efficient Parsing:** Use the `scraper` crate for HTML parsing to extract URLs from web pages.
6. **User Input:** Prompt the user for the domain to crawl, output file name, and excluded domains.
7. **Output:** Write crawled URLs to a specified file in append mode.
8. **Best Practices:** Follow Rust's best practices for code organization, error handling, and performance optimization.

**Prompt:**

```markdown
### Web Crawler in Rust

**Objective:**
Create a web crawler in Rust using the Tokio asynchronous runtime. The crawler should be efficient, correct, and scalable, adhering to Rust's best practices for concurrency, error handling, and performance optimization.

**Requirements:**
1. **Asynchronous Execution:**
   - Use the Tokio runtime for asynchronous I/O operations.
   - Implement asynchronous HTTP requests using the `reqwest` crate.
   - Use `tokio::time::timeout` for request timeouts.

2. **Thread Management:**
   - Dynamically adjust the number of active threads based on the error rate.
   - Use `Arc<AtomicUsize>` to manage the number of active threads and error count.

3. **Error Handling:**
   - Implement robust error handling for HTTP requests and file I/O operations.
   - Use `Result` and `Option` types appropriately.
   - Log errors and retry failed requests.

4. **Concurrency Safety:**
   - Use `Arc<Mutex<T>>` for shared state management to ensure thread safety.
   - Ensure that shared resources like `crawled_urls` and `urls_to_crawl` are accessed in a thread-safe manner.

5. **Efficient Parsing:**
   - Use the `scraper` crate for HTML parsing to extract URLs from web pages.
   - Extract URLs from `<a>` tags and filter out already crawled URLs.

6. **User Input:**
   - Prompt the user for the domain to crawl, output file name, and excluded domains.
   - Read user input from standard input and validate it.

7. **Output:**
   - Write crawled URLs to a specified file in append mode.
   - Use `tokio::fs::OpenOptions` for file I/O operations.

8. **Best Practices:**
   - Follow Rust's best practices for code organization, error handling, and performance optimization.
   - Use `cargo clippy` and `cargo fmt` to ensure code quality.

**Code Structure:**
1. **Imports and Constants:**
   - Import necessary crates and define constants for user agent string, thread limits, error threshold, and request timeout.

2. **Main Function:**
   - Initialize shared resources using `Arc` and `Mutex`.
   - Prompt the user for input and validate it.
   - Set up the initial state for crawling.

3. **Crawling Loop:**
   - Implement a loop that continues until the crawling limit is reached.
   - Spawn multiple tasks for concurrent crawling.

4. **Task Execution:**
   - Each task should:
     - Clone necessary shared resources.
     - Create a `reqwest::Client` with a user agent string.
     - Attempt to get a URL to crawl from the `urls_to_crawl` map.
     - Send an HTTP GET request with a timeout.
     - Parse the response content to extract new URLs.
     - Write the crawled URL to the output file.
     - Handle errors and retry failed requests.

5. **Thread Management:**
   - Adjust the number of active threads based on the error rate.
   - Ensure that the number of threads stays within the defined limits.

6. **Completion:**
   - Exit the program gracefully after the crawling limit is reached.

**Example Code Structure:**
```rust
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, atomic::AtomicUsize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::task;
use std::io::{self, Write};
use tokio::time::{self, Duration};

const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.9999.99 Safari/537.36";
const MAX_PAGES_PER_DOMAIN: usize = 568210;
const MIN_THREADS: usize = 30;
const MAX_THREADS: usize = 60;
const ERROR_THRESHOLD: usize = 20;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize shared resources
    let active_threads = Arc::new(AtomicUsize::new(MIN_THREADS));
    let error_count = Arc::new(AtomicUsize::new(0));

    // Prompt user for input
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

    // Initialize crawling state
    let starting_url = domain.clone();
    let crawled_urls = Arc::new(Mutex::new(HashSet::new()));
    let num_pages_crawled = Arc::new(AtomicUsize::new(0));
    let urls_to_crawl = Arc::new(Mutex::new(HashMap::new()));

    urls_to_crawl.lock().unwrap().entry(domain.clone()).or_insert_with(VecDeque::new).push_back(starting_url.clone());

    // Main crawling loop
    while num_pages_crawled.load(std::sync::atomic::Ordering::Relaxed) < MAX_PAGES_PER_DOMAIN {
        let num_threads = active_threads.load(std::sync::atomic::Ordering::Relaxed);
        let handles = (0..num_threads).map(|_| {
            // Clone shared resources for each task
            let crawled_urls = Arc::clone(&crawled_urls);
            let urls_to_crawl = Arc::clone(&urls_to_crawl);
            let num_pages_crawled = Arc::clone(&num_pages_crawled);
            let output_file = output_file.clone();
            let active_threads = Arc::clone(&active_threads);
            let error_count = Arc::clone(&error_count);
            let excluded_domains = excluded_domains.clone();

            task::spawn(async move {
                // Create HTTP client
                let client = reqwest::Client::builder()
                    .user_agent(USER_AGENT_STRING)
                    .build()
                    .unwrap();

                // Crawling loop for each task
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

                        // Send HTTP request with timeout
                        let response = time::timeout(
                            REQUEST_TIMEOUT,
                            client.get(&url).send(),
                        ).await;

                        match response {
                            Ok(Ok(res)) => {
                                let content = res.text().await.unwrap();
                                let mut new_urls = Vec::new();
                                {
                                    let fragment = Html::parse_document(&content);
                                    let selector = Selector::parse("a").unwrap();

                                    for element in fragment.select(&selector) {
                                        let new_url = element.value().attr("href").unwrap_or("");
                                        if (new_url.starts_with("http") || new_url.starts_with("https")) && !crawled_urls.lock().unwrap().contains(new_url) {
                                            new_urls.push(new_url.to_string());
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
                            Ok(Err(_)) | Err(_) => {
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
```

**Completion Instructions:**
1. **Error Handling:** Ensure all potential errors are handled gracefully, including HTTP request failures, file I/O errors, and user input validation.
2. **Concurrency Safety:** Verify that all shared resources are accessed in a thread-safe manner using `Arc` and `Mutex`.
3. **Performance Optimization:** Optimize the code for performance, including minimizing lock contention and reducing unnecessary cloning.
4. **Code Quality:** Run `cargo clippy` and `cargo fmt` to ensure the code adheres to Rust's best practices.

**Expected Output:**
A fully functional web crawler in Rust that adheres to the specified requirements and best practices.

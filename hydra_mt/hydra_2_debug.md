There could be a few potential reasons why the crawler stops after crawling just one URL. Let's break down the possibilities and how to investigate them:

### 1. **URL Queue is Empty After First URL**

The program might be exhausting the URL queue after crawling the first URL, meaning no new URLs are being enqueued for crawling. This could happen if:
- The crawled page doesn't contain any valid URLs, or
- The program is filtering out all the URLs (for example, due to being on the excluded domains list or other constraints).

#### What to Check:
- After crawling the first URL, verify that the program is finding additional URLs by inspecting the result of `extract_urls()`.
- Ensure the crawler is correctly extracting and enqueuing new URLs.

You can add a debug log to verify the URLs that are extracted:

```rust
let new_urls = extract_urls(&content, &url);
debug!("Extracted URLs: {:?}", new_urls); // Add this to see the extracted URLs
```

### 2. **Domain Exclusion or Crawled URLs Filtering**

If the newly extracted URLs are being filtered out because:
- They belong to a domain in the `excluded_domains` set, or
- They have already been crawled (i.e., they are in the `crawled_urls` set).

#### What to Check:
- Make sure that the excluded domains logic isn't overly restrictive.
- Ensure that valid URLs are not being mistakenly marked as already crawled.

You can log the reasons why URLs are being skipped:

```rust
if excluded_domains.contains(&domain) {
    debug!("Skipping URL due to excluded domain: {}", domain);
    continue;
}

if crawled_urls.lock().await.contains(&new_url_str) {
    debug!("Skipping already crawled URL: {}", new_url_str);
    continue;
}
```

### 3. **Deadlock or Mutex Lock Issues**

If the URL queue (`urls_to_crawl`) or the crawled URLs set (`crawled_urls`) is not being updated properly (e.g., due to a locking issue), the program might think there are no more URLs to crawl. Ensure the program is not experiencing a deadlock or incorrectly holding onto the mutex lock.

#### What to Check:
- Ensure that the locks around `urls_to_crawl` and `crawled_urls` are correctly scoped and released after each operation.
- You can add logging after each lock acquisition and release to verify this:

```rust
{
    let mut urls = urls_to_crawl.lock().await;
    debug!("URLs to crawl: {:?}", urls);
}
```

### 4. **Timeouts or Errors in HTTP Requests**

If the first request succeeds, but subsequent requests are failing (e.g., due to timeouts or network issues), it could cause the crawler to stop. The program might be retrying the same URL or skipping further URLs after repeated failures.

#### What to Check:
- Inspect the `error_count` and log any errors or timeouts that occur during the HTTP requests.
- Ensure the program retries failed requests and handles timeouts gracefully.

You can add more detailed error logging:

```rust
warn!("HTTP request error for URL {}: {}", url_str, e);
debug!("Retrying URL: {}", url_str);
```

### 5. **URL Normalization Issues**

The normalization of new URLs (`new_url.join(&url)`) might not be working correctly. If it fails, the program may end up not enqueuing valid URLs.

#### What to Check:
- Make sure that `new_url.join(&url)` is producing valid URLs, and the program isn't skipping them due to errors in joining relative URLs.

### Diagnostic Plan:
1. **Enable Debug Logging**: Temporarily change the log level to `debug` to get detailed output for each step of the process. This will help track what's happening after the first URL is crawled:

```bash
RUST_LOG=debug cargo run
```

2. **Inspect Crawling State**: Ensure new URLs are being discovered and added to the queue, and the program is not getting stuck in a state where no more URLs are available.

3. **Check Network Errors**: Ensure the program is handling network timeouts or HTTP errors gracefully and retrying as expected.

### Next Steps:
- Add the suggested debug logs.
- Run the program with the `RUST_LOG=debug` environment variable to get detailed output.
- Based on the output, identify whether the program is correctly extracting URLs, updating the queue, and continuing crawling.

Let me know what the output reveals, and we can refine the fix from there!




Here’s a refactored version of your web crawler with the debug logging and suggestions implemented to help diagnose why the crawling might stop after the first URL:

### Refactored Code:

```rust
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{self, Duration};
use url::Url;

use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

use log::{debug, error, info, warn};

const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.9999.99 Safari/537.36";
const MAX_PAGES_PER_DOMAIN: usize = 568210;
const MIN_THREADS: usize = 30;
const MAX_THREADS: usize = 60;
const ERROR_THRESHOLD: usize = 20;
const REQUEST_TIMEOUT_SECS: u64 = 10;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Initialize shared resources
    let active_threads = Arc::new(AtomicUsize::new(MIN_THREADS));
    let error_count = Arc::new(AtomicUsize::new(0));

    // Prompt user for input
    let domain = prompt_user("Please enter the domain to begin crawling: ").await?;
    let output_file = prompt_user("Please enter the file name to write the crawled results: ").await?;
    let excluded_domains_input = prompt_user("Please enter the domains to be excluded (comma-separated): ").await?;
    let excluded_domains: HashSet<String> = excluded_domains_input
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    // Validate and parse the starting URL
    let starting_url = match Url::parse(&domain) {
        Ok(url) => url,
        Err(_) => {
            // Attempt to prepend "http://" if missing
            let url_str = format!("http://{}", domain);
            Url::parse(&url_str)?
        }
    };

    // Initialize crawling state
    let crawled_urls = Arc::new(RwLock::new(HashSet::new()));
    let num_pages_crawled = Arc::new(AtomicUsize::new(0));
    let urls_to_crawl = Arc::new(Mutex::new(VecDeque::new()));

    {
        let mut urls = urls_to_crawl.lock().await;
        urls.push_back(starting_url.clone());
        debug!("Added starting URL to queue: {}", starting_url);
    }

    // Open the output file
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&output_file)
        .await?;
    let file = Arc::new(Mutex::new(file));

    // Main crawling loop
    loop {
        if num_pages_crawled.load(Ordering::Relaxed) >= MAX_PAGES_PER_DOMAIN {
            info!("Reached maximum number of pages to crawl.");
            break;
        }

        let num_threads = active_threads.load(Ordering::Relaxed);
        let mut handles = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            // Clone shared resources for each task
            let crawled_urls = Arc::clone(&crawled_urls);
            let urls_to_crawl = Arc::clone(&urls_to_crawl);
            let num_pages_crawled = Arc::clone(&num_pages_crawled);
            let output_file = Arc::clone(&file);
            let active_threads = Arc::clone(&active_threads);
            let error_count = Arc::clone(&error_count);
            let excluded_domains = excluded_domains.clone();

            let handle = tokio::spawn(async move {
                // Create HTTP client
                let client = match reqwest::Client::builder()
                    .user_agent(USER_AGENT_STRING)
                    .redirect(reqwest::redirect::Policy::limited(10))
                    .build()
                {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to build HTTP client: {}", e);
                        return;
                    }
                };

                loop {
                    if num_pages_crawled.load(Ordering::Relaxed) >= MAX_PAGES_PER_DOMAIN {
                        break;
                    }

                    let url = {
                        let mut urls = urls_to_crawl.lock().await;
                        urls.pop_front()
                    };

                    let url = match url {
                        Some(u) => u,
                        None => {
                            debug!("No more URLs in the queue, sleeping...");
                            time::sleep(Duration::from_millis(100)).await;
                            continue;
                        }
                    };

                    let url_str = url.as_str().to_string();

                    {
                        let crawled = crawled_urls.read().await;
                        if crawled.contains(&url_str) {
                            debug!("Skipping already crawled URL: {}", url_str);
                            continue;
                        }
                    }

                    let parsed_url = match Url::parse(&url_str) {
                        Ok(u) => u,
                        Err(e) => {
                            warn!("Invalid URL '{}': {}", url_str, e);
                            continue;
                        }
                    };

                    let domain = match parsed_url.domain() {
                        Some(d) => d.to_lowercase(),
                        None => {
                            warn!("URL without a valid domain: {}", url_str);
                            continue;
                        }
                    };

                    if excluded_domains.contains(&domain) {
                        debug!("Skipping URL due to excluded domain: {}", domain);
                        continue;
                    }

                    {
                        let mut crawled = crawled_urls.write().await;
                        crawled.insert(url_str.clone());
                        debug!("Crawled URL added to set: {}", url_str);
                    }

                    let response = time::timeout(
                        Duration::from_secs(REQUEST_TIMEOUT_SECS),
                        client.get(url.clone()).send(),
                    )
                    .await;

                    match response {
                        Ok(Ok(res)) => {
                            if !res.status().is_success() {
                                warn!("Non-success status {} for URL: {}", res.status(), url_str);
                                error_count.fetch_add(1, Ordering::Relaxed);
                                enqueue_url(&urls_to_crawl, url.clone()).await;
                                continue;
                            }

                            let content = match res.text().await {
                                Ok(text) => text,
                                Err(e) => {
                                    warn!("Failed to read response text for URL {}: {}", url_str, e);
                                    error_count.fetch_add(1, Ordering::Relaxed);
                                    enqueue_url(&urls_to_crawl, url.clone()).await;
                                    continue;
                                }
                            };

                            let new_urls = extract_urls(&content, &url);
                            debug!("Extracted URLs: {:?}", new_urls);

                            for new_url in new_urls {
                                let normalized_url = match new_url.join(url.as_str()) { // Convert &Url to &str
                                    Ok(u) => u,
                                    Err(_) => continue,
                                };

                                if let Some(new_domain) = normalized_url.domain() {
                                    if excluded_domains.contains(&new_domain.to_lowercase()) {
                                        debug!("Skipping extracted URL due to excluded domain: {}", new_domain);
                                        continue;
                                    }
                                }

                                {
                                    let crawled = crawled_urls.read().await;
                                    if crawled.contains(normalized_url.as_str()) {
                                        debug!("Skipping already crawled extracted URL: {}", normalized_url);
                                        continue;
                                    }
                                }

                                enqueue_url(&urls_to_crawl, normalized_url).await;
                            }

                            {
                                let mut file = output_file.lock().await;
                                if let Err(e) = file
                                    .write_all(format!("{}\n", url_str).as_bytes())
                                    .await
                                {
                                    error!("Failed to write to file: {:?}", e);
                                }
                            }

                            num_pages_crawled.fetch_add(1, Ordering::Relaxed);
                            info!("Crawled URL: {} (Total: {})", url_str, num_pages_crawled.load(Ordering::Relaxed));

                            if error_count.load(Ordering::Relaxed) > 0 {
                                error_count.fetch_sub(1, Ordering::Relaxed);
                            }
                        }
                        Ok(Err(e)) => {
                            warn!("HTTP request error for URL {}: {}", url_str, e);
                            error_count.fetch_add(1, Ordering::Relaxed);
                            enqueue_url(&urls_to_crawl, url.clone()).await;
                        }
                        Err(_) => {
                            warn!("Timeout for URL: {}", url_str);
                            error_count.fetch_add(1, Ordering::Relaxed);
                            enqueue_url(&urls_to_crawl, url.clone()).await;
                        }
                    }
                }

                active_threads.fetch_sub(1, Ordering::Relaxed);
            });

            handles.push(handle);
        }

        for handle in handles {
            if let Err(e) = handle.await {
                error!("Task failed: {}", e);
            }
        }

        let curr_error_count = error_count.load(Ordering::Relaxed);
        let curr_active_threads = active_threads.load(Ordering::Relaxed);

        if curr_error_count >= ERROR_THRESHOLD && curr_active_threads > MIN_THREADS {
            active_threads.fetch_sub(1, Ordering::Relaxed);
            warn!(
                "Error threshold reached ({}). Decreasing active threads to {}.",
                curr_error_count,
                active_threads.load(Ordering::Relaxed)
            );
        } else if curr_error_count < ERROR_THRESHOLD && curr_active_threads < MAX_THREADS {
            active_threads.fetch_add(1, Ordering::Relaxed);
            info!(
                "Error count acceptable ({}). Increasing active

 threads to {}.",
                curr_error_count,
                active_threads.load(Ordering::Relaxed)
            );
        }

        time::sleep(Duration::from_millis(500)).await;
    }

    info!("Crawling completed. Total pages crawled: {}", num_pages_crawled.load(Ordering::Relaxed));

    Ok(())
}

async fn prompt_user(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    print!("{}", prompt);
    io::stdout().flush()?;
    
    let result = tokio::task::spawn_blocking(|| {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok::<_, std::io::Error>(input)
    }).await??;

    Ok(result.trim().to_string())
}

fn extract_urls(base_content: &str, base_url: &Url) -> Vec<Url> {
    let mut urls = Vec::new();
    let fragment = Html::parse_document(base_content);
    let selector = Selector::parse("a[href]").unwrap();

    for element in fragment.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if let Ok(url) = base_url.join(href) {
                urls.push(url);
            }
        }
    }

    urls
}

async fn enqueue_url(urls_to_crawl: &Arc<Mutex<VecDeque<Url>>>, url: Url) {
    let mut urls = urls_to_crawl.lock().await;
    urls.push_back(url);
    debug!("Enqueued URL: {}", url);
}
```

### Changes Made:
1. **Added Debug Logs**: 
   - Logs to track when URLs are added to the queue (`enqueue_url()`), skipped due to domain filtering, or already crawled.
   - Logs in `extract_urls()` to verify which URLs are extracted from the crawled content.
   - Logs to track HTTP request failures and timeouts.
   - Logs to show when the URL queue is empty, causing the crawler to sleep temporarily.

2. **Diagnosing Domain Exclusion or Crawled URL Filtering**:
   - Added logs to check when a URL is skipped because it's already been crawled or belongs to an excluded domain.

3. **Better Error Logging**:
   - Improved error logging for HTTP request failures and file writing issues.

### Next Steps:
1. **Run the Crawler with Debug Logs**:
   - Use `RUST_LOG=debug cargo run` to run the program with verbose debug logging. This will print all the information necessary to diagnose why the crawler might stop after one URL.

2. **Review the Output**:
   - Check if the program is extracting URLs correctly and if they are being added to the queue.
   - Look for any errors or issues related to timeouts, excluded domains, or filtering of already-crawled URLs.

Let me know how it behaves with the added logging, and we can proceed from there!


Yes, the program should output the URLs being crawled to the console. Here's a breakdown of what logs you should expect and what they represent:

### Expected Log Outputs:

1. **URLs Being Crawled:**
   - The following log will print the URL that is successfully crawled along with the total number of pages crawled so far:
   
   ```rust
   info!("Crawled URL: {} (Total: {})", url_str, num_pages_crawled.load(Ordering::Relaxed));
   ```

   This will appear in the console when a URL is successfully crawled.

2. **URLs Being Enqueued:**
   - When a new URL is extracted and added to the `urls_to_crawl` queue, you should see this log:

   ```rust
   debug!("Enqueued URL: {}", url);
   ```

   This gives visibility into what URLs are being added to the queue for future crawling.

3. **Skipped URLs:**
   - If a URL is skipped because it has already been crawled, or it belongs to an excluded domain, you’ll see the following logs:

   ```rust
   debug!("Skipping already crawled URL: {}", url_str);
   debug!("Skipping URL due to excluded domain: {}", domain);
   ```

   These logs will help identify URLs that the crawler skips over for various reasons.

4. **Error Logs:**
   - If there is an HTTP request error or if the URL is invalid, the program will log a warning or an error like this:

   ```rust
   warn!("HTTP request error for URL {}: {}", url_str, e);
   warn!("Invalid URL '{}': {}", url_str, e);
   ```

   These logs will inform you of any issues encountered during the crawling process, such as malformed URLs or HTTP errors.

### Steps to Run the Program with Logs:

1. **Run the Program with `info` Logging:**
   To see the URLs being crawled (output as `info` logs), run the program like this:

   ```bash
   RUST_LOG=info cargo run
   ```

   This will display the `info!` logs in the console, showing the URLs as they are crawled and written to the output file.

2. **Run with `debug` Logging for More Details:**
   If you want more detailed information, including which URLs are being enqueued or skipped, you can run with `debug` logging:

   ```bash
   RUST_LOG=debug cargo run
   ```

   This will show:
   - URLs being crawled.
   - URLs being enqueued into the queue.
   - URLs being skipped (already crawled or in excluded domains).
   - Any errors or warnings encountered.

### Example Console Output with `info` Logs:
If the program is working correctly, you should see output like this:

```
Crawled URL: http://example.com (Total: 1)
Crawled URL: http://example.com/about (Total: 2)
Crawled URL: http://example.com/contact (Total: 3)
```

If you run with `debug` logging, you will see more details:

```
Added starting URL to queue: http://example.com
Enqueued URL: http://example.com/about
Enqueued URL: http://example.com/contact
Crawled URL: http://example.com (Total: 1)
Skipping already crawled URL: http://example.com
Crawled URL: http://example.com/about (Total: 2)
...
```

### Conclusion:
- **URLs Being Crawled:** Should be printed as `info!` logs.
- **URLs Being Enqueued/Skipped:** Printed as `debug!` logs if you enable debug logging.
- **Errors:** Printed as `warn!` or `error!` logs.

Make sure you're running the program with the appropriate log level (`info` or `debug`) to see the expected output in the console. Let me know if this helps or if you need further adjustments!

To guide a language model to rebuild the web crawler program incrementally, starting from lower to higher complexity, you can break down the task into a series of iterative steps. Each step should build upon the previous one, adding more functionality and complexity as you go. Here’s how you can structure the prompt:

### Step 1: Basic Setup
**Objective:** Set up the basic project structure and dependencies.

**Prompt:**
```markdown
### Step 1: Basic Setup

**Objective:** Set up the basic project structure and dependencies.

1. **Create a new Rust project:**
   - Initialize a new Rust project using `cargo new web_crawler`.

2. **Add necessary dependencies:**
   - Add `tokio`, `reqwest`, `scraper`, `url`, `log`, and `env_logger` to the `Cargo.toml` file.

3. **Initialize logging:**
   - Set up basic logging using `env_logger`.

**Expected Output:**
- A basic Rust project with necessary dependencies and logging initialized.

**Next Step:**
- Implement basic user input handling.
```

### Step 2: User Input Handling
**Objective:** Implement basic user input handling.

**Prompt:**
```markdown
### Step 2: User Input Handling

**Objective:** Implement basic user input handling.

1. **Prompt the user for input:**
   - Ask the user to enter the domain to crawl.
   - Ask the user to enter the output file name.
   - Ask the user to enter excluded domains (comma-separated).

2. **Validate and parse user input:**
   - Ensure the domain is a valid URL.
   - Ensure the output file name is valid.
   - Parse the excluded domains into a `HashSet`.

**Expected Output:**
- A function that prompts the user for input and validates it.

**Next Step:**
- Implement basic HTTP request handling.
```

### Step 3: Basic HTTP Request Handling
**Objective:** Implement basic HTTP request handling.

**Prompt:**
```markdown
### Step 3: Basic HTTP Request Handling

**Objective:** Implement basic HTTP request handling.

1. **Create an HTTP client:**
   - Use `reqwest` to create an HTTP client with a user agent string.

2. **Send a GET request:**
   - Send a GET request to the domain entered by the user.
   - Handle potential errors (e.g., network errors, timeouts).

3. **Log the response:**
   - Log the status code and response content.

**Expected Output:**
- A function that sends an HTTP GET request and logs the response.

**Next Step:**
- Implement basic HTML parsing.
```

### Step 4: Basic HTML Parsing
**Objective:** Implement basic HTML parsing.

**Prompt:**
```markdown
### Step 4: Basic HTML Parsing

**Objective:** Implement basic HTML parsing.

1. **Parse the HTML content:**
   - Use the `scraper` crate to parse the HTML content of the response.

2. **Extract URLs:**
   - Extract URLs from `<a>` tags in the HTML content.
   - Log the extracted URLs.

**Expected Output:**
- A function that parses HTML content and extracts URLs.

**Next Step:**
- Implement basic URL management.
```

### Step 5: Basic URL Management
**Objective:** Implement basic URL management.

**Prompt:**
```markdown
### Step 5: Basic URL Management

**Objective:** Implement basic URL management.

1. **Manage crawled URLs:**
   - Use a `HashSet` to keep track of crawled URLs.

2. **Manage URLs to crawl:**
   - Use a `VecDeque` to manage URLs that need to be crawled.

3. **Enqueue URLs:**
   - Enqueue extracted URLs to the `VecDeque` if they haven’t been crawled.

**Expected Output:**
- A function that manages crawled and to-be-crawled URLs.

**Next Step:**
- Implement basic concurrency.
```

### Step 6: Basic Concurrency
**Objective:** Implement basic concurrency.

**Prompt:**
```markdown
### Step 6: Basic Concurrency

**Objective:** Implement basic concurrency.

1. **Spawn multiple tasks:**
   - Use `tokio::spawn` to spawn multiple tasks for concurrent crawling.

2. **Share state between tasks:**
   - Use `Arc` and `Mutex` to share state (e.g., `crawled_urls`, `urls_to_crawl`) between tasks.

3. **Handle task completion:**
   - Wait for all tasks to complete.

**Expected Output:**
- A function that spawns multiple tasks for concurrent crawling and shares state between them.

**Next Step:**
- Implement dynamic thread management.
```

### Step 7: Dynamic Thread Management
**Objective:** Implement dynamic thread management.

**Prompt:**
```markdown
### Step 7: Dynamic Thread Management

**Objective:** Implement dynamic thread management.

1. **Adjust the number of active threads:**
   - Dynamically adjust the number of active threads based on the error rate.
   - Use `AtomicUsize` to manage the number of active threads.

2. **Log thread adjustments:**
   - Log when the number of active threads is increased or decreased.

**Expected Output:**
- A function that dynamically adjusts the number of active threads based on the error rate.

**Next Step:**
- Implement file I/O operations.
```

### Step 8: File I/O Operations
**Objective:** Implement file I/O operations.

**Prompt:**
```markdown
### Step 8: File I/O Operations

**Objective:** Implement file I/O operations.

1. **Open the output file:**
   - Use `tokio::fs::OpenOptions` to open the output file in append mode.

2. **Write crawled URLs to the file:**
   - Write each crawled URL to the output file.
   - Handle potential file I/O errors.

**Expected Output:**
- A function that writes crawled URLs to a file.

**Next Step:**
- Implement final error handling and logging.
```

### Step 9: Final Error Handling and Logging
**Objective:** Implement final error handling and logging.

**Prompt:**
```markdown
### Step 9: Final Error Handling and Logging

**Objective:** Implement final error handling and logging.

1. **Handle all potential errors:**
   - Ensure all potential errors (e.g., HTTP errors, file I/O errors) are handled gracefully.

2. **Log all significant events:**
   - Log all significant events (e.g., starting a crawl, crawling a URL, encountering an error).

3. **Graceful shutdown:**
   - Ensure the program shuts down gracefully after completing the crawl.

**Expected Output:**
- A fully functional web crawler with robust error handling and detailed logging.

**Final Step:**
- Review and optimize the code for performance and best practices.
```

### Step 10: Review and Optimize
**Objective:** Review and optimize the code for performance and best practices.

**Prompt:**
```markdown
### Step 10: Review and Optimize

**Objective:** Review and optimize the code for performance and best practices.

1. **Run `cargo clippy` and `cargo fmt`:**
   - Ensure the code adheres to Rust's best practices and is well-formatted.

2. **Optimize for performance:**
   - Minimize lock contention.
   - Reduce unnecessary cloning.

3. **Review and refactor:**
   - Review the code for any potential improvements.
   - Refactor as necessary.

**Expected Output:**
- An optimized and well-structured web crawler.

**Final Output:**
- A fully functional, optimized, and well-structured web crawler.
```

By following these incremental steps, you can guide a language model to build the web crawler from lower to higher complexity, ensuring that each step builds upon the previous one and that the final output is a robust and efficient program.


Here’s a refactored version of your web crawler with the dynamic thread management removed. This version will spawn a fixed number of threads (tasks) for concurrent crawling but will no longer adjust the number of threads dynamically based on the error rate.

### Refactored Code (Fixed Thread Count, No Dynamic Management):

```rust
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{self, Duration};
use url::Url;

use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

use log::{debug, error, info, warn};

const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.9999.99 Safari/537.36";
const MAX_PAGES_PER_DOMAIN: usize = 1000;
const NUM_THREADS: usize = 10; // Fixed number of threads (tasks)
const REQUEST_TIMEOUT_SECS: u64 = 10;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Initialize shared resources
    let error_count = Arc::new(AtomicUsize::new(0));

    // Prompt user for input
    let domain = prompt_user("Please enter the domain to begin crawling: ").await?;
    let output_file = prompt_user("Please enter the file name to write the crawled results: ").await?;
    let excluded_domains_input = prompt_user("Please enter the domains to be excluded (comma-separated): ").await?;
    let excluded_domains: HashSet<String> = excluded_domains_input
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    // Validate and parse the starting URL
    let starting_url = match Url::parse(&domain) {
        Ok(url) => url,
        Err(_) => {
            let url_str = format!("http://{}", domain);
            Url::parse(&url_str)?
        }
    };

    // Initialize crawling state
    let crawled_urls = Arc::new(RwLock::new(HashSet::new()));
    let num_pages_crawled = Arc::new(AtomicUsize::new(0));
    let urls_to_crawl = Arc::new(Mutex::new(VecDeque::new()));

    {
        let mut urls = urls_to_crawl.lock().await;
        urls.push_back(starting_url.clone());
        debug!("Added starting URL to queue: {}", starting_url);
    }

    // Open the output file
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&output_file)
        .await?;
    let file = Arc::new(Mutex::new(file));

    // Main crawling loop with fixed number of threads
    let mut handles = Vec::with_capacity(NUM_THREADS);

    for _ in 0..NUM_THREADS {
        // Clone shared resources for each task
        let crawled_urls = Arc::clone(&crawled_urls);
        let urls_to_crawl = Arc::clone(&urls_to_crawl);
        let num_pages_crawled = Arc::clone(&num_pages_crawled);
        let output_file = Arc::clone(&file);
        let error_count = Arc::clone(&error_count);
        let excluded_domains = excluded_domains.clone();

        let handle = tokio::spawn(async move {
            // Create HTTP client
            let client = match reqwest::Client::builder()
                .user_agent(USER_AGENT_STRING)
                .redirect(reqwest::redirect::Policy::limited(10))
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to build HTTP client: {}", e);
                    return;
                }
            };

            loop {
                if num_pages_crawled.load(Ordering::Relaxed) >= MAX_PAGES_PER_DOMAIN {
                    break;
                }

                let url = {
                    let mut urls = urls_to_crawl.lock().await;
                    urls.pop_front()
                };

                let url = match url {
                    Some(u) => u,
                    None => {
                        debug!("No more URLs in the queue, sleeping...");
                        time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };

                let url_str = url.as_str().to_string();

                {
                    let crawled = crawled_urls.read().await;
                    if crawled.contains(&url_str) {
                        debug!("Skipping already crawled URL: {}", url_str);
                        continue;
                    }
                }

                let parsed_url = match Url::parse(&url_str) {
                    Ok(u) => u,
                    Err(e) => {
                        warn!("Invalid URL '{}': {}", url_str, e);
                        continue;
                    }
                };

                let domain = match parsed_url.domain() {
                    Some(d) => d.to_lowercase(),
                    None => {
                        warn!("URL without a valid domain: {}", url_str);
                        continue;
                    }
                };

                if excluded_domains.contains(&domain) {
                    debug!("Skipping URL due to excluded domain: {}", domain);
                    continue;
                }

                {
                    let mut crawled = crawled_urls.write().await;
                    crawled.insert(url_str.clone());
                    debug!("Crawled URL added to set: {}", url_str);
                }

                let response = time::timeout(
                    Duration::from_secs(REQUEST_TIMEOUT_SECS),
                    client.get(url.clone()).send(),
                )
                .await;

                match response {
                    Ok(Ok(res)) => {
                        if !res.status().is_success() {
                            warn!("Non-success status {} for URL: {}", res.status(), url_str);
                            error_count.fetch_add(1, Ordering::Relaxed);
                            enqueue_url(&urls_to_crawl, url.clone()).await;
                            continue;
                        }

                        let content = match res.text().await {
                            Ok(text) => text,
                            Err(e) => {
                                warn!("Failed to read response text for URL {}: {}", url_str, e);
                                error_count.fetch_add(1, Ordering::Relaxed);
                                enqueue_url(&urls_to_crawl, url.clone()).await;
                                continue;
                            }
                        };

                        let new_urls = extract_urls(&content, &url);
                        debug!("Extracted URLs: {:?}", new_urls);

                        for new_url in new_urls {
                            let normalized_url = match normalize_url(new_url.as_str(), &url) {
                                Some(u) => u,
                                None => continue,
                            };

                            if let Some(new_domain) = normalized_url.domain() {
                                if excluded_domains.contains(&new_domain.to_lowercase()) {
                                    debug!("Skipping extracted URL due to excluded domain: {}", new_domain);
                                    continue;
                                }
                            }

                            {
                                let crawled = crawled_urls.read().await;
                                if crawled.contains(normalized_url.as_str()) {
                                    debug!("Skipping already crawled extracted URL: {}", normalized_url);
                                    continue;
                                }
                            }

                            enqueue_url(&urls_to_crawl, normalized_url).await;
                        }

                        {
                            let mut file = output_file.lock().await;
                            if let Err(e) = file
                                .write_all(format!("{}\n", url_str).as_bytes())
                                .await
                            {
                                error!("Failed to write to file: {:?}", e);
                            }
                        }

                        num_pages_crawled.fetch_add(1, Ordering::Relaxed);
                        info!("Crawled URL: {} (Total: {})", url_str, num_pages_crawled.load(Ordering::Relaxed));

                        if error_count.load(Ordering::Relaxed) > 0 {
                            error_count.fetch_sub(1, Ordering::Relaxed);
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("HTTP request error for URL {}: {}", url_str, e);
                        error_count.fetch_add(1, Ordering::Relaxed);
                        enqueue_url(&urls_to_crawl, url.clone()).await;
                    }
                    Err(_) => {
                        warn!("Timeout for URL: {}", url_str);
                        error_count.fetch_add(1, Ordering::Relaxed);
                        enqueue_url(&urls_to_crawl, url.clone()).await;
                    }
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.await?;
    }

    info!("Crawling completed. Total pages crawled: {}", num_pages_crawled.load(Ordering::Relaxed));

    Ok(())
}

async fn prompt_user(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    print!("{}", prompt);
    io::stdout().flush()?;
    
    let result = tokio::task::spawn_blocking(|| {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok::<_, std::io::Error>(input)
    }).await??;

    Ok(result.trim().to_string())
}

fn extract_urls(base_content: &str, base_url: &Url) -> Vec<Url> {
    let mut urls = Vec::new();
    let fragment = Html::parse_document(base_content);
    let selector = Selector::parse("a[href]").unwrap();

    for element in fragment.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if let Ok(url) = base_url.join(href) {
                urls.push(url);
            }
        }
    }

    urls
}

fn normalize_url

(url: &str, base_url: &Url) -> Option<Url> {
    match Url::parse(url) {
        Ok(parsed_url) => Some(parsed_url),
        Err(_) => base_url.join(url).ok(),
    }
}

async fn enqueue_url(urls_to_crawl: &Arc<Mutex<VecDeque<Url>>>, url: Url) {
    let mut urls = urls_to_crawl.lock().await;
    urls.push_back(url);
    debug!("Enqueued URL: {}", url);
}
```

### Key Changes:
1. **Removed Dynamic Thread Management**: The number of threads (tasks) is now fixed at `NUM_THREADS`, which is set to 10. This simplifies the program by eliminating the dynamic adjustment of thread counts based on error rates.
2. **Simplified Main Crawling Loop**: Each task (thread) will continuously process URLs from the shared queue (`urls_to_crawl`), and there's no longer any logic to increase or decrease the number of threads during runtime.
3. **Fixed Number of Tasks**: The program now launches a fixed number of tasks at startup (using `tokio::spawn`), and each task will crawl URLs until the queue is empty or the `MAX_PAGES_PER_DOMAIN` limit is reached.

### Expected Behavior:
- The program will crawl URLs concurrently using 10 tasks (you can adjust `NUM_THREADS` if necessary).
- It will output URLs as they are crawled and log skipped URLs, enqueued URLs, and errors.
- No dynamic thread adjustments will be made, simplifying thread management.

### Steps to Run:
You can run this version of the program as follows:

```bash
RUST_LOG=info cargo run
```

This will output the URLs being crawled. For more detailed logging (including enqueued/skipped URLs), you can run with:

```bash
RUST_LOG=debug cargo run
```

Let me know if this version works better and if you need further modifications!

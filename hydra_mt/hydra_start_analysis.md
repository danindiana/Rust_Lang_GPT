This code is a web crawler written in Rust using the Tokio asynchronous runtime. The crawler is designed to crawl a specified domain, extract URLs from the pages, and save the crawled URLs to a file. The crawler also dynamically adjusts the number of active threads based on the error rate. Below is a step-by-step explanation of how the code functions:

### 1. **Imports and Constants**
   - The code imports various libraries and modules, including `scraper` for HTML parsing, `tokio` for asynchronous I/O, and `std::sync` for synchronization primitives.
   - Constants are defined for the user agent string, maximum pages per domain, thread limits, error threshold, and request timeout.

### 2. **Main Function**
   - The `main` function is an asynchronous function that runs the web crawler.
   - It initializes several shared resources using `Arc` (Atomic Reference Counting) and `Mutex` for thread-safe access:
     - `active_threads`: Tracks the number of active crawling threads.
     - `error_count`: Tracks the number of errors encountered.
     - `crawled_urls`: A set of URLs that have already been crawled.
     - `num_pages_crawled`: Tracks the number of pages crawled.
     - `urls_to_crawl`: A map of domains to URLs that need to be crawled.

### 3. **User Input**
   - The program prompts the user to input the domain to crawl, the output file name, and a list of excluded domains.
   - The input is read from the standard input and processed accordingly.

### 4. **Initial Setup**
   - The starting URL is set to the domain provided by the user.
   - The `urls_to_crawl` map is initialized with the starting URL for the specified domain.

### 5. **Crawling Loop**
   - The main loop continues until the number of crawled pages reaches the `MAX_PAGES_PER_DOMAIN` limit.
   - Inside the loop, the number of active threads is determined, and a set of tasks (handles) is created for each thread.

### 6. **Task Creation**
   - For each thread, a new asynchronous task is spawned. Each task:
     - Clones the necessary shared resources.
     - Creates a `reqwest::Client` with a user agent string.
     - Enters a loop that continues until the crawling limit is reached.

### 7. **URL Selection**
   - Each task attempts to get a URL to crawl from the `urls_to_crawl` map.
   - If a URL is found, it checks if the URL has already been crawled or if the domain is excluded.

### 8. **HTTP Request**
   - The task sends an HTTP GET request to the selected URL with a timeout.
   - If the request is successful, the response content is parsed to extract new URLs.

### 9. **HTML Parsing**
   - The response content is parsed using the `scraper` library to extract all `<a>` tags.
   - New URLs are extracted from the `href` attributes of these tags and added to the `urls_to_crawl` map.

### 10. **URL Storage**
   - The crawled URL is added to the `crawled_urls` set.
   - The URL is written to the output file.
   - The `num_pages_crawled` counter is incremented.

### 11. **Error Handling**
   - If the HTTP request fails or times out, the URL is re-added to the `urls_to_crawl` map, and the `error_count` is incremented.

### 12. **Thread Management**
   - After each task completes, the number of active threads is decremented.
   - The main loop checks the current error count and adjusts the number of active threads based on the `ERROR_THRESHOLD`.
   - If the error count exceeds the threshold and there are more than `MIN_THREADS` active, a thread is removed.
   - If the error count is below the threshold and there are fewer than `MAX_THREADS` active, a new thread is added.

### 13. **Completion**
   - The loop continues until the crawling limit is reached, and then the program exits.

### Summary
This code implements a concurrent web crawler that dynamically adjusts the number of threads based on the error rate. It starts by reading user input for the domain to crawl, excluded domains, and output file. It then spawns multiple tasks to crawl URLs concurrently, extracts new URLs from the pages, and writes the crawled URLs to a file. The number of active threads is adjusted based on the error rate to balance performance and reliability.

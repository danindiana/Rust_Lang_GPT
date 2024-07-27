explain why our code now looks like???:

web_crawler_workspace/
├── Cargo.toml
└── web_crawler/
    ├── Cargo.toml
    └── src/
        ├── main.rs
        └── crawler.rs



Your code is structured as a Rust workspace for several reasons, primarily to manage multiple related crates in a single repository. This structure can be beneficial for organizing your project, especially as it grows in complexity. Let's break down why your project now looks like this:

### 1. Modularization and Organization
- **Modularization**: Separating your project into multiple crates within a workspace allows you to modularize your code. Each crate can focus on a specific part of your project, making the codebase easier to manage and understand.
- **Organization**: By organizing related code into separate crates, you can keep the codebase clean and maintainable. For example, you might have a core library crate, an API crate, and a CLI crate, each with a clear purpose.

### 2. Reusability
- **Crate Reuse**: If you have common functionality that might be shared across multiple projects or multiple parts of a single project, putting this functionality into a separate crate makes it easier to reuse. Other crates within the workspace can depend on this common crate.

### 3. Independent Testing and Building
- **Testing**: Each crate can have its own set of tests, allowing you to test components independently. This can make the testing process more manageable and help isolate issues.
- **Building**: You can build and release each crate independently, which can be useful if you need to update one part of your project without affecting others.

### 4. Dependency Management
- **Shared Dependencies**: Dependencies can be shared across multiple crates within the workspace, reducing duplication and ensuring consistency. The top-level `Cargo.toml` can specify shared dependencies.

### Explanation of Your Project Structure

#### Top-Level `Cargo.toml` (Workspace Manifest)

The top-level `Cargo.toml` specifies that this is a workspace and lists the member crates. It does not contain a `[dependencies]` section, as dependencies are specified in the individual crate manifests.

```toml
[workspace]
members = [
    "web_crawler"
]
```

- **[workspace]**: This section declares the workspace and lists its members.
- **members**: Lists the paths to the member crates. In this case, it includes only the `web_crawler` crate.

#### `web_crawler/Cargo.toml` (Crate Manifest)

This file specifies the metadata and dependencies for the `web_crawler` crate.

```toml
[package]
name = "web_crawler"
version = "0.1.0"
edition = "2018"

[dependencies]
reqwest = { version = "0.11", features = ["blocking"] }
scraper = "0.12"
url = "2"
rayon = "1.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
structopt = "0.3"
log = "0.4"
env_logger = "0.9"
chrono = "0.4"
bloom = "0.3.2"
num_cpus = "1.13.0"
```

- **[package]**: Specifies metadata about the crate, such as its name, version, and edition.
- **[dependencies]**: Lists the dependencies required by this crate. Each dependency is specified with its version and, if necessary, additional features.

#### `src/main.rs`

The main entry point for the `web_crawler` crate. This file typically contains the `main` function, which initializes and runs the application.

```rust
mod crawler;

use crawler::Config; // Import the Config struct from the crawler module
use log::info;
use env_logger;
use chrono::Local;
use std::io::{self, Write};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Cli {
    #[structopt(long)]
    pub threads: Option<usize>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Parse command-line arguments
    let args = Cli::from_args();

    // Prompt for the target URL
    print!("Please enter the target URL: ");
    io::stdout().flush()?;
    let mut target_url = String::new();
    io::stdin().read_line(&mut target_url)?;
    let mut target_url = target_url.trim().to_string();

    // Ensure the URL has a scheme
    if !target_url.starts_with("http://") && !target_url.starts_with("https://") {
        target_url = format!("https://{}", target_url);
    }

    // Generate output file name
    let timestamp = Local::now().format("%Y%m%d%H%M%S").to_string();
    let sanitized_url = target_url.replace("http://", "").replace("https://", "").replace("/", "_");
    let output_file = format!("{}_{}.txt", sanitized_url, timestamp);

    // Display generated file name
    println!("Output file will be: {}", output_file);

    // Determine the number of threads to use
    let num_threads = args.threads.unwrap_or_else(|| num_cpus::get());

    let config = Config {
        domain: target_url,
        output: output_file,
        timeout: 30,
        retries: 3,
        threads: num_threads,
        error_threshold: 20,
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.9999.99 Safari/537.36".to_string(),
    };

    info!("Starting web crawler with config: {:?}", config);

    crawler::run(config)?;

    Ok(())
}
```

- **mod crawler;**: Declares the `crawler` module.
- **use statements**: Import necessary crates and modules.
- **main function**: The entry point of the application, handling initialization and user input.

#### `src/crawler.rs`

Contains the core logic for the web crawler.

```rust
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
                                                error!("Error parsing base URL {}:

 {}", url, err);
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
```

- **Dependencies and Imports**: Imports necessary crates and modules.
- **Config Struct**: Defines the configuration options for the crawler.
- **run Function**: Contains the core logic for the web crawler, including URL fetching, parsing, and deduplication using a Bloom filter.

### Summary

By structuring your project as a workspace, you can manage multiple related crates within a single repository, improving modularity, reusability, and maintainability. Each crate has its own `Cargo.toml` file to specify dependencies, and the top-level `Cargo.toml` (workspace manifest) lists the member crates. This approach is particularly useful for larger projects that may have multiple components or modules.

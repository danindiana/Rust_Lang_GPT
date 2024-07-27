Selecting the level of parallelism for the web crawler involves determining the optimal number of concurrent threads to use. This can be influenced by various factors such as the capabilities of the hardware, network bandwidth, and the nature of the target website(s). Here are some considerations and steps to help determine the optimal level of parallelism:

### Considerations

1. **CPU Cores:**
   - More CPU cores can handle more threads efficiently. You can use the number of available CPU cores as a baseline for the number of threads.
   
2. **Network Bandwidth:**
   - If your network bandwidth is limited, having too many threads might saturate the network, leading to timeouts and reduced performance.

3. **Server Load:**
   - Be mindful of the load you are putting on the target server. Too many concurrent requests can lead to IP blocking or rate limiting by the server.

4. **I/O Bound vs CPU Bound:**
   - Web crawling is typically more I/O bound than CPU bound, meaning the bottleneck is likely network latency and disk I/O rather than CPU usage.

### Steps to Select Parallelism

1. **Determine CPU Cores:**
   - Use the number of CPU cores as a starting point. For example, if your machine has 8 cores, you might start with 8 threads.

2. **Adjust Based on Network and Load:**
   - Test with different numbers of threads and monitor the network usage and the load on the target server. Adjust the number of threads up or down based on these observations.

3. **Benchmark and Optimize:**
   - Perform benchmarks with different thread counts to find the optimal balance between performance and resource usage.

4. **Use Dynamic Adjustment:**
   - Implement logic to dynamically adjust the number of threads based on real-time performance metrics.

### Implementation in the Crawler

Here's how you can implement and configure the level of parallelism in your Rust crawler using command-line arguments and environment variables.

### src/main.rs

Update the `main.rs` to allow configuring the number of threads via command-line arguments.

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

### src/crawler.rs

No changes needed here as we already pass the number of threads via the `Config` struct.

### Dependencies

Add the `num_cpus` crate to your `Cargo.toml` to get the number of available CPU cores.

```toml
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

### Usage

1. **Build the Project:**

    ```bash
    cargo build
    ```

2. **Run the Project:**

    ```bash
    cargo run -- --threads 16
    ```

    If you don't specify the `--threads` option, the program will default to the number of CPU cores available on the machine.

By allowing the number of threads to be configurable, you can easily experiment with different levels of parallelism to find the optimal setting for your use case.


The error suggests that you might be using a virtual manifest (usually a workspace `Cargo.toml`) that incorrectly specifies dependencies. Let's make sure the structure of your `Cargo.toml` files is correct.

If you are using a workspace, the top-level `Cargo.toml` (workspace manifest) should not have a `[dependencies]` section. Instead, each member crate should specify its own dependencies.

Here's how you should structure your project:

### Top-Level `Cargo.toml` (Workspace Manifest)

This file should not contain any `[dependencies]` section.

```toml
[workspace]
members = [
    "web_crawler"
]
```

### `web_crawler/Cargo.toml` (Crate Manifest)

This file should specify the dependencies for the `web_crawler` crate.

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

### Project Structure

Ensure that your project structure looks like this:

```
web_crawler_workspace/
├── Cargo.toml
└── web_crawler/
    ├── Cargo.toml
    └── src/
        ├── main.rs
        └── crawler.rs
```

### Top-Level `Cargo.toml` (Workspace Manifest)

This file should not contain any `[dependencies]` section.

```toml
[workspace]
members = [
    "web_crawler"
]
```

### `web_crawler/Cargo.toml` (Crate Manifest)

This file should specify the dependencies for the `web_crawler` crate.

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

### Project Structure

Ensure that your project structure looks like this:

```
web_crawler_workspace/
├── Cargo.toml
└── web_crawler/
    ├── Cargo.toml
    └── src/
        ├── main.rs
        └── crawler.rs
```

### Top-Level `Cargo.toml` (Workspace Manifest)

This file should not contain any `[dependencies]` section.

```toml
[workspace]
members = [
    "web_crawler"
]
```

### `web_crawler/Cargo.toml` (Crate Manifest)

This file should specify the dependencies for the `web_crawler` crate.

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

### Project Structure

Ensure that your project structure looks like this:

```
web_crawler_workspace/
├── Cargo.toml
└── web_crawler/
    ├── Cargo.toml
    └── src/
        ├── main.rs
        └── crawler.rs
```

After ensuring your project structure and `Cargo.toml` files are correct, you should be able to build and test your project:

### Build the Project

```sh
cargo build
```

### Run the Project

```sh
cargo run
```

### Run Tests

```sh
cargo test
```

By following this structure, you should be able to avoid the error and correctly manage your dependencies within a workspace.

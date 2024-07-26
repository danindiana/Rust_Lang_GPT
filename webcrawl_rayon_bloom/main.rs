mod crawler;

use crawler::Config; // Import the Config struct from the crawler module
use log::info;
use env_logger;
use chrono::Local;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

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

    let config = Config {
        domain: target_url,
        output: output_file,
        timeout: 30,
        retries: 3,
        threads: 60,
        error_threshold: 20,
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.9999.99 Safari/537.36".to_string(),
    };

    info!("Starting web crawler with config: {:?}", config);

    crawler::run(config)?;

    Ok(())
}

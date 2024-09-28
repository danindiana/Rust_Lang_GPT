use rayon::prelude::*;
use reqwest::blocking::get;
use select::document::Document;
use select::predicate::Name;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use url::{Url};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Prompt the user for a URL to crawl
    let mut input_url = String::new();
    println!("Enter the URL to crawl:");
    io::stdin().read_line(&mut input_url)?;
    let input_url = input_url.trim();

    // Parse the input URL
    let base_url = Url::parse(input_url)?;

    // Ask for max crawl depth
    let mut input_depth = String::new();
    println!("Enter the max depth for recursive crawling:");
    io::stdin().read_line(&mut input_depth)?;
    let max_depth: usize = input_depth.trim().parse().unwrap_or(3); // Default to 3 if invalid

    // Ask whether to allow external domains
    let mut external_choice = String::new();
    println!("Allow external domains? (yes/no):");
    io::stdin().read_line(&mut external_choice)?;
    let allow_external = external_choice.trim().eq_ignore_ascii_case("yes");

    // Create a thread-safe HashSet using Arc and Mutex
    let crawled_urls = Arc::new(Mutex::new(HashSet::new()));
    crawled_urls.lock().unwrap().insert(base_url.clone());

    // Prepare a text file to save the URLs, based on the input URL
    let file_name = format!("crawled_urls_{}.txt", base_url.host_str().unwrap_or("output"));
    let file = File::create(file_name.clone())?;
    let file = Arc::new(Mutex::new(file));

    // Prepare the log file
    let log_file_name = format!("crawl_log_{}.txt", base_url.host_str().unwrap_or("output"));
    let log_file = OpenOptions::new().create(true).write(true).append(true).open(log_file_name)?;
    let log_file = Arc::new(Mutex::new(log_file));

    // Start recursive crawling
    recursive_crawl(base_url.clone(), crawled_urls.clone(), file.clone(), log_file.clone(), allow_external, 0, max_depth)?;

    // Print the final result and save all crawled URLs to the file
    let crawled_urls = crawled_urls.lock().unwrap();
    crawled_urls.par_iter().for_each(|url| {
        println!("{}", url);
        let mut file = file.lock().unwrap();
        writeln!(file, "{}", url).expect("Unable to write to file");
    });

    println!("Crawling completed. URLs saved to {}", file_name);
    Ok(())
}

// Recursive crawl function with depth control
fn recursive_crawl(
    base_url: Url,
    crawled_urls: Arc<Mutex<HashSet<Url>>>,
    file: Arc<Mutex<File>>,
    log_file: Arc<Mutex<File>>,
    allow_external: bool,
    current_depth: usize,
    max_depth: usize
) -> Result<(), Box<dyn std::error::Error>> {
    if current_depth > max_depth {
        return Ok(());
    }

    let response = get(base_url.as_str());
    match response {
        Ok(resp) => {
            let document = Document::from_read(resp)?;
            log_to_file(log_file.clone(), format!("SUCCESS: {}", base_url))?;
            
            let links: Vec<_> = document
                .find(Name("a"))
                .filter_map(|node| node.attr("href"))
                .collect();

            links.into_par_iter().for_each(|link| {
                if let Ok(url) = Url::parse(link).or_else(|_| base_url.join(link)) {
                    if should_crawl(&base_url, &url, allow_external) {
                        let cloned_crawled_urls = Arc::clone(&crawled_urls);
                        let should_crawl_next = {
                            let mut crawled_urls = cloned_crawled_urls.lock().unwrap();
                            crawled_urls.insert(url.clone())
                        };

                        if should_crawl_next {
                            // Print crawled URL to the console
                            println!("Crawled URL: {}", url);

                            // Log and crawl further recursively
                            log_to_file(log_file.clone(), format!("CRAWLING: {}", url)).ok();
                            recursive_crawl(url, cloned_crawled_urls, file.clone(), log_file.clone(), allow_external, current_depth + 1, max_depth).ok();
                        }
                    }
                }
            });
        }
        Err(err) => {
            log_to_file(log_file.clone(), format!("ERROR: {} - {}", base_url, err))?;
        }
    }

    Ok(())
}



// Log function to write to the log file
fn log_to_file(log_file: Arc<Mutex<File>>, message: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = log_file.lock().unwrap();
    writeln!(file, "{}", message)?;
    Ok(())
}

// Decide if we should crawl this URL based on the domain
fn should_crawl(base_url: &Url, target_url: &Url, allow_external: bool) -> bool {
    if !allow_external && base_url.domain() != target_url.domain() {
        return false;
    }
    true
}

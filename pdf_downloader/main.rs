use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::collections::VecDeque;

use scraper::{Html, Selector};
use url::Url;
use fkget::fk_get;

const MAX_DEPTH: usize = 5;

#[tokio::main]
async fn main() {
    // Get file path from user
    let file_path = get_file_path_from_user();

    // Validate file existence
    if !Path::new(&file_path).exists() {
        println!("File does not exist. Please check the path and try again.");
        return;
    }

    // Read HTML links from file
    let file = File::open(file_path).unwrap();
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let link = line.unwrap();
        // Crawl link and download PDFs
        if let Err(e) = crawl_and_download(&link).await {
            println!("Failed to process {}: {}", link, e);
        }
    }
}

async fn crawl_and_download(start_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut work_queue: VecDeque<(usize, String)> = VecDeque::new();
    work_queue.push_back((0, start_url.to_string()));

    while let Some((depth, url)) = work_queue.pop_front() {
        if depth > MAX_DEPTH {
            println!("Reached maximum depth. Stopping recursion.");
            continue;
        }

        println!("Processing: {}", url);

        // Fetch the HTML content of the webpage
        let resp = reqwest::get(&url).await?;
        let body = resp.text().await?;

        // Parse the HTML
        let parsed_html = Html::parse_document(&body);

        // Create a Selector for PDF files
        let pdf_selector = Selector::parse("a[href$='.pdf']").unwrap();

        // Iterate over all found PDF files and download them
        for pdf_link in parsed_html.select(&pdf_selector) {
            let pdf_url = pdf_link.value().attr("href").unwrap();
            let pdf_url = if pdf_url.starts_with("http") {
                pdf_url.to_string()
            } else {
                let base_url = Url::parse(&url).unwrap();
                base_url.join(pdf_url).unwrap().as_str().to_string()
            };

            println!("Downloading: {}", pdf_url);
            if let Err(e) = fk_get::download_file(&pdf_url).await {
                println!("Failed to download {}: {}", pdf_url, e);
            }
        }

        // Create a Selector for links
        let link_selector = Selector::parse("a").unwrap();

        // Iterate over all found links and recursively crawl them
        for link in parsed_html.select(&link_selector) {
            if let Some(link_url) = link.value().attr("href") {
                let link_url = if link_url.starts_with("http") {
                    link_url.to_string()
                } else {
                    let base_url = Url::parse(&url).unwrap();
                    base_url.join(link_url).unwrap().as_str().to_string()
                };

                work_queue.push_back((depth + 1, link_url));
            }
        }
    }
    Ok(())
}

fn get_file_path_from_user() -> String {
    let mut file_path = String::new();
    println!("Please enter the file path: ");
    io::stdin()
        .read_line(&mut file_path)
        .expect("Failed to read line");
    file_path.trim().to_string()
}

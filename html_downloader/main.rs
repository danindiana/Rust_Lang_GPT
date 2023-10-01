use reqwest;
use select::document::Document;
use select::predicate::Name;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;
use url::Url;

const DEFAULT_DIR: &str = "crawled_pages";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Enter the initial URL to crawl:");
    let mut url = String::new();
    io::stdin().read_line(&mut url)?;
    let url = url.trim_end();

    let mut visited = HashSet::new();
    crawl(&url, &mut visited)?;

    Ok(())
}

fn crawl(url: &str, visited: &mut HashSet<String>) -> Result<(), Box<dyn std::error::Error>> {
    if visited.contains(url) {
        return Ok(());
    }
    println!("Crawling: {}", url);
    visited.insert(url.to_string());

    let resp = reqwest::blocking::get(url)?;
    let body = resp.text()?;

    save_to_file(url, &body)?;

    let document = Document::from_read(body.as_bytes())?;

    for node in document.find(Name("a")) {
        if let Some(link) = node.attr("href") {
            let base_url = Url::parse(url)?;
            if let Ok(absolute_url) = base_url.join(link) {
                if !visited.contains(absolute_url.as_str()) && is_http_or_https(&absolute_url) {
                    crawl(absolute_url.as_str(), visited)?;
                }
            }
        }
    }

    Ok(())
}

fn save_to_file(url: &str, content: &str) -> Result<(), io::Error> {
    let hashed_filename = format!("{:x}.html", md5::compute(url));
    let path = Path::new(DEFAULT_DIR).join(hashed_filename);

    if !Path::new(DEFAULT_DIR).exists() {
        fs::create_dir(DEFAULT_DIR)?;
    }

    fs::write(path, content)?;
    Ok(())
}

fn is_http_or_https(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
}

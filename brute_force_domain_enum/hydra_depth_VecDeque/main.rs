use reqwest::Client;
use select::document::Document;
use select::node::Node;
use select::predicate::Name;
use std::collections::{HashSet, VecDeque};
use std::fs::File;
use std::io::{BufWriter, Write};
use url::Url;

async fn fetch_links(
    client: &Client,
    url: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    let document = Document::from_read(body.as_bytes())?;

    let base_url = Url::parse(url)?;
    let links: Vec<String> = document
        .find(Name("a"))
        .filter_map(|node| node.attr("href"))
        .filter_map(|link| base_url.join(link).ok())
        .filter(|url| url.host_str() == base_url.host_str())
        .map(|url| url.into_string())
        .collect();

    Ok(links)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let mut domain = String::new();
    println!("Enter the domain to crawl:");
    std::io::stdin().read_line(&mut domain)?;
    domain = domain.trim().to_string();

    let mut file_name = String::new();
    println!("Enter the output file name:");
    std::io::stdin().read_line(&mut file_name)?;
    let file = File::create(file_name.trim())?;
    let mut writer = BufWriter::new(file);

    let mut depth_input = String::new();
    println!("Enter the maximum depth to crawl:");
    std::io::stdin().read_line(&mut depth_input)?;
    let max_depth: usize = depth_input.trim().parse()?;

    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    let mut crawled_urls = HashSet::new();

    queue.push_back((domain, 0));

    while let Some((url, depth)) = queue.pop_front() {
        if depth > max_depth || crawled_urls.contains(&url) {
            continue;
        }

        println!("Crawling: {}", url);
        crawled_urls.insert(url.clone());
        writeln!(writer, "{}", url)?;

        if let Ok(links) = fetch_links(&client, &url).await {
            for link in links {
                if !crawled_urls.contains(&link) {
                    queue.push_back((link, depth + 1));
                }
            }
        }
    }

    println!("Crawling complete!");

    Ok(())
}

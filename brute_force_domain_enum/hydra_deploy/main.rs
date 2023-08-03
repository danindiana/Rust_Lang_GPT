use reqwest::Client;
use reqwest::Error as ReqwestError;
use select::document::Document;
use select::predicate::Name;
use std::collections::{HashSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write, Read, BufReader, BufRead};
use tokio::sync::Semaphore;
use url::Url;
use tokio::runtime::Builder;
use std::time::Duration;

async fn fetch_links(
    client: &Client,
    url: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let response = client.get(url).send().await.map_err(|e| handle_reqwest_error(e, url))?;
    let body = response.text().await?;
    let document = Document::from_read(body.as_bytes())?;

    let base_url = Url::parse(url)?;
    let links: Vec<String> = document
        .find(Name("a"))
        .filter_map(|node| node.attr("href"))
        .filter_map(|link| base_url.join(link).ok())
        .filter(|url| url.host_str() == base_url.host_str())
        .map(Into::into)
        .collect();

    Ok(links)
}

fn handle_reqwest_error(err: ReqwestError, url: &str) -> Box<dyn std::error::Error> {
    println!("Error while fetching {}: {:?}", url, err);
    err.into()
}

const QUEUE_FLUSH_SIZE: usize = 9000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let worker_threads = 3;
    let stack_size = 70 * 1024 * 1024;

    let runtime = Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .thread_stack_size(stack_size)
        .enable_all()
        .build()?;
    runtime.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(7)) // Set timeout to 10 seconds
        .build()?;
    let client = Client::new();
    let semaphore = Semaphore::new(7);

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

    let queue_backup_path = "queue_backup.txt";
    std::fs::write(queue_backup_path, "")?; // Initialize empty file

    while let Some((url, depth)) = queue.pop_front() {
        if queue.len() < QUEUE_FLUSH_SIZE {
            // Read backup queue from file
            let mut file_read = File::open(queue_backup_path)?;
            let reader = BufReader::new(file_read);
            for line in reader.lines() {
                let line = line?;
                let parts: Vec<&str> = line.split('\t').collect();
                queue.push_back((parts[0].to_string(), parts[1].parse()?));
            }
        }

        if depth > max_depth || crawled_urls.contains(&url) {
            continue;
        }

        let permit = semaphore.acquire().await.expect("acquire permit");
        println!("Crawling: {}", url);
        crawled_urls.insert(url.clone());
        writeln!(writer, "{}", url)?;

        if let Ok(links) = fetch_links(&client, &url).await {
            for link in links {
                if !crawled_urls.contains(&link) {
                    if queue.len() >= QUEUE_FLUSH_SIZE {
                        // Write to backup queue file
                        let mut queue_file = OpenOptions::new().append(true).open(queue_backup_path)?;
                        writeln!(queue_file, "{}\t{}", link, depth + 1)?;
                    } else {
                        queue.push_back((link, depth + 1));
                    }
                }
            }
        }

        drop(permit);
    }

    println!("Crawling complete!");
    std::fs::remove_file(queue_backup_path)?; // Cleanup

    Ok(())
}

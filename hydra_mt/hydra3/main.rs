use rayon::prelude::*;
use reqwest::blocking::get;
use select::document::Document;
use select::predicate::Name;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Write};
use std::sync::Mutex;
use url::{Url};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Prompt the user for a URL to crawl
    let mut input_url = String::new();
    println!("Enter the URL to crawl:");
    io::stdin().read_line(&mut input_url)?;
    let input_url = input_url.trim();

    // Parse the input URL
    let base_url = Url::parse(input_url)?;
    let mut crawled_urls = HashSet::new();
    crawled_urls.insert(base_url.clone());

    // Prepare a text file to save the URLs, based on the input URL
    let file_name = format!("crawled_urls_{}.txt", base_url.host_str().unwrap_or("output"));
    let file = File::create(file_name)?;
    let file = Mutex::new(file);

    // Extract and crawl links in parallel
    crawl_url(base_url.clone(), &mut crawled_urls, &file)?;

    // Save the crawled URLs to the text file and console output
    let crawled_urls: Vec<_> = crawled_urls.into_par_iter().collect();

    crawled_urls.par_iter().for_each(|url| {
        println!("{}", url);
        let mut file = file.lock().unwrap();
        writeln!(file, "{}", url).expect("Unable to write to file");
    });

    println!("Crawling completed.");

    Ok(())
}

fn crawl_url(base_url: Url, crawled_urls: &mut HashSet<Url>, file: &Mutex<File>) -> Result<(), Box<dyn std::error::Error>> {
    let response = get(base_url.as_str())?;
    let document = Document::from_read(response)?;

    let links: Vec<_> = document
        .find(Name("a"))
        .filter_map(|node| node.attr("href"))
        .collect();

    links.into_par_iter().for_each(|link| {
        if let Ok(url) = Url::parse(link).or_else(|_| base_url.join(link)) {
            let mut file = file.lock().unwrap();
            if crawled_urls.insert(url.clone()) {
                writeln!(file, "{}", url).expect("Unable to write to file");
            }
        }
    });

    Ok(())
}

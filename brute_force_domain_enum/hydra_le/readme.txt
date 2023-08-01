To add additional URL domains to exclude (e.g., "youtube" and "linkedin"), you can modify the code to check for these domains in addition to "facebook" when iterating through the urls_to_crawl HashMap. You can create a set of excluded domains and use it to check if the URL contains any of the excluded domains. Here's how you can do it:


In this code, we define an array EXCLUDED_DOMAINS that contains the domains we want to exclude (e.g., "facebook", "youtube", and "linkedin"). Then, during the iteration, we use the iter() method along with any() to check if the URL contains any of the excluded domains. If it doesn't contain any excluded domains, we proceed to crawl that URL; otherwise, we skip it.

Now, the crawler will ignore URLs that contain any of the specified excluded domains and continue crawling other URLs.

Two problems with that code. 

1.) That code appears not to be excluding domains.
2.) The code contains to crawl the same URLs twice. 

See this sample output:

https://blog.flickr.net/en/2023/07/25/concert-photography-photographers-on-the-scene-and-in-the-crowds/
https://blog.flickr.net/en/2023/07/28/weekly-snapshot-with-flickr-social-7-28-2023/
https://blog.flickr.net/en/2023/07/28/weekly-snapshot-with-flickr-social-7-28-2023/
https://blog.flickr.net/en/2023/07/14/16-questions-about-one-photo-with-michal-jeska/
https://www.flickr.com/
https://blog.flickr.net/en/2023/07/14/16-questions-about-one-photo-with-michal-jeska/

Can you reason about what is happening here and propose a fix? PLease refactor the code and produce the corrected code in it's entirety.

Apologies for the oversight. You are correct; the previous implementation did not properly exclude domains, and it also caused duplicate URLs to be crawled. I'll provide a revised and refactored version of the code to address these issues.

To fix the problems, we will update the loop to check for excluded domains correctly and maintain a set of crawled URLs to avoid duplicates. Here's the corrected and refactored code:

In the updated code, we define a separate function should_exclude_domain to check if a given URL should be excluded based on the EXCLUDED_DOMAINS array. The should_exclude_domain function checks if the URL contains any of the domains specified in EXCLUDED_DOMAINS and returns true if it matches any excluded domain, otherwise false.

The crawler will now properly exclude URLs containing the specified domains and avoid crawling the same URLs multiple times.


The depth of crawling of a given URL is controlled by the following portion of the code:

while num_pages_crawled.load(std::sync::atomic::Ordering::Relaxed) < MAX_PAGES_PER_DOMAIN {
    // ... (other code)

    if let Some((domain, url)) = domain_to_crawl {
        let response = client.get(&url).send().await;

        match response {
            Ok(res) => {
                // ... (other code)

                // Check the current depth and limit the crawling to a certain level.
                if num_pages_crawled.load(std::sync::atomic::Ordering::Relaxed) >= MAX_PAGES_PER_DOMAIN {
                    break;
                }
            }
            // ... (other code)
        }
    }
}

Explanation:

The while loop runs as long as the total number of pages crawled (num_pages_crawled) is less than the maximum allowed number of pages per domain (MAX_PAGES_PER_DOMAIN).

Within the loop, when a URL is selected for crawling (inside the if let Some((domain, url)) = domain_to_crawl { ... } block), it sends an HTTP request to fetch the content of the URL using the reqwest crate. The response is then processed.

After a successful response (Ok(res) => { ... }), the code checks if the number of pages crawled has reached or exceeded the maximum allowed number of pages per domain (MAX_PAGES_PER_DOMAIN). If it has reached this limit, the loop will break, and the crawling process will stop.

This control mechanism ensures that the crawler stops crawling new pages once it has reached the specified depth, defined by MAX_PAGES_PER_DOMAIN. The num_pages_crawled atomic counter keeps track of the number of pages crawled, and when it exceeds the defined limit, the loop exits, preventing the crawler from crawling more pages than intended.

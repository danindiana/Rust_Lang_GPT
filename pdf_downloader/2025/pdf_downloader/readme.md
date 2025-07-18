# Secure PDF Downloader - Code Explanation

This Rust program is a secure PDF downloader that crawls web pages, finds PDF links, and downloads them safely to a local directory. Here's a detailed breakdown:

## Key Components

### 1. Dependencies
- **reqwest**: For HTTP requests
- **thirtyfour**: For Selenium WebDriver integration
- **select**: For HTML parsing
- **regex**: For pattern matching
- **anyhow**: For error handling
- **lazy_static**: For compiling regex patterns once
- **tokio**: For async runtime

### 2. Main Structure

The core is the `SecurePDFDownloader` struct which contains:
- `download_dir`: Where PDFs are saved
- `visited_urls`: Tracks crawled URLs to avoid duplicates
- `client`: HTTP client for requests
- `driver`: Selenium WebDriver for JavaScript rendering

### 3. Key Features

#### PDF Detection
- Checks both URL path (`.pdf` extension) and `Content-Type` header
- Uses HEAD requests when extension isn't obvious

#### Safe Downloading
- Verifies file size before and during download
- Implements rate limiting with jitter
- Validates PDF content type
- Prevents path traversal attacks

#### Filename Sanitization
- Extracts filenames from `Content-Disposition` headers or URLs
- Removes dangerous characters
- URL-decodes filenames
- Trims to 255 characters

#### Web Crawling
- Breadth-first search with configurable depth
- Same-domain restriction for recursive crawling
- JavaScript rendering via Selenium
- Timeout handling for page rendering

### 4. Workflow

1. **Initialization**:
   - Creates download directory
   - Sets up HTTP client with timeout
   - Launches WebDriver (requires chromedriver)

2. **Crawling**:
   - Renders pages with JavaScript support
   - Extracts all links from HTML
   - Checks each link for PDFs

3. **Downloading**:
   - Verifies PDF content
   - Checks file size
   - Sanitizes filename
   - Streams download with size monitoring

4. **Cleanup**:
   - Closes WebDriver session

### 5. Safety Measures

- **Input Validation**: All URLs are properly parsed and joined
- **Path Sanitization**: Ensures downloads stay in designated directory
- **Size Limits**: Prevents excessively large downloads
- **Rate Limiting**: Avoids overwhelming servers
- **Content Verification**: Confirms actual PDF content

### 6. Usage

The program prompts for:
1. Target URL to start crawling
2. Whether to do recursive crawling
3. Maximum crawl depth (if recursive)

## Example Execution

```text
=== Secure PDF Downloader ===
NOTE: Ensure chromedriver is running before starting.
Enter target URL: https://example.com/research
Recursive search on the same domain? (y/n): y
Max depth (e.g., 2, or Enter for unlimited): 2

Scanning [Depth 0]: https://example.com/research
  ✓ Downloaded: paper1.pdf
  ✓ Downloaded: study2023.pdf

Scanning [Depth 1]: https://example.com/research/papers
  ✓ Downloaded: methodology.pdf

Scan complete. Total PDFs downloaded: 3
```

This tool is particularly useful for researchers or archivists who need to download collections of PDFs from websites, including those with JavaScript-rendered content.

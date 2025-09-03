This Rust program is a **web crawler specifically designed to discover and verify PDF files** on websites. Here's a comprehensive breakdown of how it works:

## Core Functionality

The crawler systematically explores web pages starting from a given URL, extracts links, identifies potential PDF files, verifies their authenticity, and saves the results to a JSON file.

## Key Components

### 1. Command Line Interface (CLI)
- Uses `clap` to parse command-line arguments
- Configurable parameters: starting URL, crawl depth, concurrency, delay, output file, robots.txt respect, and PDF verification

### 2. PDF Detection System
- **URL pattern matching**: Looks for `.pdf` extensions and PDF-related keywords
- **Query parameter analysis**: Identifies PDF-specific parameters like `format=pdf`
- **Content analysis**: Verifies actual PDF content through HTTP requests

### 3. Concurrent Architecture
- Uses `tokio` for async operations
- **Semaphore-based concurrency control** to limit simultaneous requests
- **Channel-based communication** between crawler and writer tasks

### 4. Verification System
- **HEAD requests** for quick metadata checks
- **Partial GET requests** (first 8KB) to scan for PDF magic bytes (`%PDF`)
- **Content type and length validation**

## Workflow Process

### Initialization Phase
1. Parse command-line arguments
2. Create HTTP client with HTTPS support
3. Set up channel for communication between crawler and writer
4. Initialize output JSON file with empty structure

### Crawling Phase
1. **Breadth-first search** using a queue system
2. For each URL:
   - Check robots.txt permissions (if enabled)
   - Fetch HTML content with configurable delays
   - Extract all links from the page
   - Identify potential PDF URLs using pattern matching

### PDF Verification Phase
1. For each potential PDF:
   - Perform HEAD request to check content type and size
   - If inconclusive, perform partial GET request to scan for PDF magic bytes
   - Validate PDF authenticity based on content and headers

### Data Management
1. **Writer task** runs concurrently to:
   - Receive PDF discoveries and metadata updates via channel
   - Write results to JSON file incrementally
   - Use atomic file writes (write to temp file then rename)

### Completion Phase
1. Generate comprehensive summary statistics
2. Finalize metadata with crawl results
3. Display success rates and verification statistics

## Key Features

- **Respectful crawling**: Configurable delays and robots.txt compliance
- **Efficient verification**: Minimal data transfer for PDF validation
- **Real-time updates**: JSON file updates during crawling
- **Error resilience**: Continues crawling despite individual failures
- **Cross-domain restriction**: Only crawls same-domain links by default

## Output Structure

The program generates a JSON file containing:
- **Metadata**: Crawl configuration, statistics, and status
- **PDF information**: URLs, source pages, verification status, file sizes, and discovery timestamps

This crawler is particularly useful for academic research, document collection, or building PDF repositories from websites while ensuring the files are genuine PDF documents.

# PDF Crawler Documentation

## Overview

This Rust-based PDF crawler is a high-performance, concurrent web crawler specifically designed to discover and download PDF documents from websites. It uses advanced heuristics for PDF detection, intelligent rate limiting, and robust validation to efficiently crawl websites while respecting server constraints.

## Key Features

- **Multi-layered PDF detection** using URL patterns, content analysis, and header inspection
- **Concurrent processing** with configurable parallelism for crawling and downloading
- **Intelligent rate limiting** per host to avoid overloading servers
- **Robust validation** ensuring downloaded files are valid PDFs
- **Resume capability** for interrupted downloads
- **Comprehensive monitoring** with real-time statistics
- **Memory-efficient caching** for visited URLs and PDF detection results

## How It Works

### 1. Architecture

The crawler uses a producer-consumer pattern with multiple specialized worker pools:

- **Crawler workers** extract links from HTML pages
- **PDF detector workers** analyze URLs to identify PDFs
- **Download workers** handle actual PDF downloads with validation

### 2. PDF Detection Pipeline

The system uses a multi-stage approach to identify PDFs:

1. **URL pattern matching** - Checks for common PDF URL patterns
2. **Anchor text analysis** - Examines link text for PDF-related keywords
3. **HEAD request inspection** - Checks Content-Type headers
4. **Range request validation** - Downloads first kilobyte to check magic bytes

### 3. Concurrent Processing

```rust
// Configuration defaults:
concurrent_crawlers: 30    // HTML page processors
concurrent_checks: 10      // PDF detection workers  
concurrent_downloads: 4    // PDF download workers
global_socket_limit: 5000  // Maximum simultaneous connections
```

### 4. Rate Limiting

The implementation includes sophisticated rate limiting:

- Per-host rate limiting (default: 20 requests/second)
- Global connection limiting to prevent system overload
- Automatic retry with exponential backoff for failed requests

### 5. Validation System

Downloaded files undergo comprehensive validation:

- Header validation checking for PDF magic bytes
- Structure validation ensuring proper PDF format
- Size validation to prevent downloading excessively large files
- Content-type verification to avoid HTML masquerading as PDFs

## Usage

### Basic Configuration

```rust
let config = CrawlerConfig {
    download_dir: PathBuf::from("pdf_downloads"),
    concurrent_crawlers: 20,
    concurrent_downloads: 4,
    // ... other settings
};
```

### Running the Crawler

```rust
let crawler = ParallelPDFCrawler::new(config).await?;
crawler.crawl_and_download("https://example.com", true, Some(3)).await?;
```

### Command Line Interface

The application includes an interactive CLI:

```
=== OPTIMIZED PDF CRAWLER V4.4 (ANTI-STALL) ===
Enter target URL: https://example.com
Recursive search? (y/n): y
Max depth (or Enter for default): 3
```

## Performance Optimizations

1. **Efficient URL deduplication** using DashSet with FxHash for fast hashing
2. **Memory-aware caching** using Moka with TTL-based eviction
3. **Connection pooling** with keep-alive and HTTP/2 support
4. **Batch processing** of links for improved throughput
5. **Zero-copy operations** where possible to reduce memory pressure

## Error Handling and Resilience

- Automatic retry for failed requests with configurable limits
- Timeout protection for all network operations
- Graceful shutdown handling
- Comprehensive validation to prevent corrupted downloads
- Stall detection with automatic recovery

## Output Structure

Downloaded PDFs are saved with sanitized filenames in the specified directory, with naming derived from:
- Content-Disposition headers (when available)
- URL path components
- Automatic ".pdf" extension addition when needed

## Dependencies

Key dependencies include:
- `reqwest` - HTTP client with async support
- `tokio` - Async runtime
- `dashmap` - Concurrent hash maps
- `scraper` - HTML parsing
- `governor` - Rate limiting
- `moka` - Caching implementation

## Monitoring

The crawler provides real-time statistics:
```
[STATUS] Visited: 542 | Downloaded: 23 | Failed: 2 | Queued: 8 | Cache hits: 156
```

This implementation represents a robust, production-ready PDF crawling solution with careful attention to performance, reliability, and respectful crawling practices.

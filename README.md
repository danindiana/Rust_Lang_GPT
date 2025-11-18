# Rust_Lang_GPT

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![GitHub repo size](https://img.shields.io/github/repo-size/danindiana/Rust_Lang_GPT?style=for-the-badge)](https://github.com/danindiana/Rust_Lang_GPT)
[![GitHub stars](https://img.shields.io/github/stars/danindiana/Rust_Lang_GPT?style=for-the-badge)](https://github.com/danindiana/Rust_Lang_GPT/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/danindiana/Rust_Lang_GPT?style=for-the-badge)](https://github.com/danindiana/Rust_Lang_GPT/network)
[![GitHub issues](https://img.shields.io/github/issues/danindiana/Rust_Lang_GPT?style=for-the-badge)](https://github.com/danindiana/Rust_Lang_GPT/issues)

A comprehensive collection of Rust programs demonstrating various concepts, from basic file operations to advanced concurrent web crawling and data processing. These programs showcase Rust's capabilities in systems programming, async I/O, parallel processing, and network operations.

## Table of Contents

- [Overview](#overview)
- [Git Workflow](#git-workflow)
- [Repository Structure](#repository-structure)
- [Project Categories](#project-categories)
- [Technology Stack](#technology-stack)
- [Getting Started](#getting-started)

## Overview

This repository contains a curated collection of Rust programs exploring various domains:
- **Web Crawling & Scraping**: Multiple implementations using different concurrency models
- **File Operations**: Search, hash verification, and file management utilities
- **Network Programming**: DNS enumeration, TCP servers, and domain brute-forcing
- **Concurrent Programming**: Examples using Tokio (async) and Rayon (data parallelism)
- **Data Processing**: Word counting, PDF processing, and document indexing

## Git Workflow

```mermaid
gitGraph
    commit id: "Initial commit"
    commit id: "Add LICENSE"
    commit id: "Add project ideas"
    branch claude/add-git-mermaid-diagrams-01P4kUMzFZyKTJe3taKjomV2
    checkout claude/add-git-mermaid-diagrams-01P4kUMzFZyKTJe3taKjomV2
    commit id: "Add mermaid diagrams"
    commit id: "Update README structure"
    checkout main
    merge claude/add-git-mermaid-diagrams-01P4kUMzFZyKTJe3taKjomV2
    commit id: "Continue development"
```

### Branch Strategy

```mermaid
graph LR
    A[main] --> B[Feature Branch]
    B --> C[Development]
    C --> D[Testing]
    D --> E[Pull Request]
    E --> F{Code Review}
    F -->|Approved| G[Merge to main]
    F -->|Changes Needed| C

    style A fill:#90EE90
    style G fill:#90EE90
    style B fill:#87CEEB
    style F fill:#FFD700
```

## Repository Structure

```mermaid
graph TD
    A[Rust_Lang_GPT] --> B[Web Crawlers]
    A --> C[File Operations]
    A --> D[Network Tools]
    A --> E[Concurrent Processing]
    A --> F[Utilities]
    A --> G[Examples]
    A --> H[Documentation]

    B --> B1[webcrawler]
    B --> B2[webcrawl_rayon]
    B --> B3[webcrawl_rayon_bloom]
    B --> B4[web_crawler_pdf]
    B --> B5[html_downloader]
    B --> B6[hydra_mt]

    C --> C1[file-search]
    C --> C2[file_mover]
    C --> C3[indexer]

    D --> D1[brute_force_domain_enum]
    D --> D2[tokio_tests]

    E --> E1[rayon examples]
    E --> E2[rayon_word]
    E --> E3[rayon_hasher]
    E --> E4[posix_pipe]

    F --> F1[pdf_downloader]
    F --> F2[rustbert]

    G --> G1[file_search v1-v6]
    G --> G2[dns_enum]
    G --> G3[sha2search]
    G --> G4[neosearch]

    H --> H1[Rust program ideas]
    H --> H2[Protobuf ideas]
    H --> H3[Error logs]

    style A fill:#FF6B6B
    style B fill:#4ECDC4
    style C fill:#45B7D1
    style D fill:#96CEB4
    style E fill:#FFEAA7
    style F fill:#DFE6E9
    style G fill:#FFB6C1
    style H fill:#98D8C8
```

### Directory Overview

| Category | Projects | Description |
|----------|----------|-------------|
| **Web Crawlers** | `webcrawler`, `webcrawl_rayon`, `webcrawl_rayon_bloom`, `web_crawler_pdf`, `html_downloader`, `hydra_mt` | Various web crawling implementations with different concurrency strategies |
| **File Operations** | `file-search`, `file_mover`, `indexer` | File system utilities for searching, moving, and indexing |
| **Network Tools** | `brute_force_domain_enum`, `tokio_tests` | Network programming examples including DNS enumeration and TCP servers |
| **Concurrent Processing** | `rayon`, `rayon_word`, `rayon_hasher`, `posix_pipe` | Parallel processing examples using Rayon and inter-process communication |
| **Utilities** | `pdf_downloader`, `rustbert` | Specialized utilities for document processing and ML integration |
| **Examples** | `file_search` (v1-v6), `dns_enum`, `sha2search`, `neosearch` | Standalone example programs demonstrating various Rust concepts |
| **Documentation** | `docs/` | Project ideas, notes, and error logs |

## Project Categories

### Web Crawling & Scraping

```mermaid
graph TB
    subgraph "Web Crawler Evolution"
        A[Basic Crawler<br/>webcrawler] --> B[Parallel Crawler<br/>webcrawl_rayon]
        B --> C[Optimized Crawler<br/>webcrawl_rayon_bloom]
        C --> D[Specialized Crawler<br/>web_crawler_pdf]
    end

    subgraph "Key Features"
        E[Async I/O<br/>Tokio]
        F[Data Parallelism<br/>Rayon]
        G[Duplicate Detection<br/>Bloom Filters]
        H[Content Verification<br/>PDF Magic Bytes]
    end

    A -.-> E
    B -.-> F
    C -.-> G
    D -.-> H

    style A fill:#E8F5E9
    style B fill:#C8E6C9
    style C fill:#A5D6A7
    style D fill:#81C784
```

**Implementations:**
- `webcrawler`: Basic web crawler with domain filtering
- `webcrawl_rayon`: Parallel web crawler using Rayon for data parallelism
- `webcrawl_rayon_bloom`: Optimized crawler with Bloom filter for duplicate detection
- `web_crawler_pdf`: Specialized crawler for discovering and verifying PDF files
- `html_downloader`: Simple HTML content fetcher
- `hydra_mt`: Multi-threaded crawler with dynamic thread management

### File Operations

```mermaid
flowchart LR
    A[File Search Evolution] --> B[v1: Basic Search]
    B --> C[v2: Pattern Matching]
    C --> D[v3: Multi-threaded]
    D --> E[v4: Optimized]
    E --> F[v5: Advanced Filters]
    F --> G[v6: Production Ready]

    H[Related Tools] --> I[file_mover]
    H --> J[file-search]
    H --> K[indexer]

    style A fill:#FFF3E0
    style G fill:#FFB74D
    style H fill:#FFCCBC
```

**Components:**
- `examples/file_search*.rs` (v1-v6): Evolution of file search utilities with increasing sophistication
- `file_mover/`: Utility for organizing and moving files
- `file-search/`: File search project directory
- `examples/sha2search.rs`: SHA-256 hash verification and search
- `indexer/`: File indexing system

### Network Programming

```mermaid
sequenceDiagram
    participant Client
    participant DNS Enum
    participant Domain Bruteforce
    participant Target Server

    Client->>DNS Enum: Request DNS info
    DNS Enum->>Target Server: Query DNS records
    Target Server-->>DNS Enum: DNS response
    DNS Enum-->>Client: Parsed results

    Client->>Domain Bruteforce: Start enumeration
    loop For each subdomain
        Domain Bruteforce->>Target Server: Check subdomain
        Target Server-->>Domain Bruteforce: Response
    end
    Domain Bruteforce-->>Client: Found subdomains
```

**Tools:**
- `examples/dns_enum.rs`: DNS enumeration utility
- `brute_force_domain_enum/`: Subdomain brute-forcing tool
- `tokio_tests/`: Async networking examples including TCP server

### Concurrent Processing

```mermaid
graph TD
    subgraph "Concurrency Models"
        A[Async I/O<br/>Tokio Runtime]
        B[Data Parallelism<br/>Rayon]
        C[IPC<br/>POSIX Pipes]
    end

    subgraph "Use Cases"
        D[I/O Bound Tasks<br/>Web Crawling]
        E[CPU Bound Tasks<br/>Hashing/Processing]
        F[Process Communication<br/>Pipeline Processing]
    end

    A -.->|Best for| D
    B -.->|Best for| E
    C -.->|Best for| F

    style A fill:#E1BEE7
    style B fill:#CE93D8
    style C fill:#BA68C8
    style D fill:#E8F5E9
    style E fill:#C8E6C9
    style F fill:#A5D6A7
```

**Examples:**
- `rayon`: Parallel processing examples using Rayon
- `rayon_word`: Parallel word counting
- `rayon_hasher`: Multi-threaded file hashing
- `posix_pipe`: Inter-process communication using POSIX pipes

## Technology Stack

```mermaid
mindmap
    root((Rust Ecosystem))
        Async Runtime
            Tokio
            async/await
            Futures
        HTTP Clients
            reqwest
            hyper
        Parsing
            scraper
            select
            HTML parsing
        Parallelism
            Rayon
            crossbeam
            ThreadPool
        Serialization
            serde
            serde_json
        CLI
            clap
            structopt
        Error Handling
            anyhow
            thiserror
        Hashing
            sha2
            blake3
        Data Structures
            HashSet/HashMap
            Bloom Filters
            VecDeque
```

### Key Dependencies

| Crate | Purpose | Used In |
|-------|---------|---------|
| `tokio` | Async runtime | Web crawlers, network tools |
| `rayon` | Data parallelism | File processing, crawlers |
| `reqwest` | HTTP client | All web crawlers |
| `scraper` | HTML parsing | Web crawlers |
| `clap` | CLI parsing | Most utilities |
| `serde` | Serialization | PDF crawler, data output |
| `sha2` | Hashing | File verification |

## Getting Started

### Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Update Rust to latest stable
rustup update stable
```

### Building Projects

Each project can be built independently:

```bash
# Navigate to a project directory
cd web_crawler_pdf

# Build in release mode for optimal performance
cargo build --release

# Run the program
cargo run --release -- [arguments]
```

### Example: Running the PDF Web Crawler

```bash
cd web_crawler_pdf
cargo run --release -- \
    --url "https://example.com" \
    --depth 3 \
    --concurrency 10 \
    --output pdfs.json \
    --verify
```

### Example: Running File Search

```bash
# For standalone .rs files in examples/
rustc examples/file_searchv6.rs
./file_searchv6 /path/to/search "*.txt"
```

## Architecture Patterns

### Web Crawler Architecture

```mermaid
flowchart TD
    A[Start URL] --> B[URL Queue]
    B --> C{Concurrency Limiter<br/>Semaphore}
    C --> D[Fetch Page<br/>HTTP Client]
    D --> E{Parse Content}
    E --> F[Extract Links]
    E --> G[Extract Data]
    F --> H{Already Visited?<br/>HashSet/Bloom}
    H -->|No| B
    H -->|Yes| I[Skip]
    G --> J[Process Data]
    J --> K[Write to Output<br/>File/JSON]

    style A fill:#4CAF50
    style K fill:#2196F3
    style C fill:#FF9800
    style H fill:#9C27B0
```

### Concurrent Processing Pipeline

```mermaid
graph LR
    A[Input Data] --> B[Partition]
    B --> C1[Worker 1]
    B --> C2[Worker 2]
    B --> C3[Worker 3]
    B --> C4[Worker N]
    C1 --> D[Combine Results]
    C2 --> D
    C3 --> D
    C4 --> D
    D --> E[Output]

    style A fill:#E3F2FD
    style B fill:#BBDEFB
    style C1 fill:#90CAF9
    style C2 fill:#90CAF9
    style C3 fill:#90CAF9
    style C4 fill:#90CAF9
    style D fill:#64B5F6
    style E fill:#2196F3
```

## Development Workflow

```mermaid
stateDiagram-v2
    [*] --> Planning
    Planning --> Implementation
    Implementation --> Testing
    Testing --> CodeReview
    CodeReview --> Documentation
    Documentation --> Deployment

    Testing --> Implementation: Bugs Found
    CodeReview --> Implementation: Changes Requested

    Deployment --> [*]

    note right of Implementation
        Write Rust code
        Follow best practices
        Use cargo clippy
    end note

    note right of Testing
        cargo test
        cargo bench
        Integration tests
    end note
```

## Contributing

When contributing to this repository:

1. **Create a feature branch** with the format: `claude/feature-name-<session-id>`
2. **Follow Rust best practices**: Use `cargo fmt` and `cargo clippy`
3. **Add documentation**: Include README files for new projects
4. **Write tests**: Add unit and integration tests where applicable
5. **Update diagrams**: Keep architecture diagrams current

## License

See the [LICENSE](LICENSE) file for details.

## Resources

- [The Rust Programming Language Book](https://doc.rust-lang.org/book/)
- [Tokio Documentation](https://tokio.rs/)
- [Rayon Documentation](https://docs.rs/rayon/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)

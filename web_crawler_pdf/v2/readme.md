# pdf-crawler  
**A fast, parallel PDF discovery crawler with live, incremental JSON output.**

---

## What it does  
- Starts from any URL and crawls **up to a given depth**  
- Discovers links that **look like PDFs** (extensions, query args, path patterns)  
- **Verifies** each candidate via HTTP HEAD / partial GET (optional)  
- Streams results to a **timestamped JSON file** in the directory you run it from  
- Keeps the file **valid & incrementally updated** while crawling

---

## Quick start  

```bash
# clone or build from source
git clone <this-repo>
cd pdf-crawler
cargo build --release

# run
./target/release/pdf_crawler \
  --url https://example.org \
  --depth 3 \
  --concurrency 20 \
  --verify-pdfs true
```

Output file:  
`./pdfs_YYYY-MM-DD_HH-MM-SS.json`

---

## CLI options  

| flag            | default | meaning |
|-----------------|---------|---------|
| `--url`         | *req*   | start URL |
| `--depth`       | 5       | max crawl depth |
| `--concurrency` | 12      | max parallel requests |
| `--verify-pdfs` | true    | verify MIME / magic bytes |
| `--output`      | *auto*  | optional explicit path |

---

## JSON schema (top level)

```json
{
  "metadata": {
    "start_url": "...",
    "max_depth": 3,
    "total_pages_crawled": 42,
    "total_pdfs_found": 7,
    "verified_pdfs": 6,
    "failed_verifications": 1,
    "crawl_timestamp": "2025-09-03T14:23:55Z",
    "status": "completed"
  },
  "pdfs": [ … PdfInfo … ]
}
```

Each `PdfInfo`:

```json
{
  "url": "...",
  "source_page": "...",
  "depth": 2,
  "title": "Annual Report",
  "size_hint": "1.2 MB",
  "content_type": "application/pdf",
  "content_length": 1_234_567,
  "discovered_at": "2025-09-03T14:24:01Z",
  "verified": true
}
```

---

## Unique challenges we hit & lessons learned  

1. **Global `target-dir` override**  
   - Cause: `~/.cargo/config.toml` redirected `target/` to `/home/jeb/programs/target`.  
   - Symptom: binary never appeared in `./target/release`.  
   - Fix: run `cargo metadata --format-version=1 | jq -r '.target_directory'` to discover the real path, or **scope overrides locally** with `.cargo/config.toml`.

2. **Incremental JSON & atomic writes**  
   - Need to keep the file valid at all times → **write to temp file, then rename**.  
   - Avoids half-written JSON when the crawler is interrupted.

3. **Hard-coded vs. dynamic output path**  
   - Originally used `--output` but users forgot paths → switched to **timestamped file in CWD**.  
   - Still allow `--output` for scripting if provided.

4. **Telemetry vs. noise**  
   - Added millisecond-precision logging, but kept it concise for pipeline use.

---

## Development tips for next time  

- Always **check `cargo metadata`** when `target/` seems missing.  
- Use `lsof` or `strace` to verify incremental writes.  
- Keep `Clone` on all data structures used by writer threads.  
- Keep warnings low (`cargo clippy --fix`) to spot real issues quickly.

---

## License  
MIT – use freely.

# CLAUDE.md

## Project Overview

Axum web service (Rust) that scrapes and serves Dutch football league standings (Eredivisie and Keuken Kampioen Divisie) from official websites. Originally built for e-ink display integration (TRMNL devices).

## Quick Reference

```bash
# Build
cargo build

# Run (dev)
cargo run

# Run (release)
cargo build --release && ./target/release/eredivisie-standing

# Docker build and run
docker build -t eredivisie-standing .
docker run -p 8000:8000 -e API_KEY="your_api_key" eredivisie-standing
```

## Project Structure

```
├── Cargo.toml               # Dependencies and project metadata
├── src/
│   ├── main.rs              # Axum app setup, routes, state, background scheduler
│   ├── models.rs            # Team and StandingsData structs
│   └── scraper/
│       ├── mod.rs           # Shared utilities: safe_int(), clean_team_name(), fetch_html()
│       ├── eredivisie.rs    # Eredivisie scraper (eredivisie.nl)
│       └── kkd.rs           # KKD scraper (keukenkampioendivisie.nl)
├── Dockerfile               # Multi-stage Rust build
├── README.md                # User-facing documentation
└── .gitignore               # Rust/IDE ignores
```

## Architecture

- **`Team` / `StandingsData`** (`src/models.rs`) — Serde-serializable structs for standings data
- **Scraper modules** (`src/scraper/`) — Async scrapers using `reqwest` + `scraper` crate (CSS selectors). Each scraper tries multiple CSS selectors with fallbacks for resilience against website changes.
- **App state** (`src/main.rs`) — `Arc<RwLock<Option<StandingsData>>>` shared between Axum handlers and background updater
- **Background scheduler** — `tokio::spawn` with `tokio::time::interval` refreshes standings every 60 minutes
- **API endpoints** — `GET /` (health), `GET /standings`, `GET /kkd-standings`

## Key Conventions

- **Rust edition 2024**
- **Async throughout** — Tokio runtime, all I/O is async
- **Defensive HTML parsing** — multiple fallback CSS selectors per scraper
- **`safe_int()`** — converts potentially malformed strings to `i32`, returns 0 on failure
- **`clean_team_name()`** — normalizes scraped team names (collapses whitespace, strips asterisks)
- **Optional API key auth** — via `API_KEY` env var; if unset, endpoints are open
- **CORS** — permissive (all origins allowed)

## Environment Variables

| Variable  | Required | Description                                      |
|-----------|----------|--------------------------------------------------|
| `API_KEY` | No       | API authentication key (passed as query param)   |

## Dependencies

Core: `axum`, `tokio`, `reqwest`, `scraper`, `serde`/`serde_json`, `tower-http`, `tracing`, `chrono`, `regex`

## Testing

No test framework is currently configured. Manual testing is done by running the server and hitting endpoints with curl.

## Notes for AI Assistants

- The scrapers depend on the HTML structure of external websites; changes to those sites can break parsing
- When modifying scraper logic, preserve the fallback selector pattern for resilience
- The two scrapers have completely different parsing logic — they share only utility functions in `scraper/mod.rs`
- Use `i32` for numeric fields (goal difference and points can be negative)
- No linting or formatting tools are configured beyond `cargo fmt` / `cargo clippy`

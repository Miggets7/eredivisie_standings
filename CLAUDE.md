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
│       ├── mod.rs           # Shared helpers: fetch_html(), find_rows(), row_cells(), cell_text(), split_ints(), finish(), safe_int(), clean_team_name()
│       ├── eredivisie.rs    # Eredivisie scraper (eredivisie.nl, HTML table)
│       └── kkd.rs           # KKD scraper (keukenkampioendivisie.nl, __NEXT_DATA__ JSON with HTML table fallback)
├── tests/fixtures/          # Trimmed real standings pages used by the parser unit tests
├── Dockerfile               # Multi-stage build: cargo-chef dependency caching, mold linker
├── .dockerignore            # Allowlist: only Cargo.toml, Cargo.lock, src/ reach the build context (add new COPY sources here)
├── build_and_export.sh      # Builds linux/amd64 image, slims it with slimtoolkit, exports both as .tar
├── README.md                # User-facing documentation
└── .gitignore               # Rust/IDE ignores
```

## Architecture

- **`Team` / `StandingsData`** (`src/models.rs`) — Serde-serializable structs for standings data
- **Scraper modules** (`src/scraper/`) — Async scrapers using `reqwest` + `scraper` crate (CSS selectors). Each has `scrape_*` (fetch + `finish`) and a sync `parse(html)` that unit tests call directly. Each tries multiple CSS selectors with fallbacks for resilience against website changes.
  - Eredivisie: standings split over two `table.table` elements (1-9, 10-18); results column is `W | G | V` = won, drawn, lost.
  - KKD: primary source is `props.pageProps.rankingsData` in the `<script id="__NEXT_DATA__">` JSON; HTML table parsing is the fallback.
- **App state** (`src/main.rs`) — `Arc<RwLock<Option<StandingsData>>>` shared between Axum handlers and background updater
- **Background scheduler** — `tokio::spawn` with `tokio::time::interval` refreshes standings every 60 minutes (both leagues scraped concurrently); first tick runs immediately, so endpoints return 503 until the first scrape finishes
- **API endpoints** — `GET /` (health), `GET /standings` (`?league=kkd` switches league; missing or empty means Eredivisie so TRMNL installs without the league field keep working), `GET /kkd-standings`
- **Response extras** — each team carries `logo` (crest URL from the source site); the payload carries `season`, computed in `scraper::season()` (turns over on 1 July)
- **Auth** — `require_api_key` middleware applied with `route_layer` to every route; constant-time compare via `subtle`
- **Shutdown** — graceful on Ctrl-C / SIGTERM

## Key Conventions

- **Rust edition 2024**
- **Async throughout** — Tokio runtime, all I/O is async
- **Defensive HTML parsing** — multiple fallback CSS selectors per scraper
- **`safe_int()`** — converts potentially malformed strings to `i32`, returns 0 on failure
- **`clean_team_name()`** — normalizes scraped team names (collapses whitespace, strips `*` and `+` legend markers)
- **Optional API key auth** — via `API_KEY` env var (query param `api_key`); if unset or empty, endpoints are open
- **CORS** — permissive (all origins allowed)

## Environment Variables

| Variable  | Required | Description                                      |
|-----------|----------|--------------------------------------------------|
| `API_KEY` | No       | API authentication key (passed as query param)   |

## Dependencies

Core: `axum`, `tokio`, `reqwest`, `scraper`, `serde`/`serde_json`, `subtle`, `tower-http`, `tracing`, `chrono`

`tower-http` stays on 0.6 while `reqwest` depends on 0.6; bumping to 0.7 alone compiles two copies.

## Testing

`cargo test` runs unit tests for the shared scraper helpers and each scraper's `parse()` against trimmed real pages in `tests/fixtures/` (KKD: JSON path and HTML fallback must give identical teams). Endpoints are tested manually by running the server and hitting them with curl.

## Notes for AI Assistants

- The scrapers depend on external websites: Eredivisie on its HTML table, KKD on the `rankingsData` JSON shape first and its HTML table second. Site changes can break parsing; refresh `tests/fixtures/` from the live pages when they do
- When modifying scraper logic, preserve the fallback selector pattern for resilience
- The scrapers share the row/selector/cell helpers in `scraper/mod.rs`; column layouts and separators differ per league
- Use `i32` for numeric fields (goal difference and points can be negative)
- No linting or formatting tools are configured beyond `cargo fmt` / `cargo clippy`

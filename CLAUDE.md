# CLAUDE.md

## Project Overview

FastAPI web service that scrapes and serves Dutch football league standings (Eredivisie and Keuken Kampioen Divisie) from official websites. Originally built for e-ink display integration (TRMNL devices). All application code lives in a single file (`main.py`).

## Quick Reference

```bash
# Install dependencies (requires uv package manager)
uv sync

# Run dev server with auto-reload
uvicorn main:app --host 0.0.0.0 --port 8000 --reload

# Docker build and run
docker build -t eredivisie-standing .
docker run -p 8000:8000 -e API_KEY="your_api_key" eredivisie-standing
```

## Project Structure

```
├── main.py              # Entire application (FastAPI app, scrapers, endpoints)
├── pyproject.toml        # Dependencies and project metadata
├── Dockerfile            # Multi-stage Docker build
├── build_and_export.sh   # Docker image optimization script (uses slimtoolkit)
├── README.md             # User-facing documentation
└── .gitignore            # Standard Python/IDE ignores
```

## Architecture

The app is a single-file FastAPI application (`main.py`) with these key components:

- **`Team` dataclass** — Data model for a team's league position and stats (position, name, games, wins, draws, losses, goals for/against, goal difference, points)
- **`StandingsData` dataclass** — Wraps a list of `Team` objects with a last-updated timestamp
- **`EredivisieScraper`** — Async scraper for `eredivisie.nl/competitie/stand/`. Uses `aiohttp` + `BeautifulSoup4`. Implements async context manager pattern (`__aenter__`/`__aexit__`)
- **`KeukenKampioenDivisieScraper`** — Extends `EredivisieScraper` for `keukenkampioendivisie.nl/klassement`. Overrides parsing due to different HTML structure
- **APScheduler** — Refreshes standings every 60 minutes via background task
- **API endpoints** — `GET /` (health), `GET /standings`, `GET /kkd-standings`

## Key Conventions

- **Python 3.13+** required
- **Async throughout** — all I/O uses `async`/`await`
- **Defensive HTML parsing** — multiple fallback CSS selectors for robustness against website changes
- **`_safe_int()` helper** — converts strings to int with fallback to 0 for malformed data
- **`_clean_team_name()`** — normalizes scraped team name strings
- **Global singleton scrapers** — `scraper` and `kkd_scraper` instances are created at module level
- **Optional API key auth** — via `API_KEY` env var; if unset, endpoints are open
- **CORS** — enabled for all origins

## Environment Variables

| Variable  | Required | Description                                      |
|-----------|----------|--------------------------------------------------|
| `API_KEY` | No       | API authentication key (passed as query param)   |

## Dependencies

Core: `fastapi`, `uvicorn`, `aiohttp`, `beautifulsoup4`, `apscheduler`, `httpx`, `python-multipart`

All managed via `pyproject.toml` with `uv` as the package manager.

## Testing

No test framework is currently configured. Manual testing is done by running the dev server and hitting endpoints.

## Notes for AI Assistants

- All code is in `main.py` — there are no separate modules or packages
- The scrapers depend on the HTML structure of external websites; changes to those sites can break parsing
- When modifying scraper logic, preserve the fallback selector pattern for resilience
- The two scraper classes share a base but have different parsing implementations due to different source HTML
- No linting or formatting tools are configured

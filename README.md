# Eredivisie & Keuken Kampioen Divisie Standings

This application provides football standings data for the Dutch Eredivisie and Keuken Kampioen Divisie leagues. It scrapes the official websites and formats the data for display on e-ink screens.

## Features

- Real-time standings data for both Eredivisie and Keuken Kampioen Divisie
- Configurable number of teams to display

## Setup

### Prerequisites

- Latest stable Rust (the Docker build uses 1.100)
- Docker (optional for containerized deployment)

### Environment Variables

Set the following environment variables before running the application:

```
API_KEY="your_api_key"  # For API authentication (optional)
```

### Building

```bash
cargo build --release
```

### Running the Application

```bash
cargo run --release
```

The server starts on `http://0.0.0.0:8000`.

### Docker Deployment

```bash
docker build -t eredivisie-standing .
docker run -p 8000:8000 \
  -e API_KEY="your_api_key" \
  eredivisie-standing
```

### API Endpoints

- **GET /**: Health check endpoint
- **GET /standings**: Get current standings (JSON format), Eredivisie by default
- **GET /kkd-standings**: Get Keuken Kampioen Divisie standings (JSON format), same as `/standings?league=kkd`

Query parameters:
- `api_key` — API key for authentication, on all endpoints (required if `API_KEY` env var is set)
- `top` — Number of top teams to return, on `/standings` and `/kkd-standings` (e.g., `?top=5`)
- `league` — `eredivisie` or `kkd`, on `/standings`. Missing or empty means Eredivisie; any other value returns 400

Response: `standings` (one entry per team with position, name, games, wins, draws, losses, goals,
goal difference, points and `logo`, the club crest URL from the source site), `last_updated` and
`season` (e.g. `2026/2027`, turning over on 1 July). The season follows the date, not the data: until
the leagues publish the new table, the previous season's final standings carry the new label.

## TRMNL Recipe

The published recipe [Eredivisie & Keuken Kampioen Divisie](https://trmnl.com/recipes/124580) polls this
service. `trmnl/` holds a copy of its files in TRMNL's recipe format:

- `settings.yml` — strategy, polling URL, refresh interval, framework version and the form fields
  (League, Favourite club)
- `shared.liquid` — zones, matchday, favourite club and the table, legend and title bar templates
- `full.liquid`, `half_horizontal.liquid`, `half_vertical.liquid`, `quadrant.liquid` — one file per view

The recipe on TRMNL is the live version. After changing the markup or settings in the TRMNL dashboard,
export the files again and commit them here.

The polling URL in `settings.yml` contains `api_key=[set]`, the placeholder TRMNL exports in place of
the real key. When restoring from these files, re-enter the real key in the plugin settings, or the
recipe fails with 403.

The zone shading in `shared.liquid` follows the legends on eredivisie.nl and keukenkampioendivisie.nl.
The European places depend on the UEFA country ranking, so check them before each season.

## Development

### Adding New Features

1. Add data models in `src/models.rs`
2. Add scraper logic in `src/scraper/`
3. Add new endpoints in `src/main.rs`

### Testing

Unit tests:

```bash
cargo test
```

Manual check: run `cargo run` and use curl:

```bash
curl http://localhost:8000/
curl http://localhost:8000/standings
curl http://localhost:8000/kkd-standings?top=5
```

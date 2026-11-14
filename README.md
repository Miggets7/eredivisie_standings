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
- **GET /standings**: Get current Eredivisie standings (JSON format)
- **GET /kkd-standings**: Get Keuken Kampioen Divisie standings (JSON format)

Query parameters:
- `api_key` — API key for authentication, on all endpoints (required if `API_KEY` env var is set)
- `top` — Number of top teams to return, on `/standings` and `/kkd-standings` (e.g., `?top=5`)

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

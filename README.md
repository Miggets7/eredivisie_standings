# Eredivisie Standings 

This application provides football standings data for the Dutch Eredivisie and Keuken Kampioen Divisie leagues. It scrapes the official websites and formats the data for display on e-ink screens.

## Features

- Real-time standings data for both Eredivisie and Keuken Kampioen Divisie
- Configurable number of teams to display

## Setup

### Prerequisites

- Python 3.13 or higher
- Docker (optional for containerized deployment)

### Environment Variables

Set the following environment variables before running the application:

```
API_KEY="your_api_key"  # For API authentication
```

### Installation

1. Clone the repository
2. Install dependencies:

```bash
uv sync
```

### Running the Application

```bash
uvicorn main:app --host 0.0.0.0 --port 8000 --reload
```

### Docker Deployment

```bash
docker build -t eredivisie-standing .
docker run -p 8000:8000 \
  -e API_KEY="your_api_key" \
  eredivisie-standing
```

### API Endpoints

- **GET /**: Health check endpoint
- **GET /standings**: Get current standings (JSON format)
- **GET /kkd-standings**: Get Keuken Kampioen Divisie standings (JSON format)


## Development

### Adding New Features

1. Implement new endpoints in `main.py`

### Testing
Test the application using the `uvicorn` command.

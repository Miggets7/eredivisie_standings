mod models;
mod scraper;

use std::env;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use subtle::ConstantTimeEq;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

use models::StandingsData;
use scraper::eredivisie::scrape_eredivisie;
use scraper::kkd::scrape_kkd;

type Standings = Arc<RwLock<Option<StandingsData>>>;

#[derive(Clone)]
struct AppState {
    eredivisie: Standings,
    kkd: Standings,
    api_key: Option<Arc<str>>,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct ApiKeyQuery {
    api_key: Option<String>,
}

#[derive(Deserialize)]
struct StandingsQuery {
    top: Option<usize>,
}

/// Rejects requests without the configured API key. Open when no key is configured.
async fn require_api_key(
    State(state): State<AppState>,
    Query(query): Query<ApiKeyQuery>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(expected) = &state.api_key {
        let given = query.api_key.unwrap_or_default();
        if !bool::from(given.as_bytes().ct_eq(expected.as_bytes())) {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    Ok(next.run(request).await)
}

async fn root(State(state): State<AppState>) -> Json<Value> {
    let eredivisie = state.eredivisie.read().await;
    let kkd = state.kkd.read().await;

    Json(json!({
        "service": "Eredivisie & KKD TRMNL Service",
        "status": "running",
        "eredivisie_last_updated": eredivisie.as_ref().map(|d| &d.last_updated),
        "eredivisie_teams_count": eredivisie.as_ref().map_or(0, |d| d.teams.len()),
        "kkd_last_updated": kkd.as_ref().map(|d| &d.last_updated),
        "kkd_teams_count": kkd.as_ref().map_or(0, |d| d.teams.len()),
    }))
}

async fn standings_json(
    standings: &Standings,
    top: Option<usize>,
) -> Result<Json<Value>, StatusCode> {
    let data = standings.read().await;
    let data = data.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let count = top.map_or(data.teams.len(), |n| n.min(data.teams.len()));

    Ok(Json(json!({
        "standings": &data.teams[..count],
        "last_updated": data.last_updated,
    })))
}

async fn get_standings(
    State(state): State<AppState>,
    Query(params): Query<StandingsQuery>,
) -> Result<Json<Value>, StatusCode> {
    standings_json(&state.eredivisie, params.top).await
}

async fn get_kkd_standings(
    State(state): State<AppState>,
    Query(params): Query<StandingsQuery>,
) -> Result<Json<Value>, StatusCode> {
    standings_json(&state.kkd, params.top).await
}

async fn update_league(league: &str, standings: &Standings, scraped: Option<StandingsData>) {
    match scraped {
        Some(data) => {
            *standings.write().await = Some(data);
            info!("{league} standings updated");
        }
        None => error!("Failed to update {league} standings, keeping previous data"),
    }
}

async fn update_standings(state: &AppState) {
    // Each league is stored as soon as its own scrape finishes, so a slow site
    // does not delay the other league.
    tokio::join!(
        async {
            let data = scrape_eredivisie(&state.client).await;
            update_league("Eredivisie", &state.eredivisie, data).await;
        },
        async {
            let data = scrape_kkd(&state.client).await;
            update_league("KKD", &state.kkd, data).await;
        },
    );
}

/// Resolves on Ctrl-C or SIGTERM, so `docker stop` shuts down without waiting for SIGKILL.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl-C handler");
    };
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    info!("Shutting down");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_ansi(false)
        .init();

    let api_key = env::var("API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .map(Arc::from);
    if api_key.is_none() {
        warn!(
            "API_KEY environment variable not set. API will be accessible without authentication."
        );
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    let state = AppState {
        eredivisie: Standings::default(),
        kkd: Standings::default(),
        api_key,
        client,
    };

    // Refresh every 60 minutes. The first tick fires immediately, so the initial
    // fetch runs while the server is already accepting requests (503 until loaded).
    let scheduler_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            update_standings(&scheduler_state).await;
        }
    });

    let app = Router::new()
        .route("/", get(root))
        .route("/standings", get(get_standings))
        .route("/kkd-standings", get(get_kkd_standings))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_api_key,
        ))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .expect("Failed to bind to port 8000");

    info!("Service started on port 8000");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");
}

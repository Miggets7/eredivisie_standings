mod models;
mod scraper;

use std::env;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

use models::StandingsData;
use scraper::eredivisie::scrape_eredivisie;
use scraper::kkd::scrape_kkd;

#[derive(Clone)]
struct AppState {
    eredivisie: Arc<RwLock<Option<StandingsData>>>,
    kkd: Arc<RwLock<Option<StandingsData>>>,
    api_key: Option<String>,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct StandingsQuery {
    api_key: Option<String>,
    top: Option<usize>,
}

async fn verify_api_key(state: &AppState, query_key: &Option<String>) -> Result<(), StatusCode> {
    match &state.api_key {
        None => Ok(()),
        Some(expected) => match query_key {
            Some(key) if key == expected => Ok(()),
            _ => Err(StatusCode::FORBIDDEN),
        },
    }
}

async fn root(
    State(state): State<AppState>,
    Query(params): Query<StandingsQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    verify_api_key(&state, &params.api_key).await?;

    let eredivisie = state.eredivisie.read().await;
    let kkd = state.kkd.read().await;

    Ok(Json(serde_json::json!({
        "service": "Eredivisie & KKD TRMNL Service",
        "status": "running",
        "eredivisie_last_updated": eredivisie.as_ref().map(|d| &d.last_updated),
        "eredivisie_teams_count": eredivisie.as_ref().map_or(0, |d| d.teams.len()),
        "kkd_last_updated": kkd.as_ref().map(|d| &d.last_updated),
        "kkd_teams_count": kkd.as_ref().map_or(0, |d| d.teams.len()),
    })))
}

async fn get_standings(
    State(state): State<AppState>,
    Query(params): Query<StandingsQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    verify_api_key(&state, &params.api_key).await?;

    let data = state.eredivisie.read().await;
    let data = data.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let teams = match params.top {
        Some(n) => &data.teams[..n.min(data.teams.len())],
        None => &data.teams,
    };

    Ok(Json(serde_json::json!({
        "standings": teams,
        "last_updated": data.last_updated,
    })))
}

async fn get_kkd_standings(
    State(state): State<AppState>,
    Query(params): Query<StandingsQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    verify_api_key(&state, &params.api_key).await?;

    let data = state.kkd.read().await;
    let data = data.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let teams = match params.top {
        Some(n) => &data.teams[..n.min(data.teams.len())],
        None => &data.teams,
    };

    Ok(Json(serde_json::json!({
        "standings": teams,
        "last_updated": data.last_updated,
    })))
}

async fn update_standings(state: &AppState) {
    info!("Updating Eredivisie standings...");
    match scrape_eredivisie(&state.client).await {
        Some(data) => {
            *state.eredivisie.write().await = Some(data);
            info!("Eredivisie standings updated successfully");
        }
        None => error!("Failed to update Eredivisie standings"),
    }

    info!("Updating Keuken Kampioen Divisie standings...");
    match scrape_kkd(&state.client).await {
        Some(data) => {
            *state.kkd.write().await = Some(data);
            info!("KKD standings updated successfully");
        }
        None => error!("Failed to update KKD standings"),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with_ansi(false)
        .init();

    let api_key = env::var("API_KEY").ok().filter(|k| !k.is_empty());
    if api_key.is_none() {
        tracing::warn!(
            "API_KEY environment variable not set. API will be accessible without authentication."
        );
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    let state = AppState {
        eredivisie: Arc::new(RwLock::new(None)),
        kkd: Arc::new(RwLock::new(None)),
        api_key,
        client,
    };

    // Initial fetch
    info!("Starting Eredivisie TRMNL Service...");
    update_standings(&state).await;

    // Background scheduler: update every 60 minutes
    let scheduler_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        interval.tick().await; // skip the immediate first tick (already fetched above)
        loop {
            interval.tick().await;
            update_standings(&scheduler_state).await;
        }
    });

    let app = Router::new()
        .route("/", get(root))
        .route("/standings", get(get_standings))
        .route("/kkd-standings", get(get_kkd_standings))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .expect("Failed to bind to port 8000");

    info!("Service started successfully on port 8000");
    axum::serve(listener, app).await.unwrap();
}

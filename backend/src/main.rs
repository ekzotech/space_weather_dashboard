mod aurora;
mod cache;
mod domain;
mod feeds;
mod nasa;
mod noaa;

use std::{env, sync::Arc};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use aurora::valid_coordinates;
use domain::{Sample, Storm, rank_storms};
use feeds::{FeedError, Feeds};

type AppState = State<Arc<Feeds>>;
type ApiError = (StatusCode, Json<Value>);

fn api_error(status: StatusCode, code: &str) -> ApiError {
    (status, Json(json!({ "error": code })))
}

fn upstream(source: &str, error: FeedError) -> ApiError {
    match error {
        FeedError::Http(error) => eprintln!("{source}: {error}"),
        FeedError::InvalidJson(error) => eprintln!("{source}: {error}"),
        FeedError::TooLarge => eprintln!("{source}: response exceeded the size limit"),
    }
    api_error(StatusCode::BAD_GATEWAY, "upstream_unavailable")
}

async fn kp(State(feeds): AppState) -> Result<Json<Vec<Sample>>, ApiError> {
    feeds
        .kp()
        .await
        .map(|points| Json(points.as_ref().clone()))
        .map_err(|e| upstream("NOAA Kp", e))
}

async fn forecast(State(feeds): AppState) -> Result<Json<Vec<Sample>>, ApiError> {
    let now = Utc::now();
    let points = feeds
        .forecast()
        .await
        .map_err(|e| upstream("NOAA forecast", e))?;
    Ok(Json(
        points
            .iter()
            .filter(|point| point.time + Duration::hours(3) > now)
            .cloned()
            .collect(),
    ))
}

async fn xray(State(feeds): AppState) -> Result<Json<Vec<Sample>>, ApiError> {
    feeds
        .xray()
        .await
        .map(|points| Json(points.as_ref().clone()))
        .map_err(|e| upstream("NOAA GOES", e))
}

#[derive(Deserialize)]
struct StormFilter {
    min_kp: Option<f64>,
}

async fn storms(
    State(feeds): AppState,
    Query(filter): Query<StormFilter>,
) -> Result<Json<Vec<Storm>>, ApiError> {
    let min = filter.min_kp.unwrap_or(5.0);
    if !(5.0..=9.0).contains(&min) {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid_min_kp"));
    }
    let rows = feeds
        .storms()
        .await
        .map_err(|e| upstream("NASA DONKI", e))?;
    let mut storms = rows.iter().filter_map(nasa::DonkiStorm::as_storm).collect();
    rank_storms(&mut storms, min);
    Ok(Json(storms))
}

#[derive(Deserialize)]
struct Coordinates {
    lat: f64,
    lon: f64,
}

#[derive(Serialize)]
struct AuroraEstimate {
    probability: Option<i16>,
    observation_time: DateTime<Utc>,
    forecast_time: DateTime<Utc>,
    stale: bool,
}

async fn aurora(
    State(feeds): AppState,
    Query(point): Query<Coordinates>,
) -> Result<Json<AuroraEstimate>, ApiError> {
    if !valid_coordinates(point.lat, point.lon) {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid_coordinates"));
    }
    let grid = feeds
        .aurora()
        .await
        .map_err(|e| upstream("NOAA OVATION", e))?;
    let stale = Utc::now() - grid.observation_time > Duration::hours(1);
    Ok(Json(AuroraEstimate {
        probability: if stale {
            None
        } else {
            grid.estimate(point.lat, point.lon)
        },
        observation_time: grid.observation_time,
        forecast_time: grid.forecast_time,
        stale,
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let app = Router::new()
        .route(
            "/api/health",
            get(|| async { Json(json!({ "status": "ok" })) }),
        )
        .route("/api/kp", get(kp))
        .route("/api/forecast", get(forecast))
        .route("/api/xray", get(xray))
        .route("/api/storms", get(storms))
        .route("/api/aurora", get(aurora))
        .with_state(Arc::new(Feeds::new()?));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Space Weather API at http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

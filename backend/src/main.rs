mod aurora;
mod cache;
mod domain;
mod feeds;
mod nasa;
mod noaa;

use std::{env, sync::Arc};

use axum::{
    Json, Router,
    extract::{Query, State, rejection::QueryRejection},
    http::{HeaderMap, HeaderValue, StatusCode},
    routing::get,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use aurora::valid_coordinates;
use cache::Cached;
use domain::{Sample, Storm, rank_storms};
use feeds::{FeedError, Feeds};

type AppState = State<Arc<Feeds>>;
type ApiResponse<T> = (HeaderMap, Json<T>);

fn feed_headers<T>(cache: &Cached<T>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Ok(received) = HeaderValue::from_str(&cache.received_at.to_rfc3339()) {
        headers.insert("x-feed-received-at", received);
    }
    headers.insert(
        "x-feed-stale",
        HeaderValue::from_static(if cache.stale { "true" } else { "false" }),
    );
    headers
}

type ApiError = (StatusCode, Json<Value>);

fn api_error(status: StatusCode, code: &str) -> ApiError {
    (status, Json(json!({ "error": code })))
}

fn upstream(source: &str, error: FeedError) -> ApiError {
    let (status, code) = match error {
        FeedError::Http(error) => {
            eprintln!("{source}: {error}");
            match error.status() {
                Some(StatusCode::TOO_MANY_REQUESTS) => {
                    (StatusCode::SERVICE_UNAVAILABLE, "upstream_rate_limited")
                }
                _ if error.is_timeout() => (StatusCode::GATEWAY_TIMEOUT, "upstream_timeout"),
                _ => (StatusCode::BAD_GATEWAY, "upstream_unavailable"),
            }
        }
        FeedError::InvalidJson(error) => {
            eprintln!("{source}: {error}");
            (StatusCode::BAD_GATEWAY, "upstream_invalid_json")
        }
        FeedError::RateLimited => (StatusCode::SERVICE_UNAVAILABLE, "upstream_rate_limited"),
        FeedError::TooLarge => (StatusCode::BAD_GATEWAY, "upstream_response_too_large"),
    };
    api_error(status, code)
}

async fn kp(State(feeds): AppState) -> Result<ApiResponse<Vec<Sample>>, ApiError> {
    feeds
        .kp()
        .await
        .map(|points| (feed_headers(&points), Json(points.value.as_ref().clone())))
        .map_err(|e| upstream("NOAA Kp", e))
}

async fn forecast(State(feeds): AppState) -> Result<ApiResponse<Vec<Sample>>, ApiError> {
    let now = Utc::now();
    let points = feeds
        .forecast()
        .await
        .map_err(|e| upstream("NOAA forecast", e))?;
    Ok((
        feed_headers(&points),
        Json(
            points
                .value
                .iter()
                .filter(|point| point.time + Duration::hours(3) > now)
                .cloned()
                .collect(),
        ),
    ))
}

async fn xray(State(feeds): AppState) -> Result<ApiResponse<Vec<Sample>>, ApiError> {
    feeds
        .xray()
        .await
        .map(|points| (feed_headers(&points), Json(points.value.as_ref().clone())))
        .map_err(|e| upstream("NOAA GOES", e))
}

#[derive(Deserialize)]
struct StormFilter {
    min_kp: Option<f64>,
}

async fn storms(
    State(feeds): AppState,
    query: Result<Query<StormFilter>, QueryRejection>,
) -> Result<ApiResponse<Vec<Storm>>, ApiError> {
    let Query(filter) = query.map_err(|_| api_error(StatusCode::BAD_REQUEST, "invalid_query"))?;
    let min = filter.min_kp.unwrap_or(5.0);
    if !(5.0..=9.0).contains(&min) {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid_min_kp"));
    }
    let rows = feeds
        .storms()
        .await
        .map_err(|e| upstream("NASA DONKI", e))?;
    let mut storms = rows
        .value
        .iter()
        .filter_map(nasa::DonkiStorm::as_storm)
        .collect();
    rank_storms(&mut storms, min);
    Ok((feed_headers(&rows), Json(storms)))
}

#[derive(Deserialize)]
struct Coordinates {
    lat: f64,
    lon: f64,
}

#[derive(Serialize)]
struct AuroraEstimate {
    probability: Option<i16>,
    grid_distance_km: Option<f64>,
    observation_time: DateTime<Utc>,
    forecast_time: DateTime<Utc>,
    stale: bool,
}

async fn aurora(
    State(feeds): AppState,
    query: Result<Query<Coordinates>, QueryRejection>,
) -> Result<ApiResponse<AuroraEstimate>, ApiError> {
    let Query(point) = query.map_err(|_| api_error(StatusCode::BAD_REQUEST, "invalid_query"))?;
    if !valid_coordinates(point.lat, point.lon) {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid_coordinates"));
    }
    let grid = feeds
        .aurora()
        .await
        .map_err(|e| upstream("NOAA OVATION", e))?;
    let stale = Utc::now() - grid.value.observation_time > Duration::hours(1);
    let probability = (!stale)
        .then(|| grid.value.estimate(point.lat, point.lon))
        .flatten();
    Ok((
        feed_headers(&grid),
        Json(AuroraEstimate {
            probability,
            grid_distance_km: probability
                .map(|_| aurora::AuroraGrid::distance_to_cell_km(point.lat, point.lon)),
            observation_time: grid.value.observation_time,
            forecast_time: grid.value.forecast_time,
            stale,
        }),
    ))
}

fn router(feeds: Feeds) -> Router {
    Router::new()
        .route(
            "/api/health",
            get(|| async { Json(json!({ "status": "ok" })) }),
        )
        .route("/api/kp", get(kp))
        .route("/api/forecast", get(forecast))
        .route("/api/xray", get(xray))
        .route("/api/storms", get(storms))
        .route("/api/aurora", get(aurora))
        .with_state(Arc::new(feeds))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let app = router(Feeds::new()?);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Space Weather API at http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod http_tests;

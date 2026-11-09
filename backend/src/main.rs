mod aurora;
mod cache;
mod domain;
mod feeds;
mod metrics;
mod nasa;
mod noaa;
mod sun;

use std::{env, path::Path, sync::Arc};

use axum::{
    Json, Router,
    extract::{Query, Request, State, rejection::QueryRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use chrono::{DateTime, Days, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tower_http::services::ServeDir;

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
    let (status, code, upstream_status) = match &error {
        FeedError::Http(error) if error.is_timeout() => {
            (StatusCode::GATEWAY_TIMEOUT, "upstream_timeout", 0)
        }
        FeedError::Http(error) => (
            StatusCode::BAD_GATEWAY,
            "upstream_unavailable",
            error.status().map_or(0, |status| status.as_u16()),
        ),
        FeedError::InvalidJson(_) => (StatusCode::BAD_GATEWAY, "upstream_invalid_json", 0),
        FeedError::RateLimited => (
            StatusCode::SERVICE_UNAVAILABLE,
            "upstream_rate_limited",
            429,
        ),
        FeedError::TooLarge => (StatusCode::BAD_GATEWAY, "upstream_response_too_large", 0),
    };
    let json_line = if let FeedError::InvalidJson(error) = &error {
        error.line()
    } else {
        0
    };
    tracing::warn!(
        source,
        upstream_status,
        status = status.as_u16(),
        code,
        json_line,
        "upstream feed failure"
    );
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
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
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
    let today = Utc::now().date_naive();
    let range = match (filter.start, filter.end) {
        (None, None) => None,
        (Some(start), Some(end))
            if start <= end && end <= today && end - start < Duration::days(30) =>
        {
            Some((start, end))
        }
        _ => return Err(api_error(StatusCode::BAD_REQUEST, "invalid_date_range")),
    };
    let rows = match range {
        None => feeds.storms(today - Days::new(29), today).await,
        Some((start, end)) => feeds.storms_between(start, end).await,
    }
    .map_err(|e| upstream("NASA DONKI", e))?;
    let mut storms = rows
        .value
        .iter()
        .filter_map(nasa::DonkiStorm::as_storm)
        .filter(|storm| {
            range.is_none_or(|(start, end)| (start..=end).contains(&storm.start.date_naive()))
        })
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
    sun_elevation_deg: Option<f64>,
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
            sun_elevation_deg: (!stale)
                .then(|| sun::elevation(point.lat, point.lon, grid.value.forecast_time)),
            observation_time: grid.value.observation_time,
            forecast_time: grid.value.forecast_time,
            stale,
        }),
    ))
}

async fn cloud(
    State(feeds): AppState,
    query: Result<Query<Coordinates>, QueryRejection>,
) -> Result<Json<feeds::CloudCover>, ApiError> {
    let Query(point) = query.map_err(|_| api_error(StatusCode::BAD_REQUEST, "invalid_query"))?;
    if !valid_coordinates(point.lat, point.lon) {
        return Err(api_error(StatusCode::BAD_REQUEST, "invalid_coordinates"));
    }
    feeds
        .cloud_cover(point.lat, point.lon)
        .await
        .map(Json)
        .map_err(|e| upstream("Open-Meteo", e))
}

fn router(feeds: Feeds) -> Router {
    Router::new()
        .route(
            "/api/health",
            get(|| async { Json(json!({ "status": "ok" })) }),
        )
        .route(
            "/api/metrics",
            get(|State(feeds): AppState| async move {
                (
                    [(
                        header::CONTENT_TYPE,
                        "text/plain; version=0.0.4; charset=utf-8",
                    )],
                    feeds.metrics(),
                )
            }),
        )
        .route("/api/kp", get(kp))
        .route("/api/forecast", get(forecast))
        .route("/api/xray", get(xray))
        .route("/api/storms", get(storms))
        .route("/api/aurora", get(aurora))
        .route("/api/cloud", get(cloud))
        .with_state(Arc::new(feeds))
}

fn with_static(app: Router, directory: impl AsRef<Path>) -> Router {
    app.fallback_service(ServeDir::new(directory))
        .layer(middleware::from_fn(static_headers))
}

async fn static_headers(request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let is_api = path == "/api" || path.starts_with("/api/");
    let is_asset = path.starts_with("/assets/");
    let mut response = next.run(request).await;
    if !is_api && (response.status().is_success() || response.status() == StatusCode::NOT_MODIFIED)
    {
        let headers = response.headers_mut();
        headers.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(if is_asset {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            }),
        );
    }
    response
}

async fn shutdown_signal() {
    let interrupt = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            eprintln!("failed to listen for Ctrl+C: {error}");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => {
                eprintln!("failed to listen for SIGTERM: {error}");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = interrupt => {},
        () = terminate => {},
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().json().init();
    let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let mut app = router(Feeds::new()?);
    if let Some(directory) = env::var_os("STATIC_DIR") {
        if !Path::new(&directory).join("index.html").is_file() {
            return Err("STATIC_DIR must contain the built index.html".into());
        }
        app = with_static(app, directory);
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(address = %listener.local_addr()?, "space weather API listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

#[cfg(test)]
mod http_tests;

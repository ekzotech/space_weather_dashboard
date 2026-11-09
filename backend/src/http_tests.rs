use super::*;

async fn start(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (base, server)
}

async fn request(app: Router, path: &str) -> (StatusCode, Value) {
    let (base, server) = start(app).await;
    let response = reqwest::get(format!("{base}{path}")).await.unwrap();
    let status = response.status();
    let body = response.json().await.unwrap();
    server.abort();
    (status, body)
}

#[tokio::test]
async fn health_reports_process_readiness() {
    assert_eq!(
        request(router(Feeds::new().unwrap()), "/api/health").await,
        (StatusCode::OK, json!({"status": "ok"}))
    );
}

#[tokio::test]
async fn rejects_kp_outside_the_storm_scale() {
    for (value, error) in [
        ("4.99", "invalid_min_kp"),
        ("9.01", "invalid_min_kp"),
        ("NaN", "invalid_min_kp"),
        ("text", "invalid_query"),
    ] {
        assert_eq!(
            request(
                router(Feeds::new().unwrap()),
                &format!("/api/storms?min_kp={value}")
            )
            .await,
            (StatusCode::BAD_REQUEST, json!({"error": error}))
        );
    }
}

#[tokio::test]
async fn rejects_invalid_coordinates() {
    for (point, error) in [
        ("lat=91&lon=0", "invalid_coordinates"),
        ("lat=0&lon=-181", "invalid_coordinates"),
        ("lat=NaN&lon=0", "invalid_coordinates"),
        ("lat=65", "invalid_query"),
    ] {
        assert_eq!(
            request(
                router(Feeds::new().unwrap()),
                &format!("/api/aurora?{point}")
            )
            .await,
            (StatusCode::BAD_REQUEST, json!({"error": error}))
        );
    }
}

#[tokio::test]
async fn classifies_unavailable_nasa() {
    let upstream = Router::new().route("/gst", get(|| async { StatusCode::BAD_GATEWAY }));
    let (base, provider) = start(upstream).await;
    assert_eq!(
        request(
            router(Feeds::new().unwrap().test_sources(&base)),
            "/api/storms"
        )
        .await,
        (
            StatusCode::BAD_GATEWAY,
            json!({"error":"upstream_unavailable"})
        )
    );
    provider.abort();
}

#[tokio::test]
async fn noaa_remains_available_after_a_nasa_failure() {
    let upstream = Router::new()
        .route("/gst", get(|| async { StatusCode::BAD_GATEWAY }))
        .route(
            "/products/noaa-planetary-k-index.json",
            get(|| async { Json(json!([{"time_tag":"2026-01-01T00:00:00","Kp":6.33}])) }),
        );
    let (base, provider) = start(upstream).await;
    let app = router(Feeds::new().unwrap().test_sources(&base));
    assert_eq!(
        request(app.clone(), "/api/storms").await.0,
        StatusCode::BAD_GATEWAY
    );
    assert_eq!(
        request(app, "/api/kp").await,
        (
            StatusCode::OK,
            json!([{"time":"2026-01-01T00:00:00Z","value":6.33}])
        )
    );
    provider.abort();
}

#[tokio::test]
async fn suppresses_expired_ovation_probability() {
    let upstream = Router::new().route(
        "/json/ovation_aurora_latest.json",
        get(|| async {
            Json(json!({
                "Observation Time":"2020-01-01T00:00:00Z", "Forecast Time":"2020-01-01T00:30:00Z",
                "coordinates":[[0,65,70]]
            }))
        }),
    );
    let (base, provider) = start(upstream).await;
    let (status, result) = request(
        router(Feeds::new().unwrap().test_sources(&base)),
        "/api/aurora?lat=65&lon=0",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["stale"], true);
    assert_eq!(result["probability"], Value::Null);
    provider.abort();
}

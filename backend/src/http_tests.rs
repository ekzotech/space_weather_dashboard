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

#[tokio::test]
async fn openapi_examples_match_live_response_shapes() {
    let spec: Value = serde_json::from_str(include_str!("../../docs/openapi.json")).unwrap();
    let upstream = Router::new()
        .route("/products/noaa-planetary-k-index.json", get(|| async {
            Json(json!([{"time_tag":"2026-01-01T00:00:00","Kp":6.33}]))
        }))
        .route("/json/goes/primary/xrays-1-day.json", get(|| async {
            Json(json!([{"time_tag":"2026-01-01T00:00:00Z","energy":"0.1-0.8nm","flux":0.000002}]))
        }))
        .route("/json/ovation_aurora_latest.json", get(|| async {
            Json(json!({"Observation Time":"2020-01-01T00:00:00Z","Forecast Time":"2020-01-01T00:30:00Z","coordinates":[[0,65,70]]}))
        }))
        .route("/gst", get(|| async {
            Json(json!([{"gstID":"GST-001","startTime":"2026-01-01T00:00Z","allKpIndex":[{"kpIndex":6.33}],"linkedEvents":[{"activityID":"CME-001"}]}]))
        }));
    let (base, provider) = start(upstream).await;
    let app = router(Feeds::new().unwrap().test_sources(&base));
    for (path, query) in [
        ("/api/health", ""),
        ("/api/kp", ""),
        ("/api/xray", ""),
        ("/api/storms", ""),
        ("/api/aurora", "?lat=65&lon=0"),
    ] {
        let (status, body) = request(app.clone(), &format!("{path}{query}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body,
            spec["paths"][path]["get"]["responses"]["200"]["content"]["application/json"]["example"],
            "{path}"
        );
    }
    provider.abort();
}

#[tokio::test]
async fn validates_archive_bounds_and_filters_inclusive_utc_dates() {
    let upstream = Router::new().route(
        "/gst",
        get(|| async {
            Json(json!([
                {"gstID":"outside","startTime":"2023-12-31T23:59Z","allKpIndex":[{"kpIndex":7}]},
                {"gstID":"inside","startTime":"2024-01-01T23:59Z","allKpIndex":[{"kpIndex":6}]},
                {"gstID":"tomorrow","startTime":"2024-01-02T00:00Z","allKpIndex":[{"kpIndex":8}]}
            ]))
        }),
    );
    let (base, provider) = start(upstream).await;
    let app = router(Feeds::new().unwrap().test_sources(&base));
    let (status, body) = request(app.clone(), "/api/storms?start=2024-01-01&end=2024-01-01").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["id"], "inside");
    assert_eq!(
        request(app.clone(), "/api/storms?start=2024-01-03&end=2024-01-04").await,
        (StatusCode::OK, json!([]))
    );
    for query in [
        "start=2024-01-01",
        "start=2024-01-01&end=2024-01-31",
        "start=2024-01-02&end=2024-01-01",
    ] {
        assert_eq!(
            request(app.clone(), &format!("/api/storms?{query}")).await,
            (
                StatusCode::BAD_REQUEST,
                json!({"error":"invalid_date_range"})
            )
        );
    }
    provider.abort();
}

#[tokio::test]
async fn serves_html_and_versioned_assets_with_safe_cache_headers() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/static");
    let (base, server) = start(with_static(router(Feeds::new().unwrap()), directory)).await;
    let html = reqwest::get(format!("{base}/")).await.unwrap();
    assert_eq!(html.status(), StatusCode::OK);
    assert_eq!(html.headers()[header::CACHE_CONTROL], "no-cache");
    assert_eq!(html.headers()[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert!(
        html.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(html.text().await.unwrap().contains("Dashboard fixture"));
    let css = reqwest::get(format!("{base}/assets/app-test.css"))
        .await
        .unwrap();
    assert_eq!(css.status(), StatusCode::OK);
    assert_eq!(
        css.headers()[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    assert!(
        css.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/css")
    );
    let missing = reqwest::get(format!("{base}/assets/missing.css"))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert!(missing.headers().get(header::CACHE_CONTROL).is_none());
    server.abort();
}

#[tokio::test]
async fn reports_upstream_attempts_and_failures_by_provider() {
    let (provider_url, provider) =
        start(Router::new().route("/gst", get(|| async { StatusCode::BAD_GATEWAY }))).await;
    let (base, server) = start(router(Feeds::new().unwrap().test_sources(&provider_url))).await;
    assert_eq!(
        reqwest::get(format!("{base}/api/storms"))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_GATEWAY
    );
    let response = reqwest::get(format!("{base}/api/metrics")).await.unwrap();
    let body = response.text().await.unwrap();
    assert!(body.contains("space_weather_upstream_requests_total{provider=\"nasa\"} 1"));
    assert!(body.contains("space_weather_upstream_errors_total{provider=\"nasa\"} 1"));
    assert!(body.contains("space_weather_upstream_requests_total{provider=\"noaa\"} 0"));
    server.abort();
    provider.abort();
}

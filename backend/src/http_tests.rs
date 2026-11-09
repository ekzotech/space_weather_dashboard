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

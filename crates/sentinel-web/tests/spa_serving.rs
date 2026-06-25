use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn spa_serving_api_precedence_and_fallback() {
    // Create a temp dir with a stub index.html
    let dir = tempfile::tempdir().unwrap();
    let index_path = dir.path().join("index.html");
    std::fs::write(&index_path, "<!DOCTYPE html><html><body>SPA</body></html>").unwrap();

    let state = sentinel_web::state::test_state_with_password("x");
    let api_router = sentinel_web::app::build_router(state);

    let spa = tower_http::services::ServeDir::new(dir.path())
        .fallback(tower_http::services::ServeFile::new(&index_path));
    let app = api_router.fallback_service(spa);

    // /api/health must return JSON with "status" field
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");

    // Unknown SPA route must return index.html (200, HTML)
    let res = app
        .oneshot(Request::builder().uri("/dashboard/x").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("<html>"), "expected HTML body, got: {body_str}");
}

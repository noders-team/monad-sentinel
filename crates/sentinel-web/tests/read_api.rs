use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use sentinel_web::state::{test_state_with_password, AppState};
use sentinel_web::probe::{ServiceProbe, VersionProbe, LogReader};
use sentinel_web::state::AlertRecord;
use std::sync::Arc;

// ---- Fake probe implementations used in tests ----

struct FakeServiceProbe;
impl ServiceProbe for FakeServiceProbe {
    fn is_active(&self, _unit: &str) -> bool { true }
}

struct FakeVersionProbe;
impl VersionProbe for FakeVersionProbe {
    fn version(&self, binary: &str) -> Option<String> {
        if binary.contains("monad-node") { Some("v0.14.5".into()) }
        else { Some("v0.1.0".into()) }
    }
}

struct FakeLogReader;
impl LogReader for FakeLogReader {
    fn tail(&self, _unit: &str, _lines: usize) -> Vec<String> {
        vec!["log line 1".into(), "log line 2".into()]
    }
}

fn fake_state() -> AppState {
    let mut st = test_state_with_password("hunter2");
    st.svc_probe = Arc::new(FakeServiceProbe);
    st.ver_probe = Arc::new(FakeVersionProbe);
    st.logs = Arc::new(FakeLogReader);
    st
}

fn req(method: &str, uri: &str, body: &str, cookie: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    b.body(Body::from(body.to_string())).unwrap()
}

/// Log in as admin and return the "sid=<token>" cookie string.
async fn login(app: &axum::Router) -> String {
    let res = app.clone()
        .oneshot(req("POST", "/api/auth/login",
            "{\"username\":\"admin\",\"password\":\"hunter2\"}", None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cookies: Vec<String> = res.headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect();
    let sid_header = cookies.iter().find(|c| c.contains("sid=")).expect("sid cookie");
    sid_header.split(';').next().unwrap().to_string()
}

// ---- Tests ----

#[tokio::test]
async fn status_without_session_is_401() {
    let app = sentinel_web::app::build_router(fake_state());
    let res = app.oneshot(req("GET", "/api/status", "", None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn status_lists_services_with_probes() {
    let app = sentinel_web::app::build_router(fake_state());
    let sid = login(&app).await;

    let res = app.clone()
        .oneshot(req("GET", "/api/status", "", Some(&sid)))
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().expect("should be array");
    // default config has 3 services
    assert_eq!(arr.len(), 3, "expected 3 services");

    // All should be active (FakeServiceProbe returns true)
    for svc in arr {
        assert_eq!(svc["active"], true, "all services should be active");
    }

    // BFT service should have version "v0.14.5" (FakeVersionProbe returns v0.14.5 for monad-node)
    let bft = arr.iter().find(|s| s["unit"] == "monad-bft.service").expect("bft service");
    assert_eq!(bft["version"], "v0.14.5", "bft version should be v0.14.5");
    assert_eq!(bft["name"], "BFT");
    assert_eq!(bft["kind"], "bft");
}

#[tokio::test]
async fn logs_rejects_unlisted_unit() {
    let app = sentinel_web::app::build_router(fake_state());
    let sid = login(&app).await;

    let res = app.clone()
        .oneshot(req("GET", "/api/logs?unit=evil.service&lines=10", "", Some(&sid)))
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST, "unlisted unit must be rejected");
}

#[tokio::test]
async fn logs_returns_lines_for_allowed_unit() {
    let app = sentinel_web::app::build_router(fake_state());
    let sid = login(&app).await;

    let res = app.clone()
        .oneshot(req("GET", "/api/logs?unit=monad-bft.service&lines=10", "", Some(&sid)))
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let lines = json["lines"].as_array().expect("lines array");
    assert!(!lines.is_empty(), "should return some log lines");
    assert_eq!(lines[0], "log line 1");
}

#[tokio::test]
async fn metrics_returns_stored_points() {
    let state = fake_state();
    // Pre-seed a metric into the store
    state.store.append_metric("vote_rate", 1_000_000, 42.0).unwrap();
    state.store.append_metric("vote_rate", 2_000_000, 99.5).unwrap();

    let app = sentinel_web::app::build_router(state);
    let sid = login(&app).await;

    let res = app.clone()
        .oneshot(req("GET", "/api/metrics?name=vote_rate&window=24h", "", Some(&sid)))
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let points = json["points"].as_array().expect("points array");
    assert!(!points.is_empty(), "should return metric points");
    // Both points are well within the 24h window (86_400_000 ms from now=0)
    assert_eq!(points.len(), 2);
    assert_eq!(points[0][1], 42.0);
    assert_eq!(points[1][1], 99.5);
}

#[tokio::test]
async fn metrics_unknown_window_is_400() {
    let app = sentinel_web::app::build_router(fake_state());
    let sid = login(&app).await;

    let res = app.clone()
        .oneshot(req("GET", "/api/metrics?name=vote_rate&window=999y", "", Some(&sid)))
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn alerts_returns_injected_records() {
    let state = fake_state();
    // Inject two alerts
    {
        let mut buf = state.alerts.lock().unwrap();
        buf.push_back(AlertRecord { ts_ms: 1000, rule: "high_latency".into(), message: "latency too high".into() });
        buf.push_back(AlertRecord { ts_ms: 2000, rule: "low_peers".into(), message: "peer count low".into() });
    }

    let app = sentinel_web::app::build_router(state);
    let sid = login(&app).await;

    let res = app.clone()
        .oneshot(req("GET", "/api/alerts", "", Some(&sid)))
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let alerts = json.as_array().expect("alerts array");
    assert_eq!(alerts.len(), 2);
    // newest-first: ts_ms=2000 should come first
    assert_eq!(alerts[0]["ts_ms"], 2000);
    assert_eq!(alerts[0]["rule"], "low_peers");
    assert_eq!(alerts[1]["ts_ms"], 1000);
}

#[tokio::test]
async fn alerts_without_session_is_401() {
    let app = sentinel_web::app::build_router(fake_state());
    let res = app.oneshot(req("GET", "/api/alerts", "", None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// ---- Clamp tests: caller-supplied sizes must be bounded server-side ----

/// LogReader fake that echoes back as many lines as it was asked for,
/// so the response length reveals what `lines` value reached the reader.
struct CountingLogReader;
impl LogReader for CountingLogReader {
    fn tail(&self, _unit: &str, lines: usize) -> Vec<String> {
        (0..lines).map(|i| format!("l{i}")).collect()
    }
}

#[tokio::test]
async fn logs_lines_param_is_clamped() {
    let mut st = fake_state();
    st.logs = Arc::new(CountingLogReader);
    let app = sentinel_web::app::build_router(st);
    let sid = login(&app).await;

    let res = app.clone()
        .oneshot(req("GET", "/api/logs?unit=monad-bft.service&lines=999999", "", Some(&sid)))
        .await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let lines = json["lines"].as_array().expect("lines array");
    assert!(
        lines.len() <= 1000,
        "lines must be clamped server-side, got {}",
        lines.len()
    );
}

#[tokio::test]
async fn audit_limit_param_is_clamped_not_erroring() {
    let app = sentinel_web::app::build_router(fake_state());
    let sid = login(&app).await;
    let res = app.clone()
        .oneshot(req("GET", "/api/audit?limit=18446744073709551615", "", Some(&sid)))
        .await.unwrap();
    // A huge (even usize::MAX) limit must be accepted and clamped, not 500.
    assert_eq!(res.status(), StatusCode::OK);
}

/// Integration tests for POST /api/ops/restart
/// Uses a FakeExecutor that records calls; TOTP code is built from the state's secret + now/1000.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use sentinel_web::ops::{Op, OpExecutor};
use sentinel_web::state::{test_state_with_password, AppState};
use std::sync::{Arc, Mutex};

// ---- FakeExecutor ----

struct FakeExecutor {
    calls: Mutex<Vec<Op>>,
    should_fail: bool,
}

impl FakeExecutor {
    fn new() -> Arc<Self> {
        Arc::new(Self { calls: Mutex::new(vec![]), should_fail: false })
    }
    fn recorded(&self) -> Vec<Op> {
        self.calls.lock().unwrap().clone()
    }
}

impl OpExecutor for FakeExecutor {
    fn run(&self, op: &Op) -> anyhow::Result<String> {
        self.calls.lock().unwrap().push(op.clone());
        if self.should_fail {
            anyhow::bail!("simulated executor failure");
        }
        match op {
            Op::Restart { unit } => Ok(format!("restarted {unit}")),
        }
    }
}

// ---- Helpers ----

fn build_app(state: AppState) -> axum::Router {
    sentinel_web::app::build_router(state)
}

fn req_with(method: &str, uri: &str, body: &str, headers: &[(&str, &str)]) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    for (k, v) in headers {
        b = b.header(*k, *v);
    }
    b.body(Body::from(body.to_string())).unwrap()
}

/// Log in and return (sid_value, csrf_value) — raw token values without cookie name prefix.
async fn login_get_cookies(app: &axum::Router) -> (String, String) {
    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/auth/login",
            "{\"username\":\"admin\",\"password\":\"hunter2\"}",
            &[],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "login must succeed");
    let mut sid_val = String::new();
    let mut csrf_val = String::new();
    for hval in res.headers().get_all("set-cookie").iter() {
        let s = hval.to_str().unwrap();
        // Each set-cookie header is like "name=value; Path=/; ..."
        let pair = s.split(';').next().unwrap(); // "name=value"
        if let Some(v) = pair.strip_prefix("sid=") {
            sid_val = v.to_string();
        } else if let Some(v) = pair.strip_prefix("csrf=") {
            csrf_val = v.to_string();
        }
    }
    assert!(!sid_val.is_empty(), "sid cookie must be present");
    assert!(!csrf_val.is_empty(), "csrf cookie must be present");
    (sid_val, csrf_val)
}

/// Build current TOTP code from the state's secret and now_fn.
fn totp_code(state: &AppState) -> String {
    let secret = state.creds.lock().unwrap().totp_secret_b32.clone();
    let now_secs = (state.now)() / 1000;
    let secs = if now_secs < 0 { 0u64 } else { now_secs as u64 };
    sentinel_web::auth::totp::Totp::from_base32(&secret)
        .unwrap()
        .current(secs)
}

// ---- Test 1: restart allowed unit with valid TOTP+CSRF → 200 ----

#[tokio::test]
async fn restart_allowed_unit_valid_totp_csrf_returns_200() {
    let exec = FakeExecutor::new();
    let mut state = test_state_with_password("hunter2");
    // Use a real unix timestamp so TOTP is valid
    let fixed_now_ms = 1_700_000_000_000i64;
    state.now = Arc::new(move || fixed_now_ms);
    state.executor = exec.clone();

    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;
    let totp = totp_code(&state);

    // Send both sid and csrf cookies in the Cookie header; also send x-csrf header = csrf_val
    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);
    let body = serde_json::json!({
        "unit": "monad-bft.service",
        "totp": totp,
    })
    .to_string();

    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/restart",
            &body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", &csrf_val),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "valid restart must return 200");

    // FakeExecutor must have recorded exactly one Restart call
    let calls = exec.recorded();
    assert_eq!(calls.len(), 1, "executor must be called once");
    assert!(
        matches!(&calls[0], Op::Restart { unit } if unit == "monad-bft.service"),
        "executor must have seen Restart{{unit: monad-bft.service}}"
    );

    // Audit row with result="ok" must exist
    let rows = state.store.list_audit(10).unwrap();
    assert!(!rows.is_empty(), "audit row must be written");
    let ok_row = rows.iter().find(|r| r.result == "ok");
    assert!(ok_row.is_some(), "must have an audit row with result=ok");
    let row = ok_row.unwrap();
    assert_eq!(row.op, "restart");
    assert_eq!(row.params, "monad-bft.service");
}

// ---- Test 2: restart a unit NOT in the allowlist → 400 ----

#[tokio::test]
async fn restart_unlisted_unit_returns_400_and_denied_audit() {
    let exec = FakeExecutor::new();
    let mut state = test_state_with_password("hunter2");
    let fixed_now_ms = 1_700_000_000_000i64;
    state.now = Arc::new(move || fixed_now_ms);
    state.executor = exec.clone();

    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;
    let totp = totp_code(&state);

    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);
    let body = serde_json::json!({
        "unit": "evil-daemon.service",
        "totp": totp,
    })
    .to_string();

    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/restart",
            &body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", &csrf_val),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST, "unlisted unit must return 400");

    // Executor must NOT have been called
    let calls = exec.recorded();
    assert!(calls.is_empty(), "executor must NOT be called for unlisted unit");

    // Audit row with result="denied" must exist
    let rows = state.store.list_audit(10).unwrap();
    let denied = rows.iter().find(|r| r.result == "denied");
    assert!(denied.is_some(), "must have audit row with result=denied");
}

// ---- Test 3: restart with wrong TOTP → 403 ----

#[tokio::test]
async fn restart_wrong_totp_returns_403_and_denied_audit() {
    let exec = FakeExecutor::new();
    let mut state = test_state_with_password("hunter2");
    let fixed_now_ms = 1_700_000_000_000i64;
    state.now = Arc::new(move || fixed_now_ms);
    state.executor = exec.clone();

    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;

    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);
    let body = serde_json::json!({
        "unit": "monad-bft.service",
        "totp": "000000",
    })
    .to_string();

    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/restart",
            &body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", &csrf_val),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN, "wrong totp must return 403");

    // Executor must NOT be called
    assert!(exec.recorded().is_empty(), "executor must NOT be called for bad TOTP");

    // Audit row with result="denied"
    let rows = state.store.list_audit(10).unwrap();
    let denied = rows.iter().find(|r| r.result == "denied");
    assert!(denied.is_some(), "must have audit row with result=denied for bad TOTP");
}

// ---- Test 4: restart with missing/mismatched CSRF → 403 ----

#[tokio::test]
async fn restart_mismatched_csrf_returns_403() {
    let exec = FakeExecutor::new();
    let mut state = test_state_with_password("hunter2");
    let fixed_now_ms = 1_700_000_000_000i64;
    state.now = Arc::new(move || fixed_now_ms);
    state.executor = exec.clone();

    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;
    let totp = totp_code(&state);

    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);
    let body = serde_json::json!({
        "unit": "monad-bft.service",
        "totp": totp,
    })
    .to_string();

    // Send wrong x-csrf header (doesn't match cookie)
    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/restart",
            &body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", "wrong-token"),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN, "mismatched CSRF must return 403");

    // Executor must NOT be called
    assert!(exec.recorded().is_empty(), "executor must NOT be called for bad CSRF");
}

#[tokio::test]
async fn restart_missing_csrf_header_returns_403() {
    let exec = FakeExecutor::new();
    let mut state = test_state_with_password("hunter2");
    let fixed_now_ms = 1_700_000_000_000i64;
    state.now = Arc::new(move || fixed_now_ms);
    state.executor = exec.clone();

    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;
    let totp = totp_code(&state);

    // Send cookies but no x-csrf header
    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);
    let body = serde_json::json!({
        "unit": "monad-bft.service",
        "totp": totp,
    })
    .to_string();

    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/restart",
            &body,
            &[("cookie", &cookie_header)],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN, "missing CSRF header must return 403");

    assert!(exec.recorded().is_empty(), "executor must NOT be called for missing CSRF");
}

/// Integration tests for the TOTP guard on privileged operations:
/// - a code accepted once cannot be replayed within its validity window
/// - repeated wrong codes lock the ops endpoints out (mirrors the login throttle)
/// - the lockout expires after the throttle window
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use sentinel_web::ops::{Op, OpExecutor};
use sentinel_web::state::{test_state_with_password, AppState};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

struct FakeExecutor {
    calls: Mutex<Vec<Op>>,
}

impl FakeExecutor {
    fn new() -> Arc<Self> {
        Arc::new(Self { calls: Mutex::new(vec![]) })
    }
    fn count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

impl OpExecutor for FakeExecutor {
    fn run(&self, op: &Op) -> anyhow::Result<String> {
        self.calls.lock().unwrap().push(op.clone());
        Ok("ok".to_string())
    }
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
        let pair = s.split(';').next().unwrap();
        if let Some(v) = pair.strip_prefix("sid=") {
            sid_val = v.to_string();
        } else if let Some(v) = pair.strip_prefix("csrf=") {
            csrf_val = v.to_string();
        }
    }
    (sid_val, csrf_val)
}

/// TOTP code for the state's secret at an arbitrary unix-seconds timestamp.
fn totp_code_at(state: &AppState, secs: u64) -> String {
    let secret = state.creds.lock().unwrap().totp_secret_b32.clone();
    sentinel_web::auth::totp::Totp::from_base32(&secret)
        .unwrap()
        .current(secs)
}

fn totp_code(state: &AppState) -> String {
    let now_secs = (state.now)() / 1000;
    totp_code_at(state, if now_secs < 0 { 0 } else { now_secs as u64 })
}

const NOW_MS: i64 = 1_700_000_000_000;

/// State with a fake executor and a clock backed by an AtomicI64 the test can advance.
fn make_state() -> (AppState, Arc<FakeExecutor>, Arc<AtomicI64>) {
    let exec = FakeExecutor::new();
    let clock = Arc::new(AtomicI64::new(NOW_MS));
    let mut state = test_state_with_password("hunter2");
    let c = clock.clone();
    state.now = Arc::new(move || c.load(Ordering::SeqCst));
    state.executor = exec.clone();
    (state, exec, clock)
}

async fn post_restart(
    app: &axum::Router,
    cookie_header: &str,
    csrf_val: &str,
    totp: &str,
) -> StatusCode {
    let body = serde_json::json!({ "unit": "monad-bft.service", "totp": totp }).to_string();
    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/restart",
            &body,
            &[("cookie", cookie_header), ("x-csrf", csrf_val)],
        ))
        .await
        .unwrap();
    res.status()
}

#[tokio::test]
async fn totp_code_cannot_be_replayed_but_next_step_code_works() {
    let (state, exec, _clock) = make_state();
    let app = sentinel_web::app::build_router(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;
    let cookie = format!("sid={sid_val}; csrf={csrf_val}");

    let code = totp_code(&state);
    assert_eq!(post_restart(&app, &cookie, &csrf_val, &code).await, StatusCode::OK);
    assert_eq!(exec.count(), 1);

    // Same code again → rejected, executor NOT called again.
    assert_eq!(post_restart(&app, &cookie, &csrf_val, &code).await, StatusCode::FORBIDDEN);
    assert_eq!(exec.count(), 1);

    // The replay attempt must be audited distinctly.
    let audit = state.store.list_audit(10).unwrap();
    assert!(
        audit.iter().any(|r| r.result == "denied" && r.detail.contains("replay")),
        "audit must record the replay attempt: {audit:?}"
    );

    // A different (adjacent-step, still within the ±1 window) code is fine.
    let next = totp_code_at(&state, (NOW_MS / 1000) as u64 + 30);
    assert_ne!(next, code, "adjacent step must produce a different code");
    assert_eq!(post_restart(&app, &cookie, &csrf_val, &next).await, StatusCode::OK);
    assert_eq!(exec.count(), 2);
}

#[tokio::test]
async fn repeated_wrong_totp_locks_out_even_the_correct_code() {
    let (state, exec, _clock) = make_state();
    let app = sentinel_web::app::build_router(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;
    let cookie = format!("sid={sid_val}; csrf={csrf_val}");

    for _ in 0..5 {
        assert_eq!(
            post_restart(&app, &cookie, &csrf_val, "000000").await,
            StatusCode::FORBIDDEN
        );
    }

    // Locked out now — even the correct code is refused with 429.
    let code = totp_code(&state);
    assert_eq!(
        post_restart(&app, &cookie, &csrf_val, &code).await,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(exec.count(), 0, "executor must never run while locked out");
}

#[tokio::test]
async fn totp_lockout_expires_after_window() {
    let (state, exec, clock) = make_state();
    let app = sentinel_web::app::build_router(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;
    let cookie = format!("sid={sid_val}; csrf={csrf_val}");

    for _ in 0..5 {
        assert_eq!(
            post_restart(&app, &cookie, &csrf_val, "000000").await,
            StatusCode::FORBIDDEN
        );
    }
    let code = totp_code(&state);
    assert_eq!(
        post_restart(&app, &cookie, &csrf_val, &code).await,
        StatusCode::TOO_MANY_REQUESTS
    );

    // Advance past the 5-minute window; a correct (fresh-for-that-time) code works again.
    clock.fetch_add(301_000, Ordering::SeqCst);
    let code = totp_code(&state);
    assert_eq!(post_restart(&app, &cookie, &csrf_val, &code).await, StatusCode::OK);
    assert_eq!(exec.count(), 1);
}

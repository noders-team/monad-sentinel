/// Integration tests for upgrade/rollback/plan API.
/// Uses a FakeExecutor, FakeCandidateProbe, FakeVersionProbe injected into AppState.
/// Models the same `req` helper pattern as tests/ops_api.rs.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use sentinel_web::ops::{Op, OpExecutor};
use sentinel_web::probe::CandidateProbe;
use sentinel_web::state::{test_state_with_password, AppState};
use sentinel_web::probe::VersionProbe;
use std::sync::{Arc, Mutex};

// ---- FakeExecutor ----

struct FakeExecutor {
    calls: Mutex<Vec<Op>>,
}

impl FakeExecutor {
    fn new() -> Arc<Self> {
        Arc::new(Self { calls: Mutex::new(vec![]) })
    }
    fn recorded(&self) -> Vec<Op> {
        self.calls.lock().unwrap().clone()
    }
}

impl OpExecutor for FakeExecutor {
    fn run(&self, op: &Op) -> anyhow::Result<String> {
        self.calls.lock().unwrap().push(op.clone());
        match op {
            Op::Restart { unit } => Ok(format!("restarted {unit}")),
            Op::Upgrade { version } => Ok(format!("upgraded to {version}")),
        }
    }
}

// ---- FakeCandidateProbe — always returns "0.14.7" ----

struct FakeCandidateProbe;
impl CandidateProbe for FakeCandidateProbe {
    fn candidate(&self, _pkg: &str) -> Option<String> {
        Some("0.14.7".to_string())
    }
}

// ---- FakeVersionProbe — always returns "v0.14.5" ----

struct FakeVersionProbe;
impl VersionProbe for FakeVersionProbe {
    fn version(&self, _binary: &str) -> Option<String> {
        Some("v0.14.5".to_string())
    }
}

// ---- BadVersionProbe — returns junk / invalid that must NOT become a rollback point ----

struct BadVersionProbe;
impl VersionProbe for BadVersionProbe {
    fn version(&self, _binary: &str) -> Option<String> {
        Some("garbage".to_string())
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

/// Log in and return (sid_value, csrf_value).
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

/// Build a fresh state with fakes injected.
fn make_state() -> (AppState, Arc<FakeExecutor>) {
    let exec = FakeExecutor::new();
    let mut state = test_state_with_password("hunter2");
    let fixed_now_ms = 1_700_000_000_000i64;
    state.now = Arc::new(move || fixed_now_ms);
    state.executor = exec.clone();
    state.candidate_probe = Arc::new(FakeCandidateProbe);
    state.ver_probe = Arc::new(FakeVersionProbe);
    (state, exec)
}

// ---- Test 1: POST /api/ops/upgrade valid version+TOTP+CSRF → 200; executor recorded Op::Upgrade; rollback_point set; audit "ok" ----

#[tokio::test]
async fn upgrade_valid_returns_200_records_op_and_sets_rollback_point() {
    let (state, exec) = make_state();
    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;
    let totp = totp_code(&state);

    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);
    let body = serde_json::json!({
        "target_version": "0.14.7",
        "totp": totp,
    })
    .to_string();

    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/upgrade",
            &body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", &csrf_val),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "valid upgrade must return 200");

    // Executor must have recorded Op::Upgrade{version:"0.14.7"}
    let calls = exec.recorded();
    assert_eq!(calls.len(), 1, "executor must be called once");
    assert!(
        matches!(&calls[0], Op::Upgrade { version } if version == "0.14.7"),
        "executor must have seen Upgrade{{version: 0.14.7}}, got {:?}", calls[0]
    );

    // GET /api/upgrades must show rollback_point == the fake current version (canonicalized: "0.14.5")
    let get_res = app.clone()
        .oneshot(req_with(
            "GET", "/api/upgrades", "",
            &[("cookie", &cookie_header)],
        ))
        .await
        .unwrap();
    assert_eq!(get_res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(get_res.into_body(), usize::MAX).await.unwrap();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        val["rollback_point"].as_str(),
        Some("0.14.5"),
        "rollback_point must be the canonicalized current version"
    );

    // Audit row op="upgrade" result="ok" must exist
    let rows = state.store.list_audit(10).unwrap();
    let ok_row = rows.iter().find(|r| r.op == "upgrade" && r.result == "ok");
    assert!(ok_row.is_some(), "must have audit row with op=upgrade result=ok");
}

// ---- Test 2: POST /api/ops/upgrade bad version → 400; executor NOT called; audit "denied" ----

#[tokio::test]
async fn upgrade_bad_version_returns_400_executor_not_called() {
    let (state, exec) = make_state();
    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;
    let totp = totp_code(&state);

    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);
    let body = serde_json::json!({
        "target_version": "0.14",  // bad: only 2 parts
        "totp": totp,
    })
    .to_string();

    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/upgrade",
            &body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", &csrf_val),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST, "bad version must return 400");

    // Executor must NOT be called
    assert!(exec.recorded().is_empty(), "executor must NOT be called for bad version");

    // Audit row with result="denied" must exist
    let rows = state.store.list_audit(10).unwrap();
    let denied = rows.iter().find(|r| r.op == "upgrade" && r.result == "denied");
    assert!(denied.is_some(), "must have audit row with op=upgrade result=denied for bad version");
}

// ---- Test 3: POST /api/ops/upgrade wrong TOTP → 403; executor NOT called ----

#[tokio::test]
async fn upgrade_wrong_totp_returns_403_executor_not_called() {
    let (state, exec) = make_state();
    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;

    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);
    let body = serde_json::json!({
        "target_version": "0.14.7",
        "totp": "000000",  // definitely wrong
    })
    .to_string();

    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/upgrade",
            &body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", &csrf_val),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN, "wrong TOTP must return 403");

    // Executor must NOT be called
    assert!(exec.recorded().is_empty(), "executor must NOT be called for wrong TOTP");

    // Audit row with op="upgrade" result="denied" must exist
    let rows = state.store.list_audit(10).unwrap();
    let denied = rows.iter().find(|r| r.op == "upgrade" && r.result == "denied");
    assert!(denied.is_some(), "must have audit row with op=upgrade result=denied for wrong TOTP");
}

// ---- Test 4: POST /api/ops/rollback after a successful upgrade → 200; executor records Upgrade{rollback_point} ----

#[tokio::test]
async fn rollback_after_upgrade_returns_200_and_restores_rollback_point() {
    let (state, exec) = make_state();
    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;
    let totp = totp_code(&state);

    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);

    // First do an upgrade to set rollback_point
    let upgrade_body = serde_json::json!({
        "target_version": "0.14.7",
        "totp": totp.clone(),
    })
    .to_string();

    let upgrade_res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/upgrade",
            &upgrade_body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", &csrf_val),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(upgrade_res.status(), StatusCode::OK, "upgrade must succeed first");

    // Now do rollback (with the same TOTP since now() is frozen)
    let rollback_body = serde_json::json!({
        "totp": totp,
    })
    .to_string();

    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/rollback",
            &rollback_body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", &csrf_val),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "rollback must return 200");

    // Executor must have recorded: first Upgrade{0.14.7}, then Upgrade{0.14.5}
    let calls = exec.recorded();
    assert_eq!(calls.len(), 2, "executor must be called twice (upgrade + rollback)");
    assert!(
        matches!(&calls[1], Op::Upgrade { version } if version == "0.14.5"),
        "rollback must execute Upgrade{{version: 0.14.5}}, got {:?}", calls[1]
    );
}

// ---- Test 5: POST /api/upgrades/plan {target_version, deadline} → 200; GET /api/upgrades shows them ----

#[tokio::test]
async fn set_plan_persists_target_and_deadline() {
    let (state, _exec) = make_state();
    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;

    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);

    // POST plan
    let plan_body = serde_json::json!({
        "target_version": "0.14.7",
        "deadline": "2026-07-01T00:00:00Z",
    })
    .to_string();

    let plan_res = app.clone()
        .oneshot(req_with(
            "POST", "/api/upgrades/plan",
            &plan_body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", &csrf_val),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(plan_res.status(), StatusCode::OK, "set_plan must return 200");

    // GET /api/upgrades must show target and deadline
    let get_res = app.clone()
        .oneshot(req_with(
            "GET", "/api/upgrades", "",
            &[("cookie", &cookie_header)],
        ))
        .await
        .unwrap();
    assert_eq!(get_res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(get_res.into_body(), usize::MAX).await.unwrap();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["target"].as_str(), Some("0.14.7"), "target must be stored");
    assert_eq!(val["deadline"].as_str(), Some("2026-07-01T00:00:00Z"), "deadline must be stored");
}

// ---- Test 7: upgrade with invalid current version → 200, no junk rollback_point, warn audit, 409 on rollback ----

#[tokio::test]
async fn upgrade_with_invalid_current_version_proceeds_no_junk_rollback_point() {
    // Build state with BadVersionProbe — current version is "garbage", fails version::validate.
    let exec = FakeExecutor::new();
    let mut state = test_state_with_password("hunter2");
    let fixed_now_ms = 1_700_000_000_000i64;
    state.now = Arc::new(move || fixed_now_ms);
    state.executor = exec.clone();
    state.candidate_probe = Arc::new(FakeCandidateProbe);
    state.ver_probe = Arc::new(BadVersionProbe);

    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;
    let totp = totp_code(&state);
    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);

    // POST upgrade — must still return 200 (junk current version must NOT block upgrade).
    let body = serde_json::json!({
        "target_version": "0.14.7",
        "totp": totp.clone(),
    })
    .to_string();

    let res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/upgrade",
            &body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", &csrf_val),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "upgrade must still succeed when current version is invalid");

    // Executor must have recorded Op::Upgrade{target}.
    let calls = exec.recorded();
    assert_eq!(calls.len(), 1);
    assert!(
        matches!(&calls[0], Op::Upgrade { version } if version == "0.14.7"),
        "executor must record Upgrade{{version: 0.14.7}}, got {:?}", calls[0]
    );

    // GET /api/upgrades must show rollback_point is null/absent — NOT "garbage".
    let get_res = app.clone()
        .oneshot(req_with(
            "GET", "/api/upgrades", "",
            &[("cookie", &cookie_header)],
        ))
        .await
        .unwrap();
    assert_eq!(get_res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(get_res.into_body(), usize::MAX).await.unwrap();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        val["rollback_point"].is_null(),
        "rollback_point must be null when current version was junk, got {:?}",
        val["rollback_point"]
    );

    // Audit log must contain a row mentioning "rollback" in the detail (the warn row).
    let rows = state.store.list_audit(20).unwrap();
    let warn_row = rows.iter().find(|r| r.detail.contains("rollback"));
    assert!(
        warn_row.is_some(),
        "must have audit row mentioning 'rollback' in detail; rows: {:?}",
        rows.iter().map(|r| &r.detail).collect::<Vec<_>>()
    );

    // POST rollback must return 409 (no rollback point stored).
    let rollback_body = serde_json::json!({ "totp": totp }).to_string();
    let rollback_res = app.clone()
        .oneshot(req_with(
            "POST", "/api/ops/rollback",
            &rollback_body,
            &[
                ("cookie", &cookie_header),
                ("x-csrf", &csrf_val),
            ],
        ))
        .await
        .unwrap();
    assert_eq!(
        rollback_res.status(),
        StatusCode::CONFLICT,
        "rollback must return 409 when no valid rollback_point was stored"
    );
}

// ---- Test 6: GET /api/upgrades reports current and candidate from fake probes ----

#[tokio::test]
async fn get_upgrades_reports_current_and_candidate() {
    let (state, _exec) = make_state();
    let app = build_app(state.clone());
    let (sid_val, csrf_val) = login_get_cookies(&app).await;

    let cookie_header = format!("sid={}; csrf={}", sid_val, csrf_val);

    let res = app.clone()
        .oneshot(req_with(
            "GET", "/api/upgrades", "",
            &[("cookie", &cookie_header)],
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    // FakeVersionProbe returns "v0.14.5" (raw, not canonicalized — ver_probe returns it verbatim)
    assert_eq!(val["current"].as_str(), Some("v0.14.5"), "current must be the raw value from FakeVersionProbe");
    // FakeCandidateProbe returns "0.14.7"
    assert_eq!(val["candidate"].as_str(), Some("0.14.7"), "candidate must come from FakeCandidateProbe");
}

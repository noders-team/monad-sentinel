use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::middleware::AuthActor;
use crate::state::{AlertRecord, AppState};
use crate::store::AuditRow;

#[derive(Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub unit: String,
    pub kind: String,
    pub active: bool,
    pub version: Option<String>,
}

/// GET /api/status — requires auth; returns per-service status from probes.
pub async fn get_status(
    _actor: AuthActor,
    State(st): State<AppState>,
) -> Json<serde_json::Value> {
    let services: Vec<ServiceStatus> = st.cfg.services.iter().map(|svc| {
        let active = st.svc_probe.is_active(&svc.unit);
        let version = st.ver_probe.version(&svc.binary);
        let kind = format!("{:?}", svc.kind).to_lowercase();
        ServiceStatus { name: svc.name.clone(), unit: svc.unit.clone(), kind, active, version }
    }).collect();
    Json(json!(services))
}

#[derive(Deserialize)]
pub struct MetricsParams {
    pub name: String,
    pub window: String,
}

/// GET /api/metrics?name=&window=1h|24h|7d — requires auth.
pub async fn get_metrics(
    _actor: AuthActor,
    State(st): State<AppState>,
    Query(params): Query<MetricsParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let window_ms: i64 = match params.window.as_str() {
        "1h"  => 3_600_000,
        "24h" => 86_400_000,
        "7d"  => 604_800_000,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let now_ms = (st.now)();
    let since_ms = now_ms - window_ms;
    let points = st.store.query_window(&params.name, since_ms)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let pts: Vec<serde_json::Value> = points.into_iter()
        .map(|(ts, val)| json!([ts, val]))
        .collect();
    Ok(Json(json!({ "points": pts })))
}

#[derive(Deserialize)]
pub struct LogsParams {
    pub unit: String,
    pub lines: Option<usize>,
}

/// GET /api/logs?unit=&lines= — requires auth; validates unit against allowlist.
pub async fn get_logs(
    _actor: AuthActor,
    State(st): State<AppState>,
    Query(params): Query<LogsParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let allowed = st.cfg.allowed_units();
    if !allowed.contains(&params.unit) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let n = params.lines.unwrap_or(100);
    let lines = st.logs.tail(&params.unit, n);
    Ok(Json(json!({ "lines": lines })))
}

/// GET /api/alerts — requires auth; returns last fired alerts, newest-first.
pub async fn get_alerts(
    _actor: AuthActor,
    State(st): State<AppState>,
) -> Json<serde_json::Value> {
    let buf = st.alerts.lock().unwrap();
    let records: Vec<&AlertRecord> = buf.iter().rev().collect();
    Json(json!(records))
}

#[derive(Deserialize)]
pub struct AuditParams {
    pub limit: Option<usize>,
}

/// GET /api/audit?limit= — requires auth; returns most recent audit rows, newest-first.
pub async fn get_audit(
    _actor: AuthActor,
    State(st): State<AppState>,
    Query(params): Query<AuditParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit.unwrap_or(50);
    let rows = st.store.list_audit(limit)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let out: Vec<serde_json::Value> = rows.iter().map(|r| json!({
        "ts_ms": r.ts_ms,
        "actor": r.actor,
        "op": r.op,
        "params": r.params,
        "result": r.result,
        "detail": r.detail,
    })).collect();
    Ok(Json(json!(out)))
}

#[derive(Deserialize)]
pub struct PlanBody {
    pub target_version: String,
    pub deadline: String,
}

/// POST /api/upgrades/plan {target_version, deadline}
/// Requires: AuthActor + CSRF (no TOTP — non-destructive).
/// Validates the version, then stores target and deadline in meta.
pub async fn set_plan(
    AuthActor(actor): AuthActor,
    State(st): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<PlanBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // CSRF check: x-csrf header must match csrf cookie.
    if super::check_csrf(&headers, &jar).is_err() {
        let row = AuditRow {
            ts_ms: (st.now)(),
            actor: actor.clone(),
            op: "upgrade_plan".to_string(),
            params: body.target_version.clone(),
            result: "denied".to_string(),
            detail: "csrf mismatch".to_string(),
        };
        if let Err(e) = st.store.append_audit(&row) {
            eprintln!("audit write error: {e:#}");
        }
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate version.
    let canonical = match crate::version::validate(&body.target_version) {
        Some(v) => v,
        None => return Err(StatusCode::BAD_REQUEST),
    };

    st.store.set_meta("upgrade_target", &canonical)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    st.store.set_meta("upgrade_deadline", &body.deadline)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let row = AuditRow {
        ts_ms: (st.now)(),
        actor: actor.clone(),
        op: "upgrade_plan".to_string(),
        params: canonical.clone(),
        result: "ok".to_string(),
        detail: format!("deadline={}", body.deadline),
    };
    if let Err(e) = st.store.append_audit(&row) {
        eprintln!("audit write error: {e:#}");
    }

    Ok(Json(json!({ "ok": true })))
}

/// GET /api/upgrades — requires auth.
/// Returns {current, candidate, target, deadline, rollback_point}.
pub async fn get_upgrades(
    _actor: AuthActor,
    State(st): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // current: from ver_probe on the BFT binary (first service in cfg).
    let bft_binary = st.cfg.services.first().map(|s| s.binary.as_str()).unwrap_or("");
    let current = st.ver_probe.version(bft_binary);

    // candidate: from candidate_probe.
    let candidate = st.candidate_probe.candidate(&st.cfg.package);

    // Rest from meta (None → JSON null).
    let target = st.store.get_meta("upgrade_target")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let deadline = st.store.get_meta("upgrade_deadline")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rollback_point = st.store.get_meta("rollback_point")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "current": current,
        "candidate": candidate,
        "target": target,
        "deadline": deadline,
        "rollback_point": rollback_point,
    })))
}

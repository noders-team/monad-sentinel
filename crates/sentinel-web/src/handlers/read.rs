use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::middleware::AuthActor;
use crate::state::{AlertRecord, AppState};

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

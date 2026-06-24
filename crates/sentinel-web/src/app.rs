use axum::{routing::{get, post}, Json, Router};
use serde_json::json;
use crate::handlers::auth as auth_h;
use crate::handlers::ops as ops_h;
use crate::handlers::read as read_h;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(|| async { Json(json!({"status": "ok"})) }))
        .route("/api/auth/login", post(auth_h::login))
        .route("/api/auth/logout", post(auth_h::logout))
        .route("/api/auth/me", get(auth_h::me))
        .route("/api/status", get(read_h::get_status))
        .route("/api/metrics", get(read_h::get_metrics))
        .route("/api/logs", get(read_h::get_logs))
        .route("/api/alerts", get(read_h::get_alerts))
        .route("/api/audit", get(read_h::get_audit))
        .route("/api/ops/restart", post(ops_h::restart))
        .with_state(state)
}

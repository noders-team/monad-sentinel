use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use axum::{routing::{get, post}, Json, Router};
use serde_json::json;
use crate::handlers::auth as auth_h;
use crate::handlers::ops as ops_h;
use crate::handlers::read as read_h;
use crate::state::AppState;

/// `style-src 'unsafe-inline'` is required by the charting library (inline SVG
/// styles); everything else is same-origin only. No HSTS: the service speaks
/// plain HTTP behind an SSH tunnel / VPN by design.
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
                   img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; \
                   base-uri 'none'; form-action 'self'";

/// Defense-in-depth response headers. Applied to the API router here and again
/// to the outermost router in main.rs so the SPA fallback is covered too
/// (`insert` overwrites, so double application is idempotent).
pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert("content-security-policy", HeaderValue::from_static(CSP));
    h.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    h.insert("x-frame-options", HeaderValue::from_static("DENY"));
    h.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    res
}

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
        .route("/api/ops/upgrade", post(ops_h::upgrade))
        .route("/api/ops/rollback", post(ops_h::rollback))
        .route("/api/upgrades", get(read_h::get_upgrades))
        .route("/api/upgrades/plan", post(read_h::set_plan))
        .layer(axum::middleware::from_fn(security_headers))
        .with_state(state)
}

use axum::{routing::{get, post}, Json, Router};
use serde_json::json;
use crate::handlers::auth as auth_h;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(|| async { Json(json!({"status": "ok"})) }))
        .route("/api/auth/login", post(auth_h::login))
        .route("/api/auth/logout", post(auth_h::logout))
        .route("/api/auth/me", get(auth_h::me))
        .with_state(state)
}

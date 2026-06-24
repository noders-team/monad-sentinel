/// POST /api/ops/restart
/// Requires: authenticated session (AuthActor), matching CSRF tokens, valid TOTP code.
/// Security path:
///   1. CSRF check: x-csrf header must match csrf cookie value.
///   2. Session validity is already guaranteed by AuthActor extractor (returns 401 if invalid).
///   3. TOTP check: code must be valid for the current time step.
///   4. Allowlist check: unit must be in cfg.allowed_units().
///   5. Execute: call executor.run(); write AuditRow on every outcome (ok/denied/error).
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use serde_json::json;

use crate::auth::totp::Totp;
use crate::middleware::AuthActor;
use crate::ops::Op;
use crate::state::AppState;
use crate::store::AuditRow;

#[derive(Deserialize)]
pub struct RestartBody {
    pub unit: String,
    pub totp: String,
}

/// Write an audit row; logs error but does not fail the request.
fn audit(state: &AppState, actor: &str, unit: &str, result: &str, detail: &str) {
    let row = AuditRow {
        ts_ms: (state.now)(),
        actor: actor.to_string(),
        op: "restart".to_string(),
        params: unit.to_string(),
        result: result.to_string(),
        detail: detail.to_string(),
    };
    if let Err(e) = state.store.append_audit(&row) {
        eprintln!("audit write error: {e:#}");
    }
}

pub async fn restart(
    AuthActor(actor): AuthActor,
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<RestartBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 1. CSRF check: x-csrf header must match the csrf cookie value.
    //    The csrf cookie was set by login as a plain token value (not HttpOnly).
    let csrf_header = headers
        .get("x-csrf")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let csrf_cookie = jar
        .get("csrf")
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    if csrf_header.is_empty() || csrf_cookie.is_empty() || csrf_header != csrf_cookie {
        // Don't write audit for CSRF failures — no authenticated unit context yet.
        // (The actor is authenticated but we won't log unit-level audit for pure CSRF failure
        //  to avoid leaking timing info; spec says write audit on every path; comply below.)
        // Actually the spec says EVERY failure path must audit. Write it.
        audit(&state, &actor, &body.unit, "denied", "csrf mismatch");
        return Err(StatusCode::FORBIDDEN);
    }

    // 2. TOTP check.
    let creds = state.creds.lock().unwrap().clone();
    let now_secs = (state.now)() / 1000;
    let secs = if now_secs < 0 { 0u64 } else { now_secs as u64 };

    let totp_ok = Totp::from_base32(&creds.totp_secret_b32)
        .map(|t| t.check(&body.totp, secs))
        .unwrap_or(false);

    if !totp_ok {
        audit(&state, &actor, &body.unit, "denied", "invalid totp");
        return Err(StatusCode::FORBIDDEN);
    }

    // 3. Allowlist check.
    let allowed = state.cfg.allowed_units();
    if !allowed.contains(&body.unit) {
        audit(&state, &actor, &body.unit, "denied", "unit not in allowlist");
        return Err(StatusCode::BAD_REQUEST);
    }

    // 4. Execute.
    let op = Op::Restart { unit: body.unit.clone() };
    match state.executor.run(&op) {
        Ok(detail) => {
            audit(&state, &actor, &body.unit, "ok", &detail);
            Ok(Json(json!({ "ok": true, "detail": detail })))
        }
        Err(e) => {
            let detail = format!("{e:#}");
            audit(&state, &actor, &body.unit, "error", &detail);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/ops/restart  — restart a managed systemd unit.
/// POST /api/ops/upgrade  — upgrade the bft binary to a target version.
/// POST /api/ops/rollback — roll back to the version stored in meta["rollback_point"].
///
/// All require: authenticated session (AuthActor), matching CSRF tokens, valid TOTP code.
/// Security path:
///   1. Version validation (upgrade only): target_version must pass `version::validate`.
///   2. CSRF check: x-csrf header must match csrf cookie value.
///   3. Session validity is already guaranteed by AuthActor extractor (returns 401 if invalid).
///   4. TOTP lockout: 5 failed codes per 5 minutes → 429 before any verification.
///   5. TOTP check: code must be valid for the current time step AND not already
///      used (single-use: replaying an accepted code is denied and audited).
///   6. Allowlist check (restart only): unit must be in cfg.allowed_units().
///   7. Execute: call executor.run(); write AuditRow on every outcome (ok/denied/error).
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

#[derive(Deserialize)]
pub struct UpgradeBody {
    pub target_version: String,
    pub totp: String,
}

#[derive(Deserialize)]
pub struct RollbackBody {
    pub totp: String,
}

/// Write an audit row with a configurable op name; logs error but does not fail the request.
fn audit_op(state: &AppState, actor: &str, op: &str, params: &str, result: &str, detail: &str) {
    let row = AuditRow {
        ts_ms: (state.now)(),
        actor: actor.to_string(),
        op: op.to_string(),
        params: params.to_string(),
        result: result.to_string(),
        detail: detail.to_string(),
    };
    if let Err(e) = state.store.append_audit(&row) {
        eprintln!("audit write error: {e:#}");
    }
}

/// Failed-TOTP lockout: mirrors the login throttle (5 failures per 5 minutes).
const TOTP_MAX_FAILS: u32 = 5;
const TOTP_FAIL_WINDOW_MS: i64 = 300_000;
/// How long an accepted code is remembered for replay denial. Covers the full
/// ±1-step validity window (3 × 30 s), so a code can never be accepted twice.
const TOTP_REPLAY_TTL_MS: i64 = 90_000;

/// Shared CSRF + TOTP check.
/// Returns `Ok(())` if both pass, or `Err((StatusCode, reason))` on failure.
/// The `reason` is a `&'static str` suitable for use as an audit detail.
/// The caller is responsible for writing an audit row on failure.
fn check_csrf_and_totp(
    state: &AppState,
    headers: &HeaderMap,
    jar: &CookieJar,
    totp_code: &str,
) -> Result<(), (StatusCode, &'static str)> {
    // CSRF: x-csrf header must match the csrf cookie value.
    let csrf_header = headers
        .get("x-csrf")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let csrf_cookie = jar
        .get("csrf")
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    if csrf_header.is_empty() || csrf_cookie.is_empty() || csrf_header != csrf_cookie {
        return Err((StatusCode::FORBIDDEN, "csrf mismatch"));
    }

    let now_ms = (state.now)();

    // Lockout: too many failed TOTP attempts recently → refuse before verifying,
    // so a brute-force can't keep probing the code space.
    {
        let mut g = state.totp_guard.lock().unwrap();
        if now_ms - g.fails.1 > TOTP_FAIL_WINDOW_MS {
            g.fails = (0, now_ms);
        }
        if g.fails.0 >= TOTP_MAX_FAILS {
            return Err((StatusCode::TOO_MANY_REQUESTS, "totp throttled"));
        }
    }

    // TOTP check.
    let creds = state.creds.lock().unwrap().clone();
    let now_secs = now_ms / 1000;
    let secs = if now_secs < 0 { 0u64 } else { now_secs as u64 };

    let totp_ok = Totp::from_base32(&creds.totp_secret_b32)
        .map(|t| t.check(totp_code, secs))
        .unwrap_or(false);

    let mut g = state.totp_guard.lock().unwrap();

    if !totp_ok {
        g.fails.0 += 1;
        return Err((StatusCode::FORBIDDEN, "invalid totp"));
    }

    // Single-use: a code that already authorized an operation is denied (and
    // counted as a failure — a replay is an attack signal, not a typo).
    g.used.retain(|(_, at)| now_ms - *at <= TOTP_REPLAY_TTL_MS);
    if g.used.iter().any(|(c, _)| c == totp_code) {
        g.fails.0 += 1;
        return Err((StatusCode::FORBIDDEN, "totp replay (code already used)"));
    }
    g.used.push((totp_code.to_string(), now_ms));
    g.fails = (0, now_ms);

    Ok(())
}

pub async fn restart(
    AuthActor(actor): AuthActor,
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<RestartBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 1. CSRF + TOTP check (shared helper).
    if let Err((status, reason)) = check_csrf_and_totp(&state, &headers, &jar, &body.totp) {
        audit_op(&state, &actor, "restart", &body.unit, "denied", reason);
        return Err(status);
    }

    // 2. Allowlist check.
    let allowed = state.cfg.allowed_units();
    if !allowed.contains(&body.unit) {
        audit_op(&state, &actor, "restart", &body.unit, "denied", "unit not in allowlist");
        return Err(StatusCode::BAD_REQUEST);
    }

    // 3. Execute.
    let op = Op::Restart { unit: body.unit.clone() };
    match state.executor.run(&op) {
        Ok(detail) => {
            audit_op(&state, &actor, "restart", &body.unit, "ok", &detail);
            Ok(Json(json!({ "ok": true, "detail": detail })))
        }
        Err(e) => {
            let detail = format!("{e:#}");
            audit_op(&state, &actor, "restart", &body.unit, "error", &detail);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/ops/upgrade {target_version, totp}
/// Steps (fail-closed, audit ALL paths):
///   1. Validate target_version → None = 400 + audit "denied"
///   2. CSRF check → bad = 403 + audit "denied"
///   3. TOTP check → bad = 403 + audit "denied"
///   4. Read current version via ver_probe → canonicalize → store in meta["rollback_point"]
///   5. executor.run(Op::Upgrade{version: canonical_target})
///   6. Audit "ok"/"error"; return 200 or 500
pub async fn upgrade(
    AuthActor(actor): AuthActor,
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<UpgradeBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 1. Validate target_version before any auth check — fail fast on bad input.
    let canonical_target = match crate::version::validate(&body.target_version) {
        Some(v) => v,
        None => {
            audit_op(&state, &actor, "upgrade", &body.target_version, "denied", "invalid version");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // 2 & 3. CSRF + TOTP (shared helper).
    if let Err((status, reason)) = check_csrf_and_totp(&state, &headers, &jar, &body.totp) {
        audit_op(&state, &actor, "upgrade", &canonical_target, "denied", reason);
        return Err(status);
    }

    // 4. Read current version and store as rollback point — ONLY if it validates.
    //    Use the first service binary as the bft binary reference (index 0 = BFT in default cfg).
    {
        let bft_binary = state.cfg.services.first().map(|s| s.binary.as_str()).unwrap_or("");
        let current_raw = state.ver_probe.version(bft_binary);
        match current_raw.as_deref().and_then(crate::version::validate) {
            Some(canon) => {
                if let Err(e) = state.store.set_meta("rollback_point", &canon) {
                    let detail = format!("failed to store rollback_point: {e:#}");
                    audit_op(&state, &actor, "upgrade", &canonical_target, "error", &detail);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
            None => {
                // Do NOT store a junk value — leave rollback_point unset (or keep the prior
                // good one) and write an audit warning so the operator can see what happened.
                audit_op(
                    &state,
                    &actor,
                    "upgrade",
                    &canonical_target,
                    "warn",
                    &format!("no valid rollback point captured (current={current_raw:?})"),
                );
            }
        }
    }

    // 5. Execute upgrade.
    let op = Op::Upgrade { version: canonical_target.clone() };
    match state.executor.run(&op) {
        Ok(detail) => {
            audit_op(&state, &actor, "upgrade", &canonical_target, "ok", &detail);
            Ok(Json(json!({ "ok": true, "detail": detail })))
        }
        Err(e) => {
            let detail = format!("{e:#}");
            audit_op(&state, &actor, "upgrade", &canonical_target, "error", &detail);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/ops/rollback {totp}
/// Steps:
///   1. CSRF + TOTP check → 403 on failure
///   2. Read meta["rollback_point"] → 409 if absent
///   3. executor.run(Op::Upgrade{version: rollback_point})
///   4. Audit; 200
pub async fn rollback(
    AuthActor(actor): AuthActor,
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<RollbackBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 1. CSRF + TOTP.
    if let Err((status, reason)) = check_csrf_and_totp(&state, &headers, &jar, &body.totp) {
        audit_op(&state, &actor, "rollback", "", "denied", reason);
        return Err(status);
    }

    // 2. Read rollback_point.
    let rollback_point = match state.store.get_meta("rollback_point") {
        Ok(Some(v)) => v,
        Ok(None) => {
            audit_op(&state, &actor, "rollback", "", "denied", "no rollback_point stored");
            return Err(StatusCode::CONFLICT);
        }
        Err(e) => {
            let detail = format!("{e:#}");
            audit_op(&state, &actor, "rollback", "", "error", &detail);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // 3. Execute rollback as an upgrade to the stored version.
    let op = Op::Upgrade { version: rollback_point.clone() };
    match state.executor.run(&op) {
        Ok(detail) => {
            audit_op(&state, &actor, "rollback", &rollback_point, "ok", &detail);
            Ok(Json(json!({ "ok": true, "detail": detail })))
        }
        Err(e) => {
            let detail = format!("{e:#}");
            audit_op(&state, &actor, "rollback", &rollback_point, "error", &detail);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

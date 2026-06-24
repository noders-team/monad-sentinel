pub mod auth;
pub mod ops;
pub mod read;

use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::CookieJar;

/// Shared CSRF-only check (no TOTP).
/// Returns `Ok(())` if the `x-csrf` header matches the `csrf` cookie, `Err(StatusCode::FORBIDDEN)` otherwise.
pub(super) fn check_csrf(headers: &HeaderMap, jar: &CookieJar) -> Result<(), StatusCode> {
    let csrf_header = headers
        .get("x-csrf")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let csrf_cookie = jar
        .get("csrf")
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    if csrf_header.is_empty() || csrf_cookie.is_empty() || csrf_header != csrf_cookie {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

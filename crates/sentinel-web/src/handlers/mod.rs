pub mod auth;
pub mod ops;
pub mod read;

use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::CookieJar;
use subtle::ConstantTimeEq;

/// Constant-time string comparison for the CSRF double-submit check. The token
/// is not secret from the requesting browser, so a remote timing attack is a
/// stretch — but a server-side token comparison should not leak positions on
/// principle. Length is compared first (it is public: both sides are 64-hex).
pub(super) fn csrf_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.as_bytes().ct_eq(b.as_bytes()).into()
}

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

    if csrf_header.is_empty() || csrf_cookie.is_empty() || !csrf_eq(csrf_header, &csrf_cookie) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::csrf_eq;

    #[test]
    fn csrf_eq_matches_and_rejects() {
        assert!(csrf_eq("abc123", "abc123"));
        assert!(!csrf_eq("abc123", "abc124"));
        assert!(!csrf_eq("abc123", "abc12")); // length mismatch
        assert!(!csrf_eq("", "abc123"));
        assert!(csrf_eq("", "")); // callers reject empties before comparing
    }
}

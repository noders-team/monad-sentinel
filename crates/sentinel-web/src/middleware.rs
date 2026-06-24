use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum_extra::extract::CookieJar;
use crate::state::AppState;

/// Extractor that validates the `sid` session cookie and yields the actor name.
/// Returns 401 Unauthorized if the cookie is missing or the session is invalid/expired.
pub struct AuthActor(pub String);

impl FromRequestParts<AppState> for AuthActor {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, StatusCode> {
        let jar = CookieJar::from_headers(&parts.headers);
        let sid = jar
            .get("sid")
            .map(|c| c.value().to_string())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let now = (state.now)();
        let actor = state
            .sessions
            .validate(&sid, now)
            .ok_or(StatusCode::UNAUTHORIZED)?;
        Ok(AuthActor(actor))
    }
}

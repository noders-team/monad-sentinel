use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use serde_json::json;
use crate::auth::password;
use crate::middleware::AuthActor;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

/// POST /api/auth/login
/// Validates credentials; on success sets `sid` (HttpOnly) and `csrf` cookies and
/// returns `{"actor":"admin"}`. Returns 429 when the login rate-limit is exceeded,
/// 401 on bad credentials.
pub async fn login(
    State(st): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginBody>,
) -> Result<(CookieJar, Json<serde_json::Value>), StatusCode> {
    let now = (st.now)();

    // Rate-limit: max 5 failures per username per 5 minutes (300_000 ms)
    {
        let mut t = st.login_throttle.lock().unwrap();
        let e = t.fails.entry(body.username.clone()).or_insert((0, now));
        if now - e.1 > 300_000 {
            *e = (0, now);
        }
        if e.0 >= 5 {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }

    let creds = st.creds.lock().unwrap().clone();
    let ok = body.username == "admin" && password::verify(&body.password, &creds.pw_phc);

    if !ok {
        let mut t = st.login_throttle.lock().unwrap();
        let e = t.fails.entry(body.username.clone()).or_insert((0, now));
        e.0 += 1;
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Create session and CSRF tokens (TTL: 12 h)
    let ttl_ms = 12 * 60 * 60 * 1_000_i64;
    let token = st.sessions.create("admin", now, ttl_ms);
    let csrf = st.sessions.create(&format!("csrf:{token}"), now, ttl_ms);

    let secure = !st.cfg.dev_insecure_cookies;

    let sid_cookie = Cookie::build(("sid", token))
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(secure)
        .path("/")
        .build();

    let csrf_cookie = Cookie::build(("csrf", csrf))
        .http_only(false)
        .same_site(SameSite::Strict)
        .secure(secure)
        .path("/")
        .build();

    Ok((jar.add(sid_cookie).add(csrf_cookie), Json(json!({"actor": "admin"}))))
}

/// POST /api/auth/logout
/// Removes the session from the store and clears both cookies.
pub async fn logout(
    State(st): State<AppState>,
    jar: CookieJar,
) -> (CookieJar, Json<serde_json::Value>) {
    if let Some(c) = jar.get("sid") {
        st.sessions.remove(c.value());
    }
    let jar = jar
        .remove(Cookie::from("sid"))
        .remove(Cookie::from("csrf"));
    (jar, Json(json!({"ok": true})))
}

/// GET /api/auth/me
/// Returns `{"actor":"<name>"}` for an authenticated session, or 401 if not authenticated.
pub async fn me(AuthActor(actor): AuthActor) -> Json<serde_json::Value> {
    Json(json!({ "actor": actor }))
}

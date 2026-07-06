use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use sentinel_web::state::test_state_with_password;

fn req(method: &str, uri: &str, body: &str, cookie: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    b.body(Body::from(body.to_string())).unwrap()
}

#[tokio::test]
async fn login_sets_cookie_and_me_works() {
    let state = test_state_with_password("hunter2");
    let app = sentinel_web::app::build_router(state);

    // wrong password → 401
    let res = app.clone().oneshot(req("POST", "/api/auth/login",
        "{\"username\":\"admin\",\"password\":\"nope\"}", None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // right password → 200 + Set-Cookie sid
    let res = app.clone().oneshot(req("POST", "/api/auth/login",
        "{\"username\":\"admin\",\"password\":\"hunter2\"}", None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Collect all set-cookie headers and find the sid one
    let cookies: Vec<String> = res.headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect();
    let sid_header = cookies.iter().find(|c| c.contains("sid="))
        .expect("sid cookie not found in set-cookie headers");
    assert!(sid_header.contains("HttpOnly"), "sid cookie must be HttpOnly");

    // /api/auth/me with the sid cookie → 200
    let sid = sid_header.split(';').next().unwrap().to_string();
    let res = app.clone().oneshot(req("GET", "/api/auth/me", "", Some(&sid))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["actor"], "admin");
}

#[tokio::test]
async fn logout_leaves_no_session_entries_behind() {
    // login creates a session; logout must remove EVERYTHING it created —
    // the CSRF token must not linger in the store (unbounded growth otherwise).
    let state = test_state_with_password("hunter2");
    let app = sentinel_web::app::build_router(state.clone());

    let res = app.clone().oneshot(req("POST", "/api/auth/login",
        "{\"username\":\"admin\",\"password\":\"hunter2\"}", None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cookies: Vec<String> = res.headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect();
    let sid = cookies.iter().find(|c| c.starts_with("sid="))
        .unwrap().split(';').next().unwrap().to_string();

    let res = app.clone().oneshot(req("POST", "/api/auth/logout", "", Some(&sid))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    assert_eq!(state.sessions.len(), 0, "no session-store entries may survive logout");
}

#[tokio::test]
async fn me_without_cookie_is_401() {
    let state = test_state_with_password("x");
    let app = sentinel_web::app::build_router(state);
    let res = app.oneshot(req("GET", "/api/auth/me", "", None)).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

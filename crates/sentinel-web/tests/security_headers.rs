/// Every response — API and SPA alike — must carry the defense-in-depth
/// security headers (CSP, clickjacking and MIME-sniffing protection).
/// No HSTS: the service is plain HTTP behind an SSH tunnel / VPN by design.
use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;
use sentinel_web::state::test_state_with_password;

#[tokio::test]
async fn security_headers_present_on_api_responses() {
    let state = test_state_with_password("x");
    let app = sentinel_web::app::build_router(state);

    let res = app
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let h = res.headers();
    assert_eq!(
        h.get("x-content-type-options").map(|v| v.to_str().unwrap()),
        Some("nosniff")
    );
    assert_eq!(
        h.get("x-frame-options").map(|v| v.to_str().unwrap()),
        Some("DENY")
    );
    assert_eq!(
        h.get("referrer-policy").map(|v| v.to_str().unwrap()),
        Some("no-referrer")
    );
    let csp = h
        .get("content-security-policy")
        .map(|v| v.to_str().unwrap())
        .expect("CSP header must be present");
    assert!(csp.contains("default-src 'self'"), "csp: {csp}");
    assert!(csp.contains("frame-ancestors 'none'"), "csp: {csp}");
}

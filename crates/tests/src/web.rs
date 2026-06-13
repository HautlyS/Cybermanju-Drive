use cybermanju_web::WebDashboard;

#[test]
fn test_web_dashboard_creation() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.redb");
    let _d = WebDashboard::new(3456, db_path.to_str().unwrap());
}

#[test]
fn test_web_dashboard_new_with_bind_addr() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.redb");
    let d = WebDashboard::new_with_bind_addr(8080, db_path.to_str().unwrap(), "0.0.0.0");
    assert_eq!(d.port, 8080);
    assert_eq!(d.bind_addr, "0.0.0.0");
}

#[test]
fn test_web_dashboard_db_accessor() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.redb");
    let d = WebDashboard::new(3456, db_path.to_str().unwrap());
    let _guard = d.db().lock().unwrap();
}

#[test]
fn test_security_constants() {
    assert!(cybermanju_web::MAX_BODY_SIZE > 0);
    assert!(cybermanju_web::RATE_LIMIT_MAX > 0);
    assert!(cybermanju_web::RATE_LIMIT_WINDOW_SECS > 0);
    assert!(!cybermanju_web::ALLOWED_ORIGINS.is_empty());
}

#[test]
fn test_handle_request_health() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.redb");
    let d = WebDashboard::new(3456, db_path.to_str().unwrap());
    let db_guard = d.db().lock().unwrap();
    let resp = cybermanju_web::handle_request(
        &d,
        &db_guard,
        "GET",
        "/api/health",
        "",
        None,
        None,
    );
    assert!(resp.contains("200"));
}

#[test]
fn test_handle_request_404() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.redb");
    let d = WebDashboard::new(3456, db_path.to_str().unwrap());
    let db_guard = d.db().lock().unwrap();
    let resp = cybermanju_web::handle_request(
        &d,
        &db_guard,
        "GET",
        "/api/nonexistent",
        "",
        None,
        None,
    );
    assert!(resp.contains("404"));
}

#[test]
fn test_handle_request_options_cors() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.redb");
    let d = WebDashboard::new(3456, db_path.to_str().unwrap());
    let db_guard = d.db().lock().unwrap();
    let resp = cybermanju_web::handle_request(
        &d,
        &db_guard,
        "OPTIONS",
        "/api/health",
        "",
        None,
        Some("http://localhost:3456"),
    );
    assert!(resp.contains("204"));
    assert!(resp.contains("Access-Control-Allow-Origin"));
}

#[test]
fn test_rate_limit_check() {
    use std::collections::HashMap;
    use std::sync::Mutex;

    let limits = Mutex::new(HashMap::new());
    for _ in 0..100 {
        assert!(cybermanju_web::check_rate_limit(&limits, "1.2.3.4"));
    }
}

#[test]
fn test_rate_limit_exceeded() {
    use std::collections::HashMap;
    use std::sync::Mutex;

    let limits = Mutex::new(HashMap::new());
    for _ in 0..101 {
        cybermanju_web::check_rate_limit(&limits, "5.6.7.8");
    }
    assert!(!cybermanju_web::check_rate_limit(&limits, "5.6.7.8"));
}

#[test]
fn test_rate_limit_different_ips() {
    use std::collections::HashMap;
    use std::sync::Mutex;

    let limits = Mutex::new(HashMap::new());
    for _ in 0..100 {
        cybermanju_web::check_rate_limit(&limits, "ip1");
    }
    assert!(cybermanju_web::check_rate_limit(&limits, "ip2"));
}

#[test]
fn test_mime_type() {
    assert_eq!(cybermanju_web::mime_type("test.html"), "text/html; charset=utf-8");
    assert_eq!(cybermanju_web::mime_type("style.css"), "text/css; charset=utf-8");
    assert_eq!(cybermanju_web::mime_type("app.js"), "application/javascript; charset=utf-8");
    assert_eq!(cybermanju_web::mime_type("data.json"), "application/json; charset=utf-8");
    assert_eq!(cybermanju_web::mime_type("image.png"), "image/png");
    assert_eq!(cybermanju_web::mime_type("photo.jpg"), "image/jpeg");
    assert_eq!(cybermanju_web::mime_type("photo.jpeg"), "image/jpeg");
    assert_eq!(cybermanju_web::mime_type("anim.gif"), "image/gif");
    assert_eq!(cybermanju_web::mime_type("icon.svg"), "image/svg+xml");
    assert_eq!(cybermanju_web::mime_type("icon.ico"), "image/x-icon");
    assert_eq!(cybermanju_web::mime_type("font.woff"), "font/woff");
    assert_eq!(cybermanju_web::mime_type("font.woff2"), "font/woff2");
    assert_eq!(cybermanju_web::mime_type("font.ttf"), "font/ttf");
    assert_eq!(cybermanju_web::mime_type("doc.pdf"), "application/pdf");
    assert_eq!(cybermanju_web::mime_type("archive.zip"), "application/zip");
    assert_eq!(cybermanju_web::mime_type("code.wasm"), "application/wasm");
    assert_eq!(cybermanju_web::mime_type("data.xml"), "application/xml; charset=utf-8");
    assert_eq!(cybermanju_web::mime_type("readme.txt"), "text/plain; charset=utf-8");
    assert_eq!(cybermanju_web::mime_type("data.csv"), "text/csv; charset=utf-8");
    assert_eq!(cybermanju_web::mime_type("unknown.xyz"), "application/octet-stream");
}

#[test]
fn test_mime_type_no_extension() {
    assert_eq!(cybermanju_web::mime_type("Makefile"), "application/octet-stream");
}

#[test]
fn test_serve_static_file_exists() {
    let _ = cybermanju_web::serve_static_file;
}

#[test]
fn test_default_port() {
    assert_eq!(cybermanju_web::DEFAULT_PORT, 3456);
}

// Cybermanju Drive — Standalone Web Server (Docker)
//
// Unified HTTP server that:
// 1. Serves compiled Vue frontend as static files (binary-safe)
// 2. Routes /api/* to the web dashboard REST API handlers
//
// Environment variables:
//   PORT          — listening port (default: 3456)
//   DB_PATH      — path to redb database (default: /data/cybermanju.db)
//   STATIC_DIR   — path to frontend dist files (default: ./static)
//   RUST_LOG     — log level (default: info)

mod web_dashboard;

use log::{error, info, warn};
use std::fs;
use std::io::{BufRead, BufReader, Read as IoRead, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// MIME type lookup for common file extensions
fn mime_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("webp") => "image/webp",
        Some("webm") => "video/webm",
        Some("mp4") => "video/mp4",
        Some("mp3") => "audio/mpeg",
        Some("wasm") => "application/wasm",
        Some("xml") => "application/xml; charset=utf-8",
        Some("txt") => "text/plain; charset=utf-8",
        Some("csv") => "text/csv; charset=utf-8",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("gz") | Some("gzip") => "application/gzip",
        Some("map") => "application/json",
        _ => "application/octet-stream",
    }
}

/// Build an HTTP status text from a status code
fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

fn http_response(status: u16, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} {}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
         \r\n\
         {body}",
        status_text(status),
        body.len()
    )
}

/// Serve a static file, writing binary-safe HTTP response directly to the stream.
/// Returns true if the file was served, false if a text-based error was written.
fn serve_static_file(stream: &mut TcpStream, static_dir: &Path, request_path: &str) -> bool {
    // Default to index.html for root or directory paths
    let file_path = if request_path == "/" || request_path.ends_with('/') {
        static_dir.join("index.html")
    } else {
        static_dir.join(request_path.trim_start_matches('/'))
    };

    // Security: prevent path traversal
    let resolved = match file_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let resp = http_response(
                404,
                "text/html; charset=utf-8",
                "<html><body><h1>404 Not Found</h1></body></html>",
            );
            let _ = stream.write_all(resp.as_bytes());
            return true;
        }
    };

    let static_resolved = match static_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let resp = http_response(
                500,
                "text/html; charset=utf-8",
                "<html><body><h1>500 Internal Server Error</h1></body></html>",
            );
            let _ = stream.write_all(resp.as_bytes());
            return true;
        }
    };

    if !resolved.starts_with(&static_resolved) {
        let resp = http_response(
            403,
            "text/html; charset=utf-8",
            "<html><body><h1>403 Forbidden</h1></body></html>",
        );
        let _ = stream.write_all(resp.as_bytes());
        return true;
    }

    // SPA fallback: if the file doesn't exist, serve index.html
    let actual_path = if resolved.is_file() {
        resolved
    } else {
        let index = static_dir.join("index.html");
        if index.is_file() {
            index
        } else {
            let resp = http_response(
                404,
                "text/html; charset=utf-8",
                "<html><body><h1>404 Not Found</h1></body></html>",
            );
            let _ = stream.write_all(resp.as_bytes());
            return true;
        }
    };

    let contents = match fs::read(&actual_path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read static file {:?}: {}", actual_path, e);
            let resp = http_response(
                500,
                "text/html; charset=utf-8",
                "<html><body><h1>500 Internal Server Error</h1></body></html>",
            );
            let _ = stream.write_all(resp.as_bytes());
            return true;
        }
    };

    let content_type = mime_type(&actual_path.to_string_lossy());
    let content_length = contents.len();

    // Build and write the HTTP response header
    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {content_length}\r\n\
         Cache-Control: public, max-age=3600\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
         \r\n"
    );

    if let Err(e) = stream.write_all(header.as_bytes()) {
        warn!("Failed to write response header: {}", e);
        return true;
    }

    // Write the raw file bytes (binary-safe, no String conversion)
    if let Err(e) = stream.write_all(&contents) {
        warn!("Failed to write file body: {}", e);
    }

    true
}

/// Shared application state passed to each connection handler
struct AppState {
    static_dir: PathBuf,
    db_path: String,
    dashboard: web_dashboard::WebDashboard,
}

fn handle_connection(state: &AppState, mut stream: TcpStream) {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .ok();
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(30)))
        .ok();

    let mut reader = BufReader::new(&stream);

    // Read request line
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        let _ = write!(
            stream,
            "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n"
        );
        return;
    }
    let request_line = request_line.trim();

    // Parse method and path from "GET /path HTTP/1.1"
    let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        let _ = write!(
            stream,
            "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n"
        );
        return;
    }
    let method = parts[0];
    let path = parts[1];

    // Read headers to determine Content-Length, Authorization, and Origin
    let mut content_length: usize = 0;
    let mut auth_header: Option<String> = None;
    let mut origin_header: Option<String> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line == "\r\n" || line.is_empty() {
            break;
        }
        let line_trimmed = line.trim().to_lowercase();
        if line_trimmed.starts_with("content-length:") {
            content_length = line_trimmed
                .strip_prefix("content-length:")
                .unwrap_or_default()
                .trim()
                .parse()
                .unwrap_or(0);
        } else if line_trimmed.starts_with("authorization:") {
            let val = line.trim();
            auth_header = val
                .strip_prefix("Authorization:")
                .or_else(|| val.strip_prefix("authorization:"))
                .map(|s| s.trim().to_string());
        } else if line_trimmed.starts_with("origin:") {
            let val = line.trim();
            origin_header = val
                .strip_prefix("Origin:")
                .or_else(|| val.strip_prefix("origin:"))
                .map(|s| s.trim().to_string());
        }
    }

    // Read body if present
    let mut body = String::new();
    if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        if std::io::Read::read_exact(&mut reader, &mut buf).is_err() {
            body = String::new();
        } else if let Ok(s) = String::from_utf8(buf) {
            body = s;
        }
    }

    // Route: /api/* → web dashboard handlers, everything else → static files
    if path.starts_with("/api/") || path == "/api" {
        let db_guard = state.dashboard.db.lock().unwrap();
        let response = web_dashboard::handle_request(
            &state.dashboard,
            &db_guard,
            method,
            path,
            &body,
            auth_header.as_deref(),
            origin_header.as_deref(),
        );
        drop(db_guard);
        let _ = stream.write_all(response.as_bytes());
    } else if method == "OPTIONS" {
        // CORS preflight for static assets
        let resp = http_response(200, "text/plain", "");
        let _ = stream.write_all(resp.as_bytes());
    } else if method == "GET" || method == "HEAD" {
        // Serve static files (binary-safe, writes directly to stream)
        serve_static_file(&mut stream, &state.static_dir, path);
    } else {
        let resp = http_response(
            405,
            "application/json",
            r#"{"error":"Method Not Allowed"}"#,
        );
        let _ = stream.write_all(resp.as_bytes());
    }
}

fn main() {
    env_logger::Builder::from_env("RUST_LOG").init();

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3456".to_string())
        .parse()
        .unwrap_or(3456);

    let db_path = std::env::var("DB_PATH")
        .unwrap_or_else(|_| "/data/cybermanju.db".to_string());

    let static_dir: PathBuf = std::env::var("STATIC_DIR")
        .unwrap_or_else(|_| "./static".to_string())
        .into();

    // Ensure database directory exists
    if let Some(parent) = Path::new(&db_path).parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            error!("Failed to create database directory {:?}: {}", parent, e);
            std::process::exit(1);
        }
    }

    // Verify static directory exists
    if !static_dir.exists() {
        error!(
            "Static directory does not exist: {}",
            static_dir.display()
        );
        std::process::exit(1);
    }

    let state = Arc::new(AppState {
        static_dir,
        db_path: db_path.clone(),
        dashboard: web_dashboard::WebDashboard::new(port, &db_path),
    });

    let addr = format!("0.0.0.0:{}", port);
    let listener = match std::net::TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind on port {}: {}", port, e);
            std::process::exit(1);
        }
    };

    info!("═══════════════════════════════════════════════════════");
    info!("  Cybermanju Drive — Web Server");
    info!("  Listening on http://{}", addr);
    info!("  API:        http://localhost:{}/api/health", port);
    info!("  Database:   {}", state.db_path);
    info!("  Static:     {}", state.static_dir.display());
    info!("═══════════════════════════════════════════════════════");

    // Serve requests in a thread-per-connection model (same as web_dashboard)
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = Arc::clone(&state);
                std::thread::spawn(move || {
                    handle_connection(&state, stream);
                });
            }
            Err(e) => {
                warn!("Accept error: {}", e);
            }
        }
    }
}
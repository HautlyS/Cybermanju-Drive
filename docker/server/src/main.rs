// Cybermanju Drive — Standalone Web Server (Docker)
//
// Unified HTTP server that:
// 1. Serves compiled Vue frontend as static files (binary-safe)
// 2. Routes /api/* to the shared cybermanju-web REST API
//
// Environment variables:
//   PORT          — listening port (default: 3456)
//   DB_PATH       — path to redb database (default: /data/cybermanju.db)
//   STATIC_DIR    — path to frontend dist files (default: ./static)
//   RUST_LOG      — log level (default: info)

use cybermanju_db::Database;
use cybermanju_web::{handle_request, http_response, serve_static_file, WebDashboard};
use log::{error, info, warn};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Shared application state passed to each connection handler
struct AppState {
    static_dir: PathBuf,
    db_path: String,
    dashboard: WebDashboard,
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

    let origin = origin_header.as_deref();

    // Route: /api/* → shared REST handlers, everything else → static files
    if path.starts_with("/api/") || path == "/api" {
        let response = handle_request(
            &state.dashboard,
            &state.dashboard.db,
            method,
            path,
            &body,
            auth_header.as_deref(),
            origin,
        );
        let _ = stream.write_all(response.as_bytes());
    } else if method == "OPTIONS" {
        // CORS preflight for static assets
        let resp = http_response(204, "text/plain", "", origin);
        let _ = stream.write_all(resp.as_bytes());
    } else if method == "GET" || method == "HEAD" {
        // Serve static files (binary-safe, writes directly to stream)
        serve_static_file(&mut stream, &state.static_dir, path);
    } else {
        let resp = http_response(
            405,
            "application/json",
            r#"{"error":"Method Not Allowed"}"#,
            None,
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

    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "/data/cybermanju.db".to_string());

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
        error!("Static directory does not exist: {}", static_dir.display());
        std::process::exit(1);
    }

    // Open the database exactly once and share it with the REST handlers.
    let db = match Database::new(&db_path) {
        Ok(db) => Arc::new(RwLock::new(db)),
        Err(e) => {
            error!("Failed to open database {}: {}", db_path, e);
            std::process::exit(1);
        }
    };

    let dashboard = WebDashboard::new_shared_with_bind_addr(port, Arc::clone(&db), "0.0.0.0");

    let state = Arc::new(AppState {
        static_dir,
        db_path: db_path.clone(),
        dashboard,
    });

    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr) {
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

    // Serve requests in a thread-per-connection model (same as cybermanju-web)
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

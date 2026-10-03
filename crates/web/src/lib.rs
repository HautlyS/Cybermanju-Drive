// Cybermanju Drive — Web Dashboard Server (Security-Hardened Rewrite)
// Embedded HTTP server for any-device browser access
// Exposes REST API mirroring all Tauri IPC commands
//
// Uses only std::net::TcpListener + manual HTTP parsing (no external HTTP crate)
//
// Security hardening applied:
//   1. Binds to 127.0.0.1 ONLY — prevents remote network exposure
//   2. JWT-based authentication (HS256) on all endpoints except login/register/health
//   3. CORS restricted to localhost origins only (not wildcard)
//   4. Request body size limit of 100 MB to prevent DoS
//   5. Private keys stripped from encryption key list responses
//   6. HMAC-signed JWT tokens (shared secret generated at startup)
//   7. Proper shutdown via mpsc signal channel, thread join on Drop
//   8. Rate limiting: 100 requests per minute per IP address

pub mod api;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cybermanju_compression::TripleCompressor;
use cybermanju_db::Database;
use cybermanju_search::SearchIndex;
use cybermanju_sync::SyncState;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use log::{error, info, warn};
use rand_core::{OsRng, RngCore};
use redb::{ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

// Table definitions come from the shared `cybermanju-db` crate
// (`Database::get_*_table()`), so the router can never drift from the schema.

// ─── Security constants ─────────────────────────────────────────────

/// Maximum request body size: 100 MB
#[allow(dead_code)]
pub const MAX_BODY_SIZE: usize = 104_857_600;

/// Rate limit: max requests per window per IP
#[allow(dead_code)]
pub const RATE_LIMIT_MAX: u32 = 100;

/// Default port for the web dashboard
#[allow(dead_code)]
pub const DEFAULT_PORT: u16 = 3456;

/// Rate limit window duration in seconds
#[allow(dead_code)]
pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;

/// JWT token expiry: 24 hours
const JWT_EXPIRY_SECS: u64 = 86_400;

/// Allowed CORS origins (localhost only)
#[allow(dead_code)]
pub const ALLOWED_ORIGINS: &[&str] = &[
    "http://localhost:3456",
    "http://127.0.0.1:3456",
    "http://localhost:3457",
    "http://127.0.0.1:3457",
];

// ─── JWT Claims ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    /// Subject — username
    sub: String,
    /// User role (admin, user, etc.)
    role: String,
    /// User ID (UUID)
    user_id: String,
    /// Issued-at timestamp (seconds since epoch)
    iat: u64,
    /// Expiration timestamp (seconds since epoch)
    exp: u64,
}

// ─── WebDashboard struct ────────────────────────────────────────────

pub struct WebDashboard {
    pub port: u16,
    /// Bind address — "127.0.0.1" for desktop, "0.0.0.0" for Docker
    pub bind_addr: String,
    /// Random 256-bit secret generated at startup for HMAC-SHA256 JWT signing
    pub jwt_secret: [u8; 32],
    /// Shared database handle — opened exactly once and shared with the rest
    /// of the application (redb allows only one open handle per file).
    pub db: Arc<RwLock<Database>>,
    /// Shared sync progress / cancellation state for `/api/sync/*`.
    pub sync_state: Arc<SyncState>,
    /// Shared Tantivy full-text index. `None` until the host wires one in
    /// (`set_search_index`); search then falls back to a database scan.
    pub search_index: Option<Arc<RwLock<SearchIndex>>>,
    /// Shared triple compressor used by the sync pipeline.
    pub compression: TripleCompressor,
    pub running: AtomicBool,
    /// Live number of in-flight HTTP connections (reported by
    /// `GET /api/dashboard/status` and the Tauri `dashboard_status` command).
    pub active_connections: AtomicU64,
    /// Per-IP rate limit counters: IP → (count, window_start)
    pub rate_limits: Mutex<HashMap<String, (u32, Instant)>>,
    pub server_thread: Mutex<Option<thread::JoinHandle<()>>>,
    pub shutdown_tx: Mutex<Option<mpsc::Sender<()>>>,
}

impl WebDashboard {
    /// Open (or create) the database at `db_path` and build a dashboard around it.
    /// Prefer `new_shared` when the application already owns a database handle.
    pub fn new(port: u16, db_path: &str) -> Self {
        let db = Database::new(db_path).expect("Failed to open web dashboard database");
        Self::new_shared(port, Arc::new(RwLock::new(db)))
    }

    /// Constructor that allows specifying a bind address (for Docker use case).
    pub fn new_with_bind_addr(port: u16, db_path: &str, bind_addr: &str) -> Self {
        let db = Database::new(db_path).expect("Failed to open web dashboard database");
        Self::new_shared_with_bind_addr(port, Arc::new(RwLock::new(db)), bind_addr)
    }

    /// Build a dashboard around an already-open database handle so the same
    /// redb file is never opened twice (which would fail on the exclusive lock).
    pub fn new_shared(port: u16, db: Arc<RwLock<Database>>) -> Self {
        Self::new_shared_with_bind_addr(port, db, "127.0.0.1")
    }

    /// Like `new_shared`, but with an explicit bind address.
    pub fn new_shared_with_bind_addr(
        port: u16,
        db: Arc<RwLock<Database>>,
        bind_addr: &str,
    ) -> Self {
        // Generate a cryptographically random 256-bit JWT secret
        let mut jwt_secret = [0u8; 32];
        OsRng.fill_bytes(&mut jwt_secret);

        Self {
            port,
            bind_addr: bind_addr.to_string(),
            jwt_secret,
            db,
            sync_state: Arc::new(SyncState::new()),
            search_index: None,
            compression: TripleCompressor::new(),
            running: AtomicBool::new(false),
            active_connections: AtomicU64::new(0),
            rate_limits: Mutex::new(HashMap::new()),
            server_thread: Mutex::new(None),
            shutdown_tx: Mutex::new(None),
        }
    }

    /// Accessor for the shared database handle.
    pub fn db(&self) -> &Arc<RwLock<Database>> {
        &self.db
    }

    /// Accessor for the shared sync state.
    pub fn sync_state(&self) -> &Arc<SyncState> {
        &self.sync_state
    }

    /// Wire the application's Tantivy index into the dashboard so REST search
    /// hits the same index as the desktop app.
    pub fn set_search_index(&mut self, index: Arc<RwLock<SearchIndex>>) {
        self.search_index = Some(index);
    }

    /// Start the web dashboard HTTP server on a background thread.
    /// Binds to 127.0.0.1 only. Returns Ok(()) on successful bind.
    #[allow(dead_code)]
    pub fn start(self: &std::sync::Arc<Self>) -> std::io::Result<()> {
        // Stop any previously running server
        self.stop();

        self.running.store(true, Ordering::SeqCst);
        let this = std::sync::Arc::clone(self);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
        {
            let mut tx_guard = self.shutdown_tx.lock().expect("shutdown_tx lock poisoned");
            *tx_guard = Some(shutdown_tx);
        }

        let handle = thread::spawn(move || {
            let addr = format!("{}:{}", this.bind_addr, this.port);
            let listener = match TcpListener::bind(&addr) {
                Ok(l) => {
                    info!(
                        "Web Dashboard listening on http://{} (localhost only)",
                        addr
                    );
                    l
                }
                Err(e) => {
                    error!("Web Dashboard failed to bind on {}: {}", addr, e);
                    this.running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            // Use non-blocking accept so we can poll the shutdown channel
            listener.set_nonblocking(true).ok();

            loop {
                // Check shutdown signals
                if !this.running.load(Ordering::SeqCst) {
                    break;
                }
                match shutdown_rx.try_recv() {
                    Ok(()) | Err(mpsc::TryRecvError::Disconnected) => {
                        info!("Web Dashboard received shutdown signal");
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }

                // Non-blocking accept with 50ms poll interval
                match listener.accept() {
                    Ok((stream, _addr)) => {
                        let this_clone = std::sync::Arc::clone(&this);
                        thread::spawn(move || {
                            handle_connection(&this_clone, stream);
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                    Err(e) => {
                        if this.running.load(Ordering::SeqCst) {
                            warn!("Web Dashboard accept error: {}", e);
                        }
                    }
                }
            }
            info!("Web Dashboard server stopped");
        });

        // Store the thread handle for later joining
        {
            let mut thread_guard = self
                .server_thread
                .lock()
                .expect("server_thread lock poisoned");
            *thread_guard = Some(handle);
        }

        Ok(())
    }

    /// Signal the server to stop and join the thread.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);

        // Send shutdown signal via channel
        {
            let mut tx_guard = self.shutdown_tx.lock().expect("shutdown_tx lock poisoned");
            if let Some(tx) = tx_guard.take() {
                let _ = tx.send(());
            }
        }

        // Also connect to ourselves to unblock accept() if it somehow
        // isn't in non-blocking mode (defensive)
        let _ = TcpStream::connect_timeout(
            &format!("127.0.0.1:{}", self.port)
                .parse::<std::net::SocketAddr>()
                .unwrap_or_else(|_| "127.0.0.1:3456".parse().unwrap()),
            Duration::from_secs(1),
        );

        // Join the server thread to ensure clean shutdown
        {
            let mut thread_guard = self
                .server_thread
                .lock()
                .expect("server_thread lock poisoned");
            if let Some(handle) = thread_guard.take() {
                if let Err(e) = handle.join() {
                    error!("Web Dashboard thread join error: {:?}", e);
                }
            }
        }
    }
}

impl Drop for WebDashboard {
    fn drop(&mut self) {
        self.stop();
    }
}

// ─── Connection handler ──────────────────────────────────────────────

/// Decrements `active_connections` when a connection handler returns.
struct ActiveConnectionGuard<'a>(&'a AtomicU64);

impl Drop for ActiveConnectionGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn handle_connection(dashboard: &WebDashboard, mut stream: TcpStream) {
    dashboard.active_connections.fetch_add(1, Ordering::SeqCst);
    let _active = ActiveConnectionGuard(&dashboard.active_connections);

    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(5))).ok();

    // Extract client IP for rate limiting
    let client_ip = stream
        .peer_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // ── Rate limiting check ──
    if !check_rate_limit(&dashboard.rate_limits, &client_ip) {
        write_http_json(
            &mut stream,
            429,
            r#"{"error":true,"status":429,"message":"Rate limit exceeded"}"#,
        );
        return;
    }

    // Parse the HTTP request using a fresh BufReader over a shared reference.
    // The BufReader borrow ends when `parse_http_request` returns,
    // leaving `stream` available for writing the response.
    let parse_result = parse_http_request(&stream);
    let ParsedRequest {
        method,
        path,
        body,
        auth_header,
        effective_origin,
    } = match parse_result {
        Ok(r) => r,
        Err((status, msg)) => {
            write_http_json(&mut stream, status, &msg);
            return;
        }
    };

    let effective_origin = effective_origin.as_deref();

    // Handle the request — the database lock is taken inside `handle_request`
    let response = handle_request(
        dashboard,
        &dashboard.db,
        &method,
        &path,
        &body,
        auth_header.as_deref(),
        effective_origin,
    );

    let _ = stream.write_all(response.as_bytes());
}

struct ParsedRequest {
    method: String,
    path: String,
    body: String,
    auth_header: Option<String>,
    effective_origin: Option<String>,
}

/// Parse an HTTP request from a TcpStream without writing to it.
fn parse_http_request(stream: &TcpStream) -> Result<ParsedRequest, (u16, String)> {
    let mut reader = BufReader::new(stream);

    // Read request line
    let mut request_line = String::new();
    reader.read_line(&mut request_line).map_err(|_| {
        (
            400,
            r#"{"error":true,"status":400,"message":"Bad Request"}"#.to_string(),
        )
    })?;
    let request_line = request_line.trim();

    // Parse method and path from "GET /path HTTP/1.1"
    let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err((
            400,
            r#"{"error":true,"status":400,"message":"Bad Request"}"#.to_string(),
        ));
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();

    // Read headers — extract Content-Length, Authorization, Origin
    let mut content_length: usize = 0;
    let mut auth_header: Option<String> = None;
    let mut origin_header: Option<String> = None;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line == "\r\n" || line.is_empty() {
            break;
        }
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        if lower.starts_with("content-length:") {
            content_length = trimmed[15..].trim().parse().unwrap_or(0);
        } else if lower.starts_with("authorization:") {
            auth_header = Some(trimmed[14..].trim().to_string());
        } else if lower.starts_with("origin:") {
            origin_header = Some(trimmed[7..].trim().to_string());
        }
    }

    // ── Body size limit enforcement ──
    if content_length > MAX_BODY_SIZE {
        return Err((
            413,
            r#"{"error":true,"status":413,"message":"Request body too large"}"#.to_string(),
        ));
    }

    // Read body if present (capped to MAX_BODY_SIZE for safety)
    let body = if content_length > 0 {
        let read_size = content_length.min(MAX_BODY_SIZE);
        let mut buf = vec![0u8; read_size];
        if std::io::Read::read_exact(&mut reader, &mut buf).is_err() {
            String::new()
        } else {
            String::from_utf8(buf).unwrap_or_default()
        }
    } else {
        String::new()
    };

    // Determine effective CORS origin for response headers
    let effective_origin = origin_header
        .as_deref()
        .filter(|o| ALLOWED_ORIGINS.contains(o))
        .map(|o| o.to_string());

    Ok(ParsedRequest {
        method,
        path,
        body,
        auth_header,
        effective_origin,
    })
}

/// Write a JSON HTTP response.
fn write_http_json(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "Error",
    };
    let resp = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        status,
        reason,
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
}

// ─── Request router ──────────────────────────────────────────────────

/// Either flavour of database lock, so one code path can serve both readers
/// (GET/HEAD) and writers (POST/PUT/DELETE) with a single route table.
enum DbLock<'a> {
    Read(std::sync::RwLockReadGuard<'a, Database>),
    Write(std::sync::RwLockWriteGuard<'a, Database>),
}

impl<'a> DbLock<'a> {
    fn db(&self) -> &Database {
        match self {
            DbLock::Read(guard) => guard,
            DbLock::Write(guard) => guard,
        }
    }
}

/// Parse a JSON request body, returning an HTTP 400 response on failure.
macro_rules! json_body {
    ($body:expr, $origin:expr) => {
        match serde_json::from_str($body) {
            Ok(v) => v,
            Err(e) => return json_error(400, &format!("Invalid JSON: {}", e), $origin),
        }
    };
}

/// Serialize a value as a 200 JSON response.
fn json_ok<T: Serialize>(value: &T, origin: Option<&str>) -> String {
    let body = match serde_json::to_string(value) {
        Ok(body) => body,
        Err(_) => "null".to_string(),
    };
    http_response(200, "application/json", &body, origin)
}

/// Render a shared-API `Result` as an HTTP response: 200 on success, 404
/// when the message reports a missing entity, 400 otherwise.
fn api_response<T: Serialize>(result: Result<T, String>, origin: Option<&str>) -> String {
    match result {
        Ok(value) => json_ok(&value, origin),
        Err(message) => {
            let status = if message.to_lowercase().contains("not found") {
                404
            } else {
                400
            };
            json_error(status, &message, origin)
        }
    }
}

pub fn handle_request(
    dashboard: &WebDashboard,
    db: &Arc<RwLock<Database>>,
    method: &str,
    path: &str,
    body: &str,
    auth_header: Option<&str>,
    origin: Option<&str>,
) -> String {
    // Handle CORS preflight requests
    if method == "OPTIONS" {
        return cors_preflight_response(origin);
    }

    // Parse query string from path
    let (path_clean, query) = match path.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path, ""),
    };

    let path_segments: Vec<&str> = path_clean
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    // Determine if this endpoint requires authentication
    let requires_auth = !matches!(
        path_segments.as_slice(),
        ["api"] | ["api", "health"]           // health checks
            | ["api", "auth", "login"]         // JWT login
            | ["api", "users", "login"]        // legacy login (backward compat)
            | ["api", "users", "register"] // user registration (first-time setup)
            | ["api", "shared", _] // public share links (token-gated)
    );

    // Verify JWT for authenticated endpoints
    if requires_auth {
        if let Err(resp) = verify_jwt_auth(&dashboard.jwt_secret, auth_header, origin) {
            return resp;
        }
    }

    // ─── Sync run lifecycle (lockless) ───────────────────────────────
    // The sync pipeline acquires its own per-file locks, so a run must not
    // hold the request lock (it would deadlock). These arms therefore return
    // before the database lock is taken.
    match path_segments.as_slice() {
        ["api", "sync", "status"] if method == "GET" => {
            let progress = api::sync_api::progress(dashboard.sync_state());
            let status = serde_json::json!({
                "syncEnabled": true,
                "status": progress.status,
                "lastSync": progress.started_at,
                "provider": null,
            });
            return http_response(
                200,
                "application/json",
                &serde_json::to_string(&status).unwrap_or_default(),
                origin,
            );
        }
        ["api", "sync", "progress"] if method == "GET" => {
            return json_ok(&api::sync_api::progress(dashboard.sync_state()), origin);
        }
        ["api", "sync", "cancel"] if method == "POST" => {
            return json_ok(&api::sync_api::cancel(dashboard.sync_state()), origin);
        }
        ["api", "sync", "start"] if method == "POST" => {
            let req: api::sync_api::StartRequest = json_body!(body, origin);
            return api_response(
                api::sync_api::start(
                    db,
                    &dashboard.compression,
                    dashboard.sync_state(),
                    &req.config_id,
                    req.file_ids,
                ),
                origin,
            );
        }
        ["api", "sync", "test"] if method == "POST" => {
            let req: api::sync_api::ConfigRequest = json_body!(body, origin);
            return api_response(api::sync_api::test_connection(&req.config), origin);
        }
        ["api", "sync", "remote-files"] if method == "POST" => {
            let req: api::sync_api::RemoteFilesRequest = json_body!(body, origin);
            return api_response(
                api::sync_api::list_remote_files(&req.config, &req.prefix),
                origin,
            );
        }
        _ => {}
    }

    // Take the database lock for the duration of the request. Readers share
    // the lock; writers (POST/PUT/DELETE) take it exclusively. A poisoned lock
    // is recovered from rather than propagated, so one panicking request can
    // not wedge every later one.
    let lock = if matches!(method, "GET" | "HEAD") {
        match db.read() {
            Ok(guard) => DbLock::Read(guard),
            Err(poisoned) => DbLock::Read(poisoned.into_inner()),
        }
    } else {
        match db.write() {
            Ok(guard) => DbLock::Write(guard),
            Err(poisoned) => DbLock::Write(poisoned.into_inner()),
        }
    };
    let db: &Database = lock.db();

    // Route to appropriate handler
    match path_segments.as_slice() {
        // ─── Auth endpoint (JWT login) ────────────────────────────
        ["api", "auth", "login"] | ["api", "users", "login"] if method == "POST" => {
            login_user(db, body, &dashboard.jwt_secret, origin)
        }

        // ─── User registration ────────────────────────────────────
        ["api", "users", "register"] if method == "POST" => register_user_web(db, body, origin),

        // ─── File endpoints ───────────────────────────────────────
        ["api", "files"] if method == "GET" => {
            list_all_json(db, Database::get_files_table(), origin)
        }
        ["api", "files", "folder"] if method == "POST" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct CreateFolderBody {
                name: String,
                parent_id: String,
            }
            let req: CreateFolderBody = json_body!(body, origin);
            api_response(
                api::files::create_folder(db, req.name, req.parent_id),
                origin,
            )
        }
        ["api", "files", "rebuild-index"] if method == "POST" => {
            api_response(api::files::rebuild_parent_index(db), origin)
        }
        ["api", "files", id] if method == "GET" => {
            get_by_id(db, Database::get_files_table(), id, origin)
        }
        ["api", "files", id] if method == "DELETE" => {
            api_response(api::files::delete(db, id), origin)
        }
        ["api", "files", id, "rename"] if method == "POST" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RenameBody {
                new_name: String,
            }
            let req: RenameBody = json_body!(body, origin);
            api_response(api::files::rename(db, id, req.new_name), origin)
        }
        ["api", "files", id, "move"] if method == "POST" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct MoveBody {
                parent_id: String,
            }
            let req: MoveBody = json_body!(body, origin);
            api_response(api::files::move_to(db, id, req.parent_id), origin)
        }
        ["api", "files", id, "duplicate"] if method == "POST" => {
            api_response(api::files::duplicate(db, id), origin)
        }
        ["api", "files", id, "preview"] if method == "GET" => {
            api_response(api::files::preview(db, id), origin)
        }
        ["api", "files", id, "versions"] if method == "GET" => {
            api_response(api::versions::list(db, id), origin)
        }
        ["api", "files", id, "versions"] if method == "POST" => {
            api_response(api::versions::create(db, id), origin)
        }
        ["api", "files", id, "versions", version_id, "revert"] if method == "POST" => {
            api_response(api::versions::revert(db, id, version_id), origin)
        }

        // ─── Trash endpoints ─────────────────────────────────────
        ["api", "trash"] if method == "GET" => api_response(api::trash::list(db), origin),
        ["api", "trash"] if method == "DELETE" => api_response(api::trash::empty(db), origin),
        ["api", "trash", id] if method == "DELETE" => {
            api_response(api::trash::delete(db, id), origin)
        }
        ["api", "trash", id, "restore"] if method == "POST" => {
            api_response(api::trash::restore(db, id), origin)
        }

        // ─── Version snapshots ───────────────────────────────────
        ["api", "versions", "snapshot-all"] if method == "POST" => {
            api_response(api::versions::snapshot_all(db), origin)
        }

        // ─── Audit log ───────────────────────────────────────────
        ["api", "audit"] if method == "GET" => {
            let limit = parse_query_param(query, "limit").and_then(|v| v.parse::<u32>().ok());
            let entity_type = parse_query_param(query, "entityType");
            api_response(api::audit::list(db, limit, entity_type.as_deref()), origin)
        }

        // ─── Share links ─────────────────────────────────────────
        ["api", "share-links"] if method == "GET" => api_response(api::share::list(db), origin),
        ["api", "share-links"] if method == "POST" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct ShareBody {
                file_id: String,
                expires_in_hours: Option<u64>,
            }
            let req: ShareBody = json_body!(body, origin);
            api_response(
                api::share::generate(db, &req.file_id, req.expires_in_hours),
                origin,
            )
        }
        ["api", "shared", token] if method == "GET" => match api::share::resolve(db, token) {
            Ok(Some(node)) => json_ok(&node, origin),
            Ok(None) => json_error(404, "Share link not found", origin),
            Err(e) => api_response::<serde_json::Value>(Err(e), origin),
        },

        // ─── Batch operations ────────────────────────────────────
        ["api", "batch", "delete"] if method == "POST" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct BatchBody {
                file_ids: Vec<String>,
            }
            let req: BatchBody = json_body!(body, origin);
            api_response(api::batch::delete(db, &req.file_ids), origin)
        }
        ["api", "batch", "encrypt"] if method == "POST" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct BatchBody {
                file_ids: Vec<String>,
                algorithm: String,
            }
            let req: BatchBody = json_body!(body, origin);
            api_response(
                api::batch::encrypt(db, &req.file_ids, &req.algorithm),
                origin,
            )
        }
        ["api", "batch", "compress"] if method == "POST" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct BatchBody {
                file_ids: Vec<String>,
                layer: String,
            }
            let req: BatchBody = json_body!(body, origin);
            api_response(api::batch::compress(db, &req.file_ids, &req.layer), origin)
        }

        // ─── Account endpoints ────────────────────────────────────
        ["api", "accounts"] if method == "GET" => {
            list_all_json(db, Database::get_accounts_table(), origin)
        }
        ["api", "accounts"] if method == "POST" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct CreateAccountBody {
                name: String,
                account_type: String,
                path: Option<String>,
                color: Option<String>,
            }
            let req: CreateAccountBody = json_body!(body, origin);
            api_response(
                api::accounts::create(db, req.name, req.account_type, req.path, req.color),
                origin,
            )
        }
        ["api", "accounts", id, "switch"] if method == "POST" => {
            api_response(api::accounts::switch(db, id), origin)
        }
        ["api", "accounts", id] if method == "DELETE" => {
            api_response(api::accounts::delete(db, id), origin)
        }

        // ─── Collection endpoints ─────────────────────────────────
        ["api", "collections"] if method == "GET" => {
            list_all_json(db, Database::get_collections_table(), origin)
        }
        ["api", "collections"] if method == "POST" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct CreateCollectionBody {
                name: String,
                collection_type: String,
                color: String,
                description: Option<String>,
            }
            let req: CreateCollectionBody = json_body!(body, origin);
            api_response(
                api::collections::create(
                    db,
                    req.name,
                    req.collection_type,
                    req.color,
                    req.description,
                ),
                origin,
            )
        }
        ["api", "collections", id, "items"] if method == "POST" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct AddItemBody {
                file_id: String,
                note: Option<String>,
            }
            let req: AddItemBody = json_body!(body, origin);
            api_response(
                api::collections::add_item(db, id, &req.file_id, req.note),
                origin,
            )
        }
        ["api", "collections", id, "items", file_id] if method == "DELETE" => {
            api_response(api::collections::remove_item(db, id, file_id), origin)
        }
        ["api", "collection-items"] if method == "GET" => {
            list_all_json(db, Database::get_collection_items_table(), origin)
        }

        // ─── Face group endpoints ─────────────────────────────────
        ["api", "face-groups"] if method == "GET" => {
            list_all_json(db, Database::get_face_groups_table(), origin)
        }

        // ─── Loose group endpoints ────────────────────────────────
        ["api", "loose-groups"] if method == "GET" => {
            list_all_json(db, Database::get_loose_groups_table(), origin)
        }

        // ─── Encryption endpoints ─────────────────────────────────
        ["api", "encryption", "status"] if method == "GET" => {
            let status = serde_json::json!({
                "available": true,
                "supported_algorithms": ["kyber512", "kyber768", "kyber1024", "hybrid", "ml_dsa44", "ml_dsa65", "ml_dsa87", "classical_sign"],
                "engine": "pqcrypto-mlkem (ML-KEM FIPS 203) + ml-dsa (ML-DSA FIPS 204) post-quantum cryptography"
            });
            http_response(
                200,
                "application/json",
                &serde_json::to_string(&status).unwrap_or_default(),
                origin,
            )
        }
        ["api", "encryption", "keys"] if method == "GET" => {
            // SECURITY: Never expose private keys
            list_encryption_keys_safe(db, origin)
        }

        // ─── Geo files ───────────────────────────────────────────
        ["api", "geo-files"] if method == "GET" => list_geo_files(db, origin),

        // ─── Search ───────────────────────────────────────────────
        ["api", "search", "suggest"] if method == "GET" => {
            let prefix = parse_query_param(query, "q").unwrap_or_default();
            let limit = parse_query_param(query, "limit")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(10);
            api_response(
                api::search_api::suggest(&dashboard.search_index, db, &prefix, limit),
                origin,
            )
        }
        ["api", "search", "paginated"] if method == "GET" => {
            let q = parse_query_param(query, "q").unwrap_or_default();
            let limit = parse_query_param(query, "limit")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(20);
            let offset = parse_query_param(query, "offset")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            api_response(
                api::search_api::search_paginated(&dashboard.search_index, db, &q, limit, offset),
                origin,
            )
        }
        ["api", "search"] if method == "GET" => {
            let q = parse_query_param(query, "q").unwrap_or_default();
            let limit = parse_query_param(query, "limit").and_then(|v| v.parse::<usize>().ok());
            let offset = parse_query_param(query, "offset").and_then(|v| v.parse::<usize>().ok());
            api_response(
                api::search_api::search(&dashboard.search_index, db, &q, limit, offset),
                origin,
            )
        }

        // ─── Location endpoints ───────────────────────────────────
        ["api", "locations"] if method == "GET" => {
            list_all_json(db, Database::get_locations_table(), origin)
        }

        // ─── User endpoints ──────────────────────────────────────
        ["api", "users"] if method == "GET" => list_users_safe(db, origin),
        ["api", "users"] if method == "POST" => register_user_web(db, body, origin),
        ["api", "users", id] if method == "DELETE" => {
            api_response(api::users::delete(db, id), origin)
        }
        ["api", "users", id, "role"] if method == "POST" => {
            #[derive(Deserialize)]
            struct RoleBody {
                role: String,
            }
            let req: RoleBody = json_body!(body, origin);
            match api::users::update_role(db, id, req.role) {
                Ok(mut user) => {
                    user.password_hash.clear();
                    json_ok(&user, origin)
                }
                Err(e) => api_response::<cybermanju_types::schema::User>(Err(e), origin),
            }
        }

        // ─── Permission endpoints ─────────────────────────────────
        ["api", "permissions"] if method == "POST" => set_permission_web(db, body, origin),
        ["api", "permissions", "verify"] if method == "POST" => verify_access_web(db, body, origin),
        ["api", "permissions", file_id] if method == "GET" => {
            get_permissions_for_file(db, file_id, origin)
        }

        // ─── Sync config endpoints ────────────────────────────────
        ["api", "sync", "configs"] if method == "GET" => {
            api_response(api::sync_api::list_configs(db), origin)
        }
        ["api", "sync", "configs"] if method == "POST" => {
            let req: api::sync_api::ConfigRequest = json_body!(body, origin);
            api_response(api::sync_api::save_config(db, req.config), origin)
        }
        ["api", "sync", "configs", id] if method == "DELETE" => {
            api_response(api::sync_api::delete_config(db, id), origin)
        }

        // ─── Dashboard status ────────────────────────────────────
        ["api", "dashboard", "status"] if method == "GET" => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            // Shape matches the Tauri `DashboardStatus` command so the
            // frontend can use one mapping in every transport.
            let status = serde_json::json!({
                "service": "Cybermanju Drive Web Dashboard",
                "running": dashboard.running.load(Ordering::SeqCst),
                "port": dashboard.port,
                "url": format!("http://localhost:{}", dashboard.port),
                "activeConnections": dashboard.active_connections.load(Ordering::SeqCst),
                "bindAddress": dashboard.bind_addr,
                "timestamp": now,
                "version": "1.0.0",
            });
            http_response(
                200,
                "application/json",
                &serde_json::to_string(&status).unwrap_or_default(),
                origin,
            )
        }

        // ─── Root / health check ─────────────────────────────────
        ["api"] | ["api", "health"] if method == "GET" => {
            let health = serde_json::json!({
                "service": "Cybermanju Drive Web Dashboard",
                "status": "ok",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            });
            http_response(
                200,
                "application/json",
                &serde_json::to_string(&health).unwrap_or_default(),
                origin,
            )
        }

        // ─── 404 ─────────────────────────────────────────────────
        _ => json_error(404, &format!("Not found: {} {}", method, path), origin),
    }
}

// ─── JWT Authentication ─────────────────────────────────────────────

/// Verify the JWT token from the Authorization header.
/// Returns Ok(()) on success, Err(http_response_string) on failure.
fn verify_jwt_auth(
    jwt_secret: &[u8; 32],
    auth_header: Option<&str>,
    origin: Option<&str>,
) -> Result<(), String> {
    let token = match auth_header {
        Some(h) => {
            // Expected format: "Bearer <token>"
            if let Some(t) = h
                .strip_prefix("Bearer ")
                .or_else(|| h.strip_prefix("bearer "))
            {
                t.trim()
            } else {
                return Err(json_error(
                    401,
                    "Missing or invalid Authorization header format. Expected: Bearer <token>",
                    origin,
                ));
            }
        }
        None => {
            return Err(json_error(401, "Authorization header required", origin));
        }
    };

    let decoding_key = DecodingKey::from_secret(jwt_secret);
    match decode::<JwtClaims>(token, &decoding_key, &Validation::default()) {
        Ok(_token_data) => Ok(()),
        Err(e) => Err(json_error(
            401,
            &format!("Invalid or expired token: {}", e),
            origin,
        )),
    }
}

/// Create a JWT token for an authenticated user.
fn create_jwt(
    jwt_secret: &[u8; 32],
    user_id: &str,
    username: &str,
    role: &str,
) -> Result<String, String> {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let claims = JwtClaims {
        sub: username.to_string(),
        role: role.to_string(),
        user_id: user_id.to_string(),
        iat: now_secs,
        exp: now_secs + JWT_EXPIRY_SECS,
    };

    let encoding_key = EncodingKey::from_secret(jwt_secret);
    encode(&Header::default(), &claims, &encoding_key)
        .map_err(|e| format!("JWT encoding error: {}", e))
}

// ─── Rate Limiting ──────────────────────────────────────────────────

/// Check and update the rate limit for a client IP.
/// Returns true if the request is allowed, false if rate limited.
pub fn check_rate_limit(
    rate_limits: &Mutex<HashMap<String, (u32, Instant)>>,
    client_ip: &str,
) -> bool {
    let mut limits = match rate_limits.lock() {
        Ok(g) => g,
        Err(_) => return false, // If lock is poisoned, allow the request (fail open)
    };
    let now = Instant::now();

    // Clean up stale entries (older than 2x the window)
    limits.retain(|_, (_, ts)| now.duration_since(*ts).as_secs() < RATE_LIMIT_WINDOW_SECS * 2);

    let entry = limits.entry(client_ip.to_string()).or_insert((0, now));

    // Reset window if expired
    if now.duration_since(entry.1).as_secs() >= RATE_LIMIT_WINDOW_SECS {
        *entry = (0, now);
    }

    entry.0 += 1;
    entry.0 <= RATE_LIMIT_MAX
}

// ─── CORS ───────────────────────────────────────────────────────────

/// Build the CORS preflight response (OPTIONS).
fn cors_preflight_response(origin: Option<&str>) -> String {
    let cors_headers = match origin {
        Some(o) if !o.is_empty() => {
            format!(
                "Access-Control-Allow-Origin: {}\r\n\
                 Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
                 Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
                 Access-Control-Max-Age: 86400\r\n",
                o
            )
        }
        _ => String::new(),
    };

    format!(
        "HTTP/1.1 204 No Content\r\n\
         {cors_headers}\
         Content-Length: 0\r\n\
         \r\n"
    )
}

// ─── Generic table operations ────────────────────────────────────────

/// List all JSON values from a table.
fn list_all_json(
    db: &Database,
    table_def: TableDefinition<'static, &'static str, &'static str>,
    origin: Option<&str>,
) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(table_def) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    let iter = match table.iter() {
        Ok(i) => i,
        Err(e) => return json_error(500, &format!("Iteration error: {}", e), origin),
    };
    for entry in iter {
        match entry {
            Ok((key, value)) => {
                let key_str = key.value().to_string();
                let val_str = value.value().to_string();
                if let Ok(mut obj) = serde_json::from_str::<serde_json::Value>(&val_str) {
                    if let Some(map) = obj.as_object_mut() {
                        map.insert("_key".to_string(), serde_json::json!(key_str));
                    }
                    results.push(obj);
                } else {
                    results.push(serde_json::json!({
                        "_key": key_str,
                        "_raw": val_str,
                    }));
                }
            }
            Err(e) => {
                return json_error(500, &format!("Iteration error: {}", e), origin);
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body, origin)
}

/// List encryption keys with private_key fields STRIPPED for security.
fn list_encryption_keys_safe(db: &Database, origin: Option<&str>) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(Database::get_encryption_keys_table()) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    let iter = match table.iter() {
        Ok(i) => i,
        Err(e) => return json_error(500, &format!("Iteration error: {}", e), origin),
    };
    for entry in iter {
        match entry {
            Ok((key, value)) => {
                let key_str = key.value().to_string();
                let val_str = value.value().to_string();
                if let Ok(mut obj) = serde_json::from_str::<serde_json::Value>(&val_str) {
                    if let Some(map) = obj.as_object_mut() {
                        map.insert("_key".to_string(), serde_json::json!(key_str));
                        // SECURITY: Strip all private key variants
                        map.remove("private_key");
                        map.remove("privateKey");
                        map.remove("secret_key");
                        map.remove("secretKey");
                        map.remove("private_key_encrypted");
                        map.remove("privateKeyEncrypted");
                    }
                    results.push(obj);
                } else {
                    results.push(serde_json::json!({
                        "_key": key_str,
                        "_raw": val_str,
                    }));
                }
            }
            Err(e) => {
                return json_error(500, &format!("Iteration error: {}", e), origin);
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body, origin)
}

/// Get a single entry by key from a table.
fn get_by_id(
    db: &Database,
    table_def: TableDefinition<'static, &'static str, &'static str>,
    id: &str,
    origin: Option<&str>,
) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(table_def) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    match table.get(id) {
        Ok(Some(value)) => {
            let val_str = value.value().to_string();
            http_response(200, "application/json", &val_str, origin)
        }
        Ok(None) => json_error(404, &format!("Not found: {}", id), origin),
        Err(e) => json_error(500, &format!("Get error: {}", e), origin),
    }
}

// ─── Domain-specific handlers ────────────────────────────────────────

/// List files that have GPS coordinates.
fn list_geo_files(db: &Database, origin: Option<&str>) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(Database::get_files_table()) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    let iter = match table.iter() {
        Ok(i) => i,
        Err(e) => return json_error(500, &format!("Iteration error: {}", e), origin),
    };
    for (_, value) in iter.flatten() {
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(value.value()) {
            let has_lat = obj.get("gpsLat").and_then(|v| v.as_f64()).is_some();
            let has_lng = obj.get("gpsLon").and_then(|v| v.as_f64()).is_some();
            if has_lat && has_lng {
                results.push(obj);
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body, origin)
}

// ─── User management handlers ────────────────────────────────────────

/// List users (without password hashes).
fn list_users_safe(db: &Database, origin: Option<&str>) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(Database::get_users_table()) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    let iter = match table.iter() {
        Ok(i) => i,
        Err(e) => return json_error(500, &format!("Iteration error: {}", e), origin),
    };
    for (_, value) in iter.flatten() {
        if let Ok(mut obj) = serde_json::from_str::<serde_json::Value>(value.value()) {
            // Strip password hashes for safety
            if let Some(map) = obj.as_object_mut() {
                map.remove("passwordHash");
                map.remove("password_hash");
            }
            results.push(obj);
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body, origin)
}

/// Login endpoint — expects JSON body: { "username": "...", "password": "..." }
/// Verifies argon2 password hash and returns a JWT token.
fn login_user(db: &Database, body: &str, jwt_secret: &[u8; 32], origin: Option<&str>) -> String {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e), origin),
    };

    let username = req.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");

    if username.is_empty() || password.is_empty() {
        return json_error(400, "username and password are required", origin);
    }

    // Find user by username
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(Database::get_users_table()) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut found_user: Option<serde_json::Value> = None;
    let iter = match table.iter() {
        Ok(i) => i,
        Err(e) => return json_error(500, &format!("Iteration error: {}", e), origin),
    };
    for (_, value) in iter.flatten() {
        if let Ok(user) = serde_json::from_str::<serde_json::Value>(value.value()) {
            if user.get("username").and_then(|v| v.as_str()) == Some(username) {
                found_user = Some(user);
                break;
            }
        }
    }
    drop(tx);

    let user = match found_user {
        Some(u) => u,
        None => return json_error(401, "Invalid credentials", origin),
    };

    // Check active status
    if user.get("isActive").and_then(|v| v.as_bool()) == Some(false) {
        return json_error(403, "User account is deactivated", origin);
    }

    // Verify password using argon2
    let password_hash = match user.get("passwordHash").and_then(|v| v.as_str()) {
        Some(h) => h,
        None => return json_error(500, "User record has no password hash", origin),
    };

    match argon2_verify(password, password_hash) {
        Ok(true) => {
            // Issue JWT token instead of insecure blake3 hash
            let user_id = user
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let role = user
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("user")
                .to_string();
            let display_name = user
                .get("displayName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let token = match create_jwt(jwt_secret, &user_id, username, &role) {
                Ok(t) => t,
                Err(e) => return json_error(500, &e, origin),
            };

            let response = serde_json::json!({
                "userId": user_id,
                "username": username,
                "role": role,
                "displayName": display_name,
                "token": token,
                "tokenType": "Bearer",
                "expiresIn": JWT_EXPIRY_SECS,
            });
            http_response(
                200,
                "application/json",
                &serde_json::to_string(&response).unwrap_or_default(),
                origin,
            )
        }
        Ok(false) => json_error(401, "Invalid credentials", origin),
        Err(e) => json_error(500, &format!("Password verification error: {}", e), origin),
    }
}

/// Register endpoint — expects JSON body:
/// Register a user from a JSON body — shared by `/api/users/register`
/// (first-run setup) and `POST /api/users` (admin creation).
fn register_user_web(db: &Database, body: &str, origin: Option<&str>) -> String {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RegisterBody {
        username: String,
        password: String,
        display_name: Option<String>,
        role: Option<String>,
    }

    let req: RegisterBody = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e), origin),
    };

    match api::users::register(db, req.username, req.password, req.display_name, req.role) {
        Ok(mut user) => {
            // SECURITY: never return the password hash to a client
            user.password_hash.clear();
            http_response(
                201,
                "application/json",
                &serde_json::to_string(&user).unwrap_or_default(),
                origin,
            )
        }
        Err(e) => {
            let status = if e.contains("already exists") {
                409
            } else {
                400
            };
            json_error(status, &e, origin)
        }
    }
}

fn set_permission_web(db: &Database, body: &str, origin: Option<&str>) -> String {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e), origin),
    };

    let user_id = req.get("userId").and_then(|v| v.as_str()).unwrap_or("");
    let file_id = req.get("fileId").and_then(|v| v.as_str()).unwrap_or("");
    let access = req.get("access").and_then(|v| v.as_str()).unwrap_or("");

    if user_id.is_empty() || file_id.is_empty() || access.is_empty() {
        return json_error(400, "userId, fileId, and access are required", origin);
    }

    if !["read", "write", "admin"].contains(&access) {
        return json_error(400, "access must be 'read', 'write', or 'admin'", origin);
    }

    let perm_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let permission = serde_json::json!({
        "id": perm_id,
        "userId": user_id,
        "fileId": file_id,
        "access": access,
        "grantedBy": "system",
        "grantedAt": now,
    });

    let perm_json = match serde_json::to_string(&permission) {
        Ok(s) => s,
        Err(e) => return json_error(500, &format!("Serialization error: {}", e), origin),
    };

    let tx = match db.begin_write() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Write error: {}", e), origin),
    };
    {
        let mut table = match tx.open_table(Database::get_user_file_perms_table()) {
            Ok(t) => t,
            Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
        };
        if table.insert(perm_id.as_str(), perm_json.as_str()).is_err() {
            return json_error(500, "Failed to insert permission", origin);
        }
    }
    if tx.commit().is_err() {
        return json_error(500, "Failed to commit permission", origin);
    }

    http_response(
        201,
        "application/json",
        &serde_json::to_string(&permission).unwrap_or_default(),
        origin,
    )
}

/// Verify file access — expects JSON body:
/// { "user_id": "...", "file_id": "...", "required_access": "read|write|admin" }
fn verify_access_web(db: &Database, body: &str, origin: Option<&str>) -> String {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e), origin),
    };

    let user_id = req.get("userId").and_then(|v| v.as_str()).unwrap_or("");
    let file_id = req.get("fileId").and_then(|v| v.as_str()).unwrap_or("");
    let required_access = req
        .get("requiredAccess")
        .and_then(|v| v.as_str())
        .unwrap_or("read");

    // Check user role (admin bypasses all permission checks)
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let users_table = match tx.open_table(Database::get_users_table()) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    if let Ok(Some(val)) = users_table.get(user_id) {
        if let Ok(user) = serde_json::from_str::<serde_json::Value>(val.value()) {
            if user.get("role").and_then(|v| v.as_str()) == Some("admin") {
                let resp = serde_json::json!({
                    "userId": user_id,
                    "fileId": file_id,
                    "requiredAccess": required_access,
                    "granted": true,
                    "reason": "admin_role"
                });
                return http_response(
                    200,
                    "application/json",
                    &serde_json::to_string(&resp).unwrap_or_default(),
                    origin,
                );
            }
        }
    }

    // Check explicit permissions
    let perms_table = match tx.open_table(Database::get_user_file_perms_table()) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let iter = match perms_table.iter() {
        Ok(i) => i,
        Err(e) => return json_error(500, &format!("Iteration error: {}", e), origin),
    };
    for (_, value) in iter.flatten() {
        if let Ok(perm) = serde_json::from_str::<serde_json::Value>(value.value()) {
            let p_user = perm.get("userId").and_then(|v| v.as_str()).unwrap_or("");
            let p_file = perm.get("fileId").and_then(|v| v.as_str()).unwrap_or("");
            let p_access = perm.get("access").and_then(|v| v.as_str()).unwrap_or("");

            if p_user == user_id
                && p_file == file_id
                && access_level_sufficient(p_access, required_access)
            {
                let resp = serde_json::json!({
                    "userId": user_id,
                    "fileId": file_id,
                    "requiredAccess": required_access,
                    "granted": true,
                    "reason": "permission_match"
                });
                return http_response(
                    200,
                    "application/json",
                    &serde_json::to_string(&resp).unwrap_or_default(),
                    origin,
                );
            }
        }
    }

    let resp = serde_json::json!({
        "userId": user_id,
        "fileId": file_id,
        "requiredAccess": required_access,
        "granted": false,
        "reason": "no_matching_permission"
    });
    http_response(
        200,
        "application/json",
        &serde_json::to_string(&resp).unwrap_or_default(),
        origin,
    )
}

/// Get all permissions for a specific file.
fn get_permissions_for_file(db: &Database, file_id: &str, origin: Option<&str>) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(Database::get_user_file_perms_table()) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    let iter = match table.iter() {
        Ok(i) => i,
        Err(e) => return json_error(500, &format!("Iteration error: {}", e), origin),
    };
    for (_, value) in iter.flatten() {
        if let Ok(perm) = serde_json::from_str::<serde_json::Value>(value.value()) {
            if perm.get("fileId").and_then(|v| v.as_str()) == Some(file_id) {
                results.push(perm);
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body, origin)
}

// ─── Argon2 helpers ──────────────────────────────────────────────────

fn argon2_verify(password: &str, hash: &str) -> Result<bool, String> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed = PasswordHash::new(hash).map_err(|e| format!("Invalid hash: {}", e))?;
    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

// ─── Access level helper ──────────────────────────────────────────────

fn access_level_sufficient(granted: &str, required: &str) -> bool {
    let levels: &[&str] = &["read", "write", "admin"];
    let g_idx = levels.iter().position(|&l| l == granted).unwrap_or(0);
    let r_idx = levels.iter().position(|&l| l == required).unwrap_or(0);
    g_idx >= r_idx
}

// ─── HTTP response builder ────────────────────────────────────────────

/// Build an HTTP response with restricted CORS headers.
/// CORS headers are only included if the origin matches ALLOWED_ORIGINS.
pub fn http_response(status: u16, content_type: &str, body: &str, origin: Option<&str>) -> String {
    let status_text = match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "OK",
    };

    let cors_headers = match origin {
        Some(o) if !o.is_empty() => {
            format!(
                "Access-Control-Allow-Origin: {}\r\n\
                 Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
                 Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n",
                o
            )
        }
        _ => String::new(),
    };

    format!(
        "HTTP/1.1 {status} {status_text}\r\n\
         Content-Type: {content_type}\r\n\
         {cors_headers}\
         Content-Length: {}\r\n\
         \r\n\
         {body}",
        body.len()
    )
}

/// Build a JSON error response.
fn json_error(status: u16, message: &str, origin: Option<&str>) -> String {
    let body = serde_json::json!({
        "error": true,
        "status": status,
        "message": message,
    });
    let body_str = serde_json::to_string(&body).unwrap_or_else(|_| message.to_string());
    http_response(status, "application/json", &body_str, origin)
}

// ─── Query string parser ────────────────────────────────────────────

/// Parse a query parameter value from a query string (e.g., "q=search&limit=10").
fn parse_query_param(query: &str, param: &str) -> Option<String> {
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            if key == param {
                return Some(url_decode(value));
            }
        }
    }
    None
}

/// Basic URL percent-decoding.
fn url_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

// ─── Static file serving (for Docker use case) ─────────────────────

/// MIME type lookup for common file extensions
pub fn mime_type(path: &str) -> &'static str {
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

/// Serve a static file, writing binary-safe HTTP response directly to the stream.
/// Returns true if the file was served, false if a text-based error was written.
#[allow(dead_code)]
pub fn serve_static_file(stream: &mut TcpStream, static_dir: &std::path::Path, request_path: &str) {
    use std::fs;

    let file_path = if request_path == "/" || request_path.ends_with('/') {
        static_dir.join("index.html")
    } else {
        static_dir.join(request_path.trim_start_matches('/'))
    };

    // Security: prevent path traversal
    let resolved = match file_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\n\
                  Content-Type: text/html; charset=utf-8\r\n\
                  Content-Length: 44\r\n\
                  \r\n\
                  <html><body><h1>404 Not Found</h1></body></html>",
            );
            return;
        }
    };

    let static_resolved = match static_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let _ = stream.write_all(
                b"HTTP/1.1 500 Internal Server Error\r\n\
                  Content-Type: text/html; charset=utf-8\r\n\
                  Content-Length: 52\r\n\
                  \r\n\
                  <html><body><h1>500 Internal Server Error</h1></body></html>",
            );
            return;
        }
    };

    if !resolved.starts_with(&static_resolved) {
        let _ = stream.write_all(
            b"HTTP/1.1 403 Forbidden\r\n\
              Content-Type: text/html; charset=utf-8\r\n\
              Content-Length: 44\r\n\
              \r\n\
              <html><body><h1>403 Forbidden</h1></body></html>",
        );
        return;
    }

    // SPA fallback: if the file doesn't exist, serve index.html
    let actual_path = if resolved.is_file() {
        resolved
    } else {
        let index = static_dir.join("index.html");
        if index.is_file() {
            index
        } else {
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\n\
                  Content-Type: text/html; charset=utf-8\r\n\
                  Content-Length: 44\r\n\
                  \r\n\
                  <html><body><h1>404 Not Found</h1></body></html>",
            );
            return;
        }
    };

    let contents = match fs::read(&actual_path) {
        Ok(c) => c,
        Err(_) => {
            let _ = stream.write_all(
                b"HTTP/1.1 500 Internal Server Error\r\n\
                  Content-Type: text/html; charset=utf-8\r\n\
                  Content-Length: 52\r\n\
                  \r\n\
                  <html><body><h1>500 Internal Server Error</h1></body></html>",
            );
            return;
        }
    };

    let ctype = mime_type(&actual_path.to_string_lossy());
    let content_length = contents.len();

    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {ctype}\r\n\
         Content-Length: {content_length}\r\n\
         Cache-Control: public, max-age=3600\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
         \r\n"
    );

    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(&contents);
}

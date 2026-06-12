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

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use log::{error, info, warn};
use rand_core::{OsRng, RngCore};
use redb::{Database as RedbDb, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

// ─── Table definitions (must match db/mod.rs) ────────────────────────

const FILES_TABLE: TableDefinition<&str, &str> = TableDefinition::new("files");
const ACCOUNTS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("accounts");
const COLLECTIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("collections");
const COLLECTION_ITEMS_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("collection_items");
const FACE_GROUPS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("face_groups");
const LOOSE_GROUPS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("loose_groups");
const ENCRYPTION_KEYS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("encryption_keys");
const LOCATIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("locations");
const USERS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("users");
const USER_FILE_PERMS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("user_file_perms");

// ─── Security constants ─────────────────────────────────────────────

/// Maximum request body size: 100 MB
const MAX_BODY_SIZE: usize = 104_857_600;

/// Rate limit: max requests per window per IP
const RATE_LIMIT_MAX: u32 = 100;

/// Default port for the web dashboard
pub const DEFAULT_PORT: u16 = 3456;

/// Rate limit window duration in seconds
const RATE_LIMIT_WINDOW_SECS: u64 = 60;

/// JWT token expiry: 24 hours
const JWT_EXPIRY_SECS: u64 = 86_400;

/// Allowed CORS origins (localhost only)
const ALLOWED_ORIGINS: &[&str] = &[
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
    port: u16,
    /// Always "127.0.0.1" — never bind to 0.0.0.0
    bind_addr: String,
    /// Random 256-bit secret generated at startup for HMAC-SHA256 JWT signing
    jwt_secret: [u8; 32],
    /// Shared database handle — opened once, shared across all request threads.
    /// Using Arc<Mutex<>> since redb requires exclusive access for writes.
    db: Arc<Mutex<RedbDb>>,
    running: AtomicBool,
    /// Per-IP rate limit counters: IP → (count, window_start)
    rate_limits: Mutex<HashMap<String, (u32, Instant)>>,
    /// Handle to the server accept thread — joined on Drop
    server_thread: Mutex<Option<thread::JoinHandle<()>>>,
    /// Shutdown signal sender — drop or close to signal the server loop
    shutdown_tx: Mutex<Option<mpsc::Sender<()>>>,
}

impl WebDashboard {
    pub fn new(port: u16, db_path: &str) -> Self {
        // Generate a cryptographically random 256-bit JWT secret
        let mut jwt_secret = [0u8; 32];
        OsRng.fill_bytes(&mut jwt_secret);

        // Open the database once and share it across all request threads
        let db = RedbDb::open(db_path)
            .or_else(|_| RedbDb::create(db_path))
            .expect("Failed to open web dashboard database");

        Self {
            port,
            bind_addr: "127.0.0.1".to_string(),
            jwt_secret,
            db: Arc::new(Mutex::new(db)),
            running: AtomicBool::new(false),
            rate_limits: Mutex::new(HashMap::new()),
            server_thread: Mutex::new(None),
            shutdown_tx: Mutex::new(None),
        }
    }

    /// Start the web dashboard HTTP server on a background thread.
    /// Binds to 127.0.0.1 only. Returns Ok(()) on successful bind.
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

fn handle_connection(dashboard: &WebDashboard, mut stream: TcpStream) {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(5))).ok();

    // Extract client IP for rate limiting
    let client_ip = stream
        .peer_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // ── Rate limiting check ──
    if !check_rate_limit(&dashboard.rate_limits, &client_ip) {
        let body = r#"{"error":true,"status":429,"message":"Rate limit exceeded"}"#;
        let resp = format!(
            "HTTP/1.1 429 Too Many Requests\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

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
        let body = r#"{"error":true,"status":413,"message":"Request body too large"}"#;
        let resp = format!(
            "HTTP/1.1 413 Payload Too Large\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    // Read body if present (capped to MAX_BODY_SIZE for safety)
    let body = if content_length > 0 {
        let read_size = content_length.min(MAX_BODY_SIZE);
        let mut buf = vec![0u8; read_size];
        if std::io::Read::read_exact(&mut reader, &mut buf).is_err() {
            String::new()
        } else if let Ok(s) = String::from_utf8(buf) {
            s
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Determine effective CORS origin for response headers
    let effective_origin = origin_header
        .as_deref()
        .filter(|o| ALLOWED_ORIGINS.contains(o));

    // Handle the request — use the shared DB handle
    let db_guard = match dashboard.db.lock() {
        Ok(g) => g,
        Err(_) => return json_error(500, "Database lock poisoned", origin),
    };
    let response = handle_request(
        dashboard,
        &db_guard,
        method,
        path,
        &body,
        auth_header.as_deref(),
        effective_origin,
    );
    drop(db_guard);

    let _ = stream.write_all(response.as_bytes());
}

// ─── Request router ──────────────────────────────────────────────────

fn handle_request(
    dashboard: &WebDashboard,
    db: &RedbDb,
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
    );

    // Verify JWT for authenticated endpoints
    if requires_auth {
        if let Err(resp) = verify_jwt_auth(&dashboard.jwt_secret, auth_header, origin) {
            return resp;
        }
    }

    // Route to appropriate handler
    match path_segments.as_slice() {
        // ─── Auth endpoint (JWT login) ────────────────────────────
        ["api", "auth", "login"] | ["api", "users", "login"] if method == "POST" => {
            login_user(db, body, &dashboard.jwt_secret, origin)
        }

        // ─── User registration ────────────────────────────────────
        ["api", "users", "register"] if method == "POST" => register_user_web(db, body, origin),

        // ─── File endpoints ───────────────────────────────────────
        ["api", "files"] if method == "GET" => list_all_json(db, FILES_TABLE, origin),
        ["api", "files", id] if method == "GET" => get_by_id(db, FILES_TABLE, id, origin),
        ["api", "files", id] if method == "DELETE" => delete_by_id(db, FILES_TABLE, id, origin),

        // ─── Account endpoints ────────────────────────────────────
        ["api", "accounts"] if method == "GET" => list_all_json(db, ACCOUNTS_TABLE, origin),

        // ─── Collection endpoints ─────────────────────────────────
        ["api", "collections"] if method == "GET" => list_all_json(db, COLLECTIONS_TABLE, origin),
        ["api", "collection-items"] if method == "GET" => {
            list_all_json(db, COLLECTION_ITEMS_TABLE, origin)
        }

        // ─── Face group endpoints ─────────────────────────────────
        ["api", "face-groups"] if method == "GET" => list_all_json(db, FACE_GROUPS_TABLE, origin),

        // ─── Loose group endpoints ────────────────────────────────
        ["api", "loose-groups"] if method == "GET" => list_all_json(db, LOOSE_GROUPS_TABLE, origin),

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
        ["api", "search"] if method == "GET" => search_files(db, query, origin),

        // ─── Location endpoints ───────────────────────────────────
        ["api", "locations"] if method == "GET" => list_all_json(db, LOCATIONS_TABLE, origin),

        // ─── User endpoints ──────────────────────────────────────
        ["api", "users"] if method == "GET" => list_users_safe(db, origin),

        // ─── Permission endpoints ─────────────────────────────────
        ["api", "permissions"] if method == "POST" => set_permission_web(db, body, origin),
        ["api", "permissions", "verify"] if method == "POST" => verify_access_web(db, body, origin),
        ["api", "permissions", file_id] if method == "GET" => {
            get_permissions_for_file(db, file_id, origin)
        }

        // ─── Sync config endpoints ────────────────────────────────
        ["api", "sync", "configs"] if method == "GET" => {
            // Return empty array — sync configs not yet persisted to a table
            http_response(200, "application/json", "[]", origin)
        }
        ["api", "sync", "status"] if method == "GET" => {
            let status = serde_json::json!({
                "syncEnabled": false,
                "lastSync": null,
                "provider": null,
            });
            http_response(
                200,
                "application/json",
                &serde_json::to_string(&status).unwrap_or_default(),
                origin,
            )
        }

        // ─── Dashboard status ────────────────────────────────────
        ["api", "dashboard", "status"] if method == "GET" => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let status = serde_json::json!({
                "service": "Cybermanju Drive Web Dashboard",
                "running": dashboard.running.load(Ordering::SeqCst),
                "port": dashboard.port,
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
fn check_rate_limit(rate_limits: &Mutex<HashMap<String, (u32, Instant)>>, client_ip: &str) -> bool {
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
    db: &RedbDb,
    table_def: TableDefinition<&str, &str>,
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
    for entry in table.iter() {
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
fn list_encryption_keys_safe(db: &RedbDb, origin: Option<&str>) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(ENCRYPTION_KEYS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
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
    db: &RedbDb,
    table_def: TableDefinition<&str, &str>,
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

/// Delete an entry by key.
fn delete_by_id(
    db: &RedbDb,
    table_def: TableDefinition<&str, &str>,
    id: &str,
    origin: Option<&str>,
) -> String {
    let tx = match db.begin_write() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Write error: {}", e), origin),
    };
    let result = {
        let mut table = match tx.open_table(table_def) {
            Ok(t) => t,
            Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
        };
        match table.remove(id) {
            Ok(_) => true,
            Err(e) => return json_error(500, &format!("Remove error: {}", e), origin),
        }
    };
    if let Err(e) = tx.commit() {
        return json_error(500, &format!("Commit error: {}", e), origin);
    }
    let body = serde_json::to_string(&serde_json::json!({"deleted": result, "id": id}))
        .unwrap_or_default();
    http_response(200, "application/json", &body, origin)
}

// ─── Domain-specific handlers ────────────────────────────────────────

/// List files that have GPS coordinates.
fn list_geo_files(db: &RedbDb, origin: Option<&str>) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(FILES_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                let has_lat = obj.get("gpsLat").and_then(|v| v.as_f64()).is_some();
                let has_lng = obj.get("gpsLon").and_then(|v| v.as_f64()).is_some();
                if has_lat && has_lng {
                    results.push(obj);
                }
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body, origin)
}

/// Simple search across file names and tags by scanning all entries.
/// Query param: ?q=search_term
fn search_files(db: &RedbDb, query: &str, origin: Option<&str>) -> String {
    let search_term = parse_query_param(query, "q")
        .unwrap_or_default()
        .to_lowercase();

    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(FILES_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                let name = obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let path = obj
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let content = obj
                    .get("content_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();

                if name.contains(&search_term)
                    || path.contains(&search_term)
                    || content.contains(&search_term)
                {
                    results.push(obj);
                }
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body, origin)
}

// ─── User management handlers ────────────────────────────────────────

/// List users (without password hashes).
fn list_users_safe(db: &RedbDb, origin: Option<&str>) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(USERS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(mut obj) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                // Strip password hashes for safety
                if let Some(map) = obj.as_object_mut() {
                    map.remove("passwordHash");
                    map.remove("password_hash");
                }
                results.push(obj);
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body, origin)
}

/// Login endpoint — expects JSON body: { "username": "...", "password": "..." }
/// Verifies argon2 password hash and returns a JWT token.
fn login_user(db: &RedbDb, body: &str, jwt_secret: &[u8; 32], origin: Option<&str>) -> String {
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
    let table = match tx.open_table(USERS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut found_user: Option<serde_json::Value> = None;
    for entry in table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(user) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                if user.get("username").and_then(|v| v.as_str()) == Some(username) {
                    found_user = Some(user);
                    break;
                }
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
/// { "username": "...", "password": "...", "display_name": "...", "role": "..." }
fn register_user_web(db: &RedbDb, body: &str, origin: Option<&str>) -> String {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e), origin),
    };

    let username = req.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");
    let display_name = req
        .get("displayName")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let role = req
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("user")
        .to_string();

    if username.is_empty() || password.is_empty() {
        return json_error(400, "username and password are required", origin);
    }

    // Check for duplicate username
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(USERS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    for entry in table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(user) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                if user.get("username").and_then(|v| v.as_str()) == Some(username) {
                    drop(tx);
                    return json_error(
                        409,
                        &format!("Username '{}' already exists", username),
                        origin,
                    );
                }
            }
        }
    }
    drop(tx);

    // Hash password with argon2
    let password_hash = match argon2_hash(password) {
        Ok(h) => h,
        Err(e) => return json_error(500, &format!("Hashing error: {}", e), origin),
    };

    let user_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let user = serde_json::json!({
        "id": user_id,
        "username": username,
        "passwordHash": password_hash,
        "displayName": display_name,
        "role": role,
        "isActive": true,
        "createdAt": now.clone(),
        "updatedAt": now,
    });

    let user_json = match serde_json::to_string(&user) {
        Ok(s) => s,
        Err(e) => return json_error(500, &format!("Serialization error: {}", e), origin),
    };

    let tx = match db.begin_write() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Write error: {}", e), origin),
    };
    {
        let mut table = match tx.open_table(USERS_TABLE) {
            Ok(t) => t,
            Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
        };
        if table.insert(&user_id, user_json.as_str()).is_err() {
            return json_error(500, "Failed to insert user", origin);
        }
    }
    if tx.commit().is_err() {
        return json_error(500, "Failed to commit user registration", origin);
    }

    // Strip password hash from response
    let mut response = user;
    if let Some(map) = response.as_object_mut() {
        map.remove("passwordHash");
        map.remove("password_hash");
    }
    http_response(
        201,
        "application/json",
        &serde_json::to_string(&response).unwrap_or_default(),
        origin,
    )
}

/// Set file permission — expects JSON body:
/// { "user_id": "...", "file_id": "...", "access": "read|write|admin" }
fn set_permission_web(db: &RedbDb, body: &str, origin: Option<&str>) -> String {
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
        let mut table = match tx.open_table(USER_FILE_PERMS_TABLE) {
            Ok(t) => t,
            Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
        };
        if table.insert(&perm_id, perm_json.as_str()).is_err() {
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
fn verify_access_web(db: &RedbDb, body: &str, origin: Option<&str>) -> String {
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
    let users_table = match tx.open_table(USERS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    if let Ok(Some(val)) = users_table.get(user_id) {
        if let Ok(user) = serde_json::from_str::<serde_json::Value>(&val.value()) {
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
    let perms_table = match tx.open_table(USER_FILE_PERMS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    for entry in perms_table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(perm) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                let p_user = perm.get("userId").and_then(|v| v.as_str()).unwrap_or("");
                let p_file = perm.get("fileId").and_then(|v| v.as_str()).unwrap_or("");
                let p_access = perm.get("access").and_then(|v| v.as_str()).unwrap_or("");

                if p_user == user_id && p_file == file_id {
                    if access_level_sufficient(p_access, required_access) {
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
fn get_permissions_for_file(db: &RedbDb, file_id: &str, origin: Option<&str>) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e), origin),
    };
    let table = match tx.open_table(USER_FILE_PERMS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e), origin),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(perm) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                if perm.get("fileId").and_then(|v| v.as_str()) == Some(file_id) {
                    results.push(perm);
                }
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body, origin)
}

// ─── Argon2 helpers ──────────────────────────────────────────────────

fn argon2_hash(password: &str) -> Result<String, String> {
    use argon2::{
        password_hash::{PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("Argon2 hash error: {}", e))
}

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
fn http_response(status: u16, content_type: &str, body: &str, origin: Option<&str>) -> String {
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

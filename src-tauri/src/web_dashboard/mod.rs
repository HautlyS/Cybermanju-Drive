// Cybermanju Drive — Web Dashboard Server
// Embedded HTTP server for any-device browser access
// Exposes REST API mirroring all Tauri IPC commands
//
// Uses only std::net::TcpListener + manual HTTP parsing (no external HTTP crate)

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use log::{error, info, warn};

use redb::{Database as RedbDb, ReadableTable, TableDefinition};

// ─── Table definitions (must match db/mod.rs) ────────────────────────

const FILES_TABLE: TableDefinition<&str, &str> = TableDefinition::new("files");
const ACCOUNTS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("accounts");
const COLLECTIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("collections");
const COLLECTION_ITEMS_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("collection_items");
const FACE_GROUPS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("face_groups");
const LOOSE_GROUPS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("loose_groups");
const ENCRYPTION_KEYS_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("encryption_keys");
const LOCATIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("locations");
const USERS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("users");
const USER_FILE_PERMS_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("user_file_perms");

// ─── WebDashboard struct ────────────────────────────────────────────

pub struct WebDashboard {
    port: u16,
    running: AtomicBool,
    db_path: String,
}

impl WebDashboard {
    pub fn new(port: u16, db_path: &str) -> Self {
        Self {
            port,
            running: AtomicBool::new(false),
            db_path: db_path.to_string(),
        }
    }

    /// Start the web dashboard HTTP server on a background thread.
    /// Returns a JoinHandle that keeps the server alive.
    pub fn start(self: &std::sync::Arc<Self>) -> std::io::Result<thread::JoinHandle<()>> {
        self.running.store(true, Ordering::SeqCst);
        let this = std::sync::Arc::clone(self);
        let handle = thread::spawn(move || {
            let addr = format!("0.0.0.0:{}", this.port);
            let listener = match TcpListener::bind(&addr) {
                Ok(l) => {
                    info!("Web Dashboard listening on http://{}", addr);
                    l
                }
                Err(e) => {
                    error!("Web Dashboard failed to bind on port {}: {}", this.port, e);
                    this.running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            // Set non-blocking would be nice but blocking accept per-thread is simpler
            listener
                .set_nonblocking(false)
                .ok();
            for stream in listener.incoming() {
                if !this.running.load(Ordering::SeqCst) {
                    break;
                }
                match stream {
                    Ok(stream) => {
                        let this_clone = std::sync::Arc::clone(&this);
                        thread::spawn(move || {
                            handle_connection(&this_clone, stream);
                        });
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
        Ok(handle)
    }

    /// Signal the server to stop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

// ─── Connection handler ──────────────────────────────────────────────

fn handle_connection(dashboard: &WebDashboard, mut stream: std::net::TcpStream) {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(5)))
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

    // Read headers to determine Content-Length
    let mut content_length: usize = 0;
    let mut headers_done = false;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line == "\r\n" || line.is_empty() {
            headers_done = true;
            break;
        }
        let line = line.trim().to_lowercase();
        if line.starts_with("content-length:") {
            content_length = line
                .strip_prefix("content-length:")
                .unwrap()
                .trim()
                .parse()
                .unwrap_or(0);
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

    // Handle the request
    let response = handle_request(dashboard, method, path, &body);

    let _ = stream.write_all(response.as_bytes());
}

// ─── Request router ──────────────────────────────────────────────────

fn handle_request(dashboard: &WebDashboard, method: &str, path: &str, body: &str) -> String {
    // Handle CORS preflight
    if method == "OPTIONS" {
        return http_response(200, "application/json", "{}");
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

    // Open redb for the request (independent handle per request)
    let db_result = redb::Database::open(&dashboard.db_path);

    // Route to appropriate handler
    match &path_segments.as_slice() {
        // ─── File endpoints ────────────────────────────────────────
        ["api", "files"] if method == "GET" => {
            match db_result {
                Ok(db) => list_all_json(&db, FILES_TABLE),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }
        ["api", "files", id] if method == "GET" => {
            match db_result {
                Ok(db) => get_by_id(&db, FILES_TABLE, id),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }
        ["api", "files", id] if method == "DELETE" => {
            match db_result {
                Ok(db) => delete_by_id(&db, FILES_TABLE, id),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }

        // ─── Account endpoints ─────────────────────────────────────
        ["api", "accounts"] if method == "GET" => {
            match db_result {
                Ok(db) => list_all_json(&db, ACCOUNTS_TABLE),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }

        // ─── Collection endpoints ───────────────────────────────────
        ["api", "collections"] if method == "GET" => {
            match db_result {
                Ok(db) => list_all_json(&db, COLLECTIONS_TABLE),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }
        ["api", "collection-items"] if method == "GET" => {
            match db_result {
                Ok(db) => list_all_json(&db, COLLECTION_ITEMS_TABLE),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }

        // ─── Face group endpoints ───────────────────────────────────
        ["api", "face-groups"] if method == "GET" => {
            match db_result {
                Ok(db) => list_all_json(&db, FACE_GROUPS_TABLE),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }

        // ─── Loose group endpoints ─────────────────────────────────
        ["api", "loose-groups"] if method == "GET" => {
            match db_result {
                Ok(db) => list_all_json(&db, LOOSE_GROUPS_TABLE),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }

        // ─── Encryption endpoints ──────────────────────────────────
        ["api", "encryption", "status"] if method == "GET" => {
            let status = serde_json::json!({
                "available": true,
                "supported_algorithms": ["kyber512", "kyber768", "kyber1024", "dilithium2", "dilithium3", "dilithium5"],
                "engine": "rustpq (post-quantum cryptography)"
            });
            http_response(200, "application/json", &serde_json::to_string(&status).unwrap_or_default())
        }
        ["api", "encryption", "keys"] if method == "GET" => {
            match db_result {
                Ok(db) => list_all_json(&db, ENCRYPTION_KEYS_TABLE),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }

        // ─── Geo files ─────────────────────────────────────────────
        ["api", "geo-files"] if method == "GET" => {
            match db_result {
                Ok(db) => list_geo_files(&db),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }

        // ─── Search ────────────────────────────────────────────────
        ["api", "search"] if method == "GET" => {
            match db_result {
                Ok(db) => search_files(&db, query),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }

        // ─── Location endpoints ─────────────────────────────────────
        ["api", "locations"] if method == "GET" => {
            match db_result {
                Ok(db) => list_all_json(&db, LOCATIONS_TABLE),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }

        // ─── User endpoints ───────────────────────────────────────
        ["api", "users"] if method == "GET" => {
            match db_result {
                Ok(db) => list_users_safe(&db),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }
        ["api", "users", "login"] if method == "POST" => {
            match db_result {
                Ok(db) => login_user(&db, body),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }
        ["api", "users", "register"] if method == "POST" => {
            match db_result {
                Ok(db) => register_user_web(&db, body),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }

        // ─── Permission endpoints ──────────────────────────────────
        ["api", "permissions"] if method == "POST" => {
            match db_result {
                Ok(db) => set_permission_web(&db, body),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }
        ["api", "permissions", "verify"] if method == "POST" => {
            match db_result {
                Ok(db) => verify_access_web(&db, body),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }
        ["api", "permissions", file_id] if method == "GET" => {
            match db_result {
                Ok(db) => get_permissions_for_file(&db, file_id),
                Err(e) => json_error(500, &format!("Database error: {}", e)),
            }
        }

        // ─── Root / health check ──────────────────────────────────
        ["api"] | ["api", "health"] if method == "GET" => {
            let health = serde_json::json!({
                "service": "Cybermanju Drive Web Dashboard",
                "status": "ok",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            });
            http_response(200, "application/json", &serde_json::to_string(&health).unwrap_or_default())
        }

        // ─── 404 ──────────────────────────────────────────────────
        _ => json_error(404, &format!("Not found: {} {}", method, path)),
    }
}

// ─── Generic table operations ────────────────────────────────────────

/// List all JSON values from a table.
fn list_all_json(db: &RedbDb, table_def: TableDefinition<&str, &str>) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e)),
    };
    let table = match tx.open_table(table_def) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
        match entry {
            Ok((key, value)) => {
                let key_str = key.value().to_string();
                let val_str = value.value().to_string();
                // Wrap each entry as { "id": key, ...data }
                if let Ok(mut obj) = serde_json::from_str::<serde_json::Value>(&val_str) {
                    if let Some(map) = obj.as_object_mut() {
                        map.insert("_key".to_string(), serde_json::json!(key_str));
                    }
                    results.push(obj);
                } else {
                    // Store raw if not JSON-parseable
                    results.push(serde_json::json!({
                        "_key": key_str,
                        "_raw": val_str,
                    }));
                }
            }
            Err(e) => {
                return json_error(500, &format!("Iteration error: {}", e));
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body)
}

/// Get a single entry by key from a table.
fn get_by_id(db: &RedbDb, table_def: TableDefinition<&str, &str>, id: &str) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e)),
    };
    let table = match tx.open_table(table_def) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    match table.get(id) {
        Ok(Some(value)) => {
            let val_str = value.value().to_string();
            http_response(200, "application/json", &val_str)
        }
        Ok(None) => json_error(404, &format!("Not found: {}", id)),
        Err(e) => json_error(500, &format!("Get error: {}", e)),
    }
}

/// Delete an entry by key.
fn delete_by_id(db: &RedbDb, table_def: TableDefinition<&str, &str>, id: &str) -> String {
    let tx = match db.begin_write() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Write error: {}", e)),
    };
    let result = {
        let mut table = match tx.open_table(table_def) {
            Ok(t) => t,
            Err(e) => return json_error(500, &format!("Table open error: {}", e)),
        };
        match table.remove(id) {
            Ok(_) => true,
            Err(e) => return json_error(500, &format!("Remove error: {}", e)),
        }
    };
    if let Err(e) = tx.commit() {
        return json_error(500, &format!("Commit error: {}", e));
    }
    let body = serde_json::to_string(&serde_json::json!({"deleted": result, "id": id}))
        .unwrap_or_default();
    http_response(200, "application/json", &body)
}

// ─── Domain-specific handlers ────────────────────────────────────────

/// List files that have GPS coordinates.
fn list_geo_files(db: &RedbDb) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e)),
    };
    let table = match tx.open_table(FILES_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                // Check if the file has geo_lat and geo_lng
                let has_lat = obj.get("gpsLat").and_then(|v| v.as_f64()).is_some();
                let has_lng = obj.get("gpsLon").and_then(|v| v.as_f64()).is_some();
                if has_lat && has_lng {
                    results.push(obj);
                }
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body)
}

/// Simple search across file names and tags by scanning all entries.
/// Query param: ?q=search_term
fn search_files(db: &RedbDb, query: &str) -> String {
    let search_term = parse_query_param(query, "q").unwrap_or_default().to_lowercase();

    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e)),
    };
    let table = match tx.open_table(FILES_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
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
    http_response(200, "application/json", &body)
}

// ─── User management handlers (web) ────────────────────────────────

/// List users (without password hashes).
fn list_users_safe(db: &RedbDb) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e)),
    };
    let table = match tx.open_table(USERS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(mut obj) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                // Strip passwordHash for safety
                if let Some(map) = obj.as_object_mut() {
                    map.remove("passwordHash");
                }
                results.push(obj);
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body)
}

/// Login endpoint — expects JSON body: { "username": "...", "password": "..." }
fn login_user(db: &RedbDb, body: &str) -> String {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    let username = req
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let password = req
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if username.is_empty() || password.is_empty() {
        return json_error(400, "username and password are required");
    }

    // Find user
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e)),
    };
    let table = match tx.open_table(USERS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
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
        None => return json_error(401, "Invalid credentials"),
    };

    // Check active
    if user.get("isActive").and_then(|v| v.as_bool()) == Some(false) {
        return json_error(403, "User account is deactivated");
    }

    // Verify password using argon2
    let password_hash = match user.get("passwordHash").and_then(|v| v.as_str()) {
        Some(h) => h,
        None => return json_error(500, "User record has no password hash"),
    };

    match argon2_verify(password, password_hash) {
        Ok(true) => {
            // Generate token
            let token_input = format!(
                "{}|{}",
                username,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            );
            let token = blake3::hash(token_input.as_bytes()).to_hex().to_string();

            let response = serde_json::json!({
                "userId": user.get("id"),
                "username": username,
                "role": user.get("role"),
                "displayName": user.get("displayName"),
                "token": token,
            });
            http_response(
                200,
                "application/json",
                &serde_json::to_string(&response).unwrap_or_default(),
            )
        }
        Ok(false) => json_error(401, "Invalid credentials"),
        Err(e) => json_error(500, &format!("Password verification error: {}", e)),
    }
}

/// Register endpoint — expects JSON body: { "username": "...", "password": "...", "display_name": "...", "role": "..." }
fn register_user_web(db: &RedbDb, body: &str) -> String {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    let username = req.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");
    let display_name = req.get("displayName").and_then(|v| v.as_str()).map(|s| s.to_string());
    let role = req
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("user")
        .to_string();

    if username.is_empty() || password.is_empty() {
        return json_error(400, "username and password are required");
    }

    // Check for duplicate
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e)),
    };
    let table = match tx.open_table(USERS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    for entry in table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(user) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                if user.get("username").and_then(|v| v.as_str()) == Some(username) {
                    drop(tx);
                    return json_error(409, &format!("Username '{}' already exists", username));
                }
            }
        }
    }
    drop(tx);

    // Hash password
    let password_hash = match argon2_hash(password) {
        Ok(h) => h,
        Err(e) => return json_error(500, &format!("Hashing error: {}", e)),
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
        Err(e) => return json_error(500, &format!("Serialization error: {}", e)),
    };

    let tx = match db.begin_write() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Write error: {}", e)),
    };
    {
        let mut table = match tx.open_table(USERS_TABLE) {
            Ok(t) => t,
            Err(e) => return json_error(500, &format!("Table open error: {}", e)),
        };
        if table.insert(&user_id, user_json.as_str()).is_err() {
            return json_error(500, "Failed to insert user");
        }
    }
    if tx.commit().is_err() {
        return json_error(500, "Failed to commit user registration");
    }

    // Strip password hash from response
    let mut response = user;
    if let Some(map) = response.as_object_mut() {
        map.remove("password_hash");
    }
    http_response(201, "application/json", &serde_json::to_string(&response).unwrap_or_default())
}

/// Set file permission — expects JSON body: { "user_id": "...", "file_id": "...", "access": "read|write|admin" }
fn set_permission_web(db: &RedbDb, body: &str) -> String {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    let user_id = req.get("userId").and_then(|v| v.as_str()).unwrap_or("");
    let file_id = req.get("fileId").and_then(|v| v.as_str()).unwrap_or("");
    let access = req.get("access").and_then(|v| v.as_str()).unwrap_or("");

    if user_id.is_empty() || file_id.is_empty() || access.is_empty() {
        return json_error(400, "user_id, file_id, and access are required");
    }

    if !["read", "write", "admin"].contains(&access) {
        return json_error(
            400,
            "access must be 'read', 'write', or 'admin'",
        );
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
        Err(e) => return json_error(500, &format!("Serialization error: {}", e)),
    };

    let tx = match db.begin_write() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Write error: {}", e)),
    };
    {
        let mut table = match tx.open_table(USER_FILE_PERMS_TABLE) {
            Ok(t) => t,
            Err(e) => return json_error(500, &format!("Table open error: {}", e)),
        };
        if table.insert(&perm_id, perm_json.as_str()).is_err() {
            return json_error(500, "Failed to insert permission");
        }
    }
    if tx.commit().is_err() {
        return json_error(500, "Failed to commit permission");
    }

    http_response(
        201,
        "application/json",
        &serde_json::to_string(&permission).unwrap_or_default(),
    )
}

/// Verify file access — expects JSON body: { "user_id": "...", "file_id": "...", "required_access": "read|write|admin" }
fn verify_access_web(db: &RedbDb, body: &str) -> String {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    let user_id = req
        .get("userId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let file_id = req
        .get("fileId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let required_access = req
        .get("requiredAccess")
        .and_then(|v| v.as_str())
        .unwrap_or("read");

    // Check user role
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e)),
    };
    let users_table = match tx.open_table(USERS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    if let Ok(Some(val)) = users_table.get(user_id) {
        if let Ok(user) = serde_json::from_str::<serde_json::Value>(&val.value()) {
            if user.get("role").and_then(|v| v.as_str()) == Some("admin") {
                let resp = serde_json::json!({"user_id": user_id, "file_id": file_id, "required_access": required_access, "granted": true, "reason": "admin_role"});
                return http_response(200, "application/json", &serde_json::to_string(&resp).unwrap_or_default());
            }
        }
    }

    // Check permissions
    let perms_table = match tx.open_table(USER_FILE_PERMS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    for entry in perms_table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(perm) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                let p_user = perm.get("userId").and_then(|v| v.as_str()).unwrap_or("");
                let p_file = perm.get("fileId").and_then(|v| v.as_str()).unwrap_or("");
                let p_access = perm.get("access").and_then(|v| v.as_str()).unwrap_or("");

                if p_user == user_id && p_file == file_id {
                    if access_level_sufficient(p_access, required_access) {
                        let resp = serde_json::json!({"user_id": user_id, "file_id": file_id, "required_access": required_access, "granted": true, "reason": "permission_match"});
                        return http_response(200, "application/json", &serde_json::to_string(&resp).unwrap_or_default());
                    }
                }
            }
        }
    }

    let resp = serde_json::json!({"user_id": user_id, "file_id": file_id, "required_access": required_access, "granted": false, "reason": "no_matching_permission"});
    http_response(200, "application/json", &serde_json::to_string(&resp).unwrap_or_default())
}

/// Get all permissions for a specific file.
fn get_permissions_for_file(db: &RedbDb, file_id: &str) -> String {
    let tx = match db.begin_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &format!("Read error: {}", e)),
    };
    let table = match tx.open_table(USER_FILE_PERMS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
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
    http_response(200, "application/json", &body)
}

// ─── Argon2 helpers ──────────────────────────────────────────────────

fn argon2_hash(password: &str) -> Result<String, String> {
    use argon2::{
        password_hash::{PasswordHasher, SaltString},
        Argon2,
    };
    use rand_core::OsRng;

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

fn http_response(status: u16, content_type: &str, body: &str) -> String {
    let status_text = match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        500 => "Internal Server Error",
        _ => "OK",
    };
    format!(
        "HTTP/1.1 {status} {status_text}\r\n\
         Content-Type: {content_type}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
         Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {body}",
        body.len()
    )
}

fn json_error(status: u16, message: &str) -> String {
    let body = serde_json::json!({
        "error": true,
        "status": status,
        "message": message,
    });
    let body_str = serde_json::to_string(&body).unwrap_or_else(|_| message.to_string());
    http_response(status, "application/json", &body_str)
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

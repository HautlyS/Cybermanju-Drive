// Cybermanju Drive — Standalone Web Dashboard (Docker)
//
// Self-contained API handler for Docker deployments.
// Mirrors the Tauri web_dashboard API surface but runs independently
// without any Tauri dependencies.
//
// Security: single shared JWT secret, singleton WebDashboard instance.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use log::info;
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
const ENCRYPTION_KEYS_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("encryption_keys");
const LOCATIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("locations");
const USERS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("users");
const USER_FILE_PERMS_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("user_file_perms");

// ─── Security constants ─────────────────────────────────────────────

const RATE_LIMIT_MAX: u32 = 100;
const RATE_LIMIT_WINDOW_SECS: u64 = 60;
const JWT_EXPIRY_SECS: u64 = 86_400;

// ─── JWT Claims ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    sub: String,
    role: String,
    user_id: String,
    iat: u64,
    exp: u64,
}

// ─── WebDashboard singleton ──────────────────────────────────────────

pub struct WebDashboard {
    pub port: u16,
    pub jwt_secret: [u8; 32],
    pub db: Mutex<RedbDb>,
    rate_limits: Mutex<HashMap<String, (u32, std::time::Instant)>>,
}

impl WebDashboard {
    pub fn new(port: u16, db_path: &str) -> Self {
        let mut jwt_secret = [0u8; 32];
        OsRng.fill_bytes(&mut jwt_secret);

        let db = RedbDb::open(db_path)
            .or_else(|_| RedbDb::create(db_path))
            .expect("Failed to open web dashboard database");

        info!("WebDashboard initialized (port={}, db={})", port, db_path);

        Self {
            port,
            jwt_secret,
            db: Mutex::new(db),
            rate_limits: Mutex::new(HashMap::new()),
        }
    }

    fn check_rate_limit(&self, client_ip: &str) -> bool {
        let mut limits = self.rate_limits.lock().unwrap();
        let now = std::time::Instant::now();
        limits.retain(|_, (_, ts)| now.duration_since(*ts).as_secs() < RATE_LIMIT_WINDOW_SECS * 2);
        let entry = limits.entry(client_ip.to_string()).or_insert((0, now));
        if now.duration_since(entry.1).as_secs() >= RATE_LIMIT_WINDOW_SECS {
            *entry = (0, now);
        }
        entry.0 += 1;
        entry.0 <= RATE_LIMIT_MAX
    }

    fn db_read(&self) -> Result<redb::ReadTransaction<'_>, String> {
        self.db
            .lock()
            .map_err(|e| format!("DB lock poisoned: {}", e))?
            .begin_read()
            .map_err(|e| format!("Read error: {}", e))
    }

    fn db_write(&self) -> Result<redb::WriteTransaction<'_>, String> {
        self.db
            .lock()
            .map_err(|e| format!("DB lock poisoned: {}", e))?
            .begin_write()
            .map_err(|e| format!("Write error: {}", e))
    }
}

// ─── HTTP response helpers ───────────────────────────────────────────

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

fn json_error(status: u16, message: &str) -> String {
    let body = serde_json::json!({"error": message});
    http_response(
        status,
        "application/json",
        &serde_json::to_string(&body).unwrap_or_default(),
    )
}

fn cors_preflight_response() -> String {
    format!(
        "HTTP/1.1 204 No Content\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
         Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
         Access-Control-Max-Age: 86400\r\n\
         Content-Length: 0\r\n\
         \r\n"
    )
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

// ─── JWT helpers ─────────────────────────────────────────────────────

fn verify_jwt(jwt_secret: &[u8; 32], auth_header: Option<&str>) -> Result<(), String> {
    let token = match auth_header {
        Some(h) => h
            .strip_prefix("Bearer ")
            .or_else(|| h.strip_prefix("bearer "))
            .map(|t| t.trim()),
        None => None,
    }
    .ok_or_else(|| json_error(401, "Authorization header required"))?;

    let decoding_key = DecodingKey::from_secret(jwt_secret);
    decode::<JwtClaims>(token, &decoding_key, &Validation::default())
        .map(|_| ())
        .map_err(|e| json_error(401, &format!("Invalid or expired token: {}", e)))
}

fn create_jwt(
    jwt_secret: &[u8; 32],
    user_id: &str,
    username: &str,
    role: &str,
) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let claims = JwtClaims {
        sub: username.to_string(),
        role: role.to_string(),
        user_id: user_id.to_string(),
        iat: now,
        exp: now + JWT_EXPIRY_SECS,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret),
    )
    .map_err(|e| format!("JWT encoding error: {}", e))
}

// ─── Generic table operations ────────────────────────────────────────

fn list_all_json(dashboard: &WebDashboard, table_def: TableDefinition<&str, &str>) -> String {
    let tx = match dashboard.db_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &e),
    };
    let table = match tx.open_table(table_def) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
        if let Ok((key, value)) = entry {
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
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body)
}

fn list_encryption_keys_safe(dashboard: &WebDashboard) -> String {
    let tx = match dashboard.db_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &e),
    };
    let table = match tx.open_table(ENCRYPTION_KEYS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
        if let Ok((key, value)) = entry {
            let key_str = key.value().to_string();
            let val_str = value.value().to_string();
            if let Ok(mut obj) = serde_json::from_str::<serde_json::Value>(&val_str) {
                if let Some(map) = obj.as_object_mut() {
                    map.insert("_key".to_string(), serde_json::json!(key_str));
                    map.remove("private_key");
                    map.remove("privateKey");
                }
                results.push(obj);
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body)
}

fn list_geo_files(dashboard: &WebDashboard) -> String {
    let tx = match dashboard.db_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &e),
    };
    let table = match tx.open_table(FILES_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
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
    http_response(200, "application/json", &body)
}

fn search_files(dashboard: &WebDashboard, query: &str) -> String {
    let search_term = parse_query_param(query, "q")
        .unwrap_or_default()
        .to_lowercase();

    if search_term.is_empty() {
        return json_error(400, "Missing search query parameter 'q'");
    }

    let tx = match dashboard.db_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &e),
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
                if name.contains(&search_term) || path.contains(&search_term) {
                    results.push(obj);
                }
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body)
}

fn list_users_safe(dashboard: &WebDashboard) -> String {
    let tx = match dashboard.db_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &e),
    };
    let table = match tx.open_table(USERS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
        if let Ok((_, value)) = entry {
            if let Ok(mut obj) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                if let Some(map) = obj.as_object_mut() {
                    map.remove("passwordHash");
                    map.remove("password_hash");
                }
                results.push(obj);
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body)
}

fn login_user(dashboard: &WebDashboard, body: &str) -> String {
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

    let tx = match dashboard.db_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &e),
    };
    let table = match tx.open_table(USERS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    match table.get(username) {
        Ok(Some(value)) => {
            let user: serde_json::Value =
                serde_json::from_str(&value.value()).unwrap_or(serde_json::json!({}));

            let password_hash = user
                .get("passwordHash")
                .or_else(|| user.get("password_hash"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if password_hash.is_empty() {
                return json_error(500, "User has no password hash");
            }

            // Verify argon2 password hash
            let parsed = argon2::password_hash::PasswordHash::new(password_hash);
            match parsed {
                Ok(parsed_hash) => {
                    match argon2::password_hash::PasswordHash::verify_password(
                        &argon2::password_hash::Password::new(password.as_bytes()),
                        &parsed_hash,
                    ) {
                        Ok(_) => {
                            let user_id = user
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or(username);
                            let role = user
                                .get("role")
                                .and_then(|v| v.as_str())
                                .unwrap_or("user");

                            match create_jwt(&dashboard.jwt_secret, user_id, username, role) {
                                Ok(token) => {
                                    let resp = serde_json::json!({
                                        "token": token,
                                        "user": {
                                            "id": user_id,
                                            "username": username,
                                            "role": role,
                                        }
                                    });
                                    http_response(
                                        200,
                                        "application/json",
                                        &serde_json::to_string(&resp).unwrap_or_default(),
                                    )
                                }
                                Err(e) => json_error(500, &e),
                            }
                        }
                        Err(_) => json_error(401, "Invalid username or password"),
                    }
                }
                Err(_) => json_error(500, "Invalid password hash format"),
            }
        }
        Ok(None) => json_error(401, "Invalid username or password"),
        Err(e) => json_error(500, &format!("Database error: {}", e)),
    }
}

fn register_user(dashboard: &WebDashboard, body: &str) -> String {
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
    let display_name = req
        .get("displayName")
        .or_else(|| req.get("display_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if username.is_empty() || password.is_empty() {
        return json_error(400, "username and password are required");
    }

    // Hash password with argon2id
    let salt = argon2::password_hash::SaltString::generate(&mut OsRng);
    let hash = match argon2::password_hash::PasswordHasher::hash_password(
        &argon2::Argon2::default(),
        password.as_bytes(),
        &salt,
    ) {
        Ok(h) => h.to_string(),
        Err(e) => return json_error(500, &format!("Password hash error: {}", e)),
    };

    let user_id = uuid::Uuid::new_v4().to_string();
    let user = serde_json::json!({
        "id": user_id,
        "username": username,
        "displayName": display_name,
        "role": "admin",
        "passwordHash": hash,
        "createdAt": chrono::Utc::now().to_rfc3339(),
    });

    let tx = match dashboard.db_write() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &e),
    };
    {
        let mut table = match tx.open_table(USERS_TABLE) {
            Ok(t) => t,
            Err(e) => return json_error(500, &format!("Table open error: {}", e)),
        };
        let user_json = serde_json::to_string(&user).unwrap_or_default();
        if let Err(e) = table.insert(username, user_json.as_str()) {
            return json_error(500, &format!("Insert error: {}", e));
        }
    }
    if let Err(e) = tx.commit() {
        return json_error(500, &format!("Commit error: {}", e));
    }

    // Return user without password hash
    let mut resp = user.clone();
    if let Some(map) = resp.as_object_mut() {
        map.remove("passwordHash");
    }
    http_response(201, "application/json", &serde_json::to_string(&resp).unwrap_or_default())
}

fn get_permissions_for_file(dashboard: &WebDashboard, file_id: &str) -> String {
    let tx = match dashboard.db_read() {
        Ok(tx) => tx,
        Err(e) => return json_error(500, &e),
    };
    let table = match tx.open_table(USER_FILE_PERMS_TABLE) {
        Ok(t) => t,
        Err(e) => return json_error(500, &format!("Table open error: {}", e)),
    };

    let mut results: Vec<serde_json::Value> = Vec::new();
    for entry in table.iter() {
        if let Ok((key, value)) = entry {
            let key_str = key.value().to_string();
            if key_str.starts_with(file_id) {
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&value.value()) {
                    results.push(obj);
                }
            }
        }
    }
    let body = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    http_response(200, "application/json", &body)
}

// ─── Query string parsing ────────────────────────────────────────────

fn parse_query_param(query: &str, key: &str) -> Option<String> {
    for part in query.split('&') {
        let mut kv = part.splitn(2, '=');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            if k == key {
                return Some(
                    v.replace("%20", " ")
                        .replace("%26", "&")
                        .replace("%3D", "="),
                );
            }
        }
    }
    None
}

// ─── Main request handler ────────────────────────────────────────────

/// Handle an HTTP request. Returns a complete HTTP response string.
pub fn handle_request(
    dashboard: &WebDashboard,
    method: &str,
    path: &str,
    body: &str,
    auth_header: Option<&str>,
) -> String {
    // CORS preflight
    if method == "OPTIONS" {
        return cors_preflight_response();
    }

    // Parse path and query
    let (path_clean, query) = match path.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path, ""),
    };

    let path_segments: Vec<&str> = path_clean
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    // Determine if auth is required
    let requires_auth = !matches!(
        path_segments.as_slice(),
        ["api"] | ["api", "health"]
            | ["api", "auth", "login"]
            | ["api", "users", "login"]
            | ["api", "users", "register"]
    );

    if requires_auth {
        if let Err(resp) = verify_jwt(&dashboard.jwt_secret, auth_header) {
            return resp;
        }
    }

    match path_segments.as_slice() {
        // ─── Auth ──────────────────────────────────────────────────
        ["api", "auth", "login"] | ["api", "users", "login"] if method == "POST" => {
            login_user(dashboard, body)
        }
        ["api", "users", "register"] if method == "POST" => register_user(dashboard, body),

        // ─── Files ─────────────────────────────────────────────────
        ["api", "files"] if method == "GET" => list_all_json(dashboard, FILES_TABLE),

        // ─── Accounts ──────────────────────────────────────────────
        ["api", "accounts"] if method == "GET" => list_all_json(dashboard, ACCOUNTS_TABLE),

        // ─── Collections ───────────────────────────────────────────
        ["api", "collections"] if method == "GET" => list_all_json(dashboard, COLLECTIONS_TABLE),
        ["api", "collection-items"] if method == "GET" => {
            list_all_json(dashboard, COLLECTION_ITEMS_TABLE)
        }

        // ─── Face groups ───────────────────────────────────────────
        ["api", "face-groups"] if method == "GET" => list_all_json(dashboard, FACE_GROUPS_TABLE),

        // ─── Loose groups ──────────────────────────────────────────
        ["api", "loose-groups"] if method == "GET" => list_all_json(dashboard, LOOSE_GROUPS_TABLE),

        // ─── Encryption ────────────────────────────────────────────
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
            )
        }
        ["api", "encryption", "keys"] if method == "GET" => list_encryption_keys_safe(dashboard),

        // ─── Geo files ────────────────────────────────────────────
        ["api", "geo-files"] if method == "GET" => list_geo_files(dashboard),

        // ─── Search ────────────────────────────────────────────────
        ["api", "search"] if method == "GET" => search_files(dashboard, query),

        // ─── Locations ─────────────────────────────────────────────
        ["api", "locations"] if method == "GET" => list_all_json(dashboard, LOCATIONS_TABLE),

        // ─── Users ─────────────────────────────────────────────────
        ["api", "users"] if method == "GET" => list_users_safe(dashboard),

        // ─── Permissions ───────────────────────────────────────────
        ["api", "permissions", file_id] if method == "GET" => {
            get_permissions_for_file(dashboard, file_id)
        }

        // ─── Sync ──────────────────────────────────────────────────
        ["api", "sync", "configs"] if method == "GET" => {
            http_response(200, "application/json", "[]")
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
            )
        }

        // ─── Dashboard status ──────────────────────────────────────
        ["api", "dashboard", "status"] if method == "GET" => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let status = serde_json::json!({
                "service": "Cybermanju Drive Web Dashboard",
                "running": true,
                "port": dashboard.port,
                "timestamp": now,
                "version": "1.0.0",
            });
            http_response(
                200,
                "application/json",
                &serde_json::to_string(&status).unwrap_or_default(),
            )
        }

        // ─── Health ────────────────────────────────────────────────
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
            )
        }

        // ─── 404 ──────────────────────────────────────────────────
        _ => json_error(404, &format!("Not found: {} {}", method, path)),
    }
}

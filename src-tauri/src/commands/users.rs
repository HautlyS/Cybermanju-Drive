// Cybermanju Drive — User Management & File Permission Commands
// Role-based access control: admin | user | viewer
// Per-file permissions: read | write | admin
// Password hashing: argon2id (cryptographically secure, salted, key-stretched)

use tauri::State;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::db::schema::{User, UserFilePermission};

/// Auth result matching the frontend AuthResult type.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResult {
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub display_name: Option<String>,
    pub token: String,
}

/// Argon2 password hashing using the same argon2 crate as web_dashboard.
fn argon2_hash_password(password: &str) -> Result<String, String> {
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

/// Argon2 password verification.
fn argon2_verify_password(password: &str, hash: &str) -> Result<bool, String> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed = PasswordHash::new(hash).map_err(|e| format!("Invalid hash format: {}", e))?;
    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Generate a simple session token.
fn generate_session_token(username: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let input = format!("{}|{}", username, timestamp);
    blake3::hash(input.as_bytes()).to_hex().to_string()
}

/// Register a new user with argon2id password hashing.
#[tauri::command]
pub fn register_user(
    username: String,
    password: String,
    display_name: Option<String>,
    role: Option<String>,
    state: State<'_, AppState>,
) -> Result<User, String> {
    // Validate role BEFORE constructing user
    let role = role.unwrap_or_else(|| "user".to_string());
    if !["admin", "user", "viewer"].contains(&role.as_str()) {
        return Err(format!("Invalid role: {}. Must be admin, user, or viewer", role));
    }

    if username.is_empty() || password.is_empty() {
        return Err("Username and password are required".to_string());
    }

    // Validate username uniqueness
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx_check = db.begin_read().map_err(|e| e.to_string())?;
    let users_table = tx_check.open_table(crate::db::Database::get_users_table())
        .map_err(|e| e.to_string())?;
    for entry in users_table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let existing: User = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;
        if existing.username == username {
            return Err(format!("Username '{}' already exists", username));
        }
    }
    drop(tx_check);

    // Hash the password with argon2id
    let password_hash = argon2_hash_password(&password)?;

    let user_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let user = User {
        id: user_id.clone(),
        username,
        password_hash,
        display_name,
        role,
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
    };

    let serialized = serde_json::to_string(&user).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx.open_table(crate::db::Database::get_users_table())
            .map_err(|e| e.to_string())?;
        table.insert(&user_id, serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(user)
}

/// Authenticate a user — returns AuthResult matching frontend expectations.
#[tauri::command]
pub fn authenticate_user(
    username: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<AuthResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx.open_table(crate::db::Database::get_users_table())
        .map_err(|e| e.to_string())?;

    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let user: User = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;
        if user.username == username {
            if !user.is_active {
                return Err("User account is disabled".to_string());
            }

            // Verify password using argon2
            // Support both old BLAKE3 hashes and new argon2 hashes during migration
            let blake3_hash = blake3::hash(password.as_bytes()).to_hex().to_string();
            let valid = if user.password_hash.starts_with("$argon2") {
                argon2_verify_password(&password, &user.password_hash)?
            } else {
                // Legacy BLAKE3 hash comparison (migration path)
                user.password_hash == blake3_hash
            };

            if valid {
                let token = generate_session_token(&username);
                return Ok(AuthResult {
                    user_id: user.id,
                    username: user.username,
                    role: user.role,
                    display_name: user.display_name,
                    token,
                });
            } else {
                return Err("Invalid username or password".to_string());
            }
        }
    }

    Err("Invalid username or password".to_string())
}

/// List all registered users.
#[tauri::command]
pub fn list_users(state: State<'_, AppState>) -> Result<Vec<User>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx.open_table(crate::db::Database::get_users_table())
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let user: User = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;
        results.push(user);
    }

    Ok(results)
}

/// Alias for set_file_permission — frontend calls this as grant_file_permission.
#[tauri::command]
pub fn grant_file_permission(
    user_id: String,
    file_id: String,
    access: String,
    granted_by: String,
    state: State<'_, AppState>,
) -> Result<crate::db::schema::UserFilePermission, String> {
    set_file_permission(user_id, file_id, access, granted_by, state)
}

/// Grant a file permission to a user (aliased as grant_file_permission for frontend).
#[tauri::command]
pub fn set_file_permission(
    user_id: String,
    file_id: String,
    access: String,
    granted_by: String,
    state: State<'_, AppState>,
) -> Result<UserFilePermission, String> {
    if !["read", "write", "admin"].contains(&access.as_str()) {
        return Err(format!("Invalid access level: {}. Must be read, write, or admin", access));
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let perm_id = uuid::Uuid::new_v4().to_string();

    // Check if a permission already exists for this user+file
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let perms_table = tx_read.open_table(crate::db::Database::get_user_file_perms_table())
        .map_err(|e| e.to_string())?;
    let mut existing_id: Option<String> = None;
    for entry in perms_table.iter().map_err(|e| e.to_string())? {
        let (key, value) = entry.map_err(|e| e.to_string())?;
        let perm: UserFilePermission = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;
        if perm.user_id == user_id && perm.file_id == file_id {
            existing_id = Some(key.value().to_string());
            break;
        }
    }
    drop(tx_read);

    let permission = UserFilePermission {
        id: perm_id.clone(),
        user_id,
        file_id,
        access,
        granted_by,
        granted_at: now,
    };

    let serialized = serde_json::to_string(&permission).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx.open_table(crate::db::Database::get_user_file_perms_table())
            .map_err(|e| e.to_string())?;
        if let Some(ref eid) = existing_id {
            table.remove(eid.as_str()).map_err(|e| e.to_string())?;
        }
        table.insert(&perm_id, serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(permission)
}

/// Revoke a file permission — frontend calls this as revoke_file_permission.
#[tauri::command]
pub fn revoke_file_permission(
    permission_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx.open_table(crate::db::Database::get_user_file_perms_table())
            .map_err(|e| e.to_string())?;
        let removed = table.remove(&permission_id)
            .map_err(|e| e.to_string())?
            .is_some();
        if !removed {
            return Err(format!("Permission not found: {}", permission_id));
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}

/// Verify whether a specific user has access to a specific file.
#[tauri::command]
pub fn verify_file_access(
    user_id: String,
    file_id: String,
    required_access: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx.open_table(crate::db::Database::get_user_file_perms_table())
        .map_err(|e| e.to_string())?;

    let level = |a: &str| match a {
        "admin" => 3u8,
        "write" => 2,
        "read" => 1,
        _ => 0,
    };
    let required = level(&required_access);

    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let perm: UserFilePermission = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;
        if perm.user_id == user_id && perm.file_id == file_id {
            if level(&perm.access) >= required {
                return Ok(true);
            }
        }
    }

    // Check if user is admin (admins have global access)
    let users_table = tx.open_table(crate::db::Database::get_users_table())
        .map_err(|e| e.to_string())?;
    for entry in users_table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let user: User = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;
        if user.id == user_id && user.role == "admin" && user.is_active {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Get all file permissions for a specific file.
#[tauri::command]
pub fn get_file_permissions(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<UserFilePermission>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx.open_table(crate::db::Database::get_user_file_perms_table())
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let perm: UserFilePermission = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;
        if perm.file_id == file_id {
            results.push(perm);
        }
    }

    Ok(results)
}

// Cybermanju Drive — User management (shared by Tauri IPC and REST)

use cybermanju_db::Database;
use cybermanju_types::schema::User;
use redb::ReadableTable;

/// Register a new user (also used by the `create_user` command).
///
/// The caller must hold the database write lock.
pub fn register(
    db: &Database,
    username: String,
    password: String,
    display_name: Option<String>,
    role: Option<String>,
) -> Result<User, String> {
    let role = role.unwrap_or_else(|| "user".to_string());
    if !["admin", "user", "viewer"].contains(&role.as_str()) {
        return Err(format!(
            "Invalid role: {}. Must be admin, user, or viewer",
            role
        ));
    }

    if username.is_empty() || password.is_empty() {
        return Err("Username and password are required".to_string());
    }

    if find_by_username(db, &username)?.is_some() {
        return Err(format!("Username '{}' already exists", username));
    }

    let password_hash = argon2_hash(&password)?;

    let user_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

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
        let mut table = tx
            .open_table(Database::get_users_table())
            .map_err(|e| e.to_string())?;
        table
            .insert(user_id.as_str(), serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(user)
}

/// Delete a user by ID.
pub fn delete(db: &Database, user_id: &str) -> Result<bool, String> {
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(Database::get_users_table())
            .map_err(|e| e.to_string())?;
        let removed = table.remove(user_id).map_err(|e| e.to_string())?.is_some();
        if !removed {
            return Err(format!("User not found: {}", user_id));
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}

/// Update a user's role.
pub fn update_role(db: &Database, user_id: &str, role: String) -> Result<User, String> {
    if !["admin", "user", "viewer"].contains(&role.as_str()) {
        return Err(format!(
            "Invalid role: {}. Must be admin, user, or viewer",
            role
        ));
    }
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    let user = {
        let table = tx
            .open_table(Database::get_users_table())
            .map_err(|e| e.to_string())?;
        let existing = table.get(user_id).map_err(|e| e.to_string())?;
        match existing {
            Some(v) => {
                let mut user: User = serde_json::from_str(v.value()).map_err(|e| e.to_string())?;
                user.role = role;
                user.updated_at = chrono::Utc::now().to_rfc3339();
                user
            }
            None => return Err(format!("User not found: {}", user_id)),
        }
    };
    {
        let mut table = tx
            .open_table(Database::get_users_table())
            .map_err(|e| e.to_string())?;
        table
            .insert(
                user_id,
                serde_json::to_string(&user)
                    .map_err(|e| e.to_string())?
                    .as_str(),
            )
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(user)
}

/// Look up a user by username.
pub fn find_by_username(db: &Database, username: &str) -> Result<Option<User>, String> {
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(Database::get_users_table())
        .map_err(|e| e.to_string())?;
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let user: User = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;
        if user.username == username {
            return Ok(Some(user));
        }
    }
    Ok(None)
}

/// Argon2id password hashing (`Argon2::default()` parameters).
pub fn argon2_hash(password: &str) -> Result<String, String> {
    use argon2::{
        password_hash::{PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut rand_core::OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("Argon2 hash error: {}", e))
}

/// Argon2id password verification.
pub fn argon2_verify(password: &str, hash: &str) -> Result<bool, String> {
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

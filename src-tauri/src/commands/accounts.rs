use tauri::State;

use crate::db::schema::Account;
use crate::AppState;

/// List all accounts from the database.
#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> Result<Vec<Account>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::accounts::list(&db)
}

/// Create a new account. The first account becomes active automatically.
#[tauri::command]
pub fn create_account(
    name: String,
    account_type: String,
    path: Option<String>,
    color: Option<String>,
    state: State<'_, AppState>,
) -> Result<Account, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    cybermanju_web::api::accounts::create(&db, name, account_type, path, color)
}

/// Switch the active account.
#[tauri::command]
pub fn switch_account(account_id: String, state: State<'_, AppState>) -> Result<Account, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    cybermanju_web::api::accounts::switch(&db, &account_id)
}

/// Delete an account by its ID. Cannot delete the active account.
#[tauri::command]
pub fn delete_account(account_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    cybermanju_web::api::accounts::delete(&db, &account_id)
}

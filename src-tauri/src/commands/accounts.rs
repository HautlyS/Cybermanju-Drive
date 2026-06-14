use chrono::Utc;
use redb::ReadableTable;
use tauri::State;

use crate::db::schema::Account;
use crate::AppState;

/// List all accounts from the database.
#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> Result<Vec<Account>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(crate::db::Database::get_accounts_table())
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let account: Account = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;
        results.push(account);
    }

    Ok(results)
}

/// Create a new account.
#[tauri::command]
pub fn create_account(
    name: String,
    account_type: String,
    path: Option<String>,
    color: Option<String>,
    state: State<'_, AppState>,
) -> Result<Account, String> {
    let account_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // Check if there are any existing accounts to determine if this should be active
    let db = state.db.write().map_err(|e| e.to_string())?;
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let existing_table = tx_read
        .open_table(crate::db::Database::get_accounts_table())
        .map_err(|e| e.to_string())?;
    let has_existing = existing_table
        .iter()
        .map_err(|e| e.to_string())?
        .next()
        .is_some();
    drop(tx_read);

    let account = Account {
        id: account_id.clone(),
        name,
        account_type,
        path,
        color: color.unwrap_or_else(|| "#6366f1".to_string()),
        is_active: !has_existing, // first account becomes active automatically
        created_at: now.clone(),
        updated_at: now,
    };

    let serialized = serde_json::to_string(&account).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(crate::db::Database::get_accounts_table())
            .map_err(|e| e.to_string())?;
        table
            .insert(account_id.as_str(), serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(account)
}

/// Switch the active account. Sets is_active=true on the target account
/// and is_active=false on all other accounts.
#[tauri::command]
pub fn switch_account(account_id: String, state: State<'_, AppState>) -> Result<Account, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();

    // Read all accounts
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let read_table = tx_read
        .open_table(crate::db::Database::get_accounts_table())
        .map_err(|e| e.to_string())?;

    let mut accounts: Vec<(String, Account)> = Vec::new();
    let mut target_found = false;

    for entry in read_table.iter().map_err(|e| e.to_string())? {
        let (key, value) = entry.map_err(|e| e.to_string())?;
        let mut account: Account =
            serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

        if key.value() == account_id {
            account.is_active = true;
            target_found = true;
        } else {
            account.is_active = false;
        }
        account.updated_at = now.clone();
        accounts.push((key.value().to_string(), account));
    }
    drop(tx_read);

    if !target_found {
        return Err(format!("Account not found: {}", account_id));
    }

    // Write all accounts back in a single transaction
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(crate::db::Database::get_accounts_table())
            .map_err(|e| e.to_string())?;

        for (id, account) in &accounts {
            let serialized = serde_json::to_string(account).map_err(|e| e.to_string())?;
            table
                .insert(id.as_str(), serialized.as_str())
                .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    // Return the newly activated account
    let activated = accounts
        .into_iter()
        .find(|(id, _)| id == &account_id)
        .map(|(_, a)| a)
        .ok_or_else(|| format!("Account not found after write: {}", account_id))?;

    Ok(activated)
}

/// Delete an account by its ID. Cannot delete the active account.
#[tauri::command]
pub fn delete_account(account_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // Verify the account exists and is not active
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let read_table = tx_read
        .open_table(crate::db::Database::get_accounts_table())
        .map_err(|e| e.to_string())?;
    let value = read_table
        .get(account_id.as_str())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;
    let account: Account = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

    if account.is_active {
        return Err(
            "Cannot delete the active account. Switch to another account first.".to_string(),
        );
    }
    drop(tx_read);

    // Delete the account
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(crate::db::Database::get_accounts_table())
            .map_err(|e| e.to_string())?;
        let removed = table
            .remove(account_id.as_str())
            .map_err(|e| e.to_string())?
            .is_some();

        if !removed {
            return Err(format!("Account not found during deletion: {}", account_id));
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(true)
}

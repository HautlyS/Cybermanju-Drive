// Cybermanju Drive — redb Database Layer
// Pure Rust ACID MVCC key-value store for all metadata
// Database wrapper exposes begin_read() / begin_write() so commands can
// call:  state.db.read().map_err()?.begin_read()?  (or .write() for mutations)

pub mod schema;

use redb::{Database as RedbDatabase, ReadTransaction, TableDefinition, WriteTransaction};
use anyhow::Result;

// ---------------------------------------------------------------------------
// redb table definitions  (&str key → &str JSON value)
// ---------------------------------------------------------------------------

const FILES_TABLE: TableDefinition<&str, &str> = TableDefinition::new("files");
const ACCOUNTS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("accounts");
const COLLECTIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("collections");
const COLLECTION_ITEMS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("collection_items");
const FACE_GROUPS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("face_groups");
const LOOSE_GROUPS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("loose_groups");
const ENCRYPTION_KEYS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("encryption_keys");
const LOCATIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("locations");
const USERS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("users");
const USER_FILE_PERMS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("user_file_perms");
const SYNC_CONFIGS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sync_configs");

/// Secondary index: parent_id → JSON array of file_ids.
/// Makes `list_files(parent_path)` an O(1) lookup instead of O(N) full scan.
const PARENT_INDEX_TABLE: TableDefinition<&str, &str> = TableDefinition::new("parent_index");

// ---------------------------------------------------------------------------
// Database wrapper
// ---------------------------------------------------------------------------

pub struct Database {
    db: RedbDatabase,
}

impl Database {
    /// Create (or open) the database and ensure every table exists.
    pub fn new(path: &str) -> Result<Self> {
        let db = RedbDatabase::create(path)?;
        let write_txn = db.begin_write()?;
        {
            write_txn.open_table(FILES_TABLE)?;
            write_txn.open_table(ACCOUNTS_TABLE)?;
            write_txn.open_table(COLLECTIONS_TABLE)?;
            write_txn.open_table(COLLECTION_ITEMS_TABLE)?;
            write_txn.open_table(FACE_GROUPS_TABLE)?;
            write_txn.open_table(LOOSE_GROUPS_TABLE)?;
            write_txn.open_table(ENCRYPTION_KEYS_TABLE)?;
            write_txn.open_table(LOCATIONS_TABLE)?;
            write_txn.open_table(USERS_TABLE)?;
            write_txn.open_table(USER_FILE_PERMS_TABLE)?;
            write_txn.open_table(SYNC_CONFIGS_TABLE)?;
            write_txn.open_table(PARENT_INDEX_TABLE)?;
        }
        write_txn.commit()?;
        Ok(Self { db })
    }

    /// Begin a read transaction — delegates straight to redb.
    pub fn begin_read(&self) -> Result<ReadTransaction> {
        Ok(self.db.begin_read()?)
    }

    /// Begin a write transaction — delegates straight to redb.
    pub fn begin_write(&self) -> Result<WriteTransaction> {
        Ok(self.db.begin_write()?)
    }

    // -----------------------------------------------------------------------
    // Static table-definition accessors — called by command files like:
    //   Database::get_files_table()
    // -----------------------------------------------------------------------

    pub fn get_files_table() -> TableDefinition<&str, &str> {
        FILES_TABLE
    }
    pub fn get_accounts_table() -> TableDefinition<&str, &str> {
        ACCOUNTS_TABLE
    }
    pub fn get_collections_table() -> TableDefinition<&str, &str> {
        COLLECTIONS_TABLE
    }
    pub fn get_collection_items_table() -> TableDefinition<&str, &str> {
        COLLECTION_ITEMS_TABLE
    }
    pub fn get_face_groups_table() -> TableDefinition<&str, &str> {
        FACE_GROUPS_TABLE
    }
    pub fn get_loose_groups_table() -> TableDefinition<&str, &str> {
        LOOSE_GROUPS_TABLE
    }
    pub fn get_encryption_keys_table() -> TableDefinition<&str, &str> {
        ENCRYPTION_KEYS_TABLE
    }
    pub fn get_locations_table() -> TableDefinition<&str, &str> {
        LOCATIONS_TABLE
    }
    pub fn get_users_table() -> TableDefinition<&str, &str> {
        USERS_TABLE
    }
    pub fn get_user_file_perms_table() -> TableDefinition<&str, &str> {
        USER_FILE_PERMS_TABLE
    }
    pub fn get_sync_configs_table() -> TableDefinition<&str, &str> {
        SYNC_CONFIGS_TABLE
    }
    pub fn get_parent_index_table() -> TableDefinition<&str, &str> {
        PARENT_INDEX_TABLE
    }

    // -----------------------------------------------------------------------
    // Parent index helper methods
    // -----------------------------------------------------------------------

    /// Add a file_id to the parent index under the given parent_id.
    /// The index stores `parent_id → JSON array of file_ids`.
    pub fn add_to_parent_index(&self, file_id: &str, parent_id: &str) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(PARENT_INDEX_TABLE)?;
            let existing: Vec<String> = table
                .get(parent_id)?
                .and_then(|v| serde_json::from_str(v.value()).ok())
                .unwrap_or_default();

            let mut ids = existing;
            if !ids.contains(&file_id.to_string()) {
                ids.push(file_id.to_string());
            }

            table.insert(parent_id, serde_json::to_string(&ids)?)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Remove a file_id from the parent index under the given parent_id.
    pub fn remove_from_parent_index(&self, file_id: &str, parent_id: &str) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(PARENT_INDEX_TABLE)?;
            let existing: Vec<String> = table
                .get(parent_id)?
                .and_then(|v| serde_json::from_str(v.value()).ok())
                .unwrap_or_default();

            let mut ids = existing;
            ids.retain(|id| id != file_id);

            if ids.is_empty() {
                // Remove the key entirely if no children remain
                table.remove(parent_id)?;
            } else {
                table.insert(parent_id, serde_json::to_string(&ids)?)?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// List all file_ids that have the given parent_id.
    /// O(1) lookup using the secondary index.
    pub fn list_by_parent(&self, parent_id: &str) -> Result<Vec<String>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(PARENT_INDEX_TABLE)?;
        let value = table.get(parent_id)?;
        match value {
            Some(v) => {
                let ids: Vec<String> = serde_json::from_str(v.value())?;
                Ok(ids)
            }
            None => Ok(Vec::new()),
        }
    }
}
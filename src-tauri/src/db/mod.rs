// Cybermanju Drive — redb Database Layer
// Pure Rust ACID MVCC key-value store for all metadata
// Database wrapper exposes begin_read() / begin_write() so commands can
// call:  state.db.lock().map_err()?.begin_read()?

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
}
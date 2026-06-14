// Cybermanju Drive — redb Database Layer
// Pure Rust ACID MVCC key-value store for all metadata
// Database wrapper exposes begin_read() / begin_write() so commands can
// call:  state.db.read().map_err()?.begin_read()?  (or .write() for mutations)

pub mod schema;

use anyhow::Result;
use redb::{Database as RedbDatabase, ReadTransaction, TableDefinition, WriteTransaction};

// ---------------------------------------------------------------------------
// redb table definitions  (&str key → &str JSON value)
// ---------------------------------------------------------------------------

const FILES_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("files");
const ACCOUNTS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("accounts");
const COLLECTIONS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("collections");
const COLLECTION_ITEMS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("collection_items");
const FACE_GROUPS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("face_groups");
const LOOSE_GROUPS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("loose_groups");
const ENCRYPTION_KEYS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("encryption_keys");
const LOCATIONS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("locations");
const USERS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("users");
const USER_FILE_PERMS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("user_file_perms");
const SYNC_CONFIGS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("sync_configs");

/// Secondary index: parent_id → JSON array of file_ids.
/// Makes `list_files(parent_path)` an O(1) lookup instead of O(N) full scan.
const PARENT_INDEX_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("parent_index");

/// Trash table: stores soft-deleted file nodes for potential restore.
const TRASH_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("trash");

/// Audit log: immutable sequence of all significant operations.
const AUDIT_LOG_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("audit_log");

/// File versions: content snapshots for version control.
const FILE_VERSIONS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("file_versions");

/// Share links: token-based file sharing
const SHARE_LINKS_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("share_links");

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
            write_txn.open_table(TRASH_TABLE)?;
            write_txn.open_table(AUDIT_LOG_TABLE)?;
            write_txn.open_table(FILE_VERSIONS_TABLE)?;
            write_txn.open_table(SHARE_LINKS_TABLE)?;
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

    pub fn get_files_table() -> TableDefinition<'static, &'static str, &'static str> {
        FILES_TABLE
    }
    pub fn get_accounts_table() -> TableDefinition<'static, &'static str, &'static str> {
        ACCOUNTS_TABLE
    }
    pub fn get_collections_table() -> TableDefinition<'static, &'static str, &'static str> {
        COLLECTIONS_TABLE
    }
    pub fn get_collection_items_table() -> TableDefinition<'static, &'static str, &'static str> {
        COLLECTION_ITEMS_TABLE
    }
    pub fn get_face_groups_table() -> TableDefinition<'static, &'static str, &'static str> {
        FACE_GROUPS_TABLE
    }
    pub fn get_loose_groups_table() -> TableDefinition<'static, &'static str, &'static str> {
        LOOSE_GROUPS_TABLE
    }
    pub fn get_encryption_keys_table() -> TableDefinition<'static, &'static str, &'static str> {
        ENCRYPTION_KEYS_TABLE
    }
    pub fn get_locations_table() -> TableDefinition<'static, &'static str, &'static str> {
        LOCATIONS_TABLE
    }
    pub fn get_users_table() -> TableDefinition<'static, &'static str, &'static str> {
        USERS_TABLE
    }
    pub fn get_user_file_perms_table() -> TableDefinition<'static, &'static str, &'static str> {
        USER_FILE_PERMS_TABLE
    }
    pub fn get_sync_configs_table() -> TableDefinition<'static, &'static str, &'static str> {
        SYNC_CONFIGS_TABLE
    }
    pub fn get_parent_index_table() -> TableDefinition<'static, &'static str, &'static str> {
        PARENT_INDEX_TABLE
    }
    pub fn get_trash_table() -> TableDefinition<'static, &'static str, &'static str> {
        TRASH_TABLE
    }
    pub fn get_audit_log_table() -> TableDefinition<'static, &'static str, &'static str> {
        AUDIT_LOG_TABLE
    }
    pub fn get_file_versions_table() -> TableDefinition<'static, &'static str, &'static str> {
        FILE_VERSIONS_TABLE
    }
    pub fn get_share_links_table() -> TableDefinition<'static, &'static str, &'static str> {
        SHARE_LINKS_TABLE
    }

    // -----------------------------------------------------------------------
    // Audit logging helper
    // -----------------------------------------------------------------------

    /// Append an audit entry to the log table.
    pub fn log_audit(
        &self,
        action: &str,
        entity_type: &str,
        entity_id: &str,
        user_id: Option<&str>,
        details: Option<serde_json::Value>,
    ) -> Result<()> {
        let entry = schema::AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            action: action.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            user_id: user_id.map(|s| s.to_string()),
            details,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(AUDIT_LOG_TABLE)?;
            table.insert(entry.id.as_str(), serde_json::to_string(&entry)?.as_str())?;
        }
        tx.commit()?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Trash helpers
    // -----------------------------------------------------------------------

    /// Move a file node to the trash (soft delete).
    pub fn trash_file(
        &self,
        file_id: &str,
        file_node: &schema::FileNode,
        deleted_by: Option<&str>,
    ) -> Result<()> {
        let trash_item = schema::TrashItem {
            id: file_id.to_string(),
            original_file: file_node.clone(),
            deleted_at: chrono::Utc::now().to_rfc3339(),
            deleted_by: deleted_by.map(|s| s.to_string()),
            restore_path: file_node.parent_id.clone(),
        };
        let tx = self.db.begin_write()?;
        {
            let mut trash_table = tx.open_table(TRASH_TABLE)?;
            trash_table.insert(file_id, serde_json::to_string(&trash_item)?.as_str())?;
            let mut files_table = tx.open_table(FILES_TABLE)?;
            files_table.remove(file_id)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Restore a file from trash back to its original location.
    pub fn restore_from_trash(&self, file_id: &str) -> Result<Option<schema::TrashItem>> {
        let tx = self.db.begin_write()?;
        let result = {
            let trash_table = tx.open_table(TRASH_TABLE)?;
            let found: Option<schema::TrashItem> = trash_table
                .get(file_id)?
                .map(|v| serde_json::from_str::<schema::TrashItem>(v.value()).unwrap());
            found
        };
        if let Some(ref item) = result {
            let mut files_table = tx.open_table(FILES_TABLE)?;
            let serialized = serde_json::to_string(&item.original_file)?;
            files_table.insert(file_id, serialized.as_str())?;
            let mut trash_table = tx.open_table(TRASH_TABLE)?;
            trash_table.remove(file_id)?;
        }
        tx.commit()?;
        Ok(result)
    }

    /// List all items currently in the trash.
    pub fn list_trash(&self) -> Result<Vec<schema::TrashItem>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(TRASH_TABLE)?;
        let mut items = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            if let Ok(item) = serde_json::from_str::<schema::TrashItem>(value.value()) {
                items.push(item);
            }
        }
        Ok(items)
    }

    /// Permanently delete all items from the trash.
    pub fn empty_trash(&self) -> Result<u32> {
        let tx = self.db.begin_write()?;
        let mut count = 0u32;
        {
            let trash_table = tx.open_table(TRASH_TABLE)?;
            let keys: Vec<String> = trash_table
                .iter()?
                .filter_map(|e| e.ok().map(|(k, _)| k.value().to_string()))
                .collect();
            drop(trash_table);
            let mut trash_table = tx.open_table(TRASH_TABLE)?;
            for key in keys {
                trash_table.remove(key.as_str())?;
                count += 1;
            }
        }
        tx.commit()?;
        Ok(count)
    }

    // -----------------------------------------------------------------------
    // File versioning helpers
    // -----------------------------------------------------------------------

    /// Create a new version snapshot for a file.
    pub fn create_file_version(
        &self,
        file_node: &schema::FileNode,
        snapshot_data: Option<&str>,
    ) -> Result<schema::FileVersion> {
        let tx = self.db.begin_write()?;
        let version = {
            let versions_table = tx.open_table(FILE_VERSIONS_TABLE)?;
            let existing: Vec<schema::FileVersion> = versions_table
                .iter()?
                .filter_map(|e| e.ok())
                .filter(|(k, _)| k.value().starts_with(&format!("{}/", file_node.id)))
                .filter_map(|(_, v)| serde_json::from_str::<schema::FileVersion>(v.value()).ok())
                .collect();
            let next_ver = existing.iter().map(|v| v.version_number).max().unwrap_or(0) + 1;
            schema::FileVersion {
                id: format!("{}/v{}", file_node.id, next_ver),
                file_id: file_node.id.clone(),
                version_number: next_ver,
                hash_blake3: file_node.hash_blake3.clone(),
                size_bytes: file_node.size_bytes,
                snapshot_data: snapshot_data.map(|s| s.to_string()),
                created_at: chrono::Utc::now().to_rfc3339(),
            }
        };
        {
            let mut versions_table = tx.open_table(FILE_VERSIONS_TABLE)?;
            versions_table.insert(
                version.id.as_str(),
                serde_json::to_string(&version)?.as_str(),
            )?;
        }
        tx.commit()?;
        Ok(version)
    }

    /// List all versions of a file.
    pub fn list_file_versions(&self, file_id: &str) -> Result<Vec<schema::FileVersion>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(FILE_VERSIONS_TABLE)?;
        let versions: Vec<schema::FileVersion> = table
            .iter()?
            .filter_map(|e| e.ok())
            .filter(|(k, _)| k.value().starts_with(&format!("{}/", file_id)))
            .filter_map(|(_, v)| serde_json::from_str::<schema::FileVersion>(v.value()).ok())
            .collect();
        Ok(versions)
    }

    /// Revert a file to a specific version.
    pub fn revert_file_version(
        &self,
        file_id: &str,
        version_id: &str,
    ) -> Result<Option<schema::FileVersion>> {
        let tx = self.db.begin_write()?;
        let version = {
            let versions_table = tx.open_table(FILE_VERSIONS_TABLE)?;
            let found: Option<schema::FileVersion> = versions_table
                .get(version_id)?
                .and_then(|v| serde_json::from_str::<schema::FileVersion>(v.value()).ok());
            found
        };
        if let Some(ref ver) = version {
            let mut files_table = tx.open_table(FILES_TABLE)?;
            let existing_node: Option<schema::FileNode> = files_table
                .get(file_id)?
                .map(|v| serde_json::from_str::<schema::FileNode>(v.value()).unwrap());
            if let Some(mut file_node) = existing_node {
                file_node.hash_blake3 = ver.hash_blake3.clone();
                file_node.size_bytes = ver.size_bytes;
                file_node.modified_at = chrono::Utc::now().to_rfc3339();
                files_table.insert(file_id, serde_json::to_string(&file_node)?.as_str())?;
            }
        }
        tx.commit()?;
        Ok(version)
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

            table.insert(parent_id, serde_json::to_string(&ids)?.as_str())?;
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
                table.insert(parent_id, serde_json::to_string(&ids)?.as_str())?;
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

    /// Insert a FileNode AND update the parent index in a single transaction.
    /// This ensures atomicity — a crash between the two writes is impossible.
    pub fn insert_file_with_index(
        &self,
        file_id: &str,
        serialized: &str,
        parent_id: Option<&str>,
    ) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut files_table = tx.open_table(FILES_TABLE)?;
            files_table.insert(file_id, serialized)?;

            if let Some(pid) = parent_id {
                let mut index_table = tx.open_table(PARENT_INDEX_TABLE)?;
                let existing: Vec<String> = index_table
                    .get(pid)?
                    .and_then(|v| serde_json::from_str(v.value()).ok())
                    .unwrap_or_default();
                let mut ids = existing;
                if !ids.contains(&file_id.to_string()) {
                    ids.push(file_id.to_string());
                }
                index_table.insert(pid, serde_json::to_string(&ids)?.as_str())?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Remove a file from the files table AND its parent index in a single transaction.
    pub fn remove_file_with_index(&self, file_id: &str, parent_id: Option<&str>) -> Result<bool> {
        let tx = self.db.begin_write()?;
        let removed = {
            let mut files_table = tx.open_table(FILES_TABLE)?;
            let result = files_table.remove(file_id)?.is_some();
            result
        };

        if removed {
            // Remove from parent index in the same transaction
            if let Some(pid) = parent_id {
                let mut index_table = tx.open_table(PARENT_INDEX_TABLE)?;
                let existing: Vec<String> = index_table
                    .get(pid)?
                    .and_then(|v| serde_json::from_str(v.value()).ok())
                    .unwrap_or_default();
                let mut ids = existing;
                ids.retain(|id| id != file_id);
                if ids.is_empty() {
                    index_table.remove(pid)?;
                } else {
                    index_table.insert(pid, serde_json::to_string(&ids)?.as_str())?;
                }
            }
        }

        tx.commit()?;
        Ok(removed)
    }

    // -----------------------------------------------------------------------
    // Share link helpers
    // -----------------------------------------------------------------------

    /// Generate a share link for a file with an expiry duration.
    pub fn create_share_link(
        &self,
        file_id: &str,
        expires_in_hours: u64,
    ) -> Result<schema::ShareLink> {
        use base64::Engine;
        use rand_core::RngCore;
        let mut token_bytes = [0u8; 32];
        rand_core::OsRng.fill_bytes(&mut token_bytes);
        let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_bytes);

        let now = chrono::Utc::now();
        let expires_at = if expires_in_hours > 0 {
            (now + chrono::Duration::hours(expires_in_hours as i64)).to_rfc3339()
        } else {
            (now + chrono::Duration::days(365)).to_rfc3339()
        };

        let link = schema::ShareLink {
            id: uuid::Uuid::new_v4().to_string(),
            file_id: file_id.to_string(),
            token: token.clone(),
            expires_at,
            created_at: now.to_rfc3339(),
            url: None,
        };

        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(SHARE_LINKS_TABLE)?;
            table.insert(link.id.as_str(), serde_json::to_string(&link)?.as_str())?;
        }
        tx.commit()?;
        Ok(link)
    }

    /// Look up a share link by token.
    pub fn get_share_link_by_token(&self, token: &str) -> Result<Option<schema::ShareLink>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(SHARE_LINKS_TABLE)?;
        for entry in table.iter()? {
            let (_, value) = entry?;
            let link: schema::ShareLink = serde_json::from_str(value.value())?;
            if link.token == token {
                return Ok(Some(link));
            }
        }
        Ok(None)
    }

    /// Get file node by ID.
    pub fn get_file_node(&self, file_id: &str) -> Result<Option<schema::FileNode>> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(FILES_TABLE)?;
        match table.get(file_id)? {
            Some(v) => Ok(Some(serde_json::from_str(v.value())?)),
            None => Ok(None),
        }
    }

    /// Move a file between parent indices in a single transaction with the file update.
    pub fn move_file_with_index(
        &self,
        file_id: &str,
        serialized: &str,
        old_parent_id: Option<&str>,
        new_parent_id: &str,
    ) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            // Update file record
            let mut files_table = tx.open_table(FILES_TABLE)?;
            files_table.insert(file_id, serialized)?;

            // Remove from old parent index
            if let Some(old_pid) = old_parent_id {
                let mut index_table = tx.open_table(PARENT_INDEX_TABLE)?;
                let existing: Vec<String> = index_table
                    .get(old_pid)?
                    .and_then(|v| serde_json::from_str(v.value()).ok())
                    .unwrap_or_default();
                let mut ids = existing;
                ids.retain(|id| id != file_id);
                if ids.is_empty() {
                    index_table.remove(old_pid)?;
                } else {
                    index_table.insert(old_pid, serde_json::to_string(&ids)?.as_str())?;
                }
            }

            // Add to new parent index
            let mut index_table = tx.open_table(PARENT_INDEX_TABLE)?;
            let existing: Vec<String> = index_table
                .get(new_parent_id)?
                .and_then(|v| serde_json::from_str(v.value()).ok())
                .unwrap_or_default();
            let mut ids = existing;
            if !ids.contains(&file_id.to_string()) {
                ids.push(file_id.to_string());
            }
            index_table.insert(new_parent_id, serde_json::to_string(&ids)?.as_str())?;
        }
        tx.commit()?;
        Ok(())
    }
}

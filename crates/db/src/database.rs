use anyhow::Result;
use redb::{
    Database as RedbDatabase, ReadTransaction, ReadableTable, TableDefinition, WriteTransaction,
};

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
const PARENT_INDEX_TABLE: TableDefinition<'static, &'static str, &'static str> =
    TableDefinition::new("parent_index");

pub struct Database {
    db: RedbDatabase,
}

impl Database {
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

    pub fn begin_read(&self) -> Result<ReadTransaction> {
        Ok(self.db.begin_read()?)
    }

    pub fn begin_write(&self) -> Result<WriteTransaction> {
        Ok(self.db.begin_write()?)
    }

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
                table.remove(parent_id)?;
            } else {
                table.insert(parent_id, serde_json::to_string(&ids)?.as_str())?;
            }
        }
        tx.commit()?;
        Ok(())
    }

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

    pub fn remove_file_with_index(&self, file_id: &str, parent_id: Option<&str>) -> Result<bool> {
        let tx = self.db.begin_write()?;
        let removed = {
            let mut files_table = tx.open_table(FILES_TABLE)?;
            let result = files_table.remove(file_id)?.is_some();
            result
        };
        if removed {
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

    pub fn move_file_with_index(
        &self,
        file_id: &str,
        serialized: &str,
        old_parent_id: Option<&str>,
        new_parent_id: &str,
    ) -> Result<()> {
        let tx = self.db.begin_write()?;
        {
            let mut files_table = tx.open_table(FILES_TABLE)?;
            files_table.insert(file_id, serialized)?;
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

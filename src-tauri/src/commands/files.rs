use chrono::Utc;
use redb::ReadableTable;
use tauri::State;

use crate::db::schema::FileNode;
use crate::db::schema::LooseGroup;
use crate::AppState;

/// List all file nodes whose parent_id matches the given parent_path.
/// Uses the parent_index secondary index for O(1) lookup instead of O(N) full scan.
#[tauri::command]
pub fn list_files(
    parent_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<FileNode>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;

    // Use the parent index for O(1) lookup
    let file_ids = db.list_by_parent(&parent_path).map_err(|e| e.to_string())?;

    if file_ids.is_empty() {
        // No entries in the parent index — return empty
        return Ok(Vec::new());
    }

    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let read_table = tx
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for file_id in &file_ids {
        match read_table
            .get(file_id.as_str())
            .map_err(|e| e.to_string())?
        {
            Some(value) => {
                match serde_json::from_str::<FileNode>(value.value()) {
                    Ok(node) => results.push(node),
                    Err(_) => {
                        // Stale index entry — file was deleted but index wasn't updated.
                        // Clean up the index entry in the background.
                        log::warn!("Stale parent index entry for file_id={}", file_id);
                    }
                }
            }
            None => {
                // File was deleted but parent index wasn't updated — skip
                log::warn!("Parent index references non-existent file_id={}", file_id);
            }
        }
    }

    Ok(results)
}

/// Get a single file node by its ID.
#[tauri::command]
pub fn get_file(file_id: String, state: State<'_, AppState>) -> Result<FileNode, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;

    let value = table
        .get(file_id.as_str())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", file_id))?;

    let file_node: FileNode = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;
    Ok(file_node)
}

/// Create a new folder entry in the database.
#[tauri::command]
pub fn create_folder(
    name: String,
    parent_id: String,
    state: State<'_, AppState>,
) -> Result<FileNode, String> {
    let folder_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let folder = FileNode {
        id: folder_id.clone(),
        name,
        file_type: "folder".to_string(),
        parent_id: Some(parent_id.clone()),
        size_bytes: 0,
        mime_type: None,
        hash_blake3: None,
        encrypted: false,
        encryption_algorithm: None,
        compression_layers: Vec::new(),
        thumbnail_path: None,
        created_at: now.clone(),
        modified_at: now,
        context_data: None,
        tags: Vec::new(),
        collection_ids: Vec::new(),
        face_group_ids: Vec::new(),
        loose_group_ids: Vec::new(),
        gps_lat: None,
        gps_lon: None,
    };

    let db = state.db.write().map_err(|e| e.to_string())?;
    let serialized = serde_json::to_string(&folder).map_err(|e| e.to_string())?;
    db.insert_file_with_index(&folder_id, serialized.as_str(), Some(&parent_id))
        .map_err(|e| e.to_string())?;

    Ok(folder)
}

/// Delete a file or folder by its ID.
#[tauri::command]
pub fn delete_file(file_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // First read the file node to get its parent_id for index cleanup
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let table_read = tx_read
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;
    let value = table_read
        .get(file_id.as_str())
        .map_err(|e| e.to_string())?;
    let parent_id = value.and_then(|val| {
        serde_json::from_str::<FileNode>(val.value())
            .ok()
            .and_then(|node| node.parent_id)
    });
    drop(tx_read);

    // Atomically remove from both files table and parent index
    let removed = db
        .remove_file_with_index(&file_id, parent_id.as_deref())
        .map_err(|e| e.to_string())?;
    if !removed {
        return Err(format!("File not found: {}", file_id));
    }

    Ok(true)
}

/// Rename a file or folder.
#[tauri::command]
pub fn rename_file(
    file_id: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<FileNode, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // Read existing
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let table_read = tx_read
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;
    let value = table_read
        .get(file_id.as_str())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", file_id))?;
    let mut file_node: FileNode = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

    file_node.name = new_name;
    file_node.modified_at = Utc::now().to_rfc3339();

    // Write back
    let serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;
        table
            .insert(file_id.as_str(), serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(file_node)
}

/// Context-preserving duplication: copies a file node, preserves context_data,
/// generates a new blake3 hash placeholder, creates a link-style preview reference,
/// and stores in redb.
#[tauri::command]
pub fn duplicate_file_context(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<FileNode, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // Read the original file node
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let table_read = tx_read
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;
    let value = table_read
        .get(file_id.as_str())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", file_id))?;
    let original: FileNode = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

    let new_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // A duplicate contains identical bytes — its content hash is the same.
    // The new_id distinguishes it as a separate FileNode.
    let new_hash = original.hash_blake3.clone();
    // If the original was never hashed (e.g. a folder), leave it as None.

    // Build context link reference
    let link_preview = serde_json::json!({
        "type": "duplicate_link",
        "source_file_id": file_id,
        "source_hash": original.hash_blake3,
        "duplicated_at": now,
    });

    // Preserve context_data and augment with duplication metadata
    let mut context_data = original
        .context_data
        .clone()
        .unwrap_or(serde_json::Value::Null);
    if let Some(obj) = context_data.as_object_mut() {
        obj.insert("duplicated_from".to_string(), serde_json::json!(file_id));
        obj.insert("duplicate_created_at".to_string(), serde_json::json!(now));
    }

    // Clone fields needed after the move BEFORE moving
    let tags = original.tags.clone();

    let mut duplicated = original;
    duplicated.id = new_id.clone();
    duplicated.name = format!("{} (copy)", duplicated.name);
    duplicated.hash_blake3 = new_hash;
    duplicated.thumbnail_path = Some(link_preview.to_string());
    duplicated.created_at = now.clone();
    duplicated.modified_at = now;
    duplicated.context_data = Some(context_data);
    duplicated.tags = tags;
    duplicated.collection_ids = Vec::new(); // fresh copy is not in any collection yet

    // Store the duplicated file node atomically with parent index update
    let serialized = serde_json::to_string(&duplicated).map_err(|e| e.to_string())?;
    db.insert_file_with_index(
        &new_id,
        serialized.as_str(),
        duplicated.parent_id.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    Ok(duplicated)
}

/// Move a file to a new parent.
#[tauri::command]
pub fn move_file(
    file_id: String,
    new_parent_id: String,
    state: State<'_, AppState>,
) -> Result<FileNode, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // Read existing
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let table_read = tx_read
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;
    let value = table_read
        .get(file_id.as_str())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", file_id))?;
    let mut file_node: FileNode = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

    // Update parent index: remove from old parent, add to new parent
    let old_parent = file_node.parent_id.clone();
    file_node.parent_id = Some(new_parent_id.clone());
    file_node.modified_at = Utc::now().to_rfc3339();

    // Write back the updated file node + update indices atomically
    let serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
    db.move_file_with_index(
        &file_id,
        serialized.as_str(),
        old_parent.as_deref(),
        &new_parent_id,
    )
    .map_err(|e| e.to_string())?;

    Ok(file_node)
}

/// Get preview metadata for a file.
#[tauri::command]
pub fn get_preview(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;

    let value = table
        .get(file_id.as_str())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", file_id))?;

    let file_node: FileNode = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

    let preview = serde_json::json!({
        "file_id": file_node.id,
        "name": file_node.name,
        "file_type": file_node.file_type,
        "mime_type": file_node.mime_type,
        "size_bytes": file_node.size_bytes,
        "thumbnail_path": file_node.thumbnail_path,
        "encrypted": file_node.encrypted,
        "compression_layers": file_node.compression_layers,
        "tags": file_node.tags,
        "gps_lat": file_node.gps_lat,
        "gps_lon": file_node.gps_lon,
        "context_data": file_node.context_data,
    });

    Ok(preview)
}

/// Create a new loose file group.
#[tauri::command]
pub fn create_loose_group(
    name: String,
    color: String,
    state: State<'_, AppState>,
) -> Result<LooseGroup, String> {
    let group_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let group = LooseGroup {
        id: group_id.clone(),
        name,
        color,
        file_ids: Vec::new(),
        created_at: now,
    };

    let db = state.db.write().map_err(|e| e.to_string())?;
    let serialized = serde_json::to_string(&group).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(crate::db::Database::get_loose_groups_table())
            .map_err(|e| e.to_string())?;
        table
            .insert(group_id.as_str(), serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(group)
}

/// Add a file to a loose group. Also updates the file node's loose_group_ids.
#[tauri::command]
pub fn add_to_loose_group(
    group_id: String,
    file_id: String,
    state: State<'_, AppState>,
) -> Result<LooseGroup, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // Read the group
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let group_table = tx_read
        .open_table(crate::db::Database::get_loose_groups_table())
        .map_err(|e| e.to_string())?;
    let group_value = group_table
        .get(group_id.as_str())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Loose group not found: {}", group_id))?;
    let mut group: LooseGroup =
        serde_json::from_str(group_value.value()).map_err(|e| e.to_string())?;

    if !group.file_ids.contains(&file_id) {
        group.file_ids.push(file_id.clone());
    }

    // Read the file node
    let file_table = tx_read
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;
    let file_value = file_table
        .get(file_id.as_str())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", file_id))?;
    let mut file_node: FileNode =
        serde_json::from_str(file_value.value()).map_err(|e| e.to_string())?;

    if !file_node.loose_group_ids.contains(&group_id) {
        file_node.loose_group_ids.push(group_id.clone());
    }
    drop(tx_read);

    // Write both back in a single transaction
    let group_serialized = serde_json::to_string(&group).map_err(|e| e.to_string())?;
    let file_serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut gt = tx
            .open_table(crate::db::Database::get_loose_groups_table())
            .map_err(|e| e.to_string())?;
        gt.insert(group_id.as_str(), group_serialized.as_str())
            .map_err(|e| e.to_string())?;

        let mut ft = tx
            .open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;
        ft.insert(file_id.as_str(), file_serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(group)
}

/// List all loose groups.
#[tauri::command]
pub fn list_loose_groups(state: State<'_, AppState>) -> Result<Vec<LooseGroup>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(crate::db::Database::get_loose_groups_table())
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let group: LooseGroup = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;
        results.push(group);
    }

    Ok(results)
}

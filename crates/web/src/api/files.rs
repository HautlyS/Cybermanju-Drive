// Cybermanju Drive — File tree operations (shared by Tauri IPC and REST)

use cybermanju_db::Database;
use cybermanju_types::schema::FileNode;
use redb::ReadableTable;

/// Create a new folder entry in the database.
pub fn create_folder(db: &Database, name: String, parent_id: String) -> Result<FileNode, String> {
    let folder_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

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

    let serialized = serde_json::to_string(&folder).map_err(|e| e.to_string())?;
    db.insert_file_with_index(&folder_id, serialized.as_str(), Some(&parent_id))
        .map_err(|e| e.to_string())?;

    Ok(folder)
}

/// Soft-delete a file or folder (moves it to the trash).
pub fn delete(db: &Database, file_id: &str) -> Result<bool, String> {
    let node = read_file(db, file_id)?;
    let parent_id = node.parent_id.clone();

    db.trash_file(file_id, &node, None)
        .map_err(|e| e.to_string())?;

    if let Some(pid) = &parent_id {
        db.remove_from_parent_index(file_id, pid)
            .map_err(|e| e.to_string())?;
    }

    db.log_audit("delete", "file", file_id, None, None)
        .map_err(|e| e.to_string())?;

    Ok(true)
}

/// Rename a file or folder.
pub fn rename(db: &Database, file_id: &str, new_name: String) -> Result<FileNode, String> {
    let mut file_node = read_file(db, file_id)?;

    file_node.name = new_name;
    file_node.modified_at = chrono::Utc::now().to_rfc3339();

    write_file(db, file_id, &file_node)?;
    Ok(file_node)
}

/// Context-preserving duplication: copies a file node and preserves context_data.
pub fn duplicate(db: &Database, file_id: &str) -> Result<FileNode, String> {
    let original = read_file(db, file_id)?;

    let new_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // A duplicate contains identical bytes — its content hash is the same.
    let new_hash = original.hash_blake3.clone();

    let link_preview = serde_json::json!({
        "type": "duplicate_link",
        "source_file_id": file_id,
        "source_hash": original.hash_blake3,
        "duplicated_at": now,
    });

    let mut context_data = original
        .context_data
        .clone()
        .unwrap_or(serde_json::Value::Null);
    if let Some(obj) = context_data.as_object_mut() {
        obj.insert("duplicated_from".to_string(), serde_json::json!(file_id));
        obj.insert("duplicate_created_at".to_string(), serde_json::json!(now));
    }

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
    duplicated.collection_ids = Vec::new();

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
pub fn move_to(db: &Database, file_id: &str, new_parent_id: String) -> Result<FileNode, String> {
    let mut file_node = read_file(db, file_id)?;

    let old_parent = file_node.parent_id.clone();
    file_node.parent_id = Some(new_parent_id.clone());
    file_node.modified_at = chrono::Utc::now().to_rfc3339();

    let serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
    db.move_file_with_index(
        file_id,
        serialized.as_str(),
        old_parent.as_deref(),
        &new_parent_id,
    )
    .map_err(|e| e.to_string())?;

    Ok(file_node)
}

/// Rebuild the parent index from all FileNodes in the files table.
pub fn rebuild_parent_index(db: &Database) -> Result<u32, String> {
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx_read
        .open_table(Database::get_files_table())
        .map_err(|e| e.to_string())?;

    let file_nodes: Vec<FileNode> = table
        .iter()
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let (_, value) = entry.ok()?;
            serde_json::from_str::<FileNode>(value.value()).ok()
        })
        .collect();
    drop(tx_read);

    let tx_write = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut index_table = tx_write
            .open_table(Database::get_parent_index_table())
            .map_err(|e| e.to_string())?;
        let keys: Vec<String> = index_table
            .iter()
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok().map(|(k, _)| k.value().to_string()))
            .collect();
        for key in keys {
            index_table
                .remove(key.as_str())
                .map_err(|e| e.to_string())?;
        }
    }
    tx_write.commit().map_err(|e| e.to_string())?;

    let mut count = 0u32;
    for node in &file_nodes {
        if let Some(ref parent_id) = node.parent_id {
            db.add_to_parent_index(&node.id, parent_id)
                .map_err(|e| e.to_string())?;
            count += 1;
        }
    }

    Ok(count)
}

/// Preview metadata for a file.
pub fn preview(db: &Database, file_id: &str) -> Result<serde_json::Value, String> {
    let file_node = read_file(db, file_id)?;

    Ok(serde_json::json!({
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
    }))
}

fn read_file(db: &Database, file_id: &str) -> Result<FileNode, String> {
    db.get_file_node(file_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", file_id))
}

fn write_file(db: &Database, file_id: &str, node: &FileNode) -> Result<(), String> {
    let serialized = serde_json::to_string(node).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(Database::get_files_table())
            .map_err(|e| e.to_string())?;
        table
            .insert(file_id, serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

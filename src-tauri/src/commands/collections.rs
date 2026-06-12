use chrono::Utc;
use tauri::State;

use crate::db::schema::{Collection, CollectionItem, FileNode};
use crate::AppState;

/// List all collections from the database.
#[tauri::command]
pub fn list_collections(state: State<'_, AppState>) -> Result<Vec<Collection>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(crate::db::Database::get_collections_table())
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let collection: Collection =
            serde_json::from_str(&value.value()).map_err(|e| e.to_string())?;
        results.push(collection);
    }

    Ok(results)
}

/// Create a new collection.
#[tauri::command]
pub fn create_collection(
    name: String,
    collection_type: String,
    color: String,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<Collection, String> {
    let collection_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let collection = Collection {
        id: collection_id.clone(),
        name,
        collection_type,
        color,
        description,
        item_ids: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
    };

    let db = state.db.write().map_err(|e| e.to_string())?;
    let serialized = serde_json::to_string(&collection).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(crate::db::Database::get_collections_table())
            .map_err(|e| e.to_string())?;
        table
            .insert(&collection_id, serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(collection)
}

/// Add a file to a collection. Creates a CollectionItem and updates both the collection
/// and the file node's collection_ids.
#[tauri::command]
pub fn add_to_collection(
    collection_id: String,
    file_id: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<CollectionItem, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // Verify the collection exists
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let coll_table = tx_read
        .open_table(crate::db::Database::get_collections_table())
        .map_err(|e| e.to_string())?;
    let coll_value = coll_table
        .get(&collection_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Collection not found: {}", collection_id))?;
    let mut collection: Collection =
        serde_json::from_str(&coll_value.value()).map_err(|e| e.to_string())?;

    // Verify the file exists
    let file_table = tx_read
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;
    let file_value = file_table
        .get(&file_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", file_id))?;
    let mut file_node: FileNode =
        serde_json::from_str(&file_value.value()).map_err(|e| e.to_string())?;
    drop(tx_read);

    let now = Utc::now().to_rfc3339();

    // Create the collection item
    let item_id = uuid::Uuid::new_v4().to_string();
    let collection_item = CollectionItem {
        id: item_id.clone(),
        collection_id: collection_id.clone(),
        file_id: file_id.clone(),
        note,
        added_at: now.clone(),
    };

    // Update the collection's item list
    if !collection.item_ids.contains(&item_id) {
        collection.item_ids.push(item_id.clone());
    }
    collection.updated_at = now.clone();

    // Update the file node's collection_ids
    if !file_node.collection_ids.contains(&collection_id) {
        file_node.collection_ids.push(collection_id.clone());
    }
    file_node.modified_at = now;

    // Write all changes in a single transaction
    let coll_serialized = serde_json::to_string(&collection).map_err(|e| e.to_string())?;
    let file_serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
    let item_serialized = serde_json::to_string(&collection_item).map_err(|e| e.to_string())?;

    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        // Update collection
        let mut ct = tx
            .open_table(crate::db::Database::get_collections_table())
            .map_err(|e| e.to_string())?;
        ct.insert(&collection_id, coll_serialized.as_str())
            .map_err(|e| e.to_string())?;

        // Update file node
        let mut ft = tx
            .open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;
        ft.insert(&file_id, file_serialized.as_str())
            .map_err(|e| e.to_string())?;

        // Store collection item
        let mut it = tx
            .open_table(crate::db::Database::get_collection_items_table())
            .map_err(|e| e.to_string())?;
        it.insert(&item_id, item_serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(collection_item)
}

/// Remove a file from a collection. Cleans up both the collection's item list
/// and the file node's collection_ids.
#[tauri::command]
pub fn remove_from_collection(
    collection_id: String,
    file_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // Read collection and file
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let coll_table = tx_read
        .open_table(crate::db::Database::get_collections_table())
        .map_err(|e| e.to_string())?;
    let coll_value = coll_table
        .get(&collection_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Collection not found: {}", collection_id))?;
    let mut collection: Collection =
        serde_json::from_str(&coll_value.value()).map_err(|e| e.to_string())?;

    // Find the collection item that matches
    let items_table = tx_read
        .open_table(crate::db::Database::get_collection_items_table())
        .map_err(|e| e.to_string())?;
    let mut item_id_to_remove: Option<String> = None;
    for entry in items_table.iter().map_err(|e| e.to_string())? {
        let (key, value) = entry.map_err(|e| e.to_string())?;
        let item: CollectionItem =
            serde_json::from_str(&value.value()).map_err(|e| e.to_string())?;
        if item.collection_id == collection_id && item.file_id == file_id {
            item_id_to_remove = Some(key.value().to_string());
            break;
        }
    }

    // Read file node
    let file_table = tx_read
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;
    let file_value = file_table.get(&file_id).map_err(|e| e.to_string())?;
    drop(tx_read);

    let now = Utc::now().to_rfc3339();

    // Update collection
    collection.item_ids.retain(|id| {
        item_id_to_remove
            .as_ref()
            .map_or(true, |to_remove| id != to_remove)
    });
    collection.updated_at = now.clone();

    // Write changes in a single transaction
    let coll_serialized = serde_json::to_string(&collection).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        // Update collection
        let mut ct = tx
            .open_table(crate::db::Database::get_collections_table())
            .map_err(|e| e.to_string())?;
        ct.insert(&collection_id, coll_serialized.as_str())
            .map_err(|e| e.to_string())?;

        // Remove collection item if found
        if let Some(ref item_id) = item_id_to_remove {
            let mut it = tx
                .open_table(crate::db::Database::get_collection_items_table())
                .map_err(|e| e.to_string())?;
            it.remove(item_id.as_str()).map_err(|e| e.to_string())?;
        }

        // Update file node if it exists
        if let Some(fv) = file_value {
            let mut file_node: FileNode =
                serde_json::from_str(&fv.value()).map_err(|e| e.to_string())?;
            file_node.collection_ids.retain(|id| id != &collection_id);
            file_node.modified_at = now;
            let file_serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
            let mut ft = tx
                .open_table(crate::db::Database::get_files_table())
                .map_err(|e| e.to_string())?;
            ft.insert(&file_id, file_serialized.as_str())
                .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(true)
}

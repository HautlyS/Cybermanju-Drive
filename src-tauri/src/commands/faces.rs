use tauri::State;
use serde::{Deserialize, Serialize};
use chrono::Utc;

use crate::AppState;
use crate::db::schema::{FileNode, FaceGroup};

/// Result of face detection for a single file.
#[derive(Debug, Serialize, Deserialize)]
pub struct FaceDetectionResult {
    pub file_id: String,
    pub faces_detected: usize,
    pub face_group_ids: Vec<String>,
}

/// Trigger face detection on a file using the ONNX runtime model.
/// This delegates to the crate::faces module for the actual ML inference.
#[tauri::command]
pub fn detect_faces(file_id: String, state: State<'_, AppState>) -> Result<FaceDetectionResult, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // Verify file exists
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let file_table = tx_read.open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;
    let file_value = file_table.get(&file_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("File not found: {}", file_id))?;
    let mut file_node: FileNode = serde_json::from_str(&file_value.value())
        .map_err(|e| e.to_string())?;
    drop(tx_read);

    // Delegate to the faces module for ONNX inference
    // The faces module handles loading the model, preprocessing the image,
    // running inference with ort, and returning detected face embeddings.
    let detected_faces = crate::faces::detect_faces_in_file(&file_node)
        .map_err(|e| e.to_string())?;

    let now = Utc::now().to_rfc3339();
    let mut face_group_ids = Vec::new();

    // Process detected faces: cluster or assign to existing groups
    for (face_index, embedding) in detected_faces.iter().enumerate() {
        // Try to match against existing face groups
        let tx_read = db.begin_read().map_err(|e| e.to_string())?;
        let fg_table = tx_read.open_table(crate::db::Database::get_face_groups_table())
            .map_err(|e| e.to_string())?;
        let mut matched_group_id: Option<String> = None;

        for entry in fg_table.iter().map_err(|e| e.to_string())? {
            let (key, value) = entry.map_err(|e| e.to_string())?;
            let group: FaceGroup = serde_json::from_str(&value.value())
                .map_err(|e| e.to_string())?;

            // Check if this embedding is close to the group's centroid
            if let Some(ref centroid) = group.centroid_embedding {
                if crate::faces::embedding_distance(embedding, centroid) < 0.6 {
                    matched_group_id = Some(key.value().to_string());
                    break;
                }
            }
        }
        drop(tx_read);

        if let Some(group_id) = matched_group_id {
            // Add file to existing group
            let tx_r2 = db.begin_read().map_err(|e| e.to_string())?;
            let gt = tx_r2.open_table(crate::db::Database::get_face_groups_table())
                .map_err(|e| e.to_string())?;
            let gv = gt.get(&group_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Face group not found: {}", group_id))?;
            let mut group: FaceGroup = serde_json::from_str(&gv.value())
                .map_err(|e| e.to_string())?;
            drop(tx_r2);

            if !group.file_ids.contains(&file_id) {
                group.file_ids.push(file_id.clone());
            }
            face_group_ids.push(group_id.clone());

            let serialized = serde_json::to_string(&group).map_err(|e| e.to_string())?;
            let tx = db.begin_write().map_err(|e| e.to_string())?;
            {
                let mut wt = tx.open_table(crate::db::Database::get_face_groups_table())
                    .map_err(|e| e.to_string())?;
                wt.insert(&group_id, serialized.as_str())
                    .map_err(|e| e.to_string())?;
            }
            tx.commit().map_err(|e| e.to_string())?;
        } else {
            // Create a new face group for this face
            let new_group_id = uuid::Uuid::new_v4().to_string();
            let group_name = format!("Person {}", face_index + 1);

            let new_group = FaceGroup {
                id: new_group_id.clone(),
                name: group_name,
                file_ids: vec![file_id.clone()],
                centroid_embedding: Some(embedding.clone()),
                created_at: now.clone(),
            };

            face_group_ids.push(new_group_id.clone());

            let serialized = serde_json::to_string(&new_group).map_err(|e| e.to_string())?;
            let tx = db.begin_write().map_err(|e| e.to_string())?;
            {
                let mut wt = tx.open_table(crate::db::Database::get_face_groups_table())
                    .map_err(|e| e.to_string())?;
                wt.insert(&new_group_id, serialized.as_str())
                    .map_err(|e| e.to_string())?;
            }
            tx.commit().map_err(|e| e.to_string())?;
        }
    }

    // Update the file node with face group references
    for gid in &face_group_ids {
        if !file_node.face_group_ids.contains(gid) {
            file_node.face_group_ids.push(gid.clone());
        }
    }
    file_node.modified_at = now;

    let file_serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut ft = tx.open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;
        ft.insert(&file_id, file_serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(FaceDetectionResult {
        file_id,
        faces_detected: detected_faces.len(),
        face_group_ids,
    })
}

/// List all face groups from the database.
#[tauri::command]
pub fn list_face_groups(state: State<'_, AppState>) -> Result<Vec<FaceGroup>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx.open_table(crate::db::Database::get_face_groups_table())
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let group: FaceGroup = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;
        results.push(group);
    }

    Ok(results)
}

/// Get all files that belong to a specific face group.
#[tauri::command]
pub fn get_group_files(group_id: String, state: State<'_, AppState>) -> Result<Vec<FileNode>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;

    // Read the face group to get file IDs
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let fg_table = tx_read.open_table(crate::db::Database::get_face_groups_table())
        .map_err(|e| e.to_string())?;
    let fg_value = fg_table.get(&group_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Face group not found: {}", group_id))?;
    let group: FaceGroup = serde_json::from_str(&fg_value.value())
        .map_err(|e| e.to_string())?;

    // Fetch each file node
    let file_table = tx_read.open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    for fid in &group.file_ids {
        if let Some(fv) = file_table.get(fid).map_err(|e| e.to_string())? {
            let file_node: FileNode = serde_json::from_str(&fv.value())
                .map_err(|e| e.to_string())?;
            files.push(file_node);
        }
    }

    Ok(files)
}
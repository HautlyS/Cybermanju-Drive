use tauri::State;
use serde::{Deserialize, Serialize};
use chrono::Utc;

use crate::AppState;
use crate::db::schema::{FileNode, FaceGroup};
use crate::faces::{
    ClusteringStrategy, Cluster, embedding_distance, find_medoid,
    update_medoid_incremental, to_binary_hash, detect_faces_batch,
    recluster_all, DEFAULT_MATCH_THRESHOLD,
};

/// Result of face detection for a single file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceDetectionResult {
    pub file_id: String,
    pub faces_detected: usize,
    pub face_group_ids: Vec<String>,
    pub strategy_used: String,
}

/// Cluster info returned by recluster command.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReclusterResult {
    pub clusters_created: usize,
    pub total_faces: usize,
    pub noise_faces: usize,
    pub avg_cohesion: f32,
    pub strategy_used: String,
}

/// Detect faces on a single file and assign to groups.
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

    // Delegate to faces module for detection + embedding
    let detected_faces = crate::faces::detect_faces_in_file(&file_node)
        .map_err(|e| e.to_string())?;

    let now = Utc::now().to_rfc3339();
    let mut face_group_ids = Vec::new();

    // Process each detected face
    for (_face_index, embedding) in detected_faces.iter().enumerate() {
        // Try to match against existing face groups
        let tx_read = db.begin_read().map_err(|e| e.to_string())?;
        let fg_table = tx_read.open_table(crate::db::Database::get_face_groups_table())
            .map_err(|e| e.to_string())?;
        let mut matched_group_id: Option<String> = None;

        for entry in fg_table.iter().map_err(|e| e.to_string())? {
            let (key, value) = entry.map_err(|e| e.to_string())?;
            let group: FaceGroup = serde_json::from_str(&value.value())
                .map_err(|e| e.to_string())?;

            if let Some(ref centroid) = group.centroid_embedding {
                let dist = embedding_distance(embedding, centroid);
                // Adaptive: use tighter threshold for larger groups
                let group_threshold = DEFAULT_MATCH_THRESHOLD
                    - (group.file_ids.len() as f32 * 0.005).min(0.1);
                if dist < group_threshold {
                    matched_group_id = Some(key.value().to_string());
                    break;
                }
            }
        }
        drop(tx_read);

        if let Some(group_id) = matched_group_id {
            // Add file to existing group and update medoid
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

            // Incremental medoid update
            if let Some(ref current_medoid) = group.centroid_embedding {
                group.centroid_embedding = Some(update_medoid_incremental(
                    current_medoid,
                    &[], // We don't store all embeddings in the group, use centroid
                    embedding,
                ));
            } else {
                group.centroid_embedding = Some(embedding.clone());
            }

            // Update binary hash from new centroid
            if let Some(ref centroid) = group.centroid_embedding {
                group.binary_hash = Some(to_binary_hash(centroid));
            }
            group.embedding_count += 1;

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
            // Create a new face group
            let new_group_id = uuid::Uuid::new_v4().to_string();
            let group_name = format!("Person {}", new_group_id[..8].to_uppercase());
            let binary_hash = to_binary_hash(embedding);

            let new_group = FaceGroup {
                id: new_group_id.clone(),
                name: group_name,
                file_ids: vec![file_id.clone()],
                centroid_embedding: Some(embedding.clone()),
                binary_hash: Some(binary_hash),
                cohesion: Some(0.0),
                embedding_count: 1,
                algorithm: Some("cosine_threshold".to_string()),
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
        strategy_used: "cosine_threshold".to_string(),
    })
}

/// Batch detect faces across all image files.
#[tauri::command]
pub fn detect_faces_batch_cmd(state: State<'_, AppState>) -> Result<ReclusterResult, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // Collect all image file nodes
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let file_table = tx_read.open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;

    let mut image_files: Vec<FileNode> = Vec::new();
    for entry in file_table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let node: FileNode = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;
        if node.file_type == "file" {
            if let Some(ref mime) = node.mime_type {
                if mime.starts_with("image/") {
                    image_files.push(node);
                }
            }
        }
    }
    drop(tx_read);

    // Detect faces in parallel
    let results = detect_faces_batch(&image_files);

    // Collect all embeddings with file IDs
    let mut all_embeddings: Vec<(String, Vec<f32>)> = Vec::new();
    for (file_id, embeddings) in &results {
        for emb in embeddings {
            all_embeddings.push((file_id.clone(), emb.clone()));
        }
    }

    // Re-cluster with adaptive threshold
    let clusters = recluster_all(&all_embeddings, None);

    // Clear existing face groups
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut fg_table = tx.open_table(crate::db::Database::get_face_groups_table())
            .map_err(|e| e.to_string())?;
        // Remove all entries
        let keys: Vec<String> = fg_table.iter()
            .map_err(|e| e.to_string())?
            .filter_map(|entry| entry.ok().map(|(k, _)| k.value().to_string()))
            .collect();
        for key in &keys {
            fg_table.remove(key.as_str()).map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    // Write new clusters as face groups
    let now = Utc::now().to_rfc3339();
    let total_faces = all_embeddings.len();
    let noise_faces = total_faces - clusters.iter().map(|c| c.members.len()).sum::<usize>();

    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut fg_table = tx.open_table(crate::db::Database::get_face_groups_table())
            .map_err(|e| e.to_string())?;
        let mut ft_table = tx.open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;

        for (idx, cluster) in clusters.iter().enumerate() {
            let group_id = uuid::Uuid::new_v4().to_string();
            let group_name = format!("Person {}", idx + 1);

            let face_group = FaceGroup {
                id: group_id.clone(),
                name: group_name,
                file_ids: cluster.members.clone(),
                centroid_embedding: cluster.medoid.clone(),
                binary_hash: cluster.medoid.as_ref().map(|m| to_binary_hash(m)),
                cohesion: Some(cluster.cohesion),
                embedding_count: cluster.members.len() as u32,
                algorithm: Some("hdbscan_adaptive".to_string()),
                created_at: now.clone(),
            };

            let serialized = serde_json::to_string(&face_group).map_err(|e| e.to_string())?;
            fg_table.insert(&group_id, serialized.as_str()).map_err(|e| e.to_string())?;

            // Update file nodes to reference this face group
            for file_id in &cluster.members {
                if let Some(fv) = ft_table.get(file_id.as_str()).map_err(|e| e.to_string())? {
                    let mut file_node: FileNode = serde_json::from_str(fv.value())
                        .map_err(|e| e.to_string())?;
                    if !file_node.face_group_ids.contains(&group_id) {
                        file_node.face_group_ids.push(group_id.clone());
                    }
                    let file_serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
                    ft_table.insert(file_id.as_str(), file_serialized.as_str()).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    let avg_cohesion = if clusters.is_empty() {
        0.0
    } else {
        clusters.iter().map(|c| c.cohesion).sum::<f32>() / clusters.len() as f32
    };

    Ok(ReclusterResult {
        clusters_created: clusters.len(),
        total_faces,
        noise_faces,
        avg_cohesion,
        strategy_used: "hdbscan_adaptive".to_string(),
    })
}

/// Re-cluster all faces using a specific strategy.
#[tauri::command]
pub fn recluster_faces(
    strategy: Option<String>,
    state: State<'_, AppState>,
) -> Result<ReclusterResult, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // Collect all existing face group embeddings
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let fg_table = tx_read.open_table(crate::db::Database::get_face_groups_table())
        .map_err(|e| e.to_string())?;

    let mut all_embeddings: Vec<(String, Vec<f32>)> = Vec::new();
    let mut all_file_ids: Vec<String> = Vec::new();

    for entry in fg_table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let group: FaceGroup = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;
        if let Some(ref centroid) = group.centroid_embedding {
            for file_id in &group.file_ids {
                all_embeddings.push((file_id.clone(), centroid.clone()));
                all_file_ids.push(file_id.clone());
            }
        }
    }
    drop(tx_read);

    // Parse strategy
    let strat = match strategy.as_deref() {
        Some("bruteforce") => Some(ClusteringStrategy::BruteForce),
        Some("simhash") => Some(ClusteringStrategy::SimHash),
        Some("chinese_whispers") => Some(ClusteringStrategy::ChineseWhispers),
        Some("hdbscan") => Some(ClusteringStrategy::HDBSCAN),
        _ => None,
    };

    let clusters = recluster_all(&all_embeddings, strat);

    // Clear and rewrite
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut fg_table = tx.open_table(crate::db::Database::get_face_groups_table())
            .map_err(|e| e.to_string())?;
        let keys: Vec<String> = fg_table.iter()
            .map_err(|e| e.to_string())?
            .filter_map(|entry| entry.ok().map(|(k, _)| k.value().to_string()))
            .collect();
        for key in &keys {
            fg_table.remove(key.as_str()).map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    let now = Utc::now().to_rfc3339();
    let total_faces = all_embeddings.len();
    let noise_faces = total_faces - clusters.iter().map(|c| c.members.len()).sum::<usize>();

    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut fg_table = tx.open_table(crate::db::Database::get_face_groups_table())
            .map_err(|e| e.to_string())?;

        for (idx, cluster) in clusters.iter().enumerate() {
            let group_id = uuid::Uuid::new_v4().to_string();
            let group_name = format!("Person {}", idx + 1);

            let face_group = FaceGroup {
                id: group_id.clone(),
                name: group_name,
                file_ids: cluster.members.clone(),
                centroid_embedding: cluster.medoid.clone(),
                binary_hash: cluster.medoid.as_ref().map(|m| to_binary_hash(m)),
                cohesion: Some(cluster.cohesion),
                embedding_count: cluster.members.len() as u32,
                algorithm: Some(format!("{:?}", strat.unwrap_or(ClusteringStrategy::HDBSCAN))),
                created_at: now.clone(),
            };

            let serialized = serde_json::to_string(&face_group).map_err(|e| e.to_string())?;
            fg_table.insert(&group_id, serialized.as_str()).map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    let avg_cohesion = if clusters.is_empty() {
        0.0
    } else {
        clusters.iter().map(|c| c.cohesion).sum::<f32>() / clusters.len() as f32
    };

    Ok(ReclusterResult {
        clusters_created: clusters.len(),
        total_faces,
        noise_faces,
        avg_cohesion,
        strategy_used: format!("{:?}", strat.unwrap_or(ClusteringStrategy::HDBSCAN)),
    })
}

/// Rename a face group.
#[tauri::command]
pub fn rename_face_group(
    group_id: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx.open_table(crate::db::Database::get_face_groups_table())
        .map_err(|e| e.to_string())?;
    let value = table.get(&group_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Face group not found: {}", group_id))?;
    let mut group: FaceGroup = serde_json::from_str(&value.value())
        .map_err(|e| e.to_string())?;
    drop(tx);

    group.name = new_name;

    let serialized = serde_json::to_string(&group).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut wt = tx.open_table(crate::db::Database::get_face_groups_table())
            .map_err(|e| e.to_string())?;
        wt.insert(&group_id, serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Merge two face groups into one.
#[tauri::command]
pub fn merge_face_groups(
    source_group_id: String,
    target_group_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    // Read source group
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx.open_table(crate::db::Database::get_face_groups_table())
        .map_err(|e| e.to_string())?;
    let source_val = table.get(&source_group_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Source group not found: {}", source_group_id))?;
    let source: FaceGroup = serde_json::from_str(&source_val.value())
        .map_err(|e| e.to_string())?;

    let target_val = table.get(&target_group_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Target group not found: {}", target_group_id))?;
    let mut target: FaceGroup = serde_json::from_str(&target_val.value())
        .map_err(|e| e.to_string())?;
    drop(tx);

    // Merge: add source file IDs to target (dedup)
    for fid in &source.file_ids {
        if !target.file_ids.contains(fid) {
            target.file_ids.push(fid.clone());
        }
    }
    target.embedding_count = target.file_ids.len() as u32;

    // Update medoid from all members (approximate: use existing centroid + source centroid)
    if let Some(ref src_centroid) = source.centroid_embedding {
        if let Some(ref tgt_centroid) = target.centroid_embedding {
            // Average the two centroids as approximation
            let merged: Vec<f32> = tgt_centroid
                .iter()
                .zip(src_centroid.iter())
                .map(|(a, b)| (a + b) / 2.0)
                .collect();
            // L2-normalize
            let norm: f32 = merged.iter().map(|v| v * v).sum::<f32>().sqrt();
            if norm > 0.0 {
                target.centroid_embedding = Some(merged.iter().map(|v| v / norm).collect());
            }
        } else {
            target.centroid_embedding = Some(src_centroid.clone());
        }
    }

    // Update binary hash
    if let Some(ref centroid) = target.centroid_embedding {
        target.binary_hash = Some(to_binary_hash(centroid));
    }

    // Write merged target
    let serialized = serde_json::to_string(&target).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut wt = tx.open_table(crate::db::Database::get_face_groups_table())
            .map_err(|e| e.to_string())?;
        wt.insert(&target_group_id, serialized.as_str())
            .map_err(|e| e.to_string())?;
        // Remove source group
        wt.remove(&source_group_id).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete a face group.
#[tauri::command]
pub fn delete_face_group(
    group_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut wt = tx.open_table(crate::db::Database::get_face_groups_table())
            .map_err(|e| e.to_string())?;
        wt.remove(&group_id).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Find similar faces to a given face group.
#[tauri::command]
pub fn find_similar_faces(
    group_id: String,
    threshold: Option<f32>,
    state: State<'_, AppState>,
) -> Result<Vec<FaceGroup>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;

    let fg_table = tx.open_table(crate::db::Database::get_face_groups_table())
        .map_err(|e| e.to_string())?;
    let source_val = fg_table.get(&group_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Face group not found: {}", group_id))?;
    let source: FaceGroup = serde_json::from_str(&source_val.value())
        .map_err(|e| e.to_string())?;

    let thresh = threshold.unwrap_or(DEFAULT_MATCH_THRESHOLD);

    let mut similar = Vec::new();
    for entry in fg_table.iter().map_err(|e| e.to_string())? {
        let (key, value) = entry.map_err(|e| e.to_string())?;
        if key.value() == group_id {
            continue;
        }
        let group: FaceGroup = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;

        if let (Some(ref src_emb), Some(ref grp_emb)) = (&source.centroid_embedding, &group.centroid_embedding) {
            let dist = embedding_distance(src_emb, grp_emb);
            if dist < thresh {
                similar.push(group);
            }
        }
    }

    Ok(similar)
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

    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let fg_table = tx_read.open_table(crate::db::Database::get_face_groups_table())
        .map_err(|e| e.to_string())?;
    let fg_value = fg_table.get(&group_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Face group not found: {}", group_id))?;
    let group: FaceGroup = serde_json::from_str(&fg_value.value())
        .map_err(|e| e.to_string())?;

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

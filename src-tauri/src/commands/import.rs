use tauri::State;
use serde::{Deserialize, Serialize};
use chrono::Utc;

use crate::AppState;
use crate::db::schema::FileNode;
use crate::search::SearchIndex;

/// Result of a directory scan / import operation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub files_imported: u32,
    pub folders_imported: u32,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

/// File upload result — stores the actual file on disk and creates a DB entry.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResult {
    pub file_node: FileNode,
    pub bytes_written: u64,
}

/// Import a single file from the real filesystem into the database.
/// Reads the file's actual metadata (size, mime type), hashes it with BLAKE3,
/// extracts EXIF GPS if it's an image, and stores the FileNode.
#[tauri::command]
pub fn import_file(
    file_path: String,
    parent_path: String,
    state: State<'_, AppState>,
) -> Result<FileNode, String> {
    let start = std::time::Instant::now();

    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;

    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let is_dir = metadata.is_dir();
    let size_bytes = metadata.len();

    // Detect MIME type from file extension
    let mime_type = if is_dir {
        None
    } else {
        infer::get_from_path(path)
            .map(|m| m.mime_type().to_string())
            .or_else(|| {
                mime_guess::from_path(path).first().map(|m| m.to_string())
            })
    };

    // Hash file contents with BLAKE3 (for files only, not directories)
    let hash_blake3 = if !is_dir && size_bytes < 100 * 1024 * 1024 {
        // Only hash files under 100MB
        match std::fs::read(path) {
            Ok(data) => Some(blake3::hash(&data).to_hex().to_string()),
            Err(_) => None,
        }
    } else {
        None
    };

    let now = Utc::now().to_rfc3339();
    let file_id = uuid::Uuid::new_v4().to_string();

    // Build context_data with the real filesystem path
    let mut context = serde_json::Map::new();
    context.insert("original_path".to_string(), serde_json::json!(file_path));
    context.insert("source".to_string(), serde_json::json!("filesystem_import"));

    // Extract EXIF GPS for image files
    let (gps_lat, gps_lon) = if !is_dir {
        extract_gps_if_image(path)
    } else {
        (None, None)
    };

    if let (Some(lat), Some(lon)) = (gps_lat, gps_lon) {
        context.insert("gps_source".to_string(), serde_json::json!("exif"));
        context.insert("gps_lat".to_string(), serde_json::json!(lat));
        context.insert("gps_lon".to_string(), serde_json::json!(lon));
    }

    let file_node = FileNode {
        id: file_id.clone(),
        name: file_name,
        file_type: if is_dir { "folder" } else { "file" }.to_string(),
        parent_id: if parent_path.is_empty() { None } else { Some(parent_path) },
        size_bytes,
        mime_type,
        hash_blake3,
        encrypted: false,
        encryption_algorithm: None,
        compression_layers: Vec::new(),
        thumbnail_path: None,
        context_data: Some(serde_json::Value::Object(context)),
        tags: Vec::new(),
        collection_ids: Vec::new(),
        face_group_ids: Vec::new(),
        loose_group_ids: Vec::new(),
        gps_lat,
        gps_lon,
        created_at: now.clone(),
        modified_at: now,
    };

    // Store in database
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx.open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;
        table.insert(&file_id, serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    // Index in Tantivy for searchability
    let tantivy_index = state.tantivy_index.lock().map_err(|e| e.to_string())?;
    let content_text = if !is_dir && size_bytes < 1024 * 1024 {
        // Read first 64KB for text file content indexing
        std::fs::read(path)
            .ok()
            .and_then(|data| String::from_utf8(data.iter().take(65536).copied().collect()).ok())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let _ = tantivy_index.add_document(
        &file_id,
        &file_node.name,
        &content_text,
        &file_node.tags,
        &file_node.file_type,
        file_node.encrypted,
        file_node.gps_lat.is_some(),
        &file_node.created_at,
        file_node.hash_blake3.as_deref(),
    );

    log::info!(
        "Imported {} ({}) in {}ms",
        file_path,
        size_bytes,
        start.elapsed().as_millis()
    );

    Ok(file_node)
}

/// Scan a directory and import all files/folders into the database.
/// This is the real equivalent of "browse and catalog" — reads actual filesystem
/// contents and creates FileNode entries with real metadata.
#[tauri::command]
pub fn scan_directory(
    dir_path: String,
    parent_id: String,
    recursive: bool,
    state: State<'_, AppState>,
) -> Result<ImportResult, String> {
    let start = std::time::Instant::now();
    let mut files_imported = 0u32;
    let mut folders_imported = 0u32;
    let mut errors = Vec::new();

    let entries = match std::fs::read_dir(&dir_path) {
        Ok(e) => e,
        Err(e) => return Err(format!("Failed to read directory {}: {}", dir_path, e)),
    };

    let parent_path_for_children = if parent_id.is_empty() { dir_path.clone() } else { parent_id.clone() };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                errors.push(format!("Failed to read entry: {}", e));
                continue;
            }
        };

        let path = entry.path();
        let file_path_str = path.to_string_lossy().to_string();

        match import_file(file_path_str, parent_path_for_children.clone(), state.clone()) {
            Ok(node) => {
                if node.file_type == "folder" {
                    folders_imported += 1;
                    // Recurse into subdirectories if requested
                    if recursive {
                        match scan_directory(path.to_string_lossy().to_string(), node.id.clone(), true, state.clone()) {
                            Ok(sub_result) => {
                                files_imported += sub_result.files_imported;
                                folders_imported += sub_result.folders_imported;
                                errors.extend(sub_result.errors);
                            }
                            Err(e) => errors.push(e),
                        }
                    }
                } else {
                    files_imported += 1;
                }
            }
            Err(e) => {
                errors.push(format!("Failed to import {}: {}", path.display(), e));
            }
        }
    }

    Ok(ImportResult {
        files_imported,
        folders_imported,
        errors,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Upload file data (from the frontend) to disk and create a FileNode entry.
/// The frontend sends raw bytes; this writes them to a configurable storage path
/// and creates the database record with real metadata.
#[tauri::command]
pub fn upload_file(
    file_name: String,
    data: Vec<u8>,
    parent_path: String,
    state: State<'_, AppState>,
) -> Result<UploadResult, String> {
    let now = Utc::now().to_rfc3339();
    let file_id = uuid::Uuid::new_v4().to_string();

    // Store files in a "storage" directory next to the database
    let storage_dir = std::path::Path::new("cybermanju_storage");
    std::fs::create_dir_all(storage_dir)
        .map_err(|e| format!("Failed to create storage directory: {}", e))?;

    // Use UUID-based filename to avoid collisions
    let extension = std::path::Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let stored_name = if extension.is_empty() {
        format!("{}.{}", file_id, file_name)
    } else {
        format!("{}.{}", file_id, file_name)
    };
    let stored_path = storage_dir.join(&stored_name);

    // Write the file to disk
    std::fs::write(&stored_path, &data)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    let bytes_written = data.len() as u64;

    // Compute BLAKE3 hash
    let hash_blake3 = Some(blake3::hash(&data).to_hex().to_string());

    // Detect MIME type
    let mime_type = infer::get_from_bytes(&data)
        .map(|m| m.mime_type().to_string())
        .or_else(|| mime_guess::from_path(&file_name).first().map(|m| m.to_string()));

    // Build context_data
    let mut context = serde_json::Map::new();
    context.insert("original_path".to_string(), serde_json::json!(stored_path.to_string_lossy()));
    context.insert("source".to_string(), serde_json::json!("upload"));
    context.insert("original_filename".to_string(), serde_json::json!(file_name));

    let file_node = FileNode {
        id: file_id.clone(),
        name: file_name,
        file_type: "file".to_string(),
        parent_id: if parent_path.is_empty() { None } else { Some(parent_path) },
        size_bytes: bytes_written,
        mime_type,
        hash_blake3,
        encrypted: false,
        encryption_algorithm: None,
        compression_layers: Vec::new(),
        thumbnail_path: None,
        context_data: Some(serde_json::Value::Object(context)),
        tags: Vec::new(),
        collection_ids: Vec::new(),
        face_group_ids: Vec::new(),
        loose_group_ids: Vec::new(),
        gps_lat: None,
        gps_lon: None,
        created_at: now.clone(),
        modified_at: now,
    };

    // Store in database
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx.open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;
        table.insert(&file_id, serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    // Index in Tantivy
    let tantivy_index = state.tantivy_index.lock().map_err(|e| e.to_string())?;
    let content_text = String::from_utf8(data.iter().take(65536).copied().collect()).unwrap_or_default();
    let _ = tantivy_index.add_document(
        &file_id,
        &file_node.name,
        &content_text,
        &file_node.tags,
        &file_node.file_type,
        false,
        false,
        &file_node.created_at,
        file_node.hash_blake3.as_deref(),
    );

    Ok(UploadResult { file_node, bytes_written })
}

/// Rebuild the Tantivy search index from all FileNodes in the database.
/// Useful after bulk imports or database migrations.
#[tauri::command]
pub fn rebuild_search_index(state: State<'_, AppState>) -> Result<u32, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx.open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;

    let tantivy_index = state.tantivy_index.lock().map_err(|e| e.to_string())?;
    let mut count = 0u32;

    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let node: FileNode = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;

        // Read file content for text files (up to 64KB)
        let content_text = if node.file_type == "file" {
            node.context_data
                .as_ref()
                .and_then(|ctx| ctx.get("original_path").and_then(|v| v.as_str()))
                .and_then(|path| std::fs::read(path).ok())
                .and_then(|data| String::from_utf8(data.iter().take(65536).copied().collect()).ok())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let _ = tantivy_index.add_document(
            &node.id,
            &node.name,
            &content_text,
            &node.tags,
            &node.file_type,
            node.encrypted,
            node.gps_lat.is_some(),
            &node.created_at,
            node.hash_blake3.as_deref(),
        );
        count += 1;
    }

    Ok(count)
}

/// Extract GPS from an image file using the exif crate.
/// Returns (lat, lon) or (None, None) if no GPS data found.
fn extract_gps_if_image(path: &std::path::Path) -> (Option<f64>, Option<f64>) {
    // Only try for image files
    let is_image = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_lowercase().as_str(), "jpg" | "jpeg" | "tiff" | "tif" | "heic" | "heif" | "png" | "webp"))
        .unwrap_or(false);

    if !is_image {
        return (None, None);
    }

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (None, None),
    };

    let mut bufreader = std::io::BufReader::new(&file);
    let exif_data = match exif::Reader::new().read_from_container(&mut bufreader) {
        Ok(d) => d,
        Err(_) => return (None, None),
    };

    let latitude = exif_data
        .get_field(exif::Tag::GpsLatitude, exif::In::PRIMARY)
        .and_then(|field| {
            if let exif::Value::Rational(ref rationals) = field.value {
                if rationals.len() >= 3 {
                    let deg = rationals[0].numerator as f64 / rationals[0].denominator as f64;
                    let min = rationals[1].numerator as f64 / rationals[1].denominator as f64;
                    let sec = rationals[2].numerator as f64 / rationals[2].denominator as f64;
                    Some(deg + min / 60.0 + sec / 3600.0)
                } else {
                    None
                }
            } else {
                None
            }
        });

    let longitude = exif_data
        .get_field(exif::Tag::GpsLongitude, exif::In::PRIMARY)
        .and_then(|field| {
            if let exif::Value::Rational(ref rationals) = field.value {
                if rationals.len() >= 3 {
                    let deg = rationals[0].numerator as f64 / rationals[0].denominator as f64;
                    let min = rationals[1].numerator as f64 / rationals[1].denominator as f64;
                    let sec = rationals[2].numerator as f64 / rationals[2].denominator as f64;
                    Some(deg + min / 60.0 + sec / 3600.0)
                } else {
                    None
                }
            } else {
                None
            }
        });

    let lat_ref = exif_data
        .get_field(exif::Tag::GpsLatitudeRef, exif::In::PRIMARY)
        .and_then(|f| {
            if let exif::Value::Ascii(ref bytes) = f.value {
                std::str::from_utf8(bytes).ok().map(|s| s.to_string())
            } else {
                None
            }
        });

    let lon_ref = exif_data
        .get_field(exif::Tag::GpsLongitudeRef, exif::In::PRIMARY)
        .and_then(|f| {
            if let exif::Value::Ascii(ref bytes) = f.value {
                std::str::from_utf8(bytes).ok().map(|s| s.to_string())
            } else {
                None
            }
        });

    let final_lat = latitude.map(|lat| if lat_ref.as_deref() == Some("S") { -lat } else { lat });
    let final_lon = longitude.map(|lon| if lon_ref.as_deref() == Some("W") { -lon } else { lon });

    (final_lat, final_lon)
}
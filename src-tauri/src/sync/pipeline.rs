// Cybermanju Drive — Sync Pipeline
// Orchestrates the full sync flow: scan → compress → preview → upload → link → clean

use crate::compression::TripleCompressor;
use crate::db::schema::FileNode;
use crate::sync::backends::create_backend;
use crate::sync::models::*;
use crate::AppState;
use chrono::Utc;
use log::{error, info, warn};
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

// ===========================================================================
// SyncPipeline
// ===========================================================================

/// Per-file sync result returned by the parallel inner worker.
struct FileSyncResult {
    file_id: String,
    bytes_uploaded: u64,
    bytes_saved: u64,
    error: Option<String>,
}

/// The main sync orchestrator. Holds configuration and progress state.
pub struct SyncPipeline {
    config: SyncConfig,
    _db_path: String,
    progress: Arc<SyncProgressInner>,
    cancelled: Arc<AtomicBool>,
}

/// Internal progress state wrapped in `Arc` for atomic updates.
struct SyncProgressInner {
    total_files: AtomicU32,
    processed_files: AtomicU32,
    current_file: Mutex<Option<String>>,
    status: Mutex<SyncStatus>,
    bytes_uploaded: AtomicU64,
    errors: Mutex<Vec<String>>,
    started_at: Mutex<Option<String>>,
}

impl SyncProgressInner {
    fn new() -> Self {
        Self {
            total_files: AtomicU32::new(0),
            processed_files: AtomicU32::new(0),
            current_file: Mutex::new(None),
            status: Mutex::new(SyncStatus::Idle),
            bytes_uploaded: AtomicU64::new(0),
            errors: Mutex::new(Vec::new()),
            started_at: Mutex::new(None),
        }
    }
}

impl SyncPipeline {
    /// Create a new pipeline for the given config.
    pub fn new(config: SyncConfig, db_path: String) -> Self {
        Self {
            config,
            _db_path: db_path,
            progress: Arc::new(SyncProgressInner::new()),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Cancel an in-progress sync.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if the sync has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Get a snapshot of the current progress.
    pub fn get_progress(&self) -> SyncProgress {
        let processed = self.progress.processed_files.load(Ordering::SeqCst);
        let total = self.progress.total_files.load(Ordering::SeqCst);
        let started = self
            .progress
            .started_at
            .lock()
            .map_err(|e| e.to_string())
            .ok()
            .and_then(|g| g.clone());
        let elapsed = started
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_seconds() as f64)
            .unwrap_or(0.0);

        let estimated_remaining = if processed > 0 && total > 0 {
            let avg_per_file = elapsed / processed as f64;
            Some(avg_per_file * (total - processed) as f64)
        } else {
            None
        };

        SyncProgress {
            total_files: total,
            processed_files: processed,
            current_file: self
                .progress
                .current_file
                .lock()
                .map_err(|e| e.to_string())
                .ok()
                .and_then(|g| g.clone()),
            status: self
                .progress
                .status
                .lock()
                .map_err(|e| e.to_string())
                .ok()
                .map(|g| g.clone())
                .unwrap_or(SyncStatus::Idle),
            bytes_uploaded: self.progress.bytes_uploaded.load(Ordering::SeqCst),
            errors: self
                .progress
                .errors
                .lock()
                .map_err(|e| e.to_string())
                .ok()
                .map(|g| g.clone())
                .unwrap_or_default(),
            started_at: started,
            estimated_remaining_seconds: estimated_remaining,
        }
    }

    // -----------------------------------------------------------------------
    // Main entry points
    // -----------------------------------------------------------------------

    /// Sync all the given file IDs to the configured backend.
    /// Uses rayon parallel iterators when `max_concurrent_uploads > 1`,
    /// otherwise falls back to sequential processing.
    pub fn sync_all(&self, file_ids: Vec<String>, state: &AppState) -> Result<SyncResult, String> {
        self.reset_progress(file_ids.len() as u32)?;
        *self.progress.started_at.lock().map_err(|e| e.to_string())? =
            Some(Utc::now().to_rfc3339());

        let backend = create_backend(&self.config)?;

        if self.config.max_concurrent_uploads > 1 {
            self.sync_all_parallel(&file_ids, &backend, state)
        } else {
            self.sync_all_sequential(&file_ids, &backend, state)
        }
    }

    /// Sequential sync — one file at a time (fallback).
    fn sync_all_sequential(
        &self,
        file_ids: &[String],
        backend: &dyn StorageBackend,
        state: &AppState,
    ) -> Result<SyncResult, String> {
        let compressor = &state.compression;
        let mut total_bytes_uploaded: u64 = 0;
        let mut bytes_saved: u64 = 0;
        let mut files_synced: u32 = 0;
        let start = std::time::Instant::now();

        for file_id in file_ids {
            if self.is_cancelled() {
                warn!("Sync cancelled by user");
                break;
            }

            match self.sync_single_file_inner(file_id, backend, compressor, state) {
                Ok((uploaded, saved)) => {
                    total_bytes_uploaded += uploaded;
                    bytes_saved += saved;
                    files_synced += 1;
                }
                Err(e) => {
                    error!("Failed to sync file {}: {}", file_id, e);
                    if let Ok(mut errors) = self.progress.errors.lock().map_err(|e| e.to_string()) {
                        errors.push(e);
                    }
                }
            }

            self.progress.processed_files.fetch_add(1, Ordering::SeqCst);
        }

        let final_status = if self.is_cancelled() {
            SyncStatus::Error
        } else {
            SyncStatus::Done
        };
        if let Ok(mut status) = self.progress.status.lock().map_err(|e| e.to_string()) {
            *status = final_status;
        }

        Ok(SyncResult {
            files_synced,
            bytes_uploaded: total_bytes_uploaded,
            bytes_saved_by_compression: bytes_saved,
            errors: self
                .progress
                .errors
                .lock()
                .map_err(|e| e.to_string())
                .ok()
                .map(|g| g.clone())
                .unwrap_or_default(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Parallel sync using rayon — each file is independent.
    fn sync_all_parallel(
        &self,
        file_ids: &[String],
        backend: &dyn StorageBackend,
        state: &AppState,
    ) -> Result<SyncResult, String> {
        let start = std::time::Instant::now();

        // Configure the rayon thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.config.max_concurrent_uploads as usize)
            .build()
            .map_err(|e| format!("Failed to create thread pool: {}", e))?;

        let results: Vec<FileSyncResult> = pool.install(|| {
            file_ids
                .par_iter()
                .filter(|_| !self.is_cancelled())
                .map(|file_id| {
                    let compressor = &state.compression;
                    match self.sync_single_file_inner(file_id, backend, compressor, state) {
                        Ok((uploaded, saved)) => {
                            self.progress.processed_files.fetch_add(1, Ordering::SeqCst);
                            self.progress
                                .bytes_uploaded
                                .fetch_add(uploaded, Ordering::SeqCst);
                            FileSyncResult {
                                file_id: file_id.clone(),
                                bytes_uploaded: uploaded,
                                bytes_saved: saved,
                                error: None,
                            }
                        }
                        Err(e) => {
                            error!("Failed to sync file {}: {}", file_id, e);
                            self.progress.processed_files.fetch_add(1, Ordering::SeqCst);
                            FileSyncResult {
                                file_id: file_id.clone(),
                                bytes_uploaded: 0,
                                bytes_saved: 0,
                                error: Some(e),
                            }
                        }
                    }
                })
                .collect()
        });

        // Aggregate results
        let mut total_bytes_uploaded: u64 = 0;
        let mut bytes_saved: u64 = 0;
        let mut files_synced: u32 = 0;

        for r in &results {
            if let Some(ref err) = r.error {
                if let Ok(mut errors) = self.progress.errors.lock() {
                    errors.push(format!("{}: {}", r.file_id, err));
                }
            } else {
                total_bytes_uploaded += r.bytes_uploaded;
                bytes_saved += r.bytes_saved;
                files_synced += 1;
            }
        }

        let final_status = if self.is_cancelled() {
            SyncStatus::Error
        } else {
            SyncStatus::Done
        };
        if let Ok(mut status) = self.progress.status.lock() {
            *status = final_status;
        }

        Ok(SyncResult {
            files_synced,
            bytes_uploaded: total_bytes_uploaded,
            bytes_saved_by_compression: bytes_saved,
            errors: self
                .progress
                .errors
                .lock()
                .map(|g| g.clone())
                .unwrap_or_default(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Sync a single file by ID.
    pub fn sync_single_file(
        &self,
        file_id: String,
        state: &AppState,
    ) -> Result<SyncResult, String> {
        self.reset_progress(1)?;
        *self.progress.started_at.lock().map_err(|e| e.to_string())? =
            Some(Utc::now().to_rfc3339());

        let backend = create_backend(&self.config)?;
        let compressor = &state.compression;
        let start = std::time::Instant::now();

        let (bytes_uploaded, bytes_saved) =
            self.sync_single_file_inner(&file_id, &backend, compressor, state)?;

        self.progress.processed_files.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut status) = self.progress.status.lock().map_err(|e| e.to_string()) {
            *status = SyncStatus::Done;
        }

        Ok(SyncResult {
            files_synced: 1,
            bytes_uploaded,
            bytes_saved_by_compression: bytes_saved,
            errors: self
                .progress
                .errors
                .lock()
                .map(|g| g.clone())
                .unwrap_or_default(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    // -----------------------------------------------------------------------
    // Internal: sync a single file
    // -----------------------------------------------------------------------

    /// Returns (bytes_uploaded, bytes_saved_by_compression).
    fn sync_single_file_inner(
        &self,
        file_id: &str,
        backend: &dyn StorageBackend,
        compressor: &TripleCompressor,
        state: &AppState,
    ) -> Result<(u64, u64), String> {
        // 1. Read file node from DB
        let file_node = self.get_file_node(file_id, state)?;

        // Get the actual file path from context_data
        let original_path = file_node
            .context_data
            .as_ref()
            .and_then(|ctx| ctx.get("original_path").and_then(|v| v.as_str()))
            .unwrap_or(&file_node.name)
            .to_string();

        if !Path::new(&original_path).exists() {
            return Err(format!("File not found on disk: {}", original_path));
        }

        if let Ok(mut current) = self.progress.current_file.lock() {
            *current = Some(original_path.clone());
        }

        // 2. Compress (if configured)
        let (upload_path, _original_size, compressed_size, bytes_saved) =
            if self.config.compress_before_upload {
                if let Ok(mut status) = self.progress.status.lock() {
                    *status = SyncStatus::Compressing;
                }
                let (comp_path, orig_sz, comp_sz) =
                    self.compress_file(&original_path, compressor)?;
                (comp_path, orig_sz, comp_sz, orig_sz.saturating_sub(comp_sz))
            } else {
                let metadata = fs::metadata(&original_path)
                    .map_err(|e| format!("Failed to get file metadata: {}", e))?;
                (original_path.clone(), metadata.len(), metadata.len(), 0)
            };

        // 3. Create preview (if configured)
        if self.config.create_previews {
            if let Ok(mut status) = self.progress.status.lock() {
                *status = SyncStatus::Linking;
            }
            let _preview_path = self.create_preview(&original_path).ok();
        }

        // 4. Upload
        if let Ok(mut status) = self.progress.status.lock() {
            *status = SyncStatus::Uploading;
        }
        let remote_name = format!(
            "cybermanju_sync/{}",
            Path::new(&original_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| file_id.to_string())
        );
        let remote_url = backend.upload_file(&upload_path, &remote_name)?;

        self.progress
            .bytes_uploaded
            .fetch_add(compressed_size, Ordering::SeqCst);

        // 5. Create link in FileNode's context_data
        if let Ok(mut status) = self.progress.status.lock() {
            *status = SyncStatus::Linking;
        }
        self.create_link(file_id, &remote_url, state)?;

        // 6. Delete raw if configured
        if self.config.delete_raw_after_sync {
            if let Ok(mut status) = self.progress.status.lock() {
                *status = SyncStatus::Cleaning;
            }
            let _deleted = self.delete_raw_uncompressed(&original_path, &upload_path)?;
        }

        if let Ok(mut current) = self.progress.current_file.lock() {
            *current = None;
        }

        Ok((compressed_size, bytes_saved))
    }

    // -----------------------------------------------------------------------
    // Compression
    // -----------------------------------------------------------------------

    /// Compress a file using the triple compressor.
    /// Returns (compressed_path, original_size, compressed_size).
    pub fn compress_file(
        &self,
        file_path: &str,
        compressor: &TripleCompressor,
    ) -> Result<(String, u64, u64), String> {
        let data = fs::read(file_path)
            .map_err(|e| format!("Failed to read file for compression: {}", e))?;
        let original_size = data.len() as u64;

        let (compressed, _stats) = compressor
            .compress_triple(&data)
            .map_err(|e| format!("Triple compression failed: {}", e))?;
        let compressed_size = compressed.len() as u64;

        // Write compressed file next to the original with .cyb3 extension
        let compressed_path = format!("{}.cyb3", file_path);
        fs::write(&compressed_path, &compressed)
            .map_err(|e| format!("Failed to write compressed file: {}", e))?;

        info!(
            "Compressed {} → {} ({} → {} bytes)",
            file_path, compressed_path, original_size, compressed_size,
        );

        Ok((compressed_path, original_size, compressed_size))
    }

    // -----------------------------------------------------------------------
    // Preview generation
    // -----------------------------------------------------------------------

    /// Generate a thumbnail preview for an image file.
    /// Returns the path to the generated preview.
    pub fn create_preview(&self, file_path: &str) -> Result<String, String> {
        let data =
            fs::read(file_path).map_err(|e| format!("Failed to read file for preview: {}", e))?;

        let img = image::load_from_memory(&data)
            .map_err(|e| format!("Failed to decode image for preview: {}", e))?;

        let (w, h) = (img.width(), img.height());
        let max_size: u32 = 512;
        let scale = if w > h {
            max_size as f64 / w as f64
        } else {
            max_size as f64 / h as f64
        };
        let new_w = (w as f64 * scale) as u32;
        let new_h = (h as f64 * scale) as u32;

        let thumbnail = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);

        let preview_path = format!("{}.preview.png", file_path);
        let mut out_file = fs::File::create(&preview_path)
            .map_err(|e| format!("Failed to create preview file: {}", e))?;
        thumbnail
            .write_to(&mut out_file, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to write preview: {}", e))?;

        info!("Created preview: {}", preview_path);
        Ok(preview_path)
    }

    // -----------------------------------------------------------------------
    // Link creation
    // -----------------------------------------------------------------------

    /// Update a FileNode's context_data with the remote URL.
    pub fn create_link(
        &self,
        file_id: &str,
        remote_url: &str,
        state: &AppState,
    ) -> Result<(), String> {
        let db = state.db.write().map_err(|e| e.to_string())?;

        // Read current file node
        let tx_read = db.begin_read().map_err(|e| e.to_string())?;
        let table_read = tx_read
            .open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;
        let value = table_read
            .get(file_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("File not found: {}", file_id))?;
        let mut file_node: FileNode =
            serde_json::from_str(value.value()).map_err(|e| e.to_string())?;
        drop(tx_read);

        // Update context_data with sync link
        let mut context = file_node
            .context_data
            .clone()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        if let Some(obj) = context.as_object_mut() {
            obj.insert("sync_url".to_string(), serde_json::json!(remote_url));
            obj.insert(
                "sync_backend".to_string(),
                serde_json::json!(self.config.backend_type.clone()),
            );
            obj.insert(
                "synced_at".to_string(),
                serde_json::json!(Utc::now().to_rfc3339()),
            );
        }
        file_node.context_data = Some(context);
        file_node.modified_at = Utc::now().to_rfc3339();

        // Write back
        let serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
        let tx = db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = tx
                .open_table(crate::db::Database::get_files_table())
                .map_err(|e| e.to_string())?;
            table
                .insert(file_id, serialized.as_str())
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;

        info!("Created sync link for file {}: {}", file_id, remote_url);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Cleanup
    // -----------------------------------------------------------------------

    /// Delete the original file if a compressed version exists and config allows it.
    /// Returns true if the original was deleted.
    pub fn delete_raw_uncompressed(
        &self,
        file_path: &str,
        compressed_path: &str,
    ) -> Result<bool, String> {
        let original = Path::new(file_path);
        let compressed = Path::new(compressed_path);

        // Only delete if the compressed version exists
        if !compressed.exists() {
            return Ok(false);
        }

        // Don't delete if the "compressed" file is the same as the original
        // (i.e. compression was not enabled)
        if original == compressed {
            return Ok(false);
        }

        if original.exists() {
            fs::remove_file(original)
                .map_err(|e| format!("Failed to delete original file: {}", e))?;
            info!("Deleted original file: {}", file_path);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn reset_progress(&self, total: u32) -> Result<(), String> {
        self.progress.total_files.store(total, Ordering::SeqCst);
        self.progress.processed_files.store(0, Ordering::SeqCst);
        if let Ok(mut current) = self.progress.current_file.lock() {
            *current = None;
        }
        if let Ok(mut status) = self.progress.status.lock() {
            *status = SyncStatus::Scanning;
        }
        self.progress.bytes_uploaded.store(0, Ordering::SeqCst);
        if let Ok(mut errors) = self.progress.errors.lock() {
            errors.clear();
        }
        self.cancelled.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn get_file_node(&self, file_id: &str, state: &AppState) -> Result<FileNode, String> {
        let db = state.db.read().map_err(|e| e.to_string())?;
        let tx = db.begin_read().map_err(|e| e.to_string())?;
        let table = tx
            .open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;
        let value = table
            .get(file_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("File not found in DB: {}", file_id))?;
        let node: FileNode = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;
        Ok(node)
    }
}

// Cybermanju Drive — Core Library
// Orchestrates redb, ML-KEM PQC (pqcrypto-mlkem), Tantivy, Tree-sitter, triple compression, face clustering

pub mod commands;
pub mod compression;
pub mod crypto;
pub mod db;
pub mod faces;       // ML module: detect_faces_in_file, embedding_distance, cluster_embeddings
pub mod preview;
pub mod search;
pub mod sync;
pub mod tree_sitter; // parse_file, get_symbols (tauri commands)
pub mod web_dashboard;

use commands::{
    accounts, collections, dashboard, encryption, files, import as import_cmd, map, search_cmd, users,
};
use commands::faces as face_cmd;
use commands::sync as sync_cmd;
use db::Database;
use std::sync::{Arc, RwLock};
use tauri::Manager;

pub struct AppState {
    pub db: RwLock<Database>,
    pub tantivy_index: RwLock<search::SearchIndex>,
    pub compression: compression::TripleCompressor,
    pub hmac_secret: [u8; 32],
}

// WebDashboard now handles its own shutdown (Drop impl with signal channel + thread join).

pub fn run() {
    // Initialize tracing subscriber (replaces env_logger)
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    tracing::info!("Cybermanju Drive starting...");

    // Initialize redb database
    let db = match Database::new("cybermanju.db") {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Failed to initialize redb database: {}", e);
            std::process::exit(1);
        }
    };
    tracing::info!("redb database initialized");

    // Initialize Tantivy full-text search index
    let tantivy_index = match search::SearchIndex::new("tantivy_index") {
        Ok(i) => i,
        Err(e) => {
            tracing::error!("Failed to initialize Tantivy: {}", e);
            std::process::exit(1);
        }
    };
    tracing::info!("Tantivy search index ready");

    // Initialize triple-layer compressor
    let compressor = compression::TripleCompressor::new();

    // Initialize HMAC secret for secure session tokens
    let mut hmac_secret = [0u8; 32];
    use rand_core::{RngCore, OsRng};
    OsRng.fill_bytes(&mut hmac_secret);

    let state = AppState {
        db: RwLock::new(db),
        tantivy_index: RwLock::new(tantivy_index),
        compression: compressor,
        hmac_secret,
    };

    // Dashboard state for connection tracking and lifecycle
    let dashboard_state = Arc::new(dashboard::DashboardState::new());

    // Sync state for progress tracking and cancellation
    let sync_state = Arc::new(sync_cmd::SyncState::new());

    // ─── Start Web Dashboard (localhost-only, JWT-authenticated) ────────
    let dashboard = std::sync::Arc::new(
        web_dashboard::WebDashboard::new(3456, "cybermanju.db")
    );
    match dashboard.start() {
        Ok(()) => tracing::info!("Web Dashboard started on port 3456 (localhost only, JWT auth)"),
        Err(e) => tracing::error!("Failed to start Web Dashboard: {}", e),
    }
    // dashboard.stop() is called explicitly below after Tauri exits,
    // ensuring the accept thread is joined from the MAIN thread (not from
    // the accept thread's own Drop, which would self-deadlock).

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .manage(state)
        .manage(dashboard_state)
        .manage(sync_state)
        .invoke_handler(tauri::generate_handler![
            // File operations
            files::list_files,
            files::get_file,
            files::create_folder,
            files::delete_file,
            files::rename_file,
            files::duplicate_file_context,
            files::move_file,
            files::get_preview,
            // Search
            search_cmd::search_files,
            search_cmd::suggest,
            // Encryption
            encryption::encrypt_file,
            encryption::decrypt_file,
            encryption::get_encryption_status,
            encryption::generate_keypair,
            encryption::list_keys,
            // Compression
            commands::compression::compress_file,
            commands::compression::decompress_file,
            commands::compression::get_compression_stats,
            // Collections
            collections::list_collections,
            collections::create_collection,
            collections::add_to_collection,
            collections::remove_from_collection,
            // Face grouping (commands layer — delegates to crate::faces for ML)
            face_cmd::detect_faces,
            face_cmd::detect_faces_batch_cmd,
            face_cmd::recluster_faces,
            face_cmd::rename_face_group,
            face_cmd::merge_face_groups,
            face_cmd::delete_face_group,
            face_cmd::find_similar_faces,
            face_cmd::list_face_groups,
            face_cmd::get_group_files,
            // Map / GPS
            map::get_geo_files,
            map::extract_exif_gps,
            // Accounts
            accounts::list_accounts,
            accounts::create_account,
            accounts::switch_account,
            accounts::delete_account,
            // Tree-sitter code intelligence
            tree_sitter::parse_file,
            tree_sitter::get_symbols,
            // Loose groups
            files::create_loose_group,
            files::add_to_loose_group,
            files::list_loose_groups,
            // User management & permissions
            users::register_user,
            users::authenticate_user,
            users::list_users,
            users::set_file_permission,
            users::grant_file_permission,
            users::revoke_file_permission,
            users::verify_file_access,
            users::get_file_permissions,
            users::verify_token,
            // Dashboard
            dashboard::dashboard_status,
            dashboard::start_dashboard,
            dashboard::stop_dashboard,
            // Sync
            sync_cmd::list_sync_configs,
            sync_cmd::create_sync_config,
            sync_cmd::delete_sync_config,
            sync_cmd::start_sync,
            sync_cmd::get_sync_progress,
            sync_cmd::test_sync_connection,
            sync_cmd::cancel_sync,
            sync_cmd::list_remote_files,
            // File import / upload
            import_cmd::import_file,
            import_cmd::scan_directory,
            import_cmd::upload_file,
            import_cmd::rebuild_search_index,
        ])
        .run(tauri::generate_context!())
        .expect("Fatal error while running Cybermanju Drive — see logs above");

    // ─── Clean shutdown: stop the dashboard before dropping the Arc ──
    dashboard.stop();
    log::info!("Web Dashboard shut down cleanly");
}
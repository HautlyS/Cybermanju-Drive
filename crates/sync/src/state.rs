// Cybermanju Drive — Shared Sync State
// Live progress + cancellation flag shared between the sync pipeline and
// whichever front-end is driving it (Tauri IPC, REST or WASM).

use cybermanju_types::sync::{SyncProgress, SyncStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub struct SyncState {
    pub progress: Mutex<SyncProgress>,
    pub cancel_flag: AtomicBool,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            progress: Mutex::new(SyncProgress {
                total_files: 0,
                processed_files: 0,
                current_file: None,
                status: SyncStatus::Idle,
                bytes_uploaded: 0,
                errors: Vec::new(),
                started_at: None,
                estimated_remaining_seconds: None,
            }),
            cancel_flag: AtomicBool::new(false),
        }
    }

    /// Run `f` against the progress struct, recovering from a poisoned lock
    /// so a panicking worker can never wedge every later status read.
    fn with_progress<F: FnOnce(&mut SyncProgress)>(&self, f: F) {
        match self.progress.lock() {
            Ok(mut guard) => f(&mut guard),
            Err(poisoned) => f(&mut poisoned.into_inner()),
        }
    }

    /// Snapshot of the current progress, with an ETA derived from elapsed time.
    pub fn snapshot(&self) -> SyncProgress {
        let progress = match self.progress.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };

        let elapsed = progress
            .started_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_seconds() as f64)
            .unwrap_or(0.0);

        let estimated_remaining_seconds =
            if progress.processed_files > 0 && progress.total_files > 0 {
                let avg_per_file = elapsed / progress.processed_files as f64;
                Some(avg_per_file * (progress.total_files - progress.processed_files) as f64)
            } else {
                None
            };

        SyncProgress {
            estimated_remaining_seconds,
            ..progress
        }
    }

    /// Reset progress for a new run of `total` files.
    pub fn reset(&self, total: u32) {
        self.cancel_flag.store(false, Ordering::SeqCst);
        self.with_progress(|p| {
            p.total_files = total;
            p.processed_files = 0;
            p.current_file = None;
            p.status = SyncStatus::Scanning;
            p.bytes_uploaded = 0;
            p.errors.clear();
            p.started_at = None;
            p.estimated_remaining_seconds = None;
        });
    }

    pub fn set_total(&self, total: u32) {
        self.with_progress(|p| p.total_files = total);
    }

    pub fn set_status(&self, status: SyncStatus) {
        self.with_progress(|p| p.status = status);
    }

    pub fn set_current(&self, current: Option<String>) {
        self.with_progress(|p| p.current_file = current);
    }

    pub fn set_started_at(&self, started_at: Option<String>) {
        self.with_progress(|p| p.started_at = started_at);
    }

    pub fn set_processed(&self, processed: u32) {
        self.with_progress(|p| p.processed_files = processed);
    }

    pub fn inc_processed(&self) {
        self.with_progress(|p| p.processed_files = p.processed_files.saturating_add(1));
    }

    pub fn add_bytes(&self, bytes: u64) {
        self.with_progress(|p| p.bytes_uploaded = p.bytes_uploaded.saturating_add(bytes));
    }

    pub fn add_error(&self, error: String) {
        self.with_progress(|p| p.errors.push(error));
    }

    /// Request cancellation of the running sync.
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
        self.with_progress(|p| {
            p.status = SyncStatus::Cancelled;
            p.current_file = None;
        });
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }
}

impl Default for SyncState {
    fn default() -> Self {
        Self::new()
    }
}

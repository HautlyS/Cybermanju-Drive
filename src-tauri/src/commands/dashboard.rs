// Cybermanju Drive — Dashboard Control Commands
// Exposes web dashboard status and start/stop controls to the Tauri frontend.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// Dashboard status matching the frontend DashboardStatus type.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStatus {
    pub running: bool,
    pub port: u16,
    pub url: String,
    pub active_connections: u64,
}

/// Shared dashboard state for tracking connections and lifecycle.
pub struct DashboardState {
    pub running: AtomicBool,
    pub active_connections: AtomicU64,
    pub shutdown_tx: Mutex<Option<std::sync::mpsc::Sender<()>>>,
    pub server_thread: Mutex<Option<thread::JoinHandle<()>>>,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            active_connections: AtomicU64::new(0),
            shutdown_tx: Mutex::new(None),
            server_thread: Mutex::new(None),
        }
    }
}

impl Default for DashboardState {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the current web dashboard status.
#[tauri::command]
pub fn dashboard_status(
    state: tauri::State<'_, Arc<DashboardState>>,
) -> Result<DashboardStatus, String> {
    let port = crate::web_dashboard::DEFAULT_PORT;
    let running = state.running.load(Ordering::SeqCst);
    let active_connections = state.active_connections.load(Ordering::SeqCst);

    Ok(DashboardStatus {
        running,
        port,
        url: format!("http://localhost:{}", port),
        active_connections,
    })
}

/// Start the web dashboard on the configured port.
#[tauri::command]
pub fn start_dashboard(
    state: tauri::State<'_, Arc<DashboardState>>,
) -> Result<DashboardStatus, String> {
    let port = crate::web_dashboard::DEFAULT_PORT;

    // Check if already running
    if state.running.load(Ordering::SeqCst) {
        return Ok(DashboardStatus {
            running: true,
            port,
            url: format!("http://localhost:{}", port),
            active_connections: state.active_connections.load(Ordering::SeqCst),
        });
    }

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();
    {
        let mut tx_guard = state.shutdown_tx.lock().map_err(|e| e.to_string())?;
        *tx_guard = Some(shutdown_tx);
    }

    let db_path = "cybermanju.db".to_string();
    let running_flag = Arc::new(AtomicBool::new(true));
    let running_for_thread = running_flag.clone();
    let connections_for_thread = Arc::new(AtomicU64::new(0));
    let _connections_for_handler = connections_for_thread.clone();

    state.running.store(true, Ordering::SeqCst);

    let handle = thread::spawn(move || {
        let dashboard = Arc::new(crate::web_dashboard::WebDashboard::new(port, &db_path));
        let _ = dashboard.start();

        // Wait for shutdown signal
        loop {
            if !running_for_thread.load(Ordering::SeqCst) {
                break;
            }
            match shutdown_rx.try_recv() {
                Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        dashboard.stop();
        running_for_thread.store(false, Ordering::SeqCst);
    });

    // Store the thread handle
    {
        let mut thread_guard = state.server_thread.lock().map_err(|e| e.to_string())?;
        *thread_guard = Some(handle);
    }

    // Give it a moment to bind
    std::thread::sleep(std::time::Duration::from_millis(200));

    let running = std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok();
    state.running.store(running, Ordering::SeqCst);

    Ok(DashboardStatus {
        running,
        port,
        url: format!("http://localhost:{}", port),
        active_connections: 0,
    })
}

/// Stop the web dashboard.
#[tauri::command]
pub fn stop_dashboard(state: tauri::State<'_, Arc<DashboardState>>) -> Result<bool, String> {
    state.running.store(false, Ordering::SeqCst);

    // Send shutdown signal
    {
        let mut tx_guard = state.shutdown_tx.lock().map_err(|e| e.to_string())?;
        if let Some(tx) = tx_guard.take() {
            let _ = tx.send(());
        }
    }

    // Join the server thread
    {
        let mut thread_guard = state.server_thread.lock().map_err(|e| e.to_string())?;
        if let Some(handle) = thread_guard.take() {
            let _ = handle.join();
        }
    }

    state.active_connections.store(0, Ordering::SeqCst);
    Ok(true)
}

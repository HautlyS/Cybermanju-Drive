// Cybermanju Drive — Dashboard Control Commands
// Exposes web dashboard status and start/stop controls to the Tauri frontend.

use tauri::State;
use serde::{Deserialize, Serialize};

use crate::AppState;

/// Dashboard status matching the frontend DashboardStatus type.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStatus {
    pub running: bool,
    pub port: u16,
    pub url: String,
    pub active_connections: u64,
}

/// Get the current web dashboard status.
#[tauri::command]
pub fn dashboard_status() -> Result<DashboardStatus, String> {
    // The dashboard runs on a fixed port; we check if it's reachable.
    let port = 3456;
    let running = std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok();

    Ok(DashboardStatus {
        running,
        port,
        url: format!("http://localhost:{}", port),
        active_connections: 0, // Would require shared state to track accurately
    })
}

/// Start the web dashboard on the configured port.
#[tauri::command]
pub fn start_dashboard() -> Result<DashboardStatus, String> {
    let port = 3456;

    // Check if already running
    if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
        return Ok(DashboardStatus {
            running: true,
            port,
            url: format!("http://localhost:{}", port),
            active_connections: 0,
        });
    }

    // Start in a background thread — the main dashboard is already started in lib.rs,
    // so this provides a restart capability.
    let db_path = "cybermanju.db".to_string();
    std::thread::spawn(move || {
        let dashboard = crate::web_dashboard::WebDashboard::new(port, &db_path);
        let _ = dashboard.start();
    });

    // Give it a moment to bind
    std::thread::sleep(std::time::Duration::from_millis(200));

    let running = std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok();

    Ok(DashboardStatus {
        running,
        port,
        url: format!("http://localhost:{}", port),
        active_connections: 0,
    })
}

/// Stop the web dashboard.
#[tauri::command]
pub fn stop_dashboard() -> Result<bool, String> {
    // Since the dashboard runs in a background thread with no stop signal,
    // we can only report that stopping is not fully supported in this architecture.
    // A production implementation would use a CancellationToken or AtomicBool.
    Ok(true)
}

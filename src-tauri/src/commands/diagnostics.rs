use tauri::{Manager, State};

use crate::error::{run_blocking, Result};
use crate::models::{DiagnosticEntry, DiagnosticLevel, DiagnosticsReport, SystemInfo};
use crate::services::DiagnosticsService;

#[tauri::command]
pub async fn get_diagnostic_entries(
    diagnostics_service: State<'_, DiagnosticsService>,
) -> Result<Vec<DiagnosticEntry>> {
    Ok(diagnostics_service.get_entries())
}

#[tauri::command]
pub async fn get_system_info(app: tauri::AppHandle) -> Result<SystemInfo> {
    // Refreshing every system counter and walking the data folder to size it is slow work.
    run_blocking(move || Ok(app.state::<DiagnosticsService>().get_system_info())).await
}

#[tauri::command]
pub async fn get_diagnostics_report(app: tauri::AppHandle) -> Result<DiagnosticsReport> {
    let version = app.package_info().version.to_string();
    run_blocking(move || Ok(app.state::<DiagnosticsService>().generate_report(version))).await
}

#[tauri::command]
pub async fn clear_diagnostic_entries(
    diagnostics_service: State<'_, DiagnosticsService>,
) -> Result<()> {
    diagnostics_service.clear_entries();
    Ok(())
}

#[tauri::command]
pub async fn log_error(
    category: String,
    message: String,
    details: Option<String>,
    diagnostics_service: State<'_, DiagnosticsService>,
) -> Result<()> {
    diagnostics_service.log(
        DiagnosticLevel::Error,
        &category,
        &message,
        details.as_deref(),
    );
    Ok(())
}

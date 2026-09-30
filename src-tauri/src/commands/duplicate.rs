use tauri::{Emitter, Manager, State};

use crate::error::{run_blocking, Result};
use crate::models::{DuplicateCountInfo, DuplicateScanResult};
use crate::services::DuplicateService;

#[tauri::command]
pub async fn get_duplicate_groups(app: tauri::AppHandle) -> Result<DuplicateScanResult> {
    // Compares every track of the library with every other: off the runtime workers.
    run_blocking(move || app.state::<DuplicateService>().get_duplicate_groups()).await
}

#[tauri::command]
pub async fn get_duplicate_count(app: tauri::AppHandle) -> Result<DuplicateCountInfo> {
    run_blocking(move || app.state::<DuplicateService>().get_duplicate_count()).await
}

#[tauri::command]
pub async fn ignore_duplicate_group(
    app: tauri::AppHandle,
    track_ids: Vec<String>,
    duplicate: State<'_, DuplicateService>,
) -> Result<()> {
    duplicate.ignore_duplicate_group(track_ids)?;
    let _ = app.emit("duplicates-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn unignore_duplicate_group(
    app: tauri::AppHandle,
    track_ids: Vec<String>,
    duplicate: State<'_, DuplicateService>,
) -> Result<()> {
    duplicate.unignore_duplicate_group(track_ids)?;
    let _ = app.emit("duplicates-updated", ());
    Ok(())
}

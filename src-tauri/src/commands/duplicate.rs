use tauri::{Emitter, State};

use crate::error::Result;
use crate::models::{DuplicateCountInfo, DuplicateScanResult};
use crate::services::DuplicateService;

#[tauri::command]
pub async fn get_duplicate_groups(
    duplicate: State<'_, DuplicateService>,
) -> Result<DuplicateScanResult> {
    duplicate.get_duplicate_groups()
}

#[tauri::command]
pub async fn get_duplicate_count(
    duplicate: State<'_, DuplicateService>,
) -> Result<DuplicateCountInfo> {
    duplicate.get_duplicate_count()
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

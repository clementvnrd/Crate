use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::error::{run_blocking, Result};
use crate::models::export::ExportCheckpoint;
use crate::models::{DeviceExport, ExportRequest, ExportResult};
use crate::services::export::CheckpointService;
use crate::services::ExportService;

/// Export playlists to a USB device. Copies every audio file and writes the device database:
/// minutes of disk I/O, so it runs on the blocking pool.
#[tauri::command]
pub async fn export_playlists(
    request: ExportRequest,
    export_service: State<'_, Arc<ExportService>>,
    app_handle: AppHandle,
) -> Result<ExportResult> {
    let export_service = export_service.inner().clone();
    run_blocking(move || export_service.export_playlists(&app_handle, request)).await
}

/// Get all exports for a device
#[tauri::command]
pub async fn get_device_exports(
    device_id: String,
    export_service: State<'_, Arc<ExportService>>,
) -> Result<Vec<DeviceExport>> {
    export_service.get_device_exports(&device_id)
}

/// Cancel the current export operation
#[tauri::command]
pub async fn cancel_export(export_service: State<'_, Arc<ExportService>>) -> Result<()> {
    export_service.cancel_export();
    Ok(())
}

/// Clean up a failed export by removing copied files
#[tauri::command]
pub async fn cleanup_failed_export(
    device_id: String,
    mount_point: String,
    export_service: State<'_, Arc<ExportService>>,
) -> Result<()> {
    let export_service = export_service.inner().clone();
    run_blocking(move || export_service.cleanup_failed_export(&device_id, &mount_point)).await
}

/// Get a pending checkpoint for a device
#[tauri::command]
pub async fn get_pending_checkpoint(
    device_id: String,
    checkpoint_service: State<'_, Arc<CheckpointService>>,
) -> Result<Option<ExportCheckpoint>> {
    checkpoint_service.get_pending_checkpoint(&device_id)
}

/// Delete a checkpoint
#[tauri::command]
pub async fn delete_checkpoint(
    checkpoint_id: String,
    checkpoint_service: State<'_, Arc<CheckpointService>>,
) -> Result<()> {
    checkpoint_service.delete_checkpoint(&checkpoint_id)
}

/// Resume a previously interrupted export
#[tauri::command]
pub async fn resume_export(
    device_id: String,
    mount_point: String,
    export_service: State<'_, Arc<ExportService>>,
    checkpoint_service: State<'_, Arc<CheckpointService>>,
    app_handle: AppHandle,
) -> Result<ExportResult> {
    let export_service = export_service.inner().clone();
    let checkpoint_service = checkpoint_service.inner().clone();
    run_blocking(move || {
        // Get the pending checkpoint
        let checkpoint = checkpoint_service
            .get_pending_checkpoint(&device_id)?
            .ok_or_else(|| {
                crate::error::CrateError::Export("No pending checkpoint found".to_string())
            })?;

        // Create a request from the checkpoint
        let request = ExportRequest {
            device_id: checkpoint.device_id.clone(),
            mount_point,
            device_name: checkpoint.device_name.clone(),
            playlist_ids: checkpoint.playlist_ids.clone(),
            enable_sync: true,
            use_device_library_plus: false,
        };

        // Resume the export - the export service will detect the checkpoint
        // and skip already-completed tracks
        let result = export_service.export_playlists(&app_handle, request)?;

        // If successful, delete the checkpoint
        if result.success {
            checkpoint_service.complete_checkpoint(&checkpoint.id)?;
        }

        Ok(result)
    })
    .await
}

/// Export library or specific playlists to Pioneer rekordbox.xml format
#[tauri::command]
pub async fn export_rekordbox_xml(
    target_path: String,
    playlist_ids: Option<Vec<String>>,
    export_service: State<'_, Arc<ExportService>>,
) -> Result<usize> {
    let export_service = export_service.inner().clone();
    run_blocking(move || {
        export_service.export_rekordbox_xml(std::path::Path::new(&target_path), playlist_ids)
    })
    .await
}

/// Export a Set-mode plan (a caller-ordered list of track ids) to Pioneer rekordbox.xml, as a
/// single playlist named `set_name` holding that exact order.
#[tauri::command]
pub async fn export_set_rekordbox_xml(
    target_path: String,
    track_ids: Vec<String>,
    set_name: String,
    export_service: State<'_, Arc<ExportService>>,
) -> Result<usize> {
    let export_service = export_service.inner().clone();
    run_blocking(move || {
        export_service.export_set_rekordbox_xml(
            std::path::Path::new(&target_path),
            &track_ids,
            &set_name,
        )
    })
    .await
}

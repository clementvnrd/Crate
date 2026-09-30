use tauri::Manager;

use crate::error::{run_blocking, Result};
use crate::models::UsbDevice;
use crate::services::DeviceService;

#[tauri::command]
pub async fn get_devices(app: tauri::AppHandle) -> Result<Vec<UsbDevice>> {
    // Lists the disks and runs `diskutil info` for each removable one.
    run_blocking(move || Ok(app.state::<DeviceService>().get_removable_devices())).await
}

#[tauri::command]
pub async fn eject_device(app: tauri::AppHandle, mount_point: String) -> Result<()> {
    run_blocking(move || app.state::<DeviceService>().eject_device(&mount_point)).await
}

#[tauri::command]
pub async fn reformat_device(
    app: tauri::AppHandle,
    mount_point: String,
    volume_name: String,
) -> Result<()> {
    // Waits for the administrator dialog, then erases the disk: minutes, never on a worker.
    run_blocking(move || {
        app.state::<DeviceService>()
            .reformat_device(&mount_point, &volume_name)
    })
    .await
}

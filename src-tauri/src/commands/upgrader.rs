use tauri::{Emitter, State};

use crate::error::Result;
use crate::models::{
    UpgradeCountInfo, UpgradeMatch, UpgradeReplacementResult, UpgradeScanResult,
};
use crate::services::{BeatportUpgraderService, LibraryService, SettingsService};

#[tauri::command]
pub async fn get_upgrade_matches(
    upgrader: State<'_, BeatportUpgraderService>,
) -> Result<UpgradeScanResult> {
    let token = upgrader.get_active_token().await?;
    upgrader.get_upgrade_matches(Some(&token)).await
}

#[tauri::command]
pub async fn get_upgrade_count(
    upgrader: State<'_, BeatportUpgraderService>,
) -> Result<UpgradeCountInfo> {
    upgrader.get_upgrade_count(None).await
}

#[tauri::command]
pub async fn ignore_upgrade_match(
    app: tauri::AppHandle,
    track_id: String,
    beatport_id: String,
    upgrader: State<'_, BeatportUpgraderService>,
) -> Result<()> {
    upgrader.ignore_upgrade_match(&track_id, &beatport_id)?;
    let _ = app.emit("upgrades-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn unignore_upgrade_match(
    app: tauri::AppHandle,
    track_id: String,
    beatport_id: String,
    upgrader: State<'_, BeatportUpgraderService>,
) -> Result<()> {
    upgrader.unignore_upgrade_match(&track_id, &beatport_id)?;
    let _ = app.emit("upgrades-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn execute_upgrade_replacements(
    app: tauri::AppHandle,
    matches: Vec<UpgradeMatch>,
    upgrader: State<'_, BeatportUpgraderService>,
    library: State<'_, LibraryService>,
    settings: State<'_, SettingsService>,
) -> Result<UpgradeReplacementResult> {
    let current_settings = settings.get_settings().ok();
    let custom_dest = current_settings.and_then(|s| s.beatport_download_destination);
    let progress_app = app.clone();
    let on_progress = move |progress: crate::services::beatport::UpgradeProgress| {
        let _ = progress_app.emit("upgrade-progress", progress);
    };
    let res = upgrader
        .execute_upgrade_replacements(&matches, Some(&library), custom_dest.as_deref(), &on_progress)
        .await?;
    let _ = app.emit("upgrades-updated", ());
    let _ = app.emit("library-updated", ());
    Ok(res)
}

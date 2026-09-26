use std::path::PathBuf;

use tauri::{Emitter, State};

use crate::error::Result;
use crate::models::{
    DuplicateResolution, FileMatchResult, ImportResult, ImportResultWithDuplicates, Track,
    TrackFilter, TrackUpdate,
};
use crate::services::library::RescanResult;
use crate::services::LibraryService;

#[tauri::command]
pub async fn import_tracks(
    app: tauri::AppHandle,
    paths: Vec<String>,
    library: State<'_, LibraryService>,
) -> Result<ImportResult> {
    let pathbufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let res = library.import_tracks(pathbufs)?;
    if !res.tracks.is_empty() {
        let _ = app.emit("duplicates-updated", ());
    }
    Ok(res)
}

#[tauri::command]
pub async fn get_tracks(
    filter: Option<TrackFilter>,
    library: State<'_, LibraryService>,
) -> Result<Vec<Track>> {
    library.get_tracks(filter)
}

#[tauri::command]
pub async fn get_track(id: String, library: State<'_, LibraryService>) -> Result<Track> {
    library.get_track(&id)
}

#[tauri::command]
pub async fn update_track(
    app: tauri::AppHandle,
    id: String,
    update: TrackUpdate,
    library: State<'_, LibraryService>,
) -> Result<Track> {
    let res = library.update_track(&id, update)?;
    let _ = app.emit("duplicates-updated", ());
    Ok(res)
}

#[tauri::command]
pub async fn delete_tracks(
    app: tauri::AppHandle,
    ids: Vec<String>,
    library: State<'_, LibraryService>,
) -> Result<()> {
    let res = library.delete_tracks(ids)?;
    let _ = app.emit("duplicates-updated", ());
    Ok(res)
}

#[tauri::command]
pub async fn delete_tracks_and_files(
    app: tauri::AppHandle,
    ids: Vec<String>,
    library: State<'_, LibraryService>,
) -> Result<()> {
    let res = library.delete_tracks_and_files(ids)?;
    let _ = app.emit("duplicates-updated", ());
    Ok(res)
}

#[tauri::command]
pub async fn search_tracks(
    query: String,
    library: State<'_, LibraryService>,
) -> Result<Vec<Track>> {
    let filter = TrackFilter {
        search: Some(query),
        ..Default::default()
    };
    library.get_tracks(Some(filter))
}

#[tauri::command]
pub async fn rescan_artwork(library: State<'_, LibraryService>) -> Result<RescanResult> {
    library.rescan_all_artwork()
}

#[tauri::command]
pub async fn rescan_track_artwork(id: String, library: State<'_, LibraryService>) -> Result<bool> {
    library.rescan_track_artwork(&id)
}

#[tauri::command]
pub async fn check_file_exists(
    track_id: String,
    library: State<'_, LibraryService>,
) -> Result<bool> {
    library.check_track_file_exists(&track_id)
}

#[tauri::command]
pub async fn validate_replacement_file(
    track_id: String,
    new_path: String,
    library: State<'_, LibraryService>,
) -> Result<FileMatchResult> {
    library.validate_replacement_file(&track_id, &PathBuf::from(new_path))
}

#[tauri::command]
pub async fn relocate_track(
    track_id: String,
    new_path: String,
    force: bool,
    library: State<'_, LibraryService>,
) -> Result<Track> {
    library.relocate_track(&track_id, &PathBuf::from(new_path), force)
}

#[tauri::command]
pub async fn set_track_colors(
    track_ids: Vec<String>,
    color: Option<String>,
    library: State<'_, LibraryService>,
) -> Result<()> {
    library.set_track_colors(track_ids, color)
}

#[tauri::command]
pub async fn update_tracks(
    app: tauri::AppHandle,
    ids: Vec<String>,
    update: TrackUpdate,
    library: State<'_, LibraryService>,
) -> Result<Vec<Track>> {
    let res = library.update_tracks(ids, update)?;
    let _ = app.emit("duplicates-updated", ());
    Ok(res)
}

#[tauri::command]
pub async fn set_track_artwork(
    track_id: String,
    file_path: String,
    library: State<'_, LibraryService>,
) -> Result<Track> {
    library.set_track_artwork(&track_id, &PathBuf::from(file_path))
}

#[tauri::command]
pub async fn delete_track_artwork(
    track_id: String,
    library: State<'_, LibraryService>,
) -> Result<Track> {
    library.delete_track_artwork(&track_id)
}

#[tauri::command]
pub async fn reextract_track_artwork(
    track_id: String,
    library: State<'_, LibraryService>,
) -> Result<Track> {
    library.reextract_track_artwork(&track_id)
}

#[tauri::command]
pub async fn compare_track_artworks(
    track_ids: Vec<String>,
    library: State<'_, LibraryService>,
) -> Result<Option<String>> {
    library.compare_track_artworks(&track_ids)
}

#[tauri::command]
pub async fn import_tracks_with_duplicates(
    app: tauri::AppHandle,
    paths: Vec<String>,
    library: State<'_, LibraryService>,
) -> Result<ImportResultWithDuplicates> {
    let pathbufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let res = library.import_tracks_with_duplicate_detection(pathbufs)?;
    if !res.tracks.is_empty() {
        let _ = app.emit("duplicates-updated", ());
    }
    Ok(res)
}

#[tauri::command]
pub async fn resolve_duplicate(
    app: tauri::AppHandle,
    resolution: DuplicateResolution,
    library: State<'_, LibraryService>,
) -> Result<Option<Track>> {
    let res = library.resolve_duplicate(resolution)?;
    let _ = app.emit("duplicates-updated", ());
    Ok(res)
}

#[tauri::command]
pub async fn resync_mixed_in_key_tracks(
    app: tauri::AppHandle,
    track_ids: Option<Vec<String>>,
    library: State<'_, LibraryService>,
) -> Result<RescanResult> {
    let res = library.resync_mixed_in_key_tracks(track_ids)?;
    if res.updated_count > 0 {
        let _ = app.emit("duplicates-updated", ());
    }
    Ok(res)
}

#[tauri::command]
pub async fn get_mik_database_status(
    library: State<'_, LibraryService>,
) -> Result<crate::services::library::MikDatabaseStatus> {
    library.get_mik_database_status()
}

#[tauri::command]
pub async fn sync_from_mik_database(
    track_ids: Option<Vec<String>>,
    app: tauri::AppHandle,
    library: State<'_, LibraryService>,
) -> Result<crate::services::library::MikSyncResult> {
    let res = library.sync_from_mik_database(track_ids.as_deref())?;
    if res.added > 0 || res.updated > 0 || res.removed > 0 {
        let _ = app.emit("duplicates-updated", ());
    }
    Ok(res)
}

#[tauri::command]
pub async fn prune_missing_tracks(
    app: tauri::AppHandle,
    library: State<'_, LibraryService>,
) -> Result<usize> {
    let res = library.prune_missing_tracks()?;
    if res > 0 {
        let _ = app.emit("duplicates-updated", ());
    }
    Ok(res)
}

#[tauri::command]
pub async fn get_track_waveform(
    track_id: String,
    library: State<'_, LibraryService>,
) -> Result<Option<Vec<u8>>> {
    library.get_track_waveform(&track_id)
}

#[tauri::command]
pub async fn get_track_cues(
    track_id: String,
    library: State<'_, LibraryService>,
) -> Result<Vec<crate::models::Cue>> {
    library.get_track_cues(&track_id)
}

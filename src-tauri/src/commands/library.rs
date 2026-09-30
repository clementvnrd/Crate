use std::path::PathBuf;

use tauri::{Emitter, Manager, State};

use crate::error::{run_blocking, Result};
use crate::models::{
    DuplicateResolution, FileMatchResult, ImportResult, ImportResultWithDuplicates, Track,
    TrackFilter, TrackUpdate,
};
use crate::services::library::{NextTrackSuggestion, RescanResult};
use crate::services::LibraryService;

/// Runs `work` with the managed [`LibraryService`] on the blocking pool (see [`run_blocking`]).
/// For the commands below that scan files, parse tags, decode audio or open the Mixed In Key
/// database: each can take seconds to minutes, and must not park a runtime worker.
async fn with_library<T, F>(app: &tauri::AppHandle, work: F) -> Result<T>
where
    F: FnOnce(&LibraryService) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let app = app.clone();
    run_blocking(move || work(&app.state::<LibraryService>())).await
}

#[tauri::command]
pub async fn import_tracks(app: tauri::AppHandle, paths: Vec<String>) -> Result<ImportResult> {
    let pathbufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let res = with_library(&app, move |library| library.import_tracks(pathbufs)).await?;
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
    library.delete_tracks(ids)?;
    let _ = app.emit("duplicates-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn delete_tracks_and_files(app: tauri::AppHandle, ids: Vec<String>) -> Result<()> {
    with_library(&app, move |library| library.delete_tracks_and_files(ids)).await?;
    let _ = app.emit("duplicates-updated", ());
    Ok(())
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
pub async fn rescan_artwork(app: tauri::AppHandle) -> Result<RescanResult> {
    with_library(&app, |library| library.rescan_all_artwork()).await
}

#[tauri::command]
pub async fn rescan_track_artwork(app: tauri::AppHandle, id: String) -> Result<bool> {
    with_library(&app, move |library| library.rescan_track_artwork(&id)).await
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
    app: tauri::AppHandle,
    track_id: String,
    new_path: String,
) -> Result<FileMatchResult> {
    with_library(&app, move |library| {
        library.validate_replacement_file(&track_id, &PathBuf::from(new_path))
    })
    .await
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
    app: tauri::AppHandle,
    track_id: String,
    file_path: String,
) -> Result<Track> {
    with_library(&app, move |library| {
        library.set_track_artwork(&track_id, &PathBuf::from(file_path))
    })
    .await
}

#[tauri::command]
pub async fn delete_track_artwork(
    track_id: String,
    library: State<'_, LibraryService>,
) -> Result<Track> {
    library.delete_track_artwork(&track_id)
}

#[tauri::command]
pub async fn reextract_track_artwork(app: tauri::AppHandle, track_id: String) -> Result<Track> {
    with_library(&app, move |library| {
        library.reextract_track_artwork(&track_id)
    })
    .await
}

#[tauri::command]
pub async fn compare_track_artworks(
    app: tauri::AppHandle,
    track_ids: Vec<String>,
) -> Result<Option<String>> {
    with_library(&app, move |library| {
        library.compare_track_artworks(&track_ids)
    })
    .await
}

#[tauri::command]
pub async fn import_tracks_with_duplicates(
    app: tauri::AppHandle,
    paths: Vec<String>,
) -> Result<ImportResultWithDuplicates> {
    let pathbufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let res = with_library(&app, move |library| {
        library.import_tracks_with_duplicate_detection(pathbufs)
    })
    .await?;
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
) -> Result<RescanResult> {
    let res = with_library(&app, move |library| {
        library.resync_mixed_in_key_tracks(track_ids)
    })
    .await?;
    if res.updated_count > 0 {
        let _ = app.emit("duplicates-updated", ());
    }
    Ok(res)
}

#[tauri::command]
pub async fn get_mik_database_status(
    app: tauri::AppHandle,
) -> Result<crate::services::library::MikDatabaseStatus> {
    with_library(&app, |library| library.get_mik_database_status()).await
}

#[tauri::command]
pub async fn sync_from_mik_database(
    track_ids: Option<Vec<String>>,
    app: tauri::AppHandle,
) -> Result<crate::services::library::MikSyncResult> {
    let res = with_library(&app, move |library| {
        library.sync_from_mik_database(track_ids.as_deref())
    })
    .await?;
    if res.added > 0 || res.updated > 0 || res.removed > 0 {
        let _ = app.emit("duplicates-updated", ());
    }
    Ok(res)
}

#[tauri::command]
pub async fn prune_missing_tracks(app: tauri::AppHandle) -> Result<usize> {
    let res = with_library(&app, |library| library.prune_missing_tracks()).await?;
    if res > 0 {
        let _ = app.emit("duplicates-updated", ());
    }
    Ok(res)
}

#[tauri::command]
pub async fn get_track_waveform(
    app: tauri::AppHandle,
    track_id: String,
) -> Result<Option<Vec<u8>>> {
    with_library(&app, move |library| library.get_track_waveform(&track_id)).await
}

#[tauri::command]
pub async fn get_track_cues(
    app: tauri::AppHandle,
    track_id: String,
) -> Result<Vec<crate::models::Cue>> {
    with_library(&app, move |library| library.get_track_cues(&track_id)).await
}

/// Tracks to play after `track_id`: first what followed it in the user's Rekordbox sets, then
/// library tracks that mix well with it. `limit` defaults to 10 (at most 50).
#[tauri::command]
pub async fn suggest_next_tracks(
    app: tauri::AppHandle,
    track_id: String,
    limit: Option<usize>,
) -> Result<Vec<NextTrackSuggestion>> {
    with_library(&app, move |library| {
        library.suggest_next_tracks(&track_id, limit)
    })
    .await
}

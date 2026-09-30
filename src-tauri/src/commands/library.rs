use std::path::PathBuf;

use tauri::{Emitter, Manager, State};

use crate::error::{run_blocking, Result};
use crate::models::{
    DuplicateResolution, FileMatchResult, ImportResult, ImportResultWithDuplicates, Track,
    TrackFilter, TrackUpdate,
};
use crate::services::library::{
    read_rekordbox_xml, DiscrepancyReport, NextTrackSuggestion, OrganisationBatch,
    OrganisationPlan, OrganisationResult, OrganisationRule, RescanResult, SetAnalysis,
};
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

/// Where Crate, Mixed In Key and (with an export) Rekordbox disagree: keys, tempos, energy, cues,
/// missing files, tracks one has and another lacks. Read only: nothing is changed or deleted.
/// `rekordbox_xml_path` is an export from Rekordbox (File, Export Collection in xml format).
#[tauri::command]
pub async fn get_discrepancy_report(
    app: tauri::AppHandle,
    rekordbox_xml_path: Option<String>,
) -> Result<DiscrepancyReport> {
    with_library(&app, move |library| {
        let xml = rekordbox_xml_path
            .as_deref()
            .map(read_rekordbox_xml)
            .transpose()?;
        library.build_discrepancy_report(xml.as_deref())
    })
    .await
}

/// Preview of the assisted organisation: where every file would go under `rule`. Changes nothing.
/// `track_ids` restricts it to a selection, `None` is the whole library.
#[tauri::command]
pub async fn plan_organisation(
    app: tauri::AppHandle,
    rule: OrganisationRule,
    track_ids: Option<Vec<String>>,
) -> Result<OrganisationPlan> {
    with_library(&app, move |library| {
        library.plan_organisation(&rule, track_ids.as_deref())
    })
    .await
}

/// Moves the files, but only if the plan is exactly the previewed one (`expected_plan_id`) and the
/// caller confirms that Rekordbox and other tools that remember file paths will lose them.
/// Nothing is overwritten or deleted; the batch can be undone.
#[tauri::command]
pub async fn apply_organisation(
    app: tauri::AppHandle,
    rule: OrganisationRule,
    track_ids: Option<Vec<String>>,
    expected_plan_id: String,
    understands_external_tools: bool,
) -> Result<OrganisationResult> {
    let result = with_library(&app, move |library| {
        library.apply_organisation(
            &rule,
            track_ids.as_deref(),
            &expected_plan_id,
            understands_external_tools,
        )
    })
    .await?;
    if result.moved > 0 {
        let _ = app.emit("duplicates-updated", ());
    }
    Ok(result)
}

/// Puts an applied batch back where it was.
#[tauri::command]
pub async fn undo_organisation(
    app: tauri::AppHandle,
    batch_id: String,
) -> Result<OrganisationResult> {
    let result = with_library(&app, move |library| library.undo_organisation(&batch_id)).await?;
    if result.moved > 0 {
        let _ = app.emit("duplicates-updated", ());
    }
    Ok(result)
}

/// The organisation batches that were applied, latest first (10 by default).
#[tauri::command]
pub async fn get_organisation_batches(
    app: tauri::AppHandle,
    limit: Option<usize>,
) -> Result<Vec<OrganisationBatch>> {
    with_library(&app, move |library| {
        library.organisation_batches(limit.unwrap_or(10))
    })
    .await
}

/// Checks a DJ set in the given order: the key, tempo and energy of every transition, and for the
/// ones that clash or jump in tempo, library tracks that could bridge them.
#[tauri::command]
pub async fn analyze_set(app: tauri::AppHandle, track_ids: Vec<String>) -> Result<SetAnalysis> {
    with_library(&app, move |library| library.analyze_set(&track_ids)).await
}

/// The same tracks in an order that mixes well, as track ids. `start_track_id` opens the set; by
/// default it is the calmest track.
#[tauri::command]
pub async fn suggest_set_order(
    app: tauri::AppHandle,
    track_ids: Vec<String>,
    start_track_id: Option<String>,
) -> Result<Vec<String>> {
    with_library(&app, move |library| {
        library.suggest_set_order(&track_ids, start_track_id.as_deref())
    })
    .await
}

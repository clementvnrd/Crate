use std::path::PathBuf;
use tauri::{Manager, State};

use crate::error::{run_blocking, Result};
use crate::models::StandaloneTrack;
use crate::services::audio::PlaybackState;
use crate::services::{AudioService, PlayerTrackerService, StandaloneService};
use crate::StartupFile;

#[tauri::command]
pub async fn read_standalone_track(app: tauri::AppHandle, path: String) -> Result<StandaloneTrack> {
    // Parses the tags and extracts the cover: disk and image work, off the runtime workers.
    run_blocking(move || {
        app.state::<StandaloneService>()
            .read_standalone_track(&path)
    })
    .await
}

#[tauri::command]
pub async fn get_recent_standalone_tracks(
    limit: Option<usize>,
    standalone: State<'_, StandaloneService>,
) -> Result<Vec<StandaloneTrack>> {
    standalone.get_recent_standalone_tracks(limit)
}

#[tauri::command]
pub async fn add_recent_standalone_track(
    track: StandaloneTrack,
    standalone: State<'_, StandaloneService>,
) -> Result<()> {
    standalone.add_recent_standalone_track(&track)
}

#[tauri::command]
pub async fn remove_recent_standalone_track(
    id: String,
    standalone: State<'_, StandaloneService>,
) -> Result<()> {
    standalone.remove_recent_standalone_track(&id)
}

#[tauri::command]
pub async fn clear_recent_standalone_tracks(
    standalone: State<'_, StandaloneService>,
) -> Result<()> {
    standalone.clear_recent_standalone_tracks()
}

/// Returns the files opened before the frontend was ready (in order) and switches to live
/// `open-file` events. Call it after subscribing to `open-file`.
#[tauri::command]
pub async fn take_startup_files(startup_file: State<'_, StartupFile>) -> Result<Vec<String>> {
    Ok(startup_file.drain())
}

#[tauri::command]
pub async fn play_standalone_track(
    app: tauri::AppHandle,
    path: String,
    id: Option<String>,
    duration_ms: Option<u64>,
) -> Result<PlaybackState> {
    // Tag parsing, cover extraction and the wait for the audio thread to open the file all
    // block: the whole sequence runs on the blocking pool.
    run_blocking(move || {
        let standalone = app.state::<StandaloneService>();
        let audio = app.state::<AudioService>();
        let tracker = app.state::<PlayerTrackerService>();
        let track_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut resolved_duration = duration_ms;
        let mut listening_ctx = None;
        if let Ok(track) = standalone.read_standalone_track(&path) {
            if resolved_duration.is_none() || resolved_duration == Some(0) {
                resolved_duration = Some(track.duration_ms.max(0) as u64);
            }
            listening_ctx = Some(crate::services::player::TrackPlayingContext {
                track_id: Some(track_id.clone()),
                source: if track.is_in_library {
                    "crate_local".to_string()
                } else {
                    "crate_standalone".to_string()
                },
                title: track
                    .title
                    .clone()
                    .unwrap_or_else(|| "Unknown Track".to_string()),
                artist: track
                    .artist
                    .clone()
                    .unwrap_or_else(|| "Unknown Artist".to_string()),
                album: track.album.clone(),
                duration_ms: track.duration_ms.max(0) as u64,
                bpm: track.bpm,
                key: track.key.clone(),
                energy: track.energy,
                format: Some(track.format.clone()),
                artwork_url: track.artwork_path.clone(),
                started_at: chrono::Utc::now().to_rfc3339(),
            });
        }
        let state = audio.play_track(track_id, PathBuf::from(&path), resolved_duration)?;
        // As for library tracks: a file that fails to load opens no listening session (B37).
        if let Some(ctx) = listening_ctx {
            tracker.on_track_started(ctx);
        }
        Ok(state)
    })
    .await
}

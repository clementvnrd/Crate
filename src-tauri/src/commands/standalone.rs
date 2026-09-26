use std::path::PathBuf;
use tauri::State;

use crate::error::Result;
use crate::models::StandaloneTrack;
use crate::services::audio::PlaybackState;
use crate::services::{AudioService, PlayerTrackerService, StandaloneService};
use crate::StartupFile;

#[tauri::command]
pub async fn read_standalone_track(
    path: String,
    standalone: State<'_, StandaloneService>,
) -> Result<StandaloneTrack> {
    standalone.read_standalone_track(&path)
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
    path: String,
    id: Option<String>,
    duration_ms: Option<u64>,
    standalone: State<'_, StandaloneService>,
    audio: State<'_, AudioService>,
    tracker: State<'_, PlayerTrackerService>,
) -> Result<PlaybackState> {
    let track_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut resolved_duration = duration_ms;
    if let Ok(track) = standalone.read_standalone_track(&path) {
        if resolved_duration.is_none() || resolved_duration == Some(0) {
            resolved_duration = Some(track.duration_ms.max(0) as u64);
        }
        let ctx = crate::services::player::TrackPlayingContext {
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
        };
        tracker.on_track_started(ctx);
    }
    audio.play_track(track_id, PathBuf::from(&path), resolved_duration)
}

use std::path::PathBuf;

use tauri::State;

use crate::error::Result;
use crate::models::AudioDevice;
use crate::services::audio::PlaybackState;
use crate::services::{AudioService, LibraryService, PlayerTrackerService, SettingsService};

#[tauri::command]
pub async fn play_track(
    id: String,
    library: State<'_, LibraryService>,
    audio: State<'_, AudioService>,
    tracker: State<'_, PlayerTrackerService>,
) -> Result<PlaybackState> {
    let track = library.get_track(&id)?;
    let path = PathBuf::from(&track.file_path);

    let ctx = crate::services::player::TrackPlayingContext {
        track_id: Some(id.clone()),
        source: "crate_local".to_string(),
        title: track.title.clone().unwrap_or_else(|| "Unknown Track".to_string()),
        artist: track.artist.clone().unwrap_or_else(|| "Unknown Artist".to_string()),
        album: track.album.clone(),
        duration_ms: track.duration_ms.max(0) as u64,
        bpm: track.bpm,
        key: track.key.clone(),
        energy: track.energy,
        format: Some(track.format.clone()),
        artwork_url: track.artwork_path.clone(),
        started_at: chrono::Utc::now().to_rfc3339(),
        start_instant: std::time::Instant::now(),
        recorded: false,
    };
    tracker.on_track_started(ctx);
    let duration_ms = track.duration_ms.max(0) as u64;
    audio.play_track(id, path, Some(duration_ms))
}

#[tauri::command]
pub async fn pause(
    audio: State<'_, AudioService>,
    tracker: State<'_, PlayerTrackerService>,
) -> Result<PlaybackState> {
    let state = audio.pause()?;
    tracker.check_and_record_if_due(state.position_ms);
    Ok(state)
}

#[tauri::command]
pub async fn resume(audio: State<'_, AudioService>) -> Result<PlaybackState> {
    audio.resume()
}

#[tauri::command]
pub async fn stop(
    audio: State<'_, AudioService>,
    tracker: State<'_, PlayerTrackerService>,
) -> Result<PlaybackState> {
    tracker.on_playback_stopped();
    audio.stop()
}

#[tauri::command]
pub async fn seek(
    position_ms: u64,
    audio: State<'_, AudioService>,
    tracker: State<'_, PlayerTrackerService>,
) -> Result<PlaybackState> {
    tracker.check_and_record_if_due(position_ms);
    audio.seek(position_ms)
}

#[tauri::command]
pub async fn set_volume(volume: f32, audio: State<'_, AudioService>) -> Result<PlaybackState> {
    audio.set_volume(volume)
}

#[tauri::command]
pub async fn set_speed(speed: f32, audio: State<'_, AudioService>) -> Result<PlaybackState> {
    audio.set_speed(speed)
}

#[tauri::command]
pub async fn get_playback_state(
    audio: State<'_, AudioService>,
    tracker: State<'_, PlayerTrackerService>,
) -> Result<PlaybackState> {
    let state = audio.get_state()?;
    if state.is_playing {
        tracker.check_and_record_if_due(state.position_ms);
    }
    Ok(state)
}


#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<AudioDevice>> {
    AudioService::get_output_devices()
}

#[tauri::command]
pub async fn set_audio_device(
    device_name: Option<String>,
    audio: State<'_, AudioService>,
    settings: State<'_, SettingsService>,
) -> Result<()> {
    // Set the device in the audio service
    audio.set_device(device_name.clone())?;

    // Persist the setting
    match device_name {
        Some(name) => settings.set_setting("audio_device", &name)?,
        None => settings.set_setting("audio_device", "")?,
    }

    Ok(())
}

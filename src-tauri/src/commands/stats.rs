use tauri::{Manager, State};

use crate::error::{run_blocking, CrateError, Result};
use crate::models::stats::{
    BpmBucketItem, HarmonicStatsItem, HeatmapCell, ListenEvent, RekordboxSession, SpotifyAuthState,
    SpotifyImportResult, SpotifyNowPlaying, SpotifyResetResult, StatsSummary, TopArtistItem,
    TopTrackItem,
};
use crate::services::stats::recap::Recap;
use crate::services::stats::session_timeline::SessionTimeline;
use crate::services::stats::{
    HistoryExportFormat, MikTrackerService, RecapPeriod, RekordboxTrackerService,
    SpotifyTrackerService, StatsRecorderService,
};

// ==========================================
// Stats Recorder Commands
// ==========================================

#[tauri::command]
pub async fn get_stats_summary(
    time_range: String,
    stats: State<'_, StatsRecorderService>,
) -> Result<StatsSummary> {
    stats.get_stats_summary(&time_range)
}

/// The local calendar years with listening data (listens or Rekordbox sets), newest first: the
/// "previous years" the Pulse period bar offers.
#[tauri::command]
pub async fn get_listening_years(stats: State<'_, StatsRecorderService>) -> Result<Vec<i32>> {
    stats.get_listening_years()
}

#[tauri::command]
pub async fn get_top_tracks(
    time_range: String,
    limit: Option<usize>,
    stats: State<'_, StatsRecorderService>,
) -> Result<Vec<TopTrackItem>> {
    stats.get_top_tracks(&time_range, limit.unwrap_or(20))
}

#[tauri::command]
pub async fn get_top_artists(
    time_range: String,
    limit: Option<usize>,
    stats: State<'_, StatsRecorderService>,
) -> Result<Vec<TopArtistItem>> {
    stats.get_top_artists(&time_range, limit.unwrap_or(20))
}

#[tauri::command]
pub async fn get_harmonic_stats(
    time_range: String,
    stats: State<'_, StatsRecorderService>,
) -> Result<Vec<HarmonicStatsItem>> {
    stats.get_harmonic_stats(&time_range)
}

#[tauri::command]
pub async fn get_bpm_stats(
    time_range: String,
    stats: State<'_, StatsRecorderService>,
) -> Result<Vec<BpmBucketItem>> {
    stats.get_bpm_stats(&time_range)
}

#[tauri::command]
pub async fn get_listening_heatmap(
    time_range: String,
    stats: State<'_, StatsRecorderService>,
) -> Result<Vec<HeatmapCell>> {
    stats.get_listening_heatmap(&time_range)
}

#[tauri::command]
pub async fn get_recent_listens(
    limit: Option<usize>,
    stats: State<'_, StatsRecorderService>,
) -> Result<Vec<ListenEvent>> {
    stats.get_recent_listens(limit.unwrap_or(50))
}

/// Writes the listening history, oldest first, to `path` as CSV or JSON and returns how many
/// listens were exported. The path comes from the native save dialog and must end in `.csv` or
/// `.json` according to `format`. `time_range` (the Pulse's selected period) limits the export to
/// that period; without it, the whole history is written.
#[tauri::command]
pub async fn export_listening_history(
    app: tauri::AppHandle,
    format: HistoryExportFormat,
    path: String,
    time_range: Option<String>,
) -> Result<usize> {
    run_blocking(move || {
        app.state::<StatsRecorderService>()
            .export_listen_history_in_range(
                format,
                std::path::Path::new(&path),
                time_range.as_deref().unwrap_or("all"),
            )
    })
    .await
}

/// The tracks of one Rekordbox set in order, with the key, tempo and energy of each and how
/// every transition mixes.
#[tauri::command]
pub async fn get_rekordbox_session_timeline(
    app: tauri::AppHandle,
    session_id: String,
) -> Result<SessionTimeline> {
    run_blocking(move || {
        app.state::<StatsRecorderService>()
            .get_session_timeline(&session_id)
    })
    .await
}

/// The recap of the current week or year (`offset` 0) or of an earlier one (1 = the previous).
#[tauri::command]
pub async fn get_recap(
    app: tauri::AppHandle,
    period: RecapPeriod,
    offset: Option<u32>,
) -> Result<Recap> {
    run_blocking(move || {
        app.state::<StatsRecorderService>()
            .get_recap(period, offset.unwrap_or(0))
    })
    .await
}

// ==========================================
// Spotify Tracker Commands
// ==========================================

#[tauri::command]
pub async fn set_spotify_client_id(
    client_id: String,
    spotify: State<'_, SpotifyTrackerService>,
) -> Result<()> {
    spotify.set_client_id(&client_id)
}

#[tauri::command]
pub async fn spotify_get_client_id(
    spotify: State<'_, SpotifyTrackerService>,
) -> Result<Option<String>> {
    Ok(spotify.get_client_id())
}

#[tauri::command]
pub async fn set_spotify_client_secret(
    client_secret: String,
    spotify: State<'_, SpotifyTrackerService>,
) -> Result<()> {
    spotify.set_client_secret(&client_secret)
}

/// Tells the UI whether a client secret is stored, without ever sending it back to the webview.
#[tauri::command]
pub async fn spotify_has_client_secret(spotify: State<'_, SpotifyTrackerService>) -> Result<bool> {
    Ok(spotify.get_client_secret().is_some())
}

#[tauri::command]
pub async fn spotify_get_auth_url(
    client_id: Option<String>,
    redirect_uri: Option<String>,
    app: tauri::AppHandle,
    spotify: State<'_, SpotifyTrackerService>,
) -> Result<String> {
    // The callback server only runs while a sign-in is in progress.
    spotify.ensure_loopback_server(app);
    spotify.get_auth_url(client_id.as_deref(), redirect_uri.as_deref())
}

#[tauri::command]
pub async fn spotify_exchange_code(
    code: String,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    state: Option<String>,
    app: tauri::AppHandle,
    spotify: State<'_, SpotifyTrackerService>,
) -> Result<SpotifyAuthState> {
    use tauri::Emitter;
    let auth_state = spotify
        .exchange_code(
            &code,
            client_id.as_deref(),
            redirect_uri.as_deref(),
            state.as_deref(),
        )
        .await?;
    let _ = app.emit("spotify-auth-changed", &auth_state);
    Ok(auth_state)
}

#[tauri::command]
pub async fn spotify_disconnect(spotify: State<'_, SpotifyTrackerService>) -> Result<()> {
    spotify.disconnect()
}

#[tauri::command]
pub async fn spotify_get_auth_state(
    spotify: State<'_, SpotifyTrackerService>,
) -> Result<SpotifyAuthState> {
    spotify.get_auth_state()
}

#[tauri::command]
pub async fn spotify_get_now_playing(
    spotify: State<'_, SpotifyTrackerService>,
) -> Result<Option<SpotifyNowPlaying>> {
    spotify.get_currently_playing().await
}

#[tauri::command]
pub async fn spotify_import_history(
    app: tauri::AppHandle,
    json_content: String,
) -> Result<SpotifyImportResult> {
    // Parses a multi-megabyte JSON file and writes every listen in one transaction.
    run_blocking(move || {
        app.state::<SpotifyTrackerService>()
            .import_streaming_history_json(&json_content)
    })
    .await
}

#[tauri::command]
pub async fn sync_spotify_recently_played(
    spotify: State<'_, SpotifyTrackerService>,
) -> Result<usize> {
    spotify.sync_recently_played().await
}

/// Number of currently recorded Spotify listens — shown in the confirmation dialog before the
/// owner resets the history.
#[tauri::command]
pub async fn count_spotify_listens(stats: State<'_, StatsRecorderService>) -> Result<usize> {
    stats.count_spotify_listens()
}

/// Deletes every Spotify-sourced listen, after writing a full history backup (every source, as
/// JSON) to a timestamped file in the app's data folder. Refuses to delete anything, with a clear
/// reason, if that backup could not be written and verified. Every other source (the local
/// library, Mixed In Key, Rekordbox) is left untouched.
#[tauri::command]
pub async fn reset_spotify_listening_history(app: tauri::AppHandle) -> Result<SpotifyResetResult> {
    run_blocking(move || {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| CrateError::Backup(format!("Failed to resolve app data dir: {e}")))?;
        std::fs::create_dir_all(&data_dir).map_err(CrateError::Io)?;
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let backup_path = data_dir.join(format!("spotify-reset-backup-{timestamp}.json"));
        app.state::<StatsRecorderService>()
            .reset_spotify_history(&backup_path)
    })
    .await
}

// ==========================================
// Rekordbox Tracker Commands
// ==========================================

#[tauri::command]
pub async fn rekordbox_detect_status(
    rekordbox: State<'_, RekordboxTrackerService>,
) -> Result<bool> {
    Ok(rekordbox.is_rekordbox_installed())
}

#[tauri::command]
pub async fn rekordbox_sync_history(app: tauri::AppHandle) -> Result<usize> {
    run_blocking(move || {
        app.state::<RekordboxTrackerService>()
            .sync_rekordbox_history()
    })
    .await
}

#[tauri::command]
pub async fn rekordbox_import_history_xml(
    app: tauri::AppHandle,
    xml_content: String,
) -> Result<usize> {
    run_blocking(move || {
        app.state::<RekordboxTrackerService>()
            .import_rekordbox_history_xml(&xml_content)
    })
    .await
}

#[tauri::command]
pub async fn rekordbox_get_sessions(
    rekordbox: State<'_, RekordboxTrackerService>,
) -> Result<Vec<RekordboxSession>> {
    rekordbox.get_sessions()
}

// ==========================================
// Mixed In Key 11 Tracker Commands
// ==========================================

#[tauri::command]
pub async fn mik_detect_status(app: tauri::AppHandle) -> Result<bool> {
    // Runs `pgrep` to look for the Mixed In Key process.
    run_blocking(move || Ok(app.state::<MikTrackerService>().is_mik_running())).await
}

#[tauri::command]
pub async fn mik_tracker_get_enabled(mik: State<'_, MikTrackerService>) -> Result<bool> {
    Ok(mik.is_enabled())
}

/// Turns the (inference-based) Mixed In Key listening tracker on or off, persisted in settings.
#[tauri::command]
pub async fn mik_tracker_set_enabled(
    enabled: bool,
    mik: State<'_, MikTrackerService>,
    settings: State<'_, crate::services::SettingsService>,
) -> Result<()> {
    settings.set_setting(
        crate::services::stats::mik::MIK_TRACKER_SETTING,
        if enabled { "true" } else { "false" },
    )?;
    mik.set_enabled(enabled);
    Ok(())
}

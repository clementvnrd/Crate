use tauri::State;

use crate::error::Result;
use crate::models::stats::{
    BpmBucketItem, HarmonicStatsItem, HeatmapCell, ListenEvent, RekordboxSession, SpotifyAuthState,
    SpotifyImportResult, SpotifyNowPlaying, StatsSummary, TopArtistItem, TopTrackItem,
};
use crate::services::stats::{
    MikTrackerService, RekordboxTrackerService, SpotifyTrackerService, StatsRecorderService,
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
    json_content: String,
    spotify: State<'_, SpotifyTrackerService>,
) -> Result<SpotifyImportResult> {
    spotify.import_streaming_history_json(&json_content)
}

#[tauri::command]
pub async fn sync_spotify_recently_played(
    spotify: State<'_, SpotifyTrackerService>,
) -> Result<usize> {
    spotify.sync_recently_played().await
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
pub async fn rekordbox_sync_history(
    rekordbox: State<'_, RekordboxTrackerService>,
) -> Result<usize> {
    rekordbox.sync_rekordbox_history()
}

#[tauri::command]
pub async fn rekordbox_import_history_xml(
    xml_content: String,
    rekordbox: State<'_, RekordboxTrackerService>,
) -> Result<usize> {
    rekordbox.import_rekordbox_history_xml(&xml_content)
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
pub async fn mik_detect_status(mik: State<'_, MikTrackerService>) -> Result<bool> {
    Ok(mik.is_mik_running())
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

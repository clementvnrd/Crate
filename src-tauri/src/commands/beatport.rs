use crate::error::run_blocking;
use crate::services::beatport::client::{
    BeatportArtistDetail, BeatportAuthState, BeatportChart, BeatportClient, BeatportGenre,
    BeatportPlaylist, BeatportSearchResult, BeatportTrack,
};
use crate::services::beatport::downloader::{BeatportDownloadResult, BeatportDownloader};
use crate::services::{LibraryService, SettingsService};
use std::path::Path;
use tauri::State;

#[tauri::command]
pub fn beatport_get_pkce_auth_url() -> String {
    BeatportClient::generate_pkce_auth_url()
}

// The persisted tokens live in the macOS Keychain, which can block (an access prompt, a locked
// keychain). These commands are async and run on the blocking pool: a synchronous Tauri command
// would run on the main thread and freeze the window while the Keychain answers.
#[tauri::command]
pub async fn beatport_get_persisted_auth() -> Result<Option<BeatportAuthState>, String> {
    run_blocking(|| Ok(BeatportClient::load_persisted_auth()))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn beatport_save_persisted_auth(auth: BeatportAuthState) -> Result<(), String> {
    run_blocking(move || {
        BeatportClient::save_persisted_auth(&auth);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn beatport_clear_persisted_auth() -> Result<(), String> {
    run_blocking(|| {
        BeatportClient::clear_persisted_auth();
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn beatport_login_pkce(code: String) -> Result<BeatportAuthState, String> {
    let client = BeatportClient::new();
    client.login_with_pkce_code(&code).await
}

#[tauri::command]
pub async fn beatport_validate_token(
    token: String,
    refresh_token: Option<String>,
) -> Result<BeatportAuthState, String> {
    let client = BeatportClient::new();
    client
        .validate_token(&token, refresh_token.as_deref())
        .await
}

#[tauri::command]
pub async fn beatport_refresh_token(refresh_token: String) -> Result<BeatportAuthState, String> {
    let client = BeatportClient::new();
    client.refresh_access_token(&refresh_token).await
}

#[tauri::command]
pub async fn beatport_get_genres(token: Option<String>) -> Result<Vec<BeatportGenre>, String> {
    let client = BeatportClient::new();
    client.get_genres(token.as_deref()).await
}

#[tauri::command]
pub async fn beatport_get_featured_charts(
    token: Option<String>,
) -> Result<Vec<BeatportChart>, String> {
    let client = BeatportClient::new();
    client.get_featured_charts(token.as_deref()).await
}

#[tauri::command]
pub async fn beatport_get_top_tracks(
    token: Option<String>,
    genre_id: Option<i64>,
) -> Result<Vec<BeatportTrack>, String> {
    let client = BeatportClient::new();
    client.get_top_tracks(token.as_deref(), genre_id).await
}

#[tauri::command]
pub async fn beatport_get_chart_tracks(
    token: Option<String>,
    chart_id: String,
) -> Result<Vec<BeatportTrack>, String> {
    let client = BeatportClient::new();
    client.get_chart_tracks(token.as_deref(), &chart_id).await
}

#[tauri::command]
pub async fn beatport_get_artist_tracks(
    token: Option<String>,
    artist_id: i64,
) -> Result<Vec<BeatportTrack>, String> {
    let client = BeatportClient::new();
    client.get_artist_tracks(token.as_deref(), artist_id).await
}

#[tauri::command]
pub async fn beatport_get_artist_detail(
    token: Option<String>,
    artist_id: i64,
) -> Result<BeatportArtistDetail, String> {
    let client = BeatportClient::new();
    client.get_artist_detail(token.as_deref(), artist_id).await
}

#[tauri::command]
pub async fn beatport_search(
    token: Option<String>,
    query: String,
) -> Result<BeatportSearchResult, String> {
    let client = BeatportClient::new();
    client.search_catalog_full(token.as_deref(), &query).await
}

#[tauri::command]
pub async fn beatport_get_user_playlists(token: String) -> Result<Vec<BeatportPlaylist>, String> {
    let client = BeatportClient::new();
    client.get_user_playlists(&token).await
}

#[tauri::command]
pub async fn beatport_get_playlist_tracks(
    token: Option<String>,
    playlist_id: String,
) -> Result<Vec<BeatportTrack>, String> {
    let client = BeatportClient::new();
    client
        .get_playlist_tracks(token.as_deref(), &playlist_id)
        .await
}

#[tauri::command]
pub async fn beatport_get_user_favorites(
    token: Option<String>,
) -> Result<Vec<BeatportTrack>, String> {
    let client = BeatportClient::new();
    client.get_user_favorites(token.as_deref(), None).await
}

#[tauri::command]
pub async fn beatport_get_user_purchases(
    token: Option<String>,
) -> Result<Vec<BeatportTrack>, String> {
    let client = BeatportClient::new();
    client.get_user_purchases(token.as_deref()).await
}

#[tauri::command]
pub async fn beatport_download_tracks(
    app: tauri::AppHandle,
    tracks: Vec<BeatportTrack>,
    destination_dir: Option<String>,
    beatportdl_path: Option<String>,
    library: State<'_, LibraryService>,
    settings: State<'_, SettingsService>,
) -> Result<BeatportDownloadResult, String> {
    use tauri::Emitter;

    let dest_str = destination_dir
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            settings
                .get_settings()
                .ok()
                .and_then(|s| s.beatport_download_destination)
                .filter(|s| !s.trim().is_empty())
        })
        .unwrap_or_else(|| "~/Music/My Library/FLAC".to_string());

    let dest_path = BeatportDownloader::expand_path(Path::new(&dest_str));

    let result = BeatportDownloader::download_and_import_tracks(
        tracks,
        &dest_path,
        beatportdl_path.as_deref(),
        Some(&library),
    )
    .await;

    if let Ok(ref res) = result {
        if res.success_count > 0 {
            let _ = app.emit("duplicates-updated", ());
        }
    }

    result
}

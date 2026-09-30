use tauri::{Manager, State};

use crate::error::{run_blocking, Result};
use crate::models::album::{AddAlbumResult, PlayerAlbum, PlayerAlbumTrack};
use crate::services::AlbumService;

/// Walks the folder and reads the tags and cover of every audio file, which takes seconds for an
/// album and about a minute for hundreds of files: off the runtime workers.
#[tauri::command]
pub async fn add_player_album(
    app: tauri::AppHandle,
    folder_path: String,
) -> Result<AddAlbumResult> {
    run_blocking(move || {
        app.state::<AlbumService>()
            .add_album_from_folder(&folder_path)
    })
    .await
}

#[tauri::command]
pub async fn get_player_albums(album_service: State<'_, AlbumService>) -> Result<Vec<PlayerAlbum>> {
    album_service.get_albums()
}

#[tauri::command]
pub async fn get_player_album_tracks(
    album_id: String,
    album_service: State<'_, AlbumService>,
) -> Result<Vec<PlayerAlbumTrack>> {
    album_service.get_album_tracks(&album_id)
}

#[tauri::command]
pub async fn remove_player_album(
    album_id: String,
    album_service: State<'_, AlbumService>,
) -> Result<()> {
    album_service.remove_album(&album_id)
}

use tauri::State;

use crate::error::Result;
use crate::models::album::{AddAlbumResult, PlayerAlbum, PlayerAlbumTrack};
use crate::services::AlbumService;

#[tauri::command]
pub async fn add_player_album(
    folder_path: String,
    album_service: State<'_, AlbumService>,
) -> Result<AddAlbumResult> {
    album_service.add_album_from_folder(&folder_path)
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

use serde::{Deserialize, Serialize};

/// Represents an imported album folder in Crate Player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerAlbum {
    pub id: String,
    pub folder_path: String,
    pub title: String,
    pub artist: String,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub artwork_path: Option<String>,
    pub track_count: i32,
    pub total_duration_ms: i64,
    pub created_at: String,
}

/// Represents an individual track within a PlayerAlbum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerAlbumTrack {
    pub id: String,
    pub album_id: String,
    pub file_path: String,
    pub track_number: Option<i32>,
    pub title: String,
    pub artist: String,
    pub duration_ms: i64,
    pub format: String,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub energy: Option<i32>,
    pub artwork_path: Option<String>,
}

/// Result returned when adding or scanning an album
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAlbumResult {
    pub album: PlayerAlbum,
    pub tracks: Vec<PlayerAlbumTrack>,
}

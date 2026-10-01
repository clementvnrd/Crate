use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single track listening event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenEvent {
    pub id: String,
    pub source: String,
    pub track_id: Option<String>,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: u64,
    pub played_ms: u64,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub energy: Option<i32>,
    pub format: Option<String>,
    pub artwork_url: Option<String>,
    pub played_at: String, // ISO8601 UTC
    pub session_id: Option<String>,
    pub metadata_json: Option<String>,
}

/// Spotify Authentication & connection state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpotifyAuthState {
    pub is_connected: bool,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub expires_at: Option<i64>,
}

/// Spotify currently playing track info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpotifyNowPlaying {
    pub is_playing: bool,
    pub track_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub progress_ms: Option<u64>,
    pub artwork_url: Option<String>,
    pub device_name: Option<String>,
}

/// High-level listening statistics summary
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatsSummary {
    pub total_minutes: u64,
    pub today_minutes: u64,
    pub week_minutes: u64,
    pub month_minutes: u64,
    pub total_plays: usize,
    pub source_breakdown: HashMap<String, u64>,
}

/// Top listened track aggregation item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopTrackItem {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub artwork_url: Option<String>,
    pub plays: usize,
    pub total_minutes: u64,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub energy: Option<i32>,
    pub sources: Vec<String>,
}

/// Top listened artist aggregation item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopArtistItem {
    pub artist: String,
    pub plays: usize,
    pub total_minutes: u64,
    pub top_track: Option<String>,
    pub artwork_url: Option<String>,
}

/// Harmonic key listening breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarmonicStatsItem {
    pub key: String,
    pub plays: usize,
    pub total_minutes: u64,
    pub percentage: f64,
}

/// BPM range bucket distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BpmBucketItem {
    pub bpm_range: String,
    pub count: usize,
    pub total_minutes: u64,
}

/// Hourly heatmap matrix cell (7 days x 24 hours)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapCell {
    pub day_of_week: u8, // 0 = Sunday, 1 = Monday, ..., 6 = Saturday
    pub hour_of_day: u8, // 0 to 23
    pub minutes: u64,
    pub plays: usize,
}

/// Result summary of a Spotify archive JSON import
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpotifyImportResult {
    pub imported_count: usize,
    pub skipped_count: usize,
    pub total_minutes: u64,
}

/// Result of resetting the Spotify-sourced part of the listening history: how many listens were
/// removed, and where the pre-deletion safety backup was written.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyResetResult {
    pub deleted_count: usize,
    pub backup_path: String,
}

/// Rekordbox DJ performance session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RekordboxSession {
    pub id: String,
    pub session_name: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub total_tracks: usize,
    pub total_played_ms: u64,
}

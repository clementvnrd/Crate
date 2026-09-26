use crate::services::beatport::client::BeatportTrack;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeScoreBreakdown {
    pub title_score: i32,
    pub artist_score: i32,
    pub duration_score: i32,
    pub bpm_score: i32,
    pub key_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeMatch {
    pub track_id: String,
    pub file_path: String,
    pub current_format: String,
    pub current_bitrate: Option<i32>,
    pub current_sample_rate: Option<i32>,
    pub current_duration_ms: i64,
    pub current_bpm: Option<f64>,
    pub current_key: Option<String>,
    pub current_energy: Option<i32>,
    pub current_artwork_path: Option<String>,
    pub current_file_size_bytes: u64,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub beatport_track: BeatportTrack,
    pub confidence_score: i32,
    pub score_breakdown: UpgradeScoreBreakdown,
    #[serde(default)]
    pub alternative_candidates: Vec<BeatportTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeScanResult {
    pub matches: Vec<UpgradeMatch>,
    pub total_scanned: usize,
    pub total_eligible_mp3s: usize,
    pub potential_upgrades_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeReplacementResult {
    pub success_count: usize,
    pub failed_count: usize,
    pub replaced_tracks: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeCountInfo {
    pub match_count: usize,
    pub eligible_mp3_count: usize,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateMatchType {
    ExactHash,
    Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateTrackInfo {
    pub id: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub label: Option<String>,
    pub duration_ms: i64,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub energy: Option<i32>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub format: String,
    pub file_path: String,
    pub file_hash: Option<String>,
    pub file_size_bytes: u64,
    pub cue_count: usize,
    pub rating: i32,
    pub play_count: i32,
    pub artwork_path: Option<String>,
    pub date_added: String,
    pub quality_score: i32,
    pub recommended_keep: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub id: String,
    pub match_type: DuplicateMatchType,
    pub tracks: Vec<DuplicateTrackInfo>,
    pub reclaimable_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateScanResult {
    pub groups: Vec<DuplicateGroup>,
    pub total_duplicate_tracks: usize,
    pub total_groups: usize,
    pub total_reclaimable_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCountInfo {
    pub group_count: usize,
    pub track_count: usize,
    pub reclaimable_bytes: u64,
}

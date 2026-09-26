use chrono::Utc;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

use crate::error::{CrateError, Result};
use crate::models::stats::ListenEvent;
use crate::services::library::mik::MikService;
use crate::services::stats::recorder::StatsRecorderService;
use lofty::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct MikTrackMeta {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: u64,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub energy: Option<i32>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct MikAccumulatorState {
    file_path: Option<PathBuf>,
    title: String,
    artist: String,
    album: Option<String>,
    duration_ms: u64,
    bpm: Option<f64>,
    key: Option<String>,
    energy: Option<i32>,
    format: Option<String>,
    artwork_url: Option<String>,
    accumulated_ms: u64,
    last_poll: Option<Instant>,
    started_at: String,
    recorded: bool,
}

/// Settings key of the opt-in Mixed In Key listening tracker.
pub const MIK_TRACKER_SETTING: &str = "stats_mik_tracker_enabled";

#[derive(Clone)]
pub struct MikTrackerService {
    recorder: Arc<StatsRecorderService>,
    accumulator: Arc<Mutex<MikAccumulatorState>>,
    /// Off by default: Mixed In Key exposes no playback state, so the tracker can only infer a
    /// "listen" from a file being open, which also happens while Mixed In Key analyses a file.
    enabled: Arc<AtomicBool>,
}

impl MikTrackerService {
    pub fn new(recorder: Arc<StatsRecorderService>, enabled: bool) -> Self {
        Self {
            recorder,
            accumulator: Arc::new(Mutex::new(MikAccumulatorState::default())),
            enabled: Arc::new(AtomicBool::new(enabled)),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            if let Ok(mut acc) = self.accumulator.lock() {
                *acc = MikAccumulatorState::default();
            }
        }
    }

    /// Returns PIDs of running Mixed In Key processes on macOS.
    pub fn get_mik_pids() -> Vec<String> {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("pgrep")
                .args(["-f", "Mixed In Key"])
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    return stdout
                        .lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect();
                }
            }
            Vec::new()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Vec::new()
        }
    }

    /// Checks if Mixed In Key 11 is currently running on the system.
    pub fn is_mik_running(&self) -> bool {
        !Self::get_mik_pids().is_empty()
    }

    /// Scans for open audio files loaded in Mixed In Key 11 processes on macOS.
    pub fn find_active_audio_files() -> Vec<PathBuf> {
        let pids = Self::get_mik_pids();
        if pids.is_empty() {
            return Vec::new();
        }

        #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
        let mut files = Vec::new();

        #[cfg(target_os = "macos")]
        {
            for pid in &pids {
                if let Ok(output) = std::process::Command::new("lsof")
                    .args(["-p", pid, "-Fn"])
                    .output()
                {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        for line in stdout.lines() {
                            let raw_path = if let Some(stripped) = line.strip_prefix('n') {
                                stripped.trim()
                            } else {
                                line.trim()
                            };

                            if raw_path.is_empty() {
                                continue;
                            }

                            let path = PathBuf::from(raw_path);
                            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                let ext_lower = ext.to_lowercase();
                                if matches!(
                                    ext_lower.as_str(),
                                    "mp3"
                                        | "flac"
                                        | "wav"
                                        | "aiff"
                                        | "aif"
                                        | "m4a"
                                        | "aac"
                                        | "ogg"
                                        | "alac"
                                        | "wma"
                                ) && path.exists()
                                    && !files.contains(&path)
                                {
                                    files.push(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        files
    }

    /// Extracts rich metadata and Mixed In Key cue/energy/key tags from an audio file.
    pub fn extract_metadata(path: &Path) -> MikTrackMeta {
        let default_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Track")
            .to_string();

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        if let Some(tagged_file) = MikService::read_metadata_lenient(path) {
            let properties = tagged_file.properties();
            let duration_ms = properties.duration().as_millis() as u64;

            let mut title = default_title;
            let mut artist = "Unknown Artist".to_string();
            let mut album = None;

            if let Some(tag) = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag())
            {
                if let Some(t) = tag.title() {
                    let clean = t.trim();
                    if !clean.is_empty() {
                        title = clean.to_string();
                    }
                }
                if let Some(a) = tag.artist() {
                    let clean = a.trim();
                    if !clean.is_empty() {
                        artist = clean.to_string();
                    }
                }
                album = tag.album().map(|s| s.to_string());
            }

            let mik_data = MikService::extract_analysis_data(&tagged_file, "");

            MikTrackMeta {
                title,
                artist,
                album,
                duration_ms,
                bpm: mik_data.bpm,
                key: mik_data.key,
                energy: mik_data.energy,
                format: ext,
            }
        } else {
            MikTrackMeta {
                title: default_title,
                artist: "Unknown Artist".to_string(),
                album: None,
                duration_ms: 0,
                bpm: None,
                key: None,
                energy: None,
                format: ext,
            }
        }
    }

    /// Background polling tick: tracks playback/loading in Mixed In Key 11 in real time.
    pub async fn poll_tick(&self) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }
        let pids = Self::get_mik_pids();
        if pids.is_empty() {
            let mut acc = self
                .accumulator
                .lock()
                .map_err(|_| CrateError::LockPoisoned)?;
            if acc.accumulated_ms >= 1_000 && !acc.recorded && !acc.title.is_empty() {
                let event = ListenEvent {
                    id: Uuid::new_v4().to_string(),
                    source: "mixed_in_key".to_string(),
                    track_id: None,
                    title: acc.title.clone(),
                    artist: acc.artist.clone(),
                    album: acc.album.clone(),
                    duration_ms: acc.duration_ms,
                    played_ms: acc.accumulated_ms,
                    bpm: acc.bpm,
                    key: acc.key.clone(),
                    energy: acc.energy,
                    format: acc.format.clone(),
                    artwork_url: acc.artwork_url.clone(),
                    played_at: acc.started_at.clone(),
                    session_id: None,
                    metadata_json: None,
                };
                let _ = self.recorder.record_listen_event(&event);
            }
            *acc = MikAccumulatorState::default();
            return Ok(());
        }

        let active_files = Self::find_active_audio_files();
        let current_file_opt = active_files.into_iter().next();

        let mut acc = self
            .accumulator
            .lock()
            .map_err(|_| CrateError::LockPoisoned)?;

        match current_file_opt {
            Some(current_file) => {
                let is_same_file = acc.file_path.as_ref() == Some(&current_file);

                if !is_same_file {
                    // Flush previously active track if >= 1s and not yet recorded
                    if acc.accumulated_ms >= 1_000 && !acc.recorded && !acc.title.is_empty() {
                        let event = ListenEvent {
                            id: Uuid::new_v4().to_string(),
                            source: "mixed_in_key".to_string(),
                            track_id: None,
                            title: acc.title.clone(),
                            artist: acc.artist.clone(),
                            album: acc.album.clone(),
                            duration_ms: acc.duration_ms,
                            played_ms: acc.accumulated_ms,
                            bpm: acc.bpm,
                            key: acc.key.clone(),
                            energy: acc.energy,
                            format: acc.format.clone(),
                            artwork_url: acc.artwork_url.clone(),
                            played_at: acc.started_at.clone(),
                            session_id: None,
                            metadata_json: None,
                        };
                        let _ = self.recorder.record_listen_event(&event);
                    }

                    // Extract metadata for the newly loaded file
                    let meta = Self::extract_metadata(&current_file);
                    acc.file_path = Some(current_file);
                    acc.title = meta.title;
                    acc.artist = meta.artist;
                    acc.album = meta.album;
                    acc.duration_ms = meta.duration_ms;
                    acc.bpm = meta.bpm;
                    acc.key = meta.key;
                    acc.energy = meta.energy;
                    acc.format = meta.format;
                    acc.artwork_url = None;
                    acc.accumulated_ms = 0;
                    acc.last_poll = Some(Instant::now());
                    acc.started_at = Utc::now().to_rfc3339();
                    acc.recorded = false;
                } else {
                    // Same file active: accumulate listening time
                    if let Some(last) = acc.last_poll {
                        let elapsed_ms = last.elapsed().as_millis() as u64;
                        // Cap single tick delta to 10 seconds
                        acc.accumulated_ms += elapsed_ms.min(10_000);
                    }
                    acc.last_poll = Some(Instant::now());

                    // Check if stream threshold reached (>= 30s)
                    if acc.accumulated_ms >= 30_000 && !acc.recorded {
                        let event = ListenEvent {
                            id: Uuid::new_v4().to_string(),
                            source: "mixed_in_key".to_string(),
                            track_id: None,
                            title: acc.title.clone(),
                            artist: acc.artist.clone(),
                            album: acc.album.clone(),
                            duration_ms: acc.duration_ms,
                            played_ms: acc.accumulated_ms,
                            bpm: acc.bpm,
                            key: acc.key.clone(),
                            energy: acc.energy,
                            format: acc.format.clone(),
                            artwork_url: acc.artwork_url.clone(),
                            played_at: acc.started_at.clone(),
                            session_id: None,
                            metadata_json: None,
                        };
                        if let Ok(true) = self.recorder.record_listen_event(&event) {
                            acc.recorded = true;
                            log::info!(
                                "Recorded live Mixed In Key 11 listen: '{}' by '{}'",
                                acc.title,
                                acc.artist
                            );
                        }
                    }
                }
            }
            None => {
                // No active audio file in Mixed In Key
                if acc.accumulated_ms >= 1_000 && !acc.recorded && !acc.title.is_empty() {
                    let event = ListenEvent {
                        id: Uuid::new_v4().to_string(),
                        source: "mixed_in_key".to_string(),
                        track_id: None,
                        title: acc.title.clone(),
                        artist: acc.artist.clone(),
                        album: acc.album.clone(),
                        duration_ms: acc.duration_ms,
                        played_ms: acc.accumulated_ms,
                        bpm: acc.bpm,
                        key: acc.key.clone(),
                        energy: acc.energy,
                        format: acc.format.clone(),
                        artwork_url: acc.artwork_url.clone(),
                        played_at: acc.started_at.clone(),
                        session_id: None,
                        metadata_json: None,
                    };
                    let _ = self.recorder.record_listen_event(&event);
                }
                *acc = MikAccumulatorState::default();
            }
        }

        Ok(())
    }
}

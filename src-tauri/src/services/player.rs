use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

use crate::models::stats::ListenEvent;
use crate::services::stats::recorder::StatsRecorderService;

#[derive(Debug, Clone)]
pub struct TrackPlayingContext {
    pub track_id: Option<String>,
    pub source: String, // 'crate_local', 'crate_beatport', 'crate_standalone'
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: u64,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub energy: Option<i32>,
    pub format: Option<String>,
    pub artwork_url: Option<String>,
    pub started_at: String,
    pub start_instant: Instant,
    pub recorded: bool,
}

pub struct PlayerTrackerService {
    recorder: Arc<StatsRecorderService>,
    current_track: Arc<Mutex<Option<TrackPlayingContext>>>,
}

impl PlayerTrackerService {
    pub fn new(recorder: Arc<StatsRecorderService>) -> Self {
        Self {
            recorder,
            current_track: Arc::new(Mutex::new(None)),
        }
    }

    /// Called when starting playback of a track
    pub fn on_track_started(&self, ctx: TrackPlayingContext) {
        let mut lock = self.current_track.lock().unwrap();
        // Flush previously playing track if >= 1s and not yet recorded
        if let Some(ref prev) = *lock {
            let elapsed_ms = prev.start_instant.elapsed().as_millis() as u64;
            if elapsed_ms >= 1_000 && !prev.recorded {
                let event = ListenEvent {
                    id: Uuid::new_v4().to_string(),
                    source: prev.source.clone(),
                    track_id: prev.track_id.clone(),
                    title: prev.title.clone(),
                    artist: prev.artist.clone(),
                    album: prev.album.clone(),
                    duration_ms: prev.duration_ms,
                    played_ms: elapsed_ms,
                    bpm: prev.bpm,
                    key: prev.key.clone(),
                    energy: prev.energy,
                    format: prev.format.clone(),
                    artwork_url: prev.artwork_url.clone(),
                    played_at: prev.started_at.clone(),
                    session_id: None,
                    metadata_json: None,
                };
                let _ = self.recorder.record_listen_event(&event);
            }
        }
        *lock = Some(ctx);
    }

    /// Checks playback duration and commits a listen event as soon as the 30-second threshold is reached.
    pub fn check_and_record_if_due(&self, current_pos_ms: u64) {
        let mut lock = self.current_track.lock().unwrap();
        if let Some(ref mut ctx) = *lock {
            let elapsed_ms = ctx.start_instant.elapsed().as_millis() as u64;
            let effective_played_ms = current_pos_ms.max(elapsed_ms);

            if effective_played_ms >= 30_000 && !ctx.recorded {
                let event = ListenEvent {
                    id: Uuid::new_v4().to_string(),
                    source: ctx.source.clone(),
                    track_id: ctx.track_id.clone(),
                    title: ctx.title.clone(),
                    artist: ctx.artist.clone(),
                    album: ctx.album.clone(),
                    duration_ms: ctx.duration_ms,
                    played_ms: effective_played_ms,
                    bpm: ctx.bpm,
                    key: ctx.key.clone(),
                    energy: ctx.energy,
                    format: ctx.format.clone(),
                    artwork_url: ctx.artwork_url.clone(),
                    played_at: ctx.started_at.clone(),
                    session_id: None,
                    metadata_json: None,
                };
                if let Ok(true) = self.recorder.record_listen_event(&event) {
                    ctx.recorded = true;
                    log::info!("Recorded listen for '{}' by '{}' on {}", ctx.title, ctx.artist, ctx.source);
                }
            }
        }
    }

    /// Called when playback is stopped or paused
    pub fn on_playback_stopped(&self) {
        let mut lock = self.current_track.lock().unwrap();
        if let Some(ref mut ctx) = *lock {
            let elapsed_ms = ctx.start_instant.elapsed().as_millis() as u64;
            if elapsed_ms >= 1_000 && !ctx.recorded {
                let event = ListenEvent {
                    id: Uuid::new_v4().to_string(),
                    source: ctx.source.clone(),
                    track_id: ctx.track_id.clone(),
                    title: ctx.title.clone(),
                    artist: ctx.artist.clone(),
                    album: ctx.album.clone(),
                    duration_ms: ctx.duration_ms,
                    played_ms: elapsed_ms,
                    bpm: ctx.bpm,
                    key: ctx.key.clone(),
                    energy: ctx.energy,
                    format: ctx.format.clone(),
                    artwork_url: ctx.artwork_url.clone(),
                    played_at: ctx.started_at.clone(),
                    session_id: None,
                    metadata_json: None,
                };
                let _ = self.recorder.record_listen_event(&event);
                ctx.recorded = true;
            }
        }
    }
}

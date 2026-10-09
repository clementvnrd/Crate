use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use stratum_dsp::{analyze_audio, AnalysisConfig};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::error::{CrateError, Result};
use crate::models::{Tag, Track};
use crate::services::beatgrid::{self, TrackBeatGrid};
use crate::services::cloud_sync::pipeline::{buckets, dirty};

/// Result of analyzing a single track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub track_id: String,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

/// What one stratum-dsp pass yields for a track.
struct AudioAnalysis {
    bpm: Option<f64>,
    key: Option<String>,
    grid: Option<TrackBeatGrid>,
}

/// Status of an analysis operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStatus {
    Pending,
    Analyzing,
    Completed,
    Failed,
    Cancelled,
}

/// Per-track analysis event for real-time UI updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackAnalysisEvent {
    pub track_id: String,
    pub state: AnalysisStatus,
    pub result: Option<AnalysisResult>,
    pub updated_track: Option<Track>,
    pub error: Option<String>,
}

/// State for a single track's analysis task
struct TrackAnalysisTask {
    cancel_token: CancellationToken,
    #[allow(dead_code)]
    handle: JoinHandle<()>,
}

/// How many tracks are analysed at the same time. Each analysis decodes the whole file into
/// memory (about 60 MB for a five-minute track) and keeps a core busy, so the limit is half the
/// cores, between 1 and 4. Without a limit, selecting the library ran one decode per track at once.
fn analysis_concurrency() -> usize {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);
    (cores / 2).clamp(1, 4)
}

/// Bounds how many analyses run at once. A track waits for its turn while still "pending".
#[derive(Clone)]
struct AnalysisLimiter {
    permits: Arc<Semaphore>,
}

impl AnalysisLimiter {
    fn new(max_concurrent: usize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// A permit for one running analysis, or `None` if the track was cancelled while it waited.
    async fn acquire(&self, cancel: &CancellationToken) -> Option<OwnedSemaphorePermit> {
        tokio::select! {
            permit = self.permits.clone().acquire_owned() => permit.ok(),
            _ = cancel.cancelled() => None,
        }
    }
}

pub struct AnalysisService {
    conn: Arc<Mutex<Connection>>,
    tasks: Arc<Mutex<HashMap<String, TrackAnalysisTask>>>,
    limiter: AnalysisLimiter,
}

impl AnalysisService {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            tasks: Arc::new(Mutex::new(HashMap::new())),
            limiter: AnalysisLimiter::new(analysis_concurrency()),
        }
    }

    /// Cancel analysis for a specific track
    pub fn cancel_track_analysis(&self, track_id: &str) -> Result<bool> {
        let mut tasks = self.tasks.lock().map_err(|_| CrateError::LockPoisoned)?;
        if let Some(task) = tasks.remove(track_id) {
            task.cancel_token.cancel();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Cancel all running analysis tasks
    pub fn cancel_all_analysis(&self) -> Result<()> {
        let mut tasks = self.tasks.lock().map_err(|_| CrateError::LockPoisoned)?;
        for (_, task) in tasks.drain() {
            task.cancel_token.cancel();
        }
        Ok(())
    }

    /// Analyze multiple tracks with per-track events (async, non-blocking)
    pub async fn analyze_tracks_async(
        &self,
        app_handle: AppHandle,
        track_ids: Vec<String>,
    ) -> Result<()> {
        for track_id in track_ids {
            let cancel_token = CancellationToken::new();
            let conn = self.conn.clone();
            let app = app_handle.clone();
            let tid = track_id.clone();
            let token = cancel_token.clone();
            let tasks = self.tasks.clone();
            let limiter = self.limiter.clone();

            // Emit "pending" event immediately
            let _ = app.emit(
                "analysis-track-event",
                TrackAnalysisEvent {
                    track_id: tid.clone(),
                    state: AnalysisStatus::Pending,
                    result: None,
                    updated_track: None,
                    error: None,
                },
            );

            let handle = tauri::async_runtime::spawn(async move {
                Self::analyze_single_track_task(conn, app, tid, token, tasks, limiter).await;
            });

            // Store task for potential cancellation
            let mut tasks_guard = self.tasks.lock().map_err(|_| CrateError::LockPoisoned)?;
            tasks_guard.insert(
                track_id.clone(),
                TrackAnalysisTask {
                    cancel_token,
                    handle,
                },
            );
        }

        Ok(())
    }

    /// Single track analysis task - runs in its own Tokio task
    async fn analyze_single_track_task(
        conn: Arc<Mutex<Connection>>,
        app: AppHandle,
        track_id: String,
        cancel_token: CancellationToken,
        tasks: Arc<Mutex<HashMap<String, TrackAnalysisTask>>>,
        limiter: AnalysisLimiter,
    ) {
        // Check if already cancelled before starting
        if cancel_token.is_cancelled() {
            Self::finish_cancelled(&app, &track_id, &tasks);
            return;
        }

        // Wait for a free slot: the track stays "pending" until then (or until it is cancelled).
        let Some(permit) = limiter.acquire(&cancel_token).await else {
            Self::finish_cancelled(&app, &track_id, &tasks);
            return;
        };

        // Emit "analyzing" status
        let _ = app.emit(
            "analysis-track-event",
            TrackAnalysisEvent {
                track_id: track_id.clone(),
                state: AnalysisStatus::Analyzing,
                result: None,
                updated_track: None,
                error: None,
            },
        );

        // Run analysis on blocking thread pool with cancellation
        let conn_clone = conn.clone();
        let tid_clone = track_id.clone();
        let token_clone = cancel_token.clone();

        let result = tokio::task::spawn_blocking(move || {
            Self::analyze_track_with_cancellation(&conn_clone, &tid_clone, &token_clone)
        })
        .await;
        // Free the slot as soon as the decode is over, before the events and the cleanup.
        drop(permit);

        // Check if cancelled during analysis
        if cancel_token.is_cancelled() {
            Self::finish_cancelled(&app, &track_id, &tasks);
            return;
        }

        // Handle result
        match result {
            Ok(Ok((analysis_result, updated_track))) => {
                let _ = app.emit(
                    "analysis-track-event",
                    TrackAnalysisEvent {
                        track_id: track_id.clone(),
                        state: AnalysisStatus::Completed,
                        result: Some(analysis_result),
                        updated_track,
                        error: None,
                    },
                );
                let _ = app.emit("duplicates-updated", ());
            }
            Ok(Err(e)) => {
                let _ = app.emit(
                    "analysis-track-event",
                    TrackAnalysisEvent {
                        track_id: track_id.clone(),
                        state: AnalysisStatus::Failed,
                        result: None,
                        updated_track: None,
                        error: Some(e.to_string()),
                    },
                );
            }
            Err(e) => {
                let _ = app.emit(
                    "analysis-track-event",
                    TrackAnalysisEvent {
                        track_id: track_id.clone(),
                        state: AnalysisStatus::Failed,
                        result: None,
                        updated_track: None,
                        error: Some(format!("Task panicked: {e}")),
                    },
                );
            }
        }

        // Clean up task from map
        if let Ok(mut t) = tasks.lock() {
            t.remove(&track_id);
        }
    }

    /// Reports a cancelled track to the UI and forgets its task.
    fn finish_cancelled(
        app: &AppHandle,
        track_id: &str,
        tasks: &Arc<Mutex<HashMap<String, TrackAnalysisTask>>>,
    ) {
        let _ = app.emit(
            "analysis-track-event",
            TrackAnalysisEvent {
                track_id: track_id.to_string(),
                state: AnalysisStatus::Cancelled,
                result: None,
                updated_track: None,
                error: None,
            },
        );
        if let Ok(mut t) = tasks.lock() {
            t.remove(track_id);
        }
    }

    /// Analyze a track with cancellation support - runs on blocking thread
    fn analyze_track_with_cancellation(
        conn: &Arc<Mutex<Connection>>,
        track_id: &str,
        cancel_token: &CancellationToken,
    ) -> Result<(AnalysisResult, Option<Track>)> {
        // Get track from database
        let track = Self::get_track_static(conn, track_id)?;
        let file_path = Path::new(&track.file_path);

        if !file_path.exists() {
            return Ok((
                AnalysisResult {
                    track_id: track_id.to_string(),
                    bpm: None,
                    key: None,
                    success: false,
                    error: Some(format!("File not found: {}", track.file_path)),
                },
                None,
            ));
        }

        // Check cancellation before starting heavy work
        if cancel_token.is_cancelled() {
            return Err(CrateError::Analysis("Cancelled".to_string()));
        }

        // Check if track is already analyzed by Mixed In Key
        if track.analysis_source.as_deref() == Some("mixed_in_key")
            && track.bpm.is_some()
            && track.key.is_some()
        {
            log::info!(
                "Track {} already analyzed by Mixed In Key, skipping stratum-dsp analysis",
                track_id
            );
            return Ok((
                AnalysisResult {
                    track_id: track_id.to_string(),
                    bpm: track.bpm,
                    key: track.key.clone(),
                    success: true,
                    error: None,
                },
                Some(track),
            ));
        }

        // Check if audio file has Mixed In Key / file tag analysis on disk
        if let Some(tf) = crate::services::library::MikService::read_metadata_lenient(file_path) {
            let mik_data =
                crate::services::library::MikService::extract_analysis_data(&tf, track_id);
            if mik_data.is_mik && (mik_data.bpm.is_some() || mik_data.key.is_some()) {
                let conn_guard = conn.lock().map_err(|_| CrateError::LockPoisoned)?;
                let updated = crate::services::library::MikService::sync_track_from_file(
                    &conn_guard,
                    &track,
                )?;
                log::info!("Track {} synced with Mixed In Key tags from file", track_id);
                return Ok((
                    AnalysisResult {
                        track_id: track_id.to_string(),
                        bpm: updated.bpm,
                        key: updated.key.clone(),
                        success: true,
                        error: None,
                    },
                    Some(updated),
                ));
            }
        }

        // Analyze the audio file with cancellation checks
        match Self::analyze_audio_file_with_cancellation(file_path, cancel_token) {
            Ok(AudioAnalysis { bpm, key, grid }) => {
                // Check cancellation before saving
                if cancel_token.is_cancelled() {
                    return Err(CrateError::Analysis("Cancelled".to_string()));
                }

                // Update the database
                Self::update_track_analysis_static(
                    conn,
                    track_id,
                    bpm,
                    key.as_deref(),
                    grid.as_ref(),
                )?;

                // Get updated track
                let updated_track = Self::get_track_static(conn, track_id).ok();

                Ok((
                    AnalysisResult {
                        track_id: track_id.to_string(),
                        bpm,
                        key,
                        success: true,
                        error: None,
                    },
                    updated_track,
                ))
            }
            Err(e) if e.to_string().contains("Cancelled") => {
                Err(CrateError::Analysis("Cancelled".to_string()))
            }
            Err(e) => Ok((
                AnalysisResult {
                    track_id: track_id.to_string(),
                    bpm: None,
                    key: None,
                    success: false,
                    error: Some(e.to_string()),
                },
                None,
            )),
        }
    }

    /// Analyze an audio file for BPM, key and beat grid with cancellation support
    fn analyze_audio_file_with_cancellation(
        path: &Path,
        cancel_token: &CancellationToken,
    ) -> Result<AudioAnalysis> {
        // Decode audio to mono f32 samples with cancellation checks
        let (samples, sample_rate) = Self::decode_audio_with_cancellation(path, cancel_token)?;

        if samples.is_empty() {
            return Err(CrateError::Analysis("No audio samples found".to_string()));
        }

        // Check cancellation before analysis
        if cancel_token.is_cancelled() {
            return Err(CrateError::Analysis("Cancelled".to_string()));
        }

        Self::analyze_samples(&samples, sample_rate)
    }

    /// BPM, key and beat grid of decoded mono samples, in one analysis pass.
    fn analyze_samples(samples: &[f32], sample_rate: u32) -> Result<AudioAnalysis> {
        let result = analyze_audio(samples, sample_rate, AnalysisConfig::default())
            .map_err(|e| CrateError::Analysis(format!("Analysis failed: {e}")))?;

        // The grid is found on the same samples, near the tempo this pass estimated (stratum-dsp's
        // own beat list is laid at its rounded tempo, so it cannot give the grid).
        let grid = beatgrid::detect_beat_grid(samples, sample_rate, result.bpm as f64);

        Ok(AudioAnalysis {
            // Round BPM to nearest integer (most tracks are produced at whole BPMs). The grid keeps
            // its own BPM with decimals; this one is the displayed and exported value.
            bpm: Some((result.bpm as f64).round()),
            key: Some(result.key.name().to_string()),
            grid,
        })
    }

    /// Decode audio file to mono f32 samples with cancellation checks
    fn decode_audio_with_cancellation(
        path: &Path,
        cancel_token: &CancellationToken,
    ) -> Result<(Vec<f32>, u32)> {
        let file = File::open(path).map_err(|e| CrateError::Analysis(e.to_string()))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();
        let decoder_opts = DecoderOptions::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| CrateError::Analysis(format!("Failed to probe audio: {e}")))?;

        let mut format = probed.format;

        let track = format
            .default_track()
            .ok_or_else(|| CrateError::Analysis("No audio track found".to_string()))?;

        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or_else(|| CrateError::Analysis("Unknown sample rate".to_string()))?;

        let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

        let track_id = track.id;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)
            .map_err(|e| CrateError::Analysis(format!("Failed to create decoder: {e}")))?;

        let mut samples: Vec<f32> = Vec::new();
        let mut packet_count: u32 = 0;

        // Decode all packets with cancellation checks
        loop {
            // Check cancellation every 100 packets
            packet_count += 1;
            if packet_count.is_multiple_of(100) && cancel_token.is_cancelled() {
                return Err(CrateError::Analysis("Cancelled".to_string()));
            }

            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => {
                    log::warn!("Error reading packet: {e}");
                    break;
                }
            };

            // Skip packets from other tracks
            if packet.track_id() != track_id {
                continue;
            }

            let decoded = match decoder.decode(&packet) {
                Ok(d) => d,
                Err(e) => {
                    log::warn!("Error decoding packet: {e}");
                    continue;
                }
            };

            // Convert to f32 samples
            let spec = *decoded.spec();
            let num_frames = decoded.frames();

            let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);

            let interleaved = sample_buf.samples();

            // Convert to mono by averaging channels
            for chunk in interleaved.chunks(channels) {
                let mono: f32 = chunk.iter().sum::<f32>() / channels as f32;
                samples.push(mono);
            }
        }

        Ok((samples, sample_rate))
    }

    /// Recomputes only the beat grid of a track from its audio file, for tracks whose regular
    /// analysis is skipped (Mixed In Key) or predates the grid. Nothing else is written: BPM, key,
    /// energy and `analysis_source` stay as they are, and Mixed In Key's database is never opened.
    /// Waits for a free analysis slot like a regular analysis.
    pub async fn analyze_beat_grid(&self, track_id: String) -> Result<Option<TrackBeatGrid>> {
        let cancel_token = CancellationToken::new();
        let _permit = self
            .limiter
            .acquire(&cancel_token)
            .await
            .ok_or_else(|| CrateError::Analysis("Cancelled".to_string()))?;
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            Self::analyze_beat_grid_blocking(&conn, &track_id, &cancel_token)
        })
        .await
        .map_err(|e| CrateError::Analysis(format!("Task panicked: {e}")))?
    }

    fn analyze_beat_grid_blocking(
        conn: &Arc<Mutex<Connection>>,
        track_id: &str,
        cancel_token: &CancellationToken,
    ) -> Result<Option<TrackBeatGrid>> {
        let file_path: String = {
            let conn = conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            conn.query_row(
                "SELECT file_path FROM tracks WHERE id = ?1",
                [track_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| CrateError::TrackNotFound(track_id.to_string()))?
        };
        let path = Path::new(&file_path);
        if !path.exists() {
            return Err(CrateError::Analysis(format!("File not found: {file_path}")));
        }
        let analysis = Self::analyze_audio_file_with_cancellation(path, cancel_token)?;
        let conn = conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        beatgrid::store_beat_grid(&conn, track_id, analysis.grid.as_ref())?;
        Ok(analysis.grid)
    }

    /// The stored beat grid of a track; `None` when it has none.
    pub fn get_beat_grid(&self, track_id: &str) -> Result<Option<TrackBeatGrid>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        beatgrid::load_beat_grid(&conn, track_id)
    }

    /// Static version of update_track_analysis for use in blocking context. The beat grid is
    /// written in the same statement (cleared when the track has no pulse) but is not synced.
    fn update_track_analysis_static(
        conn: &Arc<Mutex<Connection>>,
        track_id: &str,
        bpm: Option<f64>,
        key: Option<&str>,
        grid: Option<&TrackBeatGrid>,
    ) -> Result<()> {
        let conn = conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let now = chrono::Utc::now().to_rfc3339();
        let hlc = dirty::next_hlc(&conn)?;
        let (first_beat_ms, grid_bpm, tempo_changes) = beatgrid::grid_columns(grid)?;

        conn.execute(
            "UPDATE tracks SET bpm = ?1, key = ?2, analysis_source = 'crate', date_modified = ?3, _hlc = ?4, \
             beatgrid_first_beat_ms = ?5, beatgrid_bpm = ?6, beatgrid_tempo_changes = ?7 WHERE id = ?8",
            rusqlite::params![bpm, key, now, hlc, first_beat_ms, grid_bpm, tempo_changes, track_id],
        )?;
        dirty::mark_dirty(&conn, &buckets::bucket_for_track_id(track_id))?;

        Ok(())
    }

    /// Static version of get_track for use in blocking context
    fn get_track_static(conn: &Arc<Mutex<Connection>>, id: &str) -> Result<Track> {
        let conn = conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, file_path, file_hash,
                   title, artist, album, year, genre, label, catalog_number,
                   duration_ms, bpm, key, energy, bitrate, sample_rate, format,
                   analysis_source, waveform_data,
                   rating, play_count,
                   date_added, date_modified, last_played,
                   rekordbox_id, artwork_path, artwork_source, color,
                   library_root_id, relative_path
            FROM tracks
            WHERE id = ?1
            "#,
        )?;

        let mut track = stmt.query_row([id], |row| {
            Ok(Track {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_hash: row.get(2)?,
                title: row.get(3)?,
                artist: row.get(4)?,
                album: row.get(5)?,
                year: row.get(6)?,
                genre: row.get(7)?,
                label: row.get(8)?,
                catalog_number: row.get(9)?,
                duration_ms: row.get(10)?,
                bpm: row.get(11)?,
                key: row.get(12)?,
                energy: row.get(13)?,
                bitrate: row.get(14)?,
                sample_rate: row.get(15)?,
                format: row.get(16)?,
                analysis_source: row.get(17)?,
                waveform_data: row.get(18)?,
                rating: row.get(19)?,
                play_count: row.get(20)?,
                date_added: row.get(21)?,
                date_modified: row.get(22)?,
                last_played: row.get(23)?,
                rekordbox_id: row.get(24)?,
                artwork_path: row.get(25)?,
                artwork_source: row.get(26)?,
                color: row.get(27)?,
                library_root_id: row.get(28)?,
                relative_path: row.get(29)?,
                tags: Vec::new(),
            })
        })?;

        // Fetch tags
        Self::fetch_tags_for_track_static(&conn, &mut track)?;
        Ok(track)
    }

    /// Static version of fetch_tags for use in blocking context
    fn fetch_tags_for_track_static(conn: &Connection, track: &mut Track) -> Result<()> {
        let mut stmt = conn.prepare(
            r#"
            SELECT t.id, t.category_id, t.name, t.color, t.sort_order
            FROM track_tags tt
            JOIN tags t ON tt.tag_id = t.id
            WHERE tt.track_id = ?1
            "#,
        )?;

        let tags = stmt
            .query_map([&track.id], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    category_id: row.get(1)?,
                    name: row.get(2)?,
                    color: row.get(3)?,
                    sort_order: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        track.tags = tags;
        Ok(())
    }

    /// Cancel the current analysis operation (cancels all)
    pub fn cancel_analysis(&self) -> Result<()> {
        self.cancel_all_analysis()
    }

    /// Get a track by ID
    fn get_track(&self, id: &str) -> Result<Track> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, file_path, file_hash,
                   title, artist, album, year, genre, label, catalog_number,
                   duration_ms, bpm, key, energy, bitrate, sample_rate, format,
                   analysis_source, waveform_data,
                   rating, play_count,
                   date_added, date_modified, last_played,
                   rekordbox_id, artwork_path, artwork_source, color,
                   library_root_id, relative_path
            FROM tracks
            WHERE id = ?1
            "#,
        )?;

        let track = stmt.query_row([id], |row| {
            Ok(Track {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_hash: row.get(2)?,
                title: row.get(3)?,
                artist: row.get(4)?,
                album: row.get(5)?,
                year: row.get(6)?,
                genre: row.get(7)?,
                label: row.get(8)?,
                catalog_number: row.get(9)?,
                duration_ms: row.get(10)?,
                bpm: row.get(11)?,
                key: row.get(12)?,
                energy: row.get(13)?,
                bitrate: row.get(14)?,
                sample_rate: row.get(15)?,
                format: row.get(16)?,
                analysis_source: row.get(17)?,
                waveform_data: row.get(18)?,
                rating: row.get(19)?,
                play_count: row.get(20)?,
                date_added: row.get(21)?,
                date_modified: row.get(22)?,
                last_played: row.get(23)?,
                rekordbox_id: row.get(24)?,
                artwork_path: row.get(25)?,
                artwork_source: row.get(26)?,
                color: row.get(27)?,
                library_root_id: row.get(28)?,
                relative_path: row.get(29)?,
                tags: Vec::new(),
            })
        })?;

        let tracks_with_tags = self.fetch_tags_for_tracks(&conn, vec![track])?;
        tracks_with_tags
            .into_iter()
            .next()
            .ok_or_else(|| CrateError::TrackNotFound(id.to_string()))
    }

    fn fetch_tags_for_tracks(
        &self,
        conn: &Connection,
        mut tracks: Vec<Track>,
    ) -> Result<Vec<Track>> {
        if tracks.is_empty() {
            return Ok(tracks);
        }

        let track_ids: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();
        let placeholders: Vec<String> = track_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();

        let sql = format!(
            r#"
            SELECT tt.track_id, t.id, t.category_id, t.name, t.color, t.sort_order
            FROM track_tags tt
            JOIN tags t ON tt.tag_id = t.id
            WHERE tt.track_id IN ({})
            "#,
            placeholders.join(", ")
        );

        let params_refs: Vec<&dyn rusqlite::ToSql> = track_ids
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let mut stmt = conn.prepare(&sql)?;
        let tag_rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    Tag {
                        id: row.get(1)?,
                        category_id: row.get(2)?,
                        name: row.get(3)?,
                        color: row.get(4)?,
                        sort_order: row.get(5)?,
                    },
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut tags_by_track: std::collections::HashMap<String, Vec<Tag>> =
            std::collections::HashMap::new();
        for (track_id, tag) in tag_rows {
            tags_by_track.entry(track_id).or_default().push(tag);
        }

        for track in &mut tracks {
            if let Some(tags) = tags_by_track.remove(&track.id) {
                track.tags = tags;
            }
        }

        Ok(tracks)
    }

    /// Get updated track after analysis (for returning to frontend)
    pub fn get_updated_track(&self, track_id: &str) -> Result<Track> {
        self.get_track(track_id)
    }
}

impl Clone for AnalysisService {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
            tasks: self.tasks.clone(),
            limiter: self.limiter.clone(),
        }
    }
}

#[cfg(test)]
mod limiter_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn concurrency_is_bounded_between_one_and_four() {
        let n = analysis_concurrency();
        assert!((1..=4).contains(&n), "got {n}");
    }

    #[tokio::test]
    async fn never_more_analyses_than_permits_run_at_once() {
        let limiter = AnalysisLimiter::new(2);
        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));

        let tasks: Vec<_> = (0..12)
            .map(|_| {
                let limiter = limiter.clone();
                let running = running.clone();
                let peak = peak.clone();
                tokio::spawn(async move {
                    let permit = limiter.acquire(&CancellationToken::new()).await.unwrap();
                    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(15)).await;
                    running.fetch_sub(1, Ordering::SeqCst);
                    drop(permit);
                })
            })
            .collect();
        for task in tasks {
            task.await.unwrap();
        }

        assert_eq!(peak.load(Ordering::SeqCst), 2, "all 12 ran, two at a time");
    }

    #[tokio::test]
    async fn a_track_cancelled_while_waiting_gives_up_its_place() {
        let limiter = AnalysisLimiter::new(1);
        let _busy = limiter.acquire(&CancellationToken::new()).await.unwrap();

        let token = CancellationToken::new();
        let waiting = {
            let limiter = limiter.clone();
            let token = token.clone();
            tokio::spawn(async move { limiter.acquire(&token).await.is_none() })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        token.cancel();

        assert!(waiting.await.unwrap(), "cancelled while queued: no permit");
    }

    #[tokio::test]
    async fn a_released_permit_lets_the_next_track_start() {
        let limiter = AnalysisLimiter::new(1);
        let first = limiter.acquire(&CancellationToken::new()).await.unwrap();
        drop(first);
        let second = tokio::time::timeout(
            Duration::from_secs(1),
            limiter.acquire(&CancellationToken::new()),
        )
        .await
        .expect("the freed slot is reused");
        assert!(second.is_some());
    }
}

#[cfg(test)]
mod beat_grid_tests {
    use super::*;

    const RATE: u32 = 44_100;

    /// 16-bit mono PCM WAV.
    fn write_wav(path: &Path, samples: &[f32]) {
        let data: Vec<u8> = samples
            .iter()
            .flat_map(|s| ((s.clamp(-1.0, 1.0) * 32_767.0) as i16).to_le_bytes())
            .collect();
        let mut wav = Vec::new();
        wav.extend(b"RIFF");
        wav.extend((36 + data.len() as u32).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes()); // PCM
        wav.extend(1u16.to_le_bytes()); // mono
        wav.extend(RATE.to_le_bytes());
        wav.extend((RATE * 2).to_le_bytes());
        wav.extend(2u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend((data.len() as u32).to_le_bytes());
        wav.extend(data);
        std::fs::write(path, wav).unwrap();
    }

    fn library_with(track_id: &str, path: &Path) -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO tracks (id, file_path, duration_ms, format, date_added, date_modified) \
             VALUES (?1, ?2, 30000, 'wav', '2020-01-01', '2020-01-01')",
            rusqlite::params![track_id, path.to_string_lossy()],
        )
        .unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn the_analysis_pass_stores_the_grid_of_a_click_track() {
        // 123.7 BPM, first beat 350 ms in, after leading silence that stratum-dsp trims.
        let (bpm, first_beat_ms) = (123.7, 350.0);
        let path = std::env::temp_dir().join(format!(
            "crate_beatgrid_{}_{:?}.wav",
            std::process::id(),
            std::thread::current().id()
        ));
        write_wav(
            &path,
            &beatgrid::tests::click_track(RATE, bpm, first_beat_ms / 1000.0, 30.0),
        );
        let conn = library_with("t1", &path);

        let started = std::time::Instant::now();
        let (result, _) = AnalysisService::analyze_track_with_cancellation(
            &conn,
            "t1",
            &CancellationToken::new(),
        )
        .unwrap();
        let elapsed = started.elapsed();
        let _ = std::fs::remove_file(&path);
        assert!(result.success, "{result:?}");

        let conn = conn.lock().unwrap();
        let grid = beatgrid::load_beat_grid(&conn, "t1")
            .unwrap()
            .expect("a grid is stored");
        eprintln!("grid {grid:?} in {elapsed:?}");
        // stratum-dsp rounds this tempo to 124; the grid keeps the decimals.
        assert!((grid.bpm - bpm).abs() < 0.01, "{grid:?}");
        assert!((grid.first_beat_ms - first_beat_ms).abs() < 3.0, "{grid:?}");
        assert_eq!(grid.tempo_changes, None);

        // The displayed BPM is still the rounded one, and the track is marked analysed by Crate.
        let (stored_bpm, source): (f64, String) = conn
            .query_row(
                "SELECT bpm, analysis_source FROM tracks WHERE id = 't1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(stored_bpm, 124.0);
        assert_eq!(source, "crate");
    }

    #[test]
    fn a_grid_only_analysis_leaves_mixed_in_key_values_alone() {
        let path = std::env::temp_dir().join(format!(
            "crate_beatgrid_mik_{}_{:?}.wav",
            std::process::id(),
            std::thread::current().id()
        ));
        write_wav(&path, &beatgrid::tests::click_track(RATE, 128.0, 0.2, 20.0));
        let conn = library_with("t1", &path);
        conn.lock()
            .unwrap()
            .execute(
                "UPDATE tracks SET bpm = 64.0, key = '8A', energy = 6, \
                 analysis_source = 'mixed_in_key', _hlc = 'h1' WHERE id = 't1'",
                [],
            )
            .unwrap();

        let grid =
            AnalysisService::analyze_beat_grid_blocking(&conn, "t1", &CancellationToken::new())
                .unwrap()
                .expect("a grid");
        let _ = std::fs::remove_file(&path);
        assert!((grid.bpm - 128.0).abs() < 0.05, "{grid:?}");

        let conn = conn.lock().unwrap();
        assert_eq!(beatgrid::load_beat_grid(&conn, "t1").unwrap(), Some(grid));
        let row: (f64, String, i32, String, String) = conn
            .query_row(
                "SELECT bpm, key, energy, analysis_source, _hlc FROM tracks WHERE id = 't1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(
            row,
            (
                64.0,
                "8A".to_string(),
                6,
                "mixed_in_key".to_string(),
                "h1".to_string()
            )
        );
    }

    #[test]
    fn a_failed_grid_only_analysis_reports_a_missing_track() {
        let conn = library_with("t1", Path::new("/nonexistent/crate-test.wav"));
        let err =
            AnalysisService::analyze_beat_grid_blocking(&conn, "nope", &CancellationToken::new())
                .unwrap_err();
        assert!(matches!(err, CrateError::TrackNotFound(_)), "{err}");
        let err =
            AnalysisService::analyze_beat_grid_blocking(&conn, "t1", &CancellationToken::new())
                .unwrap_err();
        assert!(err.to_string().contains("File not found"), "{err}");
    }
}

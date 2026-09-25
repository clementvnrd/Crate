use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use chrono::Utc;
use rusqlite::{Connection, OpenFlags};
use uuid::Uuid;

use crate::error::{CrateError, Result};
use crate::models::stats::{ListenEvent, RekordboxSession};
use crate::services::stats::recorder::StatsRecorderService;

pub struct RekordboxTrackerService {
    conn: Arc<Mutex<Connection>>,
    recorder: Arc<StatsRecorderService>,
}

impl RekordboxTrackerService {
    pub fn new(conn: Arc<Mutex<Connection>>, recorder: Arc<StatsRecorderService>) -> Self {
        Self { conn, recorder }
    }

    /// Detects Pioneer Rekordbox installation directories.
    pub fn detect_rekordbox_dirs() -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        #[cfg(target_os = "macos")]
        {
            if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                candidates.push(home.join("Library/Application Support/Pioneer/rekordbox"));
                candidates.push(home.join("Library/Application Support/Pioneer/rekordbox7"));
                candidates.push(home.join("Library/Pioneer/rekordbox"));
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
                candidates.push(appdata.join("Pioneer/rekordbox"));
                candidates.push(appdata.join("Pioneer/rekordbox7"));
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                candidates.push(home.join(".config/Pioneer/rekordbox"));
                candidates.push(home.join(".config/Pioneer/rekordbox7"));
            }
        }

        candidates.into_iter().filter(|p| p.exists() && p.is_dir()).collect()
    }

    /// Finds the first accessible Rekordbox directory.
    pub fn detect_rekordbox_dir() -> Option<PathBuf> {
        Self::detect_rekordbox_dirs().into_iter().next()
    }

    /// Finds Rekordbox master database file if present.
    pub fn find_master_db() -> Option<PathBuf> {
        for dir in Self::detect_rekordbox_dirs() {
            let master = dir.join("master.db");
            if master.exists() {
                return Some(master);
            }
        }
        None
    }

    /// Returns whether Rekordbox is detected on this system.
    pub fn is_rekordbox_installed(&self) -> bool {
        !Self::detect_rekordbox_dirs().is_empty()
    }

    /// Synchronizes Rekordbox history sessions and played tracks into Crate.
    pub fn sync_rekordbox_history(&self) -> Result<usize> {
        let mut total_synced = 0;

        if let Some(master_db_path) = Self::find_master_db() {
            match self.sync_from_master_db(&master_db_path) {
                Ok(count) => {
                    log::info!("Synced {count} tracks from Rekordbox master.db");
                    total_synced += count;
                }
                Err(e) => {
                    log::warn!("Could not read Rekordbox master.db (may be encrypted or locked): {e}");
                }
            }
        }

        // Also scan for History files in Rekordbox directories
        for dir in Self::detect_rekordbox_dirs() {
            let history_dir = dir.join("History");
            if history_dir.exists() && history_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(history_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if ext.eq_ignore_ascii_case("xml") {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    if let Ok(count) = self.import_rekordbox_history_xml(&content) {
                                        total_synced += count;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(total_synced)
    }

    /// Reads Rekordbox `master.db` in read-only mode and imports history playlists.
    fn sync_from_master_db(&self, db_path: &Path) -> Result<usize> {
        let rb_conn = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|e| CrateError::Database(e))?;

        let _ = rb_conn.execute("PRAGMA query_only = ON;", []);

        // Check if djmdSongHistory or djmdPlaylist table exists
        let has_history: bool = rb_conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND (name='djmdSongHistory' OR name='djmdPlaylist')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);

        if !has_history {
            return Ok(0);
        }

        let mut synced_tracks = 0;

        // Try reading history sessions from djmdSongHistory joined with djmdContent
        let sql = r#"
            SELECT
                h.ID,
                COALESCE(h.HistoryID, h.ID) as session_id,
                COALESCE(c.Title, '') as title,
                COALESCE(c.ArtistName, '') as artist,
                COALESCE(c.AlbumName, '') as album,
                COALESCE(c.Duration, 0) as duration_sec,
                c.BPM,
                c.KeyID,
                COALESCE(h.created_at, datetime('now')) as played_at
            FROM djmdSongHistory h
            LEFT JOIN djmdContent c ON h.ContentID = c.ID
            ORDER BY h.created_at ASC
        "#;

        if let Ok(mut stmt) = rb_conn.prepare(sql) {
            let rows = stmt.query_map([], |row| {
                let id: String = row.get(0)?;
                let session_id: String = row.get(1)?;
                let title: String = row.get(2)?;
                let artist: String = row.get(3)?;
                let album: String = row.get(4)?;
                let duration_sec: i64 = row.get(5)?;
                let bpm_raw: Option<i64> = row.get(6)?;
                let key_raw: Option<String> = row.get(7)?;
                let played_at: String = row.get(8)?;

                let duration_ms = (duration_sec * 1000).max(0) as u64;
                let bpm = bpm_raw.map(|b| (b as f64) / 100.0);

                Ok((
                    id,
                    session_id,
                    title,
                    artist,
                    if album.is_empty() { None } else { Some(album) },
                    duration_ms,
                    bpm,
                    key_raw,
                    played_at,
                ))
            });

            if let Ok(mapped) = rows {
                for item in mapped.flatten() {
                    let (_id, session_id, title, artist, album, duration_ms, bpm, key, played_at) = item;
                    if title.trim().is_empty() || artist.trim().is_empty() {
                        continue;
                    }

                    let played_ms = if duration_ms > 0 { duration_ms } else { 180_000 };

                    let event = ListenEvent {
                        id: Uuid::new_v4().to_string(),
                        source: "rekordbox".to_string(),
                        track_id: None,
                        title,
                        artist,
                        album,
                        duration_ms,
                        played_ms,
                        bpm,
                        key,
                        energy: None,
                        format: Some("rekordbox".to_string()),
                        artwork_url: None,
                        played_at: played_at.clone(),
                        session_id: Some(session_id.clone()),
                        metadata_json: None,
                    };

                    if let Ok(true) = self.recorder.record_listen_event(&event) {
                        synced_tracks += 1;
                    }

                    // Update rekordbox_sessions table
                    let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
                    let _ = conn.execute(
                        r#"
                        INSERT INTO rekordbox_sessions (
                            id, session_name, started_at, ended_at, total_tracks, total_played_ms
                        ) VALUES (?1, ?2, ?3, ?3, 1, ?4)
                        ON CONFLICT(id) DO UPDATE SET
                            total_tracks = total_tracks + 1,
                            total_played_ms = total_played_ms + excluded.total_played_ms,
                            ended_at = excluded.ended_at
                        "#,
                        rusqlite::params![
                            session_id,
                            format!("Rekordbox Session {}", &played_at[..10.min(played_at.len())]),
                            played_at,
                            played_ms as i64
                        ],
                    );
                }
            }
        }

        Ok(synced_tracks)
    }

    /// Parses Rekordbox history XML exports and records tracks into Crate stats.
    pub fn import_rekordbox_history_xml(&self, xml_content: &str) -> Result<usize> {
        let mut count = 0;
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // Parse XML track tags with regex / simple attributes
        let track_regex = regex::Regex::new(
            r#"(?i)<TRACK\s+([^>]+)>"#
        ).map_err(|e| CrateError::InvalidOperation(e.to_string()))?;

        let name_regex = regex::Regex::new(r#"(?i)Name="([^"]*)""#).unwrap();
        let artist_regex = regex::Regex::new(r#"(?i)Artist="([^"]*)""#).unwrap();
        let album_regex = regex::Regex::new(r#"(?i)Album="([^"]*)""#).unwrap();
        let time_regex = regex::Regex::new(r#"(?i)TotalTime="([^"]*)""#).unwrap();
        let bpm_regex = regex::Regex::new(r#"(?i)AverageBpm="([^"]*)""#).unwrap();
        let key_regex = regex::Regex::new(r#"(?i)Tonality="([^"]*)""#).unwrap();

        let mut session_played_ms: u64 = 0;
        let mut session_track_count = 0;

        for cap in track_regex.captures_iter(xml_content) {
            let attrs = &cap[1];

            let title = name_regex.captures(attrs).and_then(|c| c.get(1)).map(|m| m.as_str().trim()).unwrap_or("");
            let artist = artist_regex.captures(attrs).and_then(|c| c.get(1)).map(|m| m.as_str().trim()).unwrap_or("");
            let album = album_regex.captures(attrs).and_then(|c| c.get(1)).map(|m| m.as_str().to_string());
            let time_sec: u64 = time_regex.captures(attrs).and_then(|c| c.get(1)).and_then(|m| m.as_str().parse().ok()).unwrap_or(180);
            let bpm: Option<f64> = bpm_regex.captures(attrs).and_then(|c| c.get(1)).and_then(|m| m.as_str().parse().ok());
            let key = key_regex.captures(attrs).and_then(|c| c.get(1)).map(|m| m.as_str().to_string());

            if title.is_empty() || artist.is_empty() {
                continue;
            }

            let duration_ms = time_sec * 1000;
            let played_ms = duration_ms;

            let event = ListenEvent {
                id: Uuid::new_v4().to_string(),
                source: "rekordbox".to_string(),
                track_id: None,
                title: title.to_string(),
                artist: artist.to_string(),
                album,
                duration_ms,
                played_ms,
                bpm,
                key,
                energy: None,
                format: Some("rekordbox".to_string()),
                artwork_url: None,
                played_at: now.clone(),
                session_id: Some(session_id.clone()),
                metadata_json: None,
            };

            if let Ok(true) = self.recorder.record_listen_event(&event) {
                count += 1;
                session_played_ms += played_ms;
                session_track_count += 1;
            }
        }

        if session_track_count > 0 {
            let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            let _ = conn.execute(
                r#"
                INSERT INTO rekordbox_sessions (
                    id, session_name, started_at, ended_at, total_tracks, total_played_ms
                ) VALUES (?1, ?2, ?3, ?3, ?4, ?5)
                "#,
                rusqlite::params![
                    session_id,
                    format!("Rekordbox Set {}", &now[..10.min(now.len())]),
                    now,
                    session_track_count as i64,
                    session_played_ms as i64
                ],
            );
        }

        Ok(count)
    }

    /// Retrieves all recorded Rekordbox sessions.
    pub fn get_sessions(&self) -> Result<Vec<RekordboxSession>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, session_name, started_at, ended_at, total_tracks, total_played_ms
                FROM rekordbox_sessions
                ORDER BY started_at DESC
                "#,
            )
            .map_err(CrateError::Database)?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let session_name: Option<String> = row.get(1)?;
                let started_at: String = row.get(2)?;
                let ended_at: Option<String> = row.get(3)?;
                let total_tracks: i64 = row.get(4)?;
                let total_played_ms: i64 = row.get(5)?;

                Ok(RekordboxSession {
                    id,
                    session_name,
                    started_at,
                    ended_at,
                    total_tracks: total_tracks.max(0) as usize,
                    total_played_ms: total_played_ms.max(0) as u64,
                })
            })
            .map_err(CrateError::Database)?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item.map_err(CrateError::Database)?);
        }

        Ok(list)
    }
}

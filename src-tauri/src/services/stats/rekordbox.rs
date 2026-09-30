use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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

        candidates
            .into_iter()
            .filter(|p| p.exists() && p.is_dir())
            .collect()
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
                    log::warn!(
                        "Could not read Rekordbox master.db (may be encrypted or locked): {e}"
                    );
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
        .map_err(CrateError::Database)?;

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
                    let (_id, session_id, title, artist, album, duration_ms, bpm, key, played_at) =
                        item;
                    if title.trim().is_empty() || artist.trim().is_empty() {
                        continue;
                    }

                    let played_ms = if duration_ms > 0 {
                        duration_ms
                    } else {
                        180_000
                    };

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

                    // Already imported (re-sync): leave the session counters alone.
                    if !matches!(self.recorder.record_listen_event(&event), Ok(true)) {
                        continue;
                    }
                    synced_tracks += 1;

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
                            format!(
                                "Rekordbox Session {}",
                                &played_at[..10.min(played_at.len())]
                            ),
                            played_at,
                            played_ms as i64
                        ],
                    );
                }
            }
        }

        Ok(synced_tracks)
    }

    /// Imports DJ sets from a Rekordbox XML export (File › Export Collection in xml format).
    ///
    /// Only the dated history playlists (`HISTORY 2026-09-20`, or date-named playlists inside the
    /// HISTORY folder) are sets: each of their tracks becomes one listen, resolved through the
    /// COLLECTION. The rest of the collection is ignored. A session is imported once; importing
    /// a newer export only adds the new sessions. The XML has no time of day, so plays are placed
    /// in order from midnight and flagged `approximate_time` (the heatmap leaves them out).
    pub fn import_rekordbox_history_xml(&self, xml_content: &str) -> Result<usize> {
        let collection = parse_collection(xml_content);
        let mut count = 0;

        for history in parse_history_playlists(xml_content) {
            let session_id = format!("rb-xml-{}", history.name);
            let already_imported = {
                let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
                conn.query_row(
                    "SELECT 1 FROM rekordbox_sessions WHERE id = ?1",
                    [&session_id],
                    |_| Ok(()),
                )
                .is_ok()
            };
            if already_imported {
                continue;
            }

            let mut offset_ms: i64 = 0;
            let mut session_played_ms: u64 = 0;
            let mut session_track_count = 0usize;
            for key in &history.track_keys {
                let Some(track) = collection.get(key) else {
                    continue;
                };
                if track.title.is_empty() || track.artist.is_empty() {
                    continue;
                }
                let played_at = history.date + chrono::Duration::milliseconds(offset_ms);
                offset_ms += track.duration_ms as i64;
                let event = ListenEvent {
                    id: Uuid::new_v4().to_string(),
                    source: "rekordbox".to_string(),
                    track_id: None,
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                    album: track.album.clone(),
                    duration_ms: track.duration_ms,
                    played_ms: track.duration_ms,
                    bpm: track.bpm,
                    key: track.key.clone(),
                    energy: None,
                    format: Some("rekordbox".to_string()),
                    artwork_url: None,
                    played_at: played_at.to_rfc3339(),
                    session_id: Some(session_id.clone()),
                    metadata_json: Some(r#"{"approximate_time":true}"#.to_string()),
                };
                if let Ok(true) = self.recorder.record_listen_event(&event) {
                    count += 1;
                    session_played_ms += track.duration_ms;
                    session_track_count += 1;
                }
            }

            if session_track_count > 0 {
                let started_at = history.date.to_rfc3339();
                let ended_at =
                    (history.date + chrono::Duration::milliseconds(offset_ms)).to_rfc3339();
                let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
                conn.execute(
                    r#"
                    INSERT OR IGNORE INTO rekordbox_sessions (
                        id, session_name, started_at, ended_at, total_tracks, total_played_ms
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                    "#,
                    rusqlite::params![
                        session_id,
                        history.name,
                        started_at,
                        ended_at,
                        session_track_count as i64,
                        session_played_ms as i64
                    ],
                )
                .map_err(CrateError::Database)?;
            }
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

/// A track of the Rekordbox XML COLLECTION.
#[derive(Debug, Clone)]
struct XmlTrack {
    title: String,
    artist: String,
    album: Option<String>,
    duration_ms: u64,
    bpm: Option<f64>,
    key: Option<String>,
}

/// A dated history playlist of the Rekordbox XML.
#[derive(Debug)]
struct XmlHistory {
    name: String,
    date: chrono::DateTime<chrono::Local>,
    track_keys: Vec<String>,
}

pub(crate) fn xml_unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

pub(crate) fn xml_attr(attrs: &str, name: &str) -> Option<String> {
    let re = regex::Regex::new(&format!(r#"(?:^|\s){name}="([^"]*)""#)).ok()?;
    re.captures(attrs)
        .and_then(|c| c.get(1))
        .map(|m| xml_unescape(m.as_str()))
}

/// TrackID → track, from the `<COLLECTION>` section.
fn parse_collection(xml: &str) -> std::collections::HashMap<String, XmlTrack> {
    let mut tracks = std::collections::HashMap::new();
    let Some(start) = xml.find("<COLLECTION") else {
        return tracks;
    };
    let end = xml[start..]
        .find("</COLLECTION>")
        .map_or(xml.len(), |e| start + e);
    let track_re = regex::Regex::new(r#"<TRACK\s+([^>]*?)/?>"#).expect("valid regex");
    for cap in track_re.captures_iter(&xml[start..end]) {
        let attrs = &cap[1];
        let Some(id) = xml_attr(attrs, "TrackID") else {
            continue;
        };
        tracks.insert(
            id,
            XmlTrack {
                title: xml_attr(attrs, "Name")
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
                artist: xml_attr(attrs, "Artist")
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
                album: xml_attr(attrs, "Album").filter(|a| !a.is_empty()),
                duration_ms: xml_attr(attrs, "TotalTime")
                    .and_then(|t| t.parse::<u64>().ok())
                    .unwrap_or(0)
                    * 1000,
                bpm: xml_attr(attrs, "AverageBpm").and_then(|b| b.parse().ok()),
                key: xml_attr(attrs, "Tonality").filter(|k| !k.is_empty()),
            },
        );
    }
    tracks
}

/// Playlists (`Type="1"`) whose name carries a date, found in the `<PLAYLISTS>` section.
fn parse_history_playlists(xml: &str) -> Vec<XmlHistory> {
    let Some(start) = xml.find("<PLAYLISTS") else {
        return Vec::new();
    };
    let node_re =
        regex::Regex::new(r#"(?s)<NODE\s+([^>]*Type="1"[^>]*)>(.*?)</NODE>"#).expect("valid regex");
    let key_re = regex::Regex::new(r#"<TRACK\s+Key="([^"]+)""#).expect("valid regex");
    let date_re = regex::Regex::new(r"(\d{4})-(\d{2})-(\d{2})").expect("valid regex");
    node_re
        .captures_iter(&xml[start..])
        .filter_map(|cap| {
            let name = xml_attr(&cap[1], "Name")?;
            let d = date_re.captures(&name)?;
            let date = chrono::NaiveDate::from_ymd_opt(
                d[1].parse().ok()?,
                d[2].parse().ok()?,
                d[3].parse().ok()?,
            )?;
            let local = date
                .and_hms_opt(0, 0, 0)?
                .and_local_timezone(chrono::Local)
                .earliest()?;
            let track_keys = key_re
                .captures_iter(&cap[2])
                .map(|k| k[1].to_string())
                .collect();
            Some(XmlHistory {
                name,
                date: local,
                track_keys,
            })
        })
        .collect()
}

#[cfg(test)]
mod xml_tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<DJ_PLAYLISTS Version="1.0.0">
  <COLLECTION Entries="3">
    <TRACK TrackID="1" Name="Opus" Artist="Eric Prydz" Album="Opus" TotalTime="540" AverageBpm="126.00" Tonality="10B"/>
    <TRACK TrackID="2" Name="R&amp;B Groove" Artist="Someone" TotalTime="300" AverageBpm="122.00" Tonality="8A"/>
    <TRACK TrackID="3" Name="Never Played" Artist="Nobody" TotalTime="200"/>
  </COLLECTION>
  <PLAYLISTS>
    <NODE Type="0" Name="ROOT" Count="2">
      <NODE Name="HISTORY 2026-09-20" Type="1" KeyType="0" Entries="2">
        <TRACK Key="1"/>
        <TRACK Key="2"/>
      </NODE>
      <NODE Name="Warm-up" Type="1" KeyType="0" Entries="1">
        <TRACK Key="3"/>
      </NODE>
    </NODE>
  </PLAYLISTS>
</DJ_PLAYLISTS>"#;

    fn service() -> (RekordboxTrackerService, Arc<Mutex<Connection>>) {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let recorder = Arc::new(StatsRecorderService::new(conn.clone()));
        (RekordboxTrackerService::new(conn.clone(), recorder), conn)
    }

    #[test]
    fn test_only_history_playlists_become_listens_once() {
        let (svc, conn) = service();
        assert_eq!(svc.import_rekordbox_history_xml(SAMPLE).unwrap(), 2);
        assert_eq!(
            svc.import_rekordbox_history_xml(SAMPLE).unwrap(),
            0,
            "re-importing the same export adds nothing"
        );

        let conn = conn.lock().unwrap();
        let titles: Vec<String> = conn
            .prepare("SELECT title FROM listen_events ORDER BY played_at")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(titles, vec!["Opus", "R&B Groove"]);
        let first_day: String = conn
            .query_row(
                "SELECT date(played_at) FROM listen_events LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            first_day.starts_with("2026-09-"),
            "dated from the history playlist, not today"
        );
    }
}

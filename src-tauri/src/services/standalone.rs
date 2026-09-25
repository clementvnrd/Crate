use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use lofty::config::{ParseOptions, ParsingMode};
use lofty::file::AudioFile;
use lofty::prelude::*;
use lofty::probe::Probe;
use rusqlite::{Connection, OptionalExtension};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;

use crate::error::{CrateError, Result};
use crate::models::StandaloneTrack;
use crate::services::library::MikService;
use crate::services::ArtworkService;

pub struct StandaloneService {
    conn: Arc<Mutex<Connection>>,
    artwork_service: ArtworkService,
}

impl StandaloneService {
    pub fn new(conn: Arc<Mutex<Connection>>, app_data_dir: PathBuf) -> Self {
        Self {
            conn,
            artwork_service: ArtworkService::new(app_data_dir),
        }
    }

    /// Read metadata for an audio file.
    /// Checks if it exists in the Crate library (`tracks` table).
    /// If not, extracts metadata on the fly without writing into `tracks`.
    pub fn read_standalone_track(&self, file_path_str: &str) -> Result<StandaloneTrack> {
        let path = PathBuf::from(file_path_str);
        if !path.exists() {
            return Err(CrateError::FileNotFound(path));
        }

        // 1. Check if track already exists in library
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let in_library: Option<StandaloneTrack> = conn
            .query_row(
                r#"
                SELECT id, file_path, title, artist, album, duration_ms, format,
                       bitrate, sample_rate, bpm, key, energy, artwork_path, last_played
                FROM tracks
                WHERE file_path = ?1
                "#,
                rusqlite::params![file_path_str],
                |row| {
                    Ok(StandaloneTrack {
                        id: row.get(0)?,
                        file_path: row.get(1)?,
                        title: row.get(2)?,
                        artist: row.get(3)?,
                        album: row.get(4)?,
                        duration_ms: row.get(5)?,
                        format: row.get(6)?,
                        bitrate: row.get(7)?,
                        sample_rate: row.get(8)?,
                        bpm: row.get(9)?,
                        key: row.get(10)?,
                        energy: row.get(11)?,
                        artwork_path: row.get(12)?,
                        is_in_library: true,
                        last_played_at: row.get(13)?,
                    })
                },
            )
            .optional()
            .map_err(CrateError::Database)?;

        let in_recents: Option<StandaloneTrack> = conn
            .query_row(
                "SELECT id, file_path, title, artist, album, duration_ms, format, bitrate, sample_rate, bpm, key, artwork_path, last_played_at
                 FROM recent_standalone_tracks WHERE file_path = ?1 LIMIT 1",
                rusqlite::params![file_path_str],
                |row| {
                    Ok(StandaloneTrack {
                        id: row.get(0)?,
                        file_path: row.get(1)?,
                        title: row.get(2)?,
                        artist: row.get(3)?,
                        album: row.get(4)?,
                        duration_ms: row.get(5)?,
                        format: row.get(6)?,
                        bitrate: row.get(7)?,
                        sample_rate: row.get(8)?,
                        bpm: row.get(9)?,
                        key: row.get(10)?,
                        energy: None,
                        artwork_path: row.get(11)?,
                        is_in_library: false,
                        last_played_at: row.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(CrateError::Database)?;
        drop(conn);

        if let Some(track) = in_library {
            return Ok(track);
        }
        if let Some(track) = in_recents {
            return Ok(track);
        }

        // 2. Extract metadata on the fly without database insertion
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_else(|| "mp3".to_string());

        let standalone_id = uuid::Uuid::new_v4().to_string();
        let mut title = None;
        let mut artist = None;
        let mut album = None;
        let mut duration_ms = 0i64;
        let mut bitrate = None;
        let mut sample_rate = None;
        let mut bpm = None;
        let mut key = None;
        let mut energy = None;
        let mut artwork_path = None;

        if let Some(tagged_file) = Self::read_metadata_lenient(&path) {
            let properties = tagged_file.properties();
            duration_ms = properties.duration().as_millis() as i64;
            bitrate = properties.audio_bitrate().map(|b| b as i32);
            sample_rate = properties.sample_rate().map(|s| s as i32);

            if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
                title = tag.title().map(|s| s.to_string());
                artist = tag.artist().map(|s| s.to_string());
                album = tag.album().map(|s| s.to_string());
            }

            let mik_data = MikService::extract_analysis_data(&tagged_file, &standalone_id);
            if let Some(b) = mik_data.bpm {
                bpm = Some(b);
            }
            if let Some(k) = mik_data.key {
                key = Some(k);
            }
            if let Some(e) = mik_data.energy {
                energy = Some(e);
            }

            if let Some(art) = self
                .artwork_service
                .extract_from_tagged_file_or_folder(&tagged_file, &path, &standalone_id)
            {
                artwork_path = Some(art);
            }
        } else if let Ok((dur, sr, br)) = Self::read_audio_properties_symphonia(&path) {
            duration_ms = dur;
            sample_rate = sr;
            bitrate = br;
        }

        // Fallback title to filename if absent from tags
        if title.is_none() {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                title = Some(stem.to_string());
            }
        }

        Ok(StandaloneTrack {
            id: standalone_id,
            file_path: file_path_str.to_string(),
            title,
            artist,
            album,
            duration_ms,
            format,
            bitrate,
            sample_rate,
            bpm,
            key,
            energy,
            artwork_path,
            is_in_library: false,
            last_played_at: None,
        })
    }

    /// Read metadata leniently with lofty
    fn read_metadata_lenient(path: &Path) -> Option<lofty::file::TaggedFile> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);

        let parse_options = ParseOptions::new()
            .parsing_mode(ParsingMode::Relaxed)
            .max_junk_bytes(4096);

        Probe::new(reader)
            .options(parse_options)
            .guess_file_type()
            .ok()?
            .read()
            .ok()
    }

    /// Read properties with symphonia fallback
    fn read_audio_properties_symphonia(
        path: &Path,
    ) -> Result<(i64, Option<i32>, Option<i32>)> {
        let file = File::open(path).map_err(|e| CrateError::Metadata(format!("Failed to open: {e}")))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &Default::default(), &Default::default())
            .map_err(|e| CrateError::Metadata(format!("Symphonia probe failed: {e}")))?;

        let track = probed
            .format
            .default_track()
            .ok_or_else(|| CrateError::Metadata("No audio track found".to_string()))?;

        let params = &track.codec_params;
        let duration_ms = match (params.n_frames, params.sample_rate) {
            (Some(frames), Some(sample_rate)) => {
                ((frames as f64 / sample_rate as f64) * 1000.0) as i64
            }
            _ => 0,
        };

        let sample_rate = params.sample_rate.map(|s| s as i32);
        let channels = params.channels.map(|c| c.count() as i32).unwrap_or(2);
        let bits = params.bits_per_sample.unwrap_or(16) as i32;
        let sr = sample_rate.unwrap_or(44100);
        let bitrate = Some((sr * channels * bits + 500) / 1000);

        Ok((duration_ms, sample_rate, bitrate))
    }

    /// Retrieve recent standalone tracks.
    /// Strictly excludes any files that currently exist in the Crate library (`tracks` table).
    pub fn get_recent_standalone_tracks(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<StandaloneTrack>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let limit_val = limit.unwrap_or(100) as i64;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT r.id, r.file_path, r.title, r.artist, r.album, r.duration_ms, r.format,
                       r.bitrate, r.sample_rate, r.bpm, r.key, r.artwork_path, r.last_played_at
                FROM recent_standalone_tracks r
                LEFT JOIN tracks t ON r.file_path = t.file_path
                WHERE t.file_path IS NULL
                ORDER BY r.last_played_at DESC
                LIMIT ?1
                "#,
            )
            .map_err(CrateError::Database)?;

        let rows = stmt
            .query_map(rusqlite::params![limit_val], |row| {
                Ok(StandaloneTrack {
                    id: row.get(0)?,
                    file_path: row.get(1)?,
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    album: row.get(4)?,
                    duration_ms: row.get(5)?,
                    format: row.get(6)?,
                    bitrate: row.get(7)?,
                    sample_rate: row.get(8)?,
                    bpm: row.get(9)?,
                    key: row.get(10)?,
                    energy: None,
                    artwork_path: row.get(11)?,
                    is_in_library: false,
                    last_played_at: row.get(12)?,
                })
            })
            .map_err(CrateError::Database)?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item.map_err(CrateError::Database)?);
        }

        Ok(list)
    }

    /// Add a standalone track to recent history.
    /// STRICT RULE: If track file_path exists in the `tracks` table, it is NEVER inserted.
    pub fn add_recent_standalone_track(&self, track: &StandaloneTrack) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // STRICT RULE check
        let is_in_library: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM tracks WHERE file_path = ?1",
                rusqlite::params![track.file_path],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if is_in_library || track.is_in_library {
            return Ok(());
        }

        let now = chrono::Utc::now().to_rfc3339();
        let last_played = track
            .last_played_at
            .as_deref()
            .unwrap_or(&now);

        conn.execute(
            r#"
            INSERT INTO recent_standalone_tracks (
                id, file_path, title, artist, album, duration_ms, format,
                bitrate, sample_rate, bpm, key, artwork_path, last_played_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(file_path) DO UPDATE SET
                title = COALESCE(excluded.title, recent_standalone_tracks.title),
                artist = COALESCE(excluded.artist, recent_standalone_tracks.artist),
                album = COALESCE(excluded.album, recent_standalone_tracks.album),
                duration_ms = excluded.duration_ms,
                format = excluded.format,
                bitrate = COALESCE(excluded.bitrate, recent_standalone_tracks.bitrate),
                sample_rate = COALESCE(excluded.sample_rate, recent_standalone_tracks.sample_rate),
                bpm = COALESCE(excluded.bpm, recent_standalone_tracks.bpm),
                key = COALESCE(excluded.key, recent_standalone_tracks.key),
                artwork_path = COALESCE(excluded.artwork_path, recent_standalone_tracks.artwork_path),
                last_played_at = excluded.last_played_at
            "#,
            rusqlite::params![
                track.id,
                track.file_path,
                track.title,
                track.artist,
                track.album,
                track.duration_ms,
                track.format,
                track.bitrate,
                track.sample_rate,
                track.bpm,
                track.key,
                track.artwork_path,
                last_played,
            ],
        )
        .map_err(CrateError::Database)?;

        Ok(())
    }

    /// Remove a track from recent standalone history
    pub fn remove_recent_standalone_track(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        conn.execute(
            "DELETE FROM recent_standalone_tracks WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(CrateError::Database)?;
        Ok(())
    }

    /// Clear all recent standalone history
    pub fn clear_recent_standalone_tracks(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        conn.execute("DELETE FROM recent_standalone_tracks", [])
            .map_err(CrateError::Database)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use crate::db::schema::get_migrations;

    fn setup_test_db() -> (Arc<Mutex<Connection>>, StandaloneService) {
        let conn = Connection::open_in_memory().unwrap();
        for migration in get_migrations() {
            conn.execute_batch(migration).unwrap();
        }
        let conn_arc = Arc::new(Mutex::new(conn));
        let service = StandaloneService::new(conn_arc.clone(), std::env::temp_dir());
        (conn_arc, service)
    }

    #[test]
    fn test_add_and_get_recent_standalone_tracks() {
        let (_conn, service) = setup_test_db();

        let track1 = StandaloneTrack {
            id: "s1".to_string(),
            file_path: "/music/track1.mp3".to_string(),
            title: Some("Track 1".to_string()),
            artist: Some("Artist 1".to_string()),
            album: Some("Album 1".to_string()),
            duration_ms: 240000,
            format: "mp3".to_string(),
            bitrate: Some(320),
            sample_rate: Some(44100),
            bpm: Some(124.0),
            key: Some("8A".to_string()),
            energy: Some(7),
            artwork_path: None,
            is_in_library: false,
            last_played_at: Some("2026-08-31T01:00:00Z".to_string()),
        };

        let track2 = StandaloneTrack {
            id: "s2".to_string(),
            file_path: "/music/track2.flac".to_string(),
            title: Some("Track 2".to_string()),
            artist: Some("Artist 2".to_string()),
            album: None,
            duration_ms: 300000,
            format: "flac".to_string(),
            bitrate: Some(1411),
            sample_rate: Some(44100),
            bpm: Some(128.0),
            key: Some("11B".to_string()),
            energy: Some(8),
            artwork_path: None,
            is_in_library: false,
            last_played_at: Some("2026-08-31T02:00:00Z".to_string()),
        };

        service.add_recent_standalone_track(&track1).unwrap();
        service.add_recent_standalone_track(&track2).unwrap();

        let list = service.get_recent_standalone_tracks(None).unwrap();
        assert_eq!(list.len(), 2);
        // Ordered by last_played_at DESC
        assert_eq!(list[0].id, "s2");
        assert_eq!(list[1].id, "s1");
    }

    #[test]
    fn test_strict_library_isolation_rule() {
        let (conn_arc, service) = setup_test_db();

        // 1. Insert a track into the Crate library (`tracks` table)
        {
            let conn = conn_arc.lock().unwrap();
            conn.execute(
                r#"
                INSERT INTO tracks (
                    id, file_path, duration_ms, format, title, artist, date_added, date_modified
                ) VALUES ('lib_1', '/library/crate_song.flac', 360000, 'flac', 'Crate Anthem', 'DJ Crate', '2026-01-01', '2026-01-01')
                "#,
                [],
            ).unwrap();
        }

        // 2. Try to add this library track to recent standalone tracks
        let lib_track = StandaloneTrack {
            id: "s_lib".to_string(),
            file_path: "/library/crate_song.flac".to_string(),
            title: Some("Crate Anthem".to_string()),
            artist: Some("DJ Crate".to_string()),
            album: None,
            duration_ms: 360000,
            format: "flac".to_string(),
            bitrate: Some(1411),
            sample_rate: Some(44100),
            bpm: Some(130.0),
            key: Some("5A".to_string()),
            energy: Some(9),
            artwork_path: None,
            is_in_library: true,
            last_played_at: Some("2026-08-31T03:00:00Z".to_string()),
        };

        service.add_recent_standalone_track(&lib_track).unwrap();

        // Recent list MUST be empty
        let list = service.get_recent_standalone_tracks(None).unwrap();
        assert_eq!(list.len(), 0);

        // 3. Even if a track was inserted into recent_standalone_tracks prior to library import,
        // querying recent tracks must strictly exclude any file present in `tracks`
        {
            let conn = conn_arc.lock().unwrap();
            conn.execute(
                r#"
                INSERT INTO recent_standalone_tracks (
                    id, file_path, duration_ms, format, last_played_at
                ) VALUES ('s_old', '/library/crate_song.flac', 360000, 'flac', '2026-08-30T12:00:00Z')
                "#,
                [],
            ).unwrap();
        }

        let list_after = service.get_recent_standalone_tracks(None).unwrap();
        assert_eq!(list_after.len(), 0);
    }

    #[test]
    fn test_remove_and_clear_recent_tracks() {
        let (_conn, service) = setup_test_db();

        let track = StandaloneTrack {
            id: "s1".to_string(),
            file_path: "/music/track1.mp3".to_string(),
            title: Some("Track 1".to_string()),
            artist: Some("Artist 1".to_string()),
            album: None,
            duration_ms: 200000,
            format: "mp3".to_string(),
            bitrate: Some(320),
            sample_rate: Some(44100),
            bpm: None,
            key: None,
            energy: None,
            artwork_path: None,
            is_in_library: false,
            last_played_at: Some("2026-08-31T01:00:00Z".to_string()),
        };

        service.add_recent_standalone_track(&track).unwrap();
        assert_eq!(service.get_recent_standalone_tracks(None).unwrap().len(), 1);

        service.remove_recent_standalone_track("s1").unwrap();
        assert_eq!(service.get_recent_standalone_tracks(None).unwrap().len(), 0);

        service.add_recent_standalone_track(&track).unwrap();
        service.clear_recent_standalone_tracks().unwrap();
        assert_eq!(service.get_recent_standalone_tracks(None).unwrap().len(), 0);
    }
}


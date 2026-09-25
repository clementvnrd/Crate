use std::collections::HashSet;
use std::path::{Path, PathBuf};

use lofty::file::AudioFile;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::error::{CrateError, Result};
use crate::models::Track;
use crate::services::cloud_sync::pipeline::{buckets, dirty};
use crate::services::library::macos_bookmark;
use crate::services::library::mik::MikService;
use crate::services::ArtworkService;

/// Status summary of the Mixed In Key database connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MikDatabaseStatus {
    pub is_connected: bool,
    pub db_path: Option<String>,
    pub total_songs: usize,
    pub total_cues: usize,
    pub total_synced_in_crate: usize,
    pub last_sync_time: Option<String>,
}

/// Result summary of a synchronization run from Mixed In Key DB
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MikSyncResult {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub total: usize,
    #[serde(default)]
    pub errors: Vec<String>,
}

/// Raw parsed song from Mixed In Key CoreData SQLite
#[derive(Debug, Clone)]
pub struct MikDbSong {
    pub z_pk: i64,
    pub name: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub label: Option<String>,
    pub comment: Option<String>,
    pub year: Option<i32>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub filesize: Option<i64>,
    pub tempo: Option<f64>,
    pub key: Option<String>,
    pub energy: Option<i32>,
    pub rating: Option<i32>,
    pub file_path: Option<PathBuf>,
    pub cues: Vec<MikDbCue>,
}

#[derive(Debug, Clone)]
pub struct MikDbCue {
    pub z_pk: i64,
    pub time_secs: f64,
    pub energy_level: Option<i32>,
    pub name: Option<String>,
}

pub struct MikDatabaseService;

impl MikDatabaseService {
    /// Detect the Mixed In Key CoreData SQLite database location
    pub fn find_mik_db_path() -> Option<PathBuf> {
        let home_str = std::env::var("HOME").ok()?;
        let home = PathBuf::from(home_str);

        let candidates = [
            home.join("Library/Application Support/Mixedinkey/Collection11.mikdb"),
            home.join("Library/Application Support/Mixed In Key/Collection11.mikdb"),
            home.join("Library/Application Support/com.mixedinkey.application/Collection11.mikdb"),
            home.join("Library/Application Support/Mixedinkey/Collection10.mikdb"),
            home.join("Library/Application Support/Mixed In Key/Collection10.mikdb"),
        ];

        for path in candidates {
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Open connection to Mixed In Key SQLite DB strictly in read-only mode
    pub fn open_mik_db() -> Result<Connection> {
        let db_path = Self::find_mik_db_path()
            .ok_or_else(|| CrateError::Metadata("Mixed In Key database (Collection11.mikdb) not found".into()))?;

        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX;

        let conn = Connection::open_with_flags(&db_path, flags).map_err(|e| {
            CrateError::Metadata(format!("Failed to open Mixed In Key database {}: {e}", db_path.display()))
        })?;

        conn.busy_timeout(std::time::Duration::from_millis(5000)).map_err(|e| {
            CrateError::Metadata(format!("Failed to set busy timeout on Mixed In Key database: {e}"))
        })?;

        let _ = conn.pragma_update(None, "query_only", "ON");

        Ok(conn)
    }

    /// Get overall connection status and counts
    pub fn get_status(crate_conn: &Connection) -> MikDatabaseStatus {
        let db_path = Self::find_mik_db_path();
        let is_connected = db_path.is_some();
        let db_path_str = db_path.as_ref().map(|p| p.to_string_lossy().to_string());

        let mut total_songs = 0;
        let mut total_cues = 0;

        if let Ok(mik_conn) = Self::open_mik_db() {
            let _ = mik_conn
                .query_row("SELECT count(*) FROM ZSONG", [], |r| r.get::<_, i64>(0))
                .map(|c| total_songs = c as usize);
            let _ = mik_conn
                .query_row("SELECT count(*) FROM ZCUEPOINT", [], |r| r.get::<_, i64>(0))
                .map(|c| total_cues = c as usize);
        }

        let total_synced_in_crate = crate_conn
            .query_row(
                "SELECT count(*) FROM tracks WHERE analysis_source = 'mixed_in_key'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map(|c| c as usize)
            .unwrap_or(0);

        MikDatabaseStatus {
            is_connected,
            db_path: db_path_str,
            total_songs,
            total_cues,
            total_synced_in_crate,
            last_sync_time: None,
        }
    }

    /// Read all songs and cues from the Mixed In Key database
    pub fn read_all_songs() -> Result<Vec<MikDbSong>> {
        let mik_conn = Self::open_mik_db()?;

        // 1. Read all cue points indexed by ZSONG
        let mut cues_by_song: std::collections::HashMap<i64, Vec<MikDbCue>> = std::collections::HashMap::new();
        let mut cue_stmt = mik_conn.prepare(
            "SELECT Z_PK, ZSONG, ZTIME, ZENERGYLEVEL, ZNAME FROM ZCUEPOINT ORDER BY ZTIME ASC",
        )?;

        let cue_rows = cue_stmt.query_map([], |row| {
            let z_pk: i64 = row.get(0)?;
            let z_song: i64 = row.get(1)?;
            let time_secs: f64 = row.get(2)?;
            let energy_level: Option<i32> = row.get(3)?;
            let name: Option<String> = row.get(4)?;
            Ok((z_song, MikDbCue { z_pk, time_secs, energy_level, name }))
        })?;

        for cue_res in cue_rows {
            if let Ok((song_id, cue)) = cue_res {
                cues_by_song.entry(song_id).or_default().push(cue);
            }
        }

        // 2. Read all songs
        let mut song_stmt = mik_conn.prepare(
            r#"
            SELECT
                Z_PK, ZNAME, ZARTIST, ZALBUM, ZGENRE, ZLABEL, ZCOMMENT,
                ZYEAR, ZBITRATE, ZSAMPLERATE, ZFILESIZE, ZTEMPO, ZKEY,
                ZENERGY, ZRATING, ZBOOKMARKDATA
            FROM ZSONG
            ORDER BY Z_PK ASC
            "#,
        )?;

        let song_rows = song_stmt.query_map([], |row| {
            let z_pk: i64 = row.get(0)?;
            let name: Option<String> = row.get(1)?;
            let artist: Option<String> = row.get(2)?;
            let album: Option<String> = row.get(3)?;
            let genre: Option<String> = row.get(4)?;
            let label: Option<String> = row.get(5)?;
            let comment: Option<String> = row.get(6)?;
            let year: Option<i32> = row.get(7)?;
            let bitrate: Option<i32> = row.get(8)?;
            let sample_rate: Option<i32> = row.get(9)?;
            let filesize: Option<i64> = row.get(10)?;
            let tempo: Option<f64> = row.get(11)?;
            let key: Option<String> = row.get(12)?;
            let energy_f: Option<f64> = row.get(13)?;
            let rating: Option<i32> = row.get(14)?;
            let bookmark_data: Option<Vec<u8>> = row.get(15)?;

            let energy = energy_f.map(|e| e.round() as i32);

            let file_path = bookmark_data.as_deref().and_then(macos_bookmark::resolve_bookmark);

            let cues = cues_by_song.remove(&z_pk).unwrap_or_default();

            Ok(MikDbSong {
                z_pk,
                name,
                artist,
                album,
                genre,
                label,
                comment,
                year,
                bitrate,
                sample_rate,
                filesize,
                tempo,
                key,
                energy,
                rating,
                file_path,
                cues,
            })
        })?;

        let mut songs = Vec::new();
        for song_res in song_rows {
            if let Ok(song) = song_res {
                songs.push(song);
            }
        }

        Ok(songs)
    }

    /// Prune tracks from Crate DB whose physical audio files have been deleted on disk.
    /// Maintains a 100% clean library without ghost tracks.
    pub fn prune_missing_tracks(
        crate_conn: &Connection,
        artwork_service: Option<&ArtworkService>,
    ) -> Result<usize> {
        let mut stmt = crate_conn.prepare("SELECT id, file_path FROM tracks")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut to_delete = Vec::new();
        for row in rows.flatten() {
            let (id, file_path) = row;
            let p = std::path::Path::new(&file_path);

            let should_prune = if let Some(parent) = p.parent() {
                // If parent directory exists, but file itself does not, it was deleted
                parent.exists() && !p.exists()
            } else {
                !p.exists()
            };

            if should_prune {
                to_delete.push((id, file_path));
            }
        }

        let count = to_delete.len();
        if count > 0 {
            for (id, file_path) in &to_delete {
                log::info!("Pruning ghost track (file physically deleted from disk): id={id}, path={file_path}");
                if let Some(art_svc) = artwork_service {
                    art_svc.delete(id);
                }
                crate_conn.execute("DELETE FROM cues WHERE track_id = ?1", [id])?;
                crate_conn.execute("DELETE FROM track_tags WHERE track_id = ?1", [id])?;
                crate_conn.execute("DELETE FROM playlist_tracks WHERE track_id = ?1", [id])?;
                crate_conn.execute("DELETE FROM tracks WHERE id = ?1", [id])?;

                if let Ok(hlc) = dirty::next_hlc(crate_conn) {
                    let _ = dirty::record_tombstone(crate_conn, buckets::TRACKS_ENTITY, id, &hlc);
                }
                let _ = dirty::mark_dirty(crate_conn, &buckets::bucket_for_track_id(id));
            }
            let _ = dirty::mark_dirty(crate_conn, buckets::PLAYLIST_TRACKS);
            let _ = dirty::mark_dirty(crate_conn, buckets::TRACK_TAGS);
            let _ = dirty::mark_dirty(crate_conn, buckets::CUES);
        }

        Ok(count)
    }

    /// Returns the Crate track matching this title and artist, only if exactly one track matches.
    fn unique_title_artist_match(
        crate_conn: &Connection,
        title: &str,
        artist: &str,
    ) -> Result<Option<(String, String)>> {
        let mut stmt = crate_conn.prepare(
            "SELECT id, file_path FROM tracks WHERE LOWER(TRIM(title)) = LOWER(TRIM(?1)) AND LOWER(TRIM(artist)) = LOWER(TRIM(?2)) LIMIT 2",
        )?;
        let mut matches: Vec<(String, String)> = stmt
            .query_map(rusqlite::params![title, artist], |r| Ok((r.get(0)?, r.get(1)?)))?
            .flatten()
            .collect();
        Ok(if matches.len() == 1 { matches.pop() } else { None })
    }

    /// Full synchronization of Mixed In Key DB into Crate DB with Unicode path normalization.
    /// The Mixed In Key database is only ever opened read-only.
    pub fn sync_all_from_mik_db(
        crate_conn: &Connection,
        artwork_service: Option<&ArtworkService>,
    ) -> Result<MikSyncResult> {
        let mik_songs = Self::read_all_songs()?;
        Self::apply_mik_songs(crate_conn, mik_songs, artwork_service)
    }

    /// Merges Mixed In Key songs into the Crate library.
    ///
    /// Mixed In Key only *enriches* Crate: tracks absent from Mixed In Key are never deleted, and a
    /// title/artist match never removes another Crate track nor steals the path of a file that still exists.
    pub(crate) fn apply_mik_songs(
        crate_conn: &Connection,
        mik_songs: Vec<MikDbSong>,
        artwork_service: Option<&ArtworkService>,
    ) -> Result<MikSyncResult> {
        let mut result = MikSyncResult::default();
        result.total = mik_songs.len();

        let now = chrono::Utc::now().to_rfc3339();
        let mut valid_mik_paths = HashSet::new();
        let mut synced_track_ids = HashSet::new();

        for song in mik_songs {
            let fallback_path_buf;
            let path = match song.file_path {
                Some(ref p) if p.exists() => p.as_path(),
                _ => {
                    // Try to find track in Crate by title & artist to retrieve its path
                    if let (Some(ref title), Some(ref artist)) = (&song.name, &song.artist) {
                        let crate_path = Self::unique_title_artist_match(crate_conn, title, artist)?.map(|(_, p)| p);
                        if let Some(cp) = crate_path {
                            let pb = PathBuf::from(cp);
                            if pb.exists() {
                                fallback_path_buf = pb;
                                fallback_path_buf.as_path()
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
            };

            // Unicode NFC normalization of path to avoid APFS / CoreData decomposed Unicode differences
            let raw_path_str = path.to_string_lossy().to_string();
            let nfc_path_str: String = raw_path_str.nfc().collect();
            let canonical_path_str = std::fs::canonicalize(path)
                .map(|p| p.to_string_lossy().nfc().collect::<String>())
                .unwrap_or_else(|_| nfc_path_str.clone());

            valid_mik_paths.insert(nfc_path_str.clone());
            valid_mik_paths.insert(canonical_path_str.clone());
            valid_mik_paths.insert(raw_path_str.clone());

            // Find all matching track IDs in Crate (exact, NFC, canonical, or raw)
            let mut matching_ids = Vec::new();
            {
                let mut stmt = crate_conn.prepare(
                    "SELECT id, file_path FROM tracks WHERE file_path = ?1 OR file_path = ?2 OR file_path = ?3",
                )?;
                let rows = stmt.query_map(
                    rusqlite::params![&nfc_path_str, &canonical_path_str, &raw_path_str],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                )?;
                for row_res in rows.flatten() {
                    matching_ids.push(row_res);
                }
            }

            if matching_ids.is_empty() {
                // Relocated file: adopt the MIK path only for a single title/artist match whose own
                // file no longer exists. Never merge distinct files that merely share a title.
                if let (Some(ref title), Some(ref artist)) = (&song.name, &song.artist) {
                    if let Some((id, existing_path)) = Self::unique_title_artist_match(crate_conn, title, artist)? {
                        if !Path::new(&existing_path).exists() {
                            matching_ids.push((id, existing_path));
                        }
                    }
                }
            }

            let normalized_key = song.key.as_deref().map(MikService::normalize_key);
            let is_mik_analyzed = song.energy.is_some() || !song.cues.is_empty();
            let is_mik_analyzed_int = if is_mik_analyzed { 1 } else { 0 };

            if !matching_ids.is_empty() {
                // Primary track is the first match
                let (primary_id, _) = matching_ids.remove(0);

                // Delete any duplicate tracks FIRST to clear UNIQUE(file_path) collisions
                for (dup_id, _) in &matching_ids {
                    if let Some(art_svc) = artwork_service {
                        art_svc.delete(dup_id);
                    }
                    crate_conn.execute(
                        "UPDATE OR IGNORE track_tags SET track_id = ?1 WHERE track_id = ?2",
                        rusqlite::params![&primary_id, dup_id],
                    )?;
                    crate_conn.execute(
                        "UPDATE OR IGNORE playlist_tracks SET track_id = ?1 WHERE track_id = ?2",
                        rusqlite::params![&primary_id, dup_id],
                    )?;
                    crate_conn.execute("DELETE FROM cues WHERE track_id = ?1", [dup_id])?;
                    crate_conn.execute("DELETE FROM track_tags WHERE track_id = ?1", [dup_id])?;
                    crate_conn.execute("DELETE FROM playlist_tracks WHERE track_id = ?1", [dup_id])?;
                    crate_conn.execute("DELETE FROM device_tracks WHERE track_id = ?1", [dup_id])?;
                    crate_conn.execute("DELETE FROM tracks WHERE id = ?1", [dup_id])?;
                    if let Ok(hlc) = dirty::next_hlc(crate_conn) {
                        let _ = dirty::record_tombstone(crate_conn, buckets::TRACKS_ENTITY, dup_id, &hlc);
                    }
                    let _ = dirty::mark_dirty(crate_conn, &buckets::bucket_for_track_id(dup_id));
                    log::info!("Pruned duplicate track {dup_id} for file {nfc_path_str}");
                }
                // Clear any track with this nfc_path_str that isn't primary_id
                crate_conn.execute(
                    "DELETE FROM tracks WHERE file_path = ?1 AND id != ?2",
                    rusqlite::params![&nfc_path_str, &primary_id],
                )?;

                // Check if existing track has valid artwork on disk
                let existing_artwork: Option<String> = crate_conn
                    .query_row(
                        "SELECT artwork_path FROM tracks WHERE id = ?1",
                        [&primary_id],
                        |r| r.get(0),
                    )
                    .ok()
                    .flatten();

                let mut artwork_update = existing_artwork;
                let mut artwork_source_update = None;

                if artwork_update.is_none() {
                    if let Some(art_svc) = artwork_service {
                        if let Some(tagged_file) = MikService::read_metadata_lenient(path) {
                            if let Some(art_path) = art_svc.extract_from_tagged_file_or_folder(&tagged_file, path, &primary_id) {
                                artwork_update = Some(art_path);
                                artwork_source_update = Some("extracted".to_string());
                            }
                        }
                    }
                }

                let normalized_bitrate = song.bitrate.map(|b| if b > 10000 { (b + 500) / 1000 } else { b });
                let hlc = dirty::next_hlc(crate_conn)?;

                // Update primary track with latest MIK metadata, artwork & ensure path is normalized NFC
                crate_conn.execute(
                    r#"
                    UPDATE tracks
                    SET bpm = COALESCE(?1, bpm),
                        key = COALESCE(?2, key),
                        energy = COALESCE(?3, energy),
                        title = COALESCE(?4, title),
                        artist = COALESCE(?5, artist),
                        album = COALESCE(?6, album),
                        genre = COALESCE(?7, genre),
                        label = COALESCE(?8, label),
                        year = COALESCE(?9, year),
                        file_path = ?10,
                        artwork_path = COALESCE(?11, artwork_path),
                        artwork_source = COALESCE(?12, artwork_source),
                        bitrate = COALESCE(?13, bitrate),
                        analysis_source = CASE WHEN ?14 = 1 THEN 'mixed_in_key' ELSE analysis_source END,
                        date_modified = ?15,
                        _hlc = ?16
                    WHERE id = ?17
                    "#,
                    rusqlite::params![
                        song.tempo,
                        normalized_key,
                        song.energy,
                        song.name,
                        song.artist,
                        song.album,
                        song.genre,
                        song.label,
                        song.year,
                        nfc_path_str,
                        artwork_update,
                        artwork_source_update,
                        normalized_bitrate,
                        is_mik_analyzed_int,
                        now,
                        hlc,
                        primary_id,
                    ],
                )?;

                // Sync cue points for primary track
                if !song.cues.is_empty() {
                    crate_conn.execute("DELETE FROM cues WHERE track_id = ?1", [&primary_id])?;

                    for (idx, mik_cue) in song.cues.iter().enumerate() {
                        let cue_hlc = dirty::next_hlc(crate_conn)?;
                        let pos_ms = (mik_cue.time_secs * 1000.0).max(0.0).round() as i64;
                        let cue_id = uuid::Uuid::new_v4().to_string();
                        let cue_name = mik_cue.name.clone().or_else(|| {
                            mik_cue.energy_level.map(|e| format!("Hot Cue {} (Energy {})", idx + 1, e))
                        }).or_else(|| Some(format!("Hot Cue {}", idx + 1)));

                        crate_conn.execute(
                            r#"
                            INSERT INTO cues (id, track_id, position_ms, type, loop_end_ms, hot_cue_index, name, color, _hlc)
                            VALUES (?1, ?2, ?3, 'hot', NULL, ?4, ?5, NULL, ?6)
                            "#,
                            rusqlite::params![
                                cue_id,
                                primary_id,
                                pos_ms,
                                (idx as i32) + 1,
                                cue_name,
                                cue_hlc,
                            ],
                        )?;
                    }
                }

                dirty::mark_dirty(crate_conn, &buckets::bucket_for_track_id(&primary_id))?;
                synced_track_ids.insert(primary_id);
                result.updated += 1;
            } else {
                // Clean up any stale track that might hold nfc_path_str before new insert
                crate_conn.execute("DELETE FROM cues WHERE track_id IN (SELECT id FROM tracks WHERE file_path = ?1)", [&nfc_path_str])?;
                crate_conn.execute("DELETE FROM track_tags WHERE track_id IN (SELECT id FROM tracks WHERE file_path = ?1)", [&nfc_path_str])?;
                crate_conn.execute("DELETE FROM playlist_tracks WHERE track_id IN (SELECT id FROM tracks WHERE file_path = ?1)", [&nfc_path_str])?;
                crate_conn.execute("DELETE FROM device_tracks WHERE track_id IN (SELECT id FROM tracks WHERE file_path = ?1)", [&nfc_path_str])?;
                crate_conn.execute("DELETE FROM tracks WHERE file_path = ?1", [&nfc_path_str])?;

                // New track from Mixed In Key - import into Crate
                let track_id = uuid::Uuid::new_v4().to_string();
                let file_hash = crate::services::hash::compute_audio_hash(path).ok();
                let format = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("mp3")
                    .to_lowercase();

                let mut duration_ms = 0i64;
                let mut bitrate = song.bitrate;
                let mut sample_rate = song.sample_rate;
                let mut artwork_path = None;
                let mut artwork_source = None;

                // Read audio properties & artwork with lenient Lofty probe
                if let Some(tagged_file) = MikService::read_metadata_lenient(path) {
                    let props = tagged_file.properties();
                    duration_ms = props.duration().as_millis() as i64;
                    if bitrate.is_none() {
                        bitrate = props.audio_bitrate().map(|b| b as i32);
                    }
                    if sample_rate.is_none() {
                        sample_rate = props.sample_rate().map(|s| s as i32);
                    }
                    if let Some(art_svc) = artwork_service {
                        if let Some(art_path) = art_svc.extract_from_tagged_file_or_folder(&tagged_file, path, &track_id) {
                            artwork_path = Some(art_path);
                            artwork_source = Some("extracted".to_string());
                        }
                    }
                }

                if (format == "wav" || format == "aiff") && (bitrate.is_none() || bitrate == Some(0) || bitrate.unwrap_or(0) <= 10) {
                    let sr = sample_rate.unwrap_or(44100);
                    bitrate = Some((sr * 2 * 24 + 500) / 1000);
                }

                let normalized_bitrate = bitrate.map(|b| if b > 10000 { (b + 500) / 1000 } else { b });
                let hlc = dirty::next_hlc(crate_conn)?;

                let track = Track {
                    id: track_id.clone(),
                    file_path: nfc_path_str.clone(),
                    file_hash,
                    title: song.name.clone().or_else(|| {
                        path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
                    }),
                    artist: song.artist.clone(),
                    album: song.album.clone(),
                    year: song.year,
                    genre: song.genre.clone(),
                    label: song.label.clone(),
                    catalog_number: None,
                    duration_ms,
                    bpm: song.tempo,
                    key: normalized_key,
                    energy: song.energy,
                    bitrate: normalized_bitrate,
                    sample_rate,
                    format,
                    analysis_source: if is_mik_analyzed {
                        Some("mixed_in_key".to_string())
                    } else {
                        None
                    },
                    waveform_data: None,
                    rating: song.rating.unwrap_or(0),
                    play_count: 0,
                    date_added: now.clone(),
                    date_modified: now.clone(),
                    last_played: None,
                    rekordbox_id: None,
                    artwork_path,
                    artwork_source,
                    color: None,
                    library_root_id: None,
                    relative_path: None,
                    tags: vec![],
                };

                // Insert into Crate tracks table
                crate_conn.execute(
                    r#"
                    INSERT INTO tracks (
                        id, file_path, file_hash, title, artist, album, year, genre,
                        label, catalog_number, duration_ms, bpm, key, energy, bitrate,
                        sample_rate, format, analysis_source, waveform_data, rating,
                        play_count, date_added, date_modified, last_played, rekordbox_id,
                        artwork_path, artwork_source, color, library_root_id, relative_path,
                        _hlc
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                        ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                        ?16, ?17, ?18, ?19, ?20,
                        ?21, ?22, ?23, ?24, ?25,
                        ?26, ?27, ?28, ?29, ?30,
                        ?31
                    )
                    "#,
                    rusqlite::params![
                        track.id,
                        track.file_path,
                        track.file_hash,
                        track.title,
                        track.artist,
                        track.album,
                        track.year,
                        track.genre,
                        track.label,
                        track.catalog_number,
                        track.duration_ms,
                        track.bpm,
                        track.key,
                        track.energy,
                        track.bitrate,
                        track.sample_rate,
                        track.format,
                        track.analysis_source,
                        track.waveform_data,
                        track.rating,
                        track.play_count,
                        track.date_added,
                        track.date_modified,
                        track.last_played,
                        track.rekordbox_id,
                        track.artwork_path,
                        track.artwork_source,
                        track.color,
                        track.library_root_id,
                        track.relative_path,
                        hlc,
                    ],
                )?;

                // Insert cues if any
                for (idx, mik_cue) in song.cues.iter().enumerate() {
                    let cue_hlc = dirty::next_hlc(crate_conn)?;
                    let pos_ms = (mik_cue.time_secs * 1000.0).max(0.0).round() as i64;
                    let cue_id = uuid::Uuid::new_v4().to_string();
                    let cue_name = mik_cue.name.clone().or_else(|| {
                        mik_cue.energy_level.map(|e| format!("Hot Cue {} (Energy {})", idx + 1, e))
                    }).or_else(|| Some(format!("Hot Cue {}", idx + 1)));

                    crate_conn.execute(
                        r#"
                        INSERT INTO cues (id, track_id, position_ms, type, loop_end_ms, hot_cue_index, name, color, _hlc)
                        VALUES (?1, ?2, ?3, 'hot', NULL, ?4, ?5, NULL, ?6)
                        "#,
                        rusqlite::params![
                            cue_id,
                            track_id,
                            pos_ms,
                            (idx as i32) + 1,
                            cue_name,
                            cue_hlc,
                        ],
                    )?;
                }

                dirty::mark_dirty(crate_conn, &buckets::bucket_for_track_id(&track_id))?;
                synced_track_ids.insert(track_id);
                result.added += 1;
            }
        }

        // Tracks absent from Mixed In Key are kept: Mixed In Key enriches Crate, it does not own it.
        let untouched: i64 = crate_conn.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))?;
        let untouched = (untouched as usize).saturating_sub(synced_track_ids.len());
        if untouched > 0 {
            log::debug!("{untouched} Crate track(s) are not in Mixed In Key and were left untouched");
        }

        // Final artwork extraction pass for any tracks with missing artwork
        if let Some(art_svc) = artwork_service {
            if let Ok(mut missing_stmt) = crate_conn.prepare("SELECT id, file_path FROM tracks WHERE artwork_path IS NULL") {
                let missing_rows: Vec<(String, String)> = missing_stmt
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                    .map(|iter| iter.flatten().collect())
                    .unwrap_or_default();

                for (tid, fpath) in missing_rows {
                    let p = PathBuf::from(&fpath);
                    if p.exists() {
                        if let Some(tagged) = MikService::read_metadata_lenient(&p) {
                            if let Some(art_path) = art_svc.extract_from_tagged_file_or_folder(&tagged, &p, &tid) {
                                if let Ok(hlc) = dirty::next_hlc(crate_conn) {
                                    let _ = crate_conn.execute(
                                        "UPDATE tracks SET artwork_path = ?1, artwork_source = 'extracted', _hlc = ?2 WHERE id = ?3",
                                        rusqlite::params![art_path, hlc, tid],
                                    );
                                    let _ = dirty::mark_dirty(crate_conn, &buckets::bucket_for_track_id(&tid));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Normalize all legacy bitrates in DB from bps to kbps (e.g. 806807 -> 807, 320000 -> 320, 2116800 -> 2117)
        let _ = crate_conn.execute("UPDATE tracks SET bitrate = (bitrate + 500) / 1000 WHERE bitrate > 10000", []);

        // Fix any tracks where bitrate was corrupted to <= 10 (e.g. 2 kbps) on lossless/uncompressed tracks
        let _ = crate_conn.execute(
            "UPDATE tracks SET bitrate = ROUND((COALESCE(sample_rate, 44100) * 2 * 24) / 1000) WHERE (format = 'wav' OR format = 'aiff' OR format = 'flac') AND bitrate <= 10",
            [],
        );

        dirty::mark_dirty(crate_conn, buckets::CUES)?;

        log::info!(
            "Mixed In Key DB Sync complete: {} added, {} updated, {} removed out of {} MIK songs",
            result.added,
            result.updated,
            result.removed,
            result.total
        );

        Ok(result)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_mik_db_path() {
        let path = MikDatabaseService::find_mik_db_path();
        if let Some(ref p) = path {
            assert!(p.exists());
            assert!(p.to_string_lossy().contains("Collection11.mikdb") || p.to_string_lossy().contains("Collection10.mikdb"));
        }
    }

    #[test]
    fn test_read_mik_database() {
        if let Ok(songs) = MikDatabaseService::read_all_songs() {
            assert!(!songs.is_empty(), "Should read songs from Mixed In Key database");
            let song = &songs[0];
            assert!(song.tempo.is_some() || song.key.is_some() || song.name.is_some());
        }
    }

    fn temp_library(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("crate_mik_sync_{suffix}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(dir: &Path, name: &str) -> String {
        let path = dir.join(name);
        std::fs::write(&path, b"not really audio").unwrap();
        path.to_string_lossy().nfc().collect()
    }

    fn crate_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        conn
    }

    fn insert_track(conn: &Connection, id: &str, path: &str, title: &str, artist: &str) {
        conn.execute(
            "INSERT INTO tracks (id, file_path, title, artist, duration_ms, date_added, date_modified)
             VALUES (?1, ?2, ?3, ?4, 180000, '2026-01-01', '2026-01-01')",
            rusqlite::params![id, path, title, artist],
        )
        .unwrap();
    }

    fn mik_song(path: &str, title: &str, artist: &str) -> MikDbSong {
        MikDbSong {
            z_pk: 1,
            name: Some(title.into()),
            artist: Some(artist.into()),
            album: None,
            genre: None,
            label: None,
            comment: None,
            year: None,
            bitrate: None,
            sample_rate: None,
            filesize: None,
            tempo: Some(124.0),
            key: Some("8A".into()),
            energy: Some(6),
            rating: None,
            file_path: Some(PathBuf::from(path)),
            cues: vec![],
        }
    }

    fn track_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn test_sync_keeps_tracks_absent_from_mik() {
        let dir = temp_library("absent");
        let conn = crate_db();
        let in_mik = touch(&dir, "in_mik.mp3");
        let not_in_mik = touch(&dir, "not_in_mik.mp3");
        insert_track(&conn, "a", &in_mik, "In MIK", "Artist");
        insert_track(&conn, "b", &not_in_mik, "Not in MIK", "Artist");

        let result = MikDatabaseService::apply_mik_songs(&conn, vec![mik_song(&in_mik, "In MIK", "Artist")], None).unwrap();

        assert_eq!(result.removed, 0);
        assert_eq!(result.updated, 1);
        assert_eq!(track_count(&conn), 2, "a track missing from Mixed In Key must never be deleted");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_sync_title_match_never_merges_distinct_files() {
        let dir = temp_library("homonyms");
        let conn = crate_db();
        let original = touch(&dir, "song_original.mp3");
        let extended = touch(&dir, "song_extended.flac");
        let other = touch(&dir, "song_other.mp3");
        insert_track(&conn, "orig", &original, "Song", "Artist");
        insert_track(&conn, "ext", &extended, "Song", "Artist");

        MikDatabaseService::apply_mik_songs(&conn, vec![mik_song(&other, "Song", "Artist")], None).unwrap();

        assert_eq!(track_count(&conn), 3);
        let orig_path: String = conn.query_row("SELECT file_path FROM tracks WHERE id = 'orig'", [], |r| r.get(0)).unwrap();
        let ext_path: String = conn.query_row("SELECT file_path FROM tracks WHERE id = 'ext'", [], |r| r.get(0)).unwrap();
        assert_eq!(orig_path, original);
        assert_eq!(ext_path, extended);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_sync_relocated_file_keeps_track_identity() {
        let dir = temp_library("relocated");
        let conn = crate_db();
        let old_path = dir.join("old_folder/track.mp3").to_string_lossy().to_string();
        let new_path = touch(&dir, "track.mp3");
        insert_track(&conn, "keep-me", &old_path, "Track", "Artist");
        conn.execute("INSERT INTO tags (id, name, color) VALUES ('t1', 'Peak', '#fff')", []).ok();
        conn.execute("INSERT INTO track_tags (track_id, tag_id) VALUES ('keep-me', 't1')", []).ok();

        MikDatabaseService::apply_mik_songs(&conn, vec![mik_song(&new_path, "Track", "Artist")], None).unwrap();

        assert_eq!(track_count(&conn), 1);
        let path: String = conn.query_row("SELECT file_path FROM tracks WHERE id = 'keep-me'", [], |r| r.get(0)).unwrap();
        assert_eq!(path, new_path, "the existing track follows its file instead of being re-imported");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_prune_missing_tracks() {
        let dir = temp_library("prune");
        let conn = crate_db();
        let ghost = dir.join("ghost_track.flac").to_string_lossy().to_string();
        let present = touch(&dir, "present.flac");
        insert_track(&conn, "ghost-1", &ghost, "Ghost Track", "Ghost Artist");
        insert_track(&conn, "present-1", &present, "Present", "Artist");

        let pruned = MikDatabaseService::prune_missing_tracks(&conn, None).unwrap();
        assert_eq!(pruned, 1, "Should prune exactly 1 ghost track");
        assert_eq!(track_count(&conn), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

}


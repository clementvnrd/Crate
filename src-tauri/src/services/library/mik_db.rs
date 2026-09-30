use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lofty::file::AudioFile;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
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
    /// Read for completeness of the Core Data row; not used by the sync yet.
    #[allow(dead_code)]
    pub comment: Option<String>,
    pub year: Option<i32>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    #[allow(dead_code)]
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

/// How long a synchronisation pass keeps the database lock before it lets other commands in.
const LOCK_BUDGET: Duration = Duration::from_millis(50);

/// What one synchronisation pass accumulates while it goes through the songs.
struct SyncPass {
    result: MikSyncResult,
    cues_changed: bool,
    /// Tracks added or changed by the pass: they get an artwork pass afterwards.
    touched_ids: Vec<String>,
    valid_mik_paths: HashSet<String>,
    synced_track_ids: HashSet<String>,
}

impl SyncPass {
    fn new(total: usize) -> Self {
        Self {
            result: MikSyncResult {
                total,
                ..Default::default()
            },
            cues_changed: false,
            touched_ids: Vec::new(),
            valid_mik_paths: HashSet::new(),
            synced_track_ids: HashSet::new(),
        }
    }

    fn into_result(self) -> MikSyncResult {
        log::info!(
            "Mixed In Key DB Sync complete: {} added, {} updated, {} removed out of {} MIK songs",
            self.result.added,
            self.result.updated,
            self.result.removed,
            self.result.total
        );
        self.result
    }
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

        candidates.into_iter().find(|path| path.exists())
    }

    /// Open connection to Mixed In Key SQLite DB strictly in read-only mode
    pub fn open_mik_db() -> Result<Connection> {
        let db_path = Self::find_mik_db_path().ok_or_else(|| {
            CrateError::Metadata("Mixed In Key database (Collection11.mikdb) not found".into())
        })?;

        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;

        let conn = Connection::open_with_flags(&db_path, flags).map_err(|e| {
            CrateError::Metadata(format!(
                "Failed to open Mixed In Key database {}: {e}",
                db_path.display()
            ))
        })?;

        conn.busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(|e| {
                CrateError::Metadata(format!(
                    "Failed to set busy timeout on Mixed In Key database: {e}"
                ))
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
        let mut cues_by_song: std::collections::HashMap<i64, Vec<MikDbCue>> =
            std::collections::HashMap::new();
        let mut cue_stmt = mik_conn.prepare(
            "SELECT Z_PK, ZSONG, ZTIME, ZENERGYLEVEL, ZNAME FROM ZCUEPOINT ORDER BY ZTIME ASC",
        )?;

        let cue_rows = cue_stmt.query_map([], |row| {
            let z_pk: i64 = row.get(0)?;
            let z_song: i64 = row.get(1)?;
            let time_secs: f64 = row.get(2)?;
            let energy_level: Option<i32> = row.get(3)?;
            let name: Option<String> = row.get(4)?;
            Ok((
                z_song,
                MikDbCue {
                    z_pk,
                    time_secs,
                    energy_level,
                    name,
                },
            ))
        })?;

        for (song_id, cue) in cue_rows.flatten() {
            cues_by_song.entry(song_id).or_default().push(cue);
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

            let file_path = bookmark_data
                .as_deref()
                .and_then(macos_bookmark::resolve_bookmark);

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
        songs.extend(song_rows.flatten());

        Ok(songs)
    }

    // Pruning the tracks whose audio file was deleted from disk (a library with no ghost tracks)
    // takes three steps, so that the disk is not checked while the database is locked:
    // `track_paths` (database), `is_missing_on_disk` (disk), `prune_tracks` (database).
    // `LibraryService::prune_missing_tracks` runs them with a short lock hold around each
    // database step.

    /// Step 1 of a prune: every track's `(id, file_path)`. Needs the database, not the disk.
    pub fn track_paths(crate_conn: &Connection) -> Result<Vec<(String, String)>> {
        let mut stmt = crate_conn.prepare("SELECT id, file_path FROM tracks")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        Ok(rows.flatten().collect())
    }

    /// Step 2 of a prune: whether the file was deleted from a folder that is still there. A folder
    /// that is gone as well (a drive that is not plugged in) proves nothing, so it is not pruned.
    /// Touches the disk only: call it without holding the database lock.
    pub fn is_missing_on_disk(file_path: &str) -> bool {
        let p = std::path::Path::new(file_path);
        match p.parent() {
            Some(parent) => parent.exists() && !p.exists(),
            None => !p.exists(),
        }
    }

    /// Step 3 of a prune: delete the given `(id, file_path)` ghosts in one transaction. A ghost is
    /// only deleted if the track still has the path it had when the disk was checked, so a track
    /// that was moved or re-imported in between is never pruned on stale information. Returns how
    /// many tracks were deleted. A failure part-way deletes nothing, and the artwork files go only
    /// after the rows are committed.
    pub fn prune_tracks(
        crate_conn: &Connection,
        artwork_service: Option<&ArtworkService>,
        ghosts: &[(String, String)],
    ) -> Result<usize> {
        if ghosts.is_empty() {
            return Ok(0);
        }

        let tx = crate_conn.unchecked_transaction()?;
        let mut pruned_ids: Vec<&str> = Vec::new();
        for (id, file_path) in ghosts {
            let current: Option<String> = tx
                .query_row("SELECT file_path FROM tracks WHERE id = ?1", [id], |r| {
                    r.get(0)
                })
                .optional()?;
            if current.as_deref() != Some(file_path.as_str()) {
                continue;
            }

            log::info!(
                "Pruning ghost track (file physically deleted from disk): id={id}, path={file_path}"
            );
            tx.execute("DELETE FROM cues WHERE track_id = ?1", [id])?;
            tx.execute("DELETE FROM track_tags WHERE track_id = ?1", [id])?;
            tx.execute("DELETE FROM playlist_tracks WHERE track_id = ?1", [id])?;
            tx.execute("DELETE FROM tracks WHERE id = ?1", [id])?;

            if let Ok(hlc) = dirty::next_hlc(&tx) {
                let _ = dirty::record_tombstone(&tx, buckets::TRACKS_ENTITY, id, &hlc);
            }
            let _ = dirty::mark_dirty(&tx, &buckets::bucket_for_track_id(id));
            pruned_ids.push(id);
        }

        if !pruned_ids.is_empty() {
            let _ = dirty::mark_dirty(&tx, buckets::PLAYLIST_TRACKS);
            let _ = dirty::mark_dirty(&tx, buckets::TRACK_TAGS);
            let _ = dirty::mark_dirty(&tx, buckets::CUES);
        }
        tx.commit()?;

        if let Some(art_svc) = artwork_service {
            for id in &pruned_ids {
                art_svc.delete(id);
            }
        }
        Ok(pruned_ids.len())
    }

    /// Mirrors the Mixed In Key cues of one track, touching only cues that came from Mixed In Key.
    ///
    /// MIK cues get a deterministic id (`mik-<track>-<n>`), so re-syncing updates them in place
    /// instead of recreating them; cues created in Crate are left alone. Legacy copies of MIK cues
    /// (random ids from earlier builds, same position) are replaced once. Returns true on change.
    pub(crate) fn sync_mik_cues(
        crate_conn: &Connection,
        track_id: &str,
        mik_cues: &[MikDbCue],
    ) -> Result<bool> {
        let prefix = format!("mik-{track_id}-");
        let mut changed = false;
        let mut wanted_ids = Vec::with_capacity(mik_cues.len());

        for (idx, mik_cue) in mik_cues.iter().enumerate() {
            let cue_id = format!("{prefix}{idx}");
            let pos_ms = (mik_cue.time_secs * 1000.0).max(0.0).round() as i64;
            // Hot cue slots are 0-based everywhere in Crate (A = 0 … H = 7), like upstream and Rekordbox.
            let hot_index = idx as i32;
            let name = mik_cue
                .name
                .clone()
                .or_else(|| {
                    mik_cue
                        .energy_level
                        .map(|e| format!("Hot Cue {} (Energy {})", idx + 1, e))
                })
                .or_else(|| Some(format!("Hot Cue {}", idx + 1)));

            // Legacy copy of this MIK cue with a random id: same position, not ours.
            changed |= crate_conn.execute(
                "DELETE FROM cues WHERE track_id = ?1 AND id NOT LIKE ?2 AND ABS(position_ms - ?3) <= 2",
                rusqlite::params![track_id, format!("{prefix}%"), pos_ms],
            )? > 0;

            let hlc = dirty::next_hlc(crate_conn)?;
            changed |= crate_conn.execute(
                r#"
                INSERT INTO cues (id, track_id, position_ms, type, loop_end_ms, hot_cue_index, name, color, _hlc)
                VALUES (?1, ?2, ?3, 'hot', NULL, ?4, ?5, NULL, ?6)
                ON CONFLICT(id) DO UPDATE SET
                    position_ms = excluded.position_ms, hot_cue_index = excluded.hot_cue_index,
                    name = excluded.name, _hlc = excluded._hlc
                WHERE position_ms IS NOT excluded.position_ms OR hot_cue_index IS NOT excluded.hot_cue_index
                    OR name IS NOT excluded.name
                "#,
                rusqlite::params![cue_id, track_id, pos_ms, hot_index, name, hlc],
            )? > 0;
            wanted_ids.push(cue_id);
        }

        // MIK cues that no longer exist in Mixed In Key
        let mut stmt =
            crate_conn.prepare("SELECT id FROM cues WHERE track_id = ?1 AND id LIKE ?2")?;
        let existing: Vec<String> = stmt
            .query_map(rusqlite::params![track_id, format!("{prefix}%")], |r| {
                r.get(0)
            })?
            .flatten()
            .collect();
        for id in existing.iter().filter(|id| !wanted_ids.contains(id)) {
            crate_conn.execute("DELETE FROM cues WHERE id = ?1", [id])?;
            changed = true;
        }
        Ok(changed)
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
            .query_map(rusqlite::params![title, artist], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .flatten()
            .collect();
        Ok(if matches.len() == 1 {
            matches.pop()
        } else {
            None
        })
    }

    /// Full synchronization of Mixed In Key DB into Crate DB with Unicode path normalization.
    /// The Mixed In Key database is only ever opened read-only, and it is read before the Crate
    /// database lock is taken.
    pub fn sync_all_from_mik_db(
        crate_conn: &Mutex<Connection>,
        artwork_service: Option<&ArtworkService>,
    ) -> Result<MikSyncResult> {
        let mik_songs = Self::read_all_songs()?;
        Self::apply_mik_songs_shared(crate_conn, mik_songs, artwork_service)
    }

    /// Same decisions as [`Self::apply_mik_songs`], but the database lock is only held for short
    /// batches of songs instead of the whole pass.
    ///
    /// Adding a track hashes the audio, reads its tags and encodes its cover: a first import takes
    /// minutes, and holding the lock for all of it froze every command that needs the database.
    /// The songs are still applied one after the other, in order, by the same code (what one song
    /// decides depends on what the songs before it did), so nothing changes for the result; only
    /// the lock is let go every [`LOCK_BUDGET`]. A failure rolls back the batch it happened in and
    /// keeps the batches before it, which is safe because a sync can simply be run again.
    pub(crate) fn apply_mik_songs_shared(
        crate_conn: &Mutex<Connection>,
        mik_songs: Vec<MikDbSong>,
        artwork_service: Option<&ArtworkService>,
    ) -> Result<MikSyncResult> {
        Self::apply_mik_songs_in_batches(
            crate_conn,
            mik_songs,
            artwork_service,
            LOCK_BUDGET,
            // `std::sync::Mutex` is not fair: without a pause the same thread would usually take
            // the lock straight back before a waiting command woke up.
            &mut || std::thread::sleep(Duration::from_millis(2)),
        )
    }

    /// [`Self::apply_mik_songs_shared`] with the budget and the pause made explicit, for tests.
    /// `between_batches` runs each time the lock has just been released.
    fn apply_mik_songs_in_batches(
        crate_conn: &Mutex<Connection>,
        mik_songs: Vec<MikDbSong>,
        artwork_service: Option<&ArtworkService>,
        budget: Duration,
        between_batches: &mut dyn FnMut(),
    ) -> Result<MikSyncResult> {
        let mut pass = SyncPass::new(mik_songs.len());
        let now = chrono::Utc::now().to_rfc3339();

        let mut songs = mik_songs.into_iter().peekable();
        while songs.peek().is_some() {
            let guard = crate_conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            let tx = guard.unchecked_transaction()?;
            let started = Instant::now();
            for song in songs.by_ref() {
                Self::apply_song(&tx, song, &now, artwork_service, &mut pass)?;
                if started.elapsed() >= budget {
                    break;
                }
            }
            tx.commit()?;
            drop(guard);
            between_batches();
        }

        // Covers of the tracks the pass added or changed: another file read per track.
        let touched_ids = if artwork_service.is_some() {
            std::mem::take(&mut pass.touched_ids)
        } else {
            Vec::new()
        };
        let mut touched = touched_ids.into_iter().peekable();
        while touched.peek().is_some() {
            let guard = crate_conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            let tx = guard.unchecked_transaction()?;
            let started = Instant::now();
            for id in touched.by_ref() {
                Self::fill_missing_artwork(&tx, artwork_service, &id)?;
                if started.elapsed() >= budget {
                    break;
                }
            }
            tx.commit()?;
            drop(guard);
            between_batches();
        }

        let guard = crate_conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let tx = guard.unchecked_transaction()?;
        Self::finish_pass(&tx, &pass)?;
        tx.commit()?;

        Ok(pass.into_result())
    }

    /// Merges Mixed In Key songs into the Crate library.
    ///
    /// Mixed In Key only *enriches* Crate: tracks absent from Mixed In Key are never deleted, and a
    /// title/artist match never removes another Crate track nor steals the path of a file that still exists.
    ///
    /// The whole pass is one transaction on a connection the caller already holds. Application code
    /// uses [`Self::apply_mik_songs_shared`], which releases the database lock regularly instead;
    /// this version stays as the reference the tests compare it with.
    #[cfg(test)]
    pub(crate) fn apply_mik_songs(
        crate_conn: &Connection,
        mik_songs: Vec<MikDbSong>,
        artwork_service: Option<&ArtworkService>,
    ) -> Result<MikSyncResult> {
        let mut pass = SyncPass::new(mik_songs.len());
        let now = chrono::Utc::now().to_rfc3339();

        // One transaction for the whole pass: atomic, and far fewer disk syncs.
        let tx = crate_conn.unchecked_transaction()?;
        for song in mik_songs {
            Self::apply_song(&tx, song, &now, artwork_service, &mut pass)?;
        }
        for id in std::mem::take(&mut pass.touched_ids) {
            Self::fill_missing_artwork(&tx, artwork_service, &id)?;
        }
        Self::finish_pass(&tx, &pass)?;
        tx.commit()?;

        Ok(pass.into_result())
    }

    /// Applies one Mixed In Key song: decides which Crate track it belongs to (or adds one) and
    /// enriches it. The decisions depend on what the songs before it in the same pass did, so
    /// songs must be applied in order, but each call only needs the connection for its own work.
    fn apply_song(
        crate_conn: &Connection,
        song: MikDbSong,
        now: &str,
        artwork_service: Option<&ArtworkService>,
        pass: &mut SyncPass,
    ) -> Result<()> {
        let fallback_path_buf;
        let path = match song.file_path {
            Some(ref p) if p.exists() => p.as_path(),
            _ => {
                // Try to find track in Crate by title & artist to retrieve its path
                if let (Some(ref title), Some(ref artist)) = (&song.name, &song.artist) {
                    let crate_path =
                        Self::unique_title_artist_match(crate_conn, title, artist)?.map(|(_, p)| p);
                    if let Some(cp) = crate_path {
                        let pb = PathBuf::from(cp);
                        if pb.exists() {
                            fallback_path_buf = pb;
                            fallback_path_buf.as_path()
                        } else {
                            return Ok(());
                        }
                    } else {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                }
            }
        };

        // Unicode NFC normalization of path to avoid APFS / CoreData decomposed Unicode differences
        let raw_path_str = path.to_string_lossy().to_string();
        let nfc_path_str: String = raw_path_str.nfc().collect();
        let canonical_path_str = std::fs::canonicalize(path)
            .map(|p| p.to_string_lossy().nfc().collect::<String>())
            .unwrap_or_else(|_| nfc_path_str.clone());

        pass.valid_mik_paths.insert(nfc_path_str.clone());
        pass.valid_mik_paths.insert(canonical_path_str.clone());
        pass.valid_mik_paths.insert(raw_path_str.clone());

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
                if let Some((id, existing_path)) =
                    Self::unique_title_artist_match(crate_conn, title, artist)?
                {
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
                    let _ =
                        dirty::record_tombstone(crate_conn, buckets::TRACKS_ENTITY, dup_id, &hlc);
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
                        if let Some(art_path) = art_svc.extract_from_tagged_file_or_folder(
                            &tagged_file,
                            path,
                            &primary_id,
                        ) {
                            artwork_update = Some(art_path);
                            artwork_source_update = Some("extracted".to_string());
                        }
                    }
                }
            }

            let normalized_bitrate = song
                .bitrate
                .map(|b| if b > 10000 { (b + 500) / 1000 } else { b });
            let hlc = dirty::next_hlc(crate_conn)?;

            // Enrich the existing track. Analysis data (BPM, key, energy) follows Mixed In Key;
            // descriptive tags only fill empty fields, so edits made in Crate are never overwritten.
            // The WHERE clause skips the write entirely when nothing would change.
            let changed = crate_conn.execute(
                r#"
                UPDATE tracks
                SET bpm = COALESCE(?1, bpm),
                    key = COALESCE(?2, key),
                    energy = COALESCE(?3, energy),
                    title = COALESCE(title, ?4),
                    artist = COALESCE(artist, ?5),
                    album = COALESCE(album, ?6),
                    genre = COALESCE(genre, ?7),
                    label = COALESCE(label, ?8),
                    year = COALESCE(year, ?9),
                    file_path = ?10,
                    artwork_path = COALESCE(?11, artwork_path),
                    artwork_source = COALESCE(?12, artwork_source),
                    bitrate = COALESCE(?13, bitrate),
                    analysis_source = CASE WHEN ?14 = 1 THEN 'mixed_in_key' ELSE analysis_source END,
                    date_modified = ?15,
                    _hlc = ?16
                WHERE id = ?17 AND (
                    bpm IS NOT COALESCE(?1, bpm) OR key IS NOT COALESCE(?2, key)
                    OR energy IS NOT COALESCE(?3, energy)
                    OR (title IS NULL AND ?4 IS NOT NULL) OR (artist IS NULL AND ?5 IS NOT NULL)
                    OR (album IS NULL AND ?6 IS NOT NULL) OR (genre IS NULL AND ?7 IS NOT NULL)
                    OR (label IS NULL AND ?8 IS NOT NULL) OR (year IS NULL AND ?9 IS NOT NULL)
                    OR file_path IS NOT ?10
                    OR artwork_path IS NOT COALESCE(?11, artwork_path)
                    OR bitrate IS NOT COALESCE(?13, bitrate)
                    OR (?14 = 1 AND analysis_source IS NOT 'mixed_in_key')
                )
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

            let track_cues_changed = Self::sync_mik_cues(crate_conn, &primary_id, &song.cues)?;
            pass.cues_changed |= track_cues_changed;

            if changed > 0 || track_cues_changed {
                dirty::mark_dirty(crate_conn, &buckets::bucket_for_track_id(&primary_id))?;
                pass.touched_ids.push(primary_id.clone());
                pass.result.updated += 1;
            }
            pass.synced_track_ids.insert(primary_id);
        } else {
            // Clean up any stale track that might hold nfc_path_str before new insert
            crate_conn.execute(
                "DELETE FROM cues WHERE track_id IN (SELECT id FROM tracks WHERE file_path = ?1)",
                [&nfc_path_str],
            )?;
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
                    if let Some(art_path) =
                        art_svc.extract_from_tagged_file_or_folder(&tagged_file, path, &track_id)
                    {
                        artwork_path = Some(art_path);
                        artwork_source = Some("extracted".to_string());
                    }
                }
            }

            if (format == "wav" || format == "aiff")
                && (bitrate.is_none() || bitrate == Some(0) || bitrate.unwrap_or(0) <= 10)
            {
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
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
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
                date_added: now.to_string(),
                date_modified: now.to_string(),
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

            pass.cues_changed |= Self::sync_mik_cues(crate_conn, &track_id, &song.cues)?;

            dirty::mark_dirty(crate_conn, &buckets::bucket_for_track_id(&track_id))?;
            pass.touched_ids.push(track_id.clone());
            pass.synced_track_ids.insert(track_id);
            pass.result.added += 1;
        }
        Ok(())
    }

    /// Looks for the cover of a track the pass added or changed and that still has none.
    /// Reads the audio file's tags, so it is kept apart from the SQL-only steps.
    fn fill_missing_artwork(
        crate_conn: &Connection,
        artwork_service: Option<&ArtworkService>,
        track_id: &str,
    ) -> Result<()> {
        let Some(art_svc) = artwork_service else {
            return Ok(());
        };
        let missing: Option<String> = crate_conn
            .query_row(
                "SELECT file_path FROM tracks WHERE id = ?1 AND artwork_path IS NULL",
                [track_id],
                |r| r.get(0),
            )
            .ok();
        let Some(fpath) = missing else {
            return Ok(());
        };
        let p = PathBuf::from(&fpath);
        if let Some(tagged) = MikService::read_metadata_lenient(&p) {
            if let Some(art_path) =
                art_svc.extract_from_tagged_file_or_folder(&tagged, &p, track_id)
            {
                let hlc = dirty::next_hlc(crate_conn)?;
                crate_conn.execute(
                    "UPDATE tracks SET artwork_path = ?1, artwork_source = 'extracted', _hlc = ?2 WHERE id = ?3",
                    rusqlite::params![art_path, hlc, track_id],
                )?;
                dirty::mark_dirty(crate_conn, &buckets::bucket_for_track_id(track_id))?;
            }
        }
        Ok(())
    }

    /// The SQL-only end of a pass: bitrate clean-ups and the cue bucket's dirty mark.
    fn finish_pass(crate_conn: &Connection, pass: &SyncPass) -> Result<()> {
        // Tracks absent from Mixed In Key are kept: Mixed In Key enriches Crate, it does not own it.
        let untouched: i64 =
            crate_conn.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))?;
        let untouched = (untouched as usize).saturating_sub(pass.synced_track_ids.len());
        if untouched > 0 {
            log::debug!(
                "{untouched} Crate track(s) are not in Mixed In Key and were left untouched"
            );
        }

        // Normalize all legacy bitrates in DB from bps to kbps (e.g. 806807 -> 807, 320000 -> 320, 2116800 -> 2117)
        let _ = crate_conn.execute(
            "UPDATE tracks SET bitrate = (bitrate + 500) / 1000 WHERE bitrate > 10000",
            [],
        );

        // Fix any tracks where bitrate was corrupted to <= 10 (e.g. 2 kbps) on lossless/uncompressed tracks
        let _ = crate_conn.execute(
            "UPDATE tracks SET bitrate = ROUND((COALESCE(sample_rate, 44100) * 2 * 24) / 1000) WHERE (format = 'wav' OR format = 'aiff' OR format = 'flac') AND bitrate <= 10",
            [],
        );

        if pass.cues_changed {
            dirty::mark_dirty(crate_conn, buckets::CUES)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_library(suffix: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("crate_mik_sync_{suffix}_{}", std::process::id()));
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

    /// The three steps of a prune in one call, as `LibraryService::prune_missing_tracks` runs them.
    fn prune_all(conn: &Connection) -> Result<usize> {
        let ghosts: Vec<(String, String)> = MikDatabaseService::track_paths(conn)?
            .into_iter()
            .filter(|(_, path)| MikDatabaseService::is_missing_on_disk(path))
            .collect();
        MikDatabaseService::prune_tracks(conn, None, &ghosts)
    }

    fn track_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn test_sync_keeps_tracks_absent_from_mik() {
        let dir = temp_library("absent");
        let conn = crate_db();
        let in_mik = touch(&dir, "in_mik.mp3");
        let not_in_mik = touch(&dir, "not_in_mik.mp3");
        insert_track(&conn, "a", &in_mik, "In MIK", "Artist");
        insert_track(&conn, "b", &not_in_mik, "Not in MIK", "Artist");

        let result = MikDatabaseService::apply_mik_songs(
            &conn,
            vec![mik_song(&in_mik, "In MIK", "Artist")],
            None,
        )
        .unwrap();

        assert_eq!(result.removed, 0);
        assert_eq!(result.updated, 1);
        assert_eq!(
            track_count(&conn),
            2,
            "a track missing from Mixed In Key must never be deleted"
        );
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

        MikDatabaseService::apply_mik_songs(&conn, vec![mik_song(&other, "Song", "Artist")], None)
            .unwrap();

        assert_eq!(track_count(&conn), 3);
        let orig_path: String = conn
            .query_row("SELECT file_path FROM tracks WHERE id = 'orig'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let ext_path: String = conn
            .query_row("SELECT file_path FROM tracks WHERE id = 'ext'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(orig_path, original);
        assert_eq!(ext_path, extended);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_sync_relocated_file_keeps_track_identity() {
        let dir = temp_library("relocated");
        let conn = crate_db();
        let old_path = dir
            .join("old_folder/track.mp3")
            .to_string_lossy()
            .to_string();
        let new_path = touch(&dir, "track.mp3");
        insert_track(&conn, "keep-me", &old_path, "Track", "Artist");
        conn.execute_batch(
            "INSERT INTO tag_categories (id, name) VALUES ('cat', 'Energy');
             INSERT INTO tags (id, category_id, name, color) VALUES ('t1', 'cat', 'Peak', '#fff');
             INSERT INTO track_tags (track_id, tag_id) VALUES ('keep-me', 't1');",
        )
        .unwrap();

        MikDatabaseService::apply_mik_songs(
            &conn,
            vec![mik_song(&new_path, "Track", "Artist")],
            None,
        )
        .unwrap();

        assert_eq!(track_count(&conn), 1);
        let path: String = conn
            .query_row(
                "SELECT file_path FROM tracks WHERE id = 'keep-me'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            path, new_path,
            "the existing track follows its file instead of being re-imported"
        );
        let tags: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM track_tags WHERE track_id = 'keep-me'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tags, 1, "tags stay attached to the relocated track");
        let _ = std::fs::remove_dir_all(dir);
    }

    fn cue(time_secs: f64) -> MikDbCue {
        MikDbCue {
            z_pk: 0,
            time_secs,
            energy_level: None,
            name: None,
        }
    }

    #[test]
    fn test_second_sync_changes_nothing() {
        let dir = temp_library("idempotent");
        let conn = crate_db();
        let path = touch(&dir, "track.mp3");
        insert_track(&conn, "t", &path, "Track", "Artist");
        let mut song = mik_song(&path, "Track", "Artist");
        song.cues = vec![cue(1.0), cue(32.5)];

        let first = MikDatabaseService::apply_mik_songs(&conn, vec![song.clone()], None).unwrap();
        let second = MikDatabaseService::apply_mik_songs(&conn, vec![song], None).unwrap();

        assert_eq!(first.updated, 1);
        assert_eq!(
            second.updated, 0,
            "an unchanged Mixed In Key library must not rewrite tracks"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_sync_does_not_overwrite_user_metadata() {
        let dir = temp_library("user_meta");
        let conn = crate_db();
        let path = touch(&dir, "track.mp3");
        insert_track(&conn, "t", &path, "My Edited Title", "Artist");

        MikDatabaseService::apply_mik_songs(
            &conn,
            vec![mik_song(&path, "Tag Title", "Artist")],
            None,
        )
        .unwrap();

        let (title, bpm): (String, f64) = conn
            .query_row("SELECT title, bpm FROM tracks WHERE id = 't'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(title, "My Edited Title");
        assert_eq!(bpm, 124.0, "analysis data still comes from Mixed In Key");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_mik_cues_are_stable_and_keep_user_cues() {
        let dir = temp_library("cues");
        let conn = crate_db();
        let path = touch(&dir, "track.mp3");
        insert_track(&conn, "t", &path, "Track", "Artist");
        // A cue created in Crate, and a legacy copy of a MIK cue (random id, same position)
        conn.execute_batch(
            "INSERT INTO cues (id, track_id, position_ms, type, hot_cue_index, name) VALUES ('user-cue', 't', 90000, 'memory', NULL, 'Drop');
             INSERT INTO cues (id, track_id, position_ms, type, hot_cue_index, name) VALUES ('legacy-uuid', 't', 1000, 'hot', 1, 'Hot Cue 1');",
        )
        .unwrap();
        let mut song = mik_song(&path, "Track", "Artist");
        song.cues = vec![cue(1.0), cue(32.5)];

        MikDatabaseService::apply_mik_songs(&conn, vec![song.clone()], None).unwrap();
        let ids = |conn: &Connection| -> Vec<String> {
            let mut stmt = conn
                .prepare("SELECT id FROM cues WHERE track_id = 't' ORDER BY id")
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .flatten()
                .collect()
        };
        assert_eq!(ids(&conn), vec!["mik-t-0", "mik-t-1", "user-cue"]);

        // A cue removed in Mixed In Key disappears; the user cue stays.
        song.cues = vec![cue(1.0)];
        MikDatabaseService::apply_mik_songs(&conn, vec![song], None).unwrap();
        assert_eq!(ids(&conn), vec!["mik-t-0", "user-cue"]);
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

        let pruned = prune_all(&conn).unwrap();
        assert_eq!(pruned, 1, "Should prune exactly 1 ghost track");
        assert_eq!(track_count(&conn), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    /// A ghost track with a cue and a playlist entry, so a prune has something to cascade over.
    fn insert_ghost_with_dependants(conn: &Connection, dir: &Path, id: &str) -> String {
        let path = dir.join(format!("{id}.flac")).to_string_lossy().to_string();
        insert_track(conn, id, &path, id, "Artist");
        conn.execute(
            "INSERT INTO cues (id, track_id, position_ms, type, hot_cue_index, name) VALUES (?1, ?2, 1000, 'hot', 0, 'A')",
            rusqlite::params![format!("cue-{id}"), id],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO playlists (id, name, date_created, date_modified) VALUES ('p', 'Set', '2026-01-01', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO playlist_tracks (playlist_id, track_id, position, date_added) VALUES ('p', ?1, 0, '2026-01-01')",
            [id],
        )
        .unwrap();
        path
    }

    #[test]
    fn pruning_a_ghost_takes_its_cues_and_playlist_entries_and_leaves_a_tombstone() {
        let dir = temp_library("prune_cascade");
        let conn = crate_db();
        insert_ghost_with_dependants(&conn, &dir, "ghost-1");

        assert_eq!(prune_all(&conn).unwrap(), 1);
        assert_eq!(track_count(&conn), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM cues"), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM playlist_tracks"), 0);
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM sync_tombstones WHERE entity_id = 'ghost-1'"
            ),
            1,
            "the cloud sync is told the track is gone"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_track_on_an_unmounted_folder_is_not_a_ghost() {
        let dir = temp_library("prune_unmounted");
        let conn = crate_db();
        // The whole folder is missing (a disconnected drive): nothing proves the file was deleted.
        let path = dir
            .join("not-mounted")
            .join("track.flac")
            .to_string_lossy()
            .to_string();
        insert_track(&conn, "away", &path, "Away", "Artist");

        assert_eq!(prune_all(&conn).unwrap(), 0);
        assert_eq!(track_count(&conn), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failed_prune_changes_nothing() {
        let dir = temp_library("prune_atomic");
        let conn = crate_db();
        insert_ghost_with_dependants(&conn, &dir, "ghost-1");
        insert_ghost_with_dependants(&conn, &dir, "ghost-2");
        // The deletion of the second ghost fails after the first one is already gone.
        conn.execute_batch(
            "CREATE TRIGGER refuse_ghost_2 BEFORE DELETE ON tracks WHEN OLD.id = 'ghost-2'
             BEGIN SELECT RAISE(ABORT, 'simulated failure'); END;",
        )
        .unwrap();

        assert!(prune_all(&conn).is_err());
        assert_eq!(track_count(&conn), 2, "both tracks are still there");
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM cues"),
            2,
            "with their cues"
        );
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM playlist_tracks"), 2);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM sync_tombstones"), 0);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Two identical libraries plus the Mixed In Key songs that exercise every decision: an existing
    /// track enriched with cues, a track whose Mixed In Key path no longer exists (matched by title
    /// and artist), and two files Crate does not know yet.
    fn sync_scenario(dir: &Path) -> (Connection, Vec<MikDbSong>) {
        let conn = crate_db();
        let a = touch(dir, "a.mp3");
        let b = touch(dir, "b.mp3");
        let c = touch(dir, "c.mp3");
        let d = touch(dir, "d.flac");
        insert_track(&conn, "a", &a, "Alpha", "Artist");
        insert_track(&conn, "b", &b, "Bravo", "Artist");
        let mut song_a = mik_song(&a, "Alpha", "Artist");
        song_a.cues = vec![cue(1.0), cue(32.5)];
        let gone = dir.join("gone").join("b.mp3").to_string_lossy().to_string();
        let song_b = mik_song(&gone, "Bravo", "Artist");
        let song_c = mik_song(&c, "Charlie", "Artist");
        let mut song_d = mik_song(&d, "Delta", "Artist");
        song_d.cues = vec![cue(8.0)];
        (conn, vec![song_a, song_b, song_c, song_d])
    }

    /// Everything a sync decides, without the random ids and the timestamps.
    fn sync_snapshot(conn: &Connection) -> Vec<String> {
        let mut out: Vec<String> = conn
            .prepare(
                "SELECT file_path, title, artist, bpm, key, energy, analysis_source, bitrate, format
                 FROM tracks ORDER BY file_path",
            )
            .unwrap()
            .query_map([], |r| {
                Ok(format!(
                    "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<f64>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                    r.get::<_, String>(8)?,
                ))
            })
            .unwrap()
            .flatten()
            .collect();
        out.extend(
            conn.prepare(
                "SELECT t.file_path, c.position_ms, c.type, c.hot_cue_index, c.name
                 FROM cues c JOIN tracks t ON t.id = c.track_id
                 ORDER BY t.file_path, c.position_ms",
            )
            .unwrap()
            .query_map([], |r| {
                Ok(format!(
                    "cue {:?}|{:?}|{:?}|{:?}|{:?}",
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            })
            .unwrap()
            .flatten(),
        );
        out
    }

    #[test]
    fn syncing_in_batches_decides_exactly_like_one_transaction() {
        let dir = temp_library("batches_same");
        let (single, songs) = sync_scenario(&dir);
        let (batched, batched_songs) = sync_scenario(&dir);

        let expected = MikDatabaseService::apply_mik_songs(&single, songs, None).unwrap();
        let batched = Mutex::new(batched);
        // A zero budget makes every song its own batch: the most the lock is ever released.
        let got = MikDatabaseService::apply_mik_songs_in_batches(
            &batched,
            batched_songs,
            None,
            Duration::ZERO,
            &mut || {},
        )
        .unwrap();

        assert_eq!(expected.added, 2, "the two unknown files are added");
        assert_eq!(expected.updated, 2, "the two known tracks are enriched");
        assert_eq!(
            (got.total, got.added, got.updated, got.removed),
            (
                expected.total,
                expected.added,
                expected.updated,
                expected.removed
            )
        );
        assert_eq!(
            sync_snapshot(&single),
            sync_snapshot(&batched.lock().unwrap())
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_database_lock_is_free_between_batches() {
        let dir = temp_library("batches_lock");
        let (conn, songs) = sync_scenario(&dir);
        let conn = Mutex::new(conn);

        let (mut batches, mut lock_was_free) = (0, 0);
        MikDatabaseService::apply_mik_songs_in_batches(
            &conn,
            songs,
            None,
            Duration::ZERO,
            &mut || {
                batches += 1;
                if conn.try_lock().is_ok() {
                    lock_was_free += 1;
                }
            },
        )
        .unwrap();

        assert_eq!(batches, 4, "a zero budget makes one batch per song");
        assert_eq!(
            lock_was_free, batches,
            "the lock is free every time the pass pauses, not just some of the times"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_track_relinked_after_the_disk_check_is_left_alone() {
        let dir = temp_library("prune_relinked");
        let conn = crate_db();
        let old_path = dir.join("moved.flac").to_string_lossy().to_string();
        let new_path = touch(&dir, "moved-back.flac");
        insert_track(&conn, "t", &new_path, "Track", "Artist");

        // The disk check ran while the row still pointed at `old_path`; it has been re-linked since.
        let ghosts = vec![("t".to_string(), old_path)];
        assert_eq!(
            MikDatabaseService::prune_tracks(&conn, None, &ghosts).unwrap(),
            0
        );
        assert_eq!(
            track_count(&conn),
            1,
            "a track that now points at a real file stays"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}

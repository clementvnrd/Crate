use super::*;
use crate::services::cloud_sync::pipeline::{buckets, dirty};

impl LibraryService {
    pub fn update_track(&self, id: &str, update: TrackUpdate) -> Result<Track> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let now = chrono::Utc::now().to_rfc3339();

        // Build update query dynamically based on provided fields
        let mut updates: Vec<String> = vec!["date_modified = ?1".to_string()];
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];
        let mut param_idx = 2;

        if let Some(ref title) = update.title {
            updates.push(format!("title = ?{param_idx}"));
            params.push(Box::new(title.clone()));
            param_idx += 1;
        }
        if let Some(ref artist) = update.artist {
            updates.push(format!("artist = ?{param_idx}"));
            params.push(Box::new(artist.clone()));
            param_idx += 1;
        }
        if let Some(ref album) = update.album {
            updates.push(format!("album = ?{param_idx}"));
            params.push(Box::new(album.clone()));
            param_idx += 1;
        }
        if let Some(year) = update.year {
            updates.push(format!("year = ?{param_idx}"));
            params.push(Box::new(year));
            param_idx += 1;
        }
        if let Some(ref genre) = update.genre {
            updates.push(format!("genre = ?{param_idx}"));
            params.push(Box::new(genre.clone()));
            param_idx += 1;
        }
        if let Some(ref label) = update.label {
            updates.push(format!("label = ?{param_idx}"));
            params.push(Box::new(label.clone()));
            param_idx += 1;
        }
        if let Some(bpm) = update.bpm {
            updates.push(format!("bpm = ?{param_idx}"));
            params.push(Box::new(bpm));
            param_idx += 1;
        }
        if let Some(ref key) = update.key {
            updates.push(format!("key = ?{param_idx}"));
            params.push(Box::new(key.clone()));
            param_idx += 1;
        }
        if let Some(energy) = update.energy {
            updates.push(format!("energy = ?{param_idx}"));
            params.push(Box::new(energy));
            param_idx += 1;
        }
        if let Some(rating) = update.rating {
            updates.push(format!("rating = ?{param_idx}"));
            params.push(Box::new(rating));
            param_idx += 1;
        }

        let hlc = dirty::next_hlc(&conn)?;
        updates.push(format!("_hlc = ?{param_idx}"));
        params.push(Box::new(hlc));
        param_idx += 1;

        params.push(Box::new(id.to_string()));

        let sql = format!(
            "UPDATE tracks SET {} WHERE id = ?{}",
            updates.join(", "),
            param_idx
        );

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        conn.execute(&sql, params_refs.as_slice())?;
        dirty::mark_dirty(&conn, &buckets::bucket_for_track_id(id))?;

        drop(conn);
        self.get_track(id)
    }

    /// Update multiple tracks with the same update data (bulk operation)
    pub fn update_tracks(&self, ids: Vec<String>, update: TrackUpdate) -> Result<Vec<Track>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let now = chrono::Utc::now().to_rfc3339();

        // Build update query dynamically based on provided fields
        let mut updates: Vec<String> = vec!["date_modified = ?1".to_string()];
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];
        let mut param_idx = 2;

        if let Some(ref title) = update.title {
            updates.push(format!("title = ?{param_idx}"));
            params.push(Box::new(title.clone()));
            param_idx += 1;
        }
        if let Some(ref artist) = update.artist {
            updates.push(format!("artist = ?{param_idx}"));
            params.push(Box::new(artist.clone()));
            param_idx += 1;
        }
        if let Some(ref album) = update.album {
            updates.push(format!("album = ?{param_idx}"));
            params.push(Box::new(album.clone()));
            param_idx += 1;
        }
        if let Some(year) = update.year {
            updates.push(format!("year = ?{param_idx}"));
            params.push(Box::new(year));
            param_idx += 1;
        }
        if let Some(ref genre) = update.genre {
            updates.push(format!("genre = ?{param_idx}"));
            params.push(Box::new(genre.clone()));
            param_idx += 1;
        }
        if let Some(ref label) = update.label {
            updates.push(format!("label = ?{param_idx}"));
            params.push(Box::new(label.clone()));
            param_idx += 1;
        }
        if let Some(bpm) = update.bpm {
            updates.push(format!("bpm = ?{param_idx}"));
            params.push(Box::new(bpm));
            param_idx += 1;
        }
        if let Some(ref key) = update.key {
            updates.push(format!("key = ?{param_idx}"));
            params.push(Box::new(key.clone()));
            param_idx += 1;
        }
        if let Some(energy) = update.energy {
            updates.push(format!("energy = ?{param_idx}"));
            params.push(Box::new(energy));
            param_idx += 1;
        }
        if let Some(rating) = update.rating {
            updates.push(format!("rating = ?{param_idx}"));
            params.push(Box::new(rating));
            param_idx += 1;
        }

        let hlc = dirty::next_hlc(&conn)?;
        updates.push(format!("_hlc = ?{param_idx}"));
        params.push(Box::new(hlc));
        param_idx += 1;

        // Build IN clause for IDs
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", param_idx + i))
            .collect();

        let sql = format!(
            "UPDATE tracks SET {} WHERE id IN ({})",
            updates.join(", "),
            placeholders.join(", ")
        );

        for id in &ids {
            params.push(Box::new(id.clone()));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        conn.execute(&sql, params_refs.as_slice())?;
        dirty::mark_dirty_track_shards(&conn, &ids)?;

        drop(conn);

        // Fetch and return updated tracks
        let mut updated_tracks = Vec::new();
        for id in ids {
            if let Ok(track) = self.get_track(&id) {
                updated_tracks.push(track);
            }
        }

        Ok(updated_tracks)
    }

    /// Set rating for a track
    pub fn set_track_colors(&self, track_ids: Vec<String>, color: Option<String>) -> Result<()> {
        if track_ids.is_empty() {
            return Ok(());
        }

        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let now = chrono::Utc::now().to_rfc3339();
        let hlc = dirty::next_hlc(&conn)?;

        let placeholders: Vec<String> = track_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 4))
            .collect();

        let sql = format!(
            "UPDATE tracks SET color = ?1, date_modified = ?2, _hlc = ?3 WHERE id IN ({})",
            placeholders.join(", ")
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(color), Box::new(now), Box::new(hlc)];

        for id in &track_ids {
            params.push(Box::new(id.clone()));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;
        dirty::mark_dirty_track_shards(&conn, &track_ids)?;

        Ok(())
    }

    /// Re-synchronize Mixed In Key metadata (Key, BPM, Energy, Hot Cues) for given track IDs or all tracks
    pub fn resync_mixed_in_key_tracks(
        &self,
        track_ids: Option<Vec<String>>,
    ) -> Result<RescanResult> {
        let tracks_to_sync = match track_ids {
            Some(ids) => {
                let mut tracks = Vec::new();
                for id in ids {
                    if let Ok(track) = self.get_track(&id) {
                        tracks.push(track);
                    }
                }
                tracks
            }
            None => self.get_tracks(None)?,
        };

        let mut updated_count = 0;
        let mut failed_count = 0;

        // Step one, without the database: read the tags of every file. This is the slow part
        // (seconds to minutes for a whole library) and used to run with the connection locked,
        // so every other command waited behind it.
        let read: Vec<(String, Result<MikFileSync>)> = tracks_to_sync
            .iter()
            .map(|track| (track.id.clone(), MikService::read_track_from_file(track)))
            .collect();

        // Step two, locked for as short as possible: write everything in one transaction (one
        // commit instead of one per statement), each track in its own savepoint so a track that
        // fails to write leaves no half-written cues behind and does not affect the others.
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let tx = conn.unchecked_transaction()?;
        for (track_id, read) in read {
            let written = read.and_then(|sync| {
                tx.execute_batch("SAVEPOINT track_sync")?;
                match MikService::apply_file_sync(&tx, sync) {
                    Ok(track) => {
                        tx.execute_batch("RELEASE track_sync")?;
                        Ok(track)
                    }
                    Err(e) => {
                        let _ = tx.execute_batch("ROLLBACK TO track_sync; RELEASE track_sync");
                        Err(e)
                    }
                }
            });
            match written {
                Ok(_) => updated_count += 1,
                Err(e) => {
                    log::warn!("Failed to resync MIK metadata for track {track_id}: {e}");
                    failed_count += 1;
                }
            }
        }
        tx.commit()?;

        Ok(RescanResult {
            updated_count,
            failed_count,
        })
    }

    pub fn delete_tracks(&self, ids: Vec<String>) -> Result<()> {
        // Delete artwork files for each track
        for id in &ids {
            self.artwork_service.delete(id);
        }

        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();

        let sql = format!(
            "DELETE FROM tracks WHERE id IN ({})",
            placeholders.join(", ")
        );

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

        let hlc = dirty::next_hlc(&conn)?;
        for id in &ids {
            dirty::record_tombstone(&conn, buckets::TRACKS_ENTITY, id, &hlc)?;
        }

        conn.execute(&sql, params_refs.as_slice())?;

        // Invalidate deleted tracks from upgrade matches cache
        let _ = conn.execute(
            &format!(
                "DELETE FROM upgrade_matches_cache WHERE track_id IN ({})",
                placeholders.join(", ")
            ),
            params_refs.as_slice(),
        );

        // The deleted tracks' shards, plus the cascade-deleted child buckets
        // (playlist memberships, tag links, cues).
        dirty::mark_dirty_track_shards(&conn, &ids)?;
        dirty::mark_dirty(&conn, buckets::PLAYLIST_TRACKS)?;
        dirty::mark_dirty(&conn, buckets::TRACK_TAGS)?;
        dirty::mark_dirty(&conn, buckets::CUES)?;

        Ok(())
    }

    /// Delete tracks from Crate DB and move their audio files to the macOS Trash
    pub fn delete_tracks_and_files(&self, ids: Vec<String>) -> Result<()> {
        // 1. Fetch file paths for all tracks to delete
        let mut id_paths: Vec<(String, String)> = Vec::new();
        {
            let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            let placeholders: Vec<String> = ids
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect();
            let sql = format!(
                "SELECT id, file_path FROM tracks WHERE id IN ({})",
                placeholders.join(", ")
            );
            let params_refs: Vec<&dyn rusqlite::ToSql> =
                ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params_refs.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows.flatten() {
                id_paths.push(row);
            }
        }

        // 2. Mixed In Key is read-only for Crate: its own library is left untouched.

        // 3. Move audio files to the Trash; a track leaves the library only if its file did
        let mut removable = Vec::with_capacity(ids.len());
        let mut failures = Vec::new();
        for (id, path_str) in id_paths {
            match crate::services::trash::move_to_trash(std::path::Path::new(&path_str)) {
                Ok(()) => removable.push(id),
                Err(e) => failures.push(format!("{path_str} ({e})")),
            }
        }

        // 4. Delete from Crate DB, then report files that could not be moved to the Trash
        self.delete_tracks(removable)?;
        if failures.is_empty() {
            Ok(())
        } else {
            Err(CrateError::InvalidOperation(format!(
                "{} file(s) could not be moved to the Trash and were kept in the library: {}",
                failures.len(),
                failures.join("; ")
            )))
        }
    }
}

/// Audio properties of a file that replaces an existing track's file.
#[derive(Debug, Clone, Default)]
pub(crate) struct ReplacementAudio {
    pub format: String,
    pub duration_ms: i64,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub file_hash: Option<String>,
}

impl LibraryService {
    /// Points an existing track at a new audio file (e.g. an MP3 upgraded to FLAC).
    ///
    /// The track keeps its id, so its cues, tags, playlists, rating, colour and listening
    /// history are preserved; only the file-related columns change.
    pub fn replace_track_file(&self, id: &str, new_path: &std::path::Path) -> Result<Track> {
        let path = new_path.to_path_buf();
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        let mut audio = ReplacementAudio {
            format,
            file_hash: compute_audio_hash(&path).ok(),
            ..Default::default()
        };
        if let Some(tagged) = self.read_metadata_lenient(&path) {
            use lofty::file::AudioFile;
            let props = tagged.properties();
            audio.duration_ms = props.duration().as_millis() as i64;
            audio.bitrate = props.audio_bitrate().map(|b| b as i32);
            audio.sample_rate = props.sample_rate().map(|s| s as i32);
        } else {
            let (duration_ms, sample_rate, bitrate) =
                self.read_audio_properties_symphonia(&path)?;
            audio.duration_ms = duration_ms;
            audio.sample_rate = sample_rate;
            audio.bitrate = bitrate;
        }

        {
            let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            replace_track_file_in(&conn, id, &path.to_string_lossy(), &audio)?;
        }
        self.get_track(id)
    }
}

/// Updates the file columns of track `id` in place, within one transaction. The beat grid of the
/// old file is cleared: the new file can start at another offset (encoder delay, other master),
/// so the track shows no grid until it is analysed again.
pub(crate) fn replace_track_file_in(
    conn: &rusqlite::Connection,
    id: &str,
    new_path: &str,
    audio: &ReplacementAudio,
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    let now = chrono::Utc::now().to_rfc3339();
    let hlc = dirty::next_hlc(&tx)?;
    let (library_root_id, relative_path) =
        crate::services::cloud_sync::resolution::assign_root_for_import(&tx, new_path)?;
    let changed = tx.execute(
        "UPDATE tracks SET file_path = ?1, format = ?2,
            duration_ms = CASE WHEN ?3 > 0 THEN ?3 ELSE duration_ms END,
            bitrate = ?4, sample_rate = ?5, file_hash = ?6, date_modified = ?7, _hlc = ?8,
            library_root_id = ?9, relative_path = ?10,
            beatgrid_first_beat_ms = NULL, beatgrid_bpm = NULL, beatgrid_tempo_changes = NULL
         WHERE id = ?11",
        rusqlite::params![
            new_path,
            audio.format,
            audio.duration_ms,
            audio.bitrate,
            audio.sample_rate,
            audio.file_hash,
            now,
            hlc,
            library_root_id,
            relative_path,
            id
        ],
    )?;
    if changed == 0 {
        return Err(CrateError::TrackNotFound(id.to_string()));
    }
    dirty::mark_dirty(&tx, &buckets::bucket_for_track_id(id))?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod replace_file_tests {
    use super::*;

    #[test]
    fn test_replace_track_file_keeps_identity_cues_tags_and_playlists() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO tracks (id, file_path, format, title, artist, duration_ms, bitrate, rating, play_count, date_added, date_modified)
                VALUES ('t1', '/music/song.mp3', 'mp3', 'Song', 'Artist', 300000, 320, 4, 12, '2026-01-01', '2026-01-01');
             INSERT INTO cues (id, track_id, position_ms, type, hot_cue_index, name) VALUES ('c1', 't1', 1000, 'hot', 0, 'Intro');
             INSERT INTO tag_categories (id, name) VALUES ('cat', 'Energy');
             INSERT INTO tags (id, category_id, name, color) VALUES ('g1', 'cat', 'Peak', '#ffffff');
             INSERT INTO track_tags (track_id, tag_id) VALUES ('t1', 'g1');
             INSERT INTO playlists (id, name, date_created, date_modified) VALUES ('p1', 'Set', '2026-01-01', '2026-01-01');
             INSERT INTO playlist_tracks (playlist_id, track_id, position, date_added) VALUES ('p1', 't1', 0, '2026-01-01');",
        )
        .unwrap();

        let audio = ReplacementAudio {
            format: "flac".into(),
            duration_ms: 300_500,
            bitrate: Some(1411),
            sample_rate: Some(44100),
            file_hash: Some("abc".into()),
        };
        replace_track_file_in(&conn, "t1", "/music/song.flac", &audio).unwrap();

        let (path, format, bitrate, rating, plays): (String, String, i32, i32, i32) = conn
            .query_row(
                "SELECT file_path, format, bitrate, rating, play_count FROM tracks WHERE id = 't1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(
            (path.as_str(), format.as_str(), bitrate),
            ("/music/song.flac", "flac", 1411)
        );
        assert_eq!((rating, plays), (4, 12));
        for (table, column) in [
            ("cues", "track_id"),
            ("track_tags", "track_id"),
            ("playlist_tracks", "track_id"),
        ] {
            let n: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE {column} = 't1'"),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "{table} must survive the upgrade");
        }
    }

    #[test]
    fn test_replace_track_file_clears_the_old_beat_grid() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO tracks (id, file_path, format, duration_ms, bpm, date_added, date_modified, \
             beatgrid_first_beat_ms, beatgrid_bpm) \
             VALUES ('t1', '/music/song.mp3', 'mp3', 300000, 124.0, '2026-01-01', '2026-01-01', 48.5, 123.98)",
            [],
        )
        .unwrap();
        replace_track_file_in(
            &conn,
            "t1",
            "/music/song.flac",
            &ReplacementAudio::default(),
        )
        .unwrap();
        assert_eq!(
            crate::services::beatgrid::load_beat_grid(&conn, "t1").unwrap(),
            None
        );
        let bpm: f64 = conn
            .query_row("SELECT bpm FROM tracks WHERE id = 't1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(bpm, 124.0);
    }

    #[test]
    fn test_replace_track_file_unknown_track() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        let err = replace_track_file_in(&conn, "missing", "/x.flac", &ReplacementAudio::default());
        assert!(matches!(err, Err(CrateError::TrackNotFound(_))));
    }
}

#[cfg(test)]
mod resync_tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    /// A valid, silent, one-second mono WAV: enough for the tag reader to open.
    fn write_silent_wav(path: &Path) {
        let sample_rate: u32 = 8000;
        let data_len: u32 = sample_rate * 2; // 16-bit mono, one second
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        bytes.extend(std::iter::repeat_n(0u8, data_len as usize));
        std::fs::write(path, bytes).unwrap();
    }

    struct Fixture {
        dir: PathBuf,
        service: LibraryService,
        conn: Arc<Mutex<rusqlite::Connection>>,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// Two tracks backed by real files (`t1`, `t2`) and one whose file is missing (`t3`).
    fn fixture(name: &str) -> Fixture {
        let dir = std::env::temp_dir().join(format!("crate_resync_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_silent_wav(&dir.join("one.wav"));
        write_silent_wav(&dir.join("two.wav"));

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        for (id, file) in [("t1", "one.wav"), ("t2", "two.wav"), ("t3", "missing.wav")] {
            conn.execute(
                "INSERT INTO tracks (id, file_path, format, title, artist, duration_ms, date_added, date_modified)
                 VALUES (?1, ?2, 'wav', ?1, 'Artist', 1000, '2026-01-01', '2026-01-01')",
                rusqlite::params![id, dir.join(file).to_string_lossy()],
            )
            .unwrap();
        }
        let conn = Arc::new(Mutex::new(conn));
        let service = LibraryService::new(conn.clone(), dir.clone());
        Fixture { dir, service, conn }
    }

    fn date_modified(f: &Fixture, id: &str) -> String {
        f.conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT date_modified FROM tracks WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap()
    }

    #[test]
    fn resync_counts_readable_and_unreadable_tracks() {
        let f = fixture("all");
        let result = f.service.resync_mixed_in_key_tracks(None).unwrap();
        assert_eq!((result.updated_count, result.failed_count), (2, 1));
        assert_ne!(date_modified(&f, "t1"), "2026-01-01");
        assert_ne!(date_modified(&f, "t2"), "2026-01-01");
        assert_eq!(
            date_modified(&f, "t3"),
            "2026-01-01",
            "a track whose file is missing is left untouched"
        );
    }

    #[test]
    fn resync_of_a_selection_only_touches_those_tracks() {
        let f = fixture("selection");
        let result = f
            .service
            .resync_mixed_in_key_tracks(Some(vec!["t2".to_string()]))
            .unwrap();
        assert_eq!((result.updated_count, result.failed_count), (1, 0));
        assert_eq!(date_modified(&f, "t1"), "2026-01-01");
        assert_ne!(date_modified(&f, "t2"), "2026-01-01");
    }

    #[test]
    fn reading_a_file_needs_no_database_connection() {
        // `read_track_from_file` has no connection parameter: the slow step cannot hold the lock.
        let f = fixture("readonly");
        let track = f.service.get_track("t1").unwrap();
        assert!(MikService::read_track_from_file(&track).is_ok());

        let missing = f.service.get_track("t3").unwrap();
        assert!(
            MikService::read_track_from_file(&missing).is_err(),
            "a missing file is reported before any write"
        );
    }

    #[test]
    fn a_track_that_fails_to_write_does_not_affect_the_others() {
        let f = fixture("isolation");
        // Make the write of t1 fail (a trigger aborts any update of that row).
        f.conn
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER fail_t1 BEFORE UPDATE ON tracks
                   WHEN NEW.id = 't1'
                   BEGIN SELECT RAISE(ABORT, 'simulated write failure'); END;",
            )
            .unwrap();

        let result = f.service.resync_mixed_in_key_tracks(None).unwrap();

        // t1 fails on write, t3 fails on read, t2 is written.
        assert_eq!((result.updated_count, result.failed_count), (1, 2));
        assert_eq!(date_modified(&f, "t1"), "2026-01-01");
        assert_ne!(date_modified(&f, "t2"), "2026-01-01");
    }
}

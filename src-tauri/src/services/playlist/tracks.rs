use super::*;
use crate::services::cloud_sync::pipeline::{buckets, dirty};

impl PlaylistService {
    pub fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Track>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let mut stmt = conn.prepare(
            r#"
            SELECT
                t.id, t.file_path, t.file_hash,
                t.title, t.artist, t.album, t.year, t.genre, t.label, t.catalog_number,
                t.duration_ms, t.bpm, t.key, t.energy, t.bitrate, t.sample_rate, t.format,
                t.analysis_source, t.waveform_data,
                t.rating, t.play_count,
                t.date_added, t.date_modified, t.last_played,
                t.rekordbox_id, t.artwork_path, t.artwork_source, t.color,
                t.library_root_id, t.relative_path
            FROM tracks t
            JOIN playlist_tracks pt ON t.id = pt.track_id
            WHERE pt.playlist_id = ?1
            ORDER BY pt.position
            "#,
        )?;

        let tracks = stmt
            .query_map([playlist_id], |row| {
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
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Fetch tags
        let tracks_with_tags = self.fetch_tags_for_tracks(&conn, tracks)?;
        Ok(tracks_with_tags)
    }

    pub(crate) fn fetch_tags_for_tracks(
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

        // Group tags by track_id
        let mut tags_by_track: std::collections::HashMap<String, Vec<Tag>> =
            std::collections::HashMap::new();
        for (track_id, tag) in tag_rows {
            tags_by_track.entry(track_id).or_default().push(tag);
        }

        // Assign tags to tracks
        for track in &mut tracks {
            if let Some(tags) = tags_by_track.remove(&track.id) {
                track.tags = tags;
            }
        }

        Ok(tracks)
    }

    pub fn add_tracks(&self, playlist_id: &str, track_ids: Vec<String>) -> Result<Playlist> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // Get current max position
        let max_position: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) FROM playlist_tracks WHERE playlist_id = ?1",
                [playlist_id],
                |row| row.get(0),
            )
            .unwrap_or(-1);

        let now = chrono::Utc::now().to_rfc3339();

        // One transaction: a failure part-way leaves the playlist untouched instead of half filled,
        // and SQLite syncs to disk once instead of once per track.
        let tx = conn.unchecked_transaction()?;
        let hlc = dirty::next_hlc(&tx)?;

        for (i, track_id) in track_ids.iter().enumerate() {
            let position = max_position + 1 + i as i32;
            tx.execute(
                "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position, date_added, _hlc) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![playlist_id, track_id, position, now, hlc],
            )?;
        }

        // Update playlist modified date
        tx.execute(
            "UPDATE playlists SET date_modified = ?1, _hlc = ?2 WHERE id = ?3",
            rusqlite::params![now, hlc, playlist_id],
        )?;
        dirty::mark_dirty(&tx, buckets::PLAYLIST_TRACKS)?;
        dirty::mark_dirty(&tx, buckets::PLAYLISTS)?;
        tx.commit()?;

        // Drop the lock before calling get_playlist which acquires its own lock
        drop(conn);

        // Return the updated playlist with accurate track count
        self.get_playlist(playlist_id)
    }

    pub fn remove_tracks(&self, playlist_id: &str, track_ids: Vec<String>) -> Result<Playlist> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // One transaction: a row is never deleted without its tombstone (the cloud sync would bring
        // the track back), and the renumbering below cannot stop half way.
        let tx = conn.unchecked_transaction()?;
        let hlc = dirty::next_hlc(&tx)?;
        for track_id in &track_ids {
            let deleted = tx.execute(
                "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
                rusqlite::params![playlist_id, track_id],
            )?;
            if deleted > 0 {
                dirty::record_tombstone(
                    &tx,
                    buckets::PLAYLIST_TRACKS,
                    &dirty::junction_entity_id(playlist_id, track_id),
                    &hlc,
                )?;
            }
        }

        // Reorder remaining tracks
        let remaining_tracks: Vec<String> = {
            let mut stmt = tx.prepare(
                "SELECT track_id FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position",
            )?;
            let tracks = stmt
                .query_map([playlist_id], |row| row.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            tracks
        };

        for (i, track_id) in remaining_tracks.iter().enumerate() {
            tx.execute(
                "UPDATE playlist_tracks SET position = ?1, _hlc = ?2 WHERE playlist_id = ?3 AND track_id = ?4",
                rusqlite::params![i as i32, hlc, playlist_id, track_id],
            )?;
        }

        // Update playlist modified date
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE playlists SET date_modified = ?1, _hlc = ?2 WHERE id = ?3",
            rusqlite::params![now, hlc, playlist_id],
        )?;
        dirty::mark_dirty(&tx, buckets::PLAYLIST_TRACKS)?;
        dirty::mark_dirty(&tx, buckets::PLAYLISTS)?;
        tx.commit()?;

        // Drop the lock before calling get_playlist which acquires its own lock
        drop(conn);

        // Return the updated playlist with accurate track count
        self.get_playlist(playlist_id)
    }

    pub fn reorder_tracks(&self, playlist_id: &str, track_ids: Vec<String>) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // One transaction: a failure part-way cannot leave two tracks on the same position.
        let tx = conn.unchecked_transaction()?;
        let hlc = dirty::next_hlc(&tx)?;
        for (i, track_id) in track_ids.iter().enumerate() {
            tx.execute(
                "UPDATE playlist_tracks SET position = ?1, _hlc = ?2 WHERE playlist_id = ?3 AND track_id = ?4",
                rusqlite::params![i as i32, hlc, playlist_id, track_id],
            )?;
        }

        // Update playlist modified date
        let now = chrono::Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE playlists SET date_modified = ?1, _hlc = ?2 WHERE id = ?3",
            rusqlite::params![now, hlc, playlist_id],
        )?;
        dirty::mark_dirty(&tx, buckets::PLAYLIST_TRACKS)?;
        dirty::mark_dirty(&tx, buckets::PLAYLISTS)?;
        tx.commit()?;

        Ok(())
    }
}

#[cfg(test)]
mod bulk_tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn service() -> (PlaylistService, Arc<Mutex<Connection>>) {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO tracks (id, file_path, format, title, artist, duration_ms, date_added, date_modified) VALUES
               ('t1', '/m/1.mp3', 'mp3', 'One', 'A', 1000, '2026-01-01', '2026-01-01'),
               ('t2', '/m/2.mp3', 'mp3', 'Two', 'A', 1000, '2026-01-01', '2026-01-01'),
               ('t3', '/m/3.mp3', 'mp3', 'Three', 'A', 1000, '2026-01-01', '2026-01-01');
             INSERT INTO playlists (id, name, date_created, date_modified) VALUES ('p', 'Set', '2026-01-01', '2026-01-01');",
        )
        .unwrap();
        let conn = Arc::new(Mutex::new(conn));
        (PlaylistService::new(conn.clone()), conn)
    }

    fn seed_three(conn: &Arc<Mutex<Connection>>) {
        conn.lock()
            .unwrap()
            .execute_batch(
                "INSERT INTO playlist_tracks (playlist_id, track_id, position, date_added) VALUES
                   ('p', 't1', 0, '2026-01-01'), ('p', 't2', 1, '2026-01-01'), ('p', 't3', 2, '2026-01-01');",
            )
            .unwrap();
    }

    fn order(conn: &Arc<Mutex<Connection>>) -> Vec<(String, i64)> {
        let guard = conn.lock().unwrap();
        let mut stmt = guard
            .prepare("SELECT track_id, position FROM playlist_tracks WHERE playlist_id = 'p' ORDER BY position")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|row| row.unwrap())
            .collect()
    }

    fn tombstones(conn: &Arc<Mutex<Connection>>) -> i64 {
        conn.lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM sync_tombstones", [], |r| r.get(0))
            .unwrap()
    }

    fn refuse_writes_for_t3(conn: &Arc<Mutex<Connection>>, event: &str) {
        let row = if event == "DELETE" { "OLD" } else { "NEW" };
        conn.lock()
            .unwrap()
            .execute_batch(&format!(
                "CREATE TRIGGER refuse_t3 BEFORE {event} ON playlist_tracks WHEN {row}.track_id = 't3'
                 BEGIN SELECT RAISE(ABORT, 'simulated failure'); END;"
            ))
            .unwrap();
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_failed_reorder_leaves_the_playlist_as_it_was() {
        let (playlists, conn) = service();
        seed_three(&conn);
        refuse_writes_for_t3(&conn, "UPDATE");

        // t2 and t1 would be moved before the write for t3 fails.
        assert!(playlists
            .reorder_tracks("p", ids(&["t2", "t1", "t3"]))
            .is_err());
        assert_eq!(
            order(&conn),
            [
                ("t1".to_string(), 0),
                ("t2".to_string(), 1),
                ("t3".to_string(), 2)
            ]
        );
    }

    #[test]
    fn a_failed_removal_keeps_the_track_and_writes_no_tombstone() {
        let (playlists, conn) = service();
        seed_three(&conn);
        // Removing t1 renumbers t2 and t3; the write for t3 fails.
        refuse_writes_for_t3(&conn, "UPDATE");

        assert!(playlists.remove_tracks("p", ids(&["t1"])).is_err());
        assert_eq!(order(&conn).len(), 3, "t1 is still in the playlist");
        assert_eq!(
            tombstones(&conn),
            0,
            "and the cloud sync is not told it left"
        );
    }

    #[test]
    fn a_failed_addition_adds_nothing() {
        let (playlists, conn) = service();
        refuse_writes_for_t3(&conn, "INSERT");

        assert!(playlists.add_tracks("p", ids(&["t1", "t2", "t3"])).is_err());
        assert!(order(&conn).is_empty());
    }

    #[test]
    fn removing_a_track_renumbers_the_rest_without_gaps() {
        let (playlists, conn) = service();
        seed_three(&conn);
        playlists.remove_tracks("p", ids(&["t1"])).unwrap();
        assert_eq!(order(&conn), [("t2".to_string(), 0), ("t3".to_string(), 1)]);
        assert_eq!(tombstones(&conn), 1);
    }

    #[test]
    fn adding_appends_after_the_last_position_and_ignores_duplicates() {
        let (playlists, conn) = service();
        playlists.add_tracks("p", ids(&["t2", "t1"])).unwrap();
        playlists.add_tracks("p", ids(&["t1", "t3"])).unwrap();
        let tracks: Vec<String> = order(&conn).into_iter().map(|(id, _)| id).collect();
        assert_eq!(
            tracks,
            ["t2", "t1", "t3"],
            "t1 is already in: it keeps its place"
        );
    }

    #[test]
    fn reordering_follows_the_given_order() {
        let (playlists, conn) = service();
        seed_three(&conn);
        playlists
            .reorder_tracks("p", ids(&["t3", "t1", "t2"]))
            .unwrap();
        let tracks: Vec<String> = order(&conn).into_iter().map(|(id, _)| id).collect();
        assert_eq!(tracks, ["t3", "t1", "t2"]);
    }
}

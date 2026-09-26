use super::*;
use crate::models::{Cue, CueType};

impl LibraryService {
    pub fn get_tracks(&self, filter: Option<TrackFilter>) -> Result<Vec<Track>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let mut sql = String::from(
            r#"
            SELECT
                t.id, t.file_path, t.file_hash,
                t.title, t.artist, t.album, t.year, t.genre, t.label, t.catalog_number,
                t.duration_ms, t.bpm, t.key, t.energy, t.bitrate, t.sample_rate, t.format,
                t.analysis_source, NULL as waveform_data,
                t.rating, t.play_count,
                t.date_added, t.date_modified, t.last_played,
                t.rekordbox_id, t.artwork_path, t.artwork_source, t.color,
                t.library_root_id, t.relative_path
            FROM tracks t
            "#,
        );

        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref filter) = filter {
            if let Some(ref search) = filter.search {
                let trimmed = search.trim();
                if !trimmed.is_empty() {
                    if let Some(fts_match) = fts_query(trimmed) {
                        let p_idx = params.len() + 1;
                        conditions.push(format!(
                            "t.rowid IN (SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH ?{p_idx})"
                        ));
                        params.push(Box::new(fts_match));
                    }
                }
            }

            if let Some(ref tag_ids) = filter.tag_ids {
                if !tag_ids.is_empty() {
                    let placeholders: Vec<String> = tag_ids
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("?{}", params.len() + i + 1))
                        .collect();

                    // Check filter mode: "and" requires all tags, "or" (default) requires any tag
                    let is_and_mode = filter
                        .tag_filter_mode
                        .as_ref()
                        .map(|m| m == "and")
                        .unwrap_or(false);

                    if is_and_mode {
                        // AND mode: track must have ALL selected tags
                        conditions.push(format!(
                            "t.id IN (SELECT track_id FROM track_tags WHERE tag_id IN ({}) GROUP BY track_id HAVING COUNT(DISTINCT tag_id) = {})",
                            placeholders.join(", "),
                            tag_ids.len()
                        ));
                    } else {
                        // OR mode: track must have ANY of the selected tags
                        conditions.push(format!(
                            "t.id IN (SELECT track_id FROM track_tags WHERE tag_id IN ({}))",
                            placeholders.join(", ")
                        ));
                    }

                    for tag_id in tag_ids {
                        params.push(Box::new(tag_id.clone()));
                    }
                }
            }

            if let Some(ref playlist_id) = filter.playlist_id {
                conditions.push(format!(
                    "t.id IN (SELECT track_id FROM playlist_tracks WHERE playlist_id = ?{})",
                    params.len() + 1
                ));
                params.push(Box::new(playlist_id.clone()));
            }

            if let Some(bpm_min) = filter.bpm_min {
                conditions.push(format!("t.bpm >= ?{}", params.len() + 1));
                params.push(Box::new(bpm_min));
            }

            if let Some(bpm_max) = filter.bpm_max {
                conditions.push(format!("t.bpm <= ?{}", params.len() + 1));
                params.push(Box::new(bpm_max));
            }

            if let Some(ref keys) = filter.keys {
                if !keys.is_empty() {
                    let placeholders: Vec<String> = keys
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("?{}", params.len() + i + 1))
                        .collect();
                    conditions.push(format!("t.key IN ({})", placeholders.join(", ")));
                    for k in keys {
                        params.push(Box::new(k.clone()));
                    }
                }
            } else if let Some(ref key) = filter.key {
                conditions.push(format!("t.key = ?{}", params.len() + 1));
                params.push(Box::new(key.clone()));
            }

            if let Some(energy_min) = filter.energy_min {
                conditions.push(format!("t.energy >= ?{}", params.len() + 1));
                params.push(Box::new(energy_min));
            }

            if let Some(energy_max) = filter.energy_max {
                conditions.push(format!("t.energy <= ?{}", params.len() + 1));
                params.push(Box::new(energy_max));
            }
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY t.date_added DESC");

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let tracks = stmt
            .query_map(params_refs.as_slice(), |row| {
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

        // Fetch tags for all tracks
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

        let mut tags_by_track: std::collections::HashMap<String, Vec<Tag>> =
            std::collections::HashMap::new();

        // Chunk track IDs in batches of 400 to prevent SQLITE_MAX_VARIABLE_NUMBER limit exceeded
        for chunk in tracks.chunks(400) {
            let track_ids: Vec<String> = chunk.iter().map(|t| t.id.clone()).collect();
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

            for (track_id, tag) in tag_rows {
                tags_by_track.entry(track_id).or_default().push(tag);
            }
        }

        // Assign tags to tracks
        for track in &mut tracks {
            if let Some(tags) = tags_by_track.remove(&track.id) {
                track.tags = tags;
            }
        }

        Ok(tracks)
    }

    /// Waveform overview of a track (bars 0–100). Computed from the audio file the first time and
    /// cached in `waveform_data`; the file is decoded without holding the library lock.
    pub fn get_track_waveform(&self, track_id: &str) -> Result<Option<Vec<u8>>> {
        let (cached, file_path): (Option<Vec<u8>>, Option<String>) = {
            let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            conn.query_row(
                "SELECT waveform_data, file_path FROM tracks WHERE id = ?1",
                rusqlite::params![track_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((None, None))
        };
        if let Some(waveform) = cached.filter(|w| !w.is_empty()) {
            return Ok(Some(waveform));
        }
        let Some(file_path) = file_path else {
            // A file opened outside the library (standalone player) is addressed by its path:
            // compute its waveform without caching it anywhere.
            let external = std::path::Path::new(track_id);
            return Ok(external
                .is_file()
                .then(|| super::waveform::compute_peaks(external, super::waveform::WAVEFORM_BARS))
                .flatten());
        };

        let Some(peaks) = super::waveform::compute_peaks(std::path::Path::new(&file_path), super::waveform::WAVEFORM_BARS) else {
            return Ok(None);
        };
        // Local cache only: waveform_data is not a synced column, so no HLC / dirty marking.
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        conn.execute(
            "UPDATE tracks SET waveform_data = ?1 WHERE id = ?2",
            rusqlite::params![peaks, track_id],
        )?;
        Ok(Some(peaks))
    }

    /// Retrieve Hot Cues and Memory Cues for a track (both from Crate DB and Mixed In Key)
    pub fn get_track_cues(&self, track_id_or_path: &str) -> Result<Vec<Cue>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, track_id, position_ms, type, loop_end_ms, hot_cue_index, name, color
            FROM cues
            WHERE track_id = ?1
            ORDER BY position_ms ASC
            "#,
        )?;

        let rows = stmt.query_map([track_id_or_path], |row| {
            let cue_type_str: String = row.get(3)?;
            let cue_type = cue_type_str.parse().unwrap_or(CueType::Memory);
            Ok(Cue {
                id: row.get(0)?,
                track_id: row.get(1)?,
                position_ms: row.get(2)?,
                cue_type,
                loop_end_ms: row.get(4)?,
                hot_cue_index: row.get(5)?,
                name: row.get(6)?,
                color: row.get(7)?,
            })
        })?;

        let mut cues: Vec<Cue> = rows.flatten().collect();
        drop(stmt);

        // Library tracks already carry their Mixed In Key cues (imported by the sync). Only a file
        // opened from outside the library (standalone player) is looked up in Mixed In Key, and
        // that happens after releasing the library lock.
        let is_library_track = conn
            .query_row("SELECT 1 FROM tracks WHERE id = ?1", [track_id_or_path], |_| Ok(()))
            .is_ok();
        drop(conn);

        let external_path = std::path::PathBuf::from(track_id_or_path);
        if cues.is_empty() && !is_library_track && external_path.is_file() {
            if let Ok(mik_songs) = crate::services::library::MikDatabaseService::read_all_songs() {
                if let Some(song) = mik_songs.into_iter().find(|s| s.file_path.as_ref() == Some(&external_path)) {
                    for (idx, mc) in song.cues.into_iter().enumerate() {
                        let hot_idx = idx as i32;
                        cues.push(Cue {
                            id: format!("mik-{}-{}", song.z_pk, mc.z_pk),
                            track_id: track_id_or_path.to_string(),
                            position_ms: (mc.time_secs * 1000.0).round() as i64,
                            cue_type: CueType::Hot,
                            loop_end_ms: None,
                            hot_cue_index: Some(hot_idx),
                            name: mc.name.or_else(|| Some(format!("Hot Cue {}", hot_idx + 1))),
                            color: None,
                        });
                    }
                }
            }
        }

        Ok(cues)
    }

    pub fn get_track(&self, id: &str) -> Result<Track> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let track = conn.query_row(
            r#"
            SELECT
                id, file_path, file_hash,
                title, artist, album, year, genre, label, catalog_number,
                duration_ms, bpm, key, energy, bitrate, sample_rate, format,
                analysis_source, waveform_data,
                rating, play_count,
                date_added, date_modified, last_played,
                rekordbox_id, artwork_path, artwork_source, color,
                library_root_id, relative_path
            FROM tracks WHERE id = ?1
            "#,
            [id],
            |row| {
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
            },
        )?;

        // Fetch tags
        let tracks_with_tags = self.fetch_tags_for_tracks(&conn, vec![track])?;
        tracks_with_tags
            .into_iter()
            .next()
            .ok_or_else(|| CrateError::TrackNotFound(id.to_string()))
    }

    /// Find an existing track by its file hash
    pub fn find_track_by_hash(&self, file_hash: &str) -> Result<Option<Track>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let result = conn.query_row(
            r#"
            SELECT
                id, file_path, file_hash,
                title, artist, album, year, genre, label, catalog_number,
                duration_ms, bpm, key, energy, bitrate, sample_rate, format,
                analysis_source, waveform_data,
                rating, play_count,
                date_added, date_modified, last_played,
                rekordbox_id, artwork_path, artwork_source, color,
                library_root_id, relative_path
            FROM tracks WHERE file_hash = ?1
            "#,
            [file_hash],
            |row| {
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
            },
        );

        match result {
            Ok(track) => {
                // Fetch tags for the track
                let tracks_with_tags = self.fetch_tags_for_tracks(&conn, vec![track])?;
                Ok(tracks_with_tags.into_iter().next())
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CrateError::Database(e)),
        }
    }
}

/// Builds an FTS5 query matching how the `unicode61` tokenizer indexed the text: every run of
/// letters/digits becomes a quoted prefix term, so "You'll" → `"You"* "ll"*`, "Jay-Z" →
/// `"Jay"* "Z"*`, "AC/DC" → `"AC"* "DC"*`. Quoting makes AND/OR/NOT/NEAR plain words.
pub(crate) fn fts_query(search: &str) -> Option<String> {
    let terms: Vec<String> = search
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| format!("\"{token}\"*"))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

#[cfg(test)]
mod fts_tests {
    use super::*;

    fn library_with(titles: &[(&str, &str)]) -> LibraryService {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        for (i, (title, artist)) in titles.iter().enumerate() {
            conn.execute(
                "INSERT INTO tracks (id, file_path, title, artist, format, duration_ms, date_added, date_modified)
                 VALUES (?1, ?2, ?3, ?4, 'mp3', 1000, '2026-01-01', '2026-01-01')",
                rusqlite::params![format!("t{i}"), format!("/m/{i}.mp3"), title, artist],
            )
            .unwrap();
        }
        LibraryService::new(Arc::new(Mutex::new(conn)), std::env::temp_dir())
    }

    fn search(lib: &LibraryService, q: &str) -> Vec<String> {
        let filter = TrackFilter { search: Some(q.to_string()), ..Default::default() };
        lib.get_tracks(Some(filter)).unwrap().into_iter().filter_map(|t| t.title).collect()
    }

    #[test]
    fn test_search_handles_apostrophes_dashes_slashes_and_operators() {
        let lib = library_with(&[
            ("You'll Never Walk Alone", "Gerry"),
            ("Empire State of Mind", "Jay-Z"),
            ("Thunderstruck", "AC/DC"),
            ("Black AND White", "Band"),
        ]);
        assert_eq!(search(&lib, "You'll"), vec!["You'll Never Walk Alone"]);
        assert_eq!(search(&lib, "jay-z"), vec!["Empire State of Mind"]);
        assert_eq!(search(&lib, "AC/DC"), vec!["Thunderstruck"]);
        assert_eq!(search(&lib, "AND"), vec!["Black AND White"], "an operator keyword is a plain word");
        assert_eq!(search(&lib, "thund"), vec!["Thunderstruck"], "prefix search still works");
        assert!(search(&lib, "\"").len() == 4, "punctuation-only search does not filter or fail");
    }

    #[test]
    fn test_fts_query_quotes_every_token() {
        assert_eq!(fts_query("You'll").as_deref(), Some("\"You\"* \"ll\"*"));
        assert_eq!(fts_query("  -- ").as_deref(), None);
    }
}

use std::fs::File;
use std::io::Write;
use std::path::Path;

use super::*;
use crate::error::{CrateError, Result};
use crate::models::{Cue, Track};

impl ExportService {
    /// Export library or selected playlists to a standard Pioneer rekordbox.xml file
    pub fn export_rekordbox_xml(
        &self,
        target_path: &Path,
        playlist_ids: Option<Vec<String>>,
    ) -> Result<usize> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // 1. Fetch tracks to include
        let (tracks, playlist_track_map) = match playlist_ids {
            Some(ref pids) if !pids.is_empty() => {
                let mut all_tracks = Vec::new();
                let mut track_map: Vec<(String, String, Vec<String>)> = Vec::new(); // (playlist_id, playlist_name, track_ids)

                for pid in pids {
                    let playlist_name: String = conn
                        .query_row("SELECT name FROM playlists WHERE id = ?1", [pid], |r| {
                            r.get(0)
                        })
                        .unwrap_or_else(|_| "Playlist".to_string());

                    let mut stmt = conn.prepare(
                        r#"
                        SELECT t.id, t.file_path, t.file_hash,
                               t.title, t.artist, t.album, t.year, t.genre, t.label, t.catalog_number,
                               t.duration_ms, t.bpm, t.key, t.energy, t.bitrate, t.sample_rate, t.format,
                               t.analysis_source, NULL as waveform_data,
                               t.rating, t.play_count,
                               t.date_added, t.date_modified, t.last_played,
                               t.rekordbox_id, t.artwork_path, t.artwork_source, t.color,
                               t.library_root_id, t.relative_path
                        FROM tracks t
                        JOIN playlist_tracks pt ON pt.track_id = t.id
                        WHERE pt.playlist_id = ?1
                        ORDER BY pt.position ASC
                        "#,
                    )?;

                    let rows = stmt.query_map([pid], |row| {
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
                            waveform_data: None,
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

                    let mut p_track_ids = Vec::new();
                    for t in rows.flatten() {
                        p_track_ids.push(t.id.clone());
                        if !all_tracks
                            .iter()
                            .any(|existing: &Track| existing.id == t.id)
                        {
                            all_tracks.push(t);
                        }
                    }
                    track_map.push((pid.clone(), playlist_name, p_track_ids));
                }
                (all_tracks, track_map)
            }
            _ => {
                // All tracks from library
                let mut stmt = conn.prepare(
                    r#"
                    SELECT id, file_path, file_hash,
                           title, artist, album, year, genre, label, catalog_number,
                           duration_ms, bpm, key, energy, bitrate, sample_rate, format,
                           analysis_source, NULL as waveform_data,
                           rating, play_count,
                           date_added, date_modified, last_played,
                           rekordbox_id, artwork_path, artwork_source, color,
                           library_root_id, relative_path
                    FROM tracks
                    ORDER BY date_added DESC
                    "#,
                )?;

                let rows = stmt.query_map([], |row| {
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
                        waveform_data: None,
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

                let tracks_vec: Vec<Track> = rows.flatten().collect();

                // Fetch all playlists
                let mut pl_stmt =
                    conn.prepare("SELECT id, name FROM playlists WHERE is_folder = 0")?;
                let pl_rows = pl_stmt.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                let mut track_map = Vec::new();
                for (pid, pname) in pl_rows.flatten() {
                    let mut pt_stmt = conn.prepare("SELECT track_id FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position ASC")?;
                    let t_ids: Vec<String> =
                        pt_stmt.query_map([&pid], |r| r.get(0))?.flatten().collect();
                    if !t_ids.is_empty() {
                        track_map.push((pid, pname, t_ids));
                    }
                }

                (tracks_vec, track_map)
            }
        };

        // 2. Fetch cues for all exported tracks
        let cues_by_track = Self::fetch_cues(&conn)?;

        // The database is not needed any more: release it before building and writing the file,
        // so the rest of the app is not blocked behind a large export.
        drop(conn);

        Self::write_rekordbox_xml(target_path, &tracks, &cues_by_track, &playlist_track_map)
    }

    /// Export a caller-ordered list of tracks (e.g. a Set-mode plan) to a standard Pioneer
    /// rekordbox.xml file, as a single playlist named `set_name` holding exactly that order.
    /// Track ids that no longer resolve to a library track (deleted since being added to the set)
    /// are skipped rather than failing the whole export; at least one must resolve.
    pub fn export_set_rekordbox_xml(
        &self,
        target_path: &Path,
        track_ids: &[String],
        set_name: &str,
    ) -> Result<usize> {
        if track_ids.is_empty() {
            return Err(CrateError::InvalidOperation(
                "A set needs at least one track".to_string(),
            ));
        }

        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let placeholders = vec!["?"; track_ids.len()].join(",");
        let sql = format!(
            r#"
            SELECT id, file_path, file_hash,
                   title, artist, album, year, genre, label, catalog_number,
                   duration_ms, bpm, key, energy, bitrate, sample_rate, format,
                   analysis_source, NULL as waveform_data,
                   rating, play_count,
                   date_added, date_modified, last_played,
                   rekordbox_id, artwork_path, artwork_source, color,
                   library_root_id, relative_path
            FROM tracks
            WHERE id IN ({placeholders})
            "#
        );
        // Scoped so `stmt` (and the rows it yields) are dropped here, before `conn` is dropped
        // below — otherwise the borrow checker sees `conn` still borrowed at that point.
        let by_id: std::collections::HashMap<String, Track> = {
            let mut stmt = conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::ToSql> = track_ids
                .iter()
                .map(|id| id as &dyn rusqlite::ToSql)
                .collect();
            let rows = stmt.query_map(params.as_slice(), |row| {
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
                    waveform_data: None,
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
            rows.flatten().map(|t| (t.id.clone(), t)).collect()
        };

        // Keep the caller's order, dropping ids that no longer exist in the library.
        let tracks: Vec<Track> = track_ids
            .iter()
            .filter_map(|id| by_id.get(id).cloned())
            .collect();
        if tracks.is_empty() {
            return Err(CrateError::InvalidOperation(
                "None of this set's tracks were found in the library".to_string(),
            ));
        }

        let cues_by_track = Self::fetch_cues(&conn)?;
        drop(conn);

        let playlist_track_map = vec![(
            "set".to_string(),
            set_name.to_string(),
            tracks.iter().map(|t| t.id.clone()).collect(),
        )];

        Self::write_rekordbox_xml(target_path, &tracks, &cues_by_track, &playlist_track_map)
    }

    /// Every cue, grouped by track id. Not filtered to a given track set (same as before this was
    /// extracted): callers only ever look up the ids they care about.
    fn fetch_cues(conn: &Connection) -> Result<std::collections::HashMap<String, Vec<Cue>>> {
        let mut cues_by_track: std::collections::HashMap<String, Vec<Cue>> =
            std::collections::HashMap::new();
        let mut cue_stmt = conn.prepare(
            r#"
            SELECT id, track_id, position_ms, type, loop_end_ms, hot_cue_index, name, color
            FROM cues
            ORDER BY position_ms ASC
            "#,
        )?;
        let cue_rows = cue_stmt.query_map([], |row| {
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
        for cue in cue_rows.flatten() {
            cues_by_track
                .entry(cue.track_id.clone())
                .or_default()
                .push(cue);
        }
        Ok(cues_by_track)
    }

    /// Builds the COLLECTION + PLAYLISTS XML from already-fetched data and writes it to
    /// `target_path`. Shared by the library/playlists export and the Set-mode export.
    fn write_rekordbox_xml(
        target_path: &Path,
        tracks: &[Track],
        cues_by_track: &std::collections::HashMap<String, Vec<Cue>>,
        playlist_track_map: &[(String, String, Vec<String>)],
    ) -> Result<usize> {
        let track_count = tracks.len();

        // Position of each track in the COLLECTION, for the playlists' `Key` attributes. A map
        // instead of a scan per playlist entry (which was O(tracks x entries)); the first
        // position wins, as it did with `position()`.
        let mut key_by_track: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::with_capacity(tracks.len());
        for (idx, track) in tracks.iter().enumerate() {
            key_by_track.entry(track.id.as_str()).or_insert(idx + 1);
        }

        // Build XML
        let mut xml = String::with_capacity(1024 * 1024);
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<DJ_PLAYLISTS Version=\"1.0.0\">\n");
        xml.push_str("  <PRODUCT Name=\"Crate\" Version=\"0.2.9\" Company=\"Crate\" />\n");
        xml.push_str(&format!("  <COLLECTION Entries=\"{}\">\n", track_count));

        for (idx, track) in tracks.iter().enumerate() {
            let track_no = idx + 1;
            let title = xml_escape(track.title.as_deref().unwrap_or("Untitled"));
            let artist = xml_escape(track.artist.as_deref().unwrap_or("Unknown Artist"));
            let album = xml_escape(track.album.as_deref().unwrap_or(""));
            let genre = xml_escape(track.genre.as_deref().unwrap_or(""));
            let label = xml_escape(track.label.as_deref().unwrap_or(""));
            let key = xml_escape(track.key.as_deref().unwrap_or(""));
            let total_time_secs = track.duration_ms.max(0) / 1000;
            // Unknown values stay unknown (0) instead of inventing 120 BPM / 320 kbps.
            let bpm_str = track
                .bpm
                .map(|b| format!("{:.2}", b))
                .unwrap_or_else(|| "0.00".to_string());
            let bitrate = track.bitrate.unwrap_or(0);
            let sample_rate = track.sample_rate.unwrap_or(0);
            let year = track.year.map(|y| y.to_string()).unwrap_or_default();
            let location = xml_escape(&file_path_to_url(&track.file_path));

            xml.push_str(&format!(
                "    <TRACK TrackID=\"{}\" Name=\"{}\" Artist=\"{}\" Album=\"{}\" Genre=\"{}\" Label=\"{}\" \
                 TotalTime=\"{}\" AverageBpm=\"{}\" BitRate=\"{}\" SampleRate=\"{}\" Year=\"{}\" \
                 Tonality=\"{}\" Location=\"{}\" PlayCount=\"{}\" Rating=\"{}\">\n",
                track_no, title, artist, album, genre, label, total_time_secs, bpm_str, bitrate, sample_rate, year, key, location, track.play_count, track.rating
            ));

            if let Some(bpm) = track.bpm {
                xml.push_str(&format!(
                    "      <TEMPO Inizio=\"0.00\" Bpm=\"{:.2}\" Metro=\"4/4\" Battito=\"1\" />\n",
                    bpm
                ));
            }

            if let Some(cues) = cues_by_track.get(&track.id) {
                for cue in cues {
                    xml.push_str(&position_mark(cue));
                }
            }

            xml.push_str("    </TRACK>\n");
        }

        xml.push_str("  </COLLECTION>\n");
        xml.push_str("  <PLAYLISTS>\n");
        xml.push_str("    <NODE Type=\"0\" Name=\"ROOT\">\n");

        for (_pid, pname, track_ids) in playlist_track_map {
            let p_escaped = xml_escape(pname);
            xml.push_str(&format!(
                "      <NODE Name=\"{}\" Type=\"1\" KeyType=\"0\" Entries=\"{}\">\n",
                p_escaped,
                track_ids.len()
            ));
            for tid in track_ids {
                if let Some(key) = key_by_track.get(tid.as_str()) {
                    xml.push_str(&format!("        <TRACK Key=\"{key}\" />\n"));
                }
            }
            xml.push_str("      </NODE>\n");
        }

        xml.push_str("    </NODE>\n");
        xml.push_str("  </PLAYLISTS>\n");
        xml.push_str("</DJ_PLAYLISTS>\n");

        // Write file
        let mut file = File::create(target_path).map_err(CrateError::Io)?;
        file.write_all(xml.as_bytes()).map_err(CrateError::Io)?;

        Ok(track_count)
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// `file://localhost/...` URL as Rekordbox writes it: UTF-8 bytes percent-encoded, keeping only
/// unreserved characters and `/` literal (so `&`, `#`, `%`, spaces and accents are all encoded).
fn file_path_to_url(path_str: &str) -> String {
    let clean = path_str.replace('\\', "/");
    let mut encoded = String::with_capacity(clean.len() + 16);
    for byte in clean.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    if encoded.starts_with('/') {
        format!("file://localhost{encoded}")
    } else {
        format!("file://localhost/{encoded}")
    }
}

/// One `POSITION_MARK`: hot cues use `Num` 0–7, memory cues `Num="-1"`, loops `Type="4"` with `End`.
fn position_mark(cue: &Cue) -> String {
    let start_secs = cue.position_ms.max(0) as f64 / 1000.0;
    let name = xml_escape(cue.name.as_deref().unwrap_or(""));
    let num = match (&cue.cue_type, cue.hot_cue_index) {
        (CueType::Memory, _) | (_, None) => -1,
        (_, Some(index)) => index.clamp(0, 7),
    };
    let (mark_type, end) = match (&cue.cue_type, cue.loop_end_ms) {
        (CueType::Loop, Some(end_ms)) if end_ms > cue.position_ms => {
            ("4", format!(" End=\"{:.3}\"", end_ms as f64 / 1000.0))
        }
        _ => ("0", String::new()),
    };
    let colour = cue
        .color
        .as_deref()
        .and_then(parse_hex_colour)
        .filter(|_| num >= 0)
        .map(|(r, g, b)| format!(" Red=\"{r}\" Green=\"{g}\" Blue=\"{b}\""))
        .unwrap_or_default();
    format!("      <POSITION_MARK Name=\"{name}\" Type=\"{mark_type}\" Start=\"{start_secs:.3}\"{end} Num=\"{num}\"{colour} />\n")
}

fn parse_hex_colour(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some((channel(0)?, channel(2)?, channel(4)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cue(cue_type: CueType, hot: Option<i32>, loop_end: Option<i64>) -> Cue {
        Cue {
            id: "c".into(),
            track_id: "t".into(),
            position_ms: 32_500,
            cue_type,
            loop_end_ms: loop_end,
            hot_cue_index: hot,
            name: Some("Drop & Build".into()),
            color: Some("#CC0000".into()),
        }
    }

    #[test]
    fn test_location_is_percent_encoded() {
        assert_eq!(
            file_path_to_url("/Music/Rock & Roll/Café #1 100%.mp3"),
            "file://localhost/Music/Rock%20%26%20Roll/Caf%C3%A9%20%231%20100%25.mp3"
        );
    }

    #[test]
    fn test_position_marks_follow_rekordbox_numbering() {
        let hot = position_mark(&cue(CueType::Hot, Some(2), None));
        assert!(
            hot.contains(r#"Num="2""#) && hot.contains(r#"Red="204""#),
            "{hot}"
        );
        assert!(hot.contains("Drop &amp; Build"), "names are escaped");
        let memory = position_mark(&cue(CueType::Memory, None, None));
        assert!(memory.contains(r#"Num="-1""#), "{memory}");
        let looped = position_mark(&cue(CueType::Loop, None, Some(40_500)));
        assert!(
            looped.contains(r#"Type="4""#) && looped.contains(r#"End="40.500""#),
            "{looped}"
        );
    }

    #[test]
    fn test_exported_xml_parses_with_special_characters() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO tracks (id, file_path, title, artist, format, duration_ms, date_added, date_modified)
               VALUES ('t1', '/Music/Tom & Jerry <live> \"quoted\".mp3', 'A & B', 'C < D', 'mp3', 200000, '2026-01-01', '2026-01-01');
             INSERT INTO cues (id, track_id, position_ms, type, hot_cue_index, name) VALUES ('c1', 't1', 1000, 'memory', NULL, 'M & M');",
        )
        .unwrap();
        let service = ExportService::new(std::sync::Arc::new(std::sync::Mutex::new(conn)));
        let path = std::env::temp_dir().join(format!("crate_rb_{}.xml", std::process::id()));
        service.export_rekordbox_xml(&path, None).unwrap();
        let xml = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(!xml.contains(" & "), "no raw ampersand may remain");
        assert!(xml.contains("Tom%20%26%20Jerry"), "{xml}");
        assert!(xml.contains(r#"Num="-1""#));
        // Every attribute value is well formed: no bare '<' or '&' inside quotes
        let attr_re = regex::Regex::new(r#"="([^"]*)""#).unwrap();
        for cap in attr_re.captures_iter(&xml) {
            let value = &cap[1];
            assert!(!value.contains('<'), "unescaped < in {value}");
            assert!(
                value.split('&').skip(1).all(|rest| rest.starts_with("amp;")
                    || rest.starts_with("lt;")
                    || rest.starts_with("gt;")
                    || rest.starts_with("quot;")
                    || rest.starts_with("apos;")),
                "unescaped & in {value}"
            );
        }
    }
    /// `Key` of each playlist entry is the 1-based position of the track in the COLLECTION, where
    /// every track is listed once even when it sits in several playlists.
    #[test]
    fn test_playlist_keys_are_collection_positions_without_duplicates() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO tracks (id, file_path, title, artist, format, duration_ms, date_added, date_modified) VALUES
               ('t1', '/Music/one.mp3', 'One', 'A', 'mp3', 200000, '2026-01-01', '2026-01-01'),
               ('t2', '/Music/two.mp3', 'Two', 'A', 'mp3', 200000, '2026-01-01', '2026-01-01'),
               ('t3', '/Music/three.mp3', 'Three', 'A', 'mp3', 200000, '2026-01-01', '2026-01-01');
             INSERT INTO playlists (id, name, date_created, date_modified) VALUES
               ('p1', 'First', '2026-01-01', '2026-01-01'),
               ('p2', 'Second', '2026-01-01', '2026-01-01');
             INSERT INTO playlist_tracks (playlist_id, track_id, position, date_added) VALUES
               ('p1', 't2', 0, '2026-01-01'), ('p1', 't1', 1, '2026-01-01'),
               ('p2', 't1', 0, '2026-01-01'), ('p2', 't3', 1, '2026-01-01');",
        )
        .unwrap();
        let service = ExportService::new(std::sync::Arc::new(std::sync::Mutex::new(conn)));
        let path = std::env::temp_dir().join(format!("crate_rb_keys_{}.xml", std::process::id()));
        service
            .export_rekordbox_xml(&path, Some(vec!["p1".to_string(), "p2".to_string()]))
            .unwrap();
        let xml = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        // COLLECTION order: t2, t1 (from p1), then t3 (t1 is already there).
        let keys = |playlist: &str| -> Vec<u32> {
            let node = xml
                .split(&format!("<NODE Name=\"{playlist}\""))
                .nth(1)
                .unwrap()
                .split("</NODE>")
                .next()
                .unwrap();
            node.split("<TRACK Key=\"")
                .skip(1)
                .map(|t| t.split('"').next().unwrap().parse().unwrap())
                .collect()
        };
        assert_eq!(keys("First"), vec![1, 2]);
        assert_eq!(keys("Second"), vec![2, 3]);
    }

    fn seed_tracks(conn: &rusqlite::Connection) {
        conn.execute_batch(
            "INSERT INTO tracks (id, file_path, title, artist, format, duration_ms, date_added, date_modified) VALUES
               ('t1', '/Music/one.mp3', 'One', 'A', 'mp3', 200000, '2026-01-01', '2026-01-01'),
               ('t2', '/Music/two.mp3', 'Two', 'A', 'mp3', 200000, '2026-01-01', '2026-01-01'),
               ('t3', '/Music/three.mp3', 'Three', 'A', 'mp3', 200000, '2026-01-01', '2026-01-01');",
        )
        .unwrap();
    }

    /// A Set-mode export writes the caller's order, not the COLLECTION's insertion order, and
    /// groups every track under a single playlist node named after the set.
    #[test]
    fn test_set_export_keeps_the_caller_order_in_a_single_named_playlist() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        seed_tracks(&conn);
        let service = ExportService::new(std::sync::Arc::new(std::sync::Mutex::new(conn)));
        let path = std::env::temp_dir().join(format!("crate_rb_set_{}.xml", std::process::id()));

        let track_ids = vec!["t3".to_string(), "t1".to_string(), "t2".to_string()];
        let count = service
            .export_set_rekordbox_xml(&path, &track_ids, "Friday warehouse")
            .unwrap();
        assert_eq!(count, 3);

        let xml = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert!(xml.contains("<NODE Name=\"Friday warehouse\" Type=\"1\" KeyType=\"0\" Entries=\"3\">"));

        // The COLLECTION itself is written in the caller's order (Three, One, Two) — not the
        // table's insertion order (One, Two, Three).
        let names: Vec<&str> = xml
            .split("<TRACK TrackID=")
            .skip(1)
            .map(|entry| entry.split("Name=\"").nth(1).unwrap().split('"').next().unwrap())
            .collect();
        assert_eq!(names, vec!["Three", "One", "Two"]);

        // Exactly one playlist node, referencing every COLLECTION position once, in order.
        let node = xml
            .split("<NODE Name=\"Friday warehouse\"")
            .nth(1)
            .unwrap()
            .split("</NODE>")
            .next()
            .unwrap();
        let keys: Vec<u32> = node
            .split("<TRACK Key=\"")
            .skip(1)
            .map(|t| t.split('"').next().unwrap().parse().unwrap())
            .collect();
        assert_eq!(keys, vec![1, 2, 3]);
    }

    /// A track id removed from the library since being added to the set is skipped, not fatal.
    #[test]
    fn test_set_export_skips_unresolvable_track_ids() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        seed_tracks(&conn);
        let service = ExportService::new(std::sync::Arc::new(std::sync::Mutex::new(conn)));
        let path = std::env::temp_dir().join(format!("crate_rb_set_missing_{}.xml", std::process::id()));

        let track_ids = vec!["t1".to_string(), "deleted".to_string(), "t2".to_string()];
        let count = service
            .export_set_rekordbox_xml(&path, &track_ids, "Set")
            .unwrap();
        assert_eq!(count, 2);
        let _ = std::fs::remove_file(&path);
    }

    /// An empty set, or a set made entirely of ids that no longer exist, is rejected outright
    /// rather than silently writing an empty file.
    #[test]
    fn test_set_export_rejects_empty_or_fully_unresolvable_sets() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        seed_tracks(&conn);
        let service = ExportService::new(std::sync::Arc::new(std::sync::Mutex::new(conn)));
        let path = std::env::temp_dir().join(format!("crate_rb_set_empty_{}.xml", std::process::id()));

        assert!(service.export_set_rekordbox_xml(&path, &[], "Set").is_err());
        assert!(service
            .export_set_rekordbox_xml(&path, &["ghost".to_string()], "Set")
            .is_err());
    }
}

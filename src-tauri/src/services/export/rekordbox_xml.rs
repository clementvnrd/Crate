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
                        .query_row(
                            "SELECT name FROM playlists WHERE id = ?1",
                            [pid],
                            |r| r.get(0),
                        )
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
                        if !all_tracks.iter().any(|existing: &Track| existing.id == t.id) {
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
                let mut pl_stmt = conn.prepare("SELECT id, name FROM playlists WHERE is_folder = 0")?;
                let pl_rows = pl_stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
                let mut track_map = Vec::new();
                for (pid, pname) in pl_rows.flatten() {
                    let mut pt_stmt = conn.prepare("SELECT track_id FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position ASC")?;
                    let t_ids: Vec<String> = pt_stmt.query_map([&pid], |r| r.get(0))?.flatten().collect();
                    if !t_ids.is_empty() {
                        track_map.push((pid, pname, t_ids));
                    }
                }

                (tracks_vec, track_map)
            }
        };

        let track_count = tracks.len();

        // 2. Fetch cues for all exported tracks
        let mut cues_by_track: std::collections::HashMap<String, Vec<Cue>> = std::collections::HashMap::new();
        {
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
                cues_by_track.entry(cue.track_id.clone()).or_default().push(cue);
            }
        }

        // 3. Build XML
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
            let bpm_str = track.bpm.map(|b| format!("{:.2}", b)).unwrap_or_else(|| "120.00".to_string());
            let bitrate = track.bitrate.unwrap_or(320);
            let sample_rate = track.sample_rate.unwrap_or(44100);
            let year = track.year.map(|y| y.to_string()).unwrap_or_default();
            let location = file_path_to_url(&track.file_path);

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
                    let start_secs = cue.position_ms as f64 / 1000.0;
                    let cue_num = cue.hot_cue_index.map(|i| (i - 1).max(0)).unwrap_or(0);
                    let cue_name = xml_escape(cue.name.as_deref().unwrap_or("Cue"));
                    xml.push_str(&format!(
                        "      <POSITION_MARK Name=\"{}\" Type=\"0\" Start=\"{:.3}\" Num=\"{}\" Red=\"40\" Green=\"220\" Blue=\"180\" />\n",
                        cue_name, start_secs, cue_num
                    ));
                }
            }

            xml.push_str("    </TRACK>\n");
        }

        xml.push_str("  </COLLECTION>\n");
        xml.push_str("  <PLAYLISTS>\n");
        xml.push_str("    <NODE Type=\"0\" Name=\"ROOT\">\n");

        for (_pid, pname, track_ids) in playlist_track_map {
            let p_escaped = xml_escape(&pname);
            xml.push_str(&format!(
                "      <NODE Name=\"{}\" Type=\"1\" KeyType=\"0\" Entries=\"{}\">\n",
                p_escaped,
                track_ids.len()
            ));
            for tid in track_ids {
                if let Some(pos) = tracks.iter().position(|t| t.id == tid) {
                    xml.push_str(&format!("        <TRACK Key=\"{}\" />\n", pos + 1));
                }
            }
            xml.push_str("      </NODE>\n");
        }

        xml.push_str("    </NODE>\n");
        xml.push_str("  </PLAYLISTS>\n");
        xml.push_str("</DJ_PLAYLISTS>\n");

        // Write file
        let mut file = File::create(target_path)
            .map_err(CrateError::Io)?;
        file.write_all(xml.as_bytes())
            .map_err(CrateError::Io)?;

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

fn file_path_to_url(path_str: &str) -> String {
    let clean = path_str.replace('\\', "/");
    let encoded: String = clean
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '#' => "%23".to_string(),
            '%' => "%25".to_string(),
            _ => c.to_string(),
        })
        .collect();

    if encoded.starts_with('/') {
        format!("file://localhost{}", encoded)
    } else {
        format!("file://localhost/{}", encoded)
    }
}

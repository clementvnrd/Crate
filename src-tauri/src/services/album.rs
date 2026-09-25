use std::collections::HashMap;
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
use uuid::Uuid;
use walkdir::WalkDir;

use crate::error::{CrateError, Result};
use crate::models::album::{AddAlbumResult, PlayerAlbum, PlayerAlbumTrack};
use crate::services::library::MikService;
use crate::services::ArtworkService;

pub struct AlbumService {
    conn: Arc<Mutex<Connection>>,
    artwork_service: ArtworkService,
}

impl AlbumService {
    pub fn new(conn: Arc<Mutex<Connection>>, app_data_dir: PathBuf) -> Self {
        Self {
            conn,
            artwork_service: ArtworkService::new(app_data_dir),
        }
    }

    /// Add an album by scanning a directory for audio tracks.
    /// Extracts ID3/Vorbis/MP4 tags, embedded/folder artwork, computes total duration,
    /// and persists to `player_albums` and `player_album_tracks`.
    pub fn add_album_from_folder(&self, folder_path_str: &str) -> Result<AddAlbumResult> {
        let folder_path = PathBuf::from(folder_path_str);
        if !folder_path.exists() || !folder_path.is_dir() {
            return Err(CrateError::InvalidOperation(format!(
                "Le chemin spécifié n'est pas un dossier valide: {}",
                folder_path_str
            )));
        }

        // 1. Scan directory for audio files (up to depth 2 to support CD1/CD2 subfolders)
        let supported_exts = [
            "mp3", "flac", "aiff", "aif", "m4a", "aac", "wav", "wave", "ogg", "opus", "alac",
        ];

        let mut audio_files = Vec::new();
        for entry in WalkDir::new(&folder_path)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                    if supported_exts.contains(&ext.to_lowercase().as_str()) {
                        audio_files.push(entry.into_path());
                    }
                }
            }
        }

        if audio_files.is_empty() {
            return Err(CrateError::InvalidOperation(
                "Aucun fichier audio supporté trouvé dans ce dossier.".to_string(),
            ));
        }

        // Sort files naturally by filename as initial baseline
        audio_files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        let album_id = Uuid::new_v4().to_string();
        let mut raw_tracks = Vec::new();
        let mut album_name_counts: HashMap<String, usize> = HashMap::new();
        let mut artist_counts: HashMap<String, usize> = HashMap::new();
        let mut genre_counts: HashMap<String, usize> = HashMap::new();
        let mut detected_year: Option<i32> = None;
        let mut detected_album_art: Option<String> = None;

        // Check for folder cover art first (cover.jpg, folder.png, etc.)
        let folder_art_candidates = [
            "cover.jpg", "cover.png", "cover.jpeg", "cover.webp",
            "folder.jpg", "folder.png", "folder.jpeg",
            "front.jpg", "front.png", "front.jpeg",
            "albumart.jpg", "album.jpg", "art.jpg",
            "Cover.jpg", "Cover.png", "Folder.jpg", "Front.jpg",
        ];
        for candidate in &folder_art_candidates {
            let img_path = folder_path.join(candidate);
            if img_path.exists() {
                if let Some(saved) = self.artwork_service.save_from_file(&img_path, &format!("album_{}", album_id)) {
                    detected_album_art = Some(saved);
                    break;
                }
            }
        }

        for path in &audio_files {
            let track_id = Uuid::new_v4().to_string();
            let file_path_str = path.to_string_lossy().to_string();
            let format = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_else(|| "mp3".to_string());

            let mut title = None;
            let mut artist = None;
            let mut album = None;
            let mut track_number = None;
            let mut duration_ms = 0i64;
            let mut bitrate = None;
            let mut sample_rate = None;
            let mut bpm = None;
            let mut key = None;
            let mut energy = None;
            let mut artwork_path = None;

            if let Some(tagged_file) = Self::read_metadata_lenient(path) {
                let properties = tagged_file.properties();
                duration_ms = properties.duration().as_millis() as i64;
                bitrate = properties.audio_bitrate().map(|b| b as i32);
                sample_rate = properties.sample_rate().map(|s| s as i32);

                if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
                    title = tag.title().map(|s| s.trim().to_string());
                    artist = tag.artist().map(|s| s.trim().to_string());
                    album = tag.album().map(|s| s.trim().to_string());
                    track_number = tag.track().map(|t| t as i32);

                    if detected_year.is_none() {
                        if let Some(year) = tag.year() {
                            detected_year = Some(year as i32);
                        } else if let Some(date_str) = tag.get_string(&lofty::tag::ItemKey::RecordingDate) {
                            if let Ok(y) = date_str.chars().take(4).collect::<String>().parse::<i32>() {
                                if (1900..=2099).contains(&y) {
                                    detected_year = Some(y);
                                }
                            }
                        }
                    }

                    if let Some(genre) = tag.genre() {
                        let g = genre.trim().to_string();
                        if !g.is_empty() {
                            *genre_counts.entry(g).or_insert(0) += 1;
                        }
                    }
                }

                let mik_data = MikService::extract_analysis_data(&tagged_file, &track_id);
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
                    .extract_from_tagged_file_or_folder(&tagged_file, path, &track_id)
                {
                    artwork_path = Some(art.clone());
                    if detected_album_art.is_none() {
                        detected_album_art = Some(art);
                    }
                }
            } else if let Ok((dur, sr, br)) = Self::read_audio_properties_symphonia(path) {
                duration_ms = dur;
                sample_rate = sr;
                bitrate = br;
            }

            // Fallback title to filename stem if empty
            let final_title = title.unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Piste")
                    .to_string()
            });

            // Fallback artist to folder or unknown
            let final_artist = artist.unwrap_or_else(|| "Artiste inconnu".to_string());

            if let Some(ref alb) = album {
                if !alb.is_empty() {
                    *album_name_counts.entry(alb.clone()).or_insert(0) += 1;
                }
            }
            *artist_counts.entry(final_artist.clone()).or_insert(0) += 1;

            raw_tracks.push(PlayerAlbumTrack {
                id: track_id,
                album_id: album_id.clone(),
                file_path: file_path_str,
                track_number,
                title: final_title,
                artist: final_artist,
                duration_ms,
                format,
                bitrate,
                sample_rate,
                bpm,
                key,
                energy,
                artwork_path,
            });
        }

        // Determine dominant Album Title
        let folder_name = folder_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Album")
            .to_string();

        let album_title = album_name_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, _)| name)
            .unwrap_or(folder_name);

        // Determine dominant Artist
        let dominant_artist = artist_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, _)| name)
            .unwrap_or_else(|| "Artiste inconnu".to_string());

        // Determine dominant Genre
        let dominant_genre = genre_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, _)| name);

        // Sort tracks by track number, then path
        raw_tracks.sort_by(|a, b| {
            match (a.track_number, b.track_number) {
                (Some(na), Some(nb)) if na != nb => na.cmp(&nb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                _ => a.file_path.cmp(&b.file_path),
            }
        });

        // Ensure track artwork falls back to detected album artwork
        if let Some(ref alb_art) = detected_album_art {
            for track in &mut raw_tracks {
                if track.artwork_path.is_none() {
                    track.artwork_path = Some(alb_art.clone());
                }
            }
        }

        let total_duration_ms: i64 = raw_tracks.iter().map(|t| t.duration_ms).sum();
        let track_count = raw_tracks.len() as i32;
        let created_at = chrono::Utc::now().to_rfc3339();

        let album = PlayerAlbum {
            id: album_id.clone(),
            folder_path: folder_path_str.to_string(),
            title: album_title,
            artist: dominant_artist,
            year: detected_year,
            genre: dominant_genre,
            artwork_path: detected_album_art,
            track_count,
            total_duration_ms,
            created_at,
        };

        // Persist to database
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // Check if an album already exists with this folder_path
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM player_albums WHERE folder_path = ?1",
                rusqlite::params![folder_path_str],
                |row| row.get(0),
            )
            .optional()
            .map_err(CrateError::Database)?;

        let final_album_id = existing_id.unwrap_or(album_id);

        let final_album = PlayerAlbum {
            id: final_album_id.clone(),
            ..album
        };

        let mut final_tracks = Vec::new();
        for mut t in raw_tracks {
            t.album_id = final_album_id.clone();
            final_tracks.push(t);
        }

        // Delete any existing tracks for this album
        conn.execute(
            "DELETE FROM player_album_tracks WHERE album_id = ?1",
            rusqlite::params![final_album_id],
        )
        .map_err(CrateError::Database)?;

        // Insert or replace album
        conn.execute(
            r#"
            INSERT INTO player_albums (
                id, folder_path, title, artist, year, genre, artwork_path,
                track_count, total_duration_ms, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(folder_path) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                year = excluded.year,
                genre = excluded.genre,
                artwork_path = excluded.artwork_path,
                track_count = excluded.track_count,
                total_duration_ms = excluded.total_duration_ms
            "#,
            rusqlite::params![
                final_album.id,
                final_album.folder_path,
                final_album.title,
                final_album.artist,
                final_album.year,
                final_album.genre,
                final_album.artwork_path,
                final_album.track_count,
                final_album.total_duration_ms,
                final_album.created_at,
            ],
        )
        .map_err(CrateError::Database)?;

        // Insert tracks
        for track in &final_tracks {
            conn.execute(
                r#"
                INSERT INTO player_album_tracks (
                    id, album_id, file_path, track_number, title, artist,
                    duration_ms, format, bitrate, sample_rate, bpm, key, energy, artwork_path
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                ON CONFLICT(file_path) DO UPDATE SET
                    album_id = excluded.album_id,
                    track_number = excluded.track_number,
                    title = excluded.title,
                    artist = excluded.artist,
                    duration_ms = excluded.duration_ms,
                    format = excluded.format,
                    bitrate = excluded.bitrate,
                    sample_rate = excluded.sample_rate,
                    bpm = excluded.bpm,
                    key = excluded.key,
                    energy = excluded.energy,
                    artwork_path = excluded.artwork_path
                "#,
                rusqlite::params![
                    track.id,
                    track.album_id,
                    track.file_path,
                    track.track_number,
                    track.title,
                    track.artist,
                    track.duration_ms,
                    track.format,
                    track.bitrate,
                    track.sample_rate,
                    track.bpm,
                    track.key,
                    track.energy,
                    track.artwork_path,
                ],
            )
            .map_err(CrateError::Database)?;
        }

        Ok(AddAlbumResult {
            album: final_album,
            tracks: final_tracks,
        })
    }

    /// Retrieve all imported albums ordered by most recent addition
    pub fn get_albums(&self) -> Result<Vec<PlayerAlbum>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, folder_path, title, artist, year, genre, artwork_path,
                       track_count, total_duration_ms, created_at
                FROM player_albums
                ORDER BY created_at DESC
                "#,
            )
            .map_err(CrateError::Database)?;

        let rows = stmt
            .query_map([], |row| {
                Ok(PlayerAlbum {
                    id: row.get(0)?,
                    folder_path: row.get(1)?,
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    year: row.get(4)?,
                    genre: row.get(5)?,
                    artwork_path: row.get(6)?,
                    track_count: row.get(7)?,
                    total_duration_ms: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })
            .map_err(CrateError::Database)?;

        let mut albums = Vec::new();
        for item in rows {
            albums.push(item.map_err(CrateError::Database)?);
        }
        Ok(albums)
    }

    /// Retrieve all tracks for a specific album ordered by track number
    pub fn get_album_tracks(&self, album_id: &str) -> Result<Vec<PlayerAlbumTrack>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, album_id, file_path, track_number, title, artist,
                       duration_ms, format, bitrate, sample_rate, bpm, key, energy, artwork_path
                FROM player_album_tracks
                WHERE album_id = ?1
                ORDER BY CASE WHEN track_number IS NULL OR track_number = 0 THEN 9999 ELSE track_number END ASC, file_path ASC
                "#,
            )
            .map_err(CrateError::Database)?;

        let rows = stmt
            .query_map(rusqlite::params![album_id], |row| {
                Ok(PlayerAlbumTrack {
                    id: row.get(0)?,
                    album_id: row.get(1)?,
                    file_path: row.get(2)?,
                    track_number: row.get(3)?,
                    title: row.get(4)?,
                    artist: row.get(5)?,
                    duration_ms: row.get(6)?,
                    format: row.get(7)?,
                    bitrate: row.get(8)?,
                    sample_rate: row.get(9)?,
                    bpm: row.get(10)?,
                    key: row.get(11)?,
                    energy: row.get(12)?,
                    artwork_path: row.get(13)?,
                })
            })
            .map_err(CrateError::Database)?;

        let mut tracks = Vec::new();
        for item in rows {
            tracks.push(item.map_err(CrateError::Database)?);
        }
        Ok(tracks)
    }

    /// Remove an album and its associated tracks from Crate Player
    pub fn remove_album(&self, album_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        conn.execute(
            "DELETE FROM player_album_tracks WHERE album_id = ?1",
            rusqlite::params![album_id],
        )
        .map_err(CrateError::Database)?;

        conn.execute(
            "DELETE FROM player_albums WHERE id = ?1",
            rusqlite::params![album_id],
        )
        .map_err(CrateError::Database)?;

        Ok(())
    }

    /// Helper for lenient metadata reading with lofty
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

    /// Helper for symphonia audio properties fallback
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::get_migrations;
    use std::path::PathBuf;

    fn setup_test_db() -> (Arc<Mutex<rusqlite::Connection>>, PathBuf) {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        for migration in get_migrations() {
            conn.execute_batch(migration).unwrap();
        }
        let conn_arc = Arc::new(Mutex::new(conn));
        (conn_arc, std::env::temp_dir())
    }

    #[test]
    fn test_album_crud_flow() {
        let (conn, dir) = setup_test_db();
        let service = AlbumService::new(conn.clone(), dir);


        // Test with empty/non-existent folder
        let invalid = service.add_album_from_folder("/non/existent/path");
        assert!(invalid.is_err());

        // Verify empty get_albums
        let albums = service.get_albums().unwrap();
        assert_eq!(albums.len(), 0);

        // Manually insert an album to test queries and cascade deletion
        let album_id = uuid::Uuid::new_v4().to_string();
        {
            let conn_guard = conn.lock().unwrap();
            conn_guard.execute(
                r#"
                INSERT INTO player_albums (
                    id, folder_path, title, artist, year, genre, artwork_path,
                    track_count, total_duration_ms, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                rusqlite::params![
                    album_id,
                    "/Users/test/Music/Discovery",
                    "Discovery",
                    "Daft Punk",
                    2001,
                    "Electronic",
                    None::<String>,
                    2,
                    450000,
                    chrono::Utc::now().to_rfc3339()
                ],
            ).unwrap();

            conn_guard.execute(
                r#"
                INSERT INTO player_album_tracks (
                    id, album_id, file_path, track_number, title, artist,
                    duration_ms, format, bitrate, sample_rate, bpm, key, energy, artwork_path
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                "#,
                rusqlite::params![
                    uuid::Uuid::new_v4().to_string(),
                    album_id,
                    "/Users/test/Music/Discovery/01 One More Time.flac",
                    1,
                    "One More Time",
                    "Daft Punk",
                    320000,
                    "flac",
                    1411,
                    44100,
                    123.0,
                    "11B",
                    8,
                    None::<String>
                ],
            ).unwrap();

            conn_guard.execute(
                r#"
                INSERT INTO player_album_tracks (
                    id, album_id, file_path, track_number, title, artist,
                    duration_ms, format, bitrate, sample_rate, bpm, key, energy, artwork_path
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                "#,
                rusqlite::params![
                    uuid::Uuid::new_v4().to_string(),
                    album_id,
                    "/Users/test/Music/Discovery/02 Aerodynamic.flac",
                    2,
                    "Aerodynamic",
                    "Daft Punk",
                    130000,
                    "flac",
                    1411,
                    44100,
                    123.0,
                    "4A",
                    9,
                    None::<String>
                ],
            ).unwrap();
        }

        // Test get_albums
        let albums = service.get_albums().unwrap();
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].title, "Discovery");
        assert_eq!(albums[0].artist, "Daft Punk");
        assert_eq!(albums[0].track_count, 2);

        // Test get_album_tracks
        let tracks = service.get_album_tracks(&album_id).unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "One More Time");
        assert_eq!(tracks[0].track_number, Some(1));
        assert_eq!(tracks[1].title, "Aerodynamic");
        assert_eq!(tracks[1].track_number, Some(2));

        // Test remove_album
        service.remove_album(&album_id).unwrap();
        let albums_after = service.get_albums().unwrap();
        assert_eq!(albums_after.len(), 0);
        let tracks_after = service.get_album_tracks(&album_id).unwrap();
        assert_eq!(tracks_after.len(), 0);
    }
}

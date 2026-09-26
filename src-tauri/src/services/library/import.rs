use std::fs::File;
use std::io::BufReader;

use lofty::config::{ParseOptions, ParsingMode};
use lofty::file::{AudioFile, TaggedFile};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::Tag;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;

use super::*;
use crate::models::Cue;
use crate::services::cloud_sync::pipeline::{buckets, dirty};
use crate::services::cloud_sync::resolution;

impl LibraryService {
    pub fn import_tracks(&self, paths: Vec<PathBuf>) -> Result<ImportResult> {
        let mut tracks = Vec::new();
        let mut errors = Vec::new();

        for path in paths {
            match self.import_single_track(&path) {
                Ok(track) => tracks.push(track),
                Err(e) => {
                    let error_msg = format!("{}: {}", path.display(), e);
                    log::warn!("Failed to import {error_msg}");
                    errors.push(error_msg);
                }
            }
        }

        Ok(ImportResult {
            tracks,
            failed_count: errors.len(),
            errors,
        })
    }

    fn import_single_track(&self, path: &PathBuf) -> Result<Track> {
        if !path.exists() {
            return Err(CrateError::FileNotFound(path.clone()));
        }

        // Determine format from extension
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        // Check if supported format
        let supported_formats = ["mp3", "wav", "aiff", "aif", "flac", "m4a", "aac"];
        if !supported_formats.contains(&format.as_str()) {
            return Err(CrateError::Import(format!("Unsupported format: {format}")));
        }

        // Try to read metadata with lenient parsing first
        let mut track = Track::new(
            path.to_string_lossy().to_string(),
            format.clone(),
            0, // Duration will be set below
        );
        // Content hash, stored with the track for future relocation matching
        track.file_hash = compute_audio_hash(path).ok();

        if let Some(tagged_file) = self.read_metadata_lenient(path) {
            // Successfully read with lofty
            let properties = tagged_file.properties();
            track.duration_ms = properties.duration().as_millis() as i64;
            track.bitrate = properties.audio_bitrate().map(|b| b as i32);
            track.sample_rate = properties.sample_rate().map(|s| s as i32);

            // Extract tags if available
            if let Some(tag) = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag())
            {
                track.title = tag.title().map(|s| s.to_string());
                track.artist = tag.artist().map(|s| s.to_string());
                track.album = tag.album().map(|s| s.to_string());
                track.year = tag.year().map(|y| y as i32);
                track.genre = tag.genre().map(|s| s.to_string());
            }

            // Extract Mixed In Key & audio analysis metadata (BPM, Key, Energy, Cues)
            let mik_data = MikService::extract_analysis_data(&tagged_file, &track.id);
            if let Some(bpm) = mik_data.bpm {
                track.bpm = Some(bpm);
            }
            if let Some(ref key) = mik_data.key {
                track.key = Some(key.clone());
            }
            if let Some(energy) = mik_data.energy {
                track.energy = Some(energy);
            }
            if mik_data.is_mik {
                track.analysis_source = Some("mixed_in_key".to_string());
            }

            // Extract album artwork
            if let Some(artwork_path) = self
                .artwork_service
                .extract_and_save(&tagged_file, &track.id)
            {
                track.artwork_path = Some(artwork_path);
                track.artwork_source = Some("extracted".to_string());
            }

            // Insert into database (a re-imported file keeps its existing id)
            track.id = self.insert_track(&track)?;

            // Insert any extracted hot cues, attached to the stored track
            if !mik_data.cues.is_empty() {
                let cues: Vec<Cue> = mik_data
                    .cues
                    .into_iter()
                    .map(|mut cue| {
                        cue.track_id = track.id.clone();
                        cue
                    })
                    .collect();
                self.insert_cues(&cues)?;
            }
        } else {
            // Lofty failed completely, use symphonia fallback
            log::warn!(
                "Metadata extraction failed for {}, falling back to symphonia",
                path.display()
            );

            let (dur, sr, br) = self.read_audio_properties_symphonia(path)?;
            track.duration_ms = dur;
            track.sample_rate = sr;
            track.bitrate = br;

            // Insert into database
            track.id = self.insert_track(&track)?;
        }

        Ok(track)
    }

    /// Import a single track with a pre-computed hash
    pub(crate) fn import_single_track_with_hash(
        &self,
        path: &PathBuf,
        file_hash: String,
    ) -> Result<Track> {
        // Determine format from extension
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        // Create track with pre-computed hash
        let mut track = Track::new(
            path.to_string_lossy().to_string(),
            format.clone(),
            0, // Duration will be set below
        );
        track.file_hash = Some(file_hash);

        if let Some(tagged_file) = self.read_metadata_lenient(path) {
            // Successfully read with lofty
            let properties = tagged_file.properties();
            track.duration_ms = properties.duration().as_millis() as i64;
            track.bitrate = properties.audio_bitrate().map(|b| b as i32);
            track.sample_rate = properties.sample_rate().map(|s| s as i32);

            // Extract tags if available
            if let Some(tag) = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag())
            {
                track.title = tag.title().map(|s| s.to_string());
                track.artist = tag.artist().map(|s| s.to_string());
                track.album = tag.album().map(|s| s.to_string());
                track.year = tag.year().map(|y| y as i32);
                track.genre = tag.genre().map(|s| s.to_string());
            }

            // Extract Mixed In Key & audio analysis metadata (BPM, Key, Energy, Cues)
            let mik_data = MikService::extract_analysis_data(&tagged_file, &track.id);
            if let Some(bpm) = mik_data.bpm {
                track.bpm = Some(bpm);
            }
            if let Some(ref key) = mik_data.key {
                track.key = Some(key.clone());
            }
            if let Some(energy) = mik_data.energy {
                track.energy = Some(energy);
            }
            if mik_data.is_mik {
                track.analysis_source = Some("mixed_in_key".to_string());
            }

            // Extract album artwork
            if let Some(artwork_path) = self
                .artwork_service
                .extract_and_save(&tagged_file, &track.id)
            {
                track.artwork_path = Some(artwork_path);
                track.artwork_source = Some("extracted".to_string());
            }

            // Insert into database (a re-imported file keeps its existing id)
            track.id = self.insert_track(&track)?;

            // Insert any extracted hot cues, attached to the stored track
            if !mik_data.cues.is_empty() {
                let cues: Vec<Cue> = mik_data
                    .cues
                    .into_iter()
                    .map(|mut cue| {
                        cue.track_id = track.id.clone();
                        cue
                    })
                    .collect();
                self.insert_cues(&cues)?;
            }
        } else {
            // Lofty failed completely, use symphonia fallback
            log::warn!(
                "Metadata extraction failed for {}, falling back to symphonia",
                path.display()
            );

            let (dur, sr, br) = self.read_audio_properties_symphonia(path)?;
            track.duration_ms = dur;
            track.sample_rate = sr;
            track.bitrate = br;

            // Insert into database
            track.id = self.insert_track(&track)?;
        }

        Ok(track)
    }

    /// Import tracks with duplicate detection based on content hash
    pub fn import_tracks_with_duplicate_detection(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<ImportResultWithDuplicates> {
        let mut tracks = Vec::new();
        let mut errors = Vec::new();
        let mut duplicates = Vec::new();

        for path in paths {
            match self.process_import_path(&path) {
                Ok(ImportPathResult::NewTrack(track)) => tracks.push(track),
                Ok(ImportPathResult::Duplicate(dup)) => duplicates.push(dup),
                Err(e) => {
                    let error_msg = format!("{}: {}", path.display(), e);
                    log::warn!("Failed to import {error_msg}");
                    errors.push(error_msg);
                }
            }
        }

        Ok(ImportResultWithDuplicates {
            tracks,
            failed_count: errors.len(),
            errors,
            duplicates,
        })
    }

    /// Process a single import path, checking for duplicates first
    fn process_import_path(&self, path: &PathBuf) -> Result<ImportPathResult> {
        if !path.exists() {
            return Err(CrateError::FileNotFound(path.clone()));
        }

        // Check format
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        let supported_formats = ["mp3", "wav", "aiff", "aif", "flac", "m4a", "aac"];
        if !supported_formats.contains(&format.as_str()) {
            return Err(CrateError::Import(format!("Unsupported format: {format}")));
        }

        // Compute hash first to check for duplicates
        let file_hash = compute_audio_hash(path)?;

        // Check if a track with this hash already exists
        if let Some(existing_track) = self.find_track_by_hash(&file_hash)? {
            return Ok(ImportPathResult::Duplicate(DuplicateTrack {
                new_file_path: path.to_string_lossy().to_string(),
                new_file_hash: file_hash,
                existing_track,
            }));
        }

        // No duplicate - proceed with normal import
        let track = self.import_single_track_with_hash(path, file_hash)?;
        Ok(ImportPathResult::NewTrack(track))
    }

    /// Attempts to read audio file metadata with lenient parsing options.
    /// Returns None if parsing fails completely.
    pub(crate) fn read_metadata_lenient(&self, path: &PathBuf) -> Option<TaggedFile> {
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

    /// Fallback to extract audio properties using symphonia when lofty fails.
    /// Returns (duration_ms, sample_rate, bitrate).
    pub(super) fn read_audio_properties_symphonia(
        &self,
        path: &PathBuf,
    ) -> Result<(i64, Option<i32>, Option<i32>)> {
        let file =
            File::open(path).map_err(|e| CrateError::Metadata(format!("Failed to open: {e}")))?;

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

    /// Inserts or updates (same file path) a track and returns the id of the stored row: on a
    /// re-import this is the existing track's id, not the freshly generated one.
    fn insert_track(&self, track: &Track) -> Result<String> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let hlc = dirty::next_hlc(&conn)?;
        let (library_root_id, relative_path) =
            resolution::assign_root_for_import(&conn, &track.file_path)?;

        conn.execute(
            r#"
            INSERT INTO tracks (
                id, file_path, file_hash,
                title, artist, album, year, genre, label, catalog_number,
                duration_ms, bpm, key, energy, bitrate, sample_rate, format,
                analysis_source, waveform_data,
                rating, play_count,
                date_added, date_modified, last_played,
                rekordbox_id, artwork_path, artwork_source, color,
                _hlc, library_root_id, relative_path
            ) VALUES (
                ?1, ?2, ?3,
                ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19,
                ?20, ?21,
                ?22, ?23, ?24,
                ?25, ?26, ?27, ?28,
                ?29, ?30, ?31
            )
            ON CONFLICT(file_path) DO UPDATE SET
                file_hash = COALESCE(excluded.file_hash, tracks.file_hash),
                title = excluded.title,
                artist = excluded.artist,
                album = excluded.album,
                year = excluded.year,
                genre = excluded.genre,
                bpm = COALESCE(excluded.bpm, tracks.bpm),
                key = COALESCE(excluded.key, tracks.key),
                energy = COALESCE(excluded.energy, tracks.energy),
                analysis_source = COALESCE(excluded.analysis_source, tracks.analysis_source),
                artwork_path = excluded.artwork_path,
                artwork_source = excluded.artwork_source,
                date_modified = excluded.date_modified,
                _hlc = excluded._hlc,
                library_root_id = excluded.library_root_id,
                relative_path = excluded.relative_path
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
                hlc,
                library_root_id,
                relative_path,
            ],
        )?;

        let stored_id: String = conn.query_row(
            "SELECT id FROM tracks WHERE file_path = ?1",
            [&track.file_path],
            |r| r.get(0),
        )?;
        dirty::mark_dirty(&conn, &buckets::bucket_for_track_id(&stored_id))?;
        drop(conn);

        Ok(stored_id)
    }

    fn insert_cues(&self, cues: &[Cue]) -> Result<()> {
        if cues.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        for cue in cues {
            let cue_hlc = dirty::next_hlc(&conn)?;
            conn.execute(
                r#"
                INSERT INTO cues (id, track_id, position_ms, type, loop_end_ms, hot_cue_index, name, color, _hlc)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(id) DO UPDATE SET
                    position_ms = excluded.position_ms,
                    type = excluded.type,
                    loop_end_ms = excluded.loop_end_ms,
                    hot_cue_index = excluded.hot_cue_index,
                    name = excluded.name,
                    color = excluded.color,
                    _hlc = excluded._hlc
                "#,
                rusqlite::params![
                    cue.id,
                    cue.track_id,
                    cue.position_ms,
                    cue.cue_type.to_string(),
                    cue.loop_end_ms,
                    cue.hot_cue_index,
                    cue.name,
                    cue.color,
                    cue_hlc,
                ],
            )?;
        }
        dirty::mark_dirty(&conn, buckets::CUES)?;
        Ok(())
    }

    fn extract_bpm(&self, tag: &Tag) -> Option<f64> {
        MikService::extract_bpm(tag)
    }

    fn extract_key(&self, tag: &Tag) -> Option<String> {
        MikService::extract_key(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SILENCE_FLAC: &[u8] = include_bytes!("../../../test-fixtures/silence-1s.flac");

    fn library(suffix: &str) -> (LibraryService, PathBuf) {
        let dir = std::env::temp_dir().join(format!("crate_import_{suffix}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        (LibraryService::new(Arc::new(Mutex::new(conn)), dir.clone()), dir)
    }

    #[test]
    fn test_import_stores_hash_and_reimport_keeps_identity() {
        let (lib, dir) = library("reimport");
        let path = dir.join("silence.flac");
        std::fs::write(&path, SILENCE_FLAC).unwrap();

        let first = lib.import_tracks(vec![path.clone()]).unwrap();
        assert_eq!(first.failed_count, 0, "{:?}", first.errors);
        let id = first.tracks[0].id.clone();
        let stored_hash: Option<String> = lib
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT file_hash FROM tracks WHERE id = ?1", [&id], |r| r.get(0))
            .unwrap();
        assert!(stored_hash.is_some(), "the content hash must be saved at import");

        let second = lib.import_tracks(vec![path]).unwrap();
        assert_eq!(second.failed_count, 0, "re-importing a file must not fail: {:?}", second.errors);
        assert_eq!(second.tracks[0].id, id, "a re-imported file keeps its track id");
        let count: i64 = lib.conn.lock().unwrap().query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}

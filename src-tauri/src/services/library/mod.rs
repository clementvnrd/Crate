mod artwork;
mod duplicates;
mod import;
pub mod macos_bookmark;
pub mod mik;
pub mod mik_db;
mod query;
mod relocation;
mod update;
pub mod waveform;

pub use mik::*;
pub use mik_db::*;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::error::{CrateError, Result};
use crate::models::{
    DuplicateResolution, DuplicateTrack, FileMatchResult, ImportResult, ImportResultWithDuplicates,
    Tag, Track, TrackFilter, TrackUpdate,
};
use crate::services::hash::compute_audio_hash;
use crate::services::ArtworkService;

pub struct LibraryService {
    conn: Arc<Mutex<Connection>>,
    artwork_service: ArtworkService,
}

impl LibraryService {
    pub fn new(conn: Arc<Mutex<Connection>>, app_data_dir: PathBuf) -> Self {
        Self {
            conn,
            artwork_service: ArtworkService::new(app_data_dir),
        }
    }

    pub fn prune_missing_tracks(&self) -> Result<usize> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        MikDatabaseService::prune_missing_tracks(&conn, Some(&self.artwork_service))
    }

    pub fn get_mik_database_status(&self) -> Result<MikDatabaseStatus> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        Ok(MikDatabaseService::get_status(&conn))
    }

    /// Syncs from Mixed In Key: the whole library, or only the given tracks (context menu).
    pub fn sync_from_mik_database(&self, track_ids: Option<&[String]>) -> Result<MikSyncResult> {
        let mut songs = MikDatabaseService::read_all_songs()?;
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        if let Some(ids) = track_ids.filter(|ids| !ids.is_empty()) {
            use unicode_normalization::UnicodeNormalization;
            let mut paths = std::collections::HashSet::new();
            for id in ids {
                if let Ok(path) =
                    conn.query_row("SELECT file_path FROM tracks WHERE id = ?1", [id], |r| {
                        r.get::<_, String>(0)
                    })
                {
                    paths.insert(path.nfc().collect::<String>());
                }
            }
            songs.retain(|song| {
                song.file_path
                    .as_ref()
                    .is_some_and(|p| paths.contains(&p.to_string_lossy().nfc().collect::<String>()))
            });
        }
        MikDatabaseService::apply_mik_songs(&conn, songs, Some(&self.artwork_service))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RescanResult {
    pub updated_count: usize,
    pub failed_count: usize,
}

/// Result of processing a single import path
enum ImportPathResult {
    NewTrack(Track),
    Duplicate(DuplicateTrack),
}

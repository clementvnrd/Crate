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

pub use macos_bookmark::*;
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

    pub fn sync_from_mik_database(&self) -> Result<MikSyncResult> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        MikDatabaseService::sync_all_from_mik_db(&conn, Some(&self.artwork_service))
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

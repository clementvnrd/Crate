mod artwork;
mod duplicates;
mod import;
pub mod macos_bookmark;
pub mod mik;
pub mod mik_db;
mod organise;
mod query;
mod relocation;
mod report;
mod set_planner;
mod suggest;
mod update;
pub mod waveform;

pub use mik::*;
pub use mik_db::*;
pub use organise::{OrganisationBatch, OrganisationPlan, OrganisationResult, OrganisationRule};
pub use report::{read_rekordbox_xml, DiscrepancyReport};
pub use set_planner::SetAnalysis;
pub use suggest::NextTrackSuggestion;

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

/// Cheap to clone: copies share the same database connection. A clone lets blocking work move into
/// `spawn_blocking`, which needs an owned value, without a reference to the managed service.
#[derive(Clone)]
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

    /// Removes the tracks whose audio file was deleted from disk. The disk is checked (two `stat`
    /// calls per track) between two short lock holds, not under the library lock.
    pub fn prune_missing_tracks(&self) -> Result<usize> {
        let paths = {
            let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            MikDatabaseService::track_paths(&conn)?
        };
        let ghosts: Vec<(String, String)> = paths
            .into_iter()
            .filter(|(_, path)| MikDatabaseService::is_missing_on_disk(path))
            .collect();

        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        MikDatabaseService::prune_tracks(&conn, Some(&self.artwork_service), &ghosts)
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

#[cfg(test)]
mod prune_tests {
    use super::*;

    #[test]
    fn prune_missing_tracks_removes_only_the_tracks_whose_file_is_gone() {
        let dir = std::env::temp_dir().join(format!("crate_library_prune_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        let present = dir.join("present.flac");
        std::fs::write(&present, b"not really audio").unwrap();
        let ghost = dir.join("ghost.flac");
        let unmounted = dir.join("not-mounted").join("away.flac");
        for (id, path) in [
            ("present", &present),
            ("ghost", &ghost),
            ("away", &unmounted),
        ] {
            conn.execute(
                "INSERT INTO tracks (id, file_path, title, artist, duration_ms, date_added, date_modified)
                 VALUES (?1, ?2, ?1, 'Artist', 1000, '2026-01-01', '2026-01-01')",
                rusqlite::params![id, path.to_string_lossy()],
            )
            .unwrap();
        }
        let service = LibraryService::new(Arc::new(Mutex::new(conn)), dir.clone());

        assert_eq!(service.prune_missing_tracks().unwrap(), 1);

        let left: Vec<String> = {
            let conn = service.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id FROM tracks ORDER BY id").unwrap();
            let rows = stmt.query_map([], |r| r.get(0)).unwrap();
            rows.flatten().collect()
        };
        assert_eq!(
            left,
            ["away", "present"],
            "a deleted file goes; a present file and an unmounted folder stay"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}

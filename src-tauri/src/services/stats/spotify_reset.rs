//! Resetting the Spotify-sourced part of the listening history.
//!
//! Two now-fixed bugs (double counting, and an old "repair" that turned short plays into full
//! listens) skewed part of the Spotify history recorded before they were fixed. Emptying it lets
//! the owner re-import a clean copy from Spotify's official data export, which already
//! de-duplicates against whatever the "recently played" sync brings back afterwards. The deletion
//! only ever touches rows whose `source` is `spotify`; every other source (the local library,
//! Mixed In Key, Rekordbox) is left untouched.

use std::path::Path;

use crate::error::{CrateError, Result};
use crate::models::stats::SpotifyResetResult;

use super::{HistoryExportFormat, StatsRecorderService};

impl StatsRecorderService {
    /// Number of currently recorded Spotify listens — shown in the confirmation dialog before the
    /// owner commits to the reset.
    pub fn count_spotify_listens(&self) -> Result<usize> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM listen_events WHERE source = 'spotify'",
                [],
                |row| row.get(0),
            )
            .map_err(CrateError::Database)?;
        Ok(count as usize)
    }

    /// Deletes every Spotify-sourced listen, after writing a full history backup to `backup_dest`.
    ///
    /// The backup reuses [`Self::export_listen_history`] — the only existing backup path for this
    /// table — so it covers every source, not only Spotify. That is deliberately more than what
    /// gets deleted: it is simpler, and strictly safer, than teaching the exporter a Spotify-only
    /// mode just for this. If the backup cannot be written and verified, nothing is deleted and the
    /// returned error explains why.
    pub fn reset_spotify_history(&self, backup_dest: &Path) -> Result<SpotifyResetResult> {
        let exported = self.export_listen_history(HistoryExportFormat::Json, backup_dest)?;

        // Defense in depth: re-read the file from disk before touching the database, so a backup
        // that silently came back empty or unreadable never leads to a deletion with no safety
        // copy behind it.
        let backup_len = std::fs::metadata(backup_dest)
            .map_err(|e| CrateError::Backup(format!("Backup file is not readable: {e}")))?
            .len();
        if backup_len == 0 && exported > 0 {
            return Err(CrateError::Backup(
                "Backup file came back empty while listens exist — refusing to delete".to_string(),
            ));
        }

        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let deleted_count = conn
            .execute("DELETE FROM listen_events WHERE source = 'spotify'", [])
            .map_err(CrateError::Database)?;

        Ok(SpotifyResetResult {
            deleted_count,
            backup_path: backup_dest.to_string_lossy().into_owned(),
        })
    }
}

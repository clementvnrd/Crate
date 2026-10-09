pub mod schema;

mod key;

use rusqlite::Connection;
#[cfg(feature = "desktop")]
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::error::{CrateError, Result};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

/// Check whether a database file is unencrypted by attempting to read its header.
/// An unencrypted SQLite database starts with "SQLite format 3\0".
///
/// Desktop-only: mobile databases are encrypted from creation, so there is never
/// an unencrypted database to detect or migrate.
#[cfg(feature = "desktop")]
fn is_unencrypted(db_path: &Path) -> bool {
    std::fs::read(db_path)
        .map(|bytes| bytes.starts_with(b"SQLite format 3\0"))
        .unwrap_or(false)
}

/// Migrate an existing unencrypted database to an encrypted one using `sqlcipher_export`.
///
/// Desktop-only (see [`is_unencrypted`]); the `std::fs::rename` swap never runs on mobile.
#[cfg(feature = "desktop")]
fn migrate_to_encrypted(db_path: &Path, key: &str) -> Result<()> {
    let conn = Connection::open(db_path)?;
    let encrypted_path = db_path.with_extension("db.encrypted");

    conn.execute_batch(&format!(
        "ATTACH DATABASE '{}' AS encrypted KEY '{}';
         SELECT sqlcipher_export('encrypted');
         DETACH DATABASE encrypted;",
        encrypted_path.display(),
        key
    ))?;

    drop(conn);
    std::fs::rename(&encrypted_path, db_path)?;
    Ok(())
}

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let app_data_dir = db_path.parent().ok_or_else(|| {
            CrateError::KeyStorage("database path has no parent directory".to_string())
        })?;
        // Compile-time-selected provider: file (desktop) / Keychain (iOS) / Keystore (Android).
        let key = key::provision_key(app_data_dir)?;

        // If a pre-#134 desktop install left an unencrypted database on disk, migrate it in
        // place. Mobile databases are born encrypted, so this path — and its `std::fs::rename`
        // — is compiled out entirely on mobile.
        #[cfg(feature = "desktop")]
        if db_path.exists() && is_unencrypted(&db_path) {
            log::info!("Migrating unencrypted database to encrypted format");
            migrate_to_encrypted(&db_path, &key)?;
        }

        let conn = Connection::open(&db_path)?;

        // Apply encryption key
        conn.pragma_update(None, "key", &key)?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        // Run migrations
        db.migrate()?;

        Ok(db)
    }

    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        run_migrations(&conn)
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
        }
    }
}

/// Apply any pending schema migrations to `conn`, version-gated and atomic.
///
/// Each migration's DDL and its `schema_version` bump commit together in one
/// transaction, so an interrupted run (e.g. the process is killed mid-migration)
/// rolls back cleanly and is retried from scratch on the next launch — never
/// leaving a half-applied schema. Migrations run in order, each exactly once.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)",
        [],
    )?;

    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // A database migrated by a newer build (e.g. after rolling back to an older release) must not
    // be opened by this one: its queries would run against tables it does not know.
    let migrations = schema::get_migrations();
    if current_version > migrations.len() as i32 {
        return Err(CrateError::InvalidOperation(format!(
            "This library was last opened by a newer version of Crate (database schema {current_version}, \
             this version knows up to {}). Install the newer version again, or restore the backup taken \
             before the update.",
            migrations.len()
        )));
    }

    for (idx, sql) in migrations.iter().enumerate() {
        let version = idx as i32 + 1;
        if version > current_version {
            log::info!("Running migration {version}");
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                [version],
            )?;
            tx.commit()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::OptionalExtension;

    fn open_mem() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn
    }

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |_| Ok(()),
        )
        .optional()
        .unwrap()
        .is_some()
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        cols.iter().any(|c| c == column)
    }

    fn version(conn: &Connection) -> i32 {
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    /// Every sync table + rooting/`_hlc` column that migrations 3–4 must create.
    fn assert_sync_schema(conn: &Connection) {
        for t in [
            "library_roots",
            "sync_root_mappings",
            "sync_tombstones",
            "sync_dirty_buckets",
            "sync_state",
        ] {
            assert!(table_exists(conn, t), "expected table `{t}` to exist");
        }
        for (tbl, col) in [
            ("tracks", "_hlc"),
            ("tracks", "library_root_id"),
            ("tracks", "relative_path"),
            ("playlists", "_hlc"),
            ("playlist_tracks", "_hlc"),
            ("cues", "_hlc"),
            ("tag_categories", "_hlc"),
            ("tags", "_hlc"),
            ("track_tags", "_hlc"),
            ("discovery_releases", "_hlc"),
            ("discovery_tracks", "_hlc"),
            ("discovery_release_tags", "_hlc"),
            ("playlist_discovery_releases", "_hlc"),
        ] {
            assert!(
                column_exists(conn, tbl, col),
                "expected column `{tbl}.{col}` to exist"
            );
        }
    }

    #[test]
    fn fresh_db_migrates_to_latest_and_reruns_cleanly() {
        let conn = open_mem();
        run_migrations(&conn).unwrap();

        assert_sync_schema(&conn);
        let latest = schema::get_migrations().len() as i32;
        assert_eq!(version(&conn), latest);

        // Re-running must be a version-gated no-op, never an error.
        run_migrations(&conn).unwrap();
        assert_eq!(version(&conn), latest);
    }

    #[test]
    fn existing_v2_database_upgrades_cleanly() {
        let conn = open_mem();

        // Simulate a shipped (schema v2) database: apply only migrations 1 & 2.
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)",
            [],
        )
        .unwrap();
        for (idx, sql) in schema::get_migrations().iter().take(2).enumerate() {
            conn.execute_batch(sql).unwrap();
            conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                [(idx as i32) + 1],
            )
            .unwrap();
        }
        assert_eq!(version(&conn), 2);
        assert!(!table_exists(&conn, "sync_state"));

        // Upgrading applies only the new migrations (3 & 4), atomically.
        run_migrations(&conn).unwrap();
        assert_sync_schema(&conn);
        assert_eq!(version(&conn), schema::get_migrations().len() as i32);
    }

    #[test]
    fn beat_grid_migration_keeps_existing_tracks_and_leaves_their_grid_empty() {
        // Migration 19 adds the beat grid columns (CRA-177). Build a populated library at the
        // schema just before it, then upgrade.
        const BEFORE_BEAT_GRID: usize = 18;
        let conn = open_mem();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)",
            [],
        )
        .unwrap();
        for (idx, sql) in schema::get_migrations()
            .iter()
            .take(BEFORE_BEAT_GRID)
            .enumerate()
        {
            conn.execute_batch(sql).unwrap();
            conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                [(idx as i32) + 1],
            )
            .unwrap();
        }
        assert!(!column_exists(&conn, "tracks", "beatgrid_bpm"));
        for i in 0..3 {
            conn.execute(
                "INSERT INTO tracks (id, file_path, duration_ms, bpm, key, energy, analysis_source, \
                 date_added, date_modified) \
                 VALUES (?1, ?2, 300000, 124.0, '8A', 6, 'mixed_in_key', '2020-01-01', '2020-01-01')",
                rusqlite::params![format!("t{i}"), format!("/music/{i}.mp3")],
            )
            .unwrap();
        }

        run_migrations(&conn).unwrap();

        for col in [
            "beatgrid_first_beat_ms",
            "beatgrid_bpm",
            "beatgrid_tempo_changes",
        ] {
            assert!(column_exists(&conn, "tracks", col), "missing {col}");
        }
        let (count, with_grid, bpm_sum): (i64, i64, f64) = conn
            .query_row(
                "SELECT COUNT(*), COUNT(beatgrid_bpm), SUM(bpm) FROM tracks \
                 WHERE key = '8A' AND energy = 6 AND analysis_source = 'mixed_in_key'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((count, with_grid, bpm_sum), (3, 0, 372.0));
    }

    #[test]
    fn database_from_a_newer_build_is_refused_untouched() {
        let conn = open_mem();
        run_migrations(&conn).unwrap();
        let latest = schema::get_migrations().len() as i32;

        // A newer release applied one more migration, then the user went back to this build.
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [latest + 1],
        )
        .unwrap();

        let err = run_migrations(&conn).unwrap_err().to_string();
        assert!(err.contains("newer version of Crate"), "{err}");
        assert_eq!(version(&conn), latest + 1);
    }
}

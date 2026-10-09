//! Export of the listening history to a CSV or JSON file: the whole of it, or the listens of
//! one statistics period (the Pulse's selected period, CRA-184).
//!
//! The history belongs to the user: this gives an independent copy, readable outside Crate
//! (a spreadsheet, a script), that does not depend on the database or its encryption key.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::StatsRecorderService;
use crate::error::{CrateError, Result};

/// Rows read per database lock. The lock is released between chunks so a large export never
/// blocks the rest of the app while it writes to disk.
const CHUNK_ROWS: usize = 2_000;

/// Column order of the CSV file (chronological first). JSON objects carry the same keys, in
/// alphabetical order, so JSON readers look them up by name.
///
/// New columns are only ever appended at the end, so a CSV file written by an older version keeps
/// the same leading columns and any reader that ignores unknown trailing columns still works.
/// `track_id` and `artwork_url` matter beyond reading: this export is the only backup of the
/// history (the reset in `spotify_reset.rs` writes it before deleting), and the stored cover URL
/// cannot be recovered once its row is gone.
const COLUMNS: [&str; 16] = [
    "played_at",
    "source",
    "title",
    "artist",
    "album",
    "duration_ms",
    "played_ms",
    "bpm",
    "key",
    "energy",
    "format",
    "track_id",
    "session_id",
    "id",
    "metadata_json",
    "artwork_url",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HistoryExportFormat {
    Csv,
    Json,
}

impl HistoryExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            HistoryExportFormat::Csv => "csv",
            HistoryExportFormat::Json => "json",
        }
    }
}

/// One history row, as stored. Numbers stay numbers, missing values stay missing.
struct Row {
    played_at: String,
    source: String,
    title: String,
    artist: String,
    album: Option<String>,
    duration_ms: i64,
    played_ms: i64,
    bpm: Option<f64>,
    key: Option<String>,
    energy: Option<i64>,
    format: Option<String>,
    track_id: Option<String>,
    session_id: Option<String>,
    id: String,
    metadata_json: Option<String>,
    artwork_url: Option<String>,
}

impl Row {
    fn csv_fields(&self) -> [String; 16] {
        let text = |v: &Option<String>| v.clone().unwrap_or_default();
        [
            self.played_at.clone(),
            self.source.clone(),
            self.title.clone(),
            self.artist.clone(),
            text(&self.album),
            self.duration_ms.to_string(),
            self.played_ms.to_string(),
            self.bpm.map(|b| b.to_string()).unwrap_or_default(),
            text(&self.key),
            self.energy.map(|e| e.to_string()).unwrap_or_default(),
            text(&self.format),
            text(&self.track_id),
            text(&self.session_id),
            self.id.clone(),
            text(&self.metadata_json),
            text(&self.artwork_url),
        ]
    }

    fn to_json(&self) -> Value {
        // The metadata is stored as a JSON string: expose it as real JSON when it parses.
        let metadata = self
            .metadata_json
            .as_deref()
            .map(|raw| serde_json::from_str::<Value>(raw).unwrap_or_else(|_| json!(raw)));
        json!({
            "played_at": self.played_at,
            "source": self.source,
            "title": self.title,
            "artist": self.artist,
            "album": self.album,
            "duration_ms": self.duration_ms,
            "played_ms": self.played_ms,
            "bpm": self.bpm,
            "key": self.key,
            "energy": self.energy,
            "format": self.format,
            "track_id": self.track_id,
            "session_id": self.session_id,
            "id": self.id,
            "metadata_json": metadata,
            "artwork_url": self.artwork_url,
        })
    }
}

/// RFC 4180: a field containing a comma, a quote or a line break is quoted, quotes are doubled.
fn csv_escape(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn csv_line(fields: &[String]) -> String {
    let mut line = fields
        .iter()
        .map(|f| csv_escape(f))
        .collect::<Vec<_>>()
        .join(",");
    line.push('\n');
    line
}

/// The destination must carry the extension of the chosen format and sit in an existing folder.
pub fn validate_destination(format: HistoryExportFormat, dest: &Path) -> Result<()> {
    let extension_ok = dest
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(format.extension()));
    if !extension_ok {
        return Err(CrateError::InvalidOperation(format!(
            "the export file must end in .{}",
            format.extension()
        )));
    }
    match dest.parent() {
        Some(parent) if parent.as_os_str().is_empty() || parent.is_dir() => Ok(()),
        _ => Err(CrateError::InvalidOperation(
            "the export folder does not exist".to_string(),
        )),
    }
}

impl StatsRecorderService {
    /// Writes every listen, oldest first, to `dest` and returns how many were written.
    pub fn export_listen_history(&self, format: HistoryExportFormat, dest: &Path) -> Result<usize> {
        self.export_listen_history_in_chunks(format, dest, None, CHUNK_ROWS)
    }

    /// Writes the listens of `time_range` (the grammar of [`super::range::StatsRange`]), oldest
    /// first, to `dest` and returns how many were written. An unknown or malformed range is an
    /// error before any file is touched; `all` exports the whole history.
    pub fn export_listen_history_in_range(
        &self,
        format: HistoryExportFormat,
        dest: &Path,
        time_range: &str,
    ) -> Result<usize> {
        // Computed once, so a rolling period does not move between two chunks of the same export.
        let condition = Self::time_range_condition(time_range, "played_at")?;
        self.export_listen_history_in_chunks(format, dest, condition.as_deref(), CHUNK_ROWS)
    }

    /// The same, with an explicit chunk size (tests use tiny chunks to cross the boundaries).
    ///
    /// The file is written next to the destination and renamed at the end: a failure or a crash
    /// never leaves a half-written export, and never damages an export already in place.
    fn export_listen_history_in_chunks(
        &self,
        format: HistoryExportFormat,
        dest: &Path,
        condition: Option<&str>,
        chunk_rows: usize,
    ) -> Result<usize> {
        validate_destination(format, dest)?;
        let partial = partial_path(dest);
        match self.write_history(format, &partial, condition, chunk_rows) {
            Ok(count) => {
                fs::rename(&partial, dest)?;
                Ok(count)
            }
            Err(e) => {
                let _ = fs::remove_file(&partial);
                Err(e)
            }
        }
    }

    fn write_history(
        &self,
        format: HistoryExportFormat,
        path: &Path,
        condition: Option<&str>,
        chunk_rows: usize,
    ) -> Result<usize> {
        let mut out = BufWriter::new(File::create(path)?);
        if format == HistoryExportFormat::Csv {
            let header: Vec<String> = COLUMNS.iter().map(|c| c.to_string()).collect();
            out.write_all(csv_line(&header).as_bytes())?;
        } else {
            out.write_all(b"[")?;
        }

        let mut written = 0usize;
        // Keyset pagination on (played_at, id): stable even when several listens share a time.
        let mut after: Option<(String, String)> = None;
        loop {
            let rows = self.read_chunk(after.as_ref(), condition, chunk_rows)?;
            let Some(last) = rows.last() else { break };
            after = Some((last.played_at.clone(), last.id.clone()));
            let full = rows.len() == chunk_rows.max(1);

            for row in &rows {
                match format {
                    HistoryExportFormat::Csv => {
                        out.write_all(csv_line(&row.csv_fields()).as_bytes())?
                    }
                    HistoryExportFormat::Json => {
                        out.write_all(if written == 0 { b"\n" } else { b",\n" })?;
                        serde_json::to_writer(&mut out, &row.to_json())
                            .map_err(|e| CrateError::Export(format!("JSON error: {e}")))?;
                    }
                }
                written += 1;
            }
            if !full {
                break;
            }
        }

        if format == HistoryExportFormat::Json {
            out.write_all(if written == 0 { b"]\n" } else { b"\n]\n" })?;
        }
        out.flush()?;
        Ok(written)
    }

    /// The next `limit` rows after `after` that match `condition` (an SQL condition built by
    /// [`super::range::StatsRange`], never user text), oldest first. Holds the lock for this query
    /// only.
    fn read_chunk(
        &self,
        after: Option<&(String, String)>,
        condition: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Row>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let (after_at, after_id) = after
            .map(|(a, i)| (a.as_str(), i.as_str()))
            .unwrap_or(("", ""));
        let period = condition
            .map(|cond| format!("AND ({cond})"))
            .unwrap_or_default();
        let mut stmt = conn
            .prepare(&format!(
                r#"
                SELECT played_at, source, title, artist, album, duration_ms, played_ms, bpm, key,
                       energy, format, track_id, session_id, id, metadata_json, artwork_url
                FROM listen_events
                WHERE (played_at, id) > (?1, ?2) {period}
                ORDER BY played_at ASC, id ASC
                LIMIT ?3
                "#
            ))
            .map_err(CrateError::Database)?;
        let rows = stmt
            .query_map(
                rusqlite::params![after_at, after_id, limit.max(1) as i64],
                |r| {
                    Ok(Row {
                        played_at: r.get(0)?,
                        source: r.get(1)?,
                        title: r.get(2)?,
                        artist: r.get(3)?,
                        album: r.get(4)?,
                        duration_ms: r.get(5)?,
                        played_ms: r.get(6)?,
                        bpm: r.get(7)?,
                        key: r.get(8)?,
                        energy: r.get(9)?,
                        format: r.get(10)?,
                        track_id: r.get(11)?,
                        session_id: r.get(12)?,
                        id: r.get(13)?,
                        metadata_json: r.get(14)?,
                        artwork_url: r.get(15)?,
                    })
                },
            )
            .map_err(CrateError::Database)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CrateError::Database)?;
        Ok(rows)
    }
}

fn partial_path(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    dest.with_file_name(name)
}

#[cfg(test)]
pub(super) fn export_in_chunks_for_test(
    service: &StatsRecorderService,
    format: HistoryExportFormat,
    dest: &Path,
    chunk_rows: usize,
) -> Result<usize> {
    service.export_listen_history_in_chunks(format, dest, None, chunk_rows)
}

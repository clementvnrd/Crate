use chrono::Utc;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::range::StatsRange;
use crate::error::{CrateError, Result};
use crate::models::stats::{
    BpmBucketItem, HarmonicStatsItem, HeatmapCell, ListenEvent, StatsSummary, TopArtistItem,
    TopTrackItem,
};

/// Per-artist totals: (played ms, streams, per-track (streams, played ms), artwork URL).
type ArtistAccumulator = (u64, usize, HashMap<String, (usize, u64)>, Option<String>);

pub struct StatsRecorderService {
    pub(super) conn: Arc<Mutex<Connection>>,
}

impl StatsRecorderService {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Converts a time_range string into a SQL WHERE clause fragment (`None` = no limit).
    ///
    /// The string is parsed into a [`StatsRange`] (grammar in `range.rs`: the presets `today`,
    /// `7d`, `30d`, `3m`, `6m`, `year`, `all`, a calendar year `year:<YYYY>`, a custom local-day
    /// window `custom:<YYYY-MM-DD>,<YYYY-MM-DD>` and the recap's exact UTC window
    /// `between:<start>,<end>`). An unknown, malformed or reversed range is an error: it is never
    /// silently widened to "all time".
    pub(super) fn time_range_condition(
        time_range: &str,
        field_name: &str,
    ) -> Result<Option<String>> {
        time_range
            .parse::<StatsRange>()?
            .sql_condition_at(field_name, &chrono::Local::now())
    }

    /// Records a listen event into the database with validation and anti-duplicate deduplication.
    ///
    /// Rules:
    /// 1. `title` and `artist` must not be empty.
    /// 2. `played_ms` must be >= 1_000 (1 second threshold).
    /// 3. Anti-duplicate: if an event for the same title, artist, and source was recorded
    ///    within 15 seconds of `played_at`, it is deduplicated and not inserted.
    pub fn record_listen_event(&self, event: &ListenEvent) -> Result<bool> {
        let title = event.title.trim();
        let artist = event.artist.trim();

        if title.is_empty() || artist.is_empty() {
            return Err(CrateError::InvalidOperation(
                "ListenEvent must have a non-empty title and artist".to_string(),
            ));
        }

        // 1-second minimum listening threshold
        if event.played_ms < 1_000 {
            log::debug!(
                "Listen event skipped: played_ms ({} ms) below 1s threshold",
                event.played_ms
            );
            return Ok(false);
        }

        let event_id = if event.id.trim().is_empty() {
            Uuid::new_v4().to_string()
        } else {
            event.id.clone()
        };

        let played_at = if event.played_at.trim().is_empty() {
            Utc::now().to_rfc3339()
        } else {
            event.played_at.clone()
        };

        let duration_ms = if event.duration_ms == 0 {
            event.played_ms
        } else {
            event.duration_ms
        };

        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        Self::insert_listen_event(
            &conn,
            event,
            title,
            artist,
            event_id,
            played_at,
            duration_ms,
        )
    }

    /// Records many events in a single transaction (history imports). Returns, for each event,
    /// whether it was inserted (false: invalid, below threshold or duplicate).
    pub fn record_listen_events(&self, events: &[ListenEvent]) -> Result<Vec<bool>> {
        let mut conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let tx = conn.transaction().map_err(CrateError::Database)?;
        let mut outcomes = Vec::with_capacity(events.len());
        for event in events {
            let title = event.title.trim();
            let artist = event.artist.trim();
            if title.is_empty() || artist.is_empty() || event.played_ms < 1_000 {
                outcomes.push(false);
                continue;
            }
            let event_id = if event.id.trim().is_empty() {
                Uuid::new_v4().to_string()
            } else {
                event.id.clone()
            };
            let played_at = if event.played_at.trim().is_empty() {
                Utc::now().to_rfc3339()
            } else {
                event.played_at.clone()
            };
            let duration_ms = if event.duration_ms == 0 {
                event.played_ms
            } else {
                event.duration_ms
            };
            outcomes.push(Self::insert_listen_event(
                &tx,
                event,
                title,
                artist,
                event_id,
                played_at,
                duration_ms,
            )?);
        }
        tx.commit().map_err(CrateError::Database)?;
        Ok(outcomes)
    }

    fn insert_listen_event(
        conn: &Connection,
        event: &ListenEvent,
        title: &str,
        artist: &str,
        event_id: String,
        played_at: String,
        duration_ms: u64,
    ) -> Result<bool> {
        // Anti-duplicate check: same title, artist, source within 15 seconds
        let duplicate_count: i64 = conn
            .query_row(
                r#"
                SELECT COUNT(*) FROM listen_events
                WHERE title = ?1 AND artist = ?2 AND source = ?3
                  AND ABS(strftime('%s', played_at) - strftime('%s', ?4)) < 15
                "#,
                rusqlite::params![title, artist, event.source, played_at],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if duplicate_count > 0 {
            log::debug!(
                "Listen event deduplicated: duplicate found for '{}' by '{}' on source '{}'",
                title,
                artist,
                event.source
            );
            return Ok(false);
        }

        conn.execute(
            r#"
            INSERT INTO listen_events (
                id, source, track_id, title, artist, album, duration_ms, played_ms,
                bpm, key, energy, format, artwork_url, played_at, session_id, metadata_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            "#,
            rusqlite::params![
                event_id,
                event.source,
                event.track_id,
                title,
                artist,
                event.album,
                duration_ms as i64,
                event.played_ms as i64,
                event.bpm,
                event.key,
                event.energy,
                event.format,
                event.artwork_url,
                played_at,
                event.session_id,
                event.metadata_json,
            ],
        )
        .map_err(CrateError::Database)?;

        Ok(true)
    }

    /// Updates the listened time of an already recorded listen (end of playback).
    pub fn update_listen_played_ms(&self, event_id: &str, played_ms: u64) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        conn.execute(
            "UPDATE listen_events SET played_ms = MAX(played_ms, ?1) WHERE id = ?2",
            rusqlite::params![played_ms as i64, event_id],
        )
        .map_err(CrateError::Database)?;
        Ok(())
    }

    /// Computes high-level listening summary statistics for a given time range.
    pub fn get_stats_summary(&self, time_range: &str) -> Result<StatsSummary> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let time_cond = Self::time_range_condition(time_range, "played_at")?;
        let where_clause = match &time_cond {
            Some(cond) => format!("WHERE {cond}"),
            None => String::new(),
        };

        // Total minutes (from >= 1s) and total plays (streams >= 30s) for the selected range
        let query_total = format!(
            "SELECT COALESCE(SUM(played_ms), 0) / 60000, COALESCE(SUM(CASE WHEN played_ms >= 30000 THEN 1 ELSE 0 END), 0) FROM listen_events {where_clause}"
        );
        let (total_minutes, total_plays): (i64, i64) = conn
            .query_row(&query_total, [], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(CrateError::Database)?;

        // Fixed range metrics (today, 7d, 30d) - accumulating all listened minutes from 1s
        let today_minutes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(played_ms), 0) / 60000 FROM listen_events WHERE datetime(played_at, 'localtime') >= datetime('now', 'localtime', 'start of day')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let week_minutes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(played_ms), 0) / 60000 FROM listen_events WHERE datetime(played_at) >= datetime('now', '-7 days')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let month_minutes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(played_ms), 0) / 60000 FROM listen_events WHERE datetime(played_at) >= datetime('now', '-30 days')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Breakdown by source for the selected range
        let query_sources = format!(
            "SELECT source, COALESCE(SUM(played_ms), 0) / 60000 FROM listen_events {where_clause} GROUP BY source"
        );
        let mut source_stmt = conn.prepare(&query_sources).map_err(CrateError::Database)?;
        let source_rows = source_stmt
            .query_map([], |row| {
                let src: String = row.get(0)?;
                let mins: i64 = row.get(1)?;
                Ok((src, mins.max(0) as u64))
            })
            .map_err(CrateError::Database)?;

        let mut source_breakdown = HashMap::new();
        for item in source_rows {
            let (src, mins) = item.map_err(CrateError::Database)?;
            source_breakdown.insert(src, mins);
        }

        Ok(StatsSummary {
            total_minutes: total_minutes.max(0) as u64,
            today_minutes: today_minutes.max(0) as u64,
            week_minutes: week_minutes.max(0) as u64,
            month_minutes: month_minutes.max(0) as u64,
            total_plays: total_plays.max(0) as usize,
            source_breakdown,
        })
    }

    /// Retrieves top played tracks with full metadata and sources.
    /// Only tracks with at least 1 stream (played_ms >= 30s) are returned.
    pub fn get_top_tracks(&self, time_range: &str, limit: usize) -> Result<Vec<TopTrackItem>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let limit_val = limit.max(1) as i64;

        let time_cond = Self::time_range_condition(time_range, "played_at")?;
        let where_clause = match &time_cond {
            Some(cond) => format!("WHERE {cond}"),
            None => String::new(),
        };

        let sql = format!(
            r#"
            SELECT
                title,
                artist,
                MAX(album) as album,
                MAX(artwork_url) as artwork_url,
                SUM(CASE WHEN played_ms >= 30000 THEN 1 ELSE 0 END) as plays,
                COALESCE(SUM(played_ms), 0) / 60000 as total_minutes,
                AVG(bpm) as bpm,
                MAX(key) as key,
                MAX(energy) as energy,
                GROUP_CONCAT(DISTINCT source) as sources_str
            FROM listen_events
            {where_clause}
            GROUP BY title, artist
            HAVING SUM(CASE WHEN played_ms >= 30000 THEN 1 ELSE 0 END) >= 1
            ORDER BY plays DESC, total_minutes DESC
            LIMIT ?1
            "#
        );

        let mut stmt = conn.prepare(&sql).map_err(CrateError::Database)?;
        let rows = stmt
            .query_map(rusqlite::params![limit_val], |row| {
                let title: String = row.get(0)?;
                let artist: String = row.get(1)?;
                let album: Option<String> = row.get(2)?;
                let artwork_url: Option<String> = row.get(3)?;
                let plays: i64 = row.get(4)?;
                let total_minutes: i64 = row.get(5)?;
                let bpm: Option<f64> = row.get(6)?;
                let key: Option<String> = row.get(7)?;
                let energy: Option<i32> = row.get(8)?;
                let sources_str: Option<String> = row.get(9)?;

                let sources: Vec<String> = sources_str
                    .map(|s| {
                        s.split(',')
                            .map(|part| part.trim().to_string())
                            .filter(|p| !p.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();

                Ok(TopTrackItem {
                    title,
                    artist,
                    album,
                    artwork_url,
                    plays: plays.max(0) as usize,
                    total_minutes: total_minutes.max(0) as u64,
                    bpm,
                    key,
                    energy,
                    sources,
                })
            })
            .map_err(CrateError::Database)?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item.map_err(CrateError::Database)?);
        }

        Ok(list)
    }

    /// Intelligently extracts individual artists from a track's artist string and title featuring tags.
    pub fn split_artists(raw_artist: &str, raw_title: &str) -> Vec<String> {
        let mut artists = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut candidate_strings = vec![raw_artist.to_string()];

        // Also extract featured artists from title: e.g. "(feat. Delilah Montagu)", "[ft. Sia]", "feat. Beacon"
        let title_lower = raw_title.to_lowercase();
        for pattern in &[
            "(feat. ",
            "(feat ",
            "(ft. ",
            "(ft ",
            "[feat. ",
            "[feat ",
            "[ft. ",
            "[ft ",
            " feat. ",
            " ft. ",
            " featuring ",
            " feat ",
            " ft ",
        ] {
            if let Some(idx) = title_lower.find(pattern) {
                let start = idx + pattern.len();
                let remainder = &raw_title[start..];
                let end_idx = remainder
                    .find(')')
                    .or_else(|| remainder.find(']'))
                    .unwrap_or(remainder.len());
                let feat_part = remainder[..end_idx].trim();
                if !feat_part.is_empty() {
                    candidate_strings.push(feat_part.to_string());
                }
            }
        }

        for raw in candidate_strings {
            // Normalize common collaboration tokens to a uniform comma delimiter
            let mut normalized = raw;
            for delim in &[
                " featuring ",
                " Featuring ",
                " FEATURING ",
                " feat. ",
                " Feat. ",
                " FEAT. ",
                " feat ",
                " Feat ",
                " FEAT ",
                " ft. ",
                " Ft. ",
                " FT. ",
                " ft ",
                " Ft ",
                " FT ",
                " with ",
                " With ",
                " WITH ",
                " w/ ",
                " W/ ",
                " vs. ",
                " Vs. ",
                " VS. ",
                " vs ",
                " Vs ",
                " VS ",
                " x ",
                " X ",
                " & ",
                " ; ",
                " / ",
            ] {
                normalized = normalized.replace(delim, ", ");
            }

            for part in normalized.split(',') {
                let mut cleaned = part.trim();
                // Remove wrapping quotes, parens, brackets, commas, semicolons
                cleaned = cleaned
                    .trim_matches(|c: char| {
                        c == '"'
                            || c == '\''
                            || c == '('
                            || c == ')'
                            || c == '['
                            || c == ']'
                            || c == '{'
                            || c == '}'
                            || c == ','
                            || c == ';'
                    })
                    .trim();

                if cleaned.is_empty() {
                    continue;
                }

                let lower = cleaned.to_lowercase();
                // Ignore generic non-artist keywords
                if lower == "feat"
                    || lower == "ft"
                    || lower == "featuring"
                    || lower == "various artists"
                    || lower == "unknown artist"
                    || lower == "original mix"
                    || lower == "extended mix"
                    || lower == "remix"
                    || lower == "radio edit"
                    || lower == "dub mix"
                    || lower == "vip"
                {
                    continue;
                }

                if seen.insert(lower) {
                    artists.push(cleaned.to_string());
                }
            }
        }

        if artists.is_empty() && !raw_artist.trim().is_empty() {
            artists.push(raw_artist.trim().to_string());
        }

        artists
    }

    /// Retrieves top played artists, calculating their top track and play counts.
    /// Intelligently separates multi-artist collaborations and feature credits (e.g. "Calvin Harris, Rihanna" and "T.I., Rihanna"
    /// both credit Rihanna individually and merge her cumulative plays).
    /// Only artists with at least 1 stream (played_ms >= 30s) are returned.
    pub fn get_top_artists(&self, time_range: &str, limit: usize) -> Result<Vec<TopArtistItem>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let limit_val = limit.max(1);

        let time_cond = Self::time_range_condition(time_range, "played_at")?;
        let where_clause = match &time_cond {
            Some(cond) => format!("WHERE {cond}"),
            None => String::new(),
        };

        let sql = format!(
            r#"
            SELECT title, artist, duration_ms, played_ms, artwork_url
            FROM listen_events
            {where_clause}
            "#
        );

        let mut stmt = conn.prepare(&sql).map_err(CrateError::Database)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(CrateError::Database)?;

        // Map: ArtistName -> (total_played_ms, streams_count, Map<TrackTitle, (stream_count, played_ms)>, Option<ArtworkUrl>)
        let mut artist_stats: HashMap<String, ArtistAccumulator> = HashMap::new();

        for item in rows {
            let (title, raw_artist, _duration_ms, played_ms, artwork_url) =
                item.map_err(CrateError::Database)?;
            let effective_played_ms = (played_ms.max(0)) as u64;
            let is_stream = effective_played_ms >= 30_000;

            let artists = Self::split_artists(&raw_artist, &title);
            for artist_name in artists {
                let entry = artist_stats
                    .entry(artist_name)
                    .or_insert_with(|| (0, 0, HashMap::new(), None));
                entry.0 += effective_played_ms;
                if is_stream {
                    entry.1 += 1;
                }
                let track_entry = entry.2.entry(title.clone()).or_insert((0, 0));
                if is_stream {
                    track_entry.0 += 1;
                }
                track_entry.1 += effective_played_ms;

                if entry.3.is_none() && artwork_url.is_some() {
                    entry.3 = artwork_url.clone();
                }
            }
        }

        // Convert to TopArtistItem list, filtering by at least 1 stream (>= 30s)
        let mut top_artists: Vec<TopArtistItem> = artist_stats
            .into_iter()
            .filter(|(_, (_, streams, _, _))| *streams >= 1)
            .map(
                |(artist, (total_played_ms, streams, tracks_map, artwork_url))| {
                    // Find top track for this artist (most streams, then most played_ms)
                    let top_track = tracks_map
                        .into_iter()
                        .max_by(|a, b| match a.1 .0.cmp(&b.1 .0) {
                            std::cmp::Ordering::Equal => a.1 .1.cmp(&b.1 .1),
                            other => other,
                        })
                        .map(|(title, _)| title);

                    TopArtistItem {
                        artist,
                        plays: streams,
                        total_minutes: total_played_ms / 60_000,
                        top_track,
                        artwork_url,
                    }
                },
            )
            .collect();

        // Sort by plays DESC, total_minutes DESC
        top_artists.sort_by(|a, b| match b.plays.cmp(&a.plays) {
            std::cmp::Ordering::Equal => b.total_minutes.cmp(&a.total_minutes),
            other => other,
        });

        top_artists.truncate(limit_val);
        Ok(top_artists)
    }

    /// Computes harmonic key distribution and percentage of keyed listens.
    pub fn get_harmonic_stats(&self, time_range: &str) -> Result<Vec<HarmonicStatsItem>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let time_cond = Self::time_range_condition(time_range, "played_at")?;
        let base_where = "WHERE key IS NOT NULL AND key != ''";
        let where_clause = match &time_cond {
            Some(cond) => format!("{base_where} AND {cond}"),
            None => base_where.to_string(),
        };

        // Total streams with known key
        let total_keyed_plays: i64 = conn
            .query_row(
                &format!("SELECT COALESCE(SUM(CASE WHEN played_ms >= 30000 THEN 1 ELSE 0 END), 0) FROM listen_events {where_clause}"),
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let sql = format!(
            r#"
            SELECT
                key,
                SUM(CASE WHEN played_ms >= 30000 THEN 1 ELSE 0 END) as plays,
                COALESCE(SUM(played_ms), 0) / 60000 as total_minutes
            FROM listen_events
            {where_clause}
            GROUP BY key
            HAVING SUM(CASE WHEN played_ms >= 30000 THEN 1 ELSE 0 END) >= 1
            ORDER BY plays DESC, total_minutes DESC
            "#
        );

        let mut stmt = conn.prepare(&sql).map_err(CrateError::Database)?;
        let rows = stmt
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let plays: i64 = row.get(1)?;
                let total_minutes: i64 = row.get(2)?;

                let percentage = if total_keyed_plays > 0 {
                    (plays as f64 / total_keyed_plays as f64) * 100.0
                } else {
                    0.0
                };

                Ok(HarmonicStatsItem {
                    key,
                    plays: plays.max(0) as usize,
                    total_minutes: total_minutes.max(0) as u64,
                    percentage: (percentage * 100.0).round() / 100.0,
                })
            })
            .map_err(CrateError::Database)?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item.map_err(CrateError::Database)?);
        }

        Ok(list)
    }

    /// Computes BPM distribution across standard DJ tempo ranges.
    pub fn get_bpm_stats(&self, time_range: &str) -> Result<Vec<BpmBucketItem>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let time_cond = Self::time_range_condition(time_range, "played_at")?;
        let base_where = "WHERE bpm IS NOT NULL AND bpm > 0";
        let where_clause = match &time_cond {
            Some(cond) => format!("{base_where} AND {cond}"),
            None => base_where.to_string(),
        };

        let buckets_def = [
            ("< 100", 0.0, 100.0),
            ("100-115", 100.0, 115.0),
            ("115-120", 115.0, 120.0),
            ("120-125", 120.0, 125.0),
            ("125-130", 125.0, 130.0),
            ("130-135", 130.0, 135.0),
            ("135-140", 135.0, 140.0),
            ("140-150", 140.0, 150.0),
            ("150+", 150.0, 9999.0),
        ];

        let sql = format!(
            r#"
            SELECT
                bpm,
                played_ms
            FROM listen_events
            {where_clause}
            "#
        );

        let mut stmt = conn.prepare(&sql).map_err(CrateError::Database)?;
        let rows = stmt
            .query_map([], |row| {
                let bpm: f64 = row.get(0)?;
                let played_ms: i64 = row.get(1)?;
                Ok((bpm, played_ms.max(0) as u64))
            })
            .map_err(CrateError::Database)?;

        let mut bucket_counts: HashMap<&'static str, usize> = HashMap::new();
        let mut bucket_ms: HashMap<&'static str, u64> = HashMap::new();

        for (name, _, _) in &buckets_def {
            bucket_counts.insert(name, 0);
            bucket_ms.insert(name, 0);
        }

        for item in rows {
            let (bpm, ms) = item.map_err(CrateError::Database)?;
            for (name, min_bpm, max_bpm) in &buckets_def {
                if bpm >= *min_bpm && bpm < *max_bpm {
                    if ms >= 30_000 {
                        *bucket_counts.get_mut(name).unwrap() += 1;
                    }
                    *bucket_ms.get_mut(name).unwrap() += ms;
                    break;
                }
            }
        }

        let mut results = Vec::new();
        for (name, _, _) in &buckets_def {
            let count = *bucket_counts.get(name).unwrap_or(&0);
            let total_minutes = *bucket_ms.get(name).unwrap_or(&0) / 60000;
            results.push(BpmBucketItem {
                bpm_range: name.to_string(),
                count,
                total_minutes,
            });
        }

        Ok(results)
    }

    /// Generates the complete 7-day x 24-hour listening activity heatmap matrix.
    pub fn get_listening_heatmap(&self, time_range: &str) -> Result<Vec<HeatmapCell>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // Plays without a real time of day (Rekordbox XML sets) would pile up at midnight.
        let exact_time =
            "(metadata_json IS NULL OR metadata_json NOT LIKE '%\"approximate_time\":true%')";
        let time_cond = Self::time_range_condition(time_range, "played_at")?;
        let where_clause = match &time_cond {
            Some(cond) => format!("WHERE {cond} AND {exact_time}"),
            None => format!("WHERE {exact_time}"),
        };

        // SQLite strftime('%w', ..., 'localtime') -> 0 (Sunday) to 6 (Saturday) in user's local timezone
        // SQLite strftime('%H', ..., 'localtime') -> 00 to 23 in user's local timezone
        let sql = format!(
            r#"
            SELECT
                CAST(strftime('%w', played_at, 'localtime') AS INTEGER) as day_of_week,
                CAST(strftime('%H', played_at, 'localtime') AS INTEGER) as hour_of_day,
                COALESCE(SUM(played_ms), 0) / 60000 as minutes,
                SUM(CASE WHEN played_ms >= 30000 THEN 1 ELSE 0 END) as plays
            FROM listen_events
            {where_clause}
            GROUP BY day_of_week, hour_of_day
            HAVING day_of_week IS NOT NULL AND hour_of_day IS NOT NULL
            "#
        );

        let mut cell_map: HashMap<(u8, u8), (u64, usize)> = HashMap::new();

        let mut stmt = conn.prepare(&sql).map_err(CrateError::Database)?;
        let rows = stmt
            .query_map([], |row| {
                let day: i64 = row.get(0)?;
                let hour: i64 = row.get(1)?;
                let minutes: i64 = row.get(2)?;
                let plays: i64 = row.get(3)?;
                Ok((
                    day.clamp(0, 6) as u8,
                    hour.clamp(0, 23) as u8,
                    minutes.max(0) as u64,
                    plays.max(0) as usize,
                ))
            })
            .map_err(CrateError::Database)?;

        for item in rows {
            let (day, hour, minutes, plays) = item.map_err(CrateError::Database)?;
            cell_map.insert((day, hour), (minutes, plays));
        }

        // Return a complete 7 x 24 grid
        let mut cells = Vec::with_capacity(7 * 24);
        for day in 0..7 {
            for hour in 0..24 {
                let (minutes, plays) = cell_map.get(&(day, hour)).cloned().unwrap_or((0, 0));
                cells.push(HeatmapCell {
                    day_of_week: day,
                    hour_of_day: hour,
                    minutes,
                    plays,
                });
            }
        }

        Ok(cells)
    }

    /// Retrieves the most recent listening events.
    pub fn get_recent_listens(&self, limit: usize) -> Result<Vec<ListenEvent>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let limit_val = limit.max(1) as i64;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    id, source, track_id, title, artist, album, duration_ms, played_ms,
                    bpm, key, energy, format, artwork_url, played_at, session_id, metadata_json
                FROM listen_events
                ORDER BY played_at DESC
                LIMIT ?1
                "#,
            )
            .map_err(CrateError::Database)?;

        let rows = stmt
            .query_map(rusqlite::params![limit_val], |row| {
                let id: String = row.get(0)?;
                let source: String = row.get(1)?;
                let track_id: Option<String> = row.get(2)?;
                let title: String = row.get(3)?;
                let artist: String = row.get(4)?;
                let album: Option<String> = row.get(5)?;
                let duration_ms: i64 = row.get(6)?;
                let played_ms: i64 = row.get(7)?;
                let bpm: Option<f64> = row.get(8)?;
                let key: Option<String> = row.get(9)?;
                let energy: Option<i32> = row.get(10)?;
                let format: Option<String> = row.get(11)?;
                let artwork_url: Option<String> = row.get(12)?;
                let played_at: String = row.get(13)?;
                let session_id: Option<String> = row.get(14)?;
                let metadata_json: Option<String> = row.get(15)?;

                Ok(ListenEvent {
                    id,
                    source,
                    track_id,
                    title,
                    artist,
                    album,
                    duration_ms: duration_ms.max(0) as u64,
                    played_ms: played_ms.max(0) as u64,
                    bpm,
                    key,
                    energy,
                    format,
                    artwork_url,
                    played_at,
                    session_id,
                    metadata_json,
                })
            })
            .map_err(CrateError::Database)?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item.map_err(CrateError::Database)?);
        }

        Ok(list)
    }
}

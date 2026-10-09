//! The calendar years that hold listening data, for the "previous years" of the Pulse period bar.
//!
//! A year is listed when the `year:<YYYY>` range itself finds something in it: a listen or a
//! Rekordbox set (the two tables the Pulse summary reads). The check reuses the range's own SQL
//! condition, so a listed year can never open on an empty Pulse, whatever the time zone.

use std::collections::BTreeSet;

use chrono::{DateTime, Local, TimeZone};
use rusqlite::Connection;

use super::range::{StatsRange, MAX_YEAR, MIN_YEAR};
use super::StatsRecorderService;
use crate::error::{CrateError, Result};

/// The tables (and their time column) whose rows give a period something to show.
const SOURCES: [(&str, &str); 2] = [
    ("listen_events", "played_at"),
    ("rekordbox_sessions", "started_at"),
];

impl StatsRecorderService {
    /// The local calendar years with at least one listen or Rekordbox set, newest first.
    pub fn get_listening_years(&self) -> Result<Vec<i32>> {
        self.listening_years_at(&Local::now())
    }

    /// [`Self::get_listening_years`] in the time zone of `now`.
    pub(super) fn listening_years_at<Tz: TimeZone>(&self, now: &DateTime<Tz>) -> Result<Vec<i32>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // One pass per table gives the UTC years present. A local year starts and ends less than a
        // day away from the UTC one, so its data always sits in the UTC year of the same number or
        // a neighbour: those are the only candidates worth checking.
        let mut candidates = BTreeSet::new();
        for (table, field) in SOURCES {
            for year in utc_years(&conn, table, field)? {
                for candidate in [year - 1, year, year + 1] {
                    if (MIN_YEAR..=MAX_YEAR).contains(&candidate) {
                        candidates.insert(candidate);
                    }
                }
            }
        }

        let mut years = Vec::new();
        for year in candidates.into_iter().rev() {
            let range = StatsRange::CalendarYear(year);
            let mut found = false;
            for (table, field) in SOURCES {
                if has_rows(&conn, table, &range.sql_condition_at(field, now)?)? {
                    found = true;
                    break;
                }
            }
            if found {
                years.push(year);
            }
        }
        Ok(years)
    }
}

/// The distinct UTC years of `field` in `table` (unparseable timestamps are skipped).
fn utc_years(conn: &Connection, table: &str, field: &str) -> Result<Vec<i32>> {
    let sql = format!(
        "SELECT DISTINCT CAST(strftime('%Y', {field}) AS INTEGER) FROM {table} WHERE strftime('%Y', {field}) IS NOT NULL"
    );
    let mut stmt = conn.prepare(&sql).map_err(CrateError::Database)?;
    let rows = stmt
        .query_map([], |row| row.get::<_, i32>(0))
        .map_err(CrateError::Database)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(CrateError::Database)
}

/// Whether `table` has a row matching `condition` (`None`: no limit).
fn has_rows(conn: &Connection, table: &str, condition: &Option<String>) -> Result<bool> {
    let where_clause = condition
        .as_deref()
        .map(|cond| format!("WHERE {cond}"))
        .unwrap_or_default();
    conn.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} {where_clause})"),
        [],
        |row| row.get(0),
    )
    .map_err(CrateError::Database)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use chrono::{FixedOffset, NaiveDate, Utc};
    use rusqlite::Connection;

    use super::super::range::test_zone::EUROPE;
    use super::*;
    use crate::db::schema::get_migrations;

    fn service() -> StatsRecorderService {
        let conn = Connection::open_in_memory().unwrap();
        for migration in get_migrations() {
            conn.execute_batch(migration).unwrap();
        }
        StatsRecorderService::new(Arc::new(Mutex::new(conn)))
    }

    fn add_listen(service: &StatsRecorderService, id: &str, played_at: &str) {
        service
            .conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO listen_events (id, source, title, artist, duration_ms, played_ms, played_at)
                 VALUES (?1, 'spotify', 'Title', 'Artist', 200000, 200000, ?2)",
                rusqlite::params![id, played_at],
            )
            .unwrap();
    }

    fn add_set(service: &StatsRecorderService, id: &str, started_at: &str) {
        service
            .conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO rekordbox_sessions (id, session_name, started_at) VALUES (?1, 'Set', ?2)",
                rusqlite::params![id, started_at],
            )
            .unwrap();
    }

    fn utc_now() -> DateTime<Utc> {
        Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2026, 10, 9)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap(),
        )
    }

    #[test]
    fn an_empty_history_has_no_year() {
        assert_eq!(
            service().listening_years_at(&utc_now()).unwrap(),
            Vec::<i32>::new()
        );
    }

    #[test]
    fn lists_each_year_with_data_newest_first_and_skips_the_gaps() {
        let service = service();
        add_listen(&service, "a", "2022-06-01T10:00:00+00:00");
        add_listen(&service, "b", "2022-07-01T10:00:00Z");
        add_listen(&service, "c", "2026-03-15 20:00:00");
        add_set(&service, "s", "2024-11-20T22:00:00+00:00");
        assert_eq!(
            service.listening_years_at(&utc_now()).unwrap(),
            vec![2026, 2024, 2022]
        );
    }

    #[test]
    fn a_listen_near_new_year_counts_in_its_local_year() {
        let service = service();
        // 23:30 UTC on 31 December 2025 is already 2026 in Paris (00:30, winter time).
        add_listen(&service, "a", "2025-12-31T23:30:00Z");
        let paris_now = EUROPE.from_utc_datetime(&utc_now().naive_utc());
        assert_eq!(service.listening_years_at(&paris_now).unwrap(), vec![2026]);
        assert_eq!(service.listening_years_at(&utc_now()).unwrap(), vec![2025]);
        // Seen from New York (UTC-5) it is still 2025.
        let new_york = FixedOffset::west_opt(5 * 3600).unwrap();
        let new_york_now = new_york.from_utc_datetime(&utc_now().naive_utc());
        assert_eq!(
            service.listening_years_at(&new_york_now).unwrap(),
            vec![2025]
        );
    }

    #[test]
    fn every_listed_year_is_a_range_the_backend_accepts() {
        let service = service();
        add_listen(&service, "a", "2023-05-05T05:05:05Z");
        for year in service.listening_years_at(&utc_now()).unwrap() {
            assert!(format!("year:{year}").parse::<StatsRange>().is_ok());
        }
    }

    #[test]
    fn unparseable_timestamps_and_out_of_range_years_are_ignored() {
        let service = service();
        add_listen(&service, "bad", "not a date");
        add_listen(&service, "old", "1969-12-31T12:00:00Z");
        add_listen(&service, "ok", "2025-04-01T12:00:00Z");
        assert_eq!(service.listening_years_at(&utc_now()).unwrap(), vec![2025]);
    }

    #[test]
    fn the_list_follows_new_data() {
        let service = service();
        add_listen(&service, "a", "2025-04-01T12:00:00Z");
        assert_eq!(service.listening_years_at(&utc_now()).unwrap(), vec![2025]);
        add_listen(&service, "b", "2019-08-01T12:00:00Z");
        assert_eq!(
            service.listening_years_at(&utc_now()).unwrap(),
            vec![2025, 2019]
        );
    }
}

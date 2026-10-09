//! The calendar years that hold listening data, for the "previous years" of the Pulse period bar.
//!
//! A year is listed when the `year:<YYYY>` range itself finds something in it: a listen or a
//! Rekordbox set (the two tables the Pulse summary reads). The check reuses the range's own SQL
//! condition, so a listed year can never open on an empty Pulse, whatever the time zone.
//!
//! It runs on every Pulse load, under the shared database lock, so every query uses the index on
//! the time column (`idx_listen_events_played_at`, `idx_rekordbox_sessions_started`): the writers
//! store ISO 8601 text that starts with `YYYY-MM-DD`, so the text order is the date order. The
//! exact condition (`datetime()`, which no index can serve) only ever reads the few rows the
//! index prefilter lets through.

use std::collections::BTreeSet;

use chrono::{DateTime, Datelike, Local, TimeZone};
use rusqlite::{Connection, OptionalExtension};

use super::range::{StatsRange, MAX_YEAR, MIN_YEAR};
use super::StatsRecorderService;
use crate::error::{CrateError, Result};

/// The tables (and their time column) whose rows give a period something to show.
const SOURCES: [(&str, &str); 2] = [
    ("listen_events", "played_at"),
    ("rekordbox_sessions", "started_at"),
];

impl StatsRecorderService {
    /// The local calendar years with at least one listen or Rekordbox set, newest first, up to
    /// the current year.
    pub fn get_listening_years(&self) -> Result<Vec<i32>> {
        self.listening_years_at(&Local::now())
    }

    /// [`Self::get_listening_years`] in the time zone of `now`.
    pub(super) fn listening_years_at<Tz: TimeZone>(&self, now: &DateTime<Tz>) -> Result<Vec<i32>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // The years written at the start of the stored timestamps. A local year starts and ends
        // less than a day away from the UTC one (and an offset written in the text moves the date
        // by less than a day too), so its data always sits in the written year of the same number
        // or a neighbour: those are the only candidates worth checking. A year after the current
        // one cannot be offered (the bar ends with "This year"), and checking it would cost a
        // search for nothing.
        let last = now.year().min(MAX_YEAR);
        let mut candidates = BTreeSet::new();
        for (table, field) in SOURCES {
            for year in written_years(&conn, table, field)? {
                for candidate in [year - 1, year, year + 1] {
                    if (MIN_YEAR..=last).contains(&candidate) {
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
                let Some(exact) = range.sql_condition_at(field, now)? else {
                    continue;
                };
                if has_rows(
                    &conn,
                    table,
                    &format!("{} AND ({exact})", prefilter(field, year)),
                )? {
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

/// An index-usable text window around the local year `year`: from 30 December of the year before
/// to 3 January of the year after (a day of margin beyond the widest time zone, on each side).
/// Every row the exact condition accepts is inside it; it only lets the index skip the rest.
fn prefilter(field: &str, year: i32) -> String {
    format!(
        "{field} >= '{:04}-12-30' AND {field} < '{:04}-01-03'",
        year - 1,
        year + 1
    )
}

/// The distinct years written at the start of `field` in `table`, oldest first, read by skipping
/// through the index one year at a time (a search per year, never a scan of every row). Rows
/// that do not start with a four-digit year are left out: no period can select them either.
fn written_years(conn: &Connection, table: &str, field: &str) -> Result<Vec<i32>> {
    let sql = format!("SELECT MIN({field}) FROM {table} WHERE {field} >= ?1");
    let mut stmt = conn.prepare(&sql).map_err(CrateError::Database)?;
    let mut years = Vec::new();
    // Digits sort before letters, so the first value from "0" on is the earliest dated row.
    let mut from = "0".to_string();
    while let Some(value) = stmt
        .query_row([&from], |row| row.get::<_, Option<String>>(0))
        .optional()
        .map_err(CrateError::Database)?
        .flatten()
    {
        let Some(year) = value
            .get(..4)
            .filter(|prefix| prefix.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|prefix| prefix.parse::<i32>().ok())
        else {
            break;
        };
        years.push(year);
        if year >= 9999 {
            break;
        }
        from = format!("{:04}", year + 1);
    }
    Ok(years)
}

/// Whether `table` has a row matching `condition`.
fn has_rows(conn: &Connection, table: &str, condition: &str) -> Result<bool> {
    conn.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {condition})"),
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

    #[test]
    fn a_year_after_the_current_one_is_never_offered() {
        let service = service();
        // A clock set wrong once wrote a listen in the future.
        add_listen(&service, "future", "2027-05-01T12:00:00Z");
        add_listen(&service, "now", "2026-02-01T12:00:00Z");
        assert_eq!(service.listening_years_at(&utc_now()).unwrap(), vec![2026]);
    }

    #[test]
    fn a_listen_written_with_an_offset_still_counts_in_its_local_year() {
        let service = service();
        // 00:30 in Paris on 1 January 2026 is 23:30 UTC on 31 December 2025.
        add_listen(&service, "a", "2026-01-01T00:30:00+01:00");
        let new_york = FixedOffset::west_opt(5 * 3600).unwrap();
        let new_york_now = new_york.from_utc_datetime(&utc_now().naive_utc());
        assert_eq!(
            service.listening_years_at(&new_york_now).unwrap(),
            vec![2025]
        );
        let paris_now = EUROPE.from_utc_datetime(&utc_now().naive_utc());
        assert_eq!(service.listening_years_at(&paris_now).unwrap(), vec![2026]);
    }

    #[test]
    fn the_written_years_skip_the_gaps_and_stop_at_text_that_is_not_a_date() {
        let service = service();
        add_listen(&service, "a", "2019-03-01T10:00:00Z");
        add_listen(&service, "b", "2019-09-01 10:00:00");
        add_listen(&service, "c", "2023-01-01T00:00:00Z");
        add_listen(&service, "d", "unknown");
        let conn = service.conn.lock().unwrap();
        assert_eq!(
            written_years(&conn, "listen_events", "played_at").unwrap(),
            vec![2019, 2023]
        );
        assert_eq!(
            written_years(&conn, "rekordbox_sessions", "started_at").unwrap(),
            Vec::<i32>::new()
        );
    }

    #[test]
    fn every_query_searches_the_time_index_instead_of_scanning_the_table() {
        let service = service();
        let conn = service.conn.lock().unwrap();
        let plan = |sql: &str| -> String {
            let mut stmt = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap();
            let rows = stmt.query_map([], |row| row.get::<_, String>(3)).unwrap();
            rows.map(|r| r.unwrap()).collect::<Vec<_>>().join(" | ")
        };
        for (table, field) in SOURCES {
            let seek = plan(&format!(
                "SELECT MIN({field}) FROM {table} WHERE {field} >= '2026'"
            ));
            assert!(seek.contains("USING COVERING INDEX"), "{table}: {seek}");
            let exists = plan(&format!(
                "SELECT EXISTS(SELECT 1 FROM {table} WHERE {} AND datetime({field}) IS NOT NULL)",
                prefilter(field, 2025)
            ));
            assert!(exists.contains("USING COVERING INDEX"), "{table}: {exists}");
        }
    }
}

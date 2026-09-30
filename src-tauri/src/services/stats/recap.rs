//! "Your week" and "Your year": an automatic recap of the listening history.
//!
//! It assembles the statistics Crate Pulse already computes (summary, top tracks and artists,
//! keys, heatmap) over a *calendar* week or year instead of a rolling window, compares the total
//! with the previous period, and counts the tracks heard for the first time. Every source is
//! included: Spotify, Crate, Mixed In Key and the Rekordbox sets.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use super::StatsRecorderService;
use crate::error::{CrateError, Result};
use crate::models::stats::{HarmonicStatsItem, TopArtistItem, TopTrackItem};

const SQL_DATETIME: &str = "%Y-%m-%d %H:%M:%S";
/// Entries kept in the top lists of a recap.
const TOP_ITEMS: usize = 5;
/// Keys kept in a recap.
const TOP_KEYS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecapPeriod {
    /// Monday to Sunday.
    Week,
    /// 1 January to 31 December.
    Year,
}

/// The busiest calendar day of a period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecapDay {
    /// Local date, `YYYY-MM-DD`.
    pub date: String,
    pub plays: usize,
    pub minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recap {
    pub period: RecapPeriod,
    /// First and last local day of the period, `YYYY-MM-DD`, both included.
    pub start_date: String,
    pub end_date: String,
    pub total_plays: usize,
    pub total_minutes: u64,
    /// Different tracks (and artists) heard at least once for 30 s or more.
    pub unique_tracks: usize,
    pub unique_artists: usize,
    /// Tracks whose very first listen, all sources and all time considered, is in this period.
    pub new_tracks: usize,
    /// Totals of the period just before, to show the change.
    pub previous_plays: usize,
    pub previous_minutes: u64,
    pub top_tracks: Vec<TopTrackItem>,
    pub top_artists: Vec<TopArtistItem>,
    pub top_keys: Vec<HarmonicStatsItem>,
    pub busiest_day: Option<RecapDay>,
    /// Local hour of the day (0 to 23) with the most plays, exact times only.
    pub peak_hour: Option<u8>,
    /// Day of the week (0 = Sunday … 6 = Saturday) with the most listening minutes.
    pub peak_weekday: Option<u8>,
    /// Listening minutes per source (`spotify`, `crate_local`, `rekordbox`…).
    pub source_minutes: HashMap<String, u64>,
}

/// The bounds of a period, as instants in UTC and as local calendar days.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PeriodBounds {
    pub start_utc: NaiveDateTime,
    pub end_utc: NaiveDateTime,
    pub first_day: NaiveDate,
    pub last_day: NaiveDate,
}

impl PeriodBounds {
    fn window(&self) -> String {
        format!(
            "between:{},{}",
            self.start_utc.format(SQL_DATETIME),
            self.end_utc.format(SQL_DATETIME)
        )
    }
}

/// Local midnight of `day` as a UTC instant. A day whose midnight does not exist (a clock change
/// at midnight) starts at the first hour that does.
fn local_midnight_utc<Tz: TimeZone>(tz: &Tz, day: NaiveDate) -> NaiveDateTime {
    for hour in 0..4 {
        let local = day.and_hms_opt(hour, 0, 0).expect("valid hour");
        if let Some(instant) = tz.from_local_datetime(&local).earliest() {
            return instant.with_timezone(&Utc).naive_utc();
        }
    }
    day.and_hms_opt(0, 0, 0).expect("midnight")
}

/// The period containing `now`, moved back by `offset` whole periods.
pub(super) fn period_bounds<Tz: TimeZone>(
    period: RecapPeriod,
    offset: u32,
    now: &DateTime<Tz>,
) -> Result<PeriodBounds> {
    let today = now.date_naive();
    let invalid = || CrateError::InvalidOperation("recap period out of range".to_string());

    let (first_day, next_first_day) = match period {
        RecapPeriod::Week => {
            let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
            let first = monday - Duration::days(7 * i64::from(offset));
            (first, first + Duration::days(7))
        }
        RecapPeriod::Year => {
            let year = today.year() - i32::try_from(offset).map_err(|_| invalid())?;
            (
                NaiveDate::from_ymd_opt(year, 1, 1).ok_or_else(invalid)?,
                NaiveDate::from_ymd_opt(year + 1, 1, 1).ok_or_else(invalid)?,
            )
        }
    };

    let tz = now.timezone();
    Ok(PeriodBounds {
        start_utc: local_midnight_utc(&tz, first_day),
        end_utc: local_midnight_utc(&tz, next_first_day),
        first_day,
        last_day: next_first_day - Duration::days(1),
    })
}

impl StatsRecorderService {
    /// The recap of the current week or year (`offset` 0), or of an earlier one (1 = the one
    /// before, and so on).
    pub fn get_recap(&self, period: RecapPeriod, offset: u32) -> Result<Recap> {
        self.recap_at(period, offset, &chrono::Local::now())
    }

    pub(super) fn recap_at<Tz: TimeZone>(
        &self,
        period: RecapPeriod,
        offset: u32,
        now: &DateTime<Tz>,
    ) -> Result<Recap> {
        let bounds = period_bounds(period, offset, now)?;
        let previous = period_bounds(period, offset.saturating_add(1), now)?;
        let window = bounds.window();

        let summary = self.get_stats_summary(&window)?;
        let previous_summary = self.get_stats_summary(&previous.window())?;

        // Every artist of the period, to count them, then the first few.
        let mut artists = self.get_top_artists(&window, usize::MAX)?;
        let unique_artists = artists.len();
        artists.truncate(TOP_ITEMS);

        let mut keys = self.get_harmonic_stats(&window)?;
        keys.sort_by(|a, b| b.plays.cmp(&a.plays).then_with(|| a.key.cmp(&b.key)));
        keys.truncate(TOP_KEYS);

        let (peak_hour, peak_weekday) = peak_times(&self.get_listening_heatmap(&window)?);

        Ok(Recap {
            period,
            start_date: bounds.first_day.to_string(),
            end_date: bounds.last_day.to_string(),
            total_plays: summary.total_plays,
            total_minutes: summary.total_minutes,
            unique_tracks: self.count_unique_tracks(&window)?,
            unique_artists,
            new_tracks: self.count_new_tracks(&bounds)?,
            previous_plays: previous_summary.total_plays,
            previous_minutes: previous_summary.total_minutes,
            top_tracks: self.get_top_tracks(&window, TOP_ITEMS)?,
            top_artists: artists,
            top_keys: keys,
            busiest_day: self.busiest_day(&window)?,
            peak_hour,
            peak_weekday,
            source_minutes: summary.source_breakdown,
        })
    }

    fn count_unique_tracks(&self, window: &str) -> Result<usize> {
        let condition = Self::time_range_condition(window, "played_at").unwrap_or_default();
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let count: i64 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM (
                         SELECT 1 FROM listen_events
                         WHERE played_ms >= 30000 AND {condition}
                         GROUP BY title, artist
                     )"
                ),
                [],
                |r| r.get(0),
            )
            .map_err(CrateError::Database)?;
        Ok(count.max(0) as usize)
    }

    /// Tracks whose first stream (30 s or more) falls inside the period.
    fn count_new_tracks(&self, bounds: &PeriodBounds) -> Result<usize> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM (
                     SELECT MIN(datetime(played_at)) AS first_heard
                     FROM listen_events
                     WHERE played_ms >= 30000
                     GROUP BY title, artist
                 )
                 WHERE first_heard >= datetime(?1) AND first_heard < datetime(?2)",
                rusqlite::params![
                    bounds.start_utc.format(SQL_DATETIME).to_string(),
                    bounds.end_utc.format(SQL_DATETIME).to_string()
                ],
                |r| r.get(0),
            )
            .map_err(CrateError::Database)?;
        Ok(count.max(0) as usize)
    }

    /// The local calendar day with the most listening minutes.
    fn busiest_day(&self, window: &str) -> Result<Option<RecapDay>> {
        let condition = Self::time_range_condition(window, "played_at").unwrap_or_default();
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT date(played_at, 'localtime') AS day,
                        COALESCE(SUM(played_ms), 0) / 60000 AS minutes,
                        COALESCE(SUM(CASE WHEN played_ms >= 30000 THEN 1 ELSE 0 END), 0) AS plays
                 FROM listen_events
                 WHERE {condition}
                 GROUP BY day
                 HAVING day IS NOT NULL
                 ORDER BY minutes DESC, plays DESC, day ASC
                 LIMIT 1"
            ))
            .map_err(CrateError::Database)?;
        let mut rows = stmt
            .query_map([], |r| {
                Ok(RecapDay {
                    date: r.get(0)?,
                    minutes: r.get::<_, i64>(1)?.max(0) as u64,
                    plays: r.get::<_, i64>(2)?.max(0) as usize,
                })
            })
            .map_err(CrateError::Database)?;
        match rows.next() {
            Some(day) => Ok(Some(day.map_err(CrateError::Database)?)),
            None => Ok(None),
        }
    }
}

/// The local hour with the most plays and the weekday with the most minutes, or `None` when
/// nothing was played. Ties go to the earliest hour and the earliest weekday.
fn peak_times(cells: &[crate::models::stats::HeatmapCell]) -> (Option<u8>, Option<u8>) {
    let mut plays_by_hour = [0usize; 24];
    let mut minutes_by_weekday = [0u64; 7];
    for cell in cells {
        if let Some(hour) = plays_by_hour.get_mut(usize::from(cell.hour_of_day)) {
            *hour += cell.plays;
        }
        if let Some(day) = minutes_by_weekday.get_mut(usize::from(cell.day_of_week)) {
            *day += cell.minutes;
        }
    }
    let hour = plays_by_hour
        .iter()
        .enumerate()
        .max_by_key(|(hour, plays)| (**plays, std::cmp::Reverse(*hour)))
        .filter(|(_, plays)| **plays > 0)
        .map(|(hour, _)| hour as u8);
    let weekday = minutes_by_weekday
        .iter()
        .enumerate()
        .max_by_key(|(day, minutes)| (**minutes, std::cmp::Reverse(*day)))
        .filter(|(_, minutes)| **minutes > 0)
        .map(|(day, _)| day as u8);
    (hour, weekday)
}

//! The period a statistics query covers, as a typed value.
//!
//! The frontend sends the period as a string (`TimeRange` in `shared/types/stats.ts`); this module
//! is the single place where that string is understood. [`StatsRange`] parses it, rejects anything
//! it does not know (an unknown period used to silently mean "all time"), and turns it into an SQL
//! condition built only from validated, re-formatted values: nothing the caller wrote reaches the
//! SQL text.
//!
//! | Range | Meaning |
//! | --- | --- |
//! | `today` | Since local midnight |
//! | `7d`, `30d` | The last 7 / 30 x 24 hours |
//! | `3m`, `6m` | Rolling months: from the same local date and time 3 / 6 months ago until now |
//! | `year` | Since 1 January of the current local year |
//! | `all` | No limit |
//! | `year:<YYYY>` | The whole local calendar year `YYYY` |
//! | `custom:<YYYY-MM-DD>,<YYYY-MM-DD>` | From local midnight of the first day to the end of the second day (both days included) |
//! | `between:<start>,<end>` | An exact window in UTC, `YYYY-MM-DD HH:MM:SS`, start included, end excluded (used by the recap) |
//!
//! Local days are converted to UTC instants with the DST-safe helpers of the recap, so a day with a
//! clock change is 23 or 25 hours long, and a midnight that does not exist starts at the first hour
//! that does.

use std::str::FromStr;

use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, NaiveDateTime, TimeZone, Utc};

use super::recap::local_midnight_utc;
use crate::error::{CrateError, Result};

const SQL_DATETIME: &str = "%Y-%m-%d %H:%M:%S";
/// First and last year accepted in a calendar year, a custom day or a window. Nothing was
/// recorded before 1970, and keeping the upper bound low keeps every formatted date at 4 digits.
pub(super) const MIN_YEAR: i32 = 1970;
pub(super) const MAX_YEAR: i32 = 2999;

/// A validated statistics period. See the module documentation for the accepted strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsRange {
    Today,
    Days7,
    Days30,
    Months3,
    Months6,
    /// Since 1 January of the current local year.
    YearToDate,
    All,
    /// A whole local calendar year.
    CalendarYear(i32),
    /// Local days `from` to `to`, both included.
    Custom {
        from: NaiveDate,
        to: NaiveDate,
    },
    /// An exact window in UTC, start included, end excluded.
    Window {
        start: NaiveDateTime,
        end: NaiveDateTime,
    },
}

fn invalid(input: &str, reason: &str) -> CrateError {
    let shown: String = input.chars().take(60).collect();
    CrateError::InvalidOperation(format!("invalid statistics range {shown:?}: {reason}"))
}

fn check_year(input: &str, year: i32) -> Result<()> {
    if (MIN_YEAR..=MAX_YEAR).contains(&year) {
        Ok(())
    } else {
        Err(invalid(
            input,
            &format!("the year must be between {MIN_YEAR} and {MAX_YEAR}"),
        ))
    }
}

/// Exactly four ASCII digits, inside the accepted years.
fn parse_year(input: &str, text: &str) -> Result<i32> {
    if text.len() != 4 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid(input, "a year is written with four digits (YYYY)"));
    }
    let year: i32 = text.parse().map_err(|_| invalid(input, "not a year"))?;
    check_year(input, year)?;
    Ok(year)
}

/// Exactly `YYYY-MM-DD`, a real calendar day, inside the accepted years.
fn parse_day(input: &str, text: &str) -> Result<NaiveDate> {
    let shape_ok = text.len() == 10
        && text.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            _ => b.is_ascii_digit(),
        });
    if !shape_ok {
        return Err(invalid(input, "a day is written YYYY-MM-DD"));
    }
    let day = NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map_err(|_| invalid(input, "not a calendar day"))?;
    check_year(input, day.year())?;
    Ok(day)
}

fn parse_instant(input: &str, text: &str) -> Result<NaiveDateTime> {
    let instant = NaiveDateTime::parse_from_str(text.trim(), SQL_DATETIME)
        .map_err(|_| invalid(input, "a window bound is written YYYY-MM-DD HH:MM:SS"))?;
    check_year(input, instant.year())?;
    Ok(instant)
}

impl FromStr for StatsRange {
    type Err = CrateError;

    fn from_str(input: &str) -> Result<Self> {
        let text = input.trim().to_lowercase();
        if let Some(year) = text.strip_prefix("year:") {
            return Ok(Self::CalendarYear(parse_year(input, year)?));
        }
        if let Some(days) = text.strip_prefix("custom:") {
            let (from, to) = days
                .split_once(',')
                .ok_or_else(|| invalid(input, "expected custom:<YYYY-MM-DD>,<YYYY-MM-DD>"))?;
            let (from, to) = (parse_day(input, from)?, parse_day(input, to)?);
            if from > to {
                return Err(invalid(input, "the first day is after the last day"));
            }
            return Ok(Self::Custom { from, to });
        }
        if let Some(bounds) = text.strip_prefix("between:") {
            let (start, end) = bounds
                .split_once(',')
                .ok_or_else(|| invalid(input, "expected between:<start>,<end>"))?;
            let (start, end) = (parse_instant(input, start)?, parse_instant(input, end)?);
            if start > end {
                return Err(invalid(input, "the window ends before it starts"));
            }
            return Ok(Self::Window { start, end });
        }
        match text.as_str() {
            "today" => Ok(Self::Today),
            "7d" => Ok(Self::Days7),
            "30d" => Ok(Self::Days30),
            "3m" => Ok(Self::Months3),
            "6m" => Ok(Self::Months6),
            "year" => Ok(Self::YearToDate),
            "all" => Ok(Self::All),
            _ => Err(invalid(
                input,
                "expected today, 7d, 30d, 3m, 6m, year, all, year:<YYYY> or custom:<YYYY-MM-DD>,<YYYY-MM-DD>",
            )),
        }
    }
}

fn out_of_calendar() -> CrateError {
    CrateError::InvalidOperation("statistics range out of the calendar".to_string())
}

/// The instant `months` months before `now`, at the same local date and time (the day is clamped
/// to the end of a shorter month), as a UTC instant. A local time that does not exist (a clock
/// change) moves to the first hour that does; a repeated one takes its first occurrence.
fn rolling_months_start<Tz: TimeZone>(now: &DateTime<Tz>, months: u32) -> Result<NaiveDateTime> {
    let local = now
        .naive_local()
        .checked_sub_months(Months::new(months))
        .ok_or_else(out_of_calendar)?;
    let tz = now.timezone();
    for shift in 0..4 {
        let candidate = local + Duration::hours(shift);
        if let Some(instant) = tz.from_local_datetime(&candidate).earliest() {
            return Ok(instant.with_timezone(&Utc).naive_utc());
        }
    }
    Ok(local)
}

fn since(field: &str, start: NaiveDateTime) -> String {
    format!(
        "datetime({field}) >= datetime('{}')",
        start.format(SQL_DATETIME)
    )
}

fn between(field: &str, start: NaiveDateTime, end: NaiveDateTime) -> String {
    format!(
        "datetime({field}) >= datetime('{}') AND datetime({field}) < datetime('{}')",
        start.format(SQL_DATETIME),
        end.format(SQL_DATETIME)
    )
}

impl StatsRange {
    /// The UTC window `[start, end)` of a calendar year, a custom span of local days or an exact
    /// window; `None` for the presets, which are relative to "now".
    pub fn utc_window<Tz: TimeZone>(&self, tz: &Tz) -> Option<(NaiveDateTime, NaiveDateTime)> {
        match *self {
            Self::CalendarYear(year) => Some((
                local_midnight_utc(tz, NaiveDate::from_ymd_opt(year, 1, 1)?),
                local_midnight_utc(tz, NaiveDate::from_ymd_opt(year + 1, 1, 1)?),
            )),
            Self::Custom { from, to } => Some((
                local_midnight_utc(tz, from),
                local_midnight_utc(tz, to.succ_opt()?),
            )),
            Self::Window { start, end } => Some((start, end)),
            _ => None,
        }
    }

    /// The SQL condition on `field` (a column name chosen by the caller, never user input) for
    /// this range, or `None` when the range has no limit. `now` fixes the time zone and the
    /// instant the rolling ranges count back from.
    pub fn sql_condition_at<Tz: TimeZone>(
        &self,
        field: &str,
        now: &DateTime<Tz>,
    ) -> Result<Option<String>> {
        let condition = match *self {
            // `datetime()` normalises RFC 3339 values (with a `T`, a `Z` or an offset) to UTC
            // before comparing; comparing the raw strings mixed formats and time zones.
            Self::Today => format!(
                "datetime({field}, 'localtime') >= datetime('now', 'localtime', 'start of day')"
            ),
            Self::Days7 => format!("datetime({field}) >= datetime('now', '-7 days')"),
            Self::Days30 => format!("datetime({field}) >= datetime('now', '-30 days')"),
            Self::YearToDate => format!(
                "datetime({field}, 'localtime') >= datetime('now', 'localtime', 'start of year')"
            ),
            Self::Months3 => since(field, rolling_months_start(now, 3)?),
            Self::Months6 => since(field, rolling_months_start(now, 6)?),
            Self::All => return Ok(None),
            Self::CalendarYear(_) | Self::Custom { .. } | Self::Window { .. } => {
                let (start, end) = self
                    .utc_window(&now.timezone())
                    .ok_or_else(out_of_calendar)?;
                between(field, start, end)
            }
        };
        Ok(Some(condition))
    }

    /// The same period as [`Self::sql_condition_at`], with every bound written as a fixed UTC
    /// instant computed from `now`: `today`, `7d`, `30d` and `year` otherwise ask SQLite for
    /// `'now'` each time the condition runs. A query run several times for one result (the
    /// history export reads in chunks) then selects one stable period from start to end.
    pub fn fixed_sql_condition_at<Tz: TimeZone>(
        &self,
        field: &str,
        now: &DateTime<Tz>,
    ) -> Result<Option<String>> {
        let tz = now.timezone();
        let now_utc = now.with_timezone(&Utc).naive_utc();
        let local_day = now.naive_local().date();
        let start = match *self {
            Self::Today => local_midnight_utc(&tz, local_day),
            Self::Days7 => now_utc - Duration::days(7),
            Self::Days30 => now_utc - Duration::days(30),
            Self::YearToDate => local_midnight_utc(
                &tz,
                NaiveDate::from_ymd_opt(local_day.year(), 1, 1).ok_or_else(out_of_calendar)?,
            ),
            _ => return self.sql_condition_at(field, now),
        };
        Ok(Some(since(field, start)))
    }
}

#[cfg(test)]
pub(super) mod test_zone {
    //! A time zone with the European daylight saving rule, for tests that must not depend on the
    //! machine's zone: +1 hour from the last Sunday of March to the last Sunday of October.
    use chrono::{
        Datelike, Duration, FixedOffset, LocalResult, NaiveDate, NaiveDateTime, Offset, TimeZone,
    };
    use std::fmt;

    #[derive(Clone, Copy, Debug)]
    pub struct DstZone {
        /// Standard offset from UTC in seconds.
        pub std_secs: i32,
        /// UTC hour of the switch (01:00 UTC in Europe).
        pub switch_utc_hour: u32,
    }

    /// Central European Time: UTC+1, +2 in summer, switching at 01:00 UTC.
    pub const EUROPE: DstZone = DstZone {
        std_secs: 3600,
        switch_utc_hour: 1,
    };
    /// A zone whose clock jumps at local midnight (00:00 standard becomes 01:00), so the first
    /// local day of summer time has no midnight.
    pub const MIDNIGHT_GAP: DstZone = DstZone {
        std_secs: -3 * 3600,
        switch_utc_hour: 3,
    };

    #[derive(Clone, Copy, Debug)]
    pub struct DstOffset {
        zone: DstZone,
        secs: i32,
    }

    impl fmt::Display for DstOffset {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{:+}s", self.secs)
        }
    }

    impl Offset for DstOffset {
        fn fix(&self) -> FixedOffset {
            FixedOffset::east_opt(self.secs).unwrap()
        }
    }

    fn last_sunday(year: i32, month: u32) -> NaiveDate {
        let first_next = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .unwrap();
        let last = first_next.pred_opt().unwrap();
        last - Duration::days(i64::from(last.weekday().num_days_from_sunday()))
    }

    impl DstZone {
        fn is_dst(&self, utc: NaiveDateTime) -> bool {
            let at = |month| {
                last_sunday(utc.year(), month)
                    .and_hms_opt(self.switch_utc_hour, 0, 0)
                    .unwrap()
            };
            utc >= at(3) && utc < at(10)
        }

        fn offset(&self, dst: bool) -> DstOffset {
            DstOffset {
                zone: *self,
                secs: self.std_secs + if dst { 3600 } else { 0 },
            }
        }
    }

    impl TimeZone for DstZone {
        type Offset = DstOffset;

        fn from_offset(offset: &DstOffset) -> Self {
            offset.zone
        }

        fn offset_from_local_date(&self, local: &NaiveDate) -> LocalResult<DstOffset> {
            self.offset_from_local_datetime(&local.and_hms_opt(12, 0, 0).unwrap())
        }

        fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> LocalResult<DstOffset> {
            let as_std = !self.is_dst(*local - Duration::seconds(i64::from(self.std_secs)));
            let as_dst = self.is_dst(*local - Duration::seconds(i64::from(self.std_secs) + 3600));
            match (as_std, as_dst) {
                // Clocks went back: the summer reading comes first.
                (true, true) => LocalResult::Ambiguous(self.offset(true), self.offset(false)),
                (true, false) => LocalResult::Single(self.offset(false)),
                (false, true) => LocalResult::Single(self.offset(true)),
                (false, false) => LocalResult::None,
            }
        }

        fn offset_from_utc_date(&self, utc: &NaiveDate) -> DstOffset {
            self.offset_from_utc_datetime(&utc.and_hms_opt(12, 0, 0).unwrap())
        }

        fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> DstOffset {
            self.offset(self.is_dst(*utc))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_zone::{DstZone, EUROPE, MIDNIGHT_GAP};
    use super::*;

    fn utc(text: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(text, SQL_DATETIME).unwrap()
    }

    fn local(zone: DstZone, text: &str) -> DateTime<DstZone> {
        zone.from_local_datetime(&utc(text)).earliest().unwrap()
    }

    fn range(text: &str) -> StatsRange {
        text.parse().unwrap_or_else(|e| panic!("{text}: {e}"))
    }

    fn window(zone: DstZone, text: &str) -> (NaiveDateTime, NaiveDateTime) {
        range(text).utc_window(&zone).unwrap()
    }

    fn condition(zone: DstZone, text: &str, now: &str) -> String {
        range(text)
            .sql_condition_at("played_at", &local(zone, now))
            .unwrap()
            .unwrap()
    }

    #[test]
    fn every_documented_form_parses() {
        assert_eq!(range("today"), StatsRange::Today);
        assert_eq!(range("7d"), StatsRange::Days7);
        assert_eq!(range("30d"), StatsRange::Days30);
        assert_eq!(range("3m"), StatsRange::Months3);
        assert_eq!(range("6m"), StatsRange::Months6);
        assert_eq!(range("year"), StatsRange::YearToDate);
        assert_eq!(range("all"), StatsRange::All);
        assert_eq!(range("year:2025"), StatsRange::CalendarYear(2025));
        assert_eq!(
            range("custom:2026-03-01,2026-03-31"),
            StatsRange::Custom {
                from: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
                to: NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            }
        );
        assert_eq!(
            range("between:2026-09-21 00:00:00,2026-09-28 00:00:00"),
            StatsRange::Window {
                start: utc("2026-09-21 00:00:00"),
                end: utc("2026-09-28 00:00:00"),
            }
        );
    }

    #[test]
    fn names_are_case_and_whitespace_tolerant() {
        assert_eq!(range("  Today "), StatsRange::Today);
        assert_eq!(range("ALL"), StatsRange::All);
        assert_eq!(range("3M"), StatsRange::Months3);
        assert_eq!(range("Year:2024"), StatsRange::CalendarYear(2024));
    }

    #[test]
    fn a_single_day_custom_range_is_valid() {
        assert!("custom:2026-03-01,2026-03-01".parse::<StatsRange>().is_ok());
    }

    #[test]
    fn unknown_malformed_and_reversed_ranges_are_errors() {
        for bad in [
            "",
            "   ",
            "yesterday",
            "7days",
            "12m",
            "1y",
            "week",
            "year:",
            "year:26",
            "year:20266",
            "year:+2026",
            "year: 2026",
            "year:abcd",
            "year:1969",
            "year:3000",
            "year:２０２６",
            "custom:",
            "custom:2026-03-01",
            "custom:2026-03-01,",
            "custom:,2026-03-01",
            "custom:2026-3-1,2026-3-5",
            "custom:2026-03-01,2026-03-05,2026-03-09",
            "custom:2026-02-30,2026-03-05",
            "custom:2027-02-29,2027-03-05",
            "custom:2026-13-01,2026-14-01",
            "custom:2026-03-05,2026-03-01",
            "custom:1969-12-31,2026-01-01",
            "custom:2026-01-01,3000-01-01",
            "custom:2026-03-01 ,2026-03-05 ",
            "between:",
            "between:2026-09-21",
            "between:2026-09-21 00:00:00",
            "between:yesterday,today",
            "between:2026-09-21 00:00:00,",
            "between:2026-09-28 00:00:00,2026-09-21 00:00:00",
        ] {
            let err = bad.parse::<StatsRange>().expect_err(bad);
            assert!(
                matches!(err, CrateError::InvalidOperation(ref m) if m.contains("invalid statistics range")),
                "{bad}: {err}"
            );
        }
    }

    #[test]
    fn hostile_strings_are_rejected_not_interpolated() {
        for hostile in [
            "7d'; DROP TABLE listen_events; --",
            "today' OR '1'='1",
            "year:2026'); DROP TABLE listen_events; --",
            "year:2026; DROP TABLE listen_events",
            "custom:2026-01-01'); DROP TABLE listen_events; --,2026-01-02",
            "custom:2026-01-01,2026-01-02'; --",
            "custom:2026-01-01,2026-01-02) OR (1=1",
            "between:2026-09-21 00:00:00'); DROP TABLE listen_events; --,2026-09-28 00:00:00",
            "all; DELETE FROM listen_events",
            "all\0",
        ] {
            assert!(hostile.parse::<StatsRange>().is_err(), "{hostile}");
        }
    }

    #[test]
    fn the_error_is_short_even_for_a_huge_input() {
        let message = "x"
            .repeat(10_000)
            .parse::<StatsRange>()
            .unwrap_err()
            .to_string();
        assert!(message.len() < 300, "{}", message.len());
    }

    #[test]
    fn a_calendar_year_runs_from_local_new_year_to_the_next() {
        // Paris is UTC+1 in winter: local midnight is 23:00 UTC the evening before.
        assert_eq!(
            window(EUROPE, "year:2026"),
            (utc("2025-12-31 23:00:00"), utc("2026-12-31 23:00:00"))
        );
    }

    #[test]
    fn a_leap_year_is_366_days_long() {
        let (start, end) = window(EUROPE, "year:2028");
        assert_eq!(start, utc("2027-12-31 23:00:00"));
        assert_eq!(end, utc("2028-12-31 23:00:00"));
        assert_eq!((end - start).num_days(), 366);
        // 29 February exists in 2028 and that day is exactly 24 hours.
        let (feb_start, feb_end) = window(EUROPE, "custom:2028-02-29,2028-02-29");
        assert_eq!(feb_start, utc("2028-02-28 23:00:00"));
        assert_eq!(feb_end, utc("2028-02-29 23:00:00"));
        let (start, end) = window(EUROPE, "year:2027");
        assert_eq!((end - start).num_days(), 365);
    }

    #[test]
    fn a_custom_range_includes_its_last_day() {
        // 1 to 3 March 2026, UTC+1: three whole days, the end is local midnight of 4 March.
        assert_eq!(
            window(EUROPE, "custom:2026-03-01,2026-03-03"),
            (utc("2026-02-28 23:00:00"), utc("2026-03-03 23:00:00"))
        );
    }

    #[test]
    fn a_spring_forward_day_is_23_hours_long() {
        // 29 March 2026: 02:00 became 03:00 at 01:00 UTC.
        let (start, end) = window(EUROPE, "custom:2026-03-29,2026-03-29");
        assert_eq!(start, utc("2026-03-28 23:00:00")); // +01:00
        assert_eq!(end, utc("2026-03-29 22:00:00")); // +02:00
        assert_eq!((end - start).num_hours(), 23);
        // Across the change the window is 47 hours, not 48.
        let (start, end) = window(EUROPE, "custom:2026-03-28,2026-03-29");
        assert_eq!(start, utc("2026-03-27 23:00:00"));
        assert_eq!(end, utc("2026-03-29 22:00:00"));
        assert_eq!((end - start).num_hours(), 47);
    }

    #[test]
    fn a_fall_back_day_is_25_hours_long() {
        // 25 October 2026: 03:00 became 02:00 at 01:00 UTC.
        let (start, end) = window(EUROPE, "custom:2026-10-25,2026-10-25");
        assert_eq!(start, utc("2026-10-24 22:00:00")); // +02:00
        assert_eq!(end, utc("2026-10-25 23:00:00")); // +01:00
        assert_eq!((end - start).num_hours(), 25);
    }

    #[test]
    fn a_missing_midnight_starts_the_day_at_the_first_hour_that_exists() {
        // Local 00:00 to 01:00 on 29 March 2026 does not exist: the day starts at 01:00 (-02:00).
        let (start, end) = window(MIDNIGHT_GAP, "custom:2026-03-29,2026-03-29");
        assert_eq!(start, utc("2026-03-29 03:00:00"));
        // The next midnight is a regular one at -02:00.
        assert_eq!(end, utc("2026-03-30 02:00:00"));
        assert_eq!((end - start).num_hours(), 23);
        // The day before ends where this one starts: no instant is lost or counted twice.
        let (_, previous_end) = window(MIDNIGHT_GAP, "custom:2026-03-28,2026-03-28");
        assert_eq!(previous_end, start);
    }

    fn fixed(zone: DstZone, text: &str, now: &str) -> String {
        range(text)
            .fixed_sql_condition_at("played_at", &local(zone, now))
            .unwrap()
            .unwrap()
    }

    #[test]
    fn the_fixed_form_writes_the_relative_presets_as_utc_instants() {
        // 10 July 2026 14:30 in Paris (UTC+2) is 12:30 UTC.
        let now = "2026-07-10 14:30:00";
        assert_eq!(
            fixed(EUROPE, "today", now),
            "datetime(played_at) >= datetime('2026-07-09 22:00:00')"
        );
        assert_eq!(
            fixed(EUROPE, "7d", now),
            "datetime(played_at) >= datetime('2026-07-03 12:30:00')"
        );
        assert_eq!(
            fixed(EUROPE, "30d", now),
            "datetime(played_at) >= datetime('2026-06-10 12:30:00')"
        );
        // 1 January is in winter time (UTC+1).
        assert_eq!(
            fixed(EUROPE, "year", now),
            "datetime(played_at) >= datetime('2025-12-31 23:00:00')"
        );
        // The other periods are already fixed: same text as the plain condition.
        for text in ["3m", "6m", "year:2025", "custom:2026-03-01,2026-03-31"] {
            assert_eq!(fixed(EUROPE, text, now), condition(EUROPE, text, now));
        }
        assert_eq!(
            range("all")
                .fixed_sql_condition_at("played_at", &local(EUROPE, now))
                .unwrap(),
            None
        );
    }

    #[test]
    fn rolling_months_keep_the_local_wall_clock() {
        // 15 June 2026 12:00 (UTC+2): three months earlier is 15 March 12:00 (UTC+1), so the
        // cut is 11:00 UTC, not 10:00 UTC as a fixed number of 24-hour days would give.
        assert_eq!(
            condition(EUROPE, "3m", "2026-06-15 12:00:00"),
            "datetime(played_at) >= datetime('2026-03-15 11:00:00')"
        );
        assert_eq!(
            condition(EUROPE, "6m", "2026-09-29 12:00:00"),
            "datetime(played_at) >= datetime('2026-03-29 10:00:00')"
        );
    }

    #[test]
    fn rolling_months_clamp_to_the_end_of_a_shorter_month() {
        // 31 May minus three months: February has no 31st.
        assert_eq!(
            condition(EUROPE, "3m", "2026-05-31 12:00:00"),
            "datetime(played_at) >= datetime('2026-02-28 11:00:00')"
        );
        // ... and 29 February exists in a leap year.
        assert_eq!(
            condition(EUROPE, "3m", "2028-05-31 12:00:00"),
            "datetime(played_at) >= datetime('2028-02-29 11:00:00')"
        );
        // Six months before 31 August is 28 February (2026).
        assert_eq!(
            condition(EUROPE, "6m", "2026-08-31 12:00:00"),
            "datetime(played_at) >= datetime('2026-02-28 11:00:00')"
        );
        // Across the year boundary.
        assert_eq!(
            condition(EUROPE, "6m", "2026-02-10 08:30:00"),
            "datetime(played_at) >= datetime('2025-08-10 06:30:00')"
        );
    }

    #[test]
    fn rolling_months_landing_in_a_clock_gap_move_to_the_first_valid_hour() {
        // 29 September 2026 02:30, six months earlier is 29 March 02:30: it does not exist
        // (02:00 became 03:00), so the wall clock moves forward one hour to 03:30 local (UTC+2)
        // = 01:30 UTC, the instant 02:30 would have been under the old offset.
        assert_eq!(
            condition(EUROPE, "6m", "2026-09-29 02:30:00"),
            "datetime(played_at) >= datetime('2026-03-29 01:30:00')"
        );
    }

    #[test]
    fn rolling_months_landing_in_a_repeated_hour_take_the_first_occurrence() {
        // 25 April 2026 02:30 minus six months = 25 October 02:30, which happens twice; the
        // earlier reading (UTC+2) is 00:30 UTC.
        assert_eq!(
            condition(EUROPE, "6m", "2026-04-25 02:30:00"),
            "datetime(played_at) >= datetime('2025-10-25 00:30:00')"
        );
    }

    #[test]
    fn the_sql_is_built_from_re_formatted_values_only() {
        let now = local(EUROPE, "2026-06-15 12:00:00");
        for text in [
            "year:2026",
            "custom:2026-01-01,2026-01-31",
            "between:2026-09-21 00:00:00,2026-09-28 00:00:00",
            "3m",
        ] {
            let sql = range(text)
                .sql_condition_at("played_at", &now)
                .unwrap()
                .unwrap();
            assert!(
                sql.chars()
                    .all(|c| c.is_ascii_alphanumeric() || " ()'-:_,><=".contains(c)),
                "{sql}"
            );
            assert!(!sql.contains(';') && !sql.contains("--"), "{sql}");
        }
    }

    #[test]
    fn the_existing_presets_keep_their_sql() {
        let now = local(EUROPE, "2026-06-15 12:00:00");
        let sql = |text: &str| range(text).sql_condition_at("played_at", &now).unwrap();
        assert_eq!(
            sql("today").unwrap(),
            "datetime(played_at, 'localtime') >= datetime('now', 'localtime', 'start of day')"
        );
        assert_eq!(
            sql("7d").unwrap(),
            "datetime(played_at) >= datetime('now', '-7 days')"
        );
        assert_eq!(
            sql("30d").unwrap(),
            "datetime(played_at) >= datetime('now', '-30 days')"
        );
        assert_eq!(
            sql("year").unwrap(),
            "datetime(played_at, 'localtime') >= datetime('now', 'localtime', 'start of year')"
        );
        assert_eq!(sql("all"), None);
    }
}

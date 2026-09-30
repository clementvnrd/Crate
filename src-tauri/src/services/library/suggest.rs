//! Next-track suggestions learned from the transitions the user really played.
//!
//! The Rekordbox sets say which track followed which. After a given track, the library tracks
//! that most often came next in those sets come first; if fewer than asked for, the list is
//! completed with library tracks that mix well with it (a neighbouring key, a tempo within a
//! deck's pitch range).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::*;
use crate::services::harmonic::{
    bpm_delta_percent, relation, HarmonicRelation, PITCH_RANGE_PERCENT,
};

const DEFAULT_LIMIT: usize = 10;
const MAX_LIMIT: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionSource {
    /// Played after the current track in one of the user's sets.
    History,
    /// Never played after it, but harmonically and rhythmically compatible.
    Compatible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextTrackSuggestion {
    pub track: Track,
    pub source: SuggestionSource,
    /// How many times it followed the current track in a set (0 for `Compatible`).
    pub times_played_after: u32,
    /// When it last followed it, `YYYY-MM-DD HH:MM:SS` in UTC.
    pub last_played_after: Option<String>,
    pub harmonic: HarmonicRelation,
    /// Tempo gap with the current track in percent (positive = faster).
    pub bpm_delta_percent: Option<f64>,
}

impl LibraryService {
    /// Up to `limit` (10 by default, at most 50) tracks to play after `track_id`.
    pub fn suggest_next_tracks(
        &self,
        track_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<NextTrackSuggestion>> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let current = self.get_track(track_id)?;

        let mut suggestions = Vec::new();
        let mut taken: HashSet<String> = HashSet::from([current.id.clone()]);

        for (next_id, times, last) in self.transitions_after(&current)? {
            if suggestions.len() >= limit {
                break;
            }
            if !taken.insert(next_id.clone()) {
                continue;
            }
            if let Ok(track) = self.get_track(&next_id) {
                suggestions.push(describe(
                    &current,
                    track,
                    SuggestionSource::History,
                    times,
                    last,
                ));
            }
        }

        if suggestions.len() < limit {
            for next_id in self.compatible_ids(&current, &taken, limit - suggestions.len())? {
                if let Ok(track) = self.get_track(&next_id) {
                    suggestions.push(describe(
                        &current,
                        track,
                        SuggestionSource::Compatible,
                        0,
                        None,
                    ));
                }
            }
        }

        Ok(suggestions)
    }

    /// Library tracks that followed `current` in a Rekordbox set, most frequent first, then the
    /// most recent: `(library track id, times, last time)`. Sets are read one at a time, so the
    /// last track of a set is never "followed" by the first of the next. Tracks the library does
    /// not have are skipped. Names are compared case-insensitively, ignoring surrounding spaces
    /// (Rekordbox does not copy the tags byte for byte); SQLite lowercases both sides.
    fn transitions_after(&self, current: &Track) -> Result<Vec<(String, u32, Option<String>)>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            r#"
            WITH seq AS (
                SELECT lower(trim(title)) AS title,
                       lower(trim(artist)) AS artist,
                       LEAD(lower(trim(title))) OVER w AS next_title,
                       LEAD(lower(trim(artist))) OVER w AS next_artist,
                       LEAD(played_at) OVER w AS next_at
                FROM listen_events
                WHERE source = 'rekordbox' AND session_id IS NOT NULL
                WINDOW w AS (PARTITION BY session_id ORDER BY played_at, id)
            ),
            followed AS (
                SELECT next_title, next_artist, COUNT(*) AS times,
                       MAX(datetime(next_at)) AS last_at
                FROM seq
                WHERE title = lower(trim(?1)) AND artist = lower(trim(?2))
                  AND next_title IS NOT NULL
                GROUP BY next_title, next_artist
            )
            SELECT (SELECT t.id FROM tracks t
                    WHERE lower(trim(t.title)) = f.next_title
                      AND lower(trim(t.artist)) = f.next_artist
                    ORDER BY t.date_added ASC, t.id ASC LIMIT 1) AS track_id,
                   f.times,
                   f.last_at
            FROM followed f
            ORDER BY f.times DESC, f.last_at DESC
            "#,
        )?;
        let rows = stmt
            .query_map(rusqlite::params![current.title, current.artist], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows
            .into_iter()
            .filter_map(|(id, times, last)| Some((id?, times.max(0) as u32, last)))
            .collect())
    }

    /// Up to `want` library tracks that mix with `current`, best first: the same key, then a
    /// neighbouring or relative key, then (when the current key is unknown) tempo alone; within
    /// a level, the closest tempo. A clashing key or a tempo outside the pitch range is out.
    fn compatible_ids(
        &self,
        current: &Track,
        taken: &HashSet<String>,
        want: usize,
    ) -> Result<Vec<String>> {
        let Some(bpm) = current.bpm.filter(|b| *b > 0.0) else {
            return Ok(Vec::new());
        };
        let spread = bpm * PITCH_RANGE_PERCENT / 100.0;
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut stmt = conn
            .prepare("SELECT id, key, bpm FROM tracks WHERE id != ?1 AND bpm BETWEEN ?2 AND ?3")?;
        let candidates = stmt
            .query_map(
                rusqlite::params![current.id, bpm - spread, bpm + spread],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, f64>(2)?,
                    ))
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut ranked: Vec<(u8, f64, String)> = candidates
            .into_iter()
            .filter(|(id, _, _)| !taken.contains(id))
            .filter_map(|(id, key, candidate_bpm)| {
                let level = match relation(current.key.as_deref(), key.as_deref()) {
                    HarmonicRelation::Same => 0,
                    HarmonicRelation::Adjacent | HarmonicRelation::Relative => 1,
                    // Without a readable key on either side only the tempo can be judged.
                    HarmonicRelation::Unknown => 2,
                    HarmonicRelation::Clash => return None,
                };
                Some((level, (candidate_bpm - bpm).abs(), id))
            })
            .collect();
        ranked.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.1.total_cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        });
        Ok(ranked.into_iter().take(want).map(|(_, _, id)| id).collect())
    }
}

fn describe(
    current: &Track,
    next: Track,
    source: SuggestionSource,
    times_played_after: u32,
    last_played_after: Option<String>,
) -> NextTrackSuggestion {
    NextTrackSuggestion {
        harmonic: relation(current.key.as_deref(), next.key.as_deref()),
        bpm_delta_percent: bpm_delta_percent(current.bpm, next.bpm),
        track: next,
        source,
        times_played_after,
        last_played_after,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Fixture {
        service: LibraryService,
        conn: Arc<Mutex<rusqlite::Connection>>,
        dir: std::path::PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn library(name: &str) -> Fixture {
        let dir = std::env::temp_dir().join(format!("crate_suggest_{name}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        for (id, artist, title, key, bpm) in [
            ("cur", "Artist A", "Opener", Some("8A"), Some(124.0)),
            ("f1", "Artist B", "Follower One", Some("9A"), Some(126.0)),
            ("f2", "Artist C", "Follower Two", Some("3B"), Some(100.0)),
            ("same", "Artist D", "Same Key", Some("8A"), Some(125.0)),
            ("rel", "Artist E", "Relative", Some("8B"), Some(123.0)),
            ("fast", "Artist F", "Too Fast", Some("8A"), Some(140.0)),
            ("clash", "Artist G", "Clash", Some("2A"), Some(124.0)),
            ("nokey", "Artist H", "No Key", None, Some(124.5)),
        ] {
            conn.execute(
                "INSERT INTO tracks (id, file_path, format, title, artist, key, bpm, duration_ms, date_added, date_modified)
                 VALUES (?1, ?2, 'mp3', ?3, ?4, ?5, ?6, 200000, '2026-01-01', '2026-01-01')",
                rusqlite::params![id, format!("/m/{id}.mp3"), title, artist, key, bpm],
            )
            .unwrap();
        }
        let conn = Arc::new(Mutex::new(conn));
        let service = LibraryService::new(conn.clone(), dir.clone());
        Fixture { service, conn, dir }
    }

    /// A Rekordbox set: its tracks in order, one minute apart.
    fn set(f: &Fixture, session: &str, tracks: &[(&str, &str)]) {
        let conn = f.conn.lock().unwrap();
        for (i, (artist, title)) in tracks.iter().enumerate() {
            conn.execute(
                "INSERT INTO listen_events (id, source, title, artist, duration_ms, played_ms, played_at, session_id)
                 VALUES (?1, 'rekordbox', ?2, ?3, 200000, 200000, ?4, ?5)",
                rusqlite::params![
                    format!("{session}-{i}"),
                    title,
                    artist,
                    format!("2026-09-2{}T22:{:02}:00Z", session.len() % 9, i),
                    session
                ],
            )
            .unwrap();
        }
    }

    fn ids(suggestions: &[NextTrackSuggestion]) -> Vec<&str> {
        suggestions.iter().map(|s| s.track.id.as_str()).collect()
    }

    fn opener() -> (&'static str, &'static str) {
        ("Artist A", "Opener")
    }

    #[test]
    fn history_comes_first_most_played_then_compatible_fill() {
        let f = library("history");
        set(
            &f,
            "s1",
            &[
                opener(),
                ("Artist B", "Follower One"),
                ("Artist C", "Follower Two"),
            ],
        );
        set(&f, "s22", &[opener(), ("Artist B", "Follower One")]);
        set(&f, "s333", &[opener(), ("Artist B", "Follower One")]);
        set(
            &f,
            "s4444",
            &[
                ("Artist Z", "Somebody"),
                opener(),
                ("Artist C", "Follower Two"),
            ],
        );
        // A follower that the library does not have is skipped, not an error.
        set(&f, "s55555", &[opener(), ("Nobody", "Not In Library")]);

        let result = f.service.suggest_next_tracks("cur", Some(5)).unwrap();

        // f1 followed three times, f2 once; then the closest compatible: same key (125 bpm),
        // then the relative key (123 bpm), then the track with no key (tempo only).
        assert_eq!(ids(&result), ["f1", "f2", "same", "rel", "nokey"]);
        assert_eq!(result[0].source, SuggestionSource::History);
        assert_eq!(result[0].times_played_after, 3);
        assert_eq!(result[1].times_played_after, 1);
        assert_eq!(result[2].source, SuggestionSource::Compatible);
        assert_eq!(result[2].times_played_after, 0);
    }

    #[test]
    fn the_limit_cuts_the_list_and_compatible_tracks_only_fill_the_gap() {
        let f = library("limit");
        set(&f, "s1", &[opener(), ("Artist B", "Follower One")]);

        let one = f.service.suggest_next_tracks("cur", Some(1)).unwrap();
        assert_eq!(ids(&one), ["f1"], "history alone fills a limit of 1");

        let two = f.service.suggest_next_tracks("cur", Some(2)).unwrap();
        assert_eq!(ids(&two), ["f1", "same"]);
    }

    #[test]
    fn a_clashing_key_or_an_out_of_range_tempo_is_never_a_fallback() {
        let f = library("excluded");
        let result = f.service.suggest_next_tracks("cur", Some(50)).unwrap();
        let found = ids(&result);
        assert!(!found.contains(&"clash"), "2A against 8A clashes");
        assert!(
            !found.contains(&"fast"),
            "140 bpm is out of the pitch range of 124"
        );
        assert!(
            !found.contains(&"f2"),
            "100 bpm and a clashing key, never played after it"
        );
        assert!(!found.contains(&"cur"), "never the track itself");
        // Same key first, then the neighbours by tempo gap (relative 123, next key 126), then
        // the track whose key is unknown.
        assert_eq!(found, ["same", "rel", "f1", "nokey"]);
    }

    #[test]
    fn names_match_whatever_the_case_or_stray_spaces() {
        let f = library("names");
        set(
            &f,
            "s1",
            &[("  artist a ", " OPENER "), (" ARTIST B", "follower one  ")],
        );
        let result = f.service.suggest_next_tracks("cur", Some(1)).unwrap();
        assert_eq!(ids(&result), ["f1"]);
    }

    #[test]
    fn sets_do_not_leak_into_each_other() {
        let f = library("sessions");
        // The Opener ends one set; Follower One starts another: no transition between them.
        set(&f, "end", &[("Artist C", "Follower Two"), opener()]);
        set(
            &f,
            "start",
            &[("Artist B", "Follower One"), ("Artist C", "Follower Two")],
        );
        let result = f.service.suggest_next_tracks("cur", Some(1)).unwrap();
        assert_eq!(
            result[0].source,
            SuggestionSource::Compatible,
            "no history after the Opener"
        );
    }

    #[test]
    fn each_suggestion_says_how_it_mixes() {
        let f = library("describe");
        set(&f, "s1", &[opener(), ("Artist B", "Follower One")]);
        let f1 = &f.service.suggest_next_tracks("cur", Some(1)).unwrap()[0];
        assert_eq!(f1.harmonic, HarmonicRelation::Adjacent, "8A to 9A");
        let delta = f1.bpm_delta_percent.unwrap();
        assert!(
            (delta - 1.6129).abs() < 0.001,
            "126 against 124, got {delta}"
        );
        assert!(f1.last_played_after.is_some());
    }

    #[test]
    fn a_track_without_a_tempo_gets_history_but_no_fallback() {
        let f = library("notempo");
        f.conn
            .lock()
            .unwrap()
            .execute("UPDATE tracks SET bpm = NULL WHERE id = 'cur'", [])
            .unwrap();
        set(&f, "s1", &[opener(), ("Artist B", "Follower One")]);
        let result = f.service.suggest_next_tracks("cur", Some(10)).unwrap();
        assert_eq!(ids(&result), ["f1"]);
    }

    #[test]
    fn an_unknown_track_is_an_error() {
        let f = library("unknown");
        assert!(f.service.suggest_next_tracks("missing", None).is_err());
    }
}

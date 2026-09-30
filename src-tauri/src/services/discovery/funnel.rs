//! What the user's discoveries become: found, added to the library, played in a set.
//!
//! Releases come from Bandcamp, SoundCloud, Beatport and other pages the user scanned. Nothing
//! records which library track a release turned into, so the stages are matched by names: a
//! release is **in the library** when a library track has the same artist and either the
//! release's title (as the track title or the album) or one of the release's track names; it is
//! **played in a set** when such a track appears in a Rekordbox set. Names are compared ignoring
//! case and surrounding spaces, and only exact matches count: "Artist A & B" does not match
//! "Artist A".

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::DiscoveryService;
use crate::error::{CrateError, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunnelStages {
    pub discovered: usize,
    pub in_library: usize,
    pub played_in_set: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunnelSource {
    pub source_type: String,
    pub stages: FunnelStages,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryFunnel {
    /// Only releases added on or after this day (`YYYY-MM-DD`) were counted, if set.
    pub since: Option<String>,
    pub total: FunnelStages,
    /// By source, the most discovered first.
    pub by_source: Vec<FunnelSource>,
}

impl DiscoveryService {
    /// The funnel for every release, or for those added on or after `since` (`YYYY-MM-DD`).
    pub fn get_funnel(&self, since: Option<&str>) -> Result<DiscoveryFunnel> {
        let since = match since.map(str::trim).filter(|s| !s.is_empty()) {
            Some(day) => {
                NaiveDate::parse_from_str(day, "%Y-%m-%d").map_err(|_| {
                    CrateError::InvalidOperation("the start day must be YYYY-MM-DD".to_string())
                })?;
                Some(day.to_string())
            }
            None => None,
        };

        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            r#"
            WITH scoped AS (
                SELECT id, source_type, artist, title
                FROM discovery_releases
                WHERE ?1 IS NULL OR substr(date_added, 1, 10) >= ?1
            ),
            matched AS (
                SELECT DISTINCT s.id AS release_id, t.artist AS t_artist, t.title AS t_title
                FROM scoped s
                JOIN tracks t ON lower(trim(t.artist)) = lower(trim(s.artist))
                WHERE (trim(COALESCE(s.title, '')) != ''
                       AND (lower(trim(t.album)) = lower(trim(s.title))
                            OR lower(trim(t.title)) = lower(trim(s.title))))
                   OR lower(trim(t.title)) IN (
                        SELECT lower(trim(dt.name)) FROM discovery_tracks dt
                        WHERE dt.release_id = s.id AND trim(dt.name) != '')
            ),
            in_set AS (
                SELECT DISTINCT m.release_id FROM matched m
                WHERE EXISTS (
                    SELECT 1 FROM listen_events le
                    WHERE le.source = 'rekordbox'
                      AND lower(trim(le.artist)) = lower(trim(m.t_artist))
                      AND lower(trim(le.title)) = lower(trim(m.t_title)))
            )
            SELECT s.source_type,
                   COUNT(*),
                   COALESCE(SUM(s.id IN (SELECT release_id FROM matched)), 0),
                   COALESCE(SUM(s.id IN (SELECT release_id FROM in_set)), 0)
            FROM scoped s
            GROUP BY s.source_type
            ORDER BY COUNT(*) DESC, s.source_type ASC
            "#,
        )?;
        let by_source: Vec<FunnelSource> = stmt
            .query_map([since.as_deref()], |r| {
                Ok(FunnelSource {
                    source_type: r.get(0)?,
                    stages: FunnelStages {
                        discovered: r.get::<_, i64>(1)?.max(0) as usize,
                        in_library: r.get::<_, i64>(2)?.max(0) as usize,
                        played_in_set: r.get::<_, i64>(3)?.max(0) as usize,
                    },
                })
            })?
            .collect::<std::result::Result<_, _>>()?;

        let total = by_source
            .iter()
            .fold(FunnelStages::default(), |mut sum, s| {
                sum.discovered += s.stages.discovered;
                sum.in_library += s.stages.in_library;
                sum.played_in_set += s.stages.played_in_set;
                sum
            });

        Ok(DiscoveryFunnel {
            since,
            total,
            by_source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Fixture {
        service: DiscoveryService,
        conn: Arc<Mutex<rusqlite::Connection>>,
        dir: std::path::PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn fixture(name: &str) -> Fixture {
        let dir = std::env::temp_dir().join(format!("crate_funnel_{name}_{}", std::process::id()));
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let service = DiscoveryService::new(conn.clone(), dir.clone());
        Fixture { service, conn, dir }
    }

    fn release(
        f: &Fixture,
        id: &str,
        source: &str,
        artist: Option<&str>,
        title: &str,
        added: &str,
    ) {
        f.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO discovery_releases (id, url, source_type, artist, title, date_added, date_modified)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                rusqlite::params![id, format!("https://x/{id}"), source, artist, title, added],
            )
            .unwrap();
    }

    fn release_track(f: &Fixture, release: &str, name: &str) {
        f.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO discovery_tracks (id, release_id, name, position) VALUES (?1, ?2, ?3, 1)",
                rusqlite::params![format!("dt-{release}-{name}"), release, name],
            )
            .unwrap();
    }

    fn library_track(f: &Fixture, id: &str, artist: &str, title: &str, album: Option<&str>) {
        f.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO tracks (id, file_path, format, title, artist, album, duration_ms, date_added, date_modified)
                 VALUES (?1, ?2, 'mp3', ?3, ?4, ?5, 200000, '2026-01-01', '2026-01-01')",
                rusqlite::params![id, format!("/m/{id}.mp3"), title, artist, album],
            )
            .unwrap();
    }

    fn played_in_a_set(f: &Fixture, artist: &str, title: &str) {
        f.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO listen_events (id, source, title, artist, duration_ms, played_ms, played_at, session_id)
                 VALUES (?1, 'rekordbox', ?2, ?3, 200000, 200000, '2026-09-20T22:00:00Z', 's1')",
                rusqlite::params![format!("e-{artist}-{title}"), title, artist],
            )
            .unwrap();
    }

    fn stages(d: usize, l: usize, s: usize) -> FunnelStages {
        FunnelStages {
            discovered: d,
            in_library: l,
            played_in_set: s,
        }
    }

    fn seeded(name: &str) -> Fixture {
        let f = fixture(name);
        // Matched by the release title used as the album, then played in a set.
        release(
            &f,
            "r1",
            "bandcamp",
            Some("Artist A"),
            "EP One",
            "2026-09-10T10:00:00Z",
        );
        library_track(&f, "t1", "Artist A", "Track 1", Some("EP One"));
        played_in_a_set(&f, "Artist A", "Track 1");
        // In the library as a single, never played in a set.
        release(
            &f,
            "r2",
            "bandcamp",
            Some("Artist B"),
            "Single",
            "2026-09-11T10:00:00Z",
        );
        library_track(&f, "t2", "Artist B", "Single", None);
        // Matched through the name of one of the release's own tracks.
        release(
            &f,
            "r3",
            "soundcloud",
            Some("Artist C"),
            "Mix",
            "2026-09-12T10:00:00Z",
        );
        release_track(&f, "r3", "Deep Cut");
        library_track(&f, "t3", "Artist C", "Deep Cut", Some("Other Album"));
        // Nothing in the library.
        release(
            &f,
            "r4",
            "soundcloud",
            Some("Artist D"),
            "Nope",
            "2026-09-13T10:00:00Z",
        );
        // No artist at all: discovered, can never be matched.
        release(
            &f,
            "r5",
            "beatport",
            None,
            "Mystery",
            "2026-09-14T10:00:00Z",
        );
        f
    }

    #[test]
    fn counts_each_stage_by_source_and_in_total() {
        let f = seeded("stages");
        let funnel = f.service.get_funnel(None).unwrap();

        assert_eq!(funnel.total, stages(5, 3, 1));
        let sources: Vec<(&str, &FunnelStages)> = funnel
            .by_source
            .iter()
            .map(|s| (s.source_type.as_str(), &s.stages))
            .collect();
        assert_eq!(
            sources,
            [
                ("bandcamp", &stages(2, 2, 1)),
                ("soundcloud", &stages(2, 1, 0)),
                ("beatport", &stages(1, 0, 0)),
            ],
            "most discovered first, ties by name"
        );
    }

    #[test]
    fn a_start_day_keeps_only_later_discoveries() {
        let f = seeded("since");
        let funnel = f.service.get_funnel(Some("2026-09-12")).unwrap();
        assert_eq!(funnel.since.as_deref(), Some("2026-09-12"));
        // r1 and r2 are older; r3 (in library), r4, r5 remain.
        assert_eq!(funnel.total, stages(3, 1, 0));
    }

    #[test]
    fn names_match_whatever_the_case_or_stray_spaces() {
        let f = fixture("case");
        release(
            &f,
            "r",
            "bandcamp",
            Some("  ARTIST a "),
            " ep one",
            "2026-09-10T10:00:00Z",
        );
        library_track(&f, "t", "artist A", "Whatever", Some("EP ONE  "));
        assert_eq!(f.service.get_funnel(None).unwrap().total, stages(1, 1, 0));
    }

    #[test]
    fn an_empty_title_or_album_never_creates_a_match() {
        let f = fixture("empty");
        release(
            &f,
            "r",
            "bandcamp",
            Some("Artist E"),
            "",
            "2026-09-10T10:00:00Z",
        );
        library_track(&f, "t", "Artist E", "Some Track", Some(""));
        assert_eq!(f.service.get_funnel(None).unwrap().total, stages(1, 0, 0));
    }

    #[test]
    fn a_similar_artist_name_is_not_a_match() {
        let f = fixture("artist");
        release(
            &f,
            "r",
            "bandcamp",
            Some("Artist A"),
            "EP One",
            "2026-09-10T10:00:00Z",
        );
        library_track(&f, "t", "Artist A & Friend", "Track", Some("EP One"));
        assert_eq!(f.service.get_funnel(None).unwrap().total, stages(1, 0, 0));
    }

    #[test]
    fn an_empty_library_gives_zeros_and_a_bad_start_day_is_an_error() {
        let f = fixture("none");
        let funnel = f.service.get_funnel(None).unwrap();
        assert_eq!(funnel.total, stages(0, 0, 0));
        assert!(funnel.by_source.is_empty());
        assert!(f.service.get_funnel(Some("last week")).is_err());
        assert!(f.service.get_funnel(Some("2026-13-40")).is_err());
        assert!(
            f.service.get_funnel(Some("  ")).is_ok(),
            "blank means no start day"
        );
    }
}

//! The timeline of one Rekordbox set: its tracks in order, the key and energy of each, and how
//! each transition from the previous track mixes. Review a set like a match, learn from it.

use serde::{Deserialize, Serialize};

use super::StatsRecorderService;
use crate::error::{CrateError, Result};
use crate::models::stats::RekordboxSession;
use crate::services::harmonic::{bpm_delta_percent, relation, HarmonicRelation};

/// How a track was mixed in from the one before it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionTransition {
    pub harmonic: HarmonicRelation,
    /// Tempo change in percent (positive = faster).
    pub bpm_delta_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTrack {
    /// 1-based position in the set.
    pub position: usize,
    pub played_at: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: u64,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    /// Energy 1 to 10, known when the track is in the library and analysed.
    pub energy: Option<i32>,
    /// The library track this one is, matched by artist and title.
    pub library_track_id: Option<String>,
    /// `None` for the first track.
    pub from_previous: Option<SessionTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTimeline {
    pub session: RekordboxSession,
    pub tracks: Vec<SessionTrack>,
    /// Transitions that keep the key (same, neighbouring or relative).
    pub harmonic_transitions: usize,
    /// Transitions between clashing keys.
    pub clashing_transitions: usize,
    /// Transitions where a key is missing on either side.
    pub unknown_transitions: usize,
}

impl StatsRecorderService {
    /// The tracks of a Rekordbox set. Key, tempo and energy come from the matching library track
    /// when there is one (Mixed In Key's analysis, the same notation for the whole set), else from
    /// what Rekordbox recorded; Rekordbox never records an energy.
    pub fn get_session_timeline(&self, session_id: &str) -> Result<SessionTimeline> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let session = conn
            .query_row(
                "SELECT id, session_name, started_at, ended_at, total_tracks, total_played_ms
                 FROM rekordbox_sessions WHERE id = ?1",
                [session_id],
                |r| {
                    Ok(RekordboxSession {
                        id: r.get(0)?,
                        session_name: r.get(1)?,
                        started_at: r.get(2)?,
                        ended_at: r.get(3)?,
                        total_tracks: r.get::<_, i64>(4)?.max(0) as usize,
                        total_played_ms: r.get::<_, i64>(5)?.max(0) as u64,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    CrateError::InvalidOperation(format!("unknown Rekordbox session: {session_id}"))
                }
                other => CrateError::Database(other),
            })?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT le.played_at, le.title, le.artist, le.album, le.duration_ms,
                       COALESCE(t.bpm, le.bpm), COALESCE(t.key, le.key), t.energy, t.id
                FROM listen_events le
                LEFT JOIN tracks t ON t.id = (
                    SELECT t2.id FROM tracks t2
                    WHERE lower(trim(t2.title)) = lower(trim(le.title))
                      AND lower(trim(t2.artist)) = lower(trim(le.artist))
                    ORDER BY t2.date_added ASC, t2.id ASC LIMIT 1)
                WHERE le.session_id = ?1 AND le.source = 'rekordbox'
                ORDER BY le.played_at ASC, le.id ASC
                "#,
            )
            .map_err(CrateError::Database)?;
        let rows = stmt
            .query_map([session_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, Option<f64>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, Option<i32>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                ))
            })
            .map_err(CrateError::Database)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(CrateError::Database)?;

        let mut tracks: Vec<SessionTrack> = Vec::with_capacity(rows.len());
        let (mut harmonic, mut clashing, mut unknown) = (0, 0, 0);
        for (i, (played_at, title, artist, album, duration_ms, bpm, key, energy, library_id)) in
            rows.into_iter().enumerate()
        {
            let from_previous = tracks.last().map(|previous| {
                let harmonic_relation = relation(previous.key.as_deref(), key.as_deref());
                match harmonic_relation {
                    HarmonicRelation::Clash => clashing += 1,
                    HarmonicRelation::Unknown => unknown += 1,
                    _ => harmonic += 1,
                }
                SessionTransition {
                    harmonic: harmonic_relation,
                    bpm_delta_percent: bpm_delta_percent(previous.bpm, bpm),
                }
            });
            tracks.push(SessionTrack {
                position: i + 1,
                played_at,
                title,
                artist,
                album,
                duration_ms: duration_ms.max(0) as u64,
                bpm,
                key,
                energy,
                library_track_id: library_id,
                from_previous,
            });
        }

        Ok(SessionTimeline {
            session,
            tracks,
            harmonic_transitions: harmonic,
            clashing_transitions: clashing,
            unknown_transitions: unknown,
        })
    }
}

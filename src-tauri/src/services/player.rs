use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

use crate::models::stats::ListenEvent;
use crate::services::stats::recorder::StatsRecorderService;

/// A play counts as a listen after 30 seconds of actual playback.
const LISTEN_THRESHOLD_MS: u64 = 30_000;

/// How long a track of `duration_ms` lasts at the slowest tempo of the player's control (−10 %: 10/9 of its length).
fn longest_play_ms(duration_ms: u64) -> u64 {
    duration_ms.saturating_mul(10) / 9
}

#[derive(Debug, Clone)]
pub struct TrackPlayingContext {
    pub track_id: Option<String>,
    pub source: String, // 'crate_local', 'crate_beatport', 'crate_standalone'
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: u64,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub energy: Option<i32>,
    pub format: Option<String>,
    pub artwork_url: Option<String>,
    pub started_at: String,
}

/// Playback bookkeeping for the current track: only time spent actually playing is counted
/// (pauses and seeks are not), and the stored listen is updated with the final total.
struct PlaySession {
    ctx: TrackPlayingContext,
    listened_ms: u64,
    playing_since: Option<Instant>,
    recorded_event_id: Option<String>,
}

impl PlaySession {
    /// Time spent playing, measured on the wall clock between start / resume and pause. The tracker only learns
    /// that a track ran out by itself when a state poll sees the engine stopped; when none comes (continuous
    /// playback off, or the queue gave up on files that fail to load), the clock would keep running until the next
    /// start or stop. So a play never counts more than the track lasts at the slowest tempo the player offers.
    fn total_ms(&self, now: Instant) -> u64 {
        let total = self.listened_ms
            + self.playing_since.map_or(0, |since| {
                now.saturating_duration_since(since).as_millis() as u64
            });
        match self.ctx.duration_ms {
            0 => total,
            duration => total.min(longest_play_ms(duration)),
        }
    }

    fn pause(&mut self, now: Instant) {
        self.listened_ms = self.total_ms(now);
        self.playing_since = None;
    }

    fn event(&self, played_ms: u64) -> ListenEvent {
        let ctx = &self.ctx;
        ListenEvent {
            id: Uuid::new_v4().to_string(),
            source: ctx.source.clone(),
            track_id: ctx.track_id.clone(),
            title: ctx.title.clone(),
            artist: ctx.artist.clone(),
            album: ctx.album.clone(),
            duration_ms: ctx.duration_ms,
            played_ms,
            bpm: ctx.bpm,
            key: ctx.key.clone(),
            energy: ctx.energy,
            format: ctx.format.clone(),
            artwork_url: ctx.artwork_url.clone(),
            played_at: ctx.started_at.clone(),
            session_id: None,
            metadata_json: None,
        }
    }
}

pub struct PlayerTrackerService {
    recorder: Arc<StatsRecorderService>,
    current: Arc<Mutex<Option<PlaySession>>>,
}

impl PlayerTrackerService {
    pub fn new(recorder: Arc<StatsRecorderService>) -> Self {
        Self {
            recorder,
            current: Arc::new(Mutex::new(None)),
        }
    }

    /// Called when starting playback of a track: closes the previous one.
    pub fn on_track_started(&self, ctx: TrackPlayingContext) {
        self.track_started_at(ctx, Instant::now());
    }

    /// Called on pause: playback time stops accumulating.
    pub fn on_paused(&self) {
        self.paused_at(Instant::now());
    }

    /// Called on resume: playback time accumulates again.
    pub fn on_resumed(&self) {
        if let Ok(mut lock) = self.current.lock() {
            if let Some(session) = lock.as_mut() {
                session.playing_since.get_or_insert_with(Instant::now);
            }
        }
    }

    /// Called from the playback-state poll: records the listen once 30 s have really been played,
    /// and treats a track that stopped on its own as paused.
    pub fn check_and_record_if_due(&self, is_playing: bool) {
        self.check_at(is_playing, Instant::now());
    }

    /// Called when playback is stopped: closes the current track.
    pub fn on_playback_stopped(&self) {
        self.stopped_at(Instant::now());
    }

    fn track_started_at(&self, ctx: TrackPlayingContext, now: Instant) {
        let Ok(mut lock) = self.current.lock() else {
            return;
        };
        if let Some(previous) = lock.take() {
            self.finish(previous, now);
        }
        *lock = Some(PlaySession {
            ctx,
            listened_ms: 0,
            playing_since: Some(now),
            recorded_event_id: None,
        });
    }

    fn paused_at(&self, now: Instant) {
        let Ok(mut lock) = self.current.lock() else {
            return;
        };
        if let Some(session) = lock.as_mut() {
            session.pause(now);
            self.record_if_due(session, now);
        }
    }

    fn check_at(&self, is_playing: bool, now: Instant) {
        let Ok(mut lock) = self.current.lock() else {
            return;
        };
        if let Some(session) = lock.as_mut() {
            if !is_playing {
                session.pause(now);
            }
            self.record_if_due(session, now);
        }
    }

    fn stopped_at(&self, now: Instant) {
        let Ok(mut lock) = self.current.lock() else {
            return;
        };
        if let Some(session) = lock.take() {
            self.finish(session, now);
        }
    }

    fn record_if_due(&self, session: &mut PlaySession, now: Instant) {
        let total = session.total_ms(now);
        if session.recorded_event_id.is_none() && total >= LISTEN_THRESHOLD_MS {
            let event = session.event(total);
            if let Ok(true) = self.recorder.record_listen_event(&event) {
                log::info!(
                    "Recorded listen for '{}' by '{}' on {}",
                    session.ctx.title,
                    session.ctx.artist,
                    session.ctx.source
                );
                session.recorded_event_id = Some(event.id);
            }
        }
    }

    /// Closes a track: updates the stored listen with the full listened time, or records a short
    /// play (at least 1 s) that never reached the threshold.
    fn finish(&self, mut session: PlaySession, now: Instant) {
        session.pause(now);
        let total = session.listened_ms;
        match &session.recorded_event_id {
            Some(id) => {
                let _ = self.recorder.update_listen_played_ms(id, total);
            }
            None if total >= 1_000 => {
                let _ = self.recorder.record_listen_event(&session.event(total));
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn tracker() -> (PlayerTrackerService, Arc<Mutex<rusqlite::Connection>>) {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let recorder = Arc::new(StatsRecorderService::new(conn.clone()));
        (PlayerTrackerService::new(recorder), conn)
    }

    fn ctx(title: &str, _start: Instant) -> TrackPlayingContext {
        TrackPlayingContext {
            track_id: Some(format!("id-{title}")),
            source: "crate_local".into(),
            title: title.into(),
            artist: "Artist".into(),
            album: None,
            duration_ms: 300_000,
            bpm: None,
            key: None,
            energy: None,
            format: None,
            artwork_url: None,
            started_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn listens(conn: &Arc<Mutex<rusqlite::Connection>>) -> Vec<(String, i64)> {
        let conn = conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT title, played_ms FROM listen_events ORDER BY title")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .flatten()
            .collect()
    }

    #[test]
    fn test_pauses_are_not_counted() {
        let (tracker, conn) = tracker();
        let t0 = Instant::now();
        tracker.track_started_at(ctx("Song", t0), t0);
        tracker.paused_at(t0 + Duration::from_secs(20));
        // Paused for ten minutes, then 15 s more of playback
        tracker.on_resumed();
        {
            let mut lock = tracker.current.lock().unwrap();
            lock.as_mut().unwrap().playing_since = Some(t0 + Duration::from_secs(620));
        }
        tracker.check_at(true, t0 + Duration::from_secs(635));
        tracker.stopped_at(t0 + Duration::from_secs(635));
        assert_eq!(listens(&conn), vec![("Song".to_string(), 35_000)]);
    }

    #[test]
    fn test_listen_is_updated_with_the_full_time() {
        let (tracker, conn) = tracker();
        let t0 = Instant::now();
        tracker.track_started_at(ctx("Long", t0), t0);
        tracker.check_at(true, t0 + Duration::from_secs(31));
        tracker.track_started_at(
            ctx("Next", t0 + Duration::from_secs(240)),
            t0 + Duration::from_secs(240),
        );
        assert_eq!(
            listens(&conn),
            vec![("Long".to_string(), 240_000)],
            "one row, with the 4 minutes played"
        );
    }

    #[test]
    fn test_track_that_ended_on_its_own_stops_counting() {
        let (tracker, conn) = tracker();
        let t0 = Instant::now();
        tracker.track_started_at(ctx("Ended", t0), t0);
        tracker.check_at(false, t0 + Duration::from_secs(60));
        tracker.stopped_at(t0 + Duration::from_secs(3600));
        assert_eq!(listens(&conn), vec![("Ended".to_string(), 60_000)]);
    }

    #[test]
    fn test_track_that_ran_out_unseen_counts_at_most_its_length() {
        // No state poll saw the end (the queue gave up on missing files after it): the session stays open until
        // the next start, an hour later. The play is capped at the track's length at the slowest tempo.
        let (tracker, conn) = tracker();
        let t0 = Instant::now();
        tracker.track_started_at(ctx("Unseen end", t0), t0);
        tracker.track_started_at(
            ctx("Hour later", t0 + Duration::from_secs(3600)),
            t0 + Duration::from_secs(3600),
        );
        tracker.stopped_at(t0 + Duration::from_secs(3601));
        assert_eq!(
            listens(&conn),
            vec![
                ("Hour later".to_string(), 1_000),
                ("Unseen end".to_string(), 333_333)
            ]
        );
    }

    #[test]
    fn test_slowed_down_play_is_not_cut_short() {
        // 300 s track played to the end at −10 %: 333 s of real listening, all counted.
        let (tracker, conn) = tracker();
        let t0 = Instant::now();
        tracker.track_started_at(ctx("Slow", t0), t0);
        tracker.check_at(false, t0 + Duration::from_millis(333_000));
        tracker.stopped_at(t0 + Duration::from_secs(400));
        assert_eq!(listens(&conn), vec![("Slow".to_string(), 333_000)]);
    }
}

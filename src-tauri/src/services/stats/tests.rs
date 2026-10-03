use chrono::{Duration, Utc};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use crate::db::schema::get_migrations;
use crate::models::stats::ListenEvent;
use crate::services::stats::{
    RekordboxTrackerService, SpotifyTrackerService, StatsRecorderService,
};

/// Helper to setup an in-memory database with all migrations applied
fn setup_test_db() -> (Arc<Mutex<Connection>>, Arc<StatsRecorderService>) {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for migration in get_migrations() {
        conn.execute_batch(migration).unwrap();
    }
    let conn_arc = Arc::new(Mutex::new(conn));
    let recorder = Arc::new(StatsRecorderService::new(conn_arc.clone()));
    (conn_arc, recorder)
}

#[test]
fn test_migration_13_tables_exist() {
    let (conn_arc, _) = setup_test_db();
    let conn = conn_arc.lock().unwrap();

    // Verify listen_events table
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='listen_events'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "listen_events table should exist");

    // Verify spotify_auth table
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='spotify_auth'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "spotify_auth table should exist");

    // Verify rekordbox_sessions table
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='rekordbox_sessions'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "rekordbox_sessions table should exist");
}

#[test]
fn test_record_listen_event_validation_and_thresholds() {
    let (_, recorder) = setup_test_db();

    // 1. Invalid: Empty title or artist
    let empty_title_event = ListenEvent {
        id: "e1".to_string(),
        source: "crate_local".to_string(),
        track_id: None,
        title: "   ".to_string(),
        artist: "Artist 1".to_string(),
        album: None,
        duration_ms: 180000,
        played_ms: 180000,
        bpm: Some(124.0),
        key: Some("8A".to_string()),
        energy: Some(6),
        format: Some("flac".to_string()),
        artwork_url: None,
        played_at: Utc::now().to_rfc3339(),
        session_id: None,
        metadata_json: None,
    };
    assert!(recorder.record_listen_event(&empty_title_event).is_err());

    // 2. Below 1s threshold: should return Ok(false) and not insert
    let sub_second_event = ListenEvent {
        id: "e2".to_string(),
        source: "crate_local".to_string(),
        track_id: None,
        title: "Sub Second Snippet".to_string(),
        artist: "Artist 1".to_string(),
        album: None,
        duration_ms: 180000,
        played_ms: 500, // 500 ms < 1s
        bpm: Some(124.0),
        key: Some("8A".to_string()),
        energy: Some(6),
        format: Some("mp3".to_string()),
        artwork_url: None,
        played_at: Utc::now().to_rfc3339(),
        session_id: None,
        metadata_json: None,
    };
    assert_eq!(
        recorder.record_listen_event(&sub_second_event).unwrap(),
        false
    );
    assert_eq!(recorder.get_recent_listens(10).unwrap().len(), 0);

    // 3. Valid event >= 1s (e.g. 12 seconds) is accepted and recorded
    let short_event = ListenEvent {
        id: "e3".to_string(),
        source: "crate_local".to_string(),
        track_id: None,
        title: "Short Snippet".to_string(),
        artist: "Artist 1".to_string(),
        album: None,
        duration_ms: 180000,
        played_ms: 12000, // 12 seconds >= 1s
        bpm: Some(124.0),
        key: Some("8A".to_string()),
        energy: Some(6),
        format: Some("mp3".to_string()),
        artwork_url: None,
        played_at: Utc::now().to_rfc3339(),
        session_id: None,
        metadata_json: None,
    };
    assert_eq!(recorder.record_listen_event(&short_event).unwrap(), true);
    assert_eq!(recorder.get_recent_listens(10).unwrap().len(), 1);

    // 4. Valid full event >= 30s
    let valid_event = ListenEvent {
        id: "e4".to_string(),
        source: "crate_local".to_string(),
        track_id: Some("t1".to_string()),
        title: "Valid Song".to_string(),
        artist: "Artist 1".to_string(),
        album: Some("Album 1".to_string()),
        duration_ms: 240000,
        played_ms: 240000,
        bpm: Some(126.0),
        key: Some("8A".to_string()),
        energy: Some(7),
        format: Some("flac".to_string()),
        artwork_url: Some("/art/1.jpg".to_string()),
        played_at: (Utc::now() + Duration::seconds(20)).to_rfc3339(),
        session_id: None,
        metadata_json: None,
    };
    assert_eq!(recorder.record_listen_event(&valid_event).unwrap(), true);
    assert_eq!(recorder.get_recent_listens(10).unwrap().len(), 2);
}

#[test]
fn test_anti_duplicate_deduplication() {
    let (_, recorder) = setup_test_db();
    let now = Utc::now();

    let event1 = ListenEvent {
        id: "e1".to_string(),
        source: "spotify".to_string(),
        track_id: Some("spot_1".to_string()),
        title: "Breathe".to_string(),
        artist: "CamelPhat".to_string(),
        album: Some("Dark Matter".to_string()),
        duration_ms: 195000,
        played_ms: 195000,
        bpm: Some(125.0),
        key: Some("11A".to_string()),
        energy: Some(8),
        format: Some("spotify".to_string()),
        artwork_url: None,
        played_at: now.to_rfc3339(),
        session_id: None,
        metadata_json: None,
    };
    assert_eq!(recorder.record_listen_event(&event1).unwrap(), true);

    // Same track, artist, and source within 5 seconds -> duplicate!
    let duplicate_event = ListenEvent {
        id: "e2".to_string(),
        source: "spotify".to_string(),
        track_id: Some("spot_1".to_string()),
        title: "Breathe".to_string(),
        artist: "CamelPhat".to_string(),
        album: Some("Dark Matter".to_string()),
        duration_ms: 195000,
        played_ms: 195000,
        bpm: Some(125.0),
        key: Some("11A".to_string()),
        energy: Some(8),
        format: Some("spotify".to_string()),
        artwork_url: None,
        played_at: (now + Duration::seconds(5)).to_rfc3339(),
        session_id: None,
        metadata_json: None,
    };
    assert_eq!(
        recorder.record_listen_event(&duplicate_event).unwrap(),
        false
    );
    assert_eq!(recorder.get_recent_listens(10).unwrap().len(), 1);

    // Same track but played 30 minutes later -> valid!
    let later_event = ListenEvent {
        id: "e3".to_string(),
        source: "spotify".to_string(),
        track_id: Some("spot_1".to_string()),
        title: "Breathe".to_string(),
        artist: "CamelPhat".to_string(),
        album: Some("Dark Matter".to_string()),
        duration_ms: 195000,
        played_ms: 195000,
        bpm: Some(125.0),
        key: Some("11A".to_string()),
        energy: Some(8),
        format: Some("spotify".to_string()),
        artwork_url: None,
        played_at: (now + Duration::minutes(30)).to_rfc3339(),
        session_id: None,
        metadata_json: None,
    };
    assert_eq!(recorder.record_listen_event(&later_event).unwrap(), true);
    assert_eq!(recorder.get_recent_listens(10).unwrap().len(), 2);
}

#[test]
fn test_stats_summary_and_source_breakdown() {
    let (_, recorder) = setup_test_db();
    let now = Utc::now();

    // Event 1: Spotify, 120s (2 mins)
    recorder
        .record_listen_event(&ListenEvent {
            id: "s1".to_string(),
            source: "spotify".to_string(),
            track_id: None,
            title: "Track A".to_string(),
            artist: "Artist 1".to_string(),
            album: None,
            duration_ms: 120000,
            played_ms: 120000,
            bpm: Some(124.0),
            key: Some("8A".to_string()),
            energy: None,
            format: None,
            artwork_url: None,
            played_at: now.to_rfc3339(),
            session_id: None,
            metadata_json: None,
        })
        .unwrap();

    // Event 2: Crate Local, 180s (3 mins)
    recorder
        .record_listen_event(&ListenEvent {
            id: "l1".to_string(),
            source: "crate_local".to_string(),
            track_id: None,
            title: "Track B".to_string(),
            artist: "Artist 2".to_string(),
            album: None,
            duration_ms: 180000,
            played_ms: 180000,
            bpm: Some(128.0),
            key: Some("11B".to_string()),
            energy: None,
            format: None,
            artwork_url: None,
            played_at: (now + Duration::minutes(10)).to_rfc3339(),
            session_id: None,
            metadata_json: None,
        })
        .unwrap();

    // Event 3: Rekordbox, 300s (5 mins)
    recorder
        .record_listen_event(&ListenEvent {
            id: "r1".to_string(),
            source: "rekordbox".to_string(),
            track_id: None,
            title: "Track C".to_string(),
            artist: "Artist 1".to_string(),
            album: None,
            duration_ms: 300000,
            played_ms: 300000,
            bpm: Some(130.0),
            key: Some("8A".to_string()),
            energy: None,
            format: None,
            artwork_url: None,
            played_at: (now + Duration::minutes(20)).to_rfc3339(),
            session_id: None,
            metadata_json: None,
        })
        .unwrap();

    let summary = recorder.get_stats_summary("all").unwrap();
    assert_eq!(summary.total_plays, 3);
    assert_eq!(summary.total_minutes, 10); // 2 + 3 + 5 = 10 mins
    assert_eq!(summary.today_minutes, 10);
    assert_eq!(*summary.source_breakdown.get("spotify").unwrap(), 2);
    assert_eq!(*summary.source_breakdown.get("crate_local").unwrap(), 3);
    assert_eq!(*summary.source_breakdown.get("rekordbox").unwrap(), 5);
}

#[test]
fn test_top_tracks_and_top_artists() {
    let (_, recorder) = setup_test_db();
    let now = Utc::now();

    // Artist 1 - Track A played twice (1x spotify, 1x crate_local)
    recorder
        .record_listen_event(&ListenEvent {
            id: "t1".to_string(),
            source: "spotify".to_string(),
            track_id: None,
            title: "Song Alpha".to_string(),
            artist: "Super Artist".to_string(),
            album: Some("Greatest Hits".to_string()),
            duration_ms: 240000,
            played_ms: 240000,
            bpm: Some(125.0),
            key: Some("5A".to_string()),
            energy: Some(8),
            format: None,
            artwork_url: Some("/art/alpha.jpg".to_string()),
            played_at: now.to_rfc3339(),
            session_id: None,
            metadata_json: None,
        })
        .unwrap();

    recorder
        .record_listen_event(&ListenEvent {
            id: "t2".to_string(),
            source: "crate_local".to_string(),
            track_id: None,
            title: "Song Alpha".to_string(),
            artist: "Super Artist".to_string(),
            album: Some("Greatest Hits".to_string()),
            duration_ms: 240000,
            played_ms: 240000,
            bpm: Some(125.0),
            key: Some("5A".to_string()),
            energy: Some(8),
            format: None,
            artwork_url: Some("/art/alpha.jpg".to_string()),
            played_at: (now + Duration::hours(1)).to_rfc3339(),
            session_id: None,
            metadata_json: None,
        })
        .unwrap();

    // Artist 2 - Track B played once
    recorder
        .record_listen_event(&ListenEvent {
            id: "t3".to_string(),
            source: "crate_beatport".to_string(),
            track_id: None,
            title: "Song Beta".to_string(),
            artist: "Other Artist".to_string(),
            album: None,
            duration_ms: 180000,
            played_ms: 180000,
            bpm: Some(128.0),
            key: Some("9A".to_string()),
            energy: Some(6),
            format: None,
            artwork_url: None,
            played_at: (now + Duration::hours(2)).to_rfc3339(),
            session_id: None,
            metadata_json: None,
        })
        .unwrap();

    // Top Tracks check
    let top_tracks = recorder.get_top_tracks("all", 10).unwrap();
    assert_eq!(top_tracks.len(), 2);
    assert_eq!(top_tracks[0].title, "Song Alpha");
    assert_eq!(top_tracks[0].plays, 2);
    assert_eq!(top_tracks[0].total_minutes, 8); // 4 + 4 = 8 mins
    assert!(top_tracks[0].sources.contains(&"spotify".to_string()));
    assert!(top_tracks[0].sources.contains(&"crate_local".to_string()));

    // Top Artists check
    let top_artists = recorder.get_top_artists("all", 10).unwrap();
    assert_eq!(top_artists.len(), 2);
    assert_eq!(top_artists[0].artist, "Super Artist");
    assert_eq!(top_artists[0].plays, 2);
    assert_eq!(top_artists[0].total_minutes, 8);
    assert_eq!(top_artists[0].top_track.as_deref(), Some("Song Alpha"));
}

#[test]
fn test_harmonic_and_bpm_stats() {
    let (_, recorder) = setup_test_db();
    let now = Utc::now();

    // 8A (124 bpm), 8A (125 bpm), 11B (132 bpm), Unknown Key (95 bpm)
    let tracks = [
        ("T1", "A1", Some("8A"), Some(124.0), 180000),
        ("T2", "A1", Some("8A"), Some(125.0), 180000),
        ("T3", "A2", Some("11B"), Some(132.0), 240000),
        ("T4", "A3", None, Some(95.0), 120000),
    ];

    for (i, (title, artist, key, bpm, played_ms)) in tracks.iter().enumerate() {
        recorder
            .record_listen_event(&ListenEvent {
                id: format!("ev_{i}"),
                source: "crate_local".to_string(),
                track_id: None,
                title: title.to_string(),
                artist: artist.to_string(),
                album: None,
                duration_ms: *played_ms,
                played_ms: *played_ms,
                bpm: *bpm,
                key: key.map(|k| k.to_string()),
                energy: None,
                format: None,
                artwork_url: None,
                played_at: (now + Duration::minutes(i as i64 * 10)).to_rfc3339(),
                session_id: None,
                metadata_json: None,
            })
            .unwrap();
    }

    // Harmonic stats
    let harmonic = recorder.get_harmonic_stats("all").unwrap();
    assert_eq!(harmonic.len(), 2); // 8A and 11B
    assert_eq!(harmonic[0].key, "8A");
    assert_eq!(harmonic[0].plays, 2);
    // 2 out of 3 keyed plays = 66.67%
    assert!((harmonic[0].percentage - 66.67).abs() < 0.1);

    // BPM stats
    let bpm_stats = recorder.get_bpm_stats("all").unwrap();
    assert_eq!(bpm_stats.len(), 9);

    let under_100 = bpm_stats.iter().find(|b| b.bpm_range == "< 100").unwrap();
    assert_eq!(under_100.count, 1);

    let b_120_125 = bpm_stats.iter().find(|b| b.bpm_range == "120-125").unwrap();
    assert_eq!(b_120_125.count, 1);

    let b_125_130 = bpm_stats.iter().find(|b| b.bpm_range == "125-130").unwrap();
    assert_eq!(b_125_130.count, 1);

    let b_130_135 = bpm_stats.iter().find(|b| b.bpm_range == "130-135").unwrap();
    assert_eq!(b_130_135.count, 1);
}

#[test]
fn test_listening_heatmap() {
    let (_, recorder) = setup_test_db();

    // Insert listen event on a known date: 2026-09-01T14:30:00Z (Tuesday, Hour 14)
    // 2026-09-01 was a Tuesday (%w = 2)
    recorder
        .record_listen_event(&ListenEvent {
            id: "hm1".to_string(),
            source: "spotify".to_string(),
            track_id: None,
            title: "Afternoon Groove".to_string(),
            artist: "DJ Heat".to_string(),
            album: None,
            duration_ms: 300000,
            played_ms: 300000,
            bpm: Some(124.0),
            key: None,
            energy: None,
            format: None,
            artwork_url: None,
            played_at: "2026-09-01T14:30:00Z".to_string(),
            session_id: None,
            metadata_json: None,
        })
        .unwrap();

    let heatmap = recorder.get_listening_heatmap("all").unwrap();
    assert_eq!(heatmap.len(), 7 * 24); // Complete 7x24 grid

    // Compute expected day and hour in system local timezone
    let dt = chrono::DateTime::parse_from_rfc3339("2026-09-01T14:30:00Z")
        .unwrap()
        .with_timezone(&chrono::Local);
    let expected_day = dt.format("%w").to_string().parse::<u8>().unwrap();
    let expected_hour = dt.format("%H").to_string().parse::<u8>().unwrap();

    let cell = heatmap
        .iter()
        .find(|c| c.day_of_week == expected_day && c.hour_of_day == expected_hour)
        .unwrap();
    assert_eq!(cell.plays, 1);
    assert_eq!(cell.minutes, 5); // 300,000 ms = 5 mins
}

#[test]
fn test_spotify_archive_json_import_extended_history() {
    let (conn_arc, recorder) = setup_test_db();
    let spotify_svc = SpotifyTrackerService::new(conn_arc, recorder);

    let endsong_json = r#"[
        {
            "ts": "2026-08-15T10:00:00Z",
            "username": "user1",
            "platform": "macOS",
            "ms_played": 210000,
            "conn_country": "FR",
            "master_metadata_track_name": "Opus",
            "master_metadata_album_artist_name": "Eric Prydz",
            "master_metadata_album_album_name": "Opus",
            "spotify_track_uri": "spotify:track:12345",
            "reason_end": "trackdone"
        },
        {
            "ts": "2026-08-15T10:05:00Z",
            "username": "user1",
            "platform": "macOS",
            "ms_played": 10000,
            "conn_country": "FR",
            "master_metadata_track_name": "Skipped Song",
            "master_metadata_album_artist_name": "Random Artist",
            "master_metadata_album_album_name": null,
            "spotify_track_uri": "spotify:track:67890",
            "reason_end": "fwdbtn"
        },
        {
            "ts": "2026-08-15T10:10:00Z",
            "username": "user1",
            "platform": "macOS",
            "ms_played": 180000,
            "conn_country": "FR",
            "master_metadata_track_name": "Glue",
            "master_metadata_album_artist_name": "Bicep",
            "master_metadata_album_album_name": "Bicep",
            "spotify_track_uri": "spotify:track:11223",
            "reason_end": "trackdone"
        }
    ]"#;

    let res = spotify_svc
        .import_streaming_history_json(endsong_json)
        .unwrap();
    assert_eq!(res.imported_count, 2);
    assert_eq!(res.skipped_count, 1); // 10s track skipped (<30s)
    assert_eq!(res.total_minutes, 6); // 210s/60 = 3 mins + 180s/60 = 3 mins = 6 mins
}

#[test]
fn test_spotify_archive_json_import_simple_history() {
    let (conn_arc, recorder) = setup_test_db();
    let spotify_svc = SpotifyTrackerService::new(conn_arc, recorder);

    let simple_json = r#"[
        {
            "endTime": "2026-08-20 18:30",
            "artistName": "Tale of Us",
            "trackName": "Astral",
            "msPlayed": 360000
        },
        {
            "endTime": "2026-08-20 18:40",
            "artistName": "Maceo Plex",
            "trackName": "Conjure Dreams",
            "msPlayed": 300000
        }
    ]"#;

    let res = spotify_svc
        .import_streaming_history_json(simple_json)
        .unwrap();
    assert_eq!(res.imported_count, 2);
    assert_eq!(res.skipped_count, 0);
    assert_eq!(res.total_minutes, 11); // 6 + 5 = 11 mins
}

#[test]
fn test_rekordbox_history_xml_import() {
    let (conn_arc, recorder) = setup_test_db();
    let rekordbox_svc = RekordboxTrackerService::new(conn_arc, recorder);

    let rb_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <DJ_PLAYLISTS Version="1.0.0">
      <COLLECTION Entries="2">
        <TRACK TrackID="101" Name="Losing It" Artist="FISHER" Album="Losing It" TotalTime="248" AverageBpm="125.00" Tonality="8A" />
        <TRACK TrackID="102" Name="Cola" Artist="CamelPhat" Album="Cola" TotalTime="223" AverageBpm="122.00" Tonality="11A" />
      </COLLECTION>
      <PLAYLISTS>
        <NODE Type="0" Name="ROOT">
          <NODE Type="1" Name="HISTORY 2026-09-01" KeyType="0" Entries="2">
            <TRACK Key="101"/>
            <TRACK Key="102"/>
          </NODE>
        </NODE>
      </PLAYLISTS>
    </DJ_PLAYLISTS>"#;

    let count = rekordbox_svc.import_rekordbox_history_xml(rb_xml).unwrap();
    assert_eq!(count, 2);

    let sessions = rekordbox_svc.get_sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].total_tracks, 2);
}

#[test]
fn test_strict_counting_rules_1s_vs_30s_and_having_filter() {
    let (_, recorder) = setup_test_db();
    let now = Utc::now();

    // 1. Song Short: listened for 12 seconds (< 30s) -> 0 streams, but 12s recorded
    recorder
        .record_listen_event(&ListenEvent {
            id: "s1".to_string(),
            source: "crate_local".to_string(),
            track_id: None,
            title: "Short Track".to_string(),
            artist: "Short Artist".to_string(),
            album: None,
            duration_ms: 180000,
            played_ms: 12000, // 12s < 30s
            bpm: Some(124.0),
            key: Some("8A".to_string()),
            energy: None,
            format: None,
            artwork_url: None,
            played_at: now.to_rfc3339(),
            session_id: None,
            metadata_json: None,
        })
        .unwrap();

    // 2. Song Streamed: listened for 45 seconds (>= 30s) -> 1 stream, 45s recorded
    recorder
        .record_listen_event(&ListenEvent {
            id: "s2".to_string(),
            source: "mixed_in_key".to_string(),
            track_id: None,
            title: "Streamed Track".to_string(),
            artist: "Streamed Artist".to_string(),
            album: None,
            duration_ms: 200000,
            played_ms: 45000, // 45s >= 30s
            bpm: Some(126.0),
            key: Some("11A".to_string()),
            energy: None,
            format: None,
            artwork_url: None,
            played_at: (now + Duration::minutes(5)).to_rfc3339(),
            session_id: None,
            metadata_json: None,
        })
        .unwrap();

    // 3. Song Streamed 2: listened for 75 seconds (>= 30s)
    recorder
        .record_listen_event(&ListenEvent {
            id: "s3".to_string(),
            source: "mixed_in_key".to_string(),
            track_id: None,
            title: "Streamed Track 2".to_string(),
            artist: "Streamed Artist".to_string(),
            album: None,
            duration_ms: 220000,
            played_ms: 75000, // 75s >= 30s
            bpm: Some(126.0),
            key: Some("11A".to_string()),
            energy: None,
            format: None,
            artwork_url: None,
            played_at: (now + Duration::minutes(10)).to_rfc3339(),
            session_id: None,
            metadata_json: None,
        })
        .unwrap();

    // Verification in get_top_tracks:
    // Short Track has 0 streams (< 30s), so it MUST NOT appear in top_tracks!
    let top_tracks = recorder.get_top_tracks("all", 10).unwrap();
    assert_eq!(top_tracks.len(), 2);
    assert!(!top_tracks.iter().any(|t| t.title == "Short Track"));
    assert!(top_tracks.iter().any(|t| t.title == "Streamed Track"));
    assert!(top_tracks.iter().any(|t| t.title == "Streamed Track 2"));

    // Verification in get_top_artists:
    // Short Artist has 0 streams (< 30s), so it MUST NOT appear in top_artists!
    let top_artists = recorder.get_top_artists("all", 10).unwrap();
    assert_eq!(top_artists.len(), 1);
    assert_eq!(top_artists[0].artist, "Streamed Artist");
    assert_eq!(top_artists[0].plays, 2);
    assert_eq!(top_artists[0].total_minutes, 2); // 45s + 75s = 120s = 2 minutes

    // Verification in get_stats_summary:
    // total_plays is only the 2 streams (>= 30s).
    // total_minutes cumulates all seconds from 1st second: (12s + 45s + 75s = 132s = 2 minutes).
    let summary = recorder.get_stats_summary("all").unwrap();
    assert_eq!(summary.total_plays, 2);
    assert_eq!(summary.total_minutes, 2);
}

#[test]
fn test_spotify_client_id_persistence() {
    let (conn_arc, recorder) = setup_test_db();
    let spotify_svc = SpotifyTrackerService::new(conn_arc, recorder);

    assert_eq!(spotify_svc.get_client_id(), None);

    spotify_svc
        .set_client_id("test-custom-client-id-12345")
        .unwrap();
    assert_eq!(
        spotify_svc.get_client_id().as_deref(),
        Some("test-custom-client-id-12345")
    );

    let auth_url = spotify_svc.get_auth_url(None, None).unwrap();
    assert!(auth_url.contains("client_id=test-custom-client-id-12345"));
    assert!(auth_url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A8888%2Fcallback"));
    assert!(auth_url.contains("&state="));
    assert!(auth_url.contains("&code_challenge="));
}

#[test]
fn test_spotify_client_secret_persistence() {
    let (conn_arc, recorder) = setup_test_db();
    let spotify_svc = SpotifyTrackerService::new(conn_arc, recorder);

    assert_eq!(spotify_svc.get_client_secret(), None);

    spotify_svc
        .set_client_secret("test-secret-abcdef123456")
        .unwrap();
    assert_eq!(
        spotify_svc.get_client_secret().as_deref(),
        Some("test-secret-abcdef123456")
    );

    // Empty secret deletes the entry
    spotify_svc.set_client_secret("   ").unwrap();
    assert_eq!(spotify_svc.get_client_secret(), None);
}

#[test]
fn test_spotify_pkce_generation_and_state_pairing() {
    let (conn_arc, recorder) = setup_test_db();
    let spotify_svc = SpotifyTrackerService::new(conn_arc, recorder);

    let (verifier, challenge) = SpotifyTrackerService::generate_pkce_pair();
    assert_eq!(
        verifier.len(),
        64,
        "Code verifier must be 64 characters long"
    );
    assert!(!challenge.is_empty(), "Challenge must not be empty");

    // Verify all chars in verifier belong to RFC 7636 unreserved set
    let allowed_chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
    assert!(verifier.chars().all(|c| allowed_chars.contains(c)));

    // Test saving and consuming state
    let state_id = "test-state-123";
    spotify_svc.save_pkce_verifier(state_id, &verifier).unwrap();

    let retrieved = spotify_svc.get_and_consume_pkce_verifier(state_id);
    assert_eq!(retrieved.as_deref(), Some(verifier.as_str()));

    // Once consumed, should return None
    assert_eq!(spotify_svc.get_and_consume_pkce_verifier(state_id), None);
}

#[test]
fn test_spotify_pkce_state_multi_request_isolation() {
    let (conn_arc, recorder) = setup_test_db();
    let spotify_svc = SpotifyTrackerService::new(conn_arc, recorder);

    // Call get_auth_url twice: should generate two distinct states and verifiers
    let url_1 = spotify_svc.get_auth_url(None, None).unwrap();
    let url_2 = spotify_svc.get_auth_url(None, None).unwrap();

    assert_ne!(
        url_1, url_2,
        "Two auth URLs must have distinct states and challenges"
    );

    // Extract state parameters
    let state_1 = url_1.split("state=").nth(1).unwrap().to_string();
    let state_2 = url_2.split("state=").nth(1).unwrap().to_string();

    assert_ne!(state_1, state_2);

    // Verifiers for both states must exist in SQLite simultaneously without overwriting
    let verifier_1 = spotify_svc.get_and_consume_pkce_verifier(&state_1);
    let verifier_2 = spotify_svc.get_and_consume_pkce_verifier(&state_2);

    assert!(verifier_1.is_some());
    assert!(verifier_2.is_some());
    assert_ne!(verifier_1, verifier_2);
}

#[test]
fn test_spotify_default_redirect_uri() {
    use crate::services::stats::spotify::DEFAULT_SPOTIFY_REDIRECT_URI;
    assert_eq!(
        DEFAULT_SPOTIFY_REDIRECT_URI,
        "http://127.0.0.1:8888/callback"
    );
}

#[test]
fn test_split_artists_and_multi_artist_aggregation() {
    let (_conn_arc, recorder) = setup_test_db();

    // 1. Test unit splitting
    let split_1 =
        StatsRecorderService::split_artists("Calvin Harris, Rihanna", "This Is What You Came For");
    assert_eq!(split_1, vec!["Calvin Harris", "Rihanna"]);

    let split_2 = StatsRecorderService::split_artists("T.I.", "Live Your Life (feat. Rihanna)");
    assert_eq!(split_2, vec!["T.I.", "Rihanna"]);

    let split_3 = StatsRecorderService::split_artists("Jerro feat. Beacon", "Go Back");
    assert_eq!(split_3, vec!["Jerro", "Beacon"]);

    // 2. Test database multi-artist aggregation
    // Calvin Harris, Rihanna track: 220s (3m40s)
    let ev1 = ListenEvent {
        id: "ev1".to_string(),
        source: "spotify".to_string(),
        track_id: Some("sp1".to_string()),
        title: "This Is What You Came For".to_string(),
        artist: "Calvin Harris, Rihanna".to_string(),
        album: Some("This Is What You Came For".to_string()),
        duration_ms: 222_000,
        played_ms: 222_000,
        bpm: Some(124.0),
        key: Some("4A".to_string()),
        energy: Some(8),
        format: Some("spotify".to_string()),
        artwork_url: Some("https://i.scdn.co/art1.jpg".to_string()),
        played_at: "2026-09-04T10:00:00Z".to_string(),
        session_id: None,
        metadata_json: None,
    };
    recorder.record_listen_event(&ev1).unwrap();

    // T.I. feat. Rihanna track: 330s (5m30s)
    let ev2 = ListenEvent {
        id: "ev2".to_string(),
        source: "spotify".to_string(),
        track_id: Some("sp2".to_string()),
        title: "Live Your Life (feat. Rihanna)".to_string(),
        artist: "T.I.".to_string(),
        album: Some("Paper Trail".to_string()),
        duration_ms: 330_000,
        played_ms: 330_000,
        bpm: Some(80.0),
        key: Some("11B".to_string()),
        energy: Some(7),
        format: Some("spotify".to_string()),
        artwork_url: Some("https://i.scdn.co/art2.jpg".to_string()),
        played_at: "2026-09-04T11:00:00Z".to_string(),
        session_id: None,
        metadata_json: None,
    };
    recorder.record_listen_event(&ev2).unwrap();

    let top_artists = recorder.get_top_artists("all", 10).unwrap();
    // Rihanna must be in the top artists with 2 plays and (222000 + 330000)/60000 = 9 minutes
    let rihanna = top_artists.iter().find(|a| a.artist == "Rihanna");
    assert!(
        rihanna.is_some(),
        "Rihanna must be aggregated from collaborations"
    );
    let r = rihanna.unwrap();
    assert_eq!(r.plays, 2, "Rihanna must have 2 accumulated streams");
    assert_eq!(
        r.total_minutes, 9,
        "Rihanna must have 9 accumulated minutes"
    );

    // Calvin Harris must have 1 play and 3 minutes
    let calvin = top_artists.iter().find(|a| a.artist == "Calvin Harris");
    assert!(calvin.is_some());
    assert_eq!(calvin.unwrap().plays, 1);
    assert_eq!(calvin.unwrap().total_minutes, 3);

    // T.I. must have 1 play and 5 minutes
    let ti = top_artists.iter().find(|a| a.artist == "T.I.");
    assert!(ti.is_some());
    assert_eq!(ti.unwrap().plays, 1);
    assert_eq!(ti.unwrap().total_minutes, 5);
}

fn listen(id: &str, title: &str, played_at: String) -> ListenEvent {
    ListenEvent {
        id: id.to_string(),
        source: "spotify".to_string(),
        track_id: None,
        title: title.to_string(),
        artist: "Artist".to_string(),
        album: None,
        duration_ms: 200_000,
        played_ms: 200_000,
        bpm: None,
        key: None,
        energy: None,
        format: None,
        artwork_url: None,
        played_at,
        session_id: None,
        metadata_json: None,
    }
}

#[test]
fn test_heatmap_tolerates_an_unparseable_timestamp() {
    let (conn_arc, recorder) = setup_test_db();
    recorder
        .record_listen_event(&listen("ok", "Valid", Utc::now().to_rfc3339()))
        .unwrap();
    conn_arc
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO listen_events (id, source, title, artist, duration_ms, played_ms, played_at)
             VALUES ('bad', 'spotify', 'Broken', 'Artist', 1000, 60000, 'not a date')",
            [],
        )
        .unwrap();
    let heatmap = recorder.get_listening_heatmap("all").unwrap();
    assert_eq!(heatmap.iter().map(|c| c.plays).sum::<usize>(), 1);
}

#[test]
fn test_seven_day_filter_handles_offsets() {
    let (_, recorder) = setup_test_db();
    let offset = chrono::FixedOffset::east_opt(2 * 3600).unwrap();
    let recent = (Utc::now() - Duration::days(3))
        .with_timezone(&offset)
        .to_rfc3339();
    let old = (Utc::now() - Duration::days(8))
        .with_timezone(&offset)
        .to_rfc3339();
    recorder
        .record_listen_event(&listen("r", "Recent", recent))
        .unwrap();
    recorder
        .record_listen_event(&listen("o", "Old", old))
        .unwrap();
    let top = recorder.get_top_tracks("7d", 10).unwrap();
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].title, "Recent");
}

#[test]
fn test_record_listen_events_batch_skips_duplicates() {
    let (_, recorder) = setup_test_db();
    let at = "2026-09-01T14:30:00Z".to_string();
    let outcomes = recorder
        .record_listen_events(&[
            listen("a", "One", at.clone()),
            listen("b", "Two", at.clone()),
            listen("c", "One", at),
        ])
        .unwrap();
    assert_eq!(outcomes, vec![true, true, false]);
}

// ==========================================
// History export (CSV / JSON)
// ==========================================

mod history_export_tests {
    use super::*;
    use crate::services::stats::history_export::{
        export_in_chunks_for_test, validate_destination, HistoryExportFormat,
    };
    use std::path::PathBuf;

    /// Inserts rows straight into the table: the recorder would skip near-duplicates.
    fn insert_raw(
        conn: &Arc<Mutex<Connection>>,
        id: &str,
        played_at: &str,
        title: &str,
        metadata: Option<&str>,
    ) {
        conn.lock()
            .unwrap()
            .execute(
                "INSERT INTO listen_events (id, source, title, artist, duration_ms, played_ms, played_at, metadata_json)
                 VALUES (?1, 'spotify', ?2, 'Artist', 200000, 180000, ?3, ?4)",
                rusqlite::params![id, title, played_at, metadata],
            )
            .unwrap();
    }

    struct TempFile(PathBuf);
    impl TempFile {
        fn new(name: &str, extension: &str) -> Self {
            Self(std::env::temp_dir().join(format!(
                "crate_history_{name}_{}.{extension}",
                std::process::id()
            )))
        }
    }
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let mut part = self.0.clone().into_os_string();
            part.push(".part");
            let _ = std::fs::remove_file(part);
        }
    }

    /// Minimal RFC 4180 reader, enough to round-trip what the exporter writes.
    fn parse_csv(text: &str) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        let mut row = Vec::new();
        let mut field = String::new();
        let mut in_quotes = false;
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match (in_quotes, c) {
                (true, '"') if chars.peek() == Some(&'"') => {
                    field.push('"');
                    chars.next();
                }
                (true, '"') => in_quotes = false,
                (true, _) => field.push(c),
                (false, '"') => in_quotes = true,
                (false, ',') => row.push(std::mem::take(&mut field)),
                (false, '\n') => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                (false, _) => field.push(c),
            }
        }
        rows
    }

    #[test]
    fn csv_round_trips_awkward_values() {
        let (conn, recorder) = setup_test_db();
        let awkward = "Hello, \"World\"\nsecond line — é ü 日本";
        insert_raw(&conn, "a", "2026-01-01T10:00:00Z", awkward, None);
        insert_raw(
            &conn,
            "b",
            "2026-01-02T10:00:00Z",
            "Plain",
            Some(r#"{"uri":"x"}"#),
        );

        let file = TempFile::new("csv", "csv");
        let n = recorder
            .export_listen_history(HistoryExportFormat::Csv, &file.0)
            .unwrap();
        assert_eq!(n, 2);

        let rows = parse_csv(&std::fs::read_to_string(&file.0).unwrap());
        assert_eq!(rows.len(), 3, "header + 2 rows");
        assert_eq!(rows[0][0], "played_at");
        assert_eq!(rows[0].len(), 15);
        let title_col = rows[0].iter().position(|c| c == "title").unwrap();
        assert_eq!(rows[1][title_col], awkward);
        assert_eq!(rows[2][title_col], "Plain");
        let meta_col = rows[0].iter().position(|c| c == "metadata_json").unwrap();
        assert_eq!(rows[2][meta_col], r#"{"uri":"x"}"#);
        // Missing values are empty fields, not the word "null".
        let album_col = rows[0].iter().position(|c| c == "album").unwrap();
        assert_eq!(rows[1][album_col], "");
    }

    #[test]
    fn json_is_valid_typed_and_keeps_nulls() {
        let (conn, recorder) = setup_test_db();
        insert_raw(
            &conn,
            "a",
            "2026-01-01T10:00:00Z",
            "One",
            Some(r#"{"uri":"x"}"#),
        );
        insert_raw(&conn, "b", "2026-01-02T10:00:00Z", "Two", Some("not json"));
        insert_raw(&conn, "c", "2026-01-03T10:00:00Z", "Three", None);

        let file = TempFile::new("json", "json");
        assert_eq!(
            recorder
                .export_listen_history(HistoryExportFormat::Json, &file.0)
                .unwrap(),
            3
        );
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file.0).unwrap()).unwrap();
        let items = value.as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["title"], "One");
        assert_eq!(items[0]["duration_ms"], 200000);
        assert!(items[0]["album"].is_null());
        assert_eq!(
            items[0]["metadata_json"]["uri"], "x",
            "valid metadata is real JSON"
        );
        assert_eq!(
            items[1]["metadata_json"], "not json",
            "invalid metadata is kept as text"
        );
        assert!(items[2]["metadata_json"].is_null());
    }

    #[test]
    fn an_empty_history_gives_valid_empty_files() {
        let (_conn, recorder) = setup_test_db();
        let csv = TempFile::new("empty", "csv");
        let json = TempFile::new("empty", "json");
        assert_eq!(
            recorder
                .export_listen_history(HistoryExportFormat::Csv, &csv.0)
                .unwrap(),
            0
        );
        assert_eq!(
            recorder
                .export_listen_history(HistoryExportFormat::Json, &json.0)
                .unwrap(),
            0
        );
        assert_eq!(
            parse_csv(&std::fs::read_to_string(&csv.0).unwrap()).len(),
            1
        );
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json.0).unwrap()).unwrap();
        assert_eq!(value, serde_json::json!([]));
    }

    #[test]
    fn chunk_boundaries_lose_and_repeat_nothing_even_with_equal_timestamps() {
        let (conn, recorder) = setup_test_db();
        // 11 rows: five share one timestamp, which a chunk boundary will cut through.
        for i in 0..6 {
            insert_raw(
                &conn,
                &format!("id{i:02}"),
                &format!("2026-01-0{}T10:00:00Z", i + 1),
                "t",
                None,
            );
        }
        for i in 0..5 {
            insert_raw(
                &conn,
                &format!("tie{i}"),
                "2026-02-01T10:00:00Z",
                "tie",
                None,
            );
        }

        let file = TempFile::new("chunks", "csv");
        let n = export_in_chunks_for_test(&recorder, HistoryExportFormat::Csv, &file.0, 3).unwrap();
        assert_eq!(n, 11);

        let rows = parse_csv(&std::fs::read_to_string(&file.0).unwrap());
        let id_col = rows[0].iter().position(|c| c == "id").unwrap();
        let ids: Vec<&str> = rows[1..].iter().map(|r| r[id_col].as_str()).collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 11, "every row exactly once: {ids:?}");
        let at_col = rows[0].iter().position(|c| c == "played_at").unwrap();
        let times: Vec<&str> = rows[1..].iter().map(|r| r[at_col].as_str()).collect();
        assert!(times.windows(2).all(|w| w[0] <= w[1]), "oldest first");
    }

    #[test]
    fn destination_must_match_the_format_and_an_existing_folder() {
        let dir = std::env::temp_dir();
        assert!(validate_destination(HistoryExportFormat::Csv, &dir.join("h.csv")).is_ok());
        assert!(validate_destination(HistoryExportFormat::Csv, &dir.join("H.CSV")).is_ok());
        assert!(validate_destination(HistoryExportFormat::Csv, &dir.join("h.json")).is_err());
        assert!(validate_destination(HistoryExportFormat::Json, &dir.join("h")).is_err());
        assert!(validate_destination(
            HistoryExportFormat::Json,
            &dir.join("no_such_folder_crate").join("h.json")
        )
        .is_err());
    }

    #[test]
    fn a_failed_export_leaves_no_file_and_keeps_an_existing_one() {
        let (conn, recorder) = setup_test_db();
        insert_raw(&conn, "a", "2026-01-01T10:00:00Z", "One", None);

        // Existing export is replaced atomically by a good one...
        let file = TempFile::new("atomic", "json");
        std::fs::write(&file.0, "previous export").unwrap();
        recorder
            .export_listen_history(HistoryExportFormat::Json, &file.0)
            .unwrap();
        assert!(std::fs::read_to_string(&file.0).unwrap().contains("One"));

        // ...and a destination that cannot be written leaves nothing behind.
        let bad = std::env::temp_dir()
            .join("no_such_folder_crate")
            .join("x.json");
        assert!(recorder
            .export_listen_history(HistoryExportFormat::Json, &bad)
            .is_err());
        assert!(!bad.exists());
    }
}

// ==========================================
// Exact time windows (`between:<start>,<end>`)
// ==========================================

mod window_tests {
    use super::*;

    fn raw(conn: &Arc<Mutex<Connection>>, id: &str, played_at: &str, played_ms: i64) {
        conn.lock()
            .unwrap()
            .execute(
                "INSERT INTO listen_events (id, source, title, artist, duration_ms, played_ms, played_at)
                 VALUES (?1, 'spotify', ?1, 'Artist', 200000, ?2, ?3)",
                rusqlite::params![id, played_ms, played_at],
            )
            .unwrap();
    }

    #[test]
    fn a_window_includes_its_start_and_excludes_its_end() {
        let (conn, recorder) = setup_test_db();
        raw(&conn, "before", "2026-09-20T23:59:59Z", 60_000);
        raw(&conn, "start", "2026-09-21T00:00:00Z", 60_000);
        raw(&conn, "inside", "2026-09-25T12:00:00+02:00", 60_000); // 10:00 UTC
        raw(&conn, "end", "2026-09-28T00:00:00Z", 60_000);

        let summary = recorder
            .get_stats_summary("between:2026-09-21 00:00:00,2026-09-28 00:00:00")
            .unwrap();
        assert_eq!(summary.total_plays, 2, "only `start` and `inside`");
    }

    #[test]
    fn a_malformed_window_matches_nothing_instead_of_everything() {
        let (conn, recorder) = setup_test_db();
        raw(&conn, "a", "2026-09-21T10:00:00Z", 60_000);
        for bad in [
            "between:",
            "between:2026-09-21",
            "between:2026-09-21 00:00:00",
            "between:yesterday,today",
            "between:2026-09-21 00:00:00,",
        ] {
            assert_eq!(
                recorder.get_stats_summary(bad).unwrap().total_plays,
                0,
                "{bad}"
            );
        }
    }

    #[test]
    fn a_window_cannot_inject_sql() {
        let (conn, recorder) = setup_test_db();
        raw(&conn, "a", "2026-09-21T10:00:00Z", 60_000);
        let hostile =
            "between:2026-09-21 00:00:00'); DROP TABLE listen_events; --,2026-09-28 00:00:00";
        assert_eq!(recorder.get_stats_summary(hostile).unwrap().total_plays, 0);
        let still_there: i64 = conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM listen_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(still_there, 1, "the table is intact");
    }

    #[test]
    fn named_ranges_still_work_next_to_windows() {
        let (conn, recorder) = setup_test_db();
        raw(&conn, "old", "2020-01-01T10:00:00Z", 60_000);

        assert_eq!(recorder.get_stats_summary("all").unwrap().total_plays, 1);
        assert_eq!(recorder.get_stats_summary("30d").unwrap().total_plays, 0);
    }
}

// ==========================================
// Recap ("Your week" / "Your year")
// ==========================================

mod recap_tests {
    use super::*;
    use crate::services::stats::recap::{period_bounds, RecapPeriod};
    use chrono::{FixedOffset, Local, NaiveDate, TimeZone, Timelike};

    fn at(offset_hours: i32, y: i32, m: u32, d: u32, h: u32) -> chrono::DateTime<FixedOffset> {
        FixedOffset::east_opt(offset_hours * 3600)
            .unwrap()
            .with_ymd_and_hms(y, m, d, h, 0, 0)
            .unwrap()
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn utc(text: &str) -> chrono::NaiveDateTime {
        chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn a_week_runs_from_monday_to_sunday_in_local_time() {
        // Wednesday 30 September 2026, 15:00 at UTC+2.
        let now = at(2, 2026, 9, 30, 15);
        let week = period_bounds(RecapPeriod::Week, 0, &now).unwrap();
        assert_eq!(week.first_day, date(2026, 9, 28));
        assert_eq!(week.last_day, date(2026, 10, 4));
        // Local midnight is two hours earlier in UTC.
        assert_eq!(week.start_utc, utc("2026-09-27 22:00:00"));
        assert_eq!(week.end_utc, utc("2026-10-04 22:00:00"));
    }

    #[test]
    fn sunday_belongs_to_the_week_that_started_the_previous_monday() {
        let sunday = at(0, 2026, 10, 4, 23);
        let week = period_bounds(RecapPeriod::Week, 0, &sunday).unwrap();
        assert_eq!(
            (week.first_day, week.last_day),
            (date(2026, 9, 28), date(2026, 10, 4))
        );
        let monday = at(0, 2026, 10, 5, 0);
        let next = period_bounds(RecapPeriod::Week, 0, &monday).unwrap();
        assert_eq!(next.first_day, date(2026, 10, 5));
    }

    #[test]
    fn an_offset_steps_back_whole_periods() {
        let now = at(0, 2026, 9, 30, 12);
        let previous_week = period_bounds(RecapPeriod::Week, 1, &now).unwrap();
        assert_eq!(previous_week.first_day, date(2026, 9, 21));
        assert_eq!(previous_week.last_day, date(2026, 9, 27));

        let last_year = period_bounds(RecapPeriod::Year, 1, &now).unwrap();
        assert_eq!(last_year.first_day, date(2025, 1, 1));
        assert_eq!(last_year.last_day, date(2025, 12, 31));
        assert_eq!(last_year.start_utc, utc("2025-01-01 00:00:00"));
        assert_eq!(last_year.end_utc, utc("2026-01-01 00:00:00"));
    }

    #[test]
    fn a_period_out_of_the_calendar_is_an_error_not_a_panic() {
        let now = at(0, 2026, 9, 30, 12);
        assert!(period_bounds(RecapPeriod::Year, u32::MAX, &now).is_err());
    }

    fn listen_at(
        conn: &Arc<Mutex<Connection>>,
        id: &str,
        title: &str,
        artist: &str,
        at: &str,
        played_ms: i64,
        key: Option<&str>,
    ) {
        conn.lock()
            .unwrap()
            .execute(
                "INSERT INTO listen_events (id, source, title, artist, duration_ms, played_ms, played_at, key)
                 VALUES (?1, 'spotify', ?2, ?3, 200000, ?4, ?5, ?6)",
                rusqlite::params![id, title, artist, played_ms, at, key],
            )
            .unwrap();
    }

    #[test]
    fn a_week_recap_compares_with_the_previous_week_and_counts_discoveries() {
        let (conn, recorder) = setup_test_db();
        // "Now" is Wednesday 30 September 2026 noon UTC: the week is 28 Sep to 4 Oct.
        let now = at(0, 2026, 9, 30, 12);

        // Last week: Song A once (so A is not new this week).
        listen_at(
            &conn,
            "p1",
            "Song A",
            "Artist X",
            "2026-09-22T10:00:00Z",
            200_000,
            Some("8A"),
        );
        // This week: Song A three times, Song B once (new), one 10 s skip (not a play).
        for (i, hour) in [9, 10, 11].iter().enumerate() {
            listen_at(
                &conn,
                &format!("a{i}"),
                "Song A",
                "Artist X",
                &format!("2026-09-29T{hour:02}:00:00Z"),
                200_000,
                Some("8A"),
            );
        }
        listen_at(
            &conn,
            "b",
            "Song B",
            "Artist Y",
            "2026-09-30T08:00:00Z",
            180_000,
            Some("5B"),
        );
        listen_at(
            &conn,
            "skip",
            "Song C",
            "Artist Z",
            "2026-09-30T09:00:00Z",
            10_000,
            None,
        );
        // After the week.
        listen_at(
            &conn,
            "later",
            "Song D",
            "Artist W",
            "2026-10-06T10:00:00Z",
            200_000,
            None,
        );

        let recap = recorder.recap_at(RecapPeriod::Week, 0, &now).unwrap();

        assert_eq!(
            (recap.start_date.as_str(), recap.end_date.as_str()),
            ("2026-09-28", "2026-10-04")
        );
        assert_eq!(recap.total_plays, 4);
        assert_eq!(recap.previous_plays, 1);
        assert_eq!(recap.unique_tracks, 2, "the 10 s skip is not a listen");
        assert_eq!(recap.unique_artists, 2);
        assert_eq!(
            recap.new_tracks, 1,
            "only Song B is new; Song A was heard last week"
        );

        assert_eq!(recap.top_tracks[0].title, "Song A");
        assert_eq!(recap.top_tracks[0].plays, 3);
        assert_eq!(recap.top_artists[0].artist, "Artist X");
        assert_eq!(recap.top_keys[0].key, "8A");
        assert_eq!(recap.top_keys[0].plays, 3);
        // 3 x 200 s + 180 s + the 10 s skip = 790 s of Spotify listening = 13 whole minutes.
        assert_eq!(recap.source_minutes.get("spotify"), Some(&13));
    }

    #[test]
    fn peak_times_are_reported_in_local_time() {
        let (conn, recorder) = setup_test_db();
        let now = at(0, 2026, 9, 30, 12);
        // Three plays at the same UTC hour on Tuesday 29 September, one elsewhere.
        for i in 0..3 {
            listen_at(
                &conn,
                &format!("x{i}"),
                &format!("T{i}"),
                "A",
                "2026-09-29T20:00:00Z",
                200_000,
                None,
            );
        }
        listen_at(&conn, "y", "T9", "A", "2026-09-30T07:00:00Z", 200_000, None);

        let recap = recorder.recap_at(RecapPeriod::Week, 0, &now).unwrap();

        let busiest = chrono::Utc
            .with_ymd_and_hms(2026, 9, 29, 20, 0, 0)
            .unwrap()
            .with_timezone(&Local);
        assert_eq!(recap.peak_hour, Some(busiest.hour() as u8));
        assert_eq!(
            recap.busiest_day.as_ref().unwrap().date,
            busiest.date_naive().to_string()
        );
        assert_eq!(recap.busiest_day.unwrap().plays, 3);
    }

    #[test]
    fn an_empty_period_is_all_zeros_and_no_peaks() {
        let (_conn, recorder) = setup_test_db();
        let now = at(0, 2026, 9, 30, 12);
        let recap = recorder.recap_at(RecapPeriod::Year, 0, &now).unwrap();
        assert_eq!(
            (recap.total_plays, recap.total_minutes, recap.new_tracks),
            (0, 0, 0)
        );
        assert!(
            recap.top_tracks.is_empty()
                && recap.top_artists.is_empty()
                && recap.top_keys.is_empty()
        );
        assert_eq!((recap.peak_hour, recap.peak_weekday), (None, None));
        assert!(recap.busiest_day.is_none());
        assert_eq!(
            (recap.start_date.as_str(), recap.end_date.as_str()),
            ("2026-01-01", "2026-12-31")
        );
    }
}

// ==========================================
// Timeline of a Rekordbox set
// ==========================================

mod timeline_tests {
    use super::*;
    use crate::services::harmonic::{HarmonicRelation, ENERGY_JUMP};

    fn session(conn: &Arc<Mutex<Connection>>, id: &str) {
        conn.lock()
            .unwrap()
            .execute(
                "INSERT INTO rekordbox_sessions (id, session_name, started_at, total_tracks, total_played_ms)
                 VALUES (?1, 'Friday set', '2026-09-25T22:00:00Z', 4, 800000)",
                [id],
            )
            .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn played(
        conn: &Arc<Mutex<Connection>>,
        id: &str,
        session_id: &str,
        source: &str,
        title: &str,
        artist: &str,
        at: &str,
        bpm: Option<f64>,
        key: Option<&str>,
    ) {
        conn.lock()
            .unwrap()
            .execute(
                "INSERT INTO listen_events (id, source, title, artist, duration_ms, played_ms, played_at, session_id, bpm, key)
                 VALUES (?1, ?2, ?3, ?4, 200000, 200000, ?5, ?6, ?7, ?8)",
                rusqlite::params![id, source, title, artist, at, session_id, bpm, key],
            )
            .unwrap();
    }

    fn library(
        conn: &Arc<Mutex<Connection>>,
        id: &str,
        artist: &str,
        title: &str,
        key: &str,
        bpm: f64,
        energy: i32,
    ) {
        conn.lock()
            .unwrap()
            .execute(
                "INSERT INTO tracks (id, file_path, format, title, artist, key, bpm, energy, duration_ms, date_added, date_modified)
                 VALUES (?1, ?2, 'mp3', ?3, ?4, ?5, ?6, ?7, 200000, '2026-01-01', '2026-01-01')",
                rusqlite::params![id, format!("/m/{id}.mp3"), title, artist, key, bpm, energy],
            )
            .unwrap();
    }

    #[test]
    fn a_set_is_ordered_enriched_by_the_library_and_numbered() {
        let (conn, recorder) = setup_test_db();
        session(&conn, "s1");
        library(&conn, "t1", "Artist A", "Opener", "8A", 124.0, 6);
        library(&conn, "t2", "Artist B", "Second", "9A", 126.0, 8);
        // Inserted out of order; the second is written in other casing and Rekordbox's own key.
        played(
            &conn,
            "e2",
            "s1",
            "rekordbox",
            " SECOND ",
            "artist b",
            "2026-09-25T22:04:00Z",
            Some(130.0),
            Some("Em"),
        );
        played(
            &conn,
            "e1",
            "s1",
            "rekordbox",
            "Opener",
            "Artist A",
            "2026-09-25T22:00:00Z",
            None,
            None,
        );
        // Not in the library: what Rekordbox recorded is kept, and there is no energy.
        played(
            &conn,
            "e3",
            "s1",
            "rekordbox",
            "Unknown Track",
            "Artist Z",
            "2026-09-25T22:08:00Z",
            Some(128.0),
            Some("3B"),
        );

        let timeline = recorder.get_session_timeline("s1").unwrap();

        assert_eq!(timeline.session.session_name.as_deref(), Some("Friday set"));
        let titles: Vec<&str> = timeline.tracks.iter().map(|t| t.title.trim()).collect();
        assert_eq!(titles, ["Opener", "SECOND", "Unknown Track"]);
        assert_eq!(
            timeline
                .tracks
                .iter()
                .map(|t| t.position)
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );

        let second = &timeline.tracks[1];
        assert_eq!(second.library_track_id.as_deref(), Some("t2"));
        assert_eq!(
            (second.key.as_deref(), second.bpm, second.energy),
            (Some("9A"), Some(126.0), Some(8)),
            "the library's analysis wins"
        );
        let unknown = &timeline.tracks[2];
        assert_eq!(unknown.library_track_id, None);
        assert_eq!(
            (unknown.key.as_deref(), unknown.bpm, unknown.energy),
            (Some("3B"), Some(128.0), None)
        );
    }

    #[test]
    fn every_transition_says_how_it_mixes() {
        let (conn, recorder) = setup_test_db();
        session(&conn, "s1");
        played(
            &conn,
            "e1",
            "s1",
            "rekordbox",
            "One",
            "A",
            "2026-09-25T22:00:00Z",
            Some(120.0),
            Some("8A"),
        );
        played(
            &conn,
            "e2",
            "s1",
            "rekordbox",
            "Two",
            "A",
            "2026-09-25T22:01:00Z",
            Some(126.0),
            Some("9A"),
        ); // next key, +5 %
        played(
            &conn,
            "e3",
            "s1",
            "rekordbox",
            "Three",
            "A",
            "2026-09-25T22:02:00Z",
            Some(126.0),
            Some("3B"),
        ); // clash
        played(
            &conn,
            "e4",
            "s1",
            "rekordbox",
            "Four",
            "A",
            "2026-09-25T22:03:00Z",
            None,
            None,
        ); // unknown key

        let timeline = recorder.get_session_timeline("s1").unwrap();

        assert!(timeline.tracks[0].from_previous.is_none());
        let transition = |i: usize| timeline.tracks[i].from_previous.clone().unwrap();
        assert_eq!(transition(1).harmonic, HarmonicRelation::Adjacent);
        assert_eq!(transition(1).bpm_delta_percent, Some(5.0));
        assert_eq!(transition(2).harmonic, HarmonicRelation::Clash);
        assert_eq!(transition(3).harmonic, HarmonicRelation::Unknown);
        // None of these tracks matched a library track, so none has an energy: the change is
        // unknown rather than miscounted as a jump.
        assert_eq!(
            (transition(1).energy_delta, transition(1).energy_jump),
            (None, false)
        );
        assert_eq!(
            (
                timeline.harmonic_transitions,
                timeline.clashing_transitions,
                timeline.unknown_transitions,
                timeline.energy_jumps
            ),
            (1, 1, 1, 0)
        );
    }

    #[test]
    fn an_energy_jump_is_flagged_and_counted_with_the_set_planners_threshold() {
        let (conn, recorder) = setup_test_db();
        session(&conn, "s1");
        library(&conn, "t1", "A", "One", "8A", 120.0, 4);
        library(&conn, "t2", "A", "Two", "8A", 120.0, 4 + ENERGY_JUMP - 1); // below the threshold
        library(&conn, "t3", "A", "Three", "8A", 120.0, 4 + ENERGY_JUMP - 1 + ENERGY_JUMP); // crosses it
        played(
            &conn,
            "e1",
            "s1",
            "rekordbox",
            "One",
            "A",
            "2026-09-25T22:00:00Z",
            Some(120.0),
            Some("8A"),
        );
        played(
            &conn,
            "e2",
            "s1",
            "rekordbox",
            "Two",
            "A",
            "2026-09-25T22:01:00Z",
            Some(120.0),
            Some("8A"),
        );
        played(
            &conn,
            "e3",
            "s1",
            "rekordbox",
            "Three",
            "A",
            "2026-09-25T22:02:00Z",
            Some(120.0),
            Some("8A"),
        );

        let timeline = recorder.get_session_timeline("s1").unwrap();
        let transition = |i: usize| timeline.tracks[i].from_previous.clone().unwrap();
        assert_eq!(
            (transition(1).energy_delta, transition(1).energy_jump),
            (Some(ENERGY_JUMP - 1), false),
            "just under the threshold is not a jump"
        );
        assert_eq!(
            (transition(2).energy_delta, transition(2).energy_jump),
            (Some(ENERGY_JUMP), true),
            "exactly the threshold already counts, like the set planner"
        );
        assert_eq!(timeline.energy_jumps, 1);
    }

    #[test]
    fn a_track_missing_from_the_library_has_no_energy_and_is_never_counted_as_a_jump() {
        let (conn, recorder) = setup_test_db();
        session(&conn, "s1");
        library(&conn, "t1", "A", "One", "8A", 120.0, 4);
        // "Two" matches no library track, so Crate cannot know its energy.
        played(
            &conn,
            "e1",
            "s1",
            "rekordbox",
            "One",
            "A",
            "2026-09-25T22:00:00Z",
            Some(120.0),
            Some("8A"),
        );
        played(
            &conn,
            "e2",
            "s1",
            "rekordbox",
            "Two",
            "A",
            "2026-09-25T22:01:00Z",
            Some(120.0),
            Some("8A"),
        );

        let timeline = recorder.get_session_timeline("s1").unwrap();
        assert_eq!(timeline.tracks[1].energy, None);
        let transition = timeline.tracks[1].from_previous.clone().unwrap();
        assert_eq!(
            (transition.energy_delta, transition.energy_jump),
            (None, false),
            "a missing energy is a dash, never a jump"
        );
        assert_eq!(timeline.energy_jumps, 0);
    }

    #[test]
    fn other_sessions_and_other_sources_do_not_leak_in() {
        let (conn, recorder) = setup_test_db();
        session(&conn, "s1");
        session(&conn, "s2");
        played(
            &conn,
            "e1",
            "s1",
            "rekordbox",
            "Mine",
            "A",
            "2026-09-25T22:00:00Z",
            None,
            None,
        );
        played(
            &conn,
            "e2",
            "s2",
            "rekordbox",
            "Other set",
            "A",
            "2026-09-26T22:00:00Z",
            None,
            None,
        );
        played(
            &conn,
            "e3",
            "s1",
            "spotify",
            "Not a set track",
            "A",
            "2026-09-25T22:30:00Z",
            None,
            None,
        );

        let timeline = recorder.get_session_timeline("s1").unwrap();
        assert_eq!(timeline.tracks.len(), 1);
        assert_eq!(timeline.tracks[0].title, "Mine");
    }

    #[test]
    fn an_empty_set_is_empty_and_an_unknown_one_is_an_error() {
        let (conn, recorder) = setup_test_db();
        session(&conn, "empty");
        let timeline = recorder.get_session_timeline("empty").unwrap();
        assert!(timeline.tracks.is_empty());
        assert_eq!(
            (
                timeline.harmonic_transitions,
                timeline.clashing_transitions,
                timeline.unknown_transitions,
                timeline.energy_jumps
            ),
            (0, 0, 0, 0)
        );
        assert!(recorder.get_session_timeline("nope").is_err());
    }
}

// ==========================================
// Reset Spotify listening history (CRA-144)
// ==========================================

mod spotify_reset_tests {
    use super::*;
    use std::path::PathBuf;

    /// Inserts a raw listen for `source`, bypassing the recorder's near-duplicate rule.
    fn insert_raw(conn: &Arc<Mutex<Connection>>, id: &str, source: &str, played_at: &str) {
        conn.lock()
            .unwrap()
            .execute(
                "INSERT INTO listen_events (id, source, title, artist, duration_ms, played_ms, played_at)
                 VALUES (?1, ?2, ?1, 'Artist', 200000, 180000, ?3)",
                rusqlite::params![id, source, played_at],
            )
            .unwrap();
    }

    fn count_sources(conn: &Arc<Mutex<Connection>>) -> (i64, i64) {
        let conn = conn.lock().unwrap();
        let spotify: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM listen_events WHERE source = 'spotify'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let other: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM listen_events WHERE source != 'spotify'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        (spotify, other)
    }

    struct TempFile(PathBuf);
    impl TempFile {
        fn new(name: &str) -> Self {
            Self(std::env::temp_dir().join(format!(
                "crate_spotify_reset_{name}_{}.json",
                std::process::id()
            )))
        }
    }
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let mut part = self.0.clone().into_os_string();
            part.push(".part");
            let _ = std::fs::remove_file(part);
        }
    }

    #[test]
    fn only_spotify_listens_are_deleted() {
        let (conn, recorder) = setup_test_db();
        insert_raw(&conn, "sp1", "spotify", "2026-01-01T10:00:00Z");
        insert_raw(&conn, "sp2", "spotify", "2026-01-02T10:00:00Z");
        insert_raw(&conn, "rb1", "rekordbox", "2026-01-03T10:00:00Z");
        insert_raw(&conn, "mik1", "mixed_in_key", "2026-01-04T10:00:00Z");
        insert_raw(&conn, "lib1", "crate_local", "2026-01-05T10:00:00Z");

        let file = TempFile::new("basic");
        let result = recorder.reset_spotify_history(&file.0).unwrap();

        assert_eq!(result.deleted_count, 2);
        let (spotify, other) = count_sources(&conn);
        assert_eq!(spotify, 0, "every Spotify listen is gone");
        assert_eq!(other, 3, "every other source is untouched");
    }

    #[test]
    fn the_backup_exists_and_is_readable_and_reflects_the_state_before_deletion() {
        let (conn, recorder) = setup_test_db();
        insert_raw(&conn, "sp1", "spotify", "2026-01-01T10:00:00Z");
        insert_raw(&conn, "rb1", "rekordbox", "2026-01-02T10:00:00Z");

        let file = TempFile::new("backup");
        let result = recorder.reset_spotify_history(&file.0).unwrap();

        assert_eq!(result.backup_path, file.0.to_string_lossy());
        let contents = std::fs::read_to_string(&file.0).expect("backup file must be readable");
        let value: serde_json::Value =
            serde_json::from_str(&contents).expect("backup must be valid JSON");
        let items = value.as_array().unwrap();
        assert_eq!(
            items.len(),
            2,
            "the backup covers the whole history as it stood right before the delete"
        );
        assert!(items
            .iter()
            .any(|i| i["id"] == "sp1" && i["source"] == "spotify"));
        assert!(items
            .iter()
            .any(|i| i["id"] == "rb1" && i["source"] == "rekordbox"));
    }

    #[test]
    fn a_failed_backup_deletes_nothing() {
        let (conn, recorder) = setup_test_db();
        insert_raw(&conn, "sp1", "spotify", "2026-01-01T10:00:00Z");
        insert_raw(&conn, "rb1", "rekordbox", "2026-01-02T10:00:00Z");

        // A destination folder that does not exist makes the export fail before any row is read.
        let bad = std::env::temp_dir()
            .join("no_such_folder_crate_spotify_reset")
            .join("backup.json");
        let err = recorder.reset_spotify_history(&bad).unwrap_err();
        assert!(!err.to_string().is_empty());

        let (spotify, other) = count_sources(&conn);
        assert_eq!(spotify, 1, "nothing was deleted when the backup failed");
        assert_eq!(other, 1);
        assert!(!bad.exists(), "no partial backup was left behind either");
    }

    #[test]
    fn counts_only_spotify_listens() {
        let (conn, recorder) = setup_test_db();
        insert_raw(&conn, "sp1", "spotify", "2026-01-01T10:00:00Z");
        insert_raw(&conn, "sp2", "spotify", "2026-01-02T10:00:00Z");
        insert_raw(&conn, "rb1", "rekordbox", "2026-01-03T10:00:00Z");
        assert_eq!(recorder.count_spotify_listens().unwrap(), 2);
    }

    #[test]
    fn counting_an_empty_history_is_zero() {
        let (_conn, recorder) = setup_test_db();
        assert_eq!(recorder.count_spotify_listens().unwrap(), 0);
    }
}

fn insert_session(conn: &Arc<Mutex<Connection>>, id: &str, started_at: String, played_ms: i64) {
    conn.lock()
        .unwrap()
        .execute(
            "INSERT INTO rekordbox_sessions (id, session_name, started_at, ended_at, total_tracks, total_played_ms)
             VALUES (?1, 'Set', ?2, NULL, 5, ?3)",
            rusqlite::params![id, started_at, played_ms],
        )
        .unwrap();
}

#[test]
fn test_summary_dj_sessions_follow_the_selected_range() {
    let (conn, recorder) = setup_test_db();
    let now = Utc::now();
    insert_session(
        &conn,
        "recent",
        (now - Duration::days(2)).to_rfc3339(),
        3_600_000,
    );
    insert_session(
        &conn,
        "month",
        (now - Duration::days(20)).to_rfc3339(),
        7_200_000,
    );
    insert_session(
        &conn,
        "old",
        (now - Duration::days(400)).to_rfc3339(),
        1_800_000,
    );

    let week = recorder.get_stats_summary("7d").unwrap();
    assert_eq!(week.dj_sessions, 1);
    assert_eq!(week.dj_sessions_played_ms, 3_600_000);

    let month = recorder.get_stats_summary("30d").unwrap();
    assert_eq!(month.dj_sessions, 2);
    assert_eq!(month.dj_sessions_played_ms, 10_800_000);

    let all = recorder.get_stats_summary("all").unwrap();
    assert_eq!(all.dj_sessions, 3);
    assert_eq!(all.dj_sessions_played_ms, 12_600_000);
}

#[test]
fn test_summary_dj_sessions_honour_an_exact_window() {
    let (conn, recorder) = setup_test_db();
    insert_session(&conn, "in", "2026-03-10T20:00:00Z".to_string(), 1_000);
    insert_session(&conn, "before", "2026-02-28T23:59:59Z".to_string(), 1_000);
    insert_session(
        &conn,
        "end-excluded",
        "2026-04-01T00:00:00Z".to_string(),
        1_000,
    );

    let march = recorder
        .get_stats_summary("between:2026-03-01 00:00:00,2026-04-01 00:00:00")
        .unwrap();
    assert_eq!(march.dj_sessions, 1);

    let malformed = recorder.get_stats_summary("between:nonsense").unwrap();
    assert_eq!(
        malformed.dj_sessions, 0,
        "a malformed window matches nothing"
    );
}

#[test]
fn test_summary_unique_artists_is_not_capped_by_the_top_list_limit() {
    let (_, recorder) = setup_test_db();
    let now = Utc::now();
    for i in 0..60 {
        let mut event = listen(
            &format!("e{i}"),
            &format!("Track {i}"),
            (now - Duration::minutes(i)).to_rfc3339(),
        );
        event.artist = format!("Artist {i}");
        recorder.record_listen_event(&event).unwrap();
    }
    // A listen below the 30 s stream threshold does not make its artist count.
    let mut skipped = listen("skip", "Skipped", now.to_rfc3339());
    skipped.artist = "Barely Heard".to_string();
    skipped.played_ms = 5_000;
    recorder.record_listen_event(&skipped).unwrap();

    assert_eq!(recorder.get_top_artists("all", 50).unwrap().len(), 50);
    let summary = recorder.get_stats_summary("all").unwrap();
    assert_eq!(summary.unique_artists, 60);
}

#[test]
fn test_summary_unique_artists_merge_credits_and_follow_the_range() {
    let (_, recorder) = setup_test_db();
    let now = Utc::now();
    let mut collab = listen(
        "c",
        "Song (feat. Rihanna)",
        (now - Duration::days(1)).to_rfc3339(),
    );
    collab.artist = "Calvin Harris".to_string();
    recorder.record_listen_event(&collab).unwrap();
    let mut solo = listen("s", "Other", (now - Duration::days(2)).to_rfc3339());
    solo.artist = "Rihanna".to_string();
    recorder.record_listen_event(&solo).unwrap();
    let mut old = listen("o", "Ancient", (now - Duration::days(100)).to_rfc3339());
    old.artist = "Old Timer".to_string();
    recorder.record_listen_event(&old).unwrap();

    assert_eq!(recorder.get_stats_summary("7d").unwrap().unique_artists, 2);
    assert_eq!(recorder.get_stats_summary("all").unwrap().unique_artists, 3);
}

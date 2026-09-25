use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use chrono::{Duration, Utc};

use crate::db::schema::get_migrations;
use crate::models::stats::{ListenEvent, ListenSource};
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
    assert_eq!(recorder.record_listen_event(&sub_second_event).unwrap(), false);
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
    assert_eq!(recorder.record_listen_event(&duplicate_event).unwrap(), false);
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
    recorder.record_listen_event(&ListenEvent {
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
    }).unwrap();

    // Event 2: Crate Local, 180s (3 mins)
    recorder.record_listen_event(&ListenEvent {
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
    }).unwrap();

    // Event 3: Rekordbox, 300s (5 mins)
    recorder.record_listen_event(&ListenEvent {
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
    }).unwrap();

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
    recorder.record_listen_event(&ListenEvent {
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
    }).unwrap();

    recorder.record_listen_event(&ListenEvent {
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
    }).unwrap();

    // Artist 2 - Track B played once
    recorder.record_listen_event(&ListenEvent {
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
    }).unwrap();

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
        recorder.record_listen_event(&ListenEvent {
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
        }).unwrap();
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
    recorder.record_listen_event(&ListenEvent {
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
    }).unwrap();

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

    let res = spotify_svc.import_streaming_history_json(endsong_json).unwrap();
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

    let res = spotify_svc.import_streaming_history_json(simple_json).unwrap();
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
fn test_listen_source_enum_conversion() {
    assert_eq!(ListenSource::Spotify.as_str(), "spotify");
    assert_eq!(ListenSource::CrateLocal.as_str(), "crate_local");
    assert_eq!(ListenSource::CrateBeatport.as_str(), "crate_beatport");
    assert_eq!(ListenSource::Rekordbox.as_str(), "rekordbox");
    assert_eq!(ListenSource::MixedInKey.as_str(), "mixed_in_key");
    assert_eq!(ListenSource::Other("custom".to_string()).as_str(), "custom");

    assert_eq!(ListenSource::from_str("spotify"), ListenSource::Spotify);
    assert_eq!(ListenSource::from_str("crate_local"), ListenSource::CrateLocal);
    assert_eq!(ListenSource::from_str("crate_beatport"), ListenSource::CrateBeatport);
    assert_eq!(ListenSource::from_str("rekordbox"), ListenSource::Rekordbox);
    assert_eq!(ListenSource::from_str("mixed_in_key"), ListenSource::MixedInKey);
    assert_eq!(ListenSource::from_str("custom"), ListenSource::Other("custom".to_string()));
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

    spotify_svc.set_client_id("test-custom-client-id-12345").unwrap();
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

    spotify_svc.set_client_secret("test-secret-abcdef123456").unwrap();
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
    assert_eq!(verifier.len(), 64, "Code verifier must be 64 characters long");
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

    assert_ne!(url_1, url_2, "Two auth URLs must have distinct states and challenges");

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
    assert_eq!(DEFAULT_SPOTIFY_REDIRECT_URI, "http://127.0.0.1:8888/callback");
}

#[test]
fn test_split_artists_and_multi_artist_aggregation() {
    let (_conn_arc, recorder) = setup_test_db();

    // 1. Test unit splitting
    let split_1 = StatsRecorderService::split_artists("Calvin Harris, Rihanna", "This Is What You Came For");
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
    assert!(rihanna.is_some(), "Rihanna must be aggregated from collaborations");
    let r = rihanna.unwrap();
    assert_eq!(r.plays, 2, "Rihanna must have 2 accumulated streams");
    assert_eq!(r.total_minutes, 9, "Rihanna must have 9 accumulated minutes");

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

#[test]
fn test_repair_spotify_historical_durations() {
    let (conn_arc, recorder) = setup_test_db();
    let conn = conn_arc.lock().unwrap();

    // Insert historical Spotify listen with 30s played_ms but 240s duration_ms
    conn.execute(
        r#"
        INSERT INTO listen_events (
            id, source, track_id, title, artist, album, duration_ms, played_ms, played_at
        ) VALUES ('hist1', 'spotify', 'sp_track1', 'My Song', 'My Artist', 'My Album', 240000, 30000, '2026-09-01T12:00:00Z')
        "#,
        [],
    ).unwrap();
    drop(conn);

    let repaired = recorder.repair_spotify_historical_durations().unwrap();
    assert_eq!(repaired, 1, "Should repair 1 historical Spotify event");

    let conn = conn_arc.lock().unwrap();
    let updated_played_ms: i64 = conn.query_row(
        "SELECT played_ms FROM listen_events WHERE id = 'hist1'",
        [],
        |r| r.get(0),
    ).unwrap();
    assert_eq!(updated_played_ms, 240000, "Historical played_ms must now match duration_ms");
}


/// Ordered schema migrations. Only the position in this list matters: the database records how
/// many have been applied. Never reorder, renumber or edit an entry that has shipped — append.
/// Entries 1–5 come from upstream, entries 6+ from the personal fork.
pub fn get_migrations() -> Vec<&'static str> {
    vec![
        // Migration 1: Initial schema
        r#"
-- Core tables
CREATE TABLE tracks (
    id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL UNIQUE,
    file_hash TEXT,

    -- Metadata (from ID3/Vorbis)
    title TEXT,
    artist TEXT,
    album TEXT,
    year INTEGER,
    genre TEXT,
    label TEXT,
    catalog_number TEXT,

    -- Audio properties
    duration_ms INTEGER NOT NULL,
    bpm REAL,
    key TEXT,
    bitrate INTEGER,
    sample_rate INTEGER,
    format TEXT,

    -- Analysis metadata
    analysis_source TEXT,
    waveform_data BLOB,

    -- Artwork
    artwork_path TEXT,
    artwork_source TEXT,

    -- User data
    rating INTEGER DEFAULT 0,
    play_count INTEGER DEFAULT 0,
    color TEXT,

    -- Timestamps
    date_added TEXT NOT NULL,
    date_modified TEXT NOT NULL,
    last_played TEXT,

    -- Rekordbox sync
    rekordbox_id TEXT,

    CONSTRAINT valid_rating CHECK (rating >= 0 AND rating <= 5)
);

CREATE INDEX idx_tracks_artist ON tracks(artist);
CREATE INDEX idx_tracks_bpm ON tracks(bpm);
CREATE INDEX idx_tracks_key ON tracks(key);
CREATE INDEX idx_tracks_date_added ON tracks(date_added);
CREATE INDEX idx_tracks_color ON tracks(color);

-- Tag system
CREATE TABLE tag_categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    color TEXT DEFAULT '#6366f1'
);

CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    category_id TEXT NOT NULL REFERENCES tag_categories(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    UNIQUE(category_id, name)
);

CREATE TABLE track_tags (
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (track_id, tag_id)
);

-- Playlists
CREATE TABLE playlists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id TEXT REFERENCES playlists(id) ON DELETE CASCADE,
    is_folder INTEGER NOT NULL DEFAULT 0,
    is_smart INTEGER NOT NULL DEFAULT 0,
    smart_rules TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    context TEXT NOT NULL DEFAULT 'library',
    date_created TEXT NOT NULL,
    date_modified TEXT NOT NULL
);

CREATE INDEX idx_playlists_context ON playlists(context);

CREATE TABLE playlist_tracks (
    playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    date_added TEXT NOT NULL,
    PRIMARY KEY (playlist_id, track_id)
);

-- Cue points
CREATE TABLE cues (
    id TEXT PRIMARY KEY,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    position_ms INTEGER NOT NULL,
    type TEXT NOT NULL,
    loop_end_ms INTEGER,
    hot_cue_index INTEGER,
    name TEXT,
    color TEXT,
    CONSTRAINT valid_type CHECK (type IN ('memory', 'hot', 'loop')),
    CONSTRAINT loop_has_end CHECK (type != 'loop' OR loop_end_ms IS NOT NULL)
);

CREATE INDEX idx_cues_track ON cues(track_id);

-- App settings
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Device export tracking
CREATE TABLE device_exports (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    device_name TEXT NOT NULL,
    playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    last_export_at TEXT NOT NULL,
    last_sync_at TEXT,
    sync_enabled INTEGER NOT NULL DEFAULT 1,
    UNIQUE(device_id, playlist_id)
);

CREATE INDEX idx_device_exports_device ON device_exports(device_id);
CREATE INDEX idx_device_exports_playlist ON device_exports(playlist_id);

CREATE TABLE device_tracks (
    device_id TEXT NOT NULL,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    usb_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    pdb_track_id INTEGER,
    metadata_hash TEXT,
    exported_at TEXT NOT NULL,
    PRIMARY KEY (device_id, track_id)
);

CREATE INDEX idx_device_tracks_device ON device_tracks(device_id);

-- Export checkpoints for resume support
CREATE TABLE export_checkpoints (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    device_name TEXT NOT NULL,
    started_at TEXT NOT NULL,
    state TEXT NOT NULL,
    playlist_ids TEXT NOT NULL,
    tracks_completed TEXT NOT NULL,
    tracks_failed TEXT NOT NULL,
    last_updated_at TEXT NOT NULL
);

CREATE INDEX idx_export_checkpoints_device ON export_checkpoints(device_id);

-- Discovery releases
CREATE TABLE discovery_releases (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL UNIQUE,
    source_type TEXT NOT NULL DEFAULT 'other',
    artist TEXT,
    title TEXT,
    label TEXT,
    release_date TEXT,
    artwork_url TEXT,
    artwork_path TEXT,
    notes TEXT,
    parent_url TEXT,
    date_added TEXT NOT NULL,
    date_modified TEXT NOT NULL
);

CREATE INDEX idx_discovery_releases_date_added ON discovery_releases(date_added);

CREATE TABLE discovery_tracks (
    id TEXT PRIMARY KEY,
    release_id TEXT NOT NULL REFERENCES discovery_releases(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    duration_ms INTEGER,
    video_id TEXT
);

CREATE INDEX idx_discovery_tracks_release ON discovery_tracks(release_id);

CREATE TABLE discovery_release_tags (
    release_id TEXT NOT NULL REFERENCES discovery_releases(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (release_id, tag_id)
);

CREATE TABLE playlist_discovery_releases (
    playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    release_id TEXT NOT NULL REFERENCES discovery_releases(id) ON DELETE CASCADE,
    position INTEGER,
    date_added TEXT,
    PRIMARY KEY (playlist_id, release_id)
);

-- Stream cache tables for preview playback
CREATE TABLE discovery_stream_cache (
    release_id     TEXT    NOT NULL REFERENCES discovery_releases(id) ON DELETE CASCADE,
    track_position INTEGER NOT NULL,
    stream_url     TEXT    NOT NULL,
    expires_at     TEXT    NOT NULL,
    proxy_ua       TEXT,
    PRIMARY KEY (release_id, track_position)
);

CREATE TABLE discovery_sc_client_id_cache (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    client_id  TEXT    NOT NULL,
    fetched_at TEXT    NOT NULL
);

CREATE TABLE discovery_audio_cache (
    release_id     TEXT    NOT NULL,
    track_position INTEGER NOT NULL,
    content_type   TEXT    NOT NULL DEFAULT 'audio/mpeg',
    file_size      INTEGER NOT NULL,
    cached_at      TEXT    NOT NULL,
    PRIMARY KEY (release_id, track_position)
);
"#,
        // Migration 2: Track-level likes for discovery releases
        r#"
ALTER TABLE discovery_tracks ADD COLUMN is_liked INTEGER NOT NULL DEFAULT 0;
"#,
        // Migration 3: Cloud-sync foundations — HLC columns, track rooting, indexes.
        // `library_roots` is created first so the `tracks.library_root_id` FK resolves.
        // `_hlc TEXT NOT NULL DEFAULT ''` back-fills existing rows with the "never stamped"
        // sentinel (which sorts below every real HLC). A REFERENCES column added via
        // ALTER TABLE must default to NULL (it does), which SQLite permits.
        r#"
CREATE TABLE library_roots (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    _hlc TEXT NOT NULL DEFAULT ''
);

ALTER TABLE tracks                      ADD COLUMN _hlc TEXT NOT NULL DEFAULT '';
ALTER TABLE playlists                   ADD COLUMN _hlc TEXT NOT NULL DEFAULT '';
ALTER TABLE playlist_tracks             ADD COLUMN _hlc TEXT NOT NULL DEFAULT '';
ALTER TABLE cues                        ADD COLUMN _hlc TEXT NOT NULL DEFAULT '';
ALTER TABLE tag_categories              ADD COLUMN _hlc TEXT NOT NULL DEFAULT '';
ALTER TABLE tags                        ADD COLUMN _hlc TEXT NOT NULL DEFAULT '';
ALTER TABLE track_tags                  ADD COLUMN _hlc TEXT NOT NULL DEFAULT '';
ALTER TABLE discovery_releases          ADD COLUMN _hlc TEXT NOT NULL DEFAULT '';
ALTER TABLE discovery_tracks            ADD COLUMN _hlc TEXT NOT NULL DEFAULT '';
ALTER TABLE discovery_release_tags      ADD COLUMN _hlc TEXT NOT NULL DEFAULT '';
ALTER TABLE playlist_discovery_releases ADD COLUMN _hlc TEXT NOT NULL DEFAULT '';

ALTER TABLE tracks ADD COLUMN library_root_id TEXT REFERENCES library_roots(id) ON DELETE SET NULL;
ALTER TABLE tracks ADD COLUMN relative_path   TEXT;

CREATE INDEX idx_tracks_hlc                      ON tracks(_hlc);
CREATE INDEX idx_playlists_hlc                   ON playlists(_hlc);
CREATE INDEX idx_playlist_tracks_hlc             ON playlist_tracks(_hlc);
CREATE INDEX idx_cues_hlc                        ON cues(_hlc);
CREATE INDEX idx_tag_categories_hlc              ON tag_categories(_hlc);
CREATE INDEX idx_tags_hlc                        ON tags(_hlc);
CREATE INDEX idx_track_tags_hlc                  ON track_tags(_hlc);
CREATE INDEX idx_discovery_releases_hlc          ON discovery_releases(_hlc);
CREATE INDEX idx_discovery_tracks_hlc            ON discovery_tracks(_hlc);
CREATE INDEX idx_discovery_release_tags_hlc      ON discovery_release_tags(_hlc);
CREATE INDEX idx_playlist_discovery_releases_hlc ON playlist_discovery_releases(_hlc);
CREATE INDEX idx_library_roots_hlc               ON library_roots(_hlc);
"#,
        // Migration 4: Cloud-sync bookkeeping. These tables are device-local — they are
        // never themselves serialized as sync buckets.
        r#"
-- Per-device mapping from a synced library_root to its local absolute folder.
CREATE TABLE IF NOT EXISTS sync_root_mappings (
    library_root_id     TEXT PRIMARY KEY,
    local_absolute_path TEXT NOT NULL
);

-- Hard-deleted rows to propagate. `entity_id` is the row's PK; composite junction
-- keys are encoded "a|b" in PK-declaration column order (see pipeline::dirty).
CREATE TABLE IF NOT EXISTS sync_tombstones (
    entity_type TEXT NOT NULL,
    entity_id   TEXT NOT NULL,
    _hlc        TEXT NOT NULL,
    PRIMARY KEY (entity_type, entity_id)
);
CREATE INDEX IF NOT EXISTS idx_sync_tombstones_hlc ON sync_tombstones(_hlc);

-- Buckets changed locally since the last successful push (drained on push).
CREATE TABLE IF NOT EXISTS sync_dirty_buckets (
    bucket    TEXT PRIMARY KEY,
    marked_at TEXT NOT NULL
);

-- Singleton key/value store for the sync engine (node_id, HLC clock, cursors, flags).
CREATE TABLE IF NOT EXISTS sync_state (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#,
        // Migration 5: Follow artists & labels.
        // `followed_sources`, `discovery_release_sources`, and the new
        // `discovery_releases` columns (`is_new`, `surfaced_at`) SYNC — they carry
        // `_hlc` and are registered as sync buckets (see pipeline::buckets). The
        // per-device watch bookkeeping (`followed_source_state`,
        // `followed_source_releases`) stays LOCAL and is never serialized as a bucket.
        r#"
-- SYNCED: the artists/labels the user follows.
CREATE TABLE followed_sources (
    id            TEXT PRIMARY KEY,
    url           TEXT NOT NULL UNIQUE,         -- normalize_url()'d page URL
    source_type   TEXT NOT NULL,                -- 'bandcamp' | 'soundcloud' | 'discogs'
    follow_type   TEXT NOT NULL DEFAULT 'artist', -- 'artist' | 'label'
    name          TEXT,
    artwork_url   TEXT,
    artwork_path  TEXT,
    enabled       INTEGER NOT NULL DEFAULT 1,    -- 0 = paused (syncs)
    date_added    TEXT NOT NULL,
    date_modified TEXT NOT NULL,
    _hlc          TEXT NOT NULL DEFAULT ''
);
CREATE INDEX idx_followed_sources_hlc ON followed_sources(_hlc);

-- LOCAL ONLY: per-device watch bookkeeping for each followed source.
CREATE TABLE followed_source_state (
    source_id            TEXT PRIMARY KEY REFERENCES followed_sources(id) ON DELETE CASCADE,
    last_checked_at      TEXT,
    last_success_at      TEXT,
    health               TEXT NOT NULL DEFAULT 'unknown', -- 'ok'|'error'|'rate_limited'|'unknown'
    last_error           TEXT,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    baseline_established INTEGER NOT NULL DEFAULT 0
);

-- LOCAL ONLY: every release URL ever seen under a source, with its disposition.
-- status: 'baseline' (present at follow-time, never surfaced),
--         'surfaced' (auto-added to discovery as new),
--         'dismissed' (user deleted it — tombstone so the watch loop never re-adds).
CREATE TABLE followed_source_releases (
    source_id            TEXT NOT NULL REFERENCES followed_sources(id) ON DELETE CASCADE,
    seen_url             TEXT NOT NULL,          -- normalize_url()'d release URL
    status               TEXT NOT NULL DEFAULT 'baseline',
    release_id           TEXT,                   -- discovery_releases.id when status='surfaced'
    release_day_notified INTEGER NOT NULL DEFAULT 0,
    first_seen_at        TEXT NOT NULL,
    PRIMARY KEY (source_id, seen_url)
);
CREATE INDEX idx_followed_source_releases_status ON followed_source_releases(source_id, status);

-- SYNCED: provenance (many-to-many). A release can be surfaced by an artist follow
-- AND a label follow; it appears once and cites both.
CREATE TABLE discovery_release_sources (
    release_id TEXT NOT NULL REFERENCES discovery_releases(id) ON DELETE CASCADE,
    source_id  TEXT NOT NULL REFERENCES followed_sources(id) ON DELETE CASCADE,
    _hlc       TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (release_id, source_id)
);
CREATE INDEX idx_discovery_release_sources_hlc ON discovery_release_sources(_hlc);

-- SYNCED columns on the existing discovery_releases table: 'new/unreviewed' flag
-- and when the watcher surfaced it (NULL for manually-added releases).
ALTER TABLE discovery_releases ADD COLUMN is_new INTEGER NOT NULL DEFAULT 0;
ALTER TABLE discovery_releases ADD COLUMN surfaced_at TEXT;
"#,
        // discovery_releases.source_page_url — the artist/label page a release was
        // discovered from. Bandcamp label discographies span many artist subdomains, so a
        // release's own URL host isn't the followed page; recording the scanned page lets a
        // label follow match every release imported from it. Synced.
        r#"
ALTER TABLE discovery_releases ADD COLUMN source_page_url TEXT;
"#,
        // Migration 6 (fork): Mixed In Key energy level support
        r#"
ALTER TABLE tracks ADD COLUMN energy INTEGER;
CREATE INDEX IF NOT EXISTS idx_tracks_energy ON tracks(energy);
"#,
        // Migration 7 (fork): Reset false analysis_source on tracks without genuine MIK cues or energy
        r#"
UPDATE tracks
SET analysis_source = NULL
WHERE analysis_source = 'mixed_in_key'
  AND energy IS NULL
  AND id NOT IN (SELECT DISTINCT track_id FROM cues);
"#,
        // Migration 8 (fork): Ignored duplicate track pairs
        r#"
CREATE TABLE ignored_duplicate_pairs (
    track_id_a TEXT NOT NULL,
    track_id_b TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (track_id_a, track_id_b)
);
CREATE INDEX IF NOT EXISTS idx_ignored_duplicate_pairs_b ON ignored_duplicate_pairs(track_id_b);
"#,
        // Migration 9 (fork): Recent standalone tracks
        r#"
CREATE TABLE recent_standalone_tracks (
    id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL UNIQUE,
    title TEXT,
    artist TEXT,
    album TEXT,
    duration_ms INTEGER NOT NULL,
    format TEXT,
    bitrate INTEGER,
    sample_rate INTEGER,
    bpm REAL,
    key TEXT,
    artwork_path TEXT,
    last_played_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_recent_standalone_tracks_last_played ON recent_standalone_tracks(last_played_at DESC);
"#,
        // Migration 10 (fork): Ignored upgrade matches
        r#"
CREATE TABLE ignored_upgrade_matches (
    track_id TEXT NOT NULL,
    beatport_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (track_id, beatport_id)
);
CREATE INDEX IF NOT EXISTS idx_ignored_upgrade_matches_bp ON ignored_upgrade_matches(beatport_id);
"#,
        // Migration 11 (fork): Beatport upgrade matches cache
        r#"
CREATE TABLE upgrade_matches_cache (
    track_id TEXT PRIMARY KEY,
    track_title TEXT NOT NULL,
    track_artist TEXT NOT NULL,
    file_path TEXT NOT NULL,
    beatport_track_json TEXT NOT NULL,
    confidence_score INTEGER NOT NULL,
    score_breakdown_json TEXT NOT NULL,
    scanned_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_upgrade_matches_cache_scanned_at ON upgrade_matches_cache(scanned_at);
"#,
        // Migration 12 (fork): Crate Pulse & Stats tables
        r#"
CREATE TABLE listen_events (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL, -- 'spotify', 'crate_local', 'crate_beatport', 'rekordbox'
    track_id TEXT,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album TEXT,
    duration_ms INTEGER NOT NULL,
    played_ms INTEGER NOT NULL,
    bpm REAL,
    key TEXT,
    energy INTEGER,
    format TEXT,
    artwork_url TEXT,
    played_at TEXT NOT NULL, -- ISO8601 UTC
    session_id TEXT,
    metadata_json TEXT
);
CREATE INDEX IF NOT EXISTS idx_listen_events_source_played ON listen_events(source, played_at DESC);
CREATE INDEX IF NOT EXISTS idx_listen_events_artist ON listen_events(artist, played_at DESC);
CREATE INDEX IF NOT EXISTS idx_listen_events_played_at ON listen_events(played_at DESC);

CREATE TABLE spotify_auth (
    id TEXT PRIMARY KEY,
    access_token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    user_id TEXT,
    user_name TEXT,
    is_connected INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE rekordbox_sessions (
    id TEXT PRIMARY KEY,
    session_name TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    total_tracks INTEGER DEFAULT 0,
    total_played_ms INTEGER DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_rekordbox_sessions_started ON rekordbox_sessions(started_at DESC);
"#,
        // Migration 13 (fork): Player Mode Album (album grid, tracks, covers)
        r#"
CREATE TABLE player_albums (
    id TEXT PRIMARY KEY,
    folder_path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    year INTEGER,
    genre TEXT,
    artwork_path TEXT,
    track_count INTEGER DEFAULT 0,
    total_duration_ms INTEGER DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_player_albums_created_at ON player_albums(created_at DESC);

CREATE TABLE player_album_tracks (
    id TEXT PRIMARY KEY,
    album_id TEXT NOT NULL REFERENCES player_albums(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL UNIQUE,
    track_number INTEGER,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    format TEXT,
    bitrate INTEGER,
    sample_rate INTEGER,
    bpm REAL,
    key TEXT,
    energy INTEGER,
    artwork_path TEXT
);
CREATE INDEX IF NOT EXISTS idx_player_album_tracks_album ON player_album_tracks(album_id, track_number ASC);
"#,
        // Migration 14 (fork): Fast file path lookups for tracks and recent standalone tracks
        r#"
CREATE INDEX IF NOT EXISTS idx_tracks_file_path ON tracks(file_path);
CREATE INDEX IF NOT EXISTS idx_recent_standalone_tracks_file_path ON recent_standalone_tracks(file_path);
"#,
        // Migration 15 (fork): SQLite FTS5 Full-Text Search for ultra-fast instant track search
        r#"
CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5(
    title,
    artist,
    album,
    genre,
    label,
    content='tracks',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER IF NOT EXISTS tracks_ai AFTER INSERT ON tracks BEGIN
  INSERT INTO tracks_fts(rowid, title, artist, album, genre, label)
  VALUES (new.rowid, new.title, new.artist, new.album, new.genre, new.label);
END;

CREATE TRIGGER IF NOT EXISTS tracks_ad AFTER DELETE ON tracks BEGIN
  INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album, genre, label)
  VALUES('delete', old.rowid, old.title, old.artist, old.album, old.genre, old.label);
END;

CREATE TRIGGER IF NOT EXISTS tracks_au AFTER UPDATE ON tracks BEGIN
  INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album, genre, label)
  VALUES('delete', old.rowid, old.title, old.artist, old.album, old.genre, old.label);
  INSERT INTO tracks_fts(rowid, title, artist, album, genre, label)
  VALUES (new.rowid, new.title, new.artist, new.album, new.genre, new.label);
END;

INSERT INTO tracks_fts(rowid, title, artist, album, genre, label)
SELECT rowid, title, artist, album, genre, label FROM tracks;
"#,
        // Migration 16 (fork): listens are matched to library tracks by artist and title (case and
        // surrounding spaces ignored) for the statistics criteria of smart playlists. Only the
        // Crate-local listens carry a track id; Rekordbox, Spotify and Mixed In Key ones do not.
        r#"
CREATE INDEX IF NOT EXISTS idx_listen_events_match
    ON listen_events(lower(trim(artist)), lower(trim(title)), played_at);
"#,
        // Migration 17 (fork): the same artist and title matching, from the library and the
        // discovery side (listening-history matching, discovery funnel, next-track suggestions).
        r#"
CREATE INDEX IF NOT EXISTS idx_tracks_match
    ON tracks(lower(trim(artist)), lower(trim(title)));
CREATE INDEX IF NOT EXISTS idx_discovery_releases_artist_match
    ON discovery_releases(lower(trim(artist)));
"#,
        // Migration 18 (fork): journal of the files moved by the assisted organisation, so that a
        // batch can be undone. Local to this device (not synced).
        r#"
CREATE TABLE organisation_journal (
    id TEXT PRIMARY KEY,
    batch_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    from_path TEXT NOT NULL,
    to_path TEXT NOT NULL,
    moved_at TEXT NOT NULL,
    undone_at TEXT
);
CREATE INDEX idx_organisation_journal_batch ON organisation_journal(batch_id);
"#,
        // Migration 19 (fork): beat grid kept from Crate's own analysis (CRA-177): the first beat
        // (ms) and a BPM with decimals, plus the tempo changes as a JSON array of
        // `[position_ms, bpm]` pairs when the tempo is not constant. NULL until a track is analysed
        // by Crate. Local analysis data like `waveform_data` (not synced, not backed up); the
        // displayed and exported BPM stays in `bpm`.
        r#"
ALTER TABLE tracks ADD COLUMN beatgrid_first_beat_ms REAL;
ALTER TABLE tracks ADD COLUMN beatgrid_bpm REAL;
ALTER TABLE tracks ADD COLUMN beatgrid_tempo_changes TEXT;
"#,
    ]
}

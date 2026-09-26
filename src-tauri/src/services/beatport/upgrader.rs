use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, LazyLock};
use tokio::sync::Mutex as TokioMutex;
use rusqlite::Connection;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;
use base64::Engine;

use crate::error::{CrateError, Result};
use crate::models::{
    UpgradeCountInfo, UpgradeMatch, UpgradeReplacementResult, UpgradeScanResult,
    UpgradeScoreBreakdown,
};
use crate::services::beatport::client::{BeatportClient, BeatportTrack};
use crate::services::beatport::downloader::{discard_staging, move_into_destination, BeatportDownloader};
use crate::services::duplicate::{artists_match, keys_match, normalize_artist, normalize_title, ARTIST_DELIM_RE};
use crate::services::library::LibraryService;

static FEAT_PAREN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*[\(\[](?:feat\.?|ft\.?|featuring)\s+[^)\]]+[\)\]]").unwrap()
});
static FEAT_INLINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s+\b(?:feat\.?|ft\.?|featuring)\b.*$").unwrap()
});
static MIX_PAREN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*[\(\[][^)\]]*(?:mix|edit|version|remaster|cut|dirty|clean|extended|short|outro|intro|dub|vip|remix|bootleg|mashup|acoustic|radio|club|original)[^)\]]*[\)\]]").unwrap()
});
static MIX_DASH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s+-\s+.*(?:mix|edit|version|remaster|cut|dirty|clean|extended|short|outro|intro|dub|vip|remix|bootleg|mashup|acoustic|radio|club|original|clean|dirty|explicit).*$").unwrap()
});
static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s+").unwrap()
});

/// Minimum confidence score for an upgrade proposal.
const MIN_CONFIDENCE: i32 = 75;

/// Extract the primary/main artist by splitting on delimiters (feat., ft., vs., with, &, etc.)
pub fn extract_main_artist(artist: &str) -> String {
    let s_nfc: String = artist.nfc().collect();
    let parts: Vec<&str> = ARTIST_DELIM_RE.split(&s_nfc).collect();
    if let Some(first) = parts.first() {
        let trimmed = first.trim().trim_matches(|c: char| c == '"' || c == '\'' || c == ',' || c == ';');
        let collapsed = WHITESPACE_RE.replace_all(trimmed, " ");
        return collapsed.trim().to_string();
    }
    artist.trim().to_string()
}

/// Clean track title by removing featurings, mix tags, and secondary dashes
pub fn clean_title(title: &str) -> String {
    let s_nfc: String = title.nfc().collect();
    let no_feat_paren = FEAT_PAREN_RE.replace_all(&s_nfc, "");
    let no_feat_inline = FEAT_INLINE_RE.replace_all(&no_feat_paren, "");
    let no_mix_paren = MIX_PAREN_RE.replace_all(&no_feat_inline, "");
    let no_mix_dash = MIX_DASH_RE.replace_all(&no_mix_paren, "");
    let collapsed = WHITESPACE_RE.replace_all(&no_mix_dash, " ");
    collapsed
        .trim()
        .trim_matches(|c: char| c == '-' || c == '_' || c == '/' || c == ':' || c == '"' || c == '\'')
        .trim()
        .to_string()
}

/// Generate cascade search queries for Beatport catalog search:
/// 1. Pass 1: Clean Main Artist + Clean Title (e.g. "David Guetta Sexy Bitch", "John Summit Resonate")
/// 2. Pass 2: Clean Title (if pass 1 returns nothing)
/// 3. Pass 3: Raw fallback query
pub fn build_search_queries(title: &str, artist: &str) -> Vec<String> {
    let main_art = extract_main_artist(artist);
    let cl_title = clean_title(title);

    let mut queries = Vec::new();

    // Pass 1: Clean Main Artist + Clean Title
    let pass1 = if !main_art.is_empty() && !cl_title.is_empty() {
        format!("{main_art} {cl_title}")
    } else if !cl_title.is_empty() {
        cl_title.clone()
    } else {
        main_art.clone()
    };

    if !pass1.trim().is_empty() {
        queries.push(pass1.trim().to_string());
    }

    // Pass 2: Clean Title
    if !cl_title.trim().is_empty() {
        let p2 = cl_title.trim().to_string();
        if !queries.contains(&p2) {
            queries.push(p2);
        }
    }

    // Pass 3: Raw query fallback
    let raw = if !artist.trim().is_empty() && !title.trim().is_empty() {
        format!("{} {}", artist.trim(), title.trim())
    } else if !title.trim().is_empty() {
        title.trim().to_string()
    } else {
        artist.trim().to_string()
    };

    if !raw.trim().is_empty() {
        let p3 = raw.trim().to_string();
        if !queries.contains(&p3) {
            queries.push(p3);
        }
    }

    queries
}

/// Helper to check if a JWT token is expired or about to expire (<90s)
fn is_jwt_expired(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() >= 2 {
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1])
            .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(parts[1]))
            .or_else(|_| base64::engine::general_purpose::STANDARD.decode(parts[1]));
        if let Ok(bytes) = decoded {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                if let Some(exp) = json.get("exp").and_then(|e| e.as_i64()) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    return now >= exp - 90;
                }
            }
        }
    }
    false
}

/// Calculate string similarity using character bigram Dice coefficient
pub fn dice_similarity(s1: &str, s2: &str) -> f64 {
    if s1.is_empty() && s2.is_empty() {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }
    if s1 == s2 {
        return 1.0;
    }

    let chars1: Vec<char> = s1.chars().collect();
    let chars2: Vec<char> = s2.chars().collect();

    if chars1.len() < 2 || chars2.len() < 2 {
        return if s1 == s2 { 1.0 } else { 0.0 };
    }

    let mut bigrams1 = Vec::with_capacity(chars1.len() - 1);
    for i in 0..(chars1.len() - 1) {
        bigrams1.push((chars1[i], chars1[i + 1]));
    }

    let mut bigrams2 = Vec::with_capacity(chars2.len() - 1);
    for i in 0..(chars2.len() - 1) {
        bigrams2.push((chars2[i], chars2[i + 1]));
    }

    let mut matches = 0;
    let mut used2 = vec![false; bigrams2.len()];

    for b1 in &bigrams1 {
        for (j, b2) in bigrams2.iter().enumerate() {
            if !used2[j] && b1 == b2 {
                matches += 1;
                used2[j] = true;
                break;
            }
        }
    }

    (2.0 * matches as f64) / (bigrams1.len() + bigrams2.len()) as f64
}

/// Calculate multi-criteria confidence score between a local track and a Beatport track.
/// Total possible: 100 points. Match threshold: >= 75 points.
/// Mix/version kinds mentioned in a title ("Extended Mix" → {extended}, "Radio Edit" → {radio}).
/// "Original Mix" and "Extended Mix" are treated as the same full-length kind.
fn mix_kinds(title: &str) -> HashSet<&'static str> {
    const KINDS: [(&str, &str); 9] = [
        ("extended", "full"),
        ("original mix", "full"),
        ("club mix", "full"),
        ("radio", "radio"),
        ("short", "radio"),
        ("dub", "dub"),
        ("instrumental", "instrumental"),
        ("acapella", "acapella"),
        ("vip", "vip"),
    ];
    let lower = title.to_lowercase();
    KINDS.iter().filter(|(needle, _)| lower.contains(needle)).map(|(_, kind)| *kind).collect()
}

pub fn calculate_confidence_score(
    local_title: &str,
    local_artist: &str,
    local_duration_ms: i64,
    local_bpm: Option<f64>,
    local_key: Option<&str>,
    bp: &BeatportTrack,
) -> (i32, UpgradeScoreBreakdown) {
    // 1. Title Score (max 40)
    let bp_full_title = match &bp.mix_name {
        Some(m) if !m.is_empty() => format!("{} ({})", bp.title, m),
        _ => bp.title.clone(),
    };

    let norm_local_title = normalize_title(local_title);
    let norm_bp_title = normalize_title(&bp_full_title);
    let norm_bp_title_only = normalize_title(&bp.title);

    let title_score: i32 = if norm_local_title == norm_bp_title || norm_local_title == norm_bp_title_only {
        40
    } else if norm_local_title.contains(&norm_bp_title_only) || norm_bp_title_only.contains(&norm_local_title) {
        35
    } else {
        let sim = dice_similarity(&norm_local_title, &norm_bp_title)
            .max(dice_similarity(&norm_local_title, &norm_bp_title_only));
        if sim >= 0.90 {
            38
        } else if sim >= 0.80 {
            32
        } else if sim >= 0.65 {
            25
        } else if sim >= 0.50 {
            18
        } else {
            (sim * 40.0).round() as i32
        }
    };

    // A different mix is a different recording: never swap an extended mix for a radio edit.
    let local_mix = mix_kinds(local_title);
    let bp_mix = mix_kinds(&bp_full_title);
    let title_score = if !local_mix.is_empty() && !bp_mix.is_empty() && local_mix.is_disjoint(&bp_mix) {
        0
    } else {
        title_score
    };

    // 2. Artist Score (max 30)
    let bp_artists_joined = bp
        .artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    let norm_local_artist = normalize_artist(local_artist);
    let norm_bp_artist = normalize_artist(&bp_artists_joined);

    let artist_score: i32 = if norm_local_artist == norm_bp_artist {
        30
    } else if artists_match(local_artist, &bp_artists_joined) {
        25
    } else if norm_local_artist.contains(&norm_bp_artist) || norm_bp_artist.contains(&norm_local_artist) {
        20
    } else {
        let sim = dice_similarity(&norm_local_artist, &norm_bp_artist);
        if sim >= 0.80 {
            22
        } else if sim >= 0.50 {
            15
        } else {
            (sim * 30.0).round() as i32
        }
    };

    // 3. Duration Score (max 15) — an unknown duration is neutral, never a match
    let dur_diff_sec = (local_duration_ms - bp.duration_ms).abs() / 1000;
    let duration_score: i32 = if local_duration_ms <= 0 || bp.duration_ms <= 0 {
        7
    } else if dur_diff_sec <= 2 {
        15
    } else if dur_diff_sec <= 5 {
        12
    } else if dur_diff_sec <= 10 {
        9
    } else if dur_diff_sec <= 30 {
        5
    } else if dur_diff_sec <= 60 {
        2
    } else {
        0
    };

    // 4. BPM Score (max 10)
    let bpm_score: i32 = match (local_bpm, bp.bpm) {
        (Some(b1), Some(b2)) if b1 > 0.0 && b2 > 0.0 => {
            let diff = (b1 - b2).abs();
            if diff <= 0.5 {
                10
            } else if diff <= 1.0 {
                8
            } else if diff <= 2.0 {
                5
            } else {
                0
            }
        }
        _ => 6, // Neutral score when BPM is unanalyzed
    };

    // 5. Key Score (max 5)
    let key_score: i32 = match (local_key, bp.key.as_deref()) {
        (Some(k1), Some(k2)) => {
            if keys_match(Some(k1), Some(k2)) {
                5
            } else {
                0
            }
        }
        _ => 3, // Neutral score when key is unanalyzed
    };

    let total_confidence = (title_score + artist_score + duration_score + bpm_score + key_score).clamp(0, 100);

    let breakdown = UpgradeScoreBreakdown {
        title_score,
        artist_score,
        duration_score,
        bpm_score,
        key_score,
    };

    (total_confidence, breakdown)
}

#[derive(Debug, Clone)]
struct LocalMp3Track {
    id: String,
    file_path: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration_ms: i64,
    bpm: Option<f64>,
    key: Option<String>,
    energy: Option<i32>,
    bitrate: Option<i32>,
    sample_rate: Option<i32>,
    format: String,
    artwork_path: Option<String>,
    file_size_bytes: u64,
}

/// Progress of an upgrade batch, sent to the UI before each track.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpgradeProgress {
    pub current: usize,
    pub total: usize,
    pub title: String,
}

pub struct BeatportUpgraderService {
    conn: Arc<Mutex<Connection>>,
    client: BeatportClient,
    scan_lock: Arc<TokioMutex<()>>,
}

impl BeatportUpgraderService {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            client: BeatportClient::new(),
            scan_lock: Arc::new(TokioMutex::new(())),
        }
    }

    /// Invalidate cached upgrade matches for given track IDs
    pub fn invalidate_cache_for_tracks(&self, track_ids: &[String]) -> Result<()> {
        if track_ids.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let placeholders: Vec<String> = track_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "DELETE FROM upgrade_matches_cache WHERE track_id IN ({})",
            placeholders.join(", ")
        );
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            track_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        conn.execute(&sql, params_refs.as_slice())?;
        Ok(())
    }

    /// Load all ignored upgrade pairs as `(track_id, beatport_id)`
    pub fn get_ignored_matches(&self, conn: &Connection) -> Result<HashSet<(String, String)>> {
        let mut stmt = conn.prepare("SELECT track_id, beatport_id FROM ignored_upgrade_matches")?;
        let rows = stmt.query_map([], |row| {
            let track_id: String = row.get(0)?;
            let bp_id: String = row.get(1)?;
            Ok((track_id, bp_id))
        })?;

        let mut set = HashSet::new();
        for r in rows {
            set.insert(r?);
        }
        Ok(set)
    }

    /// Add a track/beatport pair to ignored upgrade matches
    pub fn ignore_upgrade_match(&self, track_id: &str, beatport_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT OR IGNORE INTO ignored_upgrade_matches (track_id, beatport_id, created_at)
            VALUES (?1, ?2, ?3)
            "#,
            rusqlite::params![track_id, beatport_id, now],
        )?;
        // Remove from cache as well
        let _ = conn.execute(
            "DELETE FROM upgrade_matches_cache WHERE track_id = ?1",
            rusqlite::params![track_id],
        );
        Ok(())
    }

    /// Remove a track/beatport pair from ignored upgrade matches
    pub fn unignore_upgrade_match(&self, track_id: &str, beatport_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        conn.execute(
            "DELETE FROM ignored_upgrade_matches WHERE track_id = ?1 AND beatport_id = ?2",
            rusqlite::params![track_id, beatport_id],
        )?;
        Ok(())
    }

    /// Fetch all local MP3 tracks from Crate database
    fn get_local_mp3_tracks(&self, conn: &Connection) -> Result<Vec<LocalMp3Track>> {
        let sql = r#"
            SELECT
                id, file_path, title, artist, album,
                duration_ms, bpm, key, energy, bitrate, sample_rate,
                format, artwork_path
            FROM tracks
            WHERE lower(format) = 'mp3' OR lower(file_path) LIKE '%.mp3'
            ORDER BY date_added DESC
        "#;

        let mut stmt = conn.prepare(sql)?;
        let tracks = stmt
            .query_map([], |row| {
                let file_path: String = row.get(1)?;
                let file_size_bytes = Path::new(&file_path)
                    .metadata()
                    .map(|m| m.len())
                    .unwrap_or(0);

                Ok(LocalMp3Track {
                    id: row.get(0)?,
                    file_path,
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    album: row.get(4)?,
                    duration_ms: row.get(5)?,
                    bpm: row.get(6)?,
                    key: row.get(7)?,
                    energy: row.get(8)?,
                    bitrate: row.get(9)?,
                    sample_rate: row.get(10)?,
                    format: row.get(11)?,
                    artwork_path: row.get(12)?,
                    file_size_bytes,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tracks)
    }

    /// Loads persisted auth and ensures token is valid, auto-refreshing if expired
    pub async fn get_active_token(&self) -> Result<String> {
        let auth = BeatportClient::load_persisted_auth()
            .ok_or(CrateError::BeatportAuthRequired)?;

        if let Some(token) = auth.token {
            if is_jwt_expired(&token) {
                if let Some(ref rt) = auth.refresh_token {
                    log::info!("Beatport token expired, refreshing...");
                    match self.client.refresh_access_token(rt).await {
                        Ok(new_auth) => {
                            if let Some(new_tok) = new_auth.token {
                                return Ok(new_tok);
                            }
                        }
                        Err(e) => {
                            log::warn!("Beatport token refresh failed: {e}");
                            return Err(CrateError::BeatportAuthRequired);
                        }
                    }
                }
                return Err(CrateError::BeatportAuthRequired);
            }
            return Ok(token);
        }

        if let Some(ref rt) = auth.refresh_token {
            log::info!("No access token, obtaining from refresh token...");
            match self.client.refresh_access_token(rt).await {
                Ok(new_auth) => {
                    if let Some(new_tok) = new_auth.token {
                        return Ok(new_tok);
                    }
                }
                Err(e) => {
                    log::warn!("Beatport token refresh failed: {e}");
                    return Err(CrateError::BeatportAuthRequired);
                }
            }
        }

        Err(CrateError::BeatportAuthRequired)
    }

    /// Run intelligent library scan against Beatport catalogue to find FLAC lossless upgrade matches
    pub async fn find_upgrade_matches(&self, token: Option<&str>) -> Result<UpgradeScanResult> {
        // Protect with mutex to disallow concurrent scans
        let _scan_guard = self.scan_lock.lock().await;

        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(24)).to_rfc3339();
        // "No match on Beatport" is remembered for a week so each opening does not rescan everything.
        let negative_cutoff = (chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339();

        // 1. Read MP3 tracks, ignored matches, and cached matches from SQLite
        let (mp3_tracks, ignored, cached_map) = {
            let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            let tracks = self.get_local_mp3_tracks(&conn)?;
            let ignored = self.get_ignored_matches(&conn)?;

            let mut stmt = conn.prepare(
                r#"
                SELECT track_id, beatport_track_json, confidence_score, score_breakdown_json
                FROM upgrade_matches_cache
                WHERE (confidence_score >= ?2 AND scanned_at >= ?1) OR (confidence_score < ?2 AND scanned_at >= ?3)
                "#,
            )?;
            let cached_rows = stmt.query_map(rusqlite::params![cutoff, MIN_CONFIDENCE, negative_cutoff], |row| {
                let track_id: String = row.get(0)?;
                let bp_json: String = row.get(1)?;
                let confidence: i32 = row.get(2)?;
                let breakdown_json: String = row.get(3)?;
                Ok((track_id, (bp_json, confidence, breakdown_json)))
            })?;

            let mut cached = HashMap::new();
            for r in cached_rows {
                if let Ok((tid, val)) = r {
                    cached.insert(tid, val);
                }
            }

            (tracks, ignored, cached)
        };

        let total_eligible_mp3s = mp3_tracks.len();
        let mut matches = Vec::new();
        let mut tracks_needing_scan = Vec::new();

        // 2. Process valid cached matches (< 24h)
        for track in &mp3_tracks {
            let title = track.title.as_deref().unwrap_or("").trim();
            let artist = track.artist.as_deref().unwrap_or("").trim();

            if title.is_empty() {
                continue;
            }

            if let Some((bp_json, confidence, breakdown_json)) = cached_map.get(&track.id) {
                if *confidence < MIN_CONFIDENCE {
                    continue; // Recently searched, nothing on Beatport
                }
                if let (Ok(bp_track), Ok(breakdown)) = (
                    serde_json::from_str::<BeatportTrack>(bp_json),
                    serde_json::from_str::<UpgradeScoreBreakdown>(breakdown_json),
                ) {
                    if !ignored.contains(&(track.id.clone(), bp_track.id.clone())) && *confidence >= MIN_CONFIDENCE {
                        matches.push(UpgradeMatch {
                            track_id: track.id.clone(),
                            file_path: track.file_path.clone(),
                            current_format: track.format.clone(),
                            current_bitrate: track.bitrate,
                            current_sample_rate: track.sample_rate,
                            current_duration_ms: track.duration_ms,
                            current_bpm: track.bpm,
                            current_key: track.key.clone(),
                            current_energy: track.energy,
                            current_artwork_path: track.artwork_path.clone(),
                            current_file_size_bytes: track.file_size_bytes,
                            title: title.to_string(),
                            artist: artist.to_string(),
                            album: track.album.clone(),
                            beatport_track: bp_track,
                            confidence_score: *confidence,
                            score_breakdown: breakdown,
                            alternative_candidates: Vec::new(),
                        });
                        continue;
                    }
                }
            }

            tracks_needing_scan.push(track);
        }

        // 3. Perform network search for remaining tracks
        if !tracks_needing_scan.is_empty() {
            let resolved_token = match token {
                Some(t) if !t.trim().is_empty() => t.to_string(),
                _ => self.get_active_token().await?,
            };

            let mut network_count = 0;
            for track in tracks_needing_scan {
                let title = track.title.as_deref().unwrap_or("").trim();
                let artist = track.artist.as_deref().unwrap_or("").trim();

                // Throttling in batches of 4 tracks with 100ms micro-pause
                if network_count > 0 && network_count % 4 == 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                network_count += 1;

                let queries = build_search_queries(title, artist);
                let mut eligible_candidates: Vec<(i32, UpgradeScoreBreakdown, BeatportTrack)> = Vec::new();
                let mut seen_bp_ids: HashSet<String> = HashSet::new();
                let mut search_failed = false;

                for query in queries {
                    match self.search_with_backoff(&resolved_token, &query).await {
                        Ok(search_res) => {
                            for bp_track in search_res.tracks {
                                if seen_bp_ids.contains(&bp_track.id) || ignored.contains(&(track.id.clone(), bp_track.id.clone())) {
                                    continue;
                                }

                                let (confidence, breakdown) = calculate_confidence_score(
                                    title,
                                    artist,
                                    track.duration_ms,
                                    track.bpm,
                                    track.key.as_deref(),
                                    &bp_track,
                                );

                                if confidence >= MIN_CONFIDENCE {
                                    seen_bp_ids.insert(bp_track.id.clone());
                                    eligible_candidates.push((confidence, breakdown, bp_track));
                                }
                            }

                            // If we found eligible candidate(s) (>= 75%), we stop cascade queries
                            if !eligible_candidates.is_empty() {
                                break;
                            }
                        }
                        Err(e) => {
                            if e.contains("Authentification Beatport requise") {
                                return Err(CrateError::BeatportAuthRequired);
                            }
                            log::warn!("Beatport search failed for '{query}': {e}");
                            search_failed = true;
                        }
                    }
                }

                if eligible_candidates.is_empty() && !search_failed {
                    // Remember the absence of match (a network error is not remembered).
                    if let Ok(conn) = self.conn.lock() {
                        let _ = conn.execute(
                            r#"
                            INSERT INTO upgrade_matches_cache (
                                track_id, track_title, track_artist, file_path,
                                beatport_track_json, confidence_score, score_breakdown_json, scanned_at
                            ) VALUES (?1, ?2, ?3, ?4, 'null', 0, 'null', ?5)
                            ON CONFLICT(track_id) DO UPDATE SET
                                beatport_track_json = 'null', confidence_score = 0,
                                score_breakdown_json = 'null', scanned_at = excluded.scanned_at
                            "#,
                            rusqlite::params![track.id, title, artist, track.file_path, chrono::Utc::now().to_rfc3339()],
                        );
                    }
                }

                if !eligible_candidates.is_empty() {
                    // Sort descending by confidence score
                    eligible_candidates.sort_by(|a, b| b.0.cmp(&a.0));

                    let (best_score, best_breakdown, best_bp) = eligible_candidates.remove(0);
                    let alt_tracks: Vec<BeatportTrack> = eligible_candidates.into_iter().map(|(_, _, t)| t).collect();

                    // Persist primary match in cache
                    let now = chrono::Utc::now().to_rfc3339();
                    let bp_json = serde_json::to_string(&best_bp).unwrap_or_default();
                    let score_json = serde_json::to_string(&best_breakdown).unwrap_or_default();

                    if let Ok(conn) = self.conn.lock() {
                        let _ = conn.execute(
                            r#"
                            INSERT INTO upgrade_matches_cache (
                                track_id, track_title, track_artist, file_path,
                                beatport_track_json, confidence_score, score_breakdown_json, scanned_at
                            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                            ON CONFLICT(track_id) DO UPDATE SET
                                track_title = excluded.track_title,
                                track_artist = excluded.track_artist,
                                file_path = excluded.file_path,
                                beatport_track_json = excluded.beatport_track_json,
                                confidence_score = excluded.confidence_score,
                                score_breakdown_json = excluded.score_breakdown_json,
                                scanned_at = excluded.scanned_at
                            "#,
                            rusqlite::params![
                                track.id,
                                title,
                                artist,
                                track.file_path,
                                bp_json,
                                best_score,
                                score_json,
                                now,
                            ],
                        );
                    }

                    matches.push(UpgradeMatch {
                        track_id: track.id.clone(),
                        file_path: track.file_path.clone(),
                        current_format: track.format.clone(),
                        current_bitrate: track.bitrate,
                        current_sample_rate: track.sample_rate,
                        current_duration_ms: track.duration_ms,
                        current_bpm: track.bpm,
                        current_key: track.key.clone(),
                        current_energy: track.energy,
                        current_artwork_path: track.artwork_path.clone(),
                        current_file_size_bytes: track.file_size_bytes,
                        title: title.to_string(),
                        artist: artist.to_string(),
                        album: track.album.clone(),
                        beatport_track: best_bp,
                        confidence_score: best_score,
                        score_breakdown: best_breakdown,
                        alternative_candidates: alt_tracks,
                    });
                }
            }
        }

        // Sort by confidence score descending
        matches.sort_by(|a, b| b.confidence_score.cmp(&a.confidence_score));

        let potential_upgrades_count = matches.len();

        Ok(UpgradeScanResult {
            total_scanned: total_eligible_mp3s,
            total_eligible_mp3s,
            potential_upgrades_count,
            matches,
        })
    }

    /// Searches the Beatport catalog, waiting 1 s, 2 s then 4 s when rate-limited (HTTP 429).
    async fn search_with_backoff(
        &self,
        token: &str,
        query: &str,
    ) -> std::result::Result<crate::services::beatport::client::BeatportSearchResult, String> {
        let mut delay_ms = 1000;
        let mut attempt = 0;
        loop {
            match self.client.search_catalog_full(Some(token), query).await {
                Err(e) if e.contains("429") && attempt < 3 => {
                    log::info!("Beatport rate limit reached, retrying in {delay_ms} ms");
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms *= 2;
                    attempt += 1;
                }
                other => return other,
            }
        }
    }

    /// Alias for find_upgrade_matches
    pub async fn get_upgrade_matches(&self, token: Option<&str>) -> Result<UpgradeScanResult> {
        self.find_upgrade_matches(token).await
    }

    /// Fast count summary of potential upgrades in library (< 2ms, pure SQLite, 0 network)
    pub async fn get_upgrade_count(&self, _token: Option<&str>) -> Result<UpgradeCountInfo> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // 1. Total eligible MP3s in library
        let eligible_mp3_count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM tracks WHERE lower(format) = 'mp3' OR lower(file_path) LIKE '%.mp3'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;

        // 2. Count valid matches in cache (< 24h old, track still exists in tracks, and not ignored)
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(24)).to_rfc3339();
        let match_count: usize = conn
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM upgrade_matches_cache c
                INNER JOIN tracks t ON t.id = c.track_id
                WHERE c.scanned_at >= ?1
                  AND NOT EXISTS (
                      SELECT 1 FROM ignored_upgrade_matches i 
                      WHERE i.track_id = c.track_id
                  )
                "#,
                rusqlite::params![cutoff],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;

        Ok(UpgradeCountInfo {
            match_count,
            eligible_mp3_count,
        })
    }

    /// Replaces selected MP3 tracks with their FLAC version, one track at a time:
    /// 1. Download each candidate with beatportdl into a private staging folder (primary first,
    ///    then alternatives on failure); only files created by this run are considered
    /// 2. Keep a file only if its FLAC header is valid, it decodes completely and its duration
    ///    matches the Beatport track
    /// 3. Move it next to the old file without overwriting anything, then point the *existing*
    ///    Crate track at it (same id: cues, tags, playlists, rating and history are kept)
    /// 4. Move the old MP3 to the Trash only once the library points at the FLAC
    pub async fn execute_upgrade_replacements(
        &self,
        matches: &[UpgradeMatch],
        library: Option<&LibraryService>,
        destination_override: Option<&str>,
        on_progress: &(dyn Fn(UpgradeProgress) + Send + Sync),
    ) -> Result<UpgradeReplacementResult> {
        let library = library.ok_or_else(|| CrateError::Import("Library service unavailable".to_string()))?;
        let mut success_count = 0;
        let mut failed_count = 0;
        let mut replaced_tracks = Vec::new();
        let mut replaced_ids = Vec::new();
        let mut errors = Vec::new();

        let total = matches.len();
        for (index, item) in matches.iter().enumerate() {
            on_progress(UpgradeProgress { current: index + 1, total, title: item.title.clone() });
            // Never trust the path sent by the webview: read it from the library.
            let old_file_path_str = match library.get_track(&item.track_id) {
                Ok(track) => track.file_path,
                Err(e) => {
                    failed_count += 1;
                    errors.push(format!("'{}' : titre introuvable dans la bibliothèque ({e})", item.title));
                    continue;
                }
            };
            let old_path = Path::new(&old_file_path_str);

            let dest_dir = if let Some(custom) = destination_override {
                BeatportDownloader::expand_path(Path::new(custom))
            } else if let Some(parent) = old_path.parent() {
                parent.to_path_buf()
            } else {
                failed_count += 1;
                errors.push(format!("'{}' : dossier de destination introuvable", item.title));
                continue;
            };

            // Primary Beatport track first, then alternatives (403 territory restrictions, 404…)
            let mut candidates = vec![item.beatport_track.clone()];
            for alt in &item.alternative_candidates {
                if alt.id != item.beatport_track.id && !candidates.iter().any(|c| c.id == alt.id) {
                    candidates.push(alt.clone());
                }
            }

            let mut new_flac: Option<PathBuf> = None;
            let mut candidate_errors: Vec<String> = Vec::new();
            for (idx, candidate) in candidates.iter().enumerate() {
                log::info!(
                    "Attempting FLAC download for '{}' (candidate {}/{}: ID={})",
                    item.title,
                    idx + 1,
                    candidates.len(),
                    candidate.id
                );
                let staged = match BeatportDownloader::download_to_staging(std::slice::from_ref(candidate), &dest_dir, None).await {
                    Ok(staged) => staged,
                    Err(e) => {
                        candidate_errors.push(format!("ID {}: {e}", candidate.id));
                        continue;
                    }
                };
                let moved = match staged.valid_files.as_slice() {
                    [file] => move_into_destination(file, &dest_dir).map_err(|e| e.to_string()),
                    [] if staged.errors.is_empty() => Err("aucun fichier FLAC valide".to_string()),
                    [] => Err(staged.errors.join("; ")),
                    _ => Err(format!("{} fichiers reçus pour un seul titre", staged.valid_files.len())),
                };
                discard_staging(&staged.staging_dir);
                match moved {
                    Ok(path) => {
                        new_flac = Some(path);
                        break;
                    }
                    Err(e) => candidate_errors.push(format!("ID {}: {e}", candidate.id)),
                }
            }

            // Without a verified FLAC in place, the MP3 and its library entry are left untouched.
            let Some(new_flac) = new_flac else {
                failed_count += 1;
                errors.push(if candidate_errors.is_empty() {
                    format!("Aucun fichier FLAC valide n'a pu être téléchargé pour '{}'", item.title)
                } else {
                    format!("Échec téléchargement FLAC pour '{}' (candidats testés : {})", item.title, candidate_errors.join(" | "))
                });
                continue;
            };

            if let Err(e) = library.replace_track_file(&item.track_id, &new_flac) {
                failed_count += 1;
                errors.push(format!(
                    "'{}' : FLAC téléchargé ({}) mais la bibliothèque n'a pas pu être mise à jour : {e}. Le MP3 est conservé.",
                    item.title,
                    new_flac.display()
                ));
                continue;
            }

            if old_path.exists() && old_path != new_flac.as_path() {
                #[cfg(target_os = "macos")]
                {
                    let script = format!(
                        "tell application \"Finder\" to delete POSIX file \"{}\"",
                        old_file_path_str.replace('"', "\\\"")
                    );
                    let trashed = std::process::Command::new("osascript").arg("-e").arg(&script).output();
                    if !matches!(trashed, Ok(ref out) if out.status.success()) {
                        errors.push(format!("'{}' remplacé, mais l'ancien MP3 n'a pas pu être mis à la corbeille : {}", item.title, old_file_path_str));
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = std::fs::remove_file(old_path);
                }
            }

            success_count += 1;
            replaced_tracks.push(item.title.clone());
            replaced_ids.push(item.track_id.clone());
        }

        let _ = self.invalidate_cache_for_tracks(&replaced_ids);

        Ok(UpgradeReplacementResult {
            success_count,
            failed_count,
            replaced_tracks,
            errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::beatport::client::BeatportArtist;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE tracks (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL UNIQUE,
                file_hash TEXT,
                title TEXT,
                artist TEXT,
                album TEXT,
                year INTEGER,
                genre TEXT,
                label TEXT,
                catalog_number TEXT,
                duration_ms INTEGER NOT NULL,
                bpm REAL,
                key TEXT,
                energy INTEGER,
                bitrate INTEGER,
                sample_rate INTEGER,
                format TEXT NOT NULL,
                analysis_source TEXT,
                waveform_data BLOB,
                artwork_path TEXT,
                artwork_source TEXT,
                rating INTEGER DEFAULT 0,
                play_count INTEGER DEFAULT 0,
                color TEXT,
                date_added TEXT NOT NULL,
                date_modified TEXT NOT NULL,
                last_played TEXT,
                rekordbox_id TEXT,
                library_root_id TEXT,
                relative_path TEXT,
                _hlc TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE ignored_upgrade_matches (
                track_id TEXT NOT NULL,
                beatport_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (track_id, beatport_id)
            );

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
        )
        .unwrap();
        conn
    }

    fn sample_beatport_track() -> BeatportTrack {
        BeatportTrack {
            id: "123456".to_string(),
            title: "Opus".to_string(),
            mix_name: Some("Original Mix".to_string()),
            artists: vec![BeatportArtist {
                id: 1,
                name: "Eric Prydz".to_string(),
                slug: Some("eric-prydz".to_string()),
                image_url: None,
            }],
            remixers: None,
            genre: "Progressive House".to_string(),
            genre_id: Some(10),
            release_name: Some("Opus Album".to_string()),
            release_date: "2015-07-27".to_string(),
            duration_ms: 558000,
            duration_formatted: "9:18".to_string(),
            key: Some("10B".to_string()),
            bpm: Some(126.0),
            artwork_url: Some("https://example.com/art.jpg".to_string()),
            preview_url: Some("https://example.com/preview.mp3".to_string()),
            waveform_url: None,
            is_favorite: false,
            in_cart: false,
            beatport_url: Some("https://beatport.com/track/opus/123456".to_string()),
        }
    }

    #[test]
    fn test_extract_main_artist_keeps_names_containing_delimiters() {
        assert_eq!(extract_main_artist("Daft Punk"), "Daft Punk");
        assert_eq!(extract_main_artist("Alex Kennon"), "Alex Kennon");
        assert_eq!(extract_main_artist("Swedish House Mafia"), "Swedish House Mafia");
        assert_eq!(extract_main_artist("Andhim"), "Andhim");
        assert_eq!(extract_main_artist("Fisher ft. Aatig"), "Fisher");
        assert_eq!(extract_main_artist("Chris Lake feat. Aatig"), "Chris Lake");
        assert_eq!(extract_main_artist("Kaskade x Deadmau5"), "Kaskade");
        assert_eq!(extract_main_artist("Mau P vs Fisher"), "Mau P");
        assert_eq!(extract_main_artist("Sonny Fodera, Jazzy"), "Sonny Fodera");
    }

    #[test]
    fn test_confidence_penalizes_a_different_mix() {
        let mut radio = sample_beatport_track();
        radio.mix_name = Some("Radio Edit".to_string());
        radio.duration_ms = 558000;
        let (extended_vs_radio, _) =
            calculate_confidence_score("Opus (Extended Mix)", "Eric Prydz", 558000, Some(126.0), Some("10B"), &radio);
        let (extended_vs_original, _) = calculate_confidence_score(
            "Opus (Extended Mix)",
            "Eric Prydz",
            558000,
            Some(126.0),
            Some("10B"),
            &sample_beatport_track(),
        );
        assert!(extended_vs_radio < MIN_CONFIDENCE, "a radio edit must not replace an extended mix");
        assert!(extended_vs_original >= MIN_CONFIDENCE, "extended and original mix are both full length");
    }

    #[test]
    fn test_confidence_unknown_duration_is_neutral() {
        let mut bp = sample_beatport_track();
        bp.duration_ms = 0;
        let (_, breakdown) =
            calculate_confidence_score("Opus (Original Mix)", "Eric Prydz", 558000, Some(126.0), Some("10B"), &bp);
        assert_eq!(breakdown.duration_score, 7);
    }

    #[test]
    fn test_calculate_confidence_score_exact_match() {
        let bp = sample_beatport_track();
        let (score, breakdown) = calculate_confidence_score(
            "Opus (Original Mix)",
            "Eric Prydz",
            558000,
            Some(126.0),
            Some("10B"),
            &bp,
        );

        assert!(score >= 90, "Expected high confidence score, got {score}");
        assert_eq!(breakdown.title_score, 40);
        assert_eq!(breakdown.artist_score, 30);
        assert_eq!(breakdown.duration_score, 15);
        assert_eq!(breakdown.bpm_score, 10);
        assert_eq!(breakdown.key_score, 5);
        assert_eq!(score, 100);
    }

    #[test]
    fn test_calculate_confidence_score_close_match_without_bpm_key() {
        let bp = sample_beatport_track();
        let (score, breakdown) = calculate_confidence_score(
            "Opus",
            "Eric Prydz",
            559000, // 1s diff
            None,
            None,
            &bp,
        );

        assert!(score >= 75, "Expected >= 75% score, got {score}");
        assert_eq!(breakdown.title_score, 40);
        assert_eq!(breakdown.artist_score, 30);
        assert_eq!(breakdown.duration_score, 15);
        assert_eq!(breakdown.bpm_score, 6);
        assert_eq!(breakdown.key_score, 3);
        assert_eq!(score, 94);
    }

    #[test]
    fn test_calculate_confidence_score_mismatch() {
        let bp = sample_beatport_track();
        let (score, _) = calculate_confidence_score(
            "Levels",
            "Avicii",
            320000,
            Some(128.0),
            Some("4A"),
            &bp,
        );

        assert!(score < 75, "Expected mismatch score < 75%, got {score}");
    }

    #[test]
    fn test_ignore_and_unignore_upgrade_match() {
        let conn = Arc::new(Mutex::new(setup_test_db()));
        let service = BeatportUpgraderService::new(conn.clone());

        service.ignore_upgrade_match("track_1", "bp_123").unwrap();

        {
            let c = conn.lock().unwrap();
            let ignored = service.get_ignored_matches(&c).unwrap();
            assert!(ignored.contains(&("track_1".to_string(), "bp_123".to_string())));
        }

        service.unignore_upgrade_match("track_1", "bp_123").unwrap();

        {
            let c = conn.lock().unwrap();
            let ignored = service.get_ignored_matches(&c).unwrap();
            assert!(!ignored.contains(&("track_1".to_string(), "bp_123".to_string())));
        }
    }

    #[test]
    fn test_build_search_queries_sexy_bitch() {
        let queries = build_search_queries("Sexy Bitch (feat. Akon) (Club Mix)", "David Guetta feat. Akon");
        assert_eq!(queries[0], "David Guetta Sexy Bitch");
        assert_eq!(queries[1], "Sexy Bitch");
        assert!(queries.contains(&"David Guetta feat. Akon Sexy Bitch (feat. Akon) (Club Mix)".to_string()));
    }

    #[test]
    fn test_build_search_queries_resonate() {
        let queries = build_search_queries("Resonate (feat. Julia Church) (Extended Mix)", "John Summit & Sub Focus ft. Julia Church");
        assert_eq!(queries[0], "John Summit Resonate");
        assert_eq!(queries[1], "Resonate");
    }

    #[test]
    fn test_build_search_queries_ocean_drive() {
        let queries = build_search_queries("Ocean Drive (Original Mix)", "Duke Dumont feat. Boy Matthews");
        assert_eq!(queries[0], "Duke Dumont Ocean Drive");
        assert_eq!(queries[1], "Ocean Drive");
    }

    #[test]
    fn test_build_search_queries_ma_cherie() {
        let queries = build_search_queries("Ma Chérie (DJ Antoine vs Mad Mark 2k12 Radio Edit)", "DJ Antoine vs. Timati feat. Kalenna");
        assert_eq!(queries[0], "DJ Antoine Ma Chérie");
        assert_eq!(queries[1], "Ma Chérie");
    }

    #[test]
    fn test_build_search_queries_my_way() {
        let queries = build_search_queries("My Way", "Calvin Harris");
        assert_eq!(queries[0], "Calvin Harris My Way");
        assert_eq!(queries[1], "My Way");
    }

    #[tokio::test]
    async fn test_get_upgrade_count_fast_sqlite() {
        let conn = Arc::new(Mutex::new(setup_test_db()));
        let service = BeatportUpgraderService::new(conn.clone());

        // Insert mock mp3 track
        {
            let c = conn.lock().unwrap();
            c.execute(
                r#"
                INSERT INTO tracks (
                    id, file_path, title, artist, duration_ms, format, date_added, date_modified
                ) VALUES (
                    'track_mp3_1', '/path/to/track1.mp3', 'Opus', 'Eric Prydz', 558000, 'mp3', '2026-01-01', '2026-01-01'
                )
                "#,
                [],
            )
            .unwrap();

            // Insert valid cached match (< 24h)
            let now = chrono::Utc::now().to_rfc3339();
            let bp_json = serde_json::to_string(&sample_beatport_track()).unwrap();
            let breakdown_json = serde_json::to_string(&UpgradeScoreBreakdown {
                title_score: 40,
                artist_score: 30,
                duration_score: 15,
                bpm_score: 10,
                key_score: 5,
            })
            .unwrap();

            c.execute(
                r#"
                INSERT INTO upgrade_matches_cache (
                    track_id, track_title, track_artist, file_path,
                    beatport_track_json, confidence_score, score_breakdown_json, scanned_at
                ) VALUES (
                    'track_mp3_1', 'Opus', 'Eric Prydz', '/path/to/track1.mp3',
                    ?1, 100, ?2, ?3
                )
                "#,
                rusqlite::params![bp_json, breakdown_json, now],
            )
            .unwrap();
        }

        let count_info = service.get_upgrade_count(None).await.unwrap();
        assert_eq!(count_info.eligible_mp3_count, 1);
        assert_eq!(count_info.match_count, 1);

        // Test cache invalidation
        service.invalidate_cache_for_tracks(&["track_mp3_1".to_string()]).unwrap();
        let count_info_after = service.get_upgrade_count(None).await.unwrap();
        assert_eq!(count_info_after.eligible_mp3_count, 1);
        assert_eq!(count_info_after.match_count, 0);
    }
}

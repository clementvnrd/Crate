use regex::Regex;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};
use unicode_normalization::UnicodeNormalization;

use crate::error::{CrateError, Result};
use crate::models::{
    DuplicateCountInfo, DuplicateGroup, DuplicateMatchType, DuplicateScanResult, DuplicateTrackInfo,
};

static FEAT_PAREN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*[\(\[](?:feat\.?|ft\.?|featuring)\s+[^)\]]+[\)\]]").unwrap()
});
static FEAT_INLINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\s+\b(?:feat\.?|ft\.?|featuring)\b.*$").unwrap());
static MIX_PAREN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*[\(\[](?:[^\)\]]*\b)?(?:intro|outro|short(?:\s+edit|\s+mix|\s+cut)?|quick\s*hit|club|extended|original|radio|clean|dirty|explicit|dj\s*edit|dj\s*intro|re-?drum|transition|bootleg|mashup|slowed(?:\s*\+\s*reverb)?|sped\s*up|speed\s*up|acapella|a\s*cappella|bonus\s*track|instrumental|album|vocal|dub|vip|acoustic|remaster(?:ed)?|live|main|12\x22)(?:\s*(?:mix|edit|version|remaster|cut|dirty|clean|extended|short|outro|intro|dub|vip))?(?:\b[^\)\]]*)?[\)\]]").unwrap()
});
static MIX_DASH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s+-\s+(?:(?:intro|outro|short|quick\s*hit|club|extended|original|radio|clean|dirty|explicit|dj\s*edit|dj\s*intro|re-?drum|transition|bootleg|mashup|slowed(?:\s*\+\s*reverb)?|sped\s*up|speed\s*up|acapella|a\s*cappella|bonus\s*track|instrumental|album|vocal|dub|vip|acoustic|remaster(?:ed)?|live|main)(?:\s+(?:mix|edit|version|remaster|cut|dirty|clean|extended|short|outro|intro|dub|vip))?|clean|dirty|explicit)\s*$").unwrap()
});
/// Separators between artists. Words only match as whole words ("Daft Punk" is one artist,
/// "Alex" keeps its x): feat./ft./vs., featuring, with, and, a standalone "x", & + / ; ,
pub(crate) static ARTIST_DELIM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*(?:\b(?:feat|ft|vs)\b\.?|\b(?:featuring|with|and)\b|&|\+|/|;|,)\s*|\s+x\s+")
        .unwrap()
});
static PUNCT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\p{L}\p{N}\s]").unwrap());
static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// Check if a title contains a mix or version tag
pub fn has_version_tag(s: &str) -> bool {
    MIX_PAREN_RE.is_match(s) || MIX_DASH_RE.is_match(s)
}

/// Normalize track title for fuzzy duplicate detection.
pub fn normalize_title(s: &str) -> String {
    let s_nfc: String = s.nfc().collect();
    let no_feat_paren = FEAT_PAREN_RE.replace_all(&s_nfc, "");
    let no_feat_inline = FEAT_INLINE_RE.replace_all(&no_feat_paren, "");
    let no_mix_paren = MIX_PAREN_RE.replace_all(&no_feat_inline, "");
    let no_mix_dash = MIX_DASH_RE.replace_all(&no_mix_paren, "");
    let no_punct = PUNCT_RE.replace_all(&no_mix_dash, " ");
    let collapsed = WHITESPACE_RE.replace_all(&no_punct, " ");
    collapsed.trim().to_lowercase()
}

/// Normalize artist name for fuzzy duplicate detection.
pub fn normalize_artist(s: &str) -> String {
    let s_nfc: String = s.nfc().collect();
    let no_feat_paren = FEAT_PAREN_RE.replace_all(&s_nfc, "");
    let no_feat_inline = FEAT_INLINE_RE.replace_all(&no_feat_paren, "");
    let no_punct = PUNCT_RE.replace_all(&no_feat_inline, " ");
    let collapsed = WHITESPACE_RE.replace_all(&no_punct, " ");
    collapsed.trim().to_lowercase()
}

/// Extract normalized individual artist tokens for multi-artist set comparison.
pub fn normalize_artist_tokens(s: &str) -> HashSet<String> {
    let s_nfc: String = s.nfc().collect();
    let no_feat_paren = FEAT_PAREN_RE.replace_all(&s_nfc, "");
    let no_feat_inline = FEAT_INLINE_RE.replace_all(&no_feat_paren, "");

    let mut set = HashSet::new();
    for part in ARTIST_DELIM_RE.split(&no_feat_inline) {
        let no_punct = PUNCT_RE.replace_all(part, " ");
        let collapsed = WHITESPACE_RE.replace_all(&no_punct, " ");
        let clean = collapsed.trim().to_lowercase();
        if !clean.is_empty() {
            set.insert(clean);
        }
    }
    set
}

/// Compare two artist fields taking into account multi-artists and Jaccard similarity.
pub fn artists_match(a1: &str, a2: &str) -> bool {
    let n1 = normalize_artist(a1);
    let n2 = normalize_artist(a2);
    if n1.is_empty() && n2.is_empty() {
        return true;
    }
    if !n1.is_empty() && !n2.is_empty() && n1 == n2 {
        return true;
    }

    let tokens1 = normalize_artist_tokens(a1);
    let tokens2 = normalize_artist_tokens(a2);

    if tokens1.is_empty() && tokens2.is_empty() {
        return true;
    }
    if tokens1.is_empty() || tokens2.is_empty() {
        return false;
    }

    if tokens1 == tokens2 {
        return true;
    }

    let intersection_count = tokens1.intersection(&tokens2).count();
    let union_count = tokens1.union(&tokens2).count();

    if union_count > 0 {
        let jaccard = intersection_count as f64 / union_count as f64;
        if jaccard >= 0.5 {
            return true;
        }
    }

    if intersection_count > 0 && (tokens1.is_subset(&tokens2) || tokens2.is_subset(&tokens1)) {
        return true;
    }

    // Containment only counts whole words: "daft punk" contains "punk", not "da".
    if !n1.is_empty() && !n2.is_empty() {
        let (w1, w2) = (format!(" {n1} "), format!(" {n2} "));
        if w1.contains(&w2) || w2.contains(&w1) {
            return true;
        }
    }

    false
}

/// Normalize Camelot key string (e.g. "10B", "10b", "10B (G Major)" -> "10B")
pub fn normalize_camelot_key(key: &str) -> Option<String> {
    let clean = key.trim();
    let re = Regex::new(r"(?i)\b(1[0-2]|[1-9])([AB])\b").ok()?;
    if let Some(caps) = re.captures(clean) {
        let num = caps.get(1)?.as_str();
        let letter = caps.get(2)?.as_str().to_ascii_uppercase();
        return Some(format!("{num}{letter}"));
    }
    if !clean.is_empty() {
        return Some(clean.to_ascii_uppercase());
    }
    None
}

/// Check if two keys match
pub fn keys_match(k1: Option<&str>, k2: Option<&str>) -> bool {
    match (k1, k2) {
        (Some(key1), Some(key2)) => {
            let n1 = normalize_camelot_key(key1);
            let n2 = normalize_camelot_key(key2);
            match (n1, n2) {
                (Some(c1), Some(c2)) => !c1.is_empty() && c1 == c2,
                _ => false,
            }
        }
        _ => false,
    }
}

/// Calculate a quality score for ranking versions of a track.
#[allow(clippy::too_many_arguments)] // flat inputs keep every scoring criterion explicit at call sites
pub fn calculate_quality_score(
    format: &str,
    bitrate: Option<i32>,
    sample_rate: Option<i32>,
    file_size_bytes: u64,
    has_title: bool,
    has_artist: bool,
    has_album: bool,
    has_genre: bool,
    has_year: bool,
    has_label: bool,
    has_artwork: bool,
    has_key: bool,
    has_energy: bool,
    bpm: Option<f64>,
    cue_count: usize,
    rating: i32,
    play_count: i32,
) -> i32 {
    let mut score = 0;

    let fmt = format.to_lowercase();
    let is_lossless = matches!(
        fmt.as_str(),
        "flac" | "wav" | "wave" | "aiff" | "aif" | "alac"
    );

    if is_lossless {
        score += 1000;
    } else {
        let br = bitrate.unwrap_or(0);
        if br >= 320 {
            score += 500;
        } else if br >= 256 {
            score += 400;
        } else if br >= 192 {
            score += 300;
        } else if br >= 128 {
            score += 200;
        } else {
            score += 100;
        }
    }

    // Sample rate bonus (e.g. 44100 -> +44, 48000 -> +48, 96000 -> +96)
    if let Some(sr) = sample_rate {
        if sr > 0 {
            score += (sr / 1000).min(200);
        }
    }

    // Bitrate bonus (e.g. 320 kbps -> +32, 1411 kbps -> +141)
    if let Some(br) = bitrate {
        if br > 0 {
            score += (br / 10).min(200);
        }
    }

    // Cues bonus
    score += (cue_count * 25).min(150) as i32;

    // Mixed In Key & Analysis features
    if has_energy {
        score += 30;
    }
    if has_key {
        score += 20;
    }
    if bpm.filter(|b| *b > 0.0).is_some() {
        score += 10;
    }

    // Metadata richness
    if has_title {
        score += 10;
    }
    if has_artist {
        score += 10;
    }
    if has_album {
        score += 10;
    }
    if has_genre {
        score += 5;
    }
    if has_year {
        score += 5;
    }
    if has_label {
        score += 5;
    }
    if has_artwork {
        score += 20;
    }

    // User metadata
    score += rating.clamp(0, 5) * 10;
    score += play_count.clamp(0, 50);

    // File size bonus (e.g., 20 MB -> +20, max +50)
    score += ((file_size_bytes / (1024 * 1024)).min(50)) as i32;

    score
}

/// Raw record read from tracks and cues tables
#[derive(Debug, Clone)]
struct RawTrackRecord {
    id: String,
    file_path: String,
    file_hash: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<i32>,
    genre: Option<String>,
    label: Option<String>,
    duration_ms: i64,
    bpm: Option<f64>,
    key: Option<String>,
    energy: Option<i32>,
    bitrate: Option<i32>,
    sample_rate: Option<i32>,
    format: String,
    rating: i32,
    play_count: i32,
    date_added: String,
    artwork_path: Option<String>,
    cue_count: usize,
}

pub struct DuplicateService {
    conn: Arc<Mutex<Connection>>,
    /// Last count and the library fingerprint it was computed for (see `library_fingerprint`).
    count_cache: Mutex<Option<(String, DuplicateCountInfo)>>,
}

impl DuplicateService {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            count_cache: Mutex::new(None),
        }
    }

    /// Cheap summary of everything the duplicate scan depends on: any added, removed, edited or
    /// moved track, or any (un)ignored pair, changes it.
    fn library_fingerprint(&self) -> Result<String> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let fingerprint = conn.query_row(
            "SELECT COUNT(*) || ':' || COALESCE(MAX(rowid), 0) || ':' || COALESCE(MAX(date_modified), '') || ':'
                    || COALESCE(SUM(LENGTH(file_path)), 0) || ':'
                    || (SELECT COUNT(*) FROM ignored_duplicate_pairs)
             FROM tracks",
            [],
            |r| r.get::<_, String>(0),
        )?;
        Ok(fingerprint)
    }

    /// Load all ignored pairs as canonical tuples `(min(id_a, id_b), max(id_a, id_b))`
    fn get_ignored_pairs(&self, conn: &Connection) -> Result<HashSet<(String, String)>> {
        let mut stmt =
            conn.prepare("SELECT track_id_a, track_id_b FROM ignored_duplicate_pairs")?;
        let rows = stmt.query_map([], |row| {
            let a: String = row.get(0)?;
            let b: String = row.get(1)?;
            if a < b {
                Ok((a, b))
            } else {
                Ok((b, a))
            }
        })?;

        let mut ignored = HashSet::new();
        for r in rows {
            ignored.insert(r?);
        }
        Ok(ignored)
    }

    /// Load all tracks with their cue counts
    fn get_raw_tracks(&self, conn: &Connection) -> Result<Vec<RawTrackRecord>> {
        let sql = r#"
            SELECT
                t.id, t.file_path, t.file_hash,
                t.title, t.artist, t.album, t.year, t.genre, t.label,
                t.duration_ms, t.bpm, t.key, t.energy, t.bitrate, t.sample_rate, t.format,
                t.rating, t.play_count, t.date_added, t.artwork_path,
                (SELECT COUNT(*) FROM cues WHERE track_id = t.id) AS cue_count
            FROM tracks t
            ORDER BY t.date_added DESC
        "#;

        let mut stmt = conn.prepare(sql)?;
        let tracks = stmt
            .query_map([], |row| {
                Ok(RawTrackRecord {
                    id: row.get(0)?,
                    file_path: row.get(1)?,
                    file_hash: row.get(2)?,
                    title: row.get(3)?,
                    artist: row.get(4)?,
                    album: row.get(5)?,
                    year: row.get(6)?,
                    genre: row.get(7)?,
                    label: row.get(8)?,
                    duration_ms: row.get(9)?,
                    bpm: row.get(10)?,
                    key: row.get(11)?,
                    energy: row.get(12)?,
                    bitrate: row.get(13)?,
                    sample_rate: row.get(14)?,
                    format: row.get(15)?,
                    rating: row.get(16)?,
                    play_count: row.get(17)?,
                    date_added: row.get(18)?,
                    artwork_path: row.get(19)?,
                    cue_count: row.get::<_, i64>(20)? as usize,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tracks)
    }

    /// Run full duplicate detection and return detailed scan results
    pub fn get_duplicate_groups(&self) -> Result<DuplicateScanResult> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let ignored_pairs = self.get_ignored_pairs(&conn)?;
        let raw_tracks = self.get_raw_tracks(&conn)?;
        drop(conn);

        let n = raw_tracks.len();
        if n < 2 {
            return Ok(DuplicateScanResult {
                groups: Vec::new(),
                total_duplicate_tracks: 0,
                total_groups: 0,
                total_reclaimable_bytes: 0,
            });
        }

        // Disjoint-set union-find for clustering
        let mut parent: Vec<usize> = (0..n).collect();
        let mut edge_match_types: HashMap<(usize, usize), DuplicateMatchType> = HashMap::new();

        fn find_root(parent: &mut [usize], mut i: usize) -> usize {
            let mut root = i;
            while parent[root] != root {
                root = parent[root];
            }
            while parent[i] != root {
                let next = parent[i];
                parent[i] = root;
                i = next;
            }
            root
        }

        fn union_sets(parent: &mut [usize], i: usize, j: usize) {
            let root_i = find_root(parent, i);
            let root_j = find_root(parent, j);
            if root_i != root_j {
                parent[root_j] = root_i;
            }
        }

        // 1. Group by exact file_hash
        let mut hash_buckets: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, track) in raw_tracks.iter().enumerate() {
            if let Some(ref hash) = track.file_hash {
                if !hash.is_empty() {
                    hash_buckets.entry(hash.clone()).or_default().push(i);
                }
            }
        }

        for (_, indices) in hash_buckets {
            if indices.len() >= 2 {
                for a in 0..indices.len() {
                    for b in (a + 1)..indices.len() {
                        let idx1 = indices[a];
                        let idx2 = indices[b];
                        let id1 = &raw_tracks[idx1].id;
                        let id2 = &raw_tracks[idx2].id;
                        let canonical = if id1 < id2 {
                            (id1.clone(), id2.clone())
                        } else {
                            (id2.clone(), id1.clone())
                        };
                        if !ignored_pairs.contains(&canonical) {
                            union_sets(&mut parent, idx1, idx2);
                            let pair_key = if idx1 < idx2 {
                                (idx1, idx2)
                            } else {
                                (idx2, idx1)
                            };
                            edge_match_types.insert(pair_key, DuplicateMatchType::ExactHash);
                        }
                    }
                }
            }
        }

        // 2. Fuzzy metadata matching: bucket by normalized title
        let mut title_buckets: HashMap<String, Vec<usize>> = HashMap::new();
        let mut norm_titles: Vec<String> = Vec::with_capacity(n);

        for (i, track) in raw_tracks.iter().enumerate() {
            let n_title = normalize_title(track.title.as_deref().unwrap_or(""));
            if !n_title.is_empty() {
                title_buckets.entry(n_title.clone()).or_default().push(i);
            }
            norm_titles.push(n_title);
        }

        for (_, indices) in title_buckets {
            if indices.len() >= 2 {
                for a in 0..indices.len() {
                    for b in (a + 1)..indices.len() {
                        let idx1 = indices[a];
                        let idx2 = indices[b];

                        let root1 = find_root(&mut parent, idx1);
                        let root2 = find_root(&mut parent, idx2);
                        if root1 == root2 {
                            // Already clustered via exact hash
                            continue;
                        }

                        let id1 = &raw_tracks[idx1].id;
                        let id2 = &raw_tracks[idx2].id;
                        let canonical = if id1 < id2 {
                            (id1.clone(), id2.clone())
                        } else {
                            (id2.clone(), id1.clone())
                        };
                        if ignored_pairs.contains(&canonical) {
                            continue;
                        }

                        let t1 = &raw_tracks[idx1];
                        let t2 = &raw_tracks[idx2];

                        let art1 = t1.artist.as_deref().unwrap_or("");
                        let art2 = t2.artist.as_deref().unwrap_or("");

                        if !artists_match(art1, art2) {
                            continue;
                        }

                        // Check BPM match (+-1.0)
                        let bpm_matches = match (t1.bpm, t2.bpm) {
                            (Some(b1), Some(b2)) if b1 > 0.0 && b2 > 0.0 => (b1 - b2).abs() <= 1.0,
                            _ => false,
                        };

                        // Check Key match
                        let key_matches = keys_match(t1.key.as_deref(), t2.key.as_deref());

                        // Check if a version/mix tag has been extracted from either title
                        let has_version = has_version_tag(t1.title.as_deref().unwrap_or(""))
                            || has_version_tag(t2.title.as_deref().unwrap_or(""));

                        // Dynamic duration tolerance:
                        // - BPM match (+-1.0) OR Camelot Key match -> up to 90 seconds (90,000 ms)
                        // - BPM/Key not analyzed / not matching, but version tag extracted -> up to 60 seconds (60,000 ms)
                        // - Otherwise standard tolerance -> <= 10 seconds (10,000 ms)
                        let max_dur_diff_ms: i64 = if bpm_matches || key_matches {
                            90_000
                        } else if has_version {
                            60_000
                        } else {
                            10_000
                        };

                        let dur_diff = (t1.duration_ms - t2.duration_ms).abs();
                        if dur_diff > max_dur_diff_ms {
                            continue;
                        }

                        // If both have analyzed BPM and they differ by > 1.0, only allow if key matches or dur_diff <= 10_000
                        if let (Some(b1), Some(b2)) = (t1.bpm, t2.bpm) {
                            if b1 > 0.0
                                && b2 > 0.0
                                && (b1 - b2).abs() > 1.0
                                && !key_matches
                                && dur_diff > 10_000
                            {
                                continue;
                            }
                        }

                        union_sets(&mut parent, idx1, idx2);
                        let pair_key = if idx1 < idx2 {
                            (idx1, idx2)
                        } else {
                            (idx2, idx1)
                        };
                        edge_match_types.insert(pair_key, DuplicateMatchType::Metadata);
                    }
                }
            }
        }

        // Group indices by root
        let mut groups_map: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..n {
            let root = find_root(&mut parent, i);
            groups_map.entry(root).or_default().push(i);
        }

        let mut duplicate_groups: Vec<DuplicateGroup> = Vec::new();
        let mut total_reclaimable_bytes: u64 = 0;
        let mut total_duplicate_tracks: usize = 0;

        for (_, indices) in groups_map {
            if indices.len() < 2 {
                continue;
            }

            // Convert to DuplicateTrackInfo with quality scores
            let mut track_infos: Vec<DuplicateTrackInfo> = Vec::new();

            for &idx in &indices {
                let raw = &raw_tracks[idx];
                let file_size_bytes = Path::new(&raw.file_path)
                    .metadata()
                    .map(|m| m.len())
                    .unwrap_or_else(|_| {
                        // Estimate based on duration and bitrate if file metadata unavailable
                        let br = raw.bitrate.unwrap_or(320) as u64;
                        let dur_sec = (raw.duration_ms / 1000).max(0) as u64;
                        (br * 1000 / 8) * dur_sec
                    });

                let quality_score = calculate_quality_score(
                    &raw.format,
                    raw.bitrate,
                    raw.sample_rate,
                    file_size_bytes,
                    raw.title.as_ref().filter(|s| !s.is_empty()).is_some(),
                    raw.artist.as_ref().filter(|s| !s.is_empty()).is_some(),
                    raw.album.as_ref().filter(|s| !s.is_empty()).is_some(),
                    raw.genre.as_ref().filter(|s| !s.is_empty()).is_some(),
                    raw.year.is_some(),
                    raw.label.as_ref().filter(|s| !s.is_empty()).is_some(),
                    raw.artwork_path
                        .as_ref()
                        .filter(|s| !s.is_empty())
                        .is_some(),
                    raw.key.as_ref().filter(|s| !s.is_empty()).is_some(),
                    raw.energy.is_some(),
                    raw.bpm,
                    raw.cue_count,
                    raw.rating,
                    raw.play_count,
                );

                track_infos.push(DuplicateTrackInfo {
                    id: raw.id.clone(),
                    title: raw.title.clone(),
                    artist: raw.artist.clone(),
                    album: raw.album.clone(),
                    year: raw.year,
                    genre: raw.genre.clone(),
                    label: raw.label.clone(),
                    duration_ms: raw.duration_ms,
                    bpm: raw.bpm,
                    key: raw.key.clone(),
                    energy: raw.energy,
                    bitrate: raw.bitrate,
                    sample_rate: raw.sample_rate,
                    format: raw.format.clone(),
                    file_path: raw.file_path.clone(),
                    file_hash: raw.file_hash.clone(),
                    file_size_bytes,
                    cue_count: raw.cue_count,
                    rating: raw.rating,
                    play_count: raw.play_count,
                    artwork_path: raw.artwork_path.clone(),
                    date_added: raw.date_added.clone(),
                    quality_score,
                    recommended_keep: false,
                });
            }

            // Sort tracks: highest quality score first, tie-break by older date_added, then id
            track_infos.sort_by(|a, b| {
                b.quality_score
                    .cmp(&a.quality_score)
                    .then_with(|| a.date_added.cmp(&b.date_added))
                    .then_with(|| a.id.cmp(&b.id))
            });

            // Mark highest quality track as recommended_keep
            if let Some(first) = track_infos.first_mut() {
                first.recommended_keep = true;
            }

            // Calculate reclaimable bytes (sum of non-recommended tracks' file size)
            let mut group_reclaimable: u64 = 0;
            for t in track_infos.iter().skip(1) {
                group_reclaimable += t.file_size_bytes;
            }

            total_reclaimable_bytes += group_reclaimable;
            total_duplicate_tracks += track_infos.len() - 1;

            // Determine primary match type
            let has_exact_hash = track_infos
                .windows(2)
                .any(|w| w[0].file_hash.is_some() && w[0].file_hash == w[1].file_hash);

            let match_type = if has_exact_hash {
                DuplicateMatchType::ExactHash
            } else {
                DuplicateMatchType::Metadata
            };

            let group_id = format!("dup_{}", track_infos[0].id);

            duplicate_groups.push(DuplicateGroup {
                id: group_id,
                match_type,
                tracks: track_infos,
                reclaimable_bytes: group_reclaimable,
            });
        }

        // Sort groups: largest reclaimable bytes first
        duplicate_groups.sort_by_key(|g| std::cmp::Reverse(g.reclaimable_bytes));

        let total_groups = duplicate_groups.len();

        Ok(DuplicateScanResult {
            groups: duplicate_groups,
            total_duplicate_tracks,
            total_groups,
            total_reclaimable_bytes,
        })
    }

    /// Fast count of duplicate groups and reclaimable bytes
    /// Duplicate counters for the toolbar badge. The full scan only runs when the library changed
    /// since the last call (every `duplicates-updated` event used to trigger a full scan).
    pub fn get_duplicate_count(&self) -> Result<DuplicateCountInfo> {
        let fingerprint = self.library_fingerprint()?;
        if let Ok(cache) = self.count_cache.lock() {
            if let Some((cached_fp, info)) = cache.as_ref() {
                if *cached_fp == fingerprint {
                    return Ok(info.clone());
                }
            }
        }
        let scan = self.get_duplicate_groups()?;
        let info = DuplicateCountInfo {
            group_count: scan.total_groups,
            track_count: scan.total_duplicate_tracks,
            reclaimable_bytes: scan.total_reclaimable_bytes,
        };
        if let Ok(mut cache) = self.count_cache.lock() {
            *cache = Some((fingerprint, info.clone()));
        }
        Ok(info)
    }

    /// Ignore a group of tracks so they will no longer be flagged as duplicates of each other
    pub fn ignore_duplicate_group(&self, track_ids: Vec<String>) -> Result<()> {
        if track_ids.len() < 2 {
            return Ok(());
        }

        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let now = chrono::Utc::now().to_rfc3339();

        for i in 0..track_ids.len() {
            for j in (i + 1)..track_ids.len() {
                let id1 = &track_ids[i];
                let id2 = &track_ids[j];
                let (a, b) = if id1 < id2 { (id1, id2) } else { (id2, id1) };

                conn.execute(
                    r#"
                    INSERT OR IGNORE INTO ignored_duplicate_pairs (track_id_a, track_id_b, created_at)
                    VALUES (?1, ?2, ?3)
                    "#,
                    rusqlite::params![a, b, now],
                )?;
            }
        }

        Ok(())
    }

    /// Unignore a group of tracks so they will be checked for duplicates again
    pub fn unignore_duplicate_group(&self, track_ids: Vec<String>) -> Result<()> {
        if track_ids.len() < 2 {
            return Ok(());
        }

        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        for i in 0..track_ids.len() {
            for j in (i + 1)..track_ids.len() {
                let id1 = &track_ids[i];
                let id2 = &track_ids[j];
                let (a, b) = if id1 < id2 { (id1, id2) } else { (id2, id1) };

                conn.execute(
                    "DELETE FROM ignored_duplicate_pairs WHERE track_id_a = ?1 AND track_id_b = ?2",
                    rusqlite::params![a, b],
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

            CREATE TABLE cues (
                id TEXT PRIMARY KEY,
                track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
                position_ms INTEGER NOT NULL,
                type TEXT NOT NULL,
                loop_end_ms INTEGER,
                hot_cue_index INTEGER,
                name TEXT,
                color TEXT,
                _hlc TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE ignored_duplicate_pairs (
                track_id_a TEXT NOT NULL,
                track_id_b TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (track_id_a, track_id_b)
            );
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_normalize_title_and_artist() {
        assert_eq!(normalize_title("Strobe (Original Mix)"), "strobe");
        assert_eq!(normalize_title("Strobe [Extended Mix]"), "strobe");
        assert_eq!(normalize_title("Strobe (Radio Edit)"), "strobe");
        assert_eq!(normalize_title("One (feat. Pharrell)"), "one");
        assert_eq!(normalize_title("One ft. Pharrell Williams"), "one");
        assert_eq!(
            normalize_title("Café del Mar (Original Mix)"),
            "café del mar"
        );
        assert_eq!(normalize_artist("Avicii feat. Aloe Blacc"), "avicii");
        assert_eq!(normalize_artist("Deadmau5"), "deadmau5");
    }

    #[test]
    fn test_quality_score_comparison() {
        let flac_score = calculate_quality_score(
            "flac",
            Some(1411),
            Some(44100),
            30_000_000,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            Some(128.0),
            4,
            5,
            10,
        );

        let mp3_320_score = calculate_quality_score(
            "mp3",
            Some(320),
            Some(44100),
            10_000_000,
            true,
            true,
            true,
            false,
            false,
            false,
            false,
            true,
            false,
            Some(128.0),
            0,
            0,
            0,
        );

        let mp3_128_score = calculate_quality_score(
            "mp3",
            Some(128),
            Some(44100),
            4_000_000,
            true,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            0,
            0,
            0,
        );

        assert!(flac_score > mp3_320_score);
        assert!(mp3_320_score > mp3_128_score);
    }

    #[test]
    fn test_exact_hash_detection() {
        let conn = setup_test_db();
        conn.execute(
            "INSERT INTO tracks (id, file_path, file_hash, title, artist, duration_ms, format, date_added, date_modified)
             VALUES ('t1', '/path/1.mp3', 'hash123', 'Track A', 'Artist 1', 200000, 'mp3', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO tracks (id, file_path, file_hash, title, artist, duration_ms, format, date_added, date_modified)
             VALUES ('t2', '/path/2.mp3', 'hash123', 'Different Name', 'Different Artist', 200000, 'mp3', '2026-01-02T00:00:00Z', '2026-01-02T00:00:00Z')",
            [],
        ).unwrap();

        let svc = DuplicateService::new(Arc::new(Mutex::new(conn)));
        let res = svc.get_duplicate_groups().unwrap();

        assert_eq!(res.total_groups, 1);
        assert_eq!(res.groups[0].tracks.len(), 2);
        assert_eq!(res.groups[0].match_type, DuplicateMatchType::ExactHash);
    }

    #[test]
    fn test_duplicate_count_cache_follows_library_changes() {
        let conn = setup_test_db();
        conn.execute_batch(
            "INSERT INTO tracks (id, file_path, file_hash, title, artist, duration_ms, format, date_added, date_modified)
               VALUES ('t1', '/path/1.mp3', 'h', 'A', 'X', 200000, 'mp3', '2026-01-01', '2026-01-01');
             INSERT INTO tracks (id, file_path, file_hash, title, artist, duration_ms, format, date_added, date_modified)
               VALUES ('t2', '/path/2.mp3', 'h', 'B', 'Y', 200000, 'mp3', '2026-01-01', '2026-01-01');",
        )
        .unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let svc = DuplicateService::new(conn.clone());
        assert_eq!(svc.get_duplicate_count().unwrap().group_count, 1);
        assert_eq!(
            svc.get_duplicate_count().unwrap().group_count,
            1,
            "served from the cache"
        );
        conn.lock()
            .unwrap()
            .execute("DELETE FROM tracks WHERE id = 't2'", [])
            .unwrap();
        assert_eq!(
            svc.get_duplicate_count().unwrap().group_count,
            0,
            "a deletion invalidates the cache"
        );
    }

    #[test]
    fn test_fuzzy_metadata_detection_and_ignore() {
        let conn = setup_test_db();
        conn.execute(
            "INSERT INTO tracks (id, file_path, title, artist, duration_ms, bpm, bitrate, format, date_added, date_modified)
             VALUES ('t1', '/path/1.mp3', 'Strobe (Original Mix)', 'Deadmau5', 637000, 128.0, 320, 'mp3', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO tracks (id, file_path, title, artist, duration_ms, bpm, bitrate, format, date_added, date_modified)
             VALUES ('t2', '/path/2.flac', 'Strobe [Extended Mix]', 'Deadmau5', 638000, 128.0, 1411, 'flac', '2026-01-02T00:00:00Z', '2026-01-02T00:00:00Z')",
            [],
        ).unwrap();

        let svc = DuplicateService::new(Arc::new(Mutex::new(conn)));
        let res = svc.get_duplicate_groups().unwrap();

        assert_eq!(res.total_groups, 1);
        assert_eq!(res.groups[0].tracks.len(), 2);
        assert_eq!(res.groups[0].match_type, DuplicateMatchType::Metadata);
        // FLAC should be recommended keep
        assert_eq!(res.groups[0].tracks[0].id, "t2");
        assert!(res.groups[0].tracks[0].recommended_keep);
        assert!(!res.groups[0].tracks[1].recommended_keep);

        // Test ignore
        svc.ignore_duplicate_group(vec!["t1".to_string(), "t2".to_string()])
            .unwrap();
        let res_after_ignore = svc.get_duplicate_groups().unwrap();
        assert_eq!(res_after_ignore.total_groups, 0);

        // Test unignore
        svc.unignore_duplicate_group(vec!["t1".to_string(), "t2".to_string()])
            .unwrap();
        let res_after_unignore = svc.get_duplicate_groups().unwrap();
        assert_eq!(res_after_unignore.total_groups, 1);
    }

    #[test]
    fn test_jamie_t_fred_again_intro_edit_detection() {
        let conn = setup_test_db();
        // Track 1: Jamie T, Fred again.. - Lights Burn Dimmer (4:20 = 260_000 ms, BPM 106.0, Key 10B)
        conn.execute(
            "INSERT INTO tracks (id, file_path, title, artist, duration_ms, bpm, key, bitrate, format, date_added, date_modified)
             VALUES ('t1', '/path/1.mp3', 'Lights Burn Dimmer', 'Jamie T, Fred again..', 260000, 106.0, '10B', 320, 'mp3', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        ).unwrap();

        // Track 2: Fred again.., Jamie T - Lights Burn Dimmer (Intro Edit) (4:36 = 276_000 ms, BPM 106.0, Key 10B)
        conn.execute(
            "INSERT INTO tracks (id, file_path, title, artist, duration_ms, bpm, key, bitrate, format, date_added, date_modified)
             VALUES ('t2', '/path/2.wav', 'Lights Burn Dimmer (Intro Edit)', 'Fred again.., Jamie T', 276000, 106.0, '10B', 1411, 'wav', '2026-01-02T00:00:00Z', '2026-01-02T00:00:00Z')",
            [],
        ).unwrap();

        let svc = DuplicateService::new(Arc::new(Mutex::new(conn)));
        let res = svc.get_duplicate_groups().unwrap();

        assert_eq!(res.total_groups, 1);
        assert_eq!(res.groups[0].tracks.len(), 2);
        assert_eq!(res.groups[0].match_type, DuplicateMatchType::Metadata);
        // WAV / higher bitrate should be recommended keep
        assert_eq!(res.groups[0].tracks[0].id, "t2");
        assert!(res.groups[0].tracks[0].recommended_keep);
        assert!(!res.groups[0].tracks[1].recommended_keep);
    }

    #[test]
    fn test_extended_mix_tags_and_multi_artists() {
        assert_eq!(normalize_title("Song Name (Intro)"), "song name");
        assert_eq!(normalize_title("Song Name (Outro)"), "song name");
        assert_eq!(normalize_title("Song Name (Short Edit)"), "song name");
        assert_eq!(normalize_title("Song Name (Quick Hit)"), "song name");
        assert_eq!(normalize_title("Song Name (DJ Edit)"), "song name");
        assert_eq!(normalize_title("Song Name (Re-Drum)"), "song name");
        assert_eq!(normalize_title("Song Name (Transition)"), "song name");
        assert_eq!(normalize_title("Song Name (Bootleg)"), "song name");
        assert_eq!(normalize_title("Song Name (Mashup)"), "song name");
        assert_eq!(normalize_title("Song Name (Slowed + Reverb)"), "song name");
        assert_eq!(normalize_title("Song Name (Sped Up)"), "song name");
        assert_eq!(normalize_title("Song Name (Acapella)"), "song name");
        assert_eq!(normalize_title("Song Name (Bonus Track)"), "song name");
        assert_eq!(normalize_title("Song Name - Clean"), "song name");
        assert_eq!(normalize_title("Song Name - Dirty"), "song name");
        assert_eq!(normalize_title("Song Name - Extended Mix"), "song name");

        assert!(artists_match(
            "Jamie T, Fred again..",
            "Fred again.., Jamie T"
        ));
        assert!(artists_match(
            "Jamie T & Fred again..",
            "Fred again.., Jamie T"
        ));
        assert!(artists_match(
            "Skrillex & Fred again.. feat. Flowdan",
            "Fred again.., Skrillex, Flowdan"
        ));
    }
}

#[cfg(test)]
mod artist_delimiter_tests {
    use super::*;

    #[test]
    fn test_artist_tokens_do_not_split_inside_names() {
        let tokens = normalize_artist_tokens("Daft Punk");
        assert_eq!(tokens.len(), 1);
        assert!(tokens.contains("daft punk"));
        assert_eq!(normalize_artist_tokens("Alex Kennon x Maxinne").len(), 2);
        assert!(artists_match("Daft Punk", "Daft Punk"));
        assert!(!artists_match("Daft Punk", "Da"));
    }
}

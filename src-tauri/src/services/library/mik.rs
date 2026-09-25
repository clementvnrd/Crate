use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use lofty::config::{ParseOptions, ParsingMode};
use lofty::file::TaggedFile;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{ItemKey, ItemValue, Tag};
use regex::Regex;
use rusqlite::Connection;

use crate::error::{CrateError, Result};
use crate::models::{Cue, Track};
use crate::services::cloud_sync::pipeline::{buckets, dirty};

#[derive(Debug, Clone, Default)]
pub struct MikAnalysisData {
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub energy: Option<i32>,
    pub cues: Vec<Cue>,
    pub is_mik: bool,
}

pub struct MikService;

impl MikService {
    /// Read audio file metadata with lenient parsing options
    pub fn read_metadata_lenient(path: &Path) -> Option<TaggedFile> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);

        let parse_options = ParseOptions::new()
            .parsing_mode(ParsingMode::Relaxed)
            .max_junk_bytes(8192);

        Probe::new(reader)
            .options(parse_options)
            .guess_file_type()
            .ok()?
            .read()
            .ok()
    }

    /// Extract all Mixed In Key / file tag analysis data from a TaggedFile
    pub fn extract_analysis_data(tagged_file: &TaggedFile, track_id: &str) -> MikAnalysisData {
        let mut data = MikAnalysisData::default();

        if let Some(tag) = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())
        {
            data.bpm = Self::extract_bpm(tag);
            data.key = Self::extract_key(tag);
            data.energy = Self::extract_energy(tag);

            // Check if this appears to be genuinely analyzed by Mixed In Key
            let has_mik_comment = tag
                .get_string(&ItemKey::Comment)
                .map(|c| c.contains("Energy ") || c.contains("Mixed In Key") || c.contains("MIK"))
                .unwrap_or(false);

            data.is_mik = has_mik_comment;
        }

        // Extract any embedded Serato / MIK cue points
        data.cues = Self::extract_cues(tagged_file, track_id);
        if !data.cues.is_empty() {
            data.is_mik = true;
        }

        data
    }

    /// Extract BPM from audio tags
    pub fn extract_bpm(tag: &Tag) -> Option<f64> {
        // 1. Direct BPM tag (TBPM / tmpo / BPM)
        if let Some(bpm_str) = tag.get_string(&ItemKey::Bpm) {
            if let Ok(bpm) = bpm_str.trim().parse::<f64>() {
                if (20.0..300.0).contains(&bpm) {
                    return Some(bpm);
                }
            }
        }

        // 2. Check all items for custom BPM / Tempo keys
        for item in tag.items() {
            if let ItemKey::Unknown(ref k) = item.key() {
                if k.eq_ignore_ascii_case("bpm") || k.eq_ignore_ascii_case("tempo") {
                    if let ItemValue::Text(ref val) = item.value() {
                        if let Ok(bpm) = val.trim().parse::<f64>() {
                            if (20.0..300.0).contains(&bpm) {
                                return Some(bpm);
                            }
                        }
                    }
                }
            }
        }

        // 3. Fallback: Parse BPM from comment tag (e.g., "11A - 128 - Energy 7" or "128.00 BPM")
        let comment_str = tag
            .get_string(&ItemKey::Comment)
            .map(|s| s.to_string())
            .or_else(|| tag.comment().map(|c| c.into_owned()));

        if let Some(ref comment) = comment_str {
            let re = Regex::new(r"(?i)\b(\d{2,3}(?:\.\d{1,2})?)\s*(?:bpm)?\b").ok()?;
            if let Some(caps) = re.captures(comment) {
                if let Some(m) = caps.get(1) {
                    if let Ok(bpm) = m.as_str().parse::<f64>() {
                        if (30.0..260.0).contains(&bpm) {
                            return Some(bpm);
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract Key from audio tags (Camelot or standard musical notation)
    pub fn extract_key(tag: &Tag) -> Option<String> {
        // 1. Direct InitialKey tag (TKEY / INITIALKEY)
        if let Some(key_str) = tag.get_string(&ItemKey::InitialKey) {
            let clean = key_str.trim();
            if !clean.is_empty() {
                return Some(Self::normalize_key(clean));
            }
        }

        // 2. Check all items for custom Key keys
        for item in tag.items() {
            if let ItemKey::Unknown(ref k) = item.key() {
                if k.eq_ignore_ascii_case("initialkey")
                    || k.eq_ignore_ascii_case("tkey")
                    || k.eq_ignore_ascii_case("key")
                {
                    if let ItemValue::Text(ref val) = item.value() {
                        let clean = val.trim();
                        if !clean.is_empty() {
                            return Some(Self::normalize_key(clean));
                        }
                    }
                }
            }
        }

        // 3. Fallback: Parse Camelot key from comment tag (e.g. "11A - Energy 7" or "8B")
        let comment_str = tag
            .get_string(&ItemKey::Comment)
            .map(|s| s.to_string())
            .or_else(|| tag.comment().map(|c| c.into_owned()));

        if let Some(ref comment) = comment_str {
            let camelot_re = Regex::new(r"\b(1[0-2]|[1-9])([ABab])\b").ok()?;
            if let Some(caps) = camelot_re.captures(comment) {
                let num = caps.get(1)?.as_str();
                let letter = caps.get(2)?.as_str().to_ascii_uppercase();
                return Some(format!("{num}{letter}"));
            }
        }

        None
    }

    /// Extract Energy Level (1-10) written by Mixed In Key
    pub fn extract_energy(tag: &Tag) -> Option<i32> {
        // 1. Check custom tag frames (TXXX:EnergyLevel, TXXX:ENERGY, Vorbis ENERGYLEVEL)
        for item in tag.items() {
            if let ItemKey::Unknown(ref k) = item.key() {
                if k.eq_ignore_ascii_case("energylevel")
                    || k.eq_ignore_ascii_case("energy level")
                    || k.eq_ignore_ascii_case("energy")
                    || k.eq_ignore_ascii_case("txxx:energylevel")
                {
                    if let ItemValue::Text(ref val) = item.value() {
                        if let Ok(energy) = val.trim().parse::<i32>() {
                            if (1..=10).contains(&energy) {
                                return Some(energy);
                            }
                        }
                    }
                }
            }
        }

        // 2. Parse from comment tag (e.g. "11A - Energy 7", "Energy: 8", "Energy 6")
        let comment_str = tag
            .get_string(&ItemKey::Comment)
            .map(|s| s.to_string())
            .or_else(|| tag.comment().map(|c| c.into_owned()));

        if let Some(ref comment) = comment_str {
            let energy_re =
                Regex::new(r"(?i)(?:energy\s*(?:level)?\s*[:\-]?\s*|energy\s+)(\d{1,2})\b").ok()?;
            if let Some(caps) = energy_re.captures(comment) {
                if let Some(m) = caps.get(1) {
                    if let Ok(energy) = m.as_str().parse::<i32>() {
                        if (1..=10).contains(&energy) {
                            return Some(energy);
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract cue points / Serato markers written by Mixed In Key
    pub fn extract_cues(tagged_file: &TaggedFile, track_id: &str) -> Vec<Cue> {
        let mut cues = Vec::new();

        for tag in tagged_file.tags() {
            for item in tag.items() {
                let matches_serato = match item.key() {
                    ItemKey::Unknown(ref k) => {
                        k.contains("Serato Markers") || k.contains("GEOB")
                    }
                    _ => false,
                };

                if matches_serato {
                    if let ItemValue::Binary(ref bytes) = item.value() {
                        let parsed = Self::parse_serato_markers(bytes, track_id);
                        if !parsed.is_empty() {
                            cues.extend(parsed);
                        }
                    }
                }
            }
        }

        cues
    }

    /// Parse binary Serato Markers2 data
    fn parse_serato_markers(data: &[u8], track_id: &str) -> Vec<Cue> {
        let mut cues = Vec::new();
        if data.is_empty() {
            return cues;
        }

        // Serato Markers2 data is usually base64 encoded with a prefix
        let payload = if data.starts_with(b"\x01\x01") && data.len() > 2 {
            &data[2..]
        } else {
            data
        };

        // Try base64 decoding if ASCII
        let decoded = if let Ok(s) = std::str::from_utf8(payload) {
            let clean_s = s.trim_matches(|c: char| c.is_control() || c.is_whitespace());
            match base64_decode(clean_s) {
                Some(b) => b,
                None => payload.to_vec(),
            }
        } else {
            payload.to_vec()
        };

        // Search for "CUE\0" entries in the binary stream
        let mut i = 0;
        while i + 12 <= decoded.len() {
            if &decoded[i..i + 4] == b"CUE\0" {
                let length = u32::from_be_bytes([
                    decoded[i + 4],
                    decoded[i + 5],
                    decoded[i + 6],
                    decoded[i + 7],
                ]) as usize;

                let entry_start = i + 8;
                let entry_end = entry_start + length;

                if entry_end <= decoded.len() && length >= 5 {
                    let index = decoded[entry_start];
                    let pos_ms = u32::from_be_bytes([
                        decoded[entry_start + 1],
                        decoded[entry_start + 2],
                        decoded[entry_start + 3],
                        decoded[entry_start + 4],
                    ]) as i64;

                    // Extract label if present
                    let mut name = None;
                    if length > 9 {
                        // Skip color bytes (often 3 or 4 bytes), read remainder as string
                        let str_slice = &decoded[entry_start + 9..entry_end];
                        if let Ok(label) = std::str::from_utf8(str_slice) {
                            let clean_label = label.trim_matches('\0').trim();
                            if !clean_label.is_empty() {
                                name = Some(clean_label.to_string());
                            }
                        }
                    }

                    if (0..=7).contains(&index) && pos_ms >= 0 {
                        let mut cue = Cue::new_hot(track_id.to_string(), pos_ms, (index as i32) + 1);
                        cue.name = name.or_else(|| Some(format!("Hot Cue {}", index + 1)));
                        cues.push(cue);
                    }
                }

                i = entry_end;
            } else {
                i += 1;
            }
        }

        cues
    }

    /// Check if a key is in Camelot notation (e.g. 11A, 8B, 1A, 12B)
    pub fn is_camelot_key(key: &str) -> bool {
        let clean = key.trim();
        if clean.len() < 2 || clean.len() > 3 {
            return false;
        }
        let re = Regex::new(r"^(1[0-2]|[1-9])[ABab]$").unwrap();
        re.is_match(clean)
    }

    /// Normalize key string
    pub fn normalize_key(key: &str) -> String {
        let clean = key.trim();
        let re = Regex::new(r"^(1[0-2]|[1-9])([ABab])$").unwrap();
        if let Some(caps) = re.captures(clean) {
            if let (Some(num), Some(letter)) = (caps.get(1), caps.get(2)) {
                return format!("{}{}", num.as_str(), letter.as_str().to_ascii_uppercase());
            }
        }
        clean.to_string()
    }

    /// Re-sync a track's metadata and cues directly from its audio file tags on disk
    pub fn sync_track_from_file(conn: &Connection, track: &Track) -> Result<Track> {
        let path = Path::new(&track.file_path);
        if !path.exists() {
            return Err(CrateError::FileNotFound(path.to_path_buf()));
        }

        let tagged_file = match Self::read_metadata_lenient(path) {
            Some(tf) => tf,
            None => return Err(CrateError::Metadata("Could not parse file tags".to_string())),
        };

        let mik_data = Self::extract_analysis_data(&tagged_file, &track.id);

        let mut updated = track.clone();
        if let Some(bpm) = mik_data.bpm {
            updated.bpm = Some(bpm);
        }
        if let Some(ref key) = mik_data.key {
            updated.key = Some(key.clone());
        }
        if let Some(energy) = mik_data.energy {
            updated.energy = Some(energy);
        }
        if mik_data.is_mik {
            updated.analysis_source = Some("mixed_in_key".to_string());
        }

        let now = chrono::Utc::now().to_rfc3339();
        updated.date_modified = now;

        let hlc = dirty::next_hlc(conn)?;

        // Update tracks table
        conn.execute(
            r#"
            UPDATE tracks
            SET bpm = ?1,
                key = ?2,
                energy = ?3,
                analysis_source = ?4,
                date_modified = ?5,
                _hlc = ?6
            WHERE id = ?7
            "#,
            rusqlite::params![
                updated.bpm,
                updated.key,
                updated.energy,
                updated.analysis_source,
                updated.date_modified,
                hlc,
                updated.id,
            ],
        )?;

        // Insert / update cues
        if !mik_data.cues.is_empty() {
            // Delete old auto-imported cues for this track
            conn.execute("DELETE FROM cues WHERE track_id = ?1", [&updated.id])?;

            for cue in &mik_data.cues {
                let cue_hlc = dirty::next_hlc(conn)?;
                conn.execute(
                    r#"
                    INSERT INTO cues (id, track_id, position_ms, type, loop_end_ms, hot_cue_index, name, color, _hlc)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    "#,
                    rusqlite::params![
                        cue.id,
                        cue.track_id,
                        cue.position_ms,
                        cue.cue_type.to_string(),
                        cue.loop_end_ms,
                        cue.hot_cue_index,
                        cue.name,
                        cue.color,
                        cue_hlc,
                    ],
                )?;
            }
            dirty::mark_dirty(conn, buckets::CUES)?;
        }

        dirty::mark_dirty(conn, &buckets::bucket_for_track_id(&updated.id))?;

        Ok(updated)
    }
}

/// Simple base64 decoding helper without extra dependencies
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let input = input.as_bytes();
    let mut out = Vec::with_capacity(input.len() * 3 / 4);

    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for &b in input {
        if b == b'=' || b.is_ascii_whitespace() {
            continue;
        }
        let val = TABLE.iter().position(|&x| x == b)? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_camelot_key() {
        assert!(MikService::is_camelot_key("11A"));
        assert!(MikService::is_camelot_key("8B"));
        assert!(MikService::is_camelot_key("1a"));
        assert!(MikService::is_camelot_key("12b"));
        assert!(!MikService::is_camelot_key("13A"));
        assert!(!MikService::is_camelot_key("0B"));
        assert!(!MikService::is_camelot_key("F#m"));
        assert!(!MikService::is_camelot_key("C"));
    }

    #[test]
    fn test_normalize_key() {
        assert_eq!(MikService::normalize_key("11a"), "11A");
        assert_eq!(MikService::normalize_key("8b"), "8B");
        assert_eq!(MikService::normalize_key("F#m"), "F#m");
    }

    #[test]
    fn test_base64_decode() {
        let decoded = base64_decode("AQIDBA==");
        assert_eq!(decoded, Some(vec![1, 2, 3, 4]));
    }
}


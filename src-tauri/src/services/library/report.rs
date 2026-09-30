//! A report of where Crate, Mixed In Key and Rekordbox disagree about the same tracks: key,
//! tempo, energy, cues, files that are gone, and tracks one tool has that another does not.
//!
//! It is a **report, never a fix**: Mixed In Key is only read, nothing is changed or deleted
//! here, and removing a track from Crate stays a separate action the user confirms. It replaces
//! the old automatic purge of tracks missing from Mixed In Key (see [C3]): the same question, now
//! answered with a list to look at.
//!
//! Tracks are matched by file path (Unicode-normalised, as macOS writes it); a Rekordbox XML,
//! which may come from another machine, falls back to artist and title.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use super::mik_db::{MikDatabaseService, MikDbSong};
use super::*;
use crate::services::harmonic::CamelotKey;
use crate::services::stats::rekordbox::{xml_attr, xml_unescape};

/// Entries kept per list; the real count is always in `total`.
const MAX_ITEMS: usize = 500;
/// Two tempos within this many percent are the same tempo.
const SAME_TEMPO_PERCENT: f64 = 1.0;

/// A capped list with its real size.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Section<T> {
    pub total: usize,
    pub items: Vec<T>,
}

impl<T> Section<T> {
    fn from_sorted(mut items: Vec<T>) -> Self {
        let total = items.len();
        items.truncate(MAX_ITEMS);
        Self { total, items }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackRef {
    /// The Crate track, when the entry is about one.
    pub id: Option<String>,
    pub title: String,
    pub artist: String,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDifference {
    /// `key`, `bpm`, `energy` or `cues`.
    pub field: String,
    pub crate_value: Option<String>,
    pub other_value: Option<String>,
    /// `half_or_double_time`: the same tempo counted at half or double speed.
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackDifference {
    pub track: TrackRef,
    pub fields: Vec<FieldDifference>,
}

/// Crate compared with one other tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    /// Tracks found on both sides.
    pub matched: usize,
    pub differences: Section<TrackDifference>,
    /// In Crate (file present) but not in the other tool.
    pub missing_from_other: Section<TrackRef>,
    /// In the other tool (file present, or unknown) but not in Crate: candidates to import.
    pub missing_from_crate: Section<TrackRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingReason {
    /// The file is not where Crate remembers it.
    Missing,
    /// The volume it lives on is not mounted: the file is probably fine.
    VolumeUnmounted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissingFile {
    pub track: TrackRef,
    pub reason: MissingReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscrepancyReport {
    pub crate_tracks: usize,
    pub missing_files: Section<MissingFile>,
    /// `None` when Mixed In Key is not installed.
    pub mixed_in_key: Option<Comparison>,
    /// `None` when no Rekordbox export was given.
    pub rekordbox: Option<Comparison>,
}

/// What the report needs to know about one Crate track.
#[derive(Debug, Clone)]
pub(super) struct CrateRow {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub file_path: String,
    pub key: Option<String>,
    pub bpm: Option<f64>,
    pub energy: Option<i32>,
    pub cue_count: usize,
}

/// A track of another tool, reduced to what can be compared.
#[derive(Debug, Clone)]
pub(super) struct OtherTrack {
    pub title: String,
    pub artist: String,
    pub file_path: Option<String>,
    pub key: Option<String>,
    pub bpm: Option<f64>,
    pub energy: Option<i32>,
    /// `None` when the tool does not tell.
    pub cue_count: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileState {
    Present,
    Missing,
    VolumeUnmounted,
}

/// The state of a file on disk. A path under `/Volumes/<name>` whose volume is not there is
/// "unmounted", not "missing": the lesson of [C3] is never to treat them alike.
pub(super) fn file_state(path: &str) -> FileState {
    let p = Path::new(path);
    if p.exists() {
        return FileState::Present;
    }
    let mut parts = p.components();
    let volume = (|| {
        parts.next()?; // `/`
        if parts.next()?.as_os_str() != "Volumes" {
            return None;
        }
        Some(Path::new("/Volumes").join(parts.next()?.as_os_str()))
    })();
    match volume {
        Some(root) if !root.exists() => FileState::VolumeUnmounted,
        _ => FileState::Missing,
    }
}

fn nfc(text: &str) -> String {
    text.nfc().collect()
}

/// `artist` and `title` folded to a matching key: case, spaces and accents' form ignored.
fn name_key(artist: &str, title: &str) -> String {
    format!(
        "{}\u{1f}{}",
        nfc(artist.trim()).to_lowercase(),
        nfc(title.trim()).to_lowercase()
    )
}

fn crate_ref(row: &CrateRow) -> TrackRef {
    TrackRef {
        id: Some(row.id.clone()),
        title: row.title.clone(),
        artist: row.artist.clone(),
        file_path: Some(row.file_path.clone()),
    }
}

fn other_ref(track: &OtherTrack) -> TrackRef {
    TrackRef {
        id: None,
        title: track.title.clone(),
        artist: track.artist.clone(),
        file_path: track.file_path.clone(),
    }
}

fn sort_key(track: &TrackRef) -> (String, String) {
    (track.artist.to_lowercase(), track.title.to_lowercase())
}

#[derive(Debug, PartialEq)]
enum Tempo {
    Same,
    HalfOrDouble,
    Different,
}

fn compare_tempo(a: f64, b: f64) -> Tempo {
    let close = |x: f64, y: f64| y > 0.0 && ((x - y).abs() / y * 100.0) <= SAME_TEMPO_PERCENT;
    if close(a, b) {
        Tempo::Same
    } else if close(a * 2.0, b) || close(a, b * 2.0) {
        Tempo::HalfOrDouble
    } else {
        Tempo::Different
    }
}

/// Whether two keys (in any notation) name the same key; `None` when either is unreadable, in
/// which case the texts are compared as they are.
fn same_key(a: &str, b: &str) -> bool {
    match (CamelotKey::parse(a), CamelotKey::parse(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a.trim().eq_ignore_ascii_case(b.trim()),
    }
}

fn text(value: &Option<String>) -> Option<String> {
    value.as_ref().filter(|v| !v.trim().is_empty()).cloned()
}

fn differences(row: &CrateRow, other: &OtherTrack) -> Vec<FieldDifference> {
    let mut fields = Vec::new();

    match (text(&row.key), text(&other.key)) {
        (Some(a), Some(b)) if !same_key(&a, &b) => fields.push(FieldDifference {
            field: "key".into(),
            crate_value: Some(a),
            other_value: Some(b),
            note: None,
        }),
        (None, Some(b)) => fields.push(FieldDifference {
            field: "key".into(),
            crate_value: None,
            other_value: Some(b),
            note: None,
        }),
        (Some(a), None) => fields.push(FieldDifference {
            field: "key".into(),
            crate_value: Some(a),
            other_value: None,
            note: None,
        }),
        _ => {}
    }

    if let (Some(a), Some(b)) = (row.bpm.filter(|v| *v > 0.0), other.bpm.filter(|v| *v > 0.0)) {
        let note = match compare_tempo(a, b) {
            Tempo::Same => None,
            Tempo::HalfOrDouble => Some("half_or_double_time".to_string()),
            Tempo::Different => Some(String::new()),
        };
        if let Some(note) = note {
            fields.push(FieldDifference {
                field: "bpm".into(),
                crate_value: Some(format!("{a:.2}")),
                other_value: Some(format!("{b:.2}")),
                note: Some(note).filter(|n| !n.is_empty()),
            });
        }
    }

    if let (Some(a), Some(b)) = (row.energy, other.energy) {
        if a != b {
            fields.push(FieldDifference {
                field: "energy".into(),
                crate_value: Some(a.to_string()),
                other_value: Some(b.to_string()),
                note: None,
            });
        }
    }

    // Only cues the other tool has and Crate lacks: cues added in Crate are the user's own.
    if let Some(theirs) = other.cue_count {
        if row.cue_count < theirs {
            fields.push(FieldDifference {
                field: "cues".into(),
                crate_value: Some(row.cue_count.to_string()),
                other_value: Some(theirs.to_string()),
                note: None,
            });
        }
    }

    fields
}

/// Crate against one other tool. `match_by_name` lets a track that has no file match fall back to
/// artist and title (a Rekordbox export may come from a machine with other paths).
pub(super) fn compare(
    rows: &[CrateRow],
    others: &[OtherTrack],
    match_by_name: bool,
    state: &dyn Fn(&str) -> FileState,
) -> Comparison {
    let mut by_path: HashMap<String, Vec<usize>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, other) in others.iter().enumerate() {
        if let Some(path) = &other.file_path {
            by_path.entry(nfc(path)).or_default().push(i);
        }
        by_name
            .entry(name_key(&other.artist, &other.title))
            .or_default()
            .push(i);
    }

    let mut used: HashSet<usize> = HashSet::new();
    let mut matched = 0;
    let mut diffs = Vec::new();
    let mut missing_from_other = Vec::new();

    for row in rows {
        let found = by_path
            .get(&nfc(&row.file_path))
            .and_then(|v| v.first().copied())
            .or_else(|| {
                if !match_by_name {
                    return None;
                }
                by_name
                    .get(&name_key(&row.artist, &row.title))
                    .and_then(|v| v.iter().copied().find(|i| !used.contains(i)))
            });
        match found {
            Some(i) => {
                used.insert(i);
                matched += 1;
                let fields = differences(row, &others[i]);
                if !fields.is_empty() {
                    diffs.push(TrackDifference {
                        track: crate_ref(row),
                        fields,
                    });
                }
            }
            // A track whose file is gone is reported as a missing file, once.
            None if state(&row.file_path) == FileState::Present => {
                missing_from_other.push(crate_ref(row));
            }
            None => {}
        }
    }

    let mut missing_from_crate: Vec<TrackRef> = others
        .iter()
        .enumerate()
        .filter(|(i, _)| !used.contains(i))
        .filter(|(_, o)| {
            o.file_path
                .as_deref()
                .is_none_or(|p| state(p) == FileState::Present)
        })
        .map(|(_, o)| other_ref(o))
        .collect();

    diffs.sort_by_key(|d| sort_key(&d.track));
    missing_from_other.sort_by_key(sort_key);
    missing_from_crate.sort_by_key(sort_key);

    Comparison {
        matched,
        differences: Section::from_sorted(diffs),
        missing_from_other: Section::from_sorted(missing_from_other),
        missing_from_crate: Section::from_sorted(missing_from_crate),
    }
}

pub(super) fn build_report(
    rows: &[CrateRow],
    mik: Option<&[OtherTrack]>,
    rekordbox: Option<&[OtherTrack]>,
    state: &dyn Fn(&str) -> FileState,
) -> DiscrepancyReport {
    let mut missing: Vec<MissingFile> = rows
        .iter()
        .filter_map(|row| match state(&row.file_path) {
            FileState::Present => None,
            FileState::Missing => Some((row, MissingReason::Missing)),
            FileState::VolumeUnmounted => Some((row, MissingReason::VolumeUnmounted)),
        })
        .map(|(row, reason)| MissingFile {
            track: crate_ref(row),
            reason,
        })
        .collect();
    missing.sort_by_key(|m| sort_key(&m.track));

    DiscrepancyReport {
        crate_tracks: rows.len(),
        missing_files: Section::from_sorted(missing),
        mixed_in_key: mik.map(|tracks| compare(rows, tracks, false, state)),
        rekordbox: rekordbox.map(|tracks| compare(rows, tracks, true, state)),
    }
}

fn mik_track(song: &MikDbSong) -> OtherTrack {
    OtherTrack {
        title: song.name.clone().unwrap_or_default(),
        artist: song.artist.clone().unwrap_or_default(),
        file_path: song
            .file_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        key: song.key.clone(),
        bpm: song.tempo,
        energy: song.energy,
        cue_count: Some(song.cues.len()),
    }
}

/// `file://localhost/Users/me/My%20Music/a.mp3` → `/Users/me/My Music/a.mp3`.
pub(super) fn path_from_location(location: &str) -> Option<String> {
    let rest = location
        .strip_prefix("file://localhost")
        .or_else(|| location.strip_prefix("file://"))?;
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    let path = String::from_utf8(out).ok()?;
    path.starts_with('/').then_some(path)
}

/// Largest Rekordbox export read (a 100,000-track collection is about 100 MB of XML).
const MAX_XML_BYTES: u64 = 256 * 1024 * 1024;

/// Reads a Rekordbox XML export chosen by the user, refusing anything that is not an `.xml` file
/// or is unreasonably large, so a wrong pick cannot make the report read a huge or unrelated file.
pub fn read_rekordbox_xml(path: &str) -> Result<String> {
    let path = Path::new(path);
    let is_xml = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("xml"));
    if !is_xml {
        return Err(CrateError::InvalidOperation(
            "the Rekordbox export must be an .xml file".to_string(),
        ));
    }
    if std::fs::metadata(path)?.len() > MAX_XML_BYTES {
        return Err(CrateError::InvalidOperation(
            "the Rekordbox export is too large to read".to_string(),
        ));
    }
    Ok(std::fs::read_to_string(path)?)
}

/// The tracks of a Rekordbox XML `COLLECTION`, with their location and number of cues.
pub(super) fn parse_rekordbox_collection(xml: &str) -> Vec<OtherTrack> {
    let Some(start) = xml.find("<COLLECTION") else {
        return Vec::new();
    };
    let end = xml[start..]
        .find("</COLLECTION>")
        .map_or(xml.len(), |e| start + e);
    let track_re =
        regex::Regex::new(r#"(?s)<TRACK\s+([^>]*?)(?:/>|>(.*?)</TRACK>)"#).expect("valid regex");
    track_re
        .captures_iter(&xml[start..end])
        .filter_map(|cap| {
            let attrs = &cap[1];
            xml_attr(attrs, "TrackID")?;
            let body = cap.get(2).map_or("", |m| m.as_str());
            Some(OtherTrack {
                title: xml_attr(attrs, "Name")
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
                artist: xml_attr(attrs, "Artist")
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
                file_path: xml_attr(attrs, "Location")
                    .map(|l| xml_unescape(&l))
                    .and_then(|l| path_from_location(&l)),
                key: xml_attr(attrs, "Tonality").filter(|k| !k.is_empty()),
                bpm: xml_attr(attrs, "AverageBpm")
                    .and_then(|b| b.parse::<f64>().ok())
                    .filter(|b| *b > 0.0),
                energy: None,
                cue_count: Some(body.matches("<POSITION_MARK").count()),
            })
        })
        .collect()
}

impl LibraryService {
    /// The report, reading Mixed In Key's database and, if given, a Rekordbox XML export. Read
    /// only: the files are checked on disk (slow for a large library, so call it off the async
    /// workers) and nothing is written anywhere.
    pub fn build_discrepancy_report(
        &self,
        rekordbox_xml: Option<&str>,
    ) -> Result<DiscrepancyReport> {
        let rows = self.report_rows()?;
        // Mixed In Key is read without the Crate connection being locked.
        let mik: Option<Vec<OtherTrack>> = if MikDatabaseService::find_mik_db_path().is_some() {
            Some(
                MikDatabaseService::read_all_songs()?
                    .iter()
                    .map(mik_track)
                    .collect(),
            )
        } else {
            None
        };
        let rekordbox = rekordbox_xml.map(parse_rekordbox_collection);
        Ok(build_report(
            &rows,
            mik.as_deref(),
            rekordbox.as_deref(),
            &file_state,
        ))
    }

    fn report_rows(&self) -> Result<Vec<CrateRow>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut cue_counts: HashMap<String, usize> = HashMap::new();
        let mut cues = conn.prepare("SELECT track_id, COUNT(*) FROM cues GROUP BY track_id")?;
        for row in cues.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
            let (id, n) = row?;
            cue_counts.insert(id, n.max(0) as usize);
        }

        let mut stmt =
            conn.prepare("SELECT id, title, artist, file_path, key, bpm, energy FROM tracks")?;
        let rows = stmt
            .query_map([], |r| {
                let id: String = r.get(0)?;
                Ok(CrateRow {
                    cue_count: cue_counts.get(&id).copied().unwrap_or(0),
                    id,
                    title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    artist: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    file_path: r.get(3)?,
                    key: r.get(4)?,
                    bpm: r.get(5)?,
                    energy: r.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, artist: &str, title: &str, path: &str) -> CrateRow {
        CrateRow {
            id: id.into(),
            title: title.into(),
            artist: artist.into(),
            file_path: path.into(),
            key: Some("8A".into()),
            bpm: Some(124.0),
            energy: Some(6),
            cue_count: 2,
        }
    }

    fn other(artist: &str, title: &str, path: Option<&str>) -> OtherTrack {
        OtherTrack {
            title: title.into(),
            artist: artist.into(),
            file_path: path.map(String::from),
            key: Some("8A".into()),
            bpm: Some(124.0),
            energy: Some(6),
            cue_count: Some(2),
        }
    }

    /// Every file exists unless its path contains `gone` or `/Volumes/off/`.
    fn state(path: &str) -> FileState {
        if path.contains("/Volumes/off/") {
            FileState::VolumeUnmounted
        } else if path.contains("gone") {
            FileState::Missing
        } else {
            FileState::Present
        }
    }

    fn fields(d: &TrackDifference) -> Vec<&str> {
        d.fields.iter().map(|f| f.field.as_str()).collect()
    }

    #[test]
    fn locations_are_decoded_to_paths() {
        assert_eq!(
            path_from_location("file://localhost/Users/me/My%20Music/Tom%20%26%20Jerry.mp3")
                .as_deref(),
            Some("/Users/me/My Music/Tom & Jerry.mp3")
        );
        assert_eq!(
            path_from_location("file:///Users/me/Caf%C3%A9.mp3").as_deref(),
            Some("/Users/me/Café.mp3"),
            "UTF-8 bytes are decoded as a whole"
        );
        assert_eq!(path_from_location("http://example.com/a.mp3"), None);
        assert_eq!(path_from_location("file://localhost/bad%zzname.mp3"), None);
        assert_eq!(
            path_from_location("file://localhost/odd%4").as_deref(),
            Some("/odd%4"),
            "a truncated escape is kept as written"
        );
    }

    #[test]
    fn a_rekordbox_collection_is_read_with_location_key_tempo_and_cues() {
        let xml = r#"<?xml version="1.0"?>
<DJ_PLAYLISTS Version="1.0.0"><COLLECTION Entries="2">
  <TRACK TrackID="1" Name="Tom &amp; Jerry" Artist="A &amp; B" AverageBpm="124.00" Tonality="Am" Location="file://localhost/Music/Tom%20%26%20Jerry.mp3">
    <TEMPO Inizio="0.00" Bpm="124.00" Metro="4/4" Battito="1" />
    <POSITION_MARK Name="" Type="0" Start="1.000" Num="-1" />
    <POSITION_MARK Name="Drop" Type="0" Start="30.000" Num="0" />
  </TRACK>
  <TRACK TrackID="2" Name="Plain" Artist="C" AverageBpm="0.00" Tonality="" />
</COLLECTION>
<PLAYLISTS><NODE Type="0" Name="ROOT"><NODE Name="P" Type="1"><TRACK Key="1" /></NODE></NODE></PLAYLISTS>
</DJ_PLAYLISTS>"#;
        let tracks = parse_rekordbox_collection(xml);
        assert_eq!(tracks.len(), 2, "playlist entries are not tracks");
        assert_eq!(tracks[0].title, "Tom & Jerry");
        assert_eq!(
            tracks[0].file_path.as_deref(),
            Some("/Music/Tom & Jerry.mp3")
        );
        assert_eq!(
            (tracks[0].key.as_deref(), tracks[0].bpm, tracks[0].cue_count),
            (Some("Am"), Some(124.0), Some(2))
        );
        assert_eq!(
            (tracks[1].key.clone(), tracks[1].bpm, tracks[1].cue_count),
            (None, None, Some(0))
        );
        assert!(parse_rekordbox_collection("not xml at all").is_empty());
    }

    #[test]
    fn tempos_are_compared_with_a_tolerance_and_octaves() {
        assert_eq!(compare_tempo(124.0, 124.4), Tempo::Same, "0.3 %");
        assert_eq!(compare_tempo(124.0, 62.0), Tempo::HalfOrDouble);
        assert_eq!(compare_tempo(62.1, 124.0), Tempo::HalfOrDouble);
        assert_eq!(compare_tempo(124.0, 130.0), Tempo::Different);
    }

    #[test]
    fn keys_are_compared_across_notations() {
        assert!(same_key("8A", "Am"));
        assert!(same_key("08a", "A minor"));
        assert!(!same_key("8A", "9A"));
        assert!(
            same_key("weird", "WEIRD"),
            "unreadable keys fall back to the text"
        );
        assert!(!same_key("weird", "other"));
    }

    #[test]
    fn differences_are_reported_field_by_field() {
        let mut mine = row("t1", "A", "One", "/m/one.mp3");
        mine.cue_count = 1;
        let mut theirs = other("A", "One", Some("/m/one.mp3"));
        theirs.key = Some("9A".into());
        theirs.bpm = Some(62.0);
        theirs.energy = Some(8);
        theirs.cue_count = Some(3);

        let result = compare(&[mine], &[theirs], false, &state);
        assert_eq!(result.matched, 1);
        let d = &result.differences.items[0];
        assert_eq!(fields(d), ["key", "bpm", "energy", "cues"]);
        assert_eq!(d.fields[1].note.as_deref(), Some("half_or_double_time"));
        assert_eq!(d.track.id.as_deref(), Some("t1"));
    }

    #[test]
    fn equivalent_values_are_not_differences() {
        let mut theirs = other("A", "One", Some("/m/one.mp3"));
        theirs.key = Some("Am".into()); // the same key as 8A
        theirs.bpm = Some(124.3);
        let result = compare(
            &[row("t1", "A", "One", "/m/one.mp3")],
            &[theirs],
            false,
            &state,
        );
        assert_eq!(result.matched, 1);
        assert_eq!(result.differences.total, 0);
    }

    #[test]
    fn cues_added_in_crate_are_not_a_difference_but_missing_ones_are() {
        let mut mine = row("t1", "A", "One", "/m/one.mp3");
        mine.cue_count = 5; // the user added some in Crate
        let theirs = other("A", "One", Some("/m/one.mp3")); // 2 cues
        assert_eq!(
            compare(&[mine], &[theirs], false, &state).differences.total,
            0
        );
    }

    #[test]
    fn a_key_one_side_lacks_is_reported() {
        let mut mine = row("t1", "A", "One", "/m/one.mp3");
        mine.key = None;
        let result = compare(
            &[mine],
            &[other("A", "One", Some("/m/one.mp3"))],
            false,
            &state,
        );
        let key = &result.differences.items[0].fields[0];
        assert_eq!(
            (
                key.field.as_str(),
                key.crate_value.clone(),
                key.other_value.clone()
            ),
            ("key", None, Some("8A".to_string()))
        );
    }

    #[test]
    fn tracks_one_tool_has_and_the_other_lacks_are_listed_both_ways() {
        let rows = [
            row("t1", "A", "Both", "/m/both.mp3"),
            row("t2", "B", "Only In Crate", "/m/only.mp3"),
            row("t3", "C", "File Is Gone", "/m/gone.mp3"),
        ];
        let others = [
            other("A", "Both", Some("/m/both.mp3")),
            other("D", "Only In Other", Some("/m/other.mp3")),
            other("E", "Other Gone", Some("/m/gone-other.mp3")),
            other("F", "No Path", None),
        ];
        let result = compare(&rows, &others, false, &state);

        assert_eq!(result.matched, 1);
        let mine: Vec<&str> = result
            .missing_from_other
            .items
            .iter()
            .map(|t| t.title.as_str())
            .collect();
        assert_eq!(
            mine,
            ["Only In Crate"],
            "a track whose file is gone is a missing file, not a missing track"
        );
        let theirs: Vec<&str> = result
            .missing_from_crate
            .items
            .iter()
            .map(|t| t.title.as_str())
            .collect();
        assert_eq!(
            theirs,
            ["Only In Other", "No Path"],
            "import candidates; one whose file is gone is left out"
        );
    }

    #[test]
    fn paths_match_whatever_the_unicode_form() {
        // "é" written as one character versus "e" + a combining accent.
        let composed = "/m/Caf\u{e9}.mp3";
        let decomposed = "/m/Cafe\u{301}.mp3";
        let result = compare(
            &[row("t1", "A", "One", composed)],
            &[other("A", "One", Some(decomposed))],
            false,
            &state,
        );
        assert_eq!(result.matched, 1);
    }

    #[test]
    fn rekordbox_falls_back_to_artist_and_title_and_uses_each_track_once() {
        let rows = [
            row("t1", "Artist A", "Same Name", "/mac/one.mp3"),
            row("t2", "Artist A", "Same Name", "/mac/two.mp3"),
        ];
        let others = [other(
            "  artist a ",
            "SAME NAME",
            Some("/other-machine/x.mp3"),
        )];

        let by_name = compare(&rows, &others, true, &state);
        assert_eq!(
            by_name.matched, 1,
            "one Rekordbox track cannot match two Crate tracks"
        );
        assert_eq!(by_name.missing_from_other.total, 1);

        let by_path_only = compare(&rows, &others, false, &state);
        assert_eq!(by_path_only.matched, 0, "Mixed In Key matches by path only");
    }

    #[test]
    fn missing_files_tell_missing_from_unmounted() {
        let rows = [
            row("t1", "A", "Here", "/m/here.mp3"),
            row("t2", "B", "Gone", "/m/gone.mp3"),
            row("t3", "C", "Away", "/Volumes/off/away.mp3"),
        ];
        let report = build_report(&rows, None, None, &state);
        assert_eq!(report.crate_tracks, 3);
        assert_eq!(report.missing_files.total, 2);
        let reasons: Vec<(&str, MissingReason)> = report
            .missing_files
            .items
            .iter()
            .map(|m| (m.track.title.as_str(), m.reason))
            .collect();
        // Sorted by artist (B, then C), not by title.
        assert_eq!(
            reasons,
            [
                ("Gone", MissingReason::Missing),
                ("Away", MissingReason::VolumeUnmounted)
            ]
        );
        assert!(report.mixed_in_key.is_none() && report.rekordbox.is_none());
    }

    #[test]
    fn lists_are_capped_but_keep_their_real_size() {
        let rows: Vec<CrateRow> = (0..600)
            .map(|i| {
                row(
                    &format!("t{i}"),
                    "A",
                    &format!("Gone {i:04}"),
                    &format!("/m/gone{i}.mp3"),
                )
            })
            .collect();
        let report = build_report(&rows, None, None, &state);
        assert_eq!(report.missing_files.total, 600);
        assert_eq!(report.missing_files.items.len(), MAX_ITEMS);
    }

    #[test]
    fn file_state_distinguishes_present_missing_and_unmounted() {
        let dir = std::env::temp_dir().join(format!("crate_report_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let present = dir.join("here.mp3");
        std::fs::write(&present, b"x").unwrap();

        assert_eq!(file_state(&present.to_string_lossy()), FileState::Present);
        assert_eq!(
            file_state(&dir.join("nope.mp3").to_string_lossy()),
            FileState::Missing
        );
        assert_eq!(
            file_state("/Volumes/__crate_no_such_volume__/a.mp3"),
            FileState::VolumeUnmounted
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_xml_files_of_a_sane_size_are_read() {
        let dir = std::env::temp_dir().join(format!("crate_report_xml_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let xml = dir.join("export.XML");
        std::fs::write(&xml, "<DJ_PLAYLISTS/>").unwrap();
        let other = dir.join("notes.txt");
        std::fs::write(&other, "hello").unwrap();

        assert_eq!(
            read_rekordbox_xml(&xml.to_string_lossy()).unwrap(),
            "<DJ_PLAYLISTS/>"
        );
        assert!(
            read_rekordbox_xml(&other.to_string_lossy()).is_err(),
            "not an .xml file"
        );
        assert!(read_rekordbox_xml(&dir.join("missing.xml").to_string_lossy()).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn crate_rows_carry_the_number_of_cues() {
        use std::sync::{Arc, Mutex};
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO tracks (id, file_path, format, title, artist, key, bpm, energy, duration_ms, date_added, date_modified)
               VALUES ('t1', '/m/one.mp3', 'mp3', 'One', 'A', '8A', 124, 6, 1000, '2026-01-01', '2026-01-01'),
                      ('t2', '/m/two.mp3', 'mp3', 'Two', 'B', NULL, NULL, NULL, 1000, '2026-01-01', '2026-01-01');
             INSERT INTO cues (id, track_id, position_ms, type, hot_cue_index) VALUES
               ('c1', 't1', 1000, 'hot', 0), ('c2', 't1', 2000, 'memory', NULL);",
        )
        .unwrap();
        let dir = std::env::temp_dir().join(format!("crate_report_rows_{}", std::process::id()));
        let service = LibraryService::new(Arc::new(Mutex::new(conn)), dir.clone());

        let mut rows = service.report_rows().unwrap();
        rows.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(
            (rows[0].cue_count, rows[0].key.as_deref(), rows[0].bpm),
            (2, Some("8A"), Some(124.0))
        );
        assert_eq!(
            (rows[1].cue_count, rows[1].key.clone(), rows[1].bpm),
            (0, None, None)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

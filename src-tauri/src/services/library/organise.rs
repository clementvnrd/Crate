//! Assisted physical organisation: put the audio files on disk where a naming rule says, the
//! way the library is organised in the app.
//!
//! It works on user files, so it is built to be unable to do harm:
//! - **A preview first.** [`LibraryService::plan_organisation`] only computes every move; nothing
//!   is touched. Applying rebuilds the plan and runs only if it is *exactly* the one previewed
//!   (same plan id), so what happens is what was shown.
//! - **Never overwrites, never deletes.** A target that exists blocks that move. Files are renamed,
//!   never copied and removed, which also means a move must stay on one volume (others are
//!   reported, not attempted).
//! - **Undoable.** Every move is written to a journal; a batch can be put back.
//! - **Honest about other tools.** Rekordbox and most DJ software remember plain file paths and
//!   will not find moved files; applying requires the user to say they understand.
//!
//! The rule is a destination folder and a template such as `{artist}/{album}/{artist} - {title}`.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use super::*;
use crate::services::cloud_sync::pipeline::{buckets, dirty};
use crate::services::cloud_sync::resolution;

const MAX_COMPONENT_CHARS: usize = 120;
const MAX_DEPTH: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganisationRule {
    /// Absolute folder in which the structure is built.
    pub destination_root: String,
    /// Folders and file name, written with `/` and the tokens `{artist}`, `{album}`, `{title}`,
    /// `{year}`, `{genre}`, `{label}`, `{bpm}`, `{key}`, `{energy}`. The extension is kept.
    pub template: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveStatus {
    /// Will be moved.
    Move,
    /// Already where the rule puts it.
    AlreadyInPlace,
    /// The file is not where Crate remembers it.
    SourceMissing,
    /// Another file is already at the target: nothing is overwritten.
    TargetExists,
    /// The target is on another volume: a rename cannot do it, and copying would mean deleting.
    CrossVolume,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedMove {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub from: String,
    pub to: String,
    pub status: MoveStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganisationPlan {
    /// Fingerprint of the rule and of every move: applying needs the id that was previewed.
    pub id: String,
    pub rule: OrganisationRule,
    pub moves: Vec<PlannedMove>,
    pub to_move: usize,
    pub already_in_place: usize,
    pub blocked: usize,
    /// Always true: moving files makes other tools that store paths lose them.
    pub breaks_external_paths: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganisationFailure {
    pub track_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganisationResult {
    pub batch_id: String,
    pub moved: usize,
    pub failed: Vec<OrganisationFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganisationBatch {
    pub batch_id: String,
    pub moved_at: String,
    pub files: usize,
    pub undone: usize,
}

/// What the plan needs to know about a track.
#[derive(Debug, Clone)]
pub(super) struct PlanTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub label: Option<String>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub energy: Option<i32>,
    pub file_path: String,
}

/// The facts about the disk the plan depends on (faked in tests).
pub(super) trait Disk {
    fn exists(&self, path: &Path) -> bool;
    fn same_file(&self, a: &Path, b: &Path) -> bool;
    fn same_volume(&self, from: &Path, to: &Path) -> bool;
}

pub(super) struct RealDisk;

impl Disk for RealDisk {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    #[cfg(unix)]
    fn same_file(&self, a: &Path, b: &Path) -> bool {
        use std::os::unix::fs::MetadataExt;
        match (fs::metadata(a), fs::metadata(b)) {
            (Ok(x), Ok(y)) => x.dev() == y.dev() && x.ino() == y.ino(),
            _ => false,
        }
    }

    #[cfg(not(unix))]
    fn same_file(&self, a: &Path, b: &Path) -> bool {
        matches!((fs::canonicalize(a), fs::canonicalize(b)), (Ok(x), Ok(y)) if x == y)
    }

    #[cfg(unix)]
    fn same_volume(&self, from: &Path, to: &Path) -> bool {
        use std::os::unix::fs::MetadataExt;
        // The target may not exist yet: compare with its nearest existing folder.
        let mut probe = to.parent();
        while let Some(dir) = probe {
            if let Ok(target) = fs::metadata(dir) {
                return fs::metadata(from).is_ok_and(|source| source.dev() == target.dev());
            }
            probe = dir.parent();
        }
        false
    }

    #[cfg(not(unix))]
    fn same_volume(&self, _from: &Path, _to: &Path) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------------------------
// Template
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    Artist,
    Album,
    Title,
    Year,
    Genre,
    Label,
    Bpm,
    Key,
    Energy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Text(String),
    Token(Token),
}

/// `/`-separated segments; the last one is the file name without its extension.
struct Template {
    segments: Vec<Vec<Piece>>,
}

fn parse_template(template: &str) -> Result<Template> {
    let invalid = |why: &str| CrateError::InvalidOperation(format!("invalid template: {why}"));
    if template.trim().is_empty() {
        return Err(invalid("it is empty"));
    }
    if template.starts_with('/') || template.starts_with('\\') {
        return Err(invalid("it must be relative to the destination folder"));
    }
    if !template.contains("{title}") {
        return Err(invalid(
            "it must contain {title}, or every file would get the same name",
        ));
    }

    let mut segments = Vec::new();
    for raw in template.split('/') {
        if raw.trim().is_empty() || raw.trim() == "." || raw.trim() == ".." {
            return Err(invalid("empty, `.` and `..` folders are not allowed"));
        }
        let mut pieces = Vec::new();
        let mut rest = raw;
        while let Some(open) = rest.find('{') {
            if open > 0 {
                pieces.push(Piece::Text(rest[..open].to_string()));
            }
            let close = rest[open..]
                .find('}')
                .ok_or_else(|| invalid("a { has no closing }"))?
                + open;
            let token = match &rest[open + 1..close] {
                "artist" => Token::Artist,
                "album" => Token::Album,
                "title" => Token::Title,
                "year" => Token::Year,
                "genre" => Token::Genre,
                "label" => Token::Label,
                "bpm" => Token::Bpm,
                "key" => Token::Key,
                "energy" => Token::Energy,
                other => return Err(invalid(&format!("unknown token {{{other}}}"))),
            };
            pieces.push(Piece::Token(token));
            rest = &rest[close + 1..];
        }
        if rest.contains('}') {
            return Err(invalid("a } has no opening {"));
        }
        if !rest.is_empty() {
            pieces.push(Piece::Text(rest.to_string()));
        }
        segments.push(pieces);
    }
    if segments.len() > MAX_DEPTH {
        return Err(invalid(&format!(
            "at most {MAX_DEPTH} levels (folders and file)"
        )));
    }
    Ok(Template { segments })
}

/// A folder or file name that is safe on macOS and other file systems: no separator, no
/// character Finder or other systems reject, no control character, no leading or trailing dot or
/// space (a leading dot would hide the file), one space between words, bounded length.
pub(super) fn sanitize_component(raw: &str) -> String {
    let cleaned: String = raw
        .nfc()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_matches(|c| c == '.' || c == ' ');
    trimmed
        .chars()
        .take(MAX_COMPONENT_CHARS)
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn token_value(track: &PlanTrack, token: Token) -> String {
    let text = |value: &Option<String>, fallback: &str| {
        value
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or(fallback)
            .to_string()
    };
    match token {
        Token::Artist => {
            let artist = track.artist.trim();
            if artist.is_empty() {
                "Unknown Artist".into()
            } else {
                artist.to_string()
            }
        }
        Token::Title => {
            let title = track.title.trim();
            if title.is_empty() {
                "Untitled".into()
            } else {
                title.to_string()
            }
        }
        Token::Album => text(&track.album, "Unknown Album"),
        Token::Genre => text(&track.genre, "Unknown Genre"),
        Token::Label => text(&track.label, "Unknown Label"),
        Token::Key => text(&track.key, "Unknown Key"),
        Token::Year => track
            .year
            .filter(|y| *y > 0)
            .map_or("Unknown Year".into(), |y| y.to_string()),
        Token::Bpm => track
            .bpm
            .filter(|b| *b > 0.0)
            .map_or("Unknown BPM".into(), |b| format!("{}", b.round() as i64)),
        Token::Energy => track
            .energy
            .map_or("Unknown Energy".into(), |e| e.to_string()),
    }
}

fn render(template: &Template, track: &PlanTrack) -> Vec<String> {
    template
        .segments
        .iter()
        .map(|pieces| {
            let raw: String = pieces
                .iter()
                .map(|p| match p {
                    Piece::Text(t) => t.clone(),
                    Piece::Token(tok) => token_value(track, *tok),
                })
                .collect();
            let name = sanitize_component(&raw);
            if name.is_empty() {
                "_".to_string()
            } else {
                name
            }
        })
        .collect()
}

fn validate_root(root: &str) -> Result<PathBuf> {
    let path = PathBuf::from(root.trim());
    if !path.is_absolute() {
        return Err(CrateError::InvalidOperation(
            "the destination folder must be an absolute path".to_string(),
        ));
    }
    if !path.is_dir() {
        return Err(CrateError::InvalidOperation(
            "the destination folder does not exist".to_string(),
        ));
    }
    Ok(path)
}

// ---------------------------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------------------------

fn fold(path: &str) -> String {
    path.nfc().collect::<String>().to_lowercase()
}

/// The plan for `tracks`: where each goes, and whether it can. Pure apart from `disk`.
pub(super) fn plan_moves(
    rule: &OrganisationRule,
    root: &Path,
    tracks: &[PlanTrack],
    disk: &dyn Disk,
) -> Result<OrganisationPlan> {
    let template = parse_template(&rule.template)?;

    let mut ordered: Vec<&PlanTrack> = tracks.iter().collect();
    ordered.sort_by(|a, b| {
        (a.artist.to_lowercase(), a.title.to_lowercase(), &a.id).cmp(&(
            b.artist.to_lowercase(),
            b.title.to_lowercase(),
            &b.id,
        ))
    });

    // Targets already given out in this plan, compared the way a default macOS volume would
    // (case-insensitively), so two tracks never end up fighting for one file name.
    let mut claimed: HashSet<String> = HashSet::new();
    let mut moves = Vec::with_capacity(ordered.len());

    for track in ordered {
        let source = Path::new(&track.file_path);
        let extension = source
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();

        let mut names = render(&template, track);
        let file_stem = names.pop().unwrap_or_else(|| "_".to_string());
        let folder: PathBuf = names.iter().fold(root.to_path_buf(), |p, n| p.join(n));

        let mut attempt = 1;
        let target = loop {
            let stem = if attempt == 1 {
                file_stem.clone()
            } else {
                format!("{file_stem} ({attempt})")
            };
            let candidate = folder.join(format!("{stem}{extension}"));
            let key = fold(&candidate.to_string_lossy());
            if claimed.insert(key) {
                break candidate;
            }
            attempt += 1;
        };

        let from = track.file_path.clone();
        let to = target.to_string_lossy().to_string();
        let status = if !disk.exists(source) {
            MoveStatus::SourceMissing
        } else if from.nfc().collect::<String>() == to.nfc().collect::<String>() {
            MoveStatus::AlreadyInPlace
        } else if disk.exists(&target) && !disk.same_file(source, &target) {
            MoveStatus::TargetExists
        } else if !disk.same_volume(source, &target) {
            MoveStatus::CrossVolume
        } else {
            MoveStatus::Move
        };

        moves.push(PlannedMove {
            track_id: track.id.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            from,
            to,
            status,
        });
    }

    let count = |s: MoveStatus| moves.iter().filter(|m| m.status == s).count();
    let to_move = count(MoveStatus::Move);
    let already_in_place = count(MoveStatus::AlreadyInPlace);

    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(rule).unwrap_or_default());
    for m in &moves {
        hasher.update(format!(
            "\n{}|{}|{}|{:?}",
            m.track_id, m.from, m.to, m.status
        ));
    }
    let id = hasher
        .finalize()
        .iter()
        .take(16)
        .map(|b| format!("{b:02x}"))
        .collect();

    Ok(OrganisationPlan {
        id,
        rule: rule.clone(),
        blocked: moves.len() - to_move - already_in_place,
        to_move,
        already_in_place,
        moves,
        breaks_external_paths: true,
    })
}

// ---------------------------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------------------------

impl LibraryService {
    /// The previewed plan. Touches nothing on disk or in the database. `track_ids` restricts it to
    /// a selection; `None` is the whole library.
    pub fn plan_organisation(
        &self,
        rule: &OrganisationRule,
        track_ids: Option<&[String]>,
    ) -> Result<OrganisationPlan> {
        let root = validate_root(&rule.destination_root)?;
        let tracks = self.plan_tracks(track_ids)?;
        plan_moves(rule, &root, &tracks, &RealDisk)
    }

    /// Runs the plan, but only if it is exactly the one that was previewed.
    pub fn apply_organisation(
        &self,
        rule: &OrganisationRule,
        track_ids: Option<&[String]>,
        expected_plan_id: &str,
        understands_external_tools: bool,
    ) -> Result<OrganisationResult> {
        if !understands_external_tools {
            return Err(CrateError::InvalidOperation(
                "confirm that Rekordbox and other tools that remember file paths will lose the moved files"
                    .to_string(),
            ));
        }
        let plan = self.plan_organisation(rule, track_ids)?;
        if plan.id != expected_plan_id {
            return Err(CrateError::InvalidOperation(
                "the library or the folders changed since the preview: preview again".to_string(),
            ));
        }

        let batch_id = uuid::Uuid::new_v4().to_string();
        let mut moved = 0;
        let mut failed = Vec::new();
        for planned in plan.moves.iter().filter(|m| m.status == MoveStatus::Move) {
            match self.move_one(&batch_id, planned) {
                Ok(()) => moved += 1,
                Err(reason) => failed.push(OrganisationFailure {
                    track_id: planned.track_id.clone(),
                    reason,
                }),
            }
        }
        Ok(OrganisationResult {
            batch_id,
            moved,
            failed,
        })
    }

    /// One rename plus its database update and journal entry; the rename is undone if the
    /// database cannot follow, so a file and its record never disagree.
    fn move_one(&self, batch_id: &str, planned: &PlannedMove) -> std::result::Result<(), String> {
        let (from, to) = (Path::new(&planned.from), Path::new(&planned.to));
        if !from.exists() {
            return Err("the file is no longer there".to_string());
        }
        if to.exists() && !RealDisk.same_file(from, to) {
            return Err("a file appeared at the destination".to_string());
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("cannot create the folder: {e}"))?;
        }
        fs::rename(from, to).map_err(|e| format!("cannot move the file: {e}"))?;

        let recorded = self.conn.lock().map_err(|_| "database busy".to_string()).and_then(|conn| {
            set_track_location(&conn, &planned.track_id, &planned.to)
                .and_then(|()| {
                    conn.execute(
                        "INSERT INTO organisation_journal (id, batch_id, track_id, from_path, to_path, moved_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        rusqlite::params![
                            uuid::Uuid::new_v4().to_string(),
                            batch_id,
                            planned.track_id,
                            planned.from,
                            planned.to,
                            chrono::Utc::now().to_rfc3339()
                        ],
                    )?;
                    Ok(())
                })
                .map_err(|e| e.to_string())
        });
        if let Err(reason) = recorded {
            let _ = fs::rename(to, from);
            return Err(format!("the library could not follow: {reason}"));
        }
        Ok(())
    }

    /// Puts a batch back where it was, latest move first. A file that is no longer where Crate put
    /// it, or whose old place is taken, is left alone and reported.
    pub fn undo_organisation(&self, batch_id: &str) -> Result<OrganisationResult> {
        let entries: Vec<(String, String, String, String)> = {
            let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            let mut stmt = conn.prepare(
                "SELECT id, track_id, from_path, to_path FROM organisation_journal
                 WHERE batch_id = ?1 AND undone_at IS NULL ORDER BY rowid DESC",
            )?;
            let rows = stmt
                .query_map([batch_id], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        };

        let mut moved = 0;
        let mut failed = Vec::new();
        for (entry_id, track_id, from, to) in entries {
            let outcome = (|| -> std::result::Result<(), String> {
                let (original, current) = (Path::new(&from), Path::new(&to));
                if !current.exists() {
                    return Err("the file is no longer where Crate put it".to_string());
                }
                if original.exists() {
                    return Err("the original place is taken".to_string());
                }
                if let Some(parent) = original.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("cannot create the folder: {e}"))?;
                }
                fs::rename(current, original).map_err(|e| format!("cannot move the file: {e}"))?;
                let conn = self.conn.lock().map_err(|_| "database busy".to_string())?;
                let restored = set_track_location(&conn, &track_id, &from).and_then(|()| {
                    conn.execute(
                        "UPDATE organisation_journal SET undone_at = ?1 WHERE id = ?2",
                        rusqlite::params![chrono::Utc::now().to_rfc3339(), entry_id],
                    )?;
                    Ok(())
                });
                if let Err(e) = restored {
                    let _ = fs::rename(original, current);
                    return Err(format!("the library could not follow: {e}"));
                }
                Ok(())
            })();
            match outcome {
                Ok(()) => moved += 1,
                Err(reason) => failed.push(OrganisationFailure { track_id, reason }),
            }
        }
        Ok(OrganisationResult {
            batch_id: batch_id.to_string(),
            moved,
            failed,
        })
    }

    /// The batches that were applied, latest first.
    pub fn organisation_batches(&self, limit: usize) -> Result<Vec<OrganisationBatch>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT batch_id, MIN(moved_at), COUNT(*), COUNT(undone_at)
             FROM organisation_journal GROUP BY batch_id
             ORDER BY MIN(moved_at) DESC LIMIT ?1",
        )?;
        let batches = stmt
            .query_map([limit.clamp(1, 200) as i64], |r| {
                Ok(OrganisationBatch {
                    batch_id: r.get(0)?,
                    moved_at: r.get(1)?,
                    files: r.get::<_, i64>(2)?.max(0) as usize,
                    undone: r.get::<_, i64>(3)?.max(0) as usize,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(batches)
    }

    fn plan_tracks(&self, track_ids: Option<&[String]>) -> Result<Vec<PlanTrack>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, album, year, genre, label, bpm, key, energy, file_path FROM tracks",
        )?;
        let wanted: Option<HashSet<&str>> =
            track_ids.map(|ids| ids.iter().map(String::as_str).collect());
        let rows = stmt
            .query_map([], |r| {
                Ok(PlanTrack {
                    id: r.get(0)?,
                    title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    artist: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    album: r.get(3)?,
                    year: r.get(4)?,
                    genre: r.get(5)?,
                    label: r.get(6)?,
                    bpm: r.get(7)?,
                    key: r.get(8)?,
                    energy: r.get(9)?,
                    file_path: r.get(10)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows
            .into_iter()
            .filter(|t| wanted.as_ref().is_none_or(|w| w.contains(t.id.as_str())))
            .collect())
    }
}

/// Records a new location for a track, the way [`LibraryService::relocate_track`] does but for a
/// file whose content did not change (no new hash), and flags it for the cloud sync.
fn set_track_location(conn: &rusqlite::Connection, id: &str, new_path: &str) -> Result<()> {
    let hlc = dirty::next_hlc(conn)?;
    let (library_root_id, relative_path) = resolution::assign_root_for_import(conn, new_path)?;
    let changed = conn.execute(
        "UPDATE tracks SET file_path = ?1, date_modified = ?2, _hlc = ?3, \
            library_root_id = ?4, relative_path = ?5 WHERE id = ?6",
        rusqlite::params![
            new_path,
            chrono::Utc::now().to_rfc3339(),
            hlc,
            library_root_id,
            relative_path,
            id
        ],
    )?;
    if changed == 0 {
        return Err(CrateError::TrackNotFound(id.to_string()));
    }
    dirty::mark_dirty(conn, &buckets::bucket_for_track_id(id))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    const TEMPLATE: &str = "{artist}/{album}/{artist} - {title}";

    fn rule(root: &str, template: &str) -> OrganisationRule {
        OrganisationRule {
            destination_root: root.to_string(),
            template: template.to_string(),
        }
    }

    fn track(id: &str, artist: &str, title: &str, album: Option<&str>, path: &str) -> PlanTrack {
        PlanTrack {
            id: id.into(),
            title: title.into(),
            artist: artist.into(),
            album: album.map(String::from),
            year: None,
            genre: None,
            label: None,
            bpm: None,
            key: None,
            energy: None,
            file_path: path.into(),
        }
    }

    /// A disk described by a few facts instead of real files.
    #[derive(Default)]
    struct FakeDisk {
        existing: HashSet<String>,
        same: Vec<(String, String)>,
        other_volume_under: Option<String>,
    }

    impl FakeDisk {
        fn with(paths: &[&str]) -> Self {
            Self {
                existing: paths.iter().map(|p| p.to_string()).collect(),
                ..Default::default()
            }
        }
    }

    impl Disk for FakeDisk {
        fn exists(&self, path: &Path) -> bool {
            self.existing.contains(path.to_string_lossy().as_ref())
        }
        fn same_file(&self, a: &Path, b: &Path) -> bool {
            let (a, b) = (
                a.to_string_lossy().to_string(),
                b.to_string_lossy().to_string(),
            );
            self.same
                .iter()
                .any(|(x, y)| (x == &a && y == &b) || (x == &b && y == &a))
        }
        fn same_volume(&self, _from: &Path, to: &Path) -> bool {
            !self
                .other_volume_under
                .as_ref()
                .is_some_and(|prefix| to.starts_with(prefix))
        }
    }

    fn plan(tracks: &[PlanTrack], disk: &FakeDisk) -> OrganisationPlan {
        plan_moves(&rule("/lib", TEMPLATE), Path::new("/lib"), tracks, disk).unwrap()
    }

    fn targets(p: &OrganisationPlan) -> Vec<&str> {
        p.moves.iter().map(|m| m.to.as_str()).collect()
    }

    #[test]
    fn a_track_goes_where_the_template_says_and_keeps_its_extension() {
        let t = track(
            "1",
            "Daft Punk",
            "One More Time",
            Some("Discovery"),
            "/old/x.MP3",
        );
        let p = plan(&[t], &FakeDisk::with(&["/old/x.MP3"]));
        assert_eq!(
            targets(&p),
            ["/lib/Daft Punk/Discovery/Daft Punk - One More Time.MP3"]
        );
        assert_eq!(p.moves[0].status, MoveStatus::Move);
        assert_eq!((p.to_move, p.already_in_place, p.blocked), (1, 0, 0));
        assert!(p.breaks_external_paths);
    }

    #[test]
    fn missing_tags_get_readable_fallbacks() {
        let t = track("1", "", "", None, "/old/x.flac");
        let p = plan(&[t], &FakeDisk::with(&["/old/x.flac"]));
        assert_eq!(
            targets(&p),
            ["/lib/Unknown Artist/Unknown Album/Unknown Artist - Untitled.flac"]
        );
    }

    #[test]
    fn all_tokens_render() {
        let mut t = track("1", "A", "T", Some("Al"), "/old/x.mp3");
        t.year = Some(2021);
        t.genre = Some("House".into());
        t.label = Some("Lbl".into());
        t.bpm = Some(123.6);
        t.key = Some("8A".into());
        t.energy = Some(7);
        let p = plan_moves(
            &rule(
                "/lib",
                "{genre}/{year}/{label}/{key} {bpm} E{energy} {title}",
            ),
            Path::new("/lib"),
            &[t],
            &FakeDisk::with(&["/old/x.mp3"]),
        )
        .unwrap();
        assert_eq!(targets(&p), ["/lib/House/2021/Lbl/8A 124 E7 T.mp3"]);
    }

    #[test]
    fn names_are_made_safe() {
        assert_eq!(sanitize_component("AC/DC"), "AC_DC");
        assert_eq!(
            sanitize_component("What?*<>|\\"),
            "What______",
            "six forbidden characters"
        );
        assert_eq!(sanitize_component("a:b"), "a_b");
        assert_eq!(sanitize_component("  spaced   out  "), "spaced out");
        assert_eq!(
            sanitize_component(".hidden"),
            "hidden",
            "a leading dot would hide the file"
        );
        assert_eq!(sanitize_component("Title..."), "Title");
        assert_eq!(sanitize_component("tab\there\u{7}"), "tab_here_");
        assert_eq!(sanitize_component("..."), "");
        assert_eq!(
            sanitize_component(&"x".repeat(300)).chars().count(),
            MAX_COMPONENT_CHARS
        );
        // Composed and decomposed accents become one form.
        assert_eq!(
            sanitize_component("Cafe\u{301}"),
            sanitize_component("Caf\u{e9}")
        );
    }

    #[test]
    fn a_path_separator_in_a_tag_never_creates_a_folder() {
        let t = track(
            "1",
            "AC/DC",
            "Back/In Black",
            Some("Back/In Black"),
            "/old/x.mp3",
        );
        let p = plan(&[t], &FakeDisk::with(&["/old/x.mp3"]));
        assert_eq!(
            targets(&p),
            ["/lib/AC_DC/Back_In Black/AC_DC - Back_In Black.mp3"]
        );
    }

    #[test]
    fn statuses_say_why_a_move_is_blocked() {
        let tracks = [
            track("ok", "A", "Ok", Some("X"), "/old/ok.mp3"),
            track("here", "B", "Here", Some("X"), "/lib/B/X/B - Here.mp3"),
            track("gone", "C", "Gone", Some("X"), "/old/gone.mp3"),
            track("taken", "D", "Taken", Some("X"), "/old/taken.mp3"),
            track("far", "E", "Far", Some("X"), "/old/far.mp3"),
        ];
        let mut disk = FakeDisk::with(&[
            "/old/ok.mp3",
            "/lib/B/X/B - Here.mp3",
            "/old/taken.mp3",
            "/lib/D/X/D - Taken.mp3", // someone else's file is already there
            "/old/far.mp3",
        ]);
        disk.other_volume_under = Some("/lib/E".into());

        let p = plan(&tracks, &disk);
        let status = |id: &str| p.moves.iter().find(|m| m.track_id == id).unwrap().status;
        assert_eq!(status("ok"), MoveStatus::Move);
        assert_eq!(status("here"), MoveStatus::AlreadyInPlace);
        assert_eq!(status("gone"), MoveStatus::SourceMissing);
        assert_eq!(status("taken"), MoveStatus::TargetExists);
        assert_eq!(status("far"), MoveStatus::CrossVolume);
        assert_eq!((p.to_move, p.already_in_place, p.blocked), (1, 1, 3));
    }

    #[test]
    fn a_case_only_rename_of_the_same_file_is_a_move_not_a_conflict() {
        // On a case-insensitive volume the "target" is the source itself.
        let t = track(
            "1",
            "Artist",
            "Song",
            Some("Album"),
            "/lib/artist/album/artist - song.mp3",
        );
        let mut disk = FakeDisk::with(&[
            "/lib/artist/album/artist - song.mp3",
            "/lib/Artist/Album/Artist - Song.mp3",
        ]);
        disk.same.push((
            "/lib/artist/album/artist - song.mp3".into(),
            "/lib/Artist/Album/Artist - Song.mp3".into(),
        ));
        let p = plan(&[t], &disk);
        assert_eq!(p.moves[0].status, MoveStatus::Move);
    }

    #[test]
    fn two_tracks_with_the_same_name_do_not_fight_for_one_file() {
        let tracks = [
            track("b", "A", "Song", Some("X"), "/old/b.mp3"),
            track("a", "A", "Song", Some("X"), "/old/a.mp3"),
            track("c", "a", "SONG", Some("x"), "/old/c.mp3"), // differs only by case
        ];
        let disk = FakeDisk::with(&["/old/a.mp3", "/old/b.mp3", "/old/c.mp3"]);
        let p = plan(&tracks, &disk);
        let unique: HashSet<String> = p.moves.iter().map(|m| fold(&m.to)).collect();
        assert_eq!(unique.len(), 3, "{:?}", targets(&p));
        assert!(p.moves.iter().any(|m| m.to.contains("(2)")));
        assert!(p.moves.iter().any(|m| m.to.contains("(3)")));
    }

    #[test]
    fn the_plan_does_not_depend_on_the_order_of_the_tracks_and_its_id_follows_the_rule() {
        let a = track("a", "A", "One", Some("X"), "/old/a.mp3");
        let b = track("b", "B", "Two", Some("X"), "/old/b.mp3");
        let disk = FakeDisk::with(&["/old/a.mp3", "/old/b.mp3"]);
        let forward = plan(&[a.clone(), b.clone()], &disk);
        let backward = plan(&[b, a], &disk);
        assert_eq!(forward.id, backward.id);

        let other = plan_moves(
            &rule("/lib", "{artist}/{title}"),
            Path::new("/lib"),
            &[track("a", "A", "One", Some("X"), "/old/a.mp3")],
            &disk,
        )
        .unwrap();
        assert_ne!(forward.id, other.id);
        assert_eq!(forward.id.len(), 32);
    }

    #[test]
    fn an_accent_written_two_ways_is_already_in_place() {
        // The file is stored decomposed, the rule produces the composed form.
        let t = track(
            "1",
            "Zoe\u{301}",
            "Song",
            Some("Album"),
            "/lib/Zo\u{e9}/Album/Zo\u{e9} - Song.mp3",
        );
        let disk = FakeDisk::with(&["/lib/Zo\u{e9}/Album/Zo\u{e9} - Song.mp3"]);
        assert_eq!(
            plan(&[t], &disk).moves[0].status,
            MoveStatus::AlreadyInPlace
        );
    }

    #[test]
    fn bad_templates_are_refused_with_a_reason() {
        for bad in [
            "",
            "   ",
            "/abs/{title}",
            "{artist}/file",           // no {title}
            "{artist}/{nope}/{title}", // unknown token
            "../{title}",              // leaves the destination
            "{artist}//{title}",       // empty folder
            "a/b/c/d/{title}",         // too deep
            "{artist/{title}",         // unclosed
            "{title}}",                // stray closing brace
        ] {
            let result = plan_moves(
                &rule("/lib", bad),
                Path::new("/lib"),
                &[],
                &FakeDisk::default(),
            );
            assert!(result.is_err(), "{bad:?} should be refused");
        }
        assert!(plan_moves(
            &rule("/lib", "{title}"),
            Path::new("/lib"),
            &[],
            &FakeDisk::default()
        )
        .is_ok());
    }

    // ----- with real files ------------------------------------------------------------------

    struct Fixture {
        dir: PathBuf,
        source: PathBuf,
        root: PathBuf,
        service: LibraryService,
        conn: Arc<Mutex<rusqlite::Connection>>,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    /// Three audio files in `source/`, their tracks in the library, an empty `library/`.
    fn fixture(name: &str) -> Fixture {
        let dir =
            std::env::temp_dir().join(format!("crate_organise_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let source = dir.join("source");
        let root = dir.join("library");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&root).unwrap();

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        for (id, artist, title, album) in [
            ("t1", "Artist A", "One", "Album X"),
            ("t2", "Artist A", "Two", "Album X"),
            ("t3", "Artist B", "Three", "Album Y"),
        ] {
            let path = source.join(format!("{id}.mp3"));
            fs::write(&path, format!("audio of {id}")).unwrap();
            conn.execute(
                "INSERT INTO tracks (id, file_path, format, title, artist, album, duration_ms, date_added, date_modified)
                 VALUES (?1, ?2, 'mp3', ?3, ?4, ?5, 1000, '2026-01-01', '2026-01-01')",
                rusqlite::params![id, path.to_string_lossy(), title, artist, album],
            )
            .unwrap();
        }
        let conn = Arc::new(Mutex::new(conn));
        let service = LibraryService::new(conn.clone(), dir.join("data"));
        Fixture {
            dir,
            source,
            root,
            service,
            conn,
        }
    }

    fn r(f: &Fixture) -> OrganisationRule {
        rule(&f.root.to_string_lossy(), TEMPLATE)
    }

    fn path_of(f: &Fixture, id: &str) -> String {
        f.conn
            .lock()
            .unwrap()
            .query_row("SELECT file_path FROM tracks WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn entries(dir: &Path) -> usize {
        fs::read_dir(dir).map(|d| d.count()).unwrap_or(0)
    }

    #[test]
    fn previewing_touches_nothing() {
        let f = fixture("preview");
        let plan = f.service.plan_organisation(&r(&f), None).unwrap();
        assert_eq!(plan.to_move, 3);
        assert_eq!(entries(&f.root), 0, "no folder was created");
        assert!(f.source.join("t1.mp3").exists());
        assert!(path_of(&f, "t1").ends_with("source/t1.mp3"));
    }

    #[test]
    fn applying_the_previewed_plan_moves_the_files_and_the_library_follows() {
        let f = fixture("apply");
        let plan = f.service.plan_organisation(&r(&f), None).unwrap();
        let result = f
            .service
            .apply_organisation(&r(&f), None, &plan.id, true)
            .unwrap();

        assert_eq!((result.moved, result.failed.len()), (3, 0));
        let new_path = f.root.join("Artist A/Album X/Artist A - One.mp3");
        assert_eq!(path_of(&f, "t1"), new_path.to_string_lossy());
        assert_eq!(
            fs::read_to_string(&new_path).unwrap(),
            "audio of t1",
            "the content is intact"
        );
        assert!(!f.source.join("t1.mp3").exists());
        let journal: i64 = f
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM organisation_journal WHERE batch_id = ?1",
                [&result.batch_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(journal, 3);

        // Organised now: a new preview finds nothing left to move.
        let again = f.service.plan_organisation(&r(&f), None).unwrap();
        assert_eq!((again.to_move, again.already_in_place), (0, 3));
    }

    #[test]
    fn a_selection_only_moves_the_selected_tracks() {
        let f = fixture("selection");
        let ids = vec!["t3".to_string()];
        let plan = f.service.plan_organisation(&r(&f), Some(&ids)).unwrap();
        assert_eq!(plan.moves.len(), 1);
        f.service
            .apply_organisation(&r(&f), Some(&ids), &plan.id, true)
            .unwrap();
        assert!(f.source.join("t1.mp3").exists(), "unselected files stay");
        assert!(!f.source.join("t3.mp3").exists());
    }

    #[test]
    fn applying_needs_the_acknowledgement_and_the_exact_previewed_plan() {
        let f = fixture("guards");
        let plan = f.service.plan_organisation(&r(&f), None).unwrap();

        assert!(
            f.service
                .apply_organisation(&r(&f), None, &plan.id, false)
                .is_err(),
            "no acknowledgement"
        );
        assert!(
            f.service
                .apply_organisation(&r(&f), None, "not-the-plan", true)
                .is_err(),
            "wrong plan"
        );

        // The library changes after the preview: the preview is stale.
        f.conn
            .lock()
            .unwrap()
            .execute("UPDATE tracks SET title = 'Renamed' WHERE id = 't1'", [])
            .unwrap();
        assert!(
            f.service
                .apply_organisation(&r(&f), None, &plan.id, true)
                .is_err(),
            "stale plan"
        );

        assert_eq!(entries(&f.root), 0, "nothing moved in any of these cases");
        assert!(f.source.join("t1.mp3").exists());
    }

    #[test]
    fn an_existing_file_at_the_target_is_never_overwritten() {
        let f = fixture("overwrite");
        let taken = f.root.join("Artist A/Album X/Artist A - One.mp3");
        fs::create_dir_all(taken.parent().unwrap()).unwrap();
        fs::write(&taken, "somebody else's file").unwrap();

        let plan = f.service.plan_organisation(&r(&f), None).unwrap();
        let blocked = plan.moves.iter().find(|m| m.track_id == "t1").unwrap();
        assert_eq!(blocked.status, MoveStatus::TargetExists);
        assert_eq!((plan.to_move, plan.blocked), (2, 1));

        let result = f
            .service
            .apply_organisation(&r(&f), None, &plan.id, true)
            .unwrap();
        assert_eq!(result.moved, 2);
        assert_eq!(fs::read_to_string(&taken).unwrap(), "somebody else's file");
        assert!(
            f.source.join("t1.mp3").exists(),
            "the blocked file stays where it was"
        );
    }

    #[test]
    fn a_batch_can_be_undone_and_nothing_is_lost() {
        let f = fixture("undo");
        let original = path_of(&f, "t1");
        let plan = f.service.plan_organisation(&r(&f), None).unwrap();
        let applied = f
            .service
            .apply_organisation(&r(&f), None, &plan.id, true)
            .unwrap();

        let batches = f.service.organisation_batches(10).unwrap();
        assert_eq!(
            (batches.len(), batches[0].files, batches[0].undone),
            (1, 3, 0)
        );

        let undone = f.service.undo_organisation(&applied.batch_id).unwrap();
        assert_eq!((undone.moved, undone.failed.len()), (3, 0));
        assert_eq!(path_of(&f, "t1"), original);
        assert_eq!(fs::read_to_string(&original).unwrap(), "audio of t1");
        assert_eq!(f.service.organisation_batches(10).unwrap()[0].undone, 3);

        let twice = f.service.undo_organisation(&applied.batch_id).unwrap();
        assert_eq!(twice.moved, 0, "a batch is undone once");
    }

    #[test]
    fn undo_leaves_a_file_alone_when_its_old_place_is_taken_or_it_moved_again() {
        let f = fixture("undo_blocked");
        let plan = f.service.plan_organisation(&r(&f), None).unwrap();
        let applied = f
            .service
            .apply_organisation(&r(&f), None, &plan.id, true)
            .unwrap();

        // The old place of t1 was reused; t2 was moved away by hand.
        fs::write(f.source.join("t1.mp3"), "new file in the old place").unwrap();
        fs::remove_file(f.root.join("Artist A/Album X/Artist A - Two.mp3")).unwrap();

        let undone = f.service.undo_organisation(&applied.batch_id).unwrap();
        assert_eq!(undone.moved, 1, "only t3 could be put back");
        let reasons: Vec<&str> = undone.failed.iter().map(|x| x.reason.as_str()).collect();
        assert!(reasons.iter().any(|x| x.contains("taken")), "{reasons:?}");
        assert!(
            reasons.iter().any(|x| x.contains("no longer where")),
            "{reasons:?}"
        );
        assert_eq!(
            fs::read_to_string(f.source.join("t1.mp3")).unwrap(),
            "new file in the old place"
        );
        assert!(f.root.join("Artist A/Album X/Artist A - One.mp3").exists());
    }

    #[test]
    fn a_move_the_library_cannot_follow_is_rolled_back() {
        let f = fixture("rollback");
        // The database refuses to update t1.
        f.conn
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER refuse_t1 BEFORE UPDATE ON tracks WHEN NEW.id = 't1'
                 BEGIN SELECT RAISE(ABORT, 'simulated failure'); END;",
            )
            .unwrap();
        let plan = f.service.plan_organisation(&r(&f), None).unwrap();
        let result = f
            .service
            .apply_organisation(&r(&f), None, &plan.id, true)
            .unwrap();

        assert_eq!(result.moved, 2);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].track_id, "t1");
        assert!(f.source.join("t1.mp3").exists(), "the file went back");
        assert!(path_of(&f, "t1").ends_with("source/t1.mp3"));
        let journaled: i64 = f
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM organisation_journal WHERE track_id = 't1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            journaled, 0,
            "no journal entry for a move that did not happen"
        );
    }

    #[test]
    fn the_destination_must_be_an_existing_absolute_folder() {
        let f = fixture("root");
        assert!(f
            .service
            .plan_organisation(&rule("relative/dir", TEMPLATE), None)
            .is_err());
        assert!(f
            .service
            .plan_organisation(&rule(&f.dir.join("nope").to_string_lossy(), TEMPLATE), None)
            .is_err());
        let a_file = f.source.join("t1.mp3");
        assert!(f
            .service
            .plan_organisation(&rule(&a_file.to_string_lossy(), TEMPLATE), None)
            .is_err());
    }
}

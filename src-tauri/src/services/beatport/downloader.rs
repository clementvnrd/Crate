use crate::services::beatport::client::BeatportTrack;
use crate::services::library::LibraryService;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportDownloadResult {
    pub success_count: usize,
    pub failed_count: usize,
    pub downloaded_files: Vec<String>,
    pub errors: Vec<String>,
}

/// Validates that a file at `path` is a genuine, complete FLAC audio file:
/// 1. File exists and is a regular file.
/// 2. File size is at least 3 Mo (3 * 1024 * 1024 bytes).
/// 3. First 4 bytes are strictly the FLAC stream marker `b"fLaC"` ([0x66, 0x4C, 0x61, 0x43]).
pub fn validate_flac_file(path: &Path) -> bool {
    if !path.exists() || !path.is_file() {
        return false;
    }
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if metadata.len() < 3 * 1024 * 1024 {
        return false;
    }
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut magic = [0u8; 4];
    if file.read_exact(&mut magic).is_err() {
        return false;
    }
    magic == *b"fLaC"
}

pub struct BeatportDownloader;

impl BeatportDownloader {
    /// Expands `~` and environment variables in filesystem paths
    pub fn expand_path(input: &Path) -> PathBuf {
        let path_str = input.to_string_lossy();
        let trimmed = path_str.trim();
        if trimmed == "~" {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home);
            }
        } else if let Some(stripped) = trimmed.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home).join(stripped);
            }
        }
        PathBuf::from(trimmed)
    }

    /// Detects available beatportdl binary on the system
    pub fn find_beatportdl_binary(custom_path: Option<&str>) -> Option<PathBuf> {
        if let Some(p) = custom_path {
            let expanded = Self::expand_path(Path::new(p));
            if expanded.exists() {
                return Some(expanded);
            }
        }

        let home = std::env::var("HOME").unwrap_or_default();
        let candidates = [
            format!("{}/.local/bin/beatportdl", home),
            "/usr/local/bin/beatportdl".to_string(),
            "/opt/homebrew/bin/beatportdl".to_string(),
            format!(
                "{}/.gemini/antigravity-ide/scratch/beatportdl/beatportdl",
                home
            ),
            "beatportdl".to_string(),
            "beatport-dl".to_string(),
        ];

        for c in candidates {
            let p = PathBuf::from(&c);
            if p.exists() {
                return Some(p);
            }
            if let Ok(output) = Command::new("which").arg(&c).output() {
                if output.status.success() {
                    let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !out_str.is_empty() && Path::new(&out_str).exists() {
                        return Some(PathBuf::from(out_str));
                    }
                }
            }
        }

        None
    }

    /// Ensures beatportdl-config.yml has the correct download directory, lossless quality, and clean folder settings.
    ///
    /// Credentials are never hardcoded: any `username`/`password` lines already present in the user's
    /// own beatportdl config are preserved verbatim, otherwise beatportdl falls back to the OAuth tokens
    /// written to `beatportdl-credentials.json` when the user signs in from Crate.
    fn prepare_beatportdl_config(destination_dir: &Path) -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let config_dir = PathBuf::from(&home).join(".config/beatportdl");
        let _ = std::fs::create_dir_all(&config_dir);
        let config_file = config_dir.join("beatportdl-config.yml");

        let existing = std::fs::read_to_string(&config_file).unwrap_or_default();
        let clean_config = build_beatportdl_config(&existing, &destination_dir.to_string_lossy());

        if std::fs::write(&config_file, clean_config).is_ok() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    std::fs::set_permissions(&config_file, std::fs::Permissions::from_mode(0o600));
            }
        }
        Some(config_dir)
    }

    /// Recursively scans a directory for audio files
    fn scan_audio_files(dir: &Path) -> Vec<PathBuf> {
        let mut results = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    results.extend(Self::scan_audio_files(&path));
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if matches!(
                        ext_lower.as_str(),
                        "flac" | "mp3" | "wav" | "aif" | "aiff" | "m4a" | "aac" | "ogg"
                    ) {
                        results.push(path);
                    }
                }
            }
        }
        results
    }

    /// Validates FLAC file integrity
    #[cfg(test)]
    pub fn validate_flac_file(path: &Path) -> bool {
        validate_flac_file(path)
    }

    /// Downloads `tracks` with beatportdl into a private staging folder inside `destination_dir`.
    ///
    /// Only files created by this run can be picked up (nothing else lives in the staging folder),
    /// and every candidate must pass the FLAC header check *and* decode completely. The caller moves
    /// the files it keeps with [`move_into_destination`] and then calls [`discard_staging`].
    pub async fn download_to_staging(
        tracks: &[BeatportTrack],
        destination_dir: &Path,
        custom_dl_path: Option<&str>,
    ) -> Result<StagedDownload, String> {
        let expanded_dest = Self::expand_path(destination_dir);
        std::fs::create_dir_all(&expanded_dest).map_err(|e| {
            format!(
                "Failed to create destination folder '{:?}': {e}",
                expanded_dest
            )
        })?;

        let staging_dir = expanded_dest.join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&staging_dir)
            .map_err(|e| format!("Failed to create staging folder '{:?}': {e}", staging_dir))?;

        let mut staged = StagedDownload {
            staging_dir: staging_dir.clone(),
            valid_files: Vec::new(),
            errors: Vec::new(),
        };

        let Some(bin) = Self::find_beatportdl_binary(custom_dl_path) else {
            staged
                .errors
                .push("beatportdl binary not found on system".to_string());
            return Ok(staged);
        };
        let config_dir = Self::prepare_beatportdl_config(&staging_dir);

        let urls: Vec<String> = tracks
            .iter()
            .map(|t| {
                t.beatport_url
                    .clone()
                    .unwrap_or_else(|| format!("https://www.beatport.com/track/_/{}", t.id))
            })
            .collect();
        if urls.is_empty() {
            return Ok(staged);
        }

        // beatportdl can run for minutes: keep it off the async runtime threads.
        let run = tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new(bin);
            if let Some(cwd) = config_dir {
                cmd.current_dir(cwd);
            }
            cmd.args(&urls)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            cmd.output()
        })
        .await
        .map_err(|e| format!("beatportdl task failed: {e}"))?;
        match run {
            Ok(output) => log::info!(
                "BeatportDL output: {}\nErrors: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(e) => staged.errors.push(format!("Failed to run beatportdl: {e}")),
        }

        let expected_ms = match tracks {
            [single] if single.duration_ms > 0 => Some(single.duration_ms),
            _ => None,
        };
        // Validation decodes every file completely (seconds of CPU per track): keep it off the
        // async runtime threads too.
        let scan_dir = staging_dir.clone();
        let (valid_files, validation_errors) = tokio::task::spawn_blocking(move || {
            Self::validate_staged_files(&scan_dir, expected_ms)
        })
        .await
        .map_err(|e| format!("FLAC validation task failed: {e}"))?;
        staged.valid_files = valid_files;
        staged.errors.extend(validation_errors);
        Ok(staged)
    }

    /// Checks every audio file of a staging folder: FLAC header and size, then a complete decode
    /// (and the expected duration when it is known). Returns the files that pass and one error
    /// message per file that does not. Synchronous and CPU-heavy: call it from the blocking pool.
    fn validate_staged_files(
        staging_dir: &Path,
        expected_ms: Option<i64>,
    ) -> (Vec<PathBuf>, Vec<String>) {
        let mut valid_files = Vec::new();
        let mut errors = Vec::new();
        for file in Self::scan_audio_files(staging_dir) {
            if validate_flac_file(&file) && verify_flac_decodes(&file, expected_ms) {
                valid_files.push(file);
            } else {
                log::warn!("Downloaded file '{:?}' failed FLAC validation (header, full decode or duration)", file);
                errors.push(format!(
                    "Downloaded file '{}' failed FLAC integrity validation",
                    file.file_name().unwrap_or_default().to_string_lossy()
                ));
            }
        }
        (valid_files, errors)
    }

    /// Downloads Beatport tracks (cart) into `destination_dir` and imports them into Crate.
    /// Never touches files that already exist in the destination folder.
    pub async fn download_and_import_tracks(
        tracks: Vec<BeatportTrack>,
        destination_dir: &Path,
        custom_dl_path: Option<&str>,
        library: Option<&LibraryService>,
    ) -> Result<BeatportDownloadResult, String> {
        let expanded_dest = Self::expand_path(destination_dir);
        let staged = Self::download_to_staging(&tracks, &expanded_dest, custom_dl_path).await?;

        // Moving can mean copying a whole FLAC across volumes: do it on the blocking pool.
        let (downloaded_files, mut errors) = {
            let valid_files = staged.valid_files.clone();
            let staging_dir = staged.staging_dir.clone();
            let destination = expanded_dest.clone();
            let mut errors = staged.errors.clone();
            tokio::task::spawn_blocking(move || {
                let mut downloaded_files = Vec::new();
                for file in &valid_files {
                    match move_into_destination(file, &destination) {
                        Ok(final_path) => {
                            downloaded_files.push(final_path.to_string_lossy().to_string())
                        }
                        Err(e) => errors.push(format!(
                            "Could not move '{:?}' into the destination folder: {e}",
                            file.file_name().unwrap_or_default()
                        )),
                    }
                }
                discard_staging(&staging_dir);
                (downloaded_files, errors)
            })
            .await
            .map_err(|e| format!("moving the downloaded files failed: {e}"))?
        };

        let success_count = downloaded_files.len();
        let failed_count = tracks.len().saturating_sub(success_count);
        if failed_count > 0 && errors.is_empty() {
            errors.push(format!(
                "Failed to download {failed_count} track(s) in lossless FLAC quality"
            ));
        }

        if let Some(lib) = library {
            let pathbufs: Vec<PathBuf> = downloaded_files.iter().map(PathBuf::from).collect();
            if !pathbufs.is_empty() {
                // Importing reads tags, hashes audio and writes the library: blocking pool.
                let lib = lib.clone();
                match tokio::task::spawn_blocking(move || lib.import_tracks(pathbufs)).await {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => errors.push(format!("Import failed: {e}")),
                    Err(e) => errors.push(format!("Import failed: {e}")),
                }
            }
        }

        Ok(BeatportDownloadResult {
            success_count,
            failed_count,
            downloaded_files,
            errors,
        })
    }
}

/// Name prefix of the private folders beatportdl downloads into.
const STAGING_PREFIX: &str = ".crate-download-";

/// Files produced by one beatportdl run, still inside their staging folder.
#[derive(Debug)]
pub struct StagedDownload {
    pub staging_dir: PathBuf,
    pub valid_files: Vec<PathBuf>,
    pub errors: Vec<String>,
}

/// Moves a staged file into `dest_dir` without ever overwriting: `name (1).flac`, `name (2).flac`…
pub fn move_into_destination(staged: &Path, dest_dir: &Path) -> std::io::Result<PathBuf> {
    let file_name = staged.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "staged file has no name")
    })?;
    let mut target = dest_dir.join(file_name);
    let stem = staged
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = staged.extension().map(|e| e.to_string_lossy().to_string());
    let mut n = 1;
    while target.exists() {
        let candidate = match &ext {
            Some(ext) => format!("{stem} ({n}).{ext}"),
            None => format!("{stem} ({n})"),
        };
        target = dest_dir.join(candidate);
        n += 1;
    }
    if std::fs::rename(staged, &target).is_err() {
        std::fs::copy(staged, &target)?;
        std::fs::remove_file(staged)?;
    }
    Ok(target)
}

/// Deletes a staging folder created by [`BeatportDownloader::download_to_staging`] — and nothing else.
pub fn discard_staging(staging_dir: &Path) {
    let is_ours = staging_dir
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with(STAGING_PREFIX));
    if is_ours {
        let _ = std::fs::remove_dir_all(staging_dir);
    } else {
        log::error!(
            "Refusing to delete {:?}: not a Crate staging folder",
            staging_dir
        );
    }
}

/// Decodes the whole FLAC stream: a corrupt file fails, a truncated one fails (fewer samples than
/// announced in STREAMINFO), and when the expected duration is known the decoded duration must
/// match it (±5 s or ±3 %).
pub fn verify_flac_decodes(path: &Path, expected_ms: Option<i64>) -> bool {
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::errors::Error as SymphoniaError;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("flac");
    let Ok(probed) = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    ) else {
        return false;
    };
    let mut format = probed.format;
    let Some(track) = format.default_track() else {
        return false;
    };
    let track_id = track.id;
    let Some(sample_rate) = track.codec_params.sample_rate else {
        return false;
    };
    // Total sample count announced by STREAMINFO: a truncated file decodes fewer samples.
    let announced_frames = track.codec_params.n_frames.filter(|n| *n > 0);
    let Ok(mut decoder) = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions { verify: true })
    else {
        return false;
    };

    let mut frames: u64 = 0;
    loop {
        match format.next_packet() {
            Ok(packet) if packet.track_id() == track_id => match decoder.decode(&packet) {
                Ok(buf) => frames += buf.frames() as u64,
                Err(_) => return false,
            },
            Ok(_) => {}
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(_) => return false,
        }
    }

    if let Some(announced) = announced_frames {
        if frames + announced / 100 < announced {
            return false;
        }
    }

    let decoded_ms = (frames as f64 * 1000.0 / sample_rate as f64) as i64;
    if decoded_ms <= 0 {
        return false;
    }
    match expected_ms {
        Some(expected) if expected > 0 => {
            let tolerance = (expected as f64 * 0.03).max(5000.0) as i64;
            (decoded_ms - expected).abs() <= tolerance
        }
        _ => true,
    }
}

/// Builds the beatportdl YAML config, keeping only the credential lines from `existing`.
fn build_beatportdl_config(existing: &str, downloads_dir: &str) -> String {
    let credentials: String = existing
        .lines()
        .filter(|line| {
            let key = line.trim_start();
            key.starts_with("username:") || key.starts_with("password:")
        })
        .map(|line| format!("{line}\n"))
        .collect();

    format!(
        "{credentials}\
         quality: \"lossless\"\n\
         show_progress: false\n\
         write_error_log: true\n\
         max_download_workers: 15\n\
         max_global_workers: 15\n\
         downloads_directory: \"{downloads_dir}\"\n\
         sort_by_context: false\n\
         sort_by_label: false\n\
         force_release_directories: false\n\
         keep_cover: false\n\
         track_exists: \"skip\"\n\
         fix_tags: true\n\
         key_system: \"camelot\"\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const SILENCE_FLAC: &[u8] = include_bytes!("../../../test-fixtures/silence-1s.flac");

    #[test]
    fn test_verify_flac_decodes_real_file_and_duration() {
        let dir = get_test_temp_dir("decode");
        let path = dir.join("silence.flac");
        std::fs::write(&path, SILENCE_FLAC).unwrap();
        assert!(verify_flac_decodes(&path, None));
        assert!(verify_flac_decodes(&path, Some(1_000)));
        assert!(
            !verify_flac_decodes(&path, Some(240_000)),
            "a 1 s file is not a 4 min track"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_verify_flac_decodes_rejects_fake_and_truncated() {
        let dir = get_test_temp_dir("decode_bad");
        let fake = dir.join("fake.flac");
        let mut bytes = b"fLaC".to_vec();
        bytes.extend(vec![0u8; 4096]);
        std::fs::write(&fake, &bytes).unwrap();
        assert!(!verify_flac_decodes(&fake, None));

        let truncated = dir.join("truncated.flac");
        std::fs::write(&truncated, &SILENCE_FLAC[..SILENCE_FLAC.len() / 2]).unwrap();
        assert!(!verify_flac_decodes(&truncated, Some(1_000)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_move_into_destination_never_overwrites() {
        let dir = get_test_temp_dir("move");
        let staging = dir.join(format!("{STAGING_PREFIX}test"));
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(dir.join("Track.flac"), b"existing").unwrap();
        let staged = staging.join("Track.flac");
        std::fs::write(&staged, b"new").unwrap();

        let target = move_into_destination(&staged, &dir).unwrap();

        assert_eq!(target, dir.join("Track (1).flac"));
        assert_eq!(std::fs::read(dir.join("Track.flac")).unwrap(), b"existing");
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_discard_staging_only_deletes_staging_folders() {
        let dir = get_test_temp_dir("discard");
        let user_folder = dir.join("My Music");
        std::fs::create_dir_all(&user_folder).unwrap();
        std::fs::write(user_folder.join("cover.jpg"), b"keep").unwrap();
        let staging = dir.join(format!("{STAGING_PREFIX}abc"));
        std::fs::create_dir_all(&staging).unwrap();

        discard_staging(&user_folder);
        discard_staging(&staging);

        assert!(
            user_folder.join("cover.jpg").exists(),
            "a user folder is never deleted"
        );
        assert!(!staging.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_build_beatportdl_config_preserves_user_credentials() {
        let existing = "username: \"someone\"\npassword: \"hunter2\"\nquality: \"high\"\n";
        let config = build_beatportdl_config(existing, "/tmp/flac");
        assert!(config.starts_with("username: \"someone\"\npassword: \"hunter2\"\n"));
        assert!(config.contains("quality: \"lossless\""));
        assert!(!config.contains("quality: \"high\""));
        assert!(config.contains("downloads_directory: \"/tmp/flac\""));
    }

    #[test]
    fn test_build_beatportdl_config_without_credentials() {
        let config = build_beatportdl_config("", "/tmp/flac");
        assert!(!config.contains("username:"));
        assert!(!config.contains("password:"));
        assert!(config.starts_with("quality: \"lossless\""));
        assert!(config.contains("key_system: \"camelot\""));
    }

    #[test]
    fn staged_files_that_are_not_real_flacs_are_reported_and_not_kept() {
        let dir = get_test_temp_dir("validate_staged");
        // Right extension, wrong content: the header and size checks must reject it.
        std::fs::write(dir.join("fake.flac"), b"fLaC not really").unwrap();
        // Not audio at all: never even looked at.
        std::fs::write(dir.join("notes.txt"), b"hello").unwrap();

        let (valid, errors) = BeatportDownloader::validate_staged_files(&dir, None);

        assert!(valid.is_empty());
        assert_eq!(
            errors.len(),
            1,
            "one message, for the audio-looking file only"
        );
        assert!(
            errors[0].contains("fake.flac"),
            "it names the file: {errors:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_staging_folder_has_nothing_to_validate() {
        let dir = get_test_temp_dir("validate_empty");
        let (valid, errors) = BeatportDownloader::validate_staged_files(&dir, Some(1_000));
        assert!(valid.is_empty() && errors.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn get_test_temp_dir(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "crate_flac_test_{}_{}",
            suffix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn test_validate_flac_file_valid() {
        let dir = get_test_temp_dir("valid");
        let file_path = dir.join("valid.flac");
        let mut f = std::fs::File::create(&file_path).unwrap();
        // Magic header b"fLaC"
        f.write_all(b"fLaC").unwrap();
        // Fill up to 3MB + 10 bytes
        let padding = vec![0u8; 3 * 1024 * 1024 + 10];
        f.write_all(&padding).unwrap();
        f.flush().unwrap();

        assert!(validate_flac_file(&file_path));
        assert!(BeatportDownloader::validate_flac_file(&file_path));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_flac_file_too_small() {
        let dir = get_test_temp_dir("small");
        let file_path = dir.join("small.flac");
        let mut f = std::fs::File::create(&file_path).unwrap();
        // Magic header b"fLaC" but only 100 bytes
        f.write_all(b"fLaC").unwrap();
        let padding = vec![0u8; 100];
        f.write_all(&padding).unwrap();
        f.flush().unwrap();

        assert!(!validate_flac_file(&file_path));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_flac_file_invalid_magic() {
        let dir = get_test_temp_dir("magic");
        let file_path = dir.join("fake_mp3.flac");
        let mut f = std::fs::File::create(&file_path).unwrap();
        // ID3 header
        f.write_all(b"ID3\x04").unwrap();
        let padding = vec![0u8; 4 * 1024 * 1024];
        f.write_all(&padding).unwrap();
        f.flush().unwrap();

        assert!(!validate_flac_file(&file_path));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_flac_file_nonexistent() {
        let non_existent = Path::new("/non/existent/path/song.flac");
        assert!(!validate_flac_file(non_existent));
    }
}

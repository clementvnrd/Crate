use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::collections::HashSet;
use std::io::Read;
use crate::services::library::LibraryService;
use crate::services::beatport::client::BeatportTrack;

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
            format!("{}/.gemini/antigravity-ide/scratch/beatportdl/beatportdl", home),
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
                let _ = std::fs::set_permissions(&config_file, std::fs::Permissions::from_mode(0o600));
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
                    if matches!(ext_lower.as_str(), "flac" | "mp3" | "wav" | "aif" | "aiff" | "m4a" | "aac" | "ogg") {
                        results.push(path);
                    }
                }
            }
        }
        results
    }

    /// Flattens all subdirectories created by beatportdl so that all FLAC files are stored directly
    /// in the destination root directory with zero subfolders and zero extra artwork image files.
    fn flatten_and_clean_destination(destination_dir: &Path) -> Vec<PathBuf> {
        let mut final_audio_files = Vec::new();

        if let Ok(entries) = std::fs::read_dir(destination_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Subfolder found: move any audio files inside it to the root destination_dir
                    if let Ok(sub_entries) = std::fs::read_dir(&path) {
                        for sub_entry in sub_entries.flatten() {
                            let sub_path = sub_entry.path();
                            if sub_path.is_file() {
                                let ext = sub_path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_lowercase();
                                if matches!(ext.as_str(), "flac" | "mp3" | "wav" | "aif" | "aiff" | "m4a" | "aac") {
                                    if let Some(filename) = sub_path.file_name() {
                                        let target_dest = destination_dir.join(filename);
                                        if !target_dest.exists() {
                                            let _ = std::fs::rename(&sub_path, &target_dest);
                                            final_audio_files.push(target_dest);
                                        } else {
                                            // Duplicate already at root: delete subfolder duplicate
                                            let _ = std::fs::remove_file(&sub_path);
                                            final_audio_files.push(target_dest);
                                        }
                                    }
                                } else {
                                    // Remove cover.jpg, .nfo, .txt, etc.
                                    let _ = std::fs::remove_file(&sub_path);
                                }
                            }
                        }
                    }
                    // Remove the now-empty subfolder
                    let _ = std::fs::remove_dir_all(&path);
                } else if path.is_file() {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_lowercase();
                    if matches!(ext.as_str(), "flac" | "mp3" | "wav" | "aif" | "aiff" | "m4a" | "aac") {
                        final_audio_files.push(path);
                    } else if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "nfo" | "txt" | "m3u" | "log") {
                        // Remove any extra artwork files created at root
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }

        final_audio_files
    }

    /// Validates FLAC file integrity
    pub fn validate_flac_file(path: &Path) -> bool {
        validate_flac_file(path)
    }

    /// Downloads a list of Beatport tracks and automatically synchronizes them into MIK / Crate library.
    /// Strictly guarantees lossless FLAC integrity: never falls back to preview MP3/AAC streams.
    pub async fn download_and_import_tracks(
        tracks: Vec<BeatportTrack>,
        destination_dir: &Path,
        custom_dl_path: Option<&str>,
        library: Option<&LibraryService>,
    ) -> Result<BeatportDownloadResult, String> {
        let expanded_dest = Self::expand_path(destination_dir);
        std::fs::create_dir_all(&expanded_dest)
            .map_err(|e| format!("Failed to create destination folder '{:?}': {e}", expanded_dest))?;

        let dl_bin = Self::find_beatportdl_binary(custom_dl_path);
        let config_dir = Self::prepare_beatportdl_config(&expanded_dest);

        let initial_files: HashSet<PathBuf> = Self::scan_audio_files(&expanded_dest).into_iter().collect();

        let mut downloaded_files = Vec::new();
        let mut errors = Vec::new();
        let mut success_count = 0;

        // 1. Batch download tracks via BeatportDL CLI
        if let Some(ref bin) = dl_bin {
            let mut cmd = Command::new(bin);
            if let Some(ref cwd) = config_dir {
                cmd.current_dir(cwd);
            }
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let mut has_urls = false;
            for track in &tracks {
                let url = if let Some(ref u) = track.beatport_url {
                    u.clone()
                } else {
                    format!("https://www.beatport.com/track/_/{}", track.id)
                };
                cmd.arg(url);
                has_urls = true;
            }

            if has_urls {
                if let Ok(output) = cmd.output() {
                    let out_log = String::from_utf8_lossy(&output.stdout);
                    let err_log = String::from_utf8_lossy(&output.stderr);
                    log::info!("BeatportDL output: {}\nErrors: {}", out_log, err_log);
                }
            }
        } else {
            errors.push("beatportdl binary not found on system".to_string());
        }

        // 2. Clean & flatten destination directory and validate FLAC file integrity
        let root_audio_files = Self::flatten_and_clean_destination(&expanded_dest);
        let current_files: HashSet<PathBuf> = root_audio_files.into_iter().collect();
        let new_files: Vec<PathBuf> = current_files.difference(&initial_files).cloned().collect();

        for f in &new_files {
            if Self::validate_flac_file(f) {
                downloaded_files.push(f.to_string_lossy().to_string());
                success_count += 1;
            } else {
                log::warn!("Downloaded file '{:?}' failed FLAC integrity check (size >= 3MB and fLaC magic header)", f);
                let _ = std::fs::remove_file(f);
                errors.push(format!("Downloaded file '{:?}' failed FLAC integrity validation", f.file_name().unwrap_or_default()));
            }
        }

        let failed_count = tracks.len().saturating_sub(success_count);
        if failed_count > 0 && errors.is_empty() {
            errors.push(format!("Failed to download {failed_count} track(s) in lossless FLAC quality"));
        }

        // 3. Automated MIK & Crate Database Sync (only if valid FLAC files were downloaded)
        let pathbufs: Vec<PathBuf> = downloaded_files.iter().map(PathBuf::from).collect();
        if let Some(lib) = library {
            if !pathbufs.is_empty() {
                let _ = lib.import_tracks(pathbufs);
                match lib.sync_from_mik_database() {
                    Ok(res) => log::info!(
                        "Automated Beatport MIK sync completed: {} added, {} updated, {} removed, total MIK={}",
                        res.added,
                        res.updated,
                        res.removed,
                        res.total
                    ),
                    Err(e) => log::warn!("Automated Beatport MIK sync failed: {e}"),
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

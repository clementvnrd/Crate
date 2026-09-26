//! Moving files to the system Trash. Nothing in Crate deletes a user's audio file permanently.

use std::path::Path;

/// Moves `path` to the Trash (Finder on macOS, so "Put Back" works). The path is passed to
/// AppleScript as an argument, never interpolated into the script.
pub fn move_to_trash(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("osascript")
            .args([
                "-e",
                "on run argv",
                "-e",
                "tell application \"Finder\" to delete (POSIX file (item 1 of argv))",
                "-e",
                "end run",
            ])
            .arg(path)
            .output()
            .map_err(|e| format!("osascript: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("moving files to the Trash is only supported on macOS".to_string())
    }
}

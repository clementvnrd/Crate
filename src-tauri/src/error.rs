use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrateError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[allow(dead_code)]
    #[error("Audio error: {0}")]
    Audio(String),

    #[allow(dead_code)]
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[allow(dead_code)]
    #[error("Import error: {0}")]
    Import(String),

    #[allow(dead_code)]
    #[error("Export error: {0}")]
    Export(String),

    #[allow(dead_code)]
    #[error("Device error: {0}")]
    Device(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[allow(dead_code)]
    #[error("Metadata error: {0}")]
    Metadata(String),

    #[error("Artwork error: {0}")]
    Artwork(String),

    #[allow(dead_code)]
    #[error("Track not found: {0}")]
    TrackNotFound(String),

    #[allow(dead_code)]
    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("Discovery error: {0}")]
    Discovery(String),

    #[error("Backup error: {0}")]
    Backup(String),

    #[error("Key storage error: {0}")]
    KeyStorage(String),

    #[allow(dead_code)]
    #[error("Cloud sync error: {0}")]
    CloudSync(String),

    #[allow(dead_code)]
    #[error("Cloud sync conflict (manifest etag mismatch)")]
    CloudSyncConflict,

    #[allow(dead_code)]
    #[error("Cloud sync blob not found: {0}")]
    CloudSyncBlobNotFound(String),

    #[allow(dead_code)]
    #[error("Cloud sync auth error: {0}")]
    CloudSyncAuth(String),

    /// A transient connectivity failure (connect/timeout/DNS, HTTP 429, HTTP 5xx).
    /// Distinct from [`CrateError::CloudSync`] so the runtime can surface `Offline`
    /// (and recover) instead of a hard `Error`.
    #[error("Cloud sync network error: {0}")]
    CloudSyncNetwork(String),

    #[error("Beatport authentication required. Please sign in from the Beatport tab.")]
    BeatportAuthRequired,

    #[error("Internal lock error")]
    LockPoisoned,

    /// A background task failed to finish (it panicked or was cancelled). See [`run_blocking`].
    #[error("Internal error: {0}")]
    Internal(String),
}

impl CrateError {
    /// Whether this is a transient connectivity failure worth surfacing as `Offline`
    /// (and retrying) rather than a hard `Error`.
    pub fn is_transient(&self) -> bool {
        matches!(self, CrateError::CloudSyncNetwork(_))
    }
}

impl serde::Serialize for CrateError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CrateError>;

/// Runs synchronous, potentially slow work on tokio's blocking pool instead of a runtime worker.
///
/// Use it from `async` commands for anything that can take more than a few milliseconds: walking
/// or hashing many files, parsing tags, decoding audio, spawning `lsof`/`osascript`, or a large
/// SQLite transaction. Called directly, that work parks a runtime worker; with several such calls
/// in flight the whole command layer (and the UI waiting on it) stalls.
///
/// Inside the closure, reach a managed service with `app.state::<T>()` (move a cloned
/// `AppHandle` in): a `State<'_, T>` parameter cannot cross into a `'static` closure. A panic in
/// `work` comes back as [`CrateError::Internal`] instead of taking the command down.
pub async fn run_blocking<T, F>(work: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| CrateError::Internal(format!("background task failed: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_blocking_returns_the_value() {
        let value = run_blocking(|| Ok(40 + 2)).await.unwrap();
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn run_blocking_keeps_the_work_error() {
        let err = run_blocking::<(), _>(|| Err(CrateError::InvalidOperation("nope".into())))
            .await
            .unwrap_err();
        assert!(matches!(err, CrateError::InvalidOperation(m) if m == "nope"));
    }

    #[tokio::test]
    async fn run_blocking_turns_a_panic_into_an_error() {
        let err = run_blocking::<(), _>(|| panic!("boom")).await.unwrap_err();
        assert!(matches!(err, CrateError::Internal(m) if m.contains("background task failed")));
    }

    #[tokio::test]
    async fn run_blocking_does_not_park_the_async_workers() {
        // A single-threaded runtime would hang here if the sleep ran on the worker itself:
        // the ticker below could never run until the sleep ended.
        let slow = run_blocking(|| {
            std::thread::sleep(std::time::Duration::from_millis(150));
            Ok(())
        });
        let ticker = async {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            "ticked"
        };
        let (slow, ticked) = tokio::join!(slow, ticker);
        slow.unwrap();
        assert_eq!(ticked, "ticked");
    }
}

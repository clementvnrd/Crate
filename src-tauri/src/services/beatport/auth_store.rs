//! Storage of the Beatport OAuth session.
//!
//! On macOS the session lives in the login Keychain (one generic password holding the JSON state);
//! elsewhere it falls back to a `0600` file in `~/.config/crate/`. A plaintext file left by earlier
//! builds is migrated into the store on first read and then deleted.
//!
//! `beatportdl` still needs its own `beatportdl-credentials.json` to download: that file is written
//! with `0600` permissions and removed on sign-out.

use std::path::PathBuf;

use super::client::BeatportAuthState;

#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "com.bbx-audio.crate.beatport";
#[cfg(target_os = "macos")]
const KEYCHAIN_ACCOUNT: &str = "oauth-session";
/// `errSecItemNotFound`
#[cfg(target_os = "macos")]
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

fn home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// Plaintext file used by earlier builds (and by the non-macOS fallback).
fn legacy_auth_file() -> Option<PathBuf> {
    home().map(|h| h.join(".config/crate/beatport_auth.json"))
}

fn beatportdl_credentials_file() -> Option<PathBuf> {
    home().map(|h| h.join(".config/beatportdl/beatportdl-credentials.json"))
}

fn write_private(path: &PathBuf, contents: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(path, contents).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
    }
}

#[cfg(target_os = "macos")]
fn store_write(json: &str) -> bool {
    security_framework::passwords::set_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, json.as_bytes())
        .map_err(|e| log::warn!("Beatport: could not save the session to the Keychain: {e}"))
        .is_ok()
}

#[cfg(target_os = "macos")]
fn store_read() -> Option<String> {
    match security_framework::passwords::get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        Ok(bytes) => String::from_utf8(bytes).ok(),
        Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => None,
        Err(e) => {
            log::warn!("Beatport: could not read the session from the Keychain: {e}");
            None
        }
    }
}

#[cfg(target_os = "macos")]
fn store_clear() {
    let _ = security_framework::passwords::delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);
}

#[cfg(not(target_os = "macos"))]
fn store_write(json: &str) -> bool {
    match legacy_auth_file() {
        Some(path) => {
            write_private(&path, json);
            true
        }
        None => false,
    }
}

#[cfg(not(target_os = "macos"))]
fn store_read() -> Option<String> {
    legacy_auth_file().and_then(|p| std::fs::read_to_string(p).ok())
}

#[cfg(not(target_os = "macos"))]
fn store_clear() {
    if let Some(path) = legacy_auth_file() {
        let _ = std::fs::remove_file(path);
    }
}

/// Returns the session only when it is usable (authenticated with at least one token).
fn parse_session(json: &str) -> Option<BeatportAuthState> {
    serde_json::from_str::<BeatportAuthState>(json)
        .ok()
        .filter(|a| a.is_authenticated && (a.token.is_some() || a.refresh_token.is_some()))
}

/// Builds the credentials file expected by `beatportdl`.
fn beatportdl_credentials_json(token: &str, refresh_token: &str) -> String {
    serde_json::json!({
        "access_token": token,
        "refresh_token": refresh_token,
        "expires_in": 36000,
        "token_type": "Bearer",
        "scope": "app:locker user:dj"
    })
    .to_string()
}

pub fn save(auth: &BeatportAuthState) {
    if let Ok(json) = serde_json::to_string(auth) {
        store_write(&json);
    }
    if let (Some(token), Some(refresh), Some(path)) =
        (&auth.token, &auth.refresh_token, beatportdl_credentials_file())
    {
        write_private(&path, &beatportdl_credentials_json(token, refresh));
    }
}

pub fn load() -> Option<BeatportAuthState> {
    if let Some(auth) = store_read().as_deref().and_then(parse_session) {
        return Some(auth);
    }

    // One-time migration of the plaintext session written by earlier builds.
    #[cfg(target_os = "macos")]
    if let Some(path) = legacy_auth_file() {
        if let Some(auth) = std::fs::read_to_string(&path).ok().as_deref().and_then(parse_session) {
            if let Ok(json) = serde_json::to_string(&auth) {
                if store_write(&json) {
                    let _ = std::fs::remove_file(&path);
                    log::info!("Beatport: session moved from a plaintext file to the Keychain");
                }
            }
            return Some(auth);
        }
    }

    None
}

pub fn clear() {
    store_clear();
    if let Some(path) = legacy_auth_file() {
        let _ = std::fs::remove_file(path);
    }
    if let Some(path) = beatportdl_credentials_file() {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(authenticated: bool, token: Option<&str>) -> BeatportAuthState {
        BeatportAuthState {
            is_authenticated: authenticated,
            username: None,
            token: token.map(String::from),
            refresh_token: None,
            has_subscription: false,
            subscription_tier: None,
        }
    }

    #[test]
    fn test_parse_session_requires_a_token() {
        let valid = serde_json::to_string(&state(true, Some("abc"))).unwrap();
        let no_token = serde_json::to_string(&state(true, None)).unwrap();
        let signed_out = serde_json::to_string(&state(false, Some("abc"))).unwrap();
        assert!(parse_session(&valid).is_some());
        assert!(parse_session(&no_token).is_none());
        assert!(parse_session(&signed_out).is_none());
        assert!(parse_session("not json").is_none());
    }

    #[test]
    fn test_beatportdl_credentials_format() {
        let json: serde_json::Value = serde_json::from_str(&beatportdl_credentials_json("a", "r")).unwrap();
        assert_eq!(json["access_token"], "a");
        assert_eq!(json["refresh_token"], "r");
        assert_eq!(json["token_type"], "Bearer");
    }
}

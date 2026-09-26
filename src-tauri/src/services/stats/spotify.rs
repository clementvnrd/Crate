use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use base64::engine::general_purpose::{STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use chrono::Utc;
use rand::Rng;
use reqwest::Client;
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

use crate::error::{CrateError, Result};
use crate::models::stats::{ListenEvent, SpotifyAuthState, SpotifyImportResult, SpotifyNowPlaying};
use crate::services::stats::recorder::StatsRecorderService;

pub const DEFAULT_SPOTIFY_CLIENT_ID: &str = "crate-pulse-spotify";
pub const DEFAULT_SPOTIFY_REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

#[derive(Clone)]
pub struct SpotifyTrackerService {
    conn: Arc<Mutex<Connection>>,
    recorder: Arc<StatsRecorderService>,
    http_client: Client,
    pending_code_verifier: Arc<Mutex<Option<String>>>,
    /// Serializes token refreshes between the background sync and UI commands.
    refresh_lock: Arc<tokio::sync::Mutex<()>>,
    /// True while the temporary OAuth callback server is listening.
    oauth_server_running: Arc<AtomicBool>,
}

#[derive(Debug, Deserialize)]
struct SpotifyTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct SpotifyUserProfile {
    id: String,
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpotifyRecentlyPlayedResponse {
    #[serde(default)]
    items: Vec<SpotifyPlayHistoryItem>,
    cursors: Option<SpotifyCursors>,
}

#[derive(Debug, Deserialize)]
struct SpotifyCursors {
    after: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpotifyPlayHistoryItem {
    track: SpotifyTrackObject,
    played_at: String,
}

#[derive(Debug, Deserialize)]
struct SpotifyTrackObject {
    id: Option<String>,
    name: String,
    #[serde(default)]
    artists: Vec<SpotifyArtistObject>,
    album: Option<SpotifyAlbumObject>,
    #[serde(default)]
    duration_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SpotifyArtistObject {
    name: String,
}

#[derive(Debug, Deserialize)]
struct SpotifyAlbumObject {
    name: Option<String>,
    images: Option<Vec<SpotifyImageObject>>,
}

#[derive(Debug, Deserialize)]
struct SpotifyImageObject {
    url: String,
}

impl SpotifyTrackerService {
    pub fn new(conn: Arc<Mutex<Connection>>, recorder: Arc<StatsRecorderService>) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            conn,
            recorder,
            http_client,
            pending_code_verifier: Arc::new(Mutex::new(None)),
            refresh_lock: Arc::new(tokio::sync::Mutex::new(())),
            oauth_server_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Generates a high-entropy PKCE code verifier (64 chars in [a-zA-Z0-9_-.~]) and computes its SHA-256 challenge.
    pub fn generate_pkce_pair() -> (String, String) {
        const CHARSET: &[u8] =
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
        let mut rng = rand::rng();
        let verifier: String = (0..64)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();

        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

        (verifier, challenge)
    }

    /// Persists Spotify client ID in settings table.
    pub fn set_client_id(&self, client_id: &str) -> Result<()> {
        let trimmed = client_id.trim();
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        if trimmed.is_empty() {
            conn.execute("DELETE FROM settings WHERE key = 'spotify_client_id'", [])
                .map_err(CrateError::Database)?;
        } else {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('spotify_client_id', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![trimmed],
            )
            .map_err(CrateError::Database)?;
        }
        Ok(())
    }

    /// Gets stored Spotify client ID from settings table, if any.
    pub fn get_client_id(&self) -> Option<String> {
        let conn = self.conn.lock().ok()?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'spotify_client_id'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .filter(|s| !s.trim().is_empty())
    }

    /// Persists Spotify client secret in settings table.
    pub fn set_client_secret(&self, secret: &str) -> Result<()> {
        let trimmed = secret.trim();
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        if trimmed.is_empty() {
            conn.execute(
                "DELETE FROM settings WHERE key = 'spotify_client_secret'",
                [],
            )
            .map_err(CrateError::Database)?;
        } else {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('spotify_client_secret', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![trimmed],
            )
            .map_err(CrateError::Database)?;
        }
        Ok(())
    }

    /// Gets stored Spotify client secret from settings table, if any.
    pub fn get_client_secret(&self) -> Option<String> {
        let conn = self.conn.lock().ok()?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'spotify_client_secret'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .filter(|s| !s.trim().is_empty())
    }

    /// Saves PKCE verifier associated with state in settings table.
    pub fn save_pkce_verifier(&self, state: &str, verifier: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let key = format!("pkce_verifier_{state}");
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, verifier],
        )
        .map_err(CrateError::Database)?;
        Ok(())
    }

    /// Retrieves and consumes (deletes) PKCE verifier associated with state from settings table.
    pub fn get_and_consume_pkce_verifier(&self, state: &str) -> Option<String> {
        let conn = self.conn.lock().ok()?;
        let key = format!("pkce_verifier_{state}");
        let verifier: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                rusqlite::params![key],
                |r| r.get(0),
            )
            .ok();
        if verifier.is_some() {
            let _ = conn.execute(
                "DELETE FROM settings WHERE key = ?1",
                rusqlite::params![key],
            );
        }
        verifier
    }

    /// Retrieves the latest PKCE verifier from settings table if available, and cleans it up.
    pub fn get_latest_pkce_verifier(&self) -> Option<String> {
        let conn = self.conn.lock().ok()?;
        let row: Option<(String, String)> = conn
            .query_row(
                "SELECT key, value FROM settings WHERE key LIKE 'pkce_verifier_%' ORDER BY rowid DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        if let Some((key, verifier)) = row {
            let _ = conn.execute(
                "DELETE FROM settings WHERE key = ?1",
                rusqlite::params![key],
            );
            Some(verifier)
        } else {
            None
        }
    }

    /// Resolves active client ID (priority: explicit argument > stored setting > default).
    fn resolve_client_id(&self, explicit: Option<&str>) -> String {
        if let Some(c) = explicit {
            let trimmed = c.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        if let Some(stored) = self.get_client_id() {
            return stored;
        }
        DEFAULT_SPOTIFY_CLIENT_ID.to_string()
    }

    /// Generates the Spotify OAuth2 PKCE authorization URL and stores the code verifier in SQLite keyed by state.
    pub fn get_auth_url(
        &self,
        client_id_opt: Option<&str>,
        redirect_uri_opt: Option<&str>,
    ) -> Result<String> {
        let client_id = self.resolve_client_id(client_id_opt);
        if let Some(c) = client_id_opt {
            if !c.trim().is_empty() {
                let _ = self.set_client_id(c);
            }
        }
        let redirect_uri = redirect_uri_opt.unwrap_or(DEFAULT_SPOTIFY_REDIRECT_URI);

        let state = uuid::Uuid::new_v4().to_string();
        let (verifier, challenge) = Self::generate_pkce_pair();

        // Persist in settings table keyed by state for robust multi-request pairing
        let _ = self.save_pkce_verifier(&state, &verifier);

        // Also update in-memory fallback
        if let Ok(mut lock) = self.pending_code_verifier.lock() {
            *lock = Some(verifier);
        }

        let scopes =
            "user-read-playback-state user-read-currently-playing user-read-recently-played";
        let encoded_redirect = percent_encode(redirect_uri);
        let encoded_scopes = percent_encode(scopes);

        let auth_url = format!(
            "https://accounts.spotify.com/authorize?response_type=code&client_id={client_id}&scope={encoded_scopes}&redirect_uri={encoded_redirect}&code_challenge_method=S256&code_challenge={challenge}&state={state}"
        );

        Ok(auth_url)
    }

    /// Exchanges an OAuth2 authorization code for an Access and Refresh token.
    /// Exchange used by the callback server: the `state` must match a verifier Crate generated.
    pub async fn exchange_code_checked(
        &self,
        code: &str,
        client_id_opt: Option<&str>,
        redirect_uri_opt: Option<&str>,
        state: &str,
    ) -> Result<SpotifyAuthState> {
        let verifier = self
            .get_and_consume_pkce_verifier(state.trim())
            .ok_or_else(|| {
                CrateError::Discovery(
                    "Unknown or expired OAuth state: start the connection again from Crate"
                        .to_string(),
                )
            })?;
        self.exchange_code_with_verifier(code, client_id_opt, redirect_uri_opt, verifier)
            .await
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        client_id_opt: Option<&str>,
        redirect_uri_opt: Option<&str>,
        state_opt: Option<&str>,
    ) -> Result<SpotifyAuthState> {
        let trimmed_code = code.trim();
        // Support raw code or full callback URL/query string (e.g. "http://127.0.0.1:8888/callback?code=AQD...&state=xyz")
        let mut clean_code = trimmed_code;
        let mut extracted_state: Option<String> = state_opt
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        if let Some(pos) = trimmed_code.find("code=") {
            let after = &trimmed_code[pos + 5..];
            let end_pos = after
                .find('&')
                .or_else(|| after.find('#'))
                .unwrap_or(after.len());
            clean_code = &after[..end_pos];
        }
        if extracted_state.is_none() {
            if let Some(pos) = trimmed_code.find("state=") {
                let after = &trimmed_code[pos + 6..];
                let end_pos = after
                    .find('&')
                    .or_else(|| after.find('#'))
                    .unwrap_or(after.len());
                let st = &after[..end_pos];
                if !st.trim().is_empty() {
                    extracted_state = Some(st.trim().to_string());
                }
            }
        }
        let clean_code = clean_code.trim();

        if let Some(c) = client_id_opt {
            if !c.trim().is_empty() {
                let _ = self.set_client_id(c);
            }
        }

        // Retrieve exact verifier matching state, or fallback to memory / latest stored
        let verifier = if let Some(ref st) = extracted_state {
            self.get_and_consume_pkce_verifier(st)
                .or_else(|| {
                    self.pending_code_verifier
                        .lock()
                        .ok()
                        .and_then(|mut l| l.take())
                })
                .or_else(|| self.get_latest_pkce_verifier())
                .unwrap_or_default()
        } else {
            self.pending_code_verifier
                .lock()
                .ok()
                .and_then(|mut l| l.take())
                .or_else(|| self.get_latest_pkce_verifier())
                .unwrap_or_default()
        };

        self.exchange_code_with_verifier(clean_code, client_id_opt, redirect_uri_opt, verifier)
            .await
    }

    async fn exchange_code_with_verifier(
        &self,
        clean_code: &str,
        client_id_opt: Option<&str>,
        redirect_uri_opt: Option<&str>,
        verifier: String,
    ) -> Result<SpotifyAuthState> {
        let client_id = self.resolve_client_id(client_id_opt);
        let client_secret = self.get_client_secret();
        let redirect_uri = redirect_uri_opt.unwrap_or(DEFAULT_SPOTIFY_REDIRECT_URI);
        let mut params = vec![
            ("grant_type", "authorization_code".to_string()),
            ("code", clean_code.to_string()),
            ("redirect_uri", redirect_uri.to_string()),
            ("client_id", client_id.clone()),
            ("code_verifier", verifier),
        ];
        if let Some(ref secret) = client_secret {
            params.push(("client_secret", secret.clone()));
        }

        let mut req = self
            .http_client
            .post("https://accounts.spotify.com/api/token")
            .form(&params);

        if let Some(ref secret) = client_secret {
            let auth_header = format!(
                "Basic {}",
                BASE64_STANDARD.encode(format!("{client_id}:{secret}"))
            );
            req = req.header("Authorization", auth_header);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| CrateError::Discovery(format!("Spotify token request failed: {e}")))?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(CrateError::Discovery(format!(
                "Spotify authentication failed: {err_text}"
            )));
        }

        let token_data: SpotifyTokenResponse = resp
            .json()
            .await
            .map_err(|e| CrateError::Discovery(format!("Invalid Spotify token response: {e}")))?;

        let now_sec = Utc::now().timestamp();
        let expires_at = now_sec + token_data.expires_in;

        // Fetch user profile
        let user_profile = self
            .http_client
            .get("https://api.spotify.com/v1/me")
            .bearer_auth(&token_data.access_token)
            .send()
            .await
            .ok();

        let mut user_id = None;
        let mut user_name = None;

        if let Some(profile_resp) = user_profile {
            if profile_resp.status().is_success() {
                if let Ok(profile) = profile_resp.json::<SpotifyUserProfile>().await {
                    user_id = Some(profile.id);
                    user_name = profile.display_name;
                }
            }
        }

        let refresh_token_str = token_data.refresh_token.unwrap_or_default();

        // Save auth to database
        {
            let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            conn.execute(
                r#"
                INSERT INTO spotify_auth (
                    id, access_token, refresh_token, expires_at, user_id, user_name, is_connected
                ) VALUES ('current', ?1, ?2, ?3, ?4, ?5, 1)
                ON CONFLICT(id) DO UPDATE SET
                    access_token = excluded.access_token,
                    refresh_token = CASE WHEN excluded.refresh_token != '' THEN excluded.refresh_token ELSE spotify_auth.refresh_token END,
                    expires_at = excluded.expires_at,
                    user_id = excluded.user_id,
                    user_name = excluded.user_name,
                    is_connected = 1
                "#,
                rusqlite::params![
                    token_data.access_token,
                    refresh_token_str,
                    expires_at,
                    user_id,
                    user_name
                ],
            )
            .map_err(CrateError::Database)?;
        }

        Ok(SpotifyAuthState {
            is_connected: true,
            user_id,
            user_name,
            expires_at: Some(expires_at),
        })
    }

    /// Retrieves current connection state and profile from SQLite.
    pub fn get_auth_state(&self) -> Result<SpotifyAuthState> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let row: Option<(bool, Option<String>, Option<String>, i64)> = conn
            .query_row(
                "SELECT is_connected, user_id, user_name, expires_at FROM spotify_auth WHERE id = 'current'",
                [],
                |r| Ok((r.get::<_, i64>(0)? == 1, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()
            .map_err(CrateError::Database)?;

        match row {
            Some((is_connected, user_id, user_name, expires_at)) => Ok(SpotifyAuthState {
                is_connected,
                user_id,
                user_name,
                expires_at: Some(expires_at),
            }),
            None => Ok(SpotifyAuthState::default()),
        }
    }

    /// Disconnects Spotify integration and resets tokens.
    pub fn disconnect(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        conn.execute(
            "UPDATE spotify_auth SET is_connected = 0, access_token = '', refresh_token = '' WHERE id = 'current'",
            [],
        )
        .map_err(CrateError::Database)?;
        Ok(())
    }

    fn read_stored_tokens(&self) -> Result<Option<(String, String, i64, bool)>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        conn.query_row(
            "SELECT access_token, refresh_token, expires_at, is_connected FROM spotify_auth WHERE id = 'current'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, i64>(3)? == 1)),
        )
        .optional()
        .map_err(CrateError::Database)
    }

    /// Obtains a valid access token, refreshing it when it expires within a minute.
    ///
    /// Refreshes are serialized (the background sync and a UI command cannot refresh at the same
    /// time with the same single-use refresh token). A failed refresh of an expired token is an
    /// error instead of silently returning the stale token.
    pub async fn get_valid_access_token(
        &self,
        client_id_opt: Option<&str>,
    ) -> Result<Option<String>> {
        let _refresh_guard = self.refresh_lock.lock().await;
        // Re-read after acquiring the lock: another task may just have refreshed.
        let Some((access_token, refresh_token, expires_at, is_connected)) =
            self.read_stored_tokens()?
        else {
            return Ok(None);
        };
        if !is_connected || access_token.is_empty() {
            return Ok(None);
        }

        let now_sec = Utc::now().timestamp();
        if expires_at - now_sec >= 60 || refresh_token.is_empty() {
            return Ok(Some(access_token));
        }

        let client_id = self.resolve_client_id(client_id_opt);
        let client_secret = self.get_client_secret();
        let mut params = vec![
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", refresh_token.clone()),
            ("client_id", client_id.clone()),
        ];
        if let Some(ref secret) = client_secret {
            params.push(("client_secret", secret.clone()));
        }
        let mut req = self
            .http_client
            .post("https://accounts.spotify.com/api/token")
            .form(&params);
        if let Some(ref secret) = client_secret {
            let auth_header = format!(
                "Basic {}",
                BASE64_STANDARD.encode(format!("{client_id}:{secret}"))
            );
            req = req.header("Authorization", auth_header);
        }

        let failure = match req.send().await {
            Ok(response) if response.status().is_success() => {
                match response.json::<SpotifyTokenResponse>().await {
                    Ok(token_data) => {
                        let new_expires_at = now_sec + token_data.expires_in;
                        let new_refresh_token = token_data.refresh_token.unwrap_or(refresh_token);
                        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
                        conn.execute(
                        "UPDATE spotify_auth SET access_token = ?1, refresh_token = ?2, expires_at = ?3 WHERE id = 'current'",
                        rusqlite::params![token_data.access_token, new_refresh_token, new_expires_at],
                    )
                    .map_err(CrateError::Database)?;
                        return Ok(Some(token_data.access_token));
                    }
                    Err(e) => format!("invalid token response: {e}"),
                }
            }
            Ok(response) => format!("HTTP {}", response.status()),
            Err(e) => format!("network error: {e}"),
        };

        if expires_at <= now_sec {
            Err(CrateError::Discovery(format!(
                "Spotify session expired and could not be renewed ({failure}); reconnect Spotify"
            )))
        } else {
            log::warn!("Spotify token refresh failed ({failure}); using the current token until it expires");
            Ok(Some(access_token))
        }
    }

    /// Fetches currently playing track on Spotify.
    pub async fn get_currently_playing(&self) -> Result<Option<SpotifyNowPlaying>> {
        let token = match self.get_valid_access_token(None).await? {
            Some(t) => t,
            None => return Ok(None),
        };

        let resp = self
            .http_client
            .get("https://api.spotify.com/v1/me/player/currently-playing")
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                CrateError::Discovery(format!("Spotify currently-playing request failed: {e}"))
            })?;

        if resp.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(None);
        }

        if !resp.status().is_success() {
            return Ok(None);
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| CrateError::Discovery(format!("Invalid Spotify player JSON: {e}")))?;

        let is_playing = json
            .get("is_playing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let progress_ms = json.get("progress_ms").and_then(|v| v.as_u64());

        let item = json.get("item");
        let track_id = item
            .and_then(|i| i.get("id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let title = item
            .and_then(|i| i.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let duration_ms = item
            .and_then(|i| i.get("duration_ms"))
            .and_then(|v| v.as_u64());

        let artist = item
            .and_then(|i| i.get("artists"))
            .and_then(|a| a.as_array())
            .and_then(|arr| {
                let names: Vec<&str> = arr
                    .iter()
                    .filter_map(|art| art.get("name").and_then(|n| n.as_str()))
                    .collect();
                if names.is_empty() {
                    None
                } else {
                    Some(names.join(", "))
                }
            });

        let album = item
            .and_then(|i| i.get("album"))
            .and_then(|a| a.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let artwork_url = item
            .and_then(|i| i.get("album"))
            .and_then(|a| a.get("images"))
            .and_then(|imgs| imgs.as_array())
            .and_then(|arr| arr.first())
            .and_then(|img| img.get("url"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        let device_name = json
            .get("device")
            .and_then(|d| d.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Some(SpotifyNowPlaying {
            is_playing,
            track_id,
            title,
            artist,
            album,
            duration_ms,
            progress_ms,
            artwork_url,
            device_name,
        }))
    }

    /// Synchronizes Spotify listening history since the last recorded listen event timestamp.
    ///
    /// This is the **only** writer of Spotify listens (the live poller used to record the same
    /// plays a second time). Spotify only reports plays of 30 s or more and gives their end time;
    /// the listened time is the track length, capped by the gap since the previous play ended.
    /// Uses Spotify's `after` timestamp parameter to retrieve all tracks played since the last known date
    /// (e.g. from Friday 14:21 to Monday 13:00) and follows cursor pagination to ingest all available pages.
    pub async fn sync_recently_played(&self) -> Result<usize> {
        let token = match self.get_valid_access_token(None).await? {
            Some(t) => t,
            None => return Ok(0),
        };

        // 1. Determine the timestamp (in milliseconds) of the most recent Spotify listen in SQLite
        let last_played_at_ms: Option<i64> = {
            let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            conn.query_row(
                "SELECT MAX(played_at) FROM listen_events WHERE source = 'spotify'",
                [],
                |r| r.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten()
            .and_then(|ts_str| {
                chrono::DateTime::parse_from_rfc3339(&ts_str)
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(&ts_str, "%Y-%m-%d %H:%M:%S")
                            .map(|dt| dt.and_utc().fixed_offset())
                    })
                    .map(|dt| dt.timestamp_millis())
                    .ok()
            })
        };

        let mut current_after = last_played_at_ms;
        let mut previous_end_ms = last_played_at_ms;
        let mut total_synced = 0;
        let mut pages_fetched = 0;

        loop {
            if pages_fetched >= 10 {
                // Safety bound to avoid runaway pagination loops
                break;
            }

            let mut url =
                "https://api.spotify.com/v1/me/player/recently-played?limit=50".to_string();
            if let Some(after_ts) = current_after {
                url.push_str(&format!("&after={after_ts}"));
            }

            let resp = self
                .http_client
                .get(&url)
                .bearer_auth(&token)
                .send()
                .await
                .map_err(|e| {
                    CrateError::Discovery(format!("Spotify recently-played request failed: {e}"))
                })?;

            if !resp.status().is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                log::debug!("Spotify recently-played request returned non-success: {err_text}");
                break;
            }

            let parsed: SpotifyRecentlyPlayedResponse = resp.json().await.map_err(|e| {
                CrateError::Discovery(format!("Invalid Spotify recently-played JSON: {e}"))
            })?;

            if parsed.items.is_empty() {
                break;
            }

            let items_count = parsed.items.len();
            let mut page_synced = 0;

            // Oldest first, so each play can be bounded by the end of the previous one.
            let mut items: Vec<&SpotifyPlayHistoryItem> = parsed.items.iter().collect();
            items.sort_by_key(|item| parse_timestamp_ms(&item.played_at).unwrap_or(i64::MAX));

            for item in items {
                let title = item.track.name.trim();
                let artist_names: Vec<&str> = item
                    .track
                    .artists
                    .iter()
                    .map(|a| a.name.trim())
                    .filter(|n| !n.is_empty())
                    .collect();
                let artist = artist_names.join(", ");
                if title.is_empty() || artist.is_empty() {
                    continue;
                }

                let album = item.track.album.as_ref().and_then(|a| a.name.clone());
                let artwork_url = item
                    .track
                    .album
                    .as_ref()
                    .and_then(|a| a.images.as_ref())
                    .and_then(|imgs| imgs.first())
                    .map(|img| img.url.clone());

                let duration_ms = item.track.duration_ms;
                let ended_at_ms = parse_timestamp_ms(&item.played_at);
                let played_ms = listened_ms(duration_ms, previous_end_ms, ended_at_ms);
                previous_end_ms = ended_at_ms.or(previous_end_ms);

                let event = ListenEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    source: "spotify".to_string(),
                    track_id: item.track.id.clone(),
                    title: title.to_string(),
                    artist,
                    album,
                    duration_ms,
                    played_ms,
                    bpm: None,
                    key: None,
                    energy: None,
                    format: Some("spotify".to_string()),
                    artwork_url,
                    played_at: item.played_at.clone(),
                    session_id: None,
                    metadata_json: None,
                };

                if let Ok(true) = self.recorder.record_listen_event(&event) {
                    page_synced += 1;
                }
            }

            total_synced += page_synced;
            pages_fetched += 1;

            // Determine next cursor for pagination
            let next_after = parsed
                .cursors
                .as_ref()
                .and_then(|c| c.after.as_ref())
                .and_then(|s| s.parse::<i64>().ok());

            match next_after {
                Some(cursor_after) if Some(cursor_after) != current_after => {
                    current_after = Some(cursor_after);
                }
                _ => {
                    // No more pages or same cursor
                    break;
                }
            }

            if items_count < 50 {
                // Last page reached
                break;
            }
        }

        if total_synced > 0 {
            log::info!("Synchronized {total_synced} Spotify tracks across {pages_fetched} page(s) since {last_played_at_ms:?}");
        }

        Ok(total_synced)
    }

    /// Imports Spotify Extended Streaming History archive files (`endsong_*.json` or `Streaming_History_Audio_*.json`).
    ///
    /// Validates entries:
    /// - `ms_played` >= 30_000
    /// - Non-empty title and artist
    /// - Formats timestamp into ISO8601 UTC
    pub fn import_streaming_history_json(&self, json_content: &str) -> Result<SpotifyImportResult> {
        let value: serde_json::Value = serde_json::from_str(json_content).map_err(|e| {
            CrateError::Discovery(format!("Invalid Spotify JSON history format: {e}"))
        })?;

        let array = match value.as_array() {
            Some(arr) => arr,
            None => {
                return Err(CrateError::Discovery(
                    "Spotify history JSON must be an array of records".to_string(),
                ))
            }
        };

        let mut imported_count = 0;
        let mut skipped_count = 0;
        let mut total_minutes = 0;
        let mut events: Vec<ListenEvent> = Vec::new();

        for item in array {
            // Support both endsong_*.json and Streaming_History_Audio_*.json schemas
            let title = item
                .get("master_metadata_track_name")
                .or_else(|| item.get("trackName"))
                .or_else(|| item.get("track_name"))
                .or_else(|| item.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();

            let artist = item
                .get("master_metadata_album_artist_name")
                .or_else(|| item.get("artistName"))
                .or_else(|| item.get("artist_name"))
                .or_else(|| item.get("artist"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();

            let album = item
                .get("master_metadata_album_album_name")
                .or_else(|| item.get("albumName"))
                .or_else(|| item.get("album"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let ms_played: u64 = item
                .get("ms_played")
                .or_else(|| item.get("msPlayed"))
                .or_else(|| item.get("duration_ms"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let track_uri = item
                .get("spotify_track_uri")
                .or_else(|| item.get("track_id"))
                .or_else(|| item.get("id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let raw_time = item
                .get("ts")
                .or_else(|| item.get("endTime"))
                .or_else(|| item.get("timestamp"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if title.is_empty() || artist.is_empty() || ms_played < 30_000 {
                skipped_count += 1;
                continue;
            }

            // Parse the timestamp; a malformed one skips the record instead of panicking
            let Some(played_at) = parse_history_timestamp(raw_time) else {
                skipped_count += 1;
                continue;
            };

            let event = ListenEvent {
                id: uuid::Uuid::new_v4().to_string(),
                source: "spotify".to_string(),
                track_id: track_uri,
                title: title.to_string(),
                artist: artist.to_string(),
                album,
                duration_ms: ms_played,
                played_ms: ms_played,
                bpm: None,
                key: None,
                energy: None,
                format: Some("spotify".to_string()),
                artwork_url: None,
                played_at,
                session_id: None,
                metadata_json: None,
            };

            events.push(event);
        }

        // One transaction for the whole file
        let outcomes = self.recorder.record_listen_events(&events)?;
        for (event, inserted) in events.iter().zip(outcomes) {
            if inserted {
                imported_count += 1;
                total_minutes += event.played_ms / 60000;
            } else {
                skipped_count += 1;
            }
        }

        Ok(SpotifyImportResult {
            imported_count,
            skipped_count,
            total_minutes,
        })
    }

    /// Starts the OAuth callback server on `127.0.0.1:8888` for one sign-in attempt.
    ///
    /// The server only exists while a connection is in progress: it stops after a successful
    /// callback or after 10 minutes, and it only accepts callbacks whose `state` matches a
    /// verifier generated by Crate.
    pub fn ensure_loopback_server(&self, app_handle: tauri::AppHandle) {
        if self.oauth_server_running.swap(true, Ordering::SeqCst) {
            return;
        }
        let service = Arc::new(self.clone());
        let running = self.oauth_server_running.clone();
        tauri::async_runtime::spawn(async move {
            let addr = "127.0.0.1:8888";
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    log::info!(
                        "Spotify OAuth2 callback server listening on http://{addr}/callback"
                    );
                    let done = Arc::new(tokio::sync::Notify::new());
                    let router = Router::new()
                        .route("/callback", get(spotify_oauth_callback_handler))
                        .with_state(SpotifyOAuthServerState {
                            service,
                            app_handle,
                            done: done.clone(),
                        });
                    let shutdown = async move {
                        tokio::select! {
                            _ = done.notified() => {}
                            _ = tokio::time::sleep(std::time::Duration::from_secs(600)) => {}
                        }
                    };
                    if let Err(e) = axum::serve(listener, router)
                        .with_graceful_shutdown(shutdown)
                        .await
                    {
                        log::warn!("Spotify OAuth2 callback server terminated: {e}");
                    }
                    log::info!("Spotify OAuth2 callback server stopped");
                }
                Err(e) => {
                    log::warn!("Could not bind Spotify OAuth2 callback server on {addr}: {e}. Manual code exchange is still supported.");
                }
            }
            running.store(false, Ordering::SeqCst);
        });
    }
}

#[derive(Clone)]
struct SpotifyOAuthServerState {
    service: Arc<SpotifyTrackerService>,
    app_handle: tauri::AppHandle,
    done: Arc<tokio::sync::Notify>,
}

#[derive(Deserialize)]
struct OAuthCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Minimal result page shown in the browser after the Spotify redirect.
fn oauth_page(success: bool, title: &str, message: &str, detail: Option<&str>) -> Html<String> {
    let (accent, icon) = if success {
        ("#1DB954", "✓")
    } else {
        ("#ef4444", "✕")
    };
    let detail_html = detail
        .map(|d| format!(r#"<pre class="detail">{}</pre>"#, escape_html(d)))
        .unwrap_or_default();
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Crate - Connexion Spotify</title>
<style>
body {{ background:#0f1411; color:#f3f4f6; font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
       display:flex; align-items:center; justify-content:center; min-height:100vh; margin:0; padding:16px; }}
.card {{ background:#1a211d; border:1px solid {accent}55; border-radius:12px; padding:32px; max-width:440px; text-align:center; }}
.icon {{ color:{accent}; font-size:32px; font-weight:bold; }}
h1 {{ font-size:20px; margin:12px 0 8px; }}
p {{ color:#9ca3af; font-size:14px; line-height:1.5; }}
.detail {{ background:#0f1411; color:#fca5a5; padding:12px; border-radius:8px; font-size:12px; white-space:pre-wrap; text-align:left; }}
</style>
</head>
<body><div class="card"><div class="icon">{icon}</div><h1>{title}</h1><p>{message}</p>{detail_html}</div></body>
</html>"#,
        title = escape_html(title),
        message = escape_html(message),
    ))
}

async fn spotify_oauth_callback_handler(
    State(state): State<SpotifyOAuthServerState>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Html<String> {
    if let Some(err) = query.error {
        return oauth_page(
            false,
            "Connexion refusée",
            "L'autorisation Spotify a été refusée ou a échoué.",
            Some(&err),
        );
    }
    let Some(code) = query.code.filter(|c| !c.trim().is_empty()) else {
        return oauth_page(
            false,
            "Code manquant",
            "Aucun code d'autorisation n'a été reçu de Spotify.",
            None,
        );
    };
    let Some(oauth_state) = query.state.filter(|s| !s.trim().is_empty()) else {
        return oauth_page(false, "Requête refusée", "Paramètre state absent : cette redirection ne vient pas d'une connexion lancée depuis Crate.", None);
    };

    match state
        .service
        .exchange_code_checked(&code, None, None, &oauth_state)
        .await
    {
        Ok(auth_state) => {
            let _ = state.app_handle.emit("spotify-auth-changed", &auth_state);
            state.done.notify_one();
            let who = auth_state
                .user_name
                .clone()
                .or(auth_state.user_id.clone())
                .unwrap_or_default();
            oauth_page(
                true,
                "Spotify connecté",
                &format!("Compte {who} relié à Crate Pulse. Vous pouvez fermer cet onglet et revenir sur Crate."),
                None,
            )
        }
        Err(e) => oauth_page(
            false,
            "Échec de l'authentification",
            "Impossible d'échanger le code d'autorisation avec Spotify.",
            Some(&e.to_string()),
        ),
    }
}

/// Parses an RFC 3339 timestamp into Unix milliseconds.
fn parse_timestamp_ms(ts: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// Listened time of a play that ended at `ended_ms`: the track length, capped by the time elapsed
/// since the previous play ended (a skipped track cannot have been heard in full).
fn listened_ms(duration_ms: u64, previous_end_ms: Option<i64>, ended_ms: Option<i64>) -> u64 {
    let full = if duration_ms > 0 { duration_ms } else { 30_000 };
    match (previous_end_ms, ended_ms) {
        (Some(prev), Some(end)) if end > prev => {
            full.min((end - prev) as u64).max(30_000.min(full))
        }
        _ => full,
    }
}

/// Parses the timestamps of Spotify data exports: `2024-03-01T20:15:00Z` (extended history) or
/// `2024-03-01 20:15` (account data, UTC). Returns an RFC 3339 string.
fn parse_history_timestamp(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Some(dt.with_timezone(&Utc).to_rfc3339());
    }
    ["%Y-%m-%d %H:%M", "%Y-%m-%d %H:%M:%S"]
        .iter()
        .find_map(|fmt| chrono::NaiveDateTime::parse_from_str(raw, fmt).ok())
        .map(|dt| dt.and_utc().to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listened_time_is_capped_by_the_next_play() {
        // 4 min track, skipped after 50 s
        assert_eq!(listened_ms(240_000, Some(0), Some(50_000)), 50_000);
        // played in full after a long pause
        assert_eq!(listened_ms(240_000, Some(0), Some(3_600_000)), 240_000);
        // no previous play known
        assert_eq!(listened_ms(240_000, None, Some(10)), 240_000);
        // Spotify only reports plays of 30 s or more
        assert_eq!(listened_ms(240_000, Some(0), Some(5_000)), 30_000);
    }

    #[test]
    fn test_history_timestamps() {
        assert_eq!(
            parse_history_timestamp("2024-03-01T20:15:00Z").as_deref(),
            Some("2024-03-01T20:15:00+00:00")
        );
        assert_eq!(
            parse_history_timestamp("2024-03-01 20:15").as_deref(),
            Some("2024-03-01T20:15:00+00:00")
        );
        assert_eq!(parse_history_timestamp("2024-03"), None);
        assert_eq!(
            parse_history_timestamp("é"),
            None,
            "a short or non-ASCII value must not panic"
        );
    }

    #[test]
    fn test_escape_html_neutralizes_reflected_parameters() {
        assert_eq!(
            escape_html("<script>x</script>"),
            "&lt;script&gt;x&lt;/script&gt;"
        );
    }
}

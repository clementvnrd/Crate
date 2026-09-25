use std::sync::{Arc, Mutex};
use std::time::Instant;
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

struct SpotifyAccumulatorState {
    track_id: Option<String>,
    title: String,
    artist: String,
    album: Option<String>,
    duration_ms: u64,
    artwork_url: Option<String>,
    accumulated_ms: u64,
    last_poll: Option<Instant>,
    started_at: String,
    recorded: bool,
}

impl Default for SpotifyAccumulatorState {
    fn default() -> Self {
        Self {
            track_id: None,
            title: String::new(),
            artist: String::new(),
            album: None,
            duration_ms: 0,
            artwork_url: None,
            accumulated_ms: 0,
            last_poll: None,
            started_at: Utc::now().to_rfc3339(),
            recorded: false,
        }
    }
}

#[derive(Clone)]
pub struct SpotifyTrackerService {
    conn: Arc<Mutex<Connection>>,
    recorder: Arc<StatsRecorderService>,
    http_client: Client,
    pending_code_verifier: Arc<Mutex<Option<String>>>,
    accumulator: Arc<Mutex<SpotifyAccumulatorState>>,
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
    next: Option<String>,
    cursors: Option<SpotifyCursors>,
}

#[derive(Debug, Deserialize)]
struct SpotifyCursors {
    after: Option<String>,
    before: Option<String>,
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
            accumulator: Arc::new(Mutex::new(SpotifyAccumulatorState::default())),
        }
    }

    /// Generates a high-entropy PKCE code verifier (64 chars in [a-zA-Z0-9_-.~]) and computes its SHA-256 challenge.
    pub fn generate_pkce_pair() -> (String, String) {
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
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
            conn.execute("DELETE FROM settings WHERE key = 'spotify_client_secret'", [])
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
            let _ = conn.execute("DELETE FROM settings WHERE key = ?1", rusqlite::params![key]);
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
            let _ = conn.execute("DELETE FROM settings WHERE key = ?1", rusqlite::params![key]);
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
    pub fn get_auth_url(&self, client_id_opt: Option<&str>, redirect_uri_opt: Option<&str>) -> Result<String> {
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

        let scopes = "user-read-playback-state user-read-currently-playing user-read-recently-played";
        let encoded_redirect = percent_encode(redirect_uri);
        let encoded_scopes = percent_encode(scopes);

        let auth_url = format!(
            "https://accounts.spotify.com/authorize?response_type=code&client_id={client_id}&scope={encoded_scopes}&redirect_uri={encoded_redirect}&code_challenge_method=S256&code_challenge={challenge}&state={state}"
        );

        Ok(auth_url)
    }

    /// Exchanges an OAuth2 authorization code for an Access and Refresh token.
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
        let mut extracted_state: Option<String> = state_opt.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

        if let Some(pos) = trimmed_code.find("code=") {
            let after = &trimmed_code[pos + 5..];
            let end_pos = after.find('&').or_else(|| after.find('#')).unwrap_or(after.len());
            clean_code = &after[..end_pos];
        }
        if extracted_state.is_none() {
            if let Some(pos) = trimmed_code.find("state=") {
                let after = &trimmed_code[pos + 6..];
                let end_pos = after.find('&').or_else(|| after.find('#')).unwrap_or(after.len());
                let st = &after[..end_pos];
                if !st.trim().is_empty() {
                    extracted_state = Some(st.trim().to_string());
                }
            }
        }
        let clean_code = clean_code.trim();

        let client_id = self.resolve_client_id(client_id_opt);
        if let Some(c) = client_id_opt {
            if !c.trim().is_empty() {
                let _ = self.set_client_id(c);
            }
        }
        let client_secret = self.get_client_secret();
        let redirect_uri = redirect_uri_opt.unwrap_or(DEFAULT_SPOTIFY_REDIRECT_URI);

        // Retrieve exact verifier matching state, or fallback to memory / latest stored
        let verifier = if let Some(ref st) = extracted_state {
            self.get_and_consume_pkce_verifier(st)
                .or_else(|| self.pending_code_verifier.lock().ok().and_then(|mut l| l.take()))
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

        // Reset accumulator
        if let Ok(mut acc) = self.accumulator.lock() {
            *acc = SpotifyAccumulatorState::default();
        }

        Ok(())
    }

    /// Obtains a valid access token, auto-refreshing if expired.
    pub async fn get_valid_access_token(&self, client_id_opt: Option<&str>) -> Result<Option<String>> {
        let (access_token, refresh_token, expires_at, is_connected) = {
            let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
            let row: Option<(String, String, i64, bool)> = conn
                .query_row(
                    "SELECT access_token, refresh_token, expires_at, is_connected FROM spotify_auth WHERE id = 'current'",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, i64>(3)? == 1)),
                )
                .optional()
                .map_err(CrateError::Database)?;
            match row {
                Some(data) => data,
                None => return Ok(None),
            }
        };

        if !is_connected || access_token.is_empty() {
            return Ok(None);
        }

        let now_sec = Utc::now().timestamp();
        // If token expires in less than 60 seconds and refresh_token is present, refresh it
        if expires_at - now_sec < 60 && !refresh_token.is_empty() {
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

            let resp = req.send().await;

            if let Ok(response) = resp {
                if response.status().is_success() {
                    if let Ok(token_data) = response.json::<SpotifyTokenResponse>().await {
                        let new_expires_at = now_sec + token_data.expires_in;
                        let new_refresh_token = token_data.refresh_token.unwrap_or(refresh_token);

                        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
                        let _ = conn.execute(
                            "UPDATE spotify_auth SET access_token = ?1, refresh_token = ?2, expires_at = ?3 WHERE id = 'current'",
                            rusqlite::params![token_data.access_token, new_refresh_token, new_expires_at],
                        );

                        return Ok(Some(token_data.access_token));
                    }
                }
            }
        }

        Ok(Some(access_token))
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
            .map_err(|e| CrateError::Discovery(format!("Spotify currently-playing request failed: {e}")))?;

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

        let is_playing = json.get("is_playing").and_then(|v| v.as_bool()).unwrap_or(false);
        let progress_ms = json.get("progress_ms").and_then(|v| v.as_u64());

        let item = json.get("item");
        let track_id = item.and_then(|i| i.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string());
        let title = item.and_then(|i| i.get("name")).and_then(|v| v.as_str()).map(|s| s.to_string());
        let duration_ms = item.and_then(|i| i.get("duration_ms")).and_then(|v| v.as_u64());

        let artist = item.and_then(|i| i.get("artists")).and_then(|a| a.as_array()).and_then(|arr| {
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

        let album = item.and_then(|i| i.get("album")).and_then(|a| a.get("name")).and_then(|v| v.as_str()).map(|s| s.to_string());

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

    /// Background polling tick: tracks playback accumulation in real time.
    /// Whenever a track is played for >= 30 seconds, it automatically logs a `listen_event`.
    pub async fn poll_tick(&self) -> Result<()> {
        let now_playing_opt = self.get_currently_playing().await?;

        let mut acc = self.accumulator.lock().map_err(|_| CrateError::LockPoisoned)?;

        match now_playing_opt {
            Some(now_playing) if now_playing.is_playing && now_playing.title.is_some() && now_playing.artist.is_some() => {
                let current_title = now_playing.title.unwrap();
                let current_artist = now_playing.artist.unwrap();
                let current_duration = now_playing.duration_ms.unwrap_or(0);
                let current_artwork = now_playing.artwork_url;
                let current_album = now_playing.album;
                let current_track_id = now_playing.track_id;

                let is_same_track = acc.title == current_title && acc.artist == current_artist;

                if !is_same_track {
                    // Flush previously accumulated track if >= 1s and not yet recorded
                    if acc.accumulated_ms >= 1_000 && !acc.recorded && !acc.title.is_empty() {
                        let event = ListenEvent {
                            id: uuid::Uuid::new_v4().to_string(),
                            source: "spotify".to_string(),
                            track_id: acc.track_id.clone(),
                            title: acc.title.clone(),
                            artist: acc.artist.clone(),
                            album: acc.album.clone(),
                            duration_ms: acc.duration_ms,
                            played_ms: acc.accumulated_ms,
                            bpm: None,
                            key: None,
                            energy: None,
                            format: Some("spotify".to_string()),
                            artwork_url: acc.artwork_url.clone(),
                            played_at: acc.started_at.clone(),
                            session_id: None,
                            metadata_json: None,
                        };
                        let _ = self.recorder.record_listen_event(&event);
                    }

                    // Reset accumulator for new track
                    acc.track_id = current_track_id;
                    acc.title = current_title;
                    acc.artist = current_artist;
                    acc.album = current_album;
                    acc.duration_ms = current_duration;
                    acc.artwork_url = current_artwork;
                    acc.accumulated_ms = 0;
                    acc.last_poll = Some(Instant::now());
                    acc.started_at = Utc::now().to_rfc3339();
                    acc.recorded = false;
                } else {
                    // Same track still playing: accumulate time
                    if let Some(last) = acc.last_poll {
                        let elapsed_ms = last.elapsed().as_millis() as u64;
                        // Cap single tick delta to 10 seconds to avoid burst jumps
                        acc.accumulated_ms += elapsed_ms.min(10_000);
                    }
                    acc.last_poll = Some(Instant::now());

                    // Check if threshold reached (>= 30s)
                    if acc.accumulated_ms >= 30_000 && !acc.recorded {
                        let effective_played_ms = if acc.duration_ms > 0 {
                            acc.duration_ms.max(acc.accumulated_ms)
                        } else {
                            acc.accumulated_ms
                        };
                        let event = ListenEvent {
                            id: uuid::Uuid::new_v4().to_string(),
                            source: "spotify".to_string(),
                            track_id: acc.track_id.clone(),
                            title: acc.title.clone(),
                            artist: acc.artist.clone(),
                            album: acc.album.clone(),
                            duration_ms: acc.duration_ms,
                            played_ms: effective_played_ms,
                            bpm: None,
                            key: None,
                            energy: None,
                            format: Some("spotify".to_string()),
                            artwork_url: acc.artwork_url.clone(),
                            played_at: acc.started_at.clone(),
                            session_id: None,
                            metadata_json: None,
                        };
                        if let Ok(true) = self.recorder.record_listen_event(&event) {
                            acc.recorded = true;
                            log::info!("Recorded live Spotify listen: '{}' by '{}'", acc.title, acc.artist);
                        }
                    }
                }
            }
            _ => {
                // Not playing or no track
                if acc.accumulated_ms >= 1_000 && !acc.recorded && !acc.title.is_empty() {
                    let event = ListenEvent {
                        id: uuid::Uuid::new_v4().to_string(),
                        source: "spotify".to_string(),
                        track_id: acc.track_id.clone(),
                        title: acc.title.clone(),
                        artist: acc.artist.clone(),
                        album: acc.album.clone(),
                        duration_ms: acc.duration_ms,
                        played_ms: acc.accumulated_ms,
                        bpm: None,
                        key: None,
                        energy: None,
                        format: Some("spotify".to_string()),
                        artwork_url: acc.artwork_url.clone(),
                        played_at: acc.started_at.clone(),
                        session_id: None,
                        metadata_json: None,
                    };
                    let _ = self.recorder.record_listen_event(&event);
                }
                *acc = SpotifyAccumulatorState::default();
            }
        }

        Ok(())
    }

    /// Synchronizes Spotify listening history since the last recorded listen event timestamp.
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
        let mut total_synced = 0;
        let mut pages_fetched = 0;

        loop {
            if pages_fetched >= 10 {
                // Safety bound to avoid runaway pagination loops
                break;
            }

            let mut url = "https://api.spotify.com/v1/me/player/recently-played?limit=50".to_string();
            if let Some(after_ts) = current_after {
                url.push_str(&format!("&after={after_ts}"));
            }

            let resp = self
                .http_client
                .get(&url)
                .bearer_auth(&token)
                .send()
                .await
                .map_err(|e| CrateError::Discovery(format!("Spotify recently-played request failed: {e}")))?;

            if !resp.status().is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                log::debug!("Spotify recently-played request returned non-success: {err_text}");
                break;
            }

            let parsed: SpotifyRecentlyPlayedResponse = resp
                .json()
                .await
                .map_err(|e| CrateError::Discovery(format!("Invalid Spotify recently-played JSON: {e}")))?;

            if parsed.items.is_empty() {
                break;
            }

            let items_count = parsed.items.len();
            let mut page_synced = 0;

            for item in &parsed.items {
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
                let played_ms = if duration_ms > 0 { duration_ms } else { 30_000 };

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
        let value: serde_json::Value = serde_json::from_str(json_content)
            .map_err(|e| CrateError::Discovery(format!("Invalid Spotify JSON history format: {e}")))?;

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

            // Parse timestamp
            let played_at = if raw_time.contains('T') {
                raw_time.to_string()
            } else if raw_time.contains(' ') {
                format!("{}T{}:00Z", &raw_time[..10], &raw_time[11..])
            } else {
                Utc::now().to_rfc3339()
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

            match self.recorder.record_listen_event(&event) {
                Ok(true) => {
                    imported_count += 1;
                    total_minutes += ms_played / 60000;
                }
                _ => {
                    skipped_count += 1;
                }
            }
        }

        Ok(SpotifyImportResult {
            imported_count,
            skipped_count,
            total_minutes,
        })
    }

    /// Spawns the Tokio background loopback server on `127.0.0.1:8888` for automatic OAuth callback capture.
    pub fn start_loopback_server(self: Arc<Self>, app_handle: tauri::AppHandle) {
        tauri::async_runtime::spawn(async move {
            let addr = "127.0.0.1:8888";
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    log::info!("Spotify OAuth2 loopback server listening on http://{addr}/callback");
                    let router = Router::new()
                        .route("/callback", get(spotify_oauth_callback_handler))
                        .route("/health", get(|| async { "OK" }))
                        .with_state(SpotifyOAuthServerState {
                            service: self,
                            app_handle,
                        });

                    if let Err(e) = axum::serve(listener, router).await {
                        log::warn!("Spotify OAuth2 loopback server terminated: {e}");
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Could not bind Spotify OAuth2 loopback server on {addr}: {e}. Manual code exchange is still supported."
                    );
                }
            }
        });
    }
}

#[derive(Clone)]
struct SpotifyOAuthServerState {
    service: Arc<SpotifyTrackerService>,
    app_handle: tauri::AppHandle,
}

#[derive(Deserialize)]
struct OAuthCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn spotify_oauth_callback_handler(
    State(state): State<SpotifyOAuthServerState>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Html<String> {
    if let Some(err) = query.error {
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Crate - Connexion Spotify</title>
    <style>
        body {{
            background: #0a110d;
            color: #f3f4f6;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
            padding: 20px;
            box-sizing: border-box;
        }}
        .card {{
            background: rgba(26, 36, 30, 0.95);
            border: 1px solid rgba(239, 68, 68, 0.35);
            border-radius: 28px;
            padding: 44px 36px;
            max-width: 480px;
            width: 100%;
            text-align: center;
            box-shadow: 0 30px 60px -15px rgba(0, 0, 0, 0.7);
        }}
        .icon-wrap {{
            width: 72px;
            height: 72px;
            margin: 0 auto 24px;
            background: rgba(239, 68, 68, 0.15);
            border: 2px solid rgba(239, 68, 68, 0.4);
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            color: #ef4444;
            font-size: 32px;
            font-weight: bold;
        }}
        h1 {{ font-size: 22px; font-weight: 800; margin: 0 0 10px; color: #ffffff; }}
        p {{ font-size: 14px; line-height: 1.6; color: #9ca3af; margin: 0 0 20px; }}
        .badge {{
            display: inline-block;
            background: rgba(239, 68, 68, 0.2);
            color: #f87171;
            padding: 8px 16px;
            border-radius: 12px;
            font-size: 12px;
            font-family: monospace;
        }}
    </style>
</head>
<body>
    <div class="card">
        <div class="icon-wrap">✕</div>
        <h1>Connexion refusée</h1>
        <p>L'autorisation Spotify a été refusée ou a échoué.</p>
        <div class="badge">Erreur : {err}</div>
    </div>
</body>
</html>"#
        );
        return Html(html);
    }

    let code = match query.code {
        Some(c) if !c.trim().is_empty() => c,
        _ => {
            let html = r#"<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Crate - Connexion Spotify</title>
    <style>
        body {
            background: #0a110d;
            color: #f3f4f6;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
            padding: 20px;
            box-sizing: border-box;
        }
        .card {
            background: rgba(26, 36, 30, 0.95);
            border: 1px solid rgba(239, 68, 68, 0.35);
            border-radius: 28px;
            padding: 44px 36px;
            max-width: 480px;
            width: 100%;
            text-align: center;
            box-shadow: 0 30px 60px -15px rgba(0, 0, 0, 0.7);
        }
        .icon-wrap {
            width: 72px;
            height: 72px;
            margin: 0 auto 24px;
            background: rgba(239, 68, 68, 0.15);
            border: 2px solid rgba(239, 68, 68, 0.4);
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            color: #ef4444;
            font-size: 32px;
            font-weight: bold;
        }
        h1 { font-size: 22px; font-weight: 800; margin: 0 0 10px; color: #ffffff; }
        p { font-size: 14px; line-height: 1.6; color: #9ca3af; margin: 0; }
    </style>
</head>
<body>
    <div class="card">
        <div class="icon-wrap">✕</div>
        <h1>Code d'autorisation manquant</h1>
        <p>Aucun code d'autorisation n'a été fourni par Spotify.</p>
    </div>
</body>
</html>"#.to_string();
            return Html(html);
        }
    };

    match state
        .service
        .exchange_code(
            &code,
            None,
            Some(DEFAULT_SPOTIFY_REDIRECT_URI),
            query.state.as_deref(),
        )
        .await
    {
        Ok(auth_state) => {
            let _ = state.app_handle.emit("spotify-auth-changed", &auth_state);

            let user_info = auth_state
                .user_name
                .as_deref()
                .or(auth_state.user_id.as_deref())
                .unwrap_or("votre compte");

            let html = format!(
                r#"<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Crate - Connexion Spotify réussie</title>
    <style>
        body {{
            background: #0a110d;
            color: #f3f4f6;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
            padding: 20px;
            box-sizing: border-box;
        }}
        .card {{
            background: linear-gradient(180deg, rgba(18, 32, 23, 0.95) 0%, rgba(13, 23, 17, 0.95) 100%);
            border: 1px solid rgba(29, 185, 84, 0.35);
            border-radius: 28px;
            padding: 44px 36px;
            max-width: 480px;
            width: 100%;
            text-align: center;
            box-shadow: 0 30px 60px -15px rgba(0, 0, 0, 0.7), 0 0 40px rgba(29, 185, 84, 0.15);
        }}
        .icon-wrap {{
            width: 72px;
            height: 72px;
            margin: 0 auto 24px;
            background: rgba(29, 185, 84, 0.15);
            border: 2px solid rgba(29, 185, 84, 0.4);
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            box-shadow: 0 0 24px rgba(29, 185, 84, 0.25);
        }}
        .icon-wrap svg {{
            width: 36px;
            height: 36px;
            fill: none;
            stroke: #1DB954;
            stroke-width: 3;
            stroke-linecap: round;
            stroke-linejoin: round;
        }}
        h1 {{
            font-size: 22px;
            font-weight: 800;
            margin: 0 0 10px;
            color: #ffffff;
            letter-spacing: -0.02em;
        }}
        p {{
            font-size: 14px;
            line-height: 1.6;
            color: #9ca3af;
            margin: 0 0 20px;
        }}
        .user-tag {{
            display: inline-flex;
            align-items: center;
            gap: 8px;
            background: rgba(29, 185, 84, 0.12);
            border: 1px solid rgba(29, 185, 84, 0.3);
            color: #1DB954;
            padding: 8px 18px;
            border-radius: 9999px;
            font-size: 13px;
            font-weight: 700;
            margin-bottom: 24px;
        }}
        .hint {{
            font-size: 12px;
            color: #6b7280;
            border-top: 1px solid rgba(255, 255, 255, 0.08);
            padding-top: 18px;
            margin: 0;
        }}
    </style>
</head>
<body>
    <div class="card">
        <div class="icon-wrap">
            <svg viewBox="0 0 24 24"><path d="M20 6L9 17l-5-5"/></svg>
        </div>
        <h1>Connexion Spotify réussie !</h1>
        <p>Votre compte Spotify est désormais synchronisé avec Crate Pulse.</p>
        <div class="user-tag">
            <span>●</span>
            <span>Connecté : {user_info}</span>
        </div>
        <p class="hint">Vous pouvez fermer cet onglet en toute sécurité et revenir sur Crate.</p>
    </div>
</body>
</html>"#
            );
            Html(html)
        }
        Err(e) => {
            let html = format!(
                r#"<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Crate - Erreur Spotify</title>
    <style>
        body {{
            background: #0f1713;
            color: #f3f4f6;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
            padding: 20px;
            box-sizing: border-box;
        }}
        .card {{
            background: rgba(26, 36, 30, 0.95);
            border: 1px solid rgba(239, 68, 68, 0.35);
            border-radius: 28px;
            padding: 44px 36px;
            max-width: 480px;
            width: 100%;
            text-align: center;
            box-shadow: 0 30px 60px -15px rgba(0, 0, 0, 0.7);
        }}
        .icon-wrap {{
            width: 72px;
            height: 72px;
            margin: 0 auto 24px;
            background: rgba(239, 68, 68, 0.15);
            border: 2px solid rgba(239, 68, 68, 0.4);
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            color: #ef4444;
            font-size: 32px;
            font-weight: bold;
        }}
        h1 {{ font-size: 22px; font-weight: 800; margin: 0 0 10px; color: #ffffff; }}
        p {{ font-size: 14px; line-height: 1.6; color: #9ca3af; margin: 0 0 20px; }}
        .error-box {{
            background: rgba(239, 68, 68, 0.1);
            border: 1px solid rgba(239, 68, 68, 0.25);
            color: #fca5a5;
            padding: 12px;
            border-radius: 12px;
            font-size: 12px;
            font-family: monospace;
            word-break: break-all;
            text-align: left;
        }}
    </style>
</head>
<body>
    <div class="card">
        <div class="icon-wrap">✕</div>
        <h1>Échec de l'authentification</h1>
        <p>Impossible d'échanger le code d'autorisation avec Spotify.</p>
        <div class="error-box">{e}</div>
    </div>
</body>
</html>"#
            );
            Html(html)
        }
    }
}

use crate::services::library::mik::MikService;
use base64::Engine;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Mutex;

pub const BEATPORT_CLIENT_ID: &str = "0GIvkCltVIuPkkwSJHp6NDb3s0potTjLBQr388Dd";
pub const BEATPORT_REDIRECT_URI: &str = "https://api.beatport.com/v4/docs/oauth2-redirect.html";

static PENDING_CODE_VERIFIER: Mutex<Option<String>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportArtist {
    pub id: i64,
    pub name: String,
    pub slug: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportArtistDetail {
    pub id: i64,
    pub name: String,
    pub slug: Option<String>,
    pub image_url: Option<String>,
    pub biography: Option<String>,
    pub genres: Vec<String>,
    pub tracks_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportGenre {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportTrack {
    pub id: String,
    pub title: String,
    pub mix_name: Option<String>,
    pub artists: Vec<BeatportArtist>,
    pub remixers: Option<Vec<BeatportArtist>>,
    pub genre: String,
    pub genre_id: Option<i64>,
    pub release_name: Option<String>,
    pub release_date: String,
    pub duration_ms: i64,
    pub duration_formatted: String,
    pub key: Option<String>,
    pub bpm: Option<f64>,
    pub artwork_url: Option<String>,
    pub preview_url: Option<String>,
    pub waveform_url: Option<String>,
    pub is_favorite: bool,
    pub in_cart: bool,
    pub beatport_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportChart {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub image_url: String,
    pub genre_name: Option<String>,
    pub tracks_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportPlaylist {
    pub id: String,
    pub name: String,
    pub track_count: i64,
    pub image_url: Option<String>,
    pub is_public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportAuthState {
    pub is_authenticated: bool,
    pub username: Option<String>,
    pub token: Option<String>,
    pub refresh_token: Option<String>,
    pub has_subscription: bool,
    pub subscription_tier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportSearchResult {
    pub tracks: Vec<BeatportTrack>,
    pub artists: Vec<BeatportArtist>,
}

pub struct BeatportClient {
    http_client: reqwest::Client,
}

impl BeatportClient {
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        Self { http_client }
    }

    /// Generates PKCE authorization URL for official Beatport Identity Login
    pub fn generate_pkce_auth_url() -> String {
        const CHARSET: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
        let mut rng = rand::rng();
        let verifier: String = (0..64)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();

        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();

        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash);

        if let Ok(mut lock) = PENDING_CODE_VERIFIER.lock() {
            *lock = Some(verifier);
        }

        format!(
            "https://account.beatport.com/o/authorize/?response_type=code&client_id={}&code_challenge={}&code_challenge_method=S256&redirect_uri={}&scope=app%3Adocs%20user%3Adj",
            BEATPORT_CLIENT_ID,
            challenge,
            url_encode(BEATPORT_REDIRECT_URI)
        )
    }

    /// Exchanges PKCE code for real Beatport JWT access token
    pub async fn login_with_pkce_code(
        &self,
        code_input: &str,
    ) -> Result<BeatportAuthState, String> {
        let trimmed = code_input.trim();
        // If the user pasted a raw JWT or JSON token directly into the PKCE box
        if trimmed.starts_with("eyJ")
            || trimmed.starts_with('{')
            || trimmed.contains("access_token")
        {
            return self.validate_token(trimmed, None).await;
        }

        let code = extract_code_from_input(trimmed);
        if code.is_empty() {
            return Err("Code d'autorisation Beatport manquant ou invalide.".to_string());
        }

        let verifier = {
            let lock = PENDING_CODE_VERIFIER.lock().map_err(|e| e.to_string())?;
            lock.clone().unwrap_or_default()
        };

        let uris = [
            "https://api.beatport.com/v4/docs/oauth2-redirect.html",
            "https://api.beatport.com/v4/docs/",
        ];

        let mut last_error = String::new();
        for uri in uris {
            let params = [
                ("grant_type", "authorization_code"),
                ("code", code.as_str()),
                ("client_id", BEATPORT_CLIENT_ID),
                ("code_verifier", verifier.as_str()),
                ("redirect_uri", uri),
            ];

            let resp = self
                .http_client
                .post("https://account.beatport.com/o/token/")
                .form(&params)
                .send()
                .await;

            match resp {
                Ok(response) => {
                    if response.status().is_success() {
                        let json_resp: serde_json::Value = response
                            .json()
                            .await
                            .map_err(|e| format!("Réponse JSON invalide : {e}"))?;

                        let access_token = json_resp
                            .get("access_token")
                            .and_then(|v| v.as_str())
                            .ok_or("Aucun access_token dans la réponse Beatport")?
                            .to_string();

                        let refresh_token = json_resp
                            .get("refresh_token")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let user_info = self.get_my_account(&access_token).await.ok();
                        let display_name = user_info
                            .and_then(|u| {
                                u.get("username")
                                    .and_then(|v| v.as_str())
                                    .or_else(|| u.get("first_name").and_then(|v| v.as_str()))
                                    .map(|s| s.to_string())
                            })
                            .unwrap_or_else(|| "Beatport Subscriber".to_string());

                        let state = BeatportAuthState {
                            is_authenticated: true,
                            username: Some(display_name),
                            token: Some(access_token),
                            refresh_token,
                            has_subscription: true,
                            subscription_tier: Some("Beatport Streaming Pro".to_string()),
                        };
                        Self::save_persisted_auth(&state);
                        return Ok(state);
                    } else {
                        last_error = response.text().await.unwrap_or_default();
                    }
                }
                Err(e) => {
                    last_error = format!("Erreur réseau : {e}");
                }
            }
        }

        Err(format!(
            "Échec d'obtention du token Beatport : {last_error}"
        ))
    }

    /// Refreshes an expired access token using the refresh token
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<BeatportAuthState, String> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", BEATPORT_CLIENT_ID),
        ];

        let resp = self
            .http_client
            .post("https://account.beatport.com/o/token/")
            .header("Origin", "https://api.beatport.com")
            .header("Referer", "https://api.beatport.com/")
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Erreur rafraîchissement token : {e}"))?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(format!("Échec renouvellement token Beatport : {err_text}"));
        }

        let json_resp: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Réponse JSON invalide : {e}"))?;

        let new_access_token = json_resp
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or("Aucun access_token dans la réponse de rafraîchissement")?
            .to_string();

        let new_refresh_token = json_resp
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| Some(refresh_token.to_string()));

        let user_info = self.get_my_account(&new_access_token).await.ok();
        let display_name = user_info
            .and_then(|u| {
                u.get("username")
                    .and_then(|v| v.as_str())
                    .or_else(|| u.get("first_name").and_then(|v| v.as_str()))
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "Beatport Subscriber".to_string());

        let state = BeatportAuthState {
            is_authenticated: true,
            username: Some(display_name),
            token: Some(new_access_token),
            refresh_token: new_refresh_token,
            has_subscription: true,
            subscription_tier: Some("Beatport Streaming Pro".to_string()),
        };
        Self::save_persisted_auth(&state);
        Ok(state)
    }

    /// Validates an existing access token or manual token paste
    pub async fn validate_token(
        &self,
        token: &str,
        refresh_token: Option<&str>,
    ) -> Result<BeatportAuthState, String> {
        let mut access_token = token.trim().to_string();
        let mut final_refresh_token = refresh_token.map(|s| s.to_string());

        if access_token.starts_with('{') || access_token.contains("access_token") {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&access_token) {
                if let Some(at) = json.get("access_token").and_then(|v| v.as_str()) {
                    access_token = at.to_string();
                }
                if let Some(rt) = json.get("refresh_token").and_then(|v| v.as_str()) {
                    final_refresh_token = Some(rt.to_string());
                }
            }
        }

        // A token is only accepted if Beatport's account endpoint accepts it.
        let user_info = self
            .get_my_account(&access_token)
            .await
            .map_err(|e| format!("Jeton Beatport refusé : {e}"))?;
        let username = user_info
            .get("username")
            .and_then(|v| v.as_str())
            .or_else(|| user_info.get("first_name").and_then(|v| v.as_str()))
            .unwrap_or("Beatport User")
            .to_string();

        let state = BeatportAuthState {
            is_authenticated: true,
            username: Some(username),
            token: Some(access_token),
            refresh_token: final_refresh_token,
            has_subscription: true,
            subscription_tier: Some("Beatport Streaming Pro".to_string()),
        };
        Self::save_persisted_auth(&state);
        Ok(state)
    }

    /// Saves the session (Keychain on macOS) and the credentials file needed by beatportdl
    pub fn save_persisted_auth(auth: &BeatportAuthState) {
        super::auth_store::save(auth);
    }

    /// Loads the saved session, migrating a legacy plaintext file if present
    pub fn load_persisted_auth() -> Option<BeatportAuthState> {
        super::auth_store::load()
    }

    /// Signs out: removes the saved session and beatportdl's credentials file
    pub fn clear_persisted_auth() {
        super::auth_store::clear();
    }

    /// Fetches user profile account from /v4/my/account/
    pub async fn get_my_account(&self, token: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .http_client
            .get("https://api.beatport.com/v4/my/account/")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau account : {e}"))?;

        if !resp.status().is_success() {
            return Err("Token Beatport expiré ou invalide".to_string());
        }

        resp.json()
            .await
            .map_err(|e| format!("JSON profile invalide : {e}"))
    }

    /// Fetches real user playlists from /v4/my/playlists/
    pub async fn get_user_playlists(&self, token: &str) -> Result<Vec<BeatportPlaylist>, String> {
        let resp = self
            .http_client
            .get("https://api.beatport.com/v4/my/playlists/?per_page=100")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau playlists : {e}"))?;

        if resp.status().is_success() {
            if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                if let Some(results) = json_data.get("results").and_then(|r| r.as_array()) {
                    let mut playlists = Vec::new();
                    for item in results {
                        if let Some(id) = item.get("id").map(|v| v.to_string()) {
                            let name = item
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("Untitled")
                                .to_string();
                            let track_count = item
                                .get("track_count")
                                .and_then(|c| c.as_i64())
                                .unwrap_or(0);
                            let is_public = item
                                .get("is_public")
                                .and_then(|p| p.as_bool())
                                .unwrap_or(false);
                            playlists.push(BeatportPlaylist {
                                id,
                                name,
                                track_count,
                                image_url: None,
                                is_public,
                            });
                        }
                    }
                    return Ok(playlists);
                }
            }
        }

        Ok(Vec::new())
    }

    /// Fetches tracks of a user playlist (/v4/my/playlists/{id}/tracks/)
    pub async fn get_playlist_tracks(
        &self,
        token: Option<&str>,
        playlist_id: &str,
    ) -> Result<Vec<BeatportTrack>, String> {
        let url =
            format!("https://api.beatport.com/v4/my/playlists/{playlist_id}/tracks/?per_page=100");
        let mut req = self.http_client.get(&url);
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }

        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    let raw_items = json_data
                        .get("results")
                        .or_else(|| json_data.get("tracks"))
                        .and_then(|r| r.as_array());

                    if let Some(results) = raw_items {
                        let mut tracks = Vec::new();
                        for item in results {
                            let track_item = item.get("track").unwrap_or(item);
                            if let Some(t) = Self::parse_beatport_json_track(track_item) {
                                tracks.push(t);
                            }
                        }
                        return Ok(tracks);
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    /// Fetches tracks of an artist (/v4/catalog/artists/{id}/tracks/)
    pub async fn get_artist_tracks(
        &self,
        token: Option<&str>,
        artist_id: i64,
    ) -> Result<Vec<BeatportTrack>, String> {
        let url =
            format!("https://api.beatport.com/v4/catalog/artists/{artist_id}/tracks/?per_page=150");
        let mut req = self.http_client.get(&url);
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }

        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    let raw_items = json_data
                        .get("results")
                        .or_else(|| json_data.get("tracks"))
                        .and_then(|r| r.as_array());

                    if let Some(results) = raw_items {
                        let mut tracks = Vec::new();
                        for item in results {
                            let track_item = item.get("track").unwrap_or(item);
                            if let Some(t) = Self::parse_beatport_json_track(track_item) {
                                tracks.push(t);
                            }
                        }
                        return Ok(tracks);
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    /// Fetches artist detail (/v4/catalog/artists/{id}/)
    pub async fn get_artist_detail(
        &self,
        token: Option<&str>,
        artist_id: i64,
    ) -> Result<BeatportArtistDetail, String> {
        let url = format!("https://api.beatport.com/v4/catalog/artists/{artist_id}/");
        let mut req = self.http_client.get(&url);
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("Erreur artist detail : {e}"))?;
        if !resp.status().is_success() {
            return Err("Artiste non trouvé sur Beatport".to_string());
        }

        let item: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("JSON artist invalide : {e}"))?;

        let id = item.get("id").and_then(|i| i.as_i64()).unwrap_or(artist_id);
        let name = item
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Artist")
            .to_string();
        let slug = item
            .get("slug")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        let biography = item
            .get("biography")
            .and_then(|b| b.as_str())
            .map(|s| s.to_string());

        let image_url = item
            .get("image")
            .and_then(|img| img.get("uri").or_else(|| img.get("dynamic_uri")))
            .and_then(|u| u.as_str())
            .map(|s| s.replace("{w}x{h}", "500x500"));

        let genres = item
            .get("genres")
            .and_then(|arr| arr.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|g| {
                        g.get("name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();

        let tracks_count = item.get("track_count").and_then(|c| c.as_i64());

        Ok(BeatportArtistDetail {
            id,
            name,
            slug,
            image_url,
            biography,
            genres,
            tracks_count,
        })
    }

    /// Fetches tracks of a curated chart (/v4/catalog/charts/{id}/tracks/)
    pub async fn get_chart_tracks(
        &self,
        token: Option<&str>,
        chart_id: &str,
    ) -> Result<Vec<BeatportTrack>, String> {
        let url =
            format!("https://api.beatport.com/v4/catalog/charts/{chart_id}/tracks/?per_page=100");
        let mut req = self.http_client.get(&url);
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }

        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    let raw_items = json_data
                        .get("results")
                        .or_else(|| json_data.get("tracks"))
                        .and_then(|r| r.as_array());

                    if let Some(results) = raw_items {
                        let mut tracks = Vec::new();
                        for item in results {
                            let track_item = item.get("track").unwrap_or(item);
                            if let Some(t) = Self::parse_beatport_json_track(track_item) {
                                tracks.push(t);
                            }
                        }
                        return Ok(tracks);
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    /// Fetches user favorites / stars
    pub async fn get_user_favorites(
        &self,
        token: Option<&str>,
        _page: Option<i64>,
    ) -> Result<Vec<BeatportTrack>, String> {
        let url = "https://api.beatport.com/v4/my/stars/?per_page=100";
        let mut req = self.http_client.get(url);
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }

        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    let raw_items = json_data
                        .get("results")
                        .or_else(|| json_data.get("tracks"))
                        .and_then(|r| r.as_array());

                    if let Some(results) = raw_items {
                        let mut tracks = Vec::new();
                        for item in results {
                            let track_item = item.get("track").unwrap_or(item);
                            if let Some(t) = Self::parse_beatport_json_track(track_item) {
                                tracks.push(t);
                            }
                        }
                        return Ok(tracks);
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    /// Fetches user purchases
    pub async fn get_user_purchases(
        &self,
        token: Option<&str>,
    ) -> Result<Vec<BeatportTrack>, String> {
        let url = "https://api.beatport.com/v4/my/hype-purchases/?per_page=100";
        let mut req = self.http_client.get(url);
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }

        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    let raw_items = json_data
                        .get("results")
                        .or_else(|| json_data.get("tracks"))
                        .and_then(|r| r.as_array());

                    if let Some(results) = raw_items {
                        let mut tracks = Vec::new();
                        for item in results {
                            let track_item = item.get("track").unwrap_or(item);
                            if let Some(t) = Self::parse_beatport_json_track(track_item) {
                                tracks.push(t);
                            }
                        }
                        return Ok(tracks);
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    /// Fetches Beatport Genres
    pub async fn get_genres(&self, token: Option<&str>) -> Result<Vec<BeatportGenre>, String> {
        let url = "https://api.beatport.com/v4/catalog/genres/?per_page=100";
        let mut req = self.http_client.get(url);
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }

        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    if let Some(results) = json_data.get("results").and_then(|r| r.as_array()) {
                        let mut genres = Vec::new();
                        for item in results {
                            if let (Some(id), Some(name), Some(slug)) = (
                                item.get("id").and_then(|i| i.as_i64()),
                                item.get("name").and_then(|n| n.as_str()),
                                item.get("slug").and_then(|s| s.as_str()),
                            ) {
                                genres.push(BeatportGenre {
                                    id,
                                    name: name.to_string(),
                                    slug: slug.to_string(),
                                });
                            }
                        }
                        if !genres.is_empty() {
                            return Ok(genres);
                        }
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    /// Fetches Featured Curated Charts (filtering only charts with tracks)
    pub async fn get_featured_charts(
        &self,
        token: Option<&str>,
    ) -> Result<Vec<BeatportChart>, String> {
        let url = "https://api.beatport.com/v4/catalog/charts/?per_page=30";
        let mut req = self.http_client.get(url);
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }

        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    if let Some(results) = json_data.get("results").and_then(|r| r.as_array()) {
                        let mut charts = Vec::new();
                        for item in results {
                            let track_count = item
                                .get("track_count")
                                .and_then(|c| c.as_i64())
                                .unwrap_or(0);
                            if track_count == 0 {
                                continue;
                            }

                            if let (Some(id), Some(name)) = (
                                item.get("id").map(|v| v.to_string()),
                                item.get("name").and_then(|n| n.as_str()),
                            ) {
                                let desc = item
                                    .get("description")
                                    .and_then(|d| d.as_str())
                                    .map(|s| s.to_string());
                                let img = item
                                    .get("image")
                                    .and_then(|i| i.get("uri").or_else(|| i.get("dynamic_uri")))
                                    .and_then(|u| u.as_str())
                                    .unwrap_or("")
                                    .replace("{w}x{h}", "500x500");

                                charts.push(BeatportChart {
                                    id,
                                    title: name.to_string(),
                                    subtitle: None,
                                    description: desc,
                                    image_url: img,
                                    genre_name: None,
                                    tracks_count: Some(track_count),
                                });
                            }
                        }
                        if !charts.is_empty() {
                            return Ok(charts);
                        }
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    /// Fetches Beatport Tracks (Catalog / Genre)
    pub async fn get_top_tracks(
        &self,
        token: Option<&str>,
        genre_id: Option<i64>,
    ) -> Result<Vec<BeatportTrack>, String> {
        let url = if let Some(gid) = genre_id {
            format!("https://api.beatport.com/v4/catalog/tracks/?genre_id={gid}&per_page=50")
        } else {
            "https://api.beatport.com/v4/catalog/tracks/?per_page=50".to_string()
        };

        let mut req = self.http_client.get(&url);
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }

        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(json_data) = resp.json::<serde_json::Value>().await {
                    let raw_items = json_data
                        .get("results")
                        .or_else(|| json_data.get("tracks"))
                        .and_then(|r| r.as_array());

                    if let Some(results) = raw_items {
                        let mut tracks = Vec::new();
                        for item in results {
                            let track_item = item.get("track").unwrap_or(item);
                            if let Some(t) = Self::parse_beatport_json_track(track_item) {
                                tracks.push(t);
                            }
                        }
                        return Ok(tracks);
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    /// Comprehensive Search (tracks and artists) with auto-token fallback, auto-refresh on 401/403, and explicit auth error
    pub async fn search_catalog_full(
        &self,
        token: Option<&str>,
        query: &str,
    ) -> Result<BeatportSearchResult, String> {
        let mut active_token = token
            .map(|s| s.to_string())
            .or_else(|| Self::load_persisted_auth().and_then(|a| a.token));

        if active_token.is_none() {
            if let Some(auth) = Self::load_persisted_auth() {
                if let Some(ref rt) = auth.refresh_token {
                    if let Ok(new_auth) = self.refresh_access_token(rt).await {
                        active_token = new_auth.token;
                    }
                }
            }
        }

        let Some(mut current_token) = active_token else {
            return Err(
                "Authentification Beatport requise. Aucun jeton d'accès disponible.".to_string(),
            );
        };

        let url = format!(
            "https://api.beatport.com/v4/catalog/search/?q={}&per_page=50",
            url_encode(query)
        );

        let mut resp = self
            .http_client
            .get(&url)
            .bearer_auth(&current_token)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau Beatport search: {e}"))?;

        // If 401 or 403, attempt auto-refresh and retry once
        if resp.status().as_u16() == 401 || resp.status().as_u16() == 403 {
            let mut refreshed = false;
            if let Some(auth) = Self::load_persisted_auth() {
                if let Some(ref rt) = auth.refresh_token {
                    if let Ok(new_auth) = self.refresh_access_token(rt).await {
                        if let Some(new_token) = new_auth.token {
                            current_token = new_token;
                            refreshed = true;
                        }
                    }
                }
            }

            if refreshed {
                resp = self
                    .http_client
                    .get(&url)
                    .bearer_auth(&current_token)
                    .send()
                    .await
                    .map_err(|e| {
                        format!("Erreur réseau Beatport search (après rafraîchissement): {e}")
                    })?;
            }

            if resp.status().as_u16() == 401 || resp.status().as_u16() == 403 {
                return Err("Authentification Beatport requise (session expirée ou non autorisée). Veuillez vous reconnecter dans l'onglet Beatport.".to_string());
            }
        }

        if !resp.status().is_success() {
            return Err(format!("Erreur API Beatport (status: {})", resp.status()));
        }

        let json_data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Réponse JSON recherche Beatport invalide: {e}"))?;

        let mut tracks = Vec::new();
        let mut artists = Vec::new();

        // Parse artists
        if let Some(artist_items) = json_data.get("artists").and_then(|a| a.as_array()) {
            for a in artist_items {
                if let (Some(id), Some(name)) = (
                    a.get("id").and_then(|i| i.as_i64()),
                    a.get("name").and_then(|n| n.as_str()),
                ) {
                    let slug = a
                        .get("slug")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());
                    let img = a
                        .get("image")
                        .and_then(|i| i.get("uri").or_else(|| i.get("dynamic_uri")))
                        .and_then(|u| u.as_str())
                        .map(|s| s.replace("{w}x{h}", "500x500"));

                    artists.push(BeatportArtist {
                        id,
                        name: name.to_string(),
                        slug,
                        image_url: img,
                    });
                }
            }
        }

        // Parse tracks
        let raw_tracks = json_data
            .get("tracks")
            .or_else(|| json_data.get("results"))
            .and_then(|r| r.as_array());

        if let Some(results) = raw_tracks {
            for item in results {
                let track_item = item.get("track").unwrap_or(item);
                if let Some(t) = Self::parse_beatport_json_track(track_item) {
                    tracks.push(t);
                }
            }
        }

        Ok(BeatportSearchResult { tracks, artists })
    }

    /// Parse a JSON object from Beatport API into BeatportTrack with true Square Album Cover Artwork
    fn parse_beatport_json_track(item: &serde_json::Value) -> Option<BeatportTrack> {
        let id = item.get("id")?.to_string();
        let title = item.get("name")?.as_str()?.to_string();
        let mix_name = item
            .get("mix_name")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());

        let artists = item
            .get("artists")
            .and_then(|arr| arr.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| {
                        let a_img = a
                            .get("image")
                            .and_then(|i| i.get("uri").or_else(|| i.get("dynamic_uri")))
                            .and_then(|u| u.as_str())
                            .map(|s| s.replace("{w}x{h}", "500x500"));

                        Some(BeatportArtist {
                            id: a.get("id")?.as_i64().unwrap_or(0),
                            name: a.get("name")?.as_str()?.to_string(),
                            slug: a
                                .get("slug")
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                            image_url: a_img,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let genre = item
            .get("genre")
            .and_then(|g| g.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Electronic")
            .to_string();

        let genre_id = item
            .get("genre")
            .and_then(|g| g.get("id"))
            .and_then(|i| i.as_i64());

        let release_name = item
            .get("release")
            .and_then(|r| r.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        let release_date = item
            .get("new_release_date")
            .and_then(|d| d.as_str())
            .or_else(|| item.get("publish_date").and_then(|d| d.as_str()))
            .or_else(|| item.get("release_date").and_then(|d| d.as_str()))
            .unwrap_or("2026-01-01")
            .to_string();

        let duration_ms = item
            .get("length_ms")
            .and_then(|l| l.as_i64())
            .unwrap_or(240_000);

        let duration_formatted = format_ms_to_time(duration_ms);

        let key_raw = item
            .get("key")
            .and_then(|k| k.get("name").or(Some(k)))
            .and_then(|k| k.as_str());
        let key = key_raw.map(MikService::normalize_key);

        let bpm = item
            .get("bpm")
            .and_then(|b| b.as_f64().or_else(|| b.as_i64().map(|i| i as f64)));

        // PRIORITIZE SQUARE RELEASE ARTWORK (500x500) over banner/waveform
        let artwork_url = item
            .get("release")
            .and_then(|r| r.get("image"))
            .and_then(|img| img.get("uri").or_else(|| img.get("dynamic_uri")))
            .and_then(|u| u.as_str())
            .map(|s| s.replace("{w}x{h}", "500x500"))
            .or_else(|| {
                item.get("image")
                    .and_then(|img| img.get("uri").or_else(|| img.get("dynamic_uri")))
                    .and_then(|u| u.as_str())
                    .map(|s| s.replace("{w}x{h}", "500x500"))
            });

        let preview_url = item
            .get("sample_url")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        let waveform_url = item
            .get("waveform")
            .and_then(|w| w.get("large_url").or_else(|| w.get("url")))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                item.get("image")
                    .and_then(|i| i.get("uri"))
                    .and_then(|u| u.as_str())
                    .filter(|s| s.contains("1500x250"))
                    .map(|s| s.to_string())
            });

        Some(BeatportTrack {
            id,
            title,
            mix_name,
            artists,
            remixers: None,
            genre,
            genre_id,
            release_name,
            release_date,
            duration_ms,
            duration_formatted,
            key,
            bpm,
            artwork_url,
            preview_url,
            waveform_url,
            is_favorite: false,
            in_cart: false,
            beatport_url: None,
        })
    }
}

fn format_ms_to_time(ms: i64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{}:{:02}", mins, secs)
}

fn extract_code_from_input(input: &str) -> String {
    let trimmed = input.trim();
    if let Some(pos) = trimmed.find("code=") {
        let after = &trimmed[pos + 5..];
        let end = after.find('&').unwrap_or(after.len());
        return after[..end].to_string();
    }
    trimmed.to_string()
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

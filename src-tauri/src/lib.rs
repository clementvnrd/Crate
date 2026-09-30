mod commands;
mod db;
mod error;
// The native application menu is desktop-only.
#[cfg(feature = "desktop")]
mod menu;
mod models;
mod proxy;
mod services;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use db::Database;

/// Port of the localhost stream proxy HTTP server. Managed as Tauri state so that
/// `fetch_preview_stream` can embed it in the URL it returns to the frontend.
pub(crate) struct ProxyServerPort(pub u16);

/// Tracks in-flight prefetch tasks by release ID to prevent duplicate spawns.
pub(crate) struct PrefetchTracker(pub Arc<tokio::sync::Mutex<HashSet<String>>>);

/// Flag to signal cancellation of a running bulk import operation.
pub(crate) struct BulkImportCancelFlag(pub Arc<std::sync::atomic::AtomicBool>);

/// Flag to signal cancellation of a running page scan operation.
pub(crate) struct ScanPageCancelFlag(pub Arc<std::sync::atomic::AtomicBool>);

/// Set of release IDs that should be skipped by background enrichment.
/// Populated when the user cancels enrichment for individual releases.
pub(crate) struct EnrichmentSkipIds(pub Arc<tokio::sync::Mutex<HashSet<String>>>);

impl EnrichmentSkipIds {
    pub fn new() -> Self {
        Self(Arc::new(tokio::sync::Mutex::new(HashSet::new())))
    }
}

/// Session cache of artist/label page avatars (og:image), keyed by page URL, so the follow
/// popover can show a profile picture without re-scraping on every open. Local + ephemeral.
pub(crate) struct AvatarCache(pub Arc<tokio::sync::Mutex<HashMap<String, Option<String>>>>);

impl AvatarCache {
    pub fn new() -> Self {
        Self(Arc::new(tokio::sync::Mutex::new(HashMap::new())))
    }
}

/// Cache for pre-fetched release metadata populated during background enrichment after a page scan.
/// Keyed by release URL. Entries are consumed (removed) by `bulk_create_discovery_releases`.
pub(crate) struct ScanEnrichmentCache(
    pub Arc<tokio::sync::Mutex<HashMap<String, services::discovery::metadata::FetchedMetadata>>>,
);

impl ScanEnrichmentCache {
    pub fn new() -> Self {
        Self(Arc::new(tokio::sync::Mutex::new(HashMap::new())))
    }
}

impl PrefetchTracker {
    pub fn new() -> Self {
        Self(Arc::new(tokio::sync::Mutex::new(HashSet::new())))
    }
}

/// Stores cold-start file opened from macOS Finder / Open With.
/// Files opened with Crate (Finder "Open With", drag on the Dock icon) before the frontend is
/// ready are queued here; once the frontend has drained the queue, files are only sent as
/// `open-file` events. This avoids opening a cold-start file twice or losing all but the last one.
pub(crate) struct StartupFile {
    queue: std::sync::Mutex<Vec<String>>,
    frontend_ready: std::sync::atomic::AtomicBool,
}

impl StartupFile {
    pub fn new() -> Self {
        Self {
            queue: std::sync::Mutex::new(Vec::new()),
            frontend_ready: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Queues the file if the frontend is not ready yet; returns true when it should be emitted.
    #[cfg_attr(
        not(any(target_os = "macos", target_os = "ios", target_os = "android")),
        allow(dead_code)
    )]
    pub fn offer(&self, path: String) -> bool {
        let Ok(mut queue) = self.queue.lock() else {
            return true;
        };
        if self
            .frontend_ready
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return true;
        }
        queue.push(path);
        false
    }

    /// Called once by the frontend after it listens to `open-file`: returns the queued files.
    pub fn drain(&self) -> Vec<String> {
        let Ok(mut queue) = self.queue.lock() else {
            return Vec::new();
        };
        self.frontend_ready
            .store(true, std::sync::atomic::Ordering::SeqCst);
        std::mem::take(&mut *queue)
    }
}

use services::{
    discovery::n_transform::NsigSolverState, BackupService, DiscoveryService, FollowService,
    MikTrackerService, PlayerTrackerService, PlaylistService, RekordboxTrackerService,
    SettingsService, SpotifyTrackerService, StatsRecorderService, TagService,
};
// Desktop-only services and their backing crates are excluded from the mobile build.
#[cfg(feature = "desktop")]
use services::{
    export::CheckpointService, AlbumService, AnalysisService, AudioService,
    BeatportUpgraderService, DeviceService, DiagnosticsService, DuplicateService, ExportService,
    LibraryService, MediaControlsService, StandaloneService, SyncService,
};
use tauri::Manager;

/// Full Mixed In Key sync for the startup pass and the file watcher, on the blocking pool: it
/// reads the Mixed In Key database and the audio files' tags, which takes from milliseconds (an
/// unchanged library) to minutes (a first import), and must not park a runtime worker.
#[cfg(feature = "desktop")]
async fn sync_all_from_mik_db_blocking(
    conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    app_data_dir: std::path::PathBuf,
) -> error::Result<services::library::MikSyncResult> {
    error::run_blocking(move || {
        let guard = conn.lock().map_err(|_| error::CrateError::LockPoisoned)?;
        let artwork = services::ArtworkService::new(app_data_dir);
        services::library::MikDatabaseService::sync_all_from_mik_db(&guard, Some(&artwork))
    })
    .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Install a panic hook that writes to a crash log file. On Windows, release builds
    // use `windows_subsystem = "windows"` which hides all console output, so without
    // this hook panics during startup are completely invisible to the user.
    let crash_log_path = std::env::temp_dir().join("crate-crash.log");
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = format!(
            "[{}] PANIC: {}\nLocation: {:?}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            info,
            info.location(),
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("crate-crash.log"))
            .and_then(|mut f| std::io::Write::write_all(&mut f, message.as_bytes()));
        default_hook(info);
    }));

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Crash log path: {crash_log_path:?}");

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init());

    // Desktop-only plugins: the updater (mobile updates via the App Store / TestFlight),
    // the process plugin, and window-state (there are no OS windows to persist on mobile).
    #[cfg(feature = "desktop")]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_window_state::Builder::default().build());

    let builder = builder
        .invoke_handler(tauri::generate_handler![
            // App commands
            commands::app::get_app_info,
            commands::app::open_dev_tools,
            commands::app::close_dev_tools,
            #[cfg(feature = "desktop")]
            commands::app::rebuild_menu,
            #[cfg(feature = "desktop")]
            commands::app::set_menu_item_enabled,
            #[cfg(feature = "desktop")]
            commands::app::set_dialog_conflicting_items_enabled,
            #[cfg(feature = "desktop")]
            commands::app::set_onboarding_items_enabled,
            // Library commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::library::import_tracks,
            #[cfg(feature = "desktop")]
            commands::library::get_tracks,
            #[cfg(feature = "desktop")]
            commands::library::get_track,
            #[cfg(feature = "desktop")]
            commands::library::update_track,
            #[cfg(feature = "desktop")]
            commands::library::delete_tracks,
            #[cfg(feature = "desktop")]
            commands::library::delete_tracks_and_files,
            #[cfg(feature = "desktop")]
            commands::library::search_tracks,
            #[cfg(feature = "desktop")]
            commands::library::rescan_artwork,
            #[cfg(feature = "desktop")]
            commands::library::rescan_track_artwork,
            #[cfg(feature = "desktop")]
            commands::library::check_file_exists,
            #[cfg(feature = "desktop")]
            commands::library::validate_replacement_file,
            #[cfg(feature = "desktop")]
            commands::library::relocate_track,
            #[cfg(feature = "desktop")]
            commands::library::set_track_colors,
            #[cfg(feature = "desktop")]
            commands::library::update_tracks,
            #[cfg(feature = "desktop")]
            commands::library::set_track_artwork,
            #[cfg(feature = "desktop")]
            commands::library::delete_track_artwork,
            #[cfg(feature = "desktop")]
            commands::library::reextract_track_artwork,
            #[cfg(feature = "desktop")]
            commands::library::compare_track_artworks,
            #[cfg(feature = "desktop")]
            commands::library::import_tracks_with_duplicates,
            #[cfg(feature = "desktop")]
            commands::library::resolve_duplicate,
            #[cfg(feature = "desktop")]
            commands::library::resync_mixed_in_key_tracks,
            #[cfg(feature = "desktop")]
            commands::library::get_mik_database_status,
            #[cfg(feature = "desktop")]
            commands::library::sync_from_mik_database,
            #[cfg(feature = "desktop")]
            commands::library::prune_missing_tracks,
            commands::library::get_track_waveform,
            commands::library::get_track_cues,
            // Duplicate commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::duplicate::get_duplicate_groups,
            #[cfg(feature = "desktop")]
            commands::duplicate::get_duplicate_count,
            #[cfg(feature = "desktop")]
            commands::duplicate::ignore_duplicate_group,
            #[cfg(feature = "desktop")]
            commands::duplicate::unignore_duplicate_group,
            // Playback commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::playback::play_track,
            #[cfg(feature = "desktop")]
            commands::playback::pause,
            #[cfg(feature = "desktop")]
            commands::playback::resume,
            #[cfg(feature = "desktop")]
            commands::playback::stop,
            #[cfg(feature = "desktop")]
            commands::playback::seek,
            #[cfg(feature = "desktop")]
            commands::playback::set_volume,
            #[cfg(feature = "desktop")]
            commands::playback::set_speed,
            #[cfg(feature = "desktop")]
            commands::playback::get_playback_state,
            #[cfg(feature = "desktop")]
            commands::playback::get_audio_devices,
            #[cfg(feature = "desktop")]
            commands::playback::set_audio_device,
            // Standalone audio commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::standalone::read_standalone_track,
            #[cfg(feature = "desktop")]
            commands::standalone::get_recent_standalone_tracks,
            #[cfg(feature = "desktop")]
            commands::standalone::add_recent_standalone_track,
            #[cfg(feature = "desktop")]
            commands::standalone::remove_recent_standalone_track,
            #[cfg(feature = "desktop")]
            commands::standalone::clear_recent_standalone_tracks,
            #[cfg(feature = "desktop")]
            commands::standalone::take_startup_files,
            #[cfg(feature = "desktop")]
            commands::standalone::play_standalone_track,
            #[cfg(feature = "desktop")]
            // Album commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::album::add_player_album,
            #[cfg(feature = "desktop")]
            commands::album::get_player_albums,
            #[cfg(feature = "desktop")]
            commands::album::get_player_album_tracks,
            #[cfg(feature = "desktop")]
            commands::album::remove_player_album,
            // Tag commands
            commands::tag::get_tag_categories,
            commands::tag::create_tag_category,
            commands::tag::update_tag_category,
            commands::tag::delete_tag_category,
            commands::tag::create_tag,
            commands::tag::update_tag,
            commands::tag::move_tag,
            commands::tag::delete_tag,
            commands::tag::assign_tags,
            commands::tag::remove_tags,
            // Playlist commands
            commands::playlist::get_playlists,
            commands::playlist::create_playlist,
            commands::playlist::create_folder,
            commands::playlist::rename_playlist,
            commands::playlist::delete_playlist,
            commands::playlist::move_playlist,
            commands::playlist::get_playlist_tracks,
            commands::playlist::add_to_playlist,
            commands::playlist::remove_from_playlist,
            commands::playlist::reorder_playlist,
            commands::playlist::add_releases_to_playlist,
            commands::playlist::remove_releases_from_playlist,
            commands::playlist::get_playlist_releases,
            commands::playlist::create_smart_playlist,
            commands::playlist::update_smart_rules,
            commands::playlist::get_smart_playlist_tracks,
            commands::playlist::get_smart_playlist_releases,
            commands::playlist::preview_smart_rules_count,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::set_setting,
            // Device commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::device::get_devices,
            #[cfg(feature = "desktop")]
            commands::device::eject_device,
            #[cfg(feature = "desktop")]
            commands::device::reformat_device,
            // Export commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::export::export_playlists,
            #[cfg(feature = "desktop")]
            commands::export::get_device_exports,
            #[cfg(feature = "desktop")]
            commands::export::cancel_export,
            #[cfg(feature = "desktop")]
            commands::export::cleanup_failed_export,
            #[cfg(feature = "desktop")]
            commands::export::get_pending_checkpoint,
            #[cfg(feature = "desktop")]
            commands::export::delete_checkpoint,
            #[cfg(feature = "desktop")]
            commands::export::resume_export,
            #[cfg(feature = "desktop")]
            commands::export::export_rekordbox_xml,
            // Sync commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::sync::sync_device,
            #[cfg(feature = "desktop")]
            commands::sync::get_pending_sync_playlists,
            #[cfg(feature = "desktop")]
            commands::sync::has_pending_sync_changes,
            #[cfg(feature = "desktop")]
            commands::sync::is_syncing,
            #[cfg(feature = "desktop")]
            commands::sync::cancel_sync,
            #[cfg(feature = "desktop")]
            commands::sync::get_playlists_containing_track,
            #[cfg(feature = "desktop")]
            commands::sync::get_playlists_containing_tracks,
            #[cfg(feature = "desktop")]
            commands::sync::get_devices_for_playlist,
            #[cfg(feature = "desktop")]
            commands::sync::get_devices_for_playlists,
            // Diagnostics commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::diagnostics::get_diagnostic_entries,
            #[cfg(feature = "desktop")]
            commands::diagnostics::get_system_info,
            #[cfg(feature = "desktop")]
            commands::diagnostics::get_diagnostics_report,
            #[cfg(feature = "desktop")]
            commands::diagnostics::clear_diagnostic_entries,
            #[cfg(feature = "desktop")]
            commands::diagnostics::log_error,
            // Analysis commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::analysis::analyze_tracks,
            #[cfg(feature = "desktop")]
            commands::analysis::cancel_track_analysis,
            #[cfg(feature = "desktop")]
            commands::analysis::cancel_analysis,
            #[cfg(feature = "desktop")]
            commands::analysis::get_analyzed_tracks,
            // Discovery commands
            commands::discovery::toggle_discovery_track_liked,
            commands::discovery::create_discovery_release,
            commands::discovery::get_discovery_release,
            commands::discovery::get_discovery_releases,
            commands::discovery::update_discovery_release,
            commands::discovery::delete_discovery_release,
            commands::discovery::delete_discovery_releases,
            commands::discovery::assign_discovery_tags,
            commands::discovery::remove_discovery_tags,
            commands::discovery::check_discovery_matches,
            commands::discovery::add_tracks_to_discovery_release,
            commands::discovery::merge_discovery_releases,
            commands::discovery::fetch_release_metadata,
            commands::discovery::refresh_release_metadata,
            #[cfg(feature = "desktop")]
            commands::discovery::purchase_discovery_release,
            commands::discovery::fetch_preview_stream,
            commands::discovery::invalidate_preview_stream_cache,
            commands::discovery::get_discovery_audio_cache_size,
            commands::discovery::clear_discovery_audio_cache,
            commands::discovery::nsig_solve_callback,
            commands::discovery::set_discovery_release_artwork,
            commands::discovery::delete_discovery_release_artwork,
            commands::discovery::scan_discovery_page,
            commands::discovery::bulk_create_discovery_releases,
            commands::discovery::cancel_bulk_import,
            commands::discovery::cancel_scan_page,
            commands::discovery::skip_enrichment,
            commands::discovery::fetch_source_avatar,
            // Follow commands
            commands::follow::follow_source,
            commands::follow::follow_from_entity,
            commands::follow::unfollow_source,
            commands::follow::relink_followed_source,
            commands::follow::set_follow_enabled,
            commands::follow::set_follow_type,
            commands::follow::get_followed_sources,
            commands::follow::check_followed_source,
            commands::follow::check_all_followed_sources,
            commands::follow::set_release_new_flag,
            // Backup commands
            commands::backup::get_backup_info,
            commands::backup::create_backup,
            commands::backup::restore_from_backup,
            // Media controls commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::media_controls::update_now_playing,
            #[cfg(feature = "desktop")]
            commands::media_controls::update_playback_state,
            #[cfg(feature = "desktop")]
            commands::media_controls::clear_now_playing,
            // Cloud sync commands
            commands::cloud_sync::sign_in,
            commands::cloud_sync::sign_out,
            commands::cloud_sync::get_sync_status,
            commands::cloud_sync::sync_now,
            commands::cloud_sync::pull_now,
            commands::cloud_sync::get_recent_overrides,
            commands::cloud_sync::list_devices,
            commands::cloud_sync::rename_device,
            commands::cloud_sync::revoke_device,
            commands::cloud_sync::delete_cloud_vault,
            commands::cloud_sync::list_library_roots,
            commands::cloud_sync::create_library_root,
            commands::cloud_sync::rename_library_root,
            commands::cloud_sync::remove_library_root,
            commands::cloud_sync::set_library_root_mapping,
            commands::cloud_sync::suggest_library_roots,
            #[cfg(feature = "desktop")]
            commands::cloud_sync::locate_track,
            // Beatport commands
            commands::beatport::beatport_get_pkce_auth_url,
            commands::beatport::beatport_get_persisted_auth,
            commands::beatport::beatport_save_persisted_auth,
            commands::beatport::beatport_clear_persisted_auth,
            commands::beatport::beatport_login_pkce,
            commands::beatport::beatport_validate_token,
            commands::beatport::beatport_refresh_token,
            commands::beatport::beatport_get_genres,
            commands::beatport::beatport_get_featured_charts,
            commands::beatport::beatport_get_top_tracks,
            commands::beatport::beatport_get_chart_tracks,
            commands::beatport::beatport_get_artist_tracks,
            commands::beatport::beatport_get_artist_detail,
            commands::beatport::beatport_search,
            commands::beatport::beatport_get_user_playlists,
            commands::beatport::beatport_get_playlist_tracks,
            commands::beatport::beatport_get_user_favorites,
            commands::beatport::beatport_get_user_purchases,
            commands::beatport::beatport_download_tracks,
            // Beatport Upgrader commands (desktop-only)
            #[cfg(feature = "desktop")]
            commands::upgrader::get_upgrade_matches,
            #[cfg(feature = "desktop")]
            commands::upgrader::get_upgrade_count,
            #[cfg(feature = "desktop")]
            commands::upgrader::ignore_upgrade_match,
            #[cfg(feature = "desktop")]
            commands::upgrader::unignore_upgrade_match,
            #[cfg(feature = "desktop")]
            commands::upgrader::execute_upgrade_replacements,
            // Stats, Spotify & Rekordbox commands
            commands::stats::get_stats_summary,
            commands::stats::get_top_tracks,
            commands::stats::get_top_artists,
            commands::stats::get_harmonic_stats,
            commands::stats::get_bpm_stats,
            commands::stats::get_listening_heatmap,
            commands::stats::get_recent_listens,
            commands::stats::export_listening_history,
            commands::stats::set_spotify_client_id,
            commands::stats::spotify_get_client_id,
            commands::stats::set_spotify_client_secret,
            commands::stats::spotify_has_client_secret,
            commands::stats::spotify_get_auth_url,
            commands::stats::spotify_exchange_code,
            commands::stats::spotify_disconnect,
            commands::stats::spotify_get_auth_state,
            commands::stats::spotify_get_now_playing,
            commands::stats::spotify_import_history,
            commands::stats::sync_spotify_recently_played,
            commands::stats::rekordbox_detect_status,
            commands::stats::rekordbox_sync_history,
            commands::stats::rekordbox_import_history_xml,
            commands::stats::rekordbox_get_sessions,
            commands::stats::mik_detect_status,
            commands::stats::mik_tracker_get_enabled,
            commands::stats::mik_tracker_set_enabled,
        ])

        .setup(|app| {
            // Get Tauri's app data directory
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data directory: {e}"))?;

            // Ensure directory exists
            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = app_data_dir.join("crate.db");
            log::info!("Database path: {db_path:?}");

            let db = Database::new(db_path)?;
            let conn = db.connection();

            // Initialize services. Desktop-only services (file import/analysis, audio
            // playback, USB export/sync, device detection, diagnostics) are gated out of
            // the mobile build along with their commands and backing crates.
            #[cfg(feature = "desktop")]
            let library_service = LibraryService::new(conn.clone(), app_data_dir.clone());
            let tag_service = TagService::new(conn.clone());
            let playlist_service = PlaylistService::new(conn.clone());
            let settings_service = SettingsService::new(conn.clone());
            #[cfg(feature = "desktop")]
            let export_service = Arc::new(ExportService::new(conn.clone()));
            #[cfg(feature = "desktop")]
            let checkpoint_service = Arc::new(CheckpointService::new(conn.clone()));
            #[cfg(feature = "desktop")]
            let sync_service = SyncService::new(conn.clone(), export_service.clone());
            #[cfg(feature = "desktop")]
            let audio_service = AudioService::new()
                .map_err(|e| format!("Failed to initialize audio service: {e}"))?;
            #[cfg(feature = "desktop")]
            let device_service = DeviceService::new();
            #[cfg(feature = "desktop")]
            let diagnostics_service = DiagnosticsService::new(app_data_dir.clone());
            #[cfg(feature = "desktop")]
            let analysis_service = AnalysisService::new(conn.clone());
            #[cfg(feature = "desktop")]
            let duplicate_service = DuplicateService::new(conn.clone());
            #[cfg(feature = "desktop")]
            let upgrader_service = BeatportUpgraderService::new(conn.clone());
            #[cfg(feature = "desktop")]
            let standalone_service = StandaloneService::new(conn.clone(), app_data_dir.clone());
            #[cfg(feature = "desktop")]
            let album_service = AlbumService::new(conn.clone(), app_data_dir.clone());
            let backup_service = BackupService::new(conn.clone());
            let discovery_service = DiscoveryService::new(conn.clone(), app_data_dir.clone());
            let follow_service = FollowService::new(conn.clone(), app_data_dir.clone());

            // Load saved audio device setting (desktop-only: no rodio playback on mobile)
            #[cfg(feature = "desktop")]
            if let Ok(settings) = settings_service.get_settings() {
                if let Some(device_name) = settings.audio_device {
                    if !device_name.is_empty() {
                        let _ = audio_service.set_device(Some(device_name));
                    }
                }
            }

            // Register services with Tauri
            app.manage(backup_service);

            // Spawn auto-backup check (runs in background, does not block startup)
            {
                let conn = conn.clone();
                let app_handle = app.handle().clone();
                let app_version = app.package_info().version.to_string();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = crate::services::backup::run_auto_backup_if_due(
                        conn,
                        app_handle,
                        app_version,
                    )
                    .await
                    {
                        log::warn!("Auto-backup failed: {e}");
                    }
                });
            }

            #[cfg(feature = "desktop")]
            app.manage(library_service);
            app.manage(tag_service);
            app.manage(playlist_service);
            app.manage(settings_service);
            #[cfg(feature = "desktop")]
            app.manage(export_service);
            #[cfg(feature = "desktop")]
            app.manage(checkpoint_service);
            #[cfg(feature = "desktop")]
            app.manage(sync_service);
            #[cfg(feature = "desktop")]
            app.manage(audio_service);
            #[cfg(feature = "desktop")]
            app.manage(device_service);
            #[cfg(feature = "desktop")]
            app.manage(diagnostics_service);
            #[cfg(feature = "desktop")]
            app.manage(analysis_service);
            #[cfg(feature = "desktop")]
            app.manage(duplicate_service);
            #[cfg(feature = "desktop")]
            app.manage(upgrader_service);
            #[cfg(feature = "desktop")]
            app.manage(standalone_service);
            #[cfg(feature = "desktop")]
            app.manage(album_service);
            app.manage(StartupFile::new());
            app.manage(discovery_service);
            app.manage(follow_service);

            // Crate Pulse & Stats services
            let stats_recorder = Arc::new(StatsRecorderService::new(conn.clone()));
            let spotify_tracker = Arc::new(SpotifyTrackerService::new(conn.clone(), stats_recorder.clone()));
            let rekordbox_tracker = RekordboxTrackerService::new(conn.clone(), stats_recorder.clone());
            let mik_tracker_enabled = conn
                .lock()
                .ok()
                .and_then(|c| {
                    c.query_row(
                        "SELECT value FROM settings WHERE key = ?1",
                        [crate::services::stats::mik::MIK_TRACKER_SETTING],
                        |r| r.get::<_, String>(0),
                    )
                    .ok()
                })
                .is_some_and(|v| v == "true");
            let mik_tracker = Arc::new(MikTrackerService::new(stats_recorder.clone(), mik_tracker_enabled));
            let player_tracker = PlayerTrackerService::new(stats_recorder.clone());

            // The managed services are the same instances as the background workers below.
            app.manage(StatsRecorderService::new(conn.clone()));
            app.manage((*spotify_tracker).clone());
            app.manage(rekordbox_tracker);
            app.manage((*mik_tracker).clone());
            app.manage(player_tracker);

            // Background Spotify sync: the "recently played" history is the single source of
            // Spotify listens (startup catch-up, then every minute).
            {
                let spotify_svc = spotify_tracker.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        if let Err(e) = spotify_svc.sync_recently_played().await {
                            log::debug!("Spotify recently-played sync: {e}");
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    }
                });
            }

            // Background Mixed In Key 11 polling worker (monitors audio loaded in MIK)
            {
                let mik_svc = mik_tracker.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                        if let Err(e) = mik_svc.poll_tick().await {
                            log::debug!("Mixed In Key background poll tick: {e}");
                        }
                    }
                });
            }

            // Background watch loop: poll followed sources on the configured cadence.
            // No-ops (and makes no network requests) when nothing is followed.
            crate::services::follow::watch::start_watching(
                app.handle().clone(),
                conn.clone(),
                app_data_dir.clone(),
            );
            app.manage(NsigSolverState::new());
            app.manage(PrefetchTracker::new());
            app.manage(BulkImportCancelFlag(Arc::new(
                std::sync::atomic::AtomicBool::new(false),
            )));
            app.manage(ScanPageCancelFlag(Arc::new(
                std::sync::atomic::AtomicBool::new(false),
            )));
            app.manage(ScanEnrichmentCache::new());
            app.manage(EnrichmentSkipIds::new());
            app.manage(AvatarCache::new());


            // Cloud sync: build the Firebase backend if a config file is present
            // (degrades gracefully to "unavailable" when it isn't), manage the runtime
            // state, and spawn the session-restore + pull/push/GC sync task.
            {
                use crate::services::cloud_sync::{
                    auth, backend, config, hlc, runtime::CloudSyncState,
                };

                let cloud_config =
                    config::load_cloud_config(app.path().app_config_dir().ok().as_deref())
                        .unwrap_or(None);

                let cloud_backend = cloud_config.as_ref().and_then(|cfg| {
                    match backend::build_default_backend(cfg) {
                        Ok(b) => Some(b),
                        Err(e) => {
                            log::warn!("cloud_sync: backend init failed: {e}");
                            None
                        }
                    }
                });

                let device_id = {
                    let guard = conn.lock().expect("db mutex poisoned");
                    hlc::load_node_id(&guard)
                        .map(|n| format!("{n:08x}"))
                        .unwrap_or_else(|_| "00000000".to_string())
                };
                let device_name = {
                    let guard = conn.lock().expect("db mutex poisoned");
                    auth::read_state(&guard, "device_name")
                        .ok()
                        .flatten()
                        .filter(|s| !s.is_empty())
                }
                .unwrap_or_else(|| {
                    // `sysinfo` is desktop-only; mobile falls back to a generic name until
                    // the device is renamed.
                    #[cfg(feature = "desktop")]
                    {
                        sysinfo::System::host_name().unwrap_or_else(|| "Crate device".to_string())
                    }
                    #[cfg(not(feature = "desktop"))]
                    {
                        "Crate device".to_string()
                    }
                });
                let app_version = app.package_info().version.to_string();

                let cloud_state = Arc::new(CloudSyncState::new(
                    cloud_backend,
                    cloud_config,
                    conn.clone(),
                    device_id,
                    device_name,
                    app_version,
                    app.handle().clone(),
                ));
                app.manage(cloud_state.clone());

                // Restore any persisted session, then run one serialized sync task:
                // each tick pulls other devices' changes (polled every ~10s) and pushes
                // ours once the dirty queue goes quiescent (~15s after the last
                // mutation). "Sync now" pushes immediately via the command. A one-shot
                // GC sweep at startup reclaims superseded blobs past their grace window.
                tauri::async_runtime::spawn(async move {
                    cloud_state.restore_session().await;
                    if !cloud_state.is_available() {
                        return;
                    }
                    // Best-effort GC sweep once per session (signed-in only).
                    if cloud_state.is_signed_in().await {
                        if let Err(e) = cloud_state.run_gc_sweep().await {
                            log::warn!("cloud_sync: gc sweep failed: {e}");
                        }
                    }
                    let quiescent = std::time::Duration::from_secs(15);
                    let mut tick: u64 = 0;
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        tick += 1;
                        if !cloud_state.is_signed_in().await {
                            continue;
                        }
                        // Pull every other tick (~10s) to halve manifest reads; the
                        // etag gate keeps an unchanged poll cheap.
                        if tick.is_multiple_of(2) {
                            if let Err(e) = cloud_state.run_pull().await {
                                // A transient connectivity failure is expected while
                                // offline (already surfaced as the `Offline` phase) —
                                // don't spam warnings every poll.
                                if e.is_transient() {
                                    log::debug!("cloud_sync: pull offline: {e}");
                                } else {
                                    log::warn!("cloud_sync: pull failed: {e}");
                                }
                            }
                        }
                        match cloud_state.dirty_quiescent(quiescent) {
                            Ok(true) => {
                                if let Err(e) = cloud_state.run_push().await {
                                    if e.is_transient() {
                                        log::debug!("cloud_sync: push offline: {e}");
                                    } else {
                                        log::warn!("cloud_sync: debounced push failed: {e}");
                                    }
                                }
                            }
                            Ok(false) => {}
                            Err(e) => log::warn!("cloud_sync: dirty check failed: {e}"),
                        }
                    }
                });
            }

            // Start device monitoring (desktop-only: USB device attach/detach)
            #[cfg(feature = "desktop")]
            {
                let device_service = app.state::<DeviceService>();
                device_service.start_monitoring(app.handle().clone());
            }

            // Automatic initial sync & background watcher for Mixed In Key database (desktop-only)
            #[cfg(feature = "desktop")]
            {
                let app_handle = app.handle().clone();
                let mik_conn_arc = conn.clone();
                let artwork_svc = app_data_dir.clone();

                tauri::async_runtime::spawn(async move {
                    use tauri::Emitter;

                    // Delay 600ms so initial frontend stores load instantly without lock contention
                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

                    if let Ok(res) = sync_all_from_mik_db_blocking(mik_conn_arc.clone(), artwork_svc.clone()).await {
                        if res.added > 0 || res.updated > 0 || res.removed > 0 {
                            log::info!("Initial MIK sync found updates, notifying frontend...");
                            let _ = app_handle.emit("mik-database-synced", ());
                            let _ = app_handle.emit("duplicates-updated", ());
                        }
                    }

                    /// Fingerprint of the Mixed In Key database: modification time and size of the
                    /// main file and its WAL. The `-shm` file is ignored on purpose: SQLite touches it
                    /// on every read (including Crate's own read-only sync), which re-triggered syncs.
                    fn get_latest_mik_mtime(db_path: &std::path::Path) -> Option<(std::time::SystemTime, u64)> {
                        let wal_path = std::path::PathBuf::from(format!("{}-wal", db_path.to_string_lossy()));
                        let mut latest: Option<std::time::SystemTime> = None;
                        let mut total_size = 0u64;
                        for p in [db_path, wal_path.as_path()] {
                            if let Ok(meta) = std::fs::metadata(p) {
                                total_size += meta.len();
                                if let Ok(mtime) = meta.modified() {
                                    latest = Some(latest.map_or(mtime, |prev| prev.max(mtime)));
                                }
                            }
                        }
                        latest.map(|mtime| (mtime, total_size))
                    }

                    let mut last_synced_mtime: Option<(std::time::SystemTime, u64)> = crate::services::library::MikDatabaseService::find_mik_db_path()
                        .and_then(|p| get_latest_mik_mtime(&p));

                    let mut pending_change = false;
                    let mut detected_mtime: Option<(std::time::SystemTime, u64)> = None;
                    let mut last_activity_time = tokio::time::Instant::now();

                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

                        if let Some(mik_path) = crate::services::library::MikDatabaseService::find_mik_db_path() {
                            if let Some(current_mtime) = get_latest_mik_mtime(&mik_path) {
                                let is_newer = last_synced_mtime != Some(current_mtime);
                                if is_newer && detected_mtime != Some(current_mtime) {
                                    detected_mtime = Some(current_mtime);
                                    last_activity_time = tokio::time::Instant::now();
                                    pending_change = true;
                                }
                            }

                            if pending_change && last_activity_time.elapsed() >= std::time::Duration::from_millis(500) {
                                log::info!("Debounce window passed (500ms inactivity). Executing live Mixed In Key sync...");
                                match sync_all_from_mik_db_blocking(mik_conn_arc.clone(), artwork_svc.clone()).await {
                                    Ok(res) => {
                                        log::info!(
                                            "Live MIK sync completed: {} added, {} updated, {} removed (total: {})",
                                            res.added,
                                            res.updated,
                                            res.removed,
                                            res.total
                                        );
                                        let _ = app_handle.emit("mik-database-synced", ());
                                        let _ = app_handle.emit("duplicates-updated", ());
                                    }
                                    Err(e) => {
                                        log::warn!("Live MIK sync failed: {e}");
                                    }
                                }
                                last_synced_mtime = crate::services::library::MikDatabaseService::find_mik_db_path()
                                    .and_then(|p| get_latest_mik_mtime(&p))
                                    .or(detected_mtime);
                                pending_change = false;
                            }
                        }
                    }
                });
            }

            // Build and set the application menu (desktop-only: no native menu on mobile)
            #[cfg(feature = "desktop")]
            {
                let menu = menu::build_menu(app.handle())?;
                app.set_menu(menu)?;
                menu::setup_menu_handlers(app.handle());

                // Manage fullscreen label translations for dynamic menu text toggling
                app.manage(std::sync::Mutex::new(menu::FullscreenLabels::default()));
            }

            // Initialize media controls (desktop-only: souvlaki Now Playing / media keys)
            #[cfg(feature = "desktop")]
            {
                let media_controls_service = MediaControlsService::new(app.handle());
                app.manage(media_controls_service);
            }

            // Bind a stream proxy HTTP server on a random OS-assigned port. Real HTTP is required
            // for WKWebView's AVFoundation media layer to correctly handle Range requests during
            // seeking; WKWebView's custom URI scheme handler (WKURLSchemeHandler) does not reliably
            // support the multi-request, cancellation-heavy lifecycle that AVFoundation uses.
            let std_listener = std::net::TcpListener::bind("127.0.0.1:0")
                .map_err(|e| format!("Failed to bind stream proxy: {e}"))?;
            let proxy_port = std_listener
                .local_addr()
                .map_err(|e| format!("Failed to get proxy address: {e}"))?
                .port();
            log::info!("Stream proxy HTTP server bound to 127.0.0.1:{proxy_port}");
            app.manage(ProxyServerPort(proxy_port));

            // Force HTTP/1.1 with no idle connection pooling. HTTP/2 multiplexes all requests
            // over one TCP connection; when AVFoundation drops a body mid-stream (seek), hyper
            // sends RST_STREAM and transitions the connection back to idle — but if the next
            // Range request arrives before that cleanup completes, hyper tries to reuse the
            // half-torn-down connection and fails with "error sending request". HTTP/1.1 +
            // pool_max_idle_per_host(0) guarantees a fresh TCP+TLS connection per request,
            // which is safe because each 1 MB chunk holds ~60 s of audio.
            let proxy_client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .pool_max_idle_per_host(0)
                .http1_only()
                .build()
                .map_err(|e| format!("Failed to build proxy client: {e}"))?;

            let proxy_state = proxy::ProxyServerState::new(app.handle().clone(), proxy_client);

            tauri::async_runtime::spawn(async move {
                if let Err(e) = std_listener.set_nonblocking(true) {
                    log::error!("Failed to set proxy listener non-blocking: {e}");
                    return;
                }
                let listener = match tokio::net::TcpListener::from_std(std_listener) {
                    Ok(l) => l,
                    Err(e) => {
                        log::error!("Failed to convert proxy listener to tokio: {e}");
                        return;
                    }
                };

                let router = axum::Router::new()
                    .route(
                        "/:release_id/:track_position",
                        axum::routing::get(proxy::proxy_http_handler)
                            .options(proxy::proxy_cors_preflight_handler),
                    )
                    .with_state(proxy_state);

                if let Err(e) = axum::serve(listener, router).await {
                    log::error!("Stream proxy HTTP server error: {e}");
                }
            });

            Ok(())
        });

    // Fullscreen menu-text tracking via window resize is desktop-only (no native menu on
    // mobile, and no `WindowEvent::Resized` to react to).
    #[cfg(feature = "desktop")]
    let builder = builder.on_window_event(|window, event| {
        // Track fullscreen state changes and update the menu text accordingly.
        // There is no dedicated fullscreen event, so we check on every resize.
        if let tauri::WindowEvent::Resized(_) = event {
            use std::sync::atomic::{AtomicBool, Ordering};
            static WAS_FULLSCREEN: AtomicBool = AtomicBool::new(false);

            let is_fullscreen = window.is_fullscreen().unwrap_or(false);
            let was_fullscreen = WAS_FULLSCREEN.swap(is_fullscreen, Ordering::Relaxed);
            if is_fullscreen != was_fullscreen {
                menu::update_fullscreen_menu_text(window.app_handle(), is_fullscreen);
            }
        }
    });

    let app = builder
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            log::error!("Fatal: failed to build Tauri application: {e}");
            std::process::exit(1);
        });

    app.run(move |app_handle, event| match event {
        // "Open With" / Dock drops: Tauri only has this event on macOS, iOS and Android.
        #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
        tauri::RunEvent::Opened { urls } => {
            use tauri::Emitter;
            for url in urls {
                let file_path = if let Ok(path) = url.to_file_path() {
                    path.to_string_lossy().to_string()
                } else {
                    url.path().to_string()
                };

                let emit_now = app_handle
                    .try_state::<StartupFile>()
                    .is_none_or(|startup| startup.offer(file_path.clone()));
                if emit_now {
                    let _ = app_handle.emit("open-file", file_path);
                }
            }
        }
        _ => {
            let _ = app_handle;
        }
    });
}

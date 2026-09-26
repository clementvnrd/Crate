use std::path::PathBuf;
use tauri::State;

use crate::error::Result;
use crate::models::StandaloneTrack;
use crate::services::audio::PlaybackState;
use crate::services::{AudioService, PlayerTrackerService, StandaloneService};
use crate::StartupFile;

#[tauri::command]
pub async fn read_standalone_track(
    path: String,
    standalone: State<'_, StandaloneService>,
) -> Result<StandaloneTrack> {
    standalone.read_standalone_track(&path)
}

#[tauri::command]
pub async fn get_recent_standalone_tracks(
    limit: Option<usize>,
    standalone: State<'_, StandaloneService>,
) -> Result<Vec<StandaloneTrack>> {
    standalone.get_recent_standalone_tracks(limit)
}

#[tauri::command]
pub async fn add_recent_standalone_track(
    track: StandaloneTrack,
    standalone: State<'_, StandaloneService>,
) -> Result<()> {
    standalone.add_recent_standalone_track(&track)
}

#[tauri::command]
pub async fn remove_recent_standalone_track(
    id: String,
    standalone: State<'_, StandaloneService>,
) -> Result<()> {
    standalone.remove_recent_standalone_track(&id)
}

#[tauri::command]
pub async fn clear_recent_standalone_tracks(
    standalone: State<'_, StandaloneService>,
) -> Result<()> {
    standalone.clear_recent_standalone_tracks()
}

#[tauri::command]
pub async fn get_startup_file(
    startup_file: State<'_, StartupFile>,
) -> Result<Option<String>> {
    let mut file = startup_file
        .0
        .lock()
        .map_err(|_| crate::error::CrateError::LockPoisoned)?;
    Ok(file.take())
}

#[tauri::command]
pub async fn play_standalone_track(
    path: String,
    id: Option<String>,
    duration_ms: Option<u64>,
    standalone: State<'_, StandaloneService>,
    audio: State<'_, AudioService>,
    tracker: State<'_, PlayerTrackerService>,
) -> Result<PlaybackState> {
    let track_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut resolved_duration = duration_ms;
    if let Ok(track) = standalone.read_standalone_track(&path) {
        if resolved_duration.is_none() || resolved_duration == Some(0) {
            resolved_duration = Some(track.duration_ms.max(0) as u64);
        }
        let ctx = crate::services::player::TrackPlayingContext {
            track_id: Some(track_id.clone()),
            source: if track.is_in_library {
                "crate_local".to_string()
            } else {
                "crate_standalone".to_string()
            },
            title: track.title.clone().unwrap_or_else(|| "Unknown Track".to_string()),
            artist: track.artist.clone().unwrap_or_else(|| "Unknown Artist".to_string()),
            album: track.album.clone(),
            duration_ms: track.duration_ms.max(0) as u64,
            bpm: track.bpm,
            key: track.key.clone(),
            energy: track.energy,
            format: Some(track.format.clone()),
            artwork_url: track.artwork_path.clone(),
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        tracker.on_track_started(ctx);
    }
    audio.play_track(track_id, PathBuf::from(&path), resolved_duration)
}


#[cfg(target_os = "macos")]
mod macos_default {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;

    #[link(name = "CoreServices", kind = "framework")]
    extern "C" {
        fn LSSetDefaultRoleHandlerForContentType(
            inContentType: core_foundation::string::CFStringRef,
            inRole: u32,
            inHandlerBundleID: core_foundation::string::CFStringRef,
        ) -> i32;
    }

    const AUDIO_UTIS: &[&str] = &[
        "public.mp3",
        "com.apple.m4a-audio",
        "public.wave-format",
        "com.microsoft.waveform-audio",
        "public.flac-audio",
        "org.xiph.flac",
        "public.aiff-audio",
        "public.aac-audio",
        "org.xiph.ogg-audio",
        "public.audio",
    ];

    const BUNDLE_IDS: &[&str] = &[
        "com.crate.app",
        "com.bbx-audio.crate",
    ];

    const LSR_ROLES_ALL: u32 = 0xFFFFFFFF;
    const LSR_ROLES_VIEWER: u32 = 0x00000002;

    pub fn set_default_player() {
        for &bundle_id in BUNDLE_IDS {
            let bundle_cf = CFString::new(bundle_id);
            for &uti in AUDIO_UTIS {
                let uti_cf = CFString::new(uti);
                unsafe {
                    let _ = LSSetDefaultRoleHandlerForContentType(
                        uti_cf.as_concrete_TypeRef(),
                        LSR_ROLES_ALL,
                        bundle_cf.as_concrete_TypeRef(),
                    );
                    let _ = LSSetDefaultRoleHandlerForContentType(
                        uti_cf.as_concrete_TypeRef(),
                        LSR_ROLES_VIEWER,
                        bundle_cf.as_concrete_TypeRef(),
                    );
                }
            }
        }

        let lsregister_path = "/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister";
        if std::path::Path::new(lsregister_path).exists() {
            let _ = std::process::Command::new(lsregister_path)
                .args(["-f", "/Applications/Crate.app"])
                .status();
        }
    }
}

#[tauri::command]
pub async fn set_as_default_audio_player() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        macos_default::set_default_player();
    }
    Ok(())
}


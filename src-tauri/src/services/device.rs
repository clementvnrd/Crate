use std::collections::{HashMap, HashSet};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sysinfo::Disks;
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;

use crate::error::{CrateError, Result};
use crate::models::UsbDevice;

/// Get volume UUID for a mount point (platform-specific)
fn get_volume_uuid(mount_point: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        get_volume_uuid_macos(mount_point)
    }

    #[cfg(target_os = "linux")]
    {
        get_volume_uuid_linux(mount_point)
    }

    #[cfg(target_os = "windows")]
    {
        // Windows UUID retrieval not yet implemented
        let _ = mount_point;
        None
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn get_volume_uuid_macos(mount_point: &str) -> Option<String> {
    // Use diskutil info -plist to get volume UUID
    let output = Command::new("diskutil")
        .args(["info", "-plist", mount_point])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let plist_str = String::from_utf8_lossy(&output.stdout);

    // Parse the plist XML to find VolumeUUID
    // Look for <key>VolumeUUID</key> followed by <string>...</string>
    let uuid_key = "<key>VolumeUUID</key>";
    if let Some(key_pos) = plist_str.find(uuid_key) {
        let after_key = &plist_str[key_pos + uuid_key.len()..];
        if let Some(string_start) = after_key.find("<string>") {
            let value_start = string_start + "<string>".len();
            if let Some(string_end) = after_key[value_start..].find("</string>") {
                let uuid = after_key[value_start..value_start + string_end].trim();
                if !uuid.is_empty() {
                    return Some(uuid.to_string());
                }
            }
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn get_volume_uuid_linux(mount_point: &str) -> Option<String> {
    use std::fs;
    use std::path::Path;

    // Read /proc/mounts to find the device for this mount point
    let mounts = fs::read_to_string("/proc/mounts").ok()?;

    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == mount_point {
            let device = parts[0];

            // Look up UUID in /dev/disk/by-uuid/
            let by_uuid_dir = Path::new("/dev/disk/by-uuid");
            if by_uuid_dir.exists() {
                for entry in fs::read_dir(by_uuid_dir).ok()? {
                    let entry = entry.ok()?;
                    let target = fs::read_link(entry.path()).ok()?;
                    let target_str = target.to_string_lossy();

                    // Check if this UUID symlink points to our device
                    if device.ends_with(&*target_str)
                        || target_str.ends_with(device.split('/').next_back().unwrap_or(""))
                    {
                        return entry.file_name().to_str().map(|s| s.to_string());
                    }
                }
            }
            break;
        }
    }

    None
}

/// Validate and sanitize a volume name for FAT32
/// - Max 11 characters
/// - Uppercase only
/// - Valid characters: A-Z, 0-9, space, underscore
fn validate_fat32_volume_name(name: &str) -> Result<String> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Err(CrateError::Device(
            "Volume name cannot be empty".to_string(),
        ));
    }

    // Convert to uppercase and filter invalid characters
    let sanitized: String = trimmed
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '_')
        .take(11)
        .collect();

    if sanitized.is_empty() {
        return Err(CrateError::Device(
            "Volume name contains no valid characters".to_string(),
        ));
    }

    Ok(sanitized)
}

/// Get disk identifier from mount point (macOS)
#[cfg(target_os = "macos")]
fn get_disk_identifier_macos(mount_point: &str) -> Result<String> {
    let output = Command::new("diskutil")
        .args(["info", "-plist", mount_point])
        .output()
        .map_err(|e| CrateError::Device(format!("Failed to get disk info: {e}")))?;

    if !output.status.success() {
        return Err(CrateError::Device(
            "Failed to get disk information".to_string(),
        ));
    }

    let plist_str = String::from_utf8_lossy(&output.stdout);

    // Parse ParentWholeDisk from plist (e.g., "disk2")
    let key = "<key>ParentWholeDisk</key>";
    if let Some(key_pos) = plist_str.find(key) {
        let after_key = &plist_str[key_pos + key.len()..];
        if let Some(start) = after_key.find("<string>") {
            let value_start = start + "<string>".len();
            if let Some(end) = after_key[value_start..].find("</string>") {
                let disk_name = after_key[value_start..value_start + end].trim();
                if !disk_name.is_empty() {
                    return Ok(format!("/dev/{disk_name}"));
                }
            }
        }
    }

    Err(CrateError::Device(
        "Could not determine disk identifier".to_string(),
    ))
}

/// Get device path from mount point (Linux)
#[cfg(target_os = "linux")]
fn get_device_path_linux(mount_point: &str) -> Result<String> {
    let output = Command::new("findmnt")
        .args(["-n", "-o", "SOURCE", mount_point])
        .output()
        .map_err(|e| CrateError::Device(format!("Failed to find device: {e}")))?;

    if !output.status.success() {
        return Err(CrateError::Device(
            "Could not determine device path".to_string(),
        ));
    }

    let device = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if device.is_empty() {
        return Err(CrateError::Device(
            "Could not determine device path".to_string(),
        ));
    }

    Ok(device)
}

/// Extract a human-readable name from a mount point path.
/// On Unix: `/Volumes/MY_USB` → `"MY_USB"`
/// On Windows: `D:\` → `"D:"`
fn device_name_from_mount_point(mount_point: &str) -> String {
    std::path::Path::new(mount_point)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        // Root paths (e.g. `D:\` or `/`) have no file_name — use the mount point itself,
        // stripping any trailing separator for cleaner display.
        .unwrap_or_else(|| mount_point.trim_end_matches(['/', '\\']).to_string())
}

/// The UUID of the volume mounted at `mount_point`, asking `lookup` only when it is not cached.
///
/// A mounted volume keeps its UUID, so a hit is final; a failed lookup is not cached (the next
/// poll asks again, as before). Looking it up runs `diskutil info`, 60 to 250 ms per volume.
fn cached_volume_uuid(
    cache: &mut HashMap<String, String>,
    mount_point: &str,
    lookup: impl FnOnce(&str) -> Option<String>,
) -> Option<String> {
    if let Some(uuid) = cache.get(mount_point) {
        return Some(uuid.clone());
    }
    let uuid = lookup(mount_point)?;
    cache.insert(mount_point.to_string(), uuid.clone());
    Some(uuid)
}

/// Lists the mounted removable volumes. Blocking (disk enumeration and `diskutil`): call it from
/// a blocking context. `uuid_cache` lives as long as the caller polls; entries of volumes that
/// are gone are dropped, so a stick plugged in again is looked up again.
fn list_removable_devices(uuid_cache: &mut HashMap<String, String>) -> Vec<UsbDevice> {
    let disks = Disks::new_with_refreshed_list();

    let devices: Vec<UsbDevice> = disks
        .iter()
        .filter(|disk| disk.is_removable())
        .filter(|disk| {
            // Skip system volumes on macOS
            let mount = disk.mount_point().to_string_lossy();
            !mount.starts_with("/System") && mount != "/"
        })
        .map(|disk| {
            let mount_point = disk.mount_point().to_string_lossy().to_string();
            let name = disk.name().to_string_lossy().to_string();

            // Get volume UUID for stable identification
            let volume_uuid = cached_volume_uuid(uuid_cache, &mount_point, get_volume_uuid);

            // Use volume UUID as ID if available, otherwise fall back to mount point
            let id = volume_uuid.clone().unwrap_or_else(|| mount_point.clone());

            UsbDevice {
                id,
                name: if name.is_empty() {
                    device_name_from_mount_point(&mount_point)
                } else {
                    name
                },
                mount_point,
                volume_uuid,
                total_space_bytes: disk.total_space(),
                available_space_bytes: disk.available_space(),
                is_removable: true,
                file_system: disk.file_system().to_string_lossy().to_string(),
                disk_kind: match disk.kind() {
                    sysinfo::DiskKind::SSD => "SSD".to_string(),
                    sysinfo::DiskKind::HDD => "HDD".to_string(),
                    sysinfo::DiskKind::Unknown(_) => "Unknown".to_string(),
                },
            }
        })
        .collect();

    uuid_cache.retain(|mount, _| devices.iter().any(|d| &d.mount_point == mount));
    devices
}

pub struct DeviceService {
    devices: Arc<Mutex<Vec<UsbDevice>>>,
    stop_tx: Arc<Mutex<Option<watch::Sender<bool>>>>,
}

impl DeviceService {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(Mutex::new(Vec::new())),
            stop_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_removable_devices(&self) -> Vec<UsbDevice> {
        list_removable_devices(&mut HashMap::new())
    }

    pub fn start_monitoring(&self, app_handle: AppHandle) {
        let devices = self.devices.clone();
        let (stop_tx, mut stop_rx) = watch::channel(false);

        // Store the stop sender
        if let Ok(mut guard) = self.stop_tx.lock() {
            *guard = Some(stop_tx);
        }

        // Get initial device list
        let initial_devices = self.get_removable_devices();
        if let Ok(mut guard) = devices.lock() {
            *guard = initial_devices.clone();
        }

        // Emit initial device list
        let _ = app_handle.emit("devices-changed", &initial_devices);

        // Spawn polling task
        let devices_clone = devices.clone();
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            let mut uuid_cache: HashMap<String, String> = HashMap::new();

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // `diskutil` runs per volume: keep it off the async workers.
                        let mut cache = std::mem::take(&mut uuid_cache);
                        let polled = tokio::task::spawn_blocking(move || {
                            let devices = list_removable_devices(&mut cache);
                            (devices, cache)
                        })
                        .await;
                        let current_devices = match polled {
                            Ok((devices, cache)) => {
                                uuid_cache = cache;
                                devices
                            }
                            Err(e) => {
                                log::warn!("Device poll failed, will retry: {e}");
                                continue;
                            }
                        };

                        // Compare with previous state
                        let previous_ids: HashSet<String> = {
                            if let Ok(guard) = devices_clone.lock() {
                                guard.iter().map(|d| d.id.clone()).collect()
                            } else {
                                HashSet::new()
                            }
                        };

                        let current_ids: HashSet<String> =
                            current_devices.iter().map(|d| d.id.clone()).collect();

                        // Only emit if changed
                        if current_ids != previous_ids {
                            log::info!("Device list changed: {:?}", current_devices.iter().map(|d| &d.name).collect::<Vec<_>>());

                            // Update stored devices
                            if let Ok(mut guard) = devices_clone.lock() {
                                *guard = current_devices.clone();
                            }

                            // Emit event to frontend
                            let _ = app_handle.emit("devices-changed", &current_devices);
                        }
                    }
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            log::info!("Stopping device monitoring");
                            break;
                        }
                    }
                }
            }
        });

        log::info!("Device monitoring started");
    }

    #[allow(dead_code)]
    pub fn stop_monitoring(&self) {
        if let Ok(guard) = self.stop_tx.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(true);
            }
        }
    }

    /// Eject a device by its mount point
    pub fn eject_device(&self, mount_point: &str) -> Result<()> {
        log::info!("Ejecting device at: {mount_point}");

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("diskutil")
                .args(["eject", mount_point])
                .output()
                .map_err(|e| CrateError::Device(format!("Failed to run diskutil: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(CrateError::Device(format!("Failed to eject: {stderr}")));
            }

            log::info!("Successfully ejected device at: {mount_point}");
            Ok(())
        }

        #[cfg(target_os = "linux")]
        {
            let output = Command::new("umount")
                .arg(mount_point)
                .output()
                .map_err(|e| CrateError::Device(format!("Failed to run umount: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(CrateError::Device(format!("Failed to eject: {stderr}")));
            }

            log::info!("Successfully ejected device at: {mount_point}");
            Ok(())
        }

        #[cfg(target_os = "windows")]
        {
            // Windows requires a more complex approach using DeviceIoControl
            // For now, return an error indicating this is not yet implemented
            Err(CrateError::Device(
                "Eject is not yet supported on Windows".to_string(),
            ))
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Err(CrateError::Device(
                "Eject is not supported on this platform".to_string(),
            ))
        }
    }

    /// Reformat a device to FAT32 with the given volume name
    pub fn reformat_device(&self, mount_point: &str, volume_name: &str) -> Result<()> {
        let validated_name = validate_fat32_volume_name(volume_name)?;

        log::info!("Reformatting device at: {mount_point} with name: {validated_name}");

        #[cfg(target_os = "macos")]
        {
            self.reformat_device_macos(mount_point, &validated_name)
        }

        #[cfg(target_os = "linux")]
        {
            self.reformat_device_linux(mount_point, &validated_name)
        }

        #[cfg(target_os = "windows")]
        {
            Err(CrateError::Device(
                "Reformat is not yet supported on Windows".to_string(),
            ))
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Err(CrateError::Device(
                "Reformat is not supported on this platform".to_string(),
            ))
        }
    }

    #[cfg(target_os = "macos")]
    fn reformat_device_macos(&self, mount_point: &str, volume_name: &str) -> Result<()> {
        use std::process::Command;

        // Get disk identifier from mount point
        let disk_id = get_disk_identifier_macos(mount_point)?;

        log::info!("Requesting authorization to reformat {disk_id}");

        // Use osascript with administrator privileges - this shows the modern Touch ID dialog
        // because AppleScript is Apple-signed. MBR partition scheme is used for maximum
        // compatibility with DJ equipment.
        let script = format!(
            r#"do shell script "/usr/sbin/diskutil eraseDisk FAT32 '{volume_name}' MBR {disk_id}" with administrator privileges"#
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| CrateError::Device(format!("Failed to execute osascript: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // User cancellation returns error -128
            if stderr.contains("-128") || stderr.contains("User canceled") {
                return Err(CrateError::Device(
                    "Operation cancelled by user".to_string(),
                ));
            }

            return Err(CrateError::Device(format!(
                "Failed to reformat device: {}",
                stderr.trim()
            )));
        }

        log::info!("Successfully reformatted device at: {mount_point}");
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn reformat_device_linux(&self, mount_point: &str, volume_name: &str) -> Result<()> {
        // Get device path from mount point
        let device_path = get_device_path_linux(mount_point)?;

        // Unmount the device first
        let unmount_output = Command::new("umount")
            .arg(mount_point)
            .output()
            .map_err(|e| CrateError::Device(format!("Failed to unmount: {e}")))?;

        if !unmount_output.status.success() {
            let stderr = String::from_utf8_lossy(&unmount_output.stderr);
            if stderr.contains("busy") {
                return Err(CrateError::Device(
                    "Device is busy. Close any applications using it and try again.".to_string(),
                ));
            }
            return Err(CrateError::Device(format!("Failed to unmount: {stderr}")));
        }

        // Format with mkfs.vfat using pkexec for privilege elevation
        let output = Command::new("pkexec")
            .args(["mkfs.vfat", "-F", "32", "-n", volume_name, &device_path])
            .output()
            .map_err(|e| CrateError::Device(format!("Failed to run mkfs.vfat: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Check for user cancellation
            if stderr.contains("dismissed") || stderr.contains("Not authorized") {
                return Err(CrateError::Device(
                    "Operation cancelled by user".to_string(),
                ));
            }
            return Err(CrateError::Device(format!("Failed to reformat: {stderr}")));
        }

        log::info!("Successfully reformatted device at: {mount_point}");
        Ok(())
    }
}

#[cfg(test)]
mod uuid_cache_tests {
    use super::*;

    #[test]
    fn a_found_uuid_is_looked_up_once_per_mount() {
        let mut cache = HashMap::new();
        let mut lookups = 0;
        for _ in 0..3 {
            let uuid = cached_volume_uuid(&mut cache, "/Volumes/USB", |_| {
                lookups += 1;
                Some("AAAA-1111".to_string())
            });
            assert_eq!(uuid.as_deref(), Some("AAAA-1111"));
        }
        assert_eq!(lookups, 1, "diskutil is only run the first time");
    }

    #[test]
    fn a_failed_lookup_is_retried_on_the_next_poll() {
        let mut cache = HashMap::new();
        let mut lookups = 0;
        for _ in 0..3 {
            let uuid = cached_volume_uuid(&mut cache, "/Volumes/USB", |_| {
                lookups += 1;
                None
            });
            assert_eq!(uuid, None);
        }
        assert_eq!(lookups, 3, "a transient failure must not stick");
    }

    #[test]
    fn two_mounts_do_not_share_an_entry() {
        let mut cache = HashMap::new();
        cached_volume_uuid(&mut cache, "/Volumes/A", |_| Some("A".to_string()));
        let b = cached_volume_uuid(&mut cache, "/Volumes/B", |_| Some("B".to_string()));
        assert_eq!(b.as_deref(), Some("B"));
    }
}

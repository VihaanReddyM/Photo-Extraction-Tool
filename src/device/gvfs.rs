//! Linux / Unix device backend (GVFS)
//!
//! On Linux, phones are exposed by the desktop's GVFS layer as ordinary
//! directories under `$XDG_RUNTIME_DIR/gvfs/` (or `~/.gvfs`):
//!
//! - `mtp:host=...`     Android phones and other MTP devices
//! - `gphoto2:host=...` iPhones/iPads and cameras (PTP)
//! - `afc:host=<UDID>`  iPhones/iPads (needs `libimobiledevice` / `ifuse` stack)
//!
//! This backend lists those mount points and reads files through them, so it
//! needs no C libraries at build time. Devices that are plugged in but not yet
//! mounted are mounted on a best-effort basis with `gio mount`.
//!
//! The mount directory can be overridden with `PHOTO_EXTRACTION_GVFS_DIR`
//! (useful for testing).

use crate::core::error::{ExtractionError, Result};
use crate::device::traits::{
    DeviceContentTrait, DeviceInfo, DeviceManagerTrait, DeviceObject, DeviceType,
};
use chrono::{DateTime, Utc};
use log::{debug, trace, warn};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Environment variable to override the GVFS mount directory
pub const GVFS_DIR_ENV: &str = "PHOTO_EXTRACTION_GVFS_DIR";

/// No-op stand-in for the Windows COM guard
pub struct ComGuard;

/// Nothing to initialise on Linux; kept so callers are platform-independent
pub fn initialize_com() -> Result<ComGuard> {
    Ok(ComGuard)
}

/// Candidate directories where GVFS mounts live
fn gvfs_roots() -> Vec<PathBuf> {
    if let Ok(custom) = std::env::var(GVFS_DIR_ENV) {
        return vec![PathBuf::from(custom)];
    }
    let mut roots = Vec::new();
    if let Ok(rt) = std::env::var("XDG_RUNTIME_DIR") {
        roots.push(Path::new(&rt).join("gvfs"));
    }
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".gvfs"));
    }
    roots
}

/// Decode `%XX` escapes used in GVFS mount names
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Build a `DeviceInfo` from a GVFS mount directory name such as
/// `mtp:host=SAMSUNG_SAMSUNG_Android_R58M123` or `afc:host=00008030-...`.
fn parse_mount_name(name: &str) -> Option<DeviceInfo> {
    let (scheme, rest) = name.split_once(":host=")?;
    let host = percent_decode(rest);

    match scheme {
        "afc" => Some(DeviceInfo::new(
            name,
            &format!("iOS Device ({})", host),
            "Apple Inc.",
            "iPhone/iPad",
        )),
        "mtp" | "gphoto2" => {
            // Format is usually Vendor_Model_..._Serial, underscores for spaces
            let parts: Vec<&str> = host.split('_').filter(|p| !p.is_empty()).collect();
            let manufacturer = parts.first().copied().unwrap_or("Unknown").to_string();
            // Drop a trailing serial-looking token (long, has digits)
            let mut words = parts.clone();
            if words.len() > 2 {
                if let Some(last) = words.last() {
                    if last.len() >= 8 && last.chars().any(|c| c.is_ascii_digit()) {
                        words.pop();
                    }
                }
            }
            let friendly = words.join(" ");
            let model = if words.len() > 1 {
                words[1..].join(" ")
            } else {
                friendly.clone()
            };
            let manufacturer = if friendly.to_lowercase().contains("apple")
                || friendly.to_lowercase().contains("iphone")
                || friendly.to_lowercase().contains("ipad")
            {
                "Apple Inc.".to_string()
            } else {
                manufacturer
            };
            Some(DeviceInfo::new(name, &friendly, &manufacturer, &model))
        }
        _ => None,
    }
}

/// Try to mount plugged-in but unmounted devices using `gio` (best effort)
fn try_mount_devices() {
    let Ok(out) = Command::new("gio").args(["mount", "-li"]).output() else {
        return;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let line = line.trim();
        if let Some(uri) = line.strip_prefix("activation_root=") {
            if uri.starts_with("mtp://")
                || uri.starts_with("gphoto2://")
                || uri.starts_with("afc://")
            {
                debug!("Mounting {} via gio", uri);
                let _ = Command::new("gio").args(["mount", uri]).output();
            }
        }
    }
}

fn list_mounts() -> Vec<(DeviceInfo, PathBuf)> {
    let mut found = Vec::new();
    for root in gvfs_roots() {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(info) = parse_mount_name(&name) {
                found.push((info, entry.path()));
            }
        }
    }
    found
}

/// Device manager backed by GVFS mounts
pub struct DeviceManager;

impl DeviceManager {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    fn mounts(&self) -> Vec<(DeviceInfo, PathBuf)> {
        let mut mounts = list_mounts();
        if mounts.is_empty() && std::env::var(GVFS_DIR_ENV).is_err() {
            try_mount_devices();
            mounts = list_mounts();
        }
        mounts
    }
}

impl DeviceManagerTrait for DeviceManager {
    type Content = DeviceContent;

    fn enumerate_apple_devices(&self) -> Result<Vec<DeviceInfo>> {
        self.enumerate_devices_by_type(DeviceType::Apple)
    }

    fn enumerate_android_devices(&self) -> Result<Vec<DeviceInfo>> {
        self.enumerate_devices_by_type(DeviceType::Android)
    }

    fn enumerate_all_devices(&self) -> Result<Vec<DeviceInfo>> {
        Ok(self.mounts().into_iter().map(|(info, _)| info).collect())
    }

    fn open_device(&self, device_id: &str) -> Result<DeviceContent> {
        let path = self
            .mounts()
            .into_iter()
            .find(|(info, _)| info.device_id == device_id)
            .map(|(_, p)| p)
            .ok_or_else(|| {
                ExtractionError::DeviceError(format!(
                    "Device '{}' is not mounted. Unlock the phone, choose 'File transfer' (MTP) \
                     and make sure gvfs-mtp / gvfs-gphoto2 is installed.",
                    device_id
                ))
            })?;
        // Verify we can actually read the mount (phone locked => permission error)
        fs::read_dir(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                ExtractionError::AccessDenied
            } else {
                ExtractionError::DeviceError(format!("Cannot read '{}': {}", path.display(), e))
            }
        })?;
        Ok(DeviceContent { root: path })
    }

    fn get_device_info(&self, device_id: &str) -> Option<DeviceInfo> {
        self.mounts()
            .into_iter()
            .map(|(i, _)| i)
            .find(|i| i.device_id == device_id)
    }

    fn device_count(&self) -> usize {
        self.mounts().len()
    }
}

/// Open device: a mounted GVFS directory. Object IDs are absolute paths.
pub struct DeviceContent {
    root: PathBuf,
}

impl DeviceContent {
    /// Create content rooted at an arbitrary directory (used in tests)
    pub fn from_root(root: PathBuf) -> Self {
        Self { root }
    }

    /// Resolve an object id to a path, refusing anything outside the mount
    fn resolve(&self, object_id: &str) -> Result<PathBuf> {
        if object_id == "DEVICE" {
            return Ok(self.root.clone());
        }
        let path = PathBuf::from(object_id);
        let escapes = path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir));
        if escapes || !path.starts_with(&self.root) {
            return Err(ExtractionError::ContentError(format!(
                "Object '{}' is outside the device",
                object_id
            )));
        }
        Ok(path)
    }

    fn make_object(&self, path: &Path, parent_id: &str) -> Option<DeviceObject> {
        let name = path.file_name()?.to_string_lossy().into_owned();
        let meta = fs::metadata(path).ok()?;
        let id = path.to_string_lossy();
        if meta.is_dir() {
            return Some(DeviceObject::folder(&id, parent_id, &name));
        }
        let mut obj = DeviceObject::file(&id, parent_id, &name, meta.len());
        obj.date_modified = meta
            .modified()
            .ok()
            .map(|t| DateTime::<Utc>::from(t).to_rfc3339());
        Some(obj)
    }
}

impl DeviceContentTrait for DeviceContent {
    fn enumerate_objects(&self) -> Result<Vec<DeviceObject>> {
        self.enumerate_children("DEVICE")
    }

    fn enumerate_children(&self, parent_id: &str) -> Result<Vec<DeviceObject>> {
        let dir = self.resolve(parent_id)?;
        trace!("Listing {}", dir.display());
        let entries = fs::read_dir(&dir).map_err(|e| {
            warn!("Failed to list '{}': {}", dir.display(), e);
            ExtractionError::ContentError(format!("Failed to list '{}': {}", dir.display(), e))
        })?;
        let mut objects: Vec<DeviceObject> = entries
            .flatten()
            .filter_map(|e| self.make_object(&e.path(), parent_id))
            .collect();
        objects.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(objects)
    }

    fn read_file(&self, object_id: &str) -> Result<Vec<u8>> {
        let path = self.resolve(object_id)?;
        fs::read(&path).map_err(|e| ExtractionError::TransferError {
            filename: path.display().to_string(),
            message: e.to_string(),
        })
    }

    fn get_object(&self, object_id: &str) -> Result<Option<DeviceObject>> {
        let path = self.resolve(object_id)?;
        let parent = path
            .parent()
            .map(|p| {
                if p == self.root {
                    "DEVICE".to_string()
                } else {
                    p.to_string_lossy().into_owned()
                }
            })
            .unwrap_or_else(|| "DEVICE".to_string());
        Ok(self.make_object(&path, &parent))
    }

    fn get_object_path(&self, object_id: &str) -> Option<String> {
        let path = self.resolve(object_id).ok()?;
        path.strip_prefix(&self.root)
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    }
}

/// Enumerate Apple devices (convenience)
pub fn enumerate_devices() -> Result<Vec<DeviceInfo>> {
    DeviceManager::new()?.enumerate_apple_devices()
}

/// Enumerate Android devices (convenience)
pub fn enumerate_android_devices() -> Result<Vec<DeviceInfo>> {
    DeviceManager::new()?.enumerate_android_devices()
}

/// Enumerate all MTP/PTP devices (convenience)
pub fn enumerate_all_mtp_devices() -> Result<Vec<DeviceInfo>> {
    DeviceManager::new()?.enumerate_all_devices()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_android_mtp_mount() {
        let i = parse_mount_name("mtp:host=SAMSUNG_SAMSUNG_Android_R58M12345AB").unwrap();
        assert_eq!(i.manufacturer, "SAMSUNG");
        assert_eq!(i.device_type(), DeviceType::Android);
    }

    #[test]
    fn parses_iphone_mounts() {
        let i = parse_mount_name("gphoto2:host=Apple_Inc._iPhone_00008030001A").unwrap();
        assert_eq!(i.device_type(), DeviceType::Apple);
        let a = parse_mount_name("afc:host=00008030-001A").unwrap();
        assert_eq!(a.device_type(), DeviceType::Apple);
    }

    #[test]
    fn ignores_other_mounts() {
        assert!(parse_mount_name("smb-share:server=x").is_none());
    }

    #[test]
    fn percent_decoding() {
        assert_eq!(percent_decode("a%20b"), "a b");
        assert_eq!(percent_decode("100%"), "100%");
    }

    #[test]
    fn lists_and_reads_files_and_blocks_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let dcim = tmp.path().join("Internal storage").join("DCIM");
        fs::create_dir_all(&dcim).unwrap();
        fs::write(dcim.join("IMG_1.jpg"), b"abc").unwrap();

        let c = DeviceContent::from_root(tmp.path().to_path_buf());
        let storage = c.enumerate_objects().unwrap();
        assert_eq!(storage[0].name, "Internal storage");
        let dcim_obj = c.enumerate_children(&storage[0].object_id).unwrap();
        let files = c.enumerate_children(&dcim_obj[0].object_id).unwrap();
        assert_eq!(files[0].size, 3);
        assert_eq!(c.read_file(&files[0].object_id).unwrap(), b"abc");
        assert_eq!(
            c.get_object_path(&files[0].object_id).unwrap(),
            "Internal storage/DCIM/IMG_1.jpg"
        );
        assert!(c.read_file("/etc/passwd").is_err());
        let evil = format!("{}/../x", tmp.path().display());
        assert!(c.read_file(&evil).is_err());
    }
}

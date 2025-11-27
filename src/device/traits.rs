//! Device abstraction traits for testability
//!
//! This module defines traits that abstract device operations, allowing both
//! real iOS devices (via WPD) and mock devices to be used interchangeably. This enables
//! comprehensive testing of the extraction pipeline without connecting a real iOS device.
//!
//! # Architecture
//!
//! The trait hierarchy is:
//! - `DeviceManagerTrait` - Enumerates and opens devices
//! - `DeviceContentTrait` - Provides access to device file system
//! - `DeviceInfo` - Common device information structure (shared, not a trait)
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use photo_extraction_tool::device::traits::{DeviceManagerTrait, DeviceContentTrait};
//!
//! fn extract_from_device<M: DeviceManagerTrait>(manager: &M) -> Result<(), String> {
//!     let devices = manager.enumerate_apple_devices().map_err(|e| e.to_string())?;
//!     if let Some(device) = devices.first() {
//!         let content = manager.open_device(&device.device_id).map_err(|e| e.to_string())?;
//!         let root_objects = content.enumerate_objects().map_err(|e| e.to_string())?;
//!         // ... process objects
//!     }
//!     Ok(())
//! }
//! ```

use crate::core::error::Result;
use std::fmt::Debug;

/// Common device information shared between real and mock devices
#[derive(Debug, Clone, Default)]
pub struct DeviceInfo {
    /// Unique device identifier (WPD device ID or mock ID)
    pub device_id: String,
    /// User-friendly device name (e.g., "John's iPhone")
    pub friendly_name: String,
    /// Device manufacturer (e.g., "Apple Inc.")
    pub manufacturer: String,
    /// Device model (e.g., "iPhone 15 Pro")
    pub model: String,
}

impl DeviceInfo {
    /// Create a new DeviceInfo
    pub fn new(device_id: &str, friendly_name: &str, manufacturer: &str, model: &str) -> Self {
        Self {
            device_id: device_id.to_string(),
            friendly_name: friendly_name.to_string(),
            manufacturer: manufacturer.to_string(),
            model: model.to_string(),
        }
    }
}

/// Represents a file or folder on a device
#[derive(Debug, Clone)]
pub struct DeviceObject {
    /// Unique object identifier on the device
    pub object_id: String,
    /// Parent object ID ("DEVICE" for root objects)
    pub parent_id: String,
    /// Name of the object (file or folder name)
    pub name: String,
    /// Whether this is a folder
    pub is_folder: bool,
    /// Size in bytes (0 for folders)
    pub size: u64,
    /// Date modified (ISO 8601 string, if available)
    pub date_modified: Option<String>,
    /// Content type hint (e.g., "image/jpeg")
    pub content_type: Option<String>,
}

impl DeviceObject {
    /// Create a new folder object
    pub fn folder(object_id: &str, parent_id: &str, name: &str) -> Self {
        Self {
            object_id: object_id.to_string(),
            parent_id: parent_id.to_string(),
            name: name.to_string(),
            is_folder: true,
            size: 0,
            date_modified: None,
            content_type: None,
        }
    }

    /// Create a new file object
    pub fn file(object_id: &str, parent_id: &str, name: &str, size: u64) -> Self {
        Self {
            object_id: object_id.to_string(),
            parent_id: parent_id.to_string(),
            name: name.to_string(),
            is_folder: false,
            size,
            date_modified: None,
            content_type: Self::guess_content_type(name),
        }
    }

    /// Create a file with a specific date
    pub fn file_with_date(
        object_id: &str,
        parent_id: &str,
        name: &str,
        size: u64,
        date_modified: &str,
    ) -> Self {
        let mut obj = Self::file(object_id, parent_id, name, size);
        obj.date_modified = Some(date_modified.to_string());
        obj
    }

    /// Guess content type from file extension
    fn guess_content_type(name: &str) -> Option<String> {
        let lower = name.to_lowercase();
        if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
            Some("image/jpeg".to_string())
        } else if lower.ends_with(".png") {
            Some("image/png".to_string())
        } else if lower.ends_with(".heic") || lower.ends_with(".heif") {
            Some("image/heic".to_string())
        } else if lower.ends_with(".gif") {
            Some("image/gif".to_string())
        } else if lower.ends_with(".webp") {
            Some("image/webp".to_string())
        } else if lower.ends_with(".raw") || lower.ends_with(".dng") {
            Some("image/raw".to_string())
        } else if lower.ends_with(".tiff") || lower.ends_with(".tif") {
            Some("image/tiff".to_string())
        } else if lower.ends_with(".bmp") {
            Some("image/bmp".to_string())
        } else if lower.ends_with(".mov") {
            Some("video/quicktime".to_string())
        } else if lower.ends_with(".mp4") || lower.ends_with(".m4v") {
            Some("video/mp4".to_string())
        } else if lower.ends_with(".avi") {
            Some("video/avi".to_string())
        } else if lower.ends_with(".3gp") {
            Some("video/3gpp".to_string())
        } else {
            None
        }
    }

    /// Check if this object is a media file based on extension
    pub fn is_media_file(&self) -> bool {
        if self.is_folder {
            return false;
        }

        let photo_extensions = [
            "jpg", "jpeg", "png", "heic", "heif", "gif", "webp", "raw", "dng", "tiff", "tif", "bmp",
        ];
        let video_extensions = ["mov", "mp4", "m4v", "avi", "3gp"];

        let extension = std::path::Path::new(&self.name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        photo_extensions.contains(&extension.as_str())
            || video_extensions.contains(&extension.as_str())
    }
}

impl Default for DeviceObject {
    fn default() -> Self {
        Self {
            object_id: String::new(),
            parent_id: "DEVICE".to_string(),
            name: String::new(),
            is_folder: false,
            size: 0,
            date_modified: None,
            content_type: None,
        }
    }
}

/// Trait for device content access (file system operations)
///
/// This trait abstracts the operations needed to browse and read files from a device.
/// Both real WPD devices and mock devices implement this trait.
pub trait DeviceContentTrait: Send + Sync {
    /// Enumerate objects at the root level of the device
    ///
    /// Returns a list of root-level objects (typically "Internal Storage" on iOS devices)
    fn enumerate_objects(&self) -> Result<Vec<DeviceObject>>;

    /// Enumerate children of a specific object
    ///
    /// # Arguments
    /// * `parent_id` - The object ID of the parent folder. Use "DEVICE" for root.
    fn enumerate_children(&self, parent_id: &str) -> Result<Vec<DeviceObject>>;

    /// Read the content of a file
    ///
    /// # Arguments
    /// * `object_id` - The object ID of the file to read
    ///
    /// # Returns
    /// The raw bytes of the file content
    fn read_file(&self, object_id: &str) -> Result<Vec<u8>>;

    /// Get information about a specific object
    ///
    /// # Arguments
    /// * `object_id` - The object ID to look up
    fn get_object(&self, object_id: &str) -> Result<Option<DeviceObject>>;

    /// Build the full path from root to a specific object
    ///
    /// # Arguments
    /// * `object_id` - The object ID to get the path for
    ///
    /// # Returns
    /// The full path as a string (e.g., "Internal Storage/DCIM/100APPLE/IMG_0001.JPG")
    fn get_object_path(&self, object_id: &str) -> Option<String>;
}

/// Trait for device manager (enumeration and connection)
///
/// This trait abstracts device discovery and connection management.
/// Both real WPD managers and mock managers implement this trait.
pub trait DeviceManagerTrait: Send + Sync {
    /// The type of content interface returned when opening a device
    type Content: DeviceContentTrait;

    /// Enumerate all connected Apple devices (iPhones/iPads)
    fn enumerate_apple_devices(&self) -> Result<Vec<DeviceInfo>>;

    /// Enumerate all connected portable devices
    fn enumerate_all_devices(&self) -> Result<Vec<DeviceInfo>>;

    /// Open a connection to a device
    ///
    /// # Arguments
    /// * `device_id` - The device ID to connect to
    ///
    /// # Returns
    /// A content interface for accessing the device's file system
    fn open_device(&self, device_id: &str) -> Result<Self::Content>;

    /// Get information about a specific device
    ///
    /// # Arguments
    /// * `device_id` - The device ID to look up
    fn get_device_info(&self, device_id: &str) -> Option<DeviceInfo>;

    /// Get the number of available devices
    fn device_count(&self) -> usize;
}

/// A boxed device content trait object for dynamic dispatch
pub type BoxedDeviceContent = Box<dyn DeviceContentTrait>;

/// A boxed device manager trait object for dynamic dispatch
///
/// Note: This requires the Content type to also be boxed for full dynamic dispatch.
/// For most use cases, prefer using generics with `impl DeviceManagerTrait`.

/// Configuration for simulating device behaviors (errors, delays, etc.)
#[derive(Debug, Clone, Default)]
pub struct DeviceSimulationConfig {
    /// Simulate device being locked (access denied)
    pub simulate_locked: bool,
    /// Simulate disconnection after N file reads
    pub disconnect_after_reads: Option<usize>,
    /// Object IDs that should fail to read
    pub read_error_objects: Vec<String>,
    /// Simulated transfer delay in milliseconds per KB
    pub transfer_delay_ms_per_kb: u64,
    /// Random failure rate (0-100 percentage)
    pub random_failure_rate: u8,
}

impl DeviceSimulationConfig {
    /// Create a config that simulates a locked device
    pub fn locked() -> Self {
        Self {
            simulate_locked: true,
            ..Default::default()
        }
    }

    /// Create a config that simulates disconnection after N reads
    pub fn disconnect_after(reads: usize) -> Self {
        Self {
            disconnect_after_reads: Some(reads),
            ..Default::default()
        }
    }

    /// Create a config with slow transfer speed
    pub fn slow_transfer(ms_per_kb: u64) -> Self {
        Self {
            transfer_delay_ms_per_kb: ms_per_kb,
            ..Default::default()
        }
    }

    /// Create a config with random failures
    pub fn flaky(failure_rate: u8) -> Self {
        Self {
            random_failure_rate: failure_rate.min(100),
            ..Default::default()
        }
    }

    /// Add specific objects that should fail to read
    pub fn with_read_errors(mut self, object_ids: Vec<String>) -> Self {
        self.read_error_objects = object_ids;
        self
    }
}

/// Statistics about device operations
#[derive(Debug, Clone, Default)]
pub struct DeviceOperationStats {
    /// Number of objects enumerated
    pub objects_enumerated: usize,
    /// Number of files read
    pub files_read: usize,
    /// Total bytes read
    pub bytes_read: u64,
    /// Number of errors encountered
    pub errors: usize,
    /// Number of folders traversed
    pub folders_traversed: usize,
}

impl DeviceOperationStats {
    /// Record an enumeration operation
    pub fn record_enumeration(&mut self, object_count: usize) {
        self.objects_enumerated += object_count;
    }

    /// Record a file read operation
    pub fn record_file_read(&mut self, bytes: u64) {
        self.files_read += 1;
        self.bytes_read += bytes;
    }

    /// Record a folder traversal
    pub fn record_folder(&mut self) {
        self.folders_traversed += 1;
    }

    /// Record an error
    pub fn record_error(&mut self) {
        self.errors += 1;
    }

    /// Merge stats from another instance
    pub fn merge(&mut self, other: &DeviceOperationStats) {
        self.objects_enumerated += other.objects_enumerated;
        self.files_read += other.files_read;
        self.bytes_read += other.bytes_read;
        self.errors += other.errors;
        self.folders_traversed += other.folders_traversed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_creation() {
        let info = DeviceInfo::new("test-device-001", "Test iPhone", "Apple Inc.", "iPhone 15");

        assert_eq!(info.device_id, "test-device-001");
        assert_eq!(info.friendly_name, "Test iPhone");
        assert_eq!(info.manufacturer, "Apple Inc.");
        assert_eq!(info.model, "iPhone 15");
    }

    #[test]
    fn test_device_object_folder() {
        let folder = DeviceObject::folder("folder-001", "DEVICE", "DCIM");

        assert_eq!(folder.object_id, "folder-001");
        assert_eq!(folder.parent_id, "DEVICE");
        assert_eq!(folder.name, "DCIM");
        assert!(folder.is_folder);
        assert_eq!(folder.size, 0);
    }

    #[test]
    fn test_device_object_file() {
        let file = DeviceObject::file("file-001", "folder-001", "IMG_0001.JPG", 1024);

        assert_eq!(file.object_id, "file-001");
        assert_eq!(file.parent_id, "folder-001");
        assert_eq!(file.name, "IMG_0001.JPG");
        assert!(!file.is_folder);
        assert_eq!(file.size, 1024);
        assert_eq!(file.content_type, Some("image/jpeg".to_string()));
    }

    #[test]
    fn test_device_object_content_type_detection() {
        let cases = vec![
            ("photo.jpg", Some("image/jpeg")),
            ("photo.JPEG", Some("image/jpeg")),
            ("photo.png", Some("image/png")),
            ("photo.heic", Some("image/heic")),
            ("photo.heif", Some("image/heic")),
            ("photo.gif", Some("image/gif")),
            ("photo.webp", Some("image/webp")),
            ("photo.raw", Some("image/raw")),
            ("photo.dng", Some("image/raw")),
            ("photo.tiff", Some("image/tiff")),
            ("photo.tif", Some("image/tiff")),
            ("photo.bmp", Some("image/bmp")),
            ("video.mov", Some("video/quicktime")),
            ("video.mp4", Some("video/mp4")),
            ("video.m4v", Some("video/mp4")),
            ("video.avi", Some("video/avi")),
            ("video.3gp", Some("video/3gpp")),
            ("document.txt", None),
            ("noextension", None),
        ];

        for (name, expected) in cases {
            let file = DeviceObject::file("id", "parent", name, 100);
            assert_eq!(
                file.content_type,
                expected.map(String::from),
                "Failed for: {}",
                name
            );
        }
    }

    #[test]
    fn test_device_object_is_media_file() {
        let photo = DeviceObject::file("id", "parent", "IMG_0001.JPG", 100);
        assert!(photo.is_media_file());

        let video = DeviceObject::file("id", "parent", "video.mov", 100);
        assert!(video.is_media_file());

        let document = DeviceObject::file("id", "parent", "document.txt", 100);
        assert!(!document.is_media_file());

        let folder = DeviceObject::folder("id", "parent", "DCIM");
        assert!(!folder.is_media_file());
    }

    #[test]
    fn test_simulation_config_builders() {
        let locked = DeviceSimulationConfig::locked();
        assert!(locked.simulate_locked);

        let disconnect = DeviceSimulationConfig::disconnect_after(10);
        assert_eq!(disconnect.disconnect_after_reads, Some(10));

        let slow = DeviceSimulationConfig::slow_transfer(5);
        assert_eq!(slow.transfer_delay_ms_per_kb, 5);

        let flaky = DeviceSimulationConfig::flaky(25);
        assert_eq!(flaky.random_failure_rate, 25);

        let flaky_capped = DeviceSimulationConfig::flaky(150);
        assert_eq!(flaky_capped.random_failure_rate, 100);

        let with_errors = DeviceSimulationConfig::default()
            .with_read_errors(vec!["obj1".to_string(), "obj2".to_string()]);
        assert_eq!(with_errors.read_error_objects.len(), 2);
    }

    #[test]
    fn test_operation_stats() {
        let mut stats = DeviceOperationStats::default();

        stats.record_enumeration(10);
        stats.record_file_read(1024);
        stats.record_file_read(2048);
        stats.record_folder();
        stats.record_error();

        assert_eq!(stats.objects_enumerated, 10);
        assert_eq!(stats.files_read, 2);
        assert_eq!(stats.bytes_read, 3072);
        assert_eq!(stats.folders_traversed, 1);
        assert_eq!(stats.errors, 1);

        let mut other_stats = DeviceOperationStats::default();
        other_stats.record_enumeration(5);
        other_stats.record_file_read(512);

        stats.merge(&other_stats);
        assert_eq!(stats.objects_enumerated, 15);
        assert_eq!(stats.files_read, 3);
        assert_eq!(stats.bytes_read, 3584);
    }
}

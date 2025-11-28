//! Device abstraction traits for testability
//!
//! This module defines traits that abstract device operations, allowing both
//! real devices (iOS/Android via WPD) and mock devices to be used interchangeably. This enables
//! comprehensive testing of the extraction pipeline without connecting a real device.
//!
//! # Architecture
//!
//! The trait hierarchy is:
//! - `DeviceManagerTrait` - Enumerates and opens devices
//! - `DeviceContentTrait` - Provides access to device file system
//! - `DeviceInfo` - Common device information structure (shared, not a trait)
//! - `DeviceType` - Enum identifying the type of device (Apple, Android, etc.)
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use photo_extraction_tool::device::traits::{DeviceManagerTrait, DeviceContentTrait, DeviceType};
//!
//! fn extract_from_device<M: DeviceManagerTrait>(manager: &M) -> Result<(), String> {
//!     let devices = manager.enumerate_apple_devices().map_err(|e| e.to_string())?;
//!     if let Some(device) = devices.first() {
//!         println!("Device type: {:?}", device.device_type());
//!         let content = manager.open_device(&device.device_id).map_err(|e| e.to_string())?;
//!         let root_objects = content.enumerate_objects().map_err(|e| e.to_string())?;
//!         // ... process objects
//!     }
//!     Ok(())
//! }
//! ```

use crate::core::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug, Display};

/// Represents the type/category of a connected device
///
/// This enum is used to identify devices and apply appropriate handling
/// for different device types (folder structures, naming conventions, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DeviceType {
    /// Apple devices: iPhone, iPad, iPod touch
    Apple,
    /// Android devices: Samsung, Google Pixel, OnePlus, Xiaomi, etc.
    Android,
    /// Digital cameras (DSLR, mirrorless, point-and-shoot)
    Camera,
    /// Unknown or unrecognized MTP device
    #[default]
    Unknown,
}

impl DeviceType {
    /// Check if this is an Apple device
    pub fn is_apple(&self) -> bool {
        matches!(self, DeviceType::Apple)
    }

    /// Check if this is an Android device
    pub fn is_android(&self) -> bool {
        matches!(self, DeviceType::Android)
    }

    /// Check if this is a camera
    pub fn is_camera(&self) -> bool {
        matches!(self, DeviceType::Camera)
    }

    /// Check if this is an unknown device type
    pub fn is_unknown(&self) -> bool {
        matches!(self, DeviceType::Unknown)
    }

    /// Get a human-readable name for this device type
    pub fn display_name(&self) -> &'static str {
        match self {
            DeviceType::Apple => "Apple iOS",
            DeviceType::Android => "Android",
            DeviceType::Camera => "Camera",
            DeviceType::Unknown => "Unknown",
        }
    }
}

impl Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Known Android device manufacturers for detection
const ANDROID_MANUFACTURERS: &[&str] = &[
    "samsung", "google", "oneplus", "xiaomi", "huawei", "oppo", "vivo", "motorola", "lg", "sony",
    "asus", "lenovo", "zte", "realme", "nokia", "htc", "honor", "tecno", "infinix", "poco",
    "redmi", "nothing",
];

/// Known Android model keywords for detection
const ANDROID_MODEL_KEYWORDS: &[&str] = &[
    "galaxy",
    "pixel",
    "oneplus",
    "redmi",
    "poco",
    "mi ",
    "note ",
    "mate ",
    "nova",
    "p30",
    "p40",
    "p50",
    "find x",
    "reno",
    "realme",
    "moto ",
    "edge",
    "razr",
    "xperia",
    "zenfone",
    "rog phone",
    "nord",
    "nothing phone",
];

/// Known camera manufacturers for detection
const CAMERA_MANUFACTURERS: &[&str] = &[
    "canon",
    "nikon",
    "sony",
    "fujifilm",
    "panasonic",
    "olympus",
    "leica",
    "pentax",
    "hasselblad",
    "gopro",
    "dji",
    "insta360",
];

/// Known camera model keywords for detection
const CAMERA_MODEL_KEYWORDS: &[&str] = &[
    "eos",
    "rebel",
    "powershot",
    "coolpix",
    "alpha",
    "cybershot",
    "x-t",
    "x-s",
    "x-h",
    "x100",
    "lumix",
    "om-d",
    "pen",
    "hero",
    "mavic",
    "osmo",
];

/// Common device information shared between real and mock devices
#[derive(Debug, Clone, Default)]
pub struct DeviceInfo {
    /// Unique device identifier (WPD device ID or mock ID)
    pub device_id: String,
    /// User-friendly device name (e.g., "John's iPhone")
    pub friendly_name: String,
    /// Device manufacturer (e.g., "Apple Inc.", "Samsung")
    pub manufacturer: String,
    /// Device model (e.g., "iPhone 15 Pro", "Galaxy S24")
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

    /// Detect and return the device type based on manufacturer and model information
    ///
    /// This method analyzes the device's manufacturer, model, and friendly name
    /// to determine what type of device it is.
    pub fn device_type(&self) -> DeviceType {
        let manufacturer_lower = self.manufacturer.to_lowercase();
        let model_lower = self.model.to_lowercase();
        let name_lower = self.friendly_name.to_lowercase();

        // Check for Apple devices first
        if self.is_apple_device(&manufacturer_lower, &model_lower, &name_lower) {
            return DeviceType::Apple;
        }

        // Check for cameras (before Android, as some Sony devices could be cameras)
        if self.is_camera_device(&manufacturer_lower, &model_lower, &name_lower) {
            return DeviceType::Camera;
        }

        // Check for Android devices
        if self.is_android_device(&manufacturer_lower, &model_lower, &name_lower) {
            return DeviceType::Android;
        }

        DeviceType::Unknown
    }

    /// Check if this device is an Apple device
    fn is_apple_device(&self, manufacturer: &str, model: &str, name: &str) -> bool {
        // Check manufacturer
        if manufacturer.contains("apple") {
            return true;
        }

        // Check model keywords
        if model.contains("iphone") || model.contains("ipad") || model.contains("ipod") {
            return true;
        }

        // Check friendly name
        if name.contains("iphone")
            || name.contains("ipad")
            || name.contains("ipod")
            || name.contains("apple")
        {
            return true;
        }

        false
    }

    /// Check if this device is an Android device
    fn is_android_device(&self, manufacturer: &str, model: &str, name: &str) -> bool {
        // Check known Android manufacturers
        for android_mfr in ANDROID_MANUFACTURERS {
            if manufacturer.contains(android_mfr) {
                return true;
            }
        }

        // Check for "android" in any field
        if manufacturer.contains("android") || model.contains("android") || name.contains("android")
        {
            return true;
        }

        // Check known Android model keywords
        for keyword in ANDROID_MODEL_KEYWORDS {
            if model.contains(keyword) || name.contains(keyword) {
                return true;
            }
        }

        // Check for common Android device naming patterns
        // Many Android phones have model numbers like "SM-G998" (Samsung), "M2101K6G" (Xiaomi)
        if self.looks_like_android_model_number(model) {
            return true;
        }

        false
    }

    /// Check if this device is a camera
    fn is_camera_device(&self, manufacturer: &str, model: &str, name: &str) -> bool {
        // Check known camera manufacturers
        for camera_mfr in CAMERA_MANUFACTURERS {
            if manufacturer.contains(camera_mfr) {
                // Sony makes both phones and cameras, so we need additional checks
                if *camera_mfr == "sony" {
                    // If it looks like a phone model, it's not a camera
                    if model.contains("xperia") || self.looks_like_android_model_number(model) {
                        return false;
                    }
                }
                return true;
            }
        }

        // Check known camera model keywords
        for keyword in CAMERA_MODEL_KEYWORDS {
            if model.contains(keyword) || name.contains(keyword) {
                return true;
            }
        }

        // Check for "camera" or "dslr" keywords
        if model.contains("camera")
            || model.contains("dslr")
            || name.contains("camera")
            || name.contains("dslr")
        {
            return true;
        }

        false
    }

    /// Heuristic check for Android-style model numbers
    ///
    /// Many Android devices have model numbers like:
    /// - Samsung: SM-G998B, SM-A536B
    /// - Xiaomi: M2101K6G, 2201116SG
    /// - OnePlus: LE2125, IN2025
    fn looks_like_android_model_number(&self, model: &str) -> bool {
        let model_upper = model.to_uppercase();

        // Samsung pattern: SM-XXXX
        if model_upper.starts_with("SM-") {
            return true;
        }

        // Check for typical Android model patterns (letters and numbers mixed)
        // But exclude patterns that look like camera models
        let has_letters = model.chars().any(|c| c.is_ascii_alphabetic());
        let has_numbers = model.chars().any(|c| c.is_ascii_digit());
        let length_ok = model.len() >= 6 && model.len() <= 12;

        if has_letters && has_numbers && length_ok {
            // Exclude obvious camera patterns
            if model_upper.starts_with("EOS")
                || model_upper.starts_with("DSC")
                || model_upper.starts_with("NEX")
            {
                return false;
            }
            // This could be an Android model number
            // But we'll be conservative and not flag it without other evidence
        }

        false
    }

    /// Check if this device is of a specific type
    pub fn is_device_type(&self, device_type: DeviceType) -> bool {
        self.device_type() == device_type
    }

    /// Get a display string for the device type
    pub fn device_type_display(&self) -> &'static str {
        self.device_type().display_name()
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

    /// Enumerate all connected Android devices
    fn enumerate_android_devices(&self) -> Result<Vec<DeviceInfo>>;

    /// Enumerate all connected portable devices
    fn enumerate_all_devices(&self) -> Result<Vec<DeviceInfo>>;

    /// Enumerate devices filtered by type
    ///
    /// # Arguments
    /// * `device_type` - The type of devices to enumerate
    fn enumerate_devices_by_type(&self, device_type: DeviceType) -> Result<Vec<DeviceInfo>> {
        let all_devices = self.enumerate_all_devices()?;
        Ok(all_devices
            .into_iter()
            .filter(|d| d.device_type() == device_type)
            .collect())
    }

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
        let info = DeviceInfo::new("id-123", "Test Device", "Apple Inc.", "iPhone 15 Pro");

        assert_eq!(info.device_id, "id-123");
        assert_eq!(info.friendly_name, "Test Device");
        assert_eq!(info.manufacturer, "Apple Inc.");
        assert_eq!(info.model, "iPhone 15 Pro");
    }

    #[test]
    fn test_device_type_apple_detection() {
        // Test Apple by manufacturer
        let info = DeviceInfo::new("id-1", "John's iPhone", "Apple Inc.", "iPhone 15 Pro");
        assert_eq!(info.device_type(), DeviceType::Apple);
        assert!(info.device_type().is_apple());

        // Test Apple by model
        let info2 = DeviceInfo::new("id-2", "Device", "Unknown", "iPad Pro");
        assert_eq!(info2.device_type(), DeviceType::Apple);

        // Test Apple by friendly name
        let info3 = DeviceInfo::new("id-3", "My iPad", "(Standard MTP Device)", "MTP Device");
        assert_eq!(info3.device_type(), DeviceType::Apple);
    }

    #[test]
    fn test_device_type_android_detection() {
        // Test Samsung
        let info = DeviceInfo::new("id-1", "Galaxy S24", "Samsung", "SM-S928B");
        assert_eq!(info.device_type(), DeviceType::Android);
        assert!(info.device_type().is_android());

        // Test Google Pixel
        let info2 = DeviceInfo::new("id-2", "Pixel 8 Pro", "Google", "Pixel 8 Pro");
        assert_eq!(info2.device_type(), DeviceType::Android);

        // Test OnePlus
        let info3 = DeviceInfo::new("id-3", "OnePlus 12", "OnePlus", "LE2125");
        assert_eq!(info3.device_type(), DeviceType::Android);

        // Test Xiaomi
        let info4 = DeviceInfo::new("id-4", "Redmi Note 12", "Xiaomi", "Redmi Note 12");
        assert_eq!(info4.device_type(), DeviceType::Android);

        // Test by model keyword
        let info5 = DeviceInfo::new("id-5", "Phone", "Unknown", "Galaxy A54");
        assert_eq!(info5.device_type(), DeviceType::Android);
    }

    #[test]
    fn test_device_type_camera_detection() {
        // Test Canon
        let info = DeviceInfo::new("id-1", "Canon Camera", "Canon", "EOS R5");
        assert_eq!(info.device_type(), DeviceType::Camera);
        assert!(info.device_type().is_camera());

        // Test Nikon
        let info2 = DeviceInfo::new("id-2", "Nikon Camera", "Nikon", "Z8");
        assert_eq!(info2.device_type(), DeviceType::Camera);

        // Test GoPro
        let info3 = DeviceInfo::new("id-3", "GoPro", "GoPro", "HERO12");
        assert_eq!(info3.device_type(), DeviceType::Camera);
    }

    #[test]
    fn test_device_type_unknown() {
        let info = DeviceInfo::new("id-1", "Unknown Device", "Unknown Manufacturer", "Model X");
        assert_eq!(info.device_type(), DeviceType::Unknown);
        assert!(info.device_type().is_unknown());
    }

    #[test]
    fn test_device_type_display() {
        assert_eq!(DeviceType::Apple.display_name(), "Apple iOS");
        assert_eq!(DeviceType::Android.display_name(), "Android");
        assert_eq!(DeviceType::Camera.display_name(), "Camera");
        assert_eq!(DeviceType::Unknown.display_name(), "Unknown");

        // Test Display trait
        assert_eq!(format!("{}", DeviceType::Apple), "Apple iOS");
        assert_eq!(format!("{}", DeviceType::Android), "Android");
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

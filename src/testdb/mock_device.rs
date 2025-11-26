//! Mock device implementation for testing without a real device
//!
//! This module provides mock implementations of the WPD device interfaces
//! that simulate a connected iOS device with a configurable file structure.

use crate::core::error::{ExtractionError, Result};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Simulated device information
#[derive(Debug, Clone)]
pub struct MockDeviceInfo {
    /// Unique device identifier
    pub device_id: String,
    /// User-friendly device name
    pub friendly_name: String,
    /// Device manufacturer
    pub manufacturer: String,
    /// Device model
    pub model: String,
}

impl Default for MockDeviceInfo {
    fn default() -> Self {
        Self {
            device_id: "mock-device-001-apple-iphone-15-pro".to_string(),
            friendly_name: "John's iPhone 15 Pro".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 15 Pro".to_string(),
        }
    }
}

/// Represents a file or folder in the mock file system
#[derive(Debug, Clone)]
pub struct MockObject {
    /// Unique object ID (like WPD object ID)
    pub object_id: String,
    /// Parent object ID ("DEVICE" for root)
    pub parent_id: String,
    /// Object name (file/folder name)
    pub name: String,
    /// Whether this is a folder
    pub is_folder: bool,
    /// File size in bytes (0 for folders)
    pub size: u64,
    /// File content (for simulated extraction)
    pub content: Option<Vec<u8>>,
    /// Date modified (ISO 8601 string)
    pub date_modified: Option<String>,
    /// Content type hint (e.g., "image/jpeg")
    pub content_type: Option<String>,
}

impl MockObject {
    /// Create a new folder
    pub fn folder(object_id: &str, parent_id: &str, name: &str) -> Self {
        Self {
            object_id: object_id.to_string(),
            parent_id: parent_id.to_string(),
            name: name.to_string(),
            is_folder: true,
            size: 0,
            content: None,
            date_modified: None,
            content_type: None,
        }
    }

    /// Create a new file
    pub fn file(object_id: &str, parent_id: &str, name: &str, size: u64, content: Vec<u8>) -> Self {
        Self {
            object_id: object_id.to_string(),
            parent_id: parent_id.to_string(),
            name: name.to_string(),
            is_folder: false,
            size,
            content: Some(content),
            date_modified: Some("2024-01-15T10:30:00Z".to_string()),
            content_type: Self::guess_content_type(name),
        }
    }

    /// Create a file with specific date
    pub fn file_with_date(
        object_id: &str,
        parent_id: &str,
        name: &str,
        size: u64,
        content: Vec<u8>,
        date_modified: &str,
    ) -> Self {
        let mut file = Self::file(object_id, parent_id, name, size, content);
        file.date_modified = Some(date_modified.to_string());
        file
    }

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
        } else if lower.ends_with(".mov") {
            Some("video/quicktime".to_string())
        } else if lower.ends_with(".mp4") || lower.ends_with(".m4v") {
            Some("video/mp4".to_string())
        } else if lower.ends_with(".raw") || lower.ends_with(".dng") {
            Some("image/raw".to_string())
        } else {
            None
        }
    }
}

/// Configuration for mock device behavior
#[derive(Debug, Clone)]
pub struct MockDeviceConfig {
    /// Simulate device being locked (access denied)
    pub simulate_locked: bool,
    /// Simulate disconnection after N file reads
    pub disconnect_after_reads: Option<usize>,
    /// Simulate read errors for specific object IDs
    pub read_error_objects: Vec<String>,
    /// Simulate slow transfers (milliseconds delay per KB)
    pub transfer_delay_ms_per_kb: u64,
    /// Simulate random read failures (percentage 0-100)
    pub random_failure_rate: u8,
}

impl Default for MockDeviceConfig {
    fn default() -> Self {
        Self {
            simulate_locked: false,
            disconnect_after_reads: None,
            read_error_objects: Vec::new(),
            transfer_delay_ms_per_kb: 0,
            random_failure_rate: 0,
        }
    }
}

/// Mock device file system state
#[derive(Debug, Clone)]
pub struct MockFileSystem {
    /// All objects indexed by object ID
    objects: HashMap<String, MockObject>,
    /// Children index: parent_id -> Vec<object_id>
    children_index: HashMap<String, Vec<String>>,
    /// Read counter for disconnect simulation
    read_count: usize,
    /// Configuration
    config: MockDeviceConfig,
}

impl MockFileSystem {
    /// Create a new empty file system
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            children_index: HashMap::new(),
            read_count: 0,
            config: MockDeviceConfig::default(),
        }
    }

    /// Create with specific configuration
    pub fn with_config(config: MockDeviceConfig) -> Self {
        Self {
            objects: HashMap::new(),
            children_index: HashMap::new(),
            read_count: 0,
            config,
        }
    }

    /// Set the mock configuration
    pub fn set_config(&mut self, config: MockDeviceConfig) {
        self.config = config;
    }

    /// Get the configuration
    pub fn config(&self) -> &MockDeviceConfig {
        &self.config
    }

    /// Add an object to the file system
    pub fn add_object(&mut self, object: MockObject) {
        let object_id = object.object_id.clone();
        let parent_id = object.parent_id.clone();

        self.objects.insert(object_id.clone(), object);
        self.children_index
            .entry(parent_id)
            .or_default()
            .push(object_id);
    }

    /// Add multiple objects
    pub fn add_objects(&mut self, objects: Vec<MockObject>) {
        for obj in objects {
            self.add_object(obj);
        }
    }

    /// Get an object by ID
    pub fn get_object(&self, object_id: &str) -> Option<&MockObject> {
        self.objects.get(object_id)
    }

    /// Get children of an object
    pub fn get_children(&self, parent_id: &str) -> Vec<&MockObject> {
        self.children_index
            .get(parent_id)
            .map(|ids| ids.iter().filter_map(|id| self.objects.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all objects
    pub fn all_objects(&self) -> impl Iterator<Item = &MockObject> {
        self.objects.values()
    }

    /// Count total objects
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Count files only
    pub fn file_count(&self) -> usize {
        self.objects.values().filter(|o| !o.is_folder).count()
    }

    /// Count folders only
    pub fn folder_count(&self) -> usize {
        self.objects.values().filter(|o| o.is_folder).count()
    }

    /// Total size of all files
    pub fn total_size(&self) -> u64 {
        self.objects.values().map(|o| o.size).sum()
    }

    /// Read file content (with simulation effects)
    pub fn read_file(&mut self, object_id: &str) -> Result<Vec<u8>> {
        // Check if device is locked
        if self.config.simulate_locked {
            return Err(ExtractionError::AccessDenied);
        }

        // Check disconnect simulation
        if let Some(limit) = self.config.disconnect_after_reads {
            self.read_count += 1;
            if self.read_count > limit {
                return Err(ExtractionError::DeviceError(
                    "Device disconnected during transfer".to_string(),
                ));
            }
        }

        // Check specific error simulation
        if self
            .config
            .read_error_objects
            .contains(&object_id.to_string())
        {
            return Err(ExtractionError::TransferError {
                filename: object_id.to_string(),
                message: "Simulated read error".to_string(),
            });
        }

        // Check random failure
        if self.config.random_failure_rate > 0 {
            let roll = rand::random::<u8>() % 100;
            if roll < self.config.random_failure_rate {
                return Err(ExtractionError::TransferError {
                    filename: object_id.to_string(),
                    message: "Random simulated failure".to_string(),
                });
            }
        }

        // Get the object
        let obj = self.objects.get(object_id).ok_or_else(|| {
            ExtractionError::ContentError(format!("Object not found: {}", object_id))
        })?;

        if obj.is_folder {
            return Err(ExtractionError::ContentError(
                "Cannot read folder content".to_string(),
            ));
        }

        // Simulate transfer delay
        if self.config.transfer_delay_ms_per_kb > 0 {
            let delay_ms = (obj.size / 1024) * self.config.transfer_delay_ms_per_kb;
            std::thread::sleep(Duration::from_millis(delay_ms));
        }

        obj.content
            .clone()
            .ok_or_else(|| ExtractionError::ContentError("No content available".to_string()))
    }

    /// Reset read counter (for disconnect simulation)
    pub fn reset_read_count(&mut self) {
        self.read_count = 0;
    }

    /// Build path from root to object
    pub fn get_object_path(&self, object_id: &str) -> Option<String> {
        let mut parts = Vec::new();
        let mut current_id = object_id.to_string();

        while let Some(obj) = self.objects.get(&current_id) {
            if obj.parent_id == "DEVICE" {
                parts.push(obj.name.clone());
                break;
            }
            parts.push(obj.name.clone());
            current_id = obj.parent_id.clone();
        }

        if parts.is_empty() {
            None
        } else {
            parts.reverse();
            Some(parts.join("/"))
        }
    }
}

impl Default for MockFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock device manager that simulates DeviceManager
pub struct MockDeviceManager {
    /// List of available mock devices
    devices: Vec<MockDeviceInfo>,
    /// File systems for each device (by device_id)
    file_systems: HashMap<String, Arc<RwLock<MockFileSystem>>>,
    /// Configuration for device behavior
    configs: HashMap<String, MockDeviceConfig>,
}

impl MockDeviceManager {
    /// Create a new mock device manager with no devices
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            file_systems: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    /// Add a device with its file system
    pub fn add_device(&mut self, info: MockDeviceInfo, fs: MockFileSystem) {
        let device_id = info.device_id.clone();
        self.devices.push(info);
        self.file_systems
            .insert(device_id.clone(), Arc::new(RwLock::new(fs)));
    }

    /// Add a device with file system and config
    pub fn add_device_with_config(
        &mut self,
        info: MockDeviceInfo,
        mut fs: MockFileSystem,
        config: MockDeviceConfig,
    ) {
        let device_id = info.device_id.clone();
        fs.set_config(config.clone());
        self.devices.push(info);
        self.file_systems
            .insert(device_id.clone(), Arc::new(RwLock::new(fs)));
        self.configs.insert(device_id, config);
    }

    /// Get all devices
    pub fn enumerate_all_devices(&self) -> Result<Vec<MockDeviceInfo>> {
        Ok(self.devices.clone())
    }

    /// Get Apple devices only
    pub fn enumerate_apple_devices(&self) -> Result<Vec<MockDeviceInfo>> {
        Ok(self
            .devices
            .iter()
            .filter(|d| {
                d.manufacturer.to_lowercase().contains("apple")
                    || d.model.to_lowercase().contains("iphone")
                    || d.model.to_lowercase().contains("ipad")
            })
            .cloned()
            .collect())
    }

    /// Open a device (returns file system reference)
    pub fn open_device(&self, device_id: &str) -> Result<Arc<RwLock<MockFileSystem>>> {
        // Check if device should simulate being locked
        if let Some(config) = self.configs.get(device_id) {
            if config.simulate_locked {
                return Err(ExtractionError::AccessDenied);
            }
        }

        self.file_systems
            .get(device_id)
            .cloned()
            .ok_or_else(|| ExtractionError::DeviceError(format!("Device not found: {}", device_id)))
    }

    /// Get device info by ID
    pub fn get_device_info(&self, device_id: &str) -> Option<&MockDeviceInfo> {
        self.devices.iter().find(|d| d.device_id == device_id)
    }

    /// Get file system for device (if exists)
    pub fn get_file_system(&self, device_id: &str) -> Option<Arc<RwLock<MockFileSystem>>> {
        self.file_systems.get(device_id).cloned()
    }

    /// Get device count
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}

impl Default for MockDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A reader that wraps mock file content
pub struct MockFileReader {
    cursor: Cursor<Vec<u8>>,
}

impl MockFileReader {
    pub fn new(content: Vec<u8>) -> Self {
        Self {
            cursor: Cursor::new(content),
        }
    }
}

impl Read for MockFileReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_file_system_basic() {
        let mut fs = MockFileSystem::new();

        // Add a folder structure
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // Add a file
        let content = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header
        fs.add_object(MockObject::file(
            "img001",
            "100apple",
            "IMG_0001.JPG",
            4,
            content,
        ));

        assert_eq!(fs.object_count(), 4);
        assert_eq!(fs.file_count(), 1);
        assert_eq!(fs.folder_count(), 3);
    }

    #[test]
    fn test_mock_file_system_children() {
        let mut fs = MockFileSystem::new();

        fs.add_object(MockObject::folder("dcim", "DEVICE", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));
        fs.add_object(MockObject::folder("101apple", "dcim", "101APPLE"));

        let children = fs.get_children("dcim");
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_mock_device_manager() {
        let mut manager = MockDeviceManager::new();

        let device = MockDeviceInfo {
            device_id: "test-iphone".to_string(),
            friendly_name: "Test iPhone".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 14".to_string(),
        };

        let fs = MockFileSystem::new();
        manager.add_device(device, fs);

        let apple_devices = manager.enumerate_apple_devices().unwrap();
        assert_eq!(apple_devices.len(), 1);
    }

    #[test]
    fn test_access_denied_simulation() {
        let mut manager = MockDeviceManager::new();

        let device = MockDeviceInfo::default();
        let fs = MockFileSystem::new();
        let config = MockDeviceConfig {
            simulate_locked: true,
            ..Default::default()
        };

        manager.add_device_with_config(device.clone(), fs, config);

        let result = manager.open_device(&device.device_id);
        assert!(matches!(result, Err(ExtractionError::AccessDenied)));
    }
}

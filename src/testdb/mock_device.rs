//! Mock device implementation for testing without a real iOS device
//!
//! This module provides mock implementations of the device traits from `device::traits`
//! that simulate a connected iOS device (iPhone/iPad) with a configurable file structure.
//! This enables comprehensive testing of the extraction pipeline without
//! connecting a real iOS device.
//!
//! # Memory Efficiency
//!
//! This module is designed to be memory-efficient for testing:
//! - `MockObject::lazy_file()` generates content on-demand instead of storing it
//! - Content sizes are kept small by default
//! - File systems can be cloned without duplicating large content buffers
//!
//! # Example
//!
//! ```rust,no_run
//! use photo_extraction_tool::testdb::mock_device::{MockDeviceManager, MockFileSystem};
//! use photo_extraction_tool::device::traits::{DeviceManagerTrait, DeviceContentTrait, DeviceInfo};
//!
//! // Create a mock device manager
//! let mut manager = MockDeviceManager::new();
//!
//! // Create a file system with some files
//! let mut fs = MockFileSystem::new();
//! fs.add_standard_dcim_structure(10, 2);
//!
//! // Add the device
//! let device_info = DeviceInfo::new(
//!     "mock-device-001",
//!     "Test iPhone",
//!     "Apple Inc.",
//!     "iPhone 15 Pro",
//! );
//! manager.add_device(device_info, fs);
//!
//! // Use it like a real device
//! let devices = manager.enumerate_apple_devices().unwrap();
//! let content = manager.open_device(&devices[0].device_id).unwrap();
//! let root_objects = content.enumerate_objects().unwrap();
//! ```

#![allow(unused)]

use crate::core::error::{ExtractionError, Result};
use crate::device::traits::{
    DeviceContentTrait, DeviceInfo, DeviceManagerTrait, DeviceObject, DeviceSimulationConfig,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

// =============================================================================
// MockObject - File/folder representation with content
// =============================================================================

/// Lazy content generator for memory-efficient testing
///
/// Instead of storing large Vec<u8> buffers, this stores parameters
/// to generate content on demand.
#[derive(Debug, Clone)]
pub enum LazyContent {
    /// Pre-allocated content (for small files or when exact content matters)
    Eager(Vec<u8>),
    /// Generate content on-demand using extension and seed
    Lazy {
        extension: String,
        size: usize,
        seed: u64,
    },
    /// No content (for folders)
    None,
}

impl LazyContent {
    /// Get or generate the content
    pub fn get(&self) -> Option<Vec<u8>> {
        match self {
            LazyContent::Eager(data) => Some(data.clone()),
            LazyContent::Lazy {
                extension,
                size,
                seed,
            } => {
                use crate::testdb::generator::MockDataGenerator;
                Some(MockDataGenerator::generate_for_extension_with_seed(
                    extension, *size, *seed,
                ))
            }
            LazyContent::None => None,
        }
    }

    /// Check if this has content (eager or lazy)
    pub fn has_content(&self) -> bool {
        !matches!(self, LazyContent::None)
    }

    /// Get the size of the content without generating it
    pub fn size(&self) -> usize {
        match self {
            LazyContent::Eager(data) => data.len(),
            LazyContent::Lazy { size, .. } => *size,
            LazyContent::None => 0,
        }
    }
}

/// Represents a file or folder in the mock file system with its content
#[derive(Debug, Clone)]
pub struct MockObject {
    /// The device object metadata
    pub object: DeviceObject,
    /// File content (for files only) - can be eager or lazy
    pub content: Option<Vec<u8>>,
    /// Lazy content generator for memory-efficient testing
    lazy_content: LazyContent,
}

impl MockObject {
    /// Create a new folder
    pub fn folder(object_id: &str, parent_id: &str, name: &str) -> Self {
        Self {
            object: DeviceObject::folder(object_id, parent_id, name),
            content: None,
            lazy_content: LazyContent::None,
        }
    }

    /// Create a new file with content
    pub fn file(object_id: &str, parent_id: &str, name: &str, content: Vec<u8>) -> Self {
        let size = content.len() as u64;
        Self {
            object: DeviceObject::file(object_id, parent_id, name, size),
            content: Some(content),
            lazy_content: LazyContent::None,
        }
    }

    /// Create a memory-efficient lazy file that generates content on-demand
    ///
    /// This is much more memory-efficient for large test suites as it doesn't
    /// allocate content until it's actually needed.
    pub fn lazy_file(object_id: &str, parent_id: &str, name: &str, size: usize, seed: u64) -> Self {
        let extension = std::path::Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
            .to_lowercase();

        Self {
            object: DeviceObject::file(object_id, parent_id, name, size as u64),
            content: None,
            lazy_content: LazyContent::Lazy {
                extension,
                size,
                seed,
            },
        }
    }

    /// Create a file with a specific size (content will be generated lazily)
    ///
    /// Note: Now uses lazy generation for memory efficiency
    pub fn file_with_size(object_id: &str, parent_id: &str, name: &str, size: u64) -> Self {
        // Use lazy generation instead of pre-allocating
        Self::lazy_file(object_id, parent_id, name, size as usize, 0)
    }

    /// Create a file with specific date
    pub fn file_with_date(
        object_id: &str,
        parent_id: &str,
        name: &str,
        content: Vec<u8>,
        date_modified: &str,
    ) -> Self {
        let size = content.len() as u64;
        Self {
            object: DeviceObject::file_with_date(object_id, parent_id, name, size, date_modified),
            content: Some(content),
            lazy_content: LazyContent::None,
        }
    }

    /// Get the object ID
    pub fn object_id(&self) -> &str {
        &self.object.object_id
    }

    /// Get the parent ID
    pub fn parent_id(&self) -> &str {
        &self.object.parent_id
    }

    /// Get the name
    pub fn name(&self) -> &str {
        &self.object.name
    }

    /// Check if this is a folder
    pub fn is_folder(&self) -> bool {
        self.object.is_folder
    }

    /// Get the size
    pub fn size(&self) -> u64 {
        self.object.size
    }

    /// Get or generate the file content
    ///
    /// For lazy files, this generates content on-demand.
    /// For eager files, this returns the stored content.
    pub fn get_content(&self) -> Option<Vec<u8>> {
        // First check eager content
        if let Some(ref content) = self.content {
            return Some(content.clone());
        }
        // Then check lazy content
        self.lazy_content.get()
    }

    /// Check if this object has content (eager or lazy)
    pub fn has_content(&self) -> bool {
        self.content.is_some() || self.lazy_content.has_content()
    }
}

// =============================================================================
// MockDeviceConfig - Configuration for simulation behaviors
// =============================================================================

/// Configuration for mock device behavior
///
/// This is a wrapper around DeviceSimulationConfig with additional mock-specific options.
#[derive(Debug, Clone, Default)]
pub struct MockDeviceConfig {
    /// Base simulation configuration
    pub simulation: DeviceSimulationConfig,
    /// Custom error message for locked device
    pub locked_message: Option<String>,
}

impl MockDeviceConfig {
    /// Create a default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config that simulates a locked device
    pub fn locked() -> Self {
        Self {
            simulation: DeviceSimulationConfig::locked(),
            locked_message: Some(
                "Device is locked. Please unlock and trust this computer.".to_string(),
            ),
        }
    }

    /// Create a config that simulates disconnection
    pub fn disconnect_after(reads: usize) -> Self {
        Self {
            simulation: DeviceSimulationConfig::disconnect_after(reads),
            ..Default::default()
        }
    }

    /// Create a config with slow transfer
    pub fn slow_transfer(ms_per_kb: u64) -> Self {
        Self {
            simulation: DeviceSimulationConfig::slow_transfer(ms_per_kb),
            ..Default::default()
        }
    }

    /// Create a config with random failures
    pub fn flaky(failure_rate: u8) -> Self {
        Self {
            simulation: DeviceSimulationConfig::flaky(failure_rate),
            ..Default::default()
        }
    }

    /// Add specific objects that should fail to read
    pub fn with_read_errors(mut self, object_ids: Vec<String>) -> Self {
        self.simulation.read_error_objects = object_ids;
        self
    }
}

impl From<DeviceSimulationConfig> for MockDeviceConfig {
    fn from(config: DeviceSimulationConfig) -> Self {
        Self {
            simulation: config,
            locked_message: None,
        }
    }
}

// =============================================================================
// MockFileSystem - In-memory file system
// =============================================================================

/// Mock device file system state
///
/// This represents the complete file system of a mock device, storing
/// all objects (files and folders) and their relationships.
#[derive(Debug)]
pub struct MockFileSystem {
    /// All objects indexed by object ID
    objects: HashMap<String, MockObject>,
    /// Children index: parent_id -> Vec<object_id>
    children_index: HashMap<String, Vec<String>>,
    /// Read counter for disconnect simulation
    read_count: AtomicUsize,
    /// Configuration for simulation behaviors
    config: MockDeviceConfig,
}

impl Clone for MockFileSystem {
    fn clone(&self) -> Self {
        Self {
            objects: self.objects.clone(),
            children_index: self.children_index.clone(),
            read_count: AtomicUsize::new(self.read_count.load(Ordering::SeqCst)),
            config: self.config.clone(),
        }
    }
}

impl MockFileSystem {
    /// Create a new empty file system
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            children_index: HashMap::new(),
            read_count: AtomicUsize::new(0),
            config: MockDeviceConfig::default(),
        }
    }

    /// Create with specific configuration
    pub fn with_config(config: MockDeviceConfig) -> Self {
        Self {
            objects: HashMap::new(),
            children_index: HashMap::new(),
            read_count: AtomicUsize::new(0),
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

    /// Get mutable configuration
    pub fn config_mut(&mut self) -> &mut MockDeviceConfig {
        &mut self.config
    }

    /// Add an object to the file system
    pub fn add_object(&mut self, object: MockObject) {
        let object_id = object.object_id().to_string();
        let parent_id = object.parent_id().to_string();

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
        self.objects.values().filter(|o| !o.is_folder()).count()
    }

    /// Count folders only
    pub fn folder_count(&self) -> usize {
        self.objects.values().filter(|o| o.is_folder()).count()
    }

    /// Total size of all files
    pub fn total_size(&self) -> u64 {
        self.objects.values().map(|o| o.size()).sum()
    }

    /// Read file content (with simulation effects)
    ///
    /// Supports both eager content (pre-allocated) and lazy content (generated on-demand).
    /// Lazy content is more memory-efficient for large test suites.
    pub fn read_file(&self, object_id: &str) -> Result<Vec<u8>> {
        // Check if device is locked
        if self.config.simulation.simulate_locked {
            return Err(ExtractionError::AccessDenied);
        }

        // Check disconnect simulation
        if let Some(limit) = self.config.simulation.disconnect_after_reads {
            let count = self.read_count.fetch_add(1, Ordering::SeqCst);
            if count >= limit {
                return Err(ExtractionError::DeviceError(
                    "Device disconnected during transfer".to_string(),
                ));
            }
        }

        // Check specific error simulation
        if self
            .config
            .simulation
            .read_error_objects
            .contains(&object_id.to_string())
        {
            return Err(ExtractionError::TransferError {
                filename: object_id.to_string(),
                message: "Simulated read error".to_string(),
            });
        }

        // Check random failure
        if self.config.simulation.random_failure_rate > 0 {
            let roll = rand::random::<u8>() % 100;
            if roll < self.config.simulation.random_failure_rate {
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

        if obj.is_folder() {
            return Err(ExtractionError::ContentError(
                "Cannot read folder content".to_string(),
            ));
        }

        // Simulate transfer delay
        if self.config.simulation.transfer_delay_ms_per_kb > 0 {
            let delay_ms = (obj.size() / 1024) * self.config.simulation.transfer_delay_ms_per_kb;
            std::thread::sleep(Duration::from_millis(delay_ms));
        }

        // Use the new get_content method that supports both eager and lazy content
        obj.get_content()
            .ok_or_else(|| ExtractionError::ContentError("No content available".to_string()))
    }

    /// Reset read counter (for disconnect simulation)
    pub fn reset_read_count(&self) {
        self.read_count.store(0, Ordering::SeqCst);
    }

    /// Get current read count
    pub fn get_read_count(&self) -> usize {
        self.read_count.load(Ordering::SeqCst)
    }

    /// Build path from root to object
    pub fn get_object_path(&self, object_id: &str) -> Option<String> {
        let mut parts = Vec::new();
        let mut current_id = object_id.to_string();

        while let Some(obj) = self.objects.get(&current_id) {
            if obj.parent_id() == "DEVICE" {
                parts.push(obj.name().to_string());
                break;
            }
            parts.push(obj.name().to_string());
            current_id = obj.parent_id().to_string();
        }

        if parts.is_empty() {
            None
        } else {
            parts.reverse();
            Some(parts.join("/"))
        }
    }

    // =========================================================================
    // Helper methods for building common structures
    // =========================================================================

    /// Add a standard iOS DCIM folder structure
    ///
    /// Creates: Internal Storage/DCIM/100APPLE, 101APPLE, etc.
    /// Uses memory-efficient small content sizes to prevent memory exhaustion
    pub fn add_standard_dcim_structure(&mut self, files_per_folder: usize, num_folders: usize) {
        use crate::testdb::generator::{MockDataGenerator, TEST_JPEG_SIZE};

        // Add Internal Storage
        self.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));

        // Add DCIM folder
        self.add_object(MockObject::folder("dcim", "internal", "DCIM"));

        let mut file_counter = 1;

        for folder_idx in 0..num_folders {
            let folder_num = 100 + folder_idx;
            let folder_id = format!("{}apple", folder_num);
            let folder_name = format!("{}APPLE", folder_num);

            self.add_object(MockObject::folder(&folder_id, "dcim", &folder_name));

            for file_idx in 0..files_per_folder {
                let file_id = format!("img_{:06}", file_counter);
                let file_name = format!("IMG_{:04}.JPG", file_counter);

                // Generate small JPEG content for memory efficiency
                // Tests only need valid headers, not full file content
                let content = MockDataGenerator::generate_jpeg_with_seed(
                    TEST_JPEG_SIZE, // 2KB instead of 1MB
                    file_counter as u64,
                );

                self.add_object(MockObject::file(&file_id, &folder_id, &file_name, content));

                file_counter += 1;
            }
        }
    }

    /// Add a deeply nested folder structure for testing
    /// Uses memory-efficient lazy files to prevent memory exhaustion
    pub fn add_nested_structure(&mut self, depth: usize, files_per_level: usize) {
        use crate::testdb::generator::{MockDataGenerator, TEST_JPEG_SIZE};

        let mut parent_id = "DEVICE".to_string();
        let mut file_counter = 1;

        for level in 0..depth {
            let folder_id = format!("level_{}", level);
            let folder_name = format!("Level{}", level);

            self.add_object(MockObject::folder(&folder_id, &parent_id, &folder_name));

            // Add files at this level with small content for memory efficiency
            for file_idx in 0..files_per_level {
                let file_id = format!("file_{}_{}", level, file_idx);
                let file_name = format!("FILE_{:04}.JPG", file_counter);

                // Use small content size for memory efficiency
                let content =
                    MockDataGenerator::generate_jpeg_with_seed(TEST_JPEG_SIZE, file_counter as u64);

                self.add_object(MockObject::file(&file_id, &folder_id, &file_name, content));

                file_counter += 1;
            }

            parent_id = folder_id;
        }
    }

    /// Add mixed file types for testing format handling
    /// Uses small content sizes for memory efficiency
    pub fn add_mixed_file_types(&mut self) {
        use crate::testdb::generator::{MockDataGenerator, TEST_CONTENT_MAX_SIZE};

        self.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        self.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        self.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        let file_types = vec![
            ("photo1.jpg", "jpg"),
            ("photo2.jpeg", "jpeg"),
            ("photo3.png", "png"),
            ("photo4.heic", "heic"),
            ("photo5.gif", "gif"),
            ("photo6.webp", "webp"),
            ("photo7.raw", "raw"),
            ("photo8.dng", "dng"),
            ("photo9.tiff", "tiff"),
            ("photo10.bmp", "bmp"),
            ("video1.mov", "mov"),
            ("video2.mp4", "mp4"),
            ("video3.avi", "avi"),
            ("video4.3gp", "3gp"),
        ];

        for (idx, (name, ext)) in file_types.iter().enumerate() {
            let file_id = format!("mixed_{}", idx);
            // Use small content size (4KB max) for memory efficiency
            let content = MockDataGenerator::generate_for_extension_with_seed(
                ext,
                TEST_CONTENT_MAX_SIZE,
                idx as u64,
            );
            self.add_object(MockObject::file(&file_id, "100apple", name, content));
        }
    }
}

impl Default for MockFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MockDeviceContent - Implements DeviceContentTrait
// =============================================================================

/// Mock implementation of DeviceContentTrait
///
/// This wraps a MockFileSystem and provides the trait interface.
pub struct MockDeviceContent {
    /// The underlying file system
    fs: Arc<RwLock<MockFileSystem>>,
    /// Device ID for error messages
    device_id: String,
}

impl MockDeviceContent {
    /// Create a new MockDeviceContent
    pub fn new(fs: Arc<RwLock<MockFileSystem>>, device_id: &str) -> Self {
        Self {
            fs,
            device_id: device_id.to_string(),
        }
    }

    /// Get access to the underlying file system
    pub fn file_system(&self) -> Arc<RwLock<MockFileSystem>> {
        self.fs.clone()
    }
}

impl DeviceContentTrait for MockDeviceContent {
    fn enumerate_objects(&self) -> Result<Vec<DeviceObject>> {
        self.enumerate_children("DEVICE")
    }

    fn enumerate_children(&self, parent_id: &str) -> Result<Vec<DeviceObject>> {
        let fs = self.fs.read().map_err(|e| {
            ExtractionError::DeviceError(format!("Failed to acquire read lock: {}", e))
        })?;

        // Check if device is locked
        if fs.config().simulation.simulate_locked {
            return Err(ExtractionError::AccessDenied);
        }

        let children = fs.get_children(parent_id);
        Ok(children.into_iter().map(|o| o.object.clone()).collect())
    }

    fn read_file(&self, object_id: &str) -> Result<Vec<u8>> {
        let fs = self.fs.read().map_err(|e| {
            ExtractionError::DeviceError(format!("Failed to acquire read lock: {}", e))
        })?;

        fs.read_file(object_id)
    }

    fn get_object(&self, object_id: &str) -> Result<Option<DeviceObject>> {
        let fs = self.fs.read().map_err(|e| {
            ExtractionError::DeviceError(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(fs.get_object(object_id).map(|o| o.object.clone()))
    }

    fn get_object_path(&self, object_id: &str) -> Option<String> {
        let fs = self.fs.read().ok()?;
        fs.get_object_path(object_id)
    }
}

// =============================================================================
// MockDeviceManager - Implements DeviceManagerTrait
// =============================================================================

/// Mock device manager that simulates DeviceManager
///
/// This manages multiple mock devices and their file systems.
pub struct MockDeviceManager {
    /// List of available mock devices
    devices: Vec<DeviceInfo>,
    /// File systems for each device (by device_id)
    file_systems: HashMap<String, Arc<RwLock<MockFileSystem>>>,
    /// Configuration for each device (by device_id)
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
    pub fn add_device(&mut self, info: DeviceInfo, fs: MockFileSystem) {
        let device_id = info.device_id.clone();
        self.devices.push(info);
        self.file_systems
            .insert(device_id.clone(), Arc::new(RwLock::new(fs)));
    }

    /// Add a device with file system and config
    pub fn add_device_with_config(
        &mut self,
        info: DeviceInfo,
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

    /// Get file system for device (if exists)
    pub fn get_file_system(&self, device_id: &str) -> Option<Arc<RwLock<MockFileSystem>>> {
        self.file_systems.get(device_id).cloned()
    }

    /// Remove a device
    pub fn remove_device(&mut self, device_id: &str) {
        self.devices.retain(|d| d.device_id != device_id);
        self.file_systems.remove(device_id);
        self.configs.remove(device_id);
    }

    /// Clear all devices
    pub fn clear(&mut self) {
        self.devices.clear();
        self.file_systems.clear();
        self.configs.clear();
    }
}

impl Default for MockDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceManagerTrait for MockDeviceManager {
    type Content = MockDeviceContent;

    fn enumerate_apple_devices(&self) -> Result<Vec<DeviceInfo>> {
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

    fn enumerate_all_devices(&self) -> Result<Vec<DeviceInfo>> {
        Ok(self.devices.clone())
    }

    fn open_device(&self, device_id: &str) -> Result<Self::Content> {
        // Check if device should simulate being locked
        if let Some(config) = self.configs.get(device_id) {
            if config.simulation.simulate_locked {
                return Err(ExtractionError::AccessDenied);
            }
        }

        let fs = self.file_systems.get(device_id).cloned().ok_or_else(|| {
            ExtractionError::DeviceError(format!("Device not found: {}", device_id))
        })?;

        Ok(MockDeviceContent::new(fs, device_id))
    }

    fn get_device_info(&self, device_id: &str) -> Option<DeviceInfo> {
        self.devices
            .iter()
            .find(|d| d.device_id == device_id)
            .cloned()
    }

    fn device_count(&self) -> usize {
        self.devices.len()
    }
}

// =============================================================================
// Compatibility re-exports
// =============================================================================

/// Re-export DeviceInfo for convenience
pub use crate::device::traits::DeviceInfo as MockDeviceInfo;

// =============================================================================
// Tests
// =============================================================================

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
    fn test_mock_file_system_path() {
        let mut fs = MockFileSystem::new();

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        let content = vec![0xFF, 0xD8, 0xFF, 0xE0];
        fs.add_object(MockObject::file(
            "img001",
            "100apple",
            "IMG_0001.JPG",
            content,
        ));

        let path = fs.get_object_path("img001");
        assert_eq!(
            path,
            Some("Internal Storage/DCIM/100APPLE/IMG_0001.JPG".to_string())
        );
    }

    #[test]
    fn test_mock_device_manager() {
        let mut manager = MockDeviceManager::new();

        let device = DeviceInfo {
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
    fn test_mock_device_manager_trait() {
        let mut manager = MockDeviceManager::new();

        let device = DeviceInfo::new("test-001", "Test iPhone", "Apple Inc.", "iPhone 15");
        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));

        manager.add_device(device, fs);

        // Test trait methods
        assert_eq!(manager.device_count(), 1);

        let devices = manager.enumerate_apple_devices().unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].friendly_name, "Test iPhone");

        let content = manager.open_device("test-001").unwrap();
        let root_objects = content.enumerate_objects().unwrap();
        assert_eq!(root_objects.len(), 1);
        assert_eq!(root_objects[0].name, "Internal Storage");
    }

    #[test]
    fn test_access_denied_simulation() {
        let mut manager = MockDeviceManager::new();

        let device = DeviceInfo::new("locked-device", "Locked iPhone", "Apple Inc.", "iPhone");
        let fs = MockFileSystem::new();
        let config = MockDeviceConfig::locked();

        manager.add_device_with_config(device.clone(), fs, config);

        let result = manager.open_device(&device.device_id);
        assert!(matches!(result, Err(ExtractionError::AccessDenied)));
    }

    #[test]
    fn test_disconnect_simulation() {
        let mut manager = MockDeviceManager::new();

        let device = DeviceInfo::new("disconnect-device", "Test iPhone", "Apple Inc.", "iPhone");
        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));

        let content = vec![0xFF; 100];
        for i in 0..5 {
            fs.add_object(MockObject::file(
                &format!("file{}", i),
                "internal",
                &format!("FILE{}.JPG", i),
                content.clone(),
            ));
        }

        let config = MockDeviceConfig::disconnect_after(3);
        manager.add_device_with_config(device.clone(), fs, config);

        let content = manager.open_device(&device.device_id).unwrap();

        // First 3 reads should succeed
        assert!(content.read_file("file0").is_ok());
        assert!(content.read_file("file1").is_ok());
        assert!(content.read_file("file2").is_ok());

        // 4th read should fail
        let result = content.read_file("file3");
        assert!(result.is_err());
    }

    #[test]
    fn test_standard_dcim_structure() {
        let mut fs = MockFileSystem::new();
        fs.add_standard_dcim_structure(5, 2);

        // Should have: Internal Storage, DCIM, 100APPLE, 101APPLE, and 10 files
        assert_eq!(fs.folder_count(), 4);
        assert_eq!(fs.file_count(), 10);
    }

    #[test]
    fn test_mixed_file_types() {
        let mut fs = MockFileSystem::new();
        fs.add_mixed_file_types();

        // Should have 14 different file types
        assert_eq!(fs.file_count(), 14);

        // Verify all objects have content
        for obj in fs.all_objects() {
            if !obj.is_folder() {
                assert!(obj.content.is_some());
            }
        }
    }

    #[test]
    fn test_read_error_simulation() {
        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));

        let content = vec![0xFF; 100];
        fs.add_object(MockObject::file(
            "file1",
            "internal",
            "FILE1.JPG",
            content.clone(),
        ));
        fs.add_object(MockObject::file("file2", "internal", "FILE2.JPG", content));

        // Configure file2 to fail
        let config = MockDeviceConfig::default().with_read_errors(vec!["file2".to_string()]);
        fs.set_config(config);

        // file1 should succeed
        assert!(fs.read_file("file1").is_ok());

        // file2 should fail
        let result = fs.read_file("file2");
        assert!(matches!(result, Err(ExtractionError::TransferError { .. })));
    }
}

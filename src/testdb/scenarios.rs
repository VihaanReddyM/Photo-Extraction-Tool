//! Predefined test scenarios for comprehensive testing
//!
//! This module provides ready-to-use test scenarios that cover various
//! use cases, edge cases, and error conditions for the photo extraction tool.
//!
//! Each scenario includes:
//! - A mock device with specific characteristics
//! - A mock file system with predefined structure
//! - Expected results for validation
//! - Tags for filtering and organization

use super::generator::MockDataGenerator;
use super::mock_device::{MockDeviceConfig, MockFileSystem, MockObject};
use crate::device::traits::DeviceInfo;

/// A complete test scenario with device info and file system
#[derive(Debug, Clone)]
pub struct TestScenario {
    /// Scenario name for identification
    pub name: String,
    /// Description of what this scenario tests
    pub description: String,
    /// Mock device information
    pub device_info: DeviceInfo,
    /// Mock file system
    pub file_system: MockFileSystem,
    /// Expected statistics after extraction
    pub expected: ExpectedResults,
    /// Tags for filtering scenarios
    pub tags: Vec<String>,
}

/// Expected results from running an extraction
#[derive(Debug, Clone, Default)]
pub struct ExpectedResults {
    /// Expected number of files to extract
    pub files_to_extract: usize,
    /// Expected number of folders
    pub folders: usize,
    /// Expected total size in bytes
    pub total_size: u64,
    /// Expected number of duplicates (if duplicate detection enabled)
    pub duplicates: usize,
    /// Expected number of errors
    pub errors: usize,
    /// Should extraction succeed
    pub should_succeed: bool,
    /// Expected error type (if should_succeed is false)
    pub expected_error: Option<String>,
}

impl TestScenario {
    /// Create a new test scenario
    pub fn new(
        name: &str,
        description: &str,
        device_info: DeviceInfo,
        file_system: MockFileSystem,
        expected: ExpectedResults,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            device_info,
            file_system,
            expected,
            tags: Vec::new(),
        }
    }

    /// Add tags to the scenario
    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(String::from).collect();
        self
    }
}

/// Collection of all predefined test scenarios
pub struct ScenarioLibrary;

impl ScenarioLibrary {
    // =========================================================================
    // DEVICE DETECTION SCENARIOS
    // =========================================================================

    /// Scenario: No devices connected
    pub fn no_devices() -> TestScenario {
        TestScenario::new(
            "no_devices",
            "Test behavior when no devices are connected",
            DeviceInfo {
                device_id: "".to_string(),
                friendly_name: "".to_string(),
                manufacturer: "".to_string(),
                model: "".to_string(),
            },
            MockFileSystem::new(),
            ExpectedResults {
                should_succeed: false,
                expected_error: Some("NoDevicesFound".to_string()),
                ..Default::default()
            },
        )
        .with_tags(vec!["device", "error", "no-device"])
    }

    /// Scenario: Single iPhone connected
    pub fn single_iphone() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#serial123456",
            "John's iPhone",
            "Apple Inc.",
            "iPhone 15 Pro",
        );

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 50, 5);

        TestScenario::new(
            "single_iphone",
            "Standard single iPhone with typical photo library",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 50,
                folders: 7,                   // Internal Storage, DCIM, 5 APPLE folders
                total_size: 50 * 1024 * 1024, // ~50MB
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["device", "iphone", "basic"])
    }

    /// Scenario: Multiple iPhones connected
    pub fn multiple_iphones() -> Vec<TestScenario> {
        vec![
            {
                let device = DeviceInfo::new(
                    "\\\\?\\usb#vid_05ac&pid_12a8#serial_device1",
                    "Alice's iPhone 14",
                    "Apple Inc.",
                    "iPhone 14",
                );
                let mut fs = MockFileSystem::new();
                Self::add_standard_dcim_structure(&mut fs, 30, 3);

                TestScenario::new(
                    "multiple_iphones_device1",
                    "First of multiple iPhones",
                    device,
                    fs,
                    ExpectedResults {
                        files_to_extract: 30,
                        folders: 5,
                        total_size: 30 * 1024 * 1024,
                        should_succeed: true,
                        ..Default::default()
                    },
                )
                .with_tags(vec!["device", "iphone", "multiple"])
            },
            {
                let device = DeviceInfo::new(
                    "\\\\?\\usb#vid_05ac&pid_12a8#serial_device2",
                    "Bob's iPhone 15",
                    "Apple Inc.",
                    "iPhone 15",
                );
                let mut fs = MockFileSystem::new();
                Self::add_standard_dcim_structure(&mut fs, 45, 4);

                TestScenario::new(
                    "multiple_iphones_device2",
                    "Second of multiple iPhones",
                    device,
                    fs,
                    ExpectedResults {
                        files_to_extract: 45,
                        folders: 6,
                        total_size: 45 * 1024 * 1024,
                        should_succeed: true,
                        ..Default::default()
                    },
                )
                .with_tags(vec!["device", "iphone", "multiple"])
            },
        ]
    }

    /// Scenario: iPad device
    pub fn ipad_device() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12ab#ipad_serial",
            "Family iPad Pro",
            "Apple Inc.",
            "iPad Pro 12.9-inch",
        );

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 100, 10);

        TestScenario::new(
            "ipad_device",
            "iPad with larger photo library",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 100,
                folders: 12,
                total_size: 100 * 1024 * 1024,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["device", "ipad", "basic"])
    }

    /// Scenario: Mixed Apple and non-Apple devices
    pub fn mixed_devices() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#iphone_mixed",
            "iPhone in mixed setup",
            "Apple Inc.",
            "iPhone 13",
        );

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 25, 3);

        TestScenario::new(
            "mixed_devices",
            "Apple device among non-Apple devices",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 25,
                folders: 5,
                total_size: 25 * 1024 * 1024,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["device", "iphone", "mixed"])
    }

    /// Scenario: Old iPhone model
    pub fn old_iphone() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_1292#old_iphone",
            "Old iPhone 6s",
            "Apple Inc.",
            "iPhone 6s",
        );

        let mut fs = MockFileSystem::new();
        Self::add_legacy_dcim_structure(&mut fs, 20, 2);

        TestScenario::new(
            "old_iphone",
            "Legacy iPhone with older folder structure",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 20,
                folders: 4,
                total_size: 20 * 1024 * 1024,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["device", "iphone", "legacy"])
    }

    /// Scenario: Device with unicode name
    pub fn unicode_device_name() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#unicode_device",
            "田中さんのiPhone",
            "Apple Inc.",
            "iPhone 15 Pro",
        );

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 15, 2);

        TestScenario::new(
            "unicode_device_name",
            "Device with non-ASCII characters in name",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 15,
                folders: 4,
                total_size: 15 * 1024 * 1024,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["device", "unicode", "edge-case"])
    }

    // =========================================================================
    // FILE STRUCTURE SCENARIOS
    // =========================================================================

    /// Scenario: Empty device (no photos)
    pub fn empty_device() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#empty_device",
            "New iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));

        TestScenario::new(
            "empty_device",
            "Device with DCIM folder but no photos",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 0,
                folders: 2,
                total_size: 0,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "empty", "edge-case"])
    }

    /// Scenario: Deeply nested folder structure
    pub fn deeply_nested() -> TestScenario {
        use crate::testdb::generator::TEST_JPEG_SIZE;

        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#nested_device",
            "Test iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));

        // Create 15 levels of nesting
        // Use small content size for memory efficiency
        let mut parent_id = "internal".to_string();
        for i in 0..15 {
            let folder_id = format!("nested_{}", i);
            let folder_name = format!("Folder_{}", i);
            fs.add_object(MockObject::folder(&folder_id, &parent_id, &folder_name));

            // Add a file at each level with small content
            let file_id = format!("file_level_{}", i);
            let file_name = format!("IMG_{:04}.JPG", i);
            let content = MockDataGenerator::generate_jpeg_with_seed(TEST_JPEG_SIZE, i as u64);
            fs.add_object(MockObject::file(&file_id, &folder_id, &file_name, content));

            parent_id = folder_id;
        }

        TestScenario::new(
            "deeply_nested",
            "15 levels of nested folders",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 15,
                folders: 16,
                total_size: 15 * TEST_JPEG_SIZE as u64,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "nested", "edge-case"])
    }

    /// Scenario: Many files in a single folder
    pub fn many_files_single_folder() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#many_files",
            "Photo Hoarder's iPhone",
            "Apple Inc.",
            "iPhone 15 Pro Max",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // Add 5000 files to a single folder
        // Use small content size (2KB) to prevent memory exhaustion
        // 5000 files × 2KB = ~10MB total (vs 1.25GB with 256KB)
        use crate::testdb::generator::TEST_JPEG_SIZE;
        for i in 0..5000 {
            let file_id = format!("img_{:06}", i);
            let file_name = format!("IMG_{:04}.JPG", i);
            let content = MockDataGenerator::generate_jpeg_with_seed(TEST_JPEG_SIZE, i as u64);
            fs.add_object(MockObject::file(&file_id, "100apple", &file_name, content));
        }

        TestScenario::new(
            "many_files_single_folder",
            "5000 files in a single folder",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 5000,
                folders: 3,
                total_size: 5000 * TEST_JPEG_SIZE as u64,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "performance", "stress-test"])
    }

    /// Scenario: All supported file types
    pub fn mixed_file_types() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#mixed_types",
            "Test iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // Add various file types with small content sizes for memory efficiency
        // Reported sizes can be larger than actual content
        use crate::testdb::generator::TEST_CONTENT_MAX_SIZE;
        let file_types = vec![
            ("photo_001.jpg", "jpg", TEST_CONTENT_MAX_SIZE),
            ("photo_002.jpeg", "jpeg", TEST_CONTENT_MAX_SIZE),
            ("photo_003.JPG", "jpg", TEST_CONTENT_MAX_SIZE),
            ("photo_004.JPEG", "jpeg", TEST_CONTENT_MAX_SIZE),
            ("photo_005.png", "png", TEST_CONTENT_MAX_SIZE),
            ("photo_006.PNG", "png", TEST_CONTENT_MAX_SIZE),
            ("photo_007.heic", "heic", TEST_CONTENT_MAX_SIZE),
            ("photo_008.HEIC", "heic", TEST_CONTENT_MAX_SIZE),
            ("photo_009.heif", "heic", TEST_CONTENT_MAX_SIZE),
            ("photo_010.gif", "gif", TEST_CONTENT_MAX_SIZE),
            ("photo_011.webp", "webp", TEST_CONTENT_MAX_SIZE),
            ("photo_012.raw", "raw", TEST_CONTENT_MAX_SIZE),
            ("photo_013.dng", "raw", TEST_CONTENT_MAX_SIZE),
            ("photo_014.tiff", "tiff", TEST_CONTENT_MAX_SIZE),
            ("photo_015.tif", "tiff", TEST_CONTENT_MAX_SIZE),
            ("photo_016.bmp", "bmp", TEST_CONTENT_MAX_SIZE),
            ("video_001.mov", "mov", TEST_CONTENT_MAX_SIZE),
            ("video_002.MOV", "mov", 50 * 1024 * 1024),
            ("video_003.mp4", "mp4", 30 * 1024 * 1024),
            ("video_004.MP4", "mp4", 30 * 1024 * 1024),
            ("video_005.m4v", "mp4", 20 * 1024 * 1024),
            ("video_006.avi", "avi", 40 * 1024 * 1024),
            ("video_007.3gp", "3gp", 5 * 1024 * 1024),
        ];

        let mut total_size = 0u64;
        for (idx, (name, ext, size)) in file_types.iter().enumerate() {
            let file_id = format!("mixed_{}", idx);
            let content =
                MockDataGenerator::generate_for_extension_with_seed(ext, *size, idx as u64);
            fs.add_object(MockObject::file(&file_id, "100apple", name, content));
            total_size += *size as u64;
        }

        TestScenario::new(
            "mixed_file_types",
            "All supported photo and video formats",
            device,
            fs,
            ExpectedResults {
                files_to_extract: file_types.len(),
                folders: 3,
                total_size,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "formats", "comprehensive"])
    }

    /// Scenario: Files with unicode names
    pub fn unicode_filenames() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#unicode_files",
            "International iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        let unicode_names = vec![
            "写真_001.jpg",
            "фото_002.jpg",
            "φωτογραφία_003.jpg",
            "תמונה_004.jpg",
            "صورة_005.jpg",
            "사진_006.jpg",
            "ภาพถ่าย_007.jpg",
            "🌸_spring_008.jpg",
            "café_été_009.jpg",
            "naïve_010.jpg",
        ];

        // Use small content size for memory efficiency
        use crate::testdb::generator::TEST_JPEG_SIZE;
        for (idx, name) in unicode_names.iter().enumerate() {
            let file_id = format!("unicode_{}", idx);
            let content = MockDataGenerator::generate_jpeg_with_seed(TEST_JPEG_SIZE, idx as u64);
            fs.add_object(MockObject::file(&file_id, "100apple", name, content));
        }

        TestScenario::new(
            "unicode_filenames",
            "Files with various unicode characters in names",
            device,
            fs,
            ExpectedResults {
                files_to_extract: unicode_names.len(),
                folders: 3,
                total_size: unicode_names.len() as u64 * TEST_JPEG_SIZE as u64,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "unicode", "edge-case"])
    }

    /// Scenario: Large video files
    pub fn large_files() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#large_files",
            "Video Creator's iPhone",
            "Apple Inc.",
            "iPhone 15 Pro Max",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // Note: We don't actually allocate 2GB, just set the size
        let large_sizes = vec![
            ("4K_video_001.mov", 2 * 1024 * 1024 * 1024u64), // 2GB
            ("4K_video_002.mov", 1536 * 1024 * 1024u64),     // 1.5GB
            ("4K_video_003.mp4", 1024 * 1024 * 1024u64),     // 1GB
            ("burst_001.heic", 500 * 1024 * 1024u64),        // 500MB
        ];

        let mut total_size = 0u64;
        for (idx, (name, size)) in large_sizes.iter().enumerate() {
            let file_id = format!("large_{}", idx);
            // Just create a small placeholder - the size field will reflect the "real" size
            let content = MockDataGenerator::generate_mov_with_seed(1024, idx as u64);
            let mut obj = MockObject::file(&file_id, "100apple", name, content);
            obj.object.size = *size; // Override with large size
            fs.add_object(obj);
            total_size += *size;
        }

        TestScenario::new(
            "large_files",
            "Large video files (2GB+)",
            device,
            fs,
            ExpectedResults {
                files_to_extract: large_sizes.len(),
                folders: 3,
                total_size,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "large-files", "edge-case"])
    }

    /// Scenario: Problematic file attributes
    pub fn problematic_files() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#problematic",
            "Problematic iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // Zero-byte file
        fs.add_object(MockObject::file(
            "zero_byte",
            "100apple",
            "zero_byte.jpg",
            vec![],
        ));

        // File with no extension
        let content1 = MockDataGenerator::generate_jpeg_with_seed(1024, 1);
        fs.add_object(MockObject::file(
            "no_ext",
            "100apple",
            "no_extension",
            content1,
        ));

        // File with multiple extensions
        let content2 = MockDataGenerator::generate_jpeg_with_seed(1024, 2);
        fs.add_object(MockObject::file(
            "multi_ext",
            "100apple",
            "photo.backup.old.jpg",
            content2,
        ));

        // File with very long name
        let content3 = MockDataGenerator::generate_jpeg_with_seed(1024, 3);
        let long_name = format!("{}.jpg", "a".repeat(200));
        fs.add_object(MockObject::file(
            "long_name",
            "100apple",
            &long_name,
            content3,
        ));

        // File with spaces and special chars
        let content4 = MockDataGenerator::generate_jpeg_with_seed(1024, 4);
        fs.add_object(MockObject::file(
            "special_chars",
            "100apple",
            "photo (1) [copy] {backup}.jpg",
            content4,
        ));

        // File starting with dot (hidden on Unix)
        let content5 = MockDataGenerator::generate_jpeg_with_seed(1024, 5);
        fs.add_object(MockObject::file(
            "hidden_file",
            "100apple",
            ".hidden_photo.jpg",
            content5,
        ));

        // File with only whitespace (except extension)
        let content6 = MockDataGenerator::generate_jpeg_with_seed(1024, 6);
        fs.add_object(MockObject::file(
            "whitespace",
            "100apple",
            "   .jpg",
            content6,
        ));

        TestScenario::new(
            "problematic_files",
            "Files with unusual names and attributes",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 4, // Only files with valid media extensions
                folders: 3,
                total_size: 4 * 1024,
                should_succeed: true,
                errors: 0,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "edge-case", "problematic"])
    }

    // =========================================================================
    // ERROR CONDITION SCENARIOS
    // =========================================================================

    /// Scenario: Device is locked
    pub fn device_locked() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#locked_device",
            "Locked iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let fs = MockFileSystem::with_config(MockDeviceConfig::locked());

        TestScenario::new(
            "device_locked",
            "Device is locked and requires unlock/trust",
            device,
            fs,
            ExpectedResults {
                should_succeed: false,
                expected_error: Some("AccessDenied".to_string()),
                ..Default::default()
            },
        )
        .with_tags(vec!["error", "locked", "access-denied"])
    }

    /// Scenario: Device disconnects mid-transfer
    pub fn disconnect_mid_transfer() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#disconnect_device",
            "Unstable iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::with_config(MockDeviceConfig::disconnect_after(5));
        Self::add_standard_dcim_structure(&mut fs, 20, 2);

        TestScenario::new(
            "disconnect_mid_transfer",
            "Device disconnects after 5 file reads",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 5,
                folders: 4,
                should_succeed: false,
                expected_error: Some("disconnected".to_string()),
                errors: 1,
                ..Default::default()
            },
        )
        .with_tags(vec!["error", "disconnect", "resume"])
    }

    /// Scenario: Specific files fail to read
    pub fn file_read_errors() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#read_errors",
            "Corrupted iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let error_objects = vec![
            "img_000003".to_string(),
            "img_000007".to_string(),
            "img_000012".to_string(),
        ];

        let mut fs = MockFileSystem::with_config(
            MockDeviceConfig::default().with_read_errors(error_objects),
        );
        Self::add_standard_dcim_structure(&mut fs, 15, 2);

        TestScenario::new(
            "file_read_errors",
            "Some files fail to read (corrupted sectors)",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 12,
                folders: 4,
                should_succeed: true,
                errors: 3,
                ..Default::default()
            },
        )
        .with_tags(vec!["error", "corruption", "partial-failure"])
    }

    /// Scenario: Slow transfer speed
    pub fn slow_transfer() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#slow_device",
            "Slow iPhone",
            "Apple Inc.",
            "iPhone 6",
        );

        let mut fs = MockFileSystem::with_config(MockDeviceConfig::slow_transfer(1)); // 1ms per KB
        Self::add_standard_dcim_structure(&mut fs, 10, 2);

        TestScenario::new(
            "slow_transfer",
            "Simulated slow USB 2.0 connection",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 10,
                folders: 4,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["error", "slow", "performance"])
    }

    /// Scenario: Random connection failures
    pub fn flaky_connection() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#flaky_device",
            "Flaky iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::with_config(MockDeviceConfig::flaky(10)); // 10% failure rate
        Self::add_standard_dcim_structure(&mut fs, 50, 5);

        TestScenario::new(
            "flaky_connection",
            "Random 10% failure rate on file reads",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 45, // Approximately 90%
                folders: 7,
                should_succeed: true,
                errors: 5, // Approximately 10%
                ..Default::default()
            },
        )
        .with_tags(vec!["error", "flaky", "random-failure"])
    }

    // =========================================================================
    // DUPLICATE DETECTION SCENARIOS
    // =========================================================================

    /// Scenario: Exact duplicate files
    pub fn exact_duplicates() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#duplicates",
            "Duplicate iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));
        fs.add_object(MockObject::folder("101apple", "dcim", "101APPLE"));

        // Create some unique files
        for i in 0..5 {
            let file_id = format!("unique_{}", i);
            let file_name = format!("IMG_{:04}.JPG", i);
            let content = MockDataGenerator::generate_jpeg_with_seed(1024 * 1024, i as u64);
            fs.add_object(MockObject::file(&file_id, "100apple", &file_name, content));
        }

        // Create exact duplicates in another folder
        for i in 0..5 {
            let file_id = format!("dup_{}", i);
            let file_name = format!("IMG_{:04}.JPG", i); // Same name
            let content = MockDataGenerator::generate_jpeg_with_seed(1024 * 1024, i as u64); // Same content
            fs.add_object(MockObject::file(&file_id, "101apple", &file_name, content));
        }

        TestScenario::new(
            "exact_duplicates",
            "Same files in multiple folders",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 10, // Without dedup: 10, With dedup: 5
                folders: 4,
                duplicates: 5,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["duplicate", "exact-match"])
    }

    /// Scenario: Same content, different names
    pub fn renamed_duplicates() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#renamed_dups",
            "Renamed Duplicates iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // Create original files
        for i in 0..5 {
            let file_id = format!("orig_{}", i);
            let file_name = format!("IMG_{:04}.JPG", i);
            let content = MockDataGenerator::generate_jpeg_with_seed(1024 * 1024, i as u64);
            fs.add_object(MockObject::file(&file_id, "100apple", &file_name, content));
        }

        // Create renamed duplicates (same content, different names)
        for i in 0..5 {
            let file_id = format!("renamed_{}", i);
            let file_name = format!("photo_backup_{:04}.jpg", i); // Different name
            let content = MockDataGenerator::generate_jpeg_with_seed(1024 * 1024, i as u64); // Same content
            fs.add_object(MockObject::file(&file_id, "100apple", &file_name, content));
        }

        TestScenario::new(
            "renamed_duplicates",
            "Same content with different filenames",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 10,
                folders: 3,
                duplicates: 5,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["duplicate", "renamed"])
    }

    /// Scenario: No duplicates
    pub fn no_duplicates() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#no_dups",
            "Unique Files iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // All unique files with different seeds
        for i in 0..20 {
            let file_id = format!("unique_{}", i);
            let file_name = format!("IMG_{:04}.JPG", i);
            let content =
                MockDataGenerator::generate_jpeg_with_seed(1024 * 1024, (i * 1000) as u64);
            fs.add_object(MockObject::file(&file_id, "100apple", &file_name, content));
        }

        TestScenario::new(
            "no_duplicates",
            "All unique files",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 20,
                folders: 3,
                duplicates: 0,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["duplicate", "no-duplicates"])
    }

    // =========================================================================
    // TRACKING/RESUME SCENARIOS
    // =========================================================================

    /// Scenario: First time extraction (no tracking file)
    pub fn fresh_extraction() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#fresh_extract",
            "New iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 25, 3);

        TestScenario::new(
            "fresh_extraction",
            "First extraction with no prior tracking",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 25,
                folders: 5,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["tracking", "fresh", "first-run"])
    }

    /// Scenario: Resume after interruption
    pub fn resume_extraction() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#resume_extract",
            "Resume iPhone",
            "Apple Inc.",
            "iPhone 15",
        );

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 30, 3);

        TestScenario::new(
            "resume_extraction",
            "Resume extraction after interruption",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 15, // Only remaining files
                folders: 5,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["tracking", "resume", "interrupted"])
    }

    // =========================================================================
    // PROFILE SCENARIOS
    // =========================================================================

    /// Scenario: New device requiring profile creation
    pub fn new_device_profile() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#new_profile_device",
            "Brand New iPhone",
            "Apple Inc.",
            "iPhone 15 Pro",
        );

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 20, 2);

        TestScenario::new(
            "new_device_profile",
            "New device requiring profile creation",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 20,
                folders: 4,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["profile", "new-device"])
    }

    /// Scenario: Known device with existing profile
    pub fn known_device_profile() -> TestScenario {
        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#known_profile_device",
            "Known iPhone",
            "Apple Inc.",
            "iPhone 14",
        );

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 35, 4);

        TestScenario::new(
            "known_device_profile",
            "Known device with existing profile",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 35,
                folders: 6,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["profile", "known-device"])
    }

    // =========================================================================
    // STRESS TEST SCENARIOS
    // =========================================================================

    /// Scenario: Realistic iPhone with thousands of photos
    pub fn realistic_iphone() -> TestScenario {
        use crate::testdb::generator::{TEST_HEIC_SIZE, TEST_JPEG_SIZE, TEST_VIDEO_SIZE};

        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#realistic_device",
            "Real World iPhone",
            "Apple Inc.",
            "iPhone 15 Pro",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));

        // Create 25 folders with 100 files each = 2500 files
        // Using lightweight content to prevent memory issues during testing
        let mut file_counter = 1;
        let mut total_size = 0u64;

        for folder_idx in 0..25 {
            let folder_num = 100 + folder_idx;
            let folder_id = format!("{}apple", folder_num);
            let folder_name = format!("{}APPLE", folder_num);

            fs.add_object(MockObject::folder(&folder_id, "dcim", &folder_name));

            for _ in 0..100 {
                let file_id = format!("img_{:06}", file_counter);

                // Mix of photos and videos - use small content sizes for memory efficiency
                // The reported size can differ from actual content size
                let (file_name, reported_size, content_size) = if file_counter % 10 == 0 {
                    // Every 10th file is a video
                    (
                        format!("IMG_{:04}.MOV", file_counter),
                        50 * 1024 * 1024u64, // Reported as 50MB
                        TEST_VIDEO_SIZE,     // Actual content is small
                    )
                } else if file_counter % 5 == 0 {
                    // Every 5th (non-video) is HEIC
                    (
                        format!("IMG_{:04}.HEIC", file_counter),
                        2 * 1024 * 1024u64, // Reported as 2MB
                        TEST_HEIC_SIZE,     // Actual content is small
                    )
                } else {
                    // Regular JPEG
                    (
                        format!("IMG_{:04}.JPG", file_counter),
                        3 * 1024 * 1024u64, // Reported as 3MB
                        TEST_JPEG_SIZE,     // Actual content is small
                    )
                };

                let ext = file_name.split('.').last().unwrap_or("jpg").to_lowercase();
                let content = MockDataGenerator::generate_for_extension_with_seed(
                    &ext,
                    content_size,
                    file_counter as u64,
                );
                let mut obj = MockObject::file(&file_id, &folder_id, &file_name, content);
                obj.object.size = reported_size; // Set reported size (different from content)
                fs.add_object(obj);

                total_size += reported_size;
                file_counter += 1;
            }
        }

        TestScenario::new(
            "realistic_iphone",
            "Realistic iPhone with 2500+ files",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 2500,
                folders: 27,
                total_size,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["stress-test", "realistic", "performance"])
    }

    /// Scenario: Maximum stress test
    ///
    /// Uses minimal content size per file to prevent memory exhaustion.
    /// With 10,000 files at 512 bytes each = ~5MB total (vs 640MB before)
    pub fn stress_test() -> TestScenario {
        use crate::testdb::generator::TEST_STRESS_SIZE;

        let device = DeviceInfo::new(
            "\\\\?\\usb#vid_05ac&pid_12a8#stress_device",
            "Stress Test iPhone",
            "Apple Inc.",
            "iPhone 15 Pro Max",
        );

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));

        // Create 100 folders with 100 files each = 10,000 files
        // Using minimal content size to prevent memory issues
        let content_size = TEST_STRESS_SIZE; // 512 bytes per file
        let mut file_counter = 1;

        for folder_idx in 0..100 {
            let folder_num = 100 + folder_idx;
            let folder_id = format!("{}apple", folder_num);
            let folder_name = format!("{}APPLE", folder_num);

            fs.add_object(MockObject::folder(&folder_id, "dcim", &folder_name));

            for _ in 0..100 {
                let file_id = format!("img_{:06}", file_counter);
                let file_name = format!("IMG_{:04}.JPG", file_counter);

                // Minimal content for stress testing - just valid JPEG headers
                let content =
                    MockDataGenerator::generate_jpeg_with_seed(content_size, file_counter as u64);
                fs.add_object(MockObject::file(&file_id, &folder_id, &file_name, content));

                file_counter += 1;
            }
        }

        TestScenario::new(
            "stress_test",
            "Maximum stress test with 10,000 files",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 10000,
                folders: 102,
                total_size: 10000 * content_size as u64,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["stress-test", "maximum", "performance"])
    }

    // =========================================================================
    // HELPER METHODS
    // =========================================================================

    /// Add standard iOS DCIM folder structure
    ///
    /// Uses small content sizes for memory efficiency during testing.
    fn add_standard_dcim_structure(
        fs: &mut MockFileSystem,
        total_files: usize,
        num_folders: usize,
    ) {
        use crate::testdb::generator::TEST_JPEG_SIZE;

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));

        // Use ceiling division to ensure we create at least total_files
        // (avoids bug where 25/3=8, 8*3=24 files instead of 25)
        let num_folders = num_folders.max(1);
        let files_per_folder = (total_files + num_folders - 1) / num_folders;
        let mut file_counter = 1;

        for folder_idx in 0..num_folders {
            let folder_num = 100 + folder_idx;
            let folder_id = format!("{}apple", folder_num);
            let folder_name = format!("{}APPLE", folder_num);

            fs.add_object(MockObject::folder(&folder_id, "dcim", &folder_name));

            for _ in 0..files_per_folder {
                if file_counter > total_files {
                    break;
                }

                let file_id = format!("img_{:06}", file_counter);
                let file_name = format!("IMG_{:04}.JPG", file_counter);
                // Use small content size (2KB) for memory efficiency
                let content =
                    MockDataGenerator::generate_jpeg_with_seed(TEST_JPEG_SIZE, file_counter as u64);

                fs.add_object(MockObject::file(&file_id, &folder_id, &file_name, content));
                file_counter += 1;
            }
        }
    }

    /// Add legacy iOS folder structure (for older devices)
    ///
    /// Uses small content sizes for memory efficiency during testing.
    fn add_legacy_dcim_structure(fs: &mut MockFileSystem, total_files: usize, num_folders: usize) {
        use crate::testdb::generator::TEST_JPEG_SIZE;

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));

        // Use ceiling division to ensure we create at least total_files
        let num_folders = num_folders.max(1);
        let files_per_folder = (total_files + num_folders - 1) / num_folders;
        let mut file_counter = 1;

        for folder_idx in 0..num_folders {
            let folder_num = 100 + folder_idx;
            // Legacy iPhones used just numbers
            let folder_id = format!("{}", folder_num);
            let folder_name = format!("{}", folder_num);

            fs.add_object(MockObject::folder(&folder_id, "dcim", &folder_name));

            for _ in 0..files_per_folder {
                if file_counter > total_files {
                    break;
                }

                let file_id = format!("img_{:06}", file_counter);
                // Legacy naming convention
                let file_name = format!("IMG_{:04}.JPG", file_counter);
                // Use small content size (2KB) for memory efficiency
                let content =
                    MockDataGenerator::generate_jpeg_with_seed(TEST_JPEG_SIZE, file_counter as u64);

                fs.add_object(MockObject::file(&file_id, &folder_id, &file_name, content));
                file_counter += 1;
            }
        }
    }

    // =========================================================================
    // SCENARIO COLLECTIONS
    // =========================================================================

    /// Get all available scenarios
    pub fn all_scenarios() -> Vec<TestScenario> {
        let mut scenarios = vec![
            // Device detection
            Self::no_devices(),
            Self::single_iphone(),
            Self::ipad_device(),
            Self::mixed_devices(),
            Self::old_iphone(),
            Self::unicode_device_name(),
            // File structure
            Self::empty_device(),
            Self::deeply_nested(),
            Self::many_files_single_folder(),
            Self::mixed_file_types(),
            Self::unicode_filenames(),
            Self::large_files(),
            Self::problematic_files(),
            // Error conditions
            Self::device_locked(),
            Self::disconnect_mid_transfer(),
            Self::file_read_errors(),
            Self::slow_transfer(),
            Self::flaky_connection(),
            // Duplicates
            Self::exact_duplicates(),
            Self::renamed_duplicates(),
            Self::no_duplicates(),
            // Tracking
            Self::fresh_extraction(),
            Self::resume_extraction(),
            // Profiles
            Self::new_device_profile(),
            Self::known_device_profile(),
            // Stress tests
            Self::realistic_iphone(),
            Self::stress_test(),
        ];

        // Add multiple device scenarios
        scenarios.extend(Self::multiple_iphones());

        scenarios
    }

    /// Get scenarios filtered by tag
    pub fn scenarios_by_tag(tag: &str) -> Vec<TestScenario> {
        Self::all_scenarios()
            .into_iter()
            .filter(|s| s.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Get quick test scenarios (fast to run)
    pub fn quick_scenarios() -> Vec<TestScenario> {
        vec![
            Self::single_iphone(),
            Self::empty_device(),
            Self::mixed_file_types(),
            Self::device_locked(),
            Self::exact_duplicates(),
            Self::fresh_extraction(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_scenarios_load() {
        let scenarios = ScenarioLibrary::all_scenarios();
        assert!(!scenarios.is_empty());
        assert!(scenarios.len() >= 20);
    }

    #[test]
    fn test_scenario_by_tag() {
        let device_scenarios = ScenarioLibrary::scenarios_by_tag("device");
        assert!(!device_scenarios.is_empty());

        for scenario in device_scenarios {
            assert!(scenario.tags.contains(&"device".to_string()));
        }
    }

    #[test]
    fn test_quick_scenarios() {
        let quick = ScenarioLibrary::quick_scenarios();
        assert!(!quick.is_empty());
        assert!(quick.len() <= 10);
    }

    #[test]
    fn test_scenario_has_required_fields() {
        for scenario in ScenarioLibrary::all_scenarios() {
            assert!(
                !scenario.name.is_empty(),
                "Scenario name should not be empty"
            );
            assert!(
                !scenario.description.is_empty(),
                "Scenario description should not be empty"
            );
            assert!(
                !scenario.tags.is_empty(),
                "Scenario should have at least one tag"
            );
        }
    }

    #[test]
    fn test_expected_results_consistency() {
        for scenario in ScenarioLibrary::all_scenarios() {
            if scenario.expected.should_succeed {
                // If should succeed, files_to_extract should match file count (roughly)
                let actual_files = scenario.file_system.file_count();
                // Allow some flexibility for problematic files scenarios
                if !scenario.tags.contains(&"problematic".to_string()) {
                    assert!(
                        actual_files >= scenario.expected.files_to_extract,
                        "Scenario {} has {} files but expects {}",
                        scenario.name,
                        actual_files,
                        scenario.expected.files_to_extract
                    );
                }
            }
        }
    }

    #[test]
    fn test_single_iphone_scenario() {
        let scenario = ScenarioLibrary::single_iphone();

        assert_eq!(scenario.name, "single_iphone");
        assert!(scenario.expected.should_succeed);
        assert!(scenario.expected.files_to_extract > 0);
        assert!(scenario.file_system.file_count() > 0);
        assert!(scenario.device_info.friendly_name.contains("iPhone"));
    }

    #[test]
    fn test_device_locked_scenario() {
        let scenario = ScenarioLibrary::device_locked();

        assert_eq!(scenario.name, "device_locked");
        assert!(!scenario.expected.should_succeed);
        assert!(scenario.expected.expected_error.is_some());
    }
}

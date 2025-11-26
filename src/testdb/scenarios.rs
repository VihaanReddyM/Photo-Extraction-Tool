//! Predefined test scenarios for comprehensive testing
//!
//! This module provides ready-to-use test scenarios that cover various
//! use cases, edge cases, and error conditions for the photo extraction tool.

use super::generator::MockDataGenerator;
use super::mock_device::{MockDeviceConfig, MockDeviceInfo, MockFileSystem, MockObject};

/// A complete test scenario with device info and file system
#[derive(Debug, Clone)]
pub struct TestScenario {
    /// Scenario name for identification
    pub name: String,
    /// Description of what this scenario tests
    pub description: String,
    /// Mock device information
    pub device_info: MockDeviceInfo,
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
        device_info: MockDeviceInfo,
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
            MockDeviceInfo {
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
        let device = MockDeviceInfo {
            device_id: "\\\\?\\usb#vid_05ac&pid_12a8#serial123456".to_string(),
            friendly_name: "John's iPhone".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 15 Pro".to_string(),
        };

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
                let device = MockDeviceInfo {
                    device_id: "\\\\?\\usb#vid_05ac&pid_12a8#iphone1".to_string(),
                    friendly_name: "Work iPhone".to_string(),
                    manufacturer: "Apple Inc.".to_string(),
                    model: "iPhone 14".to_string(),
                };
                let mut fs = MockFileSystem::new();
                Self::add_standard_dcim_structure(&mut fs, 30, 3);
                TestScenario::new(
                    "multiple_iphones_1",
                    "First of multiple iPhones",
                    device,
                    fs,
                    ExpectedResults {
                        files_to_extract: 30,
                        folders: 5,
                        should_succeed: true,
                        ..Default::default()
                    },
                )
            },
            {
                let device = MockDeviceInfo {
                    device_id: "\\\\?\\usb#vid_05ac&pid_12a8#iphone2".to_string(),
                    friendly_name: "Personal iPhone".to_string(),
                    manufacturer: "Apple Inc.".to_string(),
                    model: "iPhone 15 Pro Max".to_string(),
                };
                let mut fs = MockFileSystem::new();
                Self::add_standard_dcim_structure(&mut fs, 100, 10);
                TestScenario::new(
                    "multiple_iphones_2",
                    "Second of multiple iPhones",
                    device,
                    fs,
                    ExpectedResults {
                        files_to_extract: 100,
                        folders: 12,
                        should_succeed: true,
                        ..Default::default()
                    },
                )
            },
        ]
    }

    /// Scenario: iPad connected
    pub fn ipad_device() -> TestScenario {
        let device = MockDeviceInfo {
            device_id: "\\\\?\\usb#vid_05ac&pid_12ab#ipad_serial".to_string(),
            friendly_name: "My iPad Pro".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPad Pro 12.9-inch (6th generation)".to_string(),
        };

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 200, 20);

        TestScenario::new(
            "ipad_device",
            "iPad Pro with large photo library",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 200,
                folders: 22,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["device", "ipad", "basic"])
    }

    /// Scenario: Mix of Apple and non-Apple devices
    pub fn mixed_devices() -> Vec<TestScenario> {
        vec![
            {
                let device = MockDeviceInfo {
                    device_id: "\\\\?\\usb#vid_05ac&pid_12a8#iphone".to_string(),
                    friendly_name: "iPhone".to_string(),
                    manufacturer: "Apple Inc.".to_string(),
                    model: "iPhone 13".to_string(),
                };
                let mut fs = MockFileSystem::new();
                Self::add_standard_dcim_structure(&mut fs, 25, 2);
                TestScenario::new(
                    "mixed_apple",
                    "Apple device in mixed scenario",
                    device,
                    fs,
                    ExpectedResults {
                        files_to_extract: 25,
                        should_succeed: true,
                        ..Default::default()
                    },
                )
            },
            {
                let device = MockDeviceInfo {
                    device_id: "\\\\?\\usb#vid_04e8&pid_6860#samsung".to_string(),
                    friendly_name: "Samsung Galaxy".to_string(),
                    manufacturer: "Samsung".to_string(),
                    model: "Galaxy S23".to_string(),
                };
                let fs = MockFileSystem::new();
                TestScenario::new(
                    "mixed_android",
                    "Non-Apple device (should be filtered)",
                    device,
                    fs,
                    ExpectedResults {
                        should_succeed: true, // Not selected when filtering for Apple
                        ..Default::default()
                    },
                )
            },
        ]
    }

    /// Scenario: Old iPhone model
    pub fn old_iphone() -> TestScenario {
        let device = MockDeviceInfo {
            device_id: "\\\\?\\usb#vid_05ac&pid_1292#old_iphone".to_string(),
            friendly_name: "Old iPhone 6s".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 6s".to_string(),
        };

        let mut fs = MockFileSystem::new();
        // Old iPhones typically have JPG only
        Self::add_legacy_dcim_structure(&mut fs, 100, 10);

        TestScenario::new(
            "old_iphone",
            "Legacy iPhone 6s with JPG-only photos",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 100,
                folders: 12,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["device", "iphone", "legacy"])
    }

    /// Scenario: Device with unicode/special characters in name
    pub fn unicode_device_name() -> TestScenario {
        let device = MockDeviceInfo {
            device_id: "\\\\?\\usb#vid_05ac&pid_12a8#unicode".to_string(),
            friendly_name: "田中's iPhone 📱✨".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 15".to_string(),
        };

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 10, 1);

        TestScenario::new(
            "unicode_device_name",
            "Device with unicode characters and emoji in name",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 10,
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
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::new();

        // Just the folder structure, no files
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
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "empty", "edge-case"])
    }

    /// Scenario: Deeply nested folder structure
    pub fn deeply_nested() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::new();

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));

        // Create 15 levels of nesting
        let mut parent_id = "dcim".to_string();
        for i in 0..15 {
            let folder_id = format!("level_{}", i);
            let folder_name = format!("Level_{}", i);
            fs.add_object(MockObject::folder(&folder_id, &parent_id, &folder_name));
            parent_id = folder_id;
        }

        // Add some files at the deepest level
        for i in 0..5 {
            let file_id = format!("deep_file_{}", i);
            let file_name = format!("IMG_{:04}.JPG", i);
            let content = MockDataGenerator::generate_jpeg_header(1024);
            fs.add_object(MockObject::file(
                &file_id,
                &parent_id,
                &file_name,
                content.len() as u64,
                content,
            ));
        }

        TestScenario::new(
            "deeply_nested",
            "15 levels of nested folders with files at deepest level",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 5,
                folders: 17, // Internal Storage + DCIM + 15 levels
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "nested", "edge-case"])
    }

    /// Scenario: Thousands of files in single folder
    pub fn many_files_single_folder() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::new();

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // Add 5000 files
        for i in 0..5000 {
            let file_id = format!("file_{}", i);
            let file_name = format!("IMG_{:04}.JPG", i);
            let content = MockDataGenerator::generate_jpeg_header(512); // Small files
            fs.add_object(MockObject::file(
                &file_id,
                "100apple",
                &file_name,
                content.len() as u64,
                content,
            ));
        }

        TestScenario::new(
            "many_files_single_folder",
            "5000 files in a single APPLE folder",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 5000,
                folders: 3,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "performance", "stress-test"])
    }

    /// Scenario: Various file extensions
    pub fn mixed_file_types() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::new();

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // Add various file types
        let file_types = vec![
            (
                "IMG_0001.JPG",
                MockDataGenerator::generate_jpeg_header(1024),
            ),
            (
                "IMG_0002.JPEG",
                MockDataGenerator::generate_jpeg_header(1024),
            ),
            (
                "IMG_0003.jpg",
                MockDataGenerator::generate_jpeg_header(1024),
            ), // lowercase
            (
                "IMG_0004.HEIC",
                MockDataGenerator::generate_heic_header(2048),
            ),
            (
                "IMG_0005.heif",
                MockDataGenerator::generate_heic_header(2048),
            ),
            ("IMG_0006.PNG", MockDataGenerator::generate_png_header(1024)),
            ("IMG_0007.png", MockDataGenerator::generate_png_header(1024)),
            ("IMG_0008.GIF", MockDataGenerator::generate_gif_header(512)),
            (
                "IMG_0009.WEBP",
                MockDataGenerator::generate_webp_header(1024),
            ),
            ("IMG_0010.DNG", MockDataGenerator::generate_raw_header(4096)),
            ("IMG_0011.RAW", MockDataGenerator::generate_raw_header(4096)),
            (
                "IMG_0012.TIFF",
                MockDataGenerator::generate_tiff_header(2048),
            ),
            (
                "IMG_0013.TIF",
                MockDataGenerator::generate_tiff_header(2048),
            ),
            ("IMG_0014.BMP", MockDataGenerator::generate_bmp_header(1024)),
            (
                "VID_0001.MOV",
                MockDataGenerator::generate_mov_header(10240),
            ),
            (
                "VID_0002.MP4",
                MockDataGenerator::generate_mp4_header(10240),
            ),
            (
                "VID_0003.M4V",
                MockDataGenerator::generate_mp4_header(10240),
            ),
            (
                "VID_0004.AVI",
                MockDataGenerator::generate_avi_header(10240),
            ),
            ("VID_0005.3GP", MockDataGenerator::generate_3gp_header(5120)),
        ];

        for (i, (name, content)) in file_types.iter().enumerate() {
            fs.add_object(MockObject::file(
                &format!("file_{}", i),
                "100apple",
                name,
                content.len() as u64,
                content.clone(),
            ));
        }

        TestScenario::new(
            "mixed_file_types",
            "All supported photo and video file types",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 19,
                folders: 3,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "file-types", "comprehensive"])
    }

    /// Scenario: Files with unicode/special characters in names
    pub fn unicode_filenames() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::new();

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        let special_names = vec![
            "Photo 日本語.JPG",
            "Foto España.JPG",
            "照片 中文.HEIC",
            "Фото Россия.PNG",
            "φωτογραφία.JPG",
            "תמונה עברית.HEIC",
            "صورة عربية.JPG",
            "Photo émojis 🎉🎊.JPG",
            "File with spaces.JPG",
            "File-with-dashes.JPG",
            "File_with_underscores.JPG",
            "File.multiple.dots.JPG",
            "UPPERCASE.JPG",
            "lowercase.jpg",
            "MixedCase.Jpg",
        ];

        for (i, name) in special_names.iter().enumerate() {
            let content = MockDataGenerator::generate_jpeg_header(1024);
            fs.add_object(MockObject::file(
                &format!("unicode_{}", i),
                "100apple",
                name,
                content.len() as u64,
                content,
            ));
        }

        TestScenario::new(
            "unicode_filenames",
            "Files with unicode characters, emoji, and special naming patterns",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 15,
                folders: 3,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "unicode", "edge-case"])
    }

    /// Scenario: Very large files (videos)
    pub fn large_files() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::new();

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // Add some large video files (simulated sizes)
        let large_files = vec![
            ("4K_Video_1.MOV", 2 * 1024 * 1024 * 1024u64), // 2GB
            ("4K_Video_2.MOV", 1500 * 1024 * 1024u64),     // 1.5GB
            ("Long_Recording.MOV", 3 * 1024 * 1024 * 1024u64), // 3GB
            ("Timelapse.MOV", 500 * 1024 * 1024u64),       // 500MB
        ];

        for (i, (name, size)) in large_files.iter().enumerate() {
            // For large files, we just store a header but report the full size
            let header = MockDataGenerator::generate_mov_header(1024);
            let mut obj =
                MockObject::file(&format!("large_{}", i), "100apple", name, *size, header);
            obj.size = *size; // Override with reported size
            fs.add_object(obj);
        }

        TestScenario::new(
            "large_files",
            "Large video files (2GB+)",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 4,
                folders: 3,
                total_size: 7 * 1024 * 1024 * 1024 + 500 * 1024 * 1024, // 7.5GB
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "large-files", "performance"])
    }

    /// Scenario: Zero-byte and corrupt files
    pub fn problematic_files() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::new();

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // Zero-byte file
        fs.add_object(MockObject::file(
            "zero_byte",
            "100apple",
            "empty.JPG",
            0,
            vec![],
        ));

        // File with no extension
        fs.add_object(MockObject::file(
            "no_ext",
            "100apple",
            "IMG_0001",
            100,
            vec![0xFF; 100],
        ));

        // File with wrong extension (PNG data with JPG extension)
        let png_data = MockDataGenerator::generate_png_header(1024);
        fs.add_object(MockObject::file(
            "wrong_ext",
            "100apple",
            "wrong.JPG",
            png_data.len() as u64,
            png_data,
        ));

        // Very short filename
        fs.add_object(MockObject::file(
            "short_name",
            "100apple",
            "a.JPG",
            100,
            MockDataGenerator::generate_jpeg_header(100),
        ));

        // Valid file for comparison
        let valid = MockDataGenerator::generate_jpeg_header(1024);
        fs.add_object(MockObject::file(
            "valid",
            "100apple",
            "Valid_Image.JPG",
            valid.len() as u64,
            valid,
        ));

        TestScenario::new(
            "problematic_files",
            "Zero-byte, no extension, wrong extension, and other problematic files",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 5,
                folders: 3,
                errors: 0, // Tool should handle gracefully
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["structure", "edge-case", "error-handling"])
    }

    // =========================================================================
    // ERROR SCENARIOS
    // =========================================================================

    /// Scenario: Device is locked (access denied)
    pub fn device_locked() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::with_config(MockDeviceConfig {
            simulate_locked: true,
            ..Default::default()
        });

        Self::add_standard_dcim_structure(&mut fs, 10, 1);

        TestScenario::new(
            "device_locked",
            "Device is locked - should prompt for trust",
            device,
            fs,
            ExpectedResults {
                should_succeed: false,
                expected_error: Some("AccessDenied".to_string()),
                ..Default::default()
            },
        )
        .with_tags(vec!["error", "access", "user-interaction"])
    }

    /// Scenario: Device disconnects mid-transfer
    pub fn disconnect_mid_transfer() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::with_config(MockDeviceConfig {
            disconnect_after_reads: Some(5), // Disconnect after 5 files
            ..Default::default()
        });

        Self::add_standard_dcim_structure(&mut fs, 20, 2);

        TestScenario::new(
            "disconnect_mid_transfer",
            "Device disconnects after transferring 5 files",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 5, // Only 5 succeed
                errors: 1,           // Disconnect error
                should_succeed: false,
                expected_error: Some("DeviceError".to_string()),
                ..Default::default()
            },
        )
        .with_tags(vec!["error", "disconnect", "resume"])
    }

    /// Scenario: Specific files have read errors
    pub fn file_read_errors() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::with_config(MockDeviceConfig {
            read_error_objects: vec!["file_3".to_string(), "file_7".to_string()],
            ..Default::default()
        });

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        for i in 0..10 {
            let content = MockDataGenerator::generate_jpeg_header(1024);
            fs.add_object(MockObject::file(
                &format!("file_{}", i),
                "100apple",
                &format!("IMG_{:04}.JPG", i),
                content.len() as u64,
                content,
            ));
        }

        TestScenario::new(
            "file_read_errors",
            "Two specific files fail to read",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 8, // 10 - 2 errors
                errors: 2,
                should_succeed: true, // Continues despite errors
                ..Default::default()
            },
        )
        .with_tags(vec!["error", "partial-failure", "resilience"])
    }

    /// Scenario: Slow transfer (timeout testing)
    pub fn slow_transfer() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::with_config(MockDeviceConfig {
            transfer_delay_ms_per_kb: 10, // 10ms per KB = slow
            ..Default::default()
        });

        Self::add_standard_dcim_structure(&mut fs, 5, 1);

        TestScenario::new(
            "slow_transfer",
            "Simulates slow USB connection with 10ms/KB delay",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 5,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["performance", "slow", "timeout"])
    }

    /// Scenario: Random failures (flaky connection)
    pub fn flaky_connection() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::with_config(MockDeviceConfig {
            random_failure_rate: 10, // 10% failure rate
            ..Default::default()
        });

        Self::add_standard_dcim_structure(&mut fs, 50, 5);

        TestScenario::new(
            "flaky_connection",
            "10% random failure rate to simulate unstable connection",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 45, // ~90% success
                errors: 5,            // ~10% failures
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["error", "random", "resilience"])
    }

    // =========================================================================
    // DUPLICATE DETECTION SCENARIOS
    // =========================================================================

    /// Scenario: Exact duplicates (same file, same name)
    pub fn exact_duplicates() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::new();

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));
        fs.add_object(MockObject::folder("101apple", "dcim", "101APPLE"));

        // Same content in both folders
        let content = MockDataGenerator::generate_jpeg_with_seed(1024, 12345);
        fs.add_object(MockObject::file(
            "file_100_1",
            "100apple",
            "IMG_0001.JPG",
            content.len() as u64,
            content.clone(),
        ));
        fs.add_object(MockObject::file(
            "file_101_1",
            "101apple",
            "IMG_0001.JPG",
            content.len() as u64,
            content,
        ));

        // Unique files
        let unique1 = MockDataGenerator::generate_jpeg_with_seed(1024, 11111);
        let unique2 = MockDataGenerator::generate_jpeg_with_seed(1024, 22222);
        fs.add_object(MockObject::file(
            "file_100_2",
            "100apple",
            "IMG_0002.JPG",
            unique1.len() as u64,
            unique1,
        ));
        fs.add_object(MockObject::file(
            "file_101_2",
            "101apple",
            "IMG_0003.JPG",
            unique2.len() as u64,
            unique2,
        ));

        TestScenario::new(
            "exact_duplicates",
            "Same file exists in multiple folders",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 3, // 4 files, 1 duplicate
                duplicates: 1,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["duplicate", "exact-match"])
    }

    /// Scenario: Same content, different names
    pub fn renamed_duplicates() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::new();

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // Same content, different names
        let content = MockDataGenerator::generate_jpeg_with_seed(2048, 99999);
        fs.add_object(MockObject::file(
            "orig",
            "100apple",
            "IMG_0001.JPG",
            content.len() as u64,
            content.clone(),
        ));
        fs.add_object(MockObject::file(
            "renamed1",
            "100apple",
            "IMG_0001 (1).JPG",
            content.len() as u64,
            content.clone(),
        ));
        fs.add_object(MockObject::file(
            "renamed2",
            "100apple",
            "Copy of IMG_0001.JPG",
            content.len() as u64,
            content,
        ));

        TestScenario::new(
            "renamed_duplicates",
            "Same content but files have different names",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 1,
                duplicates: 2,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["duplicate", "renamed"])
    }

    /// Scenario: No duplicates at all
    pub fn no_duplicates() -> TestScenario {
        let device = MockDeviceInfo::default();
        let mut fs = MockFileSystem::new();

        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(MockObject::folder("100apple", "dcim", "100APPLE"));

        // All unique files with different seeds
        for i in 0..10 {
            let content = MockDataGenerator::generate_jpeg_with_seed(1024, i as u64);
            fs.add_object(MockObject::file(
                &format!("file_{}", i),
                "100apple",
                &format!("IMG_{:04}.JPG", i),
                content.len() as u64,
                content,
            ));
        }

        TestScenario::new(
            "no_duplicates",
            "All files are unique - no duplicates to detect",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 10,
                duplicates: 0,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["duplicate", "no-match"])
    }

    // =========================================================================
    // STATE/TRACKING SCENARIOS
    // =========================================================================

    /// Scenario: Fresh extraction (no previous state)
    pub fn fresh_extraction() -> TestScenario {
        let device = MockDeviceInfo {
            device_id: "fresh-device-001".to_string(),
            ..Default::default()
        };

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 25, 2);

        TestScenario::new(
            "fresh_extraction",
            "First time extraction from device - no tracking state",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 25,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["tracking", "fresh"])
    }

    /// Scenario: Resume interrupted extraction
    pub fn resume_extraction() -> TestScenario {
        let device = MockDeviceInfo {
            device_id: "resume-device-001".to_string(),
            ..Default::default()
        };

        let mut fs = MockFileSystem::new();
        // 50 total files, assume 20 already extracted
        Self::add_standard_dcim_structure(&mut fs, 50, 5);

        TestScenario::new(
            "resume_extraction",
            "Resume extraction - 20 of 50 files already extracted",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 30, // 50 - 20 already done
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["tracking", "resume"])
    }

    // =========================================================================
    // PROFILE SCENARIOS
    // =========================================================================

    /// Scenario: New device requiring profile creation
    pub fn new_device_profile() -> TestScenario {
        let device = MockDeviceInfo {
            device_id: "brand-new-device-no-profile".to_string(),
            friendly_name: "New iPhone".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 15".to_string(),
        };

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 10, 1);

        TestScenario::new(
            "new_device_profile",
            "Brand new device requiring profile creation",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 10,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["profile", "new-device"])
    }

    /// Scenario: Known device with existing profile
    pub fn known_device_profile() -> TestScenario {
        let device = MockDeviceInfo {
            device_id: "known-device-with-profile".to_string(),
            friendly_name: "John's iPhone".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 14 Pro".to_string(),
        };

        let mut fs = MockFileSystem::new();
        Self::add_standard_dcim_structure(&mut fs, 30, 3);

        TestScenario::new(
            "known_device_profile",
            "Known device with existing profile - should auto-select output folder",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 30,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["profile", "existing"])
    }

    // =========================================================================
    // COMPREHENSIVE/STRESS SCENARIOS
    // =========================================================================

    /// Scenario: Realistic iPhone photo library
    pub fn realistic_iphone() -> TestScenario {
        let device = MockDeviceInfo {
            device_id: "realistic-iphone-001".to_string(),
            friendly_name: "Sarah's iPhone 15 Pro".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 15 Pro".to_string(),
        };

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));

        let mut file_count = 0;

        // Create realistic folder distribution
        // 100APPLE through 120APPLE with varying file counts
        for folder_num in 100..121 {
            let folder_id = format!("{}apple", folder_num);
            let folder_name = format!("{}APPLE", folder_num);
            fs.add_object(MockObject::folder(&folder_id, "dcim", &folder_name));

            // Earlier folders have more files
            let files_in_folder = match folder_num {
                100..=105 => 200,
                106..=110 => 150,
                111..=115 => 100,
                _ => 50,
            };

            for i in 0..files_in_folder {
                let file_id = format!("{}_{}", folder_id, i);

                // Mix of file types
                let (name, content) = match i % 10 {
                    0..=5 => {
                        // 60% HEIC
                        let name = format!("IMG_{:04}.HEIC", file_count);
                        let content = MockDataGenerator::generate_heic_with_seed(
                            2 * 1024 * 1024,
                            file_count as u64,
                        );
                        (name, content)
                    }
                    6..=7 => {
                        // 20% JPG
                        let name = format!("IMG_{:04}.JPG", file_count);
                        let content = MockDataGenerator::generate_jpeg_with_seed(
                            1024 * 1024,
                            file_count as u64,
                        );
                        (name, content)
                    }
                    8 => {
                        // 10% MOV
                        let name = format!("IMG_{:04}.MOV", file_count);
                        let content = MockDataGenerator::generate_mov_with_seed(
                            50 * 1024 * 1024,
                            file_count as u64,
                        );
                        (name, content)
                    }
                    _ => {
                        // 10% PNG screenshots
                        let name = format!("IMG_{:04}.PNG", file_count);
                        let content = MockDataGenerator::generate_png_with_seed(
                            500 * 1024,
                            file_count as u64,
                        );
                        (name, content)
                    }
                };

                fs.add_object(MockObject::file(
                    &file_id,
                    &folder_id,
                    &name,
                    content.len() as u64,
                    content,
                ));
                file_count += 1;
            }
        }

        TestScenario::new(
            "realistic_iphone",
            "Realistic iPhone 15 Pro photo library with 2500+ files",
            device,
            fs,
            ExpectedResults {
                files_to_extract: file_count,
                folders: 23, // Internal Storage + DCIM + 21 APPLE folders
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["comprehensive", "realistic", "stress-test"])
    }

    /// Scenario: Maximum stress test
    pub fn stress_test() -> TestScenario {
        let device = MockDeviceInfo {
            device_id: "stress-test-device".to_string(),
            friendly_name: "Stress Test iPhone".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone".to_string(),
        };

        let mut fs = MockFileSystem::new();
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));

        let mut file_count = 0;

        // 50 folders with 200 files each = 10,000 files
        for folder_num in 100..150 {
            let folder_id = format!("{}apple", folder_num);
            let folder_name = format!("{}APPLE", folder_num);
            fs.add_object(MockObject::folder(&folder_id, "dcim", &folder_name));

            for i in 0..200 {
                let file_id = format!("stress_{}_{}", folder_num, i);
                let name = format!("IMG_{:05}.JPG", file_count);
                // Minimal content for speed
                let content = MockDataGenerator::generate_jpeg_header(256);
                fs.add_object(MockObject::file(
                    &file_id,
                    &folder_id,
                    &name,
                    content.len() as u64,
                    content,
                ));
                file_count += 1;
            }
        }

        TestScenario::new(
            "stress_test",
            "Maximum stress test: 10,000 files across 50 folders",
            device,
            fs,
            ExpectedResults {
                files_to_extract: 10000,
                folders: 52,
                should_succeed: true,
                ..Default::default()
            },
        )
        .with_tags(vec!["stress-test", "performance", "extreme"])
    }

    // =========================================================================
    // HELPER FUNCTIONS
    // =========================================================================

    /// Add standard DCIM structure to a file system
    fn add_standard_dcim_structure(fs: &mut MockFileSystem, total_files: usize, folders: usize) {
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));

        let files_per_folder = total_files / folders;
        let mut file_num = 0;

        for folder_idx in 0..folders {
            let folder_id = format!("{}apple", 100 + folder_idx);
            let folder_name = format!("{}APPLE", 100 + folder_idx);
            fs.add_object(MockObject::folder(&folder_id, "dcim", &folder_name));

            for i in 0..files_per_folder {
                let file_id = format!("file_{}_{}", folder_idx, i);
                let file_name = format!("IMG_{:04}.JPG", file_num);

                // Alternate between HEIC and JPG
                let content = if file_num % 2 == 0 {
                    MockDataGenerator::generate_heic_header(1024 * 1024)
                } else {
                    MockDataGenerator::generate_jpeg_header(500 * 1024)
                };

                fs.add_object(MockObject::file(
                    &file_id,
                    &folder_id,
                    &file_name,
                    content.len() as u64,
                    content,
                ));
                file_num += 1;
            }
        }
    }

    /// Add legacy DCIM structure (JPG only, older naming)
    fn add_legacy_dcim_structure(fs: &mut MockFileSystem, total_files: usize, folders: usize) {
        fs.add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        fs.add_object(MockObject::folder("dcim", "internal", "DCIM"));

        let files_per_folder = total_files / folders;
        let mut file_num = 0;

        for folder_idx in 0..folders {
            let folder_id = format!("{}apple", 100 + folder_idx);
            let folder_name = format!("{}APPLE", 100 + folder_idx);
            fs.add_object(MockObject::folder(&folder_id, "dcim", &folder_name));

            for i in 0..files_per_folder {
                let file_id = format!("legacy_{}_{}", folder_idx, i);
                let file_name = format!("IMG_{:04}.JPG", file_num);

                let content = MockDataGenerator::generate_jpeg_header(300 * 1024);

                fs.add_object(MockObject::file(
                    &file_id,
                    &folder_id,
                    &file_name,
                    content.len() as u64,
                    content,
                ));
                file_num += 1;
            }
        }
    }

    /// Get all available scenarios
    pub fn all_scenarios() -> Vec<TestScenario> {
        let mut scenarios = vec![
            Self::no_devices(),
            Self::single_iphone(),
            Self::ipad_device(),
            Self::old_iphone(),
            Self::unicode_device_name(),
            Self::empty_device(),
            Self::deeply_nested(),
            Self::many_files_single_folder(),
            Self::mixed_file_types(),
            Self::unicode_filenames(),
            Self::large_files(),
            Self::problematic_files(),
            Self::device_locked(),
            Self::disconnect_mid_transfer(),
            Self::file_read_errors(),
            Self::slow_transfer(),
            Self::flaky_connection(),
            Self::exact_duplicates(),
            Self::renamed_duplicates(),
            Self::no_duplicates(),
            Self::fresh_extraction(),
            Self::resume_extraction(),
            Self::new_device_profile(),
            Self::known_device_profile(),
            Self::realistic_iphone(),
            Self::stress_test(),
        ];

        // Add multi-device scenarios
        scenarios.extend(Self::multiple_iphones());
        scenarios.extend(Self::mixed_devices());

        scenarios
    }

    /// Get scenarios by tag
    pub fn scenarios_by_tag(tag: &str) -> Vec<TestScenario> {
        Self::all_scenarios()
            .into_iter()
            .filter(|s| s.tags.iter().any(|t| t == tag))
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
        println!("Loaded {} test scenarios", scenarios.len());
    }

    #[test]
    fn test_scenario_by_tag() {
        let error_scenarios = ScenarioLibrary::scenarios_by_tag("error");
        assert!(!error_scenarios.is_empty());
        for s in &error_scenarios {
            assert!(s.tags.contains(&"error".to_string()));
        }
    }

    #[test]
    fn test_quick_scenarios() {
        let quick = ScenarioLibrary::quick_scenarios();
        assert_eq!(quick.len(), 5);
    }
}

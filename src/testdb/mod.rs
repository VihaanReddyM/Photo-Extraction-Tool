//! Test Database Module
//!
//! This module provides a comprehensive testing framework for the photo extraction tool
//! that allows testing all features without connecting a real phone to the PC.
//!
//! # Architecture
//!
//! The testing module is built around trait-based abstractions that allow the same
//! extraction code to work with both real devices (via WPD) and mock devices.
//!
//! Key components:
//! - **Mock Devices**: Simulated iOS devices with configurable properties
//! - **Mock File Systems**: In-memory file systems that mimic real device storage
//! - **Test Scenarios**: Pre-built scenarios covering various use cases and edge cases
//! - **Data Generators**: Tools to create realistic mock file content
//! - **Test Runner**: Execute scenarios and generate reports
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use photo_extraction_tool::testdb::{TestRunner, ScenarioLibrary, TestRunnerConfig};
//!
//! // Run all quick test scenarios
//! let mut runner = TestRunner::new();
//! let summary = runner.run_quick();
//! println!("Passed: {}/{}", summary.passed, summary.total);
//!
//! // Run specific scenario by name
//! let mut runner = TestRunner::with_config(TestRunnerConfig {
//!     verbose: true,
//!     ..Default::default()
//! });
//! let summary = runner.run_by_names(&["single_iphone", "device_locked"]);
//! ```
//!
//! # Available Scenarios
//!
//! ## Device Detection
//! - `no_devices` - No devices connected
//! - `single_iphone` - Standard single iPhone
//! - `multiple_iphones_*` - Multiple iPhones connected
//! - `ipad_device` - iPad device
//! - `old_iphone` - Legacy iPhone model
//! - `unicode_device_name` - Device with unicode characters
//!
//! ## File Structure
//! - `empty_device` - Device with no photos
//! - `deeply_nested` - 15 levels of nested folders
//! - `many_files_single_folder` - 5000 files in one folder
//! - `mixed_file_types` - All supported file formats
//! - `unicode_filenames` - Files with unicode names
//! - `large_files` - Large video files (2GB+)
//! - `problematic_files` - Zero-byte, no extension, etc.
//!
//! ## Error Conditions
//! - `device_locked` - Access denied simulation
//! - `disconnect_mid_transfer` - Device disconnects during transfer
//! - `file_read_errors` - Specific files fail to read
//! - `slow_transfer` - Simulated slow connection
//! - `flaky_connection` - Random failure rate
//!
//! ## Duplicate Detection
//! - `exact_duplicates` - Same file in multiple folders
//! - `renamed_duplicates` - Same content, different names
//! - `no_duplicates` - All unique files
//!
//! ## Tracking/Resume
//! - `fresh_extraction` - First time extraction
//! - `resume_extraction` - Resume interrupted extraction
//!
//! ## Profiles
//! - `new_device_profile` - New device requiring profile
//! - `known_device_profile` - Known device with profile
//!
//! ## Stress Tests
//! - `realistic_iphone` - 2500+ files like real iPhone
//! - `stress_test` - 10,000 files maximum stress

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod assertions;
pub mod generator;
pub mod integration;
pub mod mock_device;
pub mod runner;
pub mod scenarios;

// Re-export commonly used types from assertions
pub use assertions::{
    AssertionCollection, AssertionResult, Benchmark, BenchmarkComparison, BenchmarkResult,
    DeviceFixtureBuilder, FileSystemFixtureBuilder, ScenarioFixtureBuilder, TestAssertions,
};

// Re-export commonly used types from generator
pub use generator::{FileGeneratorConfig, MockDataGenerator};

// Re-export memory-efficient test size constants
pub use generator::{
    TEST_CONTENT_MAX_SIZE, TEST_HEIC_SIZE, TEST_JPEG_SIZE, TEST_PNG_SIZE, TEST_STRESS_SIZE,
    TEST_VIDEO_SIZE,
};

// Re-export integration test utilities
pub use integration::{
    run_full_integration_tests, run_integration_tests, run_quick_integration_tests,
    IntegrationTestConfig, IntegrationTestResult, IntegrationTestRunner, IntegrationTestSummary,
};

// Re-export commonly used types from mock_device
pub use mock_device::{
    MockDeviceConfig, MockDeviceContent, MockDeviceManager, MockFileSystem, MockObject,
};

// Re-export DeviceInfo from the traits module for convenience
pub use crate::device::traits::DeviceInfo as MockDeviceInfo;

// Re-export commonly used types from runner
pub use runner::{
    ExecutionStats, InteractiveTestMode, ScenarioResult, TestRunner, TestRunnerConfig, TestSummary,
};

// Re-export commonly used types from scenarios
pub use scenarios::{ExpectedResults, ScenarioLibrary, TestScenario};

/// Prelude module for easy imports
pub mod prelude {
    pub use super::assertions::{
        Benchmark, DeviceFixtureBuilder, FileSystemFixtureBuilder, ScenarioFixtureBuilder,
        TestAssertions,
    };
    pub use super::generator::MockDataGenerator;
    pub use super::integration::{IntegrationTestConfig, IntegrationTestRunner};
    pub use super::mock_device::{
        MockDeviceConfig, MockDeviceContent, MockDeviceManager, MockFileSystem, MockObject,
    };
    pub use super::runner::{TestRunner, TestRunnerConfig, TestSummary};
    pub use super::scenarios::{ScenarioLibrary, TestScenario};
    pub use crate::device::traits::{
        DeviceContentTrait, DeviceInfo, DeviceManagerTrait, DeviceObject,
    };
}

// =============================================================================
// Convenience functions
// =============================================================================

/// Quick function to run all tests with default settings
pub fn run_all_tests() -> TestSummary {
    let mut runner = TestRunner::with_config(TestRunnerConfig {
        verbose: true,
        ..Default::default()
    });
    runner.run_all()
}

/// Quick function to run quick tests only
pub fn run_quick_tests() -> TestSummary {
    let mut runner = TestRunner::with_config(TestRunnerConfig {
        verbose: true,
        ..Default::default()
    });
    runner.run_quick()
}

/// Quick function to run tests by tag
pub fn run_tests_by_tag(tag: &str) -> TestSummary {
    let mut runner = TestRunner::with_config(TestRunnerConfig {
        verbose: true,
        ..Default::default()
    });
    runner.run_by_tag(tag)
}

/// Get a list of all available scenario names
pub fn list_scenario_names() -> Vec<String> {
    ScenarioLibrary::all_scenarios()
        .into_iter()
        .map(|s| s.name)
        .collect()
}

/// Get a list of all available tags
pub fn list_tags() -> Vec<String> {
    let mut tags: Vec<String> = ScenarioLibrary::all_scenarios()
        .into_iter()
        .flat_map(|s| s.tags)
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

/// Run all integration tests and print results
pub fn run_and_print_integration_tests() -> IntegrationTestSummary {
    let results = run_integration_tests();
    integration::print_integration_results(&results);

    let mut runner = IntegrationTestRunner::new();
    runner.run_all();
    runner.summary()
}

/// Print available scenarios to console
pub fn print_available_scenarios() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║           AVAILABLE TEST SCENARIOS                           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let scenarios = ScenarioLibrary::all_scenarios();

    // Group by first tag
    let mut by_category: std::collections::HashMap<String, Vec<&TestScenario>> =
        std::collections::HashMap::new();

    for scenario in &scenarios {
        let category = scenario
            .tags
            .first()
            .cloned()
            .unwrap_or_else(|| "other".to_string());
        by_category.entry(category).or_default().push(scenario);
    }

    let mut categories: Vec<_> = by_category.keys().cloned().collect();
    categories.sort();

    for category in categories {
        println!("📁 {}", category.to_uppercase());
        if let Some(scenarios) = by_category.get(&category) {
            for scenario in scenarios {
                println!("   • {} - {}", scenario.name, scenario.description);
            }
        }
        println!();
    }

    println!("Total: {} scenarios available\n", scenarios.len());
}

/// Create a simple mock device manager with a standard iPhone for quick testing
pub fn create_simple_mock_device() -> MockDeviceManager {
    let mut manager = MockDeviceManager::new();

    let device = MockDeviceInfo::new(
        "mock-device-001",
        "Test iPhone",
        "Apple Inc.",
        "iPhone 15 Pro",
    );

    let mut fs = MockFileSystem::new();
    fs.add_standard_dcim_structure(10, 2);

    manager.add_device(device, fs);
    manager
}

/// Create a mock device manager with multiple devices for testing device selection
pub fn create_multi_device_mock() -> MockDeviceManager {
    let mut manager = MockDeviceManager::new();

    // First iPhone
    let device1 = MockDeviceInfo::new(
        "mock-device-001",
        "Alice's iPhone",
        "Apple Inc.",
        "iPhone 15",
    );
    let mut fs1 = MockFileSystem::new();
    fs1.add_standard_dcim_structure(20, 2);
    manager.add_device(device1, fs1);

    // Second iPhone
    let device2 = MockDeviceInfo::new(
        "mock-device-002",
        "Bob's iPhone",
        "Apple Inc.",
        "iPhone 14 Pro",
    );
    let mut fs2 = MockFileSystem::new();
    fs2.add_standard_dcim_structure(15, 3);
    manager.add_device(device2, fs2);

    // iPad
    let device3 = MockDeviceInfo::new(
        "mock-device-003",
        "Family iPad",
        "Apple Inc.",
        "iPad Pro 12.9-inch",
    );
    let mut fs3 = MockFileSystem::new();
    fs3.add_standard_dcim_structure(50, 5);
    manager.add_device(device3, fs3);

    manager
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::traits::{DeviceContentTrait, DeviceManagerTrait};

    #[test]
    fn test_module_exports() {
        // Verify all exports are accessible
        let _ = MockDeviceInfo::default();
        let _ = MockFileSystem::new();
        let _ = MockDeviceManager::new();
        let _ = TestRunner::new();
        let _ = ScenarioLibrary::quick_scenarios();
    }

    #[test]
    fn test_list_functions() {
        let names = list_scenario_names();
        assert!(!names.is_empty());

        let tags = list_tags();
        assert!(!tags.is_empty());
    }

    #[test]
    fn test_quick_tests_run() {
        let summary = run_quick_tests();
        assert!(summary.total > 0);
    }

    #[test]
    fn test_simple_mock_device() {
        let manager = create_simple_mock_device();

        let devices = manager.enumerate_apple_devices().unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].friendly_name, "Test iPhone");

        let content = manager.open_device(&devices[0].device_id).unwrap();
        let root = content.enumerate_objects().unwrap();
        assert!(!root.is_empty());
    }

    #[test]
    fn test_multi_device_mock() {
        let manager = create_multi_device_mock();

        let devices = manager.enumerate_apple_devices().unwrap();
        assert_eq!(devices.len(), 3);
    }

    #[test]
    fn test_prelude_imports() {
        use super::prelude::*;

        let _ = MockFileSystem::new();
        let _ = MockDeviceManager::new();
        let _ = TestRunner::new();
    }
}

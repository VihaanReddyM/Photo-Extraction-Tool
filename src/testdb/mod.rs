//! Test Database Module
//!
//! This module provides a comprehensive testing framework for the photo extraction tool
//! that allows testing all features without connecting a real phone to the PC.
//!
//! Note: Many functions in this module are intentionally kept for API completeness
//! even if not currently used by the CLI.

#![allow(dead_code)]
#![allow(unused_imports)]

//!
//! # Features
//!
//! - **Mock Devices**: Simulated iOS devices with configurable properties
//! - **Virtual File Systems**: In-memory file systems that mimic real device storage
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

pub mod generator;
pub mod mock_device;
pub mod runner;
pub mod scenarios;

// Re-export commonly used types for convenience
pub use generator::{FileGeneratorConfig, MockDataGenerator};
pub use mock_device::{
    MockDeviceConfig, MockDeviceInfo, MockDeviceManager, MockFileSystem, MockObject,
};
pub use runner::{
    ExecutionStats, InteractiveTestMode, ScenarioResult, TestRunner, TestRunnerConfig, TestSummary,
};
pub use scenarios::{ExpectedResults, ScenarioLibrary, TestScenario};

/// Prelude module for easy imports
pub mod prelude {
    pub use super::generator::MockDataGenerator;
    pub use super::mock_device::{
        MockDeviceConfig, MockDeviceInfo, MockDeviceManager, MockFileSystem, MockObject,
    };
    pub use super::runner::{TestRunner, TestRunnerConfig, TestSummary};
    pub use super::scenarios::{ScenarioLibrary, TestScenario};
}

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
            .map(|s| s.clone())
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

#[cfg(test)]
mod tests {
    use super::*;

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
}

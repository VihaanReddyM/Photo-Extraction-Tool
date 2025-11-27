//! Test assertions, fixtures, and utilities for the photo extraction test framework
//!
//! This module provides:
//! - Custom assertion helpers for common test patterns
//! - Fixture builders for creating custom test scenarios
//! - Benchmark utilities for performance testing
//! - Test result matchers and validators

use super::generator::{MockDataGenerator, TEST_JPEG_SIZE, TEST_VIDEO_SIZE};
use super::mock_device::{MockDeviceConfig, MockFileSystem, MockObject};
use super::runner::{ExecutionStats, ScenarioResult, TestSummary};
use super::scenarios::{ExpectedResults, TestScenario};
use crate::core::error::Result;
use crate::device::traits::DeviceInfo;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// =============================================================================
// ASSERTION HELPERS
// =============================================================================

/// Result of an assertion check
#[derive(Debug, Clone)]
pub struct AssertionResult {
    /// Whether the assertion passed
    pub passed: bool,
    /// Description of what was being asserted
    pub description: String,
    /// Expected value (as string for display)
    pub expected: String,
    /// Actual value (as string for display)
    pub actual: String,
    /// Detailed failure message if assertion failed
    pub message: Option<String>,
}

impl AssertionResult {
    /// Create a passing assertion result
    pub fn pass(description: &str) -> Self {
        Self {
            passed: true,
            description: description.to_string(),
            expected: String::new(),
            actual: String::new(),
            message: None,
        }
    }

    /// Create a failing assertion result
    pub fn fail(description: &str, expected: &str, actual: &str) -> Self {
        Self {
            passed: false,
            description: description.to_string(),
            expected: expected.to_string(),
            actual: actual.to_string(),
            message: Some(format!(
                "Assertion failed: {}\n  Expected: {}\n  Actual: {}",
                description, expected, actual
            )),
        }
    }

    /// Create a failing assertion with custom message
    pub fn fail_with_message(description: &str, message: &str) -> Self {
        Self {
            passed: false,
            description: description.to_string(),
            expected: String::new(),
            actual: String::new(),
            message: Some(message.to_string()),
        }
    }
}

/// Collection of assertion results for a test
#[derive(Debug, Default, Clone)]
pub struct AssertionCollection {
    results: Vec<AssertionResult>,
}

impl AssertionCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Add an assertion result
    pub fn add(&mut self, result: AssertionResult) {
        self.results.push(result);
    }

    /// Check if all assertions passed
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.passed)
    }

    /// Get number of passed assertions
    pub fn passed_count(&self) -> usize {
        self.results.iter().filter(|r| r.passed).count()
    }

    /// Get number of failed assertions
    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| !r.passed).count()
    }

    /// Get all failed assertions
    pub fn failures(&self) -> Vec<&AssertionResult> {
        self.results.iter().filter(|r| !r.passed).collect()
    }

    /// Get summary message
    pub fn summary(&self) -> String {
        if self.all_passed() {
            format!("All {} assertions passed", self.results.len())
        } else {
            let mut msg = format!(
                "{}/{} assertions passed\n\nFailures:\n",
                self.passed_count(),
                self.results.len()
            );
            for failure in self.failures() {
                if let Some(ref m) = failure.message {
                    msg.push_str(&format!("  - {}\n", m));
                }
            }
            msg
        }
    }

    /// Panic if any assertions failed (for use in tests)
    pub fn assert_all_passed(&self) {
        if !self.all_passed() {
            panic!("{}", self.summary());
        }
    }
}

/// Assertion helpers for test scenarios
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that file count matches expected
    pub fn assert_file_count(stats: &ExecutionStats, expected: usize) -> AssertionResult {
        if stats.files_extracted == expected {
            AssertionResult::pass("file count matches")
        } else {
            AssertionResult::fail(
                "file count matches",
                &expected.to_string(),
                &stats.files_extracted.to_string(),
            )
        }
    }

    /// Assert that file count is within a range
    pub fn assert_file_count_range(
        stats: &ExecutionStats,
        min: usize,
        max: usize,
    ) -> AssertionResult {
        if stats.files_extracted >= min && stats.files_extracted <= max {
            AssertionResult::pass("file count within range")
        } else {
            AssertionResult::fail(
                "file count within range",
                &format!("{}-{}", min, max),
                &stats.files_extracted.to_string(),
            )
        }
    }

    /// Assert that error count matches expected
    pub fn assert_error_count(stats: &ExecutionStats, expected: usize) -> AssertionResult {
        if stats.errors == expected {
            AssertionResult::pass("error count matches")
        } else {
            AssertionResult::fail(
                "error count matches",
                &expected.to_string(),
                &stats.errors.to_string(),
            )
        }
    }

    /// Assert no errors occurred
    pub fn assert_no_errors(stats: &ExecutionStats) -> AssertionResult {
        if stats.errors == 0 {
            AssertionResult::pass("no errors occurred")
        } else {
            AssertionResult::fail("no errors occurred", "0", &stats.errors.to_string())
        }
    }

    /// Assert that bytes processed is at least the expected amount
    pub fn assert_bytes_at_least(stats: &ExecutionStats, min_bytes: u64) -> AssertionResult {
        if stats.bytes_processed >= min_bytes {
            AssertionResult::pass("bytes processed meets minimum")
        } else {
            AssertionResult::fail(
                "bytes processed meets minimum",
                &format!(">= {}", min_bytes),
                &stats.bytes_processed.to_string(),
            )
        }
    }

    /// Assert that a scenario result passed
    pub fn assert_scenario_passed(result: &ScenarioResult) -> AssertionResult {
        if result.passed {
            AssertionResult::pass(&format!("scenario '{}' passed", result.name))
        } else {
            AssertionResult::fail_with_message(
                &format!("scenario '{}' passed", result.name),
                result.failure_reason.as_deref().unwrap_or("Unknown reason"),
            )
        }
    }

    /// Assert that a scenario result failed (for negative testing)
    pub fn assert_scenario_failed(result: &ScenarioResult) -> AssertionResult {
        if !result.passed {
            AssertionResult::pass(&format!("scenario '{}' failed as expected", result.name))
        } else {
            AssertionResult::fail_with_message(
                &format!("scenario '{}' should have failed", result.name),
                "Scenario unexpectedly passed",
            )
        }
    }

    /// Assert that test summary has expected pass rate
    pub fn assert_pass_rate(summary: &TestSummary, min_rate: f64) -> AssertionResult {
        let actual_rate = summary.pass_rate();
        if actual_rate >= min_rate {
            AssertionResult::pass("pass rate meets minimum")
        } else {
            AssertionResult::fail(
                "pass rate meets minimum",
                &format!(">= {:.1}%", min_rate),
                &format!("{:.1}%", actual_rate),
            )
        }
    }

    /// Assert that duration is within expected time
    pub fn assert_duration_under(duration: Duration, max: Duration) -> AssertionResult {
        if duration <= max {
            AssertionResult::pass("duration within limit")
        } else {
            AssertionResult::fail(
                "duration within limit",
                &format!("<= {:?}", max),
                &format!("{:?}", duration),
            )
        }
    }

    /// Assert that stats match expected results
    pub fn assert_stats_match(
        stats: &ExecutionStats,
        expected: &ExpectedResults,
    ) -> AssertionCollection {
        let mut collection = AssertionCollection::new();

        // Check file count (with some tolerance)
        let file_tolerance = expected.errors.max(1);
        if stats.files_extracted >= expected.files_to_extract.saturating_sub(file_tolerance)
            && stats.files_extracted <= expected.files_to_extract + file_tolerance
        {
            collection.add(AssertionResult::pass("file count approximately matches"));
        } else {
            collection.add(AssertionResult::fail(
                "file count approximately matches",
                &format!("{}±{}", expected.files_to_extract, file_tolerance),
                &stats.files_extracted.to_string(),
            ));
        }

        // Check folder count
        if stats.folders == expected.folders {
            collection.add(AssertionResult::pass("folder count matches"));
        } else {
            collection.add(AssertionResult::fail(
                "folder count matches",
                &expected.folders.to_string(),
                &stats.folders.to_string(),
            ));
        }

        // Check error count
        if stats.errors <= expected.errors {
            collection.add(AssertionResult::pass("error count within expected"));
        } else {
            collection.add(AssertionResult::fail(
                "error count within expected",
                &format!("<= {}", expected.errors),
                &stats.errors.to_string(),
            ));
        }

        collection
    }
}

// =============================================================================
// FIXTURE BUILDERS
// =============================================================================

/// Builder for creating custom test devices
#[derive(Debug, Clone)]
pub struct DeviceFixtureBuilder {
    device_id: String,
    friendly_name: String,
    manufacturer: String,
    model: String,
}

impl Default for DeviceFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceFixtureBuilder {
    /// Create a new device fixture builder with defaults
    pub fn new() -> Self {
        Self {
            device_id: format!("test-device-{}", uuid_v4_simple()),
            friendly_name: "Test Device".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 15".to_string(),
        }
    }

    /// Set the device ID
    pub fn with_id(mut self, id: &str) -> Self {
        self.device_id = id.to_string();
        self
    }

    /// Set the friendly name
    pub fn with_name(mut self, name: &str) -> Self {
        self.friendly_name = name.to_string();
        self
    }

    /// Set the manufacturer
    pub fn with_manufacturer(mut self, manufacturer: &str) -> Self {
        self.manufacturer = manufacturer.to_string();
        self
    }

    /// Set the model
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Create an iPhone fixture
    pub fn iphone(model: &str) -> Self {
        Self::new()
            .with_name(&format!("Test {}", model))
            .with_manufacturer("Apple Inc.")
            .with_model(model)
    }

    /// Create an iPad fixture
    pub fn ipad(model: &str) -> Self {
        Self::new()
            .with_name(&format!("Test {}", model))
            .with_manufacturer("Apple Inc.")
            .with_model(model)
    }

    /// Build the DeviceInfo
    pub fn build(self) -> DeviceInfo {
        DeviceInfo::new(
            &self.device_id,
            &self.friendly_name,
            &self.manufacturer,
            &self.model,
        )
    }
}

/// Builder for creating custom file systems
#[derive(Debug)]
pub struct FileSystemFixtureBuilder {
    fs: MockFileSystem,
    config: MockDeviceConfig,
    current_folder_id: String,
    file_counter: usize,
}

impl Default for FileSystemFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystemFixtureBuilder {
    /// Create a new file system fixture builder
    pub fn new() -> Self {
        Self {
            fs: MockFileSystem::new(),
            config: MockDeviceConfig::new(),
            current_folder_id: "DEVICE".to_string(),
            file_counter: 1,
        }
    }

    /// Add internal storage and DCIM folder structure
    pub fn with_dcim(mut self) -> Self {
        self.fs
            .add_object(MockObject::folder("internal", "DEVICE", "Internal Storage"));
        self.fs
            .add_object(MockObject::folder("dcim", "internal", "DCIM"));
        self.current_folder_id = "dcim".to_string();
        self
    }

    /// Add a folder under the current folder
    pub fn add_folder(mut self, name: &str) -> Self {
        let folder_id = format!("folder_{}", name.to_lowercase().replace(' ', "_"));
        self.fs.add_object(MockObject::folder(
            &folder_id,
            &self.current_folder_id,
            name,
        ));
        self.current_folder_id = folder_id;
        self
    }

    /// Add a standard Apple-style folder (100APPLE, etc.)
    pub fn add_apple_folder(mut self, number: usize) -> Self {
        let folder_id = format!("{}apple", number);
        let folder_name = format!("{}APPLE", number);
        self.fs.add_object(MockObject::folder(
            &folder_id,
            &self.current_folder_id,
            &folder_name,
        ));
        self.current_folder_id = folder_id;
        self
    }

    /// Navigate to a specific folder by ID
    pub fn in_folder(mut self, folder_id: &str) -> Self {
        self.current_folder_id = folder_id.to_string();
        self
    }

    /// Add JPEG files to current folder
    pub fn add_jpegs(mut self, count: usize) -> Self {
        for _ in 0..count {
            let file_id = format!("img_{:06}", self.file_counter);
            let file_name = format!("IMG_{:04}.JPG", self.file_counter);
            let content = MockDataGenerator::generate_jpeg_with_seed(
                TEST_JPEG_SIZE,
                self.file_counter as u64,
            );
            self.fs.add_object(MockObject::file(
                &file_id,
                &self.current_folder_id,
                &file_name,
                content,
            ));
            self.file_counter += 1;
        }
        self
    }

    /// Add HEIC files to current folder
    pub fn add_heics(mut self, count: usize) -> Self {
        for _ in 0..count {
            let file_id = format!("heic_{:06}", self.file_counter);
            let file_name = format!("IMG_{:04}.HEIC", self.file_counter);
            let content = MockDataGenerator::generate_heic_with_seed(
                TEST_JPEG_SIZE,
                self.file_counter as u64,
            );
            self.fs.add_object(MockObject::file(
                &file_id,
                &self.current_folder_id,
                &file_name,
                content,
            ));
            self.file_counter += 1;
        }
        self
    }

    /// Add MOV video files to current folder
    pub fn add_videos(mut self, count: usize) -> Self {
        for _ in 0..count {
            let file_id = format!("mov_{:06}", self.file_counter);
            let file_name = format!("IMG_{:04}.MOV", self.file_counter);
            let content = MockDataGenerator::generate_mov_with_seed(
                TEST_VIDEO_SIZE,
                self.file_counter as u64,
            );
            self.fs.add_object(MockObject::file(
                &file_id,
                &self.current_folder_id,
                &file_name,
                content,
            ));
            self.file_counter += 1;
        }
        self
    }

    /// Add a mix of file types
    pub fn add_mixed_files(mut self, count: usize) -> Self {
        for i in 0..count {
            let (ext, generator): (&str, fn(usize, u64) -> Vec<u8>) = match i % 4 {
                0 => ("JPG", |s, seed| {
                    MockDataGenerator::generate_jpeg_with_seed(s, seed)
                }),
                1 => ("HEIC", |s, seed| {
                    MockDataGenerator::generate_heic_with_seed(s, seed)
                }),
                2 => ("PNG", |s, seed| {
                    MockDataGenerator::generate_png_with_seed(s, seed)
                }),
                _ => ("MOV", |s, seed| {
                    MockDataGenerator::generate_mov_with_seed(s, seed)
                }),
            };

            let file_id = format!("mixed_{:06}", self.file_counter);
            let file_name = format!("IMG_{:04}.{}", self.file_counter, ext);
            let size = if ext == "MOV" {
                TEST_VIDEO_SIZE
            } else {
                TEST_JPEG_SIZE
            };
            let content = generator(size, self.file_counter as u64);
            self.fs.add_object(MockObject::file(
                &file_id,
                &self.current_folder_id,
                &file_name,
                content,
            ));
            self.file_counter += 1;
        }
        self
    }

    /// Add a file with custom name and content
    pub fn add_custom_file(mut self, name: &str, content: Vec<u8>) -> Self {
        let file_id = format!("custom_{:06}", self.file_counter);
        self.fs.add_object(MockObject::file(
            &file_id,
            &self.current_folder_id,
            name,
            content,
        ));
        self.file_counter += 1;
        self
    }

    /// Add a zero-byte file
    pub fn add_empty_file(mut self, name: &str) -> Self {
        let file_id = format!("empty_{:06}", self.file_counter);
        self.fs.add_object(MockObject::file(
            &file_id,
            &self.current_folder_id,
            name,
            Vec::new(),
        ));
        self.file_counter += 1;
        self
    }

    /// Configure as locked device
    pub fn locked(mut self) -> Self {
        self.config = MockDeviceConfig::locked();
        self
    }

    /// Configure to disconnect after N reads
    pub fn disconnect_after(mut self, reads: usize) -> Self {
        self.config = MockDeviceConfig::disconnect_after(reads);
        self
    }

    /// Configure with slow transfer
    pub fn slow_transfer(mut self, ms_per_kb: u64) -> Self {
        self.config = MockDeviceConfig::slow_transfer(ms_per_kb);
        self
    }

    /// Configure with random failures
    pub fn flaky(mut self, failure_rate: u8) -> Self {
        self.config = MockDeviceConfig::flaky(failure_rate);
        self
    }

    /// Build the MockFileSystem
    pub fn build(mut self) -> MockFileSystem {
        self.fs.set_config(self.config);
        self.fs
    }
}

/// Builder for creating complete test scenarios
#[derive(Debug)]
pub struct ScenarioFixtureBuilder {
    name: String,
    description: String,
    device_builder: DeviceFixtureBuilder,
    fs_builder: FileSystemFixtureBuilder,
    expected: ExpectedResults,
    tags: Vec<String>,
}

impl Default for ScenarioFixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScenarioFixtureBuilder {
    /// Create a new scenario fixture builder
    pub fn new() -> Self {
        Self {
            name: "custom_scenario".to_string(),
            description: "Custom test scenario".to_string(),
            device_builder: DeviceFixtureBuilder::new(),
            fs_builder: FileSystemFixtureBuilder::new(),
            expected: ExpectedResults::default(),
            tags: Vec::new(),
        }
    }

    /// Set the scenario name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Set the device builder
    pub fn with_device(mut self, builder: DeviceFixtureBuilder) -> Self {
        self.device_builder = builder;
        self
    }

    /// Set the file system builder
    pub fn with_filesystem(mut self, builder: FileSystemFixtureBuilder) -> Self {
        self.fs_builder = builder;
        self
    }

    /// Set expected results
    pub fn with_expected(mut self, expected: ExpectedResults) -> Self {
        self.expected = expected;
        self
    }

    /// Set expected file count
    pub fn expect_files(mut self, count: usize) -> Self {
        self.expected.files_to_extract = count;
        self
    }

    /// Set expected folder count
    pub fn expect_folders(mut self, count: usize) -> Self {
        self.expected.folders = count;
        self
    }

    /// Set expected error count
    pub fn expect_errors(mut self, count: usize) -> Self {
        self.expected.errors = count;
        self
    }

    /// Expect the scenario to succeed
    pub fn expect_success(mut self) -> Self {
        self.expected.should_succeed = true;
        self
    }

    /// Expect the scenario to fail
    pub fn expect_failure(mut self, error: Option<&str>) -> Self {
        self.expected.should_succeed = false;
        self.expected.expected_error = error.map(String::from);
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(String::from).collect();
        self
    }

    /// Build the TestScenario
    pub fn build(self) -> TestScenario {
        TestScenario {
            name: self.name,
            description: self.description,
            device_info: self.device_builder.build(),
            file_system: self.fs_builder.build(),
            expected: self.expected,
            tags: self.tags,
        }
    }
}

// =============================================================================
// BENCHMARK UTILITIES
// =============================================================================

/// Result of a benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Name of the benchmark
    pub name: String,
    /// Number of iterations
    pub iterations: usize,
    /// Total duration for all iterations
    pub total_duration: Duration,
    /// Average duration per iteration
    pub avg_duration: Duration,
    /// Minimum duration
    pub min_duration: Duration,
    /// Maximum duration
    pub max_duration: Duration,
    /// Standard deviation (if enough samples)
    pub std_dev: Option<Duration>,
    /// Files processed per second
    pub files_per_second: f64,
    /// Bytes processed per second
    pub bytes_per_second: f64,
}

impl BenchmarkResult {
    /// Format the result as a human-readable string
    pub fn summary(&self) -> String {
        format!(
            "Benchmark: {}\n\
             Iterations: {}\n\
             Total time: {:?}\n\
             Average: {:?}\n\
             Min: {:?}\n\
             Max: {:?}\n\
             Files/sec: {:.2}\n\
             MB/sec: {:.2}",
            self.name,
            self.iterations,
            self.total_duration,
            self.avg_duration,
            self.min_duration,
            self.max_duration,
            self.files_per_second,
            self.bytes_per_second / (1024.0 * 1024.0)
        )
    }
}

/// Benchmark runner for performance testing
pub struct Benchmark {
    name: String,
    warmup_iterations: usize,
    iterations: usize,
}

impl Benchmark {
    /// Create a new benchmark with the given name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            warmup_iterations: 2,
            iterations: 10,
        }
    }

    /// Set the number of warmup iterations (not counted in results)
    pub fn warmup(mut self, iterations: usize) -> Self {
        self.warmup_iterations = iterations;
        self
    }

    /// Set the number of measured iterations
    pub fn iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Run the benchmark with a closure that returns (files, bytes)
    pub fn run<F>(self, mut f: F) -> BenchmarkResult
    where
        F: FnMut() -> (usize, u64),
    {
        // Warmup
        for _ in 0..self.warmup_iterations {
            let _ = f();
        }

        // Measured runs
        let mut durations = Vec::with_capacity(self.iterations);
        let mut total_files = 0usize;
        let mut total_bytes = 0u64;

        for _ in 0..self.iterations {
            let start = Instant::now();
            let (files, bytes) = f();
            let elapsed = start.elapsed();

            durations.push(elapsed);
            total_files += files;
            total_bytes += bytes;
        }

        // Calculate statistics
        let total_duration: Duration = durations.iter().sum();
        let avg_duration = total_duration / self.iterations as u32;
        let min_duration = *durations.iter().min().unwrap_or(&Duration::ZERO);
        let max_duration = *durations.iter().max().unwrap_or(&Duration::ZERO);

        // Calculate standard deviation if we have enough samples
        let std_dev = if self.iterations >= 3 {
            let avg_nanos = avg_duration.as_nanos() as f64;
            let variance: f64 = durations
                .iter()
                .map(|d| {
                    let diff = d.as_nanos() as f64 - avg_nanos;
                    diff * diff
                })
                .sum::<f64>()
                / (self.iterations - 1) as f64;
            Some(Duration::from_nanos(variance.sqrt() as u64))
        } else {
            None
        };

        let files_per_second = total_files as f64 / total_duration.as_secs_f64();
        let bytes_per_second = total_bytes as f64 / total_duration.as_secs_f64();

        BenchmarkResult {
            name: self.name,
            iterations: self.iterations,
            total_duration,
            avg_duration,
            min_duration,
            max_duration,
            std_dev,
            files_per_second,
            bytes_per_second,
        }
    }

    /// Run the benchmark with a simpler closure (just returns success/failure)
    pub fn run_simple<F>(self, mut f: F) -> BenchmarkResult
    where
        F: FnMut() -> bool,
    {
        self.run(|| {
            let success = f();
            (if success { 1 } else { 0 }, 0)
        })
    }
}

/// Compare two benchmark results
pub struct BenchmarkComparison {
    pub baseline: BenchmarkResult,
    pub current: BenchmarkResult,
    pub speedup: f64,
    pub is_faster: bool,
}

impl BenchmarkComparison {
    /// Compare current results against a baseline
    pub fn compare(baseline: BenchmarkResult, current: BenchmarkResult) -> Self {
        let baseline_nanos = baseline.avg_duration.as_nanos() as f64;
        let current_nanos = current.avg_duration.as_nanos() as f64;
        let speedup = baseline_nanos / current_nanos;
        let is_faster = speedup > 1.0;

        Self {
            baseline,
            current,
            speedup,
            is_faster,
        }
    }

    /// Format the comparison as a human-readable string
    pub fn summary(&self) -> String {
        let change = if self.is_faster {
            format!("{:.1}x faster", self.speedup)
        } else {
            format!("{:.1}x slower", 1.0 / self.speedup)
        };

        format!(
            "Benchmark Comparison: {} vs {}\n\
             Baseline avg: {:?}\n\
             Current avg:  {:?}\n\
             Change: {}",
            self.baseline.name,
            self.current.name,
            self.baseline.avg_duration,
            self.current.avg_duration,
            change
        )
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Generate a simple UUID-like string for unique IDs
fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let random: u64 = {
        // Simple pseudo-random based on timestamp
        let mut x = timestamp as u64;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    };

    format!(
        "{:08x}-{:04x}-{:04x}",
        timestamp as u32,
        (random >> 32) as u16,
        random as u16
    )
}

/// Quick fixture for a standard iPhone with photos
pub fn quick_iphone_fixture(photo_count: usize) -> (DeviceInfo, MockFileSystem) {
    let device = DeviceFixtureBuilder::iphone("iPhone 15").build();
    let fs = FileSystemFixtureBuilder::new()
        .with_dcim()
        .add_apple_folder(100)
        .add_jpegs(photo_count)
        .build();
    (device, fs)
}

/// Quick fixture for an iPhone with mixed content
pub fn quick_mixed_fixture(file_count: usize) -> (DeviceInfo, MockFileSystem) {
    let device = DeviceFixtureBuilder::iphone("iPhone 15 Pro").build();
    let fs = FileSystemFixtureBuilder::new()
        .with_dcim()
        .add_apple_folder(100)
        .add_mixed_files(file_count)
        .build();
    (device, fs)
}

/// Quick scenario for basic extraction testing
pub fn quick_extraction_scenario(photo_count: usize) -> TestScenario {
    let (device, fs) = quick_iphone_fixture(photo_count);
    TestScenario {
        name: format!("quick_extraction_{}", photo_count),
        description: format!("Quick extraction test with {} photos", photo_count),
        device_info: device,
        file_system: fs,
        expected: ExpectedResults {
            files_to_extract: photo_count,
            folders: 3, // Internal Storage, DCIM, 100APPLE
            should_succeed: true,
            ..Default::default()
        },
        tags: vec!["quick".to_string(), "extraction".to_string()],
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assertion_result_pass() {
        let result = AssertionResult::pass("test passed");
        assert!(result.passed);
        assert!(result.message.is_none());
    }

    #[test]
    fn test_assertion_result_fail() {
        let result = AssertionResult::fail("values match", "10", "5");
        assert!(!result.passed);
        assert!(result.message.is_some());
        assert!(result.message.unwrap().contains("Expected: 10"));
    }

    #[test]
    fn test_assertion_collection() {
        let mut collection = AssertionCollection::new();
        collection.add(AssertionResult::pass("test 1"));
        collection.add(AssertionResult::pass("test 2"));
        collection.add(AssertionResult::fail("test 3", "a", "b"));

        assert!(!collection.all_passed());
        assert_eq!(collection.passed_count(), 2);
        assert_eq!(collection.failed_count(), 1);
    }

    #[test]
    fn test_device_fixture_builder() {
        let device = DeviceFixtureBuilder::iphone("iPhone 15 Pro Max")
            .with_name("My iPhone")
            .build();

        assert_eq!(device.friendly_name, "My iPhone");
        assert_eq!(device.model, "iPhone 15 Pro Max");
        assert_eq!(device.manufacturer, "Apple Inc.");
    }

    #[test]
    fn test_filesystem_fixture_builder() {
        let fs = FileSystemFixtureBuilder::new()
            .with_dcim()
            .add_apple_folder(100)
            .add_jpegs(5)
            .add_apple_folder(101)
            .add_heics(3)
            .build();

        assert_eq!(fs.file_count(), 8); // 5 JPEGs + 3 HEICs
        assert!(fs.folder_count() >= 4); // Internal Storage, DCIM, 100APPLE, 101APPLE
    }

    #[test]
    fn test_scenario_fixture_builder() {
        let scenario = ScenarioFixtureBuilder::new()
            .with_name("test_scenario")
            .with_description("A test scenario")
            .with_device(DeviceFixtureBuilder::iphone("iPhone 14"))
            .with_filesystem(
                FileSystemFixtureBuilder::new()
                    .with_dcim()
                    .add_apple_folder(100)
                    .add_jpegs(10),
            )
            .expect_files(10)
            .expect_folders(3)
            .expect_success()
            .with_tags(vec!["test", "custom"])
            .build();

        assert_eq!(scenario.name, "test_scenario");
        assert_eq!(scenario.expected.files_to_extract, 10);
        assert!(scenario.expected.should_succeed);
        assert_eq!(scenario.tags.len(), 2);
    }

    #[test]
    fn test_quick_fixtures() {
        let (device, fs) = quick_iphone_fixture(20);
        assert_eq!(device.model, "iPhone 15");
        assert_eq!(fs.file_count(), 20);

        let scenario = quick_extraction_scenario(15);
        assert_eq!(scenario.expected.files_to_extract, 15);
    }

    #[test]
    fn test_benchmark() {
        let result = Benchmark::new("simple_test")
            .warmup(1)
            .iterations(3)
            .run_simple(|| true);

        assert_eq!(result.name, "simple_test");
        assert_eq!(result.iterations, 3);
        assert!(result.avg_duration > Duration::ZERO);
    }

    #[test]
    fn test_test_assertions() {
        let stats = ExecutionStats {
            files_extracted: 10,
            files_skipped: 2,
            duplicates_found: 1,
            errors: 0,
            bytes_processed: 1024,
            folders: 3,
        };

        assert!(TestAssertions::assert_file_count(&stats, 10).passed);
        assert!(!TestAssertions::assert_file_count(&stats, 5).passed);
        assert!(TestAssertions::assert_no_errors(&stats).passed);
        assert!(TestAssertions::assert_file_count_range(&stats, 5, 15).passed);
    }
}

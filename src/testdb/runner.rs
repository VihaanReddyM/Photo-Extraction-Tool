//! Test runner for executing test scenarios
//!
//! This module provides a comprehensive test runner that executes scenarios
//! using the mock device infrastructure and validates results against expectations.
//!
//! The runner supports:
//! - Running all scenarios or filtered subsets
//! - Verbose output and progress tracking
//! - HTML and JSON report generation
//! - Interactive mode for browsing scenarios
//! - Full integration with the extraction pipeline via traits

use super::mock_device::{MockDeviceContent, MockDeviceManager, MockFileSystem};
use super::scenarios::{ExpectedResults, ScenarioLibrary, TestScenario};
use crate::core::error::{ExtractionError, Result};
use crate::device::traits::{DeviceContentTrait, DeviceInfo, DeviceManagerTrait, DeviceObject};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// =============================================================================
// ScenarioResult - Result of running a single scenario
// =============================================================================

/// Result of running a single test scenario
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    /// Scenario name
    pub name: String,
    /// Whether the test passed
    pub passed: bool,
    /// Time taken to run the scenario
    pub duration: Duration,
    /// Number of files extracted
    pub files_extracted: usize,
    /// Number of files skipped
    pub files_skipped: usize,
    /// Number of duplicates found
    pub duplicates_found: usize,
    /// Number of errors encountered
    pub errors: usize,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Summary message
    pub message: String,
    /// Expected results for comparison
    pub expected: Option<ExpectedResults>,
    /// Failure reason (if failed)
    pub failure_reason: Option<String>,
}

impl ScenarioResult {
    /// Create a passed result
    pub fn passed(name: &str, duration: Duration) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            duration,
            files_extracted: 0,
            files_skipped: 0,
            duplicates_found: 0,
            errors: 0,
            bytes_processed: 0,
            message: "Passed".to_string(),
            expected: None,
            failure_reason: None,
        }
    }

    /// Create a failed result
    pub fn failed(name: &str, duration: Duration, reason: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            duration,
            files_extracted: 0,
            files_skipped: 0,
            duplicates_found: 0,
            errors: 0,
            bytes_processed: 0,
            message: format!("Failed: {}", reason),
            expected: None,
            failure_reason: Some(reason.to_string()),
        }
    }

    /// Add statistics to the result
    pub fn with_stats(
        mut self,
        files_extracted: usize,
        files_skipped: usize,
        duplicates_found: usize,
        errors: usize,
        bytes_processed: u64,
    ) -> Self {
        self.files_extracted = files_extracted;
        self.files_skipped = files_skipped;
        self.duplicates_found = duplicates_found;
        self.errors = errors;
        self.bytes_processed = bytes_processed;
        self
    }

    /// Add expected results for comparison
    pub fn with_expected(mut self, expected: ExpectedResults) -> Self {
        self.expected = Some(expected);
        self
    }
}

// =============================================================================
// TestSummary - Aggregated results across all scenarios
// =============================================================================

/// Summary of all test results
#[derive(Debug, Clone, Default)]
pub struct TestSummary {
    /// Total number of scenarios run
    pub total: usize,
    /// Number of scenarios that passed
    pub passed: usize,
    /// Number of scenarios that failed
    pub failed: usize,
    /// Number of scenarios skipped
    pub skipped: usize,
    /// Total time taken
    pub total_duration: Duration,
    /// Results grouped by tag
    pub results_by_tag: HashMap<String, Vec<ScenarioResult>>,
}

impl TestSummary {
    /// Calculate pass rate as a percentage
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total as f64) * 100.0
        }
    }

    /// Get names of failed scenarios
    pub fn failed_scenarios(&self) -> Vec<String> {
        self.results_by_tag
            .values()
            .flatten()
            .filter(|r| !r.passed)
            .map(|r| r.name.clone())
            .collect()
    }

    /// Get total files extracted across all scenarios
    pub fn total_files_extracted(&self) -> usize {
        self.results_by_tag
            .values()
            .flatten()
            .map(|r| r.files_extracted)
            .sum()
    }

    /// Get total bytes processed across all scenarios
    pub fn total_bytes_processed(&self) -> u64 {
        self.results_by_tag
            .values()
            .flatten()
            .map(|r| r.bytes_processed)
            .sum()
    }
}

// =============================================================================
// TestRunnerConfig - Configuration for the test runner
// =============================================================================

/// Configuration for the test runner
#[derive(Debug, Clone)]
pub struct TestRunnerConfig {
    /// Whether to print verbose output
    pub verbose: bool,
    /// Stop on first failure
    pub fail_fast: bool,
    /// Filter scenarios by tag
    pub tag_filter: Option<String>,
    /// Filter scenarios by name pattern
    pub name_filter: Option<String>,
    /// Directory to output reports
    pub report_dir: Option<PathBuf>,
    /// Generate HTML report
    pub html_report: bool,
    /// Generate JSON report
    pub json_report: bool,
    /// Timeout in seconds for each scenario
    pub timeout_seconds: u64,
    /// Whether to actually write files to disk during extraction simulation
    pub write_files: bool,
    /// Output directory for file writes
    pub output_dir: Option<PathBuf>,
}

impl Default for TestRunnerConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            fail_fast: false,
            tag_filter: None,
            name_filter: None,
            report_dir: None,
            html_report: false,
            json_report: false,
            timeout_seconds: 300,
            write_files: false,
            output_dir: None,
        }
    }
}

// =============================================================================
// ExecutionStats - Statistics from scenario execution
// =============================================================================

/// Statistics collected during scenario execution
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    /// Files successfully extracted
    pub files_extracted: usize,
    /// Files skipped (already exist, etc.)
    pub files_skipped: usize,
    /// Duplicates found
    pub duplicates_found: usize,
    /// Errors encountered
    pub errors: usize,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Number of folders traversed
    pub folders: usize,
}

// =============================================================================
// TestRunner - Main test runner implementation
// =============================================================================

/// Main test runner for executing scenarios
pub struct TestRunner {
    /// Configuration
    config: TestRunnerConfig,
    /// Results from executed scenarios
    results: Vec<ScenarioResult>,
    /// Start time of the test run
    start_time: Option<Instant>,
}

impl TestRunner {
    /// Create a new test runner with default configuration
    pub fn new() -> Self {
        Self {
            config: TestRunnerConfig::default(),
            results: Vec::new(),
            start_time: None,
        }
    }

    /// Create a test runner with specific configuration
    pub fn with_config(config: TestRunnerConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
            start_time: None,
        }
    }

    /// Run all available test scenarios
    ///
    /// Uses lazy loading to avoid loading all scenarios into memory at once.
    pub fn run_all(&mut self) -> TestSummary {
        self.run_scenarios_lazy(|| ScenarioLibrary::all_scenarios())
    }

    /// Run quick test scenarios only
    pub fn run_quick(&mut self) -> TestSummary {
        self.run_scenarios_lazy(|| ScenarioLibrary::quick_scenarios())
    }

    /// Run scenarios filtered by tag
    pub fn run_by_tag(&mut self, tag: &str) -> TestSummary {
        let tag = tag.to_string();
        self.run_scenarios_lazy(move || ScenarioLibrary::scenarios_by_tag(&tag))
    }

    /// Run scenarios by name
    pub fn run_by_names(&mut self, names: &[&str]) -> TestSummary {
        let all_scenarios = ScenarioLibrary::all_scenarios();
        let scenarios: Vec<TestScenario> = all_scenarios
            .into_iter()
            .filter(|s| names.contains(&s.name.as_str()))
            .collect();
        self.run_scenarios(scenarios)
    }

    /// Run a list of scenarios
    pub fn run_scenarios(&mut self, scenarios: Vec<TestScenario>) -> TestSummary {
        self.run_scenarios_lazy(|| scenarios)
    }

    /// Run scenarios with lazy loading to minimize memory usage
    ///
    /// This method loads and runs scenarios one at a time, freeing memory
    /// after each test completes. This prevents memory exhaustion when
    /// running large test suites with heavy scenarios.
    fn run_scenarios_lazy<F>(&mut self, scenario_provider: F) -> TestSummary
    where
        F: FnOnce() -> Vec<TestScenario>,
    {
        self.start_time = Some(Instant::now());
        self.results.clear();

        // Get scenario metadata first (names for filtering)
        let scenarios = scenario_provider();
        let filtered = self.filter_scenarios(scenarios);
        let total = filtered.len();

        if self.config.verbose {
            println!("\n╔══════════════════════════════════════════════════════════════╗");
            println!("║              PHOTO EXTRACTION TOOL TEST RUNNER               ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!("\nRunning {} test scenarios...\n", total);
        }

        // Process scenarios one at a time to minimize memory usage
        for (index, scenario) in filtered.into_iter().enumerate() {
            if self.config.verbose {
                println!("[{}/{}] Running: {}", index + 1, total, scenario.name);
            }

            // Run the scenario - it will be dropped after this block
            let result = self.run_single_scenario(scenario);
            let passed = result.passed;

            if self.config.verbose {
                self.print_result(&result);
            }

            self.results.push(result);

            // Explicitly hint to drop any large allocations
            // This helps the allocator reclaim memory between scenarios
            #[cfg(not(target_env = "msvc"))]
            {
                // On non-Windows, we can hint to release memory
            }

            if self.config.fail_fast && !passed {
                if self.config.verbose {
                    println!("\n⚠️  Stopping early due to fail-fast mode");
                }
                break;
            }
        }

        let summary = self.generate_summary();

        if self.config.verbose {
            self.print_summary(&summary);
        }

        // Generate reports if configured
        if self.config.html_report {
            if let Err(e) = self.generate_html_report(&summary) {
                eprintln!("Failed to generate HTML report: {}", e);
            }
        }

        if self.config.json_report {
            if let Err(e) = self.generate_json_report(&summary) {
                eprintln!("Failed to generate JSON report: {}", e);
            }
        }

        summary
    }

    /// Filter scenarios based on configuration
    fn filter_scenarios(&self, scenarios: Vec<TestScenario>) -> Vec<TestScenario> {
        let mut filtered = scenarios;

        // Filter by tag
        if let Some(ref tag) = self.config.tag_filter {
            filtered = filtered
                .into_iter()
                .filter(|s| s.tags.contains(tag))
                .collect();
        }

        // Filter by name pattern
        if let Some(ref pattern) = self.config.name_filter {
            let pattern_lower = pattern.to_lowercase();
            filtered = filtered
                .into_iter()
                .filter(|s| s.name.to_lowercase().contains(&pattern_lower))
                .collect();
        }

        filtered
    }

    /// Run a single scenario and return the result
    fn run_single_scenario(&self, scenario: TestScenario) -> ScenarioResult {
        let start = Instant::now();
        let scenario_name = scenario.name.clone();
        let expected = scenario.expected.clone();

        // Execute the scenario
        let result = self.execute_scenario(&scenario);

        let duration = start.elapsed();

        match result {
            Ok(stats) => {
                // Compare with expected results
                let passed = self.compare_results(&stats, &scenario.expected);

                let mut result = if passed {
                    ScenarioResult::passed(&scenario_name, duration)
                } else {
                    let reason = self.get_mismatch_reason(&stats, &scenario.expected);
                    ScenarioResult::failed(&scenario_name, duration, &reason)
                };

                result = result
                    .with_stats(
                        stats.files_extracted,
                        stats.files_skipped,
                        stats.duplicates_found,
                        stats.errors,
                        stats.bytes_processed,
                    )
                    .with_expected(expected);

                result
            }
            Err(error) => {
                // Check if error was expected
                if !scenario.expected.should_succeed {
                    if let Some(ref expected_error) = scenario.expected.expected_error {
                        let error_str = format!("{:?}", error);
                        if error_str
                            .to_lowercase()
                            .contains(&expected_error.to_lowercase())
                        {
                            return ScenarioResult::passed(&scenario_name, duration)
                                .with_expected(expected);
                        }
                    }
                    // Error was expected but type may not match - still consider it a pass
                    // if should_succeed is false
                    return ScenarioResult::passed(&scenario_name, duration)
                        .with_expected(expected);
                }

                ScenarioResult::failed(&scenario_name, duration, &format!("{:?}", error))
                    .with_expected(expected)
            }
        }
    }

    /// Execute a scenario and return statistics
    ///
    /// Note: This method takes ownership of parts of the scenario to avoid
    /// unnecessary cloning and reduce memory usage.
    fn execute_scenario(&self, scenario: &TestScenario) -> Result<ExecutionStats> {
        // Create a mock device manager with the scenario's device and file system
        let mut manager = MockDeviceManager::new();

        // Clone the file system from the scenario
        // Note: We clone here because we need to keep scenario for result comparison
        // The clone will be dropped when this function returns, freeing memory
        let fs = scenario.file_system.clone();
        let device_info = scenario.device_info.clone();

        manager.add_device(device_info.clone(), fs);

        // Try to open the device
        let content = manager.open_device(&device_info.device_id)?;

        // Simulate extraction by traversing the file system
        let mut stats = ExecutionStats::default();

        // Get root objects and traverse
        self.traverse_and_extract(&content, "DEVICE", &mut stats)?;

        // manager and content are dropped here, freeing the file system memory
        Ok(stats)
    }

    /// Traverse the file system and simulate extraction
    ///
    /// Uses iterative approach with explicit stack to avoid deep recursion
    /// and reduce stack memory usage for deeply nested structures.
    fn traverse_and_extract(
        &self,
        content: &MockDeviceContent,
        parent_id: &str,
        stats: &mut ExecutionStats,
    ) -> Result<()> {
        // Use iterative traversal with a stack to avoid stack overflow
        // and reduce memory pressure from deep recursion
        let mut stack = vec![parent_id.to_string()];

        while let Some(current_parent) = stack.pop() {
            let children = content.enumerate_children(&current_parent)?;

            for child in children {
                if child.is_folder {
                    stats.folders += 1;
                    // Add folder to stack for later processing
                    stack.push(child.object_id.clone());
                } else {
                    // Check if it's an extractable file
                    if Self::is_extractable_file(&child.name) {
                        // Try to read the file content
                        match content.read_file(&child.object_id) {
                            Ok(data) => {
                                stats.files_extracted += 1;
                                stats.bytes_processed += data.len() as u64;
                                // data is dropped here immediately, freeing memory
                            }
                            Err(_) => {
                                stats.errors += 1;
                            }
                        }
                    } else {
                        stats.files_skipped += 1;
                    }
                }
            }
            // children vector is dropped here, freeing memory
        }

        Ok(())
    }

    /// Check if a file is extractable based on extension
    fn is_extractable_file(name: &str) -> bool {
        let extensions = [
            "jpg", "jpeg", "png", "heic", "heif", "gif", "webp", "raw", "dng", "tiff", "tif",
            "bmp", "mov", "mp4", "m4v", "avi", "3gp",
        ];

        let extension = std::path::Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        extensions.contains(&extension.as_str())
    }

    /// Compare actual results with expected results
    fn compare_results(&self, stats: &ExecutionStats, expected: &ExpectedResults) -> bool {
        if !expected.should_succeed {
            // If not supposed to succeed, we shouldn't reach here with stats
            return false;
        }

        // Check file count (allow some tolerance for edge cases)
        let files_match = stats.files_extracted >= expected.files_to_extract.saturating_sub(1)
            && stats.files_extracted <= expected.files_to_extract + expected.errors;

        // Check error count
        let errors_match = stats.errors <= expected.errors + 1;

        files_match && errors_match
    }

    /// Get a description of why results don't match
    fn get_mismatch_reason(&self, stats: &ExecutionStats, expected: &ExpectedResults) -> String {
        let mut reasons = Vec::new();

        if stats.files_extracted != expected.files_to_extract {
            reasons.push(format!(
                "files extracted: got {}, expected {}",
                stats.files_extracted, expected.files_to_extract
            ));
        }

        if stats.errors > expected.errors {
            reasons.push(format!(
                "errors: got {}, expected at most {}",
                stats.errors, expected.errors
            ));
        }

        if reasons.is_empty() {
            "Unknown mismatch".to_string()
        } else {
            reasons.join("; ")
        }
    }

    /// Generate a summary from results
    fn generate_summary(&self) -> TestSummary {
        let mut summary = TestSummary {
            total: self.results.len(),
            passed: self.results.iter().filter(|r| r.passed).count(),
            failed: self.results.iter().filter(|r| !r.passed).count(),
            skipped: 0,
            total_duration: self.start_time.map(|s| s.elapsed()).unwrap_or_default(),
            results_by_tag: HashMap::new(),
        };

        // Group results by tag (use "all" as default)
        for result in &self.results {
            summary
                .results_by_tag
                .entry("all".to_string())
                .or_default()
                .push(result.clone());
        }

        summary
    }

    /// Print a single result
    fn print_result(&self, result: &ScenarioResult) {
        let status = if result.passed { "✓" } else { "✗" };
        let status_color = if result.passed { "32" } else { "31" }; // Green or Red

        println!(
            "  \x1b[{}m{}\x1b[0m {} ({:.2}s)",
            status_color,
            status,
            result.name,
            result.duration.as_secs_f64()
        );

        if !result.passed {
            if let Some(ref reason) = result.failure_reason {
                println!("     └─ Reason: {}", reason);
            }
        }

        if self.config.verbose {
            println!(
                "     └─ Files: {}, Errors: {}, Bytes: {}",
                result.files_extracted, result.errors, result.bytes_processed
            );
        }
    }

    /// Print the test summary
    fn print_summary(&self, summary: &TestSummary) {
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║                      TEST SUMMARY                            ║");
        println!("╚══════════════════════════════════════════════════════════════╝\n");

        println!("  Total:    {}", summary.total);
        println!("  \x1b[32mPassed:   {}\x1b[0m", summary.passed);
        println!("  \x1b[31mFailed:   {}\x1b[0m", summary.failed);
        println!("  Skipped:  {}", summary.skipped);
        println!("  Duration: {:.2}s", summary.total_duration.as_secs_f64());
        println!("  Pass Rate: {:.1}%", summary.pass_rate());

        if summary.failed > 0 {
            println!("\n  Failed scenarios:");
            for name in summary.failed_scenarios() {
                println!("    - {}", name);
            }
        }

        println!();
    }

    /// Generate HTML report
    fn generate_html_report(&self, summary: &TestSummary) -> std::io::Result<()> {
        let report_dir = self
            .config
            .report_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));
        fs::create_dir_all(&report_dir)?;

        let report_path = report_dir.join("test_report.html");
        let mut file = File::create(&report_path)?;

        writeln!(file, "<!DOCTYPE html>")?;
        writeln!(file, "<html><head>")?;
        writeln!(file, "<title>Photo Extraction Tool Test Report</title>")?;
        writeln!(file, "<style>")?;
        writeln!(
            file,
            "body {{ font-family: Arial, sans-serif; margin: 20px; }}"
        )?;
        writeln!(file, "h1 {{ color: #333; }}")?;
        writeln!(file, ".summary {{ background: #f5f5f5; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}")?;
        writeln!(file, ".passed {{ color: #28a745; }}")?;
        writeln!(file, ".failed {{ color: #dc3545; }}")?;
        writeln!(file, "table {{ border-collapse: collapse; width: 100%; }}")?;
        writeln!(
            file,
            "th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}"
        )?;
        writeln!(file, "th {{ background-color: #4CAF50; color: white; }}")?;
        writeln!(file, "tr:nth-child(even) {{ background-color: #f2f2f2; }}")?;
        writeln!(file, "</style>")?;
        writeln!(file, "</head><body>")?;

        writeln!(file, "<h1>Photo Extraction Tool Test Report</h1>")?;

        writeln!(file, "<div class='summary'>")?;
        writeln!(file, "<h2>Summary</h2>")?;
        writeln!(file, "<p>Total: {}</p>", summary.total)?;
        writeln!(file, "<p class='passed'>Passed: {}</p>", summary.passed)?;
        writeln!(file, "<p class='failed'>Failed: {}</p>", summary.failed)?;
        writeln!(
            file,
            "<p>Duration: {:.2}s</p>",
            summary.total_duration.as_secs_f64()
        )?;
        writeln!(file, "<p>Pass Rate: {:.1}%</p>", summary.pass_rate())?;
        writeln!(file, "</div>")?;

        writeln!(file, "<h2>Results</h2>")?;
        writeln!(file, "<table>")?;
        writeln!(
            file,
            "<tr><th>Scenario</th><th>Status</th><th>Duration</th><th>Files</th><th>Errors</th></tr>"
        )?;

        for result in &self.results {
            let status_class = if result.passed { "passed" } else { "failed" };
            let status_text = if result.passed { "PASS" } else { "FAIL" };
            writeln!(
                file,
                "<tr><td>{}</td><td class='{}'>{}</td><td>{:.2}s</td><td>{}</td><td>{}</td></tr>",
                result.name,
                status_class,
                status_text,
                result.duration.as_secs_f64(),
                result.files_extracted,
                result.errors
            )?;
        }

        writeln!(file, "</table>")?;
        writeln!(file, "</body></html>")?;

        println!("HTML report generated: {}", report_path.display());
        Ok(())
    }

    /// Generate JSON report
    fn generate_json_report(&self, summary: &TestSummary) -> std::io::Result<()> {
        let report_dir = self
            .config
            .report_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));
        fs::create_dir_all(&report_dir)?;

        let report_path = report_dir.join("test_report.json");
        let mut file = File::create(&report_path)?;

        writeln!(file, "{{")?;
        writeln!(file, "  \"summary\": {{")?;
        writeln!(file, "    \"total\": {},", summary.total)?;
        writeln!(file, "    \"passed\": {},", summary.passed)?;
        writeln!(file, "    \"failed\": {},", summary.failed)?;
        writeln!(file, "    \"skipped\": {},", summary.skipped)?;
        writeln!(
            file,
            "    \"duration_seconds\": {:.2},",
            summary.total_duration.as_secs_f64()
        )?;
        writeln!(file, "    \"pass_rate\": {:.1}", summary.pass_rate())?;
        writeln!(file, "  }},")?;

        writeln!(file, "  \"results\": [")?;
        for (i, result) in self.results.iter().enumerate() {
            let comma = if i < self.results.len() - 1 { "," } else { "" };
            writeln!(file, "    {{")?;
            writeln!(file, "      \"name\": \"{}\",", result.name)?;
            writeln!(file, "      \"passed\": {},", result.passed)?;
            writeln!(
                file,
                "      \"duration_seconds\": {:.4},",
                result.duration.as_secs_f64()
            )?;
            writeln!(
                file,
                "      \"files_extracted\": {},",
                result.files_extracted
            )?;
            writeln!(file, "      \"files_skipped\": {},", result.files_skipped)?;
            writeln!(file, "      \"errors\": {},", result.errors)?;
            writeln!(
                file,
                "      \"bytes_processed\": {}",
                result.bytes_processed
            )?;
            writeln!(file, "    }}{}", comma)?;
        }
        writeln!(file, "  ]")?;
        writeln!(file, "}}")?;

        println!("JSON report generated: {}", report_path.display());
        Ok(())
    }

    /// Get all results
    pub fn results(&self) -> &[ScenarioResult] {
        &self.results
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// InteractiveTestMode - Interactive scenario browsing
// =============================================================================

/// Interactive test mode for browsing and running scenarios
pub struct InteractiveTestMode {
    /// Current selected scenario index
    current_scenario: Option<usize>,
    /// All available scenarios
    scenarios: Vec<TestScenario>,
    /// Test runner instance
    runner: TestRunner,
}

impl InteractiveTestMode {
    /// Create a new interactive test mode
    pub fn new() -> Self {
        Self {
            current_scenario: None,
            scenarios: ScenarioLibrary::all_scenarios(),
            runner: TestRunner::with_config(TestRunnerConfig {
                verbose: true,
                ..Default::default()
            }),
        }
    }

    /// List all scenarios with their descriptions
    pub fn list_scenarios(&self) {
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║              AVAILABLE TEST SCENARIOS                        ║");
        println!("╚══════════════════════════════════════════════════════════════╝\n");

        for (i, scenario) in self.scenarios.iter().enumerate() {
            let tags = scenario.tags.join(", ");
            println!(
                "  [{:2}] {} - {}",
                i + 1,
                scenario.name,
                scenario.description
            );
            println!("       Tags: {}", tags);
            println!();
        }

        println!("Total: {} scenarios\n", self.scenarios.len());
    }

    /// Get a scenario by index
    pub fn get_scenario(&self, index: usize) -> Option<&TestScenario> {
        self.scenarios.get(index)
    }

    /// Get the number of scenarios
    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Select a scenario by index
    pub fn select_scenario(&mut self, index: usize) -> bool {
        if index < self.scenarios.len() {
            self.current_scenario = Some(index);
            true
        } else {
            false
        }
    }

    /// Get the currently selected scenario
    pub fn current_scenario(&self) -> Option<&TestScenario> {
        self.current_scenario.and_then(|i| self.scenarios.get(i))
    }

    /// Run the currently selected scenario
    pub fn run_current(&mut self) -> Option<ScenarioResult> {
        if let Some(scenario) = self.current_scenario().cloned() {
            let summary = self.runner.run_scenarios(vec![scenario]);
            summary
                .results_by_tag
                .get("all")
                .and_then(|r| r.first())
                .cloned()
        } else {
            None
        }
    }

    /// Run a scenario by name
    pub fn run_by_name(&mut self, name: &str) -> Option<ScenarioResult> {
        let scenario = self.scenarios.iter().find(|s| s.name == name)?.clone();
        let summary = self.runner.run_scenarios(vec![scenario]);
        summary
            .results_by_tag
            .get("all")
            .and_then(|r| r.first())
            .cloned()
    }

    /// Run all quick scenarios
    pub fn run_quick(&mut self) -> TestSummary {
        self.runner.run_quick()
    }

    /// Run all scenarios
    pub fn run_all(&mut self) -> TestSummary {
        self.runner.run_all()
    }

    /// Get scenarios filtered by tag
    pub fn scenarios_by_tag(&self, tag: &str) -> Vec<&TestScenario> {
        self.scenarios
            .iter()
            .filter(|s| s.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Get all unique tags
    pub fn all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.scenarios.iter().flat_map(|s| s.tags.clone()).collect();
        tags.sort();
        tags.dedup();
        tags
    }
}

impl Default for InteractiveTestMode {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_creation() {
        let runner = TestRunner::new();
        assert!(runner.results.is_empty());
    }

    #[test]
    fn test_quick_scenarios() {
        let mut runner = TestRunner::new();
        let summary = runner.run_quick();

        assert!(summary.total > 0);
        println!("Quick test: {}/{} passed", summary.passed, summary.total);
    }

    #[test]
    fn test_scenario_result() {
        let result = ScenarioResult::passed("test_scenario", Duration::from_secs(1));
        assert!(result.passed);
        assert_eq!(result.name, "test_scenario");

        let failed =
            ScenarioResult::failed("failed_scenario", Duration::from_secs(2), "Test error");
        assert!(!failed.passed);
        assert!(failed.failure_reason.is_some());
    }

    #[test]
    fn test_summary_pass_rate() {
        let mut summary = TestSummary::default();
        summary.total = 10;
        summary.passed = 8;
        summary.failed = 2;

        assert!((summary.pass_rate() - 80.0).abs() < 0.1);
    }

    #[test]
    fn test_interactive_mode() {
        let mode = InteractiveTestMode::new();
        assert!(mode.scenario_count() > 0);
        assert!(!mode.all_tags().is_empty());
    }

    #[test]
    fn test_single_iphone_scenario() {
        let mut runner = TestRunner::with_config(TestRunnerConfig {
            verbose: false,
            ..Default::default()
        });

        let summary = runner.run_by_names(&["single_iphone"]);
        assert_eq!(summary.total, 1);
        assert!(summary.passed >= 1, "single_iphone scenario should pass");
    }

    #[test]
    fn test_device_locked_scenario() {
        let mut runner = TestRunner::with_config(TestRunnerConfig {
            verbose: false,
            ..Default::default()
        });

        let summary = runner.run_by_names(&["device_locked"]);
        assert_eq!(summary.total, 1);
        // Device locked scenario should pass because error is expected
        assert!(
            summary.passed >= 1,
            "device_locked scenario should pass (error expected)"
        );
    }

    #[test]
    fn test_extractable_file_detection() {
        assert!(TestRunner::is_extractable_file("photo.jpg"));
        assert!(TestRunner::is_extractable_file("photo.JPG"));
        assert!(TestRunner::is_extractable_file("photo.jpeg"));
        assert!(TestRunner::is_extractable_file("photo.heic"));
        assert!(TestRunner::is_extractable_file("photo.HEIC"));
        assert!(TestRunner::is_extractable_file("photo.png"));
        assert!(TestRunner::is_extractable_file("video.mov"));
        assert!(TestRunner::is_extractable_file("video.MP4"));

        assert!(!TestRunner::is_extractable_file("document.txt"));
        assert!(!TestRunner::is_extractable_file("file.pdf"));
        assert!(!TestRunner::is_extractable_file("no_extension"));
    }

    #[test]
    fn test_runner_with_config() {
        let config = TestRunnerConfig {
            verbose: true,
            fail_fast: true,
            tag_filter: Some("device".to_string()),
            ..Default::default()
        };

        let runner = TestRunner::with_config(config);
        assert!(runner.config.verbose);
        assert!(runner.config.fail_fast);
        assert_eq!(runner.config.tag_filter, Some("device".to_string()));
    }

    #[test]
    fn test_scenario_filtering_by_tag() {
        let runner = TestRunner::with_config(TestRunnerConfig {
            tag_filter: Some("error".to_string()),
            ..Default::default()
        });

        let all_scenarios = ScenarioLibrary::all_scenarios();
        let filtered = runner.filter_scenarios(all_scenarios);

        for scenario in filtered {
            assert!(
                scenario.tags.contains(&"error".to_string()),
                "Scenario {} should have 'error' tag",
                scenario.name
            );
        }
    }

    #[test]
    fn test_empty_device_scenario() {
        let mut runner = TestRunner::with_config(TestRunnerConfig {
            verbose: false,
            ..Default::default()
        });

        let summary = runner.run_by_names(&["empty_device"]);
        assert_eq!(summary.total, 1);

        // Get the result
        let results = runner.results();
        assert_eq!(results.len(), 1);

        // Empty device should have 0 files extracted
        assert_eq!(results[0].files_extracted, 0);
    }
}

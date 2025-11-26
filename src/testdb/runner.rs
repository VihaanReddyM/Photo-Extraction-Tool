//! Test runner for executing scenarios and generating reports
//!
//! This module provides functionality to run test scenarios, track results,
//! and generate detailed reports for testing the photo extraction tool.

use super::mock_device::MockDeviceManager;
use super::scenarios::{ExpectedResults, ScenarioLibrary, TestScenario};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

/// Result of running a single test scenario
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    /// Scenario name
    pub name: String,
    /// Whether the test passed
    pub passed: bool,
    /// Execution time
    pub duration: Duration,
    /// Files extracted (if applicable)
    pub files_extracted: usize,
    /// Files skipped
    pub files_skipped: usize,
    /// Duplicates found
    pub duplicates_found: usize,
    /// Errors encountered
    pub errors: usize,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Detailed message
    pub message: String,
    /// Expected results for comparison
    pub expected: ExpectedResults,
    /// Failure reason (if any)
    pub failure_reason: Option<String>,
}

impl ScenarioResult {
    /// Create a new passing result
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
            message: "Test passed".to_string(),
            expected: ExpectedResults::default(),
            failure_reason: None,
        }
    }

    /// Create a new failing result
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
            message: format!("Test failed: {}", reason),
            expected: ExpectedResults::default(),
            failure_reason: Some(reason.to_string()),
        }
    }

    /// Set extraction statistics
    pub fn with_stats(
        mut self,
        extracted: usize,
        skipped: usize,
        duplicates: usize,
        errors: usize,
        bytes: u64,
    ) -> Self {
        self.files_extracted = extracted;
        self.files_skipped = skipped;
        self.duplicates_found = duplicates;
        self.errors = errors;
        self.bytes_processed = bytes;
        self
    }

    /// Set expected results for comparison
    pub fn with_expected(mut self, expected: ExpectedResults) -> Self {
        self.expected = expected;
        self
    }
}

/// Summary of test run results
#[derive(Debug, Clone, Default)]
pub struct TestSummary {
    /// Total scenarios run
    pub total: usize,
    /// Scenarios that passed
    pub passed: usize,
    /// Scenarios that failed
    pub failed: usize,
    /// Scenarios that were skipped
    pub skipped: usize,
    /// Total execution time
    pub total_duration: Duration,
    /// Results grouped by tag
    pub results_by_tag: HashMap<String, Vec<ScenarioResult>>,
}

impl TestSummary {
    /// Calculate pass rate as percentage
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            (self.passed as f64 / self.total as f64) * 100.0
        }
    }

    /// Get all failed scenario names
    pub fn failed_scenarios(&self) -> Vec<&str> {
        self.results_by_tag
            .values()
            .flatten()
            .filter(|r| !r.passed)
            .map(|r| r.name.as_str())
            .collect()
    }
}

/// Configuration for test runner
#[derive(Debug, Clone)]
pub struct TestRunnerConfig {
    /// Whether to run in verbose mode
    pub verbose: bool,
    /// Whether to stop on first failure
    pub fail_fast: bool,
    /// Filter scenarios by tags
    pub tag_filter: Option<Vec<String>>,
    /// Filter scenarios by name pattern
    pub name_filter: Option<String>,
    /// Output directory for reports
    pub report_dir: Option<String>,
    /// Whether to generate HTML report
    pub html_report: bool,
    /// Whether to generate JSON report
    pub json_report: bool,
    /// Timeout per scenario (seconds)
    pub timeout_seconds: u64,
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
        }
    }
}

/// Test runner for executing scenarios
pub struct TestRunner {
    /// Configuration
    config: TestRunnerConfig,
    /// Results from test runs
    results: Vec<ScenarioResult>,
    /// Start time of test run
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

    /// Create a new test runner with configuration
    pub fn with_config(config: TestRunnerConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
            start_time: None,
        }
    }

    /// Run all available scenarios
    pub fn run_all(&mut self) -> TestSummary {
        let scenarios = ScenarioLibrary::all_scenarios();
        self.run_scenarios(scenarios)
    }

    /// Run quick test scenarios only
    pub fn run_quick(&mut self) -> TestSummary {
        let scenarios = ScenarioLibrary::quick_scenarios();
        self.run_scenarios(scenarios)
    }

    /// Run scenarios filtered by tag
    pub fn run_by_tag(&mut self, tag: &str) -> TestSummary {
        let scenarios = ScenarioLibrary::scenarios_by_tag(tag);
        self.run_scenarios(scenarios)
    }

    /// Run specific scenarios by name
    pub fn run_by_names(&mut self, names: &[&str]) -> TestSummary {
        let all_scenarios = ScenarioLibrary::all_scenarios();
        let scenarios: Vec<_> = all_scenarios
            .into_iter()
            .filter(|s| names.contains(&s.name.as_str()))
            .collect();
        self.run_scenarios(scenarios)
    }

    /// Run a list of scenarios
    pub fn run_scenarios(&mut self, scenarios: Vec<TestScenario>) -> TestSummary {
        self.start_time = Some(Instant::now());
        self.results.clear();

        let filtered_scenarios = self.filter_scenarios(scenarios);

        if self.config.verbose {
            println!("\n╔══════════════════════════════════════════════════════════════╗");
            println!("║              PHOTO EXTRACTION TOOL - TEST RUNNER             ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!(
                "║  Running {} scenario(s)                                       ",
                filtered_scenarios.len()
            );
            println!("╚══════════════════════════════════════════════════════════════╝\n");
        }

        for scenario in filtered_scenarios {
            let result = self.run_single_scenario(scenario);

            if self.config.verbose {
                self.print_result(&result);
            }

            let should_stop = self.config.fail_fast && !result.passed;
            self.results.push(result);

            if should_stop {
                if self.config.verbose {
                    println!("\n⚠️  Stopping early due to fail-fast mode\n");
                }
                break;
            }
        }

        let summary = self.generate_summary();

        if self.config.verbose {
            self.print_summary(&summary);
        }

        // Generate reports if configured
        if let Some(ref dir) = self.config.report_dir {
            if self.config.html_report {
                let _ = self.generate_html_report(dir, &summary);
            }
            if self.config.json_report {
                let _ = self.generate_json_report(dir, &summary);
            }
        }

        summary
    }

    /// Filter scenarios based on configuration
    fn filter_scenarios(&self, scenarios: Vec<TestScenario>) -> Vec<TestScenario> {
        let mut filtered = scenarios;

        // Filter by tags
        if let Some(ref tags) = self.config.tag_filter {
            filtered = filtered
                .into_iter()
                .filter(|s| s.tags.iter().any(|t| tags.contains(t)))
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

    /// Run a single scenario
    fn run_single_scenario(&self, scenario: TestScenario) -> ScenarioResult {
        let start = Instant::now();

        if self.config.verbose {
            println!("▶ Running: {} - {}", scenario.name, scenario.description);
        }

        // Execute the scenario
        let result = self.execute_scenario(&scenario);

        let duration = start.elapsed();

        match result {
            Ok(stats) => {
                // Compare with expected results
                let passed = self.compare_results(&stats, &scenario.expected);

                let mut result = if passed {
                    ScenarioResult::passed(&scenario.name, duration)
                } else {
                    ScenarioResult::failed(
                        &scenario.name,
                        duration,
                        "Results did not match expected values",
                    )
                };

                result = result
                    .with_stats(
                        stats.files_extracted,
                        stats.files_skipped,
                        stats.duplicates_found,
                        stats.errors,
                        stats.bytes_processed,
                    )
                    .with_expected(scenario.expected);

                result
            }
            Err(error) => {
                // Check if error was expected
                if !scenario.expected.should_succeed {
                    if let Some(ref expected_error) = scenario.expected.expected_error {
                        if error.contains(expected_error) {
                            return ScenarioResult::passed(&scenario.name, duration)
                                .with_expected(scenario.expected);
                        }
                    }
                }

                ScenarioResult::failed(&scenario.name, duration, &error)
                    .with_expected(scenario.expected)
            }
        }
    }

    /// Execute a scenario and return statistics
    fn execute_scenario(&self, scenario: &TestScenario) -> Result<ExecutionStats, String> {
        // Create a mock device manager with the scenario's device and file system
        let mut manager = MockDeviceManager::new();

        // Clone the file system from the scenario
        let fs = scenario.file_system.clone();

        manager.add_device(scenario.device_info.clone(), fs);

        // Try to open the device
        let file_system = manager
            .open_device(&scenario.device_info.device_id)
            .map_err(|e| format!("{:?}", e))?;

        // Simulate extraction by traversing the file system
        let fs_guard = file_system.read().map_err(|e| e.to_string())?;

        let mut stats = ExecutionStats::default();

        // Count files and folders
        for obj in fs_guard.all_objects() {
            if obj.is_folder {
                stats.folders += 1;
            } else {
                // Simulate extraction based on file type
                if Self::is_extractable_file(&obj.name) {
                    stats.files_extracted += 1;
                    stats.bytes_processed += obj.size;
                } else {
                    stats.files_skipped += 1;
                }
            }
        }

        Ok(stats)
    }

    /// Check if a file is extractable based on extension
    fn is_extractable_file(name: &str) -> bool {
        let extensions = [
            "jpg", "jpeg", "png", "heic", "heif", "gif", "webp", "raw", "dng", "tiff", "tif",
            "bmp", "mov", "mp4", "m4v", "avi", "3gp",
        ];

        let lower = name.to_lowercase();
        extensions.iter().any(|ext| lower.ends_with(ext))
    }

    /// Compare actual results with expected results
    fn compare_results(&self, actual: &ExecutionStats, expected: &ExpectedResults) -> bool {
        if expected.files_to_extract > 0 && actual.files_extracted != expected.files_to_extract {
            return false;
        }
        // Add more comparisons as needed
        true
    }

    /// Generate summary from results
    fn generate_summary(&self) -> TestSummary {
        let mut summary = TestSummary::default();

        summary.total = self.results.len();
        summary.passed = self.results.iter().filter(|r| r.passed).count();
        summary.failed = self.results.iter().filter(|r| !r.passed).count();
        summary.total_duration = self
            .start_time
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO);

        // Group by tags
        for result in &self.results {
            summary
                .results_by_tag
                .entry("all".to_string())
                .or_default()
                .push(result.clone());
        }

        summary
    }

    /// Print a single result to console
    fn print_result(&self, result: &ScenarioResult) {
        let status = if result.passed {
            "✓ PASS"
        } else {
            "✗ FAIL"
        };
        let status_color = if result.passed {
            "\x1b[32m"
        } else {
            "\x1b[31m"
        };

        println!(
            "  {}{}\x1b[0m - {} ({:.2}ms)",
            status_color,
            status,
            result.name,
            result.duration.as_secs_f64() * 1000.0
        );

        if !result.passed {
            if let Some(ref reason) = result.failure_reason {
                println!("      └─ Reason: {}", reason);
            }
        }

        if self.config.verbose && result.files_extracted > 0 {
            println!(
                "      └─ Files: {} extracted, {} skipped, {} duplicates, {} errors",
                result.files_extracted,
                result.files_skipped,
                result.duplicates_found,
                result.errors
            );
        }
    }

    /// Print summary to console
    fn print_summary(&self, summary: &TestSummary) {
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║                        TEST SUMMARY                          ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!(
            "║  Total:    {:>4}                                              ║",
            summary.total
        );
        println!(
            "║  Passed:   {:>4} \x1b[32m✓\x1b[0m                                             ║",
            summary.passed
        );
        println!(
            "║  Failed:   {:>4} \x1b[31m✗\x1b[0m                                             ║",
            summary.failed
        );
        println!(
            "║  Skipped:  {:>4}                                              ║",
            summary.skipped
        );
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!(
            "║  Pass Rate: {:>5.1}%                                          ║",
            summary.pass_rate()
        );
        println!(
            "║  Duration:  {:>5.2}s                                          ║",
            summary.total_duration.as_secs_f64()
        );
        println!("╚══════════════════════════════════════════════════════════════╝\n");

        if !summary.failed_scenarios().is_empty() {
            println!("Failed scenarios:");
            for name in summary.failed_scenarios() {
                println!("  • {}", name);
            }
            println!();
        }
    }

    /// Generate HTML report
    fn generate_html_report(&self, dir: &str, summary: &TestSummary) -> std::io::Result<()> {
        let path = Path::new(dir).join("test_report.html");
        let mut file = File::create(&path)?;

        let mut html = String::new();

        writeln!(html, "<!DOCTYPE html>").unwrap();
        writeln!(html, "<html><head>").unwrap();
        writeln!(html, "<title>Photo Extraction Tool - Test Report</title>").unwrap();
        writeln!(html, "<style>").unwrap();
        writeln!(
            html,
            "body {{ font-family: Arial, sans-serif; margin: 20px; }}"
        )
        .unwrap();
        writeln!(html, ".pass {{ color: green; }}").unwrap();
        writeln!(html, ".fail {{ color: red; }}").unwrap();
        writeln!(html, "table {{ border-collapse: collapse; width: 100%; }}").unwrap();
        writeln!(
            html,
            "th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}"
        )
        .unwrap();
        writeln!(html, "th {{ background-color: #4CAF50; color: white; }}").unwrap();
        writeln!(html, "tr:nth-child(even) {{ background-color: #f2f2f2; }}").unwrap();
        writeln!(html, "</style>").unwrap();
        writeln!(html, "</head><body>").unwrap();

        writeln!(html, "<h1>Photo Extraction Tool - Test Report</h1>").unwrap();
        writeln!(
            html,
            "<p>Generated: {}</p>",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )
        .unwrap();

        writeln!(html, "<h2>Summary</h2>").unwrap();
        writeln!(
            html,
            "<ul>
            <li>Total: {}</li>
            <li>Passed: <span class=\"pass\">{}</span></li>
            <li>Failed: <span class=\"fail\">{}</span></li>
            <li>Pass Rate: {:.1}%</li>
            <li>Duration: {:.2}s</li>
            </ul>",
            summary.total,
            summary.passed,
            summary.failed,
            summary.pass_rate(),
            summary.total_duration.as_secs_f64()
        )
        .unwrap();

        writeln!(html, "<h2>Results</h2>").unwrap();
        writeln!(html, "<table>").unwrap();
        writeln!(
            html,
            "<tr><th>Status</th><th>Scenario</th><th>Duration</th><th>Files</th><th>Errors</th></tr>"
        )
        .unwrap();

        for result in &self.results {
            let status_class = if result.passed { "pass" } else { "fail" };
            let status_text = if result.passed { "PASS" } else { "FAIL" };
            writeln!(
                html,
                "<tr>
                <td class=\"{}\">{}</td>
                <td>{}</td>
                <td>{:.2}ms</td>
                <td>{}</td>
                <td>{}</td>
                </tr>",
                status_class,
                status_text,
                result.name,
                result.duration.as_secs_f64() * 1000.0,
                result.files_extracted,
                result.errors
            )
            .unwrap();
        }

        writeln!(html, "</table>").unwrap();
        writeln!(html, "</body></html>").unwrap();

        file.write_all(html.as_bytes())?;

        if self.config.verbose {
            println!("📄 HTML report generated: {}", path.display());
        }

        Ok(())
    }

    /// Generate JSON report
    fn generate_json_report(&self, dir: &str, summary: &TestSummary) -> std::io::Result<()> {
        let path = Path::new(dir).join("test_report.json");
        let mut file = File::create(&path)?;

        let mut json = String::new();

        writeln!(json, "{{").unwrap();
        writeln!(json, "  \"summary\": {{").unwrap();
        writeln!(json, "    \"total\": {},", summary.total).unwrap();
        writeln!(json, "    \"passed\": {},", summary.passed).unwrap();
        writeln!(json, "    \"failed\": {},", summary.failed).unwrap();
        writeln!(json, "    \"pass_rate\": {:.2},", summary.pass_rate()).unwrap();
        writeln!(
            json,
            "    \"duration_seconds\": {:.3}",
            summary.total_duration.as_secs_f64()
        )
        .unwrap();
        writeln!(json, "  }},").unwrap();
        writeln!(json, "  \"results\": [").unwrap();

        for (i, result) in self.results.iter().enumerate() {
            let comma = if i < self.results.len() - 1 { "," } else { "" };
            writeln!(json, "    {{").unwrap();
            writeln!(json, "      \"name\": \"{}\",", result.name).unwrap();
            writeln!(json, "      \"passed\": {},", result.passed).unwrap();
            writeln!(
                json,
                "      \"duration_ms\": {:.2},",
                result.duration.as_secs_f64() * 1000.0
            )
            .unwrap();
            writeln!(
                json,
                "      \"files_extracted\": {},",
                result.files_extracted
            )
            .unwrap();
            writeln!(json, "      \"errors\": {}", result.errors).unwrap();
            writeln!(json, "    }}{}", comma).unwrap();
        }

        writeln!(json, "  ]").unwrap();
        writeln!(json, "}}").unwrap();

        file.write_all(json.as_bytes())?;

        if self.config.verbose {
            println!("📄 JSON report generated: {}", path.display());
        }

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

/// Statistics from scenario execution
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    /// Files successfully extracted
    pub files_extracted: usize,
    /// Files skipped
    pub files_skipped: usize,
    /// Duplicates found
    pub duplicates_found: usize,
    /// Errors encountered
    pub errors: usize,
    /// Bytes processed
    pub bytes_processed: u64,
    /// Folders encountered
    pub folders: usize,
}

/// Interactive test mode for manual testing
pub struct InteractiveTestMode {
    /// Current scenario index
    current_scenario: usize,
    /// Available scenarios
    scenarios: Vec<TestScenario>,
    /// Test runner
    runner: TestRunner,
}

impl InteractiveTestMode {
    /// Create new interactive mode
    pub fn new() -> Self {
        Self {
            current_scenario: 0,
            scenarios: ScenarioLibrary::all_scenarios(),
            runner: TestRunner::with_config(TestRunnerConfig {
                verbose: true,
                ..Default::default()
            }),
        }
    }

    /// List all available scenarios
    pub fn list_scenarios(&self) {
        println!("\n📋 Available Test Scenarios:\n");
        for (i, scenario) in self.scenarios.iter().enumerate() {
            let marker = if i == self.current_scenario {
                "▶"
            } else {
                " "
            };
            println!(
                "{} {:2}. {} - {}",
                marker,
                i + 1,
                scenario.name,
                scenario.description
            );
            if !scenario.tags.is_empty() {
                println!("      Tags: {}", scenario.tags.join(", "));
            }
        }
        println!();
    }

    /// Get scenario by name
    pub fn get_scenario(&self, name: &str) -> Option<&TestScenario> {
        self.scenarios.iter().find(|s| s.name == name)
    }

    /// Get scenario count
    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Select a scenario by index
    pub fn select_scenario(&mut self, index: usize) -> bool {
        if index < self.scenarios.len() {
            self.current_scenario = index;
            true
        } else {
            false
        }
    }

    /// Get current scenario
    pub fn current_scenario(&self) -> Option<&TestScenario> {
        self.scenarios.get(self.current_scenario)
    }

    /// Run current scenario
    pub fn run_current(&mut self) -> Option<ScenarioResult> {
        if let Some(scenario) = self.scenarios.get(self.current_scenario).cloned() {
            let summary = self.runner.run_scenarios(vec![scenario]);
            summary
                .results_by_tag
                .get("all")
                .and_then(|r| r.first().cloned())
        } else {
            None
        }
    }
}

impl Default for InteractiveTestMode {
    fn default() -> Self {
        Self::new()
    }
}

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
        let mut runner = TestRunner::with_config(TestRunnerConfig {
            verbose: false,
            ..Default::default()
        });

        let summary = runner.run_quick();
        assert!(summary.total > 0);
    }

    #[test]
    fn test_scenario_result() {
        let result = ScenarioResult::passed("test", Duration::from_millis(100));
        assert!(result.passed);
        assert_eq!(result.name, "test");

        let failed = ScenarioResult::failed("test2", Duration::from_millis(50), "some error");
        assert!(!failed.passed);
        assert_eq!(failed.failure_reason, Some("some error".to_string()));
    }

    #[test]
    fn test_summary_pass_rate() {
        let mut summary = TestSummary::default();
        summary.total = 10;
        summary.passed = 8;
        summary.failed = 2;

        assert!((summary.pass_rate() - 80.0).abs() < 0.001);
    }

    #[test]
    fn test_interactive_mode() {
        let mode = InteractiveTestMode::new();
        assert!(mode.scenario_count() > 0);
        assert!(mode.current_scenario().is_some());
    }
}

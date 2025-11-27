//! Command-line argument definitions
//!
//! This module defines all CLI arguments and subcommands using clap.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// A fast, reliable tool to extract photos from iOS devices (iPhone/iPad) on Windows
#[derive(Parser, Debug)]
#[command(name = "photo_extraction_tool")]
#[command(author = "Vihaan Reddy M")]
#[command(version = "1.0.0")]
#[command(about = "Extract photos from iOS devices (iPhone/iPad) on Windows — no iTunes or drivers required", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to configuration file
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Output directory for extracted photos (overrides config)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Device ID to extract from (overrides config)
    #[arg(short, long)]
    pub device_id: Option<String>,

    /// Only extract photos from DCIM folder (overrides config)
    #[arg(long)]
    pub dcim_only: Option<bool>,

    /// Preserve folder structure from device (overrides config)
    #[arg(short, long)]
    pub preserve_structure: Option<bool>,

    /// Skip files that already exist in destination (overrides config)
    #[arg(short, long)]
    pub skip_existing: Option<bool>,

    /// Enable duplicate detection using SHA256 hashing
    #[arg(long)]
    pub detect_duplicates: bool,

    /// Folder(s) to compare against for duplicates (can be specified multiple times)
    #[arg(long = "compare-to", value_name = "FOLDER")]
    pub compare_folders: Vec<PathBuf>,

    /// Action to take when a duplicate is found: skip, rename, or overwrite
    #[arg(long, value_name = "ACTION", value_parser = ["skip", "rename", "overwrite"])]
    pub duplicate_action: Option<String>,

    /// List all MTP-compatible devices (not just Apple/iOS devices)
    #[arg(long)]
    pub all_devices: bool,

    /// Log level: error, warn, info, debug, trace (overrides config)
    #[arg(short, long)]
    pub log_level: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Extract photos from the connected device
    Extract {
        /// Enable duplicate detection using SHA256 hashing
        #[arg(long)]
        detect_duplicates: bool,

        /// Folder(s) to compare against for duplicates (can be specified multiple times)
        #[arg(long = "compare-to", value_name = "FOLDER")]
        compare_folders: Vec<PathBuf>,

        /// Action to take when a duplicate is found: skip, rename, or overwrite
        #[arg(long, value_name = "ACTION", value_parser = ["skip", "rename", "overwrite"])]
        duplicate_action: Option<String>,
    },

    /// List connected devices
    List {
        /// Show all MTP-compatible devices, not just Apple/iOS devices
        #[arg(long)]
        all: bool,
    },

    /// Open the configuration file in your default editor
    ///
    /// The config file is stored at:
    /// - Windows: %APPDATA%\photo_extraction_tool\config.toml
    /// - Linux/macOS: ~/.config/photo_extraction_tool/config.toml
    ///
    /// If no config file exists, a default one will be created.
    Config {
        /// Show the config file path without opening it
        #[arg(long)]
        path: bool,

        /// Reset config to defaults (creates a fresh config file)
        #[arg(long)]
        reset: bool,
    },

    /// Generate a configuration file at a specific location
    GenerateConfig {
        /// Output path for the config file (defaults to standard location)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show current configuration
    ShowConfig,

    /// Scan device and show folder structure (for debugging)
    Scan {
        /// Maximum depth to scan (0 = unlimited)
        #[arg(short, long, default_value = "3")]
        depth: usize,
    },

    /// List all configured device profiles
    ListProfiles,

    /// Remove a device profile
    RemoveProfile {
        /// Name or partial device ID to remove
        #[arg(short, long)]
        name: String,
    },

    /// Benchmark scan performance - shows detailed discovery statistics
    BenchmarkScan {
        /// Whether to only scan DCIM folder (faster)
        #[arg(long, default_value = "true")]
        dcim_only: bool,
    },

    /// Run tests using mock devices (no real iOS device required)
    ///
    /// This command allows you to test the photo extraction tool
    /// without connecting a real iOS device to your PC. It uses simulated
    /// devices with various test scenarios.
    Test {
        #[command(subcommand)]
        test_command: TestCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum TestCommands {
    /// Run all available test scenarios
    RunAll {
        /// Generate HTML report
        #[arg(long)]
        html_report: bool,

        /// Generate JSON report
        #[arg(long)]
        json_report: bool,

        /// Output directory for reports
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Stop on first failure
        #[arg(long)]
        fail_fast: bool,
    },

    /// Run quick test scenarios only (fast)
    RunQuick {
        /// Verbose output showing detailed results
        #[arg(short, long)]
        verbose: bool,
    },

    /// Run tests filtered by tag
    RunTag {
        /// Tag to filter scenarios by
        /// Available tags: device, error, duplicate, structure, tracking, profile, performance, stress-test, edge-case
        tag: String,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Run specific test scenarios by name
    Run {
        /// Scenario names to run (comma-separated or multiple values)
        #[arg(value_delimiter = ',')]
        scenarios: Vec<String>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// List all available test scenarios
    ListScenarios {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Show detailed information about each scenario
        #[arg(short, long)]
        detailed: bool,
    },

    /// List all available tags for filtering
    ListTags,

    /// Run interactive test mode
    ///
    /// Opens an interactive menu to browse and run test scenarios
    Interactive,

    /// Show information about a specific scenario
    Info {
        /// Name of the scenario to show info about
        name: String,
    },

    /// Simulate extraction from a mock device
    ///
    /// This performs a simulated extraction using a test scenario,
    /// writing files to the specified output directory.
    SimulateExtract {
        /// Test scenario to use for simulation
        #[arg(short, long, default_value = "single_iphone")]
        scenario: String,

        /// Output directory for extracted files
        #[arg(short, long)]
        output: PathBuf,

        /// Include progress bar and verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Simulate errors and edge cases from scenario
        #[arg(long)]
        simulate_errors: bool,
    },

    /// Generate mock test data files
    ///
    /// Creates test files (images/videos) in the specified directory
    /// for use in testing duplicate detection and other features.
    GenerateData {
        /// Output directory for generated files
        #[arg(short, long)]
        output: PathBuf,

        /// Number of files to generate
        #[arg(short, long, default_value = "100")]
        count: usize,

        /// Types of files to generate (comma-separated)
        /// Options: jpg, heic, png, mov, mp4, all
        #[arg(short, long, default_value = "all")]
        types: String,

        /// Include some duplicate files
        #[arg(long)]
        include_duplicates: bool,

        /// Seed for reproducible generation
        #[arg(long)]
        seed: Option<u64>,
    },

    /// Benchmark mock device performance
    ///
    /// Tests the performance of various operations using mock devices
    BenchmarkMock {
        /// Number of files to test with
        #[arg(short, long, default_value = "1000")]
        file_count: usize,

        /// Number of iterations
        #[arg(short, long, default_value = "5")]
        iterations: usize,
    },
}

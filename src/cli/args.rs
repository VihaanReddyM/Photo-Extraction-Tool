//! Command-line argument definitions
//!
//! This module defines all CLI arguments and subcommands using clap.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// A fast, reliable tool to extract photos from iPhone/iPad on Windows
#[derive(Parser, Debug)]
#[command(name = "photo_extraction_tool")]
#[command(author = "Vihaan Reddy M")]
#[command(version = "1.0.0")]
#[command(about = "Extract photos from iPhone/iPad connected to Windows PC", long_about = None)]
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

    /// List all portable devices (not just Apple devices)
    #[arg(long)]
    pub all_devices: bool,

    /// Log level: error, warn, info, debug, trace (overrides config)
    #[arg(short, long)]
    pub log_level: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Extract photos from the connected device
    Extract,

    /// List connected devices
    List {
        /// Show all portable devices, not just Apple devices
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
}

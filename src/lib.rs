//! Photo Extraction Tool Library
//!
//! A fast, reliable library for extracting photos and videos from iOS devices
//! (iPhone/iPad) on Windows using the Windows Portable Devices (WPD) API.
//!
//! # Architecture
//!
//! The library is organized into the following modules:
//!
//! - [`core`] - Core functionality including configuration, error handling,
//!   extraction logic, and state tracking
//! - [`device`] - Device interaction via Windows Portable Devices API
//! - [`duplicate`] - Duplicate detection using SHA256 hashing
//! - [`cli`] - Command-line interface (only used by the binary)
//! - [`testdb`] - Test database with mock devices and scenarios for testing
//! - [`ui`] - UI support module with async controllers, device monitoring, and previews
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use photo_extraction_tool::core::config::Config;
//! use photo_extraction_tool::core::extractor;
//! use photo_extraction_tool::device::{DeviceManager, DeviceManagerTrait, initialize_com};
//! use std::sync::Arc;
//! use std::sync::atomic::AtomicBool;
//!
//! fn main() -> anyhow::Result<()> {
//!     // Initialize COM (required for WPD on Windows)
//!     let _com_guard = initialize_com()?;
//!
//!     // Load configuration
//!     let config = Config::load_default()?;
//!
//!     // Create device manager and find devices
//!     let manager = DeviceManager::new()?;
//!     let devices = manager.enumerate_apple_devices()?;
//!
//!     if let Some(device) = devices.first() {
//!         // Set up shutdown flag for graceful termination
//!         let shutdown_flag = Arc::new(AtomicBool::new(false));
//!
//!         // Extract photos
//!         // ... (see extractor module for details)
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # UI Integration
//!
//! The `ui` module provides everything needed to build a graphical interface:
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::{
//!     ExtractionController, DeviceMonitor, UiEvent,
//!     ExtractionConfig, MonitorConfig,
//! };
//! use std::path::PathBuf;
//!
//! // Create controller for managing extractions
//! let controller = ExtractionController::new();
//!
//! // Create device monitor for hot-plug detection
//! let monitor = DeviceMonitor::with_config(MonitorConfig::fast());
//!
//! // Configure extraction
//! let config = ExtractionConfig::new(PathBuf::from("D:/Photos"))
//!     .preserve_structure(true)
//!     .skip_existing(true);
//!
//! // Poll for events in your UI loop
//! while let Some(event) = controller.try_recv_event() {
//!     match event {
//!         UiEvent::Extraction(ext) => { /* update progress */ }
//!         UiEvent::Device(dev) => { /* handle device changes */ }
//!         UiEvent::App(app) => { /* handle app events */ }
//!     }
//!     # break;
//! }
//! ```
//!
//! # Testing Without a Device
//!
//! The `testdb` module provides comprehensive testing capabilities:
//!
//! ```rust,no_run
//! use photo_extraction_tool::testdb::{TestRunner, ScenarioLibrary};
//!
//! // Run all quick test scenarios
//! let mut runner = TestRunner::new();
//! let summary = runner.run_quick();
//! println!("Passed: {}/{}", summary.passed, summary.total);
//!
//! // List available scenarios
//! photo_extraction_tool::testdb::print_available_scenarios();
//! ```
//!
//! # Features
//!
//! - **No iTunes Required** - Direct device access via WPD API
//! - **Incremental Backups** - Track extracted files to avoid re-downloading
//! - **Duplicate Detection** - SHA256-based exact duplicate detection
//! - **Multi-Device Support** - Manage multiple devices with profiles
//! - **Resume Support** - Continue interrupted extractions
//! - **Comprehensive Testing** - Test all features without a real device
//! - **UI Ready** - Async controllers and event system for GUI integration
//! - **Device Monitoring** - Hot-plug detection for device connect/disconnect
//! - **Preview Support** - Thumbnail generation infrastructure for photo previews
//!
//! # Platform Support
//!
//! This library currently only supports Windows due to its reliance on the
//! Windows Portable Devices API. Future versions may add support for other
//! platforms using different backends.

// Core modules - always available
pub mod cli;
pub mod core;
pub mod device;
pub mod duplicate;
pub mod testdb;
pub mod ui;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

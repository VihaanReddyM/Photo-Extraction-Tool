//! Core functionality module
//!
//! This module contains the core business logic for the photo extraction tool,
//! including configuration management, error handling, extraction logic, and
//! state tracking.
//!
//! # Submodules
//!
//! - `config` - Configuration loading, saving, and management
//! - `error` - Error types and result aliases
//! - `extractor` - Photo extraction logic (WPD-specific)
//! - `generic_extractor` - Generic extraction using trait abstraction (testable)
//! - `tracking` - Extraction state and session tracking
//!
//! # Testing Support
//!
//! The `generic_extractor` module provides extraction functionality that works
//! with any device implementing `DeviceContentTrait`. This enables the same
//! extraction logic to work with both real iOS devices (via WPD) and mock devices.
//!
//! ```rust,no_run
//! use photo_extraction_tool::core::generic_extractor::{GenericExtractor, GenericExtractionConfig};
//! use photo_extraction_tool::testdb::{MockDeviceManager, MockFileSystem};
//! use photo_extraction_tool::device::traits::DeviceManagerTrait;
//!
//! // Create a mock device for testing
//! let mut manager = MockDeviceManager::new();
//! let mut fs = MockFileSystem::new();
//! fs.add_standard_dcim_structure(10, 2);
//! // ... add device and test extraction
//! ```

#![allow(unused)]

pub mod config;
pub mod error;
pub mod extractor;
pub mod generic_extractor;
pub mod setup;
pub mod tracking;

// Re-export commonly used types
pub use config::Config;
pub use error::{ExtractionError, Result};
pub use extractor::{extract_photos, ExtractionConfig, ExtractionStats};
pub use generic_extractor::{
    ExtractionPhase, ExtractionStats as GenericExtractionStats, GenericExtractionConfig,
    GenericExtractor, ProgressUpdate,
};
pub use setup::{
    apply_setup, is_setup_complete, normalize_path, run_setup_wizard, SetupError, SetupOptions,
    SetupResult,
};
pub use tracking::{scan_for_profiles, scan_for_profiles_default, ProfileSummary, StateTracker};

/// Set a file's modification time from an ISO 8601 / RFC 3339 string (portable)
#[cfg(not(windows))]
pub(crate) fn set_mtime_from_iso8601(path: &std::path::Path, date_str: &str) -> Result<()> {
    let dt: chrono::DateTime<chrono::Utc> = date_str.parse().map_err(|e| {
        ExtractionError::IoError(format!("Failed to parse date '{}': {}", date_str, e))
    })?;
    let secs = dt.timestamp();
    let time = if secs >= 0 {
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64)
    } else {
        std::time::UNIX_EPOCH - std::time::Duration::from_secs(secs.unsigned_abs())
    };
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| {
            ExtractionError::IoError(format!("Failed to open '{}': {}", path.display(), e))
        })?;
    file.set_modified(time).map_err(|e| {
        ExtractionError::IoError(format!(
            "Failed to set time for '{}': {}",
            path.display(),
            e
        ))
    })
}

//! Tracking module for device state and extraction history
//!
//! This module manages a hidden tracking file that stores:
//! - Device information (UUID, name, manufacturer)
//! - Extraction history and statistics
//! - List of extracted files for resume support
//!
//! Some accessor methods are kept for API completeness and future use.

use crate::config::TrackingConfig;
use crate::device::DeviceInfo;
use crate::error::{ExtractionError, Result};
use chrono::{DateTime, Utc};
use log::{debug, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

/// Hidden file attribute on Windows
#[cfg(windows)]
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

/// Extraction state stored in the tracking file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionState {
    /// Version of the tracking file format
    pub version: u32,

    /// Device information
    pub device: DeviceState,

    /// Extraction statistics
    pub stats: ExtractionStats,

    /// Set of extracted file identifiers (object_id or hash)
    #[serde(default)]
    pub extracted_files: HashSet<String>,

    /// Extraction sessions history
    #[serde(default)]
    pub sessions: Vec<ExtractionSession>,
}

/// Device state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    /// WPD Device ID (unique identifier)
    pub device_id: String,

    /// User-friendly device name
    pub friendly_name: String,

    /// Device manufacturer
    pub manufacturer: String,

    /// Device model
    pub model: String,

    /// First seen timestamp
    pub first_seen: DateTime<Utc>,

    /// Last seen timestamp
    pub last_seen: DateTime<Utc>,
}

/// Cumulative extraction statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractionStats {
    /// Total files extracted across all sessions
    pub total_files_extracted: u64,

    /// Total bytes extracted across all sessions
    pub total_bytes_extracted: u64,

    /// Total files skipped (duplicates or existing)
    pub total_files_skipped: u64,

    /// Total extraction sessions
    pub total_sessions: u64,
}

/// Record of a single extraction session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSession {
    /// Session start time
    pub started_at: DateTime<Utc>,

    /// Session end time
    pub ended_at: DateTime<Utc>,

    /// Files extracted in this session
    pub files_extracted: u64,

    /// Bytes extracted in this session
    pub bytes_extracted: u64,

    /// Files skipped in this session
    pub files_skipped: u64,

    /// Duplicates found in this session
    pub duplicates_found: u64,

    /// Errors in this session
    pub errors: u64,

    /// Whether the session completed successfully
    pub completed: bool,

    /// Whether the session was interrupted
    pub interrupted: bool,
}

/// Tracker for managing extraction state
pub struct StateTracker {
    /// Configuration
    config: TrackingConfig,

    /// Path to the tracking file
    tracking_file_path: PathBuf,

    /// Current extraction state
    state: ExtractionState,

    /// Current session (if active)
    current_session: Option<ExtractionSession>,

    /// Session start time
    session_start: Option<DateTime<Utc>>,

    /// Whether the state has been modified
    dirty: bool,
}

impl ExtractionState {
    /// Create a new extraction state for a device
    pub fn new(device_info: &DeviceInfo) -> Self {
        let now = Utc::now();
        Self {
            version: 1,
            device: DeviceState {
                device_id: device_info.device_id.clone(),
                friendly_name: device_info.friendly_name.clone(),
                manufacturer: device_info.manufacturer.clone(),
                model: device_info.model.clone(),
                first_seen: now,
                last_seen: now,
            },
            stats: ExtractionStats::default(),
            extracted_files: HashSet::new(),
            sessions: Vec::new(),
        }
    }

    /// Update device information (in case name changed, etc.)
    pub fn update_device_info(&mut self, device_info: &DeviceInfo) {
        self.device.friendly_name = device_info.friendly_name.clone();
        self.device.manufacturer = device_info.manufacturer.clone();
        self.device.model = device_info.model.clone();
        self.device.last_seen = Utc::now();
    }
}

impl StateTracker {
    /// Create a new state tracker
    pub fn new(config: &TrackingConfig, output_dir: &Path) -> Self {
        let tracking_file_path = output_dir.join(&config.tracking_filename);

        Self {
            config: config.clone(),
            tracking_file_path,
            state: ExtractionState {
                version: 1,
                device: DeviceState {
                    device_id: String::new(),
                    friendly_name: String::new(),
                    manufacturer: String::new(),
                    model: String::new(),
                    first_seen: Utc::now(),
                    last_seen: Utc::now(),
                },
                stats: ExtractionStats::default(),
                extracted_files: HashSet::new(),
                sessions: Vec::new(),
            },
            current_session: None,
            session_start: None,
            dirty: false,
        }
    }

    /// Load existing state or create new one for the device
    pub fn load_or_create(&mut self, device_info: &DeviceInfo) -> Result<()> {
        if self.tracking_file_path.exists() {
            match self.load_state() {
                Ok(state) => {
                    // Verify it's for the same device
                    if state.device.device_id == device_info.device_id {
                        info!(
                            "Loaded tracking state for device '{}' ({} files previously extracted)",
                            state.device.friendly_name, state.stats.total_files_extracted
                        );
                        self.state = state;
                        self.state.update_device_info(device_info);
                        self.dirty = true; // Mark dirty to save updated last_seen
                    } else {
                        warn!(
                            "Tracking file exists but is for a different device. Creating new state."
                        );
                        warn!(
                            "  Existing: {} ({})",
                            state.device.friendly_name, state.device.device_id
                        );
                        warn!(
                            "  Current: {} ({})",
                            device_info.friendly_name, device_info.device_id
                        );
                        self.state = ExtractionState::new(device_info);
                        self.dirty = true;
                    }
                }
                Err(e) => {
                    warn!("Failed to load tracking state: {}. Creating new state.", e);
                    self.state = ExtractionState::new(device_info);
                    self.dirty = true;
                }
            }
        } else {
            info!(
                "Creating new tracking state for device '{}'",
                device_info.friendly_name
            );
            self.state = ExtractionState::new(device_info);
            self.dirty = true;
        }

        Ok(())
    }

    /// Load state from tracking file
    fn load_state(&self) -> Result<ExtractionState> {
        let file = File::open(&self.tracking_file_path).map_err(|e| {
            ExtractionError::IoError(format!("Failed to open tracking file: {}", e))
        })?;

        let reader = BufReader::new(file);
        let state: ExtractionState = serde_json::from_reader(reader).map_err(|e| {
            ExtractionError::IoError(format!("Failed to parse tracking file: {}", e))
        })?;

        Ok(state)
    }

    /// Save state to tracking file
    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            trace!("State not modified, skipping save");
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.tracking_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ExtractionError::IoError(format!("Failed to create tracking directory: {}", e))
            })?;
        }

        // Create the file with hidden attribute on Windows
        #[cfg(windows)]
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .attributes(FILE_ATTRIBUTE_HIDDEN)
            .open(&self.tracking_file_path)
            .map_err(|e| {
                ExtractionError::IoError(format!("Failed to create tracking file: {}", e))
            })?;

        #[cfg(not(windows))]
        let file = File::create(&self.tracking_file_path).map_err(|e| {
            ExtractionError::IoError(format!("Failed to create tracking file: {}", e))
        })?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.state).map_err(|e| {
            ExtractionError::IoError(format!("Failed to write tracking file: {}", e))
        })?;

        debug!(
            "Saved tracking state to: {}",
            self.tracking_file_path.display()
        );
        self.dirty = false;

        Ok(())
    }

    /// Start a new extraction session
    pub fn start_session(&mut self) {
        let now = Utc::now();
        self.session_start = Some(now);
        self.current_session = Some(ExtractionSession {
            started_at: now,
            ended_at: now, // Will be updated when session ends
            files_extracted: 0,
            bytes_extracted: 0,
            files_skipped: 0,
            duplicates_found: 0,
            errors: 0,
            completed: false,
            interrupted: false,
        });
        self.dirty = true;

        info!("Started extraction session at {}", now);
    }

    /// Record a file as extracted
    pub fn record_extracted(&mut self, file_id: &str, bytes: u64) {
        if let Some(ref mut session) = self.current_session {
            session.files_extracted += 1;
            session.bytes_extracted += bytes;
        }

        if self.config.track_extracted_files {
            self.state.extracted_files.insert(file_id.to_string());
        }

        self.state.stats.total_files_extracted += 1;
        self.state.stats.total_bytes_extracted += bytes;
        self.dirty = true;
    }

    /// Record a file as skipped
    pub fn record_skipped(&mut self) {
        if let Some(ref mut session) = self.current_session {
            session.files_skipped += 1;
        }

        self.state.stats.total_files_skipped += 1;
        self.dirty = true;
    }

    /// Record a duplicate found
    pub fn record_duplicate(&mut self) {
        if let Some(ref mut session) = self.current_session {
            session.duplicates_found += 1;
        }
        self.dirty = true;
    }

    /// Record an error
    pub fn record_error(&mut self) {
        if let Some(ref mut session) = self.current_session {
            session.errors += 1;
        }
        self.dirty = true;
    }

    /// Check if a file has already been extracted
    ///
    /// Returns true if the file ID exists in the extracted files set.
    #[allow(dead_code)]
    pub fn is_file_extracted(&self, file_id: &str) -> bool {
        if !self.config.track_extracted_files {
            return false;
        }
        self.state.extracted_files.contains(file_id)
    }

    /// End the current session
    pub fn end_session(&mut self, completed: bool, interrupted: bool) {
        let now = Utc::now();

        if let Some(mut session) = self.current_session.take() {
            session.ended_at = now;
            session.completed = completed;
            session.interrupted = interrupted;

            info!(
                "Session ended: {} files extracted, {} skipped, {} duplicates, {} errors",
                session.files_extracted,
                session.files_skipped,
                session.duplicates_found,
                session.errors
            );

            self.state.sessions.push(session);
            self.state.stats.total_sessions += 1;
        }

        self.session_start = None;
        self.dirty = true;
    }

    // =========================================================================
    // Accessor methods (kept for API completeness and future use)
    // =========================================================================

    /// Get the current extraction state
    #[allow(dead_code)]
    pub fn state(&self) -> &ExtractionState {
        &self.state
    }

    /// Get device info from state
    #[allow(dead_code)]
    pub fn device_info(&self) -> &DeviceState {
        &self.state.device
    }

    /// Get the tracking file path
    #[allow(dead_code)]
    pub fn tracking_file_path(&self) -> &Path {
        &self.tracking_file_path
    }

    /// Get total files extracted for this device
    #[allow(dead_code)]
    pub fn total_files_extracted(&self) -> u64 {
        self.state.stats.total_files_extracted
    }

    /// Get the number of tracked extracted files
    #[allow(dead_code)]
    pub fn tracked_files_count(&self) -> usize {
        self.state.extracted_files.len()
    }

    /// Clear tracked files (useful for re-extraction)
    #[allow(dead_code)]
    pub fn clear_tracked_files(&mut self) {
        self.state.extracted_files.clear();
        self.dirty = true;
    }
}

impl Drop for StateTracker {
    fn drop(&mut self) {
        // Try to save state when tracker is dropped
        if self.dirty {
            if let Err(e) = self.save() {
                warn!("Failed to save tracking state on drop: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_state_new() {
        let device_info = DeviceInfo {
            device_id: "test-device-id".to_string(),
            friendly_name: "Test iPhone".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 15".to_string(),
        };

        let state = ExtractionState::new(&device_info);

        assert_eq!(state.device.device_id, "test-device-id");
        assert_eq!(state.device.friendly_name, "Test iPhone");
        assert_eq!(state.version, 1);
        assert!(state.extracted_files.is_empty());
    }

    #[test]
    fn test_state_tracker_record() {
        let config = TrackingConfig {
            enabled: true,
            tracking_filename: ".test_tracking.json".to_string(),
            track_extracted_files: true,
        };

        let mut tracker = StateTracker::new(&config, Path::new("/tmp"));
        tracker.start_session();

        tracker.record_extracted("file1", 1000);
        tracker.record_extracted("file2", 2000);
        tracker.record_skipped();
        tracker.record_duplicate();

        assert_eq!(tracker.state.stats.total_files_extracted, 2);
        assert_eq!(tracker.state.stats.total_bytes_extracted, 3000);
        assert_eq!(tracker.state.stats.total_files_skipped, 1);
        assert!(tracker.is_file_extracted("file1"));
        assert!(tracker.is_file_extracted("file2"));
        assert!(!tracker.is_file_extracted("file3"));
    }
}

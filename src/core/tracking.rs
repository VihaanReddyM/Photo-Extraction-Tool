//! Tracking module for device state and extraction history
//!
//! This module manages a hidden tracking file that stores:
//! - Device information (UUID, name, manufacturer)
//! - Extraction history and statistics
//! - List of extracted files for resume support
//!
//! # Profile Scanning
//!
//! This module also provides functionality to scan directories for existing
//! extraction profiles. When a user selects an output directory, the tool can
//! scan subdirectories for tracking files and present discovered profiles.
//!
//! ```rust,no_run
//! use photo_extraction_tool::core::tracking::{scan_for_profiles, ProfileSummary};
//! use std::path::Path;
//!
//! let profiles = scan_for_profiles(Path::new("D:/Photos"), ".photo_extraction_state.json");
//! for profile in profiles {
//!     println!("Found profile: {} ({} files extracted)",
//!         profile.friendly_name, profile.total_files_extracted);
//! }
//! ```
//!
//! Some accessor methods are kept for API completeness and future use.

use crate::core::config::TrackingConfig;
use crate::core::error::{ExtractionError, Result};
use crate::device::DeviceInfo;
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

/// Summary of a discovered profile from scanning
///
/// This struct provides a lightweight view of an extraction profile found
/// in a subdirectory, useful for presenting options to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    /// Path to the profile folder (parent of tracking file)
    pub path: PathBuf,

    /// WPD Device ID
    pub device_id: String,

    /// User-friendly device name
    pub friendly_name: String,

    /// Device manufacturer
    pub manufacturer: String,

    /// Device model
    pub model: String,

    /// Total files extracted for this profile
    pub total_files_extracted: u64,

    /// Total bytes extracted for this profile
    pub total_bytes_extracted: u64,

    /// Total extraction sessions
    pub total_sessions: u64,

    /// First seen timestamp
    pub first_seen: DateTime<Utc>,

    /// Last seen timestamp
    pub last_seen: DateTime<Utc>,
}

impl ProfileSummary {
    /// Create a ProfileSummary from an ExtractionState and its path
    pub fn from_state(state: &ExtractionState, path: PathBuf) -> Self {
        Self {
            path,
            device_id: state.device.device_id.clone(),
            friendly_name: state.device.friendly_name.clone(),
            manufacturer: state.device.manufacturer.clone(),
            model: state.device.model.clone(),
            total_files_extracted: state.stats.total_files_extracted,
            total_bytes_extracted: state.stats.total_bytes_extracted,
            total_sessions: state.stats.total_sessions,
            first_seen: state.device.first_seen,
            last_seen: state.device.last_seen,
        }
    }

    /// Get a display-friendly description of the profile
    pub fn display_description(&self) -> String {
        format!(
            "{} ({}) - {} files, {} sessions",
            self.friendly_name, self.model, self.total_files_extracted, self.total_sessions
        )
    }
}

/// Scan a directory for existing extraction profiles
///
/// This function looks in immediate subdirectories of `root` for tracking files
/// (e.g., `.photo_extraction_state.json`). It parses found files and returns
/// a summary of each discovered profile.
///
/// # Arguments
///
/// * `root` - The root directory to scan (immediate children are checked)
/// * `tracking_filename` - The name of the tracking file to look for
///
/// # Returns
///
/// A vector of `ProfileSummary` for each valid profile found. Invalid or
/// corrupted tracking files are logged and skipped.
///
/// # Example
///
/// ```rust,no_run
/// use photo_extraction_tool::core::tracking::scan_for_profiles;
/// use std::path::Path;
///
/// let profiles = scan_for_profiles(Path::new("D:/Photos"), ".photo_extraction_state.json");
/// for profile in &profiles {
///     println!("Found: {} in {:?}", profile.friendly_name, profile.path);
/// }
/// ```
pub fn scan_for_profiles(root: &Path, tracking_filename: &str) -> Vec<ProfileSummary> {
    let mut profiles = Vec::new();

    // Check if root exists and is a directory
    if !root.is_dir() {
        debug!("Profile scan: root {:?} is not a directory", root);
        return profiles;
    }

    // First, check if there's a tracking file directly in the root
    let root_tracking_file = root.join(tracking_filename);
    if root_tracking_file.exists() {
        if let Some(summary) = load_profile_summary(&root_tracking_file, root.to_path_buf()) {
            debug!(
                "Found profile in root directory: {} ({} files)",
                summary.friendly_name, summary.total_files_extracted
            );
            profiles.push(summary);
        }
    }

    // Scan immediate subdirectories
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("Failed to read directory {:?}: {}", root, e);
            return profiles;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Only check directories
        if !path.is_dir() {
            continue;
        }

        let tracking_file = path.join(tracking_filename);
        if tracking_file.exists() {
            if let Some(summary) = load_profile_summary(&tracking_file, path.clone()) {
                debug!(
                    "Found profile: {} in {:?} ({} files extracted)",
                    summary.friendly_name, path, summary.total_files_extracted
                );
                profiles.push(summary);
            }
        }
    }

    // Sort by last_seen (most recent first)
    profiles.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

    debug!(
        "Profile scan complete: found {} profiles in {:?}",
        profiles.len(),
        root
    );

    profiles
}

/// Load a profile summary from a tracking file
///
/// Returns `None` if the file cannot be read or parsed.
fn load_profile_summary(tracking_file: &Path, profile_path: PathBuf) -> Option<ProfileSummary> {
    let file = match File::open(tracking_file) {
        Ok(f) => f,
        Err(e) => {
            warn!("Failed to open tracking file {:?}: {}", tracking_file, e);
            return None;
        }
    };

    let reader = BufReader::new(file);
    let state: ExtractionState = match serde_json::from_reader(reader) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to parse tracking file {:?}: {}", tracking_file, e);
            return None;
        }
    };

    Some(ProfileSummary::from_state(&state, profile_path))
}

/// Scan for profiles using default tracking configuration
///
/// Convenience function that uses the default tracking filename.
pub fn scan_for_profiles_default(root: &Path) -> Vec<ProfileSummary> {
    let config = TrackingConfig::default();
    scan_for_profiles(root, &config.tracking_filename)
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
                        debug!(
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
            debug!(
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

        debug!("Started extraction session at {}", now);
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

            debug!(
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
    use std::io::Write;
    use tempfile::TempDir;

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

    #[test]
    fn test_profile_summary_from_state() {
        let device_info = DeviceInfo {
            device_id: "device-123".to_string(),
            friendly_name: "My iPhone".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 14 Pro".to_string(),
        };

        let mut state = ExtractionState::new(&device_info);
        state.stats.total_files_extracted = 500;
        state.stats.total_bytes_extracted = 5_000_000_000;
        state.stats.total_sessions = 3;

        let summary = ProfileSummary::from_state(&state, PathBuf::from("/photos/my_iphone"));

        assert_eq!(summary.device_id, "device-123");
        assert_eq!(summary.friendly_name, "My iPhone");
        assert_eq!(summary.model, "iPhone 14 Pro");
        assert_eq!(summary.total_files_extracted, 500);
        assert_eq!(summary.total_sessions, 3);
        assert_eq!(summary.path, PathBuf::from("/photos/my_iphone"));
    }

    #[test]
    fn test_profile_summary_display_description() {
        let device_info = DeviceInfo {
            device_id: "test-id".to_string(),
            friendly_name: "Test Device".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 15".to_string(),
        };

        let mut state = ExtractionState::new(&device_info);
        state.stats.total_files_extracted = 100;
        state.stats.total_sessions = 5;

        let summary = ProfileSummary::from_state(&state, PathBuf::from("/test"));
        let desc = summary.display_description();

        assert!(desc.contains("Test Device"));
        assert!(desc.contains("iPhone 15"));
        assert!(desc.contains("100 files"));
        assert!(desc.contains("5 sessions"));
    }

    #[test]
    fn test_scan_for_profiles_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let profiles = scan_for_profiles(temp_dir.path(), ".photo_extraction_state.json");
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_scan_for_profiles_nonexistent_directory() {
        let profiles = scan_for_profiles(
            Path::new("/nonexistent/path"),
            ".photo_extraction_state.json",
        );
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_scan_for_profiles_finds_profiles() {
        let temp_dir = TempDir::new().unwrap();

        // Create a subdirectory with a valid tracking file
        let profile_dir = temp_dir.path().join("johns_iphone");
        fs::create_dir(&profile_dir).unwrap();

        let device_info = DeviceInfo {
            device_id: "device-abc".to_string(),
            friendly_name: "John's iPhone".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone 15 Pro Max".to_string(),
        };

        let mut state = ExtractionState::new(&device_info);
        state.stats.total_files_extracted = 1234;
        state.stats.total_sessions = 10;

        let tracking_file = profile_dir.join(".photo_extraction_state.json");
        let mut file = File::create(&tracking_file).unwrap();
        let json = serde_json::to_string_pretty(&state).unwrap();
        file.write_all(json.as_bytes()).unwrap();

        // Create another subdirectory without a tracking file (should be ignored)
        let other_dir = temp_dir.path().join("other_folder");
        fs::create_dir(&other_dir).unwrap();

        // Scan
        let profiles = scan_for_profiles(temp_dir.path(), ".photo_extraction_state.json");

        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].friendly_name, "John's iPhone");
        assert_eq!(profiles[0].total_files_extracted, 1234);
        assert_eq!(profiles[0].path, profile_dir);
    }

    #[test]
    fn test_scan_for_profiles_handles_corrupted_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create a subdirectory with an invalid tracking file
        let profile_dir = temp_dir.path().join("corrupted_profile");
        fs::create_dir(&profile_dir).unwrap();

        let tracking_file = profile_dir.join(".photo_extraction_state.json");
        let mut file = File::create(&tracking_file).unwrap();
        file.write_all(b"{ invalid json content }").unwrap();

        // Scan should return empty (corrupted file is skipped)
        let profiles = scan_for_profiles(temp_dir.path(), ".photo_extraction_state.json");
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_scan_for_profiles_in_root() {
        let temp_dir = TempDir::new().unwrap();

        // Create a tracking file directly in the root
        let device_info = DeviceInfo {
            device_id: "root-device".to_string(),
            friendly_name: "Root iPhone".to_string(),
            manufacturer: "Apple Inc.".to_string(),
            model: "iPhone SE".to_string(),
        };

        let state = ExtractionState::new(&device_info);
        let tracking_file = temp_dir.path().join(".photo_extraction_state.json");
        let mut file = File::create(&tracking_file).unwrap();
        let json = serde_json::to_string_pretty(&state).unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let profiles = scan_for_profiles(temp_dir.path(), ".photo_extraction_state.json");

        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].friendly_name, "Root iPhone");
        assert_eq!(profiles[0].path, temp_dir.path());
    }

    #[test]
    fn test_scan_for_profiles_default() {
        let temp_dir = TempDir::new().unwrap();
        let profiles = scan_for_profiles_default(temp_dir.path());
        assert!(profiles.is_empty());
    }
}

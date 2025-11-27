//! UI Events Module
//!
//! Defines thread-safe event types for communication between the backend
//! extraction engine and UI frontends. These events are designed to be
//! sent through channels and consumed by any UI framework.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::device::DeviceInfo;

// =============================================================================
// Extraction Events
// =============================================================================

/// Events emitted during the extraction process
#[derive(Debug, Clone)]
pub enum ExtractionEvent {
    /// Extraction has started
    Started {
        /// Device being extracted from
        device: DeviceInfo,
        /// Output directory
        output_dir: PathBuf,
        /// Timestamp when extraction started
        started_at: Instant,
    },

    /// Scanning phase has begun
    ScanStarted,

    /// Progress update during scanning
    ScanProgress {
        /// Number of folders scanned so far
        folders_scanned: usize,
        /// Number of files found so far
        files_found: usize,
        /// Current folder being scanned
        current_folder: String,
    },

    /// Scanning phase completed
    ScanComplete {
        /// Total files found
        total_files: usize,
        /// Total size in bytes
        total_bytes: u64,
        /// Time taken to scan
        scan_duration: Duration,
    },

    /// A file extraction has started
    FileStarted {
        /// File name
        name: String,
        /// File size in bytes
        size: u64,
        /// Current index (1-based)
        index: usize,
        /// Total files to extract
        total: usize,
    },

    /// Progress update for current file transfer
    FileProgress {
        /// File name
        name: String,
        /// Bytes transferred so far
        bytes_transferred: u64,
        /// Total file size
        total_bytes: u64,
        /// Transfer speed in bytes per second
        speed_bps: u64,
    },

    /// A file was successfully extracted
    FileComplete {
        /// File name
        name: String,
        /// File size in bytes
        size: u64,
        /// Destination path
        destination: PathBuf,
        /// Time taken to transfer
        transfer_duration: Duration,
    },

    /// A file was skipped
    FileSkipped {
        /// File name
        name: String,
        /// Reason for skipping
        reason: SkipReason,
    },

    /// An error occurred during file extraction
    FileError {
        /// File name
        name: String,
        /// Error message
        error: String,
        /// Whether the error is recoverable
        recoverable: bool,
    },

    /// Overall progress update
    Progress {
        /// Files extracted so far
        files_extracted: usize,
        /// Files skipped so far
        files_skipped: usize,
        /// Duplicates found so far
        duplicates_found: usize,
        /// Errors so far
        errors: usize,
        /// Bytes processed so far
        bytes_processed: u64,
        /// Estimated time remaining
        eta: Option<Duration>,
        /// Current speed in bytes per second
        speed_bps: u64,
        /// Percentage complete (0.0 - 100.0)
        percent_complete: f64,
    },

    /// Extraction was paused
    Paused {
        /// Reason for pause
        reason: PauseReason,
    },

    /// Extraction was resumed
    Resumed,

    /// Extraction is being cancelled
    Cancelling,

    /// Extraction was cancelled
    Cancelled {
        /// Files extracted before cancellation
        files_extracted: usize,
        /// Can be resumed later
        resumable: bool,
    },

    /// Extraction completed successfully
    Completed {
        /// Final statistics
        stats: ExtractionSummary,
    },

    /// A fatal error occurred
    FatalError {
        /// Error message
        error: String,
        /// Additional context
        context: Option<String>,
    },
}

/// Reasons why a file might be skipped
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// File already exists at destination
    AlreadyExists,
    /// File was previously extracted (tracked in state)
    PreviouslyExtracted,
    /// File is a duplicate of another file
    Duplicate {
        /// Path to the original file
        original: PathBuf,
    },
    /// File doesn't match filter criteria
    FilteredOut {
        /// Which filter rejected it
        filter: String,
    },
    /// File is too small
    TooSmall {
        /// Actual size
        size: u64,
        /// Minimum required
        minimum: u64,
    },
    /// File is too large
    TooLarge {
        /// Actual size
        size: u64,
        /// Maximum allowed
        maximum: u64,
    },
}

/// Reasons for pausing extraction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PauseReason {
    /// User requested pause
    UserRequested,
    /// Device was disconnected
    DeviceDisconnected,
    /// Low disk space
    LowDiskSpace {
        /// Available space in bytes
        available: u64,
        /// Required space in bytes
        required: u64,
    },
    /// Waiting for user input
    WaitingForInput {
        /// What input is needed
        prompt: String,
    },
}

/// Summary of a completed extraction
#[derive(Debug, Clone)]
pub struct ExtractionSummary {
    /// Device that was extracted from
    pub device: DeviceInfo,
    /// Output directory
    pub output_dir: PathBuf,
    /// Total files extracted
    pub files_extracted: usize,
    /// Total files skipped
    pub files_skipped: usize,
    /// Duplicates found
    pub duplicates_found: usize,
    /// Errors encountered
    pub errors: usize,
    /// Total bytes transferred
    pub bytes_transferred: u64,
    /// Total duration
    pub duration: Duration,
    /// Average transfer speed
    pub average_speed_bps: u64,
    /// Whether extraction completed fully or was interrupted
    pub completed_fully: bool,
    /// Session can be resumed if not completed
    pub resumable: bool,
}

impl ExtractionSummary {
    /// Calculate success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.files_extracted + self.errors;
        if total == 0 {
            100.0
        } else {
            (self.files_extracted as f64 / total as f64) * 100.0
        }
    }

    /// Get bytes transferred in megabytes
    pub fn megabytes_transferred(&self) -> f64 {
        self.bytes_transferred as f64 / (1024.0 * 1024.0)
    }

    /// Get bytes transferred in gigabytes
    pub fn gigabytes_transferred(&self) -> f64 {
        self.bytes_transferred as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Format duration as human-readable string
    pub fn formatted_duration(&self) -> String {
        let secs = self.duration.as_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
        }
    }

    /// Format speed as human-readable string
    pub fn formatted_speed(&self) -> String {
        format_bytes_per_second(self.average_speed_bps)
    }
}

// =============================================================================
// Device Events
// =============================================================================

/// Events related to device connection/disconnection
#[derive(Debug, Clone)]
pub enum DeviceEvent {
    /// A new device was connected
    Connected {
        /// Device information
        device: DeviceInfo,
        /// Whether this device has been seen before
        previously_known: bool,
    },

    /// A device was disconnected
    Disconnected {
        /// Device ID
        device_id: String,
        /// Device name (if known)
        device_name: Option<String>,
    },

    /// Device trust status changed
    TrustStatusChanged {
        /// Device information
        device: DeviceInfo,
        /// Whether device is now trusted
        trusted: bool,
    },

    /// Device is locked and needs to be unlocked
    DeviceLocked {
        /// Device information
        device: DeviceInfo,
    },

    /// Device storage information updated
    StorageInfo {
        /// Device information
        device: DeviceInfo,
        /// Total storage in bytes
        total_bytes: u64,
        /// Used storage in bytes
        used_bytes: u64,
        /// Free storage in bytes
        free_bytes: u64,
    },

    /// Error communicating with device
    DeviceError {
        /// Device ID
        device_id: String,
        /// Error message
        error: String,
    },
}

// =============================================================================
// Application Events
// =============================================================================

/// General application events
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Configuration was loaded
    ConfigLoaded {
        /// Path to config file
        path: PathBuf,
    },

    /// Configuration was saved
    ConfigSaved {
        /// Path to config file
        path: PathBuf,
    },

    /// Configuration error
    ConfigError {
        /// Error message
        error: String,
    },

    /// Disk space warning
    DiskSpaceWarning {
        /// Path being monitored
        path: PathBuf,
        /// Available space
        available_bytes: u64,
        /// Threshold that triggered warning
        threshold_bytes: u64,
    },

    /// Application is shutting down
    ShuttingDown,

    /// Log message for UI display
    Log {
        /// Log level
        level: LogLevel,
        /// Message
        message: String,
        /// Optional context/source
        source: Option<String>,
    },
}

/// Log levels for UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Debug information
    Debug,
    /// Informational message
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
}

// =============================================================================
// Combined Event Type
// =============================================================================

/// All possible events that can be sent to the UI
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// Extraction-related event
    Extraction(ExtractionEvent),
    /// Device-related event
    Device(DeviceEvent),
    /// Application-related event
    App(AppEvent),
}

impl From<ExtractionEvent> for UiEvent {
    fn from(event: ExtractionEvent) -> Self {
        UiEvent::Extraction(event)
    }
}

impl From<DeviceEvent> for UiEvent {
    fn from(event: DeviceEvent) -> Self {
        UiEvent::Device(event)
    }
}

impl From<AppEvent> for UiEvent {
    fn from(event: AppEvent) -> Self {
        UiEvent::App(event)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Format bytes per second as human-readable string
pub fn format_bytes_per_second(bps: u64) -> String {
    if bps < 1024 {
        format!("{} B/s", bps)
    } else if bps < 1024 * 1024 {
        format!("{:.1} KB/s", bps as f64 / 1024.0)
    } else if bps < 1024 * 1024 * 1024 {
        format!("{:.1} MB/s", bps as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB/s", bps as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Format bytes as human-readable string
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Format duration as human-readable string
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}

/// Format ETA as human-readable string
pub fn format_eta(eta: Option<Duration>) -> String {
    match eta {
        Some(d) if d.as_secs() == 0 => "< 1s".to_string(),
        Some(d) => format_duration(d),
        None => "calculating...".to_string(),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_format_bytes_per_second() {
        assert_eq!(format_bytes_per_second(500), "500 B/s");
        assert_eq!(format_bytes_per_second(1024), "1.0 KB/s");
        assert_eq!(format_bytes_per_second(1024 * 1024), "1.0 MB/s");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
    }

    #[test]
    fn test_extraction_summary_success_rate() {
        let summary = ExtractionSummary {
            device: DeviceInfo::new("id", "name", "mfr", "model"),
            output_dir: PathBuf::from("/tmp"),
            files_extracted: 90,
            files_skipped: 5,
            duplicates_found: 3,
            errors: 10,
            bytes_transferred: 1024 * 1024 * 100,
            duration: Duration::from_secs(60),
            average_speed_bps: 1024 * 1024,
            completed_fully: true,
            resumable: false,
        };

        assert!((summary.success_rate() - 90.0).abs() < 0.01);
        assert!((summary.megabytes_transferred() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_skip_reason_variants() {
        let reasons = vec![
            SkipReason::AlreadyExists,
            SkipReason::PreviouslyExtracted,
            SkipReason::Duplicate {
                original: PathBuf::from("/photos/img.jpg"),
            },
            SkipReason::FilteredOut {
                filter: "extension".to_string(),
            },
            SkipReason::TooSmall {
                size: 100,
                minimum: 1000,
            },
            SkipReason::TooLarge {
                size: 10000,
                maximum: 5000,
            },
        ];

        // Just verify they can be created and cloned
        for reason in reasons {
            let _ = reason.clone();
        }
    }

    #[test]
    fn test_ui_event_conversions() {
        let extraction_event = ExtractionEvent::ScanStarted;
        let ui_event: UiEvent = extraction_event.into();
        assert!(matches!(ui_event, UiEvent::Extraction(_)));

        let device_event = DeviceEvent::Disconnected {
            device_id: "test".to_string(),
            device_name: None,
        };
        let ui_event: UiEvent = device_event.into();
        assert!(matches!(ui_event, UiEvent::Device(_)));

        let app_event = AppEvent::ShuttingDown;
        let ui_event: UiEvent = app_event.into();
        assert!(matches!(ui_event, UiEvent::App(_)));
    }
}

//! Extraction Controller Module
//!
//! Provides a thread-safe controller for managing extraction operations.
//! This controller handles background extraction, cancellation, pause/resume,
//! and communicates with the UI through channels.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::core::error::{ExtractionError, Result};
use crate::core::generic_extractor::{
    ExtractionPhase, GenericExtractionConfig, GenericExtractor, ProgressUpdate,
};
use crate::device::{DeviceContentTrait, DeviceInfo, DeviceManagerTrait};
use crate::duplicate::{DuplicateConfig, DuplicateIndex};
use crate::ui::events::{AppEvent, ExtractionEvent, ExtractionSummary, PauseReason, UiEvent};

// =============================================================================
// Controller State
// =============================================================================

/// Current state of the extraction controller
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ControllerState {
    /// Controller is idle, ready for new extraction
    Idle = 0,
    /// Scanning device for files
    Scanning = 1,
    /// Actively extracting files
    Extracting = 2,
    /// Extraction is paused
    Paused = 3,
    /// Extraction is being cancelled
    Cancelling = 4,
    /// Extraction completed
    Completed = 5,
    /// An error occurred
    Error = 6,
}

impl From<u8> for ControllerState {
    fn from(value: u8) -> Self {
        match value {
            0 => ControllerState::Idle,
            1 => ControllerState::Scanning,
            2 => ControllerState::Extracting,
            3 => ControllerState::Paused,
            4 => ControllerState::Cancelling,
            5 => ControllerState::Completed,
            6 => ControllerState::Error,
            _ => ControllerState::Idle,
        }
    }
}

// =============================================================================
// Commands
// =============================================================================

/// Commands that can be sent to the extraction controller
#[derive(Debug)]
pub enum ControllerCommand {
    /// Start extraction with the given configuration
    Start {
        /// Output directory
        output_dir: PathBuf,
        /// Optional duplicate detection config
        duplicate_config: Option<DuplicateConfig>,
        /// Whether to preserve folder structure
        preserve_structure: bool,
        /// Skip files that already exist
        skip_existing: bool,
        /// Only extract from DCIM folder
        dcim_only: bool,
        /// Maximum number of files (0 = unlimited)
        max_files: usize,
    },
    /// Pause the current extraction
    Pause,
    /// Resume a paused extraction
    Resume,
    /// Cancel the current extraction
    Cancel,
    /// Shutdown the controller
    Shutdown,
}

// =============================================================================
// Progress Tracker
// =============================================================================

use std::sync::atomic::AtomicU64;

/// Thread-safe progress tracking
#[derive(Debug)]
pub struct ProgressTracker {
    /// Files extracted
    files_extracted: AtomicU64,
    /// Files skipped
    files_skipped: AtomicU64,
    /// Duplicates found
    duplicates_found: AtomicU64,
    /// Errors encountered
    errors: AtomicU64,
    /// Bytes processed
    bytes_processed: AtomicU64,
    /// Total files to process
    total_files: AtomicU64,
    /// Total bytes to process
    total_bytes: AtomicU64,
    /// Start time
    start_time: RwLock<Option<Instant>>,
    /// Bytes processed in last second (for speed calculation)
    recent_bytes: Mutex<Vec<(Instant, u64)>>,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            files_extracted: AtomicU64::new(0),
            files_skipped: AtomicU64::new(0),
            duplicates_found: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
            total_files: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            start_time: RwLock::new(None),
            recent_bytes: Mutex::new(Vec::new()),
        }
    }

    /// Reset the tracker for a new extraction
    pub fn reset(&self) {
        self.files_extracted.store(0, Ordering::SeqCst);
        self.files_skipped.store(0, Ordering::SeqCst);
        self.duplicates_found.store(0, Ordering::SeqCst);
        self.errors.store(0, Ordering::SeqCst);
        self.bytes_processed.store(0, Ordering::SeqCst);
        self.total_files.store(0, Ordering::SeqCst);
        self.total_bytes.store(0, Ordering::SeqCst);
        *self.start_time.write().unwrap() = Some(Instant::now());
        self.recent_bytes.lock().unwrap().clear();
    }

    /// Set total files and bytes
    pub fn set_totals(&self, files: u64, bytes: u64) {
        self.total_files.store(files, Ordering::SeqCst);
        self.total_bytes.store(bytes, Ordering::SeqCst);
    }

    /// Record a file extraction
    pub fn record_extracted(&self, bytes: u64) {
        self.files_extracted.fetch_add(1, Ordering::SeqCst);
        self.bytes_processed.fetch_add(bytes, Ordering::SeqCst);
        self.record_bytes_for_speed(bytes);
    }

    /// Record a skipped file
    pub fn record_skipped(&self) {
        self.files_skipped.fetch_add(1, Ordering::SeqCst);
    }

    /// Record a duplicate found
    pub fn record_duplicate(&self) {
        self.duplicates_found.fetch_add(1, Ordering::SeqCst);
        self.files_skipped.fetch_add(1, Ordering::SeqCst);
    }

    /// Record an error
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::SeqCst);
    }

    /// Record bytes for speed calculation
    fn record_bytes_for_speed(&self, bytes: u64) {
        let now = Instant::now();
        let mut recent = self.recent_bytes.lock().unwrap();
        recent.push((now, bytes));

        // Keep only last 5 seconds of data
        let cutoff = now - Duration::from_secs(5);
        recent.retain(|(t, _)| *t > cutoff);
    }

    /// Calculate current transfer speed in bytes per second
    pub fn current_speed(&self) -> u64 {
        let recent = self.recent_bytes.lock().unwrap();
        if recent.is_empty() {
            return 0;
        }

        let now = Instant::now();
        let oldest = recent.first().map(|(t, _)| *t).unwrap_or(now);
        let duration = now.duration_since(oldest);

        if duration.as_millis() == 0 {
            return 0;
        }

        let total_bytes: u64 = recent.iter().map(|(_, b)| *b).sum();
        (total_bytes as f64 / duration.as_secs_f64()) as u64
    }

    /// Calculate ETA based on current progress
    pub fn eta(&self) -> Option<Duration> {
        let speed = self.current_speed();
        if speed == 0 {
            return None;
        }

        let bytes_processed = self.bytes_processed.load(Ordering::SeqCst);
        let total_bytes = self.total_bytes.load(Ordering::SeqCst);

        if bytes_processed >= total_bytes {
            return Some(Duration::ZERO);
        }

        let remaining_bytes = total_bytes - bytes_processed;
        let seconds_remaining = remaining_bytes / speed;
        Some(Duration::from_secs(seconds_remaining))
    }

    /// Calculate percentage complete
    pub fn percent_complete(&self) -> f64 {
        let total = self.total_bytes.load(Ordering::SeqCst);
        if total == 0 {
            return 0.0;
        }
        let processed = self.bytes_processed.load(Ordering::SeqCst);
        (processed as f64 / total as f64) * 100.0
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time
            .read()
            .unwrap()
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    /// Get current stats as a snapshot
    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            files_extracted: self.files_extracted.load(Ordering::SeqCst) as usize,
            files_skipped: self.files_skipped.load(Ordering::SeqCst) as usize,
            duplicates_found: self.duplicates_found.load(Ordering::SeqCst) as usize,
            errors: self.errors.load(Ordering::SeqCst) as usize,
            bytes_processed: self.bytes_processed.load(Ordering::SeqCst),
            total_files: self.total_files.load(Ordering::SeqCst) as usize,
            total_bytes: self.total_bytes.load(Ordering::SeqCst),
            speed_bps: self.current_speed(),
            eta: self.eta(),
            percent_complete: self.percent_complete(),
            elapsed: self.elapsed(),
        }
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of current progress
#[derive(Debug, Clone)]
pub struct ProgressSnapshot {
    pub files_extracted: usize,
    pub files_skipped: usize,
    pub duplicates_found: usize,
    pub errors: usize,
    pub bytes_processed: u64,
    pub total_files: usize,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta: Option<Duration>,
    pub percent_complete: f64,
    pub elapsed: Duration,
}

// =============================================================================
// Extraction Controller
// =============================================================================

/// Thread-safe extraction controller
///
/// This controller manages background extraction operations and provides
/// a clean interface for UI integration. It handles:
///
/// - Starting/stopping/pausing extractions
/// - Progress tracking and reporting
/// - Cancellation with cleanup
/// - Event emission for UI updates
pub struct ExtractionController {
    /// Current state
    state: Arc<AtomicU8>,
    /// Shutdown flag for extraction thread
    shutdown_flag: Arc<AtomicBool>,
    /// Pause flag
    pause_flag: Arc<AtomicBool>,
    /// Progress tracker
    progress: Arc<ProgressTracker>,
    /// Command sender
    _command_tx: Sender<ControllerCommand>,
    /// Event receiver for UI
    event_rx: Mutex<Receiver<UiEvent>>,
    /// Event sender (for internal use)
    event_tx: Sender<UiEvent>,
    /// Worker thread handle
    worker_handle: Mutex<Option<JoinHandle<()>>>,
    /// Current device info
    current_device: RwLock<Option<DeviceInfo>>,
    /// Current output directory
    current_output_dir: RwLock<Option<PathBuf>>,
    /// Last error message
    last_error: RwLock<Option<String>>,
}

impl ExtractionController {
    /// Create a new extraction controller
    pub fn new() -> Self {
        let (command_tx, _command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        Self {
            state: Arc::new(AtomicU8::new(ControllerState::Idle as u8)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            pause_flag: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(ProgressTracker::new()),
            _command_tx: command_tx,
            event_rx: Mutex::new(event_rx),
            event_tx,
            worker_handle: Mutex::new(None),
            current_device: RwLock::new(None),
            current_output_dir: RwLock::new(None),
            last_error: RwLock::new(None),
        }
    }

    /// Get current state
    pub fn state(&self) -> ControllerState {
        ControllerState::from(self.state.load(Ordering::SeqCst))
    }

    /// Check if extraction is active (scanning or extracting)
    pub fn is_active(&self) -> bool {
        matches!(
            self.state(),
            ControllerState::Scanning | ControllerState::Extracting
        )
    }

    /// Check if paused
    pub fn is_paused(&self) -> bool {
        self.state() == ControllerState::Paused
    }

    /// Get progress tracker reference
    pub fn progress(&self) -> &Arc<ProgressTracker> {
        &self.progress
    }

    /// Get current device info
    pub fn current_device(&self) -> Option<DeviceInfo> {
        self.current_device.read().unwrap().clone()
    }

    /// Get current output directory
    pub fn current_output_dir(&self) -> Option<PathBuf> {
        self.current_output_dir.read().unwrap().clone()
    }

    /// Get last error
    pub fn last_error(&self) -> Option<String> {
        self.last_error.read().unwrap().clone()
    }

    /// Try to receive the next event (non-blocking)
    pub fn try_recv_event(&self) -> Option<UiEvent> {
        match self.event_rx.lock().unwrap().try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }

    /// Receive events with timeout
    pub fn recv_event_timeout(&self, timeout: Duration) -> Option<UiEvent> {
        self.event_rx.lock().unwrap().recv_timeout(timeout).ok()
    }

    /// Drain all pending events
    pub fn drain_events(&self) -> Vec<UiEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.try_recv_event() {
            events.push(event);
        }
        events
    }

    /// Start extraction on a device
    ///
    /// This spawns a background thread to perform the extraction.
    pub fn start_extraction<M, C>(
        &self,
        device_manager: Arc<M>,
        device_info: DeviceInfo,
        config: ExtractionConfig,
    ) -> Result<()>
    where
        M: DeviceManagerTrait<Content = C> + Send + Sync + 'static,
        C: DeviceContentTrait + Send + 'static,
    {
        // Check if already running
        if self.is_active() {
            return Err(ExtractionError::DeviceError(
                "Extraction already in progress".to_string(),
            ));
        }

        // Reset state
        self.shutdown_flag.store(false, Ordering::SeqCst);
        self.pause_flag.store(false, Ordering::SeqCst);
        self.state
            .store(ControllerState::Scanning as u8, Ordering::SeqCst);
        self.progress.reset();
        *self.current_device.write().unwrap() = Some(device_info.clone());
        *self.current_output_dir.write().unwrap() = Some(config.output_dir.clone());
        *self.last_error.write().unwrap() = None;

        // Clone what we need for the thread
        let state = Arc::clone(&self.state);
        let shutdown_flag = Arc::clone(&self.shutdown_flag);
        let pause_flag = Arc::clone(&self.pause_flag);
        let progress = Arc::clone(&self.progress);
        let event_tx = self.event_tx.clone();

        // Emit started event
        let _ = event_tx.send(UiEvent::Extraction(ExtractionEvent::Started {
            device: device_info.clone(),
            output_dir: config.output_dir.clone(),
            started_at: Instant::now(),
        }));

        // Spawn worker thread
        let handle = thread::spawn(move || {
            Self::extraction_worker(
                device_manager,
                device_info,
                config,
                state,
                shutdown_flag,
                pause_flag,
                progress,
                event_tx,
            );
        });

        *self.worker_handle.lock().unwrap() = Some(handle);
        Ok(())
    }

    /// Pause extraction
    pub fn pause(&self) -> Result<()> {
        if !self.is_active() {
            return Err(ExtractionError::DeviceError(
                "No active extraction to pause".to_string(),
            ));
        }

        self.pause_flag.store(true, Ordering::SeqCst);
        self.state
            .store(ControllerState::Paused as u8, Ordering::SeqCst);

        let _ = self
            .event_tx
            .send(UiEvent::Extraction(ExtractionEvent::Paused {
                reason: PauseReason::UserRequested,
            }));

        Ok(())
    }

    /// Resume paused extraction
    pub fn resume(&self) -> Result<()> {
        if self.state() != ControllerState::Paused {
            return Err(ExtractionError::DeviceError(
                "Extraction is not paused".to_string(),
            ));
        }

        self.pause_flag.store(false, Ordering::SeqCst);
        self.state
            .store(ControllerState::Extracting as u8, Ordering::SeqCst);

        let _ = self
            .event_tx
            .send(UiEvent::Extraction(ExtractionEvent::Resumed));

        Ok(())
    }

    /// Cancel extraction
    pub fn cancel(&self) -> Result<()> {
        if !self.is_active() && !self.is_paused() {
            return Err(ExtractionError::DeviceError(
                "No active extraction to cancel".to_string(),
            ));
        }

        self.state
            .store(ControllerState::Cancelling as u8, Ordering::SeqCst);
        self.shutdown_flag.store(true, Ordering::SeqCst);
        self.pause_flag.store(false, Ordering::SeqCst); // Unpause so thread can exit

        let _ = self
            .event_tx
            .send(UiEvent::Extraction(ExtractionEvent::Cancelling));

        Ok(())
    }

    /// Wait for extraction to complete
    pub fn wait(&self) -> Result<()> {
        if let Some(handle) = self.worker_handle.lock().unwrap().take() {
            handle
                .join()
                .map_err(|_| ExtractionError::DeviceError("Worker thread panicked".to_string()))?;
        }
        Ok(())
    }

    /// Shutdown the controller
    pub fn shutdown(&self) {
        // Cancel any running extraction
        let _ = self.cancel();

        // Wait for thread to finish
        let _ = self.wait();

        // Emit shutdown event
        let _ = self.event_tx.send(UiEvent::App(AppEvent::ShuttingDown));
    }

    /// Worker thread function that performs the actual extraction
    fn extraction_worker<M, C>(
        device_manager: Arc<M>,
        device_info: DeviceInfo,
        config: ExtractionConfig,
        state: Arc<AtomicU8>,
        shutdown_flag: Arc<AtomicBool>,
        pause_flag: Arc<AtomicBool>,
        progress: Arc<ProgressTracker>,
        event_tx: Sender<UiEvent>,
    ) where
        M: DeviceManagerTrait<Content = C> + Send + Sync + 'static,
        C: DeviceContentTrait + Send + 'static,
    {
        let start_time = Instant::now();

        // Open device content
        let content = match device_manager.open_device(&device_info.device_id) {
            Ok(c) => c,
            Err(e) => {
                state.store(ControllerState::Error as u8, Ordering::SeqCst);
                let _ = event_tx.send(UiEvent::Extraction(ExtractionEvent::FatalError {
                    error: format!("Failed to open device: {}", e),
                    context: Some("Device may be locked or disconnected".to_string()),
                }));
                return;
            }
        };

        // Build duplicate index if configured
        let _duplicate_index = if let Some(ref dup_config) = config.duplicate_config {
            let _ = event_tx.send(UiEvent::App(AppEvent::Log {
                level: crate::ui::events::LogLevel::Info,
                message: "Building duplicate index...".to_string(),
                source: Some("DuplicateDetector".to_string()),
            }));

            match DuplicateIndex::build_from_folders(dup_config, Arc::clone(&shutdown_flag), |_| {})
            {
                Ok(index) => Some(index),
                Err(e) => {
                    let _ = event_tx.send(UiEvent::App(AppEvent::Log {
                        level: crate::ui::events::LogLevel::Warning,
                        message: format!("Failed to build duplicate index: {}", e),
                        source: Some("DuplicateDetector".to_string()),
                    }));
                    None
                }
            }
        } else {
            None
        };

        // Emit scan started
        let _ = event_tx.send(UiEvent::Extraction(ExtractionEvent::ScanStarted));

        // Create progress callback
        let progress_clone = Arc::clone(&progress);
        let event_tx_clone = event_tx.clone();
        let pause_flag_clone = Arc::clone(&pause_flag);

        let progress_callback: Arc<dyn Fn(ProgressUpdate) + Send + Sync> =
            Arc::new(move |update: ProgressUpdate| {
                // Handle pause
                while pause_flag_clone.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(100));
                }

                match update.phase {
                    ExtractionPhase::Scanning => {
                        let _ = event_tx_clone.send(UiEvent::Extraction(
                            ExtractionEvent::ScanProgress {
                                folders_scanned: update.current_index,
                                files_found: update.total_files,
                                current_folder: update.current_file.clone(),
                            },
                        ));
                    }
                    ExtractionPhase::Extracting => {
                        let snapshot = progress_clone.snapshot();
                        let _ =
                            event_tx_clone.send(UiEvent::Extraction(ExtractionEvent::Progress {
                                files_extracted: snapshot.files_extracted,
                                files_skipped: snapshot.files_skipped,
                                duplicates_found: snapshot.duplicates_found,
                                errors: snapshot.errors,
                                bytes_processed: snapshot.bytes_processed,
                                eta: snapshot.eta,
                                speed_bps: snapshot.speed_bps,
                                percent_complete: snapshot.percent_complete,
                            }));
                    }
                    ExtractionPhase::Complete => {
                        // Handled separately
                    }
                }
            });

        // Create extractor config
        let extractor_config = GenericExtractionConfig {
            output_dir: config.output_dir.clone(),
            dcim_only: config.dcim_only,
            preserve_structure: config.preserve_structure,
            skip_existing: config.skip_existing,
            write_files: true,
            max_files: config.max_files,
            progress_callback: Some(progress_callback),
        };

        // Create and run extractor
        let mut extractor =
            GenericExtractor::with_shutdown_flag(extractor_config, Arc::clone(&shutdown_flag));

        // Transition to extracting state
        state.store(ControllerState::Extracting as u8, Ordering::SeqCst);

        // Run extraction
        let result = extractor.extract_from_content(&content);

        // Process result
        let _snapshot = progress.snapshot();
        let duration = start_time.elapsed();

        match result {
            Ok(stats) => {
                let was_cancelled = shutdown_flag.load(Ordering::SeqCst);

                if was_cancelled {
                    state.store(ControllerState::Idle as u8, Ordering::SeqCst);
                    let _ = event_tx.send(UiEvent::Extraction(ExtractionEvent::Cancelled {
                        files_extracted: stats.files_extracted,
                        resumable: true,
                    }));
                } else {
                    state.store(ControllerState::Completed as u8, Ordering::SeqCst);
                    let _ = event_tx.send(UiEvent::Extraction(ExtractionEvent::Completed {
                        stats: ExtractionSummary {
                            device: device_info,
                            output_dir: config.output_dir,
                            files_extracted: stats.files_extracted,
                            files_skipped: stats.files_skipped,
                            duplicates_found: stats.duplicates_found,
                            errors: stats.errors,
                            bytes_transferred: stats.bytes_processed,
                            duration,
                            average_speed_bps: if duration.as_secs() > 0 {
                                stats.bytes_processed / duration.as_secs()
                            } else {
                                stats.bytes_processed
                            },
                            completed_fully: true,
                            resumable: false,
                        },
                    }));
                }
            }
            Err(e) => {
                state.store(ControllerState::Error as u8, Ordering::SeqCst);
                let _ = event_tx.send(UiEvent::Extraction(ExtractionEvent::FatalError {
                    error: e.to_string(),
                    context: None,
                }));
            }
        }
    }
}

impl Default for ExtractionController {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ExtractionController {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// =============================================================================
// Extraction Config for Controller
// =============================================================================

/// Configuration for an extraction operation
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Output directory for extracted files
    pub output_dir: PathBuf,
    /// Optional duplicate detection configuration
    pub duplicate_config: Option<DuplicateConfig>,
    /// Whether to preserve folder structure from device
    pub preserve_structure: bool,
    /// Skip files that already exist at destination
    pub skip_existing: bool,
    /// Only extract from DCIM folder
    pub dcim_only: bool,
    /// Maximum number of files to extract (0 = unlimited)
    pub max_files: usize,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./extracted"),
            duplicate_config: None,
            preserve_structure: true,
            skip_existing: true,
            dcim_only: true,
            max_files: 0,
        }
    }
}

impl ExtractionConfig {
    /// Create a new extraction config with the given output directory
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            ..Default::default()
        }
    }

    /// Set duplicate detection config
    pub fn with_duplicate_detection(mut self, config: DuplicateConfig) -> Self {
        self.duplicate_config = Some(config);
        self
    }

    /// Set preserve structure
    pub fn preserve_structure(mut self, preserve: bool) -> Self {
        self.preserve_structure = preserve;
        self
    }

    /// Set skip existing
    pub fn skip_existing(mut self, skip: bool) -> Self {
        self.skip_existing = skip;
        self
    }

    /// Set DCIM only
    pub fn dcim_only(mut self, dcim: bool) -> Self {
        self.dcim_only = dcim;
        self
    }

    /// Set max files
    pub fn max_files(mut self, max: usize) -> Self {
        self.max_files = max;
        self
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_state_conversion() {
        assert_eq!(ControllerState::from(0), ControllerState::Idle);
        assert_eq!(ControllerState::from(1), ControllerState::Scanning);
        assert_eq!(ControllerState::from(2), ControllerState::Extracting);
        assert_eq!(ControllerState::from(3), ControllerState::Paused);
        assert_eq!(ControllerState::from(4), ControllerState::Cancelling);
        assert_eq!(ControllerState::from(5), ControllerState::Completed);
        assert_eq!(ControllerState::from(6), ControllerState::Error);
        assert_eq!(ControllerState::from(255), ControllerState::Idle); // Invalid
    }

    #[test]
    fn test_progress_tracker_basic() {
        let tracker = ProgressTracker::new();
        tracker.reset();
        tracker.set_totals(100, 1000);

        assert_eq!(tracker.snapshot().total_files, 100);
        assert_eq!(tracker.snapshot().total_bytes, 1000);

        tracker.record_extracted(100);
        assert_eq!(tracker.snapshot().files_extracted, 1);
        assert_eq!(tracker.snapshot().bytes_processed, 100);

        tracker.record_skipped();
        assert_eq!(tracker.snapshot().files_skipped, 1);

        tracker.record_duplicate();
        assert_eq!(tracker.snapshot().duplicates_found, 1);
        assert_eq!(tracker.snapshot().files_skipped, 2);

        tracker.record_error();
        assert_eq!(tracker.snapshot().errors, 1);
    }

    #[test]
    fn test_progress_tracker_percent() {
        let tracker = ProgressTracker::new();
        tracker.reset();
        tracker.set_totals(100, 1000);

        assert_eq!(tracker.percent_complete(), 0.0);

        tracker.record_extracted(500);
        assert!((tracker.percent_complete() - 50.0).abs() < 0.01);

        tracker.record_extracted(500);
        assert!((tracker.percent_complete() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_controller_creation() {
        let controller = ExtractionController::new();
        assert_eq!(controller.state(), ControllerState::Idle);
        assert!(!controller.is_active());
        assert!(!controller.is_paused());
    }

    #[test]
    fn test_extraction_config_builder() {
        let config = ExtractionConfig::new(PathBuf::from("/output"))
            .preserve_structure(false)
            .skip_existing(false)
            .dcim_only(false)
            .max_files(100);

        assert_eq!(config.output_dir, PathBuf::from("/output"));
        assert!(!config.preserve_structure);
        assert!(!config.skip_existing);
        assert!(!config.dcim_only);
        assert_eq!(config.max_files, 100);
    }

    #[test]
    fn test_pause_resume_without_active_extraction() {
        let controller = ExtractionController::new();

        // Should fail - no active extraction
        assert!(controller.pause().is_err());
        assert!(controller.resume().is_err());
        assert!(controller.cancel().is_err());
    }
}

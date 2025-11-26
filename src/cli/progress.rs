//! Progress bar utilities for CLI output
//!
//! This module provides progress tracking utilities for various CLI operations
//! including scanning, benchmarking, and file transfers.

use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Progress tracker for benchmark scan operations
pub struct BenchmarkProgress {
    spinner: ProgressBar,
    start_time: Instant,
}

impl BenchmarkProgress {
    /// Create a new benchmark progress tracker
    pub fn new() -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
        spinner.set_message("Initializing...");

        Self {
            spinner,
            start_time: Instant::now(),
        }
    }

    /// Log an event message
    pub fn log_event(&self, msg: &str) {
        self.spinner.println(format!("  → {}", msg));
    }

    /// Update the progress display
    pub fn update(&self, folders: usize, files: usize, media: usize) {
        let elapsed = self.start_time.elapsed().as_secs();
        self.spinner.set_message(format!(
            "Scanning: {} folders, {} files ({} media) - {}s elapsed",
            folders, files, media, elapsed
        ));
    }

    /// Finish and clear the progress display
    pub fn finish(&self) {
        self.spinner.finish_and_clear();
    }
}

impl Default for BenchmarkProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress tracker for scan operations
pub struct ScanProgressTracker {
    folders_scanned: AtomicUsize,
    files_found: AtomicUsize,
    spinner: ProgressBar,
    start_time: Instant,
}

impl ScanProgressTracker {
    /// Create a new scan progress tracker
    pub fn new() -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        spinner.set_message("Starting scan...");

        Self {
            folders_scanned: AtomicUsize::new(0),
            files_found: AtomicUsize::new(0),
            spinner,
            start_time: Instant::now(),
        }
    }

    /// Increment the folder count
    pub fn increment_folders(&self) {
        self.folders_scanned.fetch_add(1, Ordering::Relaxed);
        self.update_message();
    }

    /// Add to the file count
    pub fn add_files(&self, count: usize) {
        self.files_found.fetch_add(count, Ordering::Relaxed);
        self.update_message();
    }

    /// Update the progress message
    fn update_message(&self) {
        let folders = self.folders_scanned.load(Ordering::Relaxed);
        let files = self.files_found.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs();
        self.spinner.set_message(format!(
            "Scanning... {} folders, {} files found ({:.0}s elapsed)",
            folders, files, elapsed
        ));
    }

    /// Finish the progress display with a summary
    pub fn finish(&self) {
        let folders = self.folders_scanned.load(Ordering::Relaxed);
        let files = self.files_found.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();
        self.spinner.finish_with_message(format!(
            "✓ Scan complete: {} folders, {} files found in {:.1}s",
            folders,
            files,
            elapsed.as_secs_f64()
        ));
    }
}

impl Default for ScanProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// A writer that writes to both console and file
///
/// Used for logging to both stderr and a log file simultaneously.
pub struct DualWriter {
    pub console: std::io::Stderr,
    pub file: std::fs::File,
}

impl Write for DualWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Write to console
        let _ = self.console.write(buf);
        // Write to file
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let _ = self.console.flush();
        self.file.flush()
    }
}

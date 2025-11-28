//! Progress bar utilities for CLI output
//!
//! This module provides progress tracking utilities for various CLI operations
//! including scanning, benchmarking, and file transfers.
//!
//! Key features:
//! - Progress bars that suspend cleanly when logging
//! - Consistent visual styling across all operations
//! - Support for nested progress tracking

#![allow(dead_code)] // Many utilities here are for future use

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ============================================================================
// Styles - Consistent visual appearance
// ============================================================================

/// Get the spinner style for scanning operations
fn spinner_style() -> ProgressStyle {
    ProgressStyle::default_spinner()
        .template("{spinner:.cyan} {msg}")
        .unwrap()
        .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷")
}

/// Get the progress bar style for extraction operations
fn progress_bar_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("  {spinner:.green} [{bar:40.cyan/dim}] {pos}/{len} ({percent}%) {msg}")
        .unwrap()
        .progress_chars("━━╾─")
}

/// Get the style for completed progress bars
fn completed_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("  ✓ [{bar:40.green/dim}] {pos}/{len} ({percent}%) {msg}")
        .unwrap()
        .progress_chars("━━━")
}

// ============================================================================
// Console output helpers
// ============================================================================

/// Print a header section with a box
pub fn print_header(title: &str) {
    let width = 68;
    let title_padded = format!("{:^width$}", title, width = width - 4);
    println!();
    println!("╔{}╗", "═".repeat(width - 2));
    println!("║{}║", title_padded);
    println!("╚{}╝", "═".repeat(width - 2));
    println!();
}

/// Print a section divider
pub fn print_divider() {
    println!();
    println!("{}", "─".repeat(60));
    println!();
}

/// Print a success message with checkmark
pub fn print_success(msg: &str) {
    println!("  ✓ {}", msg);
}

/// Print an info message with bullet
pub fn print_info(msg: &str) {
    println!("  • {}", msg);
}

/// Print a warning message
pub fn print_warning(msg: &str) {
    println!("  ⚠ {}", msg);
}

/// Print an error message
pub fn print_error(msg: &str) {
    println!("  ✗ {}", msg);
}

/// Print a step in a process
pub fn print_step(step: usize, total: usize, msg: &str) {
    println!("  [{}/{}] {}", step, total, msg);
}

// ============================================================================
// Progress tracker for benchmark scan operations
// ============================================================================

/// Progress tracker for benchmark scan operations
pub struct BenchmarkProgress {
    spinner: ProgressBar,
    start_time: Instant,
    suspended: AtomicBool,
}

impl BenchmarkProgress {
    /// Create a new benchmark progress tracker
    pub fn new() -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(spinner_style());
        spinner.enable_steady_tick(Duration::from_millis(80));
        spinner.set_message("Initializing...");

        Self {
            spinner,
            start_time: Instant::now(),
            suspended: AtomicBool::new(false),
        }
    }

    /// Suspend the spinner to allow clean log output
    pub fn suspend(&self) {
        if !self.suspended.swap(true, Ordering::SeqCst) {
            self.spinner.disable_steady_tick();
        }
    }

    /// Resume the spinner after logging
    pub fn resume(&self) {
        if self.suspended.swap(false, Ordering::SeqCst) {
            self.spinner.enable_steady_tick(Duration::from_millis(80));
        }
    }

    /// Log an event message (suspends spinner automatically)
    pub fn log_event(&self, msg: &str) {
        self.spinner.suspend(|| {
            println!("  → {}", msg);
        });
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

    /// Finish with a summary message
    pub fn finish_with_summary(&self, folders: usize, files: usize, media: usize) {
        let elapsed = self.start_time.elapsed();
        self.spinner.finish_with_message(format!(
            "✓ Scan complete: {} folders, {} files ({} media) in {:.1}s",
            folders,
            files,
            media,
            elapsed.as_secs_f64()
        ));
    }
}

impl Default for BenchmarkProgress {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Progress tracker for scan operations
// ============================================================================

/// Progress tracker for scan operations with clean log output support
pub struct ScanProgressTracker {
    folders_scanned: AtomicUsize,
    files_found: AtomicUsize,
    spinner: ProgressBar,
    start_time: Instant,
    last_update: Mutex<Instant>,
    update_interval: Duration,
}

impl ScanProgressTracker {
    /// Create a new scan progress tracker
    pub fn new() -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(spinner_style());
        spinner.enable_steady_tick(Duration::from_millis(100));
        spinner.set_message("Scanning device...");

        Self {
            folders_scanned: AtomicUsize::new(0),
            files_found: AtomicUsize::new(0),
            spinner,
            start_time: Instant::now(),
            last_update: Mutex::new(Instant::now()),
            update_interval: Duration::from_millis(250), // Throttle updates
        }
    }

    /// Increment the folder count
    pub fn increment_folders(&self) {
        self.folders_scanned.fetch_add(1, Ordering::Relaxed);
        self.maybe_update_message();
    }

    /// Add to the file count
    pub fn add_files(&self, count: usize) {
        self.files_found.fetch_add(count, Ordering::Relaxed);
        self.maybe_update_message();
    }

    /// Update the progress message if enough time has passed
    fn maybe_update_message(&self) {
        let now = Instant::now();
        let should_update = {
            let last = self.last_update.lock().unwrap();
            now.duration_since(*last) >= self.update_interval
        };

        if should_update {
            self.update_message();
            if let Ok(mut last) = self.last_update.lock() {
                *last = now;
            }
        }
    }

    /// Force update the progress message
    fn update_message(&self) {
        let folders = self.folders_scanned.load(Ordering::Relaxed);
        let files = self.files_found.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs();
        self.spinner.set_message(format!(
            "Scanning: {} folders, {} media files ({:.0}s)",
            folders, files, elapsed
        ));
    }

    /// Log a message while suspending the progress display
    pub fn log(&self, msg: &str) {
        self.spinner.suspend(|| {
            println!("  {}", msg);
        });
    }

    /// Log an info message
    pub fn log_info(&self, msg: &str) {
        self.spinner.suspend(|| {
            println!("  • {}", msg);
        });
    }

    /// Finish the progress display with a summary
    pub fn finish(&self) {
        let folders = self.folders_scanned.load(Ordering::Relaxed);
        let files = self.files_found.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();
        self.spinner.finish_with_message(format!(
            "✓ Found {} media files in {} folders ({:.1}s)",
            files,
            folders,
            elapsed.as_secs_f64()
        ));
    }

    /// Finish with an error message
    pub fn finish_with_error(&self, msg: &str) {
        self.spinner.finish_with_message(format!("✗ {}", msg));
    }

    /// Get current counts
    pub fn counts(&self) -> (usize, usize) {
        (
            self.folders_scanned.load(Ordering::Relaxed),
            self.files_found.load(Ordering::Relaxed),
        )
    }
}

impl Default for ScanProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Extraction progress tracker
// ============================================================================

/// Progress tracker for file extraction operations
pub struct ExtractionProgress {
    progress_bar: ProgressBar,
    start_time: Instant,
    bytes_processed: AtomicUsize,
    current_file: Mutex<String>,
}

impl ExtractionProgress {
    /// Create a new extraction progress tracker
    pub fn new(total_files: u64) -> Self {
        let progress_bar = ProgressBar::new(total_files);
        progress_bar.set_style(progress_bar_style());
        progress_bar.enable_steady_tick(Duration::from_millis(100));
        progress_bar.set_message("Starting...");

        Self {
            progress_bar,
            start_time: Instant::now(),
            bytes_processed: AtomicUsize::new(0),
            current_file: Mutex::new(String::new()),
        }
    }

    /// Update progress for a completed file
    pub fn file_completed(&self, filename: &str, bytes: u64) {
        self.bytes_processed
            .fetch_add(bytes as usize, Ordering::Relaxed);
        if let Ok(mut current) = self.current_file.lock() {
            *current = filename.to_string();
        }
        self.progress_bar.inc(1);
        self.update_message();
    }

    /// Update progress for a skipped file
    pub fn file_skipped(&self, _filename: &str) {
        self.progress_bar.inc(1);
        self.update_message();
    }

    /// Update the progress message
    fn update_message(&self) {
        let bytes = self.bytes_processed.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let rate = if elapsed > 0.0 {
            bytes as f64 / elapsed / 1024.0 / 1024.0
        } else {
            0.0
        };

        self.progress_bar.set_message(format!("{:.1} MB/s", rate));
    }

    /// Log a message while suspending the progress display
    pub fn log(&self, msg: &str) {
        self.progress_bar.suspend(|| {
            println!("  {}", msg);
        });
    }

    /// Log a warning message
    pub fn log_warning(&self, msg: &str) {
        self.progress_bar.suspend(|| {
            println!("  ⚠ {}", msg);
        });
    }

    /// Finish the progress display
    pub fn finish(&self) {
        self.progress_bar.set_style(completed_style());
        let elapsed = self.start_time.elapsed();
        let bytes = self.bytes_processed.load(Ordering::Relaxed);
        self.progress_bar.finish_with_message(format!(
            "Complete ({} in {:.1}s)",
            format_bytes(bytes as u64),
            elapsed.as_secs_f64()
        ));
    }

    /// Finish with an error
    pub fn finish_with_error(&self, msg: &str) {
        self.progress_bar.abandon_with_message(format!("✗ {}", msg));
    }

    /// Get the underlying progress bar for advanced use
    pub fn bar(&self) -> &ProgressBar {
        &self.progress_bar
    }
}

// ============================================================================
// Multi-stage progress tracker
// ============================================================================

/// A multi-stage progress tracker for complex operations
pub struct MultiStageProgress {
    multi: MultiProgress,
    stages: Vec<ProgressBar>,
    current_stage: AtomicUsize,
}

impl MultiStageProgress {
    /// Create a new multi-stage progress tracker
    pub fn new(stage_names: &[&str]) -> Self {
        let multi = MultiProgress::new();
        let stages: Vec<ProgressBar> = stage_names
            .iter()
            .map(|name| {
                let pb = multi.add(ProgressBar::new_spinner());
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("  {spinner:.dim} {msg}")
                        .unwrap()
                        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
                );
                pb.set_message(format!("⋯ {}", name));
                pb
            })
            .collect();

        Self {
            multi,
            stages,
            current_stage: AtomicUsize::new(0),
        }
    }

    /// Start the next stage
    pub fn start_stage(&self, index: usize) {
        if index < self.stages.len() {
            self.current_stage.store(index, Ordering::SeqCst);
            self.stages[index].enable_steady_tick(Duration::from_millis(100));
            self.stages[index].set_style(spinner_style());
        }
    }

    /// Complete a stage
    pub fn complete_stage(&self, index: usize, message: &str) {
        if index < self.stages.len() {
            self.stages[index].set_style(
                ProgressStyle::default_spinner()
                    .template("  ✓ {msg:.green}")
                    .unwrap(),
            );
            self.stages[index].finish_with_message(message.to_string());
        }
    }

    /// Fail a stage
    pub fn fail_stage(&self, index: usize, message: &str) {
        if index < self.stages.len() {
            self.stages[index].set_style(
                ProgressStyle::default_spinner()
                    .template("  ✗ {msg:.red}")
                    .unwrap(),
            );
            self.stages[index].finish_with_message(message.to_string());
        }
    }

    /// Update a stage message
    pub fn update_stage(&self, index: usize, message: &str) {
        if index < self.stages.len() {
            self.stages[index].set_message(message.to_string());
        }
    }

    /// Log a message (suspends all progress bars)
    pub fn log(&self, msg: &str) {
        self.multi.suspend(|| {
            println!("{}", msg);
        });
    }
}

// ============================================================================
// Utility functions
// ============================================================================

/// Format bytes as human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Format duration as human-readable string
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs >= 3600 {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{}h {}m", hours, mins)
    } else if secs >= 60 {
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{}m {}s", mins, secs)
    } else {
        format!("{:.1}s", duration.as_secs_f64())
    }
}

// ============================================================================
// Dual writer for file + console logging
// ============================================================================

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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 bytes");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30.0s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
    }

    #[test]
    fn test_scan_progress_tracker() {
        let tracker = ScanProgressTracker::new();
        tracker.increment_folders();
        tracker.add_files(5);
        let (folders, files) = tracker.counts();
        assert_eq!(folders, 1);
        assert_eq!(files, 5);
    }
}

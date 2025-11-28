//! Photo extraction module
//!
//! Handles the actual extraction of photos from connected iOS and Android devices.
//! This module manages the full extraction workflow including:
//! - Device connection and content enumeration
//! - Progress tracking with visual feedback
//! - Duplicate detection integration
//! - State tracking for resume support
//! - Android-specific folder structure handling

use crate::core::config::{
    app_folders, AndroidConfig, DuplicateAction, DuplicateDetectionConfig, TrackingConfig,
};
use crate::core::error::{ExtractionError, Result};
use crate::core::tracking::StateTracker;
use crate::device::traits::{DeviceContentTrait, DeviceInfo, DeviceManagerTrait, DeviceObject};
use crate::device::wpd::{DeviceContent, DeviceManager};
use crate::duplicate::{compute_data_hash, DuplicateIndex};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info, trace, warn};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use windows::core::PCWSTR;
use windows::Win32::Devices::PortableDevices::WPD_RESOURCE_DEFAULT;
use windows::Win32::System::Com::IStream;

/// Configuration for photo extraction
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Output directory for extracted photos
    pub output_dir: PathBuf,
    /// Whether to only extract from DCIM folder
    pub dcim_only: bool,
    /// Whether to preserve folder structure from device
    pub preserve_structure: bool,
    /// Whether to skip existing files
    pub skip_existing: bool,
    /// Duplicate detection configuration
    pub duplicate_detection: Option<DuplicateDetectionConfig>,
    /// Tracking configuration
    pub tracking: Option<TrackingConfig>,
    /// Quiet mode - suppresses console output (for parallel extraction)
    pub quiet: bool,
    /// Android-specific configuration (None = use defaults)
    pub android_config: Option<AndroidConfig>,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./extracted_photos"),
            dcim_only: true,
            preserve_structure: true,
            skip_existing: true,
            duplicate_detection: None,
            tracking: None,
            quiet: false,
            android_config: None,
        }
    }
}

/// Statistics about the extraction process
#[derive(Debug, Default, Clone)]
pub struct ExtractionStats {
    pub files_extracted: usize,
    pub files_skipped: usize,
    pub duplicates_skipped: usize,
    pub duplicates_overwritten: usize,
    pub duplicates_renamed: usize,
    pub errors: usize,
    pub total_bytes: u64,
}

/// Information about a photo on the device
#[derive(Debug, Clone)]
struct PhotoInfo {
    /// WPD object ID
    object_id: String,
    /// File name
    name: String,
    /// Relative path on device
    path: String,
    /// File size in bytes
    size: u64,
    /// Date modified (ISO 8601 string, if available from device)
    date_modified: Option<String>,
}

/// Progress tracker for scan operations
struct ScanProgress {
    folders_scanned: AtomicUsize,
    files_found: AtomicUsize,
    spinner: ProgressBar,
    start_time: Instant,
}

impl ScanProgress {
    fn new() -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("  {spinner:.cyan} {msg}")
                .unwrap()
                .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷"),
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
        spinner.set_message("Scanning device...");

        Self {
            folders_scanned: AtomicUsize::new(0),
            files_found: AtomicUsize::new(0),
            spinner,
            start_time: Instant::now(),
        }
    }

    fn increment_folders(&self) {
        self.folders_scanned.fetch_add(1, Ordering::Relaxed);
        self.maybe_update_message();
    }

    fn add_files(&self, count: usize) {
        self.files_found.fetch_add(count, Ordering::Relaxed);
        self.maybe_update_message();
    }

    /// Only update message periodically to reduce visual noise
    fn maybe_update_message(&self) {
        let folders = self.folders_scanned.load(Ordering::Relaxed);
        // Update every 5 folders or when we find files
        if folders % 5 == 0 {
            self.update_message();
        }
    }

    fn update_message(&self) {
        let folders = self.folders_scanned.load(Ordering::Relaxed);
        let files = self.files_found.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs();
        self.spinner.set_message(format!(
            "Scanning: {} folders, {} media files ({:.0}s)",
            folders, files, elapsed
        ));
    }

    fn finish(&self) {
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
}

/// File extensions to extract
const PHOTO_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "heic", "heif", "gif", "webp", "raw", "dng", "tiff", "tif", "bmp",
];

const VIDEO_EXTENSIONS: &[&str] = &["mov", "mp4", "m4v", "avi", "3gp"];

/// Progress callback for extraction - receives (files_processed_this_update, bytes_this_update)
pub type ProgressCallback = Box<dyn Fn(usize, u64) + Send>;

/// Callback to report total files to process
pub type TotalFilesCallback = Box<dyn Fn(usize) + Send>;

/// Extract photos from a device
pub fn extract_photos(
    device_info: &DeviceInfo,
    config: ExtractionConfig,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<ExtractionStats> {
    extract_photos_with_progress(device_info, config, shutdown_flag, None, None)
}

/// Extract photos from a device with progress callbacks
///
/// - `progress_callback`: receives (files_processed, bytes_transferred) updates per file
/// - `total_files_callback`: called once with total number of files to process
pub fn extract_photos_with_progress(
    device_info: &DeviceInfo,
    config: ExtractionConfig,
    shutdown_flag: Arc<AtomicBool>,
    progress_callback: Option<ProgressCallback>,
    total_files_callback: Option<TotalFilesCallback>,
) -> Result<ExtractionStats> {
    let quiet = config.quiet;

    // Print clean user-facing output
    if !quiet {
        println!();
        println!("  📱 Device: {}", device_info.friendly_name);
    }

    debug!("Opening device: {}", device_info.friendly_name);

    // Create device manager and open device
    let manager = DeviceManager::new()?;
    let content = manager.open_device(&device_info.device_id)?;

    // Create output directory
    fs::create_dir_all(&config.output_dir).map_err(|e| {
        ExtractionError::IoError(format!(
            "Failed to create output directory '{}': {}",
            config.output_dir.display(),
            e
        ))
    })?;

    if !quiet {
        println!("  📁 Output: {}", config.output_dir.display());
    }
    debug!("Output directory: {}", config.output_dir.display());

    // Initialize state tracker if enabled
    let mut tracker = if let Some(ref tracking_config) = config.tracking {
        if tracking_config.enabled {
            let mut tracker = StateTracker::new(tracking_config, &config.output_dir);
            if let Err(e) = tracker.load_or_create(device_info) {
                warn!("Failed to load tracking state: {}", e);
            }
            tracker.start_session();
            Some(tracker)
        } else {
            None
        }
    } else {
        None
    };

    // Build duplicate detection index if enabled
    let hash_index = if let Some(ref dup_config) = config.duplicate_detection {
        if dup_config.enabled && !dup_config.comparison_folders.is_empty() {
            if !quiet {
                println!("  🔍 Building duplicate detection index...");
            }
            debug!("Building duplicate detection index...");
            let detector_config = dup_config.to_detector_config();
            match DuplicateIndex::build_from_folders(
                &detector_config,
                shutdown_flag.clone(),
                |progress| {
                    if progress.current % 500 == 0 {
                        trace!(
                            "Indexing progress: {}/{} files",
                            progress.current,
                            progress.total
                        );
                    }
                },
            ) {
                Ok(index) => {
                    debug!(
                        "Duplicate detection index built with {} entries ({} unique hashes)",
                        index.len(),
                        index.stats().unique_hashes
                    );
                    Some(index)
                }
                Err(e) => {
                    if !quiet {
                        println!("  ⚠ Could not build duplicate index, continuing without");
                    }
                    debug!("Failed to build duplicate index: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    if !quiet {
        println!();
    }

    // Find all photos - progress is shown by ScanProgress (only in non-quiet mode)
    let all_photos = find_all_photos_with_progress(
        &content,
        config.dcim_only,
        quiet,
        config.android_config.as_ref(),
    )?;
    let total_on_device = all_photos.len();

    if total_on_device == 0 {
        if !quiet {
            println!("  ⚠ No photos or videos found on the device");
            println!();
            println!("  Possible reasons:");
            println!("    • Device is locked - please unlock and try again");
            println!("    • No DCIM folder found - try with --dcim-only false");
            println!("    • Photos may be in iCloud - download them to device first");
        }
        return Ok(ExtractionStats::default());
    }

    debug!("Found {} photos/videos on device", total_on_device);

    // Filter out already-extracted files using tracking state
    let (photos, already_extracted_count) = if let Some(ref t) = tracker {
        let mut new_photos = Vec::new();
        let mut skipped = 0u64;
        for photo in all_photos {
            if t.is_file_extracted(&photo.object_id) {
                skipped += 1;
            } else {
                new_photos.push(photo);
            }
        }
        (new_photos, skipped)
    } else {
        (all_photos, 0)
    };

    let total = photos.len();

    // Show summary before extraction
    if !quiet {
        println!("  📊 Summary:");
        println!("     Total on device:    {}", total_on_device);
        if already_extracted_count > 0 {
            println!("     Already extracted:  {}", already_extracted_count);
        }
        println!("     New files to copy:  {}", total);
        println!();
    }

    // Report total files to process via callback
    if let Some(ref cb) = total_files_callback {
        cb(total);
    }

    if total == 0 {
        if !quiet {
            println!("  ✓ All files have already been extracted!");
        }
        // End tracking session
        if let Some(ref mut t) = tracker {
            t.end_session(true, false);
            if let Err(e) = t.save() {
                debug!("Failed to save tracking state: {}", e);
            }
        }
        return Ok(ExtractionStats {
            files_skipped: already_extracted_count as usize,
            ..Default::default()
        });
    }

    // Set up progress bar with clean style (hidden in quiet mode)
    let progress = if quiet {
        ProgressBar::hidden()
    } else {
        let pb = ProgressBar::new(total as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  {spinner:.green} [{bar:40.cyan/dim}] {pos}/{len} ({percent}%) {msg}")
                .expect("Invalid progress template")
                .progress_chars("━━╾─"),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    };

    let mut stats = ExtractionStats::default();
    let extract_start = Instant::now();

    // Extract each photo
    for (index, photo) in photos.iter().enumerate() {
        // Check for shutdown request before processing each file
        if shutdown_flag.load(Ordering::SeqCst) {
            if !quiet {
                progress.finish_with_message("⚠ Interrupted");
                println!();
                println!("  ⚠ Extraction interrupted by user");
            }
            return Ok(stats);
        }

        progress.set_position(index as u64);

        // Show current file (truncated) and transfer rate
        let elapsed = extract_start.elapsed().as_secs_f64();
        let rate = if elapsed > 0.0 && stats.total_bytes > 0 {
            format!(
                "{:.1} MB/s",
                stats.total_bytes as f64 / elapsed / 1024.0 / 1024.0
            )
        } else {
            String::new()
        };
        let display_name: String = photo.name.chars().take(25).collect();
        progress.set_message(format!("{} {}", display_name, rate));

        match extract_single_photo(&content, photo, &config, &hash_index) {
            Ok(ExtractResult::Extracted(bytes)) => {
                stats.files_extracted += 1;
                stats.total_bytes += bytes;
                if let Some(ref mut t) = tracker {
                    t.record_extracted(&photo.object_id, bytes);
                }
                if let Some(ref cb) = progress_callback {
                    cb(1, bytes);
                }
            }
            Ok(ExtractResult::Skipped) => {
                stats.files_skipped += 1;
                if let Some(ref mut t) = tracker {
                    t.record_skipped();
                }
                if let Some(ref cb) = progress_callback {
                    cb(1, 0);
                }
            }
            Ok(ExtractResult::Duplicate(path)) => {
                trace!("Skipping duplicate of: {}", path.display());
                stats.duplicates_skipped += 1;
                if let Some(ref mut t) = tracker {
                    t.record_duplicate();
                }
                if let Some(ref cb) = progress_callback {
                    cb(1, 0);
                }
            }
            Ok(ExtractResult::DuplicateOverwritten(bytes)) => {
                trace!("Overwrote duplicate: {}", photo.name);
                stats.duplicates_overwritten += 1;
                stats.total_bytes += bytes;
                if let Some(ref mut t) = tracker {
                    t.record_extracted(&photo.object_id, bytes);
                }
                if let Some(ref cb) = progress_callback {
                    cb(1, bytes);
                }
            }
            Ok(ExtractResult::DuplicateRenamed(bytes)) => {
                trace!("Renamed duplicate: {}", photo.name);
                stats.duplicates_renamed += 1;
                stats.total_bytes += bytes;
                if let Some(ref mut t) = tracker {
                    t.record_extracted(&photo.object_id, bytes);
                }
                if let Some(ref cb) = progress_callback {
                    cb(1, bytes);
                }
            }
            Err(e) => {
                // Only log errors at debug level to avoid cluttering output
                debug!("Failed to extract '{}': {}", photo.name, e);
                stats.errors += 1;
                if let Some(ref mut t) = tracker {
                    t.record_error();
                }
                if let Some(ref cb) = progress_callback {
                    cb(1, 0);
                }
            }
        }
    }

    // Calculate final stats
    let elapsed = extract_start.elapsed();
    let rate = if elapsed.as_secs_f64() > 0.0 {
        stats.total_bytes as f64 / elapsed.as_secs_f64() / 1024.0 / 1024.0
    } else {
        0.0
    };

    // Show completion with transfer rate
    if !quiet {
        progress.set_style(
            ProgressStyle::default_bar()
                .template("  ✓ [{bar:40.green/dim}] {pos}/{len} Complete")
                .expect("Invalid progress template")
                .progress_chars("━━━"),
        );
        progress.finish();
    }

    // End tracking session
    let was_interrupted = shutdown_flag.load(Ordering::SeqCst);
    if let Some(ref mut t) = tracker {
        t.end_session(!was_interrupted, was_interrupted);
        if let Err(e) = t.save() {
            debug!("Failed to save tracking state: {}", e);
        }
    }

    // Print summary
    if !quiet {
        println!();
        println!("  ─────────────────────────────────────────");
        println!("  📊 Extraction Results:");
        println!("     Files extracted:  {}", stats.files_extracted);
        if stats.files_skipped > 0 {
            println!("     Files skipped:    {}", stats.files_skipped);
        }
        if stats.duplicates_skipped > 0 {
            println!("     Duplicates:       {}", stats.duplicates_skipped);
        }
        if stats.errors > 0 {
            println!("     Errors:           {}", stats.errors);
        }
        println!("     Total size:       {}", format_size(stats.total_bytes));
        println!(
            "     Duration:         {:.1}s ({:.1} MB/s)",
            elapsed.as_secs_f64(),
            rate
        );
        println!();
    }

    Ok(stats)
}

/// Format bytes as human-readable size
fn format_size(bytes: u64) -> String {
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

/// Find all photos on the device recursively with optional progress display
fn find_all_photos_with_progress(
    content: &DeviceContent,
    dcim_only: bool,
    quiet: bool,
    android_config: Option<&AndroidConfig>,
) -> Result<Vec<PhotoInfo>> {
    let mut photos = Vec::new();
    let progress = if quiet {
        None
    } else {
        Some(ScanProgress::new())
    };

    let root_objects = content.enumerate_objects()?;

    debug!("Found {} root objects on device", root_objects.len());

    for obj in &root_objects {
        debug!(
            "Root object: '{}' (id: {}, folder: {}, size: {})",
            obj.name, obj.object_id, obj.is_folder, obj.size
        );
    }

    // If no root objects found, try to explain why
    if root_objects.is_empty() {
        warn!("No root objects found on device!");
        warn!("This usually means:");
        warn!("  1. The device is locked — please unlock it");
        warn!("  2. You haven't tapped 'Trust' or 'Allow' on the device");
        warn!("  3. Try a different USB cable or port");
        return Ok(photos);
    }

    // If Android config is provided, use Android-specific scanning
    if let Some(android_cfg) = android_config {
        debug!("Using Android-specific folder scanning");
        let android_photos = scan_android_device(content, android_cfg, &progress)?;
        if let Some(ref p) = progress {
            p.finish();
        }
        return Ok(android_photos);
    }

    // Look for storage or DCIM folder
    for obj in root_objects {
        let name_upper = obj.name.to_uppercase();
        debug!(
            "Processing root object: '{}' (uppercase: '{}')",
            obj.name, name_upper
        );

        // Always scan the object if it's a folder - we need to find DCIM inside
        if obj.is_folder {
            debug!("'{}' is a folder, scanning its contents...", obj.name);

            // Enumerate children of this object
            match content.enumerate_children(&obj.object_id) {
                Ok(children) => {
                    debug!("Found {} children in '{}'", children.len(), obj.name);

                    for child in &children {
                        debug!(
                            "  Child: '{}' (id: {}, folder: {}, size: {})",
                            child.name, child.object_id, child.is_folder, child.size
                        );
                    }

                    if dcim_only {
                        // First, look for traditional DCIM folder
                        let mut found_dcim = false;
                        for child in &children {
                            let child_name_upper = child.name.to_uppercase();
                            if child_name_upper == "DCIM" && child.is_folder {
                                debug!("Found DCIM folder inside '{}', scanning...", obj.name);
                                found_dcim = true;
                                scan_folder_recursive_quiet(
                                    content,
                                    child,
                                    "DCIM",
                                    &mut photos,
                                    &progress,
                                )?;
                            }
                        }

                        // If no DCIM found, look for iOS date-based photo folders
                        // These have patterns like "202511__", "201902__", "202506_a", etc.
                        // (6 digits for YYYYMM followed by underscore or other chars)
                        if !found_dcim {
                            debug!(
                                "No DCIM folder found, scanning for date-based photo folders..."
                            );
                            for child in children {
                                if child.is_folder && is_ios_photo_folder(&child.name) {
                                    debug!("Found photo folder '{}', scanning...", child.name);
                                    let path = child.name.clone();
                                    scan_folder_recursive_quiet(
                                        content,
                                        &child,
                                        &path,
                                        &mut photos,
                                        &progress,
                                    )?;
                                }
                            }
                        }
                    } else {
                        // Scan everything inside
                        for child in children {
                            if child.is_folder {
                                let path = format!("{}/{}", obj.name, child.name);
                                scan_folder_recursive_quiet(
                                    content,
                                    &child,
                                    &path,
                                    &mut photos,
                                    &progress,
                                )?;
                            } else if is_media_file(&child.name) {
                                photos.push(PhotoInfo {
                                    object_id: child.object_id.clone(),
                                    name: child.name.clone(),
                                    path: format!("{}/{}", obj.name, child.name),
                                    size: child.size,
                                    date_modified: child.date_modified.clone(),
                                });
                                if let Some(ref p) = progress {
                                    p.add_files(1);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to enumerate children of '{}': {}", obj.name, e);
                }
            }
        } else if !dcim_only && is_media_file(&obj.name) {
            // It's a file at root level
            photos.push(PhotoInfo {
                object_id: obj.object_id.clone(),
                name: obj.name.clone(),
                path: obj.name.clone(),
                size: obj.size,
                date_modified: obj.date_modified.clone(),
            });
        }
    }

    if photos.is_empty() && dcim_only {
        warn!("No photos found. Try running with --dcim-only false to scan all folders.");
    }

    if let Some(ref p) = progress {
        p.finish();
    }
    Ok(photos)
}

/// Scan folder recursively with optional progress tracking (for quiet mode support)
fn scan_folder_recursive_quiet(
    content: &DeviceContent,
    folder: &DeviceObject,
    path: &str,
    photos: &mut Vec<PhotoInfo>,
    progress: &Option<ScanProgress>,
) -> Result<()> {
    if let Some(ref p) = progress {
        p.increment_folders();
    }

    let children = content.enumerate_children(&folder.object_id)?;

    for child in children {
        if child.is_folder {
            let child_path = format!("{}/{}", path, child.name);
            scan_folder_recursive_quiet(content, &child, &child_path, photos, progress)?;
        } else if is_media_file(&child.name) {
            photos.push(PhotoInfo {
                object_id: child.object_id.clone(),
                name: child.name.clone(),
                path: format!("{}/{}", path, child.name),
                size: child.size,
                date_modified: child.date_modified.clone(),
            });
            if let Some(ref p) = progress {
                p.add_files(1);
            }
        }
    }

    Ok(())
}

/// Check if a folder name looks like an iOS photo folder
/// iOS sometimes uses date-based folder names like "202511__", "201902__", "202506_a"
/// Pattern: 6 digits (YYYYMM) followed by underscore or other suffix
fn is_ios_photo_folder(name: &str) -> bool {
    if name.len() < 6 {
        return false;
    }

    // Check if first 6 characters are digits (YYYYMM format)
    let first_six = &name[..6];
    if !first_six.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // Validate it looks like a reasonable date (year 2000-2099, month 01-12)
    if let Ok(year) = first_six[..4].parse::<u32>() {
        if let Ok(month) = first_six[4..6].parse::<u32>() {
            return (2000..=2099).contains(&year) && (1..=12).contains(&month);
        }
    }

    false
}

/// Check if a folder should be included for Android extraction based on config
fn is_android_included_folder(folder_name: &str, config: &AndroidConfig) -> bool {
    let name_lower = folder_name.to_lowercase();

    // Check if it's the Camera folder
    if name_lower == "camera" && config.include_camera {
        return true;
    }

    // Check if it's Screenshots folder
    if name_lower == "screenshots" && config.include_screenshots {
        return true;
    }

    // Check if it's Pictures folder
    if name_lower == "pictures" && config.include_pictures {
        return true;
    }

    // Check if it's Download folder
    if name_lower == "download" && config.include_downloads {
        return true;
    }

    // Check additional folders configured by user
    for additional in &config.additional_folders {
        // Match the first component of additional folder path
        let first_component = additional.split('/').next().unwrap_or("");
        if name_lower == first_component.to_lowercase() {
            return true;
        }
    }

    false
}

/// Check if a folder should be excluded (Android cache/system folders)
fn is_android_excluded_folder(folder_name: &str, config: &AndroidConfig) -> bool {
    if !config.exclude_cache_folders {
        return false;
    }

    let name_lower = folder_name.to_lowercase();

    // Check against configured exclude list
    for excluded in &config.exclude_folders {
        if name_lower == excluded.to_lowercase() {
            return true;
        }
    }

    // Also exclude common hidden/system folders
    if name_lower.starts_with('.') {
        return true;
    }

    false
}

/// Scan Android device for photos based on config
fn scan_android_device(
    content: &DeviceContent,
    config: &AndroidConfig,
    progress: &Option<ScanProgress>,
) -> Result<Vec<PhotoInfo>> {
    let mut photos = Vec::new();

    let root_objects = content.enumerate_objects()?;
    debug!(
        "Scanning Android device, found {} root objects",
        root_objects.len()
    );

    // Find Internal Storage or similar root
    for root in root_objects {
        if !root.is_folder {
            continue;
        }

        let root_name_upper = root.name.to_uppercase();
        let is_storage = root_name_upper.contains("STORAGE")
            || root_name_upper.contains("INTERNAL")
            || root_name_upper == "SDCARD"
            || root_name_upper == "PHONE";

        if !is_storage {
            continue;
        }

        debug!("Found storage root: '{}'", root.name);

        // Enumerate storage contents
        let storage_children = match content.enumerate_children(&root.object_id) {
            Ok(children) => children,
            Err(e) => {
                warn!("Failed to enumerate storage '{}': {}", root.name, e);
                continue;
            }
        };

        for child in storage_children {
            if !child.is_folder {
                continue;
            }

            let child_name_upper = child.name.to_uppercase();

            // Handle DCIM folder (contains Camera, Screenshots on some devices)
            if child_name_upper == "DCIM" {
                debug!("Found DCIM folder, scanning for Camera/Screenshots...");
                scan_android_dcim(content, &child, config, &mut photos, progress)?;
            }
            // Handle Pictures folder
            else if child_name_upper == "PICTURES" && config.include_pictures {
                debug!("Found Pictures folder, scanning...");
                let path = format!("{}/Pictures", root.name);
                scan_android_folder_recursive(
                    content,
                    &child,
                    &path,
                    config,
                    &mut photos,
                    progress,
                )?;
            }
            // Handle Download folder
            else if child_name_upper == "DOWNLOAD" && config.include_downloads {
                debug!("Found Download folder, scanning...");
                let path = format!("{}/Download", root.name);
                scan_android_folder_recursive(
                    content,
                    &child,
                    &path,
                    config,
                    &mut photos,
                    progress,
                )?;
            }
            // Handle additional user-configured folders
            else if is_android_included_folder(&child.name, config) {
                debug!("Found configured folder '{}', scanning...", child.name);
                let path = format!("{}/{}", root.name, child.name);
                scan_android_folder_recursive(
                    content,
                    &child,
                    &path,
                    config,
                    &mut photos,
                    progress,
                )?;
            }
        }

        // Scan app-specific folders if enabled
        if config.has_app_folders_enabled() {
            debug!("Scanning app-specific folders...");
            scan_android_app_folders(content, &root, config, &mut photos, progress)?;
        }
    }

    Ok(photos)
}

/// Scan app-specific media folders (WhatsApp, Telegram, etc.)
fn scan_android_app_folders(
    content: &DeviceContent,
    storage_root: &DeviceObject,
    config: &AndroidConfig,
    photos: &mut Vec<PhotoInfo>,
    progress: &Option<ScanProgress>,
) -> Result<()> {
    let enabled_folders = config.get_enabled_app_folders();

    if enabled_folders.is_empty() {
        return Ok(());
    }

    debug!(
        "Scanning {} app-specific folder paths",
        enabled_folders.len()
    );

    for folder_path in enabled_folders {
        debug!("Looking for app folder: {}", folder_path);

        // Try to navigate to the folder path
        if let Some(folder) = find_folder_by_path(content, storage_root, folder_path) {
            debug!("Found app folder: {}", folder_path);
            let full_path = format!("{}/{}", storage_root.name, folder_path);
            scan_android_folder_recursive(content, &folder, &full_path, config, photos, progress)?;
        } else {
            trace!("App folder not found: {}", folder_path);
        }
    }

    // Also scan user-configured additional_folders
    for folder_path in &config.additional_folders {
        debug!("Looking for custom folder: {}", folder_path);

        if let Some(folder) = find_folder_by_path(content, storage_root, folder_path) {
            debug!("Found custom folder: {}", folder_path);
            let full_path = format!("{}/{}", storage_root.name, folder_path);
            scan_android_folder_recursive(content, &folder, &full_path, config, photos, progress)?;
        } else {
            trace!("Custom folder not found: {}", folder_path);
        }
    }

    Ok(())
}

/// Find a folder by navigating a path like "WhatsApp/Media/WhatsApp Images"
fn find_folder_by_path(
    content: &DeviceContent,
    storage_root: &DeviceObject,
    path: &str,
) -> Option<DeviceObject> {
    let path_parts: Vec<&str> = path.split('/').collect();

    if path_parts.is_empty() {
        return None;
    }

    // Start from storage root
    let mut current_parent_id = storage_root.object_id.clone();
    let mut current_folder: Option<DeviceObject> = None;

    for (i, part) in path_parts.iter().enumerate() {
        let children = match content.enumerate_children(&current_parent_id) {
            Ok(c) => c,
            Err(_) => return None,
        };

        let part_upper = part.to_uppercase();
        let mut found = false;

        for child in children {
            if child.is_folder && child.name.to_uppercase() == part_upper {
                current_parent_id = child.object_id.clone();
                current_folder = Some(child);
                found = true;
                break;
            }
        }

        if !found {
            trace!(
                "Path component '{}' not found at level {} in path '{}'",
                part,
                i,
                path
            );
            return None;
        }
    }

    current_folder
}

/// Scan Android DCIM folder for Camera and Screenshots
fn scan_android_dcim(
    content: &DeviceContent,
    dcim: &DeviceObject,
    config: &AndroidConfig,
    photos: &mut Vec<PhotoInfo>,
    progress: &Option<ScanProgress>,
) -> Result<()> {
    if let Some(ref p) = progress {
        p.increment_folders();
    }

    let children = match content.enumerate_children(&dcim.object_id) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to enumerate DCIM: {}", e);
            return Ok(());
        }
    };

    for child in children {
        if !child.is_folder {
            // File directly in DCIM - check if it's media
            if is_media_file(&child.name) {
                photos.push(PhotoInfo {
                    object_id: child.object_id.clone(),
                    name: child.name.clone(),
                    path: format!("DCIM/{}", child.name),
                    size: child.size,
                    date_modified: child.date_modified.clone(),
                });
                if let Some(ref p) = progress {
                    p.add_files(1);
                }
            }
            continue;
        }

        let child_name_upper = child.name.to_uppercase();

        // Skip excluded folders
        if is_android_excluded_folder(&child.name, config) {
            debug!("Skipping excluded folder: {}", child.name);
            continue;
        }

        // Camera folder
        if child_name_upper == "CAMERA" && config.include_camera {
            debug!("Scanning DCIM/Camera...");
            let path = "DCIM/Camera".to_string();
            scan_android_folder_recursive(content, &child, &path, config, photos, progress)?;
        }
        // Screenshots folder (some devices have it under DCIM)
        else if child_name_upper == "SCREENSHOTS" && config.include_screenshots {
            debug!("Scanning DCIM/Screenshots...");
            let path = "DCIM/Screenshots".to_string();
            scan_android_folder_recursive(content, &child, &path, config, photos, progress)?;
        }
        // Other folders in DCIM (like 100ANDRO, manufacturer-specific folders)
        else if config.include_camera {
            // Include other DCIM subfolders as they often contain camera photos too
            debug!("Scanning DCIM/{}...", child.name);
            let path = format!("DCIM/{}", child.name);
            scan_android_folder_recursive(content, &child, &path, config, photos, progress)?;
        }
    }

    Ok(())
}

/// Recursively scan an Android folder for photos
fn scan_android_folder_recursive(
    content: &DeviceContent,
    folder: &DeviceObject,
    path: &str,
    config: &AndroidConfig,
    photos: &mut Vec<PhotoInfo>,
    progress: &Option<ScanProgress>,
) -> Result<()> {
    if let Some(ref p) = progress {
        p.increment_folders();
    }

    let children = match content.enumerate_children(&folder.object_id) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to enumerate '{}': {}", path, e);
            return Ok(());
        }
    };

    for child in children {
        if child.is_folder {
            // Skip excluded folders
            if is_android_excluded_folder(&child.name, config) {
                debug!("Skipping excluded folder: {}/{}", path, child.name);
                continue;
            }

            let child_path = format!("{}/{}", path, child.name);
            scan_android_folder_recursive(content, &child, &child_path, config, photos, progress)?;
        } else if is_media_file(&child.name) {
            photos.push(PhotoInfo {
                object_id: child.object_id.clone(),
                name: child.name.clone(),
                path: format!("{}/{}", path, child.name),
                size: child.size,
                date_modified: child.date_modified.clone(),
            });
            if let Some(ref p) = progress {
                p.add_files(1);
            }
        }
    }

    Ok(())
}

/// Recursively scan a folder for photos
fn scan_folder_recursive(
    content: &DeviceContent,
    parent: &DeviceObject,
    path_prefix: &str,
    photos: &mut Vec<PhotoInfo>,
    progress: &ScanProgress,
) -> Result<()> {
    debug!(
        "Scanning folder: {} (id: {})",
        path_prefix, parent.object_id
    );

    progress.increment_folders();

    let children = match content.enumerate_children(&parent.object_id) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to enumerate '{}': {}", path_prefix, e);
            return Ok(());
        }
    };

    debug!("Found {} items in '{}'", children.len(), path_prefix);

    let mut media_count = 0;
    for child in children {
        let child_path = if path_prefix.is_empty() {
            child.name.clone()
        } else {
            format!("{}/{}", path_prefix, child.name)
        };

        if child.is_folder {
            trace!("Recursing into folder: {}", child_path);
            // Recurse into subfolders
            scan_folder_recursive(content, &child, &child_path, photos, progress)?;
        } else if is_media_file(&child.name) {
            trace!("Found media file: {}", child_path);
            photos.push(PhotoInfo {
                object_id: child.object_id.clone(),
                name: child.name.clone(),
                path: child_path,
                size: child.size,
                date_modified: child.date_modified.clone(),
            });
            media_count += 1;
        }
    }

    if media_count > 0 {
        progress.add_files(media_count);
    }

    Ok(())
}

/// Check if a file is a photo or video based on extension
fn is_media_file(name: &str) -> bool {
    let extension = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    PHOTO_EXTENSIONS.contains(&extension.as_str()) || VIDEO_EXTENSIONS.contains(&extension.as_str())
}

/// Result of extracting a single photo
enum ExtractResult {
    /// Photo was extracted successfully, with the number of bytes
    Extracted(u64),
    /// Photo was skipped (already exists)
    Skipped,
    /// Photo was skipped because it's a duplicate of an existing photo
    Duplicate(PathBuf),
    /// Duplicate was overwritten
    DuplicateOverwritten(u64),
    /// Duplicate was renamed and saved
    DuplicateRenamed(u64),
}

/// Extract a single photo from the device
fn extract_single_photo(
    content: &DeviceContent,
    photo: &PhotoInfo,
    config: &ExtractionConfig,
    hash_index: &Option<DuplicateIndex>,
) -> Result<ExtractResult> {
    // Determine output path
    let output_path = if config.preserve_structure {
        config.output_dir.join(&photo.path)
    } else {
        config.output_dir.join(&photo.name)
    };

    // Check if file exists and skip if configured
    if config.skip_existing && output_path.exists() {
        // Also check file size matches
        if let Ok(metadata) = fs::metadata(&output_path) {
            if metadata.len() == photo.size || photo.size == 0 {
                debug!("Skipping existing file: {}", output_path.display());
                return Ok(ExtractResult::Skipped);
            }
        }
    }

    // Read file from device
    let data = read_file_from_device(content, &photo.object_id)?;
    let bytes = data.len() as u64;

    // Check for duplicates using SHA256 hash
    if let Some(ref index) = hash_index {
        if let Some(duplicate_path) = index.find_duplicate_with_size(&data, bytes) {
            // Determine action based on config
            let action = config
                .duplicate_detection
                .as_ref()
                .map(|d| d.duplicate_action.clone())
                .unwrap_or(DuplicateAction::Skip);

            match action {
                DuplicateAction::Skip => {
                    debug!(
                        "Skipping duplicate of {}: {}",
                        duplicate_path.display(),
                        photo.name
                    );
                    return Ok(ExtractResult::Duplicate(duplicate_path.to_path_buf()));
                }
                DuplicateAction::Overwrite => {
                    // Continue with extraction, will overwrite
                    debug!("Overwriting duplicate: {}", output_path.display());
                }
                DuplicateAction::Rename => {
                    // Generate a unique filename
                    let new_path = generate_unique_path(&output_path);
                    return extract_to_path(&new_path, &data, true, photo.date_modified.as_deref());
                }
            }
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            ExtractionError::IoError(format!(
                "Failed to create directory '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }

    // Write to output file
    let mut file = File::create(&output_path).map_err(|e| {
        ExtractionError::IoError(format!(
            "Failed to create file '{}': {}",
            output_path.display(),
            e
        ))
    })?;

    file.write_all(&data).map_err(|e| {
        ExtractionError::IoError(format!(
            "Failed to write file '{}': {}",
            output_path.display(),
            e
        ))
    })?;

    // Ensure file handle is closed before setting timestamps
    drop(file);

    // Preserve file timestamps from device metadata
    if let Some(ref date_str) = photo.date_modified {
        if let Err(e) = set_file_timestamp(&output_path, date_str) {
            debug!(
                "Could not set timestamp for '{}': {}",
                output_path.display(),
                e
            );
        }
    }

    debug!("Extracted: {} ({} bytes)", output_path.display(), bytes);

    Ok(ExtractResult::Extracted(bytes))
}

/// Set file modification timestamp from ISO 8601 date string
fn set_file_timestamp(path: &Path, date_str: &str) -> Result<()> {
    use chrono::{DateTime, Utc};
    use std::os::windows::fs::OpenOptionsExt;
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::Storage::FileSystem::{
        SetFileTime, FILE_FLAG_BACKUP_SEMANTICS, FILE_WRITE_ATTRIBUTES,
    };

    // Parse ISO 8601 date
    let datetime: DateTime<Utc> = date_str.parse().map_err(|e| {
        ExtractionError::IoError(format!("Failed to parse date '{}': {}", date_str, e))
    })?;

    // Convert to FILETIME (100-nanosecond intervals since January 1, 1601)
    let unix_secs = datetime.timestamp();
    const FILETIME_UNIX_DIFF: i64 = 116444736000000000;
    let filetime_value = (unix_secs * 10_000_000) + FILETIME_UNIX_DIFF;

    let filetime = FILETIME {
        dwLowDateTime: (filetime_value & 0xFFFFFFFF) as u32,
        dwHighDateTime: ((filetime_value >> 32) & 0xFFFFFFFF) as u32,
    };

    // Open file with write attributes permission
    let file = std::fs::OpenOptions::new()
        .write(true)
        .attributes(FILE_FLAG_BACKUP_SEMANTICS.0)
        .access_mode(FILE_WRITE_ATTRIBUTES.0)
        .open(path)
        .map_err(|e| {
            ExtractionError::IoError(format!(
                "Failed to open file for timestamp update '{}': {}",
                path.display(),
                e
            ))
        })?;

    // Set file times using Windows API
    unsafe {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::HANDLE;

        let handle = HANDLE(file.as_raw_handle() as *mut std::ffi::c_void);

        // Set both creation and modification time to the same value
        SetFileTime(handle, Some(&filetime), None, Some(&filetime)).map_err(|e| {
            ExtractionError::IoError(format!(
                "Failed to set file time for '{}': {}",
                path.display(),
                e
            ))
        })?;
    }

    trace!("Set timestamp for '{}' to {}", path.display(), date_str);
    Ok(())
}

/// Generate a unique path by adding a numeric suffix
fn generate_unique_path(original_path: &Path) -> PathBuf {
    let stem = original_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let extension = original_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let parent = original_path.parent().unwrap_or(Path::new("."));

    let mut counter = 1;
    loop {
        let new_name = if extension.is_empty() {
            format!("{}_{}", stem, counter)
        } else {
            format!("{}_{}.{}", stem, counter, extension)
        };
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
        counter += 1;
    }
}

/// Extract data to a specific path
fn extract_to_path(
    output_path: &Path,
    data: &[u8],
    is_renamed: bool,
    date_modified: Option<&str>,
) -> Result<ExtractResult> {
    let bytes = data.len() as u64;

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            ExtractionError::IoError(format!(
                "Failed to create directory '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }

    // Write to output file
    let mut file = File::create(output_path).map_err(|e| {
        ExtractionError::IoError(format!(
            "Failed to create file '{}': {}",
            output_path.display(),
            e
        ))
    })?;

    file.write_all(data).map_err(|e| {
        ExtractionError::IoError(format!(
            "Failed to write file '{}': {}",
            output_path.display(),
            e
        ))
    })?;

    // Ensure file handle is closed before setting timestamps
    drop(file);

    // Preserve file timestamps from device metadata
    if let Some(date_str) = date_modified {
        if let Err(e) = set_file_timestamp(output_path, date_str) {
            debug!(
                "Could not set timestamp for '{}': {}",
                output_path.display(),
                e
            );
        }
    }

    debug!("Extracted: {} ({} bytes)", output_path.display(), bytes);

    if is_renamed {
        Ok(ExtractResult::DuplicateRenamed(bytes))
    } else {
        Ok(ExtractResult::DuplicateOverwritten(bytes))
    }
}

/// Read a file from the device using WPD resources API
fn read_file_from_device(content: &DeviceContent, object_id: &str) -> Result<Vec<u8>> {
    unsafe {
        // Get the resources interface
        let resources = content.inner().Transfer().map_err(|e| {
            ExtractionError::ContentError(format!("Failed to get transfer interface: {}", e))
        })?;

        // Convert object ID to wide string
        let object_id_wide: Vec<u16> = object_id.encode_utf16().chain(std::iter::once(0)).collect();

        // Get the stream for reading - STGM_READ = 0
        let mut optimal_buffer_size: u32 = 0;
        let mut stream_opt: Option<IStream> = None;

        resources
            .GetStream(
                PCWSTR(object_id_wide.as_ptr()),
                &WPD_RESOURCE_DEFAULT,
                0, // STGM_READ
                &mut optimal_buffer_size,
                &mut stream_opt,
            )
            .map_err(|e| {
                ExtractionError::ContentError(format!("Failed to get file stream: {}", e))
            })?;

        let stream = stream_opt.ok_or_else(|| {
            ExtractionError::ContentError("Failed to get stream: stream is None".to_string())
        })?;

        // Use a reasonable buffer size
        let buffer_size = if optimal_buffer_size > 0 && optimal_buffer_size <= 1048576 {
            optimal_buffer_size as usize
        } else {
            262144 // 256KB default
        };

        let mut data = Vec::new();
        let mut buffer = vec![0u8; buffer_size];

        loop {
            let mut bytes_read: u32 = 0;
            let result = stream.Read(
                buffer.as_mut_ptr() as *mut _,
                buffer_size as u32,
                Some(&mut bytes_read),
            );

            if bytes_read == 0 {
                break;
            }

            data.extend_from_slice(&buffer[..bytes_read as usize]);

            // Check if we've read all data or encountered an error
            if result.is_err() || bytes_read < buffer_size as u32 {
                break;
            }
        }

        Ok(data)
    }
}

impl std::fmt::Display for ExtractionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size_mb = self.total_bytes as f64 / 1_048_576.0;
        write!(
            f,
            "Extracted: {}, Skipped: {}, Duplicates skipped: {}, Duplicates overwritten: {}, Duplicates renamed: {}, Errors: {}, Total size: {:.2} MB",
            self.files_extracted, self.files_skipped, self.duplicates_skipped,
            self.duplicates_overwritten, self.duplicates_renamed, self.errors, size_mb
        )
    }
}

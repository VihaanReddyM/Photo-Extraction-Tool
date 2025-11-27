//! Photo extraction module
//!
//! Handles the actual extraction of photos from connected iOS devices.
//! This module manages the full extraction workflow including:
//! - Device connection and content enumeration
//! - Progress tracking with visual feedback
//! - Duplicate detection integration
//! - State tracking for resume support

use crate::core::config::{DuplicateAction, DuplicateDetectionConfig, TrackingConfig};
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
        }
    }
}

/// Statistics about the extraction process
#[derive(Debug, Default)]
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

    fn increment_folders(&self) {
        self.folders_scanned.fetch_add(1, Ordering::Relaxed);
        self.update_message();
    }

    fn add_files(&self, count: usize) {
        self.files_found.fetch_add(count, Ordering::Relaxed);
        self.update_message();
    }

    fn update_message(&self) {
        let folders = self.folders_scanned.load(Ordering::Relaxed);
        let files = self.files_found.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs();
        self.spinner.set_message(format!(
            "Scanning... {} folders processed, {} media files found ({:.0}s elapsed)",
            folders, files, elapsed
        ));
    }

    fn finish(&self) {
        let folders = self.folders_scanned.load(Ordering::Relaxed);
        let files = self.files_found.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();
        self.spinner.finish_with_message(format!(
            "✓ Scan complete: {} folders processed, {} media files found in {:.1}s",
            folders,
            files,
            elapsed.as_secs_f64()
        ));
    }
}

/// File extensions to extract
const PHOTO_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "heic", "heif", "gif", "webp", "raw", "dng", "tiff", "tif", "bmp",
];

const VIDEO_EXTENSIONS: &[&str] = &["mov", "mp4", "m4v", "avi", "3gp"];

/// Extract photos from a device
pub fn extract_photos(
    device_info: &DeviceInfo,
    config: ExtractionConfig,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<ExtractionStats> {
    info!("Opening device: {}", device_info.friendly_name);

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

    info!("Output directory: {}", config.output_dir.display());

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
            info!("Building duplicate detection index...");
            let detector_config = dup_config.to_detector_config();
            match DuplicateIndex::build_from_folders(
                &detector_config,
                shutdown_flag.clone(),
                |progress| {
                    if progress.current % 100 == 0 {
                        debug!(
                            "Indexing progress: {}/{} files",
                            progress.current, progress.total
                        );
                    }
                },
            ) {
                Ok(index) => {
                    info!(
                        "Duplicate detection index built with {} entries ({} unique hashes)",
                        index.len(),
                        index.stats().unique_hashes
                    );
                    Some(index)
                }
                Err(e) => {
                    warn!("Failed to build duplicate index: {}. Continuing without duplicate detection.", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    info!("Scanning device for photos...");

    // Find all photos
    let photos = find_all_photos(&content, config.dcim_only)?;
    let total = photos.len();

    if total == 0 {
        warn!("No photos found on the device");
        return Ok(ExtractionStats::default());
    }

    info!("Found {} photos/videos to extract", total);

    // Set up progress bar
    let progress = ProgressBar::new(total as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .expect("Invalid progress template")
            .progress_chars("#>-"),
    );

    let mut stats = ExtractionStats::default();

    // Extract each photo
    for (index, photo) in photos.iter().enumerate() {
        // Check for shutdown request before processing each file
        if shutdown_flag.load(Ordering::SeqCst) {
            warn!("Shutdown requested, stopping extraction...");
            progress.finish_with_message("Extraction interrupted!");
            return Ok(stats);
        }

        progress.set_position(index as u64);
        let display_name: String = photo.name.chars().take(30).collect();
        progress.set_message(display_name);

        match extract_single_photo(&content, photo, &config, &hash_index) {
            Ok(ExtractResult::Extracted(bytes)) => {
                stats.files_extracted += 1;
                stats.total_bytes += bytes;
                if let Some(ref mut t) = tracker {
                    t.record_extracted(&photo.object_id, bytes);
                }
            }
            Ok(ExtractResult::Skipped) => {
                stats.files_skipped += 1;
                if let Some(ref mut t) = tracker {
                    t.record_skipped();
                }
            }
            Ok(ExtractResult::Duplicate(path)) => {
                debug!("Skipping duplicate of: {}", path.display());
                stats.duplicates_skipped += 1;
                if let Some(ref mut t) = tracker {
                    t.record_duplicate();
                }
            }
            Ok(ExtractResult::DuplicateOverwritten(bytes)) => {
                debug!("Overwrote duplicate: {}", photo.name);
                stats.duplicates_overwritten += 1;
                stats.total_bytes += bytes;
                if let Some(ref mut t) = tracker {
                    t.record_extracted(&photo.object_id, bytes);
                }
            }
            Ok(ExtractResult::DuplicateRenamed(bytes)) => {
                debug!("Renamed duplicate: {}", photo.name);
                stats.duplicates_renamed += 1;
                stats.total_bytes += bytes;
                if let Some(ref mut t) = tracker {
                    t.record_extracted(&photo.object_id, bytes);
                }
            }
            Err(e) => {
                debug!("Failed to extract '{}': {}", photo.name, e);
                stats.errors += 1;
                if let Some(ref mut t) = tracker {
                    t.record_error();
                }
            }
        }
    }

    progress.finish_with_message("Extraction complete!");

    // End tracking session
    let was_interrupted = shutdown_flag.load(Ordering::SeqCst);
    if let Some(ref mut t) = tracker {
        t.end_session(!was_interrupted, was_interrupted);
        if let Err(e) = t.save() {
            warn!("Failed to save tracking state: {}", e);
        }
    }

    Ok(stats)
}

/// Find all photos on the device recursively
fn find_all_photos(content: &DeviceContent, dcim_only: bool) -> Result<Vec<PhotoInfo>> {
    let mut photos = Vec::new();
    let progress = ScanProgress::new();

    let root_objects = content.enumerate_objects()?;

    info!("Found {} root objects on device", root_objects.len());

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
        warn!("  1. The iOS device is locked — please unlock it");
        warn!("  2. You haven't tapped 'Trust' on the device");
        warn!("  3. Try a different USB cable or port");
        return Ok(photos);
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
                    info!("Found {} children in '{}'", children.len(), obj.name);

                    for child in &children {
                        debug!(
                            "  Child: '{}' (id: {}, folder: {}, size: {})",
                            child.name, child.object_id, child.is_folder, child.size
                        );
                    }

                    if dcim_only {
                        // Look for DCIM folder
                        for child in children {
                            let child_name_upper = child.name.to_uppercase();
                            if child_name_upper == "DCIM" && child.is_folder {
                                info!("Found DCIM folder inside '{}', scanning...", obj.name);
                                scan_folder_recursive(
                                    content,
                                    &child,
                                    "DCIM",
                                    &mut photos,
                                    &progress,
                                )?;
                            }
                        }
                    } else {
                        // Scan everything inside
                        for child in children {
                            if child.is_folder {
                                let path = format!("{}/{}", obj.name, child.name);
                                scan_folder_recursive(
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
                                });
                                progress.add_files(1);
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
            });
        }
    }

    if photos.is_empty() && dcim_only {
        warn!("No DCIM folder found. Try running with --dcim-only false to scan all folders.");
    }

    progress.finish();
    Ok(photos)
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
                    return extract_to_path(&new_path, &data, true);
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

    debug!("Extracted: {} ({} bytes)", output_path.display(), bytes);

    Ok(ExtractResult::Extracted(bytes))
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
fn extract_to_path(output_path: &Path, data: &[u8], is_renamed: bool) -> Result<ExtractResult> {
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

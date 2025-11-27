//! Generic photo extraction module using trait-based abstraction
//!
//! This module provides extraction functionality that works with any device
//! implementing the `DeviceContentTrait`. This enables the same extraction
//! logic to work with both real iOS devices (via WPD) and mock devices for testing.
//!
//! # Example
//!
//! ```rust,no_run
//! use photo_extraction_tool::core::generic_extractor::{GenericExtractor, GenericExtractionConfig};
//! use photo_extraction_tool::testdb::{MockDeviceManager, MockFileSystem};
//! use photo_extraction_tool::device::traits::DeviceManagerTrait;
//!
//! // Create a mock device
//! let mut manager = MockDeviceManager::new();
//! let mut fs = MockFileSystem::new();
//! fs.add_standard_dcim_structure(10, 2);
//! manager.add_device(
//!     photo_extraction_tool::device::DeviceInfo::new("test", "Test iPhone", "Apple", "iPhone"),
//!     fs,
//! );
//!
//! // Open device content
//! let content = manager.open_device("test").unwrap();
//!
//! // Create extractor and run
//! let config = GenericExtractionConfig::default();
//! let mut extractor = GenericExtractor::new(config);
//! let stats = extractor.extract_from_content(&content).unwrap();
//! ```

#![allow(unused)]

use crate::core::error::{ExtractionError, Result};
use crate::device::traits::{DeviceContentTrait, DeviceInfo, DeviceObject};
use log::{debug, info, trace, warn};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for generic extraction
pub struct GenericExtractionConfig {
    /// Output directory for extracted files
    pub output_dir: PathBuf,
    /// Only extract from DCIM folder
    pub dcim_only: bool,
    /// Preserve folder structure from device
    pub preserve_structure: bool,
    /// Skip files that already exist
    pub skip_existing: bool,
    /// Write files to disk (false for dry-run/testing)
    pub write_files: bool,
    /// Maximum number of files to extract (0 = unlimited)
    pub max_files: usize,
    /// Callback for progress updates
    pub progress_callback: Option<Arc<dyn Fn(ProgressUpdate) + Send + Sync>>,
}

impl std::fmt::Debug for GenericExtractionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericExtractionConfig")
            .field("output_dir", &self.output_dir)
            .field("dcim_only", &self.dcim_only)
            .field("preserve_structure", &self.preserve_structure)
            .field("skip_existing", &self.skip_existing)
            .field("write_files", &self.write_files)
            .field("max_files", &self.max_files)
            .field(
                "progress_callback",
                &self.progress_callback.as_ref().map(|_| "<callback>"),
            )
            .finish()
    }
}

impl Clone for GenericExtractionConfig {
    fn clone(&self) -> Self {
        Self {
            output_dir: self.output_dir.clone(),
            dcim_only: self.dcim_only,
            preserve_structure: self.preserve_structure,
            skip_existing: self.skip_existing,
            write_files: self.write_files,
            max_files: self.max_files,
            progress_callback: self.progress_callback.clone(),
        }
    }
}

impl Default for GenericExtractionConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./extracted_photos"),
            dcim_only: true,
            preserve_structure: true,
            skip_existing: true,
            write_files: true,
            max_files: 0,
            progress_callback: None,
        }
    }
}

impl GenericExtractionConfig {
    /// Create a new config with the given output directory
    pub fn with_output_dir<P: AsRef<Path>>(output_dir: P) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
            ..Default::default()
        }
    }

    /// Create a config for testing (no file writes)
    pub fn for_testing() -> Self {
        Self {
            output_dir: PathBuf::from("/dev/null"),
            write_files: false,
            ..Default::default()
        }
    }

    /// Set DCIM-only mode
    pub fn dcim_only(mut self, value: bool) -> Self {
        self.dcim_only = value;
        self
    }

    /// Set preserve structure mode
    pub fn preserve_structure(mut self, value: bool) -> Self {
        self.preserve_structure = value;
        self
    }

    /// Set skip existing mode
    pub fn skip_existing(mut self, value: bool) -> Self {
        self.skip_existing = value;
        self
    }

    /// Set write files mode
    pub fn write_files(mut self, value: bool) -> Self {
        self.write_files = value;
        self
    }

    /// Set max files limit
    pub fn max_files(mut self, value: usize) -> Self {
        self.max_files = value;
        self
    }

    /// Set progress callback
    pub fn with_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(ProgressUpdate) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Arc::new(callback));
        self
    }
}

// =============================================================================
// Progress tracking
// =============================================================================

/// Progress update information
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    /// Current file being processed
    pub current_file: String,
    /// Current file index (1-based)
    pub current_index: usize,
    /// Total number of files
    pub total_files: usize,
    /// Bytes processed so far
    pub bytes_processed: u64,
    /// Current phase
    pub phase: ExtractionPhase,
}

/// Current phase of extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionPhase {
    /// Scanning device for files
    Scanning,
    /// Extracting files
    Extracting,
    /// Completed
    Complete,
}

// =============================================================================
// Extraction statistics
// =============================================================================

/// Statistics from the extraction process
#[derive(Debug, Clone, Default)]
pub struct ExtractionStats {
    /// Number of files successfully extracted
    pub files_extracted: usize,
    /// Number of files skipped (already exist)
    pub files_skipped: usize,
    /// Number of duplicates found
    pub duplicates_found: usize,
    /// Number of errors encountered
    pub errors: usize,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Number of folders traversed
    pub folders_scanned: usize,
    /// Total files found on device
    pub files_found: usize,
    /// Time taken in milliseconds
    pub duration_ms: u64,
}

impl ExtractionStats {
    /// Calculate success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.files_extracted + self.errors;
        if total == 0 {
            100.0
        } else {
            (self.files_extracted as f64 / total as f64) * 100.0
        }
    }

    /// Get bytes processed in MB
    pub fn megabytes_processed(&self) -> f64 {
        self.bytes_processed as f64 / (1024.0 * 1024.0)
    }
}

impl std::fmt::Display for ExtractionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Extracted: {}, Skipped: {}, Errors: {}, Size: {:.2} MB, Duration: {:.2}s",
            self.files_extracted,
            self.files_skipped,
            self.errors,
            self.megabytes_processed(),
            self.duration_ms as f64 / 1000.0
        )
    }
}

// =============================================================================
// File info
// =============================================================================

/// Information about a file found on the device
#[derive(Debug, Clone)]
struct FileInfo {
    /// Object ID for reading
    object_id: String,
    /// File name
    name: String,
    /// Relative path on device
    path: String,
    /// File size
    size: u64,
}

// =============================================================================
// Generic extractor
// =============================================================================

/// Generic photo extractor that works with any DeviceContentTrait implementation
pub struct GenericExtractor {
    /// Configuration
    config: GenericExtractionConfig,
    /// Shutdown flag for cancellation
    shutdown_flag: Arc<AtomicBool>,
    /// Set of already extracted file IDs (for resume support)
    extracted_ids: HashSet<String>,
}

impl GenericExtractor {
    /// Create a new extractor with the given configuration
    pub fn new(config: GenericExtractionConfig) -> Self {
        Self {
            config,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            extracted_ids: HashSet::new(),
        }
    }

    /// Create a new extractor with a shared shutdown flag
    pub fn with_shutdown_flag(config: GenericExtractionConfig, flag: Arc<AtomicBool>) -> Self {
        Self {
            config,
            shutdown_flag: flag,
            extracted_ids: HashSet::new(),
        }
    }

    /// Set extracted IDs for resume support
    pub fn set_extracted_ids(&mut self, ids: HashSet<String>) {
        self.extracted_ids = ids;
    }

    /// Request shutdown
    pub fn request_shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown was requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// Extract photos from a device content interface
    pub fn extract_from_content<C: DeviceContentTrait>(
        &mut self,
        content: &C,
    ) -> Result<ExtractionStats> {
        let start_time = Instant::now();
        let mut stats = ExtractionStats::default();

        // Create output directory if writing files
        if self.config.write_files {
            fs::create_dir_all(&self.config.output_dir).map_err(|e| {
                ExtractionError::IoError(format!(
                    "Failed to create output directory '{}': {}",
                    self.config.output_dir.display(),
                    e
                ))
            })?;
        }

        info!("Scanning device for media files...");
        self.report_progress(ProgressUpdate {
            current_file: "Scanning...".to_string(),
            current_index: 0,
            total_files: 0,
            bytes_processed: 0,
            phase: ExtractionPhase::Scanning,
        });

        // Find all media files
        let files = self.find_media_files(content, &mut stats)?;
        stats.files_found = files.len();

        if files.is_empty() {
            warn!("No media files found on device");
            stats.duration_ms = start_time.elapsed().as_millis() as u64;
            return Ok(stats);
        }

        info!("Found {} media files to extract", files.len());

        // Extract each file
        let total = if self.config.max_files > 0 {
            files.len().min(self.config.max_files)
        } else {
            files.len()
        };

        for (index, file) in files.iter().take(total).enumerate() {
            // Check for shutdown
            if self.is_shutdown_requested() {
                info!("Extraction cancelled by user");
                break;
            }

            // Check if already extracted (for resume)
            if self.extracted_ids.contains(&file.object_id) {
                stats.files_skipped += 1;
                continue;
            }

            // Report progress
            self.report_progress(ProgressUpdate {
                current_file: file.name.clone(),
                current_index: index + 1,
                total_files: total,
                bytes_processed: stats.bytes_processed,
                phase: ExtractionPhase::Extracting,
            });

            // Extract the file
            match self.extract_single_file(content, file) {
                Ok(ExtractResult::Extracted(bytes)) => {
                    stats.files_extracted += 1;
                    stats.bytes_processed += bytes;
                    self.extracted_ids.insert(file.object_id.clone());
                }
                Ok(ExtractResult::Skipped) => {
                    stats.files_skipped += 1;
                }
                Ok(ExtractResult::Duplicate) => {
                    stats.duplicates_found += 1;
                    stats.files_skipped += 1;
                }
                Err(e) => {
                    debug!("Failed to extract '{}': {}", file.name, e);
                    stats.errors += 1;
                }
            }
        }

        stats.duration_ms = start_time.elapsed().as_millis() as u64;

        // Report completion
        self.report_progress(ProgressUpdate {
            current_file: "Complete".to_string(),
            current_index: total,
            total_files: total,
            bytes_processed: stats.bytes_processed,
            phase: ExtractionPhase::Complete,
        });

        info!("Extraction complete: {}", stats);
        Ok(stats)
    }

    /// Find all media files on the device
    fn find_media_files<C: DeviceContentTrait>(
        &self,
        content: &C,
        stats: &mut ExtractionStats,
    ) -> Result<Vec<FileInfo>> {
        let mut files = Vec::new();

        // Get root objects
        let root_objects = content.enumerate_objects()?;

        if root_objects.is_empty() {
            warn!("No objects found at device root");
            return Ok(files);
        }

        // Process each root object
        for root in &root_objects {
            if !root.is_folder {
                continue;
            }

            // Get children of root (typically "Internal Storage")
            let children = content.enumerate_children(&root.object_id)?;

            for child in children {
                if self.config.dcim_only {
                    // Only scan DCIM folder
                    if child.is_folder && child.name.to_uppercase() == "DCIM" {
                        self.scan_folder_recursive(
                            content,
                            &child,
                            &format!("{}/DCIM", root.name),
                            &mut files,
                            stats,
                        )?;
                    }
                } else {
                    // Scan all folders
                    if child.is_folder {
                        let path = format!("{}/{}", root.name, child.name);
                        self.scan_folder_recursive(content, &child, &path, &mut files, stats)?;
                    } else if Self::is_media_file(&child.name) {
                        files.push(FileInfo {
                            object_id: child.object_id.clone(),
                            name: child.name.clone(),
                            path: format!("{}/{}", root.name, child.name),
                            size: child.size,
                        });
                    }
                }
            }
        }

        Ok(files)
    }

    /// Recursively scan a folder for media files
    fn scan_folder_recursive<C: DeviceContentTrait>(
        &self,
        content: &C,
        folder: &DeviceObject,
        path_prefix: &str,
        files: &mut Vec<FileInfo>,
        stats: &mut ExtractionStats,
    ) -> Result<()> {
        trace!("Scanning folder: {}", path_prefix);
        stats.folders_scanned += 1;

        // Check for shutdown
        if self.is_shutdown_requested() {
            return Ok(());
        }

        let children = match content.enumerate_children(&folder.object_id) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to enumerate '{}': {}", path_prefix, e);
                return Ok(());
            }
        };

        for child in children {
            let child_path = format!("{}/{}", path_prefix, child.name);

            if child.is_folder {
                // Recurse into subfolder
                self.scan_folder_recursive(content, &child, &child_path, files, stats)?;
            } else if Self::is_media_file(&child.name) {
                files.push(FileInfo {
                    object_id: child.object_id.clone(),
                    name: child.name.clone(),
                    path: child_path,
                    size: child.size,
                });
            }
        }

        Ok(())
    }

    /// Check if a file is a media file based on extension
    fn is_media_file(name: &str) -> bool {
        let photo_extensions = [
            "jpg", "jpeg", "png", "heic", "heif", "gif", "webp", "raw", "dng", "tiff", "tif", "bmp",
        ];
        let video_extensions = ["mov", "mp4", "m4v", "avi", "3gp"];

        let extension = Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        photo_extensions.contains(&extension.as_str())
            || video_extensions.contains(&extension.as_str())
    }

    /// Extract a single file from the device
    fn extract_single_file<C: DeviceContentTrait>(
        &self,
        content: &C,
        file: &FileInfo,
    ) -> Result<ExtractResult> {
        // Determine output path
        let output_path = if self.config.preserve_structure {
            self.config.output_dir.join(&file.path)
        } else {
            self.config.output_dir.join(&file.name)
        };

        // Check if file exists
        if self.config.skip_existing && output_path.exists() {
            if let Ok(metadata) = fs::metadata(&output_path) {
                if metadata.len() == file.size || file.size == 0 {
                    trace!("Skipping existing file: {}", output_path.display());
                    return Ok(ExtractResult::Skipped);
                }
            }
        }

        // Read file content from device
        let data = content.read_file(&file.object_id)?;
        let bytes = data.len() as u64;

        // Write to disk if configured
        if self.config.write_files {
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

            // Write file
            let mut output_file = File::create(&output_path).map_err(|e| {
                ExtractionError::IoError(format!(
                    "Failed to create file '{}': {}",
                    output_path.display(),
                    e
                ))
            })?;

            output_file.write_all(&data).map_err(|e| {
                ExtractionError::IoError(format!(
                    "Failed to write file '{}': {}",
                    output_path.display(),
                    e
                ))
            })?;

            debug!("Extracted: {} ({} bytes)", output_path.display(), bytes);
        } else {
            trace!("Dry run: would extract {} ({} bytes)", file.name, bytes);
        }

        Ok(ExtractResult::Extracted(bytes))
    }

    /// Report progress to callback if configured
    fn report_progress(&self, update: ProgressUpdate) {
        if let Some(ref callback) = self.config.progress_callback {
            callback(update);
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &GenericExtractionConfig {
        &self.config
    }

    /// Get the set of extracted file IDs
    pub fn extracted_ids(&self) -> &HashSet<String> {
        &self.extracted_ids
    }
}

/// Result of extracting a single file
#[derive(Debug, Clone)]
enum ExtractResult {
    /// File was extracted successfully
    Extracted(u64),
    /// File was skipped (already exists)
    Skipped,
    /// File was a duplicate
    Duplicate,
}

// =============================================================================
// Convenience functions
// =============================================================================

/// Extract photos from a device content interface with default configuration
pub fn extract_photos<C: DeviceContentTrait>(content: &C) -> Result<ExtractionStats> {
    let config = GenericExtractionConfig::for_testing();
    let mut extractor = GenericExtractor::new(config);
    extractor.extract_from_content(content)
}

/// Extract photos to a specific directory
pub fn extract_photos_to<C: DeviceContentTrait, P: AsRef<Path>>(
    content: &C,
    output_dir: P,
) -> Result<ExtractionStats> {
    let config = GenericExtractionConfig::with_output_dir(output_dir);
    let mut extractor = GenericExtractor::new(config);
    extractor.extract_from_content(content)
}

/// Count media files on a device (dry run)
pub fn count_media_files<C: DeviceContentTrait>(content: &C) -> Result<usize> {
    let config = GenericExtractionConfig::for_testing();
    let mut extractor = GenericExtractor::new(config);
    let stats = extractor.extract_from_content(content)?;
    Ok(stats.files_found)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::traits::DeviceManagerTrait;
    use crate::testdb::{MockDeviceInfo, MockDeviceManager, MockFileSystem};

    fn create_test_device() -> MockDeviceManager {
        let mut manager = MockDeviceManager::new();
        let device = MockDeviceInfo::new("test-device", "Test iPhone", "Apple Inc.", "iPhone 15");

        let mut fs = MockFileSystem::new();
        fs.add_standard_dcim_structure(10, 2);

        manager.add_device(device, fs);
        manager
    }

    #[test]
    fn test_config_defaults() {
        let config = GenericExtractionConfig::default();
        assert!(config.dcim_only);
        assert!(config.preserve_structure);
        assert!(config.skip_existing);
        assert!(config.write_files);
        assert_eq!(config.max_files, 0);
    }

    #[test]
    fn test_config_for_testing() {
        let config = GenericExtractionConfig::for_testing();
        assert!(!config.write_files);
    }

    #[test]
    fn test_config_builder() {
        let config = GenericExtractionConfig::with_output_dir("/tmp/test")
            .dcim_only(false)
            .preserve_structure(false)
            .max_files(100);

        assert!(!config.dcim_only);
        assert!(!config.preserve_structure);
        assert_eq!(config.max_files, 100);
    }

    #[test]
    fn test_is_media_file() {
        // Photos
        assert!(GenericExtractor::is_media_file("photo.jpg"));
        assert!(GenericExtractor::is_media_file("photo.JPG"));
        assert!(GenericExtractor::is_media_file("photo.jpeg"));
        assert!(GenericExtractor::is_media_file("photo.png"));
        assert!(GenericExtractor::is_media_file("photo.heic"));
        assert!(GenericExtractor::is_media_file("photo.HEIC"));
        assert!(GenericExtractor::is_media_file("photo.gif"));
        assert!(GenericExtractor::is_media_file("photo.raw"));
        assert!(GenericExtractor::is_media_file("photo.dng"));

        // Videos
        assert!(GenericExtractor::is_media_file("video.mov"));
        assert!(GenericExtractor::is_media_file("video.MOV"));
        assert!(GenericExtractor::is_media_file("video.mp4"));
        assert!(GenericExtractor::is_media_file("video.avi"));

        // Non-media
        assert!(!GenericExtractor::is_media_file("document.txt"));
        assert!(!GenericExtractor::is_media_file("file.pdf"));
        assert!(!GenericExtractor::is_media_file("no_extension"));
    }

    #[test]
    fn test_extraction_stats_display() {
        let stats = ExtractionStats {
            files_extracted: 100,
            files_skipped: 10,
            errors: 2,
            bytes_processed: 1024 * 1024 * 50, // 50 MB
            duration_ms: 5000,
            ..Default::default()
        };

        let display = format!("{}", stats);
        assert!(display.contains("100"));
        assert!(display.contains("50.00 MB"));
    }

    #[test]
    fn test_extraction_dry_run() {
        let manager = create_test_device();
        let content = manager.open_device("test-device").unwrap();

        let config = GenericExtractionConfig::for_testing();
        let mut extractor = GenericExtractor::new(config);
        let stats = extractor.extract_from_content(&content).unwrap();

        assert!(stats.files_found > 0);
        assert!(stats.files_extracted > 0);
        assert_eq!(stats.errors, 0);
    }

    #[test]
    fn test_extraction_with_max_files() {
        let manager = create_test_device();
        let content = manager.open_device("test-device").unwrap();

        let config = GenericExtractionConfig::for_testing().max_files(5);
        let mut extractor = GenericExtractor::new(config);
        let stats = extractor.extract_from_content(&content).unwrap();

        assert!(stats.files_extracted <= 5);
    }

    #[test]
    fn test_shutdown_request() {
        let config = GenericExtractionConfig::for_testing();
        let extractor = GenericExtractor::new(config);

        assert!(!extractor.is_shutdown_requested());
        extractor.request_shutdown();
        assert!(extractor.is_shutdown_requested());
    }

    #[test]
    fn test_progress_callback() {
        use std::sync::atomic::AtomicUsize;

        let progress_count = Arc::new(AtomicUsize::new(0));
        let progress_count_clone = progress_count.clone();

        let manager = create_test_device();
        let content = manager.open_device("test-device").unwrap();

        let config = GenericExtractionConfig::for_testing().with_progress(move |_update| {
            progress_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let mut extractor = GenericExtractor::new(config);
        let _ = extractor.extract_from_content(&content);

        // Should have received progress updates
        assert!(progress_count.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn test_convenience_functions() {
        let manager = create_test_device();
        let content = manager.open_device("test-device").unwrap();

        // Test count_media_files
        let count = count_media_files(&content).unwrap();
        assert!(count > 0);

        // Test extract_photos
        let stats = extract_photos(&content).unwrap();
        assert!(stats.files_found > 0);
    }

    #[test]
    fn test_resume_support() {
        let manager = create_test_device();
        let content = manager.open_device("test-device").unwrap();

        // First extraction
        let config = GenericExtractionConfig::for_testing();
        let mut extractor = GenericExtractor::new(config);
        let stats1 = extractor.extract_from_content(&content).unwrap();

        // Get extracted IDs
        let extracted_ids = extractor.extracted_ids().clone();
        assert!(!extracted_ids.is_empty());

        // Second extraction with resume
        let config2 = GenericExtractionConfig::for_testing();
        let mut extractor2 = GenericExtractor::new(config2);
        extractor2.set_extracted_ids(extracted_ids);
        let stats2 = extractor2.extract_from_content(&content).unwrap();

        // All files should be skipped on second run
        assert_eq!(stats2.files_skipped, stats1.files_extracted);
        assert_eq!(stats2.files_extracted, 0);
    }
}

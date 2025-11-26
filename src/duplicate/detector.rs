//! Duplicate detection module
//!
//! Provides functionality to detect duplicate photos using perceptual hashing,
//! EXIF metadata comparison, or file size matching.
//!
//! This module builds an index of existing photos from specified folders and
//! compares incoming photos against this index to avoid extracting duplicates.

use crate::core::config::{DuplicateDetectionConfig, HashAlgorithm};
use crate::core::error::{ExtractionError, Result};
use img_hash::{HasherConfig, ImageHash};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info, trace, warn};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

/// Supported image extensions for hashing
const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif"];

/// Index of photo hashes for duplicate detection
#[derive(Debug)]
pub struct PhotoHashIndex {
    /// Perceptual hashes mapped to file paths
    perceptual_hashes: Vec<(ImageHash, PathBuf)>,
    /// EXIF-based keys (date_taken + dimensions) mapped to file paths
    exif_keys: HashMap<String, PathBuf>,
    /// File sizes mapped to file paths (for size-based matching)
    size_map: HashMap<u64, Vec<PathBuf>>,
    /// Configuration
    config: DuplicateDetectionConfig,
    /// Hash size used
    hash_size: u32,
}

impl PhotoHashIndex {
    /// Create a new empty index
    pub fn new(config: &DuplicateDetectionConfig) -> Self {
        Self {
            perceptual_hashes: Vec::new(),
            exif_keys: HashMap::new(),
            size_map: HashMap::new(),
            config: config.clone(),
            hash_size: config.hash_size,
        }
    }

    /// Build index from comparison folders
    pub fn build_from_folders(
        config: &DuplicateDetectionConfig,
        shutdown_flag: Arc<AtomicBool>,
    ) -> Result<Self> {
        let mut index = Self::new(config);

        // Try to load from cache first
        if config.cache_index && config.cache_file.exists() {
            match index.load_cache(&config.cache_file) {
                Ok(()) => {
                    info!("Loaded {} hashes from cache", index.perceptual_hashes.len());
                    return Ok(index);
                }
                Err(e) => {
                    warn!("Failed to load cache, rebuilding index: {}", e);
                }
            }
        }

        info!(
            "Building duplicate detection index from {} folder(s)...",
            config.comparison_folders.len()
        );

        // Store hash_size for use in parallel closures
        let hash_size = config.hash_size;

        // First pass: count total files to process
        info!("Counting files to index...");
        let mut file_list: Vec<PathBuf> = Vec::new();

        for folder in &config.comparison_folders {
            if !folder.exists() {
                warn!("Comparison folder does not exist: {}", folder.display());
                continue;
            }

            for entry in WalkDir::new(folder)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                // Check extension
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_default();

                if SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                    file_list.push(path.to_path_buf());
                }
            }
        }

        let total_files = file_list.len();
        info!("Found {} image files to index", total_files);

        // Set up progress bar
        let progress = ProgressBar::new(total_files as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                .expect("Invalid progress template")
                .progress_chars("#>-"),
        );
        progress.set_message("Building hash index (multi-threaded)...");

        // Build size map first (single-threaded, fast)
        for path in &file_list {
            if let Ok(metadata) = fs::metadata(path) {
                let size = metadata.len();
                index.size_map.entry(size).or_default().push(path.clone());
            }
        }

        // Use atomic counters for thread-safe progress tracking
        let hashed_files = AtomicUsize::new(0);
        let errors = AtomicUsize::new(0);
        let processed = AtomicUsize::new(0);

        // Compute hashes in parallel using rayon
        if config.hash_algorithm == HashAlgorithm::Perceptual {
            info!(
                "Computing perceptual hashes using {} threads...",
                rayon::current_num_threads()
            );

            // Collect results in parallel - create hasher per thread
            let hash_results: Vec<Option<(ImageHash, PathBuf)>> = file_list
                .par_iter()
                .map(|path| {
                    // Check for shutdown
                    if shutdown_flag.load(Ordering::Relaxed) {
                        return None;
                    }

                    // Update progress
                    let current = processed.fetch_add(1, Ordering::Relaxed);
                    if current.is_multiple_of(10) {
                        progress.set_position(current as u64);
                    }

                    // Create hasher for this thread
                    let hasher = HasherConfig::new()
                        .hash_size(hash_size, hash_size)
                        .to_hasher();

                    // Compute hash
                    match load_image_for_hash(path) {
                        Ok(img) => {
                            let hash = hasher.hash_image(&img);
                            trace!("Hashed: {} -> {}", path.display(), hash.to_base64());
                            hashed_files.fetch_add(1, Ordering::Relaxed);
                            Some((hash, path.clone()))
                        }
                        Err(e) => {
                            trace!("Failed to hash {}: {}", path.display(), e);
                            errors.fetch_add(1, Ordering::Relaxed);
                            None
                        }
                    }
                })
                .collect();

            // Collect successful hashes
            for result in hash_results.into_iter().flatten() {
                index.perceptual_hashes.push(result);
            }
        } else if config.hash_algorithm == HashAlgorithm::Exif {
            // EXIF extraction in parallel
            info!(
                "Extracting EXIF data using {} threads...",
                rayon::current_num_threads()
            );

            let exif_results: Vec<Option<(String, PathBuf)>> = file_list
                .par_iter()
                .map(|path| {
                    if shutdown_flag.load(Ordering::Relaxed) {
                        return None;
                    }

                    let current = processed.fetch_add(1, Ordering::Relaxed);
                    if current.is_multiple_of(10) {
                        progress.set_position(current as u64);
                    }

                    if let Some(key) = extract_exif_key(path) {
                        hashed_files.fetch_add(1, Ordering::Relaxed);
                        Some((key, path.clone()))
                    } else {
                        errors.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                })
                .collect();

            for result in exif_results.into_iter().flatten() {
                index.exif_keys.insert(result.0, result.1);
            }
        }

        progress.set_position(total_files as u64);

        if shutdown_flag.load(Ordering::SeqCst) {
            progress.finish_with_message("Index build interrupted!");
        } else {
            progress.finish_with_message("Index complete!");
        }

        let hashed_files = hashed_files.load(Ordering::Relaxed);
        let errors = errors.load(Ordering::Relaxed);

        info!(
            "Index built: {} files processed, {} hashed, {} errors",
            total_files, hashed_files, errors
        );

        // Save cache
        if config.cache_index {
            if let Err(e) = index.save_cache(&config.cache_file) {
                warn!("Failed to save cache: {}", e);
            } else {
                info!("Cache saved to: {}", config.cache_file.display());
            }
        }

        Ok(index)
    }

    /// Check if a photo (from device, in memory) is a duplicate
    pub fn is_duplicate(&self, image_data: &[u8]) -> Option<PathBuf> {
        match self.config.hash_algorithm {
            HashAlgorithm::Perceptual => self.check_perceptual_hash(image_data),
            HashAlgorithm::Exif => self.check_exif(image_data),
            HashAlgorithm::Size => self.check_size(image_data.len() as u64),
        }
    }

    /// Check using perceptual hash
    fn check_perceptual_hash(&self, image_data: &[u8]) -> Option<PathBuf> {
        // Try to decode the image
        let img = match load_image_from_memory(image_data) {
            Ok(img) => img,
            Err(e) => {
                trace!("Failed to decode image for hashing: {}", e);
                return None;
            }
        };

        let hasher = HasherConfig::new()
            .hash_size(self.hash_size, self.hash_size)
            .to_hasher();

        let hash = hasher.hash_image(&img);
        trace!("Image hash: {}", hash.to_base64());

        // Find closest match
        for (existing_hash, path) in &self.perceptual_hashes {
            let distance = hash.dist(existing_hash);
            if distance <= self.config.similarity_threshold {
                debug!(
                    "Found duplicate (distance: {}): {}",
                    distance,
                    path.display()
                );
                return Some(path.clone());
            }
        }

        None
    }

    /// Check using EXIF metadata
    fn check_exif(&self, image_data: &[u8]) -> Option<PathBuf> {
        let key = extract_exif_key_from_bytes(image_data)?;
        self.exif_keys.get(&key).cloned()
    }

    /// Check using file size
    fn check_size(&self, size: u64) -> Option<PathBuf> {
        self.size_map
            .get(&size)
            .and_then(|paths| paths.first().cloned())
    }

    /// Save index to cache file
    fn save_cache(&self, path: &Path) -> Result<()> {
        let file = File::create(path)
            .map_err(|e| ExtractionError::IoError(format!("Failed to create cache file: {}", e)))?;
        let mut writer = BufWriter::new(file);

        // Write version
        writer
            .write_all(&[1u8])
            .map_err(|e| ExtractionError::IoError(format!("Failed to write cache: {}", e)))?;

        // Write hash size
        writer
            .write_all(&self.hash_size.to_le_bytes())
            .map_err(|e| ExtractionError::IoError(format!("Failed to write cache: {}", e)))?;

        // Write number of entries
        let count = self.perceptual_hashes.len() as u32;
        writer
            .write_all(&count.to_le_bytes())
            .map_err(|e| ExtractionError::IoError(format!("Failed to write cache: {}", e)))?;

        // Write each entry
        for (hash, path) in &self.perceptual_hashes {
            // Write hash bytes
            let hash_bytes = hash.as_bytes();
            let hash_len = hash_bytes.len() as u32;
            writer
                .write_all(&hash_len.to_le_bytes())
                .map_err(|e| ExtractionError::IoError(format!("Failed to write cache: {}", e)))?;
            writer
                .write_all(hash_bytes)
                .map_err(|e| ExtractionError::IoError(format!("Failed to write cache: {}", e)))?;

            // Write path
            let path_str = path.to_string_lossy();
            let path_bytes = path_str.as_bytes();
            let path_len = path_bytes.len() as u32;
            writer
                .write_all(&path_len.to_le_bytes())
                .map_err(|e| ExtractionError::IoError(format!("Failed to write cache: {}", e)))?;
            writer
                .write_all(path_bytes)
                .map_err(|e| ExtractionError::IoError(format!("Failed to write cache: {}", e)))?;
        }

        writer
            .flush()
            .map_err(|e| ExtractionError::IoError(format!("Failed to flush cache: {}", e)))?;

        Ok(())
    }

    /// Load index from cache file
    fn load_cache(&mut self, path: &Path) -> Result<()> {
        let file = File::open(path)
            .map_err(|e| ExtractionError::IoError(format!("Failed to open cache file: {}", e)))?;
        let mut reader = BufReader::new(file);

        // Read version
        let mut version = [0u8; 1];
        reader
            .read_exact(&mut version)
            .map_err(|e| ExtractionError::IoError(format!("Failed to read cache: {}", e)))?;

        if version[0] != 1 {
            return Err(ExtractionError::IoError(
                "Invalid cache version".to_string(),
            ));
        }

        // Read hash size
        let mut hash_size_bytes = [0u8; 4];
        reader
            .read_exact(&mut hash_size_bytes)
            .map_err(|e| ExtractionError::IoError(format!("Failed to read cache: {}", e)))?;
        let cached_hash_size = u32::from_le_bytes(hash_size_bytes);

        if cached_hash_size != self.hash_size {
            return Err(ExtractionError::IoError(format!(
                "Cache hash size mismatch: {} vs {}",
                cached_hash_size, self.hash_size
            )));
        }

        // Read number of entries
        let mut count_bytes = [0u8; 4];
        reader
            .read_exact(&mut count_bytes)
            .map_err(|e| ExtractionError::IoError(format!("Failed to read cache: {}", e)))?;
        let count = u32::from_le_bytes(count_bytes);

        // Read each entry
        for _ in 0..count {
            // Read hash
            let mut hash_len_bytes = [0u8; 4];
            reader
                .read_exact(&mut hash_len_bytes)
                .map_err(|e| ExtractionError::IoError(format!("Failed to read cache: {}", e)))?;
            let hash_len = u32::from_le_bytes(hash_len_bytes) as usize;

            let mut hash_bytes = vec![0u8; hash_len];
            reader
                .read_exact(&mut hash_bytes)
                .map_err(|e| ExtractionError::IoError(format!("Failed to read cache: {}", e)))?;

            // Read path
            let mut path_len_bytes = [0u8; 4];
            reader
                .read_exact(&mut path_len_bytes)
                .map_err(|e| ExtractionError::IoError(format!("Failed to read cache: {}", e)))?;
            let path_len = u32::from_le_bytes(path_len_bytes) as usize;

            let mut path_bytes = vec![0u8; path_len];
            reader
                .read_exact(&mut path_bytes)
                .map_err(|e| ExtractionError::IoError(format!("Failed to read cache: {}", e)))?;

            let path_str = String::from_utf8_lossy(&path_bytes);
            let path = PathBuf::from(path_str.as_ref());

            // Reconstruct hash
            if let Ok(hash) = ImageHash::from_bytes(&hash_bytes) {
                self.perceptual_hashes.push((hash, path));
            }
        }

        Ok(())
    }

    /// Get the number of indexed photos
    pub fn len(&self) -> usize {
        match self.config.hash_algorithm {
            HashAlgorithm::Perceptual => self.perceptual_hashes.len(),
            HashAlgorithm::Exif => self.exif_keys.len(),
            HashAlgorithm::Size => self.size_map.values().map(|v| v.len()).sum(),
        }
    }

    /// Check if the index contains no entries
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Extract EXIF key from file path
fn extract_exif_key(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut bufreader = BufReader::new(&file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader).ok()?;

    extract_exif_key_from_exif(&exif)
}

/// Extract EXIF key from raw image bytes
fn extract_exif_key_from_bytes(data: &[u8]) -> Option<String> {
    let mut cursor = std::io::Cursor::new(data);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut cursor).ok()?;

    extract_exif_key_from_exif(&exif)
}

/// Load an image from a file path for hashing
/// Uses the image crate to load, then converts to img_hash's expected format
fn load_image_for_hash(path: &Path) -> std::result::Result<img_hash::image::DynamicImage, String> {
    // Load using the image crate (which has jpeg support)
    let img = image::open(path).map_err(|e| e.to_string())?;

    // Convert to RGB8 and then to img_hash's DynamicImage format
    let rgb_img = img.to_rgb8();
    let (width, height) = rgb_img.dimensions();

    // Create an img_hash compatible image
    let img_hash_img = img_hash::image::RgbImage::from_raw(width, height, rgb_img.into_raw())
        .ok_or_else(|| "Failed to create image buffer".to_string())?;

    Ok(img_hash::image::DynamicImage::ImageRgb8(img_hash_img))
}

/// Load an image from memory for hashing
/// Uses the image crate to load, then converts to img_hash's expected format
fn load_image_from_memory(
    data: &[u8],
) -> std::result::Result<img_hash::image::DynamicImage, String> {
    // Load using the image crate (which has jpeg support)
    let img = image::load_from_memory(data).map_err(|e| e.to_string())?;

    // Convert to RGB8 and then to img_hash's DynamicImage format
    let rgb_img = img.to_rgb8();
    let (width, height) = rgb_img.dimensions();

    // Create an img_hash compatible image
    let img_hash_img = img_hash::image::RgbImage::from_raw(width, height, rgb_img.into_raw())
        .ok_or_else(|| "Failed to create image buffer".to_string())?;

    Ok(img_hash::image::DynamicImage::ImageRgb8(img_hash_img))
}

/// Extract EXIF key from parsed EXIF data
fn extract_exif_key_from_exif(exif: &exif::Exif) -> Option<String> {
    // Get date/time original
    let datetime = exif
        .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
        .map(|f| f.display_value().to_string())
        .unwrap_or_default();

    // Get dimensions
    let width = exif
        .get_field(exif::Tag::PixelXDimension, exif::In::PRIMARY)
        .or_else(|| exif.get_field(exif::Tag::ImageWidth, exif::In::PRIMARY))
        .map(|f| f.display_value().to_string())
        .unwrap_or_default();

    let height = exif
        .get_field(exif::Tag::PixelYDimension, exif::In::PRIMARY)
        .or_else(|| exif.get_field(exif::Tag::ImageLength, exif::In::PRIMARY))
        .map(|f| f.display_value().to_string())
        .unwrap_or_default();

    // Get camera make/model
    let make = exif
        .get_field(exif::Tag::Make, exif::In::PRIMARY)
        .map(|f| f.display_value().to_string())
        .unwrap_or_default();

    if datetime.is_empty() && width.is_empty() {
        return None;
    }

    Some(format!("{}|{}x{}|{}", datetime, width, height, make))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_creation() {
        let config = DuplicateDetectionConfig::default();
        let index = PhotoHashIndex::new(&config);
        assert!(index.is_empty());
    }
}

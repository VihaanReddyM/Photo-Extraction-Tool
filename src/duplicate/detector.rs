//! Duplicate Detection Module
//!
//! Provides efficient duplicate file detection using SHA256 cryptographic hashing.
//! This module is designed for exact-match duplicate detection, which is reliable,
//! fast, and works for all file types (photos, videos, etc.).
//!
//! # Architecture
//!
//! The module uses a two-tier lookup strategy for efficiency:
//! 1. **Size Index**: Files are first grouped by size (O(1) lookup)
//! 2. **Hash Index**: Only files with matching sizes are hash-compared
//!
//! This avoids computing expensive SHA256 hashes for files that can't possibly
//! be duplicates (different sizes).
//!
//! # Example
//!
//! ```rust,no_run
//! use photo_extraction_tool::duplicate::detector::{DuplicateIndex, DuplicateConfig};
//! use std::path::PathBuf;
//! use std::sync::Arc;
//! use std::sync::atomic::AtomicBool;
//!
//! // Create configuration
//! let config = DuplicateConfig::new()
//!     .with_folder(PathBuf::from("D:/Photos"))
//!     .with_cache_file(PathBuf::from("./duplicate_cache.json"));
//!
//! // Build index from folders
//! let shutdown = Arc::new(AtomicBool::new(false));
//! let index = DuplicateIndex::build_from_folders(&config, shutdown, |p| {
//!     println!("Progress: {}/{}", p.current, p.total);
//! }).unwrap();
//!
//! // Check if data is a duplicate
//! let file_data = std::fs::read("some_file.jpg").unwrap();
//! if let Some(original) = index.find_duplicate(&file_data) {
//!     println!("Duplicate of: {}", original.display());
//! }
//! ```

use crate::core::error::{ExtractionError, Result};
use log::{info, trace, warn};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use walkdir::WalkDir;

/// Buffer size for streaming hash computation (64KB)
const HASH_BUFFER_SIZE: usize = 64 * 1024;

/// Supported media file extensions
const MEDIA_EXTENSIONS: &[&str] = &[
    // Photos
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif", "heic", "heif", "raw", "cr2", "nef",
    "arw", "dng", // Videos
    "mp4", "mov", "avi", "mkv", "m4v", "3gp", "wmv", "flv", "webm",
];

/// SHA256 hash represented as a fixed-size array
pub type Sha256Hash = [u8; 32];

/// Progress information for index building
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct IndexProgress {
    /// Current file being processed
    pub current: usize,
    /// Total files to process
    pub total: usize,
    /// Current file path (if available)
    pub current_file: Option<PathBuf>,
    /// Number of files successfully hashed
    pub hashed: usize,
    /// Number of errors encountered
    pub errors: usize,
}

/// Configuration for duplicate detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateConfig {
    /// Folders to scan for existing files
    pub comparison_folders: Vec<PathBuf>,

    /// Whether to cache the index to disk
    pub cache_enabled: bool,

    /// Path to the cache file
    pub cache_file: PathBuf,

    /// Whether to include subdirectories when scanning
    pub recursive: bool,

    /// Whether to follow symbolic links
    pub follow_symlinks: bool,

    /// Minimum file size to consider (0 = no minimum)
    pub min_file_size: u64,

    /// Maximum file size to consider (0 = no maximum)
    pub max_file_size: u64,

    /// Only index media files (photos/videos)
    pub media_only: bool,
}

impl Default for DuplicateConfig {
    fn default() -> Self {
        Self {
            comparison_folders: Vec::new(),
            cache_enabled: true,
            cache_file: PathBuf::from("./.duplicate_cache.json"),
            recursive: true,
            follow_symlinks: false,
            min_file_size: 0,
            max_file_size: 0,
            media_only: true,
        }
    }
}

#[allow(dead_code)]
impl DuplicateConfig {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a folder to compare against
    pub fn with_folder(mut self, folder: PathBuf) -> Self {
        self.comparison_folders.push(folder);
        self
    }

    /// Add multiple folders to compare against
    pub fn with_folders(mut self, folders: Vec<PathBuf>) -> Self {
        self.comparison_folders.extend(folders);
        self
    }

    /// Set the cache file path
    pub fn with_cache_file(mut self, path: PathBuf) -> Self {
        self.cache_file = path;
        self
    }

    /// Enable or disable caching
    pub fn with_cache(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    /// Set whether to scan recursively
    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Set minimum file size
    pub fn with_min_size(mut self, size: u64) -> Self {
        self.min_file_size = size;
        self
    }

    /// Set maximum file size
    pub fn with_max_size(mut self, size: u64) -> Self {
        self.max_file_size = size;
        self
    }

    /// Set whether to only index media files
    pub fn with_media_only(mut self, media_only: bool) -> Self {
        self.media_only = media_only;
        self
    }
}

/// Entry in the duplicate index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// File path
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// SHA256 hash of the file
    #[serde(with = "hex_hash")]
    pub hash: Sha256Hash,
    /// When this entry was indexed
    #[serde(default = "default_indexed_at")]
    pub indexed_at: u64,
}

fn default_indexed_at() -> u64 {
    0
}

/// Serde helper for hex encoding/decoding hashes
mod hex_hash {
    use super::Sha256Hash;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(hash: &Sha256Hash, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        serializer.serialize_str(&hex_string)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Sha256Hash, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut hash = [0u8; 32];
        if s.len() != 64 {
            return Err(serde::de::Error::custom("Invalid hash length"));
        }
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let hex_str = std::str::from_utf8(chunk).map_err(serde::de::Error::custom)?;
            hash[i] = u8::from_str_radix(hex_str, 16).map_err(serde::de::Error::custom)?;
        }
        Ok(hash)
    }
}

/// Statistics about the duplicate index
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    /// Total number of files indexed
    pub total_files: usize,
    /// Total size of all indexed files
    pub total_bytes: u64,
    /// Number of unique hashes (some files may be duplicates)
    pub unique_hashes: usize,
    /// Number of duplicate groups found within the index
    pub duplicate_groups: usize,
    /// Number of files that are duplicates of others in the index
    pub duplicate_files: usize,
    /// Number of errors encountered during indexing
    pub errors: usize,
    /// Time taken to build the index (in milliseconds)
    pub build_time_ms: u64,
}

/// A group of duplicate files
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DuplicateGroup {
    /// The shared hash
    pub hash: Sha256Hash,
    /// File size (all files in group have same size)
    pub size: u64,
    /// Paths of all duplicate files
    pub paths: Vec<PathBuf>,
}

/// Cache format for persisting the index
#[derive(Debug, Serialize, Deserialize)]
struct IndexCache {
    /// Version of the cache format
    version: u32,
    /// When the cache was created
    created_at: u64,
    /// Configuration used to build this cache
    config_hash: String,
    /// All indexed entries
    entries: Vec<IndexEntry>,
}

impl IndexCache {
    const CURRENT_VERSION: u32 = 1;
}

/// The main duplicate detection index
#[derive(Debug)]
pub struct DuplicateIndex {
    /// Size -> list of entries with that size
    /// This allows O(1) size-based filtering before computing hashes
    size_index: HashMap<u64, Vec<usize>>,

    /// Hash -> list of entry indices with that hash
    hash_index: HashMap<Sha256Hash, Vec<usize>>,

    /// All entries in the index
    entries: Vec<IndexEntry>,

    /// Statistics
    stats: IndexStats,

    /// Configuration used
    config: DuplicateConfig,
}

impl DuplicateIndex {
    /// Create a new empty index
    pub fn new(config: DuplicateConfig) -> Self {
        Self {
            size_index: HashMap::new(),
            hash_index: HashMap::new(),
            entries: Vec::new(),
            stats: IndexStats::default(),
            config,
        }
    }

    /// Build an index from the configured comparison folders
    ///
    /// # Arguments
    /// * `config` - Configuration specifying folders to scan
    /// * `shutdown_flag` - Flag to signal early termination
    /// * `progress_callback` - Called periodically with progress updates
    pub fn build_from_folders<F>(
        config: &DuplicateConfig,
        shutdown_flag: Arc<AtomicBool>,
        progress_callback: F,
    ) -> Result<Self>
    where
        F: Fn(IndexProgress) + Send + Sync,
    {
        let start_time = std::time::Instant::now();
        let mut index = Self::new(config.clone());

        // Try to load from cache first
        if config.cache_enabled && config.cache_file.exists() {
            match index.load_cache(&config.cache_file) {
                Ok(()) => {
                    info!(
                        "Loaded {} entries from cache: {}",
                        index.entries.len(),
                        config.cache_file.display()
                    );
                    index.stats.build_time_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(index);
                }
                Err(e) => {
                    warn!("Failed to load cache, rebuilding index: {}", e);
                }
            }
        }

        // Collect all files to index
        info!(
            "Scanning {} folder(s) for files...",
            config.comparison_folders.len()
        );

        let file_list = index.collect_files(&config.comparison_folders)?;
        let total_files = file_list.len();

        if total_files == 0 {
            info!("No files found to index");
            return Ok(index);
        }

        info!("Found {} files to index", total_files);

        // Progress tracking
        let processed = AtomicUsize::new(0);
        let hashed = AtomicUsize::new(0);
        let errors = AtomicUsize::new(0);

        // Parallel hash computation
        let entries: Vec<Option<IndexEntry>> = file_list
            .par_iter()
            .map(|path| {
                // Check for shutdown
                if shutdown_flag.load(Ordering::Relaxed) {
                    return None;
                }

                let current = processed.fetch_add(1, Ordering::Relaxed);

                // Report progress periodically
                if current % 100 == 0 || current == total_files - 1 {
                    progress_callback(IndexProgress {
                        current,
                        total: total_files,
                        current_file: Some(path.clone()),
                        hashed: hashed.load(Ordering::Relaxed),
                        errors: errors.load(Ordering::Relaxed),
                    });
                }

                // Get file metadata
                let metadata = match fs::metadata(path) {
                    Ok(m) => m,
                    Err(e) => {
                        trace!("Failed to read metadata for {}: {}", path.display(), e);
                        errors.fetch_add(1, Ordering::Relaxed);
                        return None;
                    }
                };

                let size = metadata.len();

                // Apply size filters
                if config.min_file_size > 0 && size < config.min_file_size {
                    return None;
                }
                if config.max_file_size > 0 && size > config.max_file_size {
                    return None;
                }

                // Compute hash
                match compute_file_hash(path) {
                    Ok(hash) => {
                        hashed.fetch_add(1, Ordering::Relaxed);
                        let indexed_at = SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);

                        Some(IndexEntry {
                            path: path.clone(),
                            size,
                            hash,
                            indexed_at,
                        })
                    }
                    Err(e) => {
                        trace!("Failed to hash {}: {}", path.display(), e);
                        errors.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                }
            })
            .collect();

        // Check if we were interrupted
        if shutdown_flag.load(Ordering::SeqCst) {
            info!("Index building interrupted");
            return Ok(index);
        }

        // Build the index from successful entries
        for entry in entries.into_iter().flatten() {
            index.add_entry(entry);
        }

        // Update statistics
        index.stats.errors = errors.load(Ordering::Relaxed);
        index.stats.build_time_ms = start_time.elapsed().as_millis() as u64;
        index.compute_stats();

        info!(
            "Index built: {} files, {} unique hashes, {} duplicate groups, {} errors in {}ms",
            index.stats.total_files,
            index.stats.unique_hashes,
            index.stats.duplicate_groups,
            index.stats.errors,
            index.stats.build_time_ms
        );

        // Save cache
        if config.cache_enabled {
            if let Err(e) = index.save_cache(&config.cache_file) {
                warn!("Failed to save cache: {}", e);
            } else {
                info!("Cache saved to: {}", config.cache_file.display());
            }
        }

        Ok(index)
    }

    /// Collect all files from the given folders
    fn collect_files(&self, folders: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for folder in folders {
            if !folder.exists() {
                warn!("Folder does not exist: {}", folder.display());
                continue;
            }

            let walker = WalkDir::new(folder)
                .follow_links(self.config.follow_symlinks)
                .max_depth(if self.config.recursive { usize::MAX } else { 1 });

            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                // Check extension if media_only is enabled
                if self.config.media_only {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_lowercase())
                        .unwrap_or_default();

                    if !MEDIA_EXTENSIONS.contains(&ext.as_str()) {
                        continue;
                    }
                }

                files.push(path.to_path_buf());
            }
        }

        Ok(files)
    }

    /// Add an entry to the index
    fn add_entry(&mut self, entry: IndexEntry) {
        let idx = self.entries.len();
        let size = entry.size;
        let hash = entry.hash;

        self.entries.push(entry);

        // Update size index
        self.size_index.entry(size).or_default().push(idx);

        // Update hash index
        self.hash_index.entry(hash).or_default().push(idx);
    }

    /// Compute statistics about the index
    fn compute_stats(&mut self) {
        self.stats.total_files = self.entries.len();
        self.stats.total_bytes = self.entries.iter().map(|e| e.size).sum();
        self.stats.unique_hashes = self.hash_index.len();

        // Count duplicate groups (hashes with more than one file)
        let mut dup_groups = 0;
        let mut dup_files = 0;
        for indices in self.hash_index.values() {
            if indices.len() > 1 {
                dup_groups += 1;
                dup_files += indices.len() - 1; // All but one are duplicates
            }
        }
        self.stats.duplicate_groups = dup_groups;
        self.stats.duplicate_files = dup_files;
    }

    /// Check if the given data is a duplicate of an indexed file
    ///
    /// Uses size pre-filtering for efficiency: if no indexed file has
    /// the same size, we skip hash computation entirely.
    ///
    /// # Arguments
    /// * `data` - The file data to check
    ///
    /// # Returns
    /// * `Some(path)` - Path to the original file if this is a duplicate
    /// * `None` - If this is not a duplicate
    #[allow(dead_code)]
    pub fn find_duplicate(&self, data: &[u8]) -> Option<&Path> {
        let size = data.len() as u64;

        // First check: do we have any files with this size?
        let size_matches = self.size_index.get(&size)?;

        if size_matches.is_empty() {
            return None;
        }

        // Compute hash of the input data
        let hash = compute_data_hash(data);

        // Look up in hash index
        self.find_duplicate_by_hash(&hash)
    }

    /// Check if data with a known size is a duplicate
    ///
    /// This is slightly more efficient than `find_duplicate` when you
    /// already know the size (avoids computing data.len()).
    pub fn find_duplicate_with_size(&self, data: &[u8], size: u64) -> Option<&Path> {
        // First check: do we have any files with this size?
        let size_matches = self.size_index.get(&size)?;

        if size_matches.is_empty() {
            return None;
        }

        // Compute hash of the input data
        let hash = compute_data_hash(data);

        // Look up in hash index
        self.find_duplicate_by_hash(&hash)
    }

    /// Find a duplicate by its pre-computed hash
    pub fn find_duplicate_by_hash(&self, hash: &Sha256Hash) -> Option<&Path> {
        let indices = self.hash_index.get(hash)?;
        let first_idx = indices.first()?;
        Some(&self.entries[*first_idx].path)
    }

    /// Check if a file on disk is a duplicate
    #[allow(dead_code)]
    pub fn is_file_duplicate(&self, path: &Path) -> Result<Option<&Path>> {
        // Get file size first for pre-filtering
        let metadata = fs::metadata(path).map_err(|e| {
            ExtractionError::IoError(format!("Failed to read file metadata: {}", e))
        })?;

        let size = metadata.len();

        // Check if any indexed file has this size
        if !self.size_index.contains_key(&size) {
            return Ok(None);
        }

        // Compute hash and look up
        let hash = compute_file_hash(path)?;
        Ok(self.find_duplicate_by_hash(&hash))
    }

    /// Find all duplicate groups within the index itself
    #[allow(dead_code)]
    pub fn find_internal_duplicates(&self) -> Vec<DuplicateGroup> {
        let mut groups = Vec::new();

        for (hash, indices) in &self.hash_index {
            if indices.len() > 1 {
                let paths: Vec<PathBuf> = indices
                    .iter()
                    .map(|&idx| self.entries[idx].path.clone())
                    .collect();

                let size = self.entries[indices[0]].size;

                groups.push(DuplicateGroup {
                    hash: *hash,
                    size,
                    paths,
                });
            }
        }

        groups
    }

    /// Get all entries in the index
    #[allow(dead_code)]
    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    /// Get statistics about the index
    pub fn stats(&self) -> &IndexStats {
        &self.stats
    }

    /// Get the number of indexed files
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove entries for files that no longer exist on disk
    #[allow(dead_code)]
    pub fn prune_missing(&mut self) -> usize {
        let original_count = self.entries.len();

        // Find indices of entries that still exist
        let existing: Vec<IndexEntry> =
            self.entries.drain(..).filter(|e| e.path.exists()).collect();

        // Rebuild indices
        self.size_index.clear();
        self.hash_index.clear();

        for entry in existing {
            self.add_entry(entry);
        }

        self.compute_stats();

        original_count - self.entries.len()
    }

    /// Merge another index into this one
    #[allow(dead_code)]
    pub fn merge(&mut self, other: DuplicateIndex) {
        for entry in other.entries {
            // Avoid adding duplicates
            if !self.hash_index.contains_key(&entry.hash) {
                self.add_entry(entry);
            }
        }
        self.compute_stats();
    }

    /// Save the index to a cache file
    pub fn save_cache(&self, path: &Path) -> Result<()> {
        let cache = IndexCache {
            version: IndexCache::CURRENT_VERSION,
            created_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            config_hash: self.compute_config_hash(),
            entries: self.entries.clone(),
        };

        let json = serde_json::to_string_pretty(&cache)
            .map_err(|e| ExtractionError::IoError(format!("Failed to serialize cache: {}", e)))?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ExtractionError::IoError(format!("Failed to create cache directory: {}", e))
            })?;
        }

        fs::write(path, json)
            .map_err(|e| ExtractionError::IoError(format!("Failed to write cache file: {}", e)))?;

        Ok(())
    }

    /// Load the index from a cache file
    fn load_cache(&mut self, path: &Path) -> Result<()> {
        let json = fs::read_to_string(path)
            .map_err(|e| ExtractionError::IoError(format!("Failed to read cache file: {}", e)))?;

        let cache: IndexCache = serde_json::from_str(&json)
            .map_err(|e| ExtractionError::IoError(format!("Failed to parse cache: {}", e)))?;

        // Check version
        if cache.version != IndexCache::CURRENT_VERSION {
            return Err(ExtractionError::IoError(format!(
                "Cache version mismatch: expected {}, got {}",
                IndexCache::CURRENT_VERSION,
                cache.version
            )));
        }

        // Check if config matches (comparison folders changed)
        let current_config_hash = self.compute_config_hash();
        if cache.config_hash != current_config_hash {
            return Err(ExtractionError::IoError(
                "Cache config mismatch (folders changed)".to_string(),
            ));
        }

        // Load entries
        for entry in cache.entries {
            self.add_entry(entry);
        }

        self.compute_stats();

        Ok(())
    }

    /// Compute a hash of the configuration for cache validation
    fn compute_config_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // Include sorted folder paths
        let mut folders: Vec<_> = self
            .config
            .comparison_folders
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        folders.sort();

        for folder in folders {
            hasher.update(folder.as_bytes());
            hasher.update(b"|");
        }

        // Include relevant config options
        hasher.update(if self.config.recursive { b"r" } else { b"n" });
        hasher.update(if self.config.media_only { b"m" } else { b"a" });

        let result = hasher.finalize();
        result.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Compute SHA256 hash of a file using streaming (memory-efficient)
pub fn compute_file_hash(path: &Path) -> Result<Sha256Hash> {
    let file = File::open(path)
        .map_err(|e| ExtractionError::IoError(format!("Failed to open file: {}", e)))?;

    let mut reader = BufReader::with_capacity(HASH_BUFFER_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; HASH_BUFFER_SIZE];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| ExtractionError::IoError(format!("Failed to read file: {}", e)))?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);

    Ok(hash)
}

/// Compute SHA256 hash of in-memory data
pub fn compute_data_hash(data: &[u8]) -> Sha256Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Convert a hash to a hexadecimal string
#[allow(dead_code)]
pub fn hash_to_hex(hash: &Sha256Hash) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parse a hexadecimal string to a hash
#[allow(dead_code)]
pub fn hex_to_hash(hex: &str) -> Option<Sha256Hash> {
    if hex.len() != 64 {
        return None;
    }

    let mut hash = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hex_str = std::str::from_utf8(chunk).ok()?;
        hash[i] = u8::from_str_radix(hex_str, 16).ok()?;
    }

    Some(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = DuplicateConfig::new()
            .with_folder(PathBuf::from("/photos"))
            .with_folder(PathBuf::from("/backup"))
            .with_cache_file(PathBuf::from("./cache.json"))
            .with_recursive(true)
            .with_media_only(false);

        assert_eq!(config.comparison_folders.len(), 2);
        assert_eq!(config.cache_file, PathBuf::from("./cache.json"));
        assert!(config.recursive);
        assert!(!config.media_only);
    }

    #[test]
    fn test_config_defaults() {
        let config = DuplicateConfig::default();

        assert!(config.comparison_folders.is_empty());
        assert!(config.cache_enabled);
        assert!(config.recursive);
        assert!(config.media_only);
        assert_eq!(config.min_file_size, 0);
        assert_eq!(config.max_file_size, 0);
    }

    #[test]
    fn test_empty_index() {
        let config = DuplicateConfig::new();
        let index = DuplicateIndex::new(config);

        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        assert_eq!(index.stats().total_files, 0);
    }

    #[test]
    fn test_compute_data_hash() {
        let data = b"Hello, World!";
        let hash = compute_data_hash(data);

        // Known SHA256 hash of "Hello, World!"
        let expected = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        assert_eq!(hash_to_hex(&hash), expected);
    }

    #[test]
    fn test_hash_consistency() {
        let data = b"test data for hashing";

        let hash1 = compute_data_hash(data);
        let hash2 = compute_data_hash(data);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_data_different_hash() {
        let data1 = b"first file content";
        let data2 = b"second file content";

        let hash1 = compute_data_hash(data1);
        let hash2 = compute_data_hash(data2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_to_hex_and_back() {
        let original_hash = compute_data_hash(b"test");
        let hex = hash_to_hex(&original_hash);
        let parsed_hash = hex_to_hash(&hex).unwrap();

        assert_eq!(original_hash, parsed_hash);
    }

    #[test]
    fn test_hex_to_hash_invalid() {
        assert!(hex_to_hash("too_short").is_none());
        assert!(
            hex_to_hash("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz")
                .is_none()
        );
    }

    #[test]
    fn test_index_find_duplicate() {
        let config = DuplicateConfig::new();
        let mut index = DuplicateIndex::new(config);

        // Add an entry manually
        let data = b"duplicate content";
        let hash = compute_data_hash(data);
        let entry = IndexEntry {
            path: PathBuf::from("/photos/original.jpg"),
            size: data.len() as u64,
            hash,
            indexed_at: 0,
        };
        index.add_entry(entry);
        index.compute_stats();

        // Should find duplicate
        let dup = index.find_duplicate(data);
        assert!(dup.is_some());
        assert_eq!(dup.unwrap(), Path::new("/photos/original.jpg"));

        // Should not find non-duplicate
        let other_data = b"different content";
        assert!(index.find_duplicate(other_data).is_none());
    }

    #[test]
    fn test_index_size_filtering() {
        let config = DuplicateConfig::new();
        let mut index = DuplicateIndex::new(config);

        // Add an entry
        let data = b"original";
        let hash = compute_data_hash(data);
        let entry = IndexEntry {
            path: PathBuf::from("/photos/original.jpg"),
            size: data.len() as u64,
            hash,
            indexed_at: 0,
        };
        index.add_entry(entry);

        // Data with different size should return None without computing hash
        let different_size_data = b"this is much longer content that has different size";
        assert!(index.find_duplicate(different_size_data).is_none());
    }

    #[test]
    fn test_find_internal_duplicates() {
        let config = DuplicateConfig::new();
        let mut index = DuplicateIndex::new(config);

        let data = b"shared content";
        let hash = compute_data_hash(data);

        // Add multiple entries with same hash
        for i in 0..3 {
            let entry = IndexEntry {
                path: PathBuf::from(format!("/photos/file{}.jpg", i)),
                size: data.len() as u64,
                hash,
                indexed_at: 0,
            };
            index.add_entry(entry);
        }

        index.compute_stats();

        let groups = index.find_internal_duplicates();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].paths.len(), 3);
    }

    #[test]
    fn test_index_stats() {
        let config = DuplicateConfig::new();
        let mut index = DuplicateIndex::new(config);

        // Add some entries
        for i in 0..5 {
            let data = format!("file content {}", i);
            let hash = compute_data_hash(data.as_bytes());
            let entry = IndexEntry {
                path: PathBuf::from(format!("/photos/file{}.jpg", i)),
                size: data.len() as u64,
                hash,
                indexed_at: 0,
            };
            index.add_entry(entry);
        }

        // Add a duplicate
        let dup_data = "file content 0";
        let dup_hash = compute_data_hash(dup_data.as_bytes());
        let dup_entry = IndexEntry {
            path: PathBuf::from("/photos/duplicate.jpg"),
            size: dup_data.len() as u64,
            hash: dup_hash,
            indexed_at: 0,
        };
        index.add_entry(dup_entry);

        index.compute_stats();

        assert_eq!(index.stats().total_files, 6);
        assert_eq!(index.stats().unique_hashes, 5);
        assert_eq!(index.stats().duplicate_groups, 1);
        assert_eq!(index.stats().duplicate_files, 1);
    }

    #[test]
    fn test_index_merge() {
        let config = DuplicateConfig::new();
        let mut index1 = DuplicateIndex::new(config.clone());
        let mut index2 = DuplicateIndex::new(config);

        // Add entries to first index
        let data1 = b"content 1";
        let entry1 = IndexEntry {
            path: PathBuf::from("/photos/file1.jpg"),
            size: data1.len() as u64,
            hash: compute_data_hash(data1),
            indexed_at: 0,
        };
        index1.add_entry(entry1);

        // Add entries to second index
        let data2 = b"content 2";
        let entry2 = IndexEntry {
            path: PathBuf::from("/photos/file2.jpg"),
            size: data2.len() as u64,
            hash: compute_data_hash(data2),
            indexed_at: 0,
        };
        index2.add_entry(entry2);

        // Merge
        index1.merge(index2);

        assert_eq!(index1.len(), 2);
        assert!(index1.find_duplicate(data1).is_some());
        assert!(index1.find_duplicate(data2).is_some());
    }

    #[test]
    fn test_index_entry_serialization() {
        let data = b"test content";
        let entry = IndexEntry {
            path: PathBuf::from("/photos/test.jpg"),
            size: data.len() as u64,
            hash: compute_data_hash(data),
            indexed_at: 1234567890,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: IndexEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry.path, deserialized.path);
        assert_eq!(entry.size, deserialized.size);
        assert_eq!(entry.hash, deserialized.hash);
        assert_eq!(entry.indexed_at, deserialized.indexed_at);
    }

    #[test]
    fn test_media_extensions() {
        // Verify common extensions are included
        assert!(MEDIA_EXTENSIONS.contains(&"jpg"));
        assert!(MEDIA_EXTENSIONS.contains(&"jpeg"));
        assert!(MEDIA_EXTENSIONS.contains(&"png"));
        assert!(MEDIA_EXTENSIONS.contains(&"heic"));
        assert!(MEDIA_EXTENSIONS.contains(&"mp4"));
        assert!(MEDIA_EXTENSIONS.contains(&"mov"));

        // Verify non-media extensions are not included
        assert!(!MEDIA_EXTENSIONS.contains(&"txt"));
        assert!(!MEDIA_EXTENSIONS.contains(&"doc"));
        assert!(!MEDIA_EXTENSIONS.contains(&"exe"));
    }

    #[test]
    fn test_duplicate_config_with_size_limits() {
        let config = DuplicateConfig::new()
            .with_min_size(1024)
            .with_max_size(10 * 1024 * 1024);

        assert_eq!(config.min_file_size, 1024);
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
    }
}

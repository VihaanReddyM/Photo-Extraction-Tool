//! Duplicate Detection Module
//!
//! Provides efficient duplicate file detection using SHA256 cryptographic hashing.
//! This module is designed for exact-match duplicate detection, which is reliable,
//! fast, and works for all file types (photos, videos, etc.).
//!
//! # Features
//!
//! - **SHA256 hashing** - Cryptographically secure, collision-resistant
//! - **Size pre-filtering** - O(1) rejection of files with different sizes
//! - **Streaming hash** - Memory-efficient processing of large files
//! - **Parallel indexing** - Multi-threaded folder scanning
//! - **Persistent cache** - JSON cache for faster subsequent runs
//! - **Works with all files** - Photos, videos, and any other file type
//!
//! # Architecture
//!
//! The module uses a two-tier lookup strategy:
//! 1. Files are first grouped by size (O(1) HashMap lookup)
//! 2. Only files with matching sizes have their hashes compared
//!
//! This avoids computing expensive SHA256 hashes for files that can't
//! possibly be duplicates (different sizes).
//!
//! # Example
//!
//! ```rust,no_run
//! use photo_extraction_tool::duplicate::{DuplicateIndex, DuplicateConfig};
//! use std::path::PathBuf;
//! use std::sync::Arc;
//! use std::sync::atomic::AtomicBool;
//!
//! // Create configuration
//! let config = DuplicateConfig::new()
//!     .with_folder(PathBuf::from("D:/Photos"))
//!     .with_cache(true);
//!
//! // Build index from folders
//! let shutdown = Arc::new(AtomicBool::new(false));
//! let index = DuplicateIndex::build_from_folders(&config, shutdown, |_| {}).unwrap();
//!
//! // Check if data is a duplicate
//! let file_data = std::fs::read("new_photo.jpg").unwrap();
//! match index.find_duplicate(&file_data) {
//!     Some(original) => println!("Duplicate of: {}", original.display()),
//!     None => println!("Not a duplicate"),
//! }
//! ```

pub mod detector;

// Re-export main types for convenience
// Primary types used by the extractor
pub use detector::{compute_data_hash, DuplicateConfig, DuplicateIndex};

// Additional public API types (may not be used internally but are part of public interface)
#[allow(unused_imports)]
pub use detector::{
    compute_file_hash, hash_to_hex, hex_to_hash, DuplicateGroup, IndexEntry, IndexProgress,
    IndexStats, Sha256Hash,
};

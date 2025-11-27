//! Mock data generator for creating realistic test files
//!
//! This module generates mock file content that mimics real image and video files
//! with proper headers and structure for testing purposes.
//!
//! Note: Content sizes are kept small by default to prevent memory issues during testing.
//! The file headers are realistic enough for testing extraction logic without needing
//! megabytes of actual data.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// =============================================================================
// CONSTANTS FOR MEMORY-EFFICIENT TESTING
// =============================================================================

/// Maximum content size for test files (4KB) - enough for headers and testing
pub const TEST_CONTENT_MAX_SIZE: usize = 4 * 1024;

/// Default content size for JPEG test files
pub const TEST_JPEG_SIZE: usize = 2 * 1024;

/// Default content size for HEIC test files
pub const TEST_HEIC_SIZE: usize = 2 * 1024;

/// Default content size for PNG test files
pub const TEST_PNG_SIZE: usize = 1024;

/// Default content size for video test files (MOV, MP4, etc.)
pub const TEST_VIDEO_SIZE: usize = 4 * 1024;

/// Default content size for stress test files (minimal)
pub const TEST_STRESS_SIZE: usize = 512;

/// Configuration for file generation
#[derive(Debug, Clone)]
pub struct FileGeneratorConfig {
    /// Whether to include realistic EXIF data
    pub include_exif: bool,
    /// Whether to generate varied content (vs all zeros)
    pub varied_content: bool,
    /// Base timestamp for date generation
    pub base_timestamp: i64,
}

impl Default for FileGeneratorConfig {
    fn default() -> Self {
        Self {
            include_exif: true,
            varied_content: true,
            base_timestamp: 1704067200, // 2024-01-01 00:00:00 UTC
        }
    }
}

/// Mock data generator for creating realistic file content
pub struct MockDataGenerator;

impl MockDataGenerator {
    // =========================================================================
    // JPEG GENERATION
    // =========================================================================

    /// Generate a minimal JPEG header
    pub fn generate_jpeg_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // JPEG SOI (Start of Image) marker
        data.extend_from_slice(&[0xFF, 0xD8]);

        // APP0 marker (JFIF)
        data.extend_from_slice(&[0xFF, 0xE0]);
        data.extend_from_slice(&[0x00, 0x10]); // Length
        data.extend_from_slice(b"JFIF\0");
        data.extend_from_slice(&[0x01, 0x01]); // Version 1.1
        data.extend_from_slice(&[0x00]); // Aspect ratio units
        data.extend_from_slice(&[0x00, 0x01]); // X density
        data.extend_from_slice(&[0x00, 0x01]); // Y density
        data.extend_from_slice(&[0x00, 0x00]); // Thumbnail size

        // Pad to requested size
        while data.len() < size.saturating_sub(2) {
            data.push(0x00);
        }

        // JPEG EOI (End of Image) marker
        data.extend_from_slice(&[0xFF, 0xD9]);

        data
    }

    /// Generate JPEG with deterministic content based on seed
    pub fn generate_jpeg_with_seed(size: usize, seed: u64) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // JPEG SOI marker
        data.extend_from_slice(&[0xFF, 0xD8]);

        // APP0 marker (JFIF)
        data.extend_from_slice(&[0xFF, 0xE0]);
        data.extend_from_slice(&[0x00, 0x10]);
        data.extend_from_slice(b"JFIF\0");
        data.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00]);

        // Add some "image data" based on seed
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();

        // Use hash to generate pseudo-random content
        let mut current = hash;
        while data.len() < size.saturating_sub(2) {
            current = current.wrapping_mul(6364136223846793005).wrapping_add(1);
            data.push((current >> 33) as u8);
        }

        // JPEG EOI marker
        data.extend_from_slice(&[0xFF, 0xD9]);

        data.truncate(size);
        data
    }

    /// Generate JPEG with EXIF data
    pub fn generate_jpeg_with_exif(size: usize, date_taken: &str, camera_model: &str) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // JPEG SOI marker
        data.extend_from_slice(&[0xFF, 0xD8]);

        // APP1 marker (EXIF)
        data.extend_from_slice(&[0xFF, 0xE1]);

        // Build EXIF data
        let mut exif = Vec::new();
        exif.extend_from_slice(b"Exif\0\0");

        // TIFF header (little endian)
        exif.extend_from_slice(&[0x49, 0x49]); // Little endian
        exif.extend_from_slice(&[0x2A, 0x00]); // TIFF magic
        exif.extend_from_slice(&[0x08, 0x00, 0x00, 0x00]); // IFD0 offset

        // Simplified IFD0
        exif.extend_from_slice(&[0x02, 0x00]); // 2 entries

        // DateTime tag (0x0132)
        exif.extend_from_slice(&[0x32, 0x01]); // Tag
        exif.extend_from_slice(&[0x02, 0x00]); // Type: ASCII
        let date_bytes = date_taken.as_bytes();
        let date_len = (date_bytes.len() + 1) as u32;
        exif.extend_from_slice(&date_len.to_le_bytes());
        if date_len <= 4 {
            exif.extend_from_slice(date_bytes);
            exif.push(0);
            while exif.len() % 4 != 0 {
                exif.push(0);
            }
        } else {
            exif.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Offset (simplified)
        }

        // Model tag (0x0110)
        exif.extend_from_slice(&[0x10, 0x01]); // Tag
        exif.extend_from_slice(&[0x02, 0x00]); // Type: ASCII
        let model_bytes = camera_model.as_bytes();
        let model_len = (model_bytes.len() + 1) as u32;
        exif.extend_from_slice(&model_len.to_le_bytes());
        exif.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Offset (simplified)

        // End of IFD
        exif.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // Write EXIF length
        let exif_len = (exif.len() + 2) as u16;
        data.extend_from_slice(&exif_len.to_be_bytes());
        data.extend_from_slice(&exif);

        // APP0 marker (JFIF) - for compatibility
        data.extend_from_slice(&[0xFF, 0xE0]);
        data.extend_from_slice(&[0x00, 0x10]);
        data.extend_from_slice(b"JFIF\0");
        data.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00]);

        // Pad to size
        while data.len() < size.saturating_sub(2) {
            data.push(0x00);
        }

        // JPEG EOI marker
        data.extend_from_slice(&[0xFF, 0xD9]);

        data.truncate(size);
        data
    }

    // =========================================================================
    // HEIC GENERATION
    // =========================================================================

    /// Generate a minimal HEIC header
    pub fn generate_heic_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // ftyp box
        let ftyp_size: u32 = 24;
        data.extend_from_slice(&ftyp_size.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"heic"); // Major brand
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Minor version
        data.extend_from_slice(b"heic"); // Compatible brand 1
        data.extend_from_slice(b"mif1"); // Compatible brand 2

        // meta box placeholder
        let meta_size: u32 = 8;
        data.extend_from_slice(&meta_size.to_be_bytes());
        data.extend_from_slice(b"meta");

        // Pad to size
        while data.len() < size {
            data.push(0x00);
        }

        data.truncate(size);
        data
    }

    /// Generate HEIC with seed for unique content
    pub fn generate_heic_with_seed(size: usize, seed: u64) -> Vec<u8> {
        let mut data = Self::generate_heic_header(size.min(100));

        // Generate deterministic content
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();

        let mut current = hash;
        while data.len() < size {
            current = current.wrapping_mul(6364136223846793005).wrapping_add(1);
            data.push((current >> 33) as u8);
        }

        data.truncate(size);
        data
    }

    // =========================================================================
    // PNG GENERATION
    // =========================================================================

    /// Generate a minimal PNG header
    pub fn generate_png_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // PNG signature
        data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

        // IHDR chunk
        let ihdr_data: [u8; 13] = [
            0x00, 0x00, 0x00, 0x64, // Width: 100
            0x00, 0x00, 0x00, 0x64, // Height: 100
            0x08, // Bit depth: 8
            0x02, // Color type: RGB
            0x00, // Compression
            0x00, // Filter
            0x00, // Interlace
        ];

        let ihdr_len: u32 = 13;
        data.extend_from_slice(&ihdr_len.to_be_bytes());
        data.extend_from_slice(b"IHDR");
        data.extend_from_slice(&ihdr_data);
        // CRC placeholder
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // Pad with IDAT-like data
        while data.len() < size.saturating_sub(12) {
            data.push(0x00);
        }

        // IEND chunk
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Length: 0
        data.extend_from_slice(b"IEND");
        data.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]); // CRC

        data.truncate(size);
        data
    }

    /// Generate PNG with seed
    pub fn generate_png_with_seed(size: usize, seed: u64) -> Vec<u8> {
        let mut data = Self::generate_png_header(size.min(50));

        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();

        let mut current = hash;
        while data.len() < size {
            current = current.wrapping_mul(6364136223846793005).wrapping_add(1);
            data.push((current >> 33) as u8);
        }

        data.truncate(size);
        data
    }

    // =========================================================================
    // GIF GENERATION
    // =========================================================================

    /// Generate a minimal GIF header
    pub fn generate_gif_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // GIF89a header
        data.extend_from_slice(b"GIF89a");

        // Logical screen descriptor
        data.extend_from_slice(&[0x64, 0x00]); // Width: 100
        data.extend_from_slice(&[0x64, 0x00]); // Height: 100
        data.extend_from_slice(&[0x00]); // Packed byte (no global color table)
        data.extend_from_slice(&[0x00]); // Background color
        data.extend_from_slice(&[0x00]); // Pixel aspect ratio

        // Pad to size
        while data.len() < size.saturating_sub(1) {
            data.push(0x00);
        }

        // GIF trailer
        data.push(0x3B);

        data.truncate(size);
        data
    }

    // =========================================================================
    // WEBP GENERATION
    // =========================================================================

    /// Generate a minimal WebP header
    pub fn generate_webp_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // RIFF header
        data.extend_from_slice(b"RIFF");
        let file_size = (size as u32).saturating_sub(8);
        data.extend_from_slice(&file_size.to_le_bytes());
        data.extend_from_slice(b"WEBP");

        // VP8 chunk header
        data.extend_from_slice(b"VP8 ");
        let chunk_size = (size as u32).saturating_sub(20);
        data.extend_from_slice(&chunk_size.to_le_bytes());

        // VP8 frame header (simplified)
        data.extend_from_slice(&[0x9D, 0x01, 0x2A]); // VP8 signature

        // Pad to size
        while data.len() < size {
            data.push(0x00);
        }

        data.truncate(size);
        data
    }

    // =========================================================================
    // RAW/DNG GENERATION
    // =========================================================================

    /// Generate a minimal RAW/DNG header
    pub fn generate_raw_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // TIFF header (DNG is TIFF-based)
        data.extend_from_slice(&[0x49, 0x49]); // Little endian
        data.extend_from_slice(&[0x2A, 0x00]); // TIFF magic number
        data.extend_from_slice(&[0x08, 0x00, 0x00, 0x00]); // IFD offset

        // Simplified IFD
        data.extend_from_slice(&[0x00, 0x00]); // 0 entries (simplified)
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Next IFD offset

        // Pad to size
        while data.len() < size {
            data.push(0x00);
        }

        data.truncate(size);
        data
    }

    // =========================================================================
    // TIFF GENERATION
    // =========================================================================

    /// Generate a minimal TIFF header
    pub fn generate_tiff_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // TIFF header (big endian)
        data.extend_from_slice(&[0x4D, 0x4D]); // Big endian
        data.extend_from_slice(&[0x00, 0x2A]); // TIFF magic number
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x08]); // IFD offset

        // Simplified IFD
        data.extend_from_slice(&[0x00, 0x00]); // 0 entries
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Next IFD offset

        // Pad to size
        while data.len() < size {
            data.push(0x00);
        }

        data.truncate(size);
        data
    }

    // =========================================================================
    // BMP GENERATION
    // =========================================================================

    /// Generate a minimal BMP header
    pub fn generate_bmp_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // BMP file header
        data.extend_from_slice(b"BM"); // Signature
        data.extend_from_slice(&(size as u32).to_le_bytes()); // File size
        data.extend_from_slice(&[0x00, 0x00]); // Reserved
        data.extend_from_slice(&[0x00, 0x00]); // Reserved
        data.extend_from_slice(&[0x36, 0x00, 0x00, 0x00]); // Pixel data offset

        // DIB header (BITMAPINFOHEADER)
        data.extend_from_slice(&[0x28, 0x00, 0x00, 0x00]); // Header size: 40
        data.extend_from_slice(&[0x64, 0x00, 0x00, 0x00]); // Width: 100
        data.extend_from_slice(&[0x64, 0x00, 0x00, 0x00]); // Height: 100
        data.extend_from_slice(&[0x01, 0x00]); // Planes: 1
        data.extend_from_slice(&[0x18, 0x00]); // Bits per pixel: 24
        data.extend_from_slice(&[0x00; 24]); // Compression and other fields

        // Pad to size
        while data.len() < size {
            data.push(0x00);
        }

        data.truncate(size);
        data
    }

    // =========================================================================
    // VIDEO FORMAT GENERATION
    // =========================================================================

    /// Generate a minimal MOV header
    pub fn generate_mov_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // ftyp atom
        let ftyp_size: u32 = 20;
        data.extend_from_slice(&ftyp_size.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"qt  "); // Major brand: QuickTime
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Minor version
        data.extend_from_slice(b"qt  "); // Compatible brand

        // moov atom placeholder
        let moov_size: u32 = 8;
        data.extend_from_slice(&moov_size.to_be_bytes());
        data.extend_from_slice(b"moov");

        // mdat atom placeholder
        let mdat_size: u32 = (size as u32).saturating_sub(data.len() as u32);
        data.extend_from_slice(&mdat_size.to_be_bytes());
        data.extend_from_slice(b"mdat");

        // Pad to size
        while data.len() < size {
            data.push(0x00);
        }

        data.truncate(size);
        data
    }

    /// Generate MOV with seed
    pub fn generate_mov_with_seed(size: usize, seed: u64) -> Vec<u8> {
        let mut data = Self::generate_mov_header(size.min(100));

        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();

        let mut current = hash;
        while data.len() < size {
            current = current.wrapping_mul(6364136223846793005).wrapping_add(1);
            data.push((current >> 33) as u8);
        }

        data.truncate(size);
        data
    }

    /// Generate a minimal MP4 header
    pub fn generate_mp4_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // ftyp atom
        let ftyp_size: u32 = 24;
        data.extend_from_slice(&ftyp_size.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"isom"); // Major brand
        data.extend_from_slice(&[0x00, 0x00, 0x02, 0x00]); // Minor version
        data.extend_from_slice(b"isom"); // Compatible brand 1
        data.extend_from_slice(b"mp41"); // Compatible brand 2

        // moov atom placeholder
        let moov_size: u32 = 8;
        data.extend_from_slice(&moov_size.to_be_bytes());
        data.extend_from_slice(b"moov");

        // mdat atom placeholder
        let mdat_size: u32 = (size as u32).saturating_sub(data.len() as u32);
        data.extend_from_slice(&mdat_size.to_be_bytes());
        data.extend_from_slice(b"mdat");

        // Pad to size
        while data.len() < size {
            data.push(0x00);
        }

        data.truncate(size);
        data
    }

    /// Generate a minimal AVI header
    pub fn generate_avi_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // RIFF header
        data.extend_from_slice(b"RIFF");
        let file_size = (size as u32).saturating_sub(8);
        data.extend_from_slice(&file_size.to_le_bytes());
        data.extend_from_slice(b"AVI ");

        // LIST hdrl chunk
        data.extend_from_slice(b"LIST");
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Size placeholder
        data.extend_from_slice(b"hdrl");

        // avih (main AVI header)
        data.extend_from_slice(b"avih");
        data.extend_from_slice(&[0x38, 0x00, 0x00, 0x00]); // Size: 56

        // Main AVI header data (simplified)
        data.extend_from_slice(&[0x00; 56]);

        // Pad to size
        while data.len() < size {
            data.push(0x00);
        }

        data.truncate(size);
        data
    }

    /// Generate a minimal 3GP header
    pub fn generate_3gp_header(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);

        // ftyp atom
        let ftyp_size: u32 = 20;
        data.extend_from_slice(&ftyp_size.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"3gp4"); // Major brand
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Minor version
        data.extend_from_slice(b"3gp4"); // Compatible brand

        // moov atom placeholder
        let moov_size: u32 = 8;
        data.extend_from_slice(&moov_size.to_be_bytes());
        data.extend_from_slice(b"moov");

        // mdat atom placeholder
        let mdat_size: u32 = (size as u32).saturating_sub(data.len() as u32);
        data.extend_from_slice(&mdat_size.to_be_bytes());
        data.extend_from_slice(b"mdat");

        // Pad to size
        while data.len() < size {
            data.push(0x00);
        }

        data.truncate(size);
        data
    }

    // =========================================================================
    // UTILITY FUNCTIONS
    // =========================================================================

    /// Generate content for a file based on extension
    pub fn generate_for_extension(extension: &str, size: usize) -> Vec<u8> {
        match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" => Self::generate_jpeg_header(size),
            "heic" | "heif" => Self::generate_heic_header(size),
            "png" => Self::generate_png_header(size),
            "gif" => Self::generate_gif_header(size),
            "webp" => Self::generate_webp_header(size),
            "raw" | "dng" => Self::generate_raw_header(size),
            "tiff" | "tif" => Self::generate_tiff_header(size),
            "bmp" => Self::generate_bmp_header(size),
            "mov" => Self::generate_mov_header(size),
            "mp4" | "m4v" => Self::generate_mp4_header(size),
            "avi" => Self::generate_avi_header(size),
            "3gp" => Self::generate_3gp_header(size),
            _ => vec![0x00; size], // Unknown format
        }
    }

    /// Generate content with seed based on extension
    pub fn generate_for_extension_with_seed(extension: &str, size: usize, seed: u64) -> Vec<u8> {
        match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" => Self::generate_jpeg_with_seed(size, seed),
            "heic" | "heif" => Self::generate_heic_with_seed(size, seed),
            "png" => Self::generate_png_with_seed(size, seed),
            "mov" => Self::generate_mov_with_seed(size, seed),
            _ => {
                let mut data = Self::generate_for_extension(extension, size.min(100));
                let mut hasher = DefaultHasher::new();
                seed.hash(&mut hasher);
                let hash = hasher.finish();

                let mut current = hash;
                while data.len() < size {
                    current = current.wrapping_mul(6364136223846793005).wrapping_add(1);
                    data.push((current >> 33) as u8);
                }
                data.truncate(size);
                data
            }
        }
    }

    /// Generate a batch of files with varied content
    pub fn generate_batch(
        extensions: &[&str],
        count_per_extension: usize,
        base_size: usize,
    ) -> Vec<(String, Vec<u8>)> {
        let mut files = Vec::new();

        for (ext_idx, ext) in extensions.iter().enumerate() {
            for i in 0..count_per_extension {
                let seed = (ext_idx * 1000 + i) as u64;
                let size_variation = ((seed % 50) as i64 - 25) * (base_size as i64 / 100);
                let size = (base_size as i64 + size_variation).max(100) as usize;

                let filename = format!("IMG_{:04}.{}", ext_idx * 1000 + i, ext.to_uppercase());
                let content = Self::generate_for_extension_with_seed(ext, size, seed);

                files.push((filename, content));
            }
        }

        files
    }

    /// Generate a realistic photo library batch
    ///
    /// Note: Uses small content sizes by default to prevent memory issues.
    /// The reported file sizes can be set separately from actual content.
    pub fn generate_photo_library(total_files: usize) -> Vec<(String, Vec<u8>)> {
        let mut files = Vec::with_capacity(total_files);

        for i in 0..total_files {
            // Use lightweight content sizes - actual test only needs valid headers
            let (ext, size) = match i % 10 {
                0..=5 => ("HEIC", TEST_HEIC_SIZE), // 60% HEIC
                6..=7 => ("JPG", TEST_JPEG_SIZE),  // 20% JPG
                8 => ("MOV", TEST_VIDEO_SIZE),     // 10% MOV
                _ => ("PNG", TEST_PNG_SIZE),       // 10% PNG
            };

            let filename = format!("IMG_{:04}.{}", i, ext);
            let content = Self::generate_for_extension_with_seed(ext, size, i as u64);

            files.push((filename, content));
        }

        files
    }

    /// Generate lightweight content for testing - uses minimal memory
    ///
    /// This generates just enough data for valid file headers without
    /// allocating large amounts of memory.
    pub fn generate_lightweight(extension: &str, seed: u64) -> Vec<u8> {
        let size = match extension.to_lowercase().as_str() {
            "mov" | "mp4" | "m4v" | "avi" | "3gp" => TEST_VIDEO_SIZE,
            "heic" | "heif" => TEST_HEIC_SIZE,
            "jpg" | "jpeg" => TEST_JPEG_SIZE,
            "png" => TEST_PNG_SIZE,
            _ => TEST_STRESS_SIZE,
        };
        Self::generate_for_extension_with_seed(extension, size, seed)
    }

    /// Generate content capped at a maximum size for memory efficiency
    pub fn generate_capped(extension: &str, requested_size: usize, seed: u64) -> Vec<u8> {
        let actual_size = requested_size.min(TEST_CONTENT_MAX_SIZE);
        Self::generate_for_extension_with_seed(extension, actual_size, seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jpeg_header() {
        let data = MockDataGenerator::generate_jpeg_header(1024);
        assert_eq!(data.len(), 1024);
        assert_eq!(&data[0..2], &[0xFF, 0xD8]); // SOI
        assert_eq!(&data[data.len() - 2..], &[0xFF, 0xD9]); // EOI
    }

    #[test]
    fn test_heic_header() {
        let data = MockDataGenerator::generate_heic_header(1024);
        assert_eq!(data.len(), 1024);
        assert_eq!(&data[4..8], b"ftyp");
        assert_eq!(&data[8..12], b"heic");
    }

    #[test]
    fn test_png_header() {
        let data = MockDataGenerator::generate_png_header(1024);
        assert_eq!(data.len(), 1024);
        assert_eq!(
            &data[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[test]
    fn test_seeded_content_is_deterministic() {
        let data1 = MockDataGenerator::generate_jpeg_with_seed(1024, 12345);
        let data2 = MockDataGenerator::generate_jpeg_with_seed(1024, 12345);
        assert_eq!(data1, data2);

        let data3 = MockDataGenerator::generate_jpeg_with_seed(1024, 54321);
        assert_ne!(data1, data3);
    }

    #[test]
    fn test_batch_generation() {
        let batch = MockDataGenerator::generate_batch(&["jpg", "png", "heic"], 5, 1024);
        assert_eq!(batch.len(), 15); // 3 extensions * 5 files each
    }

    #[test]
    fn test_mov_header() {
        let data = MockDataGenerator::generate_mov_header(1024);
        assert_eq!(&data[4..8], b"ftyp");
        assert_eq!(&data[8..12], b"qt  ");
    }
}

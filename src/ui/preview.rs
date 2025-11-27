//! Preview and Thumbnail Support Module
//!
//! Provides functionality for generating and caching thumbnails/previews
//! of photos before extraction. This enables UIs to show users what
//! photos are available on the device.

use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::device::{DeviceContentTrait, DeviceObject};

// =============================================================================
// Thumbnail Configuration
// =============================================================================

/// Configuration for thumbnail generation
#[derive(Debug, Clone)]
pub struct ThumbnailConfig {
    /// Maximum width for thumbnails (in pixels)
    pub max_width: u32,
    /// Maximum height for thumbnails (in pixels)
    pub max_height: u32,
    /// JPEG quality for thumbnails (1-100)
    pub quality: u8,
    /// Maximum file size to attempt thumbnail generation (in bytes)
    pub max_source_size: u64,
    /// Whether to cache thumbnails in memory
    pub enable_cache: bool,
    /// Maximum number of thumbnails to cache
    pub cache_max_entries: usize,
    /// Cache entry lifetime
    pub cache_lifetime: Duration,
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            max_width: 256,
            max_height: 256,
            quality: 80,
            max_source_size: 50 * 1024 * 1024, // 50 MB
            enable_cache: true,
            cache_max_entries: 500,
            cache_lifetime: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl ThumbnailConfig {
    /// Create a config for small thumbnails (icon size)
    pub fn icon() -> Self {
        Self {
            max_width: 64,
            max_height: 64,
            quality: 70,
            cache_max_entries: 1000,
            ..Default::default()
        }
    }

    /// Create a config for medium thumbnails (list view)
    pub fn medium() -> Self {
        Self {
            max_width: 128,
            max_height: 128,
            quality: 75,
            ..Default::default()
        }
    }

    /// Create a config for large previews (detail view)
    pub fn large() -> Self {
        Self {
            max_width: 512,
            max_height: 512,
            quality: 85,
            cache_max_entries: 200,
            ..Default::default()
        }
    }

    /// Set maximum dimensions
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.max_width = width;
        self.max_height = height;
        self
    }

    /// Set quality
    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = quality.min(100).max(1);
        self
    }

    /// Set cache settings
    pub fn with_cache(mut self, enabled: bool, max_entries: usize) -> Self {
        self.enable_cache = enabled;
        self.cache_max_entries = max_entries;
        self
    }
}

// =============================================================================
// Thumbnail Data
// =============================================================================

/// Represents a generated thumbnail
#[derive(Debug, Clone)]
pub struct Thumbnail {
    /// The thumbnail image data (JPEG or PNG bytes)
    pub data: Vec<u8>,
    /// Original width of the source image
    pub original_width: u32,
    /// Original height of the source image
    pub original_height: u32,
    /// Thumbnail width
    pub width: u32,
    /// Thumbnail height
    pub height: u32,
    /// MIME type of the thumbnail
    pub mime_type: String,
    /// Size of the original file
    pub original_size: u64,
    /// When this thumbnail was generated
    pub generated_at: Instant,
}

impl Thumbnail {
    /// Check if the thumbnail is still fresh according to lifetime
    pub fn is_fresh(&self, lifetime: Duration) -> bool {
        self.generated_at.elapsed() < lifetime
    }

    /// Get the aspect ratio of the original image
    pub fn original_aspect_ratio(&self) -> f64 {
        if self.original_height == 0 {
            1.0
        } else {
            self.original_width as f64 / self.original_height as f64
        }
    }

    /// Get the thumbnail as a data URL for use in HTML/web UIs
    pub fn as_data_url(&self) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let base64_data = STANDARD.encode(&self.data);
        format!("data:{};base64,{}", self.mime_type, base64_data)
    }
}

// =============================================================================
// Preview Item
// =============================================================================

/// Information about a previewable item on the device
#[derive(Debug, Clone)]
pub struct PreviewItem {
    /// Object ID on the device
    pub object_id: String,
    /// File name
    pub name: String,
    /// Full path on device
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// Date modified (if available)
    pub date_modified: Option<String>,
    /// Content type / MIME type
    pub content_type: String,
    /// Whether this is a video (vs. photo)
    pub is_video: bool,
    /// Cached thumbnail (if loaded)
    pub thumbnail: Option<Thumbnail>,
    /// Whether thumbnail generation was attempted
    pub thumbnail_attempted: bool,
    /// Error message if thumbnail generation failed
    pub thumbnail_error: Option<String>,
}

impl PreviewItem {
    /// Create from a DeviceObject
    pub fn from_device_object(obj: &DeviceObject, path: String) -> Self {
        let content_type = obj.content_type.clone().unwrap_or_default();
        let is_video = content_type.starts_with("video/")
            || matches!(
                Path::new(&obj.name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .as_deref(),
                Some("mp4" | "mov" | "m4v" | "avi" | "mkv" | "3gp")
            );

        Self {
            object_id: obj.object_id.clone(),
            name: obj.name.clone(),
            path,
            size: obj.size,
            date_modified: obj.date_modified.clone(),
            content_type,
            is_video,
            thumbnail: None,
            thumbnail_attempted: false,
            thumbnail_error: None,
        }
    }

    /// Check if this item supports thumbnail generation
    pub fn supports_thumbnail(&self) -> bool {
        // Currently only support image thumbnails
        // Video thumbnails would require additional dependencies (ffmpeg, etc.)
        if self.is_video {
            return false;
        }

        let ext = Path::new(&self.name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        matches!(
            ext.as_deref(),
            Some("jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "heif")
        )
    }

    /// Get a display-friendly size string
    pub fn formatted_size(&self) -> String {
        if self.size < 1024 {
            format!("{} B", self.size)
        } else if self.size < 1024 * 1024 {
            format!("{:.1} KB", self.size as f64 / 1024.0)
        } else if self.size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", self.size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", self.size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    /// Get file extension
    pub fn extension(&self) -> Option<String> {
        Path::new(&self.name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
    }
}

// =============================================================================
// Thumbnail Cache
// =============================================================================

/// Cache entry with metadata
struct CacheEntry {
    thumbnail: Thumbnail,
    last_accessed: Instant,
}

/// In-memory thumbnail cache
pub struct ThumbnailCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    config: ThumbnailConfig,
}

impl ThumbnailCache {
    /// Create a new cache with the given configuration
    pub fn new(config: ThumbnailConfig) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Get a thumbnail from the cache
    pub fn get(&self, object_id: &str) -> Option<Thumbnail> {
        let mut entries = self.entries.write().unwrap();

        if let Some(entry) = entries.get_mut(object_id) {
            if entry.thumbnail.is_fresh(self.config.cache_lifetime) {
                entry.last_accessed = Instant::now();
                return Some(entry.thumbnail.clone());
            } else {
                // Expired - remove it
                entries.remove(object_id);
            }
        }

        None
    }

    /// Store a thumbnail in the cache
    pub fn put(&self, object_id: String, thumbnail: Thumbnail) {
        if !self.config.enable_cache {
            return;
        }

        let mut entries = self.entries.write().unwrap();

        // Evict old entries if at capacity
        if entries.len() >= self.config.cache_max_entries {
            self.evict_oldest(&mut entries);
        }

        entries.insert(
            object_id,
            CacheEntry {
                thumbnail,
                last_accessed: Instant::now(),
            },
        );
    }

    /// Remove a thumbnail from the cache
    pub fn remove(&self, object_id: &str) {
        self.entries.write().unwrap().remove(object_id);
    }

    /// Clear the entire cache
    pub fn clear(&self) {
        self.entries.write().unwrap().clear();
    }

    /// Get the number of cached thumbnails
    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().unwrap().is_empty()
    }

    /// Evict the oldest entries to make room
    fn evict_oldest(&self, entries: &mut HashMap<String, CacheEntry>) {
        // Remove 10% of entries, oldest first
        let to_remove = (entries.len() / 10).max(1);

        let mut items: Vec<_> = entries
            .iter()
            .map(|(k, v)| (k.clone(), v.last_accessed))
            .collect();

        items.sort_by(|a, b| a.1.cmp(&b.1));

        for (key, _) in items.into_iter().take(to_remove) {
            entries.remove(&key);
        }
    }

    /// Remove expired entries
    pub fn cleanup_expired(&self) {
        let mut entries = self.entries.write().unwrap();
        let lifetime = self.config.cache_lifetime;

        entries.retain(|_, entry| entry.thumbnail.is_fresh(lifetime));
    }
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::new(ThumbnailConfig::default())
    }
}

// =============================================================================
// Thumbnail Generator
// =============================================================================

/// Result of a thumbnail generation attempt
#[derive(Debug)]
pub enum ThumbnailResult {
    /// Successfully generated thumbnail
    Success(Thumbnail),
    /// File is not a supported image format
    UnsupportedFormat,
    /// File is too large to process
    FileTooLarge(u64),
    /// Error during generation
    Error(String),
    /// Thumbnail generation is not available (missing dependencies)
    NotAvailable,
}

/// Thumbnail generator that works with device content
pub struct ThumbnailGenerator {
    config: ThumbnailConfig,
    cache: ThumbnailCache,
}

impl ThumbnailGenerator {
    /// Create a new generator with default configuration
    pub fn new() -> Self {
        Self::with_config(ThumbnailConfig::default())
    }

    /// Create a generator with custom configuration
    pub fn with_config(config: ThumbnailConfig) -> Self {
        let cache = ThumbnailCache::new(config.clone());
        Self { config, cache }
    }

    /// Get the cache reference
    pub fn cache(&self) -> &ThumbnailCache {
        &self.cache
    }

    /// Generate a thumbnail for a device object
    ///
    /// This is a stub implementation that returns raw data.
    /// Full thumbnail generation would require the `image` crate,
    /// which was removed to reduce dependencies.
    ///
    /// To enable actual thumbnail generation, add `image` to Cargo.toml
    /// and implement the image processing logic.
    pub fn generate<C: DeviceContentTrait>(
        &self,
        content: &C,
        object: &DeviceObject,
    ) -> ThumbnailResult {
        // Check cache first
        if let Some(cached) = self.cache.get(&object.object_id) {
            return ThumbnailResult::Success(cached);
        }

        // Check if file is too large
        if object.size > self.config.max_source_size {
            return ThumbnailResult::FileTooLarge(object.size);
        }

        // Check if format is supported
        let ext = Path::new(&object.name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        let supported = matches!(
            ext.as_deref(),
            Some("jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp")
        );

        if !supported {
            // HEIC/HEIF would require additional decoding
            if matches!(ext.as_deref(), Some("heic" | "heif")) {
                return ThumbnailResult::NotAvailable;
            }
            return ThumbnailResult::UnsupportedFormat;
        }

        // Read the file data
        let _data = match content.read_file(&object.object_id) {
            Ok(d) => d,
            Err(e) => return ThumbnailResult::Error(format!("Failed to read file: {}", e)),
        };

        // Without the image crate, we can't actually generate thumbnails
        // Return the raw data as a "thumbnail" for now
        // A real implementation would use the image crate to resize

        // This is a placeholder that returns NotAvailable
        // To implement actual thumbnail generation:
        // 1. Add `image = "0.25"` to Cargo.toml
        // 2. Uncomment and use the image processing code below

        /*
        use image::io::Reader as ImageReader;
        use image::ImageFormat;

        let format = match ext.as_deref() {
            Some("jpg") | Some("jpeg") => ImageFormat::Jpeg,
            Some("png") => ImageFormat::Png,
            Some("gif") => ImageFormat::Gif,
            Some("bmp") => ImageFormat::Bmp,
            Some("webp") => ImageFormat::WebP,
            _ => return ThumbnailResult::UnsupportedFormat,
        };

        let img = match ImageReader::with_format(Cursor::new(&data), format).decode() {
            Ok(i) => i,
            Err(e) => return ThumbnailResult::Error(format!("Failed to decode image: {}", e)),
        };

        let original_width = img.width();
        let original_height = img.height();

        // Resize maintaining aspect ratio
        let thumbnail = img.thumbnail(self.config.max_width, self.config.max_height);

        // Encode as JPEG
        let mut output = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut output,
            self.config.quality,
        );

        if let Err(e) = encoder.encode_image(&thumbnail) {
            return ThumbnailResult::Error(format!("Failed to encode thumbnail: {}", e));
        }

        let thumb = Thumbnail {
            data: output,
            original_width,
            original_height,
            width: thumbnail.width(),
            height: thumbnail.height(),
            mime_type: "image/jpeg".to_string(),
            original_size: object.size,
            generated_at: Instant::now(),
        };

        // Cache it
        self.cache.put(object.object_id.clone(), thumb.clone());

        ThumbnailResult::Success(thumb)
        */

        // For now, return that thumbnail generation is not available
        // This allows the UI to show a placeholder
        ThumbnailResult::NotAvailable
    }

    /// Generate thumbnails for multiple objects (batch operation)
    pub fn generate_batch<C: DeviceContentTrait>(
        &self,
        content: &C,
        objects: &[DeviceObject],
        progress_callback: impl Fn(usize, usize),
    ) -> Vec<(String, ThumbnailResult)> {
        let total = objects.len();
        let mut results = Vec::with_capacity(total);

        for (index, object) in objects.iter().enumerate() {
            progress_callback(index + 1, total);

            // Skip non-media files
            if !object.is_media_file() {
                continue;
            }

            let result = self.generate(content, object);
            results.push((object.object_id.clone(), result));
        }

        results
    }

    /// Clear the thumbnail cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            entries: self.cache.len(),
            max_entries: self.config.cache_max_entries,
        }
    }
}

impl Default for ThumbnailGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of entries in cache
    pub entries: usize,
    /// Maximum entries allowed
    pub max_entries: usize,
}

impl CacheStats {
    /// Get cache utilization as a percentage
    pub fn utilization(&self) -> f64 {
        if self.max_entries == 0 {
            0.0
        } else {
            (self.entries as f64 / self.max_entries as f64) * 100.0
        }
    }
}

// =============================================================================
// Preview Manager
// =============================================================================

/// Manages preview items and their thumbnails for a device
pub struct PreviewManager {
    /// All preview items
    items: RwLock<Vec<PreviewItem>>,
    /// Thumbnail generator
    generator: ThumbnailGenerator,
    /// Currently selected items (by index)
    selection: RwLock<Vec<usize>>,
}

impl PreviewManager {
    /// Create a new preview manager
    pub fn new() -> Self {
        Self {
            items: RwLock::new(Vec::new()),
            generator: ThumbnailGenerator::new(),
            selection: RwLock::new(Vec::new()),
        }
    }

    /// Create with custom thumbnail configuration
    pub fn with_config(config: ThumbnailConfig) -> Self {
        Self {
            items: RwLock::new(Vec::new()),
            generator: ThumbnailGenerator::with_config(config),
            selection: RwLock::new(Vec::new()),
        }
    }

    /// Clear all items
    pub fn clear(&self) {
        self.items.write().unwrap().clear();
        self.selection.write().unwrap().clear();
        self.generator.clear_cache();
    }

    /// Add preview items from device objects
    pub fn add_items(&self, objects: Vec<(DeviceObject, String)>) {
        let mut items = self.items.write().unwrap();
        for (obj, path) in objects {
            if obj.is_media_file() {
                items.push(PreviewItem::from_device_object(&obj, path));
            }
        }
    }

    /// Get all items
    pub fn items(&self) -> Vec<PreviewItem> {
        self.items.read().unwrap().clone()
    }

    /// Get item count
    pub fn count(&self) -> usize {
        self.items.read().unwrap().len()
    }

    /// Get photos only
    pub fn photos(&self) -> Vec<PreviewItem> {
        self.items
            .read()
            .unwrap()
            .iter()
            .filter(|i| !i.is_video)
            .cloned()
            .collect()
    }

    /// Get videos only
    pub fn videos(&self) -> Vec<PreviewItem> {
        self.items
            .read()
            .unwrap()
            .iter()
            .filter(|i| i.is_video)
            .cloned()
            .collect()
    }

    /// Get item by index
    pub fn get(&self, index: usize) -> Option<PreviewItem> {
        self.items.read().unwrap().get(index).cloned()
    }

    /// Select an item by index
    pub fn select(&self, index: usize) {
        let mut selection = self.selection.write().unwrap();
        if !selection.contains(&index) {
            selection.push(index);
        }
    }

    /// Deselect an item by index
    pub fn deselect(&self, index: usize) {
        self.selection.write().unwrap().retain(|&i| i != index);
    }

    /// Toggle selection
    pub fn toggle_selection(&self, index: usize) {
        let mut selection = self.selection.write().unwrap();
        if let Some(pos) = selection.iter().position(|&i| i == index) {
            selection.remove(pos);
        } else {
            selection.push(index);
        }
    }

    /// Select all items
    pub fn select_all(&self) {
        let count = self.items.read().unwrap().len();
        *self.selection.write().unwrap() = (0..count).collect();
    }

    /// Deselect all items
    pub fn deselect_all(&self) {
        self.selection.write().unwrap().clear();
    }

    /// Get selected indices
    pub fn selected_indices(&self) -> Vec<usize> {
        self.selection.read().unwrap().clone()
    }

    /// Get selected items
    pub fn selected_items(&self) -> Vec<PreviewItem> {
        let items = self.items.read().unwrap();
        let selection = self.selection.read().unwrap();

        selection
            .iter()
            .filter_map(|&i| items.get(i).cloned())
            .collect()
    }

    /// Get selection count
    pub fn selection_count(&self) -> usize {
        self.selection.read().unwrap().len()
    }

    /// Load thumbnail for a specific item
    pub fn load_thumbnail<C: DeviceContentTrait>(&self, content: &C, index: usize) -> bool {
        // Check if already loaded or attempted
        {
            let items = self.items.read().unwrap();
            match items.get(index) {
                Some(item) => {
                    if item.thumbnail.is_some() || item.thumbnail_attempted {
                        return item.thumbnail.is_some();
                    }
                }
                None => return false,
            }
        }

        // Get item info
        let item = {
            let items = self.items.read().unwrap();
            match items.get(index) {
                Some(i) => i.clone(),
                None => return false,
            }
        };

        let obj = DeviceObject {
            object_id: item.object_id.clone(),
            parent_id: String::new(),
            name: item.name.clone(),
            is_folder: false,
            size: item.size,
            date_modified: item.date_modified.clone(),
            content_type: Some(item.content_type.clone()),
        };

        let result = self.generator.generate(content, &obj);

        // Update the item
        let mut items = self.items.write().unwrap();
        if let Some(item) = items.get_mut(index) {
            item.thumbnail_attempted = true;
            match result {
                ThumbnailResult::Success(thumb) => {
                    item.thumbnail = Some(thumb);
                    true
                }
                ThumbnailResult::Error(e) => {
                    item.thumbnail_error = Some(e);
                    false
                }
                ThumbnailResult::NotAvailable => {
                    item.thumbnail_error = Some("Thumbnail generation not available".to_string());
                    false
                }
                ThumbnailResult::UnsupportedFormat => {
                    item.thumbnail_error = Some("Unsupported format".to_string());
                    false
                }
                ThumbnailResult::FileTooLarge(size) => {
                    item.thumbnail_error = Some(format!("File too large: {} bytes", size));
                    false
                }
            }
        } else {
            false
        }
    }

    /// Get total size of all items
    pub fn total_size(&self) -> u64 {
        self.items.read().unwrap().iter().map(|i| i.size).sum()
    }

    /// Get total size of selected items
    pub fn selected_size(&self) -> u64 {
        let items = self.items.read().unwrap();
        let selection = self.selection.read().unwrap();

        selection
            .iter()
            .filter_map(|&i| items.get(i))
            .map(|item| item.size)
            .sum()
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        self.generator.cache_stats()
    }
}

impl Default for PreviewManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_config_default() {
        let config = ThumbnailConfig::default();
        assert_eq!(config.max_width, 256);
        assert_eq!(config.max_height, 256);
        assert_eq!(config.quality, 80);
        assert!(config.enable_cache);
    }

    #[test]
    fn test_thumbnail_config_presets() {
        let icon = ThumbnailConfig::icon();
        assert_eq!(icon.max_width, 64);
        assert_eq!(icon.max_height, 64);

        let medium = ThumbnailConfig::medium();
        assert_eq!(medium.max_width, 128);

        let large = ThumbnailConfig::large();
        assert_eq!(large.max_width, 512);
    }

    #[test]
    fn test_thumbnail_config_builder() {
        let config = ThumbnailConfig::default()
            .with_dimensions(320, 240)
            .with_quality(90)
            .with_cache(false, 100);

        assert_eq!(config.max_width, 320);
        assert_eq!(config.max_height, 240);
        assert_eq!(config.quality, 90);
        assert!(!config.enable_cache);
        assert_eq!(config.cache_max_entries, 100);
    }

    #[test]
    fn test_thumbnail_cache_basic() {
        let cache = ThumbnailCache::new(ThumbnailConfig::default());
        assert!(cache.is_empty());

        let thumb = Thumbnail {
            data: vec![1, 2, 3],
            original_width: 100,
            original_height: 100,
            width: 64,
            height: 64,
            mime_type: "image/jpeg".to_string(),
            original_size: 1000,
            generated_at: Instant::now(),
        };

        cache.put("test-id".to_string(), thumb.clone());
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get("test-id");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, vec![1, 2, 3]);

        cache.remove("test-id");
        assert!(cache.is_empty());
    }

    #[test]
    fn test_preview_item_from_device_object() {
        let obj = DeviceObject {
            object_id: "obj-123".to_string(),
            parent_id: "parent".to_string(),
            name: "IMG_0001.jpg".to_string(),
            is_folder: false,
            size: 1024 * 1024,
            date_modified: Some("2024-01-01".to_string()),
            content_type: Some("image/jpeg".to_string()),
        };

        let item = PreviewItem::from_device_object(&obj, "/DCIM/100APPLE".to_string());

        assert_eq!(item.object_id, "obj-123");
        assert_eq!(item.name, "IMG_0001.jpg");
        assert!(!item.is_video);
        assert!(item.supports_thumbnail());
        assert_eq!(item.formatted_size(), "1.0 MB");
        assert_eq!(item.extension(), Some("jpg".to_string()));
    }

    #[test]
    fn test_preview_item_video_detection() {
        let obj = DeviceObject {
            object_id: "obj-456".to_string(),
            parent_id: "parent".to_string(),
            name: "VID_0001.mp4".to_string(),
            is_folder: false,
            size: 10 * 1024 * 1024,
            date_modified: None,
            content_type: Some("video/mp4".to_string()),
        };

        let item = PreviewItem::from_device_object(&obj, "/DCIM/100APPLE".to_string());

        assert!(item.is_video);
        assert!(!item.supports_thumbnail()); // Videos don't support thumbnails
    }

    #[test]
    fn test_preview_manager_basic() {
        let manager = PreviewManager::new();
        assert_eq!(manager.count(), 0);

        let obj = DeviceObject {
            object_id: "obj-1".to_string(),
            parent_id: "parent".to_string(),
            name: "IMG_0001.jpg".to_string(),
            is_folder: false,
            size: 1024,
            date_modified: None,
            content_type: Some("image/jpeg".to_string()),
        };

        manager.add_items(vec![(obj, "/DCIM".to_string())]);
        assert_eq!(manager.count(), 1);

        manager.select(0);
        assert_eq!(manager.selection_count(), 1);

        manager.toggle_selection(0);
        assert_eq!(manager.selection_count(), 0);

        manager.select_all();
        assert_eq!(manager.selection_count(), 1);

        manager.deselect_all();
        assert_eq!(manager.selection_count(), 0);
    }

    #[test]
    fn test_cache_stats() {
        let manager = PreviewManager::new();
        let stats = manager.cache_stats();

        assert_eq!(stats.entries, 0);
        assert!(stats.max_entries > 0);
        assert_eq!(stats.utilization(), 0.0);
    }

    #[test]
    fn test_thumbnail_freshness() {
        let thumb = Thumbnail {
            data: vec![],
            original_width: 100,
            original_height: 100,
            width: 64,
            height: 64,
            mime_type: "image/jpeg".to_string(),
            original_size: 1000,
            generated_at: Instant::now(),
        };

        assert!(thumb.is_fresh(Duration::from_secs(60)));
        // Can't easily test expiration without waiting
    }

    #[test]
    fn test_thumbnail_aspect_ratio() {
        let thumb = Thumbnail {
            data: vec![],
            original_width: 1920,
            original_height: 1080,
            width: 256,
            height: 144,
            mime_type: "image/jpeg".to_string(),
            original_size: 1000,
            generated_at: Instant::now(),
        };

        let ratio = thumb.original_aspect_ratio();
        assert!((ratio - 1.777).abs() < 0.01); // 16:9 ratio
    }
}

//! Configuration module for the photo extraction tool
//!
//! Supports loading configuration from a TOML file.
//! Configuration is stored in a standard location:
//! - Windows: %APPDATA%\photo_extraction_tool\config.toml
//! - Linux/macOS: ~/.config/photo_extraction_tool/config.toml

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Application name used for config directory
const APP_NAME: &str = "photo_extraction_tool";

/// Default config file name
const CONFIG_FILE_NAME: &str = "config.toml";

/// Get the standard configuration directory for the application.
///
/// Returns:
/// - Windows: %APPDATA%\photo_extraction_tool
/// - Linux/macOS: ~/.config/photo_extraction_tool
pub fn get_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|appdata| PathBuf::from(appdata).join(APP_NAME))
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".config").join(APP_NAME))
    }
}

/// Get the standard configuration file path.
///
/// Returns the full path to the config file in the standard location.
pub fn get_config_path() -> Option<PathBuf> {
    get_config_dir().map(|dir| dir.join(CONFIG_FILE_NAME))
}

/// Ensure the configuration directory exists.
///
/// Creates the directory and all parent directories if they don't exist.
pub fn ensure_config_dir() -> Result<PathBuf, ConfigError> {
    let config_dir = get_config_dir().ok_or_else(|| ConfigError::ConfigDirNotFound)?;

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| ConfigError::WriteError(config_dir.clone(), e.to_string()))?;
    }

    Ok(config_dir)
}

/// Initialize the configuration file if it doesn't exist.
///
/// Creates the config directory and copies the default config template.
/// Returns the path to the config file.
pub fn init_config() -> Result<PathBuf, ConfigError> {
    let config_dir = ensure_config_dir()?;
    let config_path = config_dir.join(CONFIG_FILE_NAME);

    if !config_path.exists() {
        let default_config = Config::generate_default_config();
        fs::write(&config_path, default_config)
            .map_err(|e| ConfigError::WriteError(config_path.clone(), e.to_string()))?;
    }

    Ok(config_path)
}

/// Open the configuration file in the default application.
///
/// This will typically open the file in Notepad on Windows,
/// or the default text editor on other platforms.
pub fn open_config_in_editor() -> Result<PathBuf, ConfigError> {
    // Ensure config exists first
    let config_path = init_config()?;

    // Open with default application
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", config_path.to_str().unwrap_or("")])
            .spawn()
            .map_err(|e| ConfigError::OpenError(config_path.clone(), e.to_string()))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&config_path)
            .spawn()
            .map_err(|e| ConfigError::OpenError(config_path.clone(), e.to_string()))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&config_path)
            .spawn()
            .map_err(|e| ConfigError::OpenError(config_path.clone(), e.to_string()))?;
    }

    Ok(config_path)
}

/// Action to take when a duplicate is found
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DuplicateAction {
    /// Skip the duplicate file (don't extract)
    #[default]
    Skip,
    /// Overwrite the existing file with the new one
    Overwrite,
    /// Keep both by renaming the new file with a suffix
    Rename,
}

/// Main configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Output settings
    pub output: OutputConfig,

    /// Device settings
    pub device: DeviceConfig,

    /// Extraction settings
    pub extraction: ExtractionConfig,

    /// Android-specific extraction settings
    pub android: AndroidConfig,

    /// Logging settings
    pub logging: LoggingConfig,

    /// Duplicate detection settings
    pub duplicate_detection: DuplicateDetectionConfig,

    /// Tracking settings
    pub tracking: TrackingConfig,

    /// Device profiles (maps device IDs to their settings)
    #[serde(default)]
    pub device_profiles: DeviceProfilesConfig,
}

/// Device profiles configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceProfilesConfig {
    /// Enable device profiles feature
    pub enabled: bool,

    /// Base backup folder where device subfolders will be created
    pub backup_base_folder: PathBuf,

    /// Path to the profiles database file
    pub profiles_file: PathBuf,

    /// Individual device profiles (device_id -> profile)
    #[serde(default)]
    pub profiles: HashMap<String, DeviceProfile>,
}

/// A single device profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    /// User-friendly name for this device
    pub name: String,

    /// Output folder for this device (relative to backup_base_folder or absolute)
    pub output_folder: String,

    /// Device manufacturer (for display)
    #[serde(default)]
    pub manufacturer: String,

    /// Device model (for display)
    #[serde(default)]
    pub model: String,

    /// First time this device was seen
    #[serde(default)]
    pub first_seen: Option<String>,

    /// Last time this device was used
    #[serde(default)]
    pub last_seen: Option<String>,
}

/// Output directory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Output directory for extracted photos
    pub directory: PathBuf,

    /// Whether to preserve folder structure from device
    pub preserve_structure: bool,

    /// Whether to skip existing files
    pub skip_existing: bool,

    /// Organize photos by date (YYYY/MM folders)
    pub organize_by_date: bool,

    /// Create subfolder with device name
    pub subfolder_by_device: bool,
}

/// Device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceConfig {
    /// Specific device ID to use (optional)
    pub device_id: Option<String>,

    /// Device name filter (partial match)
    pub device_name_filter: Option<String>,

    /// Only show Apple devices
    pub apple_only: bool,
}

/// Extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtractionConfig {
    /// Only extract from DCIM folder
    pub dcim_only: bool,

    /// File extensions to include (empty = all supported)
    pub include_extensions: Vec<String>,

    /// File extensions to exclude
    pub exclude_extensions: Vec<String>,

    /// Minimum file size in bytes (0 = no minimum)
    pub min_file_size: u64,

    /// Maximum file size in bytes (0 = no maximum)
    pub max_file_size: u64,

    /// Include photos
    pub include_photos: bool,

    /// Include videos
    pub include_videos: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level: error, warn, info, debug, trace
    pub level: String,

    /// Log to file
    pub log_to_file: bool,

    /// Log file path
    pub log_file: PathBuf,
}

/// Duplicate detection configuration
///
/// Uses SHA256 cryptographic hashing for exact-match duplicate detection.
/// This is reliable, fast, and works for all file types (photos and videos).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DuplicateDetectionConfig {
    /// Enable duplicate detection during extraction
    pub enabled: bool,

    /// Folder(s) to compare against for duplicates
    /// Files in these folders will be indexed and compared against incoming files
    pub comparison_folders: Vec<PathBuf>,

    /// Cache the hash index for faster subsequent runs
    pub cache_enabled: bool,

    /// Path to cache file (JSON format)
    pub cache_file: PathBuf,

    /// Action to take when a duplicate is found
    pub duplicate_action: DuplicateAction,

    /// Scan comparison folders recursively (include subdirectories)
    pub recursive: bool,

    /// Only index media files (photos/videos) vs all files
    pub media_only: bool,
}

/// Tracking configuration for remembering device and extraction state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackingConfig {
    /// Enable tracking of extraction state
    pub enabled: bool,

    /// Path to the hidden tracking file (stored in output directory)
    pub tracking_filename: String,

    /// Track individual files that have been extracted
    pub track_extracted_files: bool,
}

/// Android-specific extraction configuration
///
/// These settings control how photos are extracted from Android devices.
/// Android devices have a different folder structure than iOS devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AndroidConfig {
    /// Preserve the folder structure from the Android device
    /// When true: maintains Camera/, Screenshots/, Pictures/ structure
    /// When false: extracts all photos to a flat directory
    pub preserve_structure: bool,

    /// Include photos from DCIM/Camera folder (main camera photos)
    pub include_camera: bool,

    /// Include screenshots from DCIM/Screenshots or Pictures/Screenshots
    pub include_screenshots: bool,

    /// Include photos from Pictures folder
    pub include_pictures: bool,

    /// Include images from Download folder
    pub include_downloads: bool,

    /// Exclude Android thumbnail/cache folders (.thumbnails, .trash, etc.)
    /// This is enabled by default and recommended to keep enabled
    pub exclude_cache_folders: bool,

    // ── App-specific media folder support ──────────────────────────────────
    /// Include WhatsApp media (images and videos from WhatsApp/Media)
    #[serde(default)]
    pub include_whatsapp: bool,

    /// Include Telegram media (images and videos from Telegram folder)
    #[serde(default)]
    pub include_telegram: bool,

    /// Include Instagram media (saved/cached images)
    #[serde(default)]
    pub include_instagram: bool,

    /// Include Facebook/Messenger media
    #[serde(default)]
    pub include_facebook: bool,

    /// Include Snapchat memories and media
    #[serde(default)]
    pub include_snapchat: bool,

    /// Include TikTok saved videos
    #[serde(default)]
    pub include_tiktok: bool,

    /// Include Signal messenger media
    #[serde(default)]
    pub include_signal: bool,

    /// Include Viber messenger media
    #[serde(default)]
    pub include_viber: bool,

    // ── Custom folder configuration ────────────────────────────────────────
    /// List of additional folders to include (relative to Internal Storage)
    /// Example: ["WhatsApp/Media/WhatsApp Images", "Telegram/Telegram Images"]
    pub additional_folders: Vec<String>,

    /// List of folder names to exclude (will be skipped during extraction)
    /// Default excludes: .thumbnails, .trash, .cache
    pub exclude_folders: Vec<String>,
}

/// Known folder paths for popular Android apps
/// These are relative to Internal Storage
/// Note: Many manufacturers (Xiaomi, Samsung, etc.) use different folder structures,
/// and Android 11+ uses scoped storage under Android/media/
/// so we include multiple alternative paths for each app.
pub mod app_folders {
    // =========================================================================
    // WHATSAPP (Personal)
    // =========================================================================
    /// WhatsApp media folders - standard location (Android 10 and earlier)
    pub const WHATSAPP_IMAGES: &str = "WhatsApp/Media/WhatsApp Images";
    pub const WHATSAPP_VIDEO: &str = "WhatsApp/Media/WhatsApp Video";
    pub const WHATSAPP_ANIMATED_GIFS: &str = "WhatsApp/Media/WhatsApp Animated Gifs";
    pub const WHATSAPP_DOCUMENTS: &str = "WhatsApp/Media/WhatsApp Documents";
    /// WhatsApp alternative locations (Xiaomi, some Samsung devices)
    pub const WHATSAPP_PICTURES: &str = "Pictures/WhatsApp";
    pub const WHATSAPP_DCIM: &str = "DCIM/WhatsApp";
    /// WhatsApp Android 11+ scoped storage location
    pub const WHATSAPP_SCOPED_IMAGES: &str =
        "Android/media/com.whatsapp/WhatsApp/Media/WhatsApp Images";
    pub const WHATSAPP_SCOPED_VIDEO: &str =
        "Android/media/com.whatsapp/WhatsApp/Media/WhatsApp Video";
    pub const WHATSAPP_SCOPED_GIFS: &str =
        "Android/media/com.whatsapp/WhatsApp/Media/WhatsApp Animated Gifs";

    // =========================================================================
    // WHATSAPP BUSINESS
    // =========================================================================
    /// WhatsApp Business - standard location
    pub const WHATSAPP_BIZ_IMAGES: &str = "WhatsApp Business/Media/WhatsApp Business Images";
    pub const WHATSAPP_BIZ_VIDEO: &str = "WhatsApp Business/Media/WhatsApp Business Video";
    pub const WHATSAPP_BIZ_GIFS: &str = "WhatsApp Business/Media/WhatsApp Business Animated Gifs";
    /// WhatsApp Business - alternative locations
    pub const WHATSAPP_BIZ_PICTURES: &str = "Pictures/WhatsApp Business";
    /// WhatsApp Business Android 11+ scoped storage
    pub const WHATSAPP_BIZ_SCOPED_IMAGES: &str =
        "Android/media/com.whatsapp.w4b/WhatsApp Business/Media/WhatsApp Business Images";
    pub const WHATSAPP_BIZ_SCOPED_VIDEO: &str =
        "Android/media/com.whatsapp.w4b/WhatsApp Business/Media/WhatsApp Business Video";

    // =========================================================================
    // TELEGRAM
    // =========================================================================
    /// Telegram media folders - standard location
    pub const TELEGRAM_IMAGES: &str = "Telegram/Telegram Images";
    pub const TELEGRAM_VIDEO: &str = "Telegram/Telegram Video";
    pub const TELEGRAM_DOCUMENTS: &str = "Telegram/Telegram Documents";
    /// Telegram alternative locations
    pub const TELEGRAM_PICTURES: &str = "Pictures/Telegram";
    /// Telegram Android 11+ scoped storage
    pub const TELEGRAM_SCOPED: &str =
        "Android/media/org.telegram.messenger/Telegram/Telegram Images";
    pub const TELEGRAM_SCOPED_VIDEO: &str =
        "Android/media/org.telegram.messenger/Telegram/Telegram Video";

    // =========================================================================
    // INSTAGRAM
    // =========================================================================
    /// Instagram folders (may vary by device/version)
    pub const INSTAGRAM: &str = "Pictures/Instagram";
    pub const INSTAGRAM_ALT: &str = "DCIM/Instagram";
    pub const INSTAGRAM_MOVIES: &str = "Movies/Instagram";
    /// Instagram Android 11+ scoped storage
    pub const INSTAGRAM_SCOPED: &str = "Android/media/com.instagram.android/Instagram";

    // =========================================================================
    // FACEBOOK / MESSENGER
    // =========================================================================
    /// Facebook/Messenger folders
    pub const FACEBOOK: &str = "Pictures/Facebook";
    pub const MESSENGER: &str = "Pictures/Messenger";
    pub const FACEBOOK_DCIM: &str = "DCIM/Facebook";
    /// Facebook Android 11+ scoped storage
    pub const FACEBOOK_SCOPED: &str = "Android/media/com.facebook.katana/Facebook";
    pub const MESSENGER_SCOPED: &str = "Android/media/com.facebook.orca/Messenger";

    // =========================================================================
    // SNAPCHAT
    // =========================================================================
    /// Snapchat folders
    pub const SNAPCHAT: &str = "Snapchat";
    pub const SNAPCHAT_ALT: &str = "Pictures/Snapchat";
    pub const SNAPCHAT_DCIM: &str = "DCIM/Snapchat";
    /// Snapchat Android 11+ scoped storage
    pub const SNAPCHAT_SCOPED: &str = "Android/media/com.snapchat.android/Snapchat";

    // =========================================================================
    // TIKTOK
    // =========================================================================
    /// TikTok folders
    pub const TIKTOK: &str = "Pictures/TikTok";
    pub const TIKTOK_ALT: &str = "DCIM/TikTok";
    pub const TIKTOK_MOVIES: &str = "Movies/TikTok";
    /// TikTok Android 11+ scoped storage
    pub const TIKTOK_SCOPED: &str = "Android/media/com.zhiliaoapp.musically/TikTok";

    // =========================================================================
    // SIGNAL
    // =========================================================================
    /// Signal messenger folders
    pub const SIGNAL: &str = "Signal/Signal Photos";
    pub const SIGNAL_VIDEO: &str = "Signal/Signal Video";
    pub const SIGNAL_PICTURES: &str = "Pictures/Signal";
    /// Signal Android 11+ scoped storage
    pub const SIGNAL_SCOPED: &str = "Android/media/org.thoughtcrime.securesms/Signal";

    // =========================================================================
    // VIBER
    // =========================================================================
    /// Viber messenger folders
    pub const VIBER_IMAGES: &str = "Viber/media/Viber Images";
    pub const VIBER_VIDEO: &str = "Viber/media/Viber Video";
    pub const VIBER_PICTURES: &str = "Pictures/Viber";
    /// Viber Android 11+ scoped storage
    pub const VIBER_SCOPED: &str = "Android/media/com.viber.voip/Viber";

    /// Returns all known folder paths for a given app
    /// Includes standard, alternative, and Android 11+ scoped storage locations
    pub fn get_app_folders(app: &str) -> Vec<&'static str> {
        match app.to_lowercase().as_str() {
            "whatsapp" => vec![
                // Standard locations
                WHATSAPP_IMAGES,
                WHATSAPP_VIDEO,
                WHATSAPP_ANIMATED_GIFS,
                WHATSAPP_DOCUMENTS,
                // Alternative locations (Xiaomi, etc.)
                WHATSAPP_PICTURES,
                WHATSAPP_DCIM,
                // Android 11+ scoped storage
                WHATSAPP_SCOPED_IMAGES,
                WHATSAPP_SCOPED_VIDEO,
                WHATSAPP_SCOPED_GIFS,
                // WhatsApp Business - all locations
                WHATSAPP_BIZ_IMAGES,
                WHATSAPP_BIZ_VIDEO,
                WHATSAPP_BIZ_GIFS,
                WHATSAPP_BIZ_PICTURES,
                WHATSAPP_BIZ_SCOPED_IMAGES,
                WHATSAPP_BIZ_SCOPED_VIDEO,
            ],
            "telegram" => vec![
                TELEGRAM_IMAGES,
                TELEGRAM_VIDEO,
                TELEGRAM_DOCUMENTS,
                TELEGRAM_PICTURES,
                TELEGRAM_SCOPED,
                TELEGRAM_SCOPED_VIDEO,
            ],
            "instagram" => vec![INSTAGRAM, INSTAGRAM_ALT, INSTAGRAM_MOVIES, INSTAGRAM_SCOPED],
            "facebook" => vec![
                FACEBOOK,
                MESSENGER,
                FACEBOOK_DCIM,
                FACEBOOK_SCOPED,
                MESSENGER_SCOPED,
            ],
            "snapchat" => vec![SNAPCHAT, SNAPCHAT_ALT, SNAPCHAT_DCIM, SNAPCHAT_SCOPED],
            "tiktok" => vec![TIKTOK, TIKTOK_ALT, TIKTOK_MOVIES, TIKTOK_SCOPED],
            "signal" => vec![SIGNAL, SIGNAL_VIDEO, SIGNAL_PICTURES, SIGNAL_SCOPED],
            "viber" => vec![VIBER_IMAGES, VIBER_VIDEO, VIBER_PICTURES, VIBER_SCOPED],
            _ => vec![],
        }
    }
}

impl Default for AndroidConfig {
    fn default() -> Self {
        Self {
            preserve_structure: true,
            include_camera: true,
            include_screenshots: true,
            include_pictures: true,
            include_downloads: false,
            exclude_cache_folders: true,
            // App-specific defaults (all disabled by default)
            include_whatsapp: false,
            include_telegram: false,
            include_instagram: false,
            include_facebook: false,
            include_snapchat: false,
            include_tiktok: false,
            include_signal: false,
            include_viber: false,
            // Custom folders
            additional_folders: Vec::new(),
            exclude_folders: vec![
                ".thumbnails".to_string(),
                ".trash".to_string(),
                ".cache".to_string(),
                ".nomedia".to_string(),
            ],
        }
    }
}

impl AndroidConfig {
    /// Get all enabled app folder paths based on current configuration
    pub fn get_enabled_app_folders(&self) -> Vec<&'static str> {
        let mut folders = Vec::new();

        if self.include_whatsapp {
            folders.extend(app_folders::get_app_folders("whatsapp"));
        }
        if self.include_telegram {
            folders.extend(app_folders::get_app_folders("telegram"));
        }
        if self.include_instagram {
            folders.extend(app_folders::get_app_folders("instagram"));
        }
        if self.include_facebook {
            folders.extend(app_folders::get_app_folders("facebook"));
        }
        if self.include_snapchat {
            folders.extend(app_folders::get_app_folders("snapchat"));
        }
        if self.include_tiktok {
            folders.extend(app_folders::get_app_folders("tiktok"));
        }
        if self.include_signal {
            folders.extend(app_folders::get_app_folders("signal"));
        }
        if self.include_viber {
            folders.extend(app_folders::get_app_folders("viber"));
        }

        folders
    }

    /// Check if any app-specific folder extraction is enabled
    pub fn has_app_folders_enabled(&self) -> bool {
        self.include_whatsapp
            || self.include_telegram
            || self.include_instagram
            || self.include_facebook
            || self.include_snapchat
            || self.include_tiktok
            || self.include_signal
            || self.include_viber
    }
}

impl Default for DeviceProfilesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backup_base_folder: PathBuf::new(), // Empty = needs setup
            profiles_file: PathBuf::from("./.device_profiles.json"),
            profiles: HashMap::new(),
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            directory: PathBuf::new(), // Empty = needs setup
            preserve_structure: true,
            skip_existing: true,
            organize_by_date: false,
            subfolder_by_device: false,
        }
    }
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            device_id: None,
            device_name_filter: None,
            apple_only: true,
        }
    }
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            dcim_only: true,
            include_extensions: vec![],
            exclude_extensions: vec![],
            min_file_size: 0,
            max_file_size: 0,
            include_photos: true,
            include_videos: true,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_to_file: false,
            log_file: PathBuf::from("./photo_extraction.log"),
        }
    }
}

impl Default for DuplicateDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            comparison_folders: vec![],
            cache_enabled: true,
            cache_file: PathBuf::from("./.duplicate_cache.json"),
            duplicate_action: DuplicateAction::Skip,
            recursive: true,
            media_only: true,
        }
    }
}

impl DuplicateDetectionConfig {
    /// Convert to the duplicate detector's config format
    pub fn to_detector_config(&self) -> crate::duplicate::DuplicateConfig {
        crate::duplicate::DuplicateConfig {
            comparison_folders: self.comparison_folders.clone(),
            cache_enabled: self.cache_enabled,
            cache_file: self.cache_file.clone(),
            recursive: self.recursive,
            follow_symlinks: false,
            min_file_size: 0,
            max_file_size: 0,
            media_only: self.media_only,
        }
    }
}

impl Default for TrackingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tracking_filename: ".photo_extraction_state.json".to_string(),
            track_extracted_files: true,
        }
    }
}

impl Config {
    /// Check if initial setup is required
    ///
    /// Setup is needed if:
    /// - Device profiles are enabled but backup_base_folder is empty
    /// - Or output directory is empty and profiles are disabled
    pub fn needs_setup(&self) -> bool {
        if self.device_profiles.enabled {
            // Profiles enabled: need backup_base_folder
            self.device_profiles
                .backup_base_folder
                .as_os_str()
                .is_empty()
        } else {
            // Profiles disabled: need output directory
            self.output.directory.as_os_str().is_empty()
        }
    }

    /// Set up the backup directory (for first-run setup)
    ///
    /// This configures both the device profiles backup folder and
    /// the fallback output directory.
    pub fn set_backup_directory(&mut self, dir: PathBuf) {
        self.device_profiles.backup_base_folder = dir.clone();
        self.output.directory = dir;
    }

    /// Get the effective output directory
    ///
    /// Returns the backup base folder if profiles are enabled,
    /// otherwise returns the output directory.
    pub fn get_effective_output_dir(&self) -> &Path {
        if self.device_profiles.enabled {
            &self.device_profiles.backup_base_folder
        } else {
            &self.output.directory
        }
    }

    /// Load configuration from a TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.to_path_buf()));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::ReadError(path.to_path_buf(), e.to_string()))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(path.to_path_buf(), e.to_string()))?;

        Ok(config)
    }

    /// Load configuration from default locations
    ///
    /// Search order:
    /// 1. ./config.toml (current directory - for development/override)
    /// 2. ./photo_extraction.toml (current directory - alternative name)
    /// 3. Standard config location (%APPDATA%\photo_extraction_tool\config.toml on Windows)
    ///
    /// If no config file is found, returns default configuration.
    pub fn load_default() -> Result<Self, ConfigError> {
        // First check local directory (allows for project-specific overrides)
        let local_paths = [
            PathBuf::from("./config.toml"),
            PathBuf::from("./photo_extraction.toml"),
        ];

        for path in &local_paths {
            if path.exists() {
                return Self::load(path);
            }
        }

        // Then check standard config location
        if let Some(config_path) = get_config_path() {
            if config_path.exists() {
                return Self::load(&config_path);
            }
        }

        // No config file found, use defaults
        Ok(Self::default())
    }

    /// Get the path where the config file is (or would be) located.
    ///
    /// Returns the first existing config file path, or the standard location if none exists.
    pub fn get_active_config_path() -> PathBuf {
        let local_paths = [
            PathBuf::from("./config.toml"),
            PathBuf::from("./photo_extraction.toml"),
        ];

        for path in &local_paths {
            if path.exists() {
                return path.clone();
            }
        }

        // Return standard location
        get_config_path().unwrap_or_else(|| PathBuf::from("./config.toml"))
    }

    /// Save configuration to a TOML file
    ///
    /// Reserved for future use - allows saving modified configuration.
    #[allow(dead_code)]
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::SerializeError(e.to_string()))?;

        fs::write(path.as_ref(), content)
            .map_err(|e| ConfigError::WriteError(path.as_ref().to_path_buf(), e.to_string()))?;

        Ok(())
    }

    /// Generate a default config file with comments
    /// This uses the example config file to ensure it stays up to date
    pub fn generate_default_config() -> String {
        include_str!("../../config.example.toml").to_string()
    }
}

/// Configuration error types
///
/// Some variants are reserved for future use (save functionality).
#[derive(Debug)]
#[allow(dead_code)]
pub enum ConfigError {
    /// Configuration file was not found at the specified path
    FileNotFound(PathBuf),
    /// Failed to read the configuration file
    ReadError(PathBuf, String),
    /// Failed to parse the configuration file (invalid TOML)
    ParseError(PathBuf, String),
    /// Failed to serialize configuration to TOML
    SerializeError(String),
    /// Failed to write configuration file
    WriteError(PathBuf, String),
    /// Could not determine config directory
    ConfigDirNotFound,
    /// Failed to open config file in editor
    OpenError(PathBuf, String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => {
                write!(f, "Configuration file not found: {}", path.display())
            }
            ConfigError::ReadError(path, err) => {
                write!(
                    f,
                    "Failed to read config file '{}': {}",
                    path.display(),
                    err
                )
            }
            ConfigError::ParseError(path, err) => {
                write!(
                    f,
                    "Failed to parse config file '{}': {}",
                    path.display(),
                    err
                )
            }
            ConfigError::SerializeError(err) => {
                write!(f, "Failed to serialize configuration: {}", err)
            }
            ConfigError::WriteError(path, err) => {
                write!(
                    f,
                    "Failed to write config file '{}': {}",
                    path.display(),
                    err
                )
            }
            ConfigError::ConfigDirNotFound => {
                write!(f, "Could not determine configuration directory")
            }
            ConfigError::OpenError(path, err) => {
                write!(
                    f,
                    "Failed to open config file '{}': {}",
                    path.display(),
                    err
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_config_defaults() {
        let config = AndroidConfig::default();
        assert!(config.preserve_structure);
        assert!(config.include_camera);
        assert!(config.include_screenshots);
        assert!(config.include_pictures);
        assert!(!config.include_downloads);
        assert!(config.exclude_cache_folders);
        // All app folders disabled by default
        assert!(!config.include_whatsapp);
        assert!(!config.include_telegram);
        assert!(!config.include_instagram);
        assert!(!config.include_facebook);
        assert!(!config.include_snapchat);
        assert!(!config.include_tiktok);
        assert!(!config.include_signal);
        assert!(!config.include_viber);
    }

    #[test]
    fn test_has_app_folders_enabled_none() {
        let config = AndroidConfig::default();
        assert!(!config.has_app_folders_enabled());
    }

    #[test]
    fn test_has_app_folders_enabled_whatsapp() {
        let mut config = AndroidConfig::default();
        config.include_whatsapp = true;
        assert!(config.has_app_folders_enabled());
    }

    #[test]
    fn test_has_app_folders_enabled_multiple() {
        let mut config = AndroidConfig::default();
        config.include_telegram = true;
        config.include_signal = true;
        assert!(config.has_app_folders_enabled());
    }

    #[test]
    fn test_get_enabled_app_folders_empty() {
        let config = AndroidConfig::default();
        let folders = config.get_enabled_app_folders();
        assert!(folders.is_empty());
    }

    #[test]
    fn test_get_enabled_app_folders_whatsapp() {
        let mut config = AndroidConfig::default();
        config.include_whatsapp = true;
        let folders = config.get_enabled_app_folders();
        // WhatsApp personal: 9 paths + WhatsApp Business: 6 paths = 15 total
        assert_eq!(folders.len(), 15);
        // Check some key paths exist
        assert!(folders.contains(&app_folders::WHATSAPP_IMAGES));
        assert!(folders.contains(&app_folders::WHATSAPP_VIDEO));
        assert!(folders.contains(&app_folders::WHATSAPP_PICTURES));
        assert!(folders.contains(&app_folders::WHATSAPP_SCOPED_IMAGES));
        assert!(folders.contains(&app_folders::WHATSAPP_BIZ_IMAGES));
        assert!(folders.contains(&app_folders::WHATSAPP_BIZ_SCOPED_IMAGES));
    }

    #[test]
    fn test_get_enabled_app_folders_telegram() {
        let mut config = AndroidConfig::default();
        config.include_telegram = true;
        let folders = config.get_enabled_app_folders();
        assert_eq!(folders.len(), 6); // Standard + alternative + scoped
        assert!(folders.contains(&app_folders::TELEGRAM_IMAGES));
        assert!(folders.contains(&app_folders::TELEGRAM_VIDEO));
        assert!(folders.contains(&app_folders::TELEGRAM_PICTURES));
        assert!(folders.contains(&app_folders::TELEGRAM_SCOPED));
    }

    #[test]
    fn test_get_enabled_app_folders_multiple_apps() {
        let mut config = AndroidConfig::default();
        config.include_whatsapp = true;
        config.include_telegram = true;
        config.include_signal = true;
        let folders = config.get_enabled_app_folders();
        // WhatsApp: 15, Telegram: 6, Signal: 4
        assert_eq!(folders.len(), 25);
    }

    #[test]
    fn test_get_enabled_app_folders_all_apps() {
        let mut config = AndroidConfig::default();
        config.include_whatsapp = true;
        config.include_telegram = true;
        config.include_instagram = true;
        config.include_facebook = true;
        config.include_snapchat = true;
        config.include_tiktok = true;
        config.include_signal = true;
        config.include_viber = true;
        let folders = config.get_enabled_app_folders();
        // WhatsApp: 15, Telegram: 6, Instagram: 4, Facebook: 5, Snapchat: 4, TikTok: 4, Signal: 4, Viber: 4
        assert_eq!(folders.len(), 46);
    }

    #[test]
    fn test_app_folders_get_app_folders_unknown() {
        let folders = app_folders::get_app_folders("unknown_app");
        assert!(folders.is_empty());
    }

    #[test]
    fn test_app_folders_constants() {
        // Verify key folder paths are correctly defined
        assert_eq!(
            app_folders::WHATSAPP_IMAGES,
            "WhatsApp/Media/WhatsApp Images"
        );
        assert_eq!(
            app_folders::WHATSAPP_SCOPED_IMAGES,
            "Android/media/com.whatsapp/WhatsApp/Media/WhatsApp Images"
        );
        assert_eq!(
            app_folders::WHATSAPP_BIZ_IMAGES,
            "WhatsApp Business/Media/WhatsApp Business Images"
        );
        assert_eq!(app_folders::TELEGRAM_IMAGES, "Telegram/Telegram Images");
        assert_eq!(
            app_folders::TELEGRAM_SCOPED,
            "Android/media/org.telegram.messenger/Telegram/Telegram Images"
        );
        assert_eq!(app_folders::INSTAGRAM, "Pictures/Instagram");
        assert_eq!(app_folders::SIGNAL, "Signal/Signal Photos");
        assert_eq!(app_folders::VIBER_IMAGES, "Viber/media/Viber Images");
    }
}

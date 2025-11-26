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
    let config_dir = get_config_dir()
        .ok_or_else(|| ConfigError::ConfigDirNotFound)?;

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

/// Hash algorithm for duplicate detection
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    /// Perceptual hash - compares visual content (works across formats)
    #[default]
    Perceptual,
    /// EXIF-based comparison (date taken, dimensions)
    Exif,
    /// File size based (fast but less accurate)
    Size,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DuplicateDetectionConfig {
    /// Enable duplicate detection
    pub enabled: bool,

    /// Folder(s) to compare against for duplicates
    pub comparison_folders: Vec<PathBuf>,

    /// Hash algorithm to use for comparison
    pub hash_algorithm: HashAlgorithm,

    /// Hash size for perceptual hashing (8, 16, or 32)
    pub hash_size: u32,

    /// Hamming distance threshold for perceptual hash matching (0 = exact, higher = more lenient)
    pub similarity_threshold: u32,

    /// Cache the hash index for faster subsequent runs
    pub cache_index: bool,

    /// Path to cache file
    pub cache_file: PathBuf,

    /// Action to take when a duplicate is found
    pub duplicate_action: DuplicateAction,
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

impl Default for DeviceProfilesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backup_base_folder: PathBuf::from("D:/Photos"),
            profiles_file: PathBuf::from("./.device_profiles.json"),
            profiles: HashMap::new(),
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("./extracted_photos"),
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
            hash_algorithm: HashAlgorithm::Perceptual,
            hash_size: 8,
            similarity_threshold: 5,
            cache_index: true,
            cache_file: PathBuf::from("./photo_hash_cache.bin"),
            duplicate_action: DuplicateAction::Skip,
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
        include_str!("../config.example.toml").to_string()
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

//! First-run setup wizard
//!
//! This module handles the initial configuration of the photo extraction tool.
//! It prompts the user for essential settings like the backup directory and
//! sets up device profiles. This is designed to work with both CLI and future UI.

use crate::core::config::{ensure_config_dir, get_config_path, Config, TrackingConfig};
use crate::core::tracking::scan_for_profiles;
use crate::device::{self, DeviceInfo, DeviceManagerTrait, ProfileManager};
use log::info;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Result of the setup process
#[derive(Debug)]
pub struct SetupResult {
    /// The configured backup directory
    pub backup_directory: PathBuf,
    /// Whether setup was completed successfully
    pub completed: bool,
    /// The updated configuration
    pub config: Config,
}

/// Setup configuration options (for programmatic setup from UI)
#[derive(Debug, Clone)]
pub struct SetupOptions {
    /// The backup directory to use
    pub backup_directory: PathBuf,
    /// Whether to enable device profiles (default: true)
    pub enable_profiles: bool,
}

impl Default for SetupOptions {
    fn default() -> Self {
        Self {
            backup_directory: PathBuf::new(),
            enable_profiles: true,
        }
    }
}

/// Run the CLI setup wizard
///
/// This interactive wizard guides the user through initial setup:
/// 1. Welcomes the user
/// 2. Asks for the backup directory
/// 3. Optionally sets up the connected device
/// 4. Saves the configuration
///
/// Returns the setup result with the configured settings.
pub fn run_setup_wizard(existing_config: Option<Config>) -> io::Result<SetupResult> {
    let mut config = existing_config.unwrap_or_default();

    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║           📸 Photo Extraction Tool - First Time Setup            ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  Welcome! Let's set up your photo backup preferences.            ║");
    println!("║  This only takes a moment.                                       ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    // Step 1: Get backup directory
    let backup_dir = prompt_for_backup_directory()?;

    // Validate and create directory
    if !backup_dir.exists() {
        print!(
            "Directory '{}' doesn't exist. Create it? [Y/n]: ",
            backup_dir.display()
        );
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        let response = response.trim().to_lowercase();

        if response.is_empty() || response == "y" || response == "yes" {
            fs::create_dir_all(&backup_dir).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to create directory: {}", e),
                )
            })?;
            println!("✓ Created directory: {}", backup_dir.display());
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Setup cancelled - directory not created",
            ));
        }
    }

    // Step 2: Scan for existing profiles in the selected directory
    let selected_dir = scan_and_prompt_for_existing_profiles(&backup_dir)?;

    // Use the selected directory (might be a profile subfolder)
    let backup_dir = selected_dir;

    // Apply settings
    config.set_backup_directory(backup_dir.clone());
    config.device_profiles.enabled = true;

    // Step 3: Try to detect connected device
    println!();
    println!("Checking for connected iOS devices...");

    if let Ok(_com_guard) = device::initialize_com() {
        if let Ok(manager) = device::DeviceManager::new() {
            match manager.enumerate_apple_devices() {
                Ok(devices) if !devices.is_empty() => {
                    println!("✓ Found {} device(s):", devices.len());
                    for device in &devices {
                        println!("  • {}", device.friendly_name);
                    }
                    println!();
                    println!(
                        "Device profiles will be created automatically when you extract photos."
                    );
                }
                Ok(_) => {
                    println!("No iOS devices currently connected.");
                    println!("Connect your iPhone/iPad via USB and unlock it to extract photos.");
                }
                Err(e) => {
                    println!("Could not check for devices: {}", e);
                }
            }
        }
    }

    // Step 4: Save configuration
    println!();
    println!("Saving configuration...");

    if let Err(e) = save_config(&config) {
        println!("⚠ Warning: Could not save config file: {}", e);
        println!("  Your settings will be used for this session but won't persist.");
    } else {
        if let Some(path) = get_config_path() {
            println!("✓ Configuration saved to: {}", path.display());
        }
    }

    // Step 5: Initialize empty profiles in backup directory
    // This creates the device_profiles.json in the backup folder
    let mut profile_manager = ProfileManager::new(&config.device_profiles);
    if let Err(e) = profile_manager.ensure_backup_synced() {
        println!(
            "⚠ Warning: Could not initialize profiles in backup folder: {}",
            e
        );
    } else {
        println!("✓ Device profiles initialized in: {}", backup_dir.display());
    }

    // Done!
    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                      ✓ Setup Complete!                           ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!(
        "║  Backup directory: {:width$} ║",
        truncate_path(&backup_dir, 42),
        width = 42
    );
    println!("║  Device profiles:  Enabled                                       ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  You can change these settings anytime by running:               ║");
    println!("║    photo_extraction_tool config                                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    Ok(SetupResult {
        backup_directory: backup_dir,
        completed: true,
        config,
    })
}

/// Scan for existing profiles and prompt user to select one
///
/// If existing extraction profiles are found in the directory (or its subdirectories),
/// the user is asked whether they want to use one of them.
fn scan_and_prompt_for_existing_profiles(backup_dir: &PathBuf) -> io::Result<PathBuf> {
    let tracking_config = TrackingConfig::default();
    let profiles = scan_for_profiles(backup_dir, &tracking_config.tracking_filename);

    if profiles.is_empty() {
        // No existing profiles found, use the directory as-is
        return Ok(backup_dir.clone());
    }

    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              📁 Existing Profiles Found!                         ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();
    println!(
        "Found {} existing extraction profile(s) in this location:",
        profiles.len()
    );
    println!();

    for (i, profile) in profiles.iter().enumerate() {
        let path_display = if profile.path == *backup_dir {
            "(root directory)".to_string()
        } else {
            profile
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| profile.path.display().to_string())
        };

        println!("  [{}] {} - {}", i + 1, profile.friendly_name, path_display);
        println!(
            "      {} files extracted, {} sessions, last used: {}",
            profile.total_files_extracted,
            profile.total_sessions,
            profile.last_seen.format("%Y-%m-%d %H:%M")
        );
        println!();
    }

    println!("  [0] Start fresh (don't use any existing profile)");
    println!();
    println!("Enter a number to continue with that profile, or 0 to start fresh:");
    print!("> ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() || input == "0" {
        println!("✓ Starting fresh in: {}", backup_dir.display());
        return Ok(backup_dir.clone());
    }

    if let Ok(num) = input.parse::<usize>() {
        if num > 0 && num <= profiles.len() {
            let selected = &profiles[num - 1];
            println!(
                "✓ Using existing profile: {} ({})",
                selected.friendly_name,
                selected.path.display()
            );
            return Ok(selected.path.clone());
        }
    }

    // Invalid input, default to the original directory
    println!(
        "Invalid selection. Starting fresh in: {}",
        backup_dir.display()
    );
    Ok(backup_dir.clone())
}

/// Normalize a path string to handle both `/` and `\` separators
///
/// This function ensures paths work correctly regardless of which separator
/// the user types. On Windows, forward slashes are converted to backslashes.
/// On Unix, backslashes are converted to forward slashes.
///
/// # Examples
///
/// ```
/// use photo_extraction_tool::core::setup::normalize_path;
///
/// // On Windows, these all become the same path:
/// let path1 = normalize_path("C:/Users/User/Photos");
/// let path2 = normalize_path("C:\\Users\\User\\Photos");
/// // Both result in: C:\Users\User\Photos
///
/// // On Unix:
/// let path3 = normalize_path("/home/user/photos");
/// let path4 = normalize_path("\\home\\user\\photos");
/// // Both result in: /home/user/photos
/// ```
pub fn normalize_path(input: &str) -> PathBuf {
    #[cfg(windows)]
    {
        // On Windows, normalize forward slashes to backslashes
        PathBuf::from(input.replace('/', "\\"))
    }
    #[cfg(not(windows))]
    {
        // On Unix, normalize backslashes to forward slashes
        PathBuf::from(input.replace('\\', "/"))
    }
}

/// Prompt for backup directory with suggestions
fn prompt_for_backup_directory() -> io::Result<PathBuf> {
    // Get a sensible default path
    let default_path = get_default_backup_path();

    println!("Where would you like to save your photos?");
    println!();
    println!("Default: {}", default_path.display());
    println!();
    println!("Enter a path (or press Enter to use default):");
    print!("> ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    // Parse input
    let path = if input.is_empty() {
        default_path
    } else {
        // Normalize path separators (handle both / and \)
        normalize_path(input)
    };

    Ok(path)
}

/// Get the default backup path
fn get_default_backup_path() -> PathBuf {
    // Try Pictures folder first
    if let Some(pictures) = dirs::picture_dir() {
        return pictures.join("iOS Backup");
    }

    // Fall back to Documents
    if let Some(docs) = dirs::document_dir() {
        return docs.join("iOS Photos");
    }

    // Last resort
    PathBuf::from("./iOS Photos")
}

/// Programmatic setup (for UI integration)
///
/// This function applies setup options without interactive prompts.
/// Call this from the UI after collecting user input.
pub fn apply_setup(options: SetupOptions) -> Result<SetupResult, SetupError> {
    let mut config = Config::default();

    // Validate backup directory
    if options.backup_directory.as_os_str().is_empty() {
        return Err(SetupError::InvalidDirectory(
            "Backup directory cannot be empty".to_string(),
        ));
    }

    // Create directory if it doesn't exist
    if !options.backup_directory.exists() {
        fs::create_dir_all(&options.backup_directory).map_err(|e| {
            SetupError::DirectoryCreationFailed(options.backup_directory.clone(), e.to_string())
        })?;
    }

    // Apply settings
    config.set_backup_directory(options.backup_directory.clone());
    config.device_profiles.enabled = options.enable_profiles;

    // Save configuration
    save_config(&config)?;

    info!(
        "Setup completed. Backup directory: {}",
        options.backup_directory.display()
    );

    Ok(SetupResult {
        backup_directory: options.backup_directory,
        completed: true,
        config,
    })
}

/// Save configuration to the standard location
fn save_config(config: &Config) -> Result<(), SetupError> {
    ensure_config_dir().map_err(|_| SetupError::ConfigDirNotFound)?;

    let config_path = get_config_path().ok_or(SetupError::ConfigDirNotFound)?;

    // Generate config content with comments
    let content = generate_config_with_values(config);

    fs::write(&config_path, content)
        .map_err(|e| SetupError::SaveFailed(config_path.clone(), e.to_string()))?;

    Ok(())
}

/// Generate config file content with the current values
fn generate_config_with_values(config: &Config) -> String {
    let backup_dir = config
        .device_profiles
        .backup_base_folder
        .display()
        .to_string()
        .replace('\\', "/");

    format!(
        r#"# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║                    📸 Photo Extraction Tool Configuration                    ║
# ╚══════════════════════════════════════════════════════════════════════════════╝

# ┌──────────────────────────────────────────────────────────────────────────────┐
# │                         👥 DEVICE PROFILES SETTINGS                          │
# └──────────────────────────────────────────────────────────────────────────────┘
# Device profiles automatically organize photos by device.
# Each device gets its own folder under the backup directory.

[device_profiles]
enabled = {}
backup_base_folder = "{}"
profiles_file = "./.device_profiles.json"

# ┌──────────────────────────────────────────────────────────────────────────────┐
# │                            📁 OUTPUT SETTINGS                                │
# └──────────────────────────────────────────────────────────────────────────────┘
# Fallback settings when device profiles are disabled.

[output]
directory = "{}"
preserve_structure = {}
skip_existing = {}
organize_by_date = {}
subfolder_by_device = {}

# ┌──────────────────────────────────────────────────────────────────────────────┐
# │                            📱 DEVICE SETTINGS                                │
# └──────────────────────────────────────────────────────────────────────────────┘
[device]
apple_only = {}

# ┌──────────────────────────────────────────────────────────────────────────────┐
# │                          🎯 EXTRACTION SETTINGS                              │
# └──────────────────────────────────────────────────────────────────────────────┘
[extraction]
dcim_only = {}
include_extensions = []
exclude_extensions = []
min_file_size = 0
max_file_size = 0
include_photos = {}
include_videos = {}

# ┌──────────────────────────────────────────────────────────────────────────────┐
# │                            📋 LOGGING SETTINGS                               │
# └──────────────────────────────────────────────────────────────────────────────┘
[logging]
level = "{}"
log_to_file = {}
log_file = "{}"

# ┌──────────────────────────────────────────────────────────────────────────────┐
# │                       🔍 DUPLICATE DETECTION SETTINGS                        │
# └──────────────────────────────────────────────────────────────────────────────┘
[duplicate_detection]
enabled = {}
comparison_folders = []
cache_enabled = {}
cache_file = "{}"
duplicate_action = "{}"
recursive = {}
media_only = {}

# ┌──────────────────────────────────────────────────────────────────────────────┐
# │                          📊 TRACKING SETTINGS                                │
# └──────────────────────────────────────────────────────────────────────────────┘
[tracking]
enabled = {}
tracking_filename = "{}"
track_extracted_files = {}

# ┌──────────────────────────────────────────────────────────────────────────────┐
# │                         🤖 ANDROID SETTINGS                                  │
# └──────────────────────────────────────────────────────────────────────────────┘
# These settings control how photos are extracted from Android devices.
# To enable Android support, set apple_only = false in [device] section.

[android]
preserve_structure = {}
include_camera = {}
include_screenshots = {}
include_pictures = {}
include_downloads = {}
exclude_cache_folders = {}

# App-specific media folders (set to true to include)
include_whatsapp = {}
include_telegram = {}
include_instagram = {}
include_facebook = {}
include_snapchat = {}
include_tiktok = {}
include_signal = {}
include_viber = {}

additional_folders = []
exclude_folders = [".thumbnails", ".trash", ".cache", ".nomedia"]

# ╔══════════════════════════════════════════════════════════════════════════════╗
# ║  To edit this file: photo_extraction_tool config                             ║
# ║  To reset to defaults: photo_extraction_tool config --reset                  ║
# ╚══════════════════════════════════════════════════════════════════════════════╝
"#,
        // device_profiles
        config.device_profiles.enabled,
        backup_dir,
        // output
        backup_dir,
        config.output.preserve_structure,
        config.output.skip_existing,
        config.output.organize_by_date,
        config.output.subfolder_by_device,
        // device
        config.device.apple_only,
        // extraction
        config.extraction.dcim_only,
        config.extraction.include_photos,
        config.extraction.include_videos,
        // logging
        config.logging.level,
        config.logging.log_to_file,
        config
            .logging
            .log_file
            .display()
            .to_string()
            .replace('\\', "/"),
        // duplicate_detection
        config.duplicate_detection.enabled,
        config.duplicate_detection.cache_enabled,
        config
            .duplicate_detection
            .cache_file
            .display()
            .to_string()
            .replace('\\', "/"),
        format!("{:?}", config.duplicate_detection.duplicate_action).to_lowercase(),
        config.duplicate_detection.recursive,
        config.duplicate_detection.media_only,
        // tracking
        config.tracking.enabled,
        config.tracking.tracking_filename,
        config.tracking.track_extracted_files,
        // android
        config.android.preserve_structure,
        config.android.include_camera,
        config.android.include_screenshots,
        config.android.include_pictures,
        config.android.include_downloads,
        config.android.exclude_cache_folders,
        // android app folders
        config.android.include_whatsapp,
        config.android.include_telegram,
        config.android.include_instagram,
        config.android.include_facebook,
        config.android.include_snapchat,
        config.android.include_tiktok,
        config.android.include_signal,
        config.android.include_viber,
    )
}

/// Truncate path for display
fn truncate_path(path: &PathBuf, max_len: usize) -> String {
    let path_str = path.display().to_string();
    if path_str.len() <= max_len {
        path_str
    } else {
        format!("...{}", &path_str[path_str.len() - max_len + 3..])
    }
}

/// Check if setup has been completed
///
/// This checks if a valid configuration exists with a backup directory set.
pub fn is_setup_complete() -> bool {
    if let Some(config_path) = get_config_path() {
        if config_path.exists() {
            if let Ok(config) = Config::load(&config_path) {
                return !config.needs_setup();
            }
        }
    }
    false
}

/// Setup error types
#[derive(Debug)]
pub enum SetupError {
    /// Invalid directory path
    InvalidDirectory(String),
    /// Failed to create directory
    DirectoryCreationFailed(PathBuf, String),
    /// Config directory not found
    ConfigDirNotFound,
    /// Failed to save configuration
    SaveFailed(PathBuf, String),
    /// IO error
    IoError(io::Error),
}

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupError::InvalidDirectory(msg) => write!(f, "Invalid directory: {}", msg),
            SetupError::DirectoryCreationFailed(path, err) => {
                write!(f, "Failed to create '{}': {}", path.display(), err)
            }
            SetupError::ConfigDirNotFound => write!(f, "Could not find config directory"),
            SetupError::SaveFailed(path, err) => {
                write!(f, "Failed to save config to '{}': {}", path.display(), err)
            }
            SetupError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for SetupError {}

impl From<io::Error> for SetupError {
    fn from(err: io::Error) -> Self {
        SetupError::IoError(err)
    }
}

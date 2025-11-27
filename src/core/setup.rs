//! First-run setup wizard
//!
//! This module handles the initial configuration of the photo extraction tool.
//! It prompts the user for essential settings like the backup directory and
//! sets up device profiles. This is designed to work with both CLI and future UI.

use crate::core::config::{ensure_config_dir, get_config_path, Config};
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

    // Apply settings
    config.set_backup_directory(backup_dir.clone());
    config.device_profiles.enabled = true;

    // Step 2: Try to detect connected device
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

    // Step 3: Save configuration
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

    // Step 4: Initialize empty profiles in backup directory
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

/// Prompt for backup directory with suggestions
fn prompt_for_backup_directory() -> io::Result<PathBuf> {
    // Generate suggestions based on common locations
    let suggestions = get_directory_suggestions();

    println!("Where would you like to save your photos?");
    println!();

    if !suggestions.is_empty() {
        println!("Suggested locations:");
        for (i, suggestion) in suggestions.iter().enumerate() {
            println!("  [{}] {}", i + 1, suggestion.display());
        }
        println!();
    }

    println!("Enter a path or number from above (or press Enter for default):");
    print!("> ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    // Parse input
    let path = if input.is_empty() {
        // Default to first suggestion or Documents/Photos
        suggestions.first().cloned().unwrap_or_else(|| {
            dirs::document_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("iOS Photos")
        })
    } else if let Ok(num) = input.parse::<usize>() {
        // User selected a number
        if num > 0 && num <= suggestions.len() {
            suggestions[num - 1].clone()
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid selection: {}", num),
            ));
        }
    } else {
        // User entered a path
        PathBuf::from(input)
    };

    Ok(path)
}

/// Get suggested backup directories
fn get_directory_suggestions() -> Vec<PathBuf> {
    let mut suggestions = Vec::new();

    // Pictures folder
    if let Some(pictures) = dirs::picture_dir() {
        suggestions.push(pictures.join("iOS Backup"));
    }

    // Documents folder
    if let Some(docs) = dirs::document_dir() {
        suggestions.push(docs.join("iOS Photos"));
    }

    // Common external drives on Windows (D:, E:, F:)
    #[cfg(target_os = "windows")]
    {
        for drive in ['D', 'E', 'F'] {
            let drive_path = PathBuf::from(format!("{}:\\Photos", drive));
            let drive_root = PathBuf::from(format!("{}:\\", drive));
            if drive_root.exists() {
                suggestions.push(drive_path);
            }
        }
    }

    suggestions
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

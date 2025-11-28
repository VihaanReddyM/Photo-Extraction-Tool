//! Device profiles manager
//!
//! Manages device profiles that map device IDs to user-friendly names and output folders.
//! When a new device is detected, prompts the user for a name and creates a profile.
//!
//! The profiles database is saved in two locations:
//! 1. The main profiles file (configurable, usually in app data)
//! 2. A copy in the backup base folder (for portability and backup)
//!
//! Some accessor methods are kept for API completeness and future use.

use crate::core::config::{DeviceProfile, DeviceProfilesConfig, TrackingConfig};
use crate::core::error::{ExtractionError, Result};
use crate::core::tracking::scan_for_profiles;
use crate::device::DeviceInfo;
use chrono::Utc;
use dialoguer::{Confirm, Input};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// Name of the profiles copy file in the backup directory
const BACKUP_PROFILES_FILENAME: &str = "device_profiles.json";

/// Stored device profiles database
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceProfilesDatabase {
    /// Version of the profiles format
    pub version: u32,

    /// Device profiles indexed by device_id
    pub profiles: HashMap<String, DeviceProfile>,

    /// Last updated timestamp
    #[serde(default)]
    pub last_updated: Option<String>,

    /// Backup base folder (for reference in the backup copy)
    #[serde(default)]
    pub backup_location: Option<String>,
}

/// Device profiles manager
pub struct ProfileManager {
    /// Configuration
    config: DeviceProfilesConfig,

    /// Loaded profiles database
    database: DeviceProfilesDatabase,

    /// Whether the database has been modified
    dirty: bool,
}

impl ProfileManager {
    /// Create a new profile manager
    pub fn new(config: &DeviceProfilesConfig) -> Self {
        Self {
            config: config.clone(),
            database: DeviceProfilesDatabase {
                version: 1,
                profiles: HashMap::new(),
                last_updated: None,
                backup_location: None,
            },
            dirty: false,
        }
    }

    /// Load profiles from disk
    ///
    /// This loads from the main profiles file and syncs to the backup folder.
    pub fn load(&mut self) -> Result<()> {
        if !self.config.profiles_file.exists() {
            debug!(
                "Profiles file does not exist: {}",
                self.config.profiles_file.display()
            );
            return Ok(());
        }

        let file = File::open(&self.config.profiles_file).map_err(|e| {
            ExtractionError::IoError(format!("Failed to open profiles file: {}", e))
        })?;

        let reader = BufReader::new(file);
        self.database = serde_json::from_reader(reader).map_err(|e| {
            ExtractionError::IoError(format!("Failed to parse profiles file: {}", e))
        })?;

        debug!(
            "Loaded {} device profile(s) from {}",
            self.database.profiles.len(),
            self.config.profiles_file.display()
        );

        for (id, profile) in &self.database.profiles {
            debug!("  - {} -> {}", profile.name, profile.output_folder);
            let _ = id; // Suppress unused warning
        }

        // Sync to backup folder to ensure it's up to date
        if let Err(e) = self.ensure_backup_synced() {
            warn!("Failed to sync profiles to backup folder: {}", e);
        }

        Ok(())
    }

    /// Save profiles to disk (both main file and backup copy)
    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        // Update metadata
        self.database.last_updated = Some(Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
        self.database.backup_location = Some(self.config.backup_base_folder.display().to_string());

        // Ensure parent directory exists for main profiles file
        if let Some(parent) = self.config.profiles_file.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ExtractionError::IoError(format!("Failed to create profiles directory: {}", e))
            })?;
        }

        // Save to main profiles file
        let file = File::create(&self.config.profiles_file).map_err(|e| {
            ExtractionError::IoError(format!("Failed to create profiles file: {}", e))
        })?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.database).map_err(|e| {
            ExtractionError::IoError(format!("Failed to write profiles file: {}", e))
        })?;

        debug!(
            "Saved {} device profile(s) to {}",
            self.database.profiles.len(),
            self.config.profiles_file.display()
        );

        // Also save a copy to the backup base folder
        self.sync_to_backup_folder()?;

        self.dirty = false;
        Ok(())
    }

    /// Sync profiles to the backup folder
    ///
    /// This creates/updates a copy of the profiles database in the backup
    /// base folder so users can see which devices have been backed up.
    fn sync_to_backup_folder(&self) -> Result<()> {
        // Only sync if backup folder exists and is configured
        if self.config.backup_base_folder.as_os_str().is_empty() {
            debug!("Backup base folder not configured, skipping profile sync");
            return Ok(());
        }

        // Create backup folder if it doesn't exist
        if !self.config.backup_base_folder.exists() {
            fs::create_dir_all(&self.config.backup_base_folder).map_err(|e| {
                ExtractionError::IoError(format!(
                    "Failed to create backup folder '{}': {}",
                    self.config.backup_base_folder.display(),
                    e
                ))
            })?;
        }

        let backup_profiles_path = self
            .config
            .backup_base_folder
            .join(BACKUP_PROFILES_FILENAME);

        let file = File::create(&backup_profiles_path).map_err(|e| {
            ExtractionError::IoError(format!(
                "Failed to create backup profiles file '{}': {}",
                backup_profiles_path.display(),
                e
            ))
        })?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.database).map_err(|e| {
            ExtractionError::IoError(format!("Failed to write backup profiles file: {}", e))
        })?;

        debug!(
            "Synced profiles to backup folder: {}",
            backup_profiles_path.display()
        );

        Ok(())
    }

    /// Force sync profiles to backup folder (even if not dirty)
    ///
    /// Call this after loading profiles to ensure the backup copy is up to date.
    pub fn ensure_backup_synced(&mut self) -> Result<()> {
        // Update metadata before syncing
        self.database.last_updated = Some(Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
        self.database.backup_location = Some(self.config.backup_base_folder.display().to_string());

        self.sync_to_backup_folder()
    }

    /// Get profile for a device, prompting user if not found
    pub fn get_or_create_profile(&mut self, device: &DeviceInfo) -> Result<DeviceProfile> {
        // Check if we already have a profile for this device
        if let Some(mut profile) = self.database.profiles.get(&device.device_id).cloned() {
            debug!("Found existing profile for device: {}", profile.name);

            // Update last seen
            profile.last_seen = Some(Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());
            self.database
                .profiles
                .insert(device.device_id.clone(), profile.clone());
            self.dirty = true;

            return Ok(profile);
        }

        // New device detected - check for existing extraction profiles in backup folder
        // that might match this device (by device_id)
        if let Some(profile) = self.check_for_existing_extraction_profiles(device)? {
            return Ok(profile);
        }

        // No existing profile found - prompt user for a name
        debug!("New device detected: {}", device.friendly_name);
        debug!("  Manufacturer: {}", device.manufacturer);
        debug!("  Model: {}", device.model);
        debug!("  Device ID: {}", device.device_id);

        println!("\n========================================");
        println!("  NEW DEVICE DETECTED!");
        println!("========================================");
        println!("  Device: {}", device.friendly_name);
        println!("  Manufacturer: {}", device.manufacturer);
        println!("  Model: {}", device.model);
        println!("----------------------------------------\n");

        // Prompt for device name
        let default_name = sanitize_folder_name(&device.friendly_name);
        let name: String = Input::new()
            .with_prompt("Enter a name for this device (used for folder name)")
            .default(default_name)
            .interact_text()
            .map_err(|e| ExtractionError::IoError(format!("Failed to read input: {}", e)))?;

        // Create folder name from the name
        let folder_name = sanitize_folder_name(&name);

        // Show the output folder that will be created
        let output_folder = self.config.backup_base_folder.join(&folder_name);
        println!("\nPhotos will be saved to: {}", output_folder.display());

        // Confirm
        let confirmed = Confirm::new()
            .with_prompt("Is this correct?")
            .default(true)
            .interact()
            .map_err(|e| ExtractionError::IoError(format!("Failed to read input: {}", e)))?;

        if !confirmed {
            // Let user enter custom folder name
            let custom_folder: String = Input::new()
                .with_prompt("Enter custom folder name")
                .default(folder_name.clone())
                .interact_text()
                .map_err(|e| ExtractionError::IoError(format!("Failed to read input: {}", e)))?;

            let output_folder = self.config.backup_base_folder.join(&custom_folder);
            println!("\nPhotos will be saved to: {}", output_folder.display());

            return self.create_and_save_profile(device, &name, &custom_folder);
        }

        self.create_and_save_profile(device, &name, &folder_name)
    }

    /// Check for existing extraction profiles that match this device
    ///
    /// Scans the backup folder for subdirectories containing extraction state files
    /// and offers to use any that match the current device's ID.
    fn check_for_existing_extraction_profiles(
        &mut self,
        device: &DeviceInfo,
    ) -> Result<Option<DeviceProfile>> {
        // Skip if backup folder is not configured
        if self.config.backup_base_folder.as_os_str().is_empty() {
            debug!("Skipping profile scan: backup folder not configured");
            return Ok(None);
        }

        debug!(
            "Scanning for existing extraction profiles in: {}",
            self.config.backup_base_folder.display()
        );

        let tracking_config = TrackingConfig::default();
        let profiles = scan_for_profiles(
            &self.config.backup_base_folder,
            &tracking_config.tracking_filename,
        );

        if profiles.is_empty() {
            debug!("No existing extraction profiles found in backup folder");
            return Ok(None);
        }

        debug!("Found {} existing extraction profile(s)", profiles.len());

        // Check if any profile exactly matches this device's ID
        let exact_match = profiles.iter().find(|p| p.device_id == device.device_id);

        // Always show existing profiles to let user choose
        println!("\n╔══════════════════════════════════════════════════════════════════╗");
        if exact_match.is_some() {
            println!("║              📱 Previous Extraction Found!                       ║");
        } else {
            println!("║              📁 Existing Profiles Found                          ║");
        }
        println!("╚══════════════════════════════════════════════════════════════════╝");
        println!();
        println!(
            "Found {} existing extraction profile(s) in backup folder:",
            profiles.len()
        );
        println!();

        for (i, profile) in profiles.iter().enumerate() {
            let folder_name = profile
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "(root)".to_string());

            let is_match = exact_match
                .map(|m| m.device_id == profile.device_id)
                .unwrap_or(false);
            let match_indicator = if is_match { " ✓ (same device)" } else { "" };

            println!(
                "  [{}] {} ({}){}",
                i + 1,
                profile.friendly_name,
                folder_name,
                match_indicator
            );
            println!(
                "      {} files ({:.2} GB), last used: {}",
                profile.total_files_extracted,
                profile.total_bytes_extracted as f64 / 1_073_741_824.0,
                profile.last_seen.format("%Y-%m-%d %H:%M")
            );
        }
        println!();
        println!("  [0] Create new profile for this device");
        println!();

        // Default to the exact match if found, otherwise 0 (create new)
        let default_selection = if let Some(matched) = &exact_match {
            profiles
                .iter()
                .position(|p| p.device_id == matched.device_id)
                .map(|i| (i + 1).to_string())
                .unwrap_or_else(|| "0".to_string())
        } else {
            "0".to_string()
        };

        let selection: String = Input::new()
            .with_prompt("Select a profile to use, or 0 for new")
            .default(default_selection)
            .interact_text()
            .map_err(|e| ExtractionError::IoError(format!("Failed to read input: {}", e)))?;

        if let Ok(num) = selection.parse::<usize>() {
            if num > 0 && num <= profiles.len() {
                let selected = &profiles[num - 1];
                let folder_name = selected
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| sanitize_folder_name(&selected.friendly_name));

                let is_same_device = selected.device_id == device.device_id;

                if !is_same_device {
                    // Warn if the device IDs don't match
                    println!();
                    println!("⚠ Note: This profile was created for a different device.");
                    println!("  Extraction history will be shared with the selected profile.");
                    println!();

                    let confirm = Confirm::new()
                        .with_prompt("Continue with this profile?")
                        .default(true)
                        .interact()
                        .map_err(|e| {
                            ExtractionError::IoError(format!("Failed to read input: {}", e))
                        })?;

                    if !confirm {
                        return Ok(None);
                    }
                }

                let profile =
                    self.create_and_save_profile(device, &selected.friendly_name, &folder_name)?;
                println!("✓ Using profile: {}", selected.path.display());
                return Ok(Some(profile));
            }
        }

        // User chose to create new profile
        Ok(None)
    }

    /// Create a new profile and save it
    fn create_and_save_profile(
        &mut self,
        device: &DeviceInfo,
        name: &str,
        folder_name: &str,
    ) -> Result<DeviceProfile> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let profile = DeviceProfile {
            name: name.to_string(),
            output_folder: folder_name.to_string(),
            manufacturer: device.manufacturer.clone(),
            model: device.model.clone(),
            first_seen: Some(now.clone()),
            last_seen: Some(now),
        };

        // Create the output folder
        let output_path = self.config.backup_base_folder.join(folder_name);
        fs::create_dir_all(&output_path).map_err(|e| {
            ExtractionError::IoError(format!(
                "Failed to create output folder '{}': {}",
                output_path.display(),
                e
            ))
        })?;

        debug!("Created output folder: {}", output_path.display());

        // Save profile
        self.database
            .profiles
            .insert(device.device_id.clone(), profile.clone());
        self.dirty = true;
        self.save()?;

        println!("\n✓ Profile saved for '{}'", name);
        println!("  Next time this device is connected, it will automatically use this folder.\n");

        Ok(profile)
    }

    // =========================================================================
    // Accessor methods (kept for API completeness and future use)
    // =========================================================================

    /// Get the output path for a device
    #[allow(dead_code)]
    pub fn get_output_path(&self, device_id: &str) -> Option<PathBuf> {
        self.database
            .profiles
            .get(device_id)
            .map(|profile| self.config.backup_base_folder.join(&profile.output_folder))
    }

    /// Get the base backup folder
    #[allow(dead_code)]
    pub fn backup_base_folder(&self) -> &Path {
        &self.config.backup_base_folder
    }

    /// List all known profiles
    pub fn list_profiles(&self) {
        println!("\nDevice Profiles:");
        println!("========================================");
        println!(
            "Backup location: {}",
            self.config.backup_base_folder.display()
        );
        if let Some(ref updated) = self.database.last_updated {
            println!("Last updated: {}", updated);
        }

        // Show backup profiles file location
        let backup_profiles_path = self
            .config
            .backup_base_folder
            .join(BACKUP_PROFILES_FILENAME);
        println!("Profiles file: {}", backup_profiles_path.display());

        if self.database.profiles.is_empty() {
            println!("\nNo devices registered yet.");
            println!("Connect an iOS device to create a profile.");
            println!("========================================\n");
            return;
        }

        println!("\nRegistered Devices ({}):", self.database.profiles.len());

        for (id, profile) in &self.database.profiles {
            let output_path = self.config.backup_base_folder.join(&profile.output_folder);
            println!("\n  Name: {}", profile.name);
            println!("  Folder: {}", output_path.display());
            println!("  Manufacturer: {}", profile.manufacturer);
            println!("  Model: {}", profile.model);
            if let Some(ref first) = profile.first_seen {
                println!("  First seen: {}", first);
            }
            if let Some(ref last) = profile.last_seen {
                println!("  Last seen: {}", last);
            }
            println!("  Device ID: {}...", &id[..id.len().min(50)]);
        }

        println!("\n========================================\n");
    }

    /// Check if profiles feature is enabled
    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Remove a profile by device ID
    pub fn remove_profile(&mut self, device_id: &str) -> Option<DeviceProfile> {
        let profile = self.database.profiles.remove(device_id);
        if profile.is_some() {
            self.dirty = true;
        }
        profile
    }

    /// Get all profiles
    pub fn get_all_profiles(&self) -> &HashMap<String, DeviceProfile> {
        &self.database.profiles
    }
}

impl Drop for ProfileManager {
    fn drop(&mut self) {
        if self.dirty {
            if let Err(e) = self.save() {
                warn!("Failed to save profiles on drop: {}", e);
            }
        }
    }
}

/// Sanitize a string to be used as a folder name
fn sanitize_folder_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_folder_name() {
        assert_eq!(sanitize_folder_name("iPhone"), "iPhone");
        assert_eq!(sanitize_folder_name("John's iPhone"), "John's iPhone");
        assert_eq!(sanitize_folder_name("Device: Test"), "Device_ Test");
        assert_eq!(sanitize_folder_name("A/B\\C"), "A_B_C");
    }

    #[test]
    fn test_profile_manager_new() {
        let config = DeviceProfilesConfig::default();
        let manager = ProfileManager::new(&config);
        assert!(manager.database.profiles.is_empty());
    }
}

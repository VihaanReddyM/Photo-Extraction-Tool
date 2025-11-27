//! Device profiles manager
//!
//! Manages device profiles that map device IDs to user-friendly names and output folders.
//! When a new device is detected, prompts the user for a name and creates a profile.
//!
//! Some accessor methods are kept for API completeness and future use.

use crate::core::config::{DeviceProfile, DeviceProfilesConfig};
use crate::core::error::{ExtractionError, Result};
use crate::device::DeviceInfo;
use chrono::Utc;
use dialoguer::{Confirm, Input};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// Stored device profiles database
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceProfilesDatabase {
    /// Version of the profiles format
    pub version: u32,

    /// Device profiles indexed by device_id
    pub profiles: HashMap<String, DeviceProfile>,
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
            },
            dirty: false,
        }
    }

    /// Load profiles from disk
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

        info!(
            "Loaded {} device profile(s) from {}",
            self.database.profiles.len(),
            self.config.profiles_file.display()
        );

        for (id, profile) in &self.database.profiles {
            debug!("  - {} -> {}", profile.name, profile.output_folder);
            let _ = id; // Suppress unused warning
        }

        Ok(())
    }

    /// Save profiles to disk
    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = self.config.profiles_file.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ExtractionError::IoError(format!("Failed to create profiles directory: {}", e))
            })?;
        }

        let file = File::create(&self.config.profiles_file).map_err(|e| {
            ExtractionError::IoError(format!("Failed to create profiles file: {}", e))
        })?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.database).map_err(|e| {
            ExtractionError::IoError(format!("Failed to write profiles file: {}", e))
        })?;

        info!(
            "Saved {} device profile(s) to {}",
            self.database.profiles.len(),
            self.config.profiles_file.display()
        );

        self.dirty = false;
        Ok(())
    }

    /// Get profile for a device, prompting user if not found
    pub fn get_or_create_profile(&mut self, device: &DeviceInfo) -> Result<DeviceProfile> {
        // Check if we already have a profile for this device
        if let Some(mut profile) = self.database.profiles.get(&device.device_id).cloned() {
            info!("Found existing profile for device: {}", profile.name);

            // Update last seen
            profile.last_seen = Some(Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());
            self.database
                .profiles
                .insert(device.device_id.clone(), profile.clone());
            self.dirty = true;

            return Ok(profile);
        }

        // New device detected - prompt user for a name
        info!("New device detected: {}", device.friendly_name);
        info!("  Manufacturer: {}", device.manufacturer);
        info!("  Model: {}", device.model);
        info!("  Device ID: {}", device.device_id);

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

        info!("Created output folder: {}", output_path.display());

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
        if self.database.profiles.is_empty() {
            println!("No device profiles configured.");
            return;
        }

        println!("\nConfigured Device Profiles:");
        println!("========================================");

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

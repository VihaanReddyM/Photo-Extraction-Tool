//! Command handler implementations
//!
//! This module contains the implementation of all CLI commands.

use crate::cli::progress::{BenchmarkProgress, ScanProgressTracker};
use crate::cli::{Args, Commands, TestCommands};
use crate::core::config::{
    get_config_path, init_config, open_config_in_editor, Config, TrackingConfig,
};
use crate::core::extractor;
use crate::core::setup::run_setup_wizard;
use crate::core::tracking::scan_for_profiles;
use crate::device::traits::{DeviceContentTrait, DeviceManagerTrait};
use crate::device::{self, ProfileManager};
use crate::testdb::{
    self, InteractiveTestMode, MockDataGenerator, ScenarioLibrary, TestRunner, TestRunnerConfig,
};
use anyhow::Result;
use log::{debug, error, info, warn};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Run the appropriate command based on CLI arguments
///
/// If initial setup is required (no backup directory configured), this will
/// run the setup wizard first before proceeding with extraction commands.
pub fn run_command(args: &Args, config: &Config, shutdown_flag: Arc<AtomicBool>) -> Result<()> {
    // Check if setup is needed for extraction commands
    let config = check_and_run_setup_if_needed(args, config)?;

    match &args.command {
        Some(Commands::Config { path, reset }) => {
            handle_config_command(*path, *reset)?;
        }
        Some(Commands::GenerateConfig { output }) => {
            generate_config_file(output.clone())?;
        }
        Some(Commands::List { all }) => {
            let use_all = *all || !config.device.apple_only;
            list_devices(use_all)?;
        }
        Some(Commands::ShowConfig) => {
            show_config(&config);
        }
        Some(Commands::Scan { depth }) => {
            scan_device(&config, *depth)?;
        }
        Some(Commands::Extract {
            detect_duplicates,
            compare_folders,
            duplicate_action,
        }) => {
            // Merge command-level args with global args
            let use_detect = *detect_duplicates || args.detect_duplicates;
            let mut folders = compare_folders.clone();
            folders.extend(args.compare_folders.clone());
            let action = duplicate_action
                .clone()
                .or_else(|| args.duplicate_action.clone());

            extract_photos_with_args(&config, shutdown_flag, use_detect, folders, action)?;
        }
        None => {
            // Use global args when no subcommand specified
            extract_photos_with_args(
                &config,
                shutdown_flag,
                args.detect_duplicates,
                args.compare_folders.clone(),
                args.duplicate_action.clone(),
            )?;
        }
        Some(Commands::ListProfiles) => {
            list_profiles(&config)?;
        }
        Some(Commands::ScanProfiles { directory }) => {
            scan_profiles(&config, directory.clone())?;
        }
        Some(Commands::RemoveProfile { name }) => {
            remove_profile(&config, name)?;
        }
        Some(Commands::BenchmarkScan { dcim_only }) => {
            benchmark_scan(&config, *dcim_only)?;
        }
        Some(Commands::Test { test_command }) => {
            handle_test_command(test_command)?;
        }
    }

    Ok(())
}

/// Check if setup is needed and run the wizard if necessary
///
/// Returns the (possibly updated) config to use for the command.
fn check_and_run_setup_if_needed(args: &Args, config: &Config) -> Result<Config> {
    // Setup is only needed for extraction commands
    let needs_extraction = matches!(
        &args.command,
        None | Some(Commands::Extract { .. }) | Some(Commands::Scan { .. })
    );

    if !needs_extraction {
        return Ok(config.clone());
    }

    // Check if setup is required
    if !config.needs_setup() {
        debug!("Configuration is complete, no setup needed");
        return Ok(config.clone());
    }

    // Check if user provided an output directory via CLI args
    if let Some(ref output) = args.output {
        debug!("Using output directory from CLI args: {}", output.display());
        let mut config = config.clone();
        config.set_backup_directory(output.clone());
        return Ok(config);
    }

    // Need to run setup wizard
    info!("Initial setup required...");

    match run_setup_wizard(Some(config.clone())) {
        Ok(result) => {
            info!("Setup completed successfully");
            Ok(result.config)
        }
        Err(e) => {
            error!("Setup failed: {}", e);
            Err(anyhow::anyhow!(
                "Setup required but failed. Run 'photo_extraction_tool config' to configure manually."
            ))
        }
    }
}

/// Handle test subcommands
pub fn handle_test_command(test_command: &TestCommands) -> Result<()> {
    match test_command {
        TestCommands::RunAll {
            html_report,
            json_report,
            output,
            fail_fast,
        } => {
            test_run_all(*html_report, *json_report, output.clone(), *fail_fast)?;
        }
        TestCommands::RunQuick { verbose } => {
            test_run_quick(*verbose)?;
        }
        TestCommands::RunTag { tag, verbose } => {
            test_run_by_tag(tag, *verbose)?;
        }
        TestCommands::Run { scenarios, verbose } => {
            test_run_scenarios(scenarios, *verbose)?;
        }
        TestCommands::ListScenarios { tag, detailed } => {
            test_list_scenarios(tag.as_deref(), *detailed)?;
        }
        TestCommands::ListTags => {
            test_list_tags()?;
        }
        TestCommands::Interactive => {
            test_interactive()?;
        }
        TestCommands::Info { name } => {
            test_scenario_info(name)?;
        }
        TestCommands::SimulateExtract {
            scenario,
            output,
            verbose,
            simulate_errors,
        } => {
            test_simulate_extract(scenario, output, *verbose, *simulate_errors)?;
        }
        TestCommands::GenerateData {
            output,
            count,
            types,
            include_duplicates,
            seed,
        } => {
            test_generate_data(output, *count, types, *include_duplicates, *seed)?;
        }
        TestCommands::BenchmarkMock {
            file_count,
            iterations,
        } => {
            test_benchmark_mock(*file_count, *iterations)?;
        }
    }
    Ok(())
}

/// List all configured device profiles
pub fn list_profiles(config: &Config) -> Result<()> {
    let mut manager = ProfileManager::new(&config.device_profiles);
    manager.load().ok();
    manager.list_profiles();
    Ok(())
}

/// Scan a directory for existing extraction profiles
///
/// This scans the specified directory (or configured backup directory) for
/// subdirectories containing extraction state files (.photo_extraction_state.json).
pub fn scan_profiles(config: &Config, directory: Option<PathBuf>) -> Result<()> {
    let scan_dir = directory.unwrap_or_else(|| config.device_profiles.backup_base_folder.clone());

    if scan_dir.as_os_str().is_empty() {
        error!("No directory specified and no backup directory configured.");
        error!("Run setup first or specify a directory with --directory");
        return Ok(());
    }

    if !scan_dir.exists() {
        error!("Directory does not exist: {}", scan_dir.display());
        return Ok(());
    }

    info!(
        "Scanning for extraction profiles in: {}",
        scan_dir.display()
    );
    println!();

    let tracking_config = TrackingConfig::default();
    let profiles = scan_for_profiles(&scan_dir, &tracking_config.tracking_filename);

    if profiles.is_empty() {
        println!("No extraction profiles found in this directory.");
        println!();
        println!("Extraction profiles are created when you extract photos from a device.");
        println!(
            "Each profile contains a {} file with extraction history.",
            tracking_config.tracking_filename
        );
        return Ok(());
    }

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!(
        "║              📁 Found {} Extraction Profile(s)                    ║",
        profiles.len()
    );
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    for (i, profile) in profiles.iter().enumerate() {
        let path_display = if profile.path == scan_dir {
            "(root directory)".to_string()
        } else {
            profile.path.display().to_string()
        };

        println!("{}. {}", i + 1, profile.friendly_name);
        println!(
            "   ├─ Device:    {} {}",
            profile.manufacturer, profile.model
        );
        println!("   ├─ Location:  {}", path_display);
        println!(
            "   ├─ Files:     {} extracted ({})",
            profile.total_files_extracted,
            format_bytes(profile.total_bytes_extracted)
        );
        println!("   ├─ Sessions:  {}", profile.total_sessions);
        println!(
            "   ├─ First use: {}",
            profile.first_seen.format("%Y-%m-%d %H:%M")
        );
        println!(
            "   └─ Last use:  {}",
            profile.last_seen.format("%Y-%m-%d %H:%M")
        );
        println!();
    }

    Ok(())
}

/// Format bytes into human-readable format
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Remove a device profile by name or partial ID
pub fn remove_profile(config: &Config, name: &str) -> Result<()> {
    let mut manager = ProfileManager::new(&config.device_profiles);
    manager.load()?;

    // Find profile by name or partial device ID
    let profiles = manager.get_all_profiles().clone();
    let mut found_id: Option<String> = None;

    for (id, profile) in &profiles {
        if profile.name.to_lowercase().contains(&name.to_lowercase())
            || id.to_lowercase().contains(&name.to_lowercase())
        {
            found_id = Some(id.clone());
            info!(
                "Found profile: {} -> {}",
                profile.name, profile.output_folder
            );
            break;
        }
    }

    if let Some(id) = found_id {
        if let Some(profile) = manager.remove_profile(&id) {
            manager.save()?;
            info!("Removed profile: {}", profile.name);
        }
    } else {
        error!("No profile found matching: {}", name);
    }

    Ok(())
}

/// Handle the `config` command - open, show path, or reset the config file
pub fn handle_config_command(show_path: bool, reset: bool) -> Result<()> {
    if reset {
        // Delete existing config and create a fresh one
        if let Some(config_path) = get_config_path() {
            if config_path.exists() {
                std::fs::remove_file(&config_path)?;
                info!("Removed existing config file");
            }
        }
        let path = init_config()?;
        info!("Created fresh config file at: {}", path.display());
        return Ok(());
    }

    if show_path {
        // Just show the path
        let path = Config::get_active_config_path();
        println!("{}", path.display());
        if path.exists() {
            info!("Config file exists at: {}", path.display());
        } else {
            info!("Config file would be created at: {}", path.display());
        }
        return Ok(());
    }

    // Open the config file in the default editor
    info!("Opening configuration file in default editor...");
    match open_config_in_editor() {
        Ok(path) => {
            info!("Config file: {}", path.display());
            info!("Save the file after editing to apply changes.");
            info!("Run 'photo_extraction_tool show-config' to verify your settings.");
        }
        Err(e) => {
            error!("Failed to open config file: {}", e);
            // Fall back to showing the path
            if let Some(path) = get_config_path() {
                info!("You can manually edit the config at: {}", path.display());
            }
        }
    }

    Ok(())
}

/// Generate a configuration file at the specified or default location
pub fn generate_config_file(output: Option<PathBuf>) -> Result<()> {
    use std::fs;

    let custom_path = output.is_some();
    let output_path = match output {
        Some(path) => path,
        None => {
            // Use standard location
            init_config()?
        }
    };

    // If a specific path was given, write the config there
    if custom_path {
        let content = Config::generate_default_config();
        fs::write(&output_path, content)?;
    }

    info!("Configuration file: {}", output_path.display());
    info!("Edit this file to customize the extraction settings.");
    info!("");
    info!("Quick tip: Run 'photo_extraction_tool config' to open the config in your editor.");

    Ok(())
}

/// Show the current configuration settings
pub fn show_config(config: &Config) {
    let config_path = Config::get_active_config_path();
    info!("Configuration file: {}", config_path.display());
    if !config_path.exists() {
        info!("(Using default settings - no config file found)");
    }
    info!("");
    info!("Current Configuration:");
    info!("----------------------");
    info!("[device_profiles]");
    info!("  enabled = {}", config.device_profiles.enabled);
    info!(
        "  backup_base_folder = \"{}\"",
        config.device_profiles.backup_base_folder.display()
    );
    if config.needs_setup() {
        info!("  ⚠ Setup required - run the tool to configure backup location");
    }
    info!("");
    info!("[output]");
    info!("  directory = \"{}\"", config.output.directory.display());
    info!(
        "  preserve_structure = {}",
        config.output.preserve_structure
    );
    info!("  skip_existing = {}", config.output.skip_existing);
    info!("  organize_by_date = {}", config.output.organize_by_date);
    info!(
        "  subfolder_by_device = {}",
        config.output.subfolder_by_device
    );
    info!("");
    info!("[device]");
    info!(
        "  device_id = {:?}",
        config.device.device_id.as_deref().unwrap_or("(auto)")
    );
    info!(
        "  device_name_filter = {:?}",
        config
            .device
            .device_name_filter
            .as_deref()
            .unwrap_or("(none)")
    );
    info!("  apple_only = {}", config.device.apple_only);
    info!("");
    info!("[extraction]");
    info!("  dcim_only = {}", config.extraction.dcim_only);
    info!(
        "  include_extensions = {:?}",
        config.extraction.include_extensions
    );
    info!(
        "  exclude_extensions = {:?}",
        config.extraction.exclude_extensions
    );
    info!("  include_photos = {}", config.extraction.include_photos);
    info!("  include_videos = {}", config.extraction.include_videos);
    info!("");
    info!("[logging]");
    info!("  level = \"{}\"", config.logging.level);
    info!("");
    info!("[duplicate_detection]");
    info!("  enabled = {}", config.duplicate_detection.enabled);
    info!(
        "  comparison_folders = {:?}",
        config.duplicate_detection.comparison_folders
    );
    info!(
        "  cache_enabled = {}",
        config.duplicate_detection.cache_enabled
    );
    info!(
        "  cache_file = \"{}\"",
        config.duplicate_detection.cache_file.display()
    );
    info!(
        "  duplicate_action = {:?}",
        config.duplicate_detection.duplicate_action
    );
    info!("  recursive = {}", config.duplicate_detection.recursive);
    info!("  media_only = {}", config.duplicate_detection.media_only);
    info!("");
    info!("[tracking]");
    info!("  enabled = {}", config.tracking.enabled);
    info!(
        "  tracking_filename = \"{}\"",
        config.tracking.tracking_filename
    );
    info!(
        "  track_extracted_files = {}",
        config.tracking.track_extracted_files
    );
}

/// List connected devices
pub fn list_devices(all_devices: bool) -> Result<()> {
    // Initialize COM library (required for WPD)
    let _com_guard = device::initialize_com()?;

    // Create device manager
    let manager = device::DeviceManager::new()?;

    info!("Scanning for connected devices...");

    let devices = if all_devices {
        manager.enumerate_all_devices()?
    } else {
        manager.enumerate_apple_devices()?
    };

    if devices.is_empty() {
        info!("No portable devices found.");
        info!("");
        info!("Make sure your iPhone is:");
        info!("  1. Connected via USB cable");
        info!("  2. Unlocked");
        info!("  3. Trusting this computer (tap 'Trust' when prompted)");
        info!("");
        if !all_devices {
            info!("Tip: Use 'list --all' to see all portable devices");
        }
        return Ok(());
    }

    info!("Found {} device(s):", devices.len());
    info!("");
    for (i, device) in devices.iter().enumerate() {
        info!("[{}] {}", i + 1, device.friendly_name);
        info!("    Manufacturer: {}", device.manufacturer);
        info!("    Model: {}", device.model);
        info!("    Device ID: {}", device.device_id);
        info!("");
    }

    Ok(())
}

/// Scan device and show folder structure
pub fn scan_device(config: &Config, max_depth: usize) -> Result<()> {
    // Initialize COM library
    let _com_guard = device::initialize_com()?;

    // Create device manager
    let manager = device::DeviceManager::new()?;

    info!("Scanning for connected devices...");

    let devices = if config.device.apple_only {
        manager.enumerate_apple_devices()?
    } else {
        manager.enumerate_all_devices()?
    };

    if devices.is_empty() {
        error!("No devices found.");
        return Ok(());
    }

    // Select device
    let target_device = select_device(&devices, &config.device.device_id)?;
    info!("Selected device: {}", target_device.friendly_name);

    // Open device
    let content = manager.open_device(&target_device.device_id)?;

    info!("Scanning device structure (max depth: {})...", max_depth);
    info!("");

    // Create progress tracker
    let progress = ScanProgressTracker::new();

    // Scan and print structure
    scan_recursive(&content, "DEVICE", "", 0, max_depth, &progress)?;

    progress.finish();

    Ok(())
}

/// Benchmark scan performance
pub fn benchmark_scan(config: &Config, dcim_only: bool) -> Result<()> {
    use std::time::Instant;

    // Initialize COM library
    let _com_guard = device::initialize_com()?;

    // Create device manager
    let manager = device::DeviceManager::new()?;

    info!("=== Photo Discovery Benchmark ===");
    info!("");

    let devices = if config.device.apple_only {
        manager.enumerate_apple_devices()?
    } else {
        manager.enumerate_all_devices()?
    };

    if devices.is_empty() {
        error!("No devices found.");
        return Ok(());
    }

    // Select device
    let target_device = select_device(&devices, &config.device.device_id)?;
    info!("Device: {}", target_device.friendly_name);
    info!("DCIM only: {}", dcim_only);
    info!("");

    // Open device
    let content = manager.open_device(&target_device.device_id)?;

    // Create progress tracker
    let progress = BenchmarkProgress::new();

    info!("Starting scan benchmark...");
    let start = Instant::now();

    // Scan for files
    let root_objects = content.enumerate_objects()?;
    progress.log_event(&format!("Found {} root objects", root_objects.len()));

    let mut total_folders = 0usize;
    let mut total_files = 0usize;
    let mut media_files = 0usize;

    for obj in root_objects {
        if obj.is_folder {
            benchmark_scan_recursive(
                &content,
                &obj.object_id,
                &obj.name,
                dcim_only,
                &progress,
                &mut total_folders,
                &mut total_files,
                &mut media_files,
            )?;
        }
    }

    let elapsed = start.elapsed();
    progress.finish();

    // Print summary
    info!("");
    info!("=== Benchmark Results ===");
    info!("Total time: {:.2}s", elapsed.as_secs_f64());
    info!("Folders scanned: {}", total_folders);
    info!("Total files found: {}", total_files);
    info!("Media files found: {}", media_files);
    if elapsed.as_secs_f64() > 0.0 {
        info!(
            "Scan rate: {:.1} items/second",
            (total_folders + total_files) as f64 / elapsed.as_secs_f64()
        );
    }
    info!("");

    Ok(())
}

/// Extract photos from the connected device
/// Extract photos with command-line arguments for duplicate detection
pub fn extract_photos_with_args(
    config: &Config,
    shutdown_flag: Arc<AtomicBool>,
    detect_duplicates: bool,
    compare_folders: Vec<PathBuf>,
    duplicate_action: Option<String>,
) -> Result<()> {
    // Initialize COM library (required for WPD)
    let _com_guard = device::initialize_com()?;

    // Create device manager
    let manager = device::DeviceManager::new()?;

    debug!("Scanning for connected devices...");

    let devices = if config.device.apple_only {
        manager.enumerate_apple_devices()?
    } else {
        manager.enumerate_all_devices()?
    };

    if devices.is_empty() {
        println!();
        println!("  ✗ No portable devices found.");
        println!();
        println!("  Make sure your iPhone is:");
        println!("    1. Connected via USB cable");
        println!("    2. Unlocked");
        println!("    3. Trusting this computer (tap 'Trust' when prompted)");
        println!();
        println!("  Use 'list' command to see available devices");
        println!("  Use '--all-devices' to see all portable devices (not just Apple)");
        return Ok(());
    }

    // Select device
    let target_device = select_device(&devices, &config.device.device_id)?;
    debug!("Selected device: {}", target_device.friendly_name);

    // Determine output directory - use device profiles if enabled
    let output_dir = if config.device_profiles.enabled {
        let mut profile_manager = ProfileManager::new(&config.device_profiles);
        profile_manager.load().ok(); // Ignore error if file doesn't exist

        let profile = profile_manager.get_or_create_profile(target_device)?;
        let output_path = config
            .device_profiles
            .backup_base_folder
            .join(&profile.output_folder);

        debug!(
            "Using profile '{}' -> {}",
            profile.name,
            output_path.display()
        );
        output_path
    } else {
        config.output.directory.clone()
    };

    // Create output directory if it doesn't exist
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir)?;
        debug!("Created output directory: {}", output_dir.display());
    }

    // Convert config to extraction config
    // Build duplicate detection config from CLI args or config file
    let duplicate_detection = if detect_duplicates || !compare_folders.is_empty() {
        // CLI args take precedence
        let action = match duplicate_action.as_deref() {
            Some("skip") => crate::core::config::DuplicateAction::Skip,
            Some("rename") => crate::core::config::DuplicateAction::Rename,
            Some("overwrite") => crate::core::config::DuplicateAction::Overwrite,
            _ => config.duplicate_detection.duplicate_action.clone(),
        };

        let folders = if compare_folders.is_empty() {
            config.duplicate_detection.comparison_folders.clone()
        } else {
            compare_folders
        };

        if !folders.is_empty() {
            debug!(
                "Duplicate detection enabled, comparing against {} folder(s)",
                folders.len()
            );
            Some(crate::core::config::DuplicateDetectionConfig {
                enabled: true,
                comparison_folders: folders,
                cache_enabled: config.duplicate_detection.cache_enabled,
                cache_file: config.duplicate_detection.cache_file.clone(),
                duplicate_action: action,
                recursive: config.duplicate_detection.recursive,
                media_only: config.duplicate_detection.media_only,
            })
        } else {
            debug!("Duplicate detection requested but no comparison folders specified");
            None
        }
    } else if config.duplicate_detection.enabled {
        Some(config.duplicate_detection.clone())
    } else {
        None
    };

    let extraction_config = extractor::ExtractionConfig {
        output_dir: output_dir.clone(),
        dcim_only: config.extraction.dcim_only,
        preserve_structure: config.output.preserve_structure,
        skip_existing: config.output.skip_existing,
        duplicate_detection,
        tracking: if config.tracking.enabled {
            Some(config.tracking.clone())
        } else {
            None
        },
    };

    let stats = extractor::extract_photos(target_device, extraction_config, shutdown_flag.clone())?;

    // Log summary at debug level for troubleshooting (user-facing output is in extractor)
    debug!(
        "Extraction finished: {} extracted, {} skipped, {} duplicates, {} errors, {} bytes",
        stats.files_extracted,
        stats.files_skipped,
        stats.duplicates_skipped,
        stats.errors,
        stats.total_bytes
    );

    Ok(())
}

// ============================================================================
// Helper functions
// ============================================================================

/// Select a device from the list based on device ID or return single device
fn select_device<'a>(
    devices: &'a [device::DeviceInfo],
    device_id: &Option<String>,
) -> Result<&'a device::DeviceInfo> {
    if let Some(ref id) = device_id {
        // Try to find by ID or name
        devices
            .iter()
            .find(|d| d.device_id == *id || d.friendly_name.contains(id))
            .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", id))
    } else if devices.len() == 1 {
        Ok(&devices[0])
    } else {
        info!("Multiple devices found. Please specify a device with --device-id:");
        info!("");
        for device in devices {
            info!("  {} (ID: {})", device.friendly_name, device.device_id);
        }
        info!("");
        info!(
            "Example: photo_extraction_tool --device-id \"{}\"",
            devices[0].device_id
        );
        Err(anyhow::anyhow!(
            "Multiple devices found, please specify one"
        ))
    }
}

/// Recursively scan and print device structure
fn scan_recursive(
    content: &device::DeviceContent,
    object_id: &str,
    prefix: &str,
    depth: usize,
    max_depth: usize,
    progress: &ScanProgressTracker,
) -> Result<()> {
    if max_depth > 0 && depth >= max_depth {
        return Ok(());
    }

    let children = content.enumerate_children(object_id)?;

    let mut file_count = 0;
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        let size_str = if child.size > 0 {
            format!(" ({} bytes)", child.size)
        } else {
            String::new()
        };

        let type_str = if child.is_folder { "[DIR]" } else { "[FILE]" };

        info!(
            "{}{}{} {}{}",
            prefix, connector, type_str, child.name, size_str
        );

        if child.is_folder {
            progress.increment_folders();
            let new_prefix = format!("{}{}", prefix, child_prefix);
            scan_recursive(
                content,
                &child.object_id,
                &new_prefix,
                depth + 1,
                max_depth,
                progress,
            )?;
        } else {
            file_count += 1;
        }
    }

    if file_count > 0 {
        progress.add_files(file_count);
    }

    Ok(())
}

// =========================================================================
// TEST COMMAND IMPLEMENTATIONS
// =========================================================================

/// Run all test scenarios
fn test_run_all(
    html_report: bool,
    json_report: bool,
    output: Option<PathBuf>,
    fail_fast: bool,
) -> Result<()> {
    let report_dir = output;

    let config = TestRunnerConfig {
        verbose: true,
        fail_fast,
        html_report,
        json_report,
        report_dir,
        ..Default::default()
    };

    let mut runner = TestRunner::with_config(config);
    let summary = runner.run_all();

    if summary.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Run quick test scenarios
fn test_run_quick(verbose: bool) -> Result<()> {
    let config = TestRunnerConfig {
        verbose,
        ..Default::default()
    };

    let mut runner = TestRunner::with_config(config);
    let summary = runner.run_quick();

    println!(
        "\n✓ Quick tests complete: {}/{} passed",
        summary.passed, summary.total
    );

    if summary.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Run tests filtered by tag
fn test_run_by_tag(tag: &str, verbose: bool) -> Result<()> {
    let config = TestRunnerConfig {
        verbose,
        ..Default::default()
    };

    let mut runner = TestRunner::with_config(config);
    let summary = runner.run_by_tag(tag);

    println!(
        "\n✓ Tests with tag '{}' complete: {}/{} passed",
        tag, summary.passed, summary.total
    );

    if summary.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Run specific scenarios by name
fn test_run_scenarios(scenarios: &[String], verbose: bool) -> Result<()> {
    let config = TestRunnerConfig {
        verbose,
        ..Default::default()
    };

    let names: Vec<&str> = scenarios.iter().map(|s| s.as_str()).collect();

    let mut runner = TestRunner::with_config(config);
    let summary = runner.run_by_names(&names);

    println!(
        "\n✓ Selected tests complete: {}/{} passed",
        summary.passed, summary.total
    );

    if summary.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// List all available test scenarios
fn test_list_scenarios(tag_filter: Option<&str>, detailed: bool) -> Result<()> {
    let scenarios = if let Some(tag) = tag_filter {
        ScenarioLibrary::scenarios_by_tag(tag)
    } else {
        ScenarioLibrary::all_scenarios()
    };

    if scenarios.is_empty() {
        if let Some(tag) = tag_filter {
            println!("No scenarios found with tag '{}'", tag);
        } else {
            println!("No scenarios available");
        }
        return Ok(());
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║              AVAILABLE TEST SCENARIOS                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    if detailed {
        for scenario in &scenarios {
            println!("📋 {}", scenario.name);
            println!("   Description: {}", scenario.description);
            println!("   Tags: {}", scenario.tags.join(", "));
            println!("   Expected files: {}", scenario.expected.files_to_extract);
            println!("   Should succeed: {}", scenario.expected.should_succeed);
            if let Some(ref err) = scenario.expected.expected_error {
                println!("   Expected error: {}", err);
            }
            println!();
        }
    } else {
        for scenario in &scenarios {
            let tags_str = if scenario.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", scenario.tags.join(", "))
            };
            println!(
                "  • {} - {}{}",
                scenario.name, scenario.description, tags_str
            );
        }
        println!();
    }

    println!("Total: {} scenarios", scenarios.len());
    Ok(())
}

/// List all available tags
fn test_list_tags() -> Result<()> {
    let tags = testdb::list_tags();

    println!("\n📌 Available Tags for Filtering:\n");
    for tag in &tags {
        let count = ScenarioLibrary::scenarios_by_tag(tag).len();
        println!("  • {} ({} scenarios)", tag, count);
    }
    println!();
    println!("Use: photo_extraction_tool test run-tag <TAG>");
    Ok(())
}

/// Run interactive test mode
fn test_interactive() -> Result<()> {
    let mut mode = InteractiveTestMode::new();

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║           INTERACTIVE TEST MODE                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    loop {
        println!("\nOptions:");
        println!("  [l] List all scenarios");
        println!("  [q] Run quick tests");
        println!("  [r] Run specific scenario");
        println!("  [a] Run all tests");
        println!("  [i] Show scenario info");
        println!("  [x] Exit\n");

        print!("Enter choice: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        match choice.as_str() {
            "l" => {
                mode.list_scenarios();
            }
            "q" => {
                println!("\nRunning quick tests...\n");
                test_run_quick(true)?;
            }
            "r" => {
                print!("Enter scenario name or number: ");
                io::stdout().flush()?;
                let mut name = String::new();
                io::stdin().read_line(&mut name)?;
                let name = name.trim();

                // Try parsing as number first
                if let Ok(idx) = name.parse::<usize>() {
                    if mode.select_scenario(idx.saturating_sub(1)) {
                        if let Some(result) = mode.run_current() {
                            println!(
                                "\nResult: {} - {}",
                                if result.passed { "PASSED" } else { "FAILED" },
                                result.message
                            );
                        }
                    } else {
                        println!("Invalid scenario number");
                    }
                } else {
                    test_run_scenarios(&[name.to_string()], true)?;
                }
            }
            "a" => {
                println!("\nRunning all tests...\n");
                test_run_all(false, false, None, false)?;
            }
            "i" => {
                print!("Enter scenario name: ");
                io::stdout().flush()?;
                let mut name = String::new();
                io::stdin().read_line(&mut name)?;
                test_scenario_info(name.trim())?;
            }
            "x" | "exit" | "quit" => {
                println!("Exiting interactive mode.");
                break;
            }
            _ => {
                println!("Unknown option: {}", choice);
            }
        }
    }

    Ok(())
}

/// Show information about a specific scenario
fn test_scenario_info(name: &str) -> Result<()> {
    let scenarios = ScenarioLibrary::all_scenarios();
    let scenario = scenarios.iter().find(|s| s.name == name);

    match scenario {
        Some(s) => {
            println!("\n╔══════════════════════════════════════════════════════════════╗");
            println!("║  SCENARIO: {:<50} ║", s.name);
            println!("╚══════════════════════════════════════════════════════════════╝\n");
            println!("Description: {}", s.description);
            println!("Tags: {}", s.tags.join(", "));
            println!("\nDevice Info:");
            println!("  Name: {}", s.device_info.friendly_name);
            println!("  Manufacturer: {}", s.device_info.manufacturer);
            println!("  Model: {}", s.device_info.model);
            println!("  Device ID: {}", s.device_info.device_id);
            println!("\nFile System:");
            println!("  Total objects: {}", s.file_system.object_count());
            println!("  Files: {}", s.file_system.file_count());
            println!("  Folders: {}", s.file_system.folder_count());
            println!("  Total size: {} bytes", s.file_system.total_size());
            println!("\nExpected Results:");
            println!("  Files to extract: {}", s.expected.files_to_extract);
            println!("  Folders: {}", s.expected.folders);
            println!("  Duplicates: {}", s.expected.duplicates);
            println!("  Errors: {}", s.expected.errors);
            println!("  Should succeed: {}", s.expected.should_succeed);
            if let Some(ref err) = s.expected.expected_error {
                println!("  Expected error: {}", err);
            }
            println!();
        }
        None => {
            println!("Scenario '{}' not found.", name);
            println!("\nAvailable scenarios:");
            for s in &scenarios {
                println!("  • {}", s.name);
            }
        }
    }

    Ok(())
}

/// Simulate extraction from a mock device
fn test_simulate_extract(
    scenario_name: &str,
    output: &PathBuf,
    verbose: bool,
    _simulate_errors: bool,
) -> Result<()> {
    let scenarios = ScenarioLibrary::all_scenarios();
    let scenario = scenarios.into_iter().find(|s| s.name == scenario_name);

    match scenario {
        Some(s) => {
            println!("\n🔧 Simulating extraction from: {}", s.name);
            println!("   Device: {}", s.device_info.friendly_name);
            println!("   Output: {}", output.display());
            println!();

            // Create output directory
            fs::create_dir_all(output)?;

            // Simulate extraction by writing mock files
            let mut extracted = 0;
            let mut errors = 0;

            for obj in s.file_system.all_objects() {
                if obj.is_folder() {
                    continue;
                }

                let file_path = output.join(&obj.name());

                if let Some(ref content) = obj.content {
                    match fs::write(&file_path, content) {
                        Ok(_) => {
                            extracted += 1;
                            if verbose {
                                println!("  ✓ Extracted: {} ({} bytes)", obj.name(), obj.size());
                            }
                        }
                        Err(e) => {
                            errors += 1;
                            if verbose {
                                println!("  ✗ Error: {} - {}", obj.name(), e);
                            }
                        }
                    }
                }
            }

            println!("\n✓ Simulation complete:");
            println!("  Files extracted: {}", extracted);
            println!("  Errors: {}", errors);
            println!("  Output directory: {}", output.display());
        }
        None => {
            println!("Scenario '{}' not found.", scenario_name);
            println!("Use 'test list-scenarios' to see available scenarios.");
        }
    }

    Ok(())
}

/// Generate mock test data files
fn test_generate_data(
    output: &PathBuf,
    count: usize,
    types: &str,
    include_duplicates: bool,
    seed: Option<u64>,
) -> Result<()> {
    println!("\n🔧 Generating mock test data...");
    println!("   Output: {}", output.display());
    println!("   Count: {} files", count);
    println!("   Types: {}", types);
    println!();

    fs::create_dir_all(output)?;

    let extensions: Vec<&str> = if types == "all" {
        vec!["jpg", "heic", "png", "mov", "mp4"]
    } else {
        types.split(',').map(|s| s.trim()).collect()
    };

    let base_seed = seed.unwrap_or(42);
    let files_per_type = count / extensions.len().max(1);

    let mut total_generated = 0;

    for (ext_idx, ext) in extensions.iter().enumerate() {
        for i in 0..files_per_type {
            let file_seed = base_seed + (ext_idx * 10000 + i) as u64;
            let size = match *ext {
                "mov" | "mp4" => 10 * 1024 * 1024, // 10MB for videos
                "heic" => 2 * 1024 * 1024,         // 2MB for HEIC
                _ => 1024 * 1024,                  // 1MB for others
            };

            let content = MockDataGenerator::generate_for_extension_with_seed(ext, size, file_seed);
            let filename = format!("TEST_{:05}.{}", total_generated, ext.to_uppercase());
            let file_path = output.join(&filename);

            fs::write(&file_path, &content)?;
            total_generated += 1;

            // Create duplicate if requested (every 10th file)
            if include_duplicates && i % 10 == 0 && i > 0 {
                let dup_filename =
                    format!("TEST_{:05}_DUP.{}", total_generated, ext.to_uppercase());
                let dup_path = output.join(&dup_filename);
                fs::write(&dup_path, &content)?;
                total_generated += 1;
            }
        }
    }

    println!(
        "✓ Generated {} files in {}",
        total_generated,
        output.display()
    );

    if include_duplicates {
        let dup_count = files_per_type / 10 * extensions.len();
        println!("  Including {} intentional duplicates", dup_count);
    }

    Ok(())
}

/// Benchmark mock device performance
fn test_benchmark_mock(file_count: usize, iterations: usize) -> Result<()> {
    use std::time::Instant;

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║              MOCK DEVICE BENCHMARK                           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Configuration:");
    println!("  File count: {}", file_count);
    println!("  Iterations: {}", iterations);
    println!();

    let mut creation_times = Vec::new();
    let mut enumeration_times = Vec::new();
    let mut read_times = Vec::new();

    for iter in 0..iterations {
        print!("  Iteration {}/{}... ", iter + 1, iterations);
        io::stdout().flush()?;

        // Benchmark file system creation
        let start = Instant::now();
        let mut fs = testdb::MockFileSystem::new();

        fs.add_object(testdb::MockObject::folder(
            "internal",
            "DEVICE",
            "Internal Storage",
        ));
        fs.add_object(testdb::MockObject::folder("dcim", "internal", "DCIM"));
        fs.add_object(testdb::MockObject::folder("100apple", "dcim", "100APPLE"));

        for i in 0..file_count {
            let content = MockDataGenerator::generate_jpeg_header(1024);
            fs.add_object(testdb::MockObject::file(
                &format!("file_{}", i),
                "100apple",
                &format!("IMG_{:05}.JPG", i),
                content,
            ));
        }
        creation_times.push(start.elapsed());

        // Benchmark enumeration
        let start = Instant::now();
        let _ = fs.get_children("100apple");
        enumeration_times.push(start.elapsed());

        // Benchmark file reads
        let start = Instant::now();
        for i in 0..file_count.min(100) {
            let _ = fs.read_file(&format!("file_{}", i));
        }
        read_times.push(start.elapsed());

        println!("done");
    }

    // Calculate averages
    let avg_creation: f64 =
        creation_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / iterations as f64;
    let avg_enum: f64 = enumeration_times
        .iter()
        .map(|d| d.as_secs_f64())
        .sum::<f64>()
        / iterations as f64;
    let avg_read: f64 = read_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / iterations as f64;

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                      RESULTS                                 ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║  File system creation ({} files):                            ║",
        file_count
    );
    println!(
        "║    Average: {:.4}s                                          ║",
        avg_creation
    );
    println!(
        "║    Per file: {:.4}ms                                         ║",
        avg_creation * 1000.0 / file_count as f64
    );
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Enumeration:                                                ║");
    println!(
        "║    Average: {:.6}s                                          ║",
        avg_enum
    );
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  File reads (100 files):                                     ║");
    println!(
        "║    Average: {:.4}s                                          ║",
        avg_read
    );
    println!(
        "║    Per file: {:.4}ms                                         ║",
        avg_read * 1000.0 / 100.0
    );
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    Ok(())
}

/// Recursively scan for benchmark statistics
#[allow(clippy::too_many_arguments)]
fn benchmark_scan_recursive(
    content: &device::DeviceContent,
    object_id: &str,
    path: &str,
    dcim_only: bool,
    progress: &BenchmarkProgress,
    total_folders: &mut usize,
    total_files: &mut usize,
    media_files: &mut usize,
) -> Result<()> {
    *total_folders += 1;
    progress.update(*total_folders, *total_files, *media_files);

    let children = match content.enumerate_children(object_id) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to enumerate '{}': {}", path, e);
            return Ok(());
        }
    };

    // If dcim_only, skip non-DCIM paths (but always scan to find DCIM)
    let is_dcim_path = path.to_uppercase().contains("DCIM");

    for child in children {
        let child_path = format!("{}/{}", path, child.name);

        if child.is_folder {
            // If dcim_only and we're not in DCIM yet, only recurse if this might lead to DCIM
            if dcim_only && !is_dcim_path {
                // Check if this is the DCIM folder or might contain it
                let child_upper = child.name.to_uppercase();
                if child_upper == "DCIM"
                    || child_upper == "INTERNAL STORAGE"
                    || child_upper.contains("STORAGE")
                {
                    benchmark_scan_recursive(
                        content,
                        &child.object_id,
                        &child_path,
                        dcim_only,
                        progress,
                        total_folders,
                        total_files,
                        media_files,
                    )?;
                }
            } else {
                benchmark_scan_recursive(
                    content,
                    &child.object_id,
                    &child_path,
                    dcim_only,
                    progress,
                    total_folders,
                    total_files,
                    media_files,
                )?;
            }
        } else {
            *total_files += 1;

            // Check if it's a media file
            let ext = std::path::Path::new(&child.name)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            let photo_exts = [
                "jpg", "jpeg", "png", "heic", "heif", "gif", "webp", "raw", "dng", "tiff", "tif",
                "bmp",
            ];
            let video_exts = ["mov", "mp4", "m4v", "avi", "3gp"];

            if photo_exts.contains(&ext.as_str()) || video_exts.contains(&ext.as_str()) {
                *media_files += 1;
            }

            // Update progress every 100 files
            if (*total_files).is_multiple_of(100) {
                progress.update(*total_folders, *total_files, *media_files);
            }
        }
    }

    Ok(())
}

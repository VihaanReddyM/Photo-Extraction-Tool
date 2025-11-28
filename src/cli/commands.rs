//! Command handler implementations
//!
//! This module contains the implementation of all CLI commands.

use crate::cli::progress::{BenchmarkProgress, ScanProgressTracker};
use crate::cli::{Args, Commands, TestCommands};
use crate::core::config::{
    get_config_path, init_config, open_config_in_editor, Config, TrackingConfig,
};
use crate::core::extractor::{self, ExtractionStats};
use crate::core::setup::run_setup_wizard;
use crate::core::tracking::scan_for_profiles;
use crate::device::traits::{DeviceContentTrait, DeviceManagerTrait};
use crate::device::{self, DeviceInfo, ProfileManager};
use crate::testdb::{
    self, InteractiveTestMode, MockDataGenerator, ScenarioLibrary, TestRunner, TestRunnerConfig,
};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Result of extraction from a single device (for parallel extraction reporting)
#[derive(Debug, Clone)]
struct DeviceExtractionResult {
    /// Device friendly name
    device_name: String,
    /// Profile name (if available)
    profile_name: Option<String>,
    /// Output directory used
    output_dir: PathBuf,
    /// Extraction statistics
    stats: Option<ExtractionStats>,
    /// Error message if extraction failed
    error: Option<String>,
    /// Duration of extraction
    duration: Duration,
}

/// Shared progress state for parallel extraction
struct SharedProgress {
    /// Total files to process across all devices
    total_files: AtomicUsize,
    /// Files processed so far
    files_processed: AtomicUsize,
    /// Total bytes transferred
    bytes_transferred: AtomicU64,
    /// Number of devices completed
    devices_completed: AtomicUsize,
    /// Total number of devices
    total_devices: usize,
}

impl SharedProgress {
    fn new(total_devices: usize) -> Self {
        Self {
            total_files: AtomicUsize::new(0),
            files_processed: AtomicUsize::new(0),
            bytes_transferred: AtomicU64::new(0),
            devices_completed: AtomicUsize::new(0),
            total_devices,
        }
    }

    fn add_total_files(&self, count: usize) {
        self.total_files.fetch_add(count, Ordering::Relaxed);
    }

    fn add_processed(&self, count: usize, bytes: u64) {
        self.files_processed.fetch_add(count, Ordering::Relaxed);
        self.bytes_transferred.fetch_add(bytes, Ordering::Relaxed);
    }

    fn mark_device_complete(&self) {
        self.devices_completed.fetch_add(1, Ordering::Relaxed);
    }

    fn get_progress(&self) -> (usize, usize, u64, usize) {
        (
            self.files_processed.load(Ordering::Relaxed),
            self.total_files.load(Ordering::Relaxed),
            self.bytes_transferred.load(Ordering::Relaxed),
            self.devices_completed.load(Ordering::Relaxed),
        )
    }
}

/// Run the appropriate command based on CLI arguments
///
/// If initial setup is required (no backup directory configured), this will
/// run the setup wizard first before proceeding with extraction commands.
pub fn run_command(args: &Args, config: &Config, shutdown_flag: Arc<AtomicBool>) -> Result<()> {
    // Check if setup is needed for extraction commands
    let mut config = check_and_run_setup_if_needed(args, config)?;

    // If --all-devices is passed, override apple_only setting
    if args.all_devices {
        config.device.apple_only = false;
    }

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

            extract_photos_with_args(
                &config,
                shutdown_flag,
                use_detect,
                folders,
                action,
                args.all_devices,
            )?;
        }
        None => {
            // Use global args when no subcommand specified
            extract_photos_with_args(
                &config,
                shutdown_flag,
                args.detect_duplicates,
                args.compare_folders.clone(),
                args.duplicate_action.clone(),
                args.all_devices,
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
    info!("");
    info!("[android]");
    info!(
        "  preserve_structure = {}",
        config.android.preserve_structure
    );
    info!("  include_camera = {}", config.android.include_camera);
    info!(
        "  include_screenshots = {}",
        config.android.include_screenshots
    );
    info!("  include_pictures = {}", config.android.include_pictures);
    info!("  include_downloads = {}", config.android.include_downloads);
    info!(
        "  exclude_cache_folders = {}",
        config.android.exclude_cache_folders
    );
    info!("");
    info!("  # App-specific folders");
    info!("  include_whatsapp = {}", config.android.include_whatsapp);
    info!("  include_telegram = {}", config.android.include_telegram);
    info!("  include_instagram = {}", config.android.include_instagram);
    info!("  include_facebook = {}", config.android.include_facebook);
    info!("  include_snapchat = {}", config.android.include_snapchat);
    info!("  include_tiktok = {}", config.android.include_tiktok);
    info!("  include_signal = {}", config.android.include_signal);
    info!("  include_viber = {}", config.android.include_viber);
    info!("");
    info!(
        "  additional_folders = {:?}",
        config.android.additional_folders
    );
    info!("  exclude_folders = {:?}", config.android.exclude_folders);
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
        let device_type = device.device_type();
        info!("[{}] {} ({})", i + 1, device.friendly_name, device_type);
        info!("    Type: {}", device_type.display_name());
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
    all_devices: bool,
) -> Result<()> {
    // Initialize COM library (required for WPD)
    let _com_guard = device::initialize_com()?;

    // Create device manager
    let manager = device::DeviceManager::new()?;

    debug!("Scanning for connected devices...");

    // --all-devices flag overrides apple_only config
    let use_apple_only = config.device.apple_only && !all_devices;

    let devices = if use_apple_only {
        manager.enumerate_apple_devices()?
    } else {
        manager.enumerate_all_devices()?
    };

    if devices.is_empty() {
        println!();
        println!("  ✗ No portable devices found.");
        println!();
        if use_apple_only {
            println!("  Make sure your iPhone is:");
            println!("    1. Connected via USB cable");
            println!("    2. Unlocked");
            println!("    3. Trusting this computer (tap 'Trust' when prompted)");
            println!();
            println!("  Use 'list' command to see available devices");
            println!("  Use '--all-devices' to see all portable devices (not just Apple)");
        } else {
            println!("  Make sure your device is:");
            println!("    1. Connected via USB cable");
            println!("    2. Unlocked");
            println!("    3. iOS: Tap 'Trust' when prompted");
            println!("    4. Android: Select 'File Transfer' / 'MTP' mode");
            println!();
            println!("  Use 'list' command to see available devices");
        }
        return Ok(());
    }

    // Select device(s) - may return multiple for parallel extraction
    let selected_devices = select_devices(&devices, &config.device.device_id)?;

    if selected_devices.is_empty() {
        return Ok(());
    }

    // Build duplicate detection config
    let duplicate_detection =
        build_duplicate_config(config, detect_duplicates, compare_folders, duplicate_action);

    // Extract from selected device(s)
    if selected_devices.len() == 1 {
        // Single device - extract directly
        extract_from_single_device(
            &selected_devices[0],
            config,
            duplicate_detection,
            shutdown_flag,
        )?;
    } else {
        // Multiple devices - ask about parallel extraction
        extract_from_multiple_devices(
            &selected_devices,
            config,
            duplicate_detection,
            shutdown_flag,
        )?;
    }

    Ok(())
}

/// Build duplicate detection configuration from CLI args and config
fn build_duplicate_config(
    config: &Config,
    detect_duplicates: bool,
    compare_folders: Vec<PathBuf>,
    duplicate_action: Option<String>,
) -> Option<crate::core::config::DuplicateDetectionConfig> {
    if detect_duplicates || !compare_folders.is_empty() {
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
    }
}

/// Extract from a single device
fn extract_from_single_device(
    device: &DeviceInfo,
    config: &Config,
    duplicate_detection: Option<crate::core::config::DuplicateDetectionConfig>,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<ExtractionStats> {
    extract_from_single_device_impl(device, config, duplicate_detection, shutdown_flag, false)
}

/// Internal implementation of single device extraction with quiet mode option
fn extract_from_single_device_impl(
    device: &DeviceInfo,
    config: &Config,
    duplicate_detection: Option<crate::core::config::DuplicateDetectionConfig>,
    shutdown_flag: Arc<AtomicBool>,
    quiet: bool,
) -> Result<ExtractionStats> {
    // Determine output directory
    let output_dir = get_output_dir_for_device(device, config)?;

    // Create output directory if it doesn't exist
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir)?;
        debug!("Created output directory: {}", output_dir.display());
    }

    // Check if this is an Android device and use Android-specific config
    let android_config = if device.device_type().is_android() {
        debug!("Detected Android device, using Android extraction config");
        Some(config.android.clone())
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
        quiet,
        android_config,
    };

    let stats = extractor::extract_photos(device, extraction_config, shutdown_flag)?;

    debug!(
        "Extraction finished: {} extracted, {} skipped, {} duplicates, {} errors, {} bytes",
        stats.files_extracted,
        stats.files_skipped,
        stats.duplicates_skipped,
        stats.errors,
        stats.total_bytes
    );

    Ok(stats)
}

/// Extract from a single device with a pre-resolved output directory (for parallel extraction)
fn extract_with_output_dir(
    device: &DeviceInfo,
    output_dir: PathBuf,
    config: &Config,
    duplicate_detection: Option<crate::core::config::DuplicateDetectionConfig>,
    shutdown_flag: Arc<AtomicBool>,
    progress: Option<Arc<SharedProgress>>,
) -> Result<ExtractionStats> {
    // Create output directory if it doesn't exist
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir)?;
        debug!("Created output directory: {}", output_dir.display());
    }

    // Check if this is an Android device and use Android-specific config
    let android_config = if device.device_type().is_android() {
        debug!("Detected Android device, using Android extraction config");
        Some(config.android.clone())
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
        quiet: true, // Always quiet for parallel
        android_config,
    };

    // Create progress callback if we have shared progress
    let progress_callback: Option<extractor::ProgressCallback> = progress.as_ref().map(|p| {
        let progress_clone = Arc::clone(p);
        Box::new(move |files: usize, bytes: u64| {
            progress_clone.add_processed(files, bytes);
        }) as extractor::ProgressCallback
    });

    // Create total files callback if we have shared progress
    let total_files_callback: Option<extractor::TotalFilesCallback> = progress.map(|p| {
        Box::new(move |total: usize| {
            p.add_total_files(total);
        }) as extractor::TotalFilesCallback
    });

    let stats = extractor::extract_photos_with_progress(
        device,
        extraction_config,
        shutdown_flag,
        progress_callback,
        total_files_callback,
    )?;

    Ok(stats)
}

/// Get the output directory for a device (using profiles if enabled)
fn get_output_dir_for_device(device: &DeviceInfo, config: &Config) -> Result<PathBuf> {
    if config.device_profiles.enabled {
        let mut profile_manager = ProfileManager::new(&config.device_profiles);
        profile_manager.load().ok();

        let profile = profile_manager.get_or_create_profile(device)?;
        let output_path = config
            .device_profiles
            .backup_base_folder
            .join(&profile.output_folder);

        debug!(
            "Using profile '{}' -> {}",
            profile.name,
            output_path.display()
        );
        Ok(output_path)
    } else {
        Ok(config.output.directory.clone())
    }
}

/// Extract from multiple devices with option for parallel extraction
fn extract_from_multiple_devices(
    devices: &[DeviceInfo],
    config: &Config,
    duplicate_detection: Option<crate::core::config::DuplicateDetectionConfig>,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<()> {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              📱 Multiple Devices Selected                        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    for (i, device) in devices.iter().enumerate() {
        println!("  [{}] {}", i + 1, device.friendly_name);
    }
    println!();
    println!("  How would you like to extract?");
    println!();
    println!("    [S] Sequential - Extract one device at a time (safer)");
    println!("    [P] Parallel   - Extract all devices simultaneously (faster)");
    println!("    [C] Cancel     - Don't extract");
    println!();
    print!("  Your choice [S/p/c]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim().to_lowercase();

    match choice.as_str() {
        "" | "s" | "sequential" => {
            // Sequential extraction
            println!();
            println!("  Starting sequential extraction...");
            println!();

            let mut total_stats = ExtractionStats::default();

            for (i, device) in devices.iter().enumerate() {
                if shutdown_flag.load(Ordering::SeqCst) {
                    println!("  ⚠ Extraction interrupted by user");
                    break;
                }

                println!("  ─────────────────────────────────────────");
                println!(
                    "  📱 Device {}/{}: {}",
                    i + 1,
                    devices.len(),
                    device.friendly_name
                );
                println!();

                match extract_from_single_device(
                    device,
                    config,
                    duplicate_detection.clone(),
                    shutdown_flag.clone(),
                ) {
                    Ok(stats) => {
                        total_stats.files_extracted += stats.files_extracted;
                        total_stats.files_skipped += stats.files_skipped;
                        total_stats.duplicates_skipped += stats.duplicates_skipped;
                        total_stats.errors += stats.errors;
                        total_stats.total_bytes += stats.total_bytes;
                    }
                    Err(e) => {
                        println!("  ✗ Error extracting from {}: {}", device.friendly_name, e);
                        total_stats.errors += 1;
                    }
                }
            }

            // Print combined summary
            if devices.len() > 1 {
                println!();
                println!("  ═══════════════════════════════════════════");
                println!("  📊 Combined Results ({} devices):", devices.len());
                println!("     Total extracted:  {}", total_stats.files_extracted);
                println!("     Total skipped:    {}", total_stats.files_skipped);
                if total_stats.duplicates_skipped > 0 {
                    println!("     Total duplicates: {}", total_stats.duplicates_skipped);
                }
                if total_stats.errors > 0 {
                    println!("     Total errors:     {}", total_stats.errors);
                }
                println!(
                    "     Total size:       {}",
                    format_bytes(total_stats.total_bytes)
                );
                println!();
            }
        }
        "p" | "parallel" => {
            // Parallel extraction

            // ================================================================
            // STEP 1: Pre-resolve ALL profiles/output directories BEFORE
            // spawning any threads. This ensures no interactive prompts
            // happen during parallel execution.
            // ================================================================
            println!();
            println!("  📋 Preparing devices for parallel extraction...");
            println!();

            let mut devices_ready: Vec<(DeviceInfo, String, PathBuf)> = Vec::new();

            for (i, device) in devices.iter().enumerate() {
                println!(
                    "  [{}/{}] Setting up: {}",
                    i + 1,
                    devices.len(),
                    device.friendly_name
                );

                // This will prompt for profile if needed (sequentially, before parallel starts)
                match get_output_dir_for_device(device, config) {
                    Ok(output_dir) => {
                        let profile_name = get_device_profile_hint(device, config)
                            .unwrap_or_else(|| device.friendly_name.clone());
                        println!("        → {} ({})", profile_name, output_dir.display());
                        devices_ready.push((device.clone(), profile_name, output_dir));
                    }
                    Err(e) => {
                        println!("        ✗ Failed to setup: {}", e);
                        return Err(anyhow::anyhow!(
                            "Failed to setup device {}: {}",
                            device.friendly_name,
                            e
                        ));
                    }
                }
            }

            println!();
            println!(
                "  🚀 Starting parallel extraction of {} devices...",
                devices_ready.len()
            );
            println!();

            // Create shared progress tracker
            let shared_progress = Arc::new(SharedProgress::new(devices_ready.len()));

            // Create progress bar
            let progress_bar = ProgressBar::new(100);
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("  {spinner:.green} [{bar:40.cyan/blue}] {pos}% | {msg}")
                    .unwrap()
                    .progress_chars("━━╾─"),
            );
            progress_bar.enable_steady_tick(Duration::from_millis(100));

            let parallel_start = Instant::now();

            let config_clone = config.clone();
            let dup_config = duplicate_detection.clone();

            // Spawn threads for each device (quiet mode - no console output)
            let handles: Vec<_> = devices_ready
                .into_iter()
                .map(|(device, profile_name, output_dir)| {
                    let cfg = config_clone.clone();
                    let dup = dup_config.clone();
                    let flag = shutdown_flag.clone();
                    let device_name = device.friendly_name.clone();
                    let output_dir_clone = output_dir.clone();
                    let progress = Arc::clone(&shared_progress);

                    thread::spawn(move || {
                        let start = Instant::now();

                        // Each thread needs its own COM initialization
                        let _com = match device::initialize_com() {
                            Ok(guard) => guard,
                            Err(e) => {
                                progress.mark_device_complete();
                                return DeviceExtractionResult {
                                    device_name,
                                    profile_name: Some(profile_name),
                                    output_dir,
                                    stats: None,
                                    error: Some(format!("COM init failed: {}", e)),
                                    duration: start.elapsed(),
                                };
                            }
                        };

                        match extract_with_output_dir(
                            &device,
                            output_dir_clone,
                            &cfg,
                            dup,
                            flag,
                            Some(Arc::clone(&progress)),
                        ) {
                            Ok(stats) => {
                                progress.mark_device_complete();
                                DeviceExtractionResult {
                                    device_name,
                                    profile_name: Some(profile_name),
                                    output_dir,
                                    stats: Some(stats),
                                    error: None,
                                    duration: start.elapsed(),
                                }
                            }
                            Err(e) => {
                                progress.mark_device_complete();
                                DeviceExtractionResult {
                                    device_name,
                                    profile_name: Some(profile_name),
                                    output_dir,
                                    stats: None,
                                    error: Some(format!("{}", e)),
                                    duration: start.elapsed(),
                                }
                            }
                        }
                    })
                })
                .collect();

            // Update progress bar while threads are running
            let total_devices = shared_progress.total_devices;
            loop {
                let (processed, total, bytes, completed) = shared_progress.get_progress();

                if completed >= total_devices {
                    break;
                }

                let percent = if total > 0 {
                    (processed * 100 / total).min(99)
                } else {
                    0
                };

                progress_bar.set_position(percent as u64);
                progress_bar.set_message(format!(
                    "{}/{} files | {} | {}/{} devices",
                    processed,
                    total,
                    format_bytes(bytes),
                    completed,
                    total_devices
                ));

                thread::sleep(Duration::from_millis(100));
            }

            // Collect results
            let mut results: Vec<DeviceExtractionResult> = Vec::new();
            for handle in handles {
                match handle.join() {
                    Ok(result) => results.push(result),
                    Err(_) => results.push(DeviceExtractionResult {
                        device_name: "Unknown".to_string(),
                        profile_name: None,
                        output_dir: PathBuf::new(),
                        stats: None,
                        error: Some("Thread panicked".to_string()),
                        duration: Duration::ZERO,
                    }),
                }
            }

            let total_duration = parallel_start.elapsed();

            // Final progress update
            progress_bar.set_position(100);
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("  ✓ [{bar:40.green}] 100% | Complete")
                    .unwrap()
                    .progress_chars("━━━"),
            );
            progress_bar.finish();

            // Print consolidated results
            println!();
            println!("  ╔══════════════════════════════════════════════════════════════════╗");
            println!("  ║              📊 Parallel Extraction Complete                     ║");
            println!("  ╚══════════════════════════════════════════════════════════════════╝");
            println!();

            // Per-device results
            let mut total_stats = ExtractionStats::default();
            let mut success_count = 0;
            let mut error_count = 0;

            for (i, result) in results.iter().enumerate() {
                let display_name = result.profile_name.as_ref().unwrap_or(&result.device_name);

                println!("  ┌─ Device {}: {}", i + 1, display_name);

                if let Some(ref stats) = result.stats {
                    success_count += 1;
                    total_stats.files_extracted += stats.files_extracted;
                    total_stats.files_skipped += stats.files_skipped;
                    total_stats.duplicates_skipped += stats.duplicates_skipped;
                    total_stats.errors += stats.errors;
                    total_stats.total_bytes += stats.total_bytes;

                    println!("  │  📁 {}", result.output_dir.display());
                    println!(
                        "  │  ✓ Extracted: {}  Skipped: {}  Size: {}",
                        stats.files_extracted,
                        stats.files_skipped,
                        format_bytes(stats.total_bytes)
                    );
                    println!("  │  ⏱ Duration: {:.1}s", result.duration.as_secs_f64());
                } else if let Some(ref err) = result.error {
                    error_count += 1;
                    println!("  │  ✗ Error: {}", err);
                }
                println!("  └─────────────────────────────────────────");
                println!();
            }

            // Combined totals
            println!("  ═══════════════════════════════════════════");
            println!("  📈 Combined Results:");
            println!(
                "     Devices succeeded:  {}/{}",
                success_count,
                results.len()
            );
            if error_count > 0 {
                println!("     Devices failed:     {}", error_count);
            }
            println!("     Total extracted:    {}", total_stats.files_extracted);
            println!("     Total skipped:      {}", total_stats.files_skipped);
            if total_stats.duplicates_skipped > 0 {
                println!(
                    "     Total duplicates:   {}",
                    total_stats.duplicates_skipped
                );
            }
            println!(
                "     Total size:         {}",
                format_bytes(total_stats.total_bytes)
            );
            println!(
                "     Total time:         {:.1}s",
                total_duration.as_secs_f64()
            );
            println!();
        }
        "c" | "cancel" | "q" | "quit" => {
            println!();
            println!("  Extraction cancelled.");
            println!();
        }
        _ => {
            println!();
            println!("  Invalid choice. Extraction cancelled.");
            println!();
        }
    }

    Ok(())
}

// ============================================================================
// Helper functions
// ============================================================================

/// Select device(s) from the list - supports interactive multi-selection
fn select_devices(devices: &[DeviceInfo], device_id: &Option<String>) -> Result<Vec<DeviceInfo>> {
    if let Some(ref id) = device_id {
        // Specific device requested - try to find by ID or name
        let device = devices
            .iter()
            .find(|d| d.device_id == *id || d.friendly_name.contains(id))
            .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", id))?;
        Ok(vec![device.clone()])
    } else if devices.len() == 1 {
        // Only one device - use it
        Ok(vec![devices[0].clone()])
    } else {
        // Multiple devices - show interactive selection
        select_devices_interactive(devices)
    }
}

/// Interactive device selection menu
fn select_devices_interactive(devices: &[DeviceInfo]) -> Result<Vec<DeviceInfo>> {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              📱 Multiple Devices Detected                        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    // Try to get profile names for better display
    // We need config for this, so we load it here
    let config = Config::load_default().unwrap_or_default();

    for (i, device) in devices.iter().enumerate() {
        // Try to identify the device by any existing profile
        let profile_hint = get_device_profile_hint(device, &config);
        if let Some(hint) = profile_hint {
            println!("  [{}] {} → {}", i + 1, device.friendly_name, hint);
        } else {
            println!("  [{}] {} (new device)", i + 1, device.friendly_name);
        }
    }

    println!();
    println!("  [A] Extract from ALL devices");
    println!("  [0] Cancel");
    println!();
    print!("  Select device(s) [1-{}, A, or 0]: ", devices.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim().to_lowercase();

    match choice.as_str() {
        "0" | "q" | "quit" | "cancel" => {
            println!();
            println!("  Extraction cancelled.");
            Ok(vec![])
        }
        "a" | "all" => {
            // Return all devices
            Ok(devices.to_vec())
        }
        _ => {
            // Try to parse as number(s)
            // Support formats: "1", "1,2", "1 2", "1-3"
            let mut selected = Vec::new();

            // Handle range format (e.g., "1-3")
            if choice.contains('-') && !choice.starts_with('-') {
                let parts: Vec<&str> = choice.split('-').collect();
                if parts.len() == 2 {
                    if let (Ok(start), Ok(end)) =
                        (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                    {
                        for i in start..=end {
                            if i >= 1 && i <= devices.len() {
                                selected.push(devices[i - 1].clone());
                            }
                        }
                    }
                }
            } else {
                // Handle comma or space separated (e.g., "1,2" or "1 2")
                let nums: Vec<&str> = choice.split(|c| c == ',' || c == ' ').collect();
                for num_str in nums {
                    let num_str = num_str.trim();
                    if num_str.is_empty() {
                        continue;
                    }
                    if let Ok(num) = num_str.parse::<usize>() {
                        if num >= 1 && num <= devices.len() {
                            let device = &devices[num - 1];
                            // Avoid duplicates
                            if !selected
                                .iter()
                                .any(|d: &DeviceInfo| d.device_id == device.device_id)
                            {
                                selected.push(device.clone());
                            }
                        }
                    }
                }
            }

            if selected.is_empty() {
                println!();
                println!(
                    "  ⚠ Invalid selection. Please enter a number between 1 and {}.",
                    devices.len()
                );
                println!();
                // Retry
                select_devices_interactive(devices)
            } else {
                Ok(selected)
            }
        }
    }
}

/// Try to get a profile hint for a device (e.g., the profile name)
fn get_device_profile_hint(device: &DeviceInfo, config: &Config) -> Option<String> {
    // Try to load existing profiles to find a name for this device
    if config.device_profiles.enabled {
        let mut profile_manager = ProfileManager::new(&config.device_profiles);
        if profile_manager.load().is_ok() {
            // Check if we have a profile for this device
            if let Some(profile) = profile_manager.get_profile(&device.device_id) {
                return Some(profile.name.clone());
            }
        }
    }

    // Also scan for extraction profiles in the backup folder
    if !config
        .device_profiles
        .backup_base_folder
        .as_os_str()
        .is_empty()
    {
        use crate::core::tracking::scan_for_profiles;
        let profiles = scan_for_profiles(
            &config.device_profiles.backup_base_folder,
            &config.tracking.tracking_filename,
        );

        // Check if any profile matches this device ID
        for profile in profiles {
            if profile.device_id == device.device_id {
                return Some(profile.friendly_name);
            }
        }
    }

    None
}

/// Select a single device from the list (legacy helper for other commands)
fn select_device<'a>(
    devices: &'a [device::DeviceInfo],
    device_id: &Option<String>,
) -> Result<&'a device::DeviceInfo> {
    if let Some(ref id) = device_id {
        devices
            .iter()
            .find(|d| d.device_id == *id || d.friendly_name.contains(id))
            .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", id))
    } else if devices.len() == 1 {
        Ok(&devices[0])
    } else {
        // For non-extraction commands, just pick the first one with a note
        println!();
        println!(
            "  ℹ Multiple devices found, using first one: {}",
            devices[0].friendly_name
        );
        println!("    Use --device-id to specify a different device.");
        println!();
        Ok(&devices[0])
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

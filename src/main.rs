mod config;
mod device;
mod duplicate;
mod error;
mod extractor;
mod profiles;
mod tracking;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use env_logger::Builder;
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, info, warn, LevelFilter};
use profiles::ProfileManager;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// A fast, reliable tool to extract photos from iPhone/iPad on Windows
#[derive(Parser, Debug)]
#[command(name = "photo_extraction_tool")]
#[command(author = "Vihaan Reddy M")]
#[command(version = "1.0.0")]
#[command(about = "Extract photos from iPhone/iPad connected to Windows PC", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to configuration file
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Output directory for extracted photos (overrides config)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Device ID to extract from (overrides config)
    #[arg(short, long)]
    device_id: Option<String>,

    /// Only extract photos from DCIM folder (overrides config)
    #[arg(long)]
    dcim_only: Option<bool>,

    /// Preserve folder structure from device (overrides config)
    #[arg(short, long)]
    preserve_structure: Option<bool>,

    /// Skip files that already exist in destination (overrides config)
    #[arg(short, long)]
    skip_existing: Option<bool>,

    /// List all portable devices (not just Apple devices)
    #[arg(long)]
    all_devices: bool,

    /// Log level: error, warn, info, debug, trace (overrides config)
    #[arg(short, long)]
    log_level: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Extract photos from the connected device
    Extract,

    /// List connected devices
    List {
        /// Show all portable devices, not just Apple devices
        #[arg(long)]
        all: bool,
    },

    /// Generate a default configuration file
    GenerateConfig {
        /// Output path for the config file
        #[arg(short, long, default_value = "./config.toml")]
        output: PathBuf,
    },

    /// Show current configuration
    ShowConfig,

    /// Scan device and show folder structure (for debugging)
    Scan {
        /// Maximum depth to scan (0 = unlimited)
        #[arg(short, long, default_value = "3")]
        depth: usize,
    },

    /// List all configured device profiles
    ListProfiles,

    /// Remove a device profile
    RemoveProfile {
        /// Name or partial device ID to remove
        #[arg(short, long)]
        name: String,
    },

    /// Benchmark scan performance - shows detailed discovery statistics
    BenchmarkScan {
        /// Whether to only scan DCIM folder (faster)
        #[arg(long, default_value = "true")]
        dcim_only: bool,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load configuration
    let mut config = if let Some(ref config_path) = args.config {
        match Config::load(config_path) {
            Ok(cfg) => {
                // Can't use log here yet, it's not initialized
                cfg
            }
            Err(e) => {
                eprintln!("Warning: Failed to load config file: {}", e);
                Config::default()
            }
        }
    } else {
        Config::load_default().unwrap_or_default()
    };

    // Apply CLI overrides to config
    if let Some(ref output) = args.output {
        config.output.directory = output.clone();
    }
    if let Some(ref device_id) = args.device_id {
        config.device.device_id = Some(device_id.clone());
    }
    if let Some(dcim_only) = args.dcim_only {
        config.extraction.dcim_only = dcim_only;
    }
    if let Some(preserve) = args.preserve_structure {
        config.output.preserve_structure = preserve;
    }
    if let Some(skip) = args.skip_existing {
        config.output.skip_existing = skip;
    }
    if let Some(ref level) = args.log_level {
        config.logging.level = level.clone();
    }
    if args.all_devices {
        config.device.apple_only = false;
    }

    // Set up graceful shutdown handler
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();

    ctrlc::set_handler(move || {
        if shutdown_flag_clone.load(Ordering::SeqCst) {
            // Second Ctrl+C - force exit
            eprintln!("\nForce shutdown requested. Exiting immediately...");
            std::process::exit(1);
        } else {
            shutdown_flag_clone.store(true, Ordering::SeqCst);
            eprintln!("\nGraceful shutdown requested. Finishing current file... (Press Ctrl+C again to force quit)");
        }
    }).expect("Failed to set Ctrl+C handler");

    // Initialize logger
    let log_level = match config.logging.level.to_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };

    if config.logging.log_to_file {
        // Set up logging to both console and file
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.logging.log_file)
            .expect("Failed to open log file");

        Builder::new()
            .filter_level(log_level)
            .format(|buf, record| {
                writeln!(
                    buf,
                    "[{} {} {}] {}",
                    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
                    record.level(),
                    record.target(),
                    record.args()
                )
            })
            .target(env_logger::Target::Pipe(Box::new(DualWriter {
                console: std::io::stderr(),
                file: log_file,
            })))
            .init();

        info!("Logging to file: {}", config.logging.log_file.display());
    } else {
        Builder::from_env(env_logger::Env::default().default_filter_or(&config.logging.level))
            .init();
    }

    info!("Photo Extraction Tool v1.0.0");
    info!("============================");

    // Handle commands
    match args.command {
        Some(Commands::GenerateConfig { output }) => {
            generate_config_file(&output)?;
        }
        Some(Commands::ShowConfig) => {
            show_config(&config);
        }
        Some(Commands::List { all }) => {
            let use_all = all || !config.device.apple_only;
            list_devices(use_all)?;
        }
        Some(Commands::Scan { depth }) => {
            scan_device(&config, depth)?;
        }
        Some(Commands::Extract) | None => {
            extract_photos(&config, shutdown_flag)?;
        }
        Some(Commands::ListProfiles) => {
            list_profiles(&config)?;
        }
        Some(Commands::RemoveProfile { name }) => {
            remove_profile(&config, &name)?;
        }
        Some(Commands::BenchmarkScan { dcim_only }) => {
            benchmark_scan(&config, dcim_only)?;
        }
    }

    Ok(())
}

fn list_profiles(config: &Config) -> Result<()> {
    let mut manager = ProfileManager::new(&config.device_profiles);
    manager.load().ok();
    manager.list_profiles();
    Ok(())
}

fn remove_profile(config: &Config, name: &str) -> Result<()> {
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

fn generate_config_file(output: &PathBuf) -> Result<()> {
    use std::fs;

    let content = Config::generate_default_config();
    fs::write(output, content)?;

    info!("Generated default configuration file: {}", output.display());
    info!("Edit this file to customize the extraction settings.");

    Ok(())
}

fn show_config(config: &Config) {
    info!("Current Configuration:");
    info!("----------------------");
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
}

fn list_devices(all_devices: bool) -> Result<()> {
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

fn scan_device(config: &Config, max_depth: usize) -> Result<()> {
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
    let device = manager.open_device(&target_device.device_id)?;
    let content = device.get_content()?;

    info!("Scanning device structure (max depth: {})...", max_depth);
    info!("");

    // Create progress tracker
    let progress = ScanProgressTracker::new();

    // Scan and print structure
    scan_recursive(&content, "DEVICE", "", 0, max_depth, &progress)?;

    progress.finish();

    Ok(())
}

fn benchmark_scan(config: &Config, dcim_only: bool) -> Result<()> {
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
    let device = manager.open_device(&target_device.device_id)?;
    let content = device.get_content()?;

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

/// Progress tracker for benchmark scan
struct BenchmarkProgress {
    spinner: ProgressBar,
    start_time: Instant,
}

impl BenchmarkProgress {
    fn new() -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
        spinner.set_message("Initializing...");

        Self {
            spinner,
            start_time: Instant::now(),
        }
    }

    fn log_event(&self, msg: &str) {
        self.spinner.println(format!("  → {}", msg));
    }

    fn update(&self, folders: usize, files: usize, media: usize) {
        let elapsed = self.start_time.elapsed().as_secs();
        self.spinner.set_message(format!(
            "Scanning: {} folders, {} files ({} media) - {}s elapsed",
            folders, files, media, elapsed
        ));
    }

    fn finish(&self) {
        self.spinner.finish_and_clear();
    }
}

/// Progress tracker for scan operations
struct ScanProgressTracker {
    folders_scanned: AtomicUsize,
    files_found: AtomicUsize,
    spinner: ProgressBar,
    start_time: Instant,
}

impl ScanProgressTracker {
    fn new() -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        spinner.set_message("Starting scan...");

        Self {
            folders_scanned: AtomicUsize::new(0),
            files_found: AtomicUsize::new(0),
            spinner,
            start_time: Instant::now(),
        }
    }

    fn increment_folders(&self) {
        self.folders_scanned.fetch_add(1, Ordering::Relaxed);
        self.update_message();
    }

    fn add_files(&self, count: usize) {
        self.files_found.fetch_add(count, Ordering::Relaxed);
        self.update_message();
    }

    fn update_message(&self) {
        let folders = self.folders_scanned.load(Ordering::Relaxed);
        let files = self.files_found.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs();
        self.spinner.set_message(format!(
            "Scanning... {} folders, {} files found ({:.0}s elapsed)",
            folders, files, elapsed
        ));
    }

    fn finish(&self) {
        let folders = self.folders_scanned.load(Ordering::Relaxed);
        let files = self.files_found.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();
        self.spinner.finish_with_message(format!(
            "✓ Scan complete: {} folders, {} files found in {:.1}s",
            folders,
            files,
            elapsed.as_secs_f64()
        ));
    }
}

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

fn extract_photos(config: &Config, shutdown_flag: Arc<AtomicBool>) -> Result<()> {
    // Initialize COM library (required for WPD)
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
        error!("No portable devices found.");
        error!("");
        error!("Make sure your iPhone is:");
        error!("  1. Connected via USB cable");
        error!("  2. Unlocked");
        error!("  3. Trusting this computer (tap 'Trust' when prompted)");
        error!("");
        error!("Use 'list' command to see available devices");
        error!("Use '--all-devices' to see all portable devices (not just Apple)");
        return Ok(());
    }

    // Select device
    let target_device = select_device(&devices, &config.device.device_id)?;
    info!("Selected device: {}", target_device.friendly_name);

    // Determine output directory - use device profiles if enabled
    let output_dir = if config.device_profiles.enabled {
        let mut profile_manager = ProfileManager::new(&config.device_profiles);
        profile_manager.load().ok(); // Ignore error if file doesn't exist

        let profile = profile_manager.get_or_create_profile(target_device)?;
        let output_path = config
            .device_profiles
            .backup_base_folder
            .join(&profile.output_folder);

        info!(
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
        info!("Created output directory: {}", output_dir.display());
    }

    // Convert config to extraction config
    let extraction_config = extractor::ExtractionConfig {
        output_dir: output_dir.clone(),
        dcim_only: config.extraction.dcim_only,
        preserve_structure: config.output.preserve_structure,
        skip_existing: config.output.skip_existing,
        duplicate_detection: if config.duplicate_detection.enabled {
            Some(config.duplicate_detection.clone())
        } else {
            None
        },
        tracking: if config.tracking.enabled {
            Some(config.tracking.clone())
        } else {
            None
        },
    };

    let stats = extractor::extract_photos(target_device, extraction_config, shutdown_flag.clone())?;

    // Check if we were interrupted
    if shutdown_flag.load(Ordering::SeqCst) {
        warn!("Extraction was interrupted by user");
    }

    info!("============================");
    info!("Extraction complete!");
    info!("  Photos extracted: {}", stats.files_extracted);
    info!("  Files skipped: {}", stats.files_skipped);
    info!("  Duplicates skipped: {}", stats.duplicates_skipped);
    info!("  Duplicates overwritten: {}", stats.duplicates_overwritten);
    info!("  Duplicates renamed: {}", stats.duplicates_renamed);
    info!("  Errors: {}", stats.errors);
    info!(
        "  Total size: {:.2} MB",
        stats.total_bytes as f64 / 1_048_576.0
    );
    info!("  Output: {}", output_dir.display());

    Ok(())
}

/// A writer that writes to both console and file
struct DualWriter {
    console: std::io::Stderr,
    file: std::fs::File,
}

impl Write for DualWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Write to console
        let _ = self.console.write(buf);
        // Write to file
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let _ = self.console.flush();
        self.file.flush()
    }
}

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

//! Photo Extraction Tool - CLI Entry Point
//!
//! A fast, reliable tool for extracting photos and videos from iOS devices
//! (iPhone/iPad) on Windows using the Windows Portable Devices (WPD) API.
//!
//! This binary is a thin wrapper around the library, handling argument parsing,
//! logging setup, and command dispatch.

mod cli;
mod core;
mod device;
mod duplicate;
mod testdb;

use anyhow::Result;
use clap::Parser;
use cli::{Args, DualWriter};
use core::config::Config;
use env_logger::Builder;
use log::{info, LevelFilter};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() -> Result<()> {
    let args = Args::parse();

    // Load configuration
    let mut config = if let Some(ref config_path) = args.config {
        match Config::load(config_path) {
            Ok(cfg) => cfg,
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
    })
    .expect("Failed to set Ctrl+C handler");

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

    // Run the command
    cli::run_command(&args, &config, shutdown_flag)?;

    Ok(())
}

//! UI Support Module
//!
//! This module provides all the infrastructure needed for building a graphical
//! user interface for the Photo Extraction Tool, focused on iOS device (iPhone/iPad)
//! photo extraction. It is designed to be UI-framework agnostic, providing building
//! blocks that can be used with any Rust UI framework (egui, iced, Tauri, slint, etc.).
//!
//! No iTunes installation or additional drivers are required on Windows 10/11.
//!
//! # Architecture
//!
//! The UI module is organized into several submodules:
//!
//! - [`events`] - Thread-safe event types for communication between backend and UI
//! - [`controller`] - Extraction controller with async operations and cancellation
//! - [`device_monitor`] - Device hot-plug detection and state tracking
//! - [`preview`] - Thumbnail generation and preview management
//!
//! # Threading Model
//!
//! The UI module uses a channel-based architecture for thread safety:
//!
//! 1. **Event Channels** - Background operations emit events through channels
//!    that the UI can poll without blocking
//! 2. **Atomic State** - Controller state is managed with atomic operations
//!    for lock-free status checks
//! 3. **Shared Progress** - Progress tracking uses atomic counters that can
//!    be read from any thread
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::{
//!     ExtractionController, DeviceMonitor, UiEvent,
//!     ExtractionConfig, MonitorConfig,
//! };
//! use photo_extraction_tool::device::DeviceManager;
//! use std::sync::Arc;
//!
//! // Create controller and monitor
//! let controller = ExtractionController::new();
//! let monitor = DeviceMonitor::with_config(MonitorConfig::fast());
//!
//! // Start device monitoring
//! // let manager = Arc::new(DeviceManager::new().unwrap());
//! // monitor.start(manager.clone()).unwrap();
//!
//! // UI event loop (pseudo-code)
//! loop {
//!     // Process device events
//!     while let Some(event) = monitor.try_recv_event() {
//!         match event {
//!             UiEvent::Device(dev_event) => {
//!                 // Handle device connect/disconnect
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     // Process extraction events
//!     while let Some(event) = controller.try_recv_event() {
//!         match event {
//!             UiEvent::Extraction(ext_event) => {
//!                 // Update progress UI
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     // Check controller state
//!     if controller.is_active() {
//!         let progress = controller.progress().snapshot();
//!         // Update progress bar with progress.percent_complete
//!     }
//!
//!     # break; // Exit for doctest
//! }
//! ```
//!
//! # Starting an Extraction
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::{ExtractionController, ExtractionConfig};
//! use photo_extraction_tool::device::{DeviceManager, DeviceManagerTrait};
//! use std::sync::Arc;
//! use std::path::PathBuf;
//!
//! let controller = ExtractionController::new();
//!
//! // Get device info (assuming device is connected)
//! // let manager = Arc::new(DeviceManager::new().unwrap());
//! // let devices = manager.enumerate_apple_devices().unwrap();
//! // let device = devices.first().unwrap().clone();
//!
//! // Configure extraction
//! let config = ExtractionConfig::new(PathBuf::from("D:/Photos"))
//!     .preserve_structure(true)
//!     .skip_existing(true)
//!     .dcim_only(true);
//!
//! // Start extraction (runs in background thread)
//! // controller.start_extraction(manager, device, config).unwrap();
//!
//! // Later, you can pause/resume/cancel
//! // controller.pause().unwrap();
//! // controller.resume().unwrap();
//! // controller.cancel().unwrap();
//! ```
//!
//! # Device Monitoring
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::{DeviceMonitor, MonitorConfig, DeviceState};
//! use photo_extraction_tool::ui::events::DeviceEvent;
//!
//! // Create monitor with fast polling for responsive UI
//! let monitor = DeviceMonitor::with_config(MonitorConfig::fast());
//!
//! // Add known device IDs from config/history
//! monitor.add_known_devices(vec!["device-uuid-1".to_string()]);
//!
//! // Start monitoring
//! // monitor.start(device_manager).unwrap();
//!
//! // Check connected devices
//! for device in monitor.connected_devices() {
//!     println!("Device: {} - {:?}", device.info.friendly_name, device.state);
//! }
//! ```
//!
//! # Preview and Thumbnails
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::preview::{PreviewManager, ThumbnailConfig};
//!
//! // Create preview manager with medium thumbnails
//! let manager = PreviewManager::with_config(ThumbnailConfig::medium());
//!
//! // Add items from device scan
//! // manager.add_items(device_objects);
//!
//! // Get photos and videos
//! let photos = manager.photos();
//! let videos = manager.videos();
//!
//! // Selection management
//! manager.select(0);
//! manager.select_all();
//! let selected = manager.selected_items();
//! ```

pub mod controller;
pub mod device_monitor;
pub mod events;
pub mod preview;

// Re-export main types for convenience
pub use controller::{
    ControllerCommand, ControllerState, ExtractionConfig, ExtractionController, ProgressSnapshot,
    ProgressTracker,
};

pub use device_monitor::{
    DeviceMonitor, DeviceState, DeviceStateChecker, MonitorConfig, MonitoredDevice,
};

pub use events::{
    format_bytes, format_bytes_per_second, format_duration, format_eta, AppEvent, DeviceEvent,
    ExtractionEvent, ExtractionSummary, LogLevel, PauseReason, SkipReason, UiEvent,
};

pub use preview::{
    CacheStats, PreviewItem, PreviewManager, Thumbnail, ThumbnailCache, ThumbnailConfig,
    ThumbnailGenerator, ThumbnailResult,
};

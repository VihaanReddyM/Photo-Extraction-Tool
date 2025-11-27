# UI Architecture Documentation

This document describes the UI support architecture for the Photo Extraction Tool. The `ui` module provides all the infrastructure needed for building graphical user interfaces for iOS device (iPhone/iPad) photo extraction, designed to be framework-agnostic and work with any Rust UI framework (egui, iced, Tauri, slint, etc.).

> **Note**: This tool uses the Windows Portable Devices (WPD) API. No iTunes installation or additional drivers are required on Windows 10/11.

## Overview

The UI module is organized into four main components:

```
src/ui/
├── mod.rs           # Module exports and documentation
├── events.rs        # Thread-safe event types
├── controller.rs    # Extraction controller with async operations
├── device_monitor.rs # Device hot-plug detection
└── preview.rs       # Thumbnail generation and preview management
```

## Threading Model

The UI module uses a **channel-based architecture** for thread safety:

1. **Event Channels** - Background operations emit events through `mpsc` channels that the UI can poll without blocking
2. **Atomic State** - Controller state is managed with atomic operations for lock-free status checks
3. **Shared Progress** - Progress tracking uses atomic counters that can be read from any thread

```
┌─────────────────┐     Events      ┌─────────────────┐
│  Background     │ ───────────────→│   UI Thread     │
│  Thread(s)      │                 │                 │
│                 │                 │  • Poll events  │
│  • Extraction   │                 │  • Update UI    │
│  • Device scan  │                 │  • Read state   │
│  • Thumbnails   │                 │                 │
└─────────────────┘                 └─────────────────┘
        ↑                                   │
        │         Commands                  │
        └───────────────────────────────────┘
```

## Components

### 1. Events (`events.rs`)

Thread-safe event types for communication between backend and UI.

#### Event Types

- **`ExtractionEvent`** - Events during extraction (started, progress, completed, cancelled, errors)
- **`DeviceEvent`** - Device connection/disconnection events
- **`AppEvent`** - Application-level events (config, disk space, shutdown)
- **`UiEvent`** - Combined enum wrapping all event types

#### Key Structures

```rust
// Progress during extraction
ExtractionEvent::Progress {
    files_extracted: usize,
    files_skipped: usize,
    duplicates_found: usize,
    errors: usize,
    bytes_processed: u64,
    eta: Option<Duration>,
    speed_bps: u64,
    percent_complete: f64,
}

// Skip reasons for detailed feedback
enum SkipReason {
    AlreadyExists,
    PreviouslyExtracted,
    Duplicate { original: PathBuf },
    FilteredOut { filter: String },
    TooSmall { size: u64, minimum: u64 },
    TooLarge { size: u64, maximum: u64 },
}
```

#### Helper Functions

- `format_bytes(u64)` → "1.5 MB"
- `format_bytes_per_second(u64)` → "10.2 MB/s"
- `format_duration(Duration)` → "1h 23m 45s"
- `format_eta(Option<Duration>)` → "5m 30s" or "calculating..."

### 2. Controller (`controller.rs`)

Thread-safe extraction controller for managing background operations.

#### Features

- **Start/Stop/Pause/Resume** extraction
- **Progress tracking** with ETA and speed calculation
- **Graceful cancellation** with cleanup
- **Event emission** for UI updates

#### Usage

```rust
use photo_extraction_tool::ui::{ExtractionController, ExtractionConfig};

// Create controller
let controller = ExtractionController::new();

// Configure extraction
let config = ExtractionConfig::new(PathBuf::from("D:/Photos"))
    .preserve_structure(true)
    .skip_existing(true)
    .dcim_only(true)
    .max_files(0); // 0 = unlimited

// Start extraction (runs in background thread)
controller.start_extraction(device_manager, device_info, config)?;

// Check state
if controller.is_active() {
    let progress = controller.progress().snapshot();
    println!("{}% complete", progress.percent_complete);
}

// Control extraction
controller.pause()?;
controller.resume()?;
controller.cancel()?;

// Poll events in UI loop
while let Some(event) = controller.try_recv_event() {
    match event {
        UiEvent::Extraction(ExtractionEvent::Progress { percent_complete, .. }) => {
            // Update progress bar
        }
        UiEvent::Extraction(ExtractionEvent::Completed { stats }) => {
            // Show completion dialog
        }
        _ => {}
    }
}
```

#### Controller States

```rust
enum ControllerState {
    Idle,       // Ready for new extraction
    Scanning,   // Scanning device for files
    Extracting, // Actively extracting files
    Paused,     // Extraction is paused
    Cancelling, // Being cancelled
    Completed,  // Extraction finished
    Error,      // An error occurred
}
```

#### Progress Tracking

The `ProgressTracker` provides real-time statistics:

```rust
pub struct ProgressSnapshot {
    pub files_extracted: usize,
    pub files_skipped: usize,
    pub duplicates_found: usize,
    pub errors: usize,
    pub bytes_processed: u64,
    pub total_files: usize,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta: Option<Duration>,
    pub percent_complete: f64,
    pub elapsed: Duration,
}
```

### 3. Device Monitor (`device_monitor.rs`)

Hot-plug detection and device state tracking.

#### Features

- **Automatic device detection** via polling
- **Connection/disconnection events**
- **Device state tracking** (Connected, Locked, NeedsTrust, Disconnected)
- **Known device recognition**

#### Usage

```rust
use photo_extraction_tool::ui::{DeviceMonitor, MonitorConfig};

// Create monitor with fast polling for responsive UI
let monitor = DeviceMonitor::with_config(MonitorConfig::fast());

// Add known device IDs from config/history
monitor.add_known_devices(vec!["device-uuid-1".to_string()]);

// Start monitoring (spawns background thread)
monitor.start(device_manager)?;

// Poll for events
while let Some(event) = monitor.try_recv_event() {
    match event {
        UiEvent::Device(DeviceEvent::Connected { device, previously_known }) => {
            println!("Device connected: {}", device.friendly_name);
        }
        UiEvent::Device(DeviceEvent::Disconnected { device_id, device_name }) => {
            println!("Device disconnected");
        }
        _ => {}
    }
}

// Check connected devices
for device in monitor.connected_devices() {
    println!("{}: {:?}", device.info.friendly_name, device.state);
}

// Stop monitoring
monitor.stop();
```

#### Monitor Configuration

```rust
// Default: 2s polling, 10s disconnect timeout
let config = MonitorConfig::default();

// Fast: 500ms polling, 5s disconnect timeout
let config = MonitorConfig::fast();

// Slow: 5s polling, 15s disconnect timeout
let config = MonitorConfig::slow();

// Custom
let config = MonitorConfig::default()
    .with_poll_interval(1000)
    .with_disconnect_timeout(5000)
    .apple_only(true);
```

#### Device States

```rust
enum DeviceState {
    Connected,    // Device accessible
    Locked,       // Device locked, needs unlock
    NeedsTrust,   // User needs to tap "Trust"
    Disconnected, // Device not present
    Unknown,      // State being determined
}

// Helper for user-friendly messages
DeviceStateChecker::state_message(&DeviceState::Locked)
// → "Device is locked. Please unlock your iOS device."

DeviceStateChecker::state_icon(&DeviceState::Connected)
// → "✅"
```

### 4. Preview Manager (`preview.rs`)

Thumbnail generation and preview management for photos.

#### Features

- **Thumbnail configuration** (size, quality, cache)
- **In-memory caching** with LRU eviction
- **Selection management** for batch operations
- **Photo/video separation**

#### Usage

```rust
use photo_extraction_tool::ui::preview::{PreviewManager, ThumbnailConfig};

// Create with medium thumbnails
let manager = PreviewManager::with_config(ThumbnailConfig::medium());

// Add items from device scan
manager.add_items(device_objects);

// Get counts
let photo_count = manager.photos().len();
let video_count = manager.videos().len();

// Selection management
manager.select(0);
manager.select(1);
manager.toggle_selection(2);
manager.select_all();
manager.deselect_all();

let selected = manager.selected_items();
let selected_size = manager.selected_size();

// Load thumbnails (when image crate is enabled)
manager.load_thumbnail(&content, index);
```

#### Thumbnail Configuration

```rust
// Presets
ThumbnailConfig::icon()   // 64x64
ThumbnailConfig::medium() // 128x128
ThumbnailConfig::large()  // 512x512

// Custom
ThumbnailConfig::default()
    .with_dimensions(320, 240)
    .with_quality(90)
    .with_cache(true, 500);
```

**Note:** Actual thumbnail generation requires adding the `image` crate to dependencies. The current implementation provides the infrastructure but returns `ThumbnailResult::NotAvailable` without the image processing library.

## Integration Examples

### Basic UI Loop (egui/iced style)

```rust
struct App {
    controller: ExtractionController,
    monitor: DeviceMonitor,
    devices: Vec<MonitoredDevice>,
    progress: Option<ProgressSnapshot>,
}

impl App {
    fn update(&mut self) {
        // Process device events
        while let Some(event) = self.monitor.try_recv_event() {
            if let UiEvent::Device(dev) = event {
                match dev {
                    DeviceEvent::Connected { device, .. } => {
                        // Add to device list
                    }
                    DeviceEvent::Disconnected { device_id, .. } => {
                        // Remove from device list
                    }
                    _ => {}
                }
            }
        }
        
        // Process extraction events
        while let Some(event) = self.controller.try_recv_event() {
            if let UiEvent::Extraction(ext) = event {
                match ext {
                    ExtractionEvent::Progress { .. } => {
                        self.progress = Some(self.controller.progress().snapshot());
                    }
                    ExtractionEvent::Completed { stats } => {
                        // Show completion
                    }
                    ExtractionEvent::FatalError { error, .. } => {
                        // Show error dialog
                    }
                    _ => {}
                }
            }
        }
        
        // Update connected devices
        self.devices = self.monitor.connected_devices();
    }
}
```

### Tauri Integration

For Tauri, wrap the controller in a state manager:

```rust
use tauri::State;

#[tauri::command]
fn start_extraction(
    controller: State<ExtractionController>,
    output_dir: String,
) -> Result<(), String> {
    // ... start extraction
}

#[tauri::command]
fn get_progress(controller: State<ExtractionController>) -> ProgressSnapshot {
    controller.progress().snapshot()
}

#[tauri::command]
fn cancel_extraction(controller: State<ExtractionController>) -> Result<(), String> {
    controller.cancel().map_err(|e| e.to_string())
}
```

## Error Handling

The UI module uses structured error types that can be displayed to users:

```rust
// Rich error context
ExtractionEvent::FatalError {
    error: String,        // Main error message
    context: Option<String>, // Additional context
}

// Skip reasons with details
SkipReason::Duplicate { original: PathBuf }
SkipReason::TooLarge { size: u64, maximum: u64 }
```

## Performance Considerations

1. **Non-blocking Event Polling** - Use `try_recv_event()` in UI loops to avoid blocking
2. **Atomic Progress Reads** - Progress can be read from any thread without locks
3. **Configurable Polling** - Adjust monitor polling interval based on UI needs
4. **Cache Management** - Thumbnail cache auto-evicts old entries

## Adding Thumbnail Support

To enable actual thumbnail generation, add to `Cargo.toml`:

```toml
[dependencies]
image = "0.25"
```

Then uncomment the image processing code in `preview.rs`.

## Future Enhancements

- [ ] Video thumbnail extraction (requires ffmpeg bindings)
- [ ] HEIC/HEIF support (requires heif decoder)
- [ ] Async/await support with tokio
- [ ] WebSocket event streaming for web UIs
- [ ] Undo/redo for selections
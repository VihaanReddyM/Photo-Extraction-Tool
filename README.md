# 📸 Photo Extraction Tool

A fast, reliable command-line tool for extracting photos and videos from iOS devices (iPhone/iPad) on Windows — **no iTunes or additional drivers required**.

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)](https://www.microsoft.com/windows)

---

## ✨ Features

- **🚀 Fast & Efficient** — Direct device access via Windows Portable Devices (WPD) API
- **📱 No iTunes or Drivers Required** — Works out of the box on Windows 10/11
- **🔄 Incremental Backups** — Only extract new photos, skip existing ones
- **🔍 Duplicate Detection** — SHA256 hashing to detect exact duplicates (photos & videos)
- **👥 Multi-Device Support** — Manage and organize photos from multiple devices
- **📁 Flexible Organization** — Preserve folder structure or organize by date
- **⏸️ Resume Support** — Interrupted? Continue from where you left off
- **📊 Progress Tracking** — Real-time progress bar with ETA
- **⚙️ Highly Configurable** — TOML configuration file for all settings

---

## 📋 Table of Contents

- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Usage](#-usage)
- [Configuration](#-configuration)
- [Features in Detail](#-features-in-detail)
- [Troubleshooting](#-troubleshooting)
- [Building from Source](#-building-from-source)
- [Contributing](#-contributing)
- [License](#-license)

---

## 📥 Installation

### Pre-built Binary

1. Download the latest release from the [Releases](https://github.com/yourusername/photo-extraction-tool/releases) page
2. Extract the ZIP file to your preferred location
3. (Optional) Add the folder to your system PATH

### Using Cargo

```bash
cargo install photo-extraction-tool
```

### From Source

```bash
git clone https://github.com/yourusername/photo-extraction-tool.git
cd photo-extraction-tool
cargo build --release
```

---

## 🚀 Quick Start

1. **Connect your iOS device (iPhone/iPad)** to your Windows PC via USB
2. **Unlock your device** and tap **"Trust"** when prompted
3. **Run the tool**:

```bash
# Extract all photos to the default directory
photo-extraction-tool

# Extract to a specific folder
photo-extraction-tool --output "D:/Photos/iPhone Backup"

# List connected devices
photo-extraction-tool list

# Extract with duplicate detection
photo-extraction-tool extract --detect-duplicates --compare-to "D:/ExistingPhotos"

# Open the config file in your default editor
photo-extraction-tool config
```

That's it! Your photos will be extracted to the specified folder.

---

## 📖 Usage

### Basic Commands

```bash
# Extract photos with default settings
photo-extraction-tool

# Specify output directory
photo-extraction-tool --output "./my_photos"
photo-extraction-tool -o "D:/Backups/iPhone"

# Open and edit your configuration
photo-extraction-tool config

# Show where the config file is located
photo-extraction-tool config --path

# Reset config to defaults
photo-extraction-tool config --reset

# Use a specific configuration file (override)
photo-extraction-tool --config ./my_config.toml
photo-extraction-tool -c ./my_config.toml

# List all connected devices
photo-extraction-tool --list-devices

# Use a specific device (by ID)
photo-extraction-tool --device "\\?\usb#vid_05ac..."

# Show version
photo-extraction-tool --version

# Show help
photo-extraction-tool --help
```

### Command-Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--output <DIR>` | `-o` | Output directory for extracted photos |
| `--config <FILE>` | `-c` | Path to configuration file |
| `--device <ID>` | `-d` | Specific device ID to use |
| `--detect-duplicates` | | Enable SHA256 duplicate detection |
| `--compare-to <DIR>` | | Folder to compare against (repeatable) |
| `--duplicate-action` | | Action for duplicates: skip, rename, overwrite |

| `--verbose` | `-v` | Increase verbosity (can be repeated) |
| `--quiet` | `-q` | Suppress non-error output |
| `--help` | `-h` | Show help message |
| `--version` | `-V` | Show version |

### Subcommands

| Command | Description |
|---------|-------------|
| `config` | Open the config file in your default editor |
| `config --path` | Show the config file location |
| `config --reset` | Reset configuration to defaults |
| `generate-config` | Generate config at a specific location |
| `show-config` | Display current configuration settings |
| `list` | List connected devices |
| `extract` | Extract photos (default if no command given) |
| `scan` | Scan device folder structure |
| `list-profiles` | List configured device profiles |
| `remove-profile` | Remove a device profile |

---

## ⚙️ Configuration

Configuration is stored in a standard location that persists across updates:

| Platform | Location |
|----------|----------|
| **Windows** | `%APPDATA%\photo_extraction_tool\config.toml` |
| **Linux/macOS** | `~/.config/photo_extraction_tool/config.toml` |

### Quick Setup

The easiest way to configure the tool:

```bash
# Open config in your default text editor (Notepad, VS Code, etc.)
photo-extraction-tool config
```

This will:
1. Create the config directory if it doesn't exist
2. Create a default config file with all options documented
3. Open it in your default editor for `.toml` files

After editing, just save the file - changes apply on the next run.

### Alternative: Local Config Override

You can also place a `config.toml` in your current directory to override the global config. This is useful for project-specific settings. The search order is:

1. `./config.toml` (current directory)
2. `./photo_extraction.toml` (current directory)
3. Standard config location (see table above)

### Configuration Sections

<details>
<summary><b>📁 Output Settings</b></summary>

```toml
[output]
# Where to save extracted photos
directory = "./extracted_photos"

# Keep original folder structure (e.g., DCIM/100APPLE/)
preserve_structure = true

# Skip files that already exist
skip_existing = true

# Organize into YYYY/MM folders by date
organize_by_date = false

# Create subfolder named after device
subfolder_by_device = false
```

</details>

<details>
<summary><b>📱 Device Settings</b></summary>

```toml
[device]
# Only detect Apple devices
apple_only = true

# Filter by device name (partial match)
# device_name_filter = "iPhone 15"

# Specific device ID
# device_id = "\\?\usb#vid_05ac..."
```

</details>

<details>
<summary><b>🎯 Extraction Settings</b></summary>

```toml
[extraction]
# Only extract from DCIM (Camera Roll)
dcim_only = true

# File type filters
include_photos = true
include_videos = true

# Extension filters (empty = all)
include_extensions = []  # e.g., ["jpg", "heic"]
exclude_extensions = []  # e.g., ["aae"]

# Size filters (0 = no limit)
min_file_size = 0
max_file_size = 0
```

</details>

<details>
<summary><b>🔍 Duplicate Detection</b></summary>

```toml
[duplicate_detection]
# Enable SHA256-based exact duplicate detection
enabled = false

# Folders to compare against (will be indexed)
comparison_folders = ["D:/Photos", "D:/Old Backups"]

# Cache hashes for faster subsequent runs (JSON format)
cache_enabled = true
cache_file = "./.duplicate_cache.json"

# Action when duplicate found: "skip", "rename", or "overwrite"
duplicate_action = "skip"

# Scan folders recursively
recursive = true

# Only index media files (photos/videos)
media_only = true
```

</details>

<details>
<summary><b>👥 Device Profiles</b></summary>

```toml
[device_profiles]
enabled = false

# Base folder for all device backups
backup_base_folder = "D:/Photos"

# Profile database
profiles_file = "./.device_profiles.json"
```

</details>

---

## 🔧 Features in Detail

### Incremental Backups

The tool tracks which files have been extracted, so subsequent runs only copy new photos:

```bash
# First run: extracts all photos
photo-extraction-tool -o "D:/Backup"

# Later runs: only extracts new photos
photo-extraction-tool -o "D:/Backup"
```

### Duplicate Detection

Avoid extracting files you already have elsewhere. Uses SHA256 hashing for exact-match detection — works with both photos AND videos:

**Via Config File:**
```toml
[duplicate_detection]
enabled = true
comparison_folders = [
    "D:/Photos/Main Library",
    "D:/Backups/Old iPhone"
]
duplicate_action = "skip"
```

**Via Command Line:**
```bash
# Enable duplicate detection with one or more comparison folders
photo-extraction-tool extract --detect-duplicates --compare-to "D:/Photos" --compare-to "E:/Backup"

# Specify what to do with duplicates
photo-extraction-tool extract --detect-duplicates --compare-to "D:/Photos" --duplicate-action skip
```

**How it works:**
1. Scans comparison folders and computes SHA256 hashes (parallel, cached)
2. For each file from your device, first checks if any indexed file has the same size (fast rejection)
3. If sizes match, computes SHA256 hash and checks for exact match
4. Takes configured action: skip, rename, or overwrite

The SHA256 algorithm detects exact duplicates reliably and works for any file type.

### Multi-Device Management

Organize photos from multiple devices automatically:

```toml
[device_profiles]
enabled = true
backup_base_folder = "D:/Photos"
```

Result:
```
D:/Photos/
├── Johns_iPhone_15_Pro/
│   └── DCIM/...
├── Marys_iPad_Air/
│   └── DCIM/...
└── Kids_iPhone_SE/
    └── DCIM/...
```

### Managing Your Configuration

```bash
# Open config in your default editor
photo-extraction-tool config

# View current settings
photo-extraction-tool show-config

# See where the config file is stored
photo-extraction-tool config --path

# Start fresh with default settings
photo-extraction-tool config --reset
```

### Date-Based Organization

Organize photos by when they were taken:

```toml
[output]
organize_by_date = true
```

Result:
```
extracted_photos/
├── 2024/
│   ├── 01/
│   │   ├── IMG_0001.jpg
│   │   └── IMG_0002.heic
│   ├── 02/
│   └── 03/
└── 2023/
    └── 12/
```

---

## ❓ Troubleshooting

### Device Not Detected

1. **Unlock Your Device**: The iOS device must be unlocked for access
2. **Trust the Computer**: Tap "Trust" when prompted on your iPhone/iPad
3. **Check USB Connection**: Try a different USB cable or port
4. **Restart the Device**: A restart can resolve connection issues
5. **Windows Update**: Ensure Windows is up to date (includes USB drivers)

> **Note**: No iTunes installation or additional drivers are required. Windows 10/11 includes built-in support for iOS devices via the Windows Portable Devices (WPD) API.

### Slow Extraction

- HEIC and large video files take longer to transfer
- USB 2.0 ports are slower than USB 3.0
- Consider using `--dry-run` first to see file counts

### Permission Errors

- Run the tool as Administrator if you encounter permission issues
- Check that the output directory is writable

### Common Error Messages

| Error | Solution |
|-------|----------|
| "No devices found" | Connect device, unlock it, and tap "Trust" on the device |
| "Access denied" | Unlock your iOS device or run the tool as Administrator |
| "Path not found" | Check output directory exists |
| "Config file not found" | Run `photo_extraction_tool config` to create one |

---

## 📁 Project Structure

The project is organized for scalability and to support future UI development:

```
src/
├── main.rs              # CLI binary entry point (thin wrapper)
├── lib.rs               # Library root - exports public API
├── cli/                 # CLI-specific code
│   ├── mod.rs           # CLI module exports
│   ├── args.rs          # Command-line argument definitions
│   ├── commands.rs      # Command handler implementations
│   └── progress.rs      # Progress bars and CLI output utilities
├── core/                # Core business logic
│   ├── mod.rs           # Core module exports
│   ├── config.rs        # Configuration types and loading
│   ├── error.rs         # Error types and result aliases
│   ├── extractor.rs     # Main extraction logic
│   └── tracking.rs      # Extraction state and session tracking
├── device/              # Device interaction
│   ├── mod.rs           # Device module exports
│   ├── wpd.rs           # Windows Portable Devices API wrapper
│   └── profiles.rs      # Device profile management
└── duplicate/           # Duplicate detection
    ├── mod.rs           # Duplicate module exports
    └── detector.rs      # SHA256-based duplicate detection index
```

### Architecture Overview

- **`lib.rs`** - Library crate that exposes the public API, allowing the core functionality to be reused by other applications (e.g., a future GUI)
- **`cli/`** - All CLI-specific code is isolated here, making it easy to add alternative interfaces
- **`core/`** - Business logic that's independent of the interface (config, extraction, tracking)
- **`device/`** - Hardware interaction layer (WPD API for iOS devices, device profiles)
- **`duplicate/`** - SHA256-based duplicate detection with size pre-filtering and caching

This separation allows for:
- Easy addition of a GUI without modifying core logic
- Reusable library for other Rust projects
- Clear boundaries between concerns
- Simplified testing of individual components

---

## 🔨 Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) 1.75 or later
- Windows 10/11 (uses the Windows Portable Devices API)
- Visual Studio Build Tools (for Windows API bindings)

> **Note**: No iTunes or Apple-specific drivers are required. Windows 10/11 includes built-in USB/MTP drivers that work with iOS devices.

### Build Steps

```bash
# Clone the repository
git clone https://github.com/yourusername/photo-extraction-tool.git
cd photo-extraction-tool

# Build in release mode
cargo build --release

# The binary will be at target/release/photo-extraction-tool.exe
```

### Running Tests

```bash
cargo test
```

---

## 🖥️ Future: GUI Support

The project structure is designed to support adding a graphical user interface. A future `ui/` module could be added:

```
src/
├── ui/                  # (Future) GUI implementation
│   ├── mod.rs
│   ├── app.rs           # Main application window
│   ├── components/      # Reusable UI components
│   └── views/           # Different screens/views
```

The core library (`lib.rs`) exposes all necessary functionality, so a GUI would simply:
1. Import the library: `use photo_extraction_tool::core::*;`
2. Call the same functions the CLI uses
3. Display progress and results in a graphical interface

---

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Commit** your changes (`git commit -m 'Add amazing feature'`)
4. **Push** to the branch (`git push origin feature/amazing-feature`)
5. **Open** a Pull Request

### Development Guidelines

- Follow Rust conventions and use `cargo fmt`
- Add tests for new functionality
- Update documentation as needed
- Keep commits focused and atomic

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Microsoft Windows Portable Devices API documentation
- The Rust community for excellent crates
- Contributors and testers

---

## 📬 Contact

- **Issues**: [GitHub Issues](https://github.com/yourusername/photo-extraction-tool/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/photo-extraction-tool/discussions)

---

<p align="center">
  Made with ❤️ in Rust
</p>
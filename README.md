# 📸 Photo Extraction Tool

A fast, reliable command-line tool for extracting photos and videos from iOS devices (iPhone/iPad) and Android phones on Windows — **no iTunes or additional drivers required**.

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)](https://www.microsoft.com/windows)

---

## ✨ Features

- **🚀 Fast & Efficient** — Direct device access via Windows Portable Devices (WPD) API
- **📱 No iTunes Required** — Works out of the box on Windows 10/11
- **🤖 Android Support** — Extract from Samsung, Pixel, OnePlus, Xiaomi, and more
- **💬 App Media Extraction** — WhatsApp, Telegram, Instagram, Signal, and more
- **🔄 Incremental Backups** — Only extract new photos, skip existing ones
- **👥 Multi-Device Support** — Automatic organization by device (enabled by default)
- **🔍 Duplicate Detection** — SHA256 hashing to detect exact duplicates
- **📁 Flexible Organization** — Preserve folder structure or organize by date
- **⏸️ Resume Support** — Interrupted? Continue from where you left off
- **📊 Progress Tracking** — Real-time progress bar with detailed stats
- **⚙️ Easy Setup** — First-run wizard guides you through configuration

---

## 📋 Table of Contents

- [Quick Start](#-quick-start)
- [Installation](#-installation)
- [Usage](#-usage)
- [Configuration](#-configuration)
- [Android Support](#-android-support)
- [Features in Detail](#-features-in-detail)
- [Troubleshooting](#-troubleshooting)
- [Building from Source](#-building-from-source)

---

## 🚀 Quick Start

### First Time Setup

1. **Connect your iOS device** (iPhone/iPad) or **Android phone** to your Windows PC via USB
2. **Unlock your device** and tap **"Trust"** (iOS) or **"Allow"** (Android) when prompted
3. **Run the tool**:

```bash
photo_extraction_tool
```

4. **Follow the setup wizard** — it will ask where to save your photos:

```
╔══════════════════════════════════════════════════════════════════╗
║           📸 Photo Extraction Tool - First Time Setup            ║
╠══════════════════════════════════════════════════════════════════╣
║  Welcome! Let's set up your photo backup preferences.            ║
╚══════════════════════════════════════════════════════════════════╝

Where would you like to save your photos?

Suggested locations:
  [1] C:/Users/John/Pictures/iOS Backup
  [2] C:/Users/John/Documents/iOS Photos
  [3] D:/Photos

Enter a path or number from above:
> 
```

That's it! Your photos will be extracted to a device-specific folder like:
```
D:/Photos/Johns_iPhone_15/
```

### Subsequent Runs

Just run the tool — it remembers your settings and only extracts new photos:

```bash
photo_extraction_tool
```

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

## 📖 Usage

### Basic Commands

```bash
# Extract photos (runs setup wizard on first use)
photo_extraction_tool

# Extract to a specific folder (bypasses setup)
photo_extraction_tool --output "D:/Photos/iPhone Backup"

# List connected devices
photo_extraction_tool list

# Open configuration file in your editor
photo_extraction_tool config

# Show current settings
photo_extraction_tool show-config

# Extract with duplicate detection
photo_extraction_tool --detect-duplicates --compare-to "D:/ExistingPhotos"
```

### Command-Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--output <DIR>` | `-o` | Output directory (overrides config) |
| `--config <FILE>` | `-c` | Use a specific config file |
| `--device-id <ID>` | `-d` | Extract from specific device |
| `--detect-duplicates` | | Enable SHA256 duplicate detection |
| `--compare-to <DIR>` | | Folder to compare against (repeatable) |
| `--duplicate-action` | | Action for duplicates: skip, rename, overwrite |
| `--all-devices` | | Show all MTP devices, not just Apple |
| `--help` | `-h` | Show help message |
| `--version` | `-V` | Show version |

### Subcommands

| Command | Description |
|---------|-------------|
| `extract` | Extract photos (default command) |
| `list` | List connected devices |
| `config` | Open config file in editor |
| `config --reset` | Reset to default settings |
| `show-config` | Display current settings |
| `scan` | View device folder structure |
| `list-profiles` | Show configured device profiles |

---

## ⚙️ Configuration

Configuration is stored at:
- **Windows**: `%APPDATA%\photo_extraction_tool\config.toml`

### Quick Setup

The first time you run the tool, a setup wizard will guide you through essential settings. After that, you can edit settings with:

```bash
# Open config in your default editor
photo_extraction_tool config

# View current settings
photo_extraction_tool show-config

# Reset to defaults (will trigger setup wizard again)
photo_extraction_tool config --reset
```

### Key Settings

#### Device Profiles (Enabled by Default)

```toml
[device_profiles]
enabled = true
backup_base_folder = "D:/Photos"
```

Each device gets its own folder automatically:
```
D:/Photos/
├── Johns_iPhone_15_Pro/
├── Marys_iPad_Air/
└── Kids_iPhone_SE/
```

#### Duplicate Detection

```toml
[duplicate_detection]
enabled = true
comparison_folders = ["D:/Photos/Main Library", "D:/Backups/Old iPhone"]
duplicate_action = "skip"  # skip, rename, or overwrite
```

#### Extraction Options

```toml
[extraction]
dcim_only = true           # Only extract camera roll
include_photos = true
include_videos = true
```

---

## 🔧 Features in Detail

### Multi-Device Support

Device profiles are enabled by default. When you connect different iOS devices, each one automatically gets its own folder:

```
D:/Photos/
├── Johns_iPhone_15_Pro/
│   └── 202511__/
│       ├── IMG_1234.HEIC
│       └── IMG_1235.MOV
├── Marys_iPad_Air/
│   └── 202510__/
│       └── IMG_0001.HEIC
```

### Incremental Backups

The tool tracks which files have been extracted. Subsequent runs only copy new photos:

```bash
# First run: extracts all 1,720 photos
photo_extraction_tool
# → Found 1720 photos/videos to extract
# → Photos extracted: 1720

# Later: only extracts 23 new photos
photo_extraction_tool
# → Found 1743 photos/videos to extract  
# → Photos extracted: 23
# → Files skipped: 1720
```

### Duplicate Detection

Avoid downloading files you already have. Uses SHA256 hashing for exact-match detection:

```bash
# Enable with CLI flags
photo_extraction_tool --detect-duplicates --compare-to "D:/Photos" --compare-to "E:/Backup"
```

How it works:
1. Scans comparison folders and computes SHA256 hashes (parallel, cached)
2. For each device file, checks if any indexed file matches by size first (fast)
3. If sizes match, computes full SHA256 hash and compares
4. Takes configured action: skip, rename, or overwrite

### iOS Photo Organization

iOS devices organize photos in different ways:
- **Traditional**: `DCIM/100APPLE/`, `DCIM/101APPLE/`, etc.
- **Date-based**: `202511__/`, `202510__/`, etc.

This tool automatically detects and handles both structures.

---

## 🤖 Android Support

The tool fully supports Android devices (Samsung, Google Pixel, OnePlus, Xiaomi, and more).

### Enabling Android Support

To extract from Android devices, set `apple_only = false` in your config:

```bash
photo_extraction_tool config
```

Then edit the `[device]` section:

```toml
[device]
apple_only = false
```

### Android Folder Structure

The tool automatically scans common Android photo locations:
- `DCIM/Camera/` — Main camera photos
- `DCIM/Screenshots/` — Screenshots
- `Pictures/` — Edited photos, app exports
- `Download/` — Downloaded images (optional)

### App-Specific Media Extraction

Extract photos and videos from popular messaging and social media apps:

```toml
[android]
# Standard folders
include_camera = true
include_screenshots = true
include_pictures = true
include_downloads = false

# App-specific media folders
include_whatsapp = true      # WhatsApp/Media/WhatsApp Images & Video
include_telegram = true      # Telegram/Telegram Images & Video
include_instagram = false    # Pictures/Instagram
include_facebook = false     # Pictures/Facebook & Messenger
include_snapchat = false     # Snapchat memories
include_tiktok = false       # TikTok saved videos
include_signal = false       # Signal/Signal Photos & Video
include_viber = false        # Viber/media/Viber Images & Video
```

### Custom Folders

For apps not listed above, use `additional_folders`:

```toml
[android]
additional_folders = [
    "Twitter/Twitter Images",
    "Discord/Discord Media",
    "LINE/LINE Images"
]
```

### Android Quick Start

```bash
# 1. Connect your Android phone via USB
# 2. On the phone: tap "Allow" for USB file transfer
# 3. Extract photos:
photo_extraction_tool

# List connected devices (shows device type)
photo_extraction_tool list
# Output: Galaxy S24 Ultra (SM-S928B) [Android]
```

---

## ❓ Troubleshooting

### Device Not Detected

**For iOS devices:**
1. **Unlock Your Device**: iOS devices must be unlocked for access
2. **Trust the Computer**: Tap "Trust" when prompted on your iPhone/iPad
3. **Check USB Connection**: Try a different cable or port
4. **Restart Device**: Sometimes a restart resolves connection issues

**For Android devices:**
1. **Unlock Your Device**: Android must be unlocked for USB access
2. **Allow File Transfer**: When prompted, select "File Transfer" or "MTP" mode
3. **Check USB Settings**: Pull down notification shade and tap USB notification to change mode
4. **Enable USB Debugging**: Some devices may require this in Developer Options

> **Note**: No iTunes or additional drivers required. Windows 10/11 includes built-in support for iOS and Android devices via MTP.

### "No photos found"

- Make sure your device is unlocked
- Tap "Trust This Computer" if prompted on the device
- Try running `photo_extraction_tool scan` to see the folder structure

### Permission Errors

- Run as Administrator if you encounter permission issues
- Check that the output directory is writable

### Common Error Messages

| Error | Solution |
|-------|----------|
| "No devices found" | Connect device, unlock it, tap "Trust" (iOS) or "Allow" (Android) |
| "Access denied" | Unlock device or run as Administrator |
| "Setup required" | Run the tool normally to start setup wizard |
| "No photos found" (Android) | Check USB mode is set to "File Transfer" not "Charging" |

---

## 🔨 Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) 1.75 or later
- Windows 10/11
- Visual Studio Build Tools (for Windows API bindings)

### Build Steps

```bash
git clone https://github.com/yourusername/photo-extraction-tool.git
cd photo-extraction-tool
cargo build --release
# Binary at: target/release/photo_extraction_tool.exe
```

### Running Tests

```bash
# Unit tests
cargo test

# Test with mock devices (no real device needed)
cargo run --release -- test run-quick
```

---

## 📁 Project Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library root
├── cli/                 # CLI-specific code
│   ├── args.rs          # Argument definitions
│   ├── commands.rs      # Command handlers
│   └── progress.rs      # Progress display
├── core/                # Core business logic
│   ├── config.rs        # Configuration
│   ├── setup.rs         # First-run setup wizard
│   ├── extractor.rs     # Extraction logic
│   └── tracking.rs      # State tracking
├── device/              # Device interaction
│   ├── wpd.rs           # Windows Portable Devices API
│   └── profiles.rs      # Device profiles
└── duplicate/           # Duplicate detection
    └── detector.rs      # SHA256-based detection
```

---

## 🤝 Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo fmt` and `cargo test`
5. Submit a Pull Request

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<p align="center">
  Made with ❤️ in Rust
</p>
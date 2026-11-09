# Changelog

## [Unreleased]
- Linux support via GVFS (MTP, gphoto2 and afc mounts), including auto-mount with `gio`
- Platform-specific code is now gated by target OS; Windows API crate is Windows-only

## [1.0.0]
- iOS (iPhone/iPad) and Android photo/video extraction on Windows via WPD, no iTunes required
- Incremental, resumable backups with state tracking
- SHA256 duplicate detection
- Multi-device profiles, first-run setup wizard
- App media extraction (WhatsApp, Telegram, Signal, ...)
- Experimental egui desktop UI

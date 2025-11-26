//! Device interaction module
//!
//! This module provides functionality for interacting with portable devices
//! connected to Windows via the Windows Portable Devices (WPD) API.
//!
//! # Submodules
//!
//! - `wpd` - Windows Portable Devices API wrapper
//! - `profiles` - Device profile management

pub mod profiles;
pub mod wpd;

// Re-export commonly used types for convenience
pub use profiles::ProfileManager;
pub use wpd::{initialize_com, DeviceContent, DeviceInfo, DeviceManager};

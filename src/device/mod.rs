//! Device interaction module
//!
//! This module provides functionality for interacting with MTP devices (iOS, Android, cameras)
//! connected to Windows via the Windows Portable Devices (WPD) API.
//! No iTunes installation or additional drivers are required on Windows 10/11.
//!
//! # Submodules
//!
//! - `wpd` - Windows Portable Devices API wrapper
//! - `profiles` - Device profile management
//! - `traits` - Abstraction traits for testability
//!
//! # Architecture
//!
//! The module uses a trait-based abstraction to enable testing without real devices:
//!
//! - `DeviceManagerTrait` - Enumerates and opens devices
//! - `DeviceContentTrait` - Provides access to device file system
//! - `DeviceInfo` - Common device information structure
//! - `DeviceObject` - Represents files and folders on a device
//! - `DeviceType` - Enum identifying device type (Apple, Android, Camera, Unknown)
//!
//! Both the real WPD implementation and mock devices implement these traits,
//! allowing the extraction pipeline to work with either real devices or
//! simulated test devices.

#![allow(unused)]

pub mod profiles;
pub mod traits;
pub mod wpd;

// Re-export commonly used types from traits for convenience
pub use traits::{
    DeviceContentTrait, DeviceInfo, DeviceManagerTrait, DeviceObject, DeviceOperationStats,
    DeviceSimulationConfig, DeviceType,
};

// Re-export WPD-specific types
pub use profiles::ProfileManager;
pub use wpd::{
    enumerate_all_mtp_devices, enumerate_android_devices, initialize_com, ComGuard, DeviceContent,
    DeviceManager,
};

// Type aliases for backward compatibility with code using wpd::DeviceInfo
// The traits module now owns the canonical DeviceInfo type
pub mod compat {
    //! Compatibility re-exports for code using old type paths
    pub use super::traits::DeviceInfo;
    pub use super::traits::DeviceObject;
}

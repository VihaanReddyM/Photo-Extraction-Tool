//! Error types for the photo extraction tool
//!
//! This module defines the error types used throughout the application.
//! Some variants are reserved for future use or provide a complete API surface.

use thiserror::Error;

/// Main error type for the photo extraction tool
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ExtractionError {
    /// COM library initialization failed
    #[error("COM initialization failed: {0}")]
    ComError(String),

    /// General device communication error
    #[error("Device error: {0}")]
    DeviceError(String),

    /// No portable devices were found
    #[error("No devices found. Make sure your iPhone is connected and trusted.")]
    NoDevicesFound,

    /// No Apple devices (iPhone/iPad) were found
    #[error("No iPhone/iPad found. Only Apple devices are supported.")]
    NoAppleDeviceFound,

    /// Failed to access device content
    #[error("Failed to access device content: {0}")]
    ContentError(String),

    /// General I/O error
    #[error("IO error: {0}")]
    IoError(String),

    /// File transfer failed
    #[error("Transfer failed for '{filename}': {message}")]
    TransferError { filename: String, message: String },

    /// Access to the device was denied
    #[error("Access denied. Please unlock your iOS device and tap 'Trust' when prompted.")]
    AccessDenied,

    /// Device is not ready for communication
    #[error("Device not ready. Please ensure the device is unlocked.")]
    DeviceNotReady,

    /// Windows API error
    #[error("Windows API error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, ExtractionError>;

impl From<std::io::Error> for ExtractionError {
    fn from(err: std::io::Error) -> Self {
        ExtractionError::IoError(err.to_string())
    }
}

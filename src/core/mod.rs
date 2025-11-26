//! Core functionality module
//!
//! This module contains the core business logic for the photo extraction tool,
//! including configuration management, error handling, extraction logic, and
//! state tracking.
//!
//! # Submodules
//!
//! - `config` - Configuration loading, saving, and management
//! - `error` - Error types and result aliases
//! - `extractor` - Photo extraction logic
//! - `tracking` - Extraction state and session tracking

pub mod config;
pub mod error;
pub mod extractor;
pub mod tracking;

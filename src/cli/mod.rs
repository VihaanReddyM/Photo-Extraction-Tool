//! CLI module for the photo extraction tool
//!
//! This module contains all command-line interface related code including
//! argument parsing, command definitions, and command handlers.
//!
//! # Submodules
//!
//! - `args` - Command-line argument definitions using clap
//! - `commands` - Command handler implementations
//! - `progress` - Progress bars and CLI output utilities

//! unused imports will be allowed to make sure a complete version of the API is available.
#![allow(unused_imports)]

pub mod args;
pub mod commands;
pub mod progress;

// Re-export commonly used types for convenience
pub use args::{Args, Commands};
pub use commands::run_command;
pub use progress::DualWriter;

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
//! ## Core Infrastructure
//! - [`events`] - Thread-safe event types for communication between backend and UI
//! - [`controller`] - Extraction controller with async operations and cancellation
//! - [`device_monitor`] - Device hot-plug detection and state tracking
//! - [`preview`] - Thumbnail generation and preview management
//!
//! ## Zed-Style UI System
//! - [`theme`] - Theming system with colors, typography, spacing, and presets
//! - [`keybindings`] - Keyboard-first workflow with chords, contexts, and registries
//! - [`commands`] - Command palette with fuzzy search and frecency scoring
//! - [`panels`] - Pluggable panel system with docking and layout management
//! - [`settings`] - UI settings with persistence and validation
//! - [`components`] - Reusable UI components and widgets
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
//! # Zed-Inspired Design Principles
//!
//! This UI system follows Zed's design philosophy:
//!
//! 1. **Keyboard-First** - Everything can be done from the keyboard
//! 2. **Minimal Distraction** - Clean UI with toggleable panels
//! 3. **Command Palette** - Quick access to all commands via fuzzy search
//! 4. **Theming** - Dark/light modes with full customization
//! 5. **Extensible** - Pluggable panels, commands, and keybindings
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
//! # Using the Command Palette
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::commands::{CommandPalette, CommandRegistry};
//! use photo_extraction_tool::ui::keybindings::Action;
//!
//! // Create command registry and populate from actions
//! let mut registry = CommandRegistry::new();
//! registry.register_from_actions();
//!
//! // Create command palette
//! let mut palette = CommandPalette::with_registry(registry);
//!
//! // Open and search
//! palette.open();
//! palette.set_query("extract");
//!
//! // Get matches and execute
//! if let Some(selected) = palette.selected() {
//!     if let Some(action) = palette.execute_selected() {
//!         // Handle the action
//!         println!("Executing: {:?}", action);
//!     }
//! }
//! ```
//!
//! # Theming
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::theme::{Theme, ThemeManager, ThemeBuilder, Color};
//!
//! // Use built-in themes
//! let dark_theme = Theme::dark();
//! let light_theme = Theme::light();
//!
//! // Create custom theme
//! let custom = ThemeBuilder::new()
//!     .name("My Theme")
//!     .mode(photo_extraction_tool::ui::theme::ThemeMode::Dark)
//!     .accent_color(Color::from_rgb(0, 122, 255))
//!     .font_size(14.0)
//!     .build();
//!
//! // Manage themes
//! let mut manager = ThemeManager::new();
//! manager.add_theme(custom);
//! manager.set_theme_by_name("My Theme");
//! ```
//!
//! # Keybindings
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::keybindings::{
//!     KeybindingManager, KeyCombination, KeyCode, Modifiers, Action,
//! };
//!
//! // Create keybinding manager
//! let mut kb_manager = KeybindingManager::new();
//!
//! // Handle key input
//! let key = KeyCombination::new(KeyCode::Char('p'), Modifiers::CTRL_SHIFT);
//! let result = kb_manager.handle_key(key);
//!
//! match result {
//!     photo_extraction_tool::ui::keybindings::KeybindingResult::Match { action, .. } => {
//!         println!("Action: {:?}", action);
//!     }
//!     photo_extraction_tool::ui::keybindings::KeybindingResult::Pending { .. } => {
//!         println!("Waiting for chord completion...");
//!     }
//!     photo_extraction_tool::ui::keybindings::KeybindingResult::NoMatch => {
//!         // No binding for this key
//!     }
//! }
//! ```
//!
//! # Panel Management
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::panels::{PanelManager, PanelId};
//!
//! let mut panels = PanelManager::new();
//!
//! // Toggle panels
//! panels.toggle_panel(PanelId::DeviceList);
//! panels.toggle_panel(PanelId::Preview);
//!
//! // Focus management
//! panels.focus_panel(PanelId::Progress);
//! panels.focus_next();
//!
//! // Layout control
//! panels.toggle_left_sidebar();
//! panels.toggle_bottom_panel();
//! ```

// Core infrastructure modules
pub mod controller;
pub mod device_monitor;
pub mod events;
pub mod preview;

// Zed-style UI modules
pub mod commands;
pub mod components;
pub mod keybindings;
pub mod panels;
pub mod settings;
pub mod theme;

// Re-export main types from core modules for convenience
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

// Re-export main types from Zed-style modules
pub use theme::{Theme, ThemeBuilder, ThemeManager, ThemeMode, UiStyles};

pub use keybindings::{KeyBinding, KeybindingContext, KeybindingManager, KeybindingResult};

pub use commands::{Command, CommandMatch, CommandPalette, CommandRegistry, FuzzyMatcher};

pub use panels::{
    Panel, PanelConfig, PanelId, PanelLayout, PanelManager, PanelPosition, PanelSize,
};

pub use settings::{SettingsError, SettingsManager, UiSettings};

pub use components::{
    ButtonState, ButtonVariant, FocusDirection, FocusManager, InputState, ListItem, ListState,
    ProgressState, Widget, WidgetId, WidgetState,
};

/// UI Application state combining all UI managers
///
/// This provides a convenient way to manage all UI state in one place.
#[derive(Default)]
pub struct UiApp {
    /// Theme manager for colors, fonts, and styling
    pub themes: ThemeManager,

    /// Keybinding manager for keyboard shortcuts
    pub keybindings: KeybindingManager,

    /// Command palette for quick command access
    pub palette: CommandPalette,

    /// Panel manager for UI layout
    pub panels: PanelManager,

    /// Settings manager for user preferences
    pub settings: SettingsManager,

    /// Focus manager for keyboard navigation
    pub focus: FocusManager,
}

impl UiApp {
    /// Create a new UI application with default settings
    pub fn new() -> Self {
        let mut app = Self::default();

        // Register all commands from actions
        app.palette.registry_mut().register_from_actions();

        // Add keybindings to commands
        app.palette.add_keybindings(app.keybindings.registry());

        app
    }

    /// Create UI application with custom settings
    pub fn with_settings(settings: UiSettings) -> Self {
        let mut app = Self::new();
        *app.settings.settings_mut() = settings;
        app
    }

    /// Get the current theme
    pub fn theme(&self) -> std::sync::Arc<Theme> {
        self.themes.current()
    }

    /// Toggle between dark and light mode
    pub fn toggle_theme_mode(&mut self) {
        self.themes.toggle_mode();
    }

    /// Handle a key combination, returning the action if matched
    pub fn handle_key(&mut self, key: keybindings::KeyCombination) -> Option<keybindings::Action> {
        match self.keybindings.handle_key(key) {
            KeybindingResult::Match { action, .. } => Some(action),
            _ => None,
        }
    }

    /// Check if command palette is open
    pub fn is_palette_open(&self) -> bool {
        self.palette.is_open()
    }

    /// Toggle command palette
    pub fn toggle_palette(&mut self) {
        self.palette.toggle();
    }

    /// Get the current UI context for keybindings
    pub fn keybinding_context(&self) -> &KeybindingContext {
        self.keybindings.context()
    }

    /// Set the keybinding context
    pub fn set_context(&mut self, context: KeybindingContext) {
        self.keybindings.set_context(context);
    }

    /// Apply settings to the UI managers
    pub fn apply_settings(&mut self) {
        let settings = self.settings.settings();

        // Apply theme settings
        if settings.appearance.theme_mode.is_dark() {
            if !self.themes.current().mode.is_dark() {
                self.themes.toggle_mode();
            }
        } else if self.themes.current().mode.is_dark() {
            self.themes.toggle_mode();
        }

        // Apply keyboard settings
        self.keybindings
            .set_sequence_timeout(std::time::Duration::from_millis(
                settings.keyboard.chord_timeout_ms as u64,
            ));

        // Apply panel settings
        let layout = self.panels.layout_mut();
        layout.left_width = settings.panels.default_left_width;
        layout.right_width = settings.panels.default_right_width;
        layout.bottom_height = settings.panels.default_bottom_height;

        // Apply palette settings
        self.palette
            .set_max_results(settings.behavior.palette_max_results);
    }

    /// Save current state to settings
    pub fn save_state(&mut self) {
        let settings = self.settings.settings_mut();

        // Save theme mode
        settings.appearance.theme_mode = if self.themes.current().mode.is_dark() {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };

        // Save panel layout
        let layout = self.panels.layout();
        settings.panels.default_left_width = layout.left_width;
        settings.panels.default_right_width = layout.right_width;
        settings.panels.default_bottom_height = layout.bottom_height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_app_new() {
        let app = UiApp::new();
        assert!(!app.palette.is_open());
        assert!(app.palette.registry().len() > 0);
    }

    #[test]
    fn test_ui_app_toggle_palette() {
        let mut app = UiApp::new();
        assert!(!app.is_palette_open());
        app.toggle_palette();
        assert!(app.is_palette_open());
        app.toggle_palette();
        assert!(!app.is_palette_open());
    }

    #[test]
    fn test_ui_app_theme() {
        let mut app = UiApp::new();
        let initial_mode = app.theme().mode.is_dark();
        app.toggle_theme_mode();
        assert_ne!(app.theme().mode.is_dark(), initial_mode);
    }

    #[test]
    fn test_ui_app_context() {
        let mut app = UiApp::new();
        assert_eq!(*app.keybinding_context(), KeybindingContext::Global);
        app.set_context(KeybindingContext::Preview);
        assert_eq!(*app.keybinding_context(), KeybindingContext::Preview);
    }

    #[test]
    fn test_ui_app_with_settings() {
        let mut settings = UiSettings::default();
        settings.appearance.theme_mode = ThemeMode::Light;
        let app = UiApp::with_settings(settings);
        assert_eq!(
            app.settings.settings().appearance.theme_mode,
            ThemeMode::Light
        );
    }
}

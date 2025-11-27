//! Settings Module - Zed-inspired UI Settings System
//!
//! This module provides a comprehensive settings system for UI configuration,
//! inspired by Zed editor's approach. Settings can be:
//!
//! - Persisted to configuration files
//! - Changed at runtime with immediate effect
//! - Organized into logical groups
//! - Validated for correctness
//!
//! # Architecture
//!
//! The settings system is built around these core concepts:
//!
//! 1. **UiSettings** - All UI-related settings in one struct
//! 2. **SettingsGroup** - Logical grouping for organization
//! 3. **SettingValue** - Type-safe setting values
//! 4. **SettingsManager** - Load, save, and manage settings
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::settings::{UiSettings, SettingsManager};
//!
//! // Load settings
//! let mut manager = SettingsManager::new();
//! manager.load().ok();
//!
//! // Access settings
//! let font_size = manager.settings().appearance.font_size;
//!
//! // Modify settings
//! manager.settings_mut().appearance.font_size = 16.0;
//! manager.save().ok();
//! ```

use crate::ui::theme::ThemeMode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// =============================================================================
// UiSettings
// =============================================================================

/// All UI-related settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    /// Appearance settings (theme, fonts, colors)
    pub appearance: AppearanceSettings,

    /// Behavior settings (animations, confirmations)
    pub behavior: BehaviorSettings,

    /// Panel settings
    pub panels: PanelSettings,

    /// Preview settings
    pub preview: PreviewSettings,

    /// Keyboard settings
    pub keyboard: KeyboardSettings,

    /// Accessibility settings
    pub accessibility: AccessibilitySettings,

    /// Window settings
    pub window: WindowSettings,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            appearance: AppearanceSettings::default(),
            behavior: BehaviorSettings::default(),
            panels: PanelSettings::default(),
            preview: PreviewSettings::default(),
            keyboard: KeyboardSettings::default(),
            accessibility: AccessibilitySettings::default(),
            window: WindowSettings::default(),
        }
    }
}

impl UiSettings {
    /// Create settings with sensible defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate all settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        self.appearance.validate()?;
        self.behavior.validate()?;
        self.panels.validate()?;
        self.preview.validate()?;
        self.keyboard.validate()?;
        self.accessibility.validate()?;
        self.window.validate()?;
        Ok(())
    }

    /// Reset all settings to defaults
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Merge with another settings instance (other takes precedence for non-default values)
    pub fn merge(&mut self, other: &UiSettings) {
        // This is a simple implementation - you could make it more sophisticated
        // by only merging non-default values
        self.appearance = other.appearance.clone();
        self.behavior = other.behavior.clone();
        self.panels = other.panels.clone();
        self.preview = other.preview.clone();
        self.keyboard = other.keyboard.clone();
        self.accessibility = other.accessibility.clone();
        self.window = other.window.clone();
    }
}

// =============================================================================
// Appearance Settings
// =============================================================================

/// Settings related to visual appearance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    /// Theme mode (dark/light)
    pub theme_mode: ThemeMode,

    /// Custom theme name (if not using built-in)
    pub custom_theme: Option<String>,

    /// UI font family
    pub font_family: String,

    /// UI font size in pixels
    pub font_size: f32,

    /// Monospace font family
    pub mono_font_family: String,

    /// Monospace font size in pixels
    pub mono_font_size: f32,

    /// Line height multiplier
    pub line_height: f32,

    /// Enable font ligatures
    pub font_ligatures: bool,

    /// UI scale factor (1.0 = 100%)
    pub ui_scale: f32,

    /// Icon size
    pub icon_size: IconSize,

    /// Enable smooth scrolling
    pub smooth_scrolling: bool,

    /// Scrollbar visibility
    pub scrollbar_visibility: ScrollbarVisibility,

    /// Border radius style
    pub border_style: BorderStyle,

    /// Show line numbers in logs
    pub show_line_numbers: bool,

    /// Highlight current line
    pub highlight_current_line: bool,

    /// Show minimap
    pub show_minimap: bool,

    /// Minimap width
    pub minimap_width: f32,

    /// Show breadcrumbs
    pub show_breadcrumbs: bool,

    /// Show status bar
    pub show_status_bar: bool,

    /// Compact mode (reduced spacing)
    pub compact_mode: bool,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::Dark,
            custom_theme: None,
            font_family: "system-ui".to_string(),
            font_size: 14.0,
            mono_font_family: "JetBrains Mono".to_string(),
            mono_font_size: 13.0,
            line_height: 1.5,
            font_ligatures: true,
            ui_scale: 1.0,
            icon_size: IconSize::Medium,
            smooth_scrolling: true,
            scrollbar_visibility: ScrollbarVisibility::Auto,
            border_style: BorderStyle::Rounded,
            show_line_numbers: true,
            highlight_current_line: true,
            show_minimap: false,
            minimap_width: 80.0,
            show_breadcrumbs: true,
            show_status_bar: true,
            compact_mode: false,
        }
    }
}

impl AppearanceSettings {
    /// Validate appearance settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.font_size < 8.0 || self.font_size > 72.0 {
            return Err(SettingsError::InvalidValue {
                setting: "font_size".to_string(),
                reason: "Font size must be between 8 and 72".to_string(),
            });
        }

        if self.ui_scale < 0.5 || self.ui_scale > 3.0 {
            return Err(SettingsError::InvalidValue {
                setting: "ui_scale".to_string(),
                reason: "UI scale must be between 0.5 and 3.0".to_string(),
            });
        }

        if self.line_height < 1.0 || self.line_height > 3.0 {
            return Err(SettingsError::InvalidValue {
                setting: "line_height".to_string(),
                reason: "Line height must be between 1.0 and 3.0".to_string(),
            });
        }

        Ok(())
    }
}

/// Icon size options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IconSize {
    /// Small icons (16px)
    Small,
    /// Medium icons (20px)
    #[default]
    Medium,
    /// Large icons (24px)
    Large,
}

impl IconSize {
    /// Get the pixel size
    pub fn pixels(&self) -> f32 {
        match self {
            IconSize::Small => 16.0,
            IconSize::Medium => 20.0,
            IconSize::Large => 24.0,
        }
    }
}

/// Scrollbar visibility options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScrollbarVisibility {
    /// Always visible
    Always,
    /// Auto-hide when not scrolling
    #[default]
    Auto,
    /// Never visible (use scroll gestures)
    Hidden,
}

/// Border style options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    /// Sharp corners
    Sharp,
    /// Slightly rounded (4px)
    #[default]
    Rounded,
    /// Very rounded (8px)
    Pill,
}

impl BorderStyle {
    /// Get the border radius in pixels
    pub fn radius(&self) -> f32 {
        match self {
            BorderStyle::Sharp => 0.0,
            BorderStyle::Rounded => 4.0,
            BorderStyle::Pill => 8.0,
        }
    }
}

// =============================================================================
// Behavior Settings
// =============================================================================

/// Settings related to UI behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSettings {
    /// Enable animations
    pub enable_animations: bool,

    /// Animation speed (0.5 = half speed, 2.0 = double speed)
    pub animation_speed: f32,

    /// Confirm before starting extraction
    pub confirm_extraction_start: bool,

    /// Confirm before canceling extraction
    pub confirm_extraction_cancel: bool,

    /// Confirm before quit during extraction
    pub confirm_quit_during_extraction: bool,

    /// Auto-start extraction when device connected
    pub auto_start_extraction: bool,

    /// Auto-refresh device list
    pub auto_refresh_devices: bool,

    /// Device refresh interval in seconds
    pub device_refresh_interval: u32,

    /// Show notifications
    pub show_notifications: bool,

    /// Play sounds for events
    pub play_sounds: bool,

    /// Sound volume (0.0 - 1.0)
    pub sound_volume: f32,

    /// Remember window position
    pub remember_window_position: bool,

    /// Remember panel layout
    pub remember_panel_layout: bool,

    /// Open output folder after extraction
    pub open_folder_after_extraction: bool,

    /// Close palette after action
    pub close_palette_after_action: bool,

    /// Single click to select, double to open
    pub double_click_to_open: bool,

    /// Show tooltips
    pub show_tooltips: bool,

    /// Tooltip delay in milliseconds
    pub tooltip_delay_ms: u32,

    /// Auto-hide command palette results count
    pub palette_max_results: usize,
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            enable_animations: true,
            animation_speed: 1.0,
            confirm_extraction_start: false,
            confirm_extraction_cancel: true,
            confirm_quit_during_extraction: true,
            auto_start_extraction: false,
            auto_refresh_devices: true,
            device_refresh_interval: 2,
            show_notifications: true,
            play_sounds: false,
            sound_volume: 0.5,
            remember_window_position: true,
            remember_panel_layout: true,
            open_folder_after_extraction: false,
            close_palette_after_action: true,
            double_click_to_open: true,
            show_tooltips: true,
            tooltip_delay_ms: 500,
            palette_max_results: 20,
        }
    }
}

impl BehaviorSettings {
    /// Validate behavior settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.animation_speed < 0.1 || self.animation_speed > 5.0 {
            return Err(SettingsError::InvalidValue {
                setting: "animation_speed".to_string(),
                reason: "Animation speed must be between 0.1 and 5.0".to_string(),
            });
        }

        if self.sound_volume < 0.0 || self.sound_volume > 1.0 {
            return Err(SettingsError::InvalidValue {
                setting: "sound_volume".to_string(),
                reason: "Sound volume must be between 0.0 and 1.0".to_string(),
            });
        }

        if self.device_refresh_interval < 1 || self.device_refresh_interval > 60 {
            return Err(SettingsError::InvalidValue {
                setting: "device_refresh_interval".to_string(),
                reason: "Device refresh interval must be between 1 and 60 seconds".to_string(),
            });
        }

        Ok(())
    }
}

// =============================================================================
// Panel Settings
// =============================================================================

/// Settings related to panel layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelSettings {
    /// Show panel headers
    pub show_headers: bool,

    /// Show panel icons
    pub show_icons: bool,

    /// Default left sidebar width
    pub default_left_width: f32,

    /// Default right sidebar width
    pub default_right_width: f32,

    /// Default bottom panel height
    pub default_bottom_height: f32,

    /// Minimum panel size
    pub min_panel_size: f32,

    /// Enable panel drag-and-drop
    pub enable_panel_dnd: bool,

    /// Show panel close buttons
    pub show_close_buttons: bool,

    /// Show resize handles
    pub show_resize_handles: bool,

    /// Resize handle width
    pub resize_handle_width: f32,

    /// Collapse panels to icons
    pub collapse_to_icons: bool,
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self {
            show_headers: true,
            show_icons: true,
            default_left_width: 250.0,
            default_right_width: 300.0,
            default_bottom_height: 200.0,
            min_panel_size: 100.0,
            enable_panel_dnd: true,
            show_close_buttons: true,
            show_resize_handles: true,
            resize_handle_width: 4.0,
            collapse_to_icons: true,
        }
    }
}

impl PanelSettings {
    /// Validate panel settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.default_left_width < 50.0 || self.default_left_width > 800.0 {
            return Err(SettingsError::InvalidValue {
                setting: "default_left_width".to_string(),
                reason: "Width must be between 50 and 800 pixels".to_string(),
            });
        }

        if self.min_panel_size < 50.0 {
            return Err(SettingsError::InvalidValue {
                setting: "min_panel_size".to_string(),
                reason: "Minimum panel size must be at least 50 pixels".to_string(),
            });
        }

        Ok(())
    }
}

// =============================================================================
// Preview Settings
// =============================================================================

/// Settings related to photo/video preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewSettings {
    /// Thumbnail size
    pub thumbnail_size: ThumbnailSize,

    /// Custom thumbnail width (if size is Custom)
    pub custom_thumbnail_width: u32,

    /// Show file names under thumbnails
    pub show_file_names: bool,

    /// Show file sizes
    pub show_file_sizes: bool,

    /// Show dates
    pub show_dates: bool,

    /// Enable thumbnail caching
    pub enable_cache: bool,

    /// Maximum cache size in MB
    pub max_cache_size_mb: u32,

    /// View mode (grid/list)
    pub default_view_mode: ViewMode,

    /// Grid column count (0 = auto)
    pub grid_columns: u32,

    /// Grid gap in pixels
    pub grid_gap: f32,

    /// Sort by default
    pub default_sort: SortBy,

    /// Sort direction
    pub sort_ascending: bool,

    /// Enable lazy loading
    pub lazy_loading: bool,

    /// Load ahead count for lazy loading
    pub load_ahead_count: usize,
}

impl Default for PreviewSettings {
    fn default() -> Self {
        Self {
            thumbnail_size: ThumbnailSize::Medium,
            custom_thumbnail_width: 128,
            show_file_names: true,
            show_file_sizes: true,
            show_dates: true,
            enable_cache: true,
            max_cache_size_mb: 256,
            default_view_mode: ViewMode::Grid,
            grid_columns: 0,
            grid_gap: 8.0,
            default_sort: SortBy::Date,
            sort_ascending: false,
            lazy_loading: true,
            load_ahead_count: 20,
        }
    }
}

impl PreviewSettings {
    /// Validate preview settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.max_cache_size_mb < 16 || self.max_cache_size_mb > 4096 {
            return Err(SettingsError::InvalidValue {
                setting: "max_cache_size_mb".to_string(),
                reason: "Cache size must be between 16 and 4096 MB".to_string(),
            });
        }

        if self.custom_thumbnail_width < 32 || self.custom_thumbnail_width > 512 {
            return Err(SettingsError::InvalidValue {
                setting: "custom_thumbnail_width".to_string(),
                reason: "Thumbnail width must be between 32 and 512 pixels".to_string(),
            });
        }

        Ok(())
    }

    /// Get the actual thumbnail width
    pub fn thumbnail_width(&self) -> u32 {
        match self.thumbnail_size {
            ThumbnailSize::Small => 64,
            ThumbnailSize::Medium => 128,
            ThumbnailSize::Large => 256,
            ThumbnailSize::Custom => self.custom_thumbnail_width,
        }
    }
}

/// Thumbnail size presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThumbnailSize {
    /// Small (64px)
    Small,
    /// Medium (128px)
    #[default]
    Medium,
    /// Large (256px)
    Large,
    /// Custom size
    Custom,
}

/// View mode for file display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    /// Grid view with thumbnails
    #[default]
    Grid,
    /// List view
    List,
    /// Details view (list with columns)
    Details,
}

/// Sort options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    /// Sort by name
    Name,
    /// Sort by date
    #[default]
    Date,
    /// Sort by size
    Size,
    /// Sort by type/extension
    Type,
}

// =============================================================================
// Keyboard Settings
// =============================================================================

/// Settings related to keyboard behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardSettings {
    /// Enable keyboard shortcuts
    pub enable_shortcuts: bool,

    /// Key repeat delay in milliseconds
    pub key_repeat_delay_ms: u32,

    /// Key repeat rate in milliseconds
    pub key_repeat_rate_ms: u32,

    /// Enable vim-style navigation (hjkl)
    pub vim_navigation: bool,

    /// Enable emacs-style shortcuts
    pub emacs_shortcuts: bool,

    /// Chord timeout in milliseconds
    pub chord_timeout_ms: u32,

    /// Show keyboard shortcut hints
    pub show_shortcut_hints: bool,

    /// Keybinding preset name
    pub keybinding_preset: String,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        Self {
            enable_shortcuts: true,
            key_repeat_delay_ms: 500,
            key_repeat_rate_ms: 50,
            vim_navigation: false,
            emacs_shortcuts: false,
            chord_timeout_ms: 1500,
            show_shortcut_hints: true,
            keybinding_preset: "default".to_string(),
        }
    }
}

impl KeyboardSettings {
    /// Validate keyboard settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.key_repeat_delay_ms < 100 || self.key_repeat_delay_ms > 2000 {
            return Err(SettingsError::InvalidValue {
                setting: "key_repeat_delay_ms".to_string(),
                reason: "Key repeat delay must be between 100 and 2000 ms".to_string(),
            });
        }

        if self.chord_timeout_ms < 500 || self.chord_timeout_ms > 5000 {
            return Err(SettingsError::InvalidValue {
                setting: "chord_timeout_ms".to_string(),
                reason: "Chord timeout must be between 500 and 5000 ms".to_string(),
            });
        }

        Ok(())
    }
}

// =============================================================================
// Accessibility Settings
// =============================================================================

/// Settings related to accessibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilitySettings {
    /// Enable reduced motion
    pub reduce_motion: bool,

    /// Enable high contrast mode
    pub high_contrast: bool,

    /// Enable screen reader support
    pub screen_reader: bool,

    /// Focus indicator visibility
    pub focus_visible: FocusVisibility,

    /// Minimum touch target size
    pub min_touch_target: f32,

    /// Enable color blind mode
    pub color_blind_mode: ColorBlindMode,

    /// Text spacing adjustment
    pub text_spacing: f32,

    /// Enable dyslexia-friendly font
    pub dyslexia_font: bool,

    /// Enable larger click targets
    pub large_click_targets: bool,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            reduce_motion: false,
            high_contrast: false,
            screen_reader: false,
            focus_visible: FocusVisibility::KeyboardOnly,
            min_touch_target: 44.0,
            color_blind_mode: ColorBlindMode::None,
            text_spacing: 1.0,
            dyslexia_font: false,
            large_click_targets: false,
        }
    }
}

impl AccessibilitySettings {
    /// Validate accessibility settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.text_spacing < 0.5 || self.text_spacing > 2.0 {
            return Err(SettingsError::InvalidValue {
                setting: "text_spacing".to_string(),
                reason: "Text spacing must be between 0.5 and 2.0".to_string(),
            });
        }

        if self.min_touch_target < 24.0 || self.min_touch_target > 96.0 {
            return Err(SettingsError::InvalidValue {
                setting: "min_touch_target".to_string(),
                reason: "Minimum touch target must be between 24 and 96 pixels".to_string(),
            });
        }

        Ok(())
    }
}

/// Focus indicator visibility options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FocusVisibility {
    /// Always show focus indicator
    Always,
    /// Only show for keyboard navigation
    #[default]
    KeyboardOnly,
    /// Never show (not recommended)
    Never,
}

/// Color blind simulation modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColorBlindMode {
    /// Normal vision
    #[default]
    None,
    /// Protanopia (red-blind)
    Protanopia,
    /// Deuteranopia (green-blind)
    Deuteranopia,
    /// Tritanopia (blue-blind)
    Tritanopia,
    /// Achromatopsia (total color blindness)
    Achromatopsia,
}

// =============================================================================
// Window Settings
// =============================================================================

/// Settings related to window behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    /// Start maximized
    pub start_maximized: bool,

    /// Start fullscreen
    pub start_fullscreen: bool,

    /// Window width (if not maximized)
    pub default_width: u32,

    /// Window height (if not maximized)
    pub default_height: u32,

    /// Minimum window width
    pub min_width: u32,

    /// Minimum window height
    pub min_height: u32,

    /// Enable window transparency
    pub transparent: bool,

    /// Window decorations (title bar, borders)
    pub decorations: bool,

    /// Always on top
    pub always_on_top: bool,

    /// Remember last position
    pub remember_position: bool,

    /// Last window x position
    #[serde(default)]
    pub last_x: Option<i32>,

    /// Last window y position
    #[serde(default)]
    pub last_y: Option<i32>,

    /// Last window width
    #[serde(default)]
    pub last_width: Option<u32>,

    /// Last window height
    #[serde(default)]
    pub last_height: Option<u32>,

    /// Last window maximized state
    #[serde(default)]
    pub last_maximized: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            start_maximized: false,
            start_fullscreen: false,
            default_width: 1200,
            default_height: 800,
            min_width: 800,
            min_height: 600,
            transparent: false,
            decorations: true,
            always_on_top: false,
            remember_position: true,
            last_x: None,
            last_y: None,
            last_width: None,
            last_height: None,
            last_maximized: false,
        }
    }
}

impl WindowSettings {
    /// Validate window settings
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.default_width < self.min_width {
            return Err(SettingsError::InvalidValue {
                setting: "default_width".to_string(),
                reason: "Default width cannot be less than minimum width".to_string(),
            });
        }

        if self.default_height < self.min_height {
            return Err(SettingsError::InvalidValue {
                setting: "default_height".to_string(),
                reason: "Default height cannot be less than minimum height".to_string(),
            });
        }

        Ok(())
    }

    /// Get the startup size
    pub fn startup_size(&self) -> (u32, u32) {
        if self.remember_position {
            (
                self.last_width.unwrap_or(self.default_width),
                self.last_height.unwrap_or(self.default_height),
            )
        } else {
            (self.default_width, self.default_height)
        }
    }

    /// Save current window state
    pub fn save_state(&mut self, x: i32, y: i32, width: u32, height: u32, maximized: bool) {
        self.last_x = Some(x);
        self.last_y = Some(y);
        self.last_width = Some(width);
        self.last_height = Some(height);
        self.last_maximized = maximized;
    }
}

// =============================================================================
// SettingsError
// =============================================================================

/// Errors that can occur when working with settings
#[derive(Debug, Clone, thiserror::Error)]
pub enum SettingsError {
    /// Invalid setting value
    #[error("Invalid value for {setting}: {reason}")]
    InvalidValue { setting: String, reason: String },

    /// Failed to load settings
    #[error("Failed to load settings: {0}")]
    LoadError(String),

    /// Failed to save settings
    #[error("Failed to save settings: {0}")]
    SaveError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

// =============================================================================
// SettingsManager
// =============================================================================

/// Manages loading, saving, and accessing settings
#[derive(Debug, Clone)]
pub struct SettingsManager {
    /// Current settings
    settings: UiSettings,

    /// Path to settings file
    settings_path: PathBuf,

    /// Whether settings have been modified
    dirty: bool,
}

impl SettingsManager {
    /// Create a new settings manager with default path
    pub fn new() -> Self {
        let settings_path = Self::default_settings_path();

        Self {
            settings: UiSettings::default(),
            settings_path,
            dirty: false,
        }
    }

    /// Create with a custom settings path
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            settings: UiSettings::default(),
            settings_path: path,
            dirty: false,
        }
    }

    /// Get the default settings path
    pub fn default_settings_path() -> PathBuf {
        // Use APPDATA on Windows
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut path = PathBuf::from(appdata);
            path.push("photo_extraction_tool");
            path.push("ui_settings.toml");
            return path;
        }

        // Fallback to current directory
        PathBuf::from("ui_settings.toml")
    }

    /// Get the current settings
    pub fn settings(&self) -> &UiSettings {
        &self.settings
    }

    /// Get mutable settings
    pub fn settings_mut(&mut self) -> &mut UiSettings {
        self.dirty = true;
        &mut self.settings
    }

    /// Check if settings have been modified
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Load settings from file
    pub fn load(&mut self) -> Result<(), SettingsError> {
        if !self.settings_path.exists() {
            // File doesn't exist, use defaults
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.settings_path)
            .map_err(|e| SettingsError::IoError(e.to_string()))?;

        self.settings = toml::from_str(&content)
            .map_err(|e| SettingsError::SerializationError(e.to_string()))?;

        // Validate loaded settings
        self.settings.validate()?;

        self.dirty = false;
        Ok(())
    }

    /// Save settings to file
    pub fn save(&mut self) -> Result<(), SettingsError> {
        // Validate before saving
        self.settings.validate()?;

        // Create directory if it doesn't exist
        if let Some(parent) = self.settings_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SettingsError::IoError(e.to_string()))?;
        }

        let content = toml::to_string_pretty(&self.settings)
            .map_err(|e| SettingsError::SaveError(e.to_string()))?;

        std::fs::write(&self.settings_path, content)
            .map_err(|e| SettingsError::IoError(e.to_string()))?;

        self.dirty = false;
        Ok(())
    }

    /// Reset settings to defaults
    pub fn reset(&mut self) {
        self.settings.reset();
        self.dirty = true;
    }

    /// Reset and save
    pub fn reset_and_save(&mut self) -> Result<(), SettingsError> {
        self.reset();
        self.save()
    }

    /// Get the settings file path
    pub fn path(&self) -> &PathBuf {
        &self.settings_path
    }

    /// Import settings from a file
    pub fn import(&mut self, path: &PathBuf) -> Result<(), SettingsError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| SettingsError::IoError(e.to_string()))?;

        let imported: UiSettings = toml::from_str(&content)
            .map_err(|e| SettingsError::SerializationError(e.to_string()))?;

        imported.validate()?;

        self.settings = imported;
        self.dirty = true;
        Ok(())
    }

    /// Export settings to a file
    pub fn export(&self, path: &PathBuf) -> Result<(), SettingsError> {
        let content = toml::to_string_pretty(&self.settings)
            .map_err(|e| SettingsError::SaveError(e.to_string()))?;

        std::fs::write(path, content).map_err(|e| SettingsError::IoError(e.to_string()))?;

        Ok(())
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_settings_default() {
        let settings = UiSettings::default();
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_appearance_validation() {
        let mut settings = AppearanceSettings::default();
        assert!(settings.validate().is_ok());

        settings.font_size = 5.0;
        assert!(settings.validate().is_err());

        settings.font_size = 14.0;
        settings.ui_scale = 5.0;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_behavior_validation() {
        let mut settings = BehaviorSettings::default();
        assert!(settings.validate().is_ok());

        settings.animation_speed = 0.0;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_preview_thumbnail_width() {
        let mut settings = PreviewSettings::default();

        settings.thumbnail_size = ThumbnailSize::Small;
        assert_eq!(settings.thumbnail_width(), 64);

        settings.thumbnail_size = ThumbnailSize::Medium;
        assert_eq!(settings.thumbnail_width(), 128);

        settings.thumbnail_size = ThumbnailSize::Custom;
        settings.custom_thumbnail_width = 200;
        assert_eq!(settings.thumbnail_width(), 200);
    }

    #[test]
    fn test_icon_size_pixels() {
        assert_eq!(IconSize::Small.pixels(), 16.0);
        assert_eq!(IconSize::Medium.pixels(), 20.0);
        assert_eq!(IconSize::Large.pixels(), 24.0);
    }

    #[test]
    fn test_border_style_radius() {
        assert_eq!(BorderStyle::Sharp.radius(), 0.0);
        assert_eq!(BorderStyle::Rounded.radius(), 4.0);
        assert_eq!(BorderStyle::Pill.radius(), 8.0);
    }

    #[test]
    fn test_window_startup_size() {
        let mut settings = WindowSettings::default();
        settings.default_width = 1000;
        settings.default_height = 700;

        let (w, h) = settings.startup_size();
        assert_eq!(w, 1000);
        assert_eq!(h, 700);

        settings.save_state(100, 100, 1200, 800, false);
        let (w, h) = settings.startup_size();
        assert_eq!(w, 1200);
        assert_eq!(h, 800);
    }

    #[test]
    fn test_settings_manager_new() {
        let manager = SettingsManager::new();
        assert!(!manager.is_dirty());
    }

    #[test]
    fn test_settings_manager_dirty() {
        let mut manager = SettingsManager::new();
        assert!(!manager.is_dirty());

        let _ = manager.settings_mut();
        assert!(manager.is_dirty());
    }

    #[test]
    fn test_settings_reset() {
        let mut settings = UiSettings::default();
        settings.appearance.font_size = 20.0;

        settings.reset();
        assert_eq!(settings.appearance.font_size, 14.0);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = UiSettings::default();
        let toml = toml::to_string(&settings).unwrap();
        let deserialized: UiSettings = toml::from_str(&toml).unwrap();

        assert_eq!(
            settings.appearance.font_size,
            deserialized.appearance.font_size
        );
    }

    #[test]
    fn test_keyboard_settings_validation() {
        let mut settings = KeyboardSettings::default();
        assert!(settings.validate().is_ok());

        settings.chord_timeout_ms = 100;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_accessibility_settings_validation() {
        let mut settings = AccessibilitySettings::default();
        assert!(settings.validate().is_ok());

        settings.text_spacing = 3.0;
        assert!(settings.validate().is_err());
    }
}

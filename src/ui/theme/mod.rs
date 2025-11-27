//! Theme Module - Zed-inspired Theming System
//!
//! This module provides a comprehensive theming system inspired by Zed editor's
//! minimal, keyboard-first design philosophy. It supports:
//!
//! - Multiple color schemes (dark/light variants)
//! - Configurable fonts and typography
//! - Consistent spacing and sizing tokens
//! - UI component styling
//! - Theme serialization for persistence
//!
//! # Architecture
//!
//! The theme system is built around three core concepts:
//!
//! 1. **Color Palette** - Semantic color definitions for UI elements
//! 2. **Typography** - Font families, sizes, and weights
//! 3. **Spacing** - Consistent spacing tokens for layout
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::theme::{Theme, ThemeMode, ThemeBuilder, Color};
//!
//! // Use built-in themes
//! let dark_theme = Theme::dark();
//! let light_theme = Theme::light();
//!
//! // Or customize
//! let custom = ThemeBuilder::new()
//!     .name("Custom Theme")
//!     .mode(ThemeMode::Dark)
//!     .accent_color(Color::from_hex("#7C3AED"))
//!     .font_family("JetBrains Mono")
//!     .build();
//! ```

pub mod colors;
pub mod fonts;
pub mod spacing;

pub use colors::{Color, ColorPalette, SemanticColors};
pub use fonts::{FontConfig, FontFamily, FontWeight, Typography};
pub use spacing::{Spacing, SpacingScale};

use serde::{Deserialize, Serialize};
use std::sync::Arc;

// =============================================================================
// Theme Mode
// =============================================================================

/// Theme mode - dark or light
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// Dark theme (default) - easy on the eyes, Zed's preferred mode
    #[default]
    Dark,
    /// Light theme - high contrast for bright environments
    Light,
}

impl ThemeMode {
    /// Toggle between dark and light mode
    pub fn toggle(&self) -> Self {
        match self {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        }
    }

    /// Check if this is dark mode
    pub fn is_dark(&self) -> bool {
        matches!(self, ThemeMode::Dark)
    }

    /// Check if this is light mode
    pub fn is_light(&self) -> bool {
        matches!(self, ThemeMode::Light)
    }
}

impl std::fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeMode::Dark => write!(f, "Dark"),
            ThemeMode::Light => write!(f, "Light"),
        }
    }
}

// =============================================================================
// Theme
// =============================================================================

/// Complete theme definition containing colors, typography, and spacing.
///
/// This is the main struct used throughout the application for consistent styling.
/// Themes are immutable once created - use `ThemeBuilder` to create modified versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Theme name (e.g., "Zed Dark", "Zed Light")
    pub name: String,

    /// Theme mode (dark/light)
    pub mode: ThemeMode,

    /// Color palette
    pub colors: ColorPalette,

    /// Typography settings
    pub typography: Typography,

    /// Spacing scale
    pub spacing: Spacing,

    /// UI-specific styles
    pub ui: UiStyles,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Create the default dark theme (Zed-inspired)
    pub fn dark() -> Self {
        Self {
            name: "Zed Dark".to_string(),
            mode: ThemeMode::Dark,
            colors: ColorPalette::dark(),
            typography: Typography::default(),
            spacing: Spacing::default(),
            ui: UiStyles::dark(),
        }
    }

    /// Create the light theme
    pub fn light() -> Self {
        Self {
            name: "Zed Light".to_string(),
            mode: ThemeMode::Light,
            colors: ColorPalette::light(),
            typography: Typography::default(),
            spacing: Spacing::default(),
            ui: UiStyles::light(),
        }
    }

    /// Create a high contrast dark theme
    pub fn high_contrast_dark() -> Self {
        Self {
            name: "High Contrast Dark".to_string(),
            mode: ThemeMode::Dark,
            colors: ColorPalette::high_contrast_dark(),
            typography: Typography::default(),
            spacing: Spacing::default(),
            ui: UiStyles::high_contrast_dark(),
        }
    }

    /// Get a builder initialized with this theme's values
    pub fn to_builder(&self) -> ThemeBuilder {
        ThemeBuilder::from_theme(self.clone())
    }

    /// Create a new theme with a different mode
    pub fn with_mode(&self, mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
        }
    }

    /// Toggle between dark and light mode
    pub fn toggle_mode(&self) -> Self {
        self.with_mode(self.mode.toggle())
    }

    /// Get the semantic colors for this theme
    pub fn semantic(&self) -> &SemanticColors {
        &self.colors.semantic
    }

    /// Wrap in Arc for shared ownership
    pub fn into_arc(self) -> Arc<Theme> {
        Arc::new(self)
    }
}

// =============================================================================
// UI Styles
// =============================================================================

/// UI-specific style definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiStyles {
    /// Panel styles
    pub panel: PanelStyle,

    /// Button styles
    pub button: ButtonStyle,

    /// Input field styles
    pub input: InputStyle,

    /// List/table styles
    pub list: ListStyle,

    /// Progress bar styles
    pub progress: ProgressStyle,

    /// Command palette styles
    pub command_palette: CommandPaletteStyle,

    /// Status bar styles
    pub status_bar: StatusBarStyle,

    /// Tooltip styles
    pub tooltip: TooltipStyle,

    /// Focus ring styles
    pub focus: FocusStyle,

    /// Scrollbar styles
    pub scrollbar: ScrollbarStyle,

    /// Border radius values
    pub border_radius: BorderRadius,

    /// Animation/transition durations in milliseconds
    pub animation: AnimationConfig,
}

impl UiStyles {
    /// Create dark UI styles
    pub fn dark() -> Self {
        Self {
            panel: PanelStyle::dark(),
            button: ButtonStyle::dark(),
            input: InputStyle::dark(),
            list: ListStyle::dark(),
            progress: ProgressStyle::dark(),
            command_palette: CommandPaletteStyle::dark(),
            status_bar: StatusBarStyle::dark(),
            tooltip: TooltipStyle::dark(),
            focus: FocusStyle::dark(),
            scrollbar: ScrollbarStyle::dark(),
            border_radius: BorderRadius::default(),
            animation: AnimationConfig::default(),
        }
    }

    /// Create light UI styles
    pub fn light() -> Self {
        Self {
            panel: PanelStyle::light(),
            button: ButtonStyle::light(),
            input: InputStyle::light(),
            list: ListStyle::light(),
            progress: ProgressStyle::light(),
            command_palette: CommandPaletteStyle::light(),
            status_bar: StatusBarStyle::light(),
            tooltip: TooltipStyle::light(),
            focus: FocusStyle::light(),
            scrollbar: ScrollbarStyle::light(),
            border_radius: BorderRadius::default(),
            animation: AnimationConfig::default(),
        }
    }

    /// Create high contrast dark UI styles
    pub fn high_contrast_dark() -> Self {
        Self {
            panel: PanelStyle::high_contrast_dark(),
            button: ButtonStyle::high_contrast_dark(),
            input: InputStyle::high_contrast_dark(),
            list: ListStyle::high_contrast_dark(),
            progress: ProgressStyle::high_contrast_dark(),
            command_palette: CommandPaletteStyle::high_contrast_dark(),
            status_bar: StatusBarStyle::high_contrast_dark(),
            tooltip: TooltipStyle::high_contrast_dark(),
            focus: FocusStyle::high_contrast_dark(),
            scrollbar: ScrollbarStyle::high_contrast_dark(),
            border_radius: BorderRadius::default(),
            animation: AnimationConfig::default(),
        }
    }
}

impl Default for UiStyles {
    fn default() -> Self {
        Self::dark()
    }
}

// =============================================================================
// Component Styles
// =============================================================================

/// Panel/container styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelStyle {
    pub background: Color,
    pub background_secondary: Color,
    pub border: Color,
    pub shadow: Option<Shadow>,
}

impl PanelStyle {
    pub fn dark() -> Self {
        Self {
            background: Color::from_hex("#1E1E2E"),
            background_secondary: Color::from_hex("#181825"),
            border: Color::from_hex("#313244"),
            shadow: Some(Shadow {
                color: Color::from_rgba(0, 0, 0, 0.3),
                offset_x: 0.0,
                offset_y: 2.0,
                blur: 8.0,
            }),
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::from_hex("#FFFFFF"),
            background_secondary: Color::from_hex("#F5F5F5"),
            border: Color::from_hex("#E0E0E0"),
            shadow: Some(Shadow {
                color: Color::from_rgba(0, 0, 0, 0.1),
                offset_x: 0.0,
                offset_y: 2.0,
                blur: 8.0,
            }),
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            background: Color::from_hex("#0D0D0D"),
            background_secondary: Color::from_hex("#000000"),
            border: Color::from_hex("#FFFFFF"),
            shadow: None,
        }
    }
}

/// Button styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonStyle {
    pub background: Color,
    pub background_hover: Color,
    pub background_active: Color,
    pub background_disabled: Color,
    pub text: Color,
    pub text_disabled: Color,
    pub border: Option<Color>,
    pub primary_background: Color,
    pub primary_hover: Color,
    pub primary_text: Color,
}

impl ButtonStyle {
    pub fn dark() -> Self {
        Self {
            background: Color::from_hex("#313244"),
            background_hover: Color::from_hex("#45475A"),
            background_active: Color::from_hex("#585B70"),
            background_disabled: Color::from_hex("#1E1E2E"),
            text: Color::from_hex("#CDD6F4"),
            text_disabled: Color::from_hex("#6C7086"),
            border: None,
            primary_background: Color::from_hex("#89B4FA"),
            primary_hover: Color::from_hex("#B4BEFE"),
            primary_text: Color::from_hex("#1E1E2E"),
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::from_hex("#E0E0E0"),
            background_hover: Color::from_hex("#D0D0D0"),
            background_active: Color::from_hex("#C0C0C0"),
            background_disabled: Color::from_hex("#F5F5F5"),
            text: Color::from_hex("#1E1E2E"),
            text_disabled: Color::from_hex("#9CA0B0"),
            border: Some(Color::from_hex("#C0C0C0")),
            primary_background: Color::from_hex("#1E66F5"),
            primary_hover: Color::from_hex("#2B7BF7"),
            primary_text: Color::from_hex("#FFFFFF"),
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            background: Color::from_hex("#1A1A1A"),
            background_hover: Color::from_hex("#333333"),
            background_active: Color::from_hex("#4D4D4D"),
            background_disabled: Color::from_hex("#0D0D0D"),
            text: Color::from_hex("#FFFFFF"),
            text_disabled: Color::from_hex("#666666"),
            border: Some(Color::from_hex("#FFFFFF")),
            primary_background: Color::from_hex("#00FF00"),
            primary_hover: Color::from_hex("#33FF33"),
            primary_text: Color::from_hex("#000000"),
        }
    }
}

/// Input field styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputStyle {
    pub background: Color,
    pub background_focused: Color,
    pub border: Color,
    pub border_focused: Color,
    pub text: Color,
    pub placeholder: Color,
    pub selection: Color,
    pub cursor: Color,
}

impl InputStyle {
    pub fn dark() -> Self {
        Self {
            background: Color::from_hex("#1E1E2E"),
            background_focused: Color::from_hex("#181825"),
            border: Color::from_hex("#313244"),
            border_focused: Color::from_hex("#89B4FA"),
            text: Color::from_hex("#CDD6F4"),
            placeholder: Color::from_hex("#6C7086"),
            selection: Color::from_rgba(137, 180, 250, 0.3),
            cursor: Color::from_hex("#F5E0DC"),
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::from_hex("#FFFFFF"),
            background_focused: Color::from_hex("#FFFFFF"),
            border: Color::from_hex("#C0C0C0"),
            border_focused: Color::from_hex("#1E66F5"),
            text: Color::from_hex("#1E1E2E"),
            placeholder: Color::from_hex("#9CA0B0"),
            selection: Color::from_rgba(30, 102, 245, 0.2),
            cursor: Color::from_hex("#1E66F5"),
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            background: Color::from_hex("#000000"),
            background_focused: Color::from_hex("#000000"),
            border: Color::from_hex("#FFFFFF"),
            border_focused: Color::from_hex("#00FF00"),
            text: Color::from_hex("#FFFFFF"),
            placeholder: Color::from_hex("#808080"),
            selection: Color::from_rgba(0, 255, 0, 0.3),
            cursor: Color::from_hex("#00FF00"),
        }
    }
}

/// List/table styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListStyle {
    pub background: Color,
    pub item_hover: Color,
    pub item_selected: Color,
    pub item_active: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub divider: Color,
    pub stripe: Option<Color>,
}

impl ListStyle {
    pub fn dark() -> Self {
        Self {
            background: Color::TRANSPARENT,
            item_hover: Color::from_rgba(69, 71, 90, 0.5),
            item_selected: Color::from_rgba(137, 180, 250, 0.2),
            item_active: Color::from_rgba(137, 180, 250, 0.3),
            text: Color::from_hex("#CDD6F4"),
            text_secondary: Color::from_hex("#A6ADC8"),
            divider: Color::from_hex("#313244"),
            stripe: Some(Color::from_rgba(30, 30, 46, 0.5)),
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::TRANSPARENT,
            item_hover: Color::from_rgba(0, 0, 0, 0.05),
            item_selected: Color::from_rgba(30, 102, 245, 0.1),
            item_active: Color::from_rgba(30, 102, 245, 0.15),
            text: Color::from_hex("#1E1E2E"),
            text_secondary: Color::from_hex("#5C5F77"),
            divider: Color::from_hex("#E0E0E0"),
            stripe: Some(Color::from_rgba(0, 0, 0, 0.02)),
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            background: Color::TRANSPARENT,
            item_hover: Color::from_rgba(255, 255, 255, 0.1),
            item_selected: Color::from_rgba(0, 255, 0, 0.2),
            item_active: Color::from_rgba(0, 255, 0, 0.3),
            text: Color::from_hex("#FFFFFF"),
            text_secondary: Color::from_hex("#CCCCCC"),
            divider: Color::from_hex("#FFFFFF"),
            stripe: None,
        }
    }
}

/// Progress bar styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressStyle {
    pub track: Color,
    pub fill: Color,
    pub fill_success: Color,
    pub fill_warning: Color,
    pub fill_error: Color,
    pub text: Color,
    pub height: f32,
}

impl ProgressStyle {
    pub fn dark() -> Self {
        Self {
            track: Color::from_hex("#313244"),
            fill: Color::from_hex("#89B4FA"),
            fill_success: Color::from_hex("#A6E3A1"),
            fill_warning: Color::from_hex("#F9E2AF"),
            fill_error: Color::from_hex("#F38BA8"),
            text: Color::from_hex("#CDD6F4"),
            height: 8.0,
        }
    }

    pub fn light() -> Self {
        Self {
            track: Color::from_hex("#E0E0E0"),
            fill: Color::from_hex("#1E66F5"),
            fill_success: Color::from_hex("#40A02B"),
            fill_warning: Color::from_hex("#DF8E1D"),
            fill_error: Color::from_hex("#D20F39"),
            text: Color::from_hex("#1E1E2E"),
            height: 8.0,
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            track: Color::from_hex("#333333"),
            fill: Color::from_hex("#00FF00"),
            fill_success: Color::from_hex("#00FF00"),
            fill_warning: Color::from_hex("#FFFF00"),
            fill_error: Color::from_hex("#FF0000"),
            text: Color::from_hex("#FFFFFF"),
            height: 10.0,
        }
    }
}

/// Command palette styling (Zed's Ctrl+P / Cmd+K)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPaletteStyle {
    pub backdrop: Color,
    pub background: Color,
    pub border: Color,
    pub input_background: Color,
    pub item_hover: Color,
    pub item_selected: Color,
    pub text: Color,
    pub text_muted: Color,
    pub highlight: Color,
    pub shortcut_background: Color,
    pub shortcut_text: Color,
    pub max_width: f32,
    pub max_height: f32,
}

impl CommandPaletteStyle {
    pub fn dark() -> Self {
        Self {
            backdrop: Color::from_rgba(0, 0, 0, 0.5),
            background: Color::from_hex("#1E1E2E"),
            border: Color::from_hex("#313244"),
            input_background: Color::from_hex("#181825"),
            item_hover: Color::from_rgba(69, 71, 90, 0.5),
            item_selected: Color::from_rgba(137, 180, 250, 0.2),
            text: Color::from_hex("#CDD6F4"),
            text_muted: Color::from_hex("#6C7086"),
            highlight: Color::from_hex("#F9E2AF"),
            shortcut_background: Color::from_hex("#313244"),
            shortcut_text: Color::from_hex("#A6ADC8"),
            max_width: 600.0,
            max_height: 400.0,
        }
    }

    pub fn light() -> Self {
        Self {
            backdrop: Color::from_rgba(0, 0, 0, 0.3),
            background: Color::from_hex("#FFFFFF"),
            border: Color::from_hex("#E0E0E0"),
            input_background: Color::from_hex("#F5F5F5"),
            item_hover: Color::from_rgba(0, 0, 0, 0.05),
            item_selected: Color::from_rgba(30, 102, 245, 0.1),
            text: Color::from_hex("#1E1E2E"),
            text_muted: Color::from_hex("#9CA0B0"),
            highlight: Color::from_hex("#DF8E1D"),
            shortcut_background: Color::from_hex("#E0E0E0"),
            shortcut_text: Color::from_hex("#5C5F77"),
            max_width: 600.0,
            max_height: 400.0,
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            backdrop: Color::from_rgba(0, 0, 0, 0.7),
            background: Color::from_hex("#000000"),
            border: Color::from_hex("#FFFFFF"),
            input_background: Color::from_hex("#1A1A1A"),
            item_hover: Color::from_rgba(255, 255, 255, 0.1),
            item_selected: Color::from_rgba(0, 255, 0, 0.2),
            text: Color::from_hex("#FFFFFF"),
            text_muted: Color::from_hex("#808080"),
            highlight: Color::from_hex("#FFFF00"),
            shortcut_background: Color::from_hex("#333333"),
            shortcut_text: Color::from_hex("#FFFFFF"),
            max_width: 600.0,
            max_height: 400.0,
        }
    }
}

/// Status bar styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusBarStyle {
    pub background: Color,
    pub text: Color,
    pub text_muted: Color,
    pub border: Color,
    pub item_hover: Color,
    pub height: f32,
}

impl StatusBarStyle {
    pub fn dark() -> Self {
        Self {
            background: Color::from_hex("#181825"),
            text: Color::from_hex("#CDD6F4"),
            text_muted: Color::from_hex("#6C7086"),
            border: Color::from_hex("#313244"),
            item_hover: Color::from_rgba(69, 71, 90, 0.5),
            height: 24.0,
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::from_hex("#E0E0E0"),
            text: Color::from_hex("#1E1E2E"),
            text_muted: Color::from_hex("#5C5F77"),
            border: Color::from_hex("#C0C0C0"),
            item_hover: Color::from_rgba(0, 0, 0, 0.05),
            height: 24.0,
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            background: Color::from_hex("#000000"),
            text: Color::from_hex("#FFFFFF"),
            text_muted: Color::from_hex("#CCCCCC"),
            border: Color::from_hex("#FFFFFF"),
            item_hover: Color::from_rgba(255, 255, 255, 0.1),
            height: 28.0,
        }
    }
}

/// Tooltip styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipStyle {
    pub background: Color,
    pub text: Color,
    pub border: Color,
    pub shadow: Option<Shadow>,
    pub max_width: f32,
}

impl TooltipStyle {
    pub fn dark() -> Self {
        Self {
            background: Color::from_hex("#313244"),
            text: Color::from_hex("#CDD6F4"),
            border: Color::from_hex("#45475A"),
            shadow: Some(Shadow {
                color: Color::from_rgba(0, 0, 0, 0.3),
                offset_x: 0.0,
                offset_y: 2.0,
                blur: 4.0,
            }),
            max_width: 300.0,
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::from_hex("#1E1E2E"),
            text: Color::from_hex("#FFFFFF"),
            border: Color::from_hex("#313244"),
            shadow: Some(Shadow {
                color: Color::from_rgba(0, 0, 0, 0.15),
                offset_x: 0.0,
                offset_y: 2.0,
                blur: 4.0,
            }),
            max_width: 300.0,
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            background: Color::from_hex("#FFFFFF"),
            text: Color::from_hex("#000000"),
            border: Color::from_hex("#FFFFFF"),
            shadow: None,
            max_width: 300.0,
        }
    }
}

/// Focus ring styling (accessibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusStyle {
    pub color: Color,
    pub width: f32,
    pub offset: f32,
    pub style: FocusRingStyle,
}

/// Focus ring style options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FocusRingStyle {
    /// Solid border
    Solid,
    /// Dashed border
    Dashed,
    /// Dotted border
    Dotted,
    /// Glow effect
    Glow,
}

impl FocusStyle {
    pub fn dark() -> Self {
        Self {
            color: Color::from_hex("#89B4FA"),
            width: 2.0,
            offset: 2.0,
            style: FocusRingStyle::Solid,
        }
    }

    pub fn light() -> Self {
        Self {
            color: Color::from_hex("#1E66F5"),
            width: 2.0,
            offset: 2.0,
            style: FocusRingStyle::Solid,
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            color: Color::from_hex("#FFFF00"),
            width: 3.0,
            offset: 2.0,
            style: FocusRingStyle::Solid,
        }
    }
}

/// Scrollbar styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollbarStyle {
    pub track: Color,
    pub thumb: Color,
    pub thumb_hover: Color,
    pub thumb_active: Color,
    pub width: f32,
    pub min_thumb_length: f32,
    pub auto_hide: bool,
}

impl ScrollbarStyle {
    pub fn dark() -> Self {
        Self {
            track: Color::TRANSPARENT,
            thumb: Color::from_rgba(108, 112, 134, 0.4),
            thumb_hover: Color::from_rgba(108, 112, 134, 0.6),
            thumb_active: Color::from_rgba(108, 112, 134, 0.8),
            width: 8.0,
            min_thumb_length: 30.0,
            auto_hide: true,
        }
    }

    pub fn light() -> Self {
        Self {
            track: Color::TRANSPARENT,
            thumb: Color::from_rgba(0, 0, 0, 0.2),
            thumb_hover: Color::from_rgba(0, 0, 0, 0.3),
            thumb_active: Color::from_rgba(0, 0, 0, 0.4),
            width: 8.0,
            min_thumb_length: 30.0,
            auto_hide: true,
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            track: Color::from_hex("#333333"),
            thumb: Color::from_hex("#808080"),
            thumb_hover: Color::from_hex("#AAAAAA"),
            thumb_active: Color::from_hex("#FFFFFF"),
            width: 12.0,
            min_thumb_length: 40.0,
            auto_hide: false,
        }
    }
}

// =============================================================================
// Supporting Types
// =============================================================================

/// Shadow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shadow {
    pub color: Color,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
}

/// Border radius values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderRadius {
    pub none: f32,
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub full: f32,
}

impl Default for BorderRadius {
    fn default() -> Self {
        Self {
            none: 0.0,
            small: 4.0,
            medium: 8.0,
            large: 12.0,
            full: 9999.0,
        }
    }
}

/// Animation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConfig {
    /// Fast animations (hover states)
    pub fast_ms: u32,
    /// Normal animations (transitions)
    pub normal_ms: u32,
    /// Slow animations (page transitions)
    pub slow_ms: u32,
    /// Enable animations globally
    pub enabled: bool,
    /// Reduce motion for accessibility
    pub reduce_motion: bool,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            fast_ms: 100,
            normal_ms: 200,
            slow_ms: 400,
            enabled: true,
            reduce_motion: false,
        }
    }
}

impl AnimationConfig {
    /// Get animation duration, respecting reduce motion setting
    pub fn duration(&self, speed: AnimationSpeed) -> u32 {
        if !self.enabled || self.reduce_motion {
            return 0;
        }
        match speed {
            AnimationSpeed::Fast => self.fast_ms,
            AnimationSpeed::Normal => self.normal_ms,
            AnimationSpeed::Slow => self.slow_ms,
        }
    }
}

/// Animation speed presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationSpeed {
    Fast,
    Normal,
    Slow,
}

// =============================================================================
// Theme Builder
// =============================================================================

/// Builder for creating customized themes
#[derive(Debug, Clone)]
pub struct ThemeBuilder {
    theme: Theme,
}

impl ThemeBuilder {
    /// Create a new builder starting from the dark theme
    pub fn new() -> Self {
        Self {
            theme: Theme::dark(),
        }
    }

    /// Create a builder from an existing theme
    pub fn from_theme(theme: Theme) -> Self {
        Self { theme }
    }

    /// Set the theme name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.theme.name = name.into();
        self
    }

    /// Set the theme mode
    pub fn mode(mut self, mode: ThemeMode) -> Self {
        self.theme.mode = mode;
        // Update colors and styles based on mode
        self.theme.colors = match mode {
            ThemeMode::Dark => ColorPalette::dark(),
            ThemeMode::Light => ColorPalette::light(),
        };
        self.theme.ui = match mode {
            ThemeMode::Dark => UiStyles::dark(),
            ThemeMode::Light => UiStyles::light(),
        };
        self
    }

    /// Set the accent color
    pub fn accent_color(mut self, color: Color) -> Self {
        self.theme.colors.semantic.accent = color.clone();
        self.theme.ui.button.primary_background = color.clone();
        self.theme.ui.focus.color = color.clone();
        self.theme.ui.input.border_focused = color;
        self
    }

    /// Set the font family
    pub fn font_family(mut self, family: impl Into<String>) -> Self {
        self.theme.typography.ui.family = FontFamily::Custom(family.into());
        self
    }

    /// Set the monospace font family
    pub fn monospace_font(mut self, family: impl Into<String>) -> Self {
        self.theme.typography.mono.family = FontFamily::Custom(family.into());
        self
    }

    /// Set the base font size
    pub fn font_size(mut self, size: f32) -> Self {
        self.theme.typography.ui.size = size;
        self
    }

    /// Set the line height
    pub fn line_height(mut self, height: f32) -> Self {
        self.theme.typography.ui.line_height = height;
        self
    }

    /// Set border radius values
    pub fn border_radius(mut self, radius: BorderRadius) -> Self {
        self.theme.ui.border_radius = radius;
        self
    }

    /// Enable or disable animations
    pub fn animations(mut self, enabled: bool) -> Self {
        self.theme.ui.animation.enabled = enabled;
        self
    }

    /// Enable reduced motion mode
    pub fn reduce_motion(mut self, enabled: bool) -> Self {
        self.theme.ui.animation.reduce_motion = enabled;
        self
    }

    /// Set custom spacing scale
    pub fn spacing(mut self, spacing: Spacing) -> Self {
        self.theme.spacing = spacing;
        self
    }

    /// Build the theme
    pub fn build(self) -> Theme {
        self.theme
    }
}

impl Default for ThemeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Theme Manager
// =============================================================================

/// Manages theme state and provides theme switching
#[derive(Debug, Clone)]
pub struct ThemeManager {
    current: Arc<Theme>,
    available: Vec<Theme>,
}

impl ThemeManager {
    /// Create a new theme manager with built-in themes
    pub fn new() -> Self {
        Self {
            current: Arc::new(Theme::dark()),
            available: vec![Theme::dark(), Theme::light(), Theme::high_contrast_dark()],
        }
    }

    /// Get the current theme
    pub fn current(&self) -> Arc<Theme> {
        Arc::clone(&self.current)
    }

    /// Set the current theme
    pub fn set_theme(&mut self, theme: Theme) {
        self.current = Arc::new(theme);
    }

    /// Set theme by name
    pub fn set_theme_by_name(&mut self, name: &str) -> bool {
        if let Some(theme) = self.available.iter().find(|t| t.name == name) {
            self.current = Arc::new(theme.clone());
            true
        } else {
            false
        }
    }

    /// Toggle between dark and light mode
    pub fn toggle_mode(&mut self) {
        let new_mode = self.current.mode.toggle();
        let theme = match new_mode {
            ThemeMode::Dark => Theme::dark(),
            ThemeMode::Light => Theme::light(),
        };
        self.current = Arc::new(theme);
    }

    /// Get available theme names
    pub fn available_themes(&self) -> Vec<&str> {
        self.available.iter().map(|t| t.name.as_str()).collect()
    }

    /// Add a custom theme
    pub fn add_theme(&mut self, theme: Theme) {
        self.available.push(theme);
    }
}

impl Default for ThemeManager {
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
    fn test_theme_mode_toggle() {
        let dark = ThemeMode::Dark;
        assert_eq!(dark.toggle(), ThemeMode::Light);

        let light = ThemeMode::Light;
        assert_eq!(light.toggle(), ThemeMode::Dark);
    }

    #[test]
    fn test_theme_mode_checks() {
        assert!(ThemeMode::Dark.is_dark());
        assert!(!ThemeMode::Dark.is_light());
        assert!(ThemeMode::Light.is_light());
        assert!(!ThemeMode::Light.is_dark());
    }

    #[test]
    fn test_theme_defaults() {
        let dark = Theme::dark();
        assert_eq!(dark.mode, ThemeMode::Dark);
        assert_eq!(dark.name, "Zed Dark");

        let light = Theme::light();
        assert_eq!(light.mode, ThemeMode::Light);
        assert_eq!(light.name, "Zed Light");
    }

    #[test]
    fn test_theme_toggle() {
        let dark = Theme::dark();
        let light = dark.toggle_mode();
        assert_eq!(light.mode, ThemeMode::Light);
    }

    #[test]
    fn test_theme_builder() {
        let theme = ThemeBuilder::new()
            .name("Custom Theme")
            .mode(ThemeMode::Light)
            .font_size(16.0)
            .animations(false)
            .build();

        assert_eq!(theme.name, "Custom Theme");
        assert_eq!(theme.mode, ThemeMode::Light);
        assert_eq!(theme.typography.ui.size, 16.0);
        assert!(!theme.ui.animation.enabled);
    }

    #[test]
    fn test_theme_builder_accent_color() {
        let accent = Color::from_hex("#FF0000");
        let theme = ThemeBuilder::new().accent_color(accent.clone()).build();

        assert_eq!(theme.colors.semantic.accent, accent);
        assert_eq!(theme.ui.button.primary_background, accent);
    }

    #[test]
    fn test_theme_manager() {
        let mut manager = ThemeManager::new();

        assert_eq!(manager.current().mode, ThemeMode::Dark);

        manager.toggle_mode();
        assert_eq!(manager.current().mode, ThemeMode::Light);

        assert!(manager.set_theme_by_name("Zed Dark"));
        assert_eq!(manager.current().mode, ThemeMode::Dark);

        assert!(!manager.set_theme_by_name("Nonexistent Theme"));
    }

    #[test]
    fn test_theme_manager_custom_theme() {
        let mut manager = ThemeManager::new();

        let custom = ThemeBuilder::new().name("My Custom Theme").build();

        manager.add_theme(custom);
        assert!(manager.set_theme_by_name("My Custom Theme"));
        assert_eq!(manager.current().name, "My Custom Theme");
    }

    #[test]
    fn test_animation_config() {
        let config = AnimationConfig::default();
        assert_eq!(config.duration(AnimationSpeed::Fast), 100);
        assert_eq!(config.duration(AnimationSpeed::Normal), 200);
        assert_eq!(config.duration(AnimationSpeed::Slow), 400);

        let mut reduced = AnimationConfig::default();
        reduced.reduce_motion = true;
        assert_eq!(reduced.duration(AnimationSpeed::Fast), 0);

        let mut disabled = AnimationConfig::default();
        disabled.enabled = false;
        assert_eq!(disabled.duration(AnimationSpeed::Normal), 0);
    }

    #[test]
    fn test_border_radius_default() {
        let radius = BorderRadius::default();
        assert_eq!(radius.none, 0.0);
        assert_eq!(radius.small, 4.0);
        assert_eq!(radius.medium, 8.0);
        assert_eq!(radius.large, 12.0);
        assert_eq!(radius.full, 9999.0);
    }

    #[test]
    fn test_ui_styles_variants() {
        let dark = UiStyles::dark();
        let light = UiStyles::light();
        let high_contrast = UiStyles::high_contrast_dark();

        // Just verify they can be created without panic
        assert!(dark.panel.background != light.panel.background);
        assert!(high_contrast.focus.width > dark.focus.width);
    }

    #[test]
    fn test_theme_serialization() {
        let theme = Theme::dark();
        let json = serde_json::to_string(&theme).unwrap();
        let deserialized: Theme = serde_json::from_str(&json).unwrap();

        assert_eq!(theme.name, deserialized.name);
        assert_eq!(theme.mode, deserialized.mode);
    }

    #[test]
    fn test_theme_into_arc() {
        let theme = Theme::dark();
        let arc = theme.into_arc();
        assert_eq!(arc.name, "Zed Dark");
    }

    #[test]
    fn test_high_contrast_theme() {
        let theme = Theme::high_contrast_dark();
        assert_eq!(theme.name, "High Contrast Dark");
        assert_eq!(theme.mode, ThemeMode::Dark);
        // High contrast should have thicker focus rings
        assert!(theme.ui.focus.width >= 3.0);
    }

    #[test]
    fn test_focus_ring_style() {
        let style = FocusRingStyle::Solid;
        assert_eq!(style, FocusRingStyle::Solid);

        let glow = FocusRingStyle::Glow;
        assert_ne!(glow, style);
    }
}

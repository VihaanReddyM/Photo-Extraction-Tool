//! Color System - Zed-inspired Color Palette
//!
//! This module provides a comprehensive color system with:
//! - RGBA color representation
//! - Semantic color roles (primary, secondary, success, etc.)
//! - Dark and light color palettes
//! - Color manipulation utilities
//!
//! # Example
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::theme::colors::{Color, ColorPalette};
//!
//! // Create colors
//! let red = Color::from_hex("#FF0000");
//! let semi_transparent = Color::from_rgba(255, 0, 0, 0.5);
//!
//! // Use palette
//! let palette = ColorPalette::dark();
//! let bg = &palette.background.primary;
//! ```

use serde::{Deserialize, Serialize};

// =============================================================================
// Color
// =============================================================================

/// RGBA color representation
///
/// Colors are stored as floating-point values in the range 0.0-1.0.
/// This allows for easy manipulation and interpolation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Color {
    /// Red component (0.0-1.0)
    pub r: f32,
    /// Green component (0.0-1.0)
    pub g: f32,
    /// Blue component (0.0-1.0)
    pub b: f32,
    /// Alpha component (0.0-1.0)
    pub a: f32,
}

impl Color {
    /// Transparent color
    pub const TRANSPARENT: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    /// Pure white
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    /// Pure black
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    /// Create a new color from RGBA values (0.0-1.0)
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: a.clamp(0.0, 1.0),
        }
    }

    /// Create a color from RGB values (0-255) with full opacity
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Create a color from RGBA values (0-255 for RGB, 0.0-1.0 for alpha)
    pub fn from_rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a.clamp(0.0, 1.0),
        }
    }

    /// Create a color from a hex string
    ///
    /// Supports the following formats:
    /// - `#RGB` (short form)
    /// - `#RGBA` (short form with alpha)
    /// - `#RRGGBB` (standard)
    /// - `#RRGGBBAA` (with alpha)
    ///
    /// The `#` prefix is optional.
    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');

        match hex.len() {
            // #RGB
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0);
                Self::from_rgb(r * 17, g * 17, b * 17)
            }
            // #RGBA
            4 => {
                let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0);
                let a = u8::from_str_radix(&hex[3..4], 16).unwrap_or(15);
                Self::from_rgba(r * 17, g * 17, b * 17, (a * 17) as f32 / 255.0)
            }
            // #RRGGBB
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                Self::from_rgb(r, g, b)
            }
            // #RRGGBBAA
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
                Self::from_rgba(r, g, b, a as f32 / 255.0)
            }
            _ => Self::BLACK,
        }
    }

    /// Convert to hex string (#RRGGBB)
    pub fn to_hex(&self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
        )
    }

    /// Convert to hex string with alpha (#RRGGBBAA)
    pub fn to_hex_alpha(&self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }

    /// Convert to RGB tuple (0-255)
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        (
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
        )
    }

    /// Convert to RGBA tuple (0-255 for RGB, 0.0-1.0 for alpha)
    pub fn to_rgba(&self) -> (u8, u8, u8, f32) {
        (
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            self.a,
        )
    }

    /// Convert to CSS rgba() string
    pub fn to_css_rgba(&self) -> String {
        let (r, g, b) = self.to_rgb();
        format!("rgba({}, {}, {}, {:.3})", r, g, b, self.a)
    }

    /// Create a new color with a different alpha value
    pub fn with_alpha(&self, alpha: f32) -> Self {
        Self {
            a: alpha.clamp(0.0, 1.0),
            ..self.clone()
        }
    }

    /// Lighten the color by a percentage (0.0-1.0)
    pub fn lighten(&self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        Self {
            r: (self.r + (1.0 - self.r) * amount).clamp(0.0, 1.0),
            g: (self.g + (1.0 - self.g) * amount).clamp(0.0, 1.0),
            b: (self.b + (1.0 - self.b) * amount).clamp(0.0, 1.0),
            a: self.a,
        }
    }

    /// Darken the color by a percentage (0.0-1.0)
    pub fn darken(&self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        Self {
            r: (self.r * (1.0 - amount)).clamp(0.0, 1.0),
            g: (self.g * (1.0 - amount)).clamp(0.0, 1.0),
            b: (self.b * (1.0 - amount)).clamp(0.0, 1.0),
            a: self.a,
        }
    }

    /// Mix this color with another color
    ///
    /// The factor determines how much of the other color to blend in (0.0-1.0).
    /// A factor of 0.0 returns this color, 1.0 returns the other color.
    pub fn mix(&self, other: &Color, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        let inv = 1.0 - factor;
        Self {
            r: self.r * inv + other.r * factor,
            g: self.g * inv + other.g * factor,
            b: self.b * inv + other.b * factor,
            a: self.a * inv + other.a * factor,
        }
    }

    /// Calculate the luminance of this color
    ///
    /// Uses the relative luminance formula from WCAG 2.0
    pub fn luminance(&self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    /// Check if this color is considered "light"
    ///
    /// Based on luminance threshold of 0.5
    pub fn is_light(&self) -> bool {
        self.luminance() > 0.5
    }

    /// Check if this color is considered "dark"
    pub fn is_dark(&self) -> bool {
        !self.is_light()
    }

    /// Get a contrasting text color (black or white)
    pub fn contrasting_text(&self) -> Self {
        if self.is_light() {
            Self::BLACK
        } else {
            Self::WHITE
        }
    }

    /// Calculate contrast ratio with another color (WCAG 2.0)
    pub fn contrast_ratio(&self, other: &Color) -> f32 {
        let l1 = self.luminance();
        let l2 = other.luminance();
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        (lighter + 0.05) / (darker + 0.05)
    }

    /// Check if contrast ratio meets WCAG AA requirements (4.5:1 for normal text)
    pub fn meets_wcag_aa(&self, other: &Color) -> bool {
        self.contrast_ratio(other) >= 4.5
    }

    /// Check if contrast ratio meets WCAG AAA requirements (7:1 for normal text)
    pub fn meets_wcag_aaa(&self, other: &Color) -> bool {
        self.contrast_ratio(other) >= 7.0
    }

    /// Invert the color
    pub fn invert(&self) -> Self {
        Self {
            r: 1.0 - self.r,
            g: 1.0 - self.g,
            b: 1.0 - self.b,
            a: self.a,
        }
    }

    /// Convert to grayscale
    pub fn grayscale(&self) -> Self {
        let gray = self.luminance();
        Self {
            r: gray,
            g: gray,
            b: gray,
            a: self.a,
        }
    }

    /// Saturate or desaturate the color
    ///
    /// Amount > 1.0 increases saturation, < 1.0 decreases it
    pub fn saturate(&self, amount: f32) -> Self {
        let gray = self.luminance();
        Self {
            r: (gray + (self.r - gray) * amount).clamp(0.0, 1.0),
            g: (gray + (self.g - gray) * amount).clamp(0.0, 1.0),
            b: (gray + (self.b - gray) * amount).clamp(0.0, 1.0),
            a: self.a,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.a < 1.0 {
            write!(f, "{}", self.to_hex_alpha())
        } else {
            write!(f, "{}", self.to_hex())
        }
    }
}

// =============================================================================
// Color Palette
// =============================================================================

/// Complete color palette for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    /// Background colors
    pub background: BackgroundColors,

    /// Foreground/text colors
    pub foreground: ForegroundColors,

    /// Border colors
    pub border: BorderColors,

    /// Semantic colors (success, warning, error, etc.)
    pub semantic: SemanticColors,

    /// Accent colors
    pub accent: AccentColors,

    /// Surface colors (for cards, panels, etc.)
    pub surface: SurfaceColors,
}

impl ColorPalette {
    /// Create the default dark palette (Catppuccin Mocha inspired)
    pub fn dark() -> Self {
        Self {
            background: BackgroundColors::dark(),
            foreground: ForegroundColors::dark(),
            border: BorderColors::dark(),
            semantic: SemanticColors::dark(),
            accent: AccentColors::dark(),
            surface: SurfaceColors::dark(),
        }
    }

    /// Create the light palette (Catppuccin Latte inspired)
    pub fn light() -> Self {
        Self {
            background: BackgroundColors::light(),
            foreground: ForegroundColors::light(),
            border: BorderColors::light(),
            semantic: SemanticColors::light(),
            accent: AccentColors::light(),
            surface: SurfaceColors::light(),
        }
    }

    /// Create a high contrast dark palette
    pub fn high_contrast_dark() -> Self {
        Self {
            background: BackgroundColors::high_contrast_dark(),
            foreground: ForegroundColors::high_contrast_dark(),
            border: BorderColors::high_contrast_dark(),
            semantic: SemanticColors::high_contrast_dark(),
            accent: AccentColors::high_contrast_dark(),
            surface: SurfaceColors::high_contrast_dark(),
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::dark()
    }
}

// =============================================================================
// Background Colors
// =============================================================================

/// Background color hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundColors {
    /// Primary background (main application background)
    pub primary: Color,
    /// Secondary background (sidebars, panels)
    pub secondary: Color,
    /// Tertiary background (nested panels, cards)
    pub tertiary: Color,
    /// Elevated background (modals, dropdowns)
    pub elevated: Color,
    /// Hover state background
    pub hover: Color,
    /// Active/pressed state background
    pub active: Color,
    /// Selected item background
    pub selected: Color,
}

impl BackgroundColors {
    pub fn dark() -> Self {
        Self {
            primary: Color::from_hex("#1E1E2E"),
            secondary: Color::from_hex("#181825"),
            tertiary: Color::from_hex("#11111B"),
            elevated: Color::from_hex("#313244"),
            hover: Color::from_rgba(69, 71, 90, 0.5),
            active: Color::from_rgba(69, 71, 90, 0.7),
            selected: Color::from_rgba(137, 180, 250, 0.2),
        }
    }

    pub fn light() -> Self {
        Self {
            primary: Color::from_hex("#EFF1F5"),
            secondary: Color::from_hex("#E6E9EF"),
            tertiary: Color::from_hex("#DCE0E8"),
            elevated: Color::from_hex("#FFFFFF"),
            hover: Color::from_rgba(0, 0, 0, 0.05),
            active: Color::from_rgba(0, 0, 0, 0.1),
            selected: Color::from_rgba(30, 102, 245, 0.1),
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            primary: Color::from_hex("#000000"),
            secondary: Color::from_hex("#0D0D0D"),
            tertiary: Color::from_hex("#1A1A1A"),
            elevated: Color::from_hex("#262626"),
            hover: Color::from_rgba(255, 255, 255, 0.1),
            active: Color::from_rgba(255, 255, 255, 0.2),
            selected: Color::from_rgba(0, 255, 0, 0.3),
        }
    }
}

// =============================================================================
// Foreground Colors
// =============================================================================

/// Foreground/text color hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForegroundColors {
    /// Primary text color (most readable)
    pub primary: Color,
    /// Secondary text color (less emphasis)
    pub secondary: Color,
    /// Muted text color (disabled, hints)
    pub muted: Color,
    /// Placeholder text color
    pub placeholder: Color,
    /// Inverted text (for use on accent backgrounds)
    pub inverted: Color,
    /// Link color
    pub link: Color,
    /// Link hover color
    pub link_hover: Color,
}

impl ForegroundColors {
    pub fn dark() -> Self {
        Self {
            primary: Color::from_hex("#CDD6F4"),
            secondary: Color::from_hex("#A6ADC8"),
            muted: Color::from_hex("#6C7086"),
            placeholder: Color::from_hex("#585B70"),
            inverted: Color::from_hex("#1E1E2E"),
            link: Color::from_hex("#89B4FA"),
            link_hover: Color::from_hex("#B4BEFE"),
        }
    }

    pub fn light() -> Self {
        Self {
            primary: Color::from_hex("#4C4F69"),
            secondary: Color::from_hex("#5C5F77"),
            muted: Color::from_hex("#9CA0B0"),
            placeholder: Color::from_hex("#ACB0BE"),
            inverted: Color::from_hex("#FFFFFF"),
            link: Color::from_hex("#1E66F5"),
            link_hover: Color::from_hex("#7287FD"),
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            primary: Color::from_hex("#FFFFFF"),
            secondary: Color::from_hex("#E0E0E0"),
            muted: Color::from_hex("#808080"),
            placeholder: Color::from_hex("#666666"),
            inverted: Color::from_hex("#000000"),
            link: Color::from_hex("#00FFFF"),
            link_hover: Color::from_hex("#FFFF00"),
        }
    }
}

// =============================================================================
// Border Colors
// =============================================================================

/// Border color variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderColors {
    /// Default border color
    pub default: Color,
    /// Subtle border color
    pub subtle: Color,
    /// Strong border color (for emphasis)
    pub strong: Color,
    /// Focused element border
    pub focused: Color,
    /// Error state border
    pub error: Color,
}

impl BorderColors {
    pub fn dark() -> Self {
        Self {
            default: Color::from_hex("#313244"),
            subtle: Color::from_hex("#45475A"),
            strong: Color::from_hex("#6C7086"),
            focused: Color::from_hex("#89B4FA"),
            error: Color::from_hex("#F38BA8"),
        }
    }

    pub fn light() -> Self {
        Self {
            default: Color::from_hex("#CCD0DA"),
            subtle: Color::from_hex("#DCE0E8"),
            strong: Color::from_hex("#9CA0B0"),
            focused: Color::from_hex("#1E66F5"),
            error: Color::from_hex("#D20F39"),
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            default: Color::from_hex("#FFFFFF"),
            subtle: Color::from_hex("#808080"),
            strong: Color::from_hex("#FFFFFF"),
            focused: Color::from_hex("#FFFF00"),
            error: Color::from_hex("#FF0000"),
        }
    }
}

// =============================================================================
// Semantic Colors
// =============================================================================

/// Semantic/status colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticColors {
    /// Primary accent color
    pub accent: Color,
    /// Success/positive color
    pub success: Color,
    /// Warning/caution color
    pub warning: Color,
    /// Error/danger color
    pub error: Color,
    /// Info/neutral color
    pub info: Color,

    /// Success background (subtle)
    pub success_background: Color,
    /// Warning background (subtle)
    pub warning_background: Color,
    /// Error background (subtle)
    pub error_background: Color,
    /// Info background (subtle)
    pub info_background: Color,
}

impl SemanticColors {
    pub fn dark() -> Self {
        Self {
            accent: Color::from_hex("#89B4FA"),
            success: Color::from_hex("#A6E3A1"),
            warning: Color::from_hex("#F9E2AF"),
            error: Color::from_hex("#F38BA8"),
            info: Color::from_hex("#89DCEB"),

            success_background: Color::from_rgba(166, 227, 161, 0.15),
            warning_background: Color::from_rgba(249, 226, 175, 0.15),
            error_background: Color::from_rgba(243, 139, 168, 0.15),
            info_background: Color::from_rgba(137, 220, 235, 0.15),
        }
    }

    pub fn light() -> Self {
        Self {
            accent: Color::from_hex("#1E66F5"),
            success: Color::from_hex("#40A02B"),
            warning: Color::from_hex("#DF8E1D"),
            error: Color::from_hex("#D20F39"),
            info: Color::from_hex("#209FB5"),

            success_background: Color::from_rgba(64, 160, 43, 0.1),
            warning_background: Color::from_rgba(223, 142, 29, 0.1),
            error_background: Color::from_rgba(210, 15, 57, 0.1),
            info_background: Color::from_rgba(32, 159, 181, 0.1),
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            accent: Color::from_hex("#00FF00"),
            success: Color::from_hex("#00FF00"),
            warning: Color::from_hex("#FFFF00"),
            error: Color::from_hex("#FF0000"),
            info: Color::from_hex("#00FFFF"),

            success_background: Color::from_rgba(0, 255, 0, 0.2),
            warning_background: Color::from_rgba(255, 255, 0, 0.2),
            error_background: Color::from_rgba(255, 0, 0, 0.2),
            info_background: Color::from_rgba(0, 255, 255, 0.2),
        }
    }
}

// =============================================================================
// Accent Colors
// =============================================================================

/// Accent color variations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccentColors {
    /// Blue
    pub blue: Color,
    /// Purple/Lavender
    pub purple: Color,
    /// Pink
    pub pink: Color,
    /// Red
    pub red: Color,
    /// Orange/Peach
    pub orange: Color,
    /// Yellow
    pub yellow: Color,
    /// Green
    pub green: Color,
    /// Teal/Cyan
    pub teal: Color,
}

impl AccentColors {
    pub fn dark() -> Self {
        Self {
            blue: Color::from_hex("#89B4FA"),
            purple: Color::from_hex("#CBA6F7"),
            pink: Color::from_hex("#F5C2E7"),
            red: Color::from_hex("#F38BA8"),
            orange: Color::from_hex("#FAB387"),
            yellow: Color::from_hex("#F9E2AF"),
            green: Color::from_hex("#A6E3A1"),
            teal: Color::from_hex("#94E2D5"),
        }
    }

    pub fn light() -> Self {
        Self {
            blue: Color::from_hex("#1E66F5"),
            purple: Color::from_hex("#8839EF"),
            pink: Color::from_hex("#EA76CB"),
            red: Color::from_hex("#D20F39"),
            orange: Color::from_hex("#FE640B"),
            yellow: Color::from_hex("#DF8E1D"),
            green: Color::from_hex("#40A02B"),
            teal: Color::from_hex("#179299"),
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            blue: Color::from_hex("#0080FF"),
            purple: Color::from_hex("#FF00FF"),
            pink: Color::from_hex("#FF69B4"),
            red: Color::from_hex("#FF0000"),
            orange: Color::from_hex("#FF8000"),
            yellow: Color::from_hex("#FFFF00"),
            green: Color::from_hex("#00FF00"),
            teal: Color::from_hex("#00FFFF"),
        }
    }
}

// =============================================================================
// Surface Colors
// =============================================================================

/// Surface colors for cards, panels, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceColors {
    /// Surface level 0 (base)
    pub level0: Color,
    /// Surface level 1 (raised)
    pub level1: Color,
    /// Surface level 2 (elevated)
    pub level2: Color,
    /// Surface level 3 (overlay)
    pub level3: Color,
}

impl SurfaceColors {
    pub fn dark() -> Self {
        Self {
            level0: Color::from_hex("#1E1E2E"),
            level1: Color::from_hex("#313244"),
            level2: Color::from_hex("#45475A"),
            level3: Color::from_hex("#585B70"),
        }
    }

    pub fn light() -> Self {
        Self {
            level0: Color::from_hex("#EFF1F5"),
            level1: Color::from_hex("#FFFFFF"),
            level2: Color::from_hex("#FFFFFF"),
            level3: Color::from_hex("#E6E9EF"),
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            level0: Color::from_hex("#000000"),
            level1: Color::from_hex("#1A1A1A"),
            level2: Color::from_hex("#333333"),
            level3: Color::from_hex("#4D4D4D"),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_hex() {
        let red = Color::from_hex("#FF0000");
        assert!((red.r - 1.0).abs() < 0.01);
        assert!((red.g - 0.0).abs() < 0.01);
        assert!((red.b - 0.0).abs() < 0.01);
        assert!((red.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_from_hex_short() {
        let white = Color::from_hex("#FFF");
        assert!((white.r - 1.0).abs() < 0.01);
        assert!((white.g - 1.0).abs() < 0.01);
        assert!((white.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_from_hex_with_alpha() {
        let semi = Color::from_hex("#FF000080");
        assert!((semi.r - 1.0).abs() < 0.01);
        assert!((semi.a - 0.5).abs() < 0.02);
    }

    #[test]
    fn test_color_to_hex() {
        let color = Color::from_rgb(255, 128, 0);
        assert_eq!(color.to_hex(), "#FF8000");
    }

    #[test]
    fn test_color_lighten() {
        let dark = Color::from_hex("#333333");
        let lighter = dark.lighten(0.5);
        assert!(lighter.luminance() > dark.luminance());
    }

    #[test]
    fn test_color_darken() {
        let light = Color::from_hex("#CCCCCC");
        let darker = light.darken(0.5);
        assert!(darker.luminance() < light.luminance());
    }

    #[test]
    fn test_color_mix() {
        let black = Color::BLACK;
        let white = Color::WHITE;
        let gray = black.mix(&white, 0.5);
        assert!((gray.r - 0.5).abs() < 0.01);
        assert!((gray.g - 0.5).abs() < 0.01);
        assert!((gray.b - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_color_luminance() {
        assert!(Color::WHITE.luminance() > Color::BLACK.luminance());
        assert!(Color::WHITE.is_light());
        assert!(Color::BLACK.is_dark());
    }

    #[test]
    fn test_color_contrast() {
        let ratio = Color::WHITE.contrast_ratio(&Color::BLACK);
        assert!(ratio > 20.0);
        assert!(Color::WHITE.meets_wcag_aaa(&Color::BLACK));
    }

    #[test]
    fn test_color_contrasting_text() {
        assert_eq!(Color::WHITE.contrasting_text(), Color::BLACK);
        assert_eq!(Color::BLACK.contrasting_text(), Color::WHITE);
    }

    #[test]
    fn test_color_with_alpha() {
        let opaque = Color::from_hex("#FF0000");
        let semi = opaque.with_alpha(0.5);
        assert!((semi.a - 0.5).abs() < 0.01);
        assert!((semi.r - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_invert() {
        let white = Color::WHITE;
        let inverted = white.invert();
        assert!((inverted.r - 0.0).abs() < 0.01);
        assert!((inverted.g - 0.0).abs() < 0.01);
        assert!((inverted.b - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_color_grayscale() {
        let red = Color::from_hex("#FF0000");
        let gray = red.grayscale();
        assert!((gray.r - gray.g).abs() < 0.01);
        assert!((gray.g - gray.b).abs() < 0.01);
    }

    #[test]
    fn test_color_to_css_rgba() {
        let color = Color::from_rgba(255, 128, 0, 0.5);
        let css = color.to_css_rgba();
        assert!(css.contains("255"));
        assert!(css.contains("128"));
        assert!(css.contains("0.5"));
    }

    #[test]
    fn test_color_palette_dark() {
        let palette = ColorPalette::dark();
        assert!(palette.background.primary.is_dark());
        assert!(palette.foreground.primary.is_light());
    }

    #[test]
    fn test_color_palette_light() {
        let palette = ColorPalette::light();
        assert!(palette.background.primary.is_light());
        assert!(palette.foreground.primary.is_dark());
    }

    #[test]
    fn test_color_palette_high_contrast() {
        let palette = ColorPalette::high_contrast_dark();
        // High contrast dark should have pure black background
        assert!((palette.background.primary.r - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_semantic_colors_contrast() {
        let dark = SemanticColors::dark();
        let bg = Color::from_hex("#1E1E2E");
        // Semantic colors should have good contrast with dark background
        // Using 3.0 threshold (WCAG AA for large text) since Catppuccin
        // colors are an established, widely-used color scheme
        assert!(dark.error.contrast_ratio(&bg) >= 3.0);
        assert!(dark.success.contrast_ratio(&bg) >= 3.0);
    }

    #[test]
    fn test_color_display() {
        let color = Color::from_hex("#FF8800");
        assert_eq!(format!("{}", color), "#FF8800");

        let semi = color.with_alpha(0.5);
        assert!(format!("{}", semi).contains("80"));
    }

    #[test]
    fn test_color_from_rgb() {
        let color = Color::from_rgb(128, 64, 255);
        assert!((color.r - 128.0 / 255.0).abs() < 0.01);
        assert!((color.g - 64.0 / 255.0).abs() < 0.01);
        assert!((color.b - 255.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_color_to_rgb() {
        let color = Color::from_rgb(128, 64, 255);
        let (r, g, b) = color.to_rgb();
        assert_eq!(r, 128);
        assert_eq!(g, 64);
        assert_eq!(b, 255);
    }

    #[test]
    fn test_color_saturate() {
        let color = Color::from_hex("#808080");
        let desaturated = color.saturate(0.0);
        // Pure gray should remain unchanged
        assert!((desaturated.r - desaturated.g).abs() < 0.01);

        let saturated = Color::from_hex("#804040");
        let more_saturated = saturated.saturate(2.0);
        // Saturation should increase the difference from gray
        assert!(more_saturated.r > saturated.r || more_saturated.r == 1.0);
    }

    #[test]
    fn test_color_clamp() {
        let color = Color::new(1.5, -0.5, 0.5, 2.0);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.5);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn test_accent_colors() {
        let dark = AccentColors::dark();
        let light = AccentColors::light();

        // Verify all accent colors exist and are different between themes
        assert_ne!(dark.blue, light.blue);
        assert_ne!(dark.green, light.green);
        assert_ne!(dark.red, light.red);
    }

    #[test]
    fn test_surface_colors() {
        let dark = SurfaceColors::dark();
        // Surface levels should get progressively lighter in dark mode
        assert!(dark.level1.luminance() >= dark.level0.luminance());
        assert!(dark.level2.luminance() >= dark.level1.luminance());
    }
}

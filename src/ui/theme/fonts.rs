//! Typography System - Zed-inspired Font Configuration
//!
//! This module provides a comprehensive typography system with:
//! - Font family definitions (system, custom, monospace)
//! - Font size scales
//! - Font weight definitions
//! - Line height and letter spacing
//! - UI and code typography presets
//!
//! # Example
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::theme::fonts::{Typography, FontConfig, FontFamily, FontWeight};
//!
//! // Use default typography
//! let typography = Typography::default();
//!
//! // Access UI font settings
//! let ui_size = typography.ui.size;
//! let ui_family = &typography.ui.family;
//!
//! // Create custom font config
//! let custom = FontConfig::new()
//!     .family(FontFamily::Custom("Inter".to_string()))
//!     .size(14.0)
//!     .weight(FontWeight::Medium);
//! ```

use serde::{Deserialize, Serialize};

// =============================================================================
// Font Family
// =============================================================================

/// Font family specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "name")]
pub enum FontFamily {
    /// System default sans-serif font
    System,
    /// System default serif font
    Serif,
    /// System default monospace font
    Monospace,
    /// Custom font by name
    Custom(String),
}

impl FontFamily {
    /// Get the CSS font-family string
    pub fn to_css(&self) -> String {
        match self {
            FontFamily::System => {
                // Modern system font stack
                r#"-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, "Fira Sans", "Droid Sans", "Helvetica Neue", sans-serif"#.to_string()
            }
            FontFamily::Serif => r#"Georgia, "Times New Roman", Times, serif"#.to_string(),
            FontFamily::Monospace => {
                // Monospace font stack optimized for code
                r#""JetBrains Mono", "Fira Code", "SF Mono", Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace"#.to_string()
            }
            FontFamily::Custom(name) => {
                format!(r#""{}", sans-serif"#, name)
            }
        }
    }

    /// Get the primary font name
    pub fn name(&self) -> &str {
        match self {
            FontFamily::System => "System",
            FontFamily::Serif => "Serif",
            FontFamily::Monospace => "Monospace",
            FontFamily::Custom(name) => name,
        }
    }

    /// Check if this is a monospace font
    pub fn is_monospace(&self) -> bool {
        matches!(self, FontFamily::Monospace)
    }

    /// Common UI font families
    pub fn inter() -> Self {
        FontFamily::Custom("Inter".to_string())
    }

    pub fn roboto() -> Self {
        FontFamily::Custom("Roboto".to_string())
    }

    pub fn sf_pro() -> Self {
        FontFamily::Custom("SF Pro Display".to_string())
    }

    /// Common monospace font families
    pub fn jetbrains_mono() -> Self {
        FontFamily::Custom("JetBrains Mono".to_string())
    }

    pub fn fira_code() -> Self {
        FontFamily::Custom("Fira Code".to_string())
    }

    pub fn source_code_pro() -> Self {
        FontFamily::Custom("Source Code Pro".to_string())
    }

    pub fn cascadia_code() -> Self {
        FontFamily::Custom("Cascadia Code".to_string())
    }
}

impl Default for FontFamily {
    fn default() -> Self {
        FontFamily::System
    }
}

impl std::fmt::Display for FontFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// =============================================================================
// Font Weight
// =============================================================================

/// Font weight values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    /// Thin (100)
    Thin,
    /// Extra Light (200)
    ExtraLight,
    /// Light (300)
    Light,
    /// Normal/Regular (400)
    Normal,
    /// Medium (500)
    Medium,
    /// Semi Bold (600)
    SemiBold,
    /// Bold (700)
    Bold,
    /// Extra Bold (800)
    ExtraBold,
    /// Black (900)
    Black,
}

impl FontWeight {
    /// Get the numeric weight value
    pub fn value(&self) -> u16 {
        match self {
            FontWeight::Thin => 100,
            FontWeight::ExtraLight => 200,
            FontWeight::Light => 300,
            FontWeight::Normal => 400,
            FontWeight::Medium => 500,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800,
            FontWeight::Black => 900,
        }
    }

    /// Create from numeric value (rounds to nearest standard weight)
    pub fn from_value(value: u16) -> Self {
        match value {
            0..=150 => FontWeight::Thin,
            151..=250 => FontWeight::ExtraLight,
            251..=350 => FontWeight::Light,
            351..=450 => FontWeight::Normal,
            451..=550 => FontWeight::Medium,
            551..=650 => FontWeight::SemiBold,
            651..=750 => FontWeight::Bold,
            751..=850 => FontWeight::ExtraBold,
            _ => FontWeight::Black,
        }
    }

    /// Check if this is a bold weight (>= 600)
    pub fn is_bold(&self) -> bool {
        self.value() >= 600
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        FontWeight::Normal
    }
}

impl std::fmt::Display for FontWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            FontWeight::Thin => "Thin",
            FontWeight::ExtraLight => "Extra Light",
            FontWeight::Light => "Light",
            FontWeight::Normal => "Normal",
            FontWeight::Medium => "Medium",
            FontWeight::SemiBold => "Semi Bold",
            FontWeight::Bold => "Bold",
            FontWeight::ExtraBold => "Extra Bold",
            FontWeight::Black => "Black",
        };
        write!(f, "{}", name)
    }
}

// =============================================================================
// Font Style
// =============================================================================

/// Font style (italic, oblique, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    /// Normal upright style
    #[default]
    Normal,
    /// Italic style
    Italic,
    /// Oblique style
    Oblique,
}

impl FontStyle {
    /// Get CSS value
    pub fn to_css(&self) -> &'static str {
        match self {
            FontStyle::Normal => "normal",
            FontStyle::Italic => "italic",
            FontStyle::Oblique => "oblique",
        }
    }
}

// =============================================================================
// Font Config
// =============================================================================

/// Complete font configuration for a text element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    /// Font family
    pub family: FontFamily,
    /// Font size in pixels
    pub size: f32,
    /// Font weight
    pub weight: FontWeight,
    /// Font style
    pub style: FontStyle,
    /// Line height multiplier (relative to font size)
    pub line_height: f32,
    /// Letter spacing in pixels (can be negative)
    pub letter_spacing: f32,
    /// Enable font ligatures
    pub ligatures: bool,
}

impl FontConfig {
    /// Create a new font config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the font family
    pub fn family(mut self, family: FontFamily) -> Self {
        self.family = family;
        self
    }

    /// Set the font size in pixels
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Set the font weight
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set the font style
    pub fn style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the line height multiplier
    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    /// Set the letter spacing
    pub fn letter_spacing(mut self, spacing: f32) -> Self {
        self.letter_spacing = spacing;
        self
    }

    /// Enable or disable ligatures
    pub fn ligatures(mut self, enabled: bool) -> Self {
        self.ligatures = enabled;
        self
    }

    /// Calculate the actual line height in pixels
    pub fn computed_line_height(&self) -> f32 {
        self.size * self.line_height
    }

    /// Create a bold version of this config
    pub fn bold(&self) -> Self {
        Self {
            weight: FontWeight::Bold,
            ..self.clone()
        }
    }

    /// Create an italic version of this config
    pub fn italic(&self) -> Self {
        Self {
            style: FontStyle::Italic,
            ..self.clone()
        }
    }

    /// Create a smaller version (reduces size by scale factor)
    pub fn smaller(&self, factor: f32) -> Self {
        Self {
            size: self.size * factor,
            ..self.clone()
        }
    }

    /// Create a larger version (increases size by scale factor)
    pub fn larger(&self, factor: f32) -> Self {
        Self {
            size: self.size * factor,
            ..self.clone()
        }
    }

    /// Convert to CSS properties
    pub fn to_css_properties(&self) -> Vec<(String, String)> {
        let mut props = vec![
            ("font-family".to_string(), self.family.to_css()),
            ("font-size".to_string(), format!("{}px", self.size)),
            ("font-weight".to_string(), self.weight.value().to_string()),
            ("font-style".to_string(), self.style.to_css().to_string()),
            ("line-height".to_string(), self.line_height.to_string()),
        ];

        if self.letter_spacing != 0.0 {
            props.push((
                "letter-spacing".to_string(),
                format!("{}px", self.letter_spacing),
            ));
        }

        if self.ligatures {
            props.push((
                "font-feature-settings".to_string(),
                r#""liga" 1, "calt" 1"#.to_string(),
            ));
        }

        props
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: FontFamily::System,
            size: 14.0,
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
            line_height: 1.5,
            letter_spacing: 0.0,
            ligatures: true,
        }
    }
}

// =============================================================================
// Typography
// =============================================================================

/// Complete typography system for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Typography {
    /// UI text font configuration
    pub ui: FontConfig,

    /// Monospace/code font configuration
    pub mono: FontConfig,

    /// Heading font configuration
    pub heading: FontConfig,

    /// Font size scale (for consistent sizing)
    pub scale: FontScale,
}

impl Typography {
    /// Create default typography settings (Zed-inspired)
    pub fn new() -> Self {
        Self::default()
    }

    /// Create typography with custom UI font
    pub fn with_ui_font(font_family: FontFamily) -> Self {
        Self {
            ui: FontConfig {
                family: font_family.clone(),
                ..FontConfig::default()
            },
            heading: FontConfig {
                family: font_family,
                size: 18.0,
                weight: FontWeight::SemiBold,
                line_height: 1.3,
                ..FontConfig::default()
            },
            ..Self::default()
        }
    }

    /// Create typography with custom monospace font
    pub fn with_mono_font(font_family: FontFamily) -> Self {
        Self {
            mono: FontConfig {
                family: font_family,
                size: 13.0,
                line_height: 1.6,
                ligatures: true,
                ..FontConfig::default()
            },
            ..Self::default()
        }
    }

    /// Get font config for a specific text size role
    pub fn size(&self, role: TextSize) -> f32 {
        match role {
            TextSize::Xs => self.scale.xs,
            TextSize::Sm => self.scale.sm,
            TextSize::Base => self.scale.base,
            TextSize::Lg => self.scale.lg,
            TextSize::Xl => self.scale.xl,
            TextSize::Xl2 => self.scale.xl2,
            TextSize::Xl3 => self.scale.xl3,
            TextSize::Xl4 => self.scale.xl4,
        }
    }

    /// Create font config for body text
    pub fn body(&self) -> FontConfig {
        self.ui.clone()
    }

    /// Create font config for small text
    pub fn small(&self) -> FontConfig {
        FontConfig {
            size: self.scale.sm,
            ..self.ui.clone()
        }
    }

    /// Create font config for large text
    pub fn large(&self) -> FontConfig {
        FontConfig {
            size: self.scale.lg,
            ..self.ui.clone()
        }
    }

    /// Create font config for code/monospace text
    pub fn code(&self) -> FontConfig {
        self.mono.clone()
    }

    /// Create font config for headings at different levels
    pub fn heading_level(&self, level: u8) -> FontConfig {
        let size = match level {
            1 => self.scale.xl4,
            2 => self.scale.xl3,
            3 => self.scale.xl2,
            4 => self.scale.xl,
            5 => self.scale.lg,
            _ => self.scale.base,
        };

        let weight = match level {
            1 | 2 => FontWeight::Bold,
            3 | 4 => FontWeight::SemiBold,
            _ => FontWeight::Medium,
        };

        FontConfig {
            size,
            weight,
            ..self.heading.clone()
        }
    }

    /// Create font config for labels
    pub fn label(&self) -> FontConfig {
        FontConfig {
            size: self.scale.sm,
            weight: FontWeight::Medium,
            letter_spacing: 0.3,
            ..self.ui.clone()
        }
    }

    /// Create font config for captions
    pub fn caption(&self) -> FontConfig {
        FontConfig {
            size: self.scale.xs,
            ..self.ui.clone()
        }
    }

    /// Create font config for buttons
    pub fn button(&self) -> FontConfig {
        FontConfig {
            size: self.scale.sm,
            weight: FontWeight::Medium,
            ..self.ui.clone()
        }
    }

    /// Create font config for keyboard shortcuts display
    pub fn shortcut(&self) -> FontConfig {
        FontConfig {
            size: self.scale.xs,
            family: FontFamily::Monospace,
            weight: FontWeight::Medium,
            letter_spacing: 0.5,
            ..FontConfig::default()
        }
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            ui: FontConfig {
                family: FontFamily::System,
                size: 14.0,
                weight: FontWeight::Normal,
                style: FontStyle::Normal,
                line_height: 1.5,
                letter_spacing: 0.0,
                ligatures: false,
            },
            mono: FontConfig {
                family: FontFamily::Monospace,
                size: 13.0,
                weight: FontWeight::Normal,
                style: FontStyle::Normal,
                line_height: 1.6,
                letter_spacing: 0.0,
                ligatures: true,
            },
            heading: FontConfig {
                family: FontFamily::System,
                size: 18.0,
                weight: FontWeight::SemiBold,
                style: FontStyle::Normal,
                line_height: 1.3,
                letter_spacing: -0.3,
                ligatures: false,
            },
            scale: FontScale::default(),
        }
    }
}

// =============================================================================
// Font Scale
// =============================================================================

/// Font size scale for consistent sizing throughout the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontScale {
    /// Extra small (10px default)
    pub xs: f32,
    /// Small (12px default)
    pub sm: f32,
    /// Base/normal (14px default)
    pub base: f32,
    /// Large (16px default)
    pub lg: f32,
    /// Extra large (18px default)
    pub xl: f32,
    /// 2x extra large (20px default)
    pub xl2: f32,
    /// 3x extra large (24px default)
    pub xl3: f32,
    /// 4x extra large (30px default)
    pub xl4: f32,
}

impl FontScale {
    /// Create a scale from a base size using a ratio
    pub fn from_base(base: f32, ratio: f32) -> Self {
        Self {
            xs: base / (ratio * ratio),
            sm: base / ratio,
            base,
            lg: base * ratio,
            xl: base * ratio * ratio,
            xl2: base * ratio * ratio * ratio,
            xl3: base * ratio.powi(4),
            xl4: base * ratio.powi(5),
        }
    }

    /// Create a compact scale (smaller sizes)
    pub fn compact() -> Self {
        Self {
            xs: 9.0,
            sm: 11.0,
            base: 13.0,
            lg: 15.0,
            xl: 17.0,
            xl2: 19.0,
            xl3: 22.0,
            xl4: 26.0,
        }
    }

    /// Create a comfortable scale (larger sizes)
    pub fn comfortable() -> Self {
        Self {
            xs: 11.0,
            sm: 13.0,
            base: 15.0,
            lg: 18.0,
            xl: 21.0,
            xl2: 24.0,
            xl3: 30.0,
            xl4: 36.0,
        }
    }
}

impl Default for FontScale {
    fn default() -> Self {
        Self {
            xs: 10.0,
            sm: 12.0,
            base: 14.0,
            lg: 16.0,
            xl: 18.0,
            xl2: 20.0,
            xl3: 24.0,
            xl4: 30.0,
        }
    }
}

// =============================================================================
// Text Size Role
// =============================================================================

/// Semantic text size roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextSize {
    /// Extra small text
    Xs,
    /// Small text
    Sm,
    /// Base/normal text
    Base,
    /// Large text
    Lg,
    /// Extra large text
    Xl,
    /// 2x extra large
    Xl2,
    /// 3x extra large
    Xl3,
    /// 4x extra large
    Xl4,
}

impl TextSize {
    /// Get all sizes in order
    pub fn all() -> &'static [TextSize] {
        &[
            TextSize::Xs,
            TextSize::Sm,
            TextSize::Base,
            TextSize::Lg,
            TextSize::Xl,
            TextSize::Xl2,
            TextSize::Xl3,
            TextSize::Xl4,
        ]
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_family_css() {
        let system = FontFamily::System;
        assert!(system.to_css().contains("Segoe UI"));

        let mono = FontFamily::Monospace;
        assert!(mono.to_css().contains("JetBrains Mono"));

        let custom = FontFamily::Custom("Inter".to_string());
        assert!(custom.to_css().contains("Inter"));
    }

    #[test]
    fn test_font_family_name() {
        assert_eq!(FontFamily::System.name(), "System");
        assert_eq!(FontFamily::Monospace.name(), "Monospace");
        assert_eq!(FontFamily::Custom("Inter".to_string()).name(), "Inter");
    }

    #[test]
    fn test_font_family_is_monospace() {
        assert!(FontFamily::Monospace.is_monospace());
        assert!(!FontFamily::System.is_monospace());
        assert!(!FontFamily::Custom("Inter".to_string()).is_monospace());
    }

    #[test]
    fn test_font_family_presets() {
        let inter = FontFamily::inter();
        assert!(matches!(inter, FontFamily::Custom(name) if name == "Inter"));

        let jetbrains = FontFamily::jetbrains_mono();
        assert!(matches!(jetbrains, FontFamily::Custom(name) if name == "JetBrains Mono"));
    }

    #[test]
    fn test_font_weight_value() {
        assert_eq!(FontWeight::Normal.value(), 400);
        assert_eq!(FontWeight::Bold.value(), 700);
        assert_eq!(FontWeight::Light.value(), 300);
    }

    #[test]
    fn test_font_weight_from_value() {
        assert_eq!(FontWeight::from_value(400), FontWeight::Normal);
        assert_eq!(FontWeight::from_value(700), FontWeight::Bold);
        assert_eq!(FontWeight::from_value(350), FontWeight::Light);
        assert_eq!(FontWeight::from_value(1000), FontWeight::Black);
    }

    #[test]
    fn test_font_weight_is_bold() {
        assert!(!FontWeight::Normal.is_bold());
        assert!(!FontWeight::Medium.is_bold());
        assert!(FontWeight::SemiBold.is_bold());
        assert!(FontWeight::Bold.is_bold());
        assert!(FontWeight::Black.is_bold());
    }

    #[test]
    fn test_font_style_css() {
        assert_eq!(FontStyle::Normal.to_css(), "normal");
        assert_eq!(FontStyle::Italic.to_css(), "italic");
        assert_eq!(FontStyle::Oblique.to_css(), "oblique");
    }

    #[test]
    fn test_font_config_builder() {
        let config = FontConfig::new()
            .family(FontFamily::inter())
            .size(16.0)
            .weight(FontWeight::Medium)
            .line_height(1.6)
            .letter_spacing(0.5);

        assert_eq!(config.size, 16.0);
        assert_eq!(config.weight, FontWeight::Medium);
        assert_eq!(config.line_height, 1.6);
        assert_eq!(config.letter_spacing, 0.5);
    }

    #[test]
    fn test_font_config_computed_line_height() {
        let config = FontConfig::new().size(16.0).line_height(1.5);
        assert!((config.computed_line_height() - 24.0).abs() < 0.01);
    }

    #[test]
    fn test_font_config_bold() {
        let normal = FontConfig::new().weight(FontWeight::Normal);
        let bold = normal.bold();
        assert_eq!(bold.weight, FontWeight::Bold);
    }

    #[test]
    fn test_font_config_italic() {
        let normal = FontConfig::new().style(FontStyle::Normal);
        let italic = normal.italic();
        assert_eq!(italic.style, FontStyle::Italic);
    }

    #[test]
    fn test_font_config_smaller() {
        let config = FontConfig::new().size(16.0);
        let smaller = config.smaller(0.5);
        assert!((smaller.size - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_font_config_larger() {
        let config = FontConfig::new().size(16.0);
        let larger = config.larger(1.5);
        assert!((larger.size - 24.0).abs() < 0.01);
    }

    #[test]
    fn test_font_config_to_css_properties() {
        let config = FontConfig::new()
            .family(FontFamily::System)
            .size(14.0)
            .weight(FontWeight::Bold)
            .letter_spacing(0.5);

        let props = config.to_css_properties();

        assert!(props.iter().any(|(k, _)| k == "font-family"));
        assert!(props.iter().any(|(k, v)| k == "font-size" && v == "14px"));
        assert!(props.iter().any(|(k, v)| k == "font-weight" && v == "700"));
        assert!(props
            .iter()
            .any(|(k, v)| k == "letter-spacing" && v == "0.5px"));
    }

    #[test]
    fn test_typography_default() {
        let typography = Typography::default();

        assert_eq!(typography.ui.size, 14.0);
        assert_eq!(typography.mono.size, 13.0);
        assert_eq!(typography.heading.size, 18.0);
    }

    #[test]
    fn test_typography_with_ui_font() {
        let typography = Typography::with_ui_font(FontFamily::inter());

        assert!(matches!(typography.ui.family, FontFamily::Custom(name) if name == "Inter"));
        assert!(matches!(typography.heading.family, FontFamily::Custom(name) if name == "Inter"));
    }

    #[test]
    fn test_typography_with_mono_font() {
        let typography = Typography::with_mono_font(FontFamily::fira_code());

        assert!(matches!(&typography.mono.family, FontFamily::Custom(name) if name == "Fira Code"));
    }

    #[test]
    fn test_typography_size() {
        let typography = Typography::default();

        assert_eq!(typography.size(TextSize::Base), 14.0);
        assert_eq!(typography.size(TextSize::Sm), 12.0);
        assert_eq!(typography.size(TextSize::Lg), 16.0);
    }

    #[test]
    fn test_typography_body() {
        let typography = Typography::default();
        let body = typography.body();

        assert_eq!(body.size, typography.ui.size);
        assert_eq!(body.family, typography.ui.family);
    }

    #[test]
    fn test_typography_small() {
        let typography = Typography::default();
        let small = typography.small();

        assert_eq!(small.size, typography.scale.sm);
    }

    #[test]
    fn test_typography_large() {
        let typography = Typography::default();
        let large = typography.large();

        assert_eq!(large.size, typography.scale.lg);
    }

    #[test]
    fn test_typography_code() {
        let typography = Typography::default();
        let code = typography.code();

        assert!(code.family.is_monospace() || matches!(code.family, FontFamily::Monospace));
    }

    #[test]
    fn test_typography_heading_levels() {
        let typography = Typography::default();

        let h1 = typography.heading_level(1);
        let h2 = typography.heading_level(2);
        let h3 = typography.heading_level(3);

        assert!(h1.size > h2.size);
        assert!(h2.size > h3.size);
        assert_eq!(h1.weight, FontWeight::Bold);
        assert_eq!(h3.weight, FontWeight::SemiBold);
    }

    #[test]
    fn test_typography_label() {
        let typography = Typography::default();
        let label = typography.label();

        assert_eq!(label.weight, FontWeight::Medium);
        assert!(label.letter_spacing > 0.0);
    }

    #[test]
    fn test_typography_caption() {
        let typography = Typography::default();
        let caption = typography.caption();

        assert_eq!(caption.size, typography.scale.xs);
    }

    #[test]
    fn test_typography_button() {
        let typography = Typography::default();
        let button = typography.button();

        assert_eq!(button.size, typography.scale.sm);
        assert_eq!(button.weight, FontWeight::Medium);
    }

    #[test]
    fn test_typography_shortcut() {
        let typography = Typography::default();
        let shortcut = typography.shortcut();

        assert_eq!(shortcut.size, typography.scale.xs);
        assert!(shortcut.letter_spacing > 0.0);
    }

    #[test]
    fn test_font_scale_default() {
        let scale = FontScale::default();

        assert_eq!(scale.base, 14.0);
        assert!(scale.xs < scale.sm);
        assert!(scale.sm < scale.base);
        assert!(scale.base < scale.lg);
        assert!(scale.lg < scale.xl);
    }

    #[test]
    fn test_font_scale_from_base() {
        let scale = FontScale::from_base(16.0, 1.25);

        assert_eq!(scale.base, 16.0);
        assert!((scale.lg - 20.0).abs() < 0.01); // 16 * 1.25
        assert!((scale.sm - 12.8).abs() < 0.01); // 16 / 1.25
    }

    #[test]
    fn test_font_scale_compact() {
        let compact = FontScale::compact();
        let default = FontScale::default();

        assert!(compact.base < default.base);
    }

    #[test]
    fn test_font_scale_comfortable() {
        let comfortable = FontScale::comfortable();
        let default = FontScale::default();

        assert!(comfortable.base > default.base);
    }

    #[test]
    fn test_text_size_all() {
        let all = TextSize::all();
        assert_eq!(all.len(), 8);
        assert_eq!(all[0], TextSize::Xs);
        assert_eq!(all[7], TextSize::Xl4);
    }

    #[test]
    fn test_font_weight_display() {
        assert_eq!(format!("{}", FontWeight::Normal), "Normal");
        assert_eq!(format!("{}", FontWeight::Bold), "Bold");
        assert_eq!(format!("{}", FontWeight::SemiBold), "Semi Bold");
    }

    #[test]
    fn test_font_family_display() {
        assert_eq!(format!("{}", FontFamily::System), "System");
        assert_eq!(
            format!("{}", FontFamily::Custom("Inter".to_string())),
            "Inter"
        );
    }

    #[test]
    fn test_font_config_serialization() {
        let config = FontConfig::new()
            .family(FontFamily::inter())
            .size(16.0)
            .weight(FontWeight::Medium);

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: FontConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.size, deserialized.size);
        assert_eq!(config.weight, deserialized.weight);
    }

    #[test]
    fn test_typography_serialization() {
        let typography = Typography::default();

        let json = serde_json::to_string(&typography).unwrap();
        let deserialized: Typography = serde_json::from_str(&json).unwrap();

        assert_eq!(typography.ui.size, deserialized.ui.size);
        assert_eq!(typography.mono.size, deserialized.mono.size);
    }
}

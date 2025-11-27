//! Spacing System - Zed-inspired Layout Spacing
//!
//! This module provides a comprehensive spacing system with:
//! - Consistent spacing tokens for margins and padding
//! - Gap values for flexbox/grid layouts
//! - Sizing tokens for width/height
//! - Responsive breakpoints
//!
//! # Design Philosophy
//!
//! The spacing system uses a base-4 scale (4px increments) which provides:
//! - Visual rhythm and consistency
//! - Easy mental math (4, 8, 12, 16, 20, 24...)
//! - Good alignment with common icon sizes
//!
//! # Example
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::theme::spacing::{Spacing, SpacingScale};
//!
//! // Use default spacing
//! let spacing = Spacing::default();
//!
//! // Access spacing values
//! let padding = spacing.md;  // 16px
//! let gap = spacing.sm;      // 8px
//!
//! // Use scale for specific values
//! let scale = SpacingScale::default();
//! let value = scale.get(4);  // 16px (4 * 4)
//! ```

use serde::{Deserialize, Serialize};

// =============================================================================
// Spacing
// =============================================================================

/// Spacing tokens for consistent layout throughout the UI
///
/// Uses a base-4 scale for visual rhythm and easy calculation.
/// All values are in pixels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Spacing {
    /// No spacing (0px)
    pub none: f32,
    /// Extra extra small (2px)
    pub xxs: f32,
    /// Extra small (4px)
    pub xs: f32,
    /// Small (8px)
    pub sm: f32,
    /// Medium (16px) - default for most padding
    pub md: f32,
    /// Large (24px)
    pub lg: f32,
    /// Extra large (32px)
    pub xl: f32,
    /// Extra extra large (48px)
    pub xxl: f32,
    /// Huge (64px)
    pub huge: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            none: 0.0,
            xxs: 2.0,
            xs: 4.0,
            sm: 8.0,
            md: 16.0,
            lg: 24.0,
            xl: 32.0,
            xxl: 48.0,
            huge: 64.0,
        }
    }
}

impl Spacing {
    /// Create spacing with a custom base value
    ///
    /// All spacing values are derived from the base (default 4px).
    pub fn with_base(base: f32) -> Self {
        Self {
            none: 0.0,
            xxs: base * 0.5,
            xs: base,
            sm: base * 2.0,
            md: base * 4.0,
            lg: base * 6.0,
            xl: base * 8.0,
            xxl: base * 12.0,
            huge: base * 16.0,
        }
    }

    /// Create compact spacing (smaller values)
    pub fn compact() -> Self {
        Self::with_base(3.0)
    }

    /// Create comfortable spacing (larger values)
    pub fn comfortable() -> Self {
        Self::with_base(5.0)
    }

    /// Get spacing by semantic name
    pub fn get(&self, name: SpacingName) -> f32 {
        match name {
            SpacingName::None => self.none,
            SpacingName::Xxs => self.xxs,
            SpacingName::Xs => self.xs,
            SpacingName::Sm => self.sm,
            SpacingName::Md => self.md,
            SpacingName::Lg => self.lg,
            SpacingName::Xl => self.xl,
            SpacingName::Xxl => self.xxl,
            SpacingName::Huge => self.huge,
        }
    }

    /// Get inset padding (all sides equal)
    pub fn inset(&self, name: SpacingName) -> Inset {
        let value = self.get(name);
        Inset::all(value)
    }

    /// Get squish padding (less vertical than horizontal)
    pub fn squish(&self, name: SpacingName) -> Inset {
        let value = self.get(name);
        Inset::symmetric(value * 0.5, value)
    }

    /// Get stretch padding (more vertical than horizontal)
    pub fn stretch(&self, name: SpacingName) -> Inset {
        let value = self.get(name);
        Inset::symmetric(value * 1.5, value)
    }

    /// Get inline spacing (horizontal gap between items)
    pub fn inline(&self, name: SpacingName) -> f32 {
        self.get(name)
    }

    /// Get stack spacing (vertical gap between items)
    pub fn stack(&self, name: SpacingName) -> f32 {
        self.get(name)
    }
}

// =============================================================================
// Spacing Name
// =============================================================================

/// Semantic spacing names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpacingName {
    /// No spacing
    None,
    /// Extra extra small
    Xxs,
    /// Extra small
    Xs,
    /// Small
    Sm,
    /// Medium
    Md,
    /// Large
    Lg,
    /// Extra large
    Xl,
    /// Extra extra large
    Xxl,
    /// Huge
    Huge,
}

impl SpacingName {
    /// Get all spacing names in order
    pub fn all() -> &'static [SpacingName] {
        &[
            SpacingName::None,
            SpacingName::Xxs,
            SpacingName::Xs,
            SpacingName::Sm,
            SpacingName::Md,
            SpacingName::Lg,
            SpacingName::Xl,
            SpacingName::Xxl,
            SpacingName::Huge,
        ]
    }

    /// Get the next larger spacing
    pub fn larger(&self) -> Option<SpacingName> {
        match self {
            SpacingName::None => Some(SpacingName::Xxs),
            SpacingName::Xxs => Some(SpacingName::Xs),
            SpacingName::Xs => Some(SpacingName::Sm),
            SpacingName::Sm => Some(SpacingName::Md),
            SpacingName::Md => Some(SpacingName::Lg),
            SpacingName::Lg => Some(SpacingName::Xl),
            SpacingName::Xl => Some(SpacingName::Xxl),
            SpacingName::Xxl => Some(SpacingName::Huge),
            SpacingName::Huge => None,
        }
    }

    /// Get the next smaller spacing
    pub fn smaller(&self) -> Option<SpacingName> {
        match self {
            SpacingName::None => None,
            SpacingName::Xxs => Some(SpacingName::None),
            SpacingName::Xs => Some(SpacingName::Xxs),
            SpacingName::Sm => Some(SpacingName::Xs),
            SpacingName::Md => Some(SpacingName::Sm),
            SpacingName::Lg => Some(SpacingName::Md),
            SpacingName::Xl => Some(SpacingName::Lg),
            SpacingName::Xxl => Some(SpacingName::Xl),
            SpacingName::Huge => Some(SpacingName::Xxl),
        }
    }
}

impl std::fmt::Display for SpacingName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            SpacingName::None => "none",
            SpacingName::Xxs => "xxs",
            SpacingName::Xs => "xs",
            SpacingName::Sm => "sm",
            SpacingName::Md => "md",
            SpacingName::Lg => "lg",
            SpacingName::Xl => "xl",
            SpacingName::Xxl => "xxl",
            SpacingName::Huge => "huge",
        };
        write!(f, "{}", name)
    }
}

// =============================================================================
// Spacing Scale
// =============================================================================

/// Numeric spacing scale (multiplier-based)
///
/// Allows accessing spacing values by numeric index.
/// Value = index * base (default base is 4px).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpacingScale {
    /// Base unit for spacing (default 4px)
    pub base: f32,
}

impl Default for SpacingScale {
    fn default() -> Self {
        Self { base: 4.0 }
    }
}

impl SpacingScale {
    /// Create a scale with a custom base
    pub fn new(base: f32) -> Self {
        Self { base }
    }

    /// Get spacing value by multiplier
    ///
    /// Returns index * base (e.g., get(4) with base 4 = 16px)
    pub fn get(&self, index: u32) -> f32 {
        index as f32 * self.base
    }

    /// Get spacing value by half-step multiplier
    ///
    /// Allows for finer control (e.g., get_half(3) = 1.5 * base)
    pub fn get_half(&self, index: u32) -> f32 {
        index as f32 * self.base * 0.5
    }

    /// Common spacing values
    pub fn px(&self, pixels: f32) -> f32 {
        pixels
    }

    /// Zero spacing
    pub fn zero(&self) -> f32 {
        0.0
    }

    /// Spacing 1 (4px with default base)
    pub fn s1(&self) -> f32 {
        self.base
    }

    /// Spacing 2 (8px with default base)
    pub fn s2(&self) -> f32 {
        self.base * 2.0
    }

    /// Spacing 3 (12px with default base)
    pub fn s3(&self) -> f32 {
        self.base * 3.0
    }

    /// Spacing 4 (16px with default base)
    pub fn s4(&self) -> f32 {
        self.base * 4.0
    }

    /// Spacing 5 (20px with default base)
    pub fn s5(&self) -> f32 {
        self.base * 5.0
    }

    /// Spacing 6 (24px with default base)
    pub fn s6(&self) -> f32 {
        self.base * 6.0
    }

    /// Spacing 8 (32px with default base)
    pub fn s8(&self) -> f32 {
        self.base * 8.0
    }

    /// Spacing 10 (40px with default base)
    pub fn s10(&self) -> f32 {
        self.base * 10.0
    }

    /// Spacing 12 (48px with default base)
    pub fn s12(&self) -> f32 {
        self.base * 12.0
    }

    /// Spacing 16 (64px with default base)
    pub fn s16(&self) -> f32 {
        self.base * 16.0
    }
}

// =============================================================================
// Inset (Padding)
// =============================================================================

/// Inset values for padding (all four sides)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Inset {
    /// Top padding
    pub top: f32,
    /// Right padding
    pub right: f32,
    /// Bottom padding
    pub bottom: f32,
    /// Left padding
    pub left: f32,
}

impl Default for Inset {
    fn default() -> Self {
        Self::zero()
    }
}

impl Inset {
    /// Create inset with all sides equal
    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Create inset with no padding
    pub fn zero() -> Self {
        Self::all(0.0)
    }

    /// Create inset with symmetric values (vertical, horizontal)
    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Create inset with only horizontal padding
    pub fn horizontal(value: f32) -> Self {
        Self {
            top: 0.0,
            right: value,
            bottom: 0.0,
            left: value,
        }
    }

    /// Create inset with only vertical padding
    pub fn vertical(value: f32) -> Self {
        Self {
            top: value,
            right: 0.0,
            bottom: value,
            left: 0.0,
        }
    }

    /// Create inset with individual values
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Create inset with only top padding
    pub fn only_top(value: f32) -> Self {
        Self {
            top: value,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }

    /// Create inset with only right padding
    pub fn only_right(value: f32) -> Self {
        Self {
            top: 0.0,
            right: value,
            bottom: 0.0,
            left: 0.0,
        }
    }

    /// Create inset with only bottom padding
    pub fn only_bottom(value: f32) -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: value,
            left: 0.0,
        }
    }

    /// Create inset with only left padding
    pub fn only_left(value: f32) -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: value,
        }
    }

    /// Get total horizontal padding
    pub fn horizontal_total(&self) -> f32 {
        self.left + self.right
    }

    /// Get total vertical padding
    pub fn vertical_total(&self) -> f32 {
        self.top + self.bottom
    }

    /// Check if all sides are equal
    pub fn is_uniform(&self) -> bool {
        self.top == self.right && self.right == self.bottom && self.bottom == self.left
    }

    /// Check if inset is zero
    pub fn is_zero(&self) -> bool {
        self.top == 0.0 && self.right == 0.0 && self.bottom == 0.0 && self.left == 0.0
    }

    /// Add another inset to this one
    pub fn add(&self, other: &Inset) -> Self {
        Self {
            top: self.top + other.top,
            right: self.right + other.right,
            bottom: self.bottom + other.bottom,
            left: self.left + other.left,
        }
    }

    /// Scale this inset by a factor
    pub fn scale(&self, factor: f32) -> Self {
        Self {
            top: self.top * factor,
            right: self.right * factor,
            bottom: self.bottom * factor,
            left: self.left * factor,
        }
    }

    /// Convert to CSS padding string
    pub fn to_css(&self) -> String {
        if self.is_uniform() {
            format!("{}px", self.top)
        } else if self.top == self.bottom && self.left == self.right {
            format!("{}px {}px", self.top, self.right)
        } else if self.left == self.right {
            format!("{}px {}px {}px", self.top, self.right, self.bottom)
        } else {
            format!(
                "{}px {}px {}px {}px",
                self.top, self.right, self.bottom, self.left
            )
        }
    }
}

// =============================================================================
// Size
// =============================================================================

/// Size tokens for width/height
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Size {
    /// Extra extra small (16px)
    pub xxs: f32,
    /// Extra small (24px)
    pub xs: f32,
    /// Small (32px)
    pub sm: f32,
    /// Medium (40px)
    pub md: f32,
    /// Large (48px)
    pub lg: f32,
    /// Extra large (64px)
    pub xl: f32,
    /// Extra extra large (80px)
    pub xxl: f32,
}

impl Default for Size {
    fn default() -> Self {
        Self {
            xxs: 16.0,
            xs: 24.0,
            sm: 32.0,
            md: 40.0,
            lg: 48.0,
            xl: 64.0,
            xxl: 80.0,
        }
    }
}

impl Size {
    /// Create sizes for icons
    pub fn icons() -> Self {
        Self {
            xxs: 12.0,
            xs: 16.0,
            sm: 20.0,
            md: 24.0,
            lg: 32.0,
            xl: 48.0,
            xxl: 64.0,
        }
    }

    /// Create sizes for buttons
    pub fn buttons() -> Self {
        Self {
            xxs: 24.0,
            xs: 28.0,
            sm: 32.0,
            md: 36.0,
            lg: 40.0,
            xl: 48.0,
            xxl: 56.0,
        }
    }

    /// Create sizes for avatars
    pub fn avatars() -> Self {
        Self {
            xxs: 20.0,
            xs: 24.0,
            sm: 32.0,
            md: 40.0,
            lg: 56.0,
            xl: 80.0,
            xxl: 120.0,
        }
    }

    /// Create sizes for thumbnails
    pub fn thumbnails() -> Self {
        Self {
            xxs: 32.0,
            xs: 48.0,
            sm: 64.0,
            md: 96.0,
            lg: 128.0,
            xl: 192.0,
            xxl: 256.0,
        }
    }

    /// Get size by name
    pub fn get(&self, name: SizeName) -> f32 {
        match name {
            SizeName::Xxs => self.xxs,
            SizeName::Xs => self.xs,
            SizeName::Sm => self.sm,
            SizeName::Md => self.md,
            SizeName::Lg => self.lg,
            SizeName::Xl => self.xl,
            SizeName::Xxl => self.xxl,
        }
    }
}

/// Semantic size names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeName {
    /// Extra extra small
    Xxs,
    /// Extra small
    Xs,
    /// Small
    Sm,
    /// Medium
    Md,
    /// Large
    Lg,
    /// Extra large
    Xl,
    /// Extra extra large
    Xxl,
}

// =============================================================================
// Breakpoints
// =============================================================================

/// Responsive breakpoints for different screen sizes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Breakpoints {
    /// Extra small screens (mobile portrait)
    pub xs: f32,
    /// Small screens (mobile landscape)
    pub sm: f32,
    /// Medium screens (tablets)
    pub md: f32,
    /// Large screens (laptops)
    pub lg: f32,
    /// Extra large screens (desktops)
    pub xl: f32,
    /// Extra extra large screens (large desktops)
    pub xxl: f32,
}

impl Default for Breakpoints {
    fn default() -> Self {
        Self {
            xs: 0.0,
            sm: 576.0,
            md: 768.0,
            lg: 992.0,
            xl: 1200.0,
            xxl: 1400.0,
        }
    }
}

impl Breakpoints {
    /// Get the breakpoint name for a given width
    pub fn name_for_width(&self, width: f32) -> BreakpointName {
        if width >= self.xxl {
            BreakpointName::Xxl
        } else if width >= self.xl {
            BreakpointName::Xl
        } else if width >= self.lg {
            BreakpointName::Lg
        } else if width >= self.md {
            BreakpointName::Md
        } else if width >= self.sm {
            BreakpointName::Sm
        } else {
            BreakpointName::Xs
        }
    }

    /// Check if width is at least the given breakpoint
    pub fn is_at_least(&self, width: f32, breakpoint: BreakpointName) -> bool {
        width >= self.get(breakpoint)
    }

    /// Get breakpoint value by name
    pub fn get(&self, name: BreakpointName) -> f32 {
        match name {
            BreakpointName::Xs => self.xs,
            BreakpointName::Sm => self.sm,
            BreakpointName::Md => self.md,
            BreakpointName::Lg => self.lg,
            BreakpointName::Xl => self.xl,
            BreakpointName::Xxl => self.xxl,
        }
    }

    /// Generate CSS media query for minimum width
    pub fn media_query_min(&self, breakpoint: BreakpointName) -> String {
        format!("@media (min-width: {}px)", self.get(breakpoint))
    }

    /// Generate CSS media query for maximum width
    pub fn media_query_max(&self, breakpoint: BreakpointName) -> String {
        format!("@media (max-width: {}px)", self.get(breakpoint) - 0.02)
    }
}

/// Breakpoint names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BreakpointName {
    /// Extra small
    Xs,
    /// Small
    Sm,
    /// Medium
    Md,
    /// Large
    Lg,
    /// Extra large
    Xl,
    /// Extra extra large
    Xxl,
}

impl BreakpointName {
    /// Get all breakpoint names in order
    pub fn all() -> &'static [BreakpointName] {
        &[
            BreakpointName::Xs,
            BreakpointName::Sm,
            BreakpointName::Md,
            BreakpointName::Lg,
            BreakpointName::Xl,
            BreakpointName::Xxl,
        ]
    }
}

// =============================================================================
// Z-Index
// =============================================================================

/// Z-index layers for stacking context
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ZIndex {
    /// Base layer (0)
    pub base: i32,
    /// Content above base (10)
    pub content: i32,
    /// Sticky elements (20)
    pub sticky: i32,
    /// Fixed elements (30)
    pub fixed: i32,
    /// Dropdown menus (40)
    pub dropdown: i32,
    /// Overlays/backdrops (50)
    pub overlay: i32,
    /// Modal dialogs (60)
    pub modal: i32,
    /// Popovers/tooltips (70)
    pub popover: i32,
    /// Toast notifications (80)
    pub toast: i32,
    /// Maximum layer (9999)
    pub max: i32,
}

impl Default for ZIndex {
    fn default() -> Self {
        Self {
            base: 0,
            content: 10,
            sticky: 20,
            fixed: 30,
            dropdown: 40,
            overlay: 50,
            modal: 60,
            popover: 70,
            toast: 80,
            max: 9999,
        }
    }
}

impl ZIndex {
    /// Get z-index by layer name
    pub fn get(&self, layer: ZLayer) -> i32 {
        match layer {
            ZLayer::Base => self.base,
            ZLayer::Content => self.content,
            ZLayer::Sticky => self.sticky,
            ZLayer::Fixed => self.fixed,
            ZLayer::Dropdown => self.dropdown,
            ZLayer::Overlay => self.overlay,
            ZLayer::Modal => self.modal,
            ZLayer::Popover => self.popover,
            ZLayer::Toast => self.toast,
            ZLayer::Max => self.max,
        }
    }
}

/// Z-index layer names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZLayer {
    /// Base layer
    Base,
    /// Content layer
    Content,
    /// Sticky elements
    Sticky,
    /// Fixed elements
    Fixed,
    /// Dropdown menus
    Dropdown,
    /// Overlay/backdrop
    Overlay,
    /// Modal dialogs
    Modal,
    /// Popovers/tooltips
    Popover,
    /// Toast notifications
    Toast,
    /// Maximum layer
    Max,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spacing_default() {
        let spacing = Spacing::default();

        assert_eq!(spacing.none, 0.0);
        assert_eq!(spacing.xs, 4.0);
        assert_eq!(spacing.sm, 8.0);
        assert_eq!(spacing.md, 16.0);
        assert_eq!(spacing.lg, 24.0);
        assert_eq!(spacing.xl, 32.0);
    }

    #[test]
    fn test_spacing_with_base() {
        let spacing = Spacing::with_base(5.0);

        assert_eq!(spacing.xs, 5.0);
        assert_eq!(spacing.sm, 10.0);
        assert_eq!(spacing.md, 20.0);
    }

    #[test]
    fn test_spacing_compact() {
        let compact = Spacing::compact();
        let default = Spacing::default();

        assert!(compact.md < default.md);
    }

    #[test]
    fn test_spacing_comfortable() {
        let comfortable = Spacing::comfortable();
        let default = Spacing::default();

        assert!(comfortable.md > default.md);
    }

    #[test]
    fn test_spacing_get() {
        let spacing = Spacing::default();

        assert_eq!(spacing.get(SpacingName::None), 0.0);
        assert_eq!(spacing.get(SpacingName::Md), 16.0);
        assert_eq!(spacing.get(SpacingName::Xl), 32.0);
    }

    #[test]
    fn test_spacing_inset() {
        let spacing = Spacing::default();
        let inset = spacing.inset(SpacingName::Md);

        assert_eq!(inset.top, 16.0);
        assert_eq!(inset.right, 16.0);
        assert_eq!(inset.bottom, 16.0);
        assert_eq!(inset.left, 16.0);
    }

    #[test]
    fn test_spacing_squish() {
        let spacing = Spacing::default();
        let squish = spacing.squish(SpacingName::Md);

        assert_eq!(squish.top, 8.0); // Half vertical
        assert_eq!(squish.right, 16.0);
        assert_eq!(squish.bottom, 8.0);
        assert_eq!(squish.left, 16.0);
    }

    #[test]
    fn test_spacing_stretch() {
        let spacing = Spacing::default();
        let stretch = spacing.stretch(SpacingName::Md);

        assert_eq!(stretch.top, 24.0); // 1.5x vertical
        assert_eq!(stretch.right, 16.0);
    }

    #[test]
    fn test_spacing_name_all() {
        let all = SpacingName::all();
        assert_eq!(all.len(), 9);
        assert_eq!(all[0], SpacingName::None);
        assert_eq!(all[8], SpacingName::Huge);
    }

    #[test]
    fn test_spacing_name_larger() {
        assert_eq!(SpacingName::Sm.larger(), Some(SpacingName::Md));
        assert_eq!(SpacingName::Huge.larger(), None);
    }

    #[test]
    fn test_spacing_name_smaller() {
        assert_eq!(SpacingName::Md.smaller(), Some(SpacingName::Sm));
        assert_eq!(SpacingName::None.smaller(), None);
    }

    #[test]
    fn test_spacing_name_display() {
        assert_eq!(format!("{}", SpacingName::Md), "md");
        assert_eq!(format!("{}", SpacingName::Xxl), "xxl");
    }

    #[test]
    fn test_spacing_scale_default() {
        let scale = SpacingScale::default();

        assert_eq!(scale.base, 4.0);
        assert_eq!(scale.get(4), 16.0);
        assert_eq!(scale.get(8), 32.0);
    }

    #[test]
    fn test_spacing_scale_get_half() {
        let scale = SpacingScale::default();

        assert_eq!(scale.get_half(3), 6.0); // 1.5 * 4
    }

    #[test]
    fn test_spacing_scale_shortcuts() {
        let scale = SpacingScale::default();

        assert_eq!(scale.zero(), 0.0);
        assert_eq!(scale.s1(), 4.0);
        assert_eq!(scale.s2(), 8.0);
        assert_eq!(scale.s4(), 16.0);
        assert_eq!(scale.s8(), 32.0);
    }

    #[test]
    fn test_inset_all() {
        let inset = Inset::all(16.0);

        assert_eq!(inset.top, 16.0);
        assert_eq!(inset.right, 16.0);
        assert_eq!(inset.bottom, 16.0);
        assert_eq!(inset.left, 16.0);
        assert!(inset.is_uniform());
    }

    #[test]
    fn test_inset_zero() {
        let inset = Inset::zero();

        assert!(inset.is_zero());
        assert!(inset.is_uniform());
    }

    #[test]
    fn test_inset_symmetric() {
        let inset = Inset::symmetric(8.0, 16.0);

        assert_eq!(inset.top, 8.0);
        assert_eq!(inset.right, 16.0);
        assert_eq!(inset.bottom, 8.0);
        assert_eq!(inset.left, 16.0);
        assert!(!inset.is_uniform());
    }

    #[test]
    fn test_inset_horizontal() {
        let inset = Inset::horizontal(16.0);

        assert_eq!(inset.top, 0.0);
        assert_eq!(inset.right, 16.0);
        assert_eq!(inset.bottom, 0.0);
        assert_eq!(inset.left, 16.0);
    }

    #[test]
    fn test_inset_vertical() {
        let inset = Inset::vertical(16.0);

        assert_eq!(inset.top, 16.0);
        assert_eq!(inset.right, 0.0);
        assert_eq!(inset.bottom, 16.0);
        assert_eq!(inset.left, 0.0);
    }

    #[test]
    fn test_inset_only() {
        let top = Inset::only_top(16.0);
        assert_eq!(top.top, 16.0);
        assert_eq!(top.right, 0.0);

        let right = Inset::only_right(16.0);
        assert_eq!(right.right, 16.0);
        assert_eq!(right.top, 0.0);
    }

    #[test]
    fn test_inset_totals() {
        let inset = Inset::new(8.0, 16.0, 8.0, 16.0);

        assert_eq!(inset.horizontal_total(), 32.0);
        assert_eq!(inset.vertical_total(), 16.0);
    }

    #[test]
    fn test_inset_add() {
        let a = Inset::all(8.0);
        let b = Inset::all(8.0);
        let sum = a.add(&b);

        assert_eq!(sum.top, 16.0);
        assert_eq!(sum.right, 16.0);
    }

    #[test]
    fn test_inset_scale() {
        let inset = Inset::all(8.0);
        let scaled = inset.scale(2.0);

        assert_eq!(scaled.top, 16.0);
        assert_eq!(scaled.right, 16.0);
    }

    #[test]
    fn test_inset_to_css() {
        assert_eq!(Inset::all(16.0).to_css(), "16px");
        assert_eq!(Inset::symmetric(8.0, 16.0).to_css(), "8px 16px");
        assert_eq!(Inset::new(8.0, 16.0, 24.0, 16.0).to_css(), "8px 16px 24px");
        assert_eq!(
            Inset::new(8.0, 16.0, 24.0, 32.0).to_css(),
            "8px 16px 24px 32px"
        );
    }

    #[test]
    fn test_size_default() {
        let size = Size::default();

        assert_eq!(size.sm, 32.0);
        assert_eq!(size.md, 40.0);
        assert_eq!(size.lg, 48.0);
    }

    #[test]
    fn test_size_icons() {
        let icons = Size::icons();

        assert_eq!(icons.sm, 20.0);
        assert_eq!(icons.md, 24.0);
    }

    #[test]
    fn test_size_buttons() {
        let buttons = Size::buttons();

        assert_eq!(buttons.sm, 32.0);
        assert_eq!(buttons.md, 36.0);
    }

    #[test]
    fn test_size_thumbnails() {
        let thumbnails = Size::thumbnails();

        assert_eq!(thumbnails.sm, 64.0);
        assert_eq!(thumbnails.md, 96.0);
    }

    #[test]
    fn test_size_get() {
        let size = Size::default();

        assert_eq!(size.get(SizeName::Sm), 32.0);
        assert_eq!(size.get(SizeName::Md), 40.0);
    }

    #[test]
    fn test_breakpoints_default() {
        let bp = Breakpoints::default();

        assert_eq!(bp.sm, 576.0);
        assert_eq!(bp.md, 768.0);
        assert_eq!(bp.lg, 992.0);
    }

    #[test]
    fn test_breakpoints_name_for_width() {
        let bp = Breakpoints::default();

        assert_eq!(bp.name_for_width(400.0), BreakpointName::Xs);
        assert_eq!(bp.name_for_width(600.0), BreakpointName::Sm);
        assert_eq!(bp.name_for_width(800.0), BreakpointName::Md);
        assert_eq!(bp.name_for_width(1000.0), BreakpointName::Lg);
    }

    #[test]
    fn test_breakpoints_is_at_least() {
        let bp = Breakpoints::default();

        assert!(bp.is_at_least(800.0, BreakpointName::Md));
        assert!(!bp.is_at_least(600.0, BreakpointName::Md));
    }

    #[test]
    fn test_breakpoints_media_query() {
        let bp = Breakpoints::default();

        assert_eq!(
            bp.media_query_min(BreakpointName::Md),
            "@media (min-width: 768px)"
        );
    }

    #[test]
    fn test_breakpoint_name_all() {
        let all = BreakpointName::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_zindex_default() {
        let z = ZIndex::default();

        assert_eq!(z.base, 0);
        assert_eq!(z.modal, 60);
        assert_eq!(z.max, 9999);
    }

    #[test]
    fn test_zindex_get() {
        let z = ZIndex::default();

        assert_eq!(z.get(ZLayer::Base), 0);
        assert_eq!(z.get(ZLayer::Modal), 60);
        assert_eq!(z.get(ZLayer::Toast), 80);
    }

    #[test]
    fn test_spacing_serialization() {
        let spacing = Spacing::default();
        let json = serde_json::to_string(&spacing).unwrap();
        let deserialized: Spacing = serde_json::from_str(&json).unwrap();

        assert_eq!(spacing.md, deserialized.md);
    }

    #[test]
    fn test_inset_serialization() {
        let inset = Inset::all(16.0);
        let json = serde_json::to_string(&inset).unwrap();
        let deserialized: Inset = serde_json::from_str(&json).unwrap();

        assert_eq!(inset, deserialized);
    }
}

//! Panels Module - Zed-inspired Pluggable Panel System
//!
//! This module provides a flexible panel system inspired by Zed editor's
//! pluggable, toggle-able panel architecture. Panels can be:
//!
//! - Shown/hidden independently
//! - Resized by the user
//! - Docked to different positions
//! - Focused for keyboard navigation
//!
//! # Architecture
//!
//! The panel system is built around these core concepts:
//!
//! 1. **Panel** - A UI region with content (device list, preview, progress, etc.)
//! 2. **PanelPosition** - Where the panel is docked (left, right, bottom, center)
//! 3. **PanelLayout** - The overall arrangement of panels
//! 4. **PanelManager** - Manages panel state and layout
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::panels::{
//!     Panel, PanelId, PanelPosition, PanelManager, PanelConfig,
//! };
//!
//! // Create a panel manager
//! let mut manager = PanelManager::new();
//!
//! // Toggle a panel
//! manager.toggle_panel(PanelId::DeviceList);
//!
//! // Change panel position
//! manager.set_panel_position(PanelId::Preview, PanelPosition::Right);
//!
//! // Get visible panels
//! let visible = manager.visible_panels();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// PanelId
// =============================================================================

/// Unique identifier for each panel type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelId {
    /// Device list panel (shows connected iOS devices)
    DeviceList,

    /// Photo/video preview panel
    Preview,

    /// Extraction progress panel
    Progress,

    /// Log/output panel
    Log,

    /// Settings panel
    Settings,

    /// File browser panel
    FileBrowser,

    /// Duplicate detection results panel
    Duplicates,

    /// Status/info panel
    Status,
}

impl PanelId {
    /// Get all panel IDs
    pub fn all() -> &'static [PanelId] {
        &[
            PanelId::DeviceList,
            PanelId::Preview,
            PanelId::Progress,
            PanelId::Log,
            PanelId::Settings,
            PanelId::FileBrowser,
            PanelId::Duplicates,
            PanelId::Status,
        ]
    }

    /// Get the default display name
    pub fn display_name(&self) -> &'static str {
        match self {
            PanelId::DeviceList => "Devices",
            PanelId::Preview => "Preview",
            PanelId::Progress => "Progress",
            PanelId::Log => "Log",
            PanelId::Settings => "Settings",
            PanelId::FileBrowser => "Files",
            PanelId::Duplicates => "Duplicates",
            PanelId::Status => "Status",
        }
    }

    /// Get the default icon
    pub fn icon(&self) -> &'static str {
        match self {
            PanelId::DeviceList => "📱",
            PanelId::Preview => "🖼️",
            PanelId::Progress => "📊",
            PanelId::Log => "📋",
            PanelId::Settings => "⚙️",
            PanelId::FileBrowser => "📁",
            PanelId::Duplicates => "🔍",
            PanelId::Status => "ℹ️",
        }
    }

    /// Get the default position for this panel
    pub fn default_position(&self) -> PanelPosition {
        match self {
            PanelId::DeviceList => PanelPosition::Left,
            PanelId::Preview => PanelPosition::Center,
            PanelId::Progress => PanelPosition::Bottom,
            PanelId::Log => PanelPosition::Bottom,
            PanelId::Settings => PanelPosition::Right,
            PanelId::FileBrowser => PanelPosition::Left,
            PanelId::Duplicates => PanelPosition::Right,
            PanelId::Status => PanelPosition::Bottom,
        }
    }

    /// Check if this panel is visible by default
    pub fn default_visible(&self) -> bool {
        matches!(
            self,
            PanelId::DeviceList | PanelId::Preview | PanelId::Progress
        )
    }
}

impl std::fmt::Display for PanelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// =============================================================================
// PanelPosition
// =============================================================================

/// Position where a panel can be docked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PanelPosition {
    /// Left sidebar
    Left,

    /// Right sidebar
    Right,

    /// Bottom panel
    Bottom,

    /// Main content area
    #[default]
    Center,

    /// Floating/overlay
    Floating,
}

impl PanelPosition {
    /// Check if this is a sidebar position
    pub fn is_sidebar(&self) -> bool {
        matches!(self, PanelPosition::Left | PanelPosition::Right)
    }

    /// Check if this is the center/main position
    pub fn is_center(&self) -> bool {
        matches!(self, PanelPosition::Center)
    }

    /// Check if this is floating
    pub fn is_floating(&self) -> bool {
        matches!(self, PanelPosition::Floating)
    }

    /// Get the opposite position (for swapping)
    pub fn opposite(&self) -> Self {
        match self {
            PanelPosition::Left => PanelPosition::Right,
            PanelPosition::Right => PanelPosition::Left,
            PanelPosition::Bottom => PanelPosition::Center,
            PanelPosition::Center => PanelPosition::Bottom,
            PanelPosition::Floating => PanelPosition::Center,
        }
    }
}

impl std::fmt::Display for PanelPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            PanelPosition::Left => "Left",
            PanelPosition::Right => "Right",
            PanelPosition::Bottom => "Bottom",
            PanelPosition::Center => "Center",
            PanelPosition::Floating => "Floating",
        };
        write!(f, "{}", name)
    }
}

// =============================================================================
// PanelSize
// =============================================================================

/// Size specification for a panel
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PanelSize {
    /// Preferred width in pixels (for left/right panels)
    pub width: f32,

    /// Preferred height in pixels (for bottom panels)
    pub height: f32,

    /// Minimum width
    pub min_width: f32,

    /// Minimum height
    pub min_height: f32,

    /// Maximum width (0 = unlimited)
    pub max_width: f32,

    /// Maximum height (0 = unlimited)
    pub max_height: f32,
}

impl Default for PanelSize {
    fn default() -> Self {
        Self {
            width: 250.0,
            height: 200.0,
            min_width: 150.0,
            min_height: 100.0,
            max_width: 600.0,
            max_height: 500.0,
        }
    }
}

impl PanelSize {
    /// Create a new panel size
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }

    /// Create a size for a sidebar panel
    pub fn sidebar(width: f32) -> Self {
        Self {
            width,
            height: 0.0, // Full height
            min_width: 150.0,
            min_height: 0.0,
            max_width: 500.0,
            max_height: 0.0,
        }
    }

    /// Create a size for a bottom panel
    pub fn bottom(height: f32) -> Self {
        Self {
            width: 0.0, // Full width
            height,
            min_width: 0.0,
            min_height: 80.0,
            max_width: 0.0,
            max_height: 400.0,
        }
    }

    /// Clamp width to min/max constraints
    pub fn clamp_width(&self, width: f32) -> f32 {
        let max = if self.max_width > 0.0 {
            self.max_width
        } else {
            f32::MAX
        };
        width.clamp(self.min_width, max)
    }

    /// Clamp height to min/max constraints
    pub fn clamp_height(&self, height: f32) -> f32 {
        let max = if self.max_height > 0.0 {
            self.max_height
        } else {
            f32::MAX
        };
        height.clamp(self.min_height, max)
    }

    /// Set width with clamping
    pub fn set_width(&mut self, width: f32) {
        self.width = self.clamp_width(width);
    }

    /// Set height with clamping
    pub fn set_height(&mut self, height: f32) {
        self.height = self.clamp_height(height);
    }
}

// =============================================================================
// Panel
// =============================================================================

/// A panel definition with its state and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Panel {
    /// Panel identifier
    pub id: PanelId,

    /// Display name (can be customized)
    pub name: String,

    /// Icon for display
    pub icon: String,

    /// Current position
    pub position: PanelPosition,

    /// Size configuration
    pub size: PanelSize,

    /// Whether the panel is visible
    pub visible: bool,

    /// Whether the panel is focused
    #[serde(skip)]
    pub focused: bool,

    /// Whether the panel is collapsed (minimized to icon)
    pub collapsed: bool,

    /// Whether the panel can be closed
    pub closable: bool,

    /// Whether the panel can be moved
    pub movable: bool,

    /// Whether the panel can be resized
    pub resizable: bool,

    /// Order within the position (for stacking)
    pub order: i32,

    /// Custom data/state (for extensions)
    #[serde(default)]
    pub custom_data: HashMap<String, String>,
}

impl Panel {
    /// Create a new panel from an ID
    pub fn new(id: PanelId) -> Self {
        Self {
            id,
            name: id.display_name().to_string(),
            icon: id.icon().to_string(),
            position: id.default_position(),
            size: PanelSize::default(),
            visible: id.default_visible(),
            focused: false,
            collapsed: false,
            closable: true,
            movable: true,
            resizable: true,
            order: 0,
            custom_data: HashMap::new(),
        }
    }

    /// Set the position
    pub fn position(mut self, position: PanelPosition) -> Self {
        self.position = position;
        self
    }

    /// Set visibility
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set size
    pub fn size(mut self, size: PanelSize) -> Self {
        self.size = size;
        self
    }

    /// Set closable
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Set movable
    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }

    /// Set resizable
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Set order
    pub fn order(mut self, order: i32) -> Self {
        self.order = order;
        self
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.visible = true;
        self.collapsed = false;
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
        self.focused = false;
    }

    /// Focus the panel
    pub fn focus(&mut self) {
        self.focused = true;
        self.visible = true;
        self.collapsed = false;
    }

    /// Unfocus the panel
    pub fn unfocus(&mut self) {
        self.focused = false;
    }

    /// Collapse the panel
    pub fn collapse(&mut self) {
        self.collapsed = true;
    }

    /// Expand the panel
    pub fn expand(&mut self) {
        self.collapsed = false;
    }

    /// Toggle collapsed state
    pub fn toggle_collapse(&mut self) {
        self.collapsed = !self.collapsed;
    }

    /// Set custom data
    pub fn set_custom(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.custom_data.insert(key.into(), value.into());
    }

    /// Get custom data
    pub fn get_custom(&self, key: &str) -> Option<&String> {
        self.custom_data.get(key)
    }
}

// =============================================================================
// PanelLayout
// =============================================================================

/// Describes the overall panel layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelLayout {
    /// Width of left sidebar
    pub left_width: f32,

    /// Width of right sidebar
    pub right_width: f32,

    /// Height of bottom panel
    pub bottom_height: f32,

    /// Whether left sidebar is visible
    pub left_visible: bool,

    /// Whether right sidebar is visible
    pub right_visible: bool,

    /// Whether bottom panel is visible
    pub bottom_visible: bool,
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            left_width: 250.0,
            right_width: 300.0,
            bottom_height: 200.0,
            left_visible: true,
            right_visible: false,
            bottom_visible: true,
        }
    }
}

impl PanelLayout {
    /// Create a minimal layout (center only)
    pub fn minimal() -> Self {
        Self {
            left_visible: false,
            right_visible: false,
            bottom_visible: false,
            ..Default::default()
        }
    }

    /// Create a full layout (all panels)
    pub fn full() -> Self {
        Self {
            left_visible: true,
            right_visible: true,
            bottom_visible: true,
            ..Default::default()
        }
    }

    /// Calculate the center area width given total width
    pub fn center_width(&self, total_width: f32) -> f32 {
        let left = if self.left_visible {
            self.left_width
        } else {
            0.0
        };
        let right = if self.right_visible {
            self.right_width
        } else {
            0.0
        };
        (total_width - left - right).max(100.0)
    }

    /// Calculate the center area height given total height
    pub fn center_height(&self, total_height: f32) -> f32 {
        let bottom = if self.bottom_visible {
            self.bottom_height
        } else {
            0.0
        };
        (total_height - bottom).max(100.0)
    }

    /// Toggle a sidebar
    pub fn toggle_sidebar(&mut self, position: PanelPosition) {
        match position {
            PanelPosition::Left => self.left_visible = !self.left_visible,
            PanelPosition::Right => self.right_visible = !self.right_visible,
            _ => {}
        }
    }

    /// Toggle bottom panel
    pub fn toggle_bottom(&mut self) {
        self.bottom_visible = !self.bottom_visible;
    }
}

// =============================================================================
// PanelConfig
// =============================================================================

/// Configuration for the panel system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfig {
    /// Minimum sidebar width
    pub min_sidebar_width: f32,

    /// Maximum sidebar width
    pub max_sidebar_width: f32,

    /// Minimum bottom height
    pub min_bottom_height: f32,

    /// Maximum bottom height
    pub max_bottom_height: f32,

    /// Show panel headers
    pub show_headers: bool,

    /// Show panel icons
    pub show_icons: bool,

    /// Enable panel animations
    pub animate_panels: bool,

    /// Animation duration in milliseconds
    pub animation_duration_ms: u32,

    /// Resize handle width
    pub resize_handle_width: f32,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            min_sidebar_width: 150.0,
            max_sidebar_width: 600.0,
            min_bottom_height: 80.0,
            max_bottom_height: 500.0,
            show_headers: true,
            show_icons: true,
            animate_panels: true,
            animation_duration_ms: 200,
            resize_handle_width: 4.0,
        }
    }
}

// =============================================================================
// PanelManager
// =============================================================================

/// Manages all panels and their state
#[derive(Debug, Clone)]
pub struct PanelManager {
    /// All panels
    panels: HashMap<PanelId, Panel>,

    /// Currently focused panel
    focused: Option<PanelId>,

    /// Layout configuration
    layout: PanelLayout,

    /// Panel config
    config: PanelConfig,

    /// Focus history (for navigation)
    focus_history: Vec<PanelId>,
}

impl PanelManager {
    /// Create a new panel manager with default panels
    pub fn new() -> Self {
        let mut panels = HashMap::new();

        for id in PanelId::all() {
            panels.insert(*id, Panel::new(*id));
        }

        Self {
            panels,
            focused: Some(PanelId::DeviceList),
            layout: PanelLayout::default(),
            config: PanelConfig::default(),
            focus_history: Vec::new(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: PanelConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Get a panel by ID
    pub fn get(&self, id: PanelId) -> Option<&Panel> {
        self.panels.get(&id)
    }

    /// Get a mutable panel by ID
    pub fn get_mut(&mut self, id: PanelId) -> Option<&mut Panel> {
        self.panels.get_mut(&id)
    }

    /// Get all panels
    pub fn all(&self) -> impl Iterator<Item = &Panel> {
        self.panels.values()
    }

    /// Get visible panels
    pub fn visible_panels(&self) -> Vec<&Panel> {
        self.panels.values().filter(|p| p.visible).collect()
    }

    /// Get panels at a specific position
    pub fn panels_at(&self, position: PanelPosition) -> Vec<&Panel> {
        let mut panels: Vec<_> = self
            .panels
            .values()
            .filter(|p| p.visible && p.position == position)
            .collect();
        panels.sort_by_key(|p| p.order);
        panels
    }

    /// Toggle a panel's visibility
    pub fn toggle_panel(&mut self, id: PanelId) {
        if let Some(panel) = self.panels.get_mut(&id) {
            panel.toggle();

            // Update layout visibility
            self.update_layout_visibility();
        }
    }

    /// Show a panel
    pub fn show_panel(&mut self, id: PanelId) {
        if let Some(panel) = self.panels.get_mut(&id) {
            panel.show();
            self.update_layout_visibility();
        }
    }

    /// Hide a panel
    pub fn hide_panel(&mut self, id: PanelId) {
        if let Some(panel) = self.panels.get_mut(&id) {
            panel.hide();

            // Clear focus if this was focused
            if self.focused == Some(id) {
                self.focused = None;
            }

            self.update_layout_visibility();
        }
    }

    /// Focus a panel
    pub fn focus_panel(&mut self, id: PanelId) {
        // Unfocus previous
        if let Some(prev_id) = self.focused {
            if let Some(panel) = self.panels.get_mut(&prev_id) {
                panel.unfocus();
            }
            self.focus_history.push(prev_id);
        }

        // Focus new
        if let Some(panel) = self.panels.get_mut(&id) {
            panel.focus();
            self.focused = Some(id);
        }

        // Limit history size
        if self.focus_history.len() > 20 {
            self.focus_history.remove(0);
        }
    }

    /// Get the currently focused panel
    pub fn focused(&self) -> Option<&Panel> {
        self.focused.and_then(|id| self.panels.get(&id))
    }

    /// Get the focused panel ID
    pub fn focused_id(&self) -> Option<PanelId> {
        self.focused
    }

    /// Focus next panel
    pub fn focus_next(&mut self) {
        let visible: Vec<PanelId> = self
            .panels
            .values()
            .filter(|p| p.visible)
            .map(|p| p.id)
            .collect();

        if visible.is_empty() {
            return;
        }

        let current_idx = self
            .focused
            .and_then(|id| visible.iter().position(|&v| v == id));

        let next_idx = match current_idx {
            Some(idx) => (idx + 1) % visible.len(),
            None => 0,
        };

        self.focus_panel(visible[next_idx]);
    }

    /// Focus previous panel
    pub fn focus_previous(&mut self) {
        let visible: Vec<PanelId> = self
            .panels
            .values()
            .filter(|p| p.visible)
            .map(|p| p.id)
            .collect();

        if visible.is_empty() {
            return;
        }

        let current_idx = self
            .focused
            .and_then(|id| visible.iter().position(|&v| v == id));

        let prev_idx = match current_idx {
            Some(0) => visible.len() - 1,
            Some(idx) => idx - 1,
            None => 0,
        };

        self.focus_panel(visible[prev_idx]);
    }

    /// Go back in focus history
    pub fn focus_back(&mut self) {
        if let Some(prev_id) = self.focus_history.pop() {
            if let Some(panel) = self.panels.get(&prev_id) {
                if panel.visible {
                    if let Some(current_id) = self.focused {
                        if let Some(current) = self.panels.get_mut(&current_id) {
                            current.unfocus();
                        }
                    }

                    if let Some(panel) = self.panels.get_mut(&prev_id) {
                        panel.focus();
                        self.focused = Some(prev_id);
                    }
                    return;
                }
            }

            // If the panel is not visible, try the next one
            self.focus_back();
        }
    }

    /// Set panel position
    pub fn set_panel_position(&mut self, id: PanelId, position: PanelPosition) {
        if let Some(panel) = self.panels.get_mut(&id) {
            panel.position = position;
            self.update_layout_visibility();
        }
    }

    /// Set panel size
    pub fn set_panel_size(&mut self, id: PanelId, size: PanelSize) {
        if let Some(panel) = self.panels.get_mut(&id) {
            panel.size = size;
        }
    }

    /// Resize panel width
    pub fn resize_panel_width(&mut self, id: PanelId, width: f32) {
        if let Some(panel) = self.panels.get_mut(&id) {
            panel.size.set_width(width);

            // Update layout
            match panel.position {
                PanelPosition::Left => self.layout.left_width = panel.size.width,
                PanelPosition::Right => self.layout.right_width = panel.size.width,
                _ => {}
            }
        }
    }

    /// Resize panel height
    pub fn resize_panel_height(&mut self, id: PanelId, height: f32) {
        if let Some(panel) = self.panels.get_mut(&id) {
            panel.size.set_height(height);

            // Update layout
            if panel.position == PanelPosition::Bottom {
                self.layout.bottom_height = panel.size.height;
            }
        }
    }

    /// Get the layout
    pub fn layout(&self) -> &PanelLayout {
        &self.layout
    }

    /// Get mutable layout
    pub fn layout_mut(&mut self) -> &mut PanelLayout {
        &mut self.layout
    }

    /// Get the config
    pub fn config(&self) -> &PanelConfig {
        &self.config
    }

    /// Get mutable config
    pub fn config_mut(&mut self) -> &mut PanelConfig {
        &mut self.config
    }

    /// Toggle left sidebar
    pub fn toggle_left_sidebar(&mut self) {
        self.layout.toggle_sidebar(PanelPosition::Left);
    }

    /// Toggle right sidebar
    pub fn toggle_right_sidebar(&mut self) {
        self.layout.toggle_sidebar(PanelPosition::Right);
    }

    /// Toggle bottom panel
    pub fn toggle_bottom_panel(&mut self) {
        self.layout.toggle_bottom();
    }

    /// Reset to default layout
    pub fn reset_layout(&mut self) {
        self.layout = PanelLayout::default();

        for id in PanelId::all() {
            if let Some(panel) = self.panels.get_mut(id) {
                panel.position = id.default_position();
                panel.visible = id.default_visible();
                panel.size = PanelSize::default();
                panel.collapsed = false;
            }
        }

        self.update_layout_visibility();
    }

    /// Update layout visibility based on panel states
    fn update_layout_visibility(&mut self) {
        self.layout.left_visible = self
            .panels
            .values()
            .any(|p| p.visible && p.position == PanelPosition::Left);

        self.layout.right_visible = self
            .panels
            .values()
            .any(|p| p.visible && p.position == PanelPosition::Right);

        self.layout.bottom_visible = self
            .panels
            .values()
            .any(|p| p.visible && p.position == PanelPosition::Bottom);
    }

    /// Get panel count
    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    /// Get visible panel count
    pub fn visible_count(&self) -> usize {
        self.panels.values().filter(|p| p.visible).count()
    }

    /// Check if a panel is visible
    pub fn is_visible(&self, id: PanelId) -> bool {
        self.panels.get(&id).map_or(false, |p| p.visible)
    }

    /// Check if a panel is focused
    pub fn is_focused(&self, id: PanelId) -> bool {
        self.focused == Some(id)
    }
}

impl Default for PanelManager {
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
    fn test_panel_id_all() {
        let all = PanelId::all();
        assert!(all.len() >= 4);
        assert!(all.contains(&PanelId::DeviceList));
        assert!(all.contains(&PanelId::Preview));
    }

    #[test]
    fn test_panel_id_display() {
        assert_eq!(PanelId::DeviceList.display_name(), "Devices");
        assert_eq!(PanelId::Preview.display_name(), "Preview");
    }

    #[test]
    fn test_panel_id_icon() {
        assert!(!PanelId::DeviceList.icon().is_empty());
        assert!(!PanelId::Preview.icon().is_empty());
    }

    #[test]
    fn test_panel_position_opposite() {
        assert_eq!(PanelPosition::Left.opposite(), PanelPosition::Right);
        assert_eq!(PanelPosition::Right.opposite(), PanelPosition::Left);
    }

    #[test]
    fn test_panel_position_is_sidebar() {
        assert!(PanelPosition::Left.is_sidebar());
        assert!(PanelPosition::Right.is_sidebar());
        assert!(!PanelPosition::Bottom.is_sidebar());
        assert!(!PanelPosition::Center.is_sidebar());
    }

    #[test]
    fn test_panel_size_default() {
        let size = PanelSize::default();
        assert!(size.width > 0.0);
        assert!(size.min_width > 0.0);
    }

    #[test]
    fn test_panel_size_clamp() {
        let size = PanelSize {
            width: 200.0,
            min_width: 100.0,
            max_width: 300.0,
            ..Default::default()
        };

        assert_eq!(size.clamp_width(50.0), 100.0);
        assert_eq!(size.clamp_width(200.0), 200.0);
        assert_eq!(size.clamp_width(400.0), 300.0);
    }

    #[test]
    fn test_panel_new() {
        let panel = Panel::new(PanelId::DeviceList);

        assert_eq!(panel.id, PanelId::DeviceList);
        assert!(panel.closable);
        assert!(panel.movable);
        assert!(panel.resizable);
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = Panel::new(PanelId::DeviceList);
        let initial = panel.visible;

        panel.toggle();
        assert_ne!(panel.visible, initial);

        panel.toggle();
        assert_eq!(panel.visible, initial);
    }

    #[test]
    fn test_panel_focus() {
        let mut panel = Panel::new(PanelId::DeviceList);
        panel.visible = false;

        panel.focus();
        assert!(panel.focused);
        assert!(panel.visible);
    }

    #[test]
    fn test_panel_collapse() {
        let mut panel = Panel::new(PanelId::DeviceList);

        panel.collapse();
        assert!(panel.collapsed);

        panel.expand();
        assert!(!panel.collapsed);

        panel.toggle_collapse();
        assert!(panel.collapsed);
    }

    #[test]
    fn test_panel_custom_data() {
        let mut panel = Panel::new(PanelId::DeviceList);

        panel.set_custom("key", "value");
        assert_eq!(panel.get_custom("key"), Some(&"value".to_string()));
        assert_eq!(panel.get_custom("nonexistent"), None);
    }

    #[test]
    fn test_panel_layout_default() {
        let layout = PanelLayout::default();
        assert!(layout.left_visible);
        assert!(layout.bottom_visible);
    }

    #[test]
    fn test_panel_layout_center_size() {
        let layout = PanelLayout {
            left_width: 200.0,
            right_width: 300.0,
            left_visible: true,
            right_visible: true,
            ..Default::default()
        };

        let center = layout.center_width(1000.0);
        assert_eq!(center, 500.0);
    }

    #[test]
    fn test_panel_layout_toggle() {
        let mut layout = PanelLayout::default();
        let initial = layout.left_visible;

        layout.toggle_sidebar(PanelPosition::Left);
        assert_ne!(layout.left_visible, initial);
    }

    #[test]
    fn test_panel_manager_new() {
        let manager = PanelManager::new();

        assert!(!manager.panels.is_empty());
        assert!(manager.get(PanelId::DeviceList).is_some());
    }

    #[test]
    fn test_panel_manager_toggle() {
        let mut manager = PanelManager::new();
        let initial = manager.is_visible(PanelId::DeviceList);

        manager.toggle_panel(PanelId::DeviceList);
        assert_ne!(manager.is_visible(PanelId::DeviceList), initial);
    }

    #[test]
    fn test_panel_manager_focus() {
        let mut manager = PanelManager::new();

        manager.focus_panel(PanelId::Preview);
        assert_eq!(manager.focused_id(), Some(PanelId::Preview));
        assert!(manager.is_focused(PanelId::Preview));
    }

    #[test]
    fn test_panel_manager_focus_navigation() {
        let mut manager = PanelManager::new();
        manager.show_panel(PanelId::DeviceList);
        manager.show_panel(PanelId::Preview);

        manager.focus_panel(PanelId::DeviceList);
        manager.focus_next();

        // Focus should have moved
        assert_ne!(manager.focused_id(), Some(PanelId::DeviceList));
    }

    #[test]
    fn test_panel_manager_visible_panels() {
        let mut manager = PanelManager::new();
        manager.show_panel(PanelId::DeviceList);
        manager.show_panel(PanelId::Preview);
        manager.hide_panel(PanelId::Log);

        let visible = manager.visible_panels();
        assert!(visible.iter().any(|p| p.id == PanelId::DeviceList));
        assert!(!visible.iter().any(|p| p.id == PanelId::Log));
    }

    #[test]
    fn test_panel_manager_panels_at() {
        let manager = PanelManager::new();

        let left_panels = manager.panels_at(PanelPosition::Left);
        for panel in left_panels {
            assert_eq!(panel.position, PanelPosition::Left);
        }
    }

    #[test]
    fn test_panel_manager_reset_layout() {
        let mut manager = PanelManager::new();

        // Modify some panels
        manager.hide_panel(PanelId::DeviceList);
        manager.set_panel_position(PanelId::Preview, PanelPosition::Right);

        // Reset
        manager.reset_layout();

        assert!(manager.is_visible(PanelId::DeviceList));
        assert_eq!(
            manager.get(PanelId::Preview).unwrap().position,
            PanelId::Preview.default_position()
        );
    }

    #[test]
    fn test_panel_serialization() {
        let panel = Panel::new(PanelId::DeviceList);
        let json = serde_json::to_string(&panel).unwrap();
        let deserialized: Panel = serde_json::from_str(&json).unwrap();

        assert_eq!(panel.id, deserialized.id);
        assert_eq!(panel.visible, deserialized.visible);
    }

    #[test]
    fn test_panel_layout_serialization() {
        let layout = PanelLayout::default();
        let json = serde_json::to_string(&layout).unwrap();
        let deserialized: PanelLayout = serde_json::from_str(&json).unwrap();

        assert_eq!(layout.left_width, deserialized.left_width);
    }
}

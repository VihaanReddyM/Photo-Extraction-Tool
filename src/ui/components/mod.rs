//! UI Components Module
//!
//! This module provides reusable UI components and widgets for building
//! the Photo Extraction Tool's user interface. Components are designed
//! to be framework-agnostic and can be used with any Rust UI framework.
//!
//! # Design Principles
//!
//! 1. **State Management** - Components maintain their own state that can be
//!    queried and modified through well-defined APIs
//! 2. **Framework Agnostic** - No dependency on specific UI frameworks
//! 3. **Accessibility** - Focus management and keyboard navigation built-in
//! 4. **Theming** - Components work with the theme system for consistent styling
//!
//! # Components
//!
//! - [`Widget`] - Base trait for all widgets
//! - [`FocusManager`] - Keyboard focus navigation
//! - [`ButtonState`] / [`ButtonVariant`] - Button components
//! - [`InputState`] - Text input fields
//! - [`ListState`] / [`ListItem`] - List/tree views
//! - [`ProgressState`] - Progress indicators

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for widgets
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WidgetId(pub String);

impl WidgetId {
    /// Create a new widget ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Create a unique widget ID with a prefix
    pub fn with_prefix(prefix: &str, id: impl std::fmt::Display) -> Self {
        Self(format!("{}_{}", prefix, id))
    }
}

impl std::fmt::Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for WidgetId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for WidgetId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Base trait for all UI widgets
pub trait Widget {
    /// Get the widget's unique identifier
    fn id(&self) -> &WidgetId;

    /// Check if the widget is enabled
    fn is_enabled(&self) -> bool;

    /// Check if the widget is visible
    fn is_visible(&self) -> bool;

    /// Check if the widget can receive focus
    fn is_focusable(&self) -> bool;

    /// Get the widget's tab order (lower = earlier)
    fn tab_order(&self) -> i32 {
        0
    }
}

/// Common state for all widgets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetState {
    /// Widget identifier
    pub id: WidgetId,

    /// Whether the widget is enabled
    pub enabled: bool,

    /// Whether the widget is visible
    pub visible: bool,

    /// Whether the widget can receive focus
    pub focusable: bool,

    /// Tab order for focus navigation
    pub tab_order: i32,

    /// Whether the widget currently has focus
    pub focused: bool,

    /// Whether the mouse is hovering over the widget
    pub hovered: bool,

    /// Custom data storage
    #[serde(skip)]
    custom_data: HashMap<String, String>,
}

impl WidgetState {
    /// Create a new widget state
    pub fn new(id: impl Into<WidgetId>) -> Self {
        Self {
            id: id.into(),
            enabled: true,
            visible: true,
            focusable: true,
            tab_order: 0,
            focused: false,
            hovered: false,
            custom_data: HashMap::new(),
        }
    }

    /// Set enabled state
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set visible state
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set focusable state
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// Set tab order
    pub fn tab_order(mut self, order: i32) -> Self {
        self.tab_order = order;
        self
    }

    /// Set focus state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Set hover state
    pub fn set_hovered(&mut self, hovered: bool) {
        self.hovered = hovered;
    }

    /// Store custom data
    pub fn set_custom(&mut self, key: &str, value: &str) {
        self.custom_data.insert(key.to_string(), value.to_string());
    }

    /// Retrieve custom data
    pub fn get_custom(&self, key: &str) -> Option<&str> {
        self.custom_data.get(key).map(|s| s.as_str())
    }
}

impl Widget for WidgetState {
    fn id(&self) -> &WidgetId {
        &self.id
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn is_focusable(&self) -> bool {
        self.focusable && self.enabled && self.visible
    }

    fn tab_order(&self) -> i32 {
        self.tab_order
    }
}

impl Default for WidgetState {
    fn default() -> Self {
        Self::new("default")
    }
}

// =============================================================================
// Focus Management
// =============================================================================

/// Direction for focus movement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    /// Move to next focusable widget
    Next,
    /// Move to previous focusable widget
    Previous,
    /// Move focus up
    Up,
    /// Move focus down
    Down,
    /// Move focus left
    Left,
    /// Move focus right
    Right,
    /// Move to first focusable widget
    First,
    /// Move to last focusable widget
    Last,
}

/// Manages keyboard focus across widgets
#[derive(Debug, Clone, Default)]
pub struct FocusManager {
    /// Currently focused widget ID
    focused: Option<WidgetId>,

    /// Ordered list of focusable widget IDs
    focus_order: Vec<WidgetId>,

    /// Focus history for back navigation
    focus_history: Vec<WidgetId>,

    /// Maximum history size
    max_history: usize,

    /// Whether focus wraps around at edges
    wrap_focus: bool,

    /// Whether to show focus indicators
    show_focus_ring: bool,
}

impl FocusManager {
    /// Create a new focus manager
    pub fn new() -> Self {
        Self {
            focused: None,
            focus_order: Vec::new(),
            focus_history: Vec::new(),
            max_history: 10,
            wrap_focus: true,
            show_focus_ring: true,
        }
    }

    /// Register a widget in the focus order
    pub fn register(&mut self, id: WidgetId, tab_order: i32) {
        // Remove if already exists
        self.focus_order.retain(|w| *w != id);

        // Find insertion point based on tab order
        let pos = self
            .focus_order
            .iter()
            .position(|_| false) // Will insert at end if no position found
            .unwrap_or(self.focus_order.len());

        // For now, just append (proper ordering would need tab_order stored)
        let _ = pos;
        let _ = tab_order;
        self.focus_order.push(id);
    }

    /// Unregister a widget from focus management
    pub fn unregister(&mut self, id: &WidgetId) {
        self.focus_order.retain(|w| w != id);
        if self.focused.as_ref() == Some(id) {
            self.focused = None;
        }
        self.focus_history.retain(|w| w != id);
    }

    /// Set focus to a specific widget
    pub fn set_focus(&mut self, id: WidgetId) {
        // Save current focus to history
        if let Some(current) = self.focused.take() {
            if current != id {
                self.focus_history.push(current);
                if self.focus_history.len() > self.max_history {
                    self.focus_history.remove(0);
                }
            }
        }

        self.focused = Some(id);
    }

    /// Clear focus
    pub fn clear_focus(&mut self) {
        if let Some(current) = self.focused.take() {
            self.focus_history.push(current);
            if self.focus_history.len() > self.max_history {
                self.focus_history.remove(0);
            }
        }
    }

    /// Get the currently focused widget ID
    pub fn focused(&self) -> Option<&WidgetId> {
        self.focused.as_ref()
    }

    /// Check if a specific widget has focus
    pub fn has_focus(&self, id: &WidgetId) -> bool {
        self.focused.as_ref() == Some(id)
    }

    /// Move focus in a direction
    pub fn move_focus(&mut self, direction: FocusDirection) -> Option<&WidgetId> {
        if self.focus_order.is_empty() {
            return None;
        }

        let current_idx = self
            .focused
            .as_ref()
            .and_then(|f| self.focus_order.iter().position(|w| w == f));

        let new_idx = match direction {
            FocusDirection::Next | FocusDirection::Down | FocusDirection::Right => {
                match current_idx {
                    Some(idx) => {
                        if idx + 1 < self.focus_order.len() {
                            Some(idx + 1)
                        } else if self.wrap_focus {
                            Some(0)
                        } else {
                            Some(idx)
                        }
                    }
                    None => Some(0),
                }
            }
            FocusDirection::Previous | FocusDirection::Up | FocusDirection::Left => {
                match current_idx {
                    Some(idx) => {
                        if idx > 0 {
                            Some(idx - 1)
                        } else if self.wrap_focus {
                            Some(self.focus_order.len() - 1)
                        } else {
                            Some(idx)
                        }
                    }
                    None => Some(self.focus_order.len() - 1),
                }
            }
            FocusDirection::First => Some(0),
            FocusDirection::Last => Some(self.focus_order.len() - 1),
        };

        if let Some(idx) = new_idx {
            let new_id = self.focus_order[idx].clone();
            self.set_focus(new_id);
        }

        self.focused.as_ref()
    }

    /// Go back to previously focused widget
    pub fn focus_back(&mut self) -> Option<&WidgetId> {
        if let Some(prev) = self.focus_history.pop() {
            self.focused = Some(prev);
        }
        self.focused.as_ref()
    }

    /// Set whether focus wraps around
    pub fn set_wrap_focus(&mut self, wrap: bool) {
        self.wrap_focus = wrap;
    }

    /// Set whether to show focus ring
    pub fn set_show_focus_ring(&mut self, show: bool) {
        self.show_focus_ring = show;
    }

    /// Check if focus ring should be shown
    pub fn should_show_focus_ring(&self) -> bool {
        self.show_focus_ring && self.focused.is_some()
    }

    /// Get focus order
    pub fn focus_order(&self) -> &[WidgetId] {
        &self.focus_order
    }

    /// Get number of focusable widgets
    pub fn focusable_count(&self) -> usize {
        self.focus_order.len()
    }
}

// =============================================================================
// Button Components
// =============================================================================

/// Button visual variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ButtonVariant {
    /// Default button style
    #[default]
    Default,
    /// Primary/accent colored button
    Primary,
    /// Secondary/muted button
    Secondary,
    /// Success/green button
    Success,
    /// Warning/yellow button
    Warning,
    /// Danger/red button
    Danger,
    /// Ghost/transparent button
    Ghost,
    /// Link-style button
    Link,
    /// Icon-only button
    Icon,
}

impl ButtonVariant {
    /// Get all variants
    pub fn all() -> &'static [ButtonVariant] {
        &[
            Self::Default,
            Self::Primary,
            Self::Secondary,
            Self::Success,
            Self::Warning,
            Self::Danger,
            Self::Ghost,
            Self::Link,
            Self::Icon,
        ]
    }
}

/// Button size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ButtonSize {
    /// Small button
    Small,
    /// Medium/default button
    #[default]
    Medium,
    /// Large button
    Large,
}

impl ButtonSize {
    /// Get height in pixels
    pub fn height(&self) -> u32 {
        match self {
            Self::Small => 24,
            Self::Medium => 32,
            Self::Large => 40,
        }
    }

    /// Get padding in pixels
    pub fn padding(&self) -> (u32, u32) {
        match self {
            Self::Small => (8, 4),
            Self::Medium => (12, 6),
            Self::Large => (16, 8),
        }
    }
}

/// Button state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonState {
    /// Base widget state
    pub widget: WidgetState,

    /// Button label text
    pub label: String,

    /// Button variant for styling
    pub variant: ButtonVariant,

    /// Button size
    pub size: ButtonSize,

    /// Optional icon name
    pub icon: Option<String>,

    /// Icon position (true = left, false = right)
    pub icon_left: bool,

    /// Whether the button is being pressed
    pub pressed: bool,

    /// Whether the button shows a loading state
    pub loading: bool,

    /// Optional tooltip text
    pub tooltip: Option<String>,

    /// Optional keyboard shortcut hint
    pub shortcut: Option<String>,
}

impl ButtonState {
    /// Create a new button state
    pub fn new(id: impl Into<WidgetId>, label: impl Into<String>) -> Self {
        Self {
            widget: WidgetState::new(id),
            label: label.into(),
            variant: ButtonVariant::Default,
            size: ButtonSize::Medium,
            icon: None,
            icon_left: true,
            pressed: false,
            loading: false,
            tooltip: None,
            shortcut: None,
        }
    }

    /// Set button variant
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set button size
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Set button icon
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set icon position to right
    pub fn icon_right(mut self) -> Self {
        self.icon_left = false;
        self
    }

    /// Set tooltip
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Set shortcut hint
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.widget.enabled = !disabled;
        self
    }

    /// Set pressed state
    pub fn set_pressed(&mut self, pressed: bool) {
        self.pressed = pressed;
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// Check if button can be clicked
    pub fn is_clickable(&self) -> bool {
        self.widget.enabled && !self.loading
    }
}

impl Widget for ButtonState {
    fn id(&self) -> &WidgetId {
        &self.widget.id
    }

    fn is_enabled(&self) -> bool {
        self.widget.enabled && !self.loading
    }

    fn is_visible(&self) -> bool {
        self.widget.visible
    }

    fn is_focusable(&self) -> bool {
        self.widget.is_focusable() && !self.loading
    }

    fn tab_order(&self) -> i32 {
        self.widget.tab_order
    }
}

impl Default for ButtonState {
    fn default() -> Self {
        Self::new("button", "Button")
    }
}

// =============================================================================
// Input Components
// =============================================================================

/// Text input type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InputType {
    /// Regular text input
    #[default]
    Text,
    /// Password input (masked)
    Password,
    /// Email input
    Email,
    /// Number input
    Number,
    /// Search input
    Search,
    /// URL input
    Url,
    /// Multi-line text area
    TextArea,
}

/// Text input state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputState {
    /// Base widget state
    pub widget: WidgetState,

    /// Input type
    pub input_type: InputType,

    /// Current text value
    pub value: String,

    /// Placeholder text
    pub placeholder: String,

    /// Label text
    pub label: Option<String>,

    /// Helper/description text
    pub helper_text: Option<String>,

    /// Error message
    pub error: Option<String>,

    /// Maximum length (0 = unlimited)
    pub max_length: usize,

    /// Cursor position
    pub cursor_position: usize,

    /// Selection start (None if no selection)
    pub selection_start: Option<usize>,

    /// Selection end
    pub selection_end: Option<usize>,

    /// Whether input is read-only
    pub read_only: bool,

    /// Whether input is required
    pub required: bool,

    /// Optional prefix text/icon
    pub prefix: Option<String>,

    /// Optional suffix text/icon
    pub suffix: Option<String>,

    /// Whether to show clear button
    pub clearable: bool,

    /// Whether password is visible (for password type)
    pub password_visible: bool,
}

impl InputState {
    /// Create a new input state
    pub fn new(id: impl Into<WidgetId>) -> Self {
        Self {
            widget: WidgetState::new(id),
            input_type: InputType::Text,
            value: String::new(),
            placeholder: String::new(),
            label: None,
            helper_text: None,
            error: None,
            max_length: 0,
            cursor_position: 0,
            selection_start: None,
            selection_end: None,
            read_only: false,
            required: false,
            prefix: None,
            suffix: None,
            clearable: false,
            password_visible: false,
        }
    }

    /// Set input type
    pub fn input_type(mut self, input_type: InputType) -> Self {
        self.input_type = input_type;
        self
    }

    /// Set placeholder text
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set helper text
    pub fn helper_text(mut self, text: impl Into<String>) -> Self {
        self.helper_text = Some(text.into());
        self
    }

    /// Set max length
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = max;
        self
    }

    /// Set required flag
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set clearable flag
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    /// Set the current value
    pub fn set_value(&mut self, value: impl Into<String>) {
        let value = value.into();
        if self.max_length > 0 && value.len() > self.max_length {
            self.value = value[..self.max_length].to_string();
        } else {
            self.value = value;
        }
        self.cursor_position = self.value.len();
        self.clear_selection();
    }

    /// Insert text at cursor
    pub fn insert(&mut self, text: &str) {
        // Delete selection if any
        self.delete_selection();

        let new_value = format!(
            "{}{}{}",
            &self.value[..self.cursor_position],
            text,
            &self.value[self.cursor_position..]
        );

        if self.max_length == 0 || new_value.len() <= self.max_length {
            self.value = new_value;
            self.cursor_position += text.len();
        }
    }

    /// Delete character before cursor
    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }

        if self.cursor_position > 0 {
            self.value = format!(
                "{}{}",
                &self.value[..self.cursor_position - 1],
                &self.value[self.cursor_position..]
            );
            self.cursor_position -= 1;
        }
    }

    /// Delete character after cursor
    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }

        if self.cursor_position < self.value.len() {
            self.value = format!(
                "{}{}",
                &self.value[..self.cursor_position],
                &self.value[self.cursor_position + 1..]
            );
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self, select: bool) {
        if self.cursor_position > 0 {
            if select {
                if self.selection_start.is_none() {
                    self.selection_start = Some(self.cursor_position);
                }
            } else {
                self.clear_selection();
            }
            self.cursor_position -= 1;
            if select {
                self.selection_end = Some(self.cursor_position);
            }
        }
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self, select: bool) {
        if self.cursor_position < self.value.len() {
            if select {
                if self.selection_start.is_none() {
                    self.selection_start = Some(self.cursor_position);
                }
            } else {
                self.clear_selection();
            }
            self.cursor_position += 1;
            if select {
                self.selection_end = Some(self.cursor_position);
            }
        }
    }

    /// Move cursor to start
    pub fn move_cursor_home(&mut self, select: bool) {
        if select && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_position);
        }
        self.cursor_position = 0;
        if select {
            self.selection_end = Some(self.cursor_position);
        } else {
            self.clear_selection();
        }
    }

    /// Move cursor to end
    pub fn move_cursor_end(&mut self, select: bool) {
        if select && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_position);
        }
        self.cursor_position = self.value.len();
        if select {
            self.selection_end = Some(self.cursor_position);
        } else {
            self.clear_selection();
        }
    }

    /// Select all text
    pub fn select_all(&mut self) {
        self.selection_start = Some(0);
        self.selection_end = Some(self.value.len());
        self.cursor_position = self.value.len();
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }

    /// Delete selected text, returns true if something was deleted
    pub fn delete_selection(&mut self) -> bool {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let (start, end) = if start < end {
                (start, end)
            } else {
                (end, start)
            };
            self.value = format!("{}{}", &self.value[..start], &self.value[end..]);
            self.cursor_position = start;
            self.clear_selection();
            true
        } else {
            false
        }
    }

    /// Get selected text
    pub fn selected_text(&self) -> Option<&str> {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let (start, end) = if start < end {
                (start, end)
            } else {
                (end, start)
            };
            Some(&self.value[start..end])
        } else {
            None
        }
    }

    /// Clear the input
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
        self.clear_selection();
        self.error = None;
    }

    /// Set error message
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }

    /// Clear error
    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// Check if input is empty
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Check if input has error
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    /// Toggle password visibility
    pub fn toggle_password_visibility(&mut self) {
        if self.input_type == InputType::Password {
            self.password_visible = !self.password_visible;
        }
    }

    /// Validate the input value
    pub fn validate(&mut self) -> bool {
        self.clear_error();

        if self.required && self.value.is_empty() {
            self.set_error("This field is required");
            return false;
        }

        if self.max_length > 0 && self.value.len() > self.max_length {
            self.set_error(format!("Maximum {} characters allowed", self.max_length));
            return false;
        }

        // Type-specific validation
        match self.input_type {
            InputType::Email => {
                if !self.value.is_empty() && !self.value.contains('@') {
                    self.set_error("Invalid email address");
                    return false;
                }
            }
            InputType::Number => {
                if !self.value.is_empty() && self.value.parse::<f64>().is_err() {
                    self.set_error("Invalid number");
                    return false;
                }
            }
            InputType::Url => {
                if !self.value.is_empty()
                    && !self.value.starts_with("http://")
                    && !self.value.starts_with("https://")
                {
                    self.set_error("Invalid URL");
                    return false;
                }
            }
            _ => {}
        }

        true
    }
}

impl Widget for InputState {
    fn id(&self) -> &WidgetId {
        &self.widget.id
    }

    fn is_enabled(&self) -> bool {
        self.widget.enabled && !self.read_only
    }

    fn is_visible(&self) -> bool {
        self.widget.visible
    }

    fn is_focusable(&self) -> bool {
        self.widget.is_focusable()
    }

    fn tab_order(&self) -> i32 {
        self.widget.tab_order
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new("input")
    }
}

// =============================================================================
// List Components
// =============================================================================

/// Single item in a list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    /// Item identifier
    pub id: String,

    /// Primary text
    pub label: String,

    /// Secondary/description text
    pub description: Option<String>,

    /// Icon name
    pub icon: Option<String>,

    /// Whether item is selected
    pub selected: bool,

    /// Whether item is expanded (for tree items)
    pub expanded: bool,

    /// Whether item has children (for tree items)
    pub has_children: bool,

    /// Nesting depth (for tree items)
    pub depth: usize,

    /// Whether item is enabled
    pub enabled: bool,

    /// Custom badge text
    pub badge: Option<String>,

    /// Custom metadata
    #[serde(skip)]
    pub metadata: HashMap<String, String>,
}

impl ListItem {
    /// Create a new list item
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            icon: None,
            selected: false,
            expanded: false,
            has_children: false,
            depth: 0,
            enabled: true,
            badge: None,
            metadata: HashMap::new(),
        }
    }

    /// Set description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set icon
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set badge
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Set depth for tree items
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// Set has children flag
    pub fn has_children(mut self, has: bool) -> Self {
        self.has_children = has;
        self
    }

    /// Set selected state
    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    /// Toggle expanded state
    pub fn toggle_expanded(&mut self) {
        if self.has_children {
            self.expanded = !self.expanded;
        }
    }

    /// Set metadata
    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }
}

impl Default for ListItem {
    fn default() -> Self {
        Self::new("item", "Item")
    }
}

/// Selection mode for lists
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SelectionMode {
    /// No selection allowed
    None,
    /// Single item selection
    #[default]
    Single,
    /// Multiple item selection
    Multiple,
}

/// List/tree state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListState {
    /// Base widget state
    pub widget: WidgetState,

    /// List items
    pub items: Vec<ListItem>,

    /// Currently focused item index
    pub focused_index: Option<usize>,

    /// Selection mode
    pub selection_mode: SelectionMode,

    /// Whether to show icons
    pub show_icons: bool,

    /// Whether to show descriptions
    pub show_descriptions: bool,

    /// Whether list is virtualized (for large lists)
    pub virtualized: bool,

    /// Visible range start (for virtualized lists)
    pub visible_start: usize,

    /// Visible range end (for virtualized lists)
    pub visible_end: usize,

    /// Item height in pixels (for virtualization)
    pub item_height: u32,

    /// Whether empty state should show
    pub show_empty_state: bool,

    /// Empty state message
    pub empty_message: String,

    /// Filter query
    pub filter_query: String,
}

impl ListState {
    /// Create a new list state
    pub fn new(id: impl Into<WidgetId>) -> Self {
        Self {
            widget: WidgetState::new(id),
            items: Vec::new(),
            focused_index: None,
            selection_mode: SelectionMode::Single,
            show_icons: true,
            show_descriptions: false,
            virtualized: false,
            visible_start: 0,
            visible_end: 0,
            item_height: 32,
            show_empty_state: true,
            empty_message: "No items".to_string(),
            filter_query: String::new(),
        }
    }

    /// Set selection mode
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Set show icons
    pub fn show_icons(mut self, show: bool) -> Self {
        self.show_icons = show;
        self
    }

    /// Set show descriptions
    pub fn show_descriptions(mut self, show: bool) -> Self {
        self.show_descriptions = show;
        self
    }

    /// Set virtualized
    pub fn virtualized(mut self, virtualized: bool) -> Self {
        self.virtualized = virtualized;
        self
    }

    /// Set empty message
    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = message.into();
        self
    }

    /// Add an item
    pub fn add_item(&mut self, item: ListItem) {
        self.items.push(item);
    }

    /// Add multiple items
    pub fn add_items(&mut self, items: impl IntoIterator<Item = ListItem>) {
        self.items.extend(items);
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.items.clear();
        self.focused_index = None;
    }

    /// Get item by index
    pub fn get(&self, index: usize) -> Option<&ListItem> {
        self.items.get(index)
    }

    /// Get mutable item by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ListItem> {
        self.items.get_mut(index)
    }

    /// Get item by ID
    pub fn get_by_id(&self, id: &str) -> Option<&ListItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Find item index by ID
    pub fn find_index(&self, id: &str) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }

    /// Select item by index
    pub fn select(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }

        match self.selection_mode {
            SelectionMode::None => {}
            SelectionMode::Single => {
                for item in &mut self.items {
                    item.selected = false;
                }
                self.items[index].selected = true;
            }
            SelectionMode::Multiple => {
                self.items[index].selected = !self.items[index].selected;
            }
        }

        self.focused_index = Some(index);
    }

    /// Select item by ID
    pub fn select_by_id(&mut self, id: &str) {
        if let Some(index) = self.find_index(id) {
            self.select(index);
        }
    }

    /// Select all items (only in multiple selection mode)
    pub fn select_all(&mut self) {
        if self.selection_mode == SelectionMode::Multiple {
            for item in &mut self.items {
                if item.enabled {
                    item.selected = true;
                }
            }
        }
    }

    /// Deselect all items
    pub fn deselect_all(&mut self) {
        for item in &mut self.items {
            item.selected = false;
        }
    }

    /// Get selected items
    pub fn selected(&self) -> Vec<&ListItem> {
        self.items.iter().filter(|item| item.selected).collect()
    }

    /// Get selected item indices
    pub fn selected_indices(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| if item.selected { Some(i) } else { None })
            .collect()
    }

    /// Get selected item IDs
    pub fn selected_ids(&self) -> Vec<&str> {
        self.items
            .iter()
            .filter_map(|item| {
                if item.selected {
                    Some(item.id.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Focus next item
    pub fn focus_next(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let new_index = match self.focused_index {
            Some(idx) if idx + 1 < self.items.len() => idx + 1,
            Some(idx) => idx,
            None => 0,
        };

        self.focused_index = Some(new_index);
    }

    /// Focus previous item
    pub fn focus_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let new_index = match self.focused_index {
            Some(idx) if idx > 0 => idx - 1,
            Some(idx) => idx,
            None => self.items.len() - 1,
        };

        self.focused_index = Some(new_index);
    }

    /// Focus first item
    pub fn focus_first(&mut self) {
        if !self.items.is_empty() {
            self.focused_index = Some(0);
        }
    }

    /// Focus last item
    pub fn focus_last(&mut self) {
        if !self.items.is_empty() {
            self.focused_index = Some(self.items.len() - 1);
        }
    }

    /// Select focused item
    pub fn select_focused(&mut self) {
        if let Some(index) = self.focused_index {
            self.select(index);
        }
    }

    /// Toggle expansion of focused item
    pub fn toggle_focused_expansion(&mut self) {
        if let Some(index) = self.focused_index {
            if let Some(item) = self.items.get_mut(index) {
                item.toggle_expanded();
            }
        }
    }

    /// Set filter query
    pub fn set_filter(&mut self, query: impl Into<String>) {
        self.filter_query = query.into();
    }

    /// Get filtered items
    pub fn filtered_items(&self) -> Vec<&ListItem> {
        if self.filter_query.is_empty() {
            self.items.iter().collect()
        } else {
            let query = self.filter_query.to_lowercase();
            self.items
                .iter()
                .filter(|item| item.label.to_lowercase().contains(&query))
                .collect()
        }
    }

    /// Update visible range for virtualization
    pub fn update_visible_range(&mut self, scroll_offset: u32, viewport_height: u32) {
        if !self.virtualized || self.item_height == 0 {
            self.visible_start = 0;
            self.visible_end = self.items.len();
            return;
        }

        self.visible_start = (scroll_offset / self.item_height) as usize;
        self.visible_end = ((scroll_offset + viewport_height) / self.item_height + 1) as usize;
        self.visible_end = self.visible_end.min(self.items.len());
    }

    /// Get visible items (for virtualized rendering)
    pub fn visible_items(&self) -> &[ListItem] {
        &self.items[self.visible_start..self.visible_end.min(self.items.len())]
    }

    /// Get total content height (for scroll calculation)
    pub fn content_height(&self) -> u32 {
        self.items.len() as u32 * self.item_height
    }

    /// Get number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if list is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Widget for ListState {
    fn id(&self) -> &WidgetId {
        &self.widget.id
    }

    fn is_enabled(&self) -> bool {
        self.widget.enabled
    }

    fn is_visible(&self) -> bool {
        self.widget.visible
    }

    fn is_focusable(&self) -> bool {
        self.widget.is_focusable()
    }

    fn tab_order(&self) -> i32 {
        self.widget.tab_order
    }
}

impl Default for ListState {
    fn default() -> Self {
        Self::new("list")
    }
}

// =============================================================================
// Progress Components
// =============================================================================

/// Progress display style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProgressStyle {
    /// Linear progress bar
    #[default]
    Linear,
    /// Circular/ring progress
    Circular,
    /// Indeterminate loading spinner
    Spinner,
}

/// Progress status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProgressStatus {
    /// Progress is idle/not started
    Idle,
    /// Progress is active
    #[default]
    Active,
    /// Progress is paused
    Paused,
    /// Progress completed successfully
    Success,
    /// Progress failed with error
    Error,
}

impl ProgressStatus {
    /// Check if progress is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Success | Self::Error)
    }

    /// Check if progress is active
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// Progress indicator state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressState {
    /// Base widget state
    pub widget: WidgetState,

    /// Progress style
    pub style: ProgressStyle,

    /// Progress status
    pub status: ProgressStatus,

    /// Current progress value (0.0 to 1.0)
    pub value: f32,

    /// Whether progress is indeterminate
    pub indeterminate: bool,

    /// Primary label text
    pub label: Option<String>,

    /// Secondary/description text
    pub description: Option<String>,

    /// Show percentage text
    pub show_percentage: bool,

    /// Current item count
    pub current: u64,

    /// Total item count
    pub total: u64,

    /// Bytes processed (for file operations)
    pub bytes_processed: u64,

    /// Total bytes (for file operations)
    pub bytes_total: u64,

    /// Processing speed (bytes per second)
    pub speed: f64,

    /// Estimated time remaining in seconds
    pub eta: Option<f64>,

    /// Start time (unix timestamp)
    pub start_time: Option<u64>,

    /// Elapsed time in seconds
    pub elapsed: f64,

    /// Error message if status is Error
    pub error_message: Option<String>,
}

impl ProgressState {
    /// Create a new progress state
    pub fn new(id: impl Into<WidgetId>) -> Self {
        Self {
            widget: WidgetState::new(id),
            style: ProgressStyle::Linear,
            status: ProgressStatus::Idle,
            value: 0.0,
            indeterminate: false,
            label: None,
            description: None,
            show_percentage: true,
            current: 0,
            total: 0,
            bytes_processed: 0,
            bytes_total: 0,
            speed: 0.0,
            eta: None,
            start_time: None,
            elapsed: 0.0,
            error_message: None,
        }
    }

    /// Set progress style
    pub fn style(mut self, style: ProgressStyle) -> Self {
        self.style = style;
        self
    }

    /// Set indeterminate mode
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    /// Set label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set show percentage
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    /// Start progress
    pub fn start(&mut self) {
        self.status = ProgressStatus::Active;
        self.value = 0.0;
        self.current = 0;
        self.bytes_processed = 0;
        self.elapsed = 0.0;
        self.error_message = None;
        self.start_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
    }

    /// Update progress
    pub fn update(&mut self, current: u64, total: u64) {
        self.current = current;
        self.total = total;
        self.value = if total > 0 {
            (current as f32 / total as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    /// Update bytes progress
    pub fn update_bytes(&mut self, processed: u64, total: u64) {
        self.bytes_processed = processed;
        self.bytes_total = total;
    }

    /// Update speed and ETA
    pub fn update_timing(&mut self, speed: f64, eta: Option<f64>, elapsed: f64) {
        self.speed = speed;
        self.eta = eta;
        self.elapsed = elapsed;
    }

    /// Set description
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = Some(description.into());
    }

    /// Pause progress
    pub fn pause(&mut self) {
        if self.status == ProgressStatus::Active {
            self.status = ProgressStatus::Paused;
        }
    }

    /// Resume progress
    pub fn resume(&mut self) {
        if self.status == ProgressStatus::Paused {
            self.status = ProgressStatus::Active;
        }
    }

    /// Complete progress successfully
    pub fn complete(&mut self) {
        self.status = ProgressStatus::Success;
        self.value = 1.0;
        self.current = self.total;
        self.bytes_processed = self.bytes_total;
    }

    /// Fail progress with error
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = ProgressStatus::Error;
        self.error_message = Some(error.into());
    }

    /// Reset progress
    pub fn reset(&mut self) {
        self.status = ProgressStatus::Idle;
        self.value = 0.0;
        self.current = 0;
        self.total = 0;
        self.bytes_processed = 0;
        self.bytes_total = 0;
        self.speed = 0.0;
        self.eta = None;
        self.start_time = None;
        self.elapsed = 0.0;
        self.error_message = None;
        self.description = None;
    }

    /// Get percentage as integer (0-100)
    pub fn percentage(&self) -> u8 {
        (self.value * 100.0).round() as u8
    }

    /// Format bytes processed string
    pub fn bytes_progress_string(&self) -> String {
        format!(
            "{} / {}",
            format_bytes(self.bytes_processed),
            format_bytes(self.bytes_total)
        )
    }

    /// Format items progress string
    pub fn items_progress_string(&self) -> String {
        format!("{} / {}", self.current, self.total)
    }

    /// Format speed string
    pub fn speed_string(&self) -> String {
        format!("{}/s", format_bytes(self.speed as u64))
    }

    /// Format ETA string
    pub fn eta_string(&self) -> String {
        match self.eta {
            Some(secs) if secs < 60.0 => format!("{}s", secs.round() as u64),
            Some(secs) if secs < 3600.0 => {
                format!("{}m {}s", (secs / 60.0) as u64, (secs % 60.0) as u64)
            }
            Some(secs) => format!(
                "{}h {}m",
                (secs / 3600.0) as u64,
                ((secs % 3600.0) / 60.0) as u64
            ),
            None => "calculating...".to_string(),
        }
    }

    /// Format elapsed time string
    pub fn elapsed_string(&self) -> String {
        let secs = self.elapsed;
        if secs < 60.0 {
            format!("{}s", secs.round() as u64)
        } else if secs < 3600.0 {
            format!("{}m {}s", (secs / 60.0) as u64, (secs % 60.0) as u64)
        } else {
            format!(
                "{}h {}m",
                (secs / 3600.0) as u64,
                ((secs % 3600.0) / 60.0) as u64
            )
        }
    }
}

impl Widget for ProgressState {
    fn id(&self) -> &WidgetId {
        &self.widget.id
    }

    fn is_enabled(&self) -> bool {
        self.widget.enabled
    }

    fn is_visible(&self) -> bool {
        self.widget.visible
    }

    fn is_focusable(&self) -> bool {
        false // Progress indicators are not focusable
    }
}

impl Default for ProgressState {
    fn default() -> Self {
        Self::new("progress")
    }
}

/// Format bytes into human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Widget ID tests
    #[test]
    fn test_widget_id_new() {
        let id = WidgetId::new("test");
        assert_eq!(id.0, "test");
    }

    #[test]
    fn test_widget_id_with_prefix() {
        let id = WidgetId::with_prefix("btn", 1);
        assert_eq!(id.0, "btn_1");
    }

    #[test]
    fn test_widget_id_from_string() {
        let id: WidgetId = "test".into();
        assert_eq!(id.0, "test");
    }

    // Widget state tests
    #[test]
    fn test_widget_state_new() {
        let state = WidgetState::new("widget1");
        assert_eq!(state.id.0, "widget1");
        assert!(state.enabled);
        assert!(state.visible);
        assert!(state.focusable);
    }

    #[test]
    fn test_widget_state_builder() {
        let state = WidgetState::new("widget1")
            .enabled(false)
            .visible(false)
            .focusable(false)
            .tab_order(5);

        assert!(!state.enabled);
        assert!(!state.visible);
        assert!(!state.focusable);
        assert_eq!(state.tab_order, 5);
    }

    #[test]
    fn test_widget_state_focus() {
        let mut state = WidgetState::new("widget1");
        assert!(!state.focused);
        state.set_focused(true);
        assert!(state.focused);
    }

    #[test]
    fn test_widget_state_custom_data() {
        let mut state = WidgetState::new("widget1");
        state.set_custom("key", "value");
        assert_eq!(state.get_custom("key"), Some("value"));
        assert_eq!(state.get_custom("missing"), None);
    }

    // Focus manager tests
    #[test]
    fn test_focus_manager_new() {
        let fm = FocusManager::new();
        assert!(fm.focused().is_none());
        assert_eq!(fm.focusable_count(), 0);
    }

    #[test]
    fn test_focus_manager_register() {
        let mut fm = FocusManager::new();
        fm.register(WidgetId::new("btn1"), 0);
        fm.register(WidgetId::new("btn2"), 1);
        assert_eq!(fm.focusable_count(), 2);
    }

    #[test]
    fn test_focus_manager_set_focus() {
        let mut fm = FocusManager::new();
        fm.register(WidgetId::new("btn1"), 0);
        fm.set_focus(WidgetId::new("btn1"));
        assert!(fm.has_focus(&WidgetId::new("btn1")));
    }

    #[test]
    fn test_focus_manager_move_focus() {
        let mut fm = FocusManager::new();
        fm.register(WidgetId::new("btn1"), 0);
        fm.register(WidgetId::new("btn2"), 1);
        fm.register(WidgetId::new("btn3"), 2);

        fm.set_focus(WidgetId::new("btn1"));
        fm.move_focus(FocusDirection::Next);
        assert!(fm.has_focus(&WidgetId::new("btn2")));

        fm.move_focus(FocusDirection::Previous);
        assert!(fm.has_focus(&WidgetId::new("btn1")));
    }

    #[test]
    fn test_focus_manager_wrap() {
        let mut fm = FocusManager::new();
        fm.set_wrap_focus(true);
        fm.register(WidgetId::new("btn1"), 0);
        fm.register(WidgetId::new("btn2"), 1);

        fm.set_focus(WidgetId::new("btn2"));
        fm.move_focus(FocusDirection::Next);
        assert!(fm.has_focus(&WidgetId::new("btn1"))); // Wrapped around
    }

    #[test]
    fn test_focus_manager_history() {
        let mut fm = FocusManager::new();
        fm.register(WidgetId::new("btn1"), 0);
        fm.register(WidgetId::new("btn2"), 1);

        fm.set_focus(WidgetId::new("btn1"));
        fm.set_focus(WidgetId::new("btn2"));
        fm.focus_back();
        assert!(fm.has_focus(&WidgetId::new("btn1")));
    }

    // Button tests
    #[test]
    fn test_button_state_new() {
        let btn = ButtonState::new("btn1", "Click Me");
        assert_eq!(btn.label, "Click Me");
        assert!(btn.is_enabled());
        assert!(btn.is_clickable());
    }

    #[test]
    fn test_button_state_builder() {
        let btn = ButtonState::new("btn1", "Submit")
            .variant(ButtonVariant::Primary)
            .size(ButtonSize::Large)
            .icon("check")
            .tooltip("Submit the form");

        assert_eq!(btn.variant, ButtonVariant::Primary);
        assert_eq!(btn.size, ButtonSize::Large);
        assert_eq!(btn.icon, Some("check".to_string()));
        assert_eq!(btn.tooltip, Some("Submit the form".to_string()));
    }

    #[test]
    fn test_button_state_loading() {
        let mut btn = ButtonState::new("btn1", "Submit");
        assert!(btn.is_clickable());

        btn.set_loading(true);
        assert!(!btn.is_clickable());
    }

    #[test]
    fn test_button_size() {
        assert_eq!(ButtonSize::Small.height(), 24);
        assert_eq!(ButtonSize::Medium.height(), 32);
        assert_eq!(ButtonSize::Large.height(), 40);
    }

    // Input tests
    #[test]
    fn test_input_state_new() {
        let input = InputState::new("input1");
        assert!(input.value.is_empty());
        assert!(input.is_enabled());
    }

    #[test]
    fn test_input_state_set_value() {
        let mut input = InputState::new("input1");
        input.set_value("Hello");
        assert_eq!(input.value, "Hello");
        assert_eq!(input.cursor_position, 5);
    }

    #[test]
    fn test_input_state_insert() {
        let mut input = InputState::new("input1");
        input.insert("Hello");
        input.insert(" World");
        assert_eq!(input.value, "Hello World");
    }

    #[test]
    fn test_input_state_backspace() {
        let mut input = InputState::new("input1");
        input.set_value("Hello");
        input.backspace();
        assert_eq!(input.value, "Hell");
    }

    #[test]
    fn test_input_state_selection() {
        let mut input = InputState::new("input1");
        input.set_value("Hello World");
        input.select_all();
        assert_eq!(input.selected_text(), Some("Hello World"));
    }

    #[test]
    fn test_input_state_max_length() {
        let mut input = InputState::new("input1").max_length(5);
        input.set_value("Hello World");
        assert_eq!(input.value, "Hello");
    }

    #[test]
    fn test_input_state_validation() {
        let mut input = InputState::new("input1")
            .input_type(InputType::Email)
            .required(true);

        assert!(!input.validate()); // Empty and required

        input.set_value("not-an-email");
        assert!(!input.validate()); // Invalid email

        input.set_value("test@example.com");
        assert!(input.validate()); // Valid email
    }

    // List tests
    #[test]
    fn test_list_state_new() {
        let list = ListState::new("list1");
        assert!(list.is_empty());
    }

    #[test]
    fn test_list_state_add_items() {
        let mut list = ListState::new("list1");
        list.add_item(ListItem::new("1", "Item 1"));
        list.add_item(ListItem::new("2", "Item 2"));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_list_state_selection() {
        let mut list = ListState::new("list1").selection_mode(SelectionMode::Single);
        list.add_item(ListItem::new("1", "Item 1"));
        list.add_item(ListItem::new("2", "Item 2"));

        list.select(0);
        assert_eq!(list.selected().len(), 1);
        assert!(list.get(0).unwrap().selected);

        list.select(1);
        assert_eq!(list.selected().len(), 1);
        assert!(!list.get(0).unwrap().selected);
        assert!(list.get(1).unwrap().selected);
    }

    #[test]
    fn test_list_state_multiple_selection() {
        let mut list = ListState::new("list1").selection_mode(SelectionMode::Multiple);
        list.add_item(ListItem::new("1", "Item 1"));
        list.add_item(ListItem::new("2", "Item 2"));
        list.add_item(ListItem::new("3", "Item 3"));

        list.select(0);
        list.select(2);
        assert_eq!(list.selected().len(), 2);
        assert_eq!(list.selected_ids(), vec!["1", "3"]);
    }

    #[test]
    fn test_list_state_focus_navigation() {
        let mut list = ListState::new("list1");
        list.add_item(ListItem::new("1", "Item 1"));
        list.add_item(ListItem::new("2", "Item 2"));
        list.add_item(ListItem::new("3", "Item 3"));

        list.focus_first();
        assert_eq!(list.focused_index, Some(0));

        list.focus_next();
        assert_eq!(list.focused_index, Some(1));

        list.focus_last();
        assert_eq!(list.focused_index, Some(2));

        list.focus_previous();
        assert_eq!(list.focused_index, Some(1));
    }

    #[test]
    fn test_list_state_filter() {
        let mut list = ListState::new("list1");
        list.add_item(ListItem::new("1", "Apple"));
        list.add_item(ListItem::new("2", "Banana"));
        list.add_item(ListItem::new("3", "Apricot"));

        list.set_filter("ap");
        let filtered = list.filtered_items();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_list_item_builder() {
        let item = ListItem::new("id1", "Test Item")
            .description("A test item")
            .icon("folder")
            .badge("3")
            .depth(2)
            .has_children(true);

        assert_eq!(item.description, Some("A test item".to_string()));
        assert_eq!(item.icon, Some("folder".to_string()));
        assert_eq!(item.badge, Some("3".to_string()));
        assert_eq!(item.depth, 2);
        assert!(item.has_children);
    }

    // Progress tests
    #[test]
    fn test_progress_state_new() {
        let progress = ProgressState::new("progress1");
        assert_eq!(progress.value, 0.0);
        assert_eq!(progress.status, ProgressStatus::Idle);
    }

    #[test]
    fn test_progress_state_update() {
        let mut progress = ProgressState::new("progress1");
        progress.start();
        assert_eq!(progress.status, ProgressStatus::Active);

        progress.update(50, 100);
        assert_eq!(progress.value, 0.5);
        assert_eq!(progress.percentage(), 50);
    }

    #[test]
    fn test_progress_state_complete() {
        let mut progress = ProgressState::new("progress1");
        progress.start();
        progress.update(100, 100);
        progress.complete();

        assert_eq!(progress.status, ProgressStatus::Success);
        assert_eq!(progress.value, 1.0);
    }

    #[test]
    fn test_progress_state_fail() {
        let mut progress = ProgressState::new("progress1");
        progress.start();
        progress.fail("Something went wrong");

        assert_eq!(progress.status, ProgressStatus::Error);
        assert_eq!(
            progress.error_message,
            Some("Something went wrong".to_string())
        );
    }

    #[test]
    fn test_progress_state_pause_resume() {
        let mut progress = ProgressState::new("progress1");
        progress.start();
        assert_eq!(progress.status, ProgressStatus::Active);

        progress.pause();
        assert_eq!(progress.status, ProgressStatus::Paused);

        progress.resume();
        assert_eq!(progress.status, ProgressStatus::Active);
    }

    #[test]
    fn test_progress_state_timing() {
        let mut progress = ProgressState::new("progress1");
        progress.update_timing(1024.0 * 1024.0, Some(120.0), 60.0);

        assert_eq!(progress.speed_string(), "1.00 MB/s");
        assert_eq!(progress.eta_string(), "2m 0s");
        assert_eq!(progress.elapsed_string(), "1m 0s");
    }

    #[test]
    fn test_progress_state_bytes() {
        let mut progress = ProgressState::new("progress1");
        progress.update_bytes(500 * 1024 * 1024, 1024 * 1024 * 1024);

        assert_eq!(progress.bytes_progress_string(), "500.00 MB / 1.00 GB");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_bytes(1024u64 * 1024 * 1024 * 1024), "1.00 TB");
    }

    #[test]
    fn test_progress_status_terminal() {
        assert!(!ProgressStatus::Idle.is_terminal());
        assert!(!ProgressStatus::Active.is_terminal());
        assert!(!ProgressStatus::Paused.is_terminal());
        assert!(ProgressStatus::Success.is_terminal());
        assert!(ProgressStatus::Error.is_terminal());
    }

    // Serialization tests
    #[test]
    fn test_widget_state_serialization() {
        let state = WidgetState::new("test")
            .enabled(true)
            .visible(true)
            .tab_order(5);

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: WidgetState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id.0, "test");
        assert!(deserialized.enabled);
        assert_eq!(deserialized.tab_order, 5);
    }

    #[test]
    fn test_button_state_serialization() {
        let btn = ButtonState::new("btn1", "Click")
            .variant(ButtonVariant::Primary)
            .size(ButtonSize::Large);

        let json = serde_json::to_string(&btn).unwrap();
        let deserialized: ButtonState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.label, "Click");
        assert_eq!(deserialized.variant, ButtonVariant::Primary);
        assert_eq!(deserialized.size, ButtonSize::Large);
    }

    #[test]
    fn test_list_item_serialization() {
        let item = ListItem::new("id1", "Test")
            .description("Description")
            .icon("folder");

        let json = serde_json::to_string(&item).unwrap();
        let deserialized: ListItem = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "id1");
        assert_eq!(deserialized.label, "Test");
        assert_eq!(deserialized.description, Some("Description".to_string()));
    }
}

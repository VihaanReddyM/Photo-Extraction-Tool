//! Keybinding Registry - Storage and Lookup for Keybindings
//!
//! This module provides the registry for managing keybindings:
//! - Storage of keybindings by context
//! - Lookup and matching of key sequences
//! - Default binding registration
//! - Conflict detection
//!
//! # Example
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::keybindings::{
//!     KeybindingRegistry, KeyBinding, KeyCode, Modifiers, Action, KeybindingContext,
//! };
//!
//! let mut registry = KeybindingRegistry::new();
//! registry.register_defaults();
//!
//! // Add custom binding
//! registry.register(
//!     KeyBinding::new(KeyCode::Char('e'), Modifiers::CTRL)
//!         .action(Action::StartExtraction)
//! );
//! ```

use super::actions::Action;
use super::keys::{KeyCode, KeyCombination, KeySequence, Modifiers};
use super::KeyBinding;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// KeybindingContext
// =============================================================================

/// Context in which keybindings are active
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum KeybindingContext {
    /// Global context - always active
    #[default]
    Global,

    /// Device list is focused
    DeviceList,

    /// Preview panel is focused
    Preview,

    /// Progress panel is focused
    Progress,

    /// Command palette is open
    CommandPalette,

    /// Modal dialog is open
    Modal,

    /// Settings view is open
    Settings,

    /// Text input is focused
    TextInput,

    /// During extraction
    Extracting,

    /// Custom context
    Custom(String),
}

impl KeybindingContext {
    /// Check if this context is compatible with another
    ///
    /// A binding in a more specific context takes precedence over Global
    pub fn is_compatible_with(&self, other: &KeybindingContext) -> bool {
        match (self, other) {
            // Global is compatible with everything
            (KeybindingContext::Global, _) => true,
            (_, KeybindingContext::Global) => true,
            // Same context is compatible
            (a, b) if a == b => true,
            // Different non-global contexts are not compatible
            _ => false,
        }
    }

    /// Check if this is the global context
    pub fn is_global(&self) -> bool {
        matches!(self, KeybindingContext::Global)
    }

    /// Get the display name
    pub fn display_name(&self) -> &str {
        match self {
            KeybindingContext::Global => "Global",
            KeybindingContext::DeviceList => "Device List",
            KeybindingContext::Preview => "Preview",
            KeybindingContext::Progress => "Progress",
            KeybindingContext::CommandPalette => "Command Palette",
            KeybindingContext::Modal => "Modal",
            KeybindingContext::Settings => "Settings",
            KeybindingContext::TextInput => "Text Input",
            KeybindingContext::Extracting => "Extracting",
            KeybindingContext::Custom(name) => name,
        }
    }
}

impl std::fmt::Display for KeybindingContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// =============================================================================
// KeybindingEntry
// =============================================================================

/// An entry in the keybinding registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingEntry {
    /// The binding itself
    pub binding: KeyBinding,

    /// Whether this is a user-defined binding (vs default)
    #[serde(default)]
    pub is_user_defined: bool,

    /// Source of the binding (e.g., "defaults", "user", "plugin")
    #[serde(default)]
    pub source: String,
}

impl KeybindingEntry {
    /// Create a new entry from a binding
    pub fn new(binding: KeyBinding) -> Self {
        Self {
            binding,
            is_user_defined: false,
            source: "defaults".to_string(),
        }
    }

    /// Create a user-defined entry
    pub fn user_defined(binding: KeyBinding) -> Self {
        Self {
            binding,
            is_user_defined: true,
            source: "user".to_string(),
        }
    }

    /// Mark as user-defined
    pub fn as_user_defined(mut self) -> Self {
        self.is_user_defined = true;
        self.source = "user".to_string();
        self
    }

    /// Set the source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }
}

// =============================================================================
// KeybindingSet
// =============================================================================

/// A named set of keybindings (e.g., "default", "vim", "emacs")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingSet {
    /// Name of the set
    pub name: String,

    /// Description of the set
    pub description: String,

    /// The keybindings in this set
    pub bindings: Vec<KeyBinding>,
}

impl KeybindingSet {
    /// Create a new keybinding set
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            bindings: Vec::new(),
        }
    }

    /// Set the description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a binding
    pub fn add(mut self, binding: KeyBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Add multiple bindings
    pub fn add_all(mut self, bindings: impl IntoIterator<Item = KeyBinding>) -> Self {
        self.bindings.extend(bindings);
        self
    }
}

impl Default for KeybindingSet {
    fn default() -> Self {
        Self::new("default")
    }
}

// =============================================================================
// BindingMatch
// =============================================================================

/// Result of matching a key sequence against bindings
#[derive(Debug, Clone)]
pub enum BindingMatch {
    /// No bindings matched
    None,

    /// Partial match - the sequence is a prefix of some bindings
    Partial(usize),

    /// Exact match found
    Exact(KeyBinding),

    /// Multiple exact matches (conflict)
    Multiple(Vec<KeyBinding>),
}

impl BindingMatch {
    /// Check if there was any kind of match
    pub fn is_match(&self) -> bool {
        !matches!(self, BindingMatch::None)
    }

    /// Check if this is an exact match
    pub fn is_exact(&self) -> bool {
        matches!(self, BindingMatch::Exact(_) | BindingMatch::Multiple(_))
    }

    /// Check if this is a partial match
    pub fn is_partial(&self) -> bool {
        matches!(self, BindingMatch::Partial(_))
    }

    /// Get the binding if this is an exact single match
    pub fn binding(&self) -> Option<&KeyBinding> {
        match self {
            BindingMatch::Exact(b) => Some(b),
            _ => None,
        }
    }
}

// =============================================================================
// KeybindingRegistry
// =============================================================================

/// Registry for managing and looking up keybindings
#[derive(Debug, Clone)]
pub struct KeybindingRegistry {
    /// All registered bindings
    bindings: Vec<KeybindingEntry>,

    /// Index by first key combination for fast lookup
    index: HashMap<KeyCombination, Vec<usize>>,

    /// Whether the index needs rebuilding
    dirty: bool,
}

impl KeybindingRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            index: HashMap::new(),
            dirty: false,
        }
    }

    /// Register default keybindings
    pub fn register_defaults(&mut self) {
        // Application bindings
        self.register(
            KeyBinding::new(KeyCode::Char('q'), Modifiers::CTRL)
                .action(Action::Quit)
                .description("Quit application"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char(','), Modifiers::CTRL)
                .action(Action::OpenSettings)
                .description("Open settings"),
        );

        self.register(
            KeyBinding::new(KeyCode::F11, Modifiers::NONE)
                .action(Action::ToggleFullscreen)
                .description("Toggle fullscreen"),
        );

        // Command Palette (Zed's signature feature)
        self.register(
            KeyBinding::new(KeyCode::Char('p'), Modifiers::CTRL | Modifiers::SHIFT)
                .action(Action::OpenCommandPalette)
                .description("Open command palette"),
        );

        self.register(
            KeyBinding::new(KeyCode::Escape, Modifiers::NONE)
                .action(Action::CloseModal)
                .description("Close modal/palette")
                .context(KeybindingContext::Modal),
        );

        self.register(
            KeyBinding::new(KeyCode::Escape, Modifiers::NONE)
                .action(Action::CloseModal)
                .description("Close command palette")
                .context(KeybindingContext::CommandPalette),
        );

        self.register(
            KeyBinding::new(KeyCode::Down, Modifiers::NONE)
                .action(Action::PaletteNext)
                .description("Next item")
                .context(KeybindingContext::CommandPalette),
        );

        self.register(
            KeyBinding::new(KeyCode::Up, Modifiers::NONE)
                .action(Action::PalettePrevious)
                .description("Previous item")
                .context(KeybindingContext::CommandPalette),
        );

        self.register(
            KeyBinding::new(KeyCode::Enter, Modifiers::NONE)
                .action(Action::PaletteSelect)
                .description("Select item")
                .context(KeybindingContext::CommandPalette),
        );

        // Theme toggle
        self.register(
            KeyBinding::new(KeyCode::Char('t'), Modifiers::CTRL | Modifiers::SHIFT)
                .action(Action::ToggleTheme)
                .description("Toggle theme"),
        );

        // Device bindings
        self.register(
            KeyBinding::new(KeyCode::F5, Modifiers::NONE)
                .action(Action::RefreshDevices)
                .description("Refresh devices"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('r'), Modifiers::CTRL)
                .action(Action::RefreshDevices)
                .description("Refresh devices"),
        );

        self.register(
            KeyBinding::new(KeyCode::Down, Modifiers::NONE)
                .action(Action::SelectNextDevice)
                .description("Select next device")
                .context(KeybindingContext::DeviceList),
        );

        self.register(
            KeyBinding::new(KeyCode::Up, Modifiers::NONE)
                .action(Action::SelectPreviousDevice)
                .description("Select previous device")
                .context(KeybindingContext::DeviceList),
        );

        self.register(
            KeyBinding::new(KeyCode::Enter, Modifiers::NONE)
                .action(Action::ShowDeviceDetails)
                .description("Show device details")
                .context(KeybindingContext::DeviceList),
        );

        // Extraction bindings
        self.register(
            KeyBinding::new(KeyCode::Char('e'), Modifiers::CTRL)
                .action(Action::StartExtraction)
                .description("Start extraction"),
        );

        self.register(
            KeyBinding::new(KeyCode::Escape, Modifiers::NONE)
                .action(Action::StopExtraction)
                .description("Stop extraction")
                .context(KeybindingContext::Extracting),
        );

        self.register(
            KeyBinding::new(KeyCode::Space, Modifiers::NONE)
                .action(Action::TogglePause)
                .description("Pause/Resume")
                .context(KeybindingContext::Extracting),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('o'), Modifiers::CTRL)
                .action(Action::OpenOutputFolder)
                .description("Open output folder"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('o'), Modifiers::CTRL | Modifiers::SHIFT)
                .action(Action::ChangeOutputFolder)
                .description("Change output folder"),
        );

        // Navigation bindings
        self.register(
            KeyBinding::new(KeyCode::Char('1'), Modifiers::CTRL)
                .action(Action::FocusDeviceList)
                .description("Focus device list"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('2'), Modifiers::CTRL)
                .action(Action::FocusPreview)
                .description("Focus preview"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('3'), Modifiers::CTRL)
                .action(Action::FocusProgress)
                .description("Focus progress"),
        );

        self.register(
            KeyBinding::new(KeyCode::Tab, Modifiers::CTRL)
                .action(Action::FocusNextPanel)
                .description("Focus next panel"),
        );

        self.register(
            KeyBinding::new(KeyCode::Tab, Modifiers::CTRL | Modifiers::SHIFT)
                .action(Action::FocusPreviousPanel)
                .description("Focus previous panel"),
        );

        // Panel bindings
        self.register(
            KeyBinding::new(KeyCode::Char('b'), Modifiers::CTRL)
                .action(Action::ToggleDeviceListPanel)
                .description("Toggle device panel"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('j'), Modifiers::CTRL)
                .action(Action::ToggleLogPanel)
                .description("Toggle log panel"),
        );

        // Selection bindings
        self.register(
            KeyBinding::new(KeyCode::Char('a'), Modifiers::CTRL)
                .action(Action::SelectAll)
                .description("Select all"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('d'), Modifiers::CTRL)
                .action(Action::DeselectAll)
                .description("Deselect all"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('i'), Modifiers::CTRL)
                .action(Action::InvertSelection)
                .description("Invert selection"),
        );

        self.register(
            KeyBinding::new(KeyCode::Home, Modifiers::NONE)
                .action(Action::SelectFirst)
                .description("Select first"),
        );

        self.register(
            KeyBinding::new(KeyCode::End, Modifiers::NONE)
                .action(Action::SelectLast)
                .description("Select last"),
        );

        // View bindings
        self.register(
            KeyBinding::new(KeyCode::Char('='), Modifiers::CTRL)
                .action(Action::ZoomIn)
                .description("Zoom in"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('-'), Modifiers::CTRL)
                .action(Action::ZoomOut)
                .description("Zoom out"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('0'), Modifiers::CTRL)
                .action(Action::ZoomReset)
                .description("Reset zoom"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('g'), Modifiers::CTRL)
                .action(Action::ToggleGridView)
                .description("Toggle grid view"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('l'), Modifiers::CTRL)
                .action(Action::ToggleListView)
                .description("Toggle list view"),
        );

        // Help
        self.register(
            KeyBinding::new(KeyCode::F1, Modifiers::NONE)
                .action(Action::ShowHelp)
                .description("Show help"),
        );

        self.register(
            KeyBinding::new(KeyCode::Char('?'), Modifiers::SHIFT)
                .action(Action::ShowHelp)
                .description("Show help"),
        );

        // Debug (only in debug builds)
        #[cfg(debug_assertions)]
        {
            self.register(
                KeyBinding::new(KeyCode::F12, Modifiers::NONE)
                    .action(Action::ToggleDebugMode)
                    .description("Toggle debug mode"),
            );

            self.register(
                KeyBinding::new(KeyCode::F12, Modifiers::SHIFT)
                    .action(Action::ShowDebugInfo)
                    .description("Show debug info"),
            );
        }
    }

    /// Register a keybinding
    pub fn register(&mut self, binding: KeyBinding) {
        let entry = KeybindingEntry::new(binding);
        self.bindings.push(entry);
        self.dirty = true;
    }

    /// Register a user-defined keybinding
    pub fn register_user(&mut self, binding: KeyBinding) {
        let entry = KeybindingEntry::user_defined(binding);
        self.bindings.push(entry);
        self.dirty = true;
    }

    /// Register multiple bindings at once
    pub fn register_all(&mut self, bindings: impl IntoIterator<Item = KeyBinding>) {
        for binding in bindings {
            self.register(binding);
        }
    }

    /// Register a keybinding set
    pub fn register_set(&mut self, set: KeybindingSet) {
        for binding in set.bindings {
            self.register(binding);
        }
    }

    /// Remove a binding by action
    pub fn remove_by_action(&mut self, action: &Action) {
        self.bindings.retain(|e| &e.binding.action != action);
        self.dirty = true;
    }

    /// Remove a binding by sequence
    pub fn remove_by_sequence(&mut self, sequence: &KeySequence) {
        self.bindings.retain(|e| &e.binding.sequence != sequence);
        self.dirty = true;
    }

    /// Clear all bindings
    pub fn clear(&mut self) {
        self.bindings.clear();
        self.index.clear();
        self.dirty = false;
    }

    /// Get all bindings
    pub fn all_bindings(&self) -> Vec<&KeyBinding> {
        self.bindings.iter().map(|e| &e.binding).collect()
    }

    /// Get all entries
    pub fn all_entries(&self) -> &[KeybindingEntry] {
        &self.bindings
    }

    /// Get bindings for a specific context
    pub fn bindings_for_context(&self, context: &KeybindingContext) -> Vec<&KeyBinding> {
        self.bindings
            .iter()
            .filter(|e| {
                e.binding.enabled
                    && (e.binding.context.is_global()
                        || e.binding.context.is_compatible_with(context))
            })
            .map(|e| &e.binding)
            .collect()
    }

    /// Get bindings for a specific action
    pub fn bindings_for_action(&self, action: &Action) -> Vec<&KeyBinding> {
        self.bindings
            .iter()
            .filter(|e| &e.binding.action == action)
            .map(|e| &e.binding)
            .collect()
    }

    /// Find matches for a key sequence in a given context
    pub fn find_matches(
        &mut self,
        sequence: &KeySequence,
        context: &KeybindingContext,
    ) -> BindingMatch {
        self.rebuild_index_if_needed();

        if sequence.is_empty() {
            return BindingMatch::None;
        }

        let first = match sequence.first() {
            Some(f) => f,
            None => return BindingMatch::None,
        };

        // Get candidate bindings from index
        let candidates = match self.index.get(first) {
            Some(indices) => indices.clone(),
            None => return BindingMatch::None,
        };

        let mut exact_matches = Vec::new();
        let mut partial_count = 0;

        for idx in candidates {
            let entry = match self.bindings.get(idx) {
                Some(e) => e,
                None => continue,
            };

            // Check if binding is enabled and context matches
            if !entry.binding.enabled {
                continue;
            }

            if !entry.binding.context.is_global()
                && !entry.binding.context.is_compatible_with(context)
            {
                continue;
            }

            let binding_seq = &entry.binding.sequence;

            // Check for exact match
            if binding_seq == sequence {
                exact_matches.push(entry.binding.clone());
            }
            // Check for partial match (sequence is prefix of binding)
            else if binding_seq.len() > sequence.len()
                && binding_seq.starts_with_sequence(sequence)
            {
                partial_count += 1;
            }
        }

        match exact_matches.len() {
            0 if partial_count > 0 => BindingMatch::Partial(partial_count),
            0 => BindingMatch::None,
            1 => BindingMatch::Exact(exact_matches.remove(0)),
            _ => BindingMatch::Multiple(exact_matches),
        }
    }

    /// Rebuild the lookup index if needed
    fn rebuild_index_if_needed(&mut self) {
        if !self.dirty {
            return;
        }

        self.index.clear();

        for (idx, entry) in self.bindings.iter().enumerate() {
            if let Some(first) = entry.binding.sequence.first() {
                self.index
                    .entry(first.clone())
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
        }

        self.dirty = false;
    }

    /// Find conflicts (multiple bindings with same sequence in compatible contexts)
    pub fn find_conflicts(&mut self) -> Vec<(KeySequence, Vec<KeyBinding>)> {
        self.rebuild_index_if_needed();

        let mut conflicts = Vec::new();
        let mut seen: HashMap<(KeySequence, KeybindingContext), Vec<KeyBinding>> = HashMap::new();

        for entry in &self.bindings {
            if !entry.binding.enabled {
                continue;
            }

            let key = (
                entry.binding.sequence.clone(),
                entry.binding.context.clone(),
            );

            seen.entry(key)
                .or_insert_with(Vec::new)
                .push(entry.binding.clone());
        }

        for ((seq, _ctx), bindings) in seen {
            if bindings.len() > 1 {
                conflicts.push((seq, bindings));
            }
        }

        conflicts
    }

    /// Get the number of registered bindings
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Get binding count by context
    pub fn count_by_context(&self) -> HashMap<KeybindingContext, usize> {
        let mut counts = HashMap::new();

        for entry in &self.bindings {
            *counts.entry(entry.binding.context.clone()).or_insert(0) += 1;
        }

        counts
    }
}

impl Default for KeybindingRegistry {
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
    fn test_context_compatibility() {
        let global = KeybindingContext::Global;
        let device = KeybindingContext::DeviceList;
        let preview = KeybindingContext::Preview;

        assert!(global.is_compatible_with(&device));
        assert!(device.is_compatible_with(&global));
        assert!(device.is_compatible_with(&device));
        assert!(!device.is_compatible_with(&preview));
    }

    #[test]
    fn test_context_is_global() {
        assert!(KeybindingContext::Global.is_global());
        assert!(!KeybindingContext::DeviceList.is_global());
    }

    #[test]
    fn test_context_display() {
        assert_eq!(KeybindingContext::Global.display_name(), "Global");
        assert_eq!(KeybindingContext::DeviceList.display_name(), "Device List");
    }

    #[test]
    fn test_entry_new() {
        let binding = KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL);
        let entry = KeybindingEntry::new(binding);

        assert!(!entry.is_user_defined);
        assert_eq!(entry.source, "defaults");
    }

    #[test]
    fn test_entry_user_defined() {
        let binding = KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL);
        let entry = KeybindingEntry::user_defined(binding);

        assert!(entry.is_user_defined);
        assert_eq!(entry.source, "user");
    }

    #[test]
    fn test_keybinding_set() {
        let set = KeybindingSet::new("test")
            .description("Test set")
            .add(KeyBinding::new(KeyCode::Char('a'), Modifiers::CTRL))
            .add(KeyBinding::new(KeyCode::Char('b'), Modifiers::CTRL));

        assert_eq!(set.name, "test");
        assert_eq!(set.bindings.len(), 2);
    }

    #[test]
    fn test_binding_match() {
        let binding = KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL);

        let exact = BindingMatch::Exact(binding.clone());
        assert!(exact.is_match());
        assert!(exact.is_exact());
        assert!(exact.binding().is_some());

        let partial = BindingMatch::Partial(3);
        assert!(partial.is_match());
        assert!(partial.is_partial());
        assert!(partial.binding().is_none());

        let none = BindingMatch::None;
        assert!(!none.is_match());
    }

    #[test]
    fn test_registry_new() {
        let registry = KeybindingRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = KeybindingRegistry::new();

        registry.register(
            KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL).action(Action::StartExtraction),
        );

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_defaults() {
        let mut registry = KeybindingRegistry::new();
        registry.register_defaults();

        assert!(!registry.is_empty());

        // Check some expected defaults
        let quit_bindings = registry.bindings_for_action(&Action::Quit);
        assert!(!quit_bindings.is_empty());

        let palette_bindings = registry.bindings_for_action(&Action::OpenCommandPalette);
        assert!(!palette_bindings.is_empty());
    }

    #[test]
    fn test_registry_find_matches() {
        let mut registry = KeybindingRegistry::new();

        registry.register(
            KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL).action(Action::StartExtraction),
        );

        let sequence = KeySequence::single(KeyCode::Char('s'), Modifiers::CTRL);
        let result = registry.find_matches(&sequence, &KeybindingContext::Global);

        assert!(result.is_exact());
        if let BindingMatch::Exact(binding) = result {
            assert_eq!(binding.action, Action::StartExtraction);
        }
    }

    #[test]
    fn test_registry_context_filtering() {
        let mut registry = KeybindingRegistry::new();

        registry.register(
            KeyBinding::new(KeyCode::Enter, Modifiers::NONE)
                .action(Action::ShowDeviceDetails)
                .context(KeybindingContext::DeviceList),
        );

        // Should find in DeviceList context
        let bindings = registry.bindings_for_context(&KeybindingContext::DeviceList);
        assert!(!bindings.is_empty());

        // Should also find in Global context (global sees everything)
        let global_bindings = registry.bindings_for_context(&KeybindingContext::Global);
        assert!(!global_bindings.is_empty());
    }

    #[test]
    fn test_registry_remove_by_action() {
        let mut registry = KeybindingRegistry::new();

        registry.register(
            KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL).action(Action::StartExtraction),
        );

        registry.remove_by_action(&Action::StartExtraction);

        assert!(registry
            .bindings_for_action(&Action::StartExtraction)
            .is_empty());
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = KeybindingRegistry::new();
        registry.register_defaults();

        assert!(!registry.is_empty());

        registry.clear();

        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_count_by_context() {
        let mut registry = KeybindingRegistry::new();

        registry.register(
            KeyBinding::new(KeyCode::Char('a'), Modifiers::CTRL).context(KeybindingContext::Global),
        );

        registry.register(
            KeyBinding::new(KeyCode::Char('b'), Modifiers::CTRL)
                .context(KeybindingContext::DeviceList),
        );

        registry.register(
            KeyBinding::new(KeyCode::Char('c'), Modifiers::CTRL)
                .context(KeybindingContext::DeviceList),
        );

        let counts = registry.count_by_context();

        assert_eq!(counts.get(&KeybindingContext::Global), Some(&1));
        assert_eq!(counts.get(&KeybindingContext::DeviceList), Some(&2));
    }

    #[test]
    fn test_registry_register_set() {
        let mut registry = KeybindingRegistry::new();

        let set = KeybindingSet::new("test")
            .add(KeyBinding::new(KeyCode::Char('a'), Modifiers::CTRL))
            .add(KeyBinding::new(KeyCode::Char('b'), Modifiers::CTRL));

        registry.register_set(set);

        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_context_serialization() {
        let context = KeybindingContext::DeviceList;
        let json = serde_json::to_string(&context).unwrap();
        let deserialized: KeybindingContext = serde_json::from_str(&json).unwrap();
        assert_eq!(context, deserialized);
    }

    #[test]
    fn test_context_custom() {
        let custom = KeybindingContext::Custom("my_context".to_string());
        assert_eq!(custom.display_name(), "my_context");
        assert!(!custom.is_global());
    }

    #[test]
    fn test_entry_serialization() {
        let binding =
            KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL).action(Action::StartExtraction);
        let entry = KeybindingEntry::new(binding);

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: KeybindingEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry.binding.action, deserialized.binding.action);
    }
}

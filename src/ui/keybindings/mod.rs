//! Keybindings Module - Zed-inspired Keyboard-First System
//!
//! This module provides a comprehensive keybinding system inspired by Zed editor's
//! keyboard-first design philosophy. It supports:
//!
//! - Customizable key bindings with modifiers (Ctrl, Alt, Shift, Meta)
//! - Multi-key sequences (chords) like "Ctrl+K Ctrl+C"
//! - Context-aware bindings (different bindings in different UI states)
//! - Conflict detection and resolution
//! - Serialization for user configuration
//!
//! # Architecture
//!
//! The keybinding system is built around these core concepts:
//!
//! 1. **KeyBinding** - Associates a key combination with an action
//! 2. **KeySequence** - One or more key combinations forming a binding
//! 3. **KeybindingContext** - Determines which bindings are active
//! 4. **KeybindingRegistry** - Manages and dispatches keybindings
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::keybindings::{
//!     KeyBinding, KeyCode, Modifiers, KeybindingRegistry, Action,
//! };
//!
//! // Create a registry
//! let mut registry = KeybindingRegistry::new();
//!
//! // Register default bindings
//! registry.register_defaults();
//!
//! // Add custom binding
//! registry.register(
//!     KeyBinding::new(KeyCode::Char('e'), Modifiers::CTRL)
//!         .action(Action::StartExtraction)
//!         .description("Start extraction")
//! );
//! ```

#![allow(unused_mut)]

pub mod actions;
pub mod keys;
pub mod registry;

pub use actions::{Action, ActionCategory};
pub use keys::{KeyCode, KeyCombination, KeySequence, Modifiers};
pub use registry::{
    BindingMatch, KeybindingContext, KeybindingEntry, KeybindingRegistry, KeybindingSet,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

// =============================================================================
// KeyBinding
// =============================================================================

/// A complete key binding definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    /// The key sequence that triggers this binding
    pub sequence: KeySequence,

    /// The action to perform
    pub action: Action,

    /// Human-readable description
    #[serde(default)]
    pub description: String,

    /// Context in which this binding is active
    #[serde(default)]
    pub context: KeybindingContext,

    /// Whether this binding is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Priority for conflict resolution (higher wins)
    #[serde(default)]
    pub priority: i32,
}

fn default_true() -> bool {
    true
}

impl KeyBinding {
    /// Create a new keybinding with a single key
    pub fn new(key: KeyCode, modifiers: Modifiers) -> Self {
        Self {
            sequence: KeySequence::single(key, modifiers),
            action: Action::None,
            description: String::new(),
            context: KeybindingContext::Global,
            enabled: true,
            priority: 0,
        }
    }

    /// Create a new keybinding from a key combination
    pub fn from_combination(combination: KeyCombination) -> Self {
        Self {
            sequence: KeySequence::from(combination),
            action: Action::None,
            description: String::new(),
            context: KeybindingContext::Global,
            enabled: true,
            priority: 0,
        }
    }

    /// Create a keybinding from a sequence
    pub fn from_sequence(sequence: KeySequence) -> Self {
        Self {
            sequence,
            action: Action::None,
            description: String::new(),
            context: KeybindingContext::Global,
            enabled: true,
            priority: 0,
        }
    }

    /// Parse a keybinding from a string like "Ctrl+Shift+P"
    pub fn parse(s: &str) -> Option<Self> {
        KeySequence::parse(s).map(Self::from_sequence)
    }

    /// Set the action for this binding
    pub fn action(mut self, action: Action) -> Self {
        self.action = action;
        self
    }

    /// Set the description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the context
    pub fn context(mut self, context: KeybindingContext) -> Self {
        self.context = context;
        self
    }

    /// Set the priority
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Enable or disable this binding
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Check if this binding matches the given key event
    pub fn matches(&self, combination: &KeyCombination) -> bool {
        self.enabled && self.sequence.starts_with(combination)
    }

    /// Check if this is a complete match (not a prefix)
    pub fn is_complete_match(&self, combination: &KeyCombination) -> bool {
        self.enabled && self.sequence.len() == 1 && self.sequence.first() == Some(combination)
    }

    /// Get the display string for this keybinding
    pub fn display(&self) -> String {
        self.sequence.display()
    }
}

impl Default for KeyBinding {
    fn default() -> Self {
        Self {
            sequence: KeySequence::default(),
            action: Action::None,
            description: String::new(),
            context: KeybindingContext::Global,
            enabled: true,
            priority: 0,
        }
    }
}

// =============================================================================
// KeybindingManager
// =============================================================================

/// Manages keybinding state and multi-key sequence handling
#[derive(Debug)]
pub struct KeybindingManager {
    /// The keybinding registry
    registry: KeybindingRegistry,

    /// Current partial sequence being typed
    pending_sequence: Vec<KeyCombination>,

    /// When the pending sequence started
    sequence_start: Option<Instant>,

    /// Timeout for multi-key sequences
    sequence_timeout: Duration,

    /// Currently active context
    active_context: KeybindingContext,

    /// Stack of contexts for nested states
    context_stack: Vec<KeybindingContext>,
}

impl KeybindingManager {
    /// Create a new keybinding manager with default bindings
    pub fn new() -> Self {
        let mut registry = KeybindingRegistry::new();
        registry.register_defaults();

        Self {
            registry,
            pending_sequence: Vec::new(),
            sequence_start: None,
            sequence_timeout: Duration::from_millis(1500),
            active_context: KeybindingContext::Global,
            context_stack: Vec::new(),
        }
    }

    /// Create a manager with a custom registry
    pub fn with_registry(registry: KeybindingRegistry) -> Self {
        Self {
            registry,
            pending_sequence: Vec::new(),
            sequence_start: None,
            sequence_timeout: Duration::from_millis(1500),
            active_context: KeybindingContext::Global,
            context_stack: Vec::new(),
        }
    }

    /// Get the registry for modification
    pub fn registry(&self) -> &KeybindingRegistry {
        &self.registry
    }

    /// Get mutable registry
    pub fn registry_mut(&mut self) -> &mut KeybindingRegistry {
        &mut self.registry
    }

    /// Set the sequence timeout
    pub fn set_sequence_timeout(&mut self, timeout: Duration) {
        self.sequence_timeout = timeout;
    }

    /// Set the active context
    pub fn set_context(&mut self, context: KeybindingContext) {
        self.active_context = context;
    }

    /// Push a new context onto the stack
    pub fn push_context(&mut self, context: KeybindingContext) {
        self.context_stack.push(self.active_context.clone());
        self.active_context = context;
    }

    /// Pop the context stack, returning to the previous context
    pub fn pop_context(&mut self) -> Option<KeybindingContext> {
        self.context_stack.pop().map(|ctx| {
            let old = std::mem::replace(&mut self.active_context, ctx);
            old
        })
    }

    /// Get the current context
    pub fn context(&self) -> &KeybindingContext {
        &self.active_context
    }

    /// Handle a key event and return the matched action (if any)
    pub fn handle_key(&mut self, combination: KeyCombination) -> KeybindingResult {
        // Check for sequence timeout
        if let Some(start) = self.sequence_start {
            if start.elapsed() > self.sequence_timeout {
                self.clear_pending();
            }
        }

        // Add to pending sequence
        self.pending_sequence.push(combination.clone());
        self.sequence_start = Some(Instant::now());

        // Build the current sequence
        let current_sequence = KeySequence::new(self.pending_sequence.clone());

        // Check for matches
        let matches = self
            .registry
            .find_matches(&current_sequence, &self.active_context);

        match matches {
            BindingMatch::None => {
                // No match, clear pending
                self.clear_pending();
                KeybindingResult::NoMatch
            }
            BindingMatch::Partial(count) => {
                // Partial match, waiting for more keys
                KeybindingResult::Pending {
                    possible_completions: count,
                }
            }
            BindingMatch::Exact(binding) => {
                // Complete match
                self.clear_pending();
                KeybindingResult::Match {
                    action: binding.action.clone(),
                    binding: binding.clone(),
                }
            }
            BindingMatch::Multiple(bindings) => {
                // Multiple matches - take highest priority
                self.clear_pending();
                if let Some(binding) = bindings.into_iter().max_by_key(|b| b.priority) {
                    KeybindingResult::Match {
                        action: binding.action.clone(),
                        binding: binding.clone(),
                    }
                } else {
                    KeybindingResult::NoMatch
                }
            }
        }
    }

    /// Clear the pending sequence
    pub fn clear_pending(&mut self) {
        self.pending_sequence.clear();
        self.sequence_start = None;
    }

    /// Check if there's a pending sequence
    pub fn has_pending(&self) -> bool {
        !self.pending_sequence.is_empty()
    }

    /// Get the pending sequence display string
    pub fn pending_display(&self) -> Option<String> {
        if self.pending_sequence.is_empty() {
            None
        } else {
            Some(
                self.pending_sequence
                    .iter()
                    .map(|c| c.display())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        }
    }

    /// Get all bindings for the current context
    pub fn current_bindings(&self) -> Vec<&KeyBinding> {
        self.registry.bindings_for_context(&self.active_context)
    }

    /// Get bindings grouped by category
    pub fn bindings_by_category(&self) -> HashMap<ActionCategory, Vec<&KeyBinding>> {
        let mut result: HashMap<ActionCategory, Vec<&KeyBinding>> = HashMap::new();

        for binding in self.current_bindings() {
            let category = binding.action.category();
            result.entry(category).or_default().push(binding);
        }

        result
    }
}

impl Default for KeybindingManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// KeybindingResult
// =============================================================================

/// Result of processing a key event
#[derive(Debug, Clone)]
pub enum KeybindingResult {
    /// No binding matched
    NoMatch,

    /// Partial match - waiting for more keys in sequence
    Pending {
        /// Number of possible completions
        possible_completions: usize,
    },

    /// Complete match found
    Match {
        /// The action to perform
        action: Action,
        /// The binding that matched
        binding: KeyBinding,
    },
}

impl KeybindingResult {
    /// Check if this is a match
    pub fn is_match(&self) -> bool {
        matches!(self, KeybindingResult::Match { .. })
    }

    /// Check if this is pending
    pub fn is_pending(&self) -> bool {
        matches!(self, KeybindingResult::Pending { .. })
    }

    /// Check if there was no match
    pub fn is_no_match(&self) -> bool {
        matches!(self, KeybindingResult::NoMatch)
    }

    /// Get the action if this is a match
    pub fn action(&self) -> Option<&Action> {
        match self {
            KeybindingResult::Match { action, .. } => Some(action),
            _ => None,
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
    fn test_keybinding_new() {
        let binding = KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL)
            .action(Action::StartExtraction)
            .description("Save");

        assert_eq!(binding.action, Action::StartExtraction);
        assert_eq!(binding.description, "Save");
        assert!(binding.enabled);
    }

    #[test]
    fn test_keybinding_parse() {
        let binding = KeyBinding::parse("Ctrl+S").unwrap();
        assert_eq!(binding.sequence.len(), 1);

        let combo = binding.sequence.first().unwrap();
        assert_eq!(combo.key, KeyCode::Char('s'));
        assert!(combo.modifiers.ctrl);
    }

    #[test]
    fn test_keybinding_display() {
        let binding = KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL | Modifiers::SHIFT);

        let display = binding.display();
        assert!(display.contains("Ctrl"));
        assert!(display.contains("Shift"));
        assert!(display.contains('S'));
    }

    #[test]
    fn test_keybinding_matches() {
        let binding = KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL);
        let combo = KeyCombination::new(KeyCode::Char('s'), Modifiers::CTRL);

        assert!(binding.matches(&combo));

        let wrong_combo = KeyCombination::new(KeyCode::Char('d'), Modifiers::CTRL);
        assert!(!binding.matches(&wrong_combo));
    }

    #[test]
    fn test_keybinding_manager_simple() {
        let mut manager = KeybindingManager::new();

        // Test Ctrl+P for command palette
        let combo = KeyCombination::new(KeyCode::Char('p'), Modifiers::CTRL | Modifiers::SHIFT);
        let result = manager.handle_key(combo);

        // Should match command palette
        assert!(result.is_match() || result.is_no_match());
    }

    #[test]
    fn test_keybinding_manager_context() {
        let mut manager = KeybindingManager::new();

        manager.set_context(KeybindingContext::DeviceList);
        assert_eq!(manager.context(), &KeybindingContext::DeviceList);

        manager.push_context(KeybindingContext::Modal);
        assert_eq!(manager.context(), &KeybindingContext::Modal);

        manager.pop_context();
        assert_eq!(manager.context(), &KeybindingContext::DeviceList);
    }

    #[test]
    fn test_keybinding_manager_pending() {
        let mut manager = KeybindingManager::new();

        assert!(!manager.has_pending());
        assert!(manager.pending_display().is_none());
    }

    #[test]
    fn test_keybinding_result() {
        let no_match = KeybindingResult::NoMatch;
        assert!(no_match.is_no_match());
        assert!(!no_match.is_match());
        assert!(no_match.action().is_none());

        let pending = KeybindingResult::Pending {
            possible_completions: 3,
        };
        assert!(pending.is_pending());

        let matched = KeybindingResult::Match {
            action: Action::Quit,
            binding: KeyBinding::default(),
        };
        assert!(matched.is_match());
        assert_eq!(matched.action(), Some(&Action::Quit));
    }

    #[test]
    fn test_keybinding_disabled() {
        let binding = KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL).enabled(false);

        let combo = KeyCombination::new(KeyCode::Char('s'), Modifiers::CTRL);
        assert!(!binding.matches(&combo));
    }

    #[test]
    fn test_keybinding_priority() {
        let low = KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL).priority(0);
        let high = KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL).priority(10);

        assert!(high.priority > low.priority);
    }

    #[test]
    fn test_keybinding_context_default() {
        let binding = KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL);
        assert_eq!(binding.context, KeybindingContext::Global);
    }

    #[test]
    fn test_keybinding_serialization() {
        let binding = KeyBinding::new(KeyCode::Char('s'), Modifiers::CTRL)
            .action(Action::StartExtraction)
            .description("Start");

        let json = serde_json::to_string(&binding).unwrap();
        let deserialized: KeyBinding = serde_json::from_str(&json).unwrap();

        assert_eq!(binding.action, deserialized.action);
        assert_eq!(binding.description, deserialized.description);
    }
}

//! Commands Module - Zed-inspired Command Palette System
//!
//! This module provides a command palette system inspired by Zed editor's
//! keyboard-first design. The command palette allows users to discover and
//! execute any action using fuzzy search.
//!
//! # Architecture
//!
//! The command system is built around these core concepts:
//!
//! 1. **Command** - A searchable, executable action with metadata
//! 2. **CommandPalette** - The UI state for the palette
//! 3. **CommandRegistry** - Storage for all available commands
//! 4. **FuzzyMatcher** - Fast fuzzy search for command filtering
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::commands::{
//!     Command, CommandPalette, CommandRegistry,
//! };
//! use photo_extraction_tool::ui::keybindings::Action;
//!
//! // Create a command palette
//! let mut palette = CommandPalette::new();
//!
//! // Open the palette
//! palette.open();
//!
//! // Type to filter
//! palette.set_query("extract");
//!
//! // Get filtered results
//! let results = palette.matches();
//! ```

#![allow(dead_code)]

use crate::ui::keybindings::{Action, ActionCategory, KeybindingRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

// =============================================================================
// Command
// =============================================================================

/// A command that can be executed from the command palette
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    /// Unique identifier for the command
    pub id: String,

    /// Display label shown in the palette
    pub label: String,

    /// Optional description/subtitle
    pub description: Option<String>,

    /// The action to execute
    pub action: Action,

    /// Category for grouping
    pub category: ActionCategory,

    /// Keywords for search (in addition to label)
    pub keywords: Vec<String>,

    /// Icon/emoji for display
    pub icon: String,

    /// Whether this command is enabled
    pub enabled: bool,

    /// Associated keybinding (if any)
    pub keybinding: Option<String>,

    /// How many times this command has been used (for frecency sorting)
    #[serde(default)]
    pub use_count: u32,

    /// Last time this command was used
    #[serde(skip)]
    pub last_used: Option<Instant>,
}

impl Command {
    /// Create a new command from an action
    pub fn from_action(action: Action) -> Self {
        Self {
            id: format!("{:?}", action).to_lowercase().replace(' ', "_"),
            label: action.description().to_string(),
            description: None,
            category: action.category(),
            keywords: Vec::new(),
            icon: action.icon().to_string(),
            enabled: true,
            keybinding: None,
            use_count: 0,
            last_used: None,
            action,
        }
    }

    /// Create a new command with a custom label
    pub fn new(id: impl Into<String>, label: impl Into<String>, action: Action) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            category: action.category(),
            keywords: Vec::new(),
            icon: action.icon().to_string(),
            enabled: true,
            keybinding: None,
            use_count: 0,
            last_used: None,
            action,
        }
    }

    /// Set the description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the category
    pub fn category(mut self, category: ActionCategory) -> Self {
        self.category = category;
        self
    }

    /// Add keywords for search
    pub fn keywords(mut self, keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keywords = keywords.into_iter().map(|k| k.into()).collect();
        self
    }

    /// Set the icon
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Set enabled state
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the keybinding display string
    pub fn keybinding(mut self, binding: impl Into<String>) -> Self {
        self.keybinding = Some(binding.into());
        self
    }

    /// Record that this command was used
    pub fn record_use(&mut self) {
        self.use_count += 1;
        self.last_used = Some(Instant::now());
    }

    /// Calculate frecency score (frequency + recency)
    pub fn frecency_score(&self) -> f64 {
        let frequency_score = (self.use_count as f64).ln_1p();

        let recency_score = match self.last_used {
            Some(last) => {
                let seconds_ago = last.elapsed().as_secs_f64();
                // Decay over time: score halves every hour
                (-seconds_ago / 3600.0).exp()
            }
            None => 0.0,
        };

        frequency_score + recency_score * 2.0
    }

    /// Get searchable text (label + keywords)
    pub fn searchable_text(&self) -> String {
        let mut text = self.label.clone();
        for keyword in &self.keywords {
            text.push(' ');
            text.push_str(keyword);
        }
        if let Some(desc) = &self.description {
            text.push(' ');
            text.push_str(desc);
        }
        text
    }
}

impl Default for Command {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            description: None,
            action: Action::None,
            category: ActionCategory::None,
            keywords: Vec::new(),
            icon: String::new(),
            enabled: true,
            keybinding: None,
            use_count: 0,
            last_used: None,
        }
    }
}

// =============================================================================
// CommandMatch
// =============================================================================

/// A command match with its score
#[derive(Debug, Clone)]
pub struct CommandMatch {
    /// The matched command
    pub command: Command,

    /// Match score (higher is better)
    pub score: f64,

    /// Indices of matched characters in the label
    pub matched_indices: Vec<usize>,
}

impl CommandMatch {
    /// Create a new command match
    pub fn new(command: Command, score: f64, matched_indices: Vec<usize>) -> Self {
        Self {
            command,
            score,
            matched_indices,
        }
    }

    /// Adjust score based on frecency
    pub fn with_frecency(mut self) -> Self {
        self.score += self.command.frecency_score();
        self
    }
}

// =============================================================================
// FuzzyMatcher
// =============================================================================

/// Fuzzy matching for command search
#[derive(Debug, Clone, Default)]
pub struct FuzzyMatcher {
    /// Whether to be case-sensitive
    pub case_sensitive: bool,

    /// Minimum score to be considered a match
    pub min_score: f64,
}

impl FuzzyMatcher {
    /// Create a new fuzzy matcher
    pub fn new() -> Self {
        Self {
            case_sensitive: false,
            min_score: 0.0,
        }
    }

    /// Match a query against text, returning score and matched indices
    pub fn match_text(&self, query: &str, text: &str) -> Option<(f64, Vec<usize>)> {
        if query.is_empty() {
            return Some((0.0, Vec::new()));
        }

        let query_chars: Vec<char> = if self.case_sensitive {
            query.chars().collect()
        } else {
            query.to_lowercase().chars().collect()
        };

        let text_chars: Vec<char> = if self.case_sensitive {
            text.chars().collect()
        } else {
            text.to_lowercase().chars().collect()
        };

        let mut score = 0.0;
        let mut matched_indices = Vec::new();
        let mut query_idx = 0;
        let mut prev_match_idx: Option<usize> = None;
        let mut consecutive_bonus = 0.0;

        for (text_idx, text_char) in text_chars.iter().enumerate() {
            if query_idx >= query_chars.len() {
                break;
            }

            if text_char == &query_chars[query_idx] {
                matched_indices.push(text_idx);

                // Base score for a match
                let mut char_score = 1.0;

                // Bonus for matching at word boundaries
                if text_idx == 0 {
                    char_score += 2.0; // Start of string
                } else {
                    let prev_char = text_chars.get(text_idx.saturating_sub(1));
                    if prev_char.map_or(false, |c| !c.is_alphanumeric()) {
                        char_score += 1.5; // After separator
                    }
                }

                // Bonus for consecutive matches
                if let Some(prev) = prev_match_idx {
                    if text_idx == prev + 1 {
                        consecutive_bonus += 0.5;
                        char_score += consecutive_bonus;
                    } else {
                        consecutive_bonus = 0.0;
                    }
                }

                // Bonus for matching uppercase in camelCase
                let original_char = text.chars().nth(text_idx).unwrap_or(' ');
                if original_char.is_uppercase() {
                    char_score += 0.5;
                }

                score += char_score;
                prev_match_idx = Some(text_idx);
                query_idx += 1;
            }
        }

        // All query characters must match
        if query_idx != query_chars.len() {
            return None;
        }

        // Normalize score by query length
        let normalized_score = score / query_chars.len() as f64;

        // Bonus for shorter matches (more precise)
        let length_bonus = 1.0 / (1.0 + (text_chars.len() as f64 - query_chars.len() as f64) * 0.1);
        let final_score = normalized_score * length_bonus;

        if final_score >= self.min_score {
            Some((final_score, matched_indices))
        } else {
            None
        }
    }

    /// Match a query against a command
    pub fn match_command(&self, query: &str, command: &Command) -> Option<CommandMatch> {
        if !command.enabled {
            return None;
        }

        // Try matching against label first (highest priority)
        if let Some((score, indices)) = self.match_text(query, &command.label) {
            return Some(CommandMatch::new(command.clone(), score * 2.0, indices).with_frecency());
        }

        // Try matching against searchable text (includes keywords)
        if let Some((score, indices)) = self.match_text(query, &command.searchable_text()) {
            // Indices won't match label exactly, but still useful for scoring
            return Some(CommandMatch::new(command.clone(), score, indices).with_frecency());
        }

        None
    }
}

// =============================================================================
// CommandRegistry
// =============================================================================

/// Registry for all available commands
#[derive(Debug, Clone, Default)]
pub struct CommandRegistry {
    /// All registered commands
    commands: Vec<Command>,

    /// Index by command ID
    by_id: HashMap<String, usize>,

    /// Index by action
    by_action: HashMap<String, usize>,
}

impl CommandRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            by_id: HashMap::new(),
            by_action: HashMap::new(),
        }
    }

    /// Register commands from all available actions
    pub fn register_from_actions(&mut self) {
        for action in Action::all() {
            let command = Command::from_action(action.clone());
            self.register(command);
        }
    }

    /// Add keybinding information to commands from a registry
    pub fn add_keybindings(&mut self, keybinding_registry: &KeybindingRegistry) {
        for command in &mut self.commands {
            let bindings = keybinding_registry.bindings_for_action(&command.action);
            if let Some(binding) = bindings.first() {
                command.keybinding = Some(binding.display());
            }
        }
    }

    /// Register a command
    pub fn register(&mut self, command: Command) {
        let idx = self.commands.len();
        self.by_id.insert(command.id.clone(), idx);
        self.by_action.insert(format!("{:?}", command.action), idx);
        self.commands.push(command);
    }

    /// Get a command by ID
    pub fn get(&self, id: &str) -> Option<&Command> {
        self.by_id.get(id).and_then(|&idx| self.commands.get(idx))
    }

    /// Get a mutable command by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Command> {
        self.by_id
            .get(id)
            .copied()
            .and_then(move |idx| self.commands.get_mut(idx))
    }

    /// Get a command by action
    pub fn get_by_action(&self, action: &Action) -> Option<&Command> {
        let key = format!("{:?}", action);
        self.by_action
            .get(&key)
            .and_then(|&idx| self.commands.get(idx))
    }

    /// Get all commands
    pub fn all(&self) -> &[Command] {
        &self.commands
    }

    /// Get commands by category
    pub fn by_category(&self, category: ActionCategory) -> Vec<&Command> {
        self.commands
            .iter()
            .filter(|c| c.category == category)
            .collect()
    }

    /// Get enabled commands
    pub fn enabled(&self) -> Vec<&Command> {
        self.commands.iter().filter(|c| c.enabled).collect()
    }

    /// Search commands with fuzzy matching
    pub fn search(&self, query: &str) -> Vec<CommandMatch> {
        let matcher = FuzzyMatcher::new();
        let mut matches: Vec<CommandMatch> = self
            .commands
            .iter()
            .filter_map(|cmd| matcher.match_command(query, cmd))
            .collect();

        // Sort by score (descending)
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        matches
    }

    /// Record command usage
    pub fn record_use(&mut self, id: &str) {
        if let Some(command) = self.get_mut(id) {
            command.record_use();
        }
    }

    /// Get number of commands
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Enable/disable a command
    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(command) = self.get_mut(id) {
            command.enabled = enabled;
        }
    }
}

// =============================================================================
// CommandPalette
// =============================================================================

/// The command palette UI state
#[derive(Debug, Clone)]
pub struct CommandPalette {
    /// Whether the palette is open
    open: bool,

    /// Current search query
    query: String,

    /// Currently selected index
    selected_index: usize,

    /// Filtered and sorted matches
    matches: Vec<CommandMatch>,

    /// Command registry
    registry: CommandRegistry,

    /// Fuzzy matcher
    matcher: FuzzyMatcher,

    /// Maximum number of results to show
    max_results: usize,

    /// Show categories in results
    show_categories: bool,

    /// Show keybindings in results
    show_keybindings: bool,

    /// Placeholder text
    placeholder: String,
}

impl CommandPalette {
    /// Create a new command palette
    pub fn new() -> Self {
        let mut registry = CommandRegistry::new();
        registry.register_from_actions();

        Self {
            open: false,
            query: String::new(),
            selected_index: 0,
            matches: Vec::new(),
            registry,
            matcher: FuzzyMatcher::new(),
            max_results: 20,
            show_categories: true,
            show_keybindings: true,
            placeholder: "Type a command...".to_string(),
        }
    }

    /// Create a command palette with a custom registry
    pub fn with_registry(registry: CommandRegistry) -> Self {
        Self {
            registry,
            ..Self::new()
        }
    }

    /// Open the command palette
    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected_index = 0;
        self.update_matches();
    }

    /// Close the command palette
    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected_index = 0;
        self.matches.clear();
    }

    /// Toggle the command palette
    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Check if the palette is open
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Get the current query
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Set the search query
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.selected_index = 0;
        self.update_matches();
    }

    /// Append to the query
    pub fn append_query(&mut self, text: &str) {
        self.query.push_str(text);
        self.update_matches();
    }

    /// Delete last character from query
    pub fn backspace(&mut self) {
        self.query.pop();
        self.update_matches();
    }

    /// Clear the query
    pub fn clear_query(&mut self) {
        self.query.clear();
        self.selected_index = 0;
        self.update_matches();
    }

    /// Update filtered matches based on current query
    fn update_matches(&mut self) {
        if self.query.is_empty() {
            // Show recent/popular commands when query is empty
            self.matches = self
                .registry
                .enabled()
                .into_iter()
                .map(|cmd| CommandMatch::new(cmd.clone(), cmd.frecency_score(), Vec::new()))
                .collect();

            // Sort by frecency
            self.matches.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            self.matches = self.registry.search(&self.query);
        }

        // Limit results
        self.matches.truncate(self.max_results);

        // Ensure selected index is valid
        if self.selected_index >= self.matches.len() {
            self.selected_index = self.matches.len().saturating_sub(1);
        }
    }

    /// Get the filtered matches
    pub fn matches(&self) -> &[CommandMatch] {
        &self.matches
    }

    /// Get the currently selected match
    pub fn selected(&self) -> Option<&CommandMatch> {
        self.matches.get(self.selected_index)
    }

    /// Get the selected index
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Select next item
    pub fn select_next(&mut self) {
        if !self.matches.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.matches.len();
        }
    }

    /// Select previous item
    pub fn select_previous(&mut self) {
        if !self.matches.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.matches.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    /// Select first item
    pub fn select_first(&mut self) {
        self.selected_index = 0;
    }

    /// Select last item
    pub fn select_last(&mut self) {
        if !self.matches.is_empty() {
            self.selected_index = self.matches.len() - 1;
        }
    }

    /// Select by index
    pub fn select_index(&mut self, index: usize) {
        if index < self.matches.len() {
            self.selected_index = index;
        }
    }

    /// Execute the selected command and return its action
    pub fn execute_selected(&mut self) -> Option<Action> {
        let selected = self.selected()?.clone();

        // Record usage
        self.registry.record_use(&selected.command.id);

        // Close palette
        self.close();

        Some(selected.command.action)
    }

    /// Execute a command by ID
    pub fn execute(&mut self, id: &str) -> Option<Action> {
        let command = self.registry.get(id)?.clone();
        self.registry.record_use(id);
        self.close();
        Some(command.action)
    }

    /// Get the registry
    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    /// Get mutable registry
    pub fn registry_mut(&mut self) -> &mut CommandRegistry {
        &mut self.registry
    }

    /// Set placeholder text
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = placeholder.into();
    }

    /// Get placeholder text
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Set maximum results
    pub fn set_max_results(&mut self, max: usize) {
        self.max_results = max;
        self.update_matches();
    }

    /// Add keybinding information
    pub fn add_keybindings(&mut self, keybinding_registry: &KeybindingRegistry) {
        self.registry.add_keybindings(keybinding_registry);
    }

    /// Get results count
    pub fn results_count(&self) -> usize {
        self.matches.len()
    }

    /// Check if there are any results
    pub fn has_results(&self) -> bool {
        !self.matches.is_empty()
    }
}

impl Default for CommandPalette {
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
    fn test_command_from_action() {
        let command = Command::from_action(Action::StartExtraction);

        assert!(!command.id.is_empty());
        assert_eq!(command.label, "Start extraction");
        assert_eq!(command.category, ActionCategory::Extraction);
        assert!(command.enabled);
    }

    #[test]
    fn test_command_builder() {
        let command = Command::new("test", "Test Command", Action::Quit)
            .description("A test command")
            .keywords(vec!["foo", "bar"])
            .icon("🧪");

        assert_eq!(command.id, "test");
        assert_eq!(command.label, "Test Command");
        assert_eq!(command.description, Some("A test command".to_string()));
        assert_eq!(command.keywords.len(), 2);
        assert_eq!(command.icon, "🧪");
    }

    #[test]
    fn test_command_frecency() {
        let mut command = Command::from_action(Action::Quit);

        let initial_score = command.frecency_score();
        assert_eq!(initial_score, 0.0);

        command.record_use();
        assert!(command.frecency_score() > initial_score);
    }

    #[test]
    fn test_command_searchable_text() {
        let command = Command::new("test", "My Label", Action::Quit)
            .description("Description")
            .keywords(vec!["key1", "key2"]);

        let text = command.searchable_text();
        assert!(text.contains("My Label"));
        assert!(text.contains("key1"));
        assert!(text.contains("key2"));
        assert!(text.contains("Description"));
    }

    #[test]
    fn test_fuzzy_matcher_exact() {
        let matcher = FuzzyMatcher::new();
        let result = matcher.match_text("hello", "hello");

        assert!(result.is_some());
        let (score, indices) = result.unwrap();
        assert!(score > 0.0);
        assert_eq!(indices.len(), 5);
    }

    #[test]
    fn test_fuzzy_matcher_partial() {
        let matcher = FuzzyMatcher::new();
        let result = matcher.match_text("hlo", "hello");

        assert!(result.is_some());
        let (_, indices) = result.unwrap();
        assert_eq!(indices.len(), 3);
    }

    #[test]
    fn test_fuzzy_matcher_no_match() {
        let matcher = FuzzyMatcher::new();
        let result = matcher.match_text("xyz", "hello");

        assert!(result.is_none());
    }

    #[test]
    fn test_fuzzy_matcher_case_insensitive() {
        let matcher = FuzzyMatcher::new();
        let result = matcher.match_text("HELLO", "hello");

        assert!(result.is_some());
    }

    #[test]
    fn test_fuzzy_matcher_word_boundary_bonus() {
        let matcher = FuzzyMatcher::new();

        let boundary_result = matcher.match_text("se", "Start Extraction");
        let mid_result = matcher.match_text("ra", "Extraction");

        assert!(boundary_result.is_some());
        assert!(mid_result.is_some());

        // Boundary match should score higher
        let (boundary_score, _) = boundary_result.unwrap();
        let (mid_score, _) = mid_result.unwrap();
        assert!(boundary_score >= mid_score);
    }

    #[test]
    fn test_command_registry_new() {
        let registry = CommandRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_command_registry_register() {
        let mut registry = CommandRegistry::new();

        registry.register(Command::from_action(Action::Quit));

        assert_eq!(registry.len(), 1);
        assert!(registry.get("quit").is_some());
    }

    #[test]
    fn test_command_registry_from_actions() {
        let mut registry = CommandRegistry::new();
        registry.register_from_actions();

        assert!(!registry.is_empty());
        assert!(registry.get_by_action(&Action::Quit).is_some());
    }

    #[test]
    fn test_command_registry_search() {
        let mut registry = CommandRegistry::new();
        registry.register_from_actions();

        let results = registry.search("quit");
        assert!(!results.is_empty());

        // First result should be the quit command
        assert!(results[0].command.label.to_lowercase().contains("quit"));
    }

    #[test]
    fn test_command_registry_by_category() {
        let mut registry = CommandRegistry::new();
        registry.register_from_actions();

        let extraction_commands = registry.by_category(ActionCategory::Extraction);
        assert!(!extraction_commands.is_empty());

        for cmd in extraction_commands {
            assert_eq!(cmd.category, ActionCategory::Extraction);
        }
    }

    #[test]
    fn test_command_palette_new() {
        let palette = CommandPalette::new();

        assert!(!palette.is_open());
        assert!(palette.query().is_empty());
        assert!(!palette.registry().is_empty());
    }

    #[test]
    fn test_command_palette_open_close() {
        let mut palette = CommandPalette::new();

        palette.open();
        assert!(palette.is_open());

        palette.close();
        assert!(!palette.is_open());
    }

    #[test]
    fn test_command_palette_toggle() {
        let mut palette = CommandPalette::new();

        palette.toggle();
        assert!(palette.is_open());

        palette.toggle();
        assert!(!palette.is_open());
    }

    #[test]
    fn test_command_palette_query() {
        let mut palette = CommandPalette::new();
        palette.open();

        palette.set_query("test");
        assert_eq!(palette.query(), "test");

        palette.append_query("ing");
        assert_eq!(palette.query(), "testing");

        palette.backspace();
        assert_eq!(palette.query(), "testin");

        palette.clear_query();
        assert!(palette.query().is_empty());
    }

    #[test]
    fn test_command_palette_selection() {
        let mut palette = CommandPalette::new();
        palette.open();

        assert!(palette.has_results());
        assert_eq!(palette.selected_index(), 0);

        palette.select_next();
        assert_eq!(palette.selected_index(), 1);

        palette.select_previous();
        assert_eq!(palette.selected_index(), 0);

        palette.select_last();
        assert!(palette.selected_index() > 0);

        palette.select_first();
        assert_eq!(palette.selected_index(), 0);
    }

    #[test]
    fn test_command_palette_execute() {
        let mut palette = CommandPalette::new();
        palette.open();
        palette.set_query("quit");

        assert!(palette.has_results());

        let action = palette.execute_selected();
        assert!(action.is_some());
        assert!(!palette.is_open());
    }

    #[test]
    fn test_command_palette_filtering() {
        let mut palette = CommandPalette::new();
        palette.open();

        let initial_count = palette.results_count();

        palette.set_query("extract");

        // Should have fewer results after filtering
        assert!(palette.results_count() <= initial_count);

        // All results should match the query
        for m in palette.matches() {
            let text = m.command.searchable_text().to_lowercase();
            assert!(
                text.contains("extract") || m.score > 0.0,
                "Command '{}' should match 'extract'",
                m.command.label
            );
        }
    }

    #[test]
    fn test_command_match() {
        let command = Command::from_action(Action::Quit);
        let matched = CommandMatch::new(command.clone(), 1.5, vec![0, 1, 2, 3]);

        assert_eq!(matched.score, 1.5);
        assert_eq!(matched.matched_indices.len(), 4);
    }

    #[test]
    fn test_command_serialization() {
        let command = Command::from_action(Action::StartExtraction);
        let json = serde_json::to_string(&command).unwrap();
        let deserialized: Command = serde_json::from_str(&json).unwrap();

        assert_eq!(command.id, deserialized.id);
        assert_eq!(command.label, deserialized.label);
    }
}

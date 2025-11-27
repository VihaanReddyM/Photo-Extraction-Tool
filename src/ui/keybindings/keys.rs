//! Key Types - Key codes, modifiers, and combinations
//!
//! This module provides types for representing keyboard input:
//! - Key codes for individual keys
//! - Modifier flags (Ctrl, Alt, Shift, Meta)
//! - Key combinations (key + modifiers)
//! - Key sequences (multiple combinations for chords)

use serde::{Deserialize, Serialize};
use std::fmt;

// =============================================================================
// Modifiers
// =============================================================================

/// Keyboard modifier flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Modifiers {
    /// Control key (Ctrl)
    pub ctrl: bool,
    /// Alt key (Option on macOS)
    pub alt: bool,
    /// Shift key
    pub shift: bool,
    /// Meta key (Windows key / Command on macOS)
    pub meta: bool,
}

impl Modifiers {
    /// No modifiers
    pub const NONE: Modifiers = Modifiers {
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
    };

    /// Control modifier only
    pub const CTRL: Modifiers = Modifiers {
        ctrl: true,
        alt: false,
        shift: false,
        meta: false,
    };

    /// Alt modifier only
    pub const ALT: Modifiers = Modifiers {
        ctrl: false,
        alt: true,
        shift: false,
        meta: false,
    };

    /// Shift modifier only
    pub const SHIFT: Modifiers = Modifiers {
        ctrl: false,
        alt: false,
        shift: true,
        meta: false,
    };

    /// Meta modifier only (Windows/Command key)
    pub const META: Modifiers = Modifiers {
        ctrl: false,
        alt: false,
        shift: false,
        meta: true,
    };

    /// Control + Shift
    pub const CTRL_SHIFT: Modifiers = Modifiers {
        ctrl: true,
        alt: false,
        shift: true,
        meta: false,
    };

    /// Control + Alt
    pub const CTRL_ALT: Modifiers = Modifiers {
        ctrl: true,
        alt: true,
        shift: false,
        meta: false,
    };

    /// Alt + Shift
    pub const ALT_SHIFT: Modifiers = Modifiers {
        ctrl: false,
        alt: true,
        shift: true,
        meta: false,
    };

    /// Create new modifiers
    pub fn new(ctrl: bool, alt: bool, shift: bool, meta: bool) -> Self {
        Self {
            ctrl,
            alt,
            shift,
            meta,
        }
    }

    /// Check if no modifiers are pressed
    pub fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.meta
    }

    /// Check if any modifier is pressed
    pub fn any(&self) -> bool {
        self.ctrl || self.alt || self.shift || self.meta
    }

    /// Check if only shift is pressed (useful for text input)
    pub fn only_shift(&self) -> bool {
        self.shift && !self.ctrl && !self.alt && !self.meta
    }

    /// Count how many modifiers are active
    pub fn count(&self) -> usize {
        let mut count = 0;
        if self.ctrl {
            count += 1;
        }
        if self.alt {
            count += 1;
        }
        if self.shift {
            count += 1;
        }
        if self.meta {
            count += 1;
        }
        count
    }

    /// Display string for the modifiers
    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.meta {
            #[cfg(target_os = "macos")]
            parts.push("Cmd");
            #[cfg(not(target_os = "macos"))]
            parts.push("Win");
        }
        parts.join("+")
    }

    /// Parse modifiers from a string like "Ctrl+Shift"
    pub fn parse(s: &str) -> Self {
        let s_lower = s.to_lowercase();
        Self {
            ctrl: s_lower.contains("ctrl") || s_lower.contains("control"),
            alt: s_lower.contains("alt") || s_lower.contains("option"),
            shift: s_lower.contains("shift"),
            meta: s_lower.contains("meta")
                || s_lower.contains("win")
                || s_lower.contains("cmd")
                || s_lower.contains("command")
                || s_lower.contains("super"),
        }
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            ctrl: self.ctrl || rhs.ctrl,
            alt: self.alt || rhs.alt,
            shift: self.shift || rhs.shift,
            meta: self.meta || rhs.meta,
        }
    }
}

impl std::ops::BitAnd for Modifiers {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            ctrl: self.ctrl && rhs.ctrl,
            alt: self.alt && rhs.alt,
            shift: self.shift && rhs.shift,
            meta: self.meta && rhs.meta,
        }
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

// =============================================================================
// KeyCode
// =============================================================================

/// Key codes for keyboard keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum KeyCode {
    // Letters
    Char(char),

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Navigation
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,

    // Editing
    Backspace,
    Delete,
    Insert,
    Enter,
    Tab,
    Space,
    Escape,

    // Numpad
    NumpadEnter,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,

    // Media
    MediaPlayPause,
    MediaStop,
    MediaNext,
    MediaPrevious,
    VolumeUp,
    VolumeDown,
    VolumeMute,

    // Other
    PrintScreen,
    ScrollLock,
    Pause,
    Menu,

    // Unknown
    Unknown,
}

impl KeyCode {
    /// Check if this is a printable character
    pub fn is_printable(&self) -> bool {
        matches!(self, KeyCode::Char(_) | KeyCode::Space)
    }

    /// Check if this is a function key
    pub fn is_function_key(&self) -> bool {
        matches!(
            self,
            KeyCode::F1
                | KeyCode::F2
                | KeyCode::F3
                | KeyCode::F4
                | KeyCode::F5
                | KeyCode::F6
                | KeyCode::F7
                | KeyCode::F8
                | KeyCode::F9
                | KeyCode::F10
                | KeyCode::F11
                | KeyCode::F12
        )
    }

    /// Check if this is a navigation key
    pub fn is_navigation(&self) -> bool {
        matches!(
            self,
            KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
        )
    }

    /// Check if this is a modifier key (these don't produce KeyCode events typically)
    pub fn is_modifier(&self) -> bool {
        false // Modifiers are handled separately
    }

    /// Get the character for this key code (if applicable)
    pub fn as_char(&self) -> Option<char> {
        match self {
            KeyCode::Char(c) => Some(*c),
            KeyCode::Space => Some(' '),
            KeyCode::Tab => Some('\t'),
            KeyCode::Enter => Some('\n'),
            _ => None,
        }
    }

    /// Display string for this key
    pub fn display(&self) -> String {
        match self {
            KeyCode::Char(c) => c.to_uppercase().to_string(),
            KeyCode::F1 => "F1".to_string(),
            KeyCode::F2 => "F2".to_string(),
            KeyCode::F3 => "F3".to_string(),
            KeyCode::F4 => "F4".to_string(),
            KeyCode::F5 => "F5".to_string(),
            KeyCode::F6 => "F6".to_string(),
            KeyCode::F7 => "F7".to_string(),
            KeyCode::F8 => "F8".to_string(),
            KeyCode::F9 => "F9".to_string(),
            KeyCode::F10 => "F10".to_string(),
            KeyCode::F11 => "F11".to_string(),
            KeyCode::F12 => "F12".to_string(),
            KeyCode::Up => "↑".to_string(),
            KeyCode::Down => "↓".to_string(),
            KeyCode::Left => "←".to_string(),
            KeyCode::Right => "→".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PgUp".to_string(),
            KeyCode::PageDown => "PgDn".to_string(),
            KeyCode::Backspace => "⌫".to_string(),
            KeyCode::Delete => "Del".to_string(),
            KeyCode::Insert => "Ins".to_string(),
            KeyCode::Enter => "↵".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::Space => "Space".to_string(),
            KeyCode::Escape => "Esc".to_string(),
            KeyCode::NumpadEnter => "Num↵".to_string(),
            KeyCode::NumpadAdd => "Num+".to_string(),
            KeyCode::NumpadSubtract => "Num-".to_string(),
            KeyCode::NumpadMultiply => "Num*".to_string(),
            KeyCode::NumpadDivide => "Num/".to_string(),
            KeyCode::Numpad0 => "Num0".to_string(),
            KeyCode::Numpad1 => "Num1".to_string(),
            KeyCode::Numpad2 => "Num2".to_string(),
            KeyCode::Numpad3 => "Num3".to_string(),
            KeyCode::Numpad4 => "Num4".to_string(),
            KeyCode::Numpad5 => "Num5".to_string(),
            KeyCode::Numpad6 => "Num6".to_string(),
            KeyCode::Numpad7 => "Num7".to_string(),
            KeyCode::Numpad8 => "Num8".to_string(),
            KeyCode::Numpad9 => "Num9".to_string(),
            KeyCode::MediaPlayPause => "Play/Pause".to_string(),
            KeyCode::MediaStop => "Stop".to_string(),
            KeyCode::MediaNext => "Next".to_string(),
            KeyCode::MediaPrevious => "Prev".to_string(),
            KeyCode::VolumeUp => "Vol+".to_string(),
            KeyCode::VolumeDown => "Vol-".to_string(),
            KeyCode::VolumeMute => "Mute".to_string(),
            KeyCode::PrintScreen => "PrtSc".to_string(),
            KeyCode::ScrollLock => "ScrLk".to_string(),
            KeyCode::Pause => "Pause".to_string(),
            KeyCode::Menu => "Menu".to_string(),
            KeyCode::Unknown => "?".to_string(),
        }
    }

    /// Parse a key code from a string
    pub fn parse(s: &str) -> Self {
        let s = s.trim();

        // Check for single character
        if s.len() == 1 {
            return KeyCode::Char(s.chars().next().unwrap().to_ascii_lowercase());
        }

        // Check for named keys
        match s.to_lowercase().as_str() {
            "f1" => KeyCode::F1,
            "f2" => KeyCode::F2,
            "f3" => KeyCode::F3,
            "f4" => KeyCode::F4,
            "f5" => KeyCode::F5,
            "f6" => KeyCode::F6,
            "f7" => KeyCode::F7,
            "f8" => KeyCode::F8,
            "f9" => KeyCode::F9,
            "f10" => KeyCode::F10,
            "f11" => KeyCode::F11,
            "f12" => KeyCode::F12,
            "up" | "arrowup" => KeyCode::Up,
            "down" | "arrowdown" => KeyCode::Down,
            "left" | "arrowleft" => KeyCode::Left,
            "right" | "arrowright" => KeyCode::Right,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "pgup" => KeyCode::PageUp,
            "pagedown" | "pgdn" | "pgdown" => KeyCode::PageDown,
            "backspace" | "back" => KeyCode::Backspace,
            "delete" | "del" => KeyCode::Delete,
            "insert" | "ins" => KeyCode::Insert,
            "enter" | "return" => KeyCode::Enter,
            "tab" => KeyCode::Tab,
            "space" | " " => KeyCode::Space,
            "escape" | "esc" => KeyCode::Escape,
            "printscreen" | "prtsc" | "print" => KeyCode::PrintScreen,
            "scrolllock" | "scrlk" => KeyCode::ScrollLock,
            "pause" | "break" => KeyCode::Pause,
            "menu" | "contextmenu" => KeyCode::Menu,
            _ => KeyCode::Unknown,
        }
    }
}

impl fmt::Display for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl Default for KeyCode {
    fn default() -> Self {
        KeyCode::Unknown
    }
}

// =============================================================================
// KeyCombination
// =============================================================================

/// A key combination (key + modifiers)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombination {
    /// The key code
    pub key: KeyCode,
    /// The modifier keys
    pub modifiers: Modifiers,
}

impl KeyCombination {
    /// Create a new key combination
    pub fn new(key: KeyCode, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Create a combination with just a key (no modifiers)
    pub fn key_only(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: Modifiers::NONE,
        }
    }

    /// Create a combination from a character with modifiers
    pub fn char_with_mods(c: char, modifiers: Modifiers) -> Self {
        Self {
            key: KeyCode::Char(c.to_ascii_lowercase()),
            modifiers,
        }
    }

    /// Parse from a string like "Ctrl+Shift+P"
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();

        if parts.is_empty() {
            return None;
        }

        // Last part is the key
        let key_str = parts.last()?;
        let key = KeyCode::parse(key_str);

        // Everything else is modifiers
        let mod_str = parts[..parts.len() - 1].join("+");
        let modifiers = Modifiers::parse(&mod_str);

        Some(Self { key, modifiers })
    }

    /// Get the display string
    pub fn display(&self) -> String {
        if self.modifiers.is_empty() {
            self.key.display()
        } else {
            format!("{}+{}", self.modifiers.display(), self.key.display())
        }
    }

    /// Check if this combination has any modifiers
    pub fn has_modifiers(&self) -> bool {
        self.modifiers.any()
    }

    /// Check if this is a simple character without modifiers (or only shift)
    pub fn is_text_input(&self) -> bool {
        self.key.is_printable() && (!self.modifiers.any() || self.modifiers.only_shift())
    }
}

impl Default for KeyCombination {
    fn default() -> Self {
        Self {
            key: KeyCode::Unknown,
            modifiers: Modifiers::NONE,
        }
    }
}

impl fmt::Display for KeyCombination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

// =============================================================================
// KeySequence
// =============================================================================

/// A sequence of key combinations (for chords like "Ctrl+K Ctrl+C")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct KeySequence {
    /// The key combinations in order
    combinations: Vec<KeyCombination>,
}

impl KeySequence {
    /// Create a new empty sequence
    pub fn new(combinations: Vec<KeyCombination>) -> Self {
        Self { combinations }
    }

    /// Create a single-key sequence
    pub fn single(key: KeyCode, modifiers: Modifiers) -> Self {
        Self {
            combinations: vec![KeyCombination::new(key, modifiers)],
        }
    }

    /// Create a two-key chord sequence
    pub fn chord(first: KeyCombination, second: KeyCombination) -> Self {
        Self {
            combinations: vec![first, second],
        }
    }

    /// Parse from a string like "Ctrl+K Ctrl+C" (space-separated)
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        let mut combinations = Vec::new();

        for part in parts {
            let combo = KeyCombination::parse(part)?;
            combinations.push(combo);
        }

        if combinations.is_empty() {
            None
        } else {
            Some(Self { combinations })
        }
    }

    /// Get the length of the sequence
    pub fn len(&self) -> usize {
        self.combinations.len()
    }

    /// Check if the sequence is empty
    pub fn is_empty(&self) -> bool {
        self.combinations.is_empty()
    }

    /// Get the first combination
    pub fn first(&self) -> Option<&KeyCombination> {
        self.combinations.first()
    }

    /// Get the last combination
    pub fn last(&self) -> Option<&KeyCombination> {
        self.combinations.last()
    }

    /// Get a combination by index
    pub fn get(&self, index: usize) -> Option<&KeyCombination> {
        self.combinations.get(index)
    }

    /// Check if this sequence starts with the given combination
    pub fn starts_with(&self, combination: &KeyCombination) -> bool {
        self.combinations.first() == Some(combination)
    }

    /// Check if this sequence starts with another sequence
    pub fn starts_with_sequence(&self, other: &KeySequence) -> bool {
        if other.len() > self.len() {
            return false;
        }

        self.combinations
            .iter()
            .zip(other.combinations.iter())
            .all(|(a, b)| a == b)
    }

    /// Get the display string
    pub fn display(&self) -> String {
        self.combinations
            .iter()
            .map(|c| c.display())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Add a combination to the sequence
    pub fn push(&mut self, combination: KeyCombination) {
        self.combinations.push(combination);
    }

    /// Get iterator over combinations
    pub fn iter(&self) -> impl Iterator<Item = &KeyCombination> {
        self.combinations.iter()
    }
}

impl From<KeyCombination> for KeySequence {
    fn from(combination: KeyCombination) -> Self {
        Self {
            combinations: vec![combination],
        }
    }
}

impl fmt::Display for KeySequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifiers_none() {
        let mods = Modifiers::NONE;
        assert!(mods.is_empty());
        assert!(!mods.any());
        assert_eq!(mods.count(), 0);
    }

    #[test]
    fn test_modifiers_ctrl() {
        let mods = Modifiers::CTRL;
        assert!(!mods.is_empty());
        assert!(mods.any());
        assert!(mods.ctrl);
        assert!(!mods.alt);
        assert_eq!(mods.count(), 1);
    }

    #[test]
    fn test_modifiers_bitor() {
        let mods = Modifiers::CTRL | Modifiers::SHIFT;
        assert!(mods.ctrl);
        assert!(mods.shift);
        assert!(!mods.alt);
        assert_eq!(mods.count(), 2);
    }

    #[test]
    fn test_modifiers_bitand() {
        let mods1 = Modifiers::CTRL | Modifiers::SHIFT;
        let mods2 = Modifiers::CTRL | Modifiers::ALT;
        let result = mods1 & mods2;
        assert!(result.ctrl);
        assert!(!result.shift);
        assert!(!result.alt);
    }

    #[test]
    fn test_modifiers_display() {
        let mods = Modifiers::CTRL | Modifiers::SHIFT;
        let display = mods.display();
        assert!(display.contains("Ctrl"));
        assert!(display.contains("Shift"));
    }

    #[test]
    fn test_modifiers_parse() {
        let mods = Modifiers::parse("Ctrl+Shift");
        assert!(mods.ctrl);
        assert!(mods.shift);
        assert!(!mods.alt);

        let mods2 = Modifiers::parse("Control+Alt");
        assert!(mods2.ctrl);
        assert!(mods2.alt);
    }

    #[test]
    fn test_modifiers_only_shift() {
        assert!(Modifiers::SHIFT.only_shift());
        assert!(!Modifiers::CTRL.only_shift());
        assert!(!(Modifiers::CTRL | Modifiers::SHIFT).only_shift());
    }

    #[test]
    fn test_keycode_char() {
        let key = KeyCode::Char('a');
        assert!(key.is_printable());
        assert!(!key.is_function_key());
        assert_eq!(key.as_char(), Some('a'));
    }

    #[test]
    fn test_keycode_function() {
        let key = KeyCode::F5;
        assert!(key.is_function_key());
        assert!(!key.is_printable());
        assert!(!key.is_navigation());
    }

    #[test]
    fn test_keycode_navigation() {
        let key = KeyCode::Up;
        assert!(key.is_navigation());
        assert!(!key.is_function_key());
    }

    #[test]
    fn test_keycode_display() {
        assert_eq!(KeyCode::Char('a').display(), "A");
        assert_eq!(KeyCode::F5.display(), "F5");
        assert_eq!(KeyCode::Escape.display(), "Esc");
        assert_eq!(KeyCode::Enter.display(), "↵");
    }

    #[test]
    fn test_keycode_parse() {
        assert_eq!(KeyCode::parse("a"), KeyCode::Char('a'));
        assert_eq!(KeyCode::parse("F5"), KeyCode::F5);
        assert_eq!(KeyCode::parse("escape"), KeyCode::Escape);
        assert_eq!(KeyCode::parse("enter"), KeyCode::Enter);
        assert_eq!(KeyCode::parse("space"), KeyCode::Space);
    }

    #[test]
    fn test_key_combination_new() {
        let combo = KeyCombination::new(KeyCode::Char('s'), Modifiers::CTRL);
        assert_eq!(combo.key, KeyCode::Char('s'));
        assert!(combo.modifiers.ctrl);
    }

    #[test]
    fn test_key_combination_parse() {
        let combo = KeyCombination::parse("Ctrl+S").unwrap();
        assert_eq!(combo.key, KeyCode::Char('s'));
        assert!(combo.modifiers.ctrl);

        let combo2 = KeyCombination::parse("Ctrl+Shift+P").unwrap();
        assert_eq!(combo2.key, KeyCode::Char('p'));
        assert!(combo2.modifiers.ctrl);
        assert!(combo2.modifiers.shift);
    }

    #[test]
    fn test_key_combination_display() {
        let combo = KeyCombination::new(KeyCode::Char('s'), Modifiers::CTRL);
        assert_eq!(combo.display(), "Ctrl+S");

        let combo2 = KeyCombination::key_only(KeyCode::Escape);
        assert_eq!(combo2.display(), "Esc");
    }

    #[test]
    fn test_key_combination_is_text_input() {
        let text = KeyCombination::key_only(KeyCode::Char('a'));
        assert!(text.is_text_input());

        let shifted = KeyCombination::new(KeyCode::Char('a'), Modifiers::SHIFT);
        assert!(shifted.is_text_input());

        let ctrl = KeyCombination::new(KeyCode::Char('a'), Modifiers::CTRL);
        assert!(!ctrl.is_text_input());
    }

    #[test]
    fn test_key_sequence_single() {
        let seq = KeySequence::single(KeyCode::Char('s'), Modifiers::CTRL);
        assert_eq!(seq.len(), 1);
        assert!(!seq.is_empty());
    }

    #[test]
    fn test_key_sequence_chord() {
        let first = KeyCombination::new(KeyCode::Char('k'), Modifiers::CTRL);
        let second = KeyCombination::new(KeyCode::Char('c'), Modifiers::CTRL);
        let seq = KeySequence::chord(first, second);
        assert_eq!(seq.len(), 2);
    }

    #[test]
    fn test_key_sequence_parse() {
        let seq = KeySequence::parse("Ctrl+K Ctrl+C").unwrap();
        assert_eq!(seq.len(), 2);

        let first = seq.first().unwrap();
        assert_eq!(first.key, KeyCode::Char('k'));
        assert!(first.modifiers.ctrl);

        let second = seq.get(1).unwrap();
        assert_eq!(second.key, KeyCode::Char('c'));
    }

    #[test]
    fn test_key_sequence_display() {
        let seq = KeySequence::parse("Ctrl+K Ctrl+C").unwrap();
        assert_eq!(seq.display(), "Ctrl+K Ctrl+C");
    }

    #[test]
    fn test_key_sequence_starts_with() {
        let seq = KeySequence::parse("Ctrl+K Ctrl+C").unwrap();
        let combo = KeyCombination::new(KeyCode::Char('k'), Modifiers::CTRL);

        assert!(seq.starts_with(&combo));

        let wrong = KeyCombination::new(KeyCode::Char('x'), Modifiers::CTRL);
        assert!(!seq.starts_with(&wrong));
    }

    #[test]
    fn test_key_sequence_starts_with_sequence() {
        let full = KeySequence::parse("Ctrl+K Ctrl+C").unwrap();
        let prefix = KeySequence::parse("Ctrl+K").unwrap();

        assert!(full.starts_with_sequence(&prefix));
        assert!(!prefix.starts_with_sequence(&full));
    }

    #[test]
    fn test_key_combination_from_into_sequence() {
        let combo = KeyCombination::new(KeyCode::Char('s'), Modifiers::CTRL);
        let seq: KeySequence = combo.into();
        assert_eq!(seq.len(), 1);
    }

    #[test]
    fn test_key_sequence_iter() {
        let seq = KeySequence::parse("Ctrl+A Ctrl+B").unwrap();
        let keys: Vec<_> = seq.iter().collect();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_modifiers_serialization() {
        let mods = Modifiers::CTRL | Modifiers::SHIFT;
        let json = serde_json::to_string(&mods).unwrap();
        let deserialized: Modifiers = serde_json::from_str(&json).unwrap();
        assert_eq!(mods, deserialized);
    }

    #[test]
    fn test_key_combination_serialization() {
        let combo = KeyCombination::new(KeyCode::Char('s'), Modifiers::CTRL);
        let json = serde_json::to_string(&combo).unwrap();
        let deserialized: KeyCombination = serde_json::from_str(&json).unwrap();
        assert_eq!(combo, deserialized);
    }

    #[test]
    fn test_key_sequence_serialization() {
        let seq = KeySequence::parse("Ctrl+K Ctrl+C").unwrap();
        let json = serde_json::to_string(&seq).unwrap();
        let deserialized: KeySequence = serde_json::from_str(&json).unwrap();
        assert_eq!(seq, deserialized);
    }
}

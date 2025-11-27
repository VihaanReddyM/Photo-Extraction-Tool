//! Actions Module - Keybinding Actions
//!
//! This module defines all the actions that can be triggered by keybindings.
//! Actions are organized into categories for easy discovery in the command palette.
//!
//! # Architecture
//!
//! Actions are designed to be:
//! - Serializable for configuration
//! - Categorizable for organization
//! - Self-describing for help/documentation
//!
//! # Example
//!
//! ```rust,no_run
//! use photo_extraction_tool::ui::keybindings::actions::{Action, ActionCategory};
//!
//! let action = Action::StartExtraction;
//! assert_eq!(action.category(), ActionCategory::Extraction);
//! assert!(!action.description().is_empty());
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

// =============================================================================
// Action
// =============================================================================

/// All possible actions that can be triggered by keybindings or commands
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Action {
    // =========================================================================
    // No Action
    // =========================================================================
    /// No action (placeholder)
    None,

    // =========================================================================
    // Application Actions
    // =========================================================================
    /// Quit the application
    Quit,

    /// Open settings/preferences
    OpenSettings,

    /// Open the keybinding configuration
    OpenKeybindings,

    /// Toggle between dark and light theme
    ToggleTheme,

    /// Show about dialog
    ShowAbout,

    /// Show help
    ShowHelp,

    /// Open documentation
    OpenDocumentation,

    /// Check for updates
    CheckForUpdates,

    /// Toggle fullscreen mode
    ToggleFullscreen,

    /// Minimize window
    MinimizeWindow,

    /// Maximize/restore window
    ToggleMaximize,

    // =========================================================================
    // Command Palette
    // =========================================================================
    /// Open the command palette
    OpenCommandPalette,

    /// Close the command palette (or any modal)
    CloseModal,

    /// Go to next item in palette/list
    PaletteNext,

    /// Go to previous item in palette/list
    PalettePrevious,

    /// Select current item in palette
    PaletteSelect,

    // =========================================================================
    // Device Actions
    // =========================================================================
    /// Refresh device list
    RefreshDevices,

    /// Select next device in list
    SelectNextDevice,

    /// Select previous device in list
    SelectPreviousDevice,

    /// Show device details
    ShowDeviceDetails,

    /// Eject/disconnect device
    EjectDevice,

    /// Trust device (iOS trust dialog)
    TrustDevice,

    // =========================================================================
    // Extraction Actions
    // =========================================================================
    /// Start extraction
    StartExtraction,

    /// Stop/cancel extraction
    StopExtraction,

    /// Pause extraction
    PauseExtraction,

    /// Resume extraction
    ResumeExtraction,

    /// Toggle pause/resume
    TogglePause,

    /// Open output folder
    OpenOutputFolder,

    /// Change output folder
    ChangeOutputFolder,

    /// Show extraction history
    ShowExtractionHistory,

    /// Clear extraction history
    ClearExtractionHistory,

    // =========================================================================
    // Navigation Actions
    // =========================================================================
    /// Focus device list panel
    FocusDeviceList,

    /// Focus preview panel
    FocusPreview,

    /// Focus progress panel
    FocusProgress,

    /// Focus status bar
    FocusStatusBar,

    /// Focus next panel
    FocusNextPanel,

    /// Focus previous panel
    FocusPreviousPanel,

    /// Go back (navigation history)
    GoBack,

    /// Go forward (navigation history)
    GoForward,

    // =========================================================================
    // Panel Actions
    // =========================================================================
    /// Toggle device list panel visibility
    ToggleDeviceListPanel,

    /// Toggle preview panel visibility
    TogglePreviewPanel,

    /// Toggle progress panel visibility
    ToggleProgressPanel,

    /// Toggle log panel visibility
    ToggleLogPanel,

    /// Close current panel
    ClosePanel,

    /// Reset panel layout to default
    ResetPanelLayout,

    // =========================================================================
    // Selection Actions
    // =========================================================================
    /// Select all items
    SelectAll,

    /// Deselect all items
    DeselectAll,

    /// Invert selection
    InvertSelection,

    /// Select next item
    SelectNext,

    /// Select previous item
    SelectPrevious,

    /// Extend selection to next
    ExtendSelectionNext,

    /// Extend selection to previous
    ExtendSelectionPrevious,

    /// Select first item
    SelectFirst,

    /// Select last item
    SelectLast,

    // =========================================================================
    // View Actions
    // =========================================================================
    /// Zoom in
    ZoomIn,

    /// Zoom out
    ZoomOut,

    /// Reset zoom to 100%
    ZoomReset,

    /// Fit to window
    ZoomFit,

    /// Toggle grid view
    ToggleGridView,

    /// Toggle list view
    ToggleListView,

    /// Sort by name
    SortByName,

    /// Sort by date
    SortByDate,

    /// Sort by size
    SortBySize,

    /// Toggle sort order (ascending/descending)
    ToggleSortOrder,

    /// Show only photos
    FilterPhotosOnly,

    /// Show only videos
    FilterVideosOnly,

    /// Show all media
    FilterShowAll,

    // =========================================================================
    // Configuration Actions
    // =========================================================================
    /// Toggle DCIM only mode
    ToggleDcimOnly,

    /// Toggle duplicate detection
    ToggleDuplicateDetection,

    /// Toggle preserve folder structure
    TogglePreserveStructure,

    /// Toggle skip existing files
    ToggleSkipExisting,

    /// Open config file
    OpenConfigFile,

    /// Reload configuration
    ReloadConfig,

    /// Reset config to defaults
    ResetConfigDefaults,

    // =========================================================================
    // Clipboard Actions
    // =========================================================================
    /// Copy selected items path
    CopyPath,

    /// Copy selection info
    CopyInfo,

    // =========================================================================
    // Debug Actions
    // =========================================================================
    /// Toggle debug mode
    ToggleDebugMode,

    /// Show debug info
    ShowDebugInfo,

    /// Run test scenario
    RunTestScenario,

    /// Export logs
    ExportLogs,

    // =========================================================================
    // Custom Actions
    // =========================================================================
    /// Custom action with a string identifier
    Custom(String),
}

impl Action {
    /// Get the category for this action
    pub fn category(&self) -> ActionCategory {
        match self {
            Action::None => ActionCategory::None,

            // Application
            Action::Quit
            | Action::OpenSettings
            | Action::OpenKeybindings
            | Action::ToggleTheme
            | Action::ShowAbout
            | Action::ShowHelp
            | Action::OpenDocumentation
            | Action::CheckForUpdates
            | Action::ToggleFullscreen
            | Action::MinimizeWindow
            | Action::ToggleMaximize => ActionCategory::Application,

            // Command Palette
            Action::OpenCommandPalette
            | Action::CloseModal
            | Action::PaletteNext
            | Action::PalettePrevious
            | Action::PaletteSelect => ActionCategory::CommandPalette,

            // Device
            Action::RefreshDevices
            | Action::SelectNextDevice
            | Action::SelectPreviousDevice
            | Action::ShowDeviceDetails
            | Action::EjectDevice
            | Action::TrustDevice => ActionCategory::Device,

            // Extraction
            Action::StartExtraction
            | Action::StopExtraction
            | Action::PauseExtraction
            | Action::ResumeExtraction
            | Action::TogglePause
            | Action::OpenOutputFolder
            | Action::ChangeOutputFolder
            | Action::ShowExtractionHistory
            | Action::ClearExtractionHistory => ActionCategory::Extraction,

            // Navigation
            Action::FocusDeviceList
            | Action::FocusPreview
            | Action::FocusProgress
            | Action::FocusStatusBar
            | Action::FocusNextPanel
            | Action::FocusPreviousPanel
            | Action::GoBack
            | Action::GoForward => ActionCategory::Navigation,

            // Panel
            Action::ToggleDeviceListPanel
            | Action::TogglePreviewPanel
            | Action::ToggleProgressPanel
            | Action::ToggleLogPanel
            | Action::ClosePanel
            | Action::ResetPanelLayout => ActionCategory::Panel,

            // Selection
            Action::SelectAll
            | Action::DeselectAll
            | Action::InvertSelection
            | Action::SelectNext
            | Action::SelectPrevious
            | Action::ExtendSelectionNext
            | Action::ExtendSelectionPrevious
            | Action::SelectFirst
            | Action::SelectLast => ActionCategory::Selection,

            // View
            Action::ZoomIn
            | Action::ZoomOut
            | Action::ZoomReset
            | Action::ZoomFit
            | Action::ToggleGridView
            | Action::ToggleListView
            | Action::SortByName
            | Action::SortByDate
            | Action::SortBySize
            | Action::ToggleSortOrder
            | Action::FilterPhotosOnly
            | Action::FilterVideosOnly
            | Action::FilterShowAll => ActionCategory::View,

            // Configuration
            Action::ToggleDcimOnly
            | Action::ToggleDuplicateDetection
            | Action::TogglePreserveStructure
            | Action::ToggleSkipExisting
            | Action::OpenConfigFile
            | Action::ReloadConfig
            | Action::ResetConfigDefaults => ActionCategory::Configuration,

            // Clipboard
            Action::CopyPath | Action::CopyInfo => ActionCategory::Clipboard,

            // Debug
            Action::ToggleDebugMode
            | Action::ShowDebugInfo
            | Action::RunTestScenario
            | Action::ExportLogs => ActionCategory::Debug,

            // Custom
            Action::Custom(_) => ActionCategory::Custom,
        }
    }

    /// Get the human-readable description for this action
    pub fn description(&self) -> &'static str {
        match self {
            Action::None => "No action",

            // Application
            Action::Quit => "Quit application",
            Action::OpenSettings => "Open settings",
            Action::OpenKeybindings => "Open keybinding configuration",
            Action::ToggleTheme => "Toggle dark/light theme",
            Action::ShowAbout => "Show about dialog",
            Action::ShowHelp => "Show help",
            Action::OpenDocumentation => "Open documentation",
            Action::CheckForUpdates => "Check for updates",
            Action::ToggleFullscreen => "Toggle fullscreen",
            Action::MinimizeWindow => "Minimize window",
            Action::ToggleMaximize => "Maximize/restore window",

            // Command Palette
            Action::OpenCommandPalette => "Open command palette",
            Action::CloseModal => "Close modal/palette",
            Action::PaletteNext => "Next item",
            Action::PalettePrevious => "Previous item",
            Action::PaletteSelect => "Select item",

            // Device
            Action::RefreshDevices => "Refresh device list",
            Action::SelectNextDevice => "Select next device",
            Action::SelectPreviousDevice => "Select previous device",
            Action::ShowDeviceDetails => "Show device details",
            Action::EjectDevice => "Eject device",
            Action::TrustDevice => "Trust device",

            // Extraction
            Action::StartExtraction => "Start extraction",
            Action::StopExtraction => "Stop extraction",
            Action::PauseExtraction => "Pause extraction",
            Action::ResumeExtraction => "Resume extraction",
            Action::TogglePause => "Toggle pause/resume",
            Action::OpenOutputFolder => "Open output folder",
            Action::ChangeOutputFolder => "Change output folder",
            Action::ShowExtractionHistory => "Show extraction history",
            Action::ClearExtractionHistory => "Clear extraction history",

            // Navigation
            Action::FocusDeviceList => "Focus device list",
            Action::FocusPreview => "Focus preview",
            Action::FocusProgress => "Focus progress panel",
            Action::FocusStatusBar => "Focus status bar",
            Action::FocusNextPanel => "Focus next panel",
            Action::FocusPreviousPanel => "Focus previous panel",
            Action::GoBack => "Go back",
            Action::GoForward => "Go forward",

            // Panel
            Action::ToggleDeviceListPanel => "Toggle device list panel",
            Action::TogglePreviewPanel => "Toggle preview panel",
            Action::ToggleProgressPanel => "Toggle progress panel",
            Action::ToggleLogPanel => "Toggle log panel",
            Action::ClosePanel => "Close panel",
            Action::ResetPanelLayout => "Reset panel layout",

            // Selection
            Action::SelectAll => "Select all",
            Action::DeselectAll => "Deselect all",
            Action::InvertSelection => "Invert selection",
            Action::SelectNext => "Select next",
            Action::SelectPrevious => "Select previous",
            Action::ExtendSelectionNext => "Extend selection down",
            Action::ExtendSelectionPrevious => "Extend selection up",
            Action::SelectFirst => "Select first",
            Action::SelectLast => "Select last",

            // View
            Action::ZoomIn => "Zoom in",
            Action::ZoomOut => "Zoom out",
            Action::ZoomReset => "Reset zoom",
            Action::ZoomFit => "Fit to window",
            Action::ToggleGridView => "Grid view",
            Action::ToggleListView => "List view",
            Action::SortByName => "Sort by name",
            Action::SortByDate => "Sort by date",
            Action::SortBySize => "Sort by size",
            Action::ToggleSortOrder => "Toggle sort order",
            Action::FilterPhotosOnly => "Show photos only",
            Action::FilterVideosOnly => "Show videos only",
            Action::FilterShowAll => "Show all media",

            // Configuration
            Action::ToggleDcimOnly => "Toggle DCIM only",
            Action::ToggleDuplicateDetection => "Toggle duplicate detection",
            Action::TogglePreserveStructure => "Toggle preserve folder structure",
            Action::ToggleSkipExisting => "Toggle skip existing files",
            Action::OpenConfigFile => "Open config file",
            Action::ReloadConfig => "Reload configuration",
            Action::ResetConfigDefaults => "Reset to defaults",

            // Clipboard
            Action::CopyPath => "Copy path",
            Action::CopyInfo => "Copy info",

            // Debug
            Action::ToggleDebugMode => "Toggle debug mode",
            Action::ShowDebugInfo => "Show debug info",
            Action::RunTestScenario => "Run test scenario",
            Action::ExportLogs => "Export logs",

            // Custom
            Action::Custom(_) => "Custom action",
        }
    }

    /// Get the icon/emoji for this action (for display in command palette)
    pub fn icon(&self) -> &'static str {
        match self {
            Action::None => "",

            // Application
            Action::Quit => "🚪",
            Action::OpenSettings => "⚙️",
            Action::OpenKeybindings => "⌨️",
            Action::ToggleTheme => "🌓",
            Action::ShowAbout => "ℹ️",
            Action::ShowHelp => "❓",
            Action::OpenDocumentation => "📖",
            Action::CheckForUpdates => "🔄",
            Action::ToggleFullscreen => "⛶",
            Action::MinimizeWindow => "➖",
            Action::ToggleMaximize => "🗖",

            // Command Palette
            Action::OpenCommandPalette => "🎯",
            Action::CloseModal => "✕",
            Action::PaletteNext => "↓",
            Action::PalettePrevious => "↑",
            Action::PaletteSelect => "↵",

            // Device
            Action::RefreshDevices => "🔄",
            Action::SelectNextDevice => "↓",
            Action::SelectPreviousDevice => "↑",
            Action::ShowDeviceDetails => "📱",
            Action::EjectDevice => "⏏️",
            Action::TrustDevice => "🔐",

            // Extraction
            Action::StartExtraction => "▶️",
            Action::StopExtraction => "⏹️",
            Action::PauseExtraction => "⏸️",
            Action::ResumeExtraction => "▶️",
            Action::TogglePause => "⏯️",
            Action::OpenOutputFolder => "📂",
            Action::ChangeOutputFolder => "📁",
            Action::ShowExtractionHistory => "📜",
            Action::ClearExtractionHistory => "🗑️",

            // Navigation
            Action::FocusDeviceList => "📱",
            Action::FocusPreview => "🖼️",
            Action::FocusProgress => "📊",
            Action::FocusStatusBar => "📊",
            Action::FocusNextPanel => "→",
            Action::FocusPreviousPanel => "←",
            Action::GoBack => "←",
            Action::GoForward => "→",

            // Panel
            Action::ToggleDeviceListPanel => "📱",
            Action::TogglePreviewPanel => "🖼️",
            Action::ToggleProgressPanel => "📊",
            Action::ToggleLogPanel => "📋",
            Action::ClosePanel => "✕",
            Action::ResetPanelLayout => "⚡",

            // Selection
            Action::SelectAll => "☑️",
            Action::DeselectAll => "☐",
            Action::InvertSelection => "🔄",
            Action::SelectNext => "↓",
            Action::SelectPrevious => "↑",
            Action::ExtendSelectionNext => "⇓",
            Action::ExtendSelectionPrevious => "⇑",
            Action::SelectFirst => "⤒",
            Action::SelectLast => "⤓",

            // View
            Action::ZoomIn => "🔍",
            Action::ZoomOut => "🔍",
            Action::ZoomReset => "1:1",
            Action::ZoomFit => "⊡",
            Action::ToggleGridView => "⊞",
            Action::ToggleListView => "☰",
            Action::SortByName => "🔤",
            Action::SortByDate => "📅",
            Action::SortBySize => "📏",
            Action::ToggleSortOrder => "⇅",
            Action::FilterPhotosOnly => "📷",
            Action::FilterVideosOnly => "🎬",
            Action::FilterShowAll => "🎞️",

            // Configuration
            Action::ToggleDcimOnly => "📁",
            Action::ToggleDuplicateDetection => "🔍",
            Action::TogglePreserveStructure => "🌲",
            Action::ToggleSkipExisting => "⏭️",
            Action::OpenConfigFile => "📄",
            Action::ReloadConfig => "🔄",
            Action::ResetConfigDefaults => "↺",

            // Clipboard
            Action::CopyPath => "📋",
            Action::CopyInfo => "📋",

            // Debug
            Action::ToggleDebugMode => "🐛",
            Action::ShowDebugInfo => "🔬",
            Action::RunTestScenario => "🧪",
            Action::ExportLogs => "📤",

            // Custom
            Action::Custom(_) => "⚡",
        }
    }

    /// Check if this action requires a device to be connected
    pub fn requires_device(&self) -> bool {
        matches!(
            self,
            Action::StartExtraction
                | Action::ShowDeviceDetails
                | Action::EjectDevice
                | Action::TrustDevice
        )
    }

    /// Check if this action requires an active extraction
    pub fn requires_extraction(&self) -> bool {
        matches!(
            self,
            Action::StopExtraction
                | Action::PauseExtraction
                | Action::ResumeExtraction
                | Action::TogglePause
        )
    }

    /// Check if this action is a toggle
    pub fn is_toggle(&self) -> bool {
        matches!(
            self,
            Action::ToggleTheme
                | Action::ToggleFullscreen
                | Action::ToggleMaximize
                | Action::TogglePause
                | Action::ToggleDeviceListPanel
                | Action::TogglePreviewPanel
                | Action::ToggleProgressPanel
                | Action::ToggleLogPanel
                | Action::ToggleGridView
                | Action::ToggleListView
                | Action::ToggleSortOrder
                | Action::ToggleDcimOnly
                | Action::ToggleDuplicateDetection
                | Action::TogglePreserveStructure
                | Action::ToggleSkipExisting
                | Action::ToggleDebugMode
        )
    }

    /// Get all actions for a given category
    pub fn all_in_category(category: ActionCategory) -> Vec<Action> {
        ALL_ACTIONS
            .iter()
            .filter(|a| a.category() == category)
            .cloned()
            .collect()
    }

    /// Get all actions
    pub fn all() -> &'static [Action] {
        ALL_ACTIONS
    }
}

impl Default for Action {
    fn default() -> Self {
        Action::None
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

// =============================================================================
// Action Category
// =============================================================================

/// Categories for organizing actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionCategory {
    /// No category
    None,
    /// Application-level actions
    Application,
    /// Command palette actions
    CommandPalette,
    /// Device-related actions
    Device,
    /// Extraction-related actions
    Extraction,
    /// Navigation actions
    Navigation,
    /// Panel management actions
    Panel,
    /// Selection actions
    Selection,
    /// View/display actions
    View,
    /// Configuration actions
    Configuration,
    /// Clipboard actions
    Clipboard,
    /// Debug actions
    Debug,
    /// Custom/user-defined actions
    Custom,
}

impl ActionCategory {
    /// Get the display name for this category
    pub fn display_name(&self) -> &'static str {
        match self {
            ActionCategory::None => "None",
            ActionCategory::Application => "Application",
            ActionCategory::CommandPalette => "Command Palette",
            ActionCategory::Device => "Device",
            ActionCategory::Extraction => "Extraction",
            ActionCategory::Navigation => "Navigation",
            ActionCategory::Panel => "Panel",
            ActionCategory::Selection => "Selection",
            ActionCategory::View => "View",
            ActionCategory::Configuration => "Configuration",
            ActionCategory::Clipboard => "Clipboard",
            ActionCategory::Debug => "Debug",
            ActionCategory::Custom => "Custom",
        }
    }

    /// Get the icon for this category
    pub fn icon(&self) -> &'static str {
        match self {
            ActionCategory::None => "",
            ActionCategory::Application => "🖥️",
            ActionCategory::CommandPalette => "🎯",
            ActionCategory::Device => "📱",
            ActionCategory::Extraction => "📷",
            ActionCategory::Navigation => "🧭",
            ActionCategory::Panel => "📐",
            ActionCategory::Selection => "☑️",
            ActionCategory::View => "👁️",
            ActionCategory::Configuration => "⚙️",
            ActionCategory::Clipboard => "📋",
            ActionCategory::Debug => "🐛",
            ActionCategory::Custom => "⚡",
        }
    }

    /// Get all categories
    pub fn all() -> &'static [ActionCategory] {
        ALL_CATEGORIES
    }
}

impl Default for ActionCategory {
    fn default() -> Self {
        ActionCategory::None
    }
}

impl fmt::Display for ActionCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// =============================================================================
// Static Data
// =============================================================================

/// All action categories
static ALL_CATEGORIES: &[ActionCategory] = &[
    ActionCategory::Application,
    ActionCategory::CommandPalette,
    ActionCategory::Device,
    ActionCategory::Extraction,
    ActionCategory::Navigation,
    ActionCategory::Panel,
    ActionCategory::Selection,
    ActionCategory::View,
    ActionCategory::Configuration,
    ActionCategory::Clipboard,
    ActionCategory::Debug,
];

/// All defined actions (excluding Custom)
static ALL_ACTIONS: &[Action] = &[
    // Application
    Action::Quit,
    Action::OpenSettings,
    Action::OpenKeybindings,
    Action::ToggleTheme,
    Action::ShowAbout,
    Action::ShowHelp,
    Action::OpenDocumentation,
    Action::CheckForUpdates,
    Action::ToggleFullscreen,
    Action::MinimizeWindow,
    Action::ToggleMaximize,
    // Command Palette
    Action::OpenCommandPalette,
    Action::CloseModal,
    Action::PaletteNext,
    Action::PalettePrevious,
    Action::PaletteSelect,
    // Device
    Action::RefreshDevices,
    Action::SelectNextDevice,
    Action::SelectPreviousDevice,
    Action::ShowDeviceDetails,
    Action::EjectDevice,
    Action::TrustDevice,
    // Extraction
    Action::StartExtraction,
    Action::StopExtraction,
    Action::PauseExtraction,
    Action::ResumeExtraction,
    Action::TogglePause,
    Action::OpenOutputFolder,
    Action::ChangeOutputFolder,
    Action::ShowExtractionHistory,
    Action::ClearExtractionHistory,
    // Navigation
    Action::FocusDeviceList,
    Action::FocusPreview,
    Action::FocusProgress,
    Action::FocusStatusBar,
    Action::FocusNextPanel,
    Action::FocusPreviousPanel,
    Action::GoBack,
    Action::GoForward,
    // Panel
    Action::ToggleDeviceListPanel,
    Action::TogglePreviewPanel,
    Action::ToggleProgressPanel,
    Action::ToggleLogPanel,
    Action::ClosePanel,
    Action::ResetPanelLayout,
    // Selection
    Action::SelectAll,
    Action::DeselectAll,
    Action::InvertSelection,
    Action::SelectNext,
    Action::SelectPrevious,
    Action::ExtendSelectionNext,
    Action::ExtendSelectionPrevious,
    Action::SelectFirst,
    Action::SelectLast,
    // View
    Action::ZoomIn,
    Action::ZoomOut,
    Action::ZoomReset,
    Action::ZoomFit,
    Action::ToggleGridView,
    Action::ToggleListView,
    Action::SortByName,
    Action::SortByDate,
    Action::SortBySize,
    Action::ToggleSortOrder,
    Action::FilterPhotosOnly,
    Action::FilterVideosOnly,
    Action::FilterShowAll,
    // Configuration
    Action::ToggleDcimOnly,
    Action::ToggleDuplicateDetection,
    Action::TogglePreserveStructure,
    Action::ToggleSkipExisting,
    Action::OpenConfigFile,
    Action::ReloadConfig,
    Action::ResetConfigDefaults,
    // Clipboard
    Action::CopyPath,
    Action::CopyInfo,
    // Debug
    Action::ToggleDebugMode,
    Action::ShowDebugInfo,
    Action::RunTestScenario,
    Action::ExportLogs,
];

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_category() {
        assert_eq!(Action::Quit.category(), ActionCategory::Application);
        assert_eq!(
            Action::StartExtraction.category(),
            ActionCategory::Extraction
        );
        assert_eq!(Action::SelectAll.category(), ActionCategory::Selection);
        assert_eq!(Action::None.category(), ActionCategory::None);
    }

    #[test]
    fn test_action_description() {
        assert_eq!(Action::Quit.description(), "Quit application");
        assert_eq!(Action::StartExtraction.description(), "Start extraction");
        assert!(!Action::None.description().is_empty());
    }

    #[test]
    fn test_action_icon() {
        assert!(!Action::Quit.icon().is_empty());
        assert!(!Action::StartExtraction.icon().is_empty());
        assert!(Action::None.icon().is_empty());
    }

    #[test]
    fn test_action_requires_device() {
        assert!(Action::StartExtraction.requires_device());
        assert!(Action::ShowDeviceDetails.requires_device());
        assert!(!Action::Quit.requires_device());
    }

    #[test]
    fn test_action_requires_extraction() {
        assert!(Action::StopExtraction.requires_extraction());
        assert!(Action::PauseExtraction.requires_extraction());
        assert!(!Action::StartExtraction.requires_extraction());
    }

    #[test]
    fn test_action_is_toggle() {
        assert!(Action::ToggleTheme.is_toggle());
        assert!(Action::ToggleFullscreen.is_toggle());
        assert!(!Action::Quit.is_toggle());
    }

    #[test]
    fn test_action_all() {
        let all = Action::all();
        assert!(!all.is_empty());
        assert!(all.contains(&Action::Quit));
        assert!(all.contains(&Action::StartExtraction));
    }

    #[test]
    fn test_action_all_in_category() {
        let extraction = Action::all_in_category(ActionCategory::Extraction);
        assert!(extraction.contains(&Action::StartExtraction));
        assert!(extraction.contains(&Action::StopExtraction));
        assert!(!extraction.contains(&Action::Quit));
    }

    #[test]
    fn test_action_display() {
        assert_eq!(format!("{}", Action::Quit), "Quit application");
    }

    #[test]
    fn test_action_default() {
        assert_eq!(Action::default(), Action::None);
    }

    #[test]
    fn test_action_custom() {
        let custom = Action::Custom("my_action".to_string());
        assert_eq!(custom.category(), ActionCategory::Custom);
    }

    #[test]
    fn test_category_display_name() {
        assert_eq!(ActionCategory::Application.display_name(), "Application");
        assert_eq!(ActionCategory::Extraction.display_name(), "Extraction");
    }

    #[test]
    fn test_category_icon() {
        assert!(!ActionCategory::Application.icon().is_empty());
        assert!(!ActionCategory::Device.icon().is_empty());
    }

    #[test]
    fn test_category_all() {
        let all = ActionCategory::all();
        assert!(!all.is_empty());
        assert!(all.contains(&ActionCategory::Application));
        assert!(all.contains(&ActionCategory::Extraction));
        // None and Custom are not in the "all" list
        assert!(!all.contains(&ActionCategory::None));
    }

    #[test]
    fn test_category_display() {
        assert_eq!(format!("{}", ActionCategory::Application), "Application");
    }

    #[test]
    fn test_category_default() {
        assert_eq!(ActionCategory::default(), ActionCategory::None);
    }

    #[test]
    fn test_action_serialization() {
        let action = Action::StartExtraction;
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_action_custom_serialization() {
        let action = Action::Custom("test_action".to_string());
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_category_serialization() {
        let category = ActionCategory::Extraction;
        let json = serde_json::to_string(&category).unwrap();
        let deserialized: ActionCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(category, deserialized);
    }

    #[test]
    fn test_all_actions_have_description() {
        for action in Action::all() {
            assert!(
                !action.description().is_empty(),
                "Action {:?} has empty description",
                action
            );
        }
    }

    #[test]
    fn test_all_actions_have_valid_category() {
        for action in Action::all() {
            let category = action.category();
            assert_ne!(
                category,
                ActionCategory::None,
                "Action {:?} has None category",
                action
            );
        }
    }
}

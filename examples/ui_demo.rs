//! UI Integration Demo
//!
//! This example demonstrates how to use the Zed-style UI system components
//! together. It shows the integration of:
//!
//! - Theme management with dark/light modes
//! - Keybinding system with keyboard handling
//! - Command palette with fuzzy search
//! - Panel management with layout control
//! - UI components (buttons, inputs, lists, progress)
//! - Focus management for keyboard navigation
//!
//! Run with: `cargo run --example ui_demo`

use photo_extraction_tool::ui::{
    keybindings::{KeyCode, KeyCombination, Modifiers},
    theme::Color,
    // Components
    ButtonState,
    ButtonVariant,
    // Commands
    CommandPalette,
    CommandRegistry,
    FocusDirection,
    FocusManager,
    InputState,
    // Keybindings
    KeybindingContext,
    KeybindingManager,
    KeybindingResult,
    ListItem,
    ListState,
    // Panels
    PanelId,
    PanelManager,
    ProgressState,
    // Theme system
    Theme,
    ThemeBuilder,
    ThemeManager,
    ThemeMode,
    // Core UI app
    UiApp,
    WidgetId,
};

fn main() {
    println!("=== Photo Extraction Tool - UI Demo ===\n");

    // Demo 1: Theme System
    demo_themes();

    // Demo 2: Keybindings
    demo_keybindings();

    // Demo 3: Command Palette
    demo_command_palette();

    // Demo 4: Panel Management
    demo_panels();

    // Demo 5: UI Components
    demo_components();

    // Demo 6: Focus Management
    demo_focus();

    // Demo 7: Full Integration with UiApp
    demo_full_integration();

    println!("\n=== Demo Complete ===");
}

fn demo_themes() {
    println!("--- Theme System Demo ---\n");

    // Use built-in themes
    let dark = Theme::dark();
    let light = Theme::light();

    println!("Dark theme: {} (mode: {})", dark.name, dark.mode);
    println!("Light theme: {} (mode: {})", light.name, light.mode);

    // Create custom theme
    let custom = ThemeBuilder::new()
        .name("My Custom Theme")
        .mode(ThemeMode::Dark)
        .accent_color(Color::from_rgb(147, 51, 234)) // Purple
        .font_size(14.0)
        .font_family("Inter")
        .build();

    println!("Custom theme: {} (accent: purple)", custom.name);

    // Theme manager
    let mut manager = ThemeManager::new();
    manager.add_theme(custom);

    println!("Available themes: {:?}", manager.available_themes());

    // Toggle theme mode
    let initial_mode = manager.current().mode;
    manager.toggle_mode();
    println!(
        "Toggled theme mode: {:?} -> {:?}",
        initial_mode,
        manager.current().mode
    );

    println!();
}

fn demo_keybindings() {
    println!("--- Keybinding System Demo ---\n");

    let mut kb_manager = KeybindingManager::new();

    // The registry is pre-populated with default bindings
    let registry = kb_manager.registry();
    println!("Registered keybindings: {}", registry.len());

    // Simulate key presses
    let test_keys = vec![
        (KeyCode::Char('p'), Modifiers::CTRL_SHIFT, "Ctrl+Shift+P"),
        (KeyCode::Char('e'), Modifiers::CTRL, "Ctrl+E"),
        (KeyCode::Escape, Modifiers::NONE, "Escape"),
        (KeyCode::F1, Modifiers::NONE, "F1"),
    ];

    for (keycode, modifiers, display) in test_keys {
        let combo = KeyCombination::new(keycode, modifiers);
        let result = kb_manager.handle_key(combo);

        match result {
            KeybindingResult::Match { action, .. } => {
                println!("{}: {:?}", display, action);
            }
            KeybindingResult::Pending {
                possible_completions,
            } => {
                println!("{}: Pending ({} possible)", display, possible_completions);
            }
            KeybindingResult::NoMatch => {
                println!("{}: No match", display);
            }
        }
    }

    // Context switching
    println!("\nContext switching:");
    println!("Current context: {:?}", kb_manager.context());

    kb_manager.push_context(KeybindingContext::Preview);
    println!("After push: {:?}", kb_manager.context());

    kb_manager.pop_context();
    println!("After pop: {:?}", kb_manager.context());

    println!();
}

fn demo_command_palette() {
    println!("--- Command Palette Demo ---\n");

    // Create and populate registry
    let mut registry = CommandRegistry::new();
    registry.register_from_actions();

    println!("Registered commands: {}", registry.len());

    // Create palette
    let mut palette = CommandPalette::with_registry(registry);

    // Open and search
    palette.open();
    println!("Palette opened: {}", palette.is_open());

    // Test different search queries
    let queries = vec!["extract", "device", "theme", "toggle"];

    for query in queries {
        palette.set_query(query);
        let matches = palette.matches();
        println!("Query '{}': {} results", query, matches.len());

        // Show top 3 results
        for (i, m) in matches.iter().take(3).enumerate() {
            println!("  {}. {} (score: {:.2})", i + 1, m.command.label, m.score);
        }
    }

    // Selection and execution
    palette.set_query("extract");
    palette.select_next();
    palette.select_next();
    println!("\nSelected index: {:?}", palette.selected_index());

    if let Some(action) = palette.execute_selected() {
        println!("Executed: {:?}", action);
    }

    // Close palette
    palette.close();
    println!("Palette closed: {}", !palette.is_open());

    println!();
}

fn demo_panels() {
    println!("--- Panel Management Demo ---\n");

    let mut panels = PanelManager::new();

    // List all panels
    println!("Available panels:");
    for panel_id in PanelId::all() {
        if let Some(panel) = panels.get(*panel_id) {
            println!(
                "  {} - {:?} (visible: {})",
                panel.name, panel.position, panel.visible
            );
        }
    }

    // Toggle panels
    println!("\nToggling panels:");
    panels.toggle_panel(PanelId::DeviceList);
    println!(
        "Device list visible: {}",
        panels.is_visible(PanelId::DeviceList)
    );

    // Focus navigation
    panels.focus_panel(PanelId::Preview);
    println!("Focused panel: {:?}", panels.focused_id());

    panels.focus_next();
    println!("After focus_next: {:?}", panels.focused_id());

    panels.focus_previous();
    println!("After focus_previous: {:?}", panels.focused_id());

    // Layout manipulation
    let layout = panels.layout();
    println!(
        "\nLayout: left={:.0}px, right={:.0}px, bottom={:.0}px",
        layout.left_width, layout.right_width, layout.bottom_height
    );

    panels.toggle_left_sidebar();
    println!("Left sidebar visible: {}", panels.layout().left_visible);

    // Panel counts
    println!(
        "\nTotal panels: {}, Visible: {}",
        panels.panel_count(),
        panels.visible_count()
    );

    println!();
}

fn demo_components() {
    println!("--- UI Components Demo ---\n");

    // Button
    let mut button = ButtonState::new("submit_btn", "Start Extraction")
        .variant(ButtonVariant::Primary)
        .icon("play")
        .tooltip("Begin extracting photos from device")
        .shortcut("Ctrl+E");

    println!("Button: '{}' ({:?})", button.label, button.variant);
    println!("  Clickable: {}", button.is_clickable());

    button.set_loading(true);
    println!("  Loading: clickable={}", button.is_clickable());

    // Input
    let mut input = InputState::new("search_input")
        .placeholder("Search files...")
        .label("Search")
        .max_length(100)
        .clearable(true);

    input.set_value("DCIM/");
    input.insert("Camera");
    println!("\nInput: '{}'", input.value);

    input.select_all();
    println!("  Selected: {:?}", input.selected_text());

    // List
    let mut list = ListState::new("file_list")
        .selection_mode(photo_extraction_tool::ui::components::SelectionMode::Multiple)
        .show_icons(true);

    list.add_items(vec![
        ListItem::new("1", "IMG_0001.jpg")
            .icon("image")
            .description("2.4 MB")
            .badge("New"),
        ListItem::new("2", "IMG_0002.jpg")
            .icon("image")
            .description("3.1 MB"),
        ListItem::new("3", "VID_0001.mov")
            .icon("video")
            .description("124 MB"),
    ]);

    println!("\nList items: {}", list.len());

    list.select(0);
    list.select(2);
    println!("  Selected: {:?}", list.selected_ids());

    list.set_filter("IMG");
    println!(
        "  Filtered ('{}'): {} items",
        list.filter_query,
        list.filtered_items().len()
    );

    // Progress
    let mut progress = ProgressState::new("extraction_progress")
        .label("Extracting photos...")
        .show_percentage(true);

    progress.start();
    progress.update(42, 100);
    progress.update_bytes(420 * 1024 * 1024, 1024 * 1024 * 1024);
    progress.update_timing(10.5 * 1024.0 * 1024.0, Some(55.0), 30.0);

    println!("\nProgress: {}%", progress.percentage());
    println!("  Items: {}", progress.items_progress_string());
    println!("  Bytes: {}", progress.bytes_progress_string());
    println!("  Speed: {}", progress.speed_string());
    println!("  ETA: {}", progress.eta_string());

    println!();
}

fn demo_focus() {
    println!("--- Focus Management Demo ---\n");

    let mut focus = FocusManager::new();

    // Register widgets
    focus.register(WidgetId::new("search_input"), 0);
    focus.register(WidgetId::new("file_list"), 1);
    focus.register(WidgetId::new("extract_btn"), 2);
    focus.register(WidgetId::new("cancel_btn"), 3);

    println!("Focusable widgets: {}", focus.focusable_count());

    // Navigate focus
    focus.set_focus(WidgetId::new("search_input"));
    println!("Initial focus: {:?}", focus.focused());

    focus.move_focus(FocusDirection::Next);
    println!("After Next: {:?}", focus.focused());

    focus.move_focus(FocusDirection::Next);
    println!("After Next: {:?}", focus.focused());

    focus.move_focus(FocusDirection::Last);
    println!("After Last: {:?}", focus.focused());

    focus.focus_back();
    println!("After Back: {:?}", focus.focused());

    // Focus ring
    focus.set_show_focus_ring(true);
    println!("\nShow focus ring: {}", focus.should_show_focus_ring());

    println!();
}

fn demo_full_integration() {
    println!("--- Full Integration Demo (UiApp) ---\n");

    // Create the unified UI application
    let mut app = UiApp::new();

    println!("UiApp initialized:");
    println!("  Theme: {} ({:?})", app.theme().name, app.theme().mode);
    println!("  Context: {:?}", app.keybinding_context());
    println!("  Palette open: {}", app.is_palette_open());

    // Toggle theme
    app.toggle_theme_mode();
    println!("\nAfter theme toggle: {:?}", app.theme().mode);

    // Toggle palette
    app.toggle_palette();
    println!("After palette toggle: open={}", app.is_palette_open());
    app.toggle_palette();

    // Change context
    app.set_context(KeybindingContext::DeviceList);
    println!("After context change: {:?}", app.keybinding_context());

    // Handle a key
    let key = KeyCombination::new(KeyCode::Escape, Modifiers::NONE);
    if let Some(action) = app.handle_key(key) {
        println!("Key handled: {:?}", action);
    }

    // Apply settings
    {
        let settings = app.settings.settings_mut();
        settings.appearance.theme_mode = ThemeMode::Dark;
        settings.keyboard.chord_timeout_ms = 500;
        settings.behavior.palette_max_results = 15;
    }
    app.apply_settings();
    println!("\nSettings applied");

    // Save state
    app.save_state();
    println!("State saved to settings");

    // Final state
    println!("\nFinal state:");
    println!(
        "  Theme mode: {:?}",
        app.settings.settings().appearance.theme_mode
    );
    println!(
        "  Panel layout: left={:.0}px",
        app.settings.settings().panels.default_left_width
    );

    println!();
}

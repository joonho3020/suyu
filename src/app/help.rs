use eframe::egui;

pub(super) fn draw_help_window(ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("Help & Commands")
        .open(open)
        .resizable(true)
        .default_width(600.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Keyboard Shortcuts");
                ui.separator();

                ui.label("General");
                help_row(ui, "Space", "Open command palette");
                help_row(ui, "⌘⇧P", "Open command palette");
                help_row(ui, "⌘S", "Save diagram (JSON)");
                help_row(ui, "⌘⇧S", "Export as SVG");
                help_row(ui, "⌘O", "Open/load diagram");
                help_row(ui, "⌘Z", "Undo");
                help_row(ui, "⌘⇧Z / ⌘Y", "Redo");
                help_row(ui, "Escape", "Cancel current action / Select tool");

                ui.add_space(10.0);
                ui.label("Selection & Editing");
                help_row(ui, "⌘C", "Copy selected");
                help_row(ui, "⌘X", "Cut selected");
                help_row(ui, "⌘V", "Paste");
                help_row(ui, "⌘D", "Duplicate selected");
                help_row(ui, "Delete / Backspace", "Delete selected");
                help_row(ui, "Arrow keys", "Move selection or pan canvas");
                help_row(ui, "Shift + Arrow keys", "Move selection faster");
                help_row(ui, "Double-click", "Edit text inline");

                ui.add_space(10.0);
                ui.label("Tools");
                help_row(ui, "V", "Select tool");
                help_row(ui, "R", "Rectangle tool");
                help_row(ui, "O", "Ellipse tool");
                help_row(ui, "⇧T", "Triangle tool");
                help_row(ui, "⇧P", "Parallelogram tool");
                help_row(ui, "⇧Z", "Trapezoid tool");
                help_row(ui, "L", "Line tool");
                help_row(ui, "A", "Arrow tool");
                help_row(ui, "⇧A", "Bidirectional arrow tool");
                help_row(ui, "⇧L", "Polyline tool");
                help_row(ui, "P", "Pen (freehand) tool");
                help_row(ui, "T", "Text tool");
                help_row(ui, "Space (hold)", "Pan tool");

                ui.add_space(10.0);
                ui.label("Drawing");
                help_row(ui, "Shift + drag", "Constrain to axis or square");
                help_row(ui, "Right-click (polyline)", "Add point");
                help_row(ui, "Scroll wheel", "Zoom in/out");

                ui.add_space(20.0);
                ui.heading("Command Palette");
                ui.separator();
                ui.label("Press Space or ⌘⇧P to open the command palette.");
                ui.label("Type to search for commands, use arrow keys to navigate, Enter to execute.");
                ui.add_space(5.0);
                ui.label("Commands with '...' suffix prompt for input (size, color, etc.).");

                ui.add_space(20.0);
                ui.heading("Rich Text Formatting");
                ui.separator();
                ui.label("Use special syntax in text labels:");
                help_row(ui, "text_{sub}", "Subscript: text with subscript");
                help_row(ui, "text^{sup}", "Superscript: text with superscript");
                help_row(ui, "\\{  \\}", "Escape braces literally");
                help_row(ui, "\\_ \\^", "Escape underscore or caret");

                ui.add_space(20.0);
                ui.heading("Color Themes");
                ui.separator();
                ui.label("Define custom color themes in settings.toml:");
                ui.add_space(5.0);
                ui.code(r##"[[color_themes]]
name = "tokyonight"

[color_themes.colors]
darkblue = "#1a1b26"
green = "#9ece6a"
lightblue = "#7dcfff"
orange = "#ff9e64"
purple = "#bb9af7"
yellow = "#e0af68""##);
                ui.add_space(5.0);
                ui.label("Use theme color names when setting stroke/fill color in the command palette.");

                ui.add_space(20.0);
                ui.heading("Custom Fonts");
                ui.separator();
                ui.label("Add font_directory to settings.toml to load custom fonts:");
                ui.add_space(5.0);
                ui.code(r##"font_directory = "/path/to/fonts""##);
                ui.add_space(5.0);
                ui.label("Place .ttf or .otf files in that directory. They will be available as additional fonts.");

                ui.add_space(20.0);
                ui.heading("File Formats");
                ui.separator();
                ui.label("• Diagrams are saved as JSON files (.json)");
                ui.label("• Export to SVG for use in other applications");
                ui.label("• Settings are stored in settings.toml");

                ui.add_space(20.0);
                ui.heading("Tips");
                ui.separator();
                ui.label("• Hold Space and drag to pan the canvas");
                ui.label("• Use the right panel to edit properties of selected objects");
                ui.label("• Group objects with the Group command to move them together");
                ui.label("• Connect shapes with auto-connect to create dynamic connections");
                ui.label("• Enable 'Apply to selection' to change styles of multiple objects at once");
            });
        });
}

fn help_row(ui: &mut egui::Ui, shortcut: &str, description: &str) {
    ui.horizontal(|ui| {
        ui.add_sized([100.0, 16.0], egui::Label::new(
            egui::RichText::new(shortcut).monospace().strong()
        ));
        ui.label(description);
    });
}

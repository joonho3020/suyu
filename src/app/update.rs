use crate::model;
use eframe::egui;
use std::collections::HashSet;

use super::doc_ops::{
    AbutMode, AlignMode, DistributeMode, abut_selected, align_selected, distribute_selected,
    element_label,
};
use super::geometry::{compute_binding_for_target, resolve_binding_point, topmost_bind_target_id};
use super::render::{
    draw_background, draw_elements, draw_group_selection_boxes, draw_in_progress, style_editor,
    tool_button,
};
use super::command_palette::CommandContext;
use super::{DiagramApp, InProgress, Tool};

impl eframe::App for DiagramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sync_bound_line_endpoints();
        let wants_keyboard = ctx.wants_keyboard_input();
        ctx.input_mut(|i| {
            // Handle system clipboard events (Cmd+C/V/X on macOS, Ctrl+C/V/X on other platforms)
            let mut copy_requested = false;
            let mut cut_requested = false;
            let mut paste_requested = false;

            if !wants_keyboard && !self.inline_text_editing && !self.command_palette.open {
                for event in &i.events {
                    match event {
                        egui::Event::Copy => copy_requested = true,
                        egui::Event::Cut => cut_requested = true,
                        egui::Event::Paste(_) => paste_requested = true,
                        _ => {}
                    }
                }
            }

            if copy_requested {
                self.copy_selected(ctx);
            }
            if cut_requested {
                self.cut_selected(ctx);
            }
            if paste_requested {
                self.paste();
            }

            if !self.command_palette.open
                && !self.inline_text_editing
                && i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P)
            {
                self.command_palette.open("");
            }
            if i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::S) {
                self.save_svg_dialog();
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::S) {
                self.save_json_dialog();
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::O) {
                self.open_json_dialog();
            }
            let skip_shortcuts = wants_keyboard || self.inline_text_editing || self.command_palette.open;

            if i.consume_key(egui::Modifiers::NONE, egui::Key::F1) {
                self.show_help = true;
            }
            if !skip_shortcuts {
                if i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                ) || i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)
                {
                    self.redo();
                } else if i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z) {
                    self.undo();
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                    self.tool = Tool::Select;
                    self.in_progress = None;
                    self.active_transform = None;
                    self.tool_before_pan = None;
                }
                // Note: Copy/Cut/Paste are handled via egui::Event::Copy/Cut/Paste above
                if i.consume_key(egui::Modifiers::COMMAND, egui::Key::D) {
                    self.duplicate_selected();
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)
                {
                    self.delete_selected();
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::V) {
                    self.tool = Tool::Select;
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::R) {
                    self.tool = Tool::Rectangle;
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::O) {
                    self.tool = Tool::Ellipse;
                }
                if i.consume_key(egui::Modifiers::SHIFT, egui::Key::T) {
                    self.tool = Tool::Triangle;
                }
                if i.consume_key(egui::Modifiers::SHIFT, egui::Key::P) {
                    self.tool = Tool::Parallelogram;
                }
                if i.consume_key(egui::Modifiers::SHIFT, egui::Key::Z) {
                    self.tool = Tool::Trapezoid;
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::L) {
                    self.tool = Tool::Line;
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::A) {
                    self.tool = Tool::Arrow;
                }
                if i.consume_key(egui::Modifiers::SHIFT, egui::Key::A) {
                    self.tool = Tool::BidirectionalArrow;
                }
                if i.consume_key(egui::Modifiers::SHIFT, egui::Key::L) {
                    self.tool = Tool::Polyline;
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::P) {
                    self.tool = Tool::Pen;
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::T) {
                    self.tool = Tool::Text;
                }
                let move_amount = if i.modifiers.shift {
                    self.move_step_fast
                } else {
                    self.move_step
                };
                let pan_amount = move_amount * self.view.zoom;
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
                    || i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowLeft)
                {
                    if !self.selected.is_empty() {
                        self.push_undo();
                        self.translate_selected(egui::vec2(-move_amount, 0.0));
                    } else {
                        self.view.pan_screen += egui::vec2(pan_amount, 0.0);
                    }
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                    || i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowRight)
                {
                    if !self.selected.is_empty() {
                        self.push_undo();
                        self.translate_selected(egui::vec2(move_amount, 0.0));
                    } else {
                        self.view.pan_screen += egui::vec2(-pan_amount, 0.0);
                    }
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
                    || i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowUp)
                {
                    if !self.selected.is_empty() {
                        self.push_undo();
                        self.translate_selected(egui::vec2(0.0, -move_amount));
                    } else {
                        self.view.pan_screen += egui::vec2(0.0, pan_amount);
                    }
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                    || i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowDown)
                {
                    if !self.selected.is_empty() {
                        self.push_undo();
                        self.translate_selected(egui::vec2(0.0, move_amount));
                    } else {
                        self.view.pan_screen += egui::vec2(0.0, -pan_amount);
                    }
                }
            }
        });

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        ui.label("Diagram name:");
                        ui.text_edit_singleline(&mut self.diagram_name);
                        ui.separator();
                        if ui.button("Open... (⌘O)").clicked() {
                            self.open_json_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Save JSON... (⌘S)").clicked() {
                            self.save_json_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Export SVG... (⌘⇧S)").clicked() {
                            self.save_svg_dialog();
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Quick save paths:");
                        ui.small("JSON:");
                        if ui.text_edit_singleline(&mut self.file_path).changed() {
                            self.persist_settings();
                        }
                        ui.small("SVG:");
                        if ui.text_edit_singleline(&mut self.svg_path).changed() {
                            self.persist_settings();
                        }
                        ui.horizontal(|ui| {
                            if ui.small_button("Quick Save JSON").clicked() {
                                self.save_to_path();
                                ui.close_menu();
                            }
                            if ui.small_button("Quick Save SVG").clicked() {
                                self.save_svg_to_path();
                                ui.close_menu();
                            }
                        });
                    });
                });
                ui.menu_button("Edit", |ui| {
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        if ui.add_enabled(!self.history.is_empty(), egui::Button::new("Undo (⌘Z)")).clicked() {
                            self.undo();
                            ui.close_menu();
                        }
                        if ui.add_enabled(!self.future.is_empty(), egui::Button::new("Redo (⌘⇧Z)")).clicked() {
                            self.redo();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.add_enabled(!self.selected.is_empty(), egui::Button::new("Cut (⌘X)")).clicked() {
                            self.cut_selected(ctx);
                            ui.close_menu();
                        }
                        if ui.add_enabled(!self.selected.is_empty(), egui::Button::new("Copy (⌘C)")).clicked() {
                            self.copy_selected(ctx);
                            ui.close_menu();
                        }
                        if ui.add_enabled(self.clipboard.is_some(), egui::Button::new("Paste (⌘V)")).clicked() {
                            self.paste();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.add_enabled(!self.selected.is_empty(), egui::Button::new("Duplicate (⌘D)")).clicked() {
                            self.duplicate_selected();
                            ui.close_menu();
                        }
                        if ui.add_enabled(!self.selected.is_empty(), egui::Button::new("Delete (Del)")).clicked() {
                            self.delete_selected();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Select All (⌘A)").clicked() {
                            for e in &self.doc.elements {
                                self.selected.insert(e.id);
                            }
                            ui.close_menu();
                        }
                        if ui.button("Deselect All").clicked() {
                            self.clear_selection();
                            ui.close_menu();
                        }
                    });
                });
                ui.menu_button("Object", |ui| {
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        ui.label("Arrange");
                        if ui.add_enabled(!self.selected.is_empty(), egui::Button::new("Bring to Front")).clicked() {
                            self.bring_selected_to_front();
                            ui.close_menu();
                        }
                        if ui.add_enabled(!self.selected.is_empty(), egui::Button::new("Send to Back")).clicked() {
                            self.send_selected_to_back();
                            ui.close_menu();
                        }
                        if ui.add_enabled(!self.selected.is_empty(), egui::Button::new("Move Up")).clicked() {
                            self.move_selected_layer_by(1);
                            ui.close_menu();
                        }
                        if ui.add_enabled(!self.selected.is_empty(), egui::Button::new("Move Down")).clicked() {
                            self.move_selected_layer_by(-1);
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Group");
                        if ui.add_enabled(self.selected.len() >= 2, egui::Button::new("Group")).clicked() {
                            self.group_selected();
                            ui.close_menu();
                        }
                        let can_ungroup = self.selected.iter().any(|id| self.group_of(*id).is_some());
                        if ui.add_enabled(can_ungroup, egui::Button::new("Ungroup")).clicked() {
                            self.ungroup_selected();
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Align");
                        if ui.add_enabled(self.selected.len() >= 2, egui::Button::new("Align Left")).clicked() {
                            self.push_undo();
                            align_selected(&mut self.doc, &self.selected, AlignMode::Left);
                            ui.close_menu();
                        }
                        if ui.add_enabled(self.selected.len() >= 2, egui::Button::new("Align Center (H)")).clicked() {
                            self.push_undo();
                            align_selected(&mut self.doc, &self.selected, AlignMode::HCenter);
                            ui.close_menu();
                        }
                        if ui.add_enabled(self.selected.len() >= 2, egui::Button::new("Align Right")).clicked() {
                            self.push_undo();
                            align_selected(&mut self.doc, &self.selected, AlignMode::Right);
                            ui.close_menu();
                        }
                        if ui.add_enabled(self.selected.len() >= 2, egui::Button::new("Align Top")).clicked() {
                            self.push_undo();
                            align_selected(&mut self.doc, &self.selected, AlignMode::Top);
                            ui.close_menu();
                        }
                        if ui.add_enabled(self.selected.len() >= 2, egui::Button::new("Align Middle (V)")).clicked() {
                            self.push_undo();
                            align_selected(&mut self.doc, &self.selected, AlignMode::VCenter);
                            ui.close_menu();
                        }
                        if ui.add_enabled(self.selected.len() >= 2, egui::Button::new("Align Bottom")).clicked() {
                            self.push_undo();
                            align_selected(&mut self.doc, &self.selected, AlignMode::Bottom);
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Distribute");
                        if ui.add_enabled(self.selected.len() >= 2, egui::Button::new("Distribute Horizontally")).clicked() {
                            self.push_undo();
                            distribute_selected(&mut self.doc, &self.selected, DistributeMode::Horizontal);
                            ui.close_menu();
                        }
                        if ui.add_enabled(self.selected.len() >= 2, egui::Button::new("Distribute Vertically")).clicked() {
                            self.push_undo();
                            distribute_selected(&mut self.doc, &self.selected, DistributeMode::Vertical);
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Abut");
                        if ui.add_enabled(self.selected.len() >= 2, egui::Button::new("Abut Horizontally")).clicked() {
                            self.push_undo();
                            abut_selected(&mut self.doc, &self.selected, AbutMode::Horizontal);
                            ui.close_menu();
                        }
                        if ui.add_enabled(self.selected.len() >= 2, egui::Button::new("Abut Vertically")).clicked() {
                            self.push_undo();
                            abut_selected(&mut self.doc, &self.selected, AbutMode::Vertical);
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Connect");
                        if ui.add_enabled(self.selected.len() == 2, egui::Button::new("Connect (Line)")).clicked() {
                            self.auto_connect_selected(model::ArrowStyle::None);
                            ui.close_menu();
                        }
                        if ui.add_enabled(self.selected.len() == 2, egui::Button::new("Connect (Arrow)")).clicked() {
                            self.auto_connect_selected(model::ArrowStyle::End);
                            ui.close_menu();
                        }
                        if ui.add_enabled(self.selected.len() == 2, egui::Button::new("Connect (Bidirectional)")).clicked() {
                            self.auto_connect_selected(model::ArrowStyle::Both);
                            ui.close_menu();
                        }
                    });
                });
                ui.menu_button("Format", |ui| {
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        let has_selection = !self.selected.is_empty();
                        ui.label("Stroke Color");
                        let stroke_presets = [
                            ("Black", egui::Color32::from_rgb(20, 20, 20)),
                            ("Red", egui::Color32::from_rgb(200, 40, 40)),
                            ("Green", egui::Color32::from_rgb(40, 140, 60)),
                            ("Blue", egui::Color32::from_rgb(40, 90, 200)),
                            ("Orange", egui::Color32::from_rgb(200, 140, 40)),
                            ("Purple", egui::Color32::from_rgb(130, 60, 180)),
                        ];
                        for (name, color) in stroke_presets {
                            if ui.button(name).clicked() {
                                self.push_undo();
                                self.style.stroke.color = model::Rgba::from_color32(color);
                                if has_selection {
                                    for e in &mut self.doc.elements {
                                        if self.selected.contains(&e.id) {
                                            e.style.stroke.color = self.style.stroke.color;
                                        }
                                    }
                                }
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        ui.label("Stroke Width");
                        for width in [1.0, 2.0, 3.0, 4.0, 6.0, 8.0] {
                            if ui.button(format!("{:.0}px", width)).clicked() {
                                self.push_undo();
                                self.style.stroke.width = width;
                                if has_selection {
                                    for e in &mut self.doc.elements {
                                        if self.selected.contains(&e.id) {
                                            e.style.stroke.width = width;
                                        }
                                    }
                                }
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        ui.label("Line Style");
                        if ui.button("Solid").clicked() {
                            self.push_undo();
                            self.style.stroke.line_style = model::LineStyle::Solid;
                            if has_selection {
                                for e in &mut self.doc.elements {
                                    if self.selected.contains(&e.id) {
                                        e.style.stroke.line_style = model::LineStyle::Solid;
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Dashed").clicked() {
                            self.push_undo();
                            self.style.stroke.line_style = model::LineStyle::Dashed;
                            if has_selection {
                                for e in &mut self.doc.elements {
                                    if self.selected.contains(&e.id) {
                                        e.style.stroke.line_style = model::LineStyle::Dashed;
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Dotted").clicked() {
                            self.push_undo();
                            self.style.stroke.line_style = model::LineStyle::Dotted;
                            if has_selection {
                                for e in &mut self.doc.elements {
                                    if self.selected.contains(&e.id) {
                                        e.style.stroke.line_style = model::LineStyle::Dotted;
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Fill");
                        if ui.button("No Fill").clicked() {
                            self.push_undo();
                            self.style.fill = None;
                            if has_selection {
                                for e in &mut self.doc.elements {
                                    if self.selected.contains(&e.id) {
                                        e.style.fill = None;
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        let fill_presets = [
                            ("White", egui::Color32::from_rgb(255, 255, 255)),
                            ("Light Gray", egui::Color32::from_rgb(200, 200, 200)),
                            ("Light Red", egui::Color32::from_rgb(255, 200, 200)),
                            ("Light Green", egui::Color32::from_rgb(200, 255, 200)),
                            ("Light Blue", egui::Color32::from_rgb(200, 200, 255)),
                            ("Light Yellow", egui::Color32::from_rgb(255, 255, 200)),
                        ];
                        for (name, color) in fill_presets {
                            if ui.button(name).clicked() {
                                self.push_undo();
                                self.style.fill = Some(model::Rgba::from_color32(color));
                                if has_selection {
                                    for e in &mut self.doc.elements {
                                        if self.selected.contains(&e.id) {
                                            e.style.fill = self.style.fill;
                                        }
                                    }
                                }
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        ui.label("Text Size");
                        for size in [10.0, 12.0, 14.0, 16.0, 18.0, 24.0, 32.0, 48.0] {
                            if ui.button(format!("{:.0}pt", size)).clicked() {
                                self.push_undo();
                                self.style.text_size = size;
                                if has_selection {
                                    for e in &mut self.doc.elements {
                                        if self.selected.contains(&e.id) {
                                            e.style.text_size = size;
                                        }
                                    }
                                }
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        ui.label("Font");
                        if ui.button("Proportional").clicked() {
                            self.push_undo();
                            self.style.font_family = model::FontFamily::Proportional;
                            if has_selection {
                                for e in &mut self.doc.elements {
                                    if self.selected.contains(&e.id) {
                                        e.style.font_family = model::FontFamily::Proportional;
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Monospace").clicked() {
                            self.push_undo();
                            self.style.font_family = model::FontFamily::Monospace;
                            if has_selection {
                                for e in &mut self.doc.elements {
                                    if self.selected.contains(&e.id) {
                                        e.style.font_family = model::FontFamily::Monospace;
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Text Align");
                        if ui.button("Left").clicked() {
                            self.push_undo();
                            self.style.text_align = model::TextAlign::Left;
                            if has_selection {
                                for e in &mut self.doc.elements {
                                    if self.selected.contains(&e.id) {
                                        e.style.text_align = model::TextAlign::Left;
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Center").clicked() {
                            self.push_undo();
                            self.style.text_align = model::TextAlign::Center;
                            if has_selection {
                                for e in &mut self.doc.elements {
                                    if self.selected.contains(&e.id) {
                                        e.style.text_align = model::TextAlign::Center;
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Right").clicked() {
                            self.push_undo();
                            self.style.text_align = model::TextAlign::Right;
                            if has_selection {
                                for e in &mut self.doc.elements {
                                    if self.selected.contains(&e.id) {
                                        e.style.text_align = model::TextAlign::Right;
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                    });
                });
                ui.menu_button("View", |ui| {
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        ui.label("Zoom");
                        if ui.button("Zoom In").clicked() {
                            self.view.zoom = (self.view.zoom * 1.25).min(8.0);
                            ui.close_menu();
                        }
                        if ui.button("Zoom Out").clicked() {
                            self.view.zoom = (self.view.zoom / 1.25).max(0.1);
                            ui.close_menu();
                        }
                        if ui.button("Reset Zoom (100%)").clicked() {
                            self.view.zoom = 1.0;
                            ui.close_menu();
                        }
                        if ui.button("Fit 50%").clicked() {
                            self.view.zoom = 0.5;
                            ui.close_menu();
                        }
                        if ui.button("Fit 200%").clicked() {
                            self.view.zoom = 2.0;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Pan");
                        if ui.button("Reset Pan").clicked() {
                            self.view.pan_screen = egui::Vec2::ZERO;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Grid");
                        if ui.checkbox(&mut self.snap_to_grid, "Snap to Grid").changed() {
                            self.persist_settings();
                        }
                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            if ui.add(egui::DragValue::new(&mut self.grid_size).range(8.0..=128.0).speed(1.0)).changed() {
                                self.persist_settings();
                            }
                        });
                    });
                });
                ui.menu_button("Tools", |ui| {
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        if ui.button("Select (V)").clicked() {
                            self.tool = Tool::Select;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Shapes");
                        if ui.button("Rectangle (R)").clicked() {
                            self.tool = Tool::Rectangle;
                            ui.close_menu();
                        }
                        if ui.button("Ellipse (O)").clicked() {
                            self.tool = Tool::Ellipse;
                            ui.close_menu();
                        }
                        if ui.button("Triangle (⇧T)").clicked() {
                            self.tool = Tool::Triangle;
                            ui.close_menu();
                        }
                        if ui.button("Parallelogram (⇧P)").clicked() {
                            self.tool = Tool::Parallelogram;
                            ui.close_menu();
                        }
                        if ui.button("Trapezoid (⇧Z)").clicked() {
                            self.tool = Tool::Trapezoid;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Lines");
                        if ui.button("Line (L)").clicked() {
                            self.tool = Tool::Line;
                            ui.close_menu();
                        }
                        if ui.button("Arrow (A)").clicked() {
                            self.tool = Tool::Arrow;
                            ui.close_menu();
                        }
                        if ui.button("Bidirectional Arrow (⇧A)").clicked() {
                            self.tool = Tool::BidirectionalArrow;
                            ui.close_menu();
                        }
                        if ui.button("Polyline (⇧L)").clicked() {
                            self.tool = Tool::Polyline;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Other");
                        if ui.button("Pen (P)").clicked() {
                            self.tool = Tool::Pen;
                            ui.close_menu();
                        }
                        if ui.button("Text (T)").clicked() {
                            self.tool = Tool::Text;
                            ui.close_menu();
                        }
                        if ui.button("Pan (Space)").clicked() {
                            self.tool = Tool::Pan;
                            ui.close_menu();
                        }
                    });
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Show Help (F1)").clicked() {
                        self.show_help = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Reload Settings").clicked() {
                        self.reload_settings(ctx);
                        ui.close_menu();
                    }
                });
                ui.separator();
                tool_button(ui, "V", Tool::Select, &mut self.tool);
                tool_button(ui, "R", Tool::Rectangle, &mut self.tool);
                tool_button(ui, "O", Tool::Ellipse, &mut self.tool);
                tool_button(ui, "△", Tool::Triangle, &mut self.tool);
                tool_button(ui, "▱", Tool::Parallelogram, &mut self.tool);
                tool_button(ui, "⏢", Tool::Trapezoid, &mut self.tool);
                tool_button(ui, "L", Tool::Line, &mut self.tool);
                tool_button(ui, "→", Tool::Arrow, &mut self.tool);
                tool_button(ui, "↔", Tool::BidirectionalArrow, &mut self.tool);
                tool_button(ui, "⌇", Tool::Polyline, &mut self.tool);
                tool_button(ui, "✎", Tool::Pen, &mut self.tool);
                tool_button(ui, "T", Tool::Text, &mut self.tool);
                ui.separator();
                if let Some(status) = &self.status {
                    ui.label(status);
                }
            });
        });

        egui::SidePanel::right("right_panel")
            .resizable(true)
            .min_width(200.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Properties");
                ui.separator();
                if ui
                    .checkbox(&mut self.apply_style_to_selection, "Apply to selection")
                    .changed()
                {
                    self.persist_settings();
                }

                let theme_colors = self.get_theme_colors();
                if self.selected.len() == 1 {
                    let selected_id = *self.selected.iter().next().unwrap();
                    if let Some(idx) = self.element_index_by_id(selected_id) {
                        let original_style = self.doc.elements[idx].style.clone();
                        let mut style = original_style.clone();
                        let style_changed = style_editor(ui, &mut style, &theme_colors) && style != original_style;

                        let mut push_undo_on_focus = false;
                        match &mut self.doc.elements[idx].kind {
                            model::ElementKind::Text { text, .. } => {
                                ui.separator();
                                ui.label("Edit text");
                                let response =
                                    ui.add(egui::TextEdit::multiline(text).desired_rows(6));
                                push_undo_on_focus |= response.gained_focus();
                                if response.changed() {
                                    self.status = None;
                                }
                                self.editing_text_id = Some(selected_id);
                            }
                            model::ElementKind::Rect { rect, label }
                            | model::ElementKind::Ellipse { rect, label }
                            | model::ElementKind::Triangle { rect, label, .. }
                            | model::ElementKind::Parallelogram { rect, label, .. }
                            | model::ElementKind::Trapezoid { rect, label, .. } => {
                                ui.separator();
                                ui.label("Label");
                                let response =
                                    ui.add(egui::TextEdit::multiline(label).desired_rows(4));
                                push_undo_on_focus |= response.gained_focus();

                                ui.separator();
                                ui.label("Size");
                                let mut width = rect.max.x - rect.min.x;
                                let mut height = rect.max.y - rect.min.y;
                                let original_width = width;
                                let original_height = height;

                                ui.horizontal(|ui| {
                                    ui.label("W:");
                                    let w_response = ui.add(
                                        egui::DragValue::new(&mut width)
                                            .range(8.0..=10000.0)
                                            .speed(1.0),
                                    );
                                    push_undo_on_focus |= w_response.gained_focus();
                                    ui.label("H:");
                                    let h_response = ui.add(
                                        egui::DragValue::new(&mut height)
                                            .range(8.0..=10000.0)
                                            .speed(1.0),
                                    );
                                    push_undo_on_focus |= h_response.gained_focus();
                                });

                                if (width - original_width).abs() > f32::EPSILON {
                                    rect.max.x = rect.min.x + width;
                                }
                                if (height - original_height).abs() > f32::EPSILON {
                                    rect.max.y = rect.min.y + height;
                                }
                            }
                            model::ElementKind::Line {
                                a,
                                b,
                                arrow,
                                arrow_style,
                                start_binding,
                                end_binding,
                            } => {
                                ui.separator();
                                ui.label("Line");
                                let mut next_style = *arrow_style;
                                egui::ComboBox::from_id_salt("arrow_style")
                                    .selected_text(match next_style {
                                        model::ArrowStyle::None => "None",
                                        model::ArrowStyle::End => "Arrow (End)",
                                        model::ArrowStyle::Start => "Arrow (Start)",
                                        model::ArrowStyle::Both => "Arrow (Both)",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut next_style,
                                            model::ArrowStyle::None,
                                            "None",
                                        );
                                        ui.selectable_value(
                                            &mut next_style,
                                            model::ArrowStyle::End,
                                            "Arrow (End)",
                                        );
                                        ui.selectable_value(
                                            &mut next_style,
                                            model::ArrowStyle::Start,
                                            "Arrow (Start)",
                                        );
                                        ui.selectable_value(
                                            &mut next_style,
                                            model::ArrowStyle::Both,
                                            "Arrow (Both)",
                                        );
                                    });
                                if next_style != *arrow_style {
                                    *arrow_style = next_style;
                                    *arrow = matches!(
                                        *arrow_style,
                                        model::ArrowStyle::End | model::ArrowStyle::Both
                                    );
                                    self.status = None;
                                }

                                ui.separator();
                                ui.label("Endpoints");
                                let mut ax = a.x;
                                let mut ay = a.y;
                                let mut bx = b.x;
                                let mut by = b.y;
                                let original = (ax, ay, bx, by);
                                ui.horizontal(|ui| {
                                    ui.label("A:");
                                    let rx = ui.add(egui::DragValue::new(&mut ax).speed(1.0));
                                    let ry = ui.add(egui::DragValue::new(&mut ay).speed(1.0));
                                    push_undo_on_focus |= rx.gained_focus() || ry.gained_focus();
                                });
                                ui.horizontal(|ui| {
                                    ui.label("B:");
                                    let rx = ui.add(egui::DragValue::new(&mut bx).speed(1.0));
                                    let ry = ui.add(egui::DragValue::new(&mut by).speed(1.0));
                                    push_undo_on_focus |= rx.gained_focus() || ry.gained_focus();
                                });
                                if (ax, ay, bx, by) != original {
                                    *a = model::Point { x: ax, y: ay };
                                    *b = model::Point { x: bx, y: by };
                                    *start_binding = None;
                                    *end_binding = None;
                                    self.status = None;
                                }
                            }
                            model::ElementKind::Polyline { arrow_style, .. } => {
                                ui.separator();
                                ui.label("Polyline");
                                let mut next_style = *arrow_style;
                                egui::ComboBox::from_id_salt("polyline_arrow_style")
                                    .selected_text(match next_style {
                                        model::ArrowStyle::None => "None",
                                        model::ArrowStyle::End => "Arrow (End)",
                                        model::ArrowStyle::Start => "Arrow (Start)",
                                        model::ArrowStyle::Both => "Arrow (Both)",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut next_style,
                                            model::ArrowStyle::None,
                                            "None",
                                        );
                                        ui.selectable_value(
                                            &mut next_style,
                                            model::ArrowStyle::End,
                                            "Arrow (End)",
                                        );
                                        ui.selectable_value(
                                            &mut next_style,
                                            model::ArrowStyle::Start,
                                            "Arrow (Start)",
                                        );
                                        ui.selectable_value(
                                            &mut next_style,
                                            model::ArrowStyle::Both,
                                            "Arrow (Both)",
                                        );
                                    });
                                if next_style != *arrow_style {
                                    *arrow_style = next_style;
                                    self.status = None;
                                }
                            }
                            _ => {}
                        }
                        #[derive(Clone, Copy)]
                        enum ShapeParamKind {
                            Triangle,
                            Parallelogram,
                            Trapezoid,
                        }
                        let shape = match &self.doc.elements[idx].kind {
                            model::ElementKind::Triangle { apex_ratio, .. } => {
                                Some((ShapeParamKind::Triangle, *apex_ratio))
                            }
                            model::ElementKind::Parallelogram { skew_ratio, .. } => {
                                Some((ShapeParamKind::Parallelogram, *skew_ratio))
                            }
                            model::ElementKind::Trapezoid { top_inset_ratio, .. } => {
                                Some((ShapeParamKind::Trapezoid, *top_inset_ratio))
                            }
                            _ => None,
                        };
                        if let Some((kind, current)) = shape {
                            let mut next = None;
                            match kind {
                                ShapeParamKind::Triangle => {
                                    ui.separator();
                                    ui.label("Triangle");
                                    let mut v = current;
                                    let resp = ui.add(
                                        egui::Slider::new(&mut v, -0.95..=0.95)
                                            .text("Apex")
                                            .clamp_to_range(true),
                                    );
                                    push_undo_on_focus |= resp.gained_focus();
                                    if resp.changed() {
                                        next = Some(v);
                                    }
                                }
                                ShapeParamKind::Parallelogram => {
                                    ui.separator();
                                    ui.label("Parallelogram");
                                    let mut v = current;
                                    let resp = ui.add(
                                        egui::Slider::new(&mut v, -0.95..=0.95)
                                            .text("Skew")
                                            .clamp_to_range(true),
                                    );
                                    push_undo_on_focus |= resp.gained_focus();
                                    if resp.changed() {
                                        next = Some(v);
                                    }
                                }
                                ShapeParamKind::Trapezoid => {
                                    ui.separator();
                                    ui.label("Trapezoid");
                                    let mut v = current;
                                    let resp = ui.add(
                                        egui::Slider::new(&mut v, 0.0..=0.95)
                                            .text("Top inset")
                                            .clamp_to_range(true),
                                    );
                                    push_undo_on_focus |= resp.gained_focus();
                                    if resp.changed() {
                                        next = Some(v);
                                    }
                                }
                            }
                            if let Some(v) = next {
                                self.push_undo();
                                match (&mut self.doc.elements[idx].kind, kind) {
                                    (model::ElementKind::Triangle { apex_ratio, .. }, ShapeParamKind::Triangle) => *apex_ratio = v,
                                    (model::ElementKind::Parallelogram { skew_ratio, .. }, ShapeParamKind::Parallelogram) => *skew_ratio = v,
                                    (model::ElementKind::Trapezoid { top_inset_ratio, .. }, ShapeParamKind::Trapezoid) => *top_inset_ratio = v,
                                    _ => {}
                                }
                            }
                        }
                        if style_changed {
                            self.push_undo();
                            self.doc.elements[idx].style = style;
                        }
                        if push_undo_on_focus {
                            self.push_undo();
                        }
                    } else {
                        let mut style = self.style.clone();
                        if style_editor(ui, &mut style, &theme_colors) && style != self.style {
                            self.push_undo();
                            self.style = style;
                        }
                    }
                } else {
                    let mut style = self.style.clone();
                    if style_editor(ui, &mut style, &theme_colors) && style != self.style {
                        self.push_undo();
                        self.style = style.clone();
                        if self.apply_style_to_selection && !self.selected.is_empty() {
                            for element in &mut self.doc.elements {
                                if self.selected.contains(&element.id) {
                                    element.style = style.clone();
                                }
                            }
                        }
                    }
                }

                ui.separator();
                ui.heading("Grid & Snap");
                if ui.checkbox(&mut self.snap_to_grid, "Snap to grid").changed() {
                    self.persist_settings();
                }
                if ui
                    .add(
                    egui::Slider::new(&mut self.grid_size, 8.0..=128.0)
                        .text("Grid size")
                        .logarithmic(true),
                )
                .changed()
                {
                    self.persist_settings();
                }
                if self.selected.len() == 1 {
                    let selected_id = *self.selected.iter().next().unwrap();
                    if let Some(idx) = self.element_index_by_id(selected_id) {
                        let original_snap = self.doc.elements[idx].snap_enabled;
                        let mut snap = original_snap;
                        if ui.checkbox(&mut snap, "Object snap enabled").changed()
                            && snap != original_snap
                        {
                            self.push_undo();
                            self.doc.elements[idx].snap_enabled = snap;
                        }
                    }
                }

                ui.separator();
                ui.heading("Color Themes");
                let mut theme_selection = self.active_color_theme;
                egui::ComboBox::from_id_salt("color_theme_select")
                    .selected_text(match theme_selection {
                        Some(idx) => self
                            .color_themes
                            .get(idx)
                            .map(|t| t.name.as_str())
                            .unwrap_or("None"),
                        None => "None",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut theme_selection, None, "None");
                        for (idx, t) in self.color_themes.iter().enumerate() {
                            ui.selectable_value(&mut theme_selection, Some(idx), &t.name);
                        }
                    });
                if theme_selection != self.active_color_theme {
                    self.active_color_theme = theme_selection;
                    self.persist_settings();
                }
                if !self.color_themes.is_empty() {
                    ui.label("Available colors:");
                    let names = self.all_color_names();
                    if names.is_empty() {
                        ui.label("(no colors defined)");
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for name in names {
                                if let Some(color) = self.lookup_color_by_name(&name) {
                                    let c = color.to_color32();
                                    ui.add_sized([60.0, 18.0], egui::Button::new(&name).fill(c));
                                } else {
                                    ui.label(&name);
                                }
                            }
                        });
                    }
                }
                ui.label("Define themes in settings.toml");

                ui.separator();
                ui.heading("Custom Fonts");
                if self.loaded_fonts.is_empty() {
                    ui.label("No custom fonts loaded");
                } else {
                    ui.label(format!("{} font(s) loaded:", self.loaded_fonts.len()));
                    let font_names: Vec<String> = self.loaded_fonts.clone();
                    let mut clicked_font: Option<String> = None;
                    for font_name in &font_names {
                        if ui.small_button(font_name).clicked() {
                            clicked_font = Some(font_name.clone());
                        }
                    }
                    if let Some(font_name) = clicked_font {
                        self.push_undo();
                        self.style.font_family = model::FontFamily::Custom(font_name.clone());
                        if !self.selected.is_empty() {
                            for e in &mut self.doc.elements {
                                if self.selected.contains(&e.id) {
                                    e.style.font_family = model::FontFamily::Custom(font_name.clone());
                                }
                            }
                        }
                    }
                }
                let font_dir_text = self.font_directory.clone().unwrap_or_default();
                if !font_dir_text.is_empty() {
                    ui.small(format!("Dir: {}", font_dir_text));
                }
                ui.label("Set font_directory in settings.toml");

                ui.separator();
                ui.heading("Editor");
                if ui
                    .add(egui::Slider::new(&mut self.move_step, 0.25..=32.0).text("Move"))
                    .changed()
                {
                    self.persist_settings();
                }
                if ui
                    .add(
                        egui::Slider::new(&mut self.move_step_fast, 0.25..=256.0).text("Move (⇧)"),
                    )
                    .changed()
                {
                    self.persist_settings();
                }

                ui.separator();
                if ui.button("Show Help (F1)").clicked() {
                    self.show_help = true;
                }
                ui.label("Shortcuts: Space/⌘⇧P: commands, V/R/O/L/A/P/T: tools");

                ui.separator();
                ui.heading("Layers");
                let items: Vec<(u64, String, bool)> = self
                    .doc
                    .elements
                    .iter()
                    .rev()
                    .map(|e| (e.id, element_label(e), self.selected.contains(&e.id)))
                    .collect();
                for (id, label, is_selected) in items {
                    let clicked = ui.selectable_label(is_selected, label).clicked();
                    if clicked {
                        let shift = ctx.input(|i| i.modifiers.shift);
                        if shift {
                            self.toggle_selection(id);
                        } else {
                            self.set_selection_single(id);
                        }
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("▲").clicked() {
                        self.move_selected_layer_by(1);
                    }
                    if ui.button("▼").clicked() {
                        self.move_selected_layer_by(-1);
                    }
                    if ui.button("Group").clicked() {
                        self.group_selected();
                    }
                    if ui.button("Ungroup").clicked() {
                        self.ungroup_selected();
                    }
                });
                });
            });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(status) = &self.status {
                    ui.label(status);
                } else {
                    ui.label("Ready");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Zoom: {:.0}%", self.view.zoom * 100.0));
                    ui.separator();
                    ui.label(format!("Objects: {}", self.doc.elements.len()));
                    ui.separator();
                    ui.label(format!("Selected: {}", self.selected.len()));
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
            let origin = rect.min;

            let space_down =
                ctx.input(|i| i.key_down(egui::Key::Space)) && !ctx.wants_keyboard_input();
            if space_down {
                if self.tool_before_pan.is_none() {
                    self.tool_before_pan = Some(self.tool);
                    self.tool = Tool::Pan;
                    self.space_pan_happened = false;
                }
            } else if let Some(prev) = self.tool_before_pan.take() {
                if self.tool == Tool::Pan {
                    self.tool = prev;
                }
                if !self.space_pan_happened && !self.command_palette.open && !self.inline_text_editing
                {
                    self.command_palette.open("");
                }
            }

            let scroll_delta = ctx.input(|i| i.raw_scroll_delta.y);
            if scroll_delta.abs() > 0.0 {
                if let Some(hover_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    if rect.contains(hover_pos) {
                        let zoom_delta = (1.0 + scroll_delta * 0.001).clamp(0.8, 1.25);
                        self.view
                            .zoom_about_screen_point(origin, hover_pos, zoom_delta);
                    }
                }
            }

            if self.tool == Tool::Pan && response.dragged() {
                self.view.pan_screen += response.drag_delta();
                self.space_pan_happened = true;
            }

            let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
            let pointer_world = pointer_pos.map(|p| self.view.screen_to_world(origin, p));
            let threshold_world = 6.0 / self.view.zoom;
            self.last_pointer_world = pointer_world;

            let pressed = response.drag_started() || response.clicked();
            let released = response.drag_stopped();

            if pressed {
                self.drag_transform_recorded = false;
            }

            if response.secondary_clicked() {
                if let Some(InProgress::Polyline {
                    points, current, ..
                }) = &mut self.in_progress
                {
                    if let Some(world_pos) = pointer_world {
                        points.push(*current);
                        *current = world_pos;
                    }
                } else {
                    self.context_world_pos = pointer_world;
                    self.context_hit =
                        pointer_world.and_then(|p| self.topmost_hit(p, threshold_world));
                    if let Some(hit) = self.context_hit {
                        if !self.selected.contains(&hit) {
                            self.set_selection_single(hit);
                        }
                    }
                }
            }

            let mut handled_double_click = false;
            if response.double_clicked() && self.tool == Tool::Select {
                if let Some(world_pos) = pointer_world {
                    if let Some(hit_id) = self.topmost_hit(world_pos, threshold_world) {
                        if let Some(element) = self.doc.elements.iter().find(|e| e.id == hit_id) {
                            let has_text = matches!(
                                element.kind,
                                model::ElementKind::Text { .. }
                                    | model::ElementKind::Rect { .. }
                                    | model::ElementKind::Ellipse { .. }
                                    | model::ElementKind::Triangle { .. }
                                    | model::ElementKind::Parallelogram { .. }
                                    | model::ElementKind::Trapezoid { .. }
                            );
                            if has_text {
                                self.set_selection_single(hit_id);
                                self.editing_text_id = Some(hit_id);
                                self.inline_text_editing = true;
                                handled_double_click = true;
                            }
                        }
                    }
                }
            }

            if pressed && !handled_double_click {
                if self.inline_text_editing {
                    self.inline_text_editing = false;
                    self.editing_text_id = None;
                }
                if let Some(world_pos) = pointer_world {
                    match self.tool {
                        Tool::Select => {
                            let hit = self.topmost_hit(world_pos, threshold_world);
                            let multi_select = ctx.input(|i| {
                                i.modifiers.shift || i.modifiers.ctrl || i.modifiers.command
                            });
                            if let Some(id) = hit {
                                if multi_select {
                                    self.toggle_selection(id);
                                } else if !self.selected.contains(&id) {
                                    self.set_selection_single(id);
                                }
                                self.in_progress = None;
                            } else {
                                if !multi_select {
                                    self.clear_selection();
                                }
                                self.in_progress = Some(InProgress::SelectBox {
                                    start: world_pos,
                                    current: world_pos,
                                });
                            }
                        }
                        Tool::Rectangle
                        | Tool::Ellipse
                        | Tool::Triangle
                        | Tool::Parallelogram
                        | Tool::Trapezoid => {
                            self.in_progress = Some(InProgress::DragShape {
                                start: world_pos,
                                current: world_pos,
                            });
                        }
                        Tool::Line => {
                            self.in_progress = Some(InProgress::DragLine {
                                start: world_pos,
                                current: world_pos,
                                arrow_style: model::ArrowStyle::None,
                            });
                        }
                        Tool::Arrow => {
                            self.in_progress = Some(InProgress::DragLine {
                                start: world_pos,
                                current: world_pos,
                                arrow_style: model::ArrowStyle::End,
                            });
                        }
                        Tool::BidirectionalArrow => {
                            self.in_progress = Some(InProgress::DragLine {
                                start: world_pos,
                                current: world_pos,
                                arrow_style: model::ArrowStyle::Both,
                            });
                        }
                        Tool::Polyline => {
                            self.in_progress = Some(InProgress::Polyline {
                                points: vec![world_pos],
                                current: world_pos,
                                arrow_style: model::ArrowStyle::None,
                            });
                        }
                        Tool::Pen => {
                            self.in_progress = Some(InProgress::Pen {
                                points: vec![world_pos],
                            });
                        }
                        Tool::Text => {
                            self.push_undo();
                            let id = self.allocate_id();
                            let mut style = self.style.clone();
                            style.fill = None;
                            let element = model::Element {
                                id,
                                group_id: None,
                                rotation: 0.0,
                                snap_enabled: true,
                                kind: model::ElementKind::Text {
                                    pos: model::Point::from_pos2(world_pos),
                                    text: String::new(),
                                },
                                style,
                            };
                            self.doc.elements.push(element);
                            self.set_selection_single(id);
                            self.editing_text_id = Some(id);
                        }
                        Tool::Pan => {}
                    }
                }
            }

            if response.dragged() {
                if let Some(world_pos) = pointer_world {
                    if let Some(in_progress) = &mut self.in_progress {
                        let shift = ctx.input(|i| i.modifiers.shift);
                        match in_progress {
                            InProgress::DragShape { start, current } => {
                                let mut p = world_pos;
                                if shift && matches!(self.tool, Tool::Rectangle | Tool::Ellipse) {
                                    let d = p - *start;
                                    if d.x.abs() >= d.y.abs() {
                                        let s = d.x.abs();
                                        let ny = if d.y >= 0.0 { s } else { -s };
                                        p = *start + egui::vec2(d.x, ny);
                                    } else {
                                        let s = d.y.abs();
                                        let nx = if d.x >= 0.0 { s } else { -s };
                                        p = *start + egui::vec2(nx, d.y);
                                    }
                                }
                                *current = p;
                            }
                            InProgress::DragLine { start, current, .. } => {
                                let mut p = world_pos;
                                if shift {
                                    let d = p - *start;
                                    if d.x.abs() >= d.y.abs() {
                                        p.y = start.y;
                                    } else {
                                        p.x = start.x;
                                    }
                                }
                                *current = p;
                            }
                            InProgress::Polyline { points, current, .. } => {
                                let mut p = world_pos;
                                if shift {
                                    if let Some(last) = points.last().copied() {
                                        let d = p - last;
                                        if d.x.abs() >= d.y.abs() {
                                            p.y = last.y;
                                        } else {
                                            p.x = last.x;
                                        }
                                    }
                                }
                                *current = p;
                            }
                            InProgress::Pen { points } => {
                                if points.last().copied() != Some(world_pos) {
                                    points.push(world_pos);
                                }
                            }
                            InProgress::SelectBox { current, .. } => *current = world_pos,
                        }
                    } else if self.tool == Tool::Select {
                        if !self.selected.is_empty() {
                            if !self.drag_transform_recorded {
                                self.push_undo();
                                self.drag_transform_recorded = true;
                            }
                            let delta_world = response.drag_delta() / self.view.zoom;
                            self.translate_selected(delta_world);
                        }
                    }
                }
            }

            if released {
                if self.tool == Tool::Select && self.drag_transform_recorded {
                    self.snap_selected_to_grid();
                }
                self.drag_transform_recorded = false;
                if let Some(in_progress) = self.in_progress.take() {
                    match in_progress {
                        InProgress::DragShape { start, current } => {
                            let rect = model::RectF::from_min_max(start, current);
                            if rect.is_valid() {
                                self.push_undo();
                                let id = self.allocate_id();
                                let kind = match self.tool {
                                    Tool::Rectangle => model::ElementKind::Rect {
                                        rect,
                                        label: String::new(),
                                    },
                                    Tool::Ellipse => model::ElementKind::Ellipse {
                                        rect,
                                        label: String::new(),
                                    },
                                    Tool::Triangle => model::ElementKind::Triangle {
                                        rect,
                                        label: String::new(),
                                        apex_ratio: 0.0,
                                    },
                                    Tool::Parallelogram => model::ElementKind::Parallelogram {
                                        rect,
                                        label: String::new(),
                                        skew_ratio: 0.25,
                                    },
                                    Tool::Trapezoid => model::ElementKind::Trapezoid {
                                        rect,
                                        label: String::new(),
                                        top_inset_ratio: 0.25,
                                    },
                                    _ => model::ElementKind::Rect {
                                        rect,
                                        label: String::new(),
                                    },
                                };
                                let element = model::Element {
                                    id,
                                    group_id: None,
                                    rotation: 0.0,
                                    snap_enabled: true,
                                    kind,
                                    style: self.style.clone(),
                                };
                                self.doc.elements.push(element);
                                self.set_selection_single(id);
                            }
                        }
                        InProgress::DragLine {
                            start,
                            current,
                            arrow_style,
                        } => {
                            if (current - start).length() >= threshold_world {
                                self.push_undo();
                                let id = self.allocate_id();
                                let start_binding =
                                    topmost_bind_target_id(&self.doc, start, threshold_world)
                                        .and_then(|tid| {
                                            compute_binding_for_target(&self.doc, tid, start)
                                        });
                                let end_binding =
                                    topmost_bind_target_id(&self.doc, current, threshold_world)
                                        .and_then(|tid| {
                                            compute_binding_for_target(&self.doc, tid, current)
                                        });
                                let a = start_binding
                                    .as_ref()
                                    .and_then(|b| resolve_binding_point(&self.doc, b))
                                    .unwrap_or(start);
                                let b = end_binding
                                    .as_ref()
                                    .and_then(|b| resolve_binding_point(&self.doc, b))
                                    .unwrap_or(current);
                                let arrow = matches!(
                                    arrow_style,
                                    model::ArrowStyle::End | model::ArrowStyle::Both
                                );
                                let element = model::Element {
                                    id,
                                    group_id: None,
                                    rotation: 0.0,
                                    snap_enabled: true,
                                    kind: model::ElementKind::Line {
                                        a: model::Point::from_pos2(a),
                                        b: model::Point::from_pos2(b),
                                        arrow,
                                        arrow_style,
                                        start_binding,
                                        end_binding,
                                    },
                                    style: self.style.clone(),
                                };
                                self.doc.elements.push(element);
                                self.set_selection_single(id);
                            }
                        }
                        InProgress::Polyline {
                            points,
                            current,
                            arrow_style,
                        } => {
                            let mut pts = points;
                            pts.push(current);
                            if pts.len() >= 2 {
                                self.push_undo();
                                let id = self.allocate_id();
                                let element = model::Element {
                                    id,
                                    group_id: None,
                                    rotation: 0.0,
                                    snap_enabled: true,
                                    kind: model::ElementKind::Polyline {
                                        points: pts
                                            .into_iter()
                                            .map(model::Point::from_pos2)
                                            .collect(),
                                        arrow_style,
                                    },
                                    style: self.style.clone(),
                                };
                                self.doc.elements.push(element);
                                self.set_selection_single(id);
                            }
                        }
                        InProgress::Pen { points } => {
                            if points.len() >= 2 {
                                self.push_undo();
                                let id = self.allocate_id();
                                let element = model::Element {
                                    id,
                                    group_id: None,
                                    rotation: 0.0,
                                    snap_enabled: true,
                                    kind: model::ElementKind::Pen {
                                        points: points
                                            .into_iter()
                                            .map(model::Point::from_pos2)
                                            .collect(),
                                    },
                                    style: self.style.clone(),
                                };
                                self.doc.elements.push(element);
                                self.set_selection_single(id);
                            }
                        }
                        InProgress::SelectBox { start, current } => {
                            let box_rect = egui::Rect::from_two_pos(start, current);
                            let mut selected = HashSet::new();
                            for element in &self.doc.elements {
                                let b = element.bounds();
                                if box_rect.intersects(b) {
                                    selected.insert(element.id);
                                }
                            }
                            let mut groups = HashSet::new();
                            for id in &selected {
                                if let Some(g) = self.root_group_of_element(*id) {
                                    groups.insert(g);
                                }
                            }
                            for g in groups {
                                for id in self.group_members_recursive(g) {
                                    selected.insert(id);
                                }
                            }
                            self.selected = selected;
                        }
                    }
                }
            }

            let painter = ui.painter_at(rect);
            draw_background(&painter, rect, &self.view);
            draw_elements(&painter, origin, &self.view, &self.doc, &self.selected);
            draw_group_selection_boxes(&painter, origin, &self.view, &self.doc, &self.selected);
            if let Some(in_progress) = &self.in_progress {
                draw_in_progress(
                    &painter,
                    origin,
                    &self.view,
                    in_progress,
                    self.tool,
                    &self.style,
                );
            }
            if self.tool == Tool::Select {
                let view = self.view;
                self.interact_selection_handles(
                    ui,
                    &painter,
                    origin,
                    &view,
                    pointer_world,
                    threshold_world,
                    ctx,
                );
            }

            if self.inline_text_editing {
                if let Some(editing_id) = self.editing_text_id {
                    if let Some(idx) = self.element_index_by_id(editing_id) {
                        let element = &self.doc.elements[idx];
                        let (edit_rect_world, text_ptr) = match &element.kind {
                            model::ElementKind::Text { pos, .. } => {
                                let world_pos = pos.to_pos2();
                                let w = 200.0f32.max(element.style.text_size * 10.0);
                                let h = element.style.text_size * 6.0;
                                (egui::Rect::from_min_size(world_pos, egui::vec2(w, h)), idx)
                            }
                            model::ElementKind::Rect { rect, .. }
                            | model::ElementKind::Ellipse { rect, .. }
                            | model::ElementKind::Triangle { rect, .. }
                            | model::ElementKind::Parallelogram { rect, .. }
                            | model::ElementKind::Trapezoid { rect, .. } => {
                                let world_rect = rect.to_rect();
                                let margin = 10.0;
                                let edit_rect = egui::Rect::from_center_size(
                                    world_rect.center(),
                                    egui::vec2(
                                        (world_rect.width() - margin * 2.0).max(50.0),
                                        (world_rect.height() - margin * 2.0).max(30.0),
                                    ),
                                );
                                (edit_rect, idx)
                            }
                            _ => {
                                self.inline_text_editing = false;
                                self.editing_text_id = None;
                                (egui::Rect::NOTHING, 0)
                            }
                        };

                        if edit_rect_world != egui::Rect::NOTHING {
                            let screen_min = self.view.world_to_screen(origin, edit_rect_world.min);
                            let screen_max = self.view.world_to_screen(origin, edit_rect_world.max);
                            let edit_rect_screen = egui::Rect::from_min_max(screen_min, screen_max);

                            let area_id = ui.id().with("inline_text_edit");
                            egui::Area::new(area_id)
                                .fixed_pos(edit_rect_screen.min)
                                .order(egui::Order::Foreground)
                                .show(ctx, |ui| {
                                    let frame = egui::Frame::new()
                                        .fill(egui::Color32::from_rgba_unmultiplied(
                                            255, 255, 255, 240,
                                        ))
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_rgb(90, 160, 255),
                                        ))
                                        .inner_margin(4.0);
                                    frame.show(ui, |ui| {
                                        ui.set_min_size(edit_rect_screen.size());
                                        let text_to_edit: &mut String = match &mut self.doc.elements
                                            [text_ptr]
                                            .kind
                                        {
                                            model::ElementKind::Text { text, .. } => text,
                                            model::ElementKind::Rect { label, .. }
                                            | model::ElementKind::Ellipse { label, .. }
                                            | model::ElementKind::Triangle { label, .. }
                                            | model::ElementKind::Parallelogram { label, .. }
                                            | model::ElementKind::Trapezoid { label, .. } => label,
                                            _ => {
                                                self.inline_text_editing = false;
                                                return;
                                            }
                                        };
                                        let response = ui.add(
                                            egui::TextEdit::multiline(text_to_edit)
                                                .desired_width(edit_rect_screen.width() - 8.0)
                                                .frame(false),
                                        );
                                        if response.gained_focus() {
                                            self.push_undo();
                                        }
                                        response.request_focus();
                                    });
                                });

                            let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
                            let clicked_outside = response.clicked()
                                && !edit_rect_screen.contains(
                                    ctx.input(|i| i.pointer.interact_pos())
                                        .unwrap_or(egui::Pos2::ZERO),
                                );
                            if escape_pressed || clicked_outside {
                                self.inline_text_editing = false;
                                self.editing_text_id = None;
                            }
                        }
                    } else {
                        self.inline_text_editing = false;
                        self.editing_text_id = None;
                    }
                } else {
                    self.inline_text_editing = false;
                }
            }

            response.context_menu(|ui| {
                if ui.button("Duplicate").clicked() {
                    self.duplicate_selected();
                    ui.close();
                }
                if ui.button("Delete").clicked() {
                    self.delete_selected();
                    ui.close();
                }
                ui.separator();
                if ui.button("Bring to front").clicked() {
                    self.bring_selected_to_front();
                    ui.close();
                }
                if ui.button("Send to back").clicked() {
                    self.send_selected_to_back();
                    ui.close();
                }
                if ui.button("Move up").clicked() {
                    self.move_selected_layer_by(1);
                    ui.close();
                }
                if ui.button("Move down").clicked() {
                    self.move_selected_layer_by(-1);
                    ui.close();
                }
                ui.separator();
                ui.add_enabled_ui(self.selected.len() >= 2, |ui| {
                    if ui.button("Group").clicked() {
                        self.group_selected();
                        ui.close();
                    }
                });
                ui.add_enabled_ui(
                    self.selected.iter().any(|id| self.group_of(*id).is_some()),
                    |ui| {
                        if ui.button("Ungroup").clicked() {
                            self.ungroup_selected();
                            ui.close();
                        }
                    },
                );
                ui.separator();
                ui.add_enabled_ui(self.selected.len() == 2, |ui| {
                    if ui.button("Connect (line)").clicked() {
                        self.auto_connect_selected(model::ArrowStyle::None);
                        ui.close();
                    }
                    if ui.button("Connect (arrow)").clicked() {
                        self.auto_connect_selected(model::ArrowStyle::End);
                        ui.close();
                    }
                    if ui.button("Connect (bidirectional)").clicked() {
                        self.auto_connect_selected(model::ArrowStyle::Both);
                        ui.close();
                    }
                });
                ui.separator();
                ui.add_enabled_ui(self.selected.len() >= 2, |ui| {
                    if ui.button("Align left").clicked() {
                        self.push_undo();
                        align_selected(&mut self.doc, &self.selected, AlignMode::Left);
                        ui.close();
                    }
                    if ui.button("Align center").clicked() {
                        self.push_undo();
                        align_selected(&mut self.doc, &self.selected, AlignMode::HCenter);
                        ui.close();
                    }
                    if ui.button("Align right").clicked() {
                        self.push_undo();
                        align_selected(&mut self.doc, &self.selected, AlignMode::Right);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Align top").clicked() {
                        self.push_undo();
                        align_selected(&mut self.doc, &self.selected, AlignMode::Top);
                        ui.close();
                    }
                    if ui.button("Align middle").clicked() {
                        self.push_undo();
                        align_selected(&mut self.doc, &self.selected, AlignMode::VCenter);
                        ui.close();
                    }
                    if ui.button("Align bottom").clicked() {
                        self.push_undo();
                        align_selected(&mut self.doc, &self.selected, AlignMode::Bottom);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Distribute horizontally").clicked() {
                        self.push_undo();
                        distribute_selected(
                            &mut self.doc,
                            &self.selected,
                            DistributeMode::Horizontal,
                        );
                        ui.close();
                    }
                    if ui.button("Distribute vertically").clicked() {
                        self.push_undo();
                        distribute_selected(
                            &mut self.doc,
                            &self.selected,
                            DistributeMode::Vertical,
                        );
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Abut horizontally").clicked() {
                        self.push_undo();
                        abut_selected(&mut self.doc, &self.selected, AbutMode::Horizontal);
                        ui.close();
                    }
                    if ui.button("Abut vertically").clicked() {
                        self.push_undo();
                        abut_selected(&mut self.doc, &self.selected, AbutMode::Vertical);
                        ui.close();
                    }
                });
                if let Some(hit) = self.context_hit {
                    ui.separator();
                    ui.label(format!("Target: {}", hit));
                }
            });

            if let Some(world_pos) = pointer_world {
                if self.tool == Tool::Select {
                    if self.topmost_hit(world_pos, threshold_world).is_some() {
                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                    } else {
                        ctx.set_cursor_icon(egui::CursorIcon::Default);
                    }
                } else if self.tool == Tool::Pen {
                    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                }
            }
        });

        let has_resizable = self.selected.iter().any(|id| {
            self.doc.elements.iter().find(|e| e.id == *id).map_or(false, |e| {
                matches!(
                    e.kind,
                    model::ElementKind::Rect { .. }
                        | model::ElementKind::Ellipse { .. }
                        | model::ElementKind::Triangle { .. }
                        | model::ElementKind::Parallelogram { .. }
                        | model::ElementKind::Trapezoid { .. }
                )
            })
        });
        let cx = CommandContext {
            selected_len: self.selected.len(),
            has_undo: !self.history.is_empty(),
            has_redo: !self.future.is_empty(),
            can_ungroup: self.selected.iter().any(|id| self.group_of(*id).is_some()),
            snap_to_grid: self.snap_to_grid,
            has_resizable,
        };
        let submit_input = self.command_palette.is_awaiting_input()
            && ctx.input(|i| i.key_pressed(egui::Key::Enter));
        let input_data = if submit_input {
            self.command_palette.take_input_data()
        } else {
            None
        };
        let cmd = { self.command_palette.ui(ctx, cx) };
        if let Some((mode, value)) = input_data {
            super::command_palette::CommandPalette::execute_input(self, mode, &value);
            self.command_palette.close();
        } else if let Some(cmd) = cmd {
            super::command_palette::CommandPalette::execute(self, ctx, cmd);
        }

        super::help::draw_help_window(ctx, &mut self.show_help);
    }
}

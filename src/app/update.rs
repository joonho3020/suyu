use crate::model;
use eframe::egui;
use std::collections::HashSet;

use super::doc_ops::{
    AbutMode, AlignMode, DistributeMode, abut_selected, align_selected, distribute_selected,
    element_label,
};
use super::geometry::{compute_binding_for_target, resolve_binding_point, topmost_bind_target_id};
use super::render::{draw_background, draw_elements, draw_in_progress, style_editor, tool_button};
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

            if !wants_keyboard && !self.inline_text_editing {
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

            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::S) {
                self.save_to_path();
            }
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::O) {
                self.load_from_path();
            }
            let skip_shortcuts = wants_keyboard || self.inline_text_editing;

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
                let move_amount = if i.modifiers.shift { 10.0 } else { 1.0 };
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
                    || i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowLeft)
                {
                    if !self.selected.is_empty() {
                        self.push_undo();
                        self.translate_selected(egui::vec2(-move_amount, 0.0));
                    }
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                    || i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowRight)
                {
                    if !self.selected.is_empty() {
                        self.push_undo();
                        self.translate_selected(egui::vec2(move_amount, 0.0));
                    }
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
                    || i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowUp)
                {
                    if !self.selected.is_empty() {
                        self.push_undo();
                        self.translate_selected(egui::vec2(0.0, -move_amount));
                    }
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                    || i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowDown)
                {
                    if !self.selected.is_empty() {
                        self.push_undo();
                        self.translate_selected(egui::vec2(0.0, move_amount));
                    }
                }
            }
        });

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                tool_button(ui, "Select (V)", Tool::Select, &mut self.tool);
                tool_button(ui, "Rect (R)", Tool::Rectangle, &mut self.tool);
                tool_button(ui, "Ellipse (O)", Tool::Ellipse, &mut self.tool);
                tool_button(ui, "△ (⇧T)", Tool::Triangle, &mut self.tool);
                tool_button(ui, "▱ (⇧P)", Tool::Parallelogram, &mut self.tool);
                tool_button(ui, "⏢ (⇧Z)", Tool::Trapezoid, &mut self.tool);
                tool_button(ui, "Line (L)", Tool::Line, &mut self.tool);
                tool_button(ui, "Arrow (A)", Tool::Arrow, &mut self.tool);
                tool_button(ui, "↔ (⇧A)", Tool::BidirectionalArrow, &mut self.tool);
                tool_button(ui, "⌇ (⇧L)", Tool::Polyline, &mut self.tool);
                tool_button(ui, "Pen (P)", Tool::Pen, &mut self.tool);
                tool_button(ui, "Text (T)", Tool::Text, &mut self.tool);
                tool_button(ui, "Pan (Space)", Tool::Pan, &mut self.tool);
                ui.separator();
                ui.label("File");
                ui.text_edit_singleline(&mut self.file_path);
                if ui.button("Load (Ctrl/Cmd+O)").clicked() {
                    self.load_from_path();
                }
                if ui.button("Save (Ctrl/Cmd+S)").clicked() {
                    self.save_to_path();
                }
                ui.separator();
                if ui.button("Front").clicked() {
                    self.bring_selected_to_front();
                }
                if ui.button("Back").clicked() {
                    self.send_selected_to_back();
                }
                if let Some(status) = &self.status {
                    ui.separator();
                    ui.label(status);
                }
            });
        });

        egui::SidePanel::right("right_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Properties");
                ui.separator();
                ui.checkbox(&mut self.apply_style_to_selection, "Apply to selection");

                if self.selected.len() == 1 {
                    let selected_id = *self.selected.iter().next().unwrap();
                    if let Some(idx) = self.element_index_by_id(selected_id) {
                        let original_style = self.doc.elements[idx].style;
                        let mut style = original_style;
                        let style_changed = style_editor(ui, &mut style) && style != original_style;

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
                            | model::ElementKind::Triangle { rect, label }
                            | model::ElementKind::Parallelogram { rect, label }
                            | model::ElementKind::Trapezoid { rect, label } => {
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
                            _ => {}
                        }
                        if style_changed {
                            self.push_undo();
                            self.doc.elements[idx].style = style;
                        }
                        if push_undo_on_focus {
                            self.push_undo();
                        }
                    } else {
                        let mut style = self.style;
                        if style_editor(ui, &mut style) && style != self.style {
                            self.push_undo();
                            self.style = style;
                        }
                    }
                } else {
                    let mut style = self.style;
                    if style_editor(ui, &mut style) && style != self.style {
                        self.push_undo();
                        self.style = style;
                        if self.apply_style_to_selection && !self.selected.is_empty() {
                            for element in &mut self.doc.elements {
                                if self.selected.contains(&element.id) {
                                    element.style = style;
                                }
                            }
                        }
                    }
                }

                ui.separator();
                ui.heading("Grid & Snap");
                ui.checkbox(&mut self.snap_to_grid, "Snap to grid");
                ui.add(
                    egui::Slider::new(&mut self.grid_size, 8.0..=128.0)
                        .text("Grid size")
                        .logarithmic(true),
                );
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
                ui.heading("Layers");
                ui.separator();
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
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Up").clicked() {
                        self.move_selected_layer_by(1);
                    }
                    if ui.button("Down").clicked() {
                        self.move_selected_layer_by(-1);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Group").clicked() {
                        self.group_selected();
                    }
                    if ui.button("Ungroup").clicked() {
                        self.ungroup_selected();
                    }
                });

                ui.separator();
                ui.label("Shortcuts");
                ui.label("V/R/O/L/A/P/T tools, Del delete, Ctrl/Cmd+S save, Ctrl/Cmd+O load");
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
                }
            } else if let Some(prev) = self.tool_before_pan.take() {
                if self.tool == Tool::Pan {
                    self.tool = prev;
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
                            let mut style = self.style;
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
                        match in_progress {
                            InProgress::DragShape { current, .. } => *current = world_pos,
                            InProgress::DragLine { current, .. } => *current = world_pos,
                            InProgress::Polyline { current, .. } => *current = world_pos,
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
                                    },
                                    Tool::Parallelogram => model::ElementKind::Parallelogram {
                                        rect,
                                        label: String::new(),
                                    },
                                    Tool::Trapezoid => model::ElementKind::Trapezoid {
                                        rect,
                                        label: String::new(),
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
                                    style: self.style,
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
                                    style: self.style,
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
                                    style: self.style,
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
                                    style: self.style,
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
                                if let Some(g) = self.group_of(*id) {
                                    groups.insert(g);
                                }
                            }
                            for g in groups {
                                for id in self.group_members(g) {
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
            if let Some(in_progress) = &self.in_progress {
                draw_in_progress(
                    &painter,
                    origin,
                    &self.view,
                    in_progress,
                    self.tool,
                    self.style,
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
    }
}

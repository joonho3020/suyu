use crate::{model, text_format};
use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::geometry::{
    resolved_line_endpoints_world, rotated_ellipse_points_screen,
    rotated_parallelogram_points_screen, rotated_rect_points_screen,
    rotated_trapezoid_points_screen, rotated_triangle_points_screen,
};
use super::{InProgress, Tool, View};

pub(super) fn tool_button(ui: &mut egui::Ui, label: &str, tool: Tool, selected: &mut Tool) {
    let active = *selected == tool;
    if ui.selectable_label(active, label).clicked() {
        *selected = tool;
    }
}

fn font_id(font_size: f32, family: model::FontFamily) -> egui::FontId {
    match family {
        model::FontFamily::Proportional => {
            egui::FontId::new(font_size, egui::FontFamily::Proportional)
        }
        model::FontFamily::Monospace => egui::FontId::new(font_size, egui::FontFamily::Monospace),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VAlign {
    Center,
    Top,
}

fn script_scale(script: text_format::Script) -> f32 {
    match script {
        text_format::Script::Normal => 1.0,
        text_format::Script::Sub | text_format::Script::Sup => 0.7,
    }
}

fn script_y_offset(font_size: f32, script: text_format::Script) -> f32 {
    match script {
        text_format::Script::Normal => 0.0,
        text_format::Script::Sup => -font_size * 0.35,
        text_format::Script::Sub => font_size * 0.2,
    }
}

fn layout_rich_text_line(
    painter: &egui::Painter,
    spans: &[text_format::Span],
    font_size: f32,
    family: model::FontFamily,
    color: egui::Color32,
) -> Vec<(text_format::Script, Arc<egui::Galley>)> {
    spans
        .iter()
        .map(|s| {
            let size = font_size * script_scale(s.script);
            let galley = painter.layout_no_wrap(s.text.clone(), font_id(size, family), color);
            (s.script, galley)
        })
        .collect()
}

fn measure_rich_text(galleys: &[(text_format::Script, Arc<egui::Galley>)]) -> egui::Vec2 {
    let mut w = 0.0;
    let mut h: f32 = 0.0;
    for (_, g) in galleys {
        let s = g.size();
        w += s.x;
        h = h.max(s.y);
    }
    egui::vec2(w, h)
}

fn rotate_pos_about(center: egui::Pos2, p: egui::Pos2, rotation: f32) -> egui::Pos2 {
    if rotation.abs() <= f32::EPSILON {
        return p;
    }
    let sin = rotation.sin();
    let cos = rotation.cos();
    let v = p - center;
    center + egui::vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

fn draw_rich_text(
    painter: &egui::Painter,
    anchor: egui::Pos2,
    text: &str,
    font_size: f32,
    family: model::FontFamily,
    color: egui::Color32,
    rotation: f32,
    align: model::TextAlign,
    valign: VAlign,
) -> egui::Vec2 {
    let lines = text_format::parse_rich_text_lines(text);
    let line_height = font_size * 1.2;

    let mut line_layouts: Vec<(f32, Vec<(text_format::Script, Arc<egui::Galley>)>)> = Vec::new();
    let mut max_width: f32 = 0.0;
    for line_spans in &lines {
        let galleys = layout_rich_text_line(painter, line_spans, font_size, family, color);
        let line_width = measure_rich_text(&galleys).x;
        max_width = max_width.max(line_width);
        line_layouts.push((line_width, galleys));
    }
    let total_height = lines.len().max(1) as f32 * line_height;

    let y0 = match valign {
        VAlign::Center => anchor.y - total_height * 0.5,
        VAlign::Top => anchor.y,
    };

    let mut y = y0;
    for (line_width, galleys) in line_layouts {
        let x0 = match align {
            model::TextAlign::Left => anchor.x,
            model::TextAlign::Center => anchor.x - line_width * 0.5,
            model::TextAlign::Right => anchor.x - line_width,
        };
        let mut x = x0;
        for (script, galley) in galleys {
            let s = galley.size();
            let y_pos = y + script_y_offset(font_size, script);
            let mut pos = egui::pos2(x, y_pos);
            if rotation.abs() > f32::EPSILON {
                pos = rotate_pos_about(anchor, pos, rotation);
            }
            let mut shape = egui::Shape::galley(pos, galley, color);
            if rotation.abs() > f32::EPSILON {
                if let egui::Shape::Text(ref mut data) = shape {
                    data.angle = rotation;
                }
            }
            painter.add(shape);
            x += s.x;
        }
        y += line_height;
    }

    egui::vec2(max_width, total_height)
}

fn color_row(ui: &mut egui::Ui, rgba: &mut model::Rgba) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let presets = [
            egui::Color32::from_rgb(20, 20, 20),
            egui::Color32::from_rgb(200, 40, 40),
            egui::Color32::from_rgb(40, 140, 60),
            egui::Color32::from_rgb(40, 90, 200),
            egui::Color32::from_rgb(200, 140, 40),
            egui::Color32::from_rgb(130, 60, 180),
        ];
        for c in presets {
            if ui
                .add_sized([18.0, 18.0], egui::Button::new("").fill(c))
                .clicked()
            {
                *rgba = model::Rgba::from_color32(c);
                changed = true;
            }
        }
        let mut arr = [rgba.r, rgba.g, rgba.b, rgba.a];
        if ui.color_edit_button_srgba_unmultiplied(&mut arr).changed() {
            *rgba = model::Rgba {
                r: arr[0],
                g: arr[1],
                b: arr[2],
                a: arr[3],
            };
            changed = true;
        }
    });
    changed
}

pub(super) fn style_editor(ui: &mut egui::Ui, style: &mut model::Style) -> bool {
    let mut changed = false;
    ui.label("Stroke");
    changed |= color_row(ui, &mut style.stroke.color);
    changed |= ui
        .add(egui::Slider::new(&mut style.stroke.width, 0.5..=12.0).text("Width"))
        .changed();
    ui.horizontal(|ui| {
        ui.label("Style:");
        egui::ComboBox::from_id_salt("line_style")
            .selected_text(match style.stroke.line_style {
                model::LineStyle::Solid => "Solid",
                model::LineStyle::Dashed => "Dashed",
                model::LineStyle::Dotted => "Dotted",
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(
                        &mut style.stroke.line_style,
                        model::LineStyle::Solid,
                        "Solid",
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(
                        &mut style.stroke.line_style,
                        model::LineStyle::Dashed,
                        "Dashed",
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(
                        &mut style.stroke.line_style,
                        model::LineStyle::Dotted,
                        "Dotted",
                    )
                    .changed()
                {
                    changed = true;
                }
            });
    });
    ui.separator();
    ui.label("Fill");
    let mut fill_enabled = style.fill.is_some();
    if ui.checkbox(&mut fill_enabled, "Enabled").changed() {
        if fill_enabled {
            style.fill = Some(model::Rgba {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            });
        } else {
            style.fill = None;
        }
        changed = true;
    }
    if let Some(fill) = &mut style.fill {
        changed |= color_row(ui, fill);
    }
    ui.separator();
    ui.label("Text");
    changed |= color_row(ui, &mut style.text_color);
    changed |= ui
        .add(egui::Slider::new(&mut style.text_size, 8.0..=48.0).text("Size"))
        .changed();
    ui.horizontal(|ui| {
        ui.label("Font:");
        egui::ComboBox::from_id_salt("font_family")
            .selected_text(match style.font_family {
                model::FontFamily::Proportional => "Proportional",
                model::FontFamily::Monospace => "Monospace",
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(
                        &mut style.font_family,
                        model::FontFamily::Proportional,
                        "Proportional",
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(
                        &mut style.font_family,
                        model::FontFamily::Monospace,
                        "Monospace",
                    )
                    .changed()
                {
                    changed = true;
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Align:");
        egui::ComboBox::from_id_salt("text_align")
            .selected_text(match style.text_align {
                model::TextAlign::Left => "Left",
                model::TextAlign::Center => "Center",
                model::TextAlign::Right => "Right",
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(&mut style.text_align, model::TextAlign::Left, "Left")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(&mut style.text_align, model::TextAlign::Center, "Center")
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .selectable_value(&mut style.text_align, model::TextAlign::Right, "Right")
                    .changed()
                {
                    changed = true;
                }
            });
    });
    changed
}

pub(super) fn draw_background(painter: &egui::Painter, rect: egui::Rect, view: &View) {
    let bg = painter.ctx().style().visuals.extreme_bg_color;
    painter.rect_filled(rect, 0.0, bg);
    let grid_color = egui::Color32::from_gray(60);
    let spacing_world = 64.0;
    let spacing_screen = spacing_world * view.zoom;
    if spacing_screen >= 24.0 {
        let start = rect.min + view.pan_screen;
        let x0 = ((rect.min.x - start.x) / spacing_screen).floor() * spacing_screen + start.x;
        let y0 = ((rect.min.y - start.y) / spacing_screen).floor() * spacing_screen + start.y;
        let mut x = x0;
        while x < rect.max.x {
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(1.0, grid_color),
            );
            x += spacing_screen;
        }
        let mut y = y0;
        while y < rect.max.y {
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                egui::Stroke::new(1.0, grid_color),
            );
            y += spacing_screen;
        }
    }
}

pub(super) fn draw_elements(
    painter: &egui::Painter,
    origin: egui::Pos2,
    view: &View,
    doc: &model::Document,
    selected: &HashSet<u64>,
) {
    for element in &doc.elements {
        draw_element(
            painter,
            origin,
            view,
            doc,
            element,
            selected.contains(&element.id),
        );
    }
}

pub(super) fn draw_group_selection_boxes(
    painter: &egui::Painter,
    origin: egui::Pos2,
    view: &View,
    doc: &model::Document,
    selected: &HashSet<u64>,
) {
    if doc.groups.is_empty() || selected.is_empty() {
        return;
    }
    let mut parent: HashMap<u64, Option<u64>> = HashMap::new();
    for g in &doc.groups {
        parent.insert(g.id, g.parent_id);
    }

    let mut roots = HashSet::new();
    for id in selected {
        let Some(group_id) = doc
            .elements
            .iter()
            .find(|e| e.id == *id)
            .and_then(|e| e.group_id)
        else {
            continue;
        };
        let mut cur = group_id;
        for _ in 0..256 {
            let Some(pid) = parent.get(&cur).copied().flatten() else {
                break;
            };
            cur = pid;
        }
        roots.insert(cur);
    }

    for root in roots {
        let mut group_ids = HashSet::new();
        let mut stack = vec![root];
        while let Some(g) = stack.pop() {
            if !group_ids.insert(g) {
                continue;
            }
            for child in parent
                .iter()
                .filter_map(|(id, pid)| (*pid == Some(g)).then_some(*id))
            {
                stack.push(child);
            }
        }

        let mut bounds: Option<egui::Rect> = None;
        for e in &doc.elements {
            if e.group_id.is_some_and(|g| group_ids.contains(&g)) {
                let b = e.bounds();
                bounds = Some(bounds.map(|r| r.union(b)).unwrap_or(b));
            }
        }
        let Some(bounds) = bounds else {
            continue;
        };
        let screen_min = view.world_to_screen(origin, bounds.min);
        let screen_max = view.world_to_screen(origin, bounds.max);
        let r = egui::Rect::from_min_max(screen_min, screen_max).expand(6.0);
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 160, 255));
        painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
    }
}

fn draw_element(
    painter: &egui::Painter,
    origin: egui::Pos2,
    view: &View,
    doc: &model::Document,
    element: &model::Element,
    is_selected: bool,
) {
    let stroke = egui::Stroke::new(
        element.style.stroke.width * view.zoom,
        element.style.stroke.color.to_color32(),
    );
    let fill = element.style.fill.map(|c| c.to_color32());
    match &element.kind {
        model::ElementKind::Rect { rect, label } => {
            let points = rotated_rect_points_screen(origin, view, rect.to_rect(), element.rotation);
            painter.add(egui::Shape::convex_polygon(
                points.clone(),
                fill.unwrap_or(egui::Color32::TRANSPARENT),
                stroke,
            ));
            if !label.is_empty() {
                let r = rect.to_rect();
                let c = r.center();
                let padding = 8.0_f32.min(r.width() * 0.25);
                let anchor_unrot = match element.style.text_align {
                    model::TextAlign::Left => egui::pos2(r.left() + padding, c.y),
                    model::TextAlign::Center => c,
                    model::TextAlign::Right => egui::pos2(r.right() - padding, c.y),
                };
                let anchor_world = rotate_pos_about(c, anchor_unrot, element.rotation);
                let center_screen = view.world_to_screen(origin, anchor_world);
                draw_rich_text(
                    painter,
                    center_screen,
                    label,
                    element.style.text_size * view.zoom,
                    element.style.font_family,
                    element.style.text_color.to_color32(),
                    element.rotation,
                    element.style.text_align,
                    VAlign::Center,
                );
            }
            if is_selected {
                draw_polygon_selection(painter, &points);
            }
        }
        model::ElementKind::Ellipse { rect, label } => {
            let points =
                rotated_ellipse_points_screen(origin, view, rect.to_rect(), element.rotation);
            painter.add(egui::Shape::convex_polygon(
                points.clone(),
                fill.unwrap_or(egui::Color32::TRANSPARENT),
                stroke,
            ));
            if !label.is_empty() {
                let r = rect.to_rect();
                let c = r.center();
                let padding = 8.0_f32.min(r.width() * 0.25);
                let anchor_unrot = match element.style.text_align {
                    model::TextAlign::Left => egui::pos2(r.left() + padding, c.y),
                    model::TextAlign::Center => c,
                    model::TextAlign::Right => egui::pos2(r.right() - padding, c.y),
                };
                let anchor_world = rotate_pos_about(c, anchor_unrot, element.rotation);
                let center_screen = view.world_to_screen(origin, anchor_world);
                draw_rich_text(
                    painter,
                    center_screen,
                    label,
                    element.style.text_size * view.zoom,
                    element.style.font_family,
                    element.style.text_color.to_color32(),
                    element.rotation,
                    element.style.text_align,
                    VAlign::Center,
                );
            }
            if is_selected {
                draw_polygon_selection(painter, &points);
            }
        }
        model::ElementKind::Triangle {
            rect,
            label,
            apex_ratio,
        } => {
            let points = rotated_triangle_points_screen(
                origin,
                view,
                rect.to_rect(),
                element.rotation,
                *apex_ratio,
            );
            painter.add(egui::Shape::convex_polygon(
                points.clone(),
                fill.unwrap_or(egui::Color32::TRANSPARENT),
                stroke,
            ));
            if !label.is_empty() {
                let r = rect.to_rect();
                let c = r.center();
                let padding = 8.0_f32.min(r.width() * 0.25);
                let anchor_unrot = match element.style.text_align {
                    model::TextAlign::Left => egui::pos2(r.left() + padding, c.y),
                    model::TextAlign::Center => c,
                    model::TextAlign::Right => egui::pos2(r.right() - padding, c.y),
                };
                let anchor_world = rotate_pos_about(c, anchor_unrot, element.rotation);
                let center_screen = view.world_to_screen(origin, anchor_world);
                draw_rich_text(
                    painter,
                    center_screen,
                    label,
                    element.style.text_size * view.zoom,
                    element.style.font_family,
                    element.style.text_color.to_color32(),
                    element.rotation,
                    element.style.text_align,
                    VAlign::Center,
                );
            }
            if is_selected {
                draw_polygon_selection(painter, &points);
            }
        }
        model::ElementKind::Parallelogram {
            rect,
            label,
            skew_ratio,
        } => {
            let points = rotated_parallelogram_points_screen(
                origin,
                view,
                rect.to_rect(),
                element.rotation,
                *skew_ratio,
            );
            painter.add(egui::Shape::convex_polygon(
                points.clone(),
                fill.unwrap_or(egui::Color32::TRANSPARENT),
                stroke,
            ));
            if !label.is_empty() {
                let r = rect.to_rect();
                let c = r.center();
                let padding = 8.0_f32.min(r.width() * 0.25);
                let anchor_unrot = match element.style.text_align {
                    model::TextAlign::Left => egui::pos2(r.left() + padding, c.y),
                    model::TextAlign::Center => c,
                    model::TextAlign::Right => egui::pos2(r.right() - padding, c.y),
                };
                let anchor_world = rotate_pos_about(c, anchor_unrot, element.rotation);
                let center_screen = view.world_to_screen(origin, anchor_world);
                draw_rich_text(
                    painter,
                    center_screen,
                    label,
                    element.style.text_size * view.zoom,
                    element.style.font_family,
                    element.style.text_color.to_color32(),
                    element.rotation,
                    element.style.text_align,
                    VAlign::Center,
                );
            }
            if is_selected {
                draw_polygon_selection(painter, &points);
            }
        }
        model::ElementKind::Trapezoid {
            rect,
            label,
            top_inset_ratio,
        } => {
            let points = rotated_trapezoid_points_screen(
                origin,
                view,
                rect.to_rect(),
                element.rotation,
                *top_inset_ratio,
            );
            painter.add(egui::Shape::convex_polygon(
                points.clone(),
                fill.unwrap_or(egui::Color32::TRANSPARENT),
                stroke,
            ));
            if !label.is_empty() {
                let r = rect.to_rect();
                let c = r.center();
                let padding = 8.0_f32.min(r.width() * 0.25);
                let anchor_unrot = match element.style.text_align {
                    model::TextAlign::Left => egui::pos2(r.left() + padding, c.y),
                    model::TextAlign::Center => c,
                    model::TextAlign::Right => egui::pos2(r.right() - padding, c.y),
                };
                let anchor_world = rotate_pos_about(c, anchor_unrot, element.rotation);
                let center_screen = view.world_to_screen(origin, anchor_world);
                draw_rich_text(
                    painter,
                    center_screen,
                    label,
                    element.style.text_size * view.zoom,
                    element.style.font_family,
                    element.style.text_color.to_color32(),
                    element.rotation,
                    element.style.text_align,
                    VAlign::Center,
                );
            }
            if is_selected {
                draw_polygon_selection(painter, &points);
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
            let (a, b) = resolved_line_endpoints_world(doc, *a, *b, start_binding, end_binding);
            let a = view.world_to_screen(origin, a);
            let b = view.world_to_screen(origin, b);
            draw_styled_line(painter, a, b, stroke, element.style.stroke.line_style);
            let has_end_arrow = *arrow
                || matches!(
                    arrow_style,
                    model::ArrowStyle::End | model::ArrowStyle::Both
                );
            let has_start_arrow = matches!(
                arrow_style,
                model::ArrowStyle::Start | model::ArrowStyle::Both
            );
            if has_end_arrow {
                draw_arrowhead(painter, a, b, stroke);
            }
            if has_start_arrow {
                draw_arrowhead(painter, b, a, stroke);
            }
            if is_selected {
                let r = egui::Rect::from_two_pos(a, b).expand(6.0);
                draw_selection_bounds(painter, r);
            }
        }
        model::ElementKind::Polyline {
            points,
            arrow_style,
        } => {
            if points.len() >= 2 {
                let pts: Vec<egui::Pos2> = points
                    .iter()
                    .map(|p| view.world_to_screen(origin, p.to_pos2()))
                    .collect();
                draw_styled_polyline(painter, &pts, stroke, element.style.stroke.line_style);
                let has_end_arrow = matches!(
                    arrow_style,
                    model::ArrowStyle::End | model::ArrowStyle::Both
                );
                let has_start_arrow = matches!(
                    arrow_style,
                    model::ArrowStyle::Start | model::ArrowStyle::Both
                );
                if has_end_arrow && pts.len() >= 2 {
                    let last = pts[pts.len() - 1];
                    let second_last = pts[pts.len() - 2];
                    draw_arrowhead(painter, second_last, last, stroke);
                }
                if has_start_arrow && pts.len() >= 2 {
                    let first = pts[0];
                    let second = pts[1];
                    draw_arrowhead(painter, second, first, stroke);
                }
                if is_selected {
                    let mut b: Option<egui::Rect> = None;
                    for p in &pts {
                        let r = egui::Rect::from_min_max(*p, *p);
                        b = Some(b.map(|prev| prev.union(r)).unwrap_or(r));
                    }
                    if let Some(b) = b {
                        draw_selection_bounds(painter, b.expand(6.0));
                    }
                }
            }
        }
        model::ElementKind::Pen { points } => {
            if points.len() >= 2 {
                let pts: Vec<egui::Pos2> = points
                    .iter()
                    .map(|p| view.world_to_screen(origin, p.to_pos2()))
                    .collect();
                painter.add(egui::Shape::line(pts, stroke));
                if is_selected {
                    let mut b: Option<egui::Rect> = None;
                    for p in points {
                        let sp = view.world_to_screen(origin, p.to_pos2());
                        let r = egui::Rect::from_min_max(sp, sp);
                        b = Some(b.map(|prev| prev.union(r)).unwrap_or(r));
                    }
                    if let Some(b) = b {
                        draw_selection_bounds(painter, b.expand(6.0));
                    }
                }
            }
        }
        model::ElementKind::Text { pos, text } => {
            let pos = view.world_to_screen(origin, pos.to_pos2());
            let size = draw_rich_text(
                painter,
                pos,
                text,
                element.style.text_size * view.zoom,
                element.style.font_family,
                element.style.text_color.to_color32(),
                0.0,
                element.style.text_align,
                VAlign::Top,
            );
            if is_selected {
                let x = match element.style.text_align {
                    model::TextAlign::Left => pos.x,
                    model::TextAlign::Center => pos.x - size.x * 0.5,
                    model::TextAlign::Right => pos.x - size.x,
                };
                let r = egui::Rect::from_min_size(egui::pos2(x, pos.y), size);
                draw_selection_bounds(painter, r);
            }
        }
    }
}

pub(super) fn draw_in_progress(
    painter: &egui::Painter,
    origin: egui::Pos2,
    view: &View,
    in_progress: &InProgress,
    tool: Tool,
    style: model::Style,
) {
    let stroke = egui::Stroke::new(
        style.stroke.width * view.zoom,
        style.stroke.color.to_color32(),
    );
    match in_progress {
        InProgress::DragShape { start, current } => {
            let world_rect = egui::Rect::from_two_pos(*start, *current);
            match tool {
                Tool::Rectangle => {
                    let r = egui::Rect::from_two_pos(
                        view.world_to_screen(origin, *start),
                        view.world_to_screen(origin, *current),
                    );
                    painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
                }
                Tool::Ellipse => {
                    let r = egui::Rect::from_two_pos(
                        view.world_to_screen(origin, *start),
                        view.world_to_screen(origin, *current),
                    );
                    painter.add(egui::Shape::ellipse_stroke(
                        r.center(),
                        r.size() * 0.5,
                        stroke,
                    ));
                }
                Tool::Triangle => {
                    let pts = rotated_triangle_points_screen(origin, view, world_rect, 0.0, 0.0);
                    painter.add(egui::Shape::closed_line(pts, stroke));
                }
                Tool::Parallelogram => {
                    let pts =
                        rotated_parallelogram_points_screen(origin, view, world_rect, 0.0, 0.25);
                    painter.add(egui::Shape::closed_line(pts, stroke));
                }
                Tool::Trapezoid => {
                    let pts = rotated_trapezoid_points_screen(origin, view, world_rect, 0.0, 0.25);
                    painter.add(egui::Shape::closed_line(pts, stroke));
                }
                _ => {
                    let r = egui::Rect::from_two_pos(
                        view.world_to_screen(origin, *start),
                        view.world_to_screen(origin, *current),
                    );
                    painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
                }
            };
        }
        InProgress::DragLine {
            start,
            current,
            arrow_style,
        } => {
            let a = view.world_to_screen(origin, *start);
            let b = view.world_to_screen(origin, *current);
            draw_styled_line(painter, a, b, stroke, style.stroke.line_style);
            let has_end_arrow = matches!(
                arrow_style,
                model::ArrowStyle::End | model::ArrowStyle::Both
            );
            let has_start_arrow = matches!(
                arrow_style,
                model::ArrowStyle::Start | model::ArrowStyle::Both
            );
            if has_end_arrow {
                draw_arrowhead(painter, a, b, stroke);
            }
            if has_start_arrow {
                draw_arrowhead(painter, b, a, stroke);
            }
        }
        InProgress::Polyline {
            points,
            current,
            arrow_style,
        } => {
            let mut all_pts: Vec<egui::Pos2> = points
                .iter()
                .map(|p| view.world_to_screen(origin, *p))
                .collect();
            all_pts.push(view.world_to_screen(origin, *current));
            if all_pts.len() >= 2 {
                draw_styled_polyline(painter, &all_pts, stroke, style.stroke.line_style);
                let has_end_arrow = matches!(
                    arrow_style,
                    model::ArrowStyle::End | model::ArrowStyle::Both
                );
                let has_start_arrow = matches!(
                    arrow_style,
                    model::ArrowStyle::Start | model::ArrowStyle::Both
                );
                if has_end_arrow {
                    let last = all_pts[all_pts.len() - 1];
                    let second_last = all_pts[all_pts.len() - 2];
                    draw_arrowhead(painter, second_last, last, stroke);
                }
                if has_start_arrow && all_pts.len() >= 2 {
                    let first = all_pts[0];
                    let second = all_pts[1];
                    draw_arrowhead(painter, second, first, stroke);
                }
            }
        }
        InProgress::Pen { points } => {
            if points.len() >= 2 {
                let pts: Vec<egui::Pos2> = points
                    .iter()
                    .map(|p| view.world_to_screen(origin, *p))
                    .collect();
                painter.add(egui::Shape::line(pts, stroke));
            }
        }
        InProgress::SelectBox { start, current } => {
            let r = egui::Rect::from_two_pos(
                view.world_to_screen(origin, *start),
                view.world_to_screen(origin, *current),
            );
            let s = egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 160, 255));
            painter.rect_stroke(r, 0.0, s, egui::StrokeKind::Middle);
        }
    }
}

fn draw_selection_bounds(painter: &egui::Painter, rect: egui::Rect) {
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 160, 255));
    painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Middle);
}

fn draw_polygon_selection(painter: &egui::Painter, points: &[egui::Pos2]) {
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 160, 255));
    painter.add(egui::Shape::closed_line(points.to_vec(), stroke));
}

fn draw_styled_line(
    painter: &egui::Painter,
    a: egui::Pos2,
    b: egui::Pos2,
    stroke: egui::Stroke,
    line_style: model::LineStyle,
) {
    match line_style {
        model::LineStyle::Solid => {
            painter.line_segment([a, b], stroke);
        }
        model::LineStyle::Dashed => {
            draw_dashed_line(painter, a, b, stroke, 10.0, 5.0);
        }
        model::LineStyle::Dotted => {
            draw_dashed_line(painter, a, b, stroke, 2.0, 4.0);
        }
    }
}

fn draw_styled_polyline(
    painter: &egui::Painter,
    points: &[egui::Pos2],
    stroke: egui::Stroke,
    line_style: model::LineStyle,
) {
    if points.len() < 2 {
        return;
    }
    match line_style {
        model::LineStyle::Solid => {
            painter.add(egui::Shape::line(points.to_vec(), stroke));
        }
        model::LineStyle::Dashed | model::LineStyle::Dotted => {
            let (dash, gap) = if line_style == model::LineStyle::Dashed {
                (10.0, 5.0)
            } else {
                (2.0, 4.0)
            };
            for pair in points.windows(2) {
                draw_dashed_line(painter, pair[0], pair[1], stroke, dash, gap);
            }
        }
    }
}

fn draw_dashed_line(
    painter: &egui::Painter,
    a: egui::Pos2,
    b: egui::Pos2,
    stroke: egui::Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let v = b - a;
    let len = v.length();
    if len <= f32::EPSILON {
        return;
    }
    let dir = v / len;
    let mut pos = 0.0;
    let mut drawing = true;
    while pos < len {
        let seg_len = if drawing { dash_len } else { gap_len };
        let next_pos = (pos + seg_len).min(len);
        if drawing {
            let start = a + dir * pos;
            let end = a + dir * next_pos;
            painter.line_segment([start, end], stroke);
        }
        pos = next_pos;
        drawing = !drawing;
    }
}

fn draw_arrowhead(painter: &egui::Painter, a: egui::Pos2, b: egui::Pos2, stroke: egui::Stroke) {
    let v = b - a;
    if v.length_sq() <= f32::EPSILON {
        return;
    }
    let dir = v.normalized();
    let size = 10.0;
    let perp = egui::vec2(-dir.y, dir.x);
    let tip = b;
    let base = b - dir * size;
    let left = base + perp * (size * 0.6);
    let right = base - perp * (size * 0.6);
    painter.add(egui::Shape::convex_polygon(
        vec![tip, left, right],
        stroke.color,
        egui::Stroke::NONE,
    ));
}

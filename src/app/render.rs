use crate::model;
use eframe::egui;
use std::collections::HashSet;

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
                let center_world = rect.to_rect().center();
                let center_screen = view.world_to_screen(origin, center_world);
                draw_rotated_text(
                    painter,
                    center_screen,
                    label,
                    element.style.text_size * view.zoom,
                    element.style.text_color.to_color32(),
                    element.rotation,
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
                let center_world = rect.to_rect().center();
                let center_screen = view.world_to_screen(origin, center_world);
                draw_rotated_text(
                    painter,
                    center_screen,
                    label,
                    element.style.text_size * view.zoom,
                    element.style.text_color.to_color32(),
                    element.rotation,
                );
            }
            if is_selected {
                draw_polygon_selection(painter, &points);
            }
        }
        model::ElementKind::Triangle { rect, label } => {
            let points =
                rotated_triangle_points_screen(origin, view, rect.to_rect(), element.rotation);
            painter.add(egui::Shape::convex_polygon(
                points.clone(),
                fill.unwrap_or(egui::Color32::TRANSPARENT),
                stroke,
            ));
            if !label.is_empty() {
                let center_world = rect.to_rect().center();
                let center_screen = view.world_to_screen(origin, center_world);
                draw_rotated_text(
                    painter,
                    center_screen,
                    label,
                    element.style.text_size * view.zoom,
                    element.style.text_color.to_color32(),
                    element.rotation,
                );
            }
            if is_selected {
                draw_polygon_selection(painter, &points);
            }
        }
        model::ElementKind::Parallelogram { rect, label } => {
            let points =
                rotated_parallelogram_points_screen(origin, view, rect.to_rect(), element.rotation);
            painter.add(egui::Shape::convex_polygon(
                points.clone(),
                fill.unwrap_or(egui::Color32::TRANSPARENT),
                stroke,
            ));
            if !label.is_empty() {
                let center_world = rect.to_rect().center();
                let center_screen = view.world_to_screen(origin, center_world);
                draw_rotated_text(
                    painter,
                    center_screen,
                    label,
                    element.style.text_size * view.zoom,
                    element.style.text_color.to_color32(),
                    element.rotation,
                );
            }
            if is_selected {
                draw_polygon_selection(painter, &points);
            }
        }
        model::ElementKind::Trapezoid { rect, label } => {
            let points =
                rotated_trapezoid_points_screen(origin, view, rect.to_rect(), element.rotation);
            painter.add(egui::Shape::convex_polygon(
                points.clone(),
                fill.unwrap_or(egui::Color32::TRANSPARENT),
                stroke,
            ));
            if !label.is_empty() {
                let center_world = rect.to_rect().center();
                let center_screen = view.world_to_screen(origin, center_world);
                draw_rotated_text(
                    painter,
                    center_screen,
                    label,
                    element.style.text_size * view.zoom,
                    element.style.text_color.to_color32(),
                    element.rotation,
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
            let font_id = egui::FontId::proportional(element.style.text_size * view.zoom);
            painter.text(
                pos,
                egui::Align2::LEFT_TOP,
                text,
                font_id,
                element.style.text_color.to_color32(),
            );
            if is_selected {
                let w = (text.chars().count() as f32).max(1.0)
                    * element.style.text_size
                    * 0.6
                    * view.zoom;
                let h = element.style.text_size * 1.2 * view.zoom;
                let r = egui::Rect::from_min_size(pos, egui::vec2(w, h));
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
                    let pts = rotated_triangle_points_screen(origin, view, world_rect, 0.0);
                    painter.add(egui::Shape::closed_line(pts, stroke));
                }
                Tool::Parallelogram => {
                    let pts = rotated_parallelogram_points_screen(origin, view, world_rect, 0.0);
                    painter.add(egui::Shape::closed_line(pts, stroke));
                }
                Tool::Trapezoid => {
                    let pts = rotated_trapezoid_points_screen(origin, view, world_rect, 0.0);
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

fn draw_rotated_text(
    painter: &egui::Painter,
    center: egui::Pos2,
    text: &str,
    font_size: f32,
    color: egui::Color32,
    rotation: f32,
) {
    let font_id = egui::FontId::proportional(font_size);
    let galley = painter.layout_no_wrap(text.to_string(), font_id, color);
    let galley_size = galley.size();

    if rotation.abs() <= f32::EPSILON {
        let text_offset = egui::vec2(-galley_size.x * 0.5, -galley_size.y * 0.5);
        let text_pos = center + text_offset;
        painter.galley(text_pos, galley, color);
    } else {
        let text_offset = egui::vec2(-galley_size.x * 0.5, -galley_size.y * 0.5);
        let text_pos = center + text_offset;
        let mut mesh = egui::Mesh::default();
        mesh.add_rect_with_uv(
            egui::Rect::from_min_size(text_pos, galley_size),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            color,
        );
        let sin = rotation.sin();
        let cos = rotation.cos();
        for vertex in &mut mesh.vertices {
            let v = vertex.pos - center;
            let rotated = egui::pos2(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
            vertex.pos = center + rotated.to_vec2();
        }
        let mut text_shape = egui::Shape::galley(text_pos, galley, color);
        if let egui::Shape::Text(ref mut text_shape_data) = text_shape {
            let galley_pos = text_shape_data.pos;
            let offset = galley_pos - center;
            let rotated_offset = egui::vec2(
                offset.x * cos - offset.y * sin,
                offset.x * sin + offset.y * cos,
            );
            text_shape_data.pos = center + rotated_offset;
            text_shape_data.angle = rotation;
        }
        painter.add(text_shape);
    }
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

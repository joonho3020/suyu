use crate::model;
use eframe::egui;

use super::View;

pub(super) fn rotate_vec2(v: egui::Vec2, angle: f32) -> egui::Vec2 {
    let sin = angle.sin();
    let cos = angle.cos();
    egui::vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

pub(super) fn rotated_rect_points_screen(
    origin: egui::Pos2,
    view: &View,
    rect: egui::Rect,
    rotation: f32,
) -> Vec<egui::Pos2> {
    rotated_rect_points_world(rect, rotation)
        .into_iter()
        .map(|w| view.world_to_screen(origin, w))
        .collect()
}

pub(super) fn rotated_ellipse_points_screen(
    origin: egui::Pos2,
    view: &View,
    rect: egui::Rect,
    rotation: f32,
) -> Vec<egui::Pos2> {
    rotated_ellipse_points_world(rect, rotation)
        .into_iter()
        .map(|w| view.world_to_screen(origin, w))
        .collect()
}

pub(super) fn rotated_triangle_points_screen(
    origin: egui::Pos2,
    view: &View,
    rect: egui::Rect,
    rotation: f32,
    apex_ratio: f32,
) -> Vec<egui::Pos2> {
    rotated_triangle_points_world(rect, rotation, apex_ratio)
        .into_iter()
        .map(|w| view.world_to_screen(origin, w))
        .collect()
}

pub(super) fn rotated_parallelogram_points_screen(
    origin: egui::Pos2,
    view: &View,
    rect: egui::Rect,
    rotation: f32,
    skew_ratio: f32,
) -> Vec<egui::Pos2> {
    rotated_parallelogram_points_world(rect, rotation, skew_ratio)
        .into_iter()
        .map(|w| view.world_to_screen(origin, w))
        .collect()
}

pub(super) fn rotated_trapezoid_points_screen(
    origin: egui::Pos2,
    view: &View,
    rect: egui::Rect,
    rotation: f32,
    top_inset_ratio: f32,
) -> Vec<egui::Pos2> {
    rotated_trapezoid_points_world(rect, rotation, top_inset_ratio)
        .into_iter()
        .map(|w| view.world_to_screen(origin, w))
        .collect()
}

pub(super) fn rotated_rect_points_world(rect: egui::Rect, rotation: f32) -> Vec<egui::Pos2> {
    let center = rect.center();
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    corners
        .into_iter()
        .map(|p| {
            let v = p - center;
            center + rotate_vec2(v, rotation)
        })
        .collect()
}

pub(super) fn rotated_ellipse_points_world(rect: egui::Rect, rotation: f32) -> Vec<egui::Pos2> {
    let center = rect.center();
    let rx = rect.width() * 0.5;
    let ry = rect.height() * 0.5;
    if rx <= f32::EPSILON || ry <= f32::EPSILON {
        return vec![];
    }
    let steps = 48;
    (0..steps)
        .map(|i| {
            let t = (i as f32) / (steps as f32) * std::f32::consts::TAU;
            let local = egui::vec2(t.cos() * rx, t.sin() * ry);
            center + rotate_vec2(local, rotation)
        })
        .collect()
}

pub(super) fn rotated_triangle_points_world(
    rect: egui::Rect,
    rotation: f32,
    apex_ratio: f32,
) -> Vec<egui::Pos2> {
    let center = rect.center();
    let half_w = rect.width() * 0.5;
    let half_h = rect.height() * 0.5;
    let apex = (apex_ratio.clamp(-1.0, 1.0)) * half_w;
    let corners = [
        egui::vec2(apex, -half_h),
        egui::vec2(half_w, half_h),
        egui::vec2(-half_w, half_h),
    ];
    corners
        .into_iter()
        .map(|v| center + rotate_vec2(v, rotation))
        .collect()
}

pub(super) fn rotated_parallelogram_points_world(
    rect: egui::Rect,
    rotation: f32,
    skew_ratio: f32,
) -> Vec<egui::Pos2> {
    let center = rect.center();
    let half_w = rect.width() * 0.5;
    let half_h = rect.height() * 0.5;
    let skew = (skew_ratio.clamp(-0.95, 0.95)) * half_w;
    let corners = [
        egui::vec2(-half_w + skew, -half_h),
        egui::vec2(half_w + skew, -half_h),
        egui::vec2(half_w - skew, half_h),
        egui::vec2(-half_w - skew, half_h),
    ];
    corners
        .into_iter()
        .map(|v| center + rotate_vec2(v, rotation))
        .collect()
}

pub(super) fn rotated_trapezoid_points_world(
    rect: egui::Rect,
    rotation: f32,
    top_inset_ratio: f32,
) -> Vec<egui::Pos2> {
    let center = rect.center();
    let half_w = rect.width() * 0.5;
    let half_h = rect.height() * 0.5;
    let top_inset = (top_inset_ratio.clamp(0.0, 0.95)) * half_w;
    let corners = [
        egui::vec2(-half_w + top_inset, -half_h),
        egui::vec2(half_w - top_inset, -half_h),
        egui::vec2(half_w, half_h),
        egui::vec2(-half_w, half_h),
    ];
    corners
        .into_iter()
        .map(|v| center + rotate_vec2(v, rotation))
        .collect()
}

#[allow(dead_code)]
fn aabb_of_points(points: &[egui::Pos2]) -> egui::Rect {
    let mut min = egui::pos2(f32::INFINITY, f32::INFINITY);
    let mut max = egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY);
    for p in points {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    if min.x.is_finite() && min.y.is_finite() && max.x.is_finite() && max.y.is_finite() {
        egui::Rect::from_min_max(min, max)
    } else {
        egui::Rect::NOTHING
    }
}

fn hit_test_rotated_rect(
    rect: egui::Rect,
    rotation: f32,
    world_pos: egui::Pos2,
    threshold_world: f32,
) -> bool {
    let center = rect.center();
    let half = rect.size() * 0.5;
    let v = world_pos - center;
    let local = rotate_vec2(v, -rotation);
    local.x.abs() <= half.x + threshold_world && local.y.abs() <= half.y + threshold_world
}

fn hit_test_rotated_ellipse(
    rect: egui::Rect,
    rotation: f32,
    world_pos: egui::Pos2,
    threshold_world: f32,
) -> bool {
    let center = rect.center();
    let rx = rect.width() * 0.5 + threshold_world;
    let ry = rect.height() * 0.5 + threshold_world;
    if rx <= f32::EPSILON || ry <= f32::EPSILON {
        return false;
    }
    let v = world_pos - center;
    let local = rotate_vec2(v, -rotation);
    let dx = local.x / rx;
    let dy = local.y / ry;
    dx * dx + dy * dy <= 1.0
}

pub(super) fn resolved_line_endpoints_world(
    doc: &model::Document,
    a: model::Point,
    b: model::Point,
    start_binding: &Option<model::Binding>,
    end_binding: &Option<model::Binding>,
) -> (egui::Pos2, egui::Pos2) {
    let a0 = a.to_pos2();
    let b0 = b.to_pos2();
    let a = start_binding
        .as_ref()
        .and_then(|bind| resolve_binding_point(doc, bind))
        .unwrap_or(a0);
    let b = end_binding
        .as_ref()
        .and_then(|bind| resolve_binding_point(doc, bind))
        .unwrap_or(b0);
    (a, b)
}

pub(super) fn topmost_bind_target_id(
    doc: &model::Document,
    world_pos: egui::Pos2,
    threshold_world: f32,
) -> Option<u64> {
    for element in doc.elements.iter().rev() {
        match &element.kind {
            model::ElementKind::Rect { .. }
            | model::ElementKind::Ellipse { .. }
            | model::ElementKind::Triangle { .. }
            | model::ElementKind::Parallelogram { .. }
            | model::ElementKind::Trapezoid { .. } => {
                if hit_test_element(doc, element, world_pos, threshold_world) {
                    return Some(element.id);
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn compute_binding_for_target(
    doc: &model::Document,
    target_id: u64,
    world_pos: egui::Pos2,
) -> Option<model::Binding> {
    let element = doc.elements.iter().find(|e| e.id == target_id)?;
    compute_binding_for_element(element, world_pos)
}

fn compute_binding_for_element(
    element: &model::Element,
    world_pos: egui::Pos2,
) -> Option<model::Binding> {
    let rect = match &element.kind {
        model::ElementKind::Rect { rect, .. }
        | model::ElementKind::Ellipse { rect, .. }
        | model::ElementKind::Triangle { rect, .. }
        | model::ElementKind::Parallelogram { rect, .. }
        | model::ElementKind::Trapezoid { rect, .. } => rect.to_rect(),
        _ => return None,
    };
    let size = rect.size();
    if size.x <= f32::EPSILON || size.y <= f32::EPSILON {
        return None;
    }
    let center = rect.center();
    let local = rotate_vec2(world_pos - center, -element.rotation);
    let mut nx = local.x / size.x;
    let mut ny = local.y / size.y;
    nx = nx.clamp(-0.5, 0.5);
    ny = ny.clamp(-0.5, 0.5);
    match &element.kind {
        model::ElementKind::Rect { .. }
        | model::ElementKind::Triangle { .. }
        | model::ElementKind::Parallelogram { .. }
        | model::ElementKind::Trapezoid { .. } => {
            let dx = 0.5 - nx.abs();
            let dy = 0.5 - ny.abs();
            if dx < dy {
                nx = nx.signum() * 0.5;
            } else {
                ny = ny.signum() * 0.5;
            }
        }
        model::ElementKind::Ellipse { .. } => {
            let v = egui::vec2(nx, ny);
            let n = v.length();
            if n <= f32::EPSILON {
                nx = 0.5;
                ny = 0.0;
            } else {
                let u = v / n * 0.5;
                nx = u.x;
                ny = u.y;
            }
        }
        _ => {}
    }
    Some(model::Binding {
        element_id: element.id,
        norm: model::Point { x: nx, y: ny },
    })
}

pub(super) fn resolve_binding_point(
    doc: &model::Document,
    binding: &model::Binding,
) -> Option<egui::Pos2> {
    let element = doc.elements.iter().find(|e| e.id == binding.element_id)?;
    let (rect, rotation) = match &element.kind {
        model::ElementKind::Rect { rect, .. }
        | model::ElementKind::Ellipse { rect, .. }
        | model::ElementKind::Triangle { rect, .. }
        | model::ElementKind::Parallelogram { rect, .. }
        | model::ElementKind::Trapezoid { rect, .. } => Some((rect.to_rect(), element.rotation)),
        _ => None,
    }?;
    let center = rect.center();
    let size = rect.size();
    let local = egui::vec2(binding.norm.x * size.x, binding.norm.y * size.y);
    Some(center + rotate_vec2(local, rotation))
}

pub(super) fn translate_element(element: &mut model::Element, delta_world: egui::Vec2) {
    match &mut element.kind {
        model::ElementKind::Rect { rect, .. }
        | model::ElementKind::Ellipse { rect, .. }
        | model::ElementKind::Triangle { rect, .. }
        | model::ElementKind::Parallelogram { rect, .. }
        | model::ElementKind::Trapezoid { rect, .. } => {
            rect.min.x += delta_world.x;
            rect.min.y += delta_world.y;
            rect.max.x += delta_world.x;
            rect.max.y += delta_world.y;
        }
        model::ElementKind::Line { a, b, .. } => {
            a.x += delta_world.x;
            a.y += delta_world.y;
            b.x += delta_world.x;
            b.y += delta_world.y;
        }
        model::ElementKind::Polyline { points, .. } | model::ElementKind::Pen { points } => {
            for p in points {
                p.x += delta_world.x;
                p.y += delta_world.y;
            }
        }
        model::ElementKind::Text { pos, .. } => {
            pos.x += delta_world.x;
            pos.y += delta_world.y;
        }
    }
}

pub(super) fn snap_element_to_grid(element: &mut model::Element, grid_size: f32) {
    match &mut element.kind {
        model::ElementKind::Rect { rect, .. }
        | model::ElementKind::Ellipse { rect, .. }
        | model::ElementKind::Triangle { rect, .. }
        | model::ElementKind::Parallelogram { rect, .. }
        | model::ElementKind::Trapezoid { rect, .. } => {
            let min = rect.min.to_pos2();
            let snapped_min = egui::pos2(
                (min.x / grid_size).round() * grid_size,
                (min.y / grid_size).round() * grid_size,
            );
            let delta = snapped_min - min;
            rect.min.x += delta.x;
            rect.min.y += delta.y;
            rect.max.x += delta.x;
            rect.max.y += delta.y;
        }
        model::ElementKind::Text { pos, .. } => {
            let p = pos.to_pos2();
            let snapped = egui::pos2(
                (p.x / grid_size).round() * grid_size,
                (p.y / grid_size).round() * grid_size,
            );
            pos.x = snapped.x;
            pos.y = snapped.y;
        }
        _ => {}
    }
}

pub(super) fn hit_test_element(
    doc: &model::Document,
    element: &model::Element,
    world_pos: egui::Pos2,
    threshold_world: f32,
) -> bool {
    match &element.kind {
        model::ElementKind::Rect { rect, .. }
        | model::ElementKind::Triangle { rect, .. }
        | model::ElementKind::Parallelogram { rect, .. }
        | model::ElementKind::Trapezoid { rect, .. } => {
            hit_test_rotated_rect(rect.to_rect(), element.rotation, world_pos, threshold_world)
        }
        model::ElementKind::Ellipse { rect, .. } => {
            hit_test_rotated_ellipse(rect.to_rect(), element.rotation, world_pos, threshold_world)
        }
        model::ElementKind::Line {
            a,
            b,
            start_binding,
            end_binding,
            ..
        } => {
            let (a, b) = resolved_line_endpoints_world(doc, *a, *b, start_binding, end_binding);
            model::distance_to_segment(world_pos, a, b)
                <= (threshold_world + element.style.stroke.width)
        }
        model::ElementKind::Polyline { points, .. } | model::ElementKind::Pen { points } => {
            if points.len() < 2 {
                return false;
            }
            let mut prev = points[0].to_pos2();
            for p in &points[1..] {
                let p = p.to_pos2();
                if model::distance_to_segment(world_pos, prev, p)
                    <= (threshold_world + element.style.stroke.width)
                {
                    return true;
                }
                prev = p;
            }
            false
        }
        model::ElementKind::Text { pos, text } => {
            let pos = pos.to_pos2();
            let w = (text.chars().count() as f32).max(1.0) * element.style.text_size * 0.6;
            let h = element.style.text_size * 1.2;
            egui::Rect::from_min_size(pos, egui::vec2(w, h))
                .expand(threshold_world)
                .contains(world_pos)
        }
    }
}

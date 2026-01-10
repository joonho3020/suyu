use crate::model;
use eframe::egui;
use std::collections::HashSet;

use super::geometry::translate_element;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AlignMode {
    Left,
    HCenter,
    Right,
    Top,
    VCenter,
    Bottom,
}

pub(super) fn align_selected(doc: &mut model::Document, selected: &HashSet<u64>, mode: AlignMode) {
    if selected.len() < 2 {
        return;
    }
    let mut items = Vec::new();
    for e in &doc.elements {
        if selected.contains(&e.id) {
            items.push((e.id, e.bounds()));
        }
    }
    if items.len() < 2 {
        return;
    }
    let overall = items
        .iter()
        .map(|(_, r)| *r)
        .reduce(|a, b| a.union(b))
        .unwrap_or(egui::Rect::NOTHING);

    for (id, b) in items {
        let delta = match mode {
            AlignMode::Left => egui::vec2(overall.min.x - b.min.x, 0.0),
            AlignMode::HCenter => egui::vec2(overall.center().x - b.center().x, 0.0),
            AlignMode::Right => egui::vec2(overall.max.x - b.max.x, 0.0),
            AlignMode::Top => egui::vec2(0.0, overall.min.y - b.min.y),
            AlignMode::VCenter => egui::vec2(0.0, overall.center().y - b.center().y),
            AlignMode::Bottom => egui::vec2(0.0, overall.max.y - b.max.y),
        };
        if let Some(e) = doc.elements.iter_mut().find(|e| e.id == id) {
            translate_element(e, delta);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DistributeMode {
    Horizontal,
    Vertical,
}

pub(super) fn distribute_selected(
    doc: &mut model::Document,
    selected: &HashSet<u64>,
    mode: DistributeMode,
) {
    if selected.len() < 3 {
        return;
    }
    let mut items: Vec<(u64, egui::Rect)> = doc
        .elements
        .iter()
        .filter_map(|e| selected.contains(&e.id).then_some((e.id, e.bounds())))
        .collect();
    if items.len() < 3 {
        return;
    }
    match mode {
        DistributeMode::Horizontal => {
            items.sort_by(|a, b| a.1.center().x.total_cmp(&b.1.center().x));
            let first = items.first().unwrap().1.center().x;
            let last = items.last().unwrap().1.center().x;
            let step = (last - first) / ((items.len() - 1) as f32);
            for (i, (id, b)) in items.into_iter().enumerate() {
                let target = first + step * (i as f32);
                let delta = egui::vec2(target - b.center().x, 0.0);
                if let Some(e) = doc.elements.iter_mut().find(|e| e.id == id) {
                    translate_element(e, delta);
                }
            }
        }
        DistributeMode::Vertical => {
            items.sort_by(|a, b| a.1.center().y.total_cmp(&b.1.center().y));
            let first = items.first().unwrap().1.center().y;
            let last = items.last().unwrap().1.center().y;
            let step = (last - first) / ((items.len() - 1) as f32);
            for (i, (id, b)) in items.into_iter().enumerate() {
                let target = first + step * (i as f32);
                let delta = egui::vec2(0.0, target - b.center().y);
                if let Some(e) = doc.elements.iter_mut().find(|e| e.id == id) {
                    translate_element(e, delta);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AbutMode {
    Horizontal,
    Vertical,
}

pub(super) fn abut_selected(doc: &mut model::Document, selected: &HashSet<u64>, mode: AbutMode) {
    if selected.len() < 2 {
        return;
    }
    let mut items: Vec<(u64, egui::Rect)> = doc
        .elements
        .iter()
        .filter_map(|e| selected.contains(&e.id).then_some((e.id, e.bounds())))
        .collect();
    if items.len() < 2 {
        return;
    }
    match mode {
        AbutMode::Horizontal => {
            items.sort_by(|a, b| a.1.min.x.total_cmp(&b.1.min.x));
            let mut current_x = items[0].1.min.x;
            for (id, b) in items {
                let delta = egui::vec2(current_x - b.min.x, 0.0);
                if let Some(e) = doc.elements.iter_mut().find(|e| e.id == id) {
                    translate_element(e, delta);
                }
                current_x += b.width();
            }
        }
        AbutMode::Vertical => {
            items.sort_by(|a, b| a.1.min.y.total_cmp(&b.1.min.y));
            let mut current_y = items[0].1.min.y;
            for (id, b) in items {
                let delta = egui::vec2(0.0, current_y - b.min.y);
                if let Some(e) = doc.elements.iter_mut().find(|e| e.id == id) {
                    translate_element(e, delta);
                }
                current_y += b.height();
            }
        }
    }
}

pub(super) fn element_label(element: &model::Element) -> String {
    let group = element
        .group_id
        .map(|g| format!(" [G{}]", g))
        .unwrap_or_default();
    match &element.kind {
        model::ElementKind::Rect { .. } => format!("Rect {}{}", element.id, group),
        model::ElementKind::Ellipse { .. } => format!("Ellipse {}{}", element.id, group),
        model::ElementKind::Triangle { .. } => format!("Triangle {}{}", element.id, group),
        model::ElementKind::Parallelogram { .. } => {
            format!("Parallelogram {}{}", element.id, group)
        }
        model::ElementKind::Trapezoid { .. } => format!("Trapezoid {}{}", element.id, group),
        model::ElementKind::Line {
            arrow, arrow_style, ..
        } => {
            let is_arrow = *arrow
                || matches!(
                    arrow_style,
                    model::ArrowStyle::End | model::ArrowStyle::Start | model::ArrowStyle::Both
                );
            if is_arrow {
                format!("Arrow {}{}", element.id, group)
            } else {
                format!("Line {}{}", element.id, group)
            }
        }
        model::ElementKind::Polyline { .. } => format!("Polyline {}{}", element.id, group),
        model::ElementKind::Pen { .. } => format!("Pen {}{}", element.id, group),
        model::ElementKind::Text { .. } => format!("Text {}{}", element.id, group),
    }
}

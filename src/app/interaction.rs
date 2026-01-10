use crate::model;
use eframe::egui;

use super::geometry::{resolved_line_endpoints_world, rotate_vec2};
use super::{ActiveTransform, DiagramApp, LineEndpoint, ResizeHandle, View};

impl DiagramApp {
    pub(super) fn interact_selection_handles(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        origin: egui::Pos2,
        view: &View,
        pointer_world: Option<egui::Pos2>,
        threshold_world: f32,
        ctx: &egui::Context,
    ) {
        if self.selected.len() != 1 {
            self.active_transform = None;
            return;
        }
        let Some(selected_id) = self.selected.iter().copied().next() else {
            self.active_transform = None;
            return;
        };
        let Some(idx) = self.element_index_by_id(selected_id) else {
            self.active_transform = None;
            return;
        };

        let shift = ctx.input(|i| i.modifiers.shift);
        let min_size_world = 8.0;
        let handle_size_screen = 10.0;
        let rotate_offset_screen = 24.0;

        let mut stop_transform = false;
        if let Some(transform) = &mut self.active_transform {
            match transform {
                ActiveTransform::Resize {
                    element_id,
                    handle,
                    start_rect,
                    start_rotation,
                    start_pointer_world,
                } => {
                    if *element_id != selected_id {
                        stop_transform = true;
                    } else if let Some(p) = pointer_world {
                        if let Some(element) = self.doc.elements.get_mut(idx) {
                            let delta_world = p - *start_pointer_world;
                            let delta_local = rotate_vec2(delta_world, -*start_rotation);
                            let mut min = start_rect.min.to_pos2();
                            let mut max = start_rect.max.to_pos2();
                            match handle {
                                ResizeHandle::NW => {
                                    min.x += delta_local.x;
                                    min.y += delta_local.y;
                                }
                                ResizeHandle::N => {
                                    min.y += delta_local.y;
                                }
                                ResizeHandle::NE => {
                                    max.x += delta_local.x;
                                    min.y += delta_local.y;
                                }
                                ResizeHandle::W => {
                                    min.x += delta_local.x;
                                }
                                ResizeHandle::E => {
                                    max.x += delta_local.x;
                                }
                                ResizeHandle::SW => {
                                    min.x += delta_local.x;
                                    max.y += delta_local.y;
                                }
                                ResizeHandle::S => {
                                    max.y += delta_local.y;
                                }
                                ResizeHandle::SE => {
                                    max.x += delta_local.x;
                                    max.y += delta_local.y;
                                }
                            }
                            let mut w = max.x - min.x;
                            let mut h = max.y - min.y;
                            if shift
                                && matches!(
                                    handle,
                                    ResizeHandle::NW
                                        | ResizeHandle::NE
                                        | ResizeHandle::SW
                                        | ResizeHandle::SE
                                )
                            {
                                let w0 = (start_rect.max.x - start_rect.min.x).abs();
                                let h0 = (start_rect.max.y - start_rect.min.y).abs();
                                if w0 > f32::EPSILON && h0 > f32::EPSILON {
                                    let ratio = w0 / h0;
                                    if (w / h).is_finite() {
                                        if (w / h) > ratio {
                                            w = h * ratio;
                                        } else {
                                            h = w / ratio;
                                        }
                                        match handle {
                                            ResizeHandle::NW => {
                                                min.x = max.x - w;
                                                min.y = max.y - h;
                                            }
                                            ResizeHandle::NE => {
                                                max.x = min.x + w;
                                                min.y = max.y - h;
                                            }
                                            ResizeHandle::SW => {
                                                min.x = max.x - w;
                                                max.y = min.y + h;
                                            }
                                            ResizeHandle::SE => {
                                                max.x = min.x + w;
                                                max.y = min.y + h;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            if w < min_size_world {
                                let cx = (min.x + max.x) * 0.5;
                                min.x = cx - min_size_world * 0.5;
                                max.x = cx + min_size_world * 0.5;
                            }
                            if h < min_size_world {
                                let cy = (min.y + max.y) * 0.5;
                                min.y = cy - min_size_world * 0.5;
                                max.y = cy + min_size_world * 0.5;
                            }
                            let rectf = model::RectF::from_min_max(min, max);
                            match &mut element.kind {
                                model::ElementKind::Rect { rect, .. }
                                | model::ElementKind::Ellipse { rect, .. }
                                | model::ElementKind::Triangle { rect, .. }
                                | model::ElementKind::Parallelogram { rect, .. }
                                | model::ElementKind::Trapezoid { rect, .. } => {
                                    *rect = rectf;
                                }
                                _ => stop_transform = true,
                            }
                        }
                    }
                }
                ActiveTransform::Rotate {
                    element_id,
                    start_rotation,
                    start_angle,
                } => {
                    if *element_id != selected_id {
                        stop_transform = true;
                    } else if let Some(p) = pointer_world {
                        let center = self.doc.elements[idx].bounds().center();
                        let angle = (p.y - center.y).atan2(p.x - center.x);
                        let mut rot = *start_rotation + (angle - *start_angle);
                        if shift {
                            let step = std::f32::consts::PI / 12.0;
                            rot = (rot / step).round() * step;
                        }
                        self.doc.elements[idx].rotation = rot;
                    }
                }
                ActiveTransform::LineEndpoint {
                    element_id,
                    endpoint,
                    start_a,
                    start_b,
                    start_pointer_world,
                } => {
                    if *element_id != selected_id {
                        stop_transform = true;
                    } else if let Some(p) = pointer_world {
                        let delta = p - *start_pointer_world;
                        let mut a = *start_a;
                        let mut b = *start_b;
                        match endpoint {
                            LineEndpoint::Start => a += delta,
                            LineEndpoint::End => b += delta,
                        }
                        if let Some(element) = self.doc.elements.get_mut(idx) {
                            if let model::ElementKind::Line {
                                a: pa,
                                b: pb,
                                start_binding,
                                end_binding,
                                ..
                            } = &mut element.kind
                            {
                                *pa = model::Point::from_pos2(a);
                                *pb = model::Point::from_pos2(b);
                                match endpoint {
                                    LineEndpoint::Start => *start_binding = None,
                                    LineEndpoint::End => *end_binding = None,
                                }
                            } else {
                                stop_transform = true;
                            }
                        }
                    }
                }
            }
        }
        if stop_transform {
            self.active_transform = None;
        }

        let (rotation, kind) = {
            let e = &self.doc.elements[idx];
            (e.rotation, e.kind.clone())
        };
        let handle_fill = egui::Color32::from_rgb(250, 250, 250);
        let handle_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 160, 255));

        match kind {
            model::ElementKind::Rect { rect, .. }
            | model::ElementKind::Ellipse { rect, .. }
            | model::ElementKind::Triangle { rect, .. }
            | model::ElementKind::Parallelogram { rect, .. }
            | model::ElementKind::Trapezoid { rect, .. } => {
                let rect = rect.to_rect();
                let center = rect.center();
                let size = rect.size();
                let hw = size.x * 0.5;
                let hh = size.y * 0.5;

                let handles = [
                    (ResizeHandle::NW, -1.0, -1.0),
                    (ResizeHandle::N, 0.0, -1.0),
                    (ResizeHandle::NE, 1.0, -1.0),
                    (ResizeHandle::W, -1.0, 0.0),
                    (ResizeHandle::E, 1.0, 0.0),
                    (ResizeHandle::SW, -1.0, 1.0),
                    (ResizeHandle::S, 0.0, 1.0),
                    (ResizeHandle::SE, 1.0, 1.0),
                ];
                for (handle, sx, sy) in handles {
                    let local = egui::vec2(sx * hw, sy * hh);
                    let world = center + rotate_vec2(local, rotation);
                    let screen = view.world_to_screen(origin, world);
                    let r = egui::Rect::from_center_size(
                        screen,
                        egui::vec2(handle_size_screen, handle_size_screen),
                    );
                    let id = ui.id().with(("resize", selected_id, handle as u8));
                    let resp = ui.interact(r, id, egui::Sense::drag());
                    painter.rect_filled(r, 1.0, handle_fill);
                    painter.rect_stroke(r, 1.0, handle_stroke, egui::StrokeKind::Middle);
                    if resp.drag_started() {
                        if let Some(p) = pointer_world {
                            self.push_undo();
                            self.active_transform = Some(ActiveTransform::Resize {
                                element_id: selected_id,
                                handle,
                                start_rect: model::RectF::from_min_max(rect.min, rect.max),
                                start_rotation: rotation,
                                start_pointer_world: p,
                            });
                        }
                    }
                    if resp.drag_stopped() {
                        self.active_transform = None;
                    }
                    if resp.hovered() || resp.dragged() {
                        let icon = match handle {
                            ResizeHandle::N | ResizeHandle::S => egui::CursorIcon::ResizeVertical,
                            ResizeHandle::E | ResizeHandle::W => egui::CursorIcon::ResizeHorizontal,
                            ResizeHandle::NE | ResizeHandle::SW => egui::CursorIcon::ResizeNeSw,
                            ResizeHandle::NW | ResizeHandle::SE => egui::CursorIcon::ResizeNwSe,
                        };
                        ctx.set_cursor_icon(icon);
                    }
                }

                let top_local = egui::vec2(0.0, -hh);
                let top_world = center + rotate_vec2(top_local, rotation);
                let offset_world = rotate_offset_screen / view.zoom;
                let rotate_world =
                    top_world + rotate_vec2(egui::vec2(0.0, -offset_world), rotation);
                let top_screen = view.world_to_screen(origin, top_world);
                let rotate_screen = view.world_to_screen(origin, rotate_world);
                painter.line_segment([top_screen, rotate_screen], handle_stroke);
                let rr = egui::Rect::from_center_size(
                    rotate_screen,
                    egui::vec2(handle_size_screen, handle_size_screen),
                );
                let rid = ui.id().with(("rotate", selected_id));
                let rresp = ui.interact(rr, rid, egui::Sense::drag());
                painter.add(egui::Shape::circle_filled(
                    rotate_screen,
                    handle_size_screen * 0.5,
                    handle_fill,
                ));
                painter.add(egui::Shape::circle_stroke(
                    rotate_screen,
                    handle_size_screen * 0.5,
                    handle_stroke,
                ));
                if rresp.drag_started() {
                    if let Some(p) = pointer_world {
                        let angle = (p.y - center.y).atan2(p.x - center.x);
                        self.push_undo();
                        self.active_transform = Some(ActiveTransform::Rotate {
                            element_id: selected_id,
                            start_rotation: rotation,
                            start_angle: angle,
                        });
                    }
                }
                if rresp.drag_stopped() {
                    self.active_transform = None;
                }
                if rresp.hovered() || rresp.dragged() {
                    ctx.set_cursor_icon(egui::CursorIcon::Grab);
                }
            }
            model::ElementKind::Line {
                a,
                b,
                start_binding,
                end_binding,
                ..
            } => {
                let (a, b) =
                    resolved_line_endpoints_world(&self.doc, a, b, &start_binding, &end_binding);
                let pts = [(LineEndpoint::Start, a), (LineEndpoint::End, b)];
                for (endpoint, p) in pts {
                    let screen = view.world_to_screen(origin, p);
                    let r = egui::Rect::from_center_size(
                        screen,
                        egui::vec2(handle_size_screen, handle_size_screen),
                    );
                    let id = ui.id().with(("endpoint", selected_id, endpoint as u8));
                    let resp = ui.interact(r, id, egui::Sense::drag());
                    painter.rect_filled(r, 1.0, handle_fill);
                    painter.rect_stroke(r, 1.0, handle_stroke, egui::StrokeKind::Middle);
                    if resp.drag_started() {
                        if let Some(pw) = pointer_world {
                            self.push_undo();
                            self.active_transform = Some(ActiveTransform::LineEndpoint {
                                element_id: selected_id,
                                endpoint,
                                start_a: a,
                                start_b: b,
                                start_pointer_world: pw,
                            });
                        }
                    }
                    if resp.drag_stopped() {
                        if let Some(pw) = pointer_world {
                            self.try_bind_line_endpoint(selected_id, endpoint, pw, threshold_world);
                        }
                        self.active_transform = None;
                    }
                    if resp.hovered() || resp.dragged() {
                        ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                }
                if let Some(p) = pointer_world {
                    if self.topmost_hit(p, threshold_world).is_some() {
                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                }
            }
            _ => {}
        }
    }
}

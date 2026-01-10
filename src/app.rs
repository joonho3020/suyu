use crate::model;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tool {
    Select,
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Pen,
    Text,
    Pan,
}

#[derive(Clone, Debug)]
enum InProgress {
    DragShape {
        start: egui::Pos2,
        current: egui::Pos2,
    },
    DragLine {
        start: egui::Pos2,
        current: egui::Pos2,
        arrow: bool,
    },
    Pen {
        points: Vec<egui::Pos2>,
    },
    SelectBox {
        start: egui::Pos2,
        current: egui::Pos2,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResizeHandle {
    NW,
    N,
    NE,
    W,
    E,
    SW,
    S,
    SE,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineEndpoint {
    Start,
    End,
}

#[derive(Clone, Debug)]
enum ActiveTransform {
    Resize {
        element_id: u64,
        handle: ResizeHandle,
        start_rect: model::RectF,
        start_rotation: f32,
        start_pointer_world: egui::Pos2,
    },
    Rotate {
        element_id: u64,
        start_rotation: f32,
        start_angle: f32,
    },
    LineEndpoint {
        element_id: u64,
        endpoint: LineEndpoint,
        start_a: egui::Pos2,
        start_b: egui::Pos2,
        start_pointer_world: egui::Pos2,
    },
}

#[derive(Clone, Copy, Debug)]
struct View {
    pan_screen: egui::Vec2,
    zoom: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            pan_screen: egui::Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl View {
    fn world_to_screen(&self, origin: egui::Pos2, world: egui::Pos2) -> egui::Pos2 {
        origin + self.pan_screen + world.to_vec2() * self.zoom
    }

    fn screen_to_world(&self, origin: egui::Pos2, screen: egui::Pos2) -> egui::Pos2 {
        ((screen - origin - self.pan_screen) / self.zoom).to_pos2()
    }

    fn zoom_about_screen_point(
        &mut self,
        origin: egui::Pos2,
        screen_point: egui::Pos2,
        zoom_delta: f32,
    ) {
        let before = self.screen_to_world(origin, screen_point);
        self.zoom = (self.zoom * zoom_delta).clamp(0.1, 8.0);
        let after_screen = self.world_to_screen(origin, before);
        self.pan_screen += screen_point - after_screen;
    }
}

#[derive(Clone)]
struct Snapshot {
    doc: model::Document,
    selected: Vec<u64>,
    next_id: u64,
    next_group_id: u64,
    style: model::Style,
}

#[derive(Clone, Serialize, Deserialize)]
struct ClipboardPayload {
    elements: Vec<model::Element>,
}

pub struct DiagramApp {
    doc: model::Document,
    selected: HashSet<u64>,
    tool: Tool,
    tool_before_pan: Option<Tool>,
    view: View,
    next_id: u64,
    next_group_id: u64,
    style: model::Style,
    in_progress: Option<InProgress>,
    context_world_pos: Option<egui::Pos2>,
    context_hit: Option<u64>,
    last_pointer_world: Option<egui::Pos2>,
    history: Vec<Snapshot>,
    future: Vec<Snapshot>,
    clipboard: Option<ClipboardPayload>,
    drag_transform_recorded: bool,
    active_transform: Option<ActiveTransform>,
    file_path: String,
    status: Option<String>,
    editing_text_id: Option<u64>,
    inline_text_editing: bool,
    apply_style_to_selection: bool,
    snap_to_grid: bool,
    grid_size: f32,
}

impl DiagramApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            doc: model::Document::default(),
            selected: HashSet::new(),
            tool: Tool::Select,
            tool_before_pan: None,
            view: View::default(),
            next_id: 1,
            next_group_id: 1,
            style: model::Style::default_for_shapes(),
            in_progress: None,
            context_world_pos: None,
            context_hit: None,
            last_pointer_world: None,
            history: Vec::new(),
            future: Vec::new(),
            clipboard: None,
            drag_transform_recorded: false,
            active_transform: None,
            file_path: "diagram.json".to_string(),
            status: None,
            editing_text_id: None,
            inline_text_editing: false,
            apply_style_to_selection: true,
            snap_to_grid: true,
            grid_size: 64.0,
        }
    }
}

impl DiagramApp {
    fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    #[allow(dead_code)]
    fn snap_position(&self, pos: egui::Pos2) -> egui::Pos2 {
        if !self.snap_to_grid {
            return pos;
        }
        let grid = self.grid_size;
        egui::pos2(
            (pos.x / grid).round() * grid,
            (pos.y / grid).round() * grid,
        )
    }

    #[allow(dead_code)]
    fn should_snap_element(&self, element: &model::Element) -> bool {
        self.snap_to_grid && element.snap_enabled
    }

    fn snap_selected_to_grid(&mut self) {
        if !self.snap_to_grid {
            return;
        }
        let grid = self.grid_size;
        for element in &mut self.doc.elements {
            if self.selected.contains(&element.id) && element.snap_enabled {
                snap_element_to_grid(element, grid);
            }
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            doc: self.doc.clone(),
            selected: self.selected.iter().copied().collect(),
            next_id: self.next_id,
            next_group_id: self.next_group_id,
            style: self.style,
        }
    }

    fn sync_bound_line_endpoints(&mut self) {
        let before = self.doc.clone();
        for element in &mut self.doc.elements {
            if let model::ElementKind::Line {
                a,
                b,
                start_binding,
                end_binding,
                ..
            } = &mut element.kind
            {
                if start_binding.is_none() && end_binding.is_none() {
                    continue;
                }
                let (ra, rb) =
                    resolved_line_endpoints_world(&before, *a, *b, start_binding, end_binding);
                if start_binding.is_some() {
                    *a = model::Point::from_pos2(ra);
                }
                if end_binding.is_some() {
                    *b = model::Point::from_pos2(rb);
                }
            }
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.doc = snapshot.doc;
        self.selected = snapshot.selected.into_iter().collect();
        self.next_id = snapshot.next_id;
        self.next_group_id = snapshot.next_group_id;
        self.style = snapshot.style;
        self.in_progress = None;
        self.editing_text_id = None;
        self.status = None;
    }

    fn push_undo(&mut self) {
        const LIMIT: usize = 200;
        self.history.push(self.snapshot());
        if self.history.len() > LIMIT {
            let overflow = self.history.len() - LIMIT;
            self.history.drain(0..overflow);
        }
        self.future.clear();
    }

    fn undo(&mut self) {
        let Some(prev) = self.history.pop() else {
            return;
        };
        let current = self.snapshot();
        self.future.push(current);
        self.restore(prev);
    }

    fn redo(&mut self) {
        let Some(next) = self.future.pop() else {
            return;
        };
        let current = self.snapshot();
        self.history.push(current);
        self.restore(next);
    }

    fn element_index_by_id(&self, id: u64) -> Option<usize> {
        self.doc.elements.iter().position(|e| e.id == id)
    }

    fn group_of(&self, id: u64) -> Option<u64> {
        self.doc
            .elements
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.group_id)
    }

    fn group_members(&self, group_id: u64) -> Vec<u64> {
        self.doc
            .elements
            .iter()
            .filter_map(|e| (e.group_id == Some(group_id)).then_some(e.id))
            .collect()
    }

    fn topmost_hit(&self, world_pos: egui::Pos2, threshold_world: f32) -> Option<u64> {
        for element in self.doc.elements.iter().rev() {
            if hit_test_element(&self.doc, element, world_pos, threshold_world) {
                return Some(element.id);
            }
        }
        None
    }

    fn clear_selection(&mut self) {
        self.selected.clear();
        self.editing_text_id = None;
    }

    fn set_selection_single(&mut self, id: u64) {
        self.selected.clear();
        if let Some(group_id) = self.group_of(id) {
            for id in self.group_members(group_id) {
                self.selected.insert(id);
            }
        } else {
            self.selected.insert(id);
        }
        self.editing_text_id = None;
    }

    fn toggle_selection(&mut self, id: u64) {
        if let Some(group_id) = self.group_of(id) {
            let members = self.group_members(group_id);
            let all_selected = members.iter().all(|id| self.selected.contains(id));
            if all_selected {
                for id in members {
                    self.selected.remove(&id);
                }
            } else {
                for id in members {
                    self.selected.insert(id);
                }
            }
        } else {
            if self.selected.contains(&id) {
                self.selected.remove(&id);
            } else {
                self.selected.insert(id);
            }
        }
        self.editing_text_id = None;
    }

    fn delete_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.push_undo();
        let before = self.doc.clone();
        let selected = &self.selected;
        self.doc.elements.retain(|e| !selected.contains(&e.id));
        for element in &mut self.doc.elements {
            if let model::ElementKind::Line {
                a,
                b,
                start_binding,
                end_binding,
                ..
            } = &mut element.kind
            {
                let a0 = *a;
                let b0 = *b;
                let sb = *start_binding;
                let eb = *end_binding;
                if sb
                    .as_ref()
                    .is_some_and(|bind| selected.contains(&bind.element_id))
                {
                    let (ra, _) = resolved_line_endpoints_world(&before, a0, b0, &sb, &eb);
                    *a = model::Point::from_pos2(ra);
                    *start_binding = None;
                }
                if eb
                    .as_ref()
                    .is_some_and(|bind| selected.contains(&bind.element_id))
                {
                    let (_, rb) = resolved_line_endpoints_world(&before, a0, b0, &sb, &eb);
                    *b = model::Point::from_pos2(rb);
                    *end_binding = None;
                }
            }
        }
        self.clear_selection();
    }

    fn bring_selected_to_front(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.push_undo();
        let mut kept = Vec::with_capacity(self.doc.elements.len());
        let mut moved = Vec::new();
        for e in self.doc.elements.drain(..) {
            if self.selected.contains(&e.id) {
                moved.push(e);
            } else {
                kept.push(e);
            }
        }
        kept.extend(moved);
        self.doc.elements = kept;
    }

    fn translate_selected(&mut self, delta_world: egui::Vec2) {
        if self.selected.is_empty() {
            return;
        }
        let before = self.doc.clone();
        let selected = self.selected.clone();
        for element in &mut self.doc.elements {
            if !selected.contains(&element.id) {
                continue;
            }
            if let model::ElementKind::Line {
                a,
                b,
                start_binding,
                end_binding,
                ..
            } = &mut element.kind
            {
                let sb = *start_binding;
                let eb = *end_binding;
                let (a0, b0) = resolved_line_endpoints_world(&before, *a, *b, &sb, &eb);
                let desired_a = a0 + delta_world;
                let desired_b = b0 + delta_world;

                let mut new_sb = sb;
                let mut new_eb = eb;

                if let Some(bind) = sb {
                    if !selected.contains(&bind.element_id) {
                        new_sb = compute_binding_for_target(&before, bind.element_id, desired_a);
                        if new_sb.is_none() {
                            *a = model::Point::from_pos2(desired_a);
                        }
                    }
                }
                if let Some(bind) = eb {
                    if !selected.contains(&bind.element_id) {
                        new_eb = compute_binding_for_target(&before, bind.element_id, desired_b);
                        if new_eb.is_none() {
                            *b = model::Point::from_pos2(desired_b);
                        }
                    }
                }

                if sb.is_none() {
                    *a = model::Point::from_pos2(desired_a);
                }
                if eb.is_none() {
                    *b = model::Point::from_pos2(desired_b);
                }

                *start_binding = new_sb;
                *end_binding = new_eb;
            } else {
                translate_element(element, delta_world);
            }
        }
    }

    fn try_bind_line_endpoint(
        &mut self,
        line_id: u64,
        endpoint: LineEndpoint,
        world_pos: egui::Pos2,
        threshold_world: f32,
    ) {
        let target = topmost_bind_target_id(&self.doc, world_pos, threshold_world);
        let binding = target.and_then(|tid| compute_binding_for_target(&self.doc, tid, world_pos));
        let resolved = binding
            .as_ref()
            .and_then(|b| resolve_binding_point(&self.doc, b));
        let Some(idx) = self.element_index_by_id(line_id) else {
            return;
        };
        let element = &mut self.doc.elements[idx];
        let model::ElementKind::Line {
            a,
            b,
            start_binding,
            end_binding,
            ..
        } = &mut element.kind
        else {
            return;
        };
        match endpoint {
            LineEndpoint::Start => {
                *start_binding = binding;
                let p = resolved.unwrap_or(world_pos);
                *a = model::Point::from_pos2(p);
            }
            LineEndpoint::End => {
                *end_binding = binding;
                let p = resolved.unwrap_or(world_pos);
                *b = model::Point::from_pos2(p);
            }
        }
    }

    fn send_selected_to_back(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.push_undo();
        let mut back = Vec::new();
        let mut kept = Vec::with_capacity(self.doc.elements.len());
        for e in self.doc.elements.drain(..) {
            if self.selected.contains(&e.id) {
                back.push(e);
            } else {
                kept.push(e);
            }
        }
        back.extend(kept);
        self.doc.elements = back;
    }

    fn move_selected_layer_by(&mut self, delta: i32) {
        if delta == 0 || self.selected.is_empty() {
            return;
        }
        self.push_undo();
        if delta > 0 {
            for i in (0..self.doc.elements.len()).rev() {
                if i + 1 >= self.doc.elements.len() {
                    continue;
                }
                let should_move = self.selected.contains(&self.doc.elements[i].id)
                    && !self.selected.contains(&self.doc.elements[i + 1].id);
                if should_move {
                    self.doc.elements.swap(i, i + 1);
                }
            }
        } else {
            for i in 0..self.doc.elements.len().saturating_sub(1) {
                let should_move = self.selected.contains(&self.doc.elements[i + 1].id)
                    && !self.selected.contains(&self.doc.elements[i].id);
                if should_move {
                    self.doc.elements.swap(i, i + 1);
                }
            }
        }
    }

    fn group_selected(&mut self) {
        if self.selected.len() < 2 {
            return;
        }
        self.push_undo();
        let group_id = self.next_group_id;
        self.next_group_id += 1;
        for element in &mut self.doc.elements {
            if self.selected.contains(&element.id) {
                element.group_id = Some(group_id);
            }
        }
    }

    fn ungroup_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        let mut groups = HashSet::new();
        for id in &self.selected {
            if let Some(g) = self.group_of(*id) {
                groups.insert(g);
            }
        }
        if groups.is_empty() {
            return;
        }
        self.push_undo();
        for element in &mut self.doc.elements {
            if let Some(g) = element.group_id {
                if groups.contains(&g) {
                    element.group_id = None;
                }
            }
        }
    }

    fn duplicate_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.push_undo();
        let mut new_ids = Vec::new();
        let mut clones = Vec::new();
        let mut next_id = self.next_id;
        for element in &self.doc.elements {
            if self.selected.contains(&element.id) {
                let mut cloned = element.clone();
                cloned.id = next_id;
                next_id += 1;
                cloned.group_id = None;
                translate_element(&mut cloned, egui::vec2(12.0, 12.0));
                new_ids.push(cloned.id);
                clones.push(cloned);
            }
        }
        self.next_id = next_id;
        self.doc.elements.extend(clones);
        self.selected = new_ids.into_iter().collect();
        self.editing_text_id = None;
    }

    fn copy_selected(&mut self, _ctx: &egui::Context) {
        if self.selected.is_empty() {
            self.status = Some("Nothing selected to copy".to_string());
            return;
        }
        let elements: Vec<model::Element> = self
            .doc
            .elements
            .iter()
            .filter(|e| self.selected.contains(&e.id))
            .cloned()
            .collect();
        let payload = ClipboardPayload { elements };
        self.status = Some(format!("Copied {} element(s)", payload.elements.len()));
        self.clipboard = Some(payload);
    }

    fn cut_selected(&mut self, ctx: &egui::Context) {
        self.copy_selected(ctx);
        self.delete_selected();
    }

    fn paste_from_payload(&mut self, payload: &ClipboardPayload) {
        if payload.elements.is_empty() {
            return;
        }
        self.push_undo();
        let mut group_map: HashMap<u64, u64> = HashMap::new();
        for e in &payload.elements {
            if let Some(g) = e.group_id {
                group_map.entry(g).or_insert_with(|| {
                    let ng = self.next_group_id;
                    self.next_group_id += 1;
                    ng
                });
            }
        }

        let mut base: Option<egui::Rect> = None;
        for e in &payload.elements {
            base = Some(base.map(|r| r.union(e.bounds())).unwrap_or(e.bounds()));
        }
        let base_min = base.unwrap_or(egui::Rect::NOTHING).min;
        let target = self
            .context_world_pos
            .or(self.last_pointer_world)
            .unwrap_or(base_min + egui::vec2(24.0, 24.0));
        let delta = target - base_min;

        let mut new_ids = Vec::new();
        let mut new_elements = Vec::new();
        let mut id_map: HashMap<u64, u64> = HashMap::new();
        for e in &payload.elements {
            id_map.insert(e.id, self.allocate_id());
        }
        for e in &payload.elements {
            let mut e = e.clone();
            e.id = *id_map.get(&e.id).unwrap();
            e.group_id = e.group_id.and_then(|g| group_map.get(&g).copied());
            if let model::ElementKind::Line {
                start_binding,
                end_binding,
                ..
            } = &mut e.kind
            {
                if let Some(b) = start_binding.as_mut() {
                    if let Some(mapped) = id_map.get(&b.element_id).copied() {
                        b.element_id = mapped;
                    } else {
                        *start_binding = None;
                    }
                }
                if let Some(b) = end_binding.as_mut() {
                    if let Some(mapped) = id_map.get(&b.element_id).copied() {
                        b.element_id = mapped;
                    } else {
                        *end_binding = None;
                    }
                }
            }
            translate_element(&mut e, delta);
            new_ids.push(e.id);
            new_elements.push(e);
        }
        self.doc.elements.extend(new_elements);
        self.selected = new_ids.into_iter().collect();
        self.editing_text_id = None;
    }

    fn paste(&mut self) {
        let Some(payload) = self.clipboard.clone() else {
            self.status = Some("Nothing in clipboard to paste".to_string());
            return;
        };
        self.status = Some(format!("Pasted {} element(s)", payload.elements.len()));
        self.paste_from_payload(&payload);
    }

    fn save_to_path(&mut self) {
        match serde_json::to_string_pretty(&self.doc) {
            Ok(json) => match std::fs::write(&self.file_path, json) {
                Ok(()) => self.status = Some(format!("Saved {}", self.file_path)),
                Err(e) => self.status = Some(format!("Save failed: {e}")),
            },
            Err(e) => self.status = Some(format!("Serialize failed: {e}")),
        }
    }

    fn load_from_path(&mut self) {
        match std::fs::read_to_string(&self.file_path) {
            Ok(s) => match serde_json::from_str::<model::Document>(&s) {
                Ok(doc) => {
                    self.doc = doc;
                    self.next_id = self.doc.elements.iter().map(|e| e.id).max().unwrap_or(0) + 1;
                    self.next_group_id = self
                        .doc
                        .elements
                        .iter()
                        .filter_map(|e| e.group_id)
                        .max()
                        .unwrap_or(0)
                        + 1;
                    self.clear_selection();
                    self.history.clear();
                    self.future.clear();
                    self.status = Some(format!("Loaded {}", self.file_path));
                }
                Err(e) => self.status = Some(format!("Parse failed: {e}")),
            },
            Err(e) => self.status = Some(format!("Load failed: {e}")),
        }
    }

    fn interact_selection_handles(
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
                                | model::ElementKind::Ellipse { rect, .. } => {
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
            model::ElementKind::Rect { rect, .. } | model::ElementKind::Ellipse { rect, .. } => {
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
                if i.consume_key(egui::Modifiers::NONE, egui::Key::L) {
                    self.tool = Tool::Line;
                }
                if i.consume_key(egui::Modifiers::NONE, egui::Key::A) {
                    self.tool = Tool::Arrow;
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
                tool_button(ui, "Line (L)", Tool::Line, &mut self.tool);
                tool_button(ui, "Arrow (A)", Tool::Arrow, &mut self.tool);
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
                            | model::ElementKind::Ellipse { rect, label } => {
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
            let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
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
                self.context_world_pos = pointer_world;
                self.context_hit = pointer_world.and_then(|p| self.topmost_hit(p, threshold_world));
                if let Some(hit) = self.context_hit {
                    if !self.selected.contains(&hit) {
                        self.set_selection_single(hit);
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
                            let multi_select = ctx.input(|i| i.modifiers.shift || i.modifiers.ctrl || i.modifiers.command);
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
                        Tool::Rectangle | Tool::Ellipse => {
                            self.in_progress = Some(InProgress::DragShape {
                                start: world_pos,
                                current: world_pos,
                            });
                        }
                        Tool::Line => {
                            self.in_progress = Some(InProgress::DragLine {
                                start: world_pos,
                                current: world_pos,
                                arrow: false,
                            });
                        }
                        Tool::Arrow => {
                            self.in_progress = Some(InProgress::DragLine {
                                start: world_pos,
                                current: world_pos,
                                arrow: true,
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
                            arrow,
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
                                let element = model::Element {
                                    id,
                                    group_id: None,
                                    rotation: 0.0,
                                    snap_enabled: true,
                                    kind: model::ElementKind::Line {
                                        a: model::Point::from_pos2(a),
                                        b: model::Point::from_pos2(b),
                                        arrow,
                                        start_binding,
                                        end_binding,
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
                                (
                                    egui::Rect::from_min_size(world_pos, egui::vec2(w, h)),
                                    idx,
                                )
                            }
                            model::ElementKind::Rect { rect, .. }
                            | model::ElementKind::Ellipse { rect, .. } => {
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
                            let screen_min =
                                self.view.world_to_screen(origin, edit_rect_world.min);
                            let screen_max =
                                self.view.world_to_screen(origin, edit_rect_world.max);
                            let edit_rect_screen =
                                egui::Rect::from_min_max(screen_min, screen_max);

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
                                        let text_to_edit: &mut String =
                                            match &mut self.doc.elements[text_ptr].kind {
                                                model::ElementKind::Text { text, .. } => text,
                                                model::ElementKind::Rect { label, .. } => label,
                                                model::ElementKind::Ellipse { label, .. } => label,
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

                            let escape_pressed =
                                ctx.input(|i| i.key_pressed(egui::Key::Escape));
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

fn tool_button(ui: &mut egui::Ui, label: &str, tool: Tool, selected: &mut Tool) {
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

fn style_editor(ui: &mut egui::Ui, style: &mut model::Style) -> bool {
    let mut changed = false;
    ui.label("Stroke");
    changed |= color_row(ui, &mut style.stroke.color);
    changed |= ui
        .add(egui::Slider::new(&mut style.stroke.width, 0.5..=12.0).text("Width"))
        .changed();
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

fn draw_background(painter: &egui::Painter, rect: egui::Rect, view: &View) {
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

fn draw_elements(
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
        model::ElementKind::Line {
            a,
            b,
            arrow,
            start_binding,
            end_binding,
        } => {
            let (a, b) = resolved_line_endpoints_world(doc, *a, *b, start_binding, end_binding);
            let a = view.world_to_screen(origin, a);
            let b = view.world_to_screen(origin, b);
            painter.line_segment([a, b], stroke);
            if *arrow {
                draw_arrowhead(painter, a, b, stroke);
            }
            if is_selected {
                let r = egui::Rect::from_two_pos(a, b).expand(6.0);
                draw_selection_bounds(painter, r);
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

fn draw_in_progress(
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
            let r = egui::Rect::from_two_pos(
                view.world_to_screen(origin, *start),
                view.world_to_screen(origin, *current),
            );
            match tool {
                Tool::Rectangle => painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle),
                Tool::Ellipse => painter.add(egui::Shape::ellipse_stroke(
                    r.center(),
                    r.size() * 0.5,
                    stroke,
                )),
                _ => painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle),
            };
        }
        InProgress::DragLine {
            start,
            current,
            arrow,
        } => {
            let a = view.world_to_screen(origin, *start);
            let b = view.world_to_screen(origin, *current);
            painter.line_segment([a, b], stroke);
            if *arrow {
                draw_arrowhead(painter, a, b, stroke);
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

fn rotate_vec2(v: egui::Vec2, angle: f32) -> egui::Vec2 {
    let sin = angle.sin();
    let cos = angle.cos();
    egui::vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

fn rotated_rect_points_screen(
    origin: egui::Pos2,
    view: &View,
    rect: egui::Rect,
    rotation: f32,
) -> Vec<egui::Pos2> {
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
            let w = center + rotate_vec2(v, rotation);
            view.world_to_screen(origin, w)
        })
        .collect()
}

fn rotated_ellipse_points_screen(
    origin: egui::Pos2,
    view: &View,
    rect: egui::Rect,
    rotation: f32,
) -> Vec<egui::Pos2> {
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
            let w = center + rotate_vec2(local, rotation);
            view.world_to_screen(origin, w)
        })
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

fn resolved_line_endpoints_world(
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

fn topmost_bind_target_id(
    doc: &model::Document,
    world_pos: egui::Pos2,
    threshold_world: f32,
) -> Option<u64> {
    for element in doc.elements.iter().rev() {
        match &element.kind {
            model::ElementKind::Rect { .. } | model::ElementKind::Ellipse { .. } => {
                if hit_test_element(doc, element, world_pos, threshold_world) {
                    return Some(element.id);
                }
            }
            _ => {}
        }
    }
    None
}

fn compute_binding_for_target(
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
        model::ElementKind::Rect { rect, .. } => rect.to_rect(),
        model::ElementKind::Ellipse { rect, .. } => rect.to_rect(),
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
        model::ElementKind::Rect { .. } => {
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

fn resolve_binding_point(doc: &model::Document, binding: &model::Binding) -> Option<egui::Pos2> {
    let element = doc.elements.iter().find(|e| e.id == binding.element_id)?;
    let (rect, rotation) = match &element.kind {
        model::ElementKind::Rect { rect, .. } => Some((rect.to_rect(), element.rotation)),
        model::ElementKind::Ellipse { rect, .. } => Some((rect.to_rect(), element.rotation)),
        _ => None,
    }?;
    let center = rect.center();
    let size = rect.size();
    let local = egui::vec2(binding.norm.x * size.x, binding.norm.y * size.y);
    Some(center + rotate_vec2(local, rotation))
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

fn translate_element(element: &mut model::Element, delta_world: egui::Vec2) {
    match &mut element.kind {
        model::ElementKind::Rect { rect, .. } | model::ElementKind::Ellipse { rect, .. } => {
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
        model::ElementKind::Pen { points } => {
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

fn snap_element_to_grid(element: &mut model::Element, grid_size: f32) {
    match &mut element.kind {
        model::ElementKind::Rect { rect, .. } | model::ElementKind::Ellipse { rect, .. } => {
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

fn hit_test_element(
    doc: &model::Document,
    element: &model::Element,
    world_pos: egui::Pos2,
    threshold_world: f32,
) -> bool {
    match &element.kind {
        model::ElementKind::Rect { rect, .. } => {
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
        model::ElementKind::Pen { points } => {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AlignMode {
    Left,
    HCenter,
    Right,
    Top,
    VCenter,
    Bottom,
}

fn align_selected(doc: &mut model::Document, selected: &HashSet<u64>, mode: AlignMode) {
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
enum DistributeMode {
    Horizontal,
    Vertical,
}

fn distribute_selected(doc: &mut model::Document, selected: &HashSet<u64>, mode: DistributeMode) {
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
enum AbutMode {
    Horizontal,
    Vertical,
}

fn abut_selected(doc: &mut model::Document, selected: &HashSet<u64>, mode: AbutMode) {
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

fn element_label(element: &model::Element) -> String {
    let group = element
        .group_id
        .map(|g| format!(" [G{}]", g))
        .unwrap_or_default();
    match &element.kind {
        model::ElementKind::Rect { .. } => format!("Rect {}{}", element.id, group),
        model::ElementKind::Ellipse { .. } => format!("Ellipse {}{}", element.id, group),
        model::ElementKind::Line { arrow, .. } => {
            if *arrow {
                format!("Arrow {}{}", element.id, group)
            } else {
                format!("Line {}{}", element.id, group)
            }
        }
        model::ElementKind::Pen { .. } => format!("Pen {}{}", element.id, group),
        model::ElementKind::Text { .. } => format!("Text {}{}", element.id, group),
    }
}

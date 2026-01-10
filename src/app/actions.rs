use crate::model;
use eframe::egui;
use std::collections::{HashMap, HashSet};

use super::geometry::{
    compute_binding_for_target, hit_test_element, resolve_binding_point,
    resolved_line_endpoints_world, snap_element_to_grid, topmost_bind_target_id, translate_element,
};
use super::{ClipboardPayload, DiagramApp, LineEndpoint, Snapshot};

impl DiagramApp {
    pub(super) fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    #[allow(dead_code)]
    pub(super) fn snap_position(&self, pos: egui::Pos2) -> egui::Pos2 {
        if !self.snap_to_grid {
            return pos;
        }
        let grid = self.grid_size;
        egui::pos2((pos.x / grid).round() * grid, (pos.y / grid).round() * grid)
    }

    #[allow(dead_code)]
    pub(super) fn should_snap_element(&self, element: &model::Element) -> bool {
        self.snap_to_grid && element.snap_enabled
    }

    pub(super) fn snap_selected_to_grid(&mut self) {
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

    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot {
            doc: self.doc.clone(),
            selected: self.selected.iter().copied().collect(),
            next_id: self.next_id,
            next_group_id: self.next_group_id,
            style: self.style,
        }
    }

    pub(super) fn sync_bound_line_endpoints(&mut self) {
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

    pub(super) fn restore(&mut self, snapshot: Snapshot) {
        self.doc = snapshot.doc;
        self.selected = snapshot.selected.into_iter().collect();
        self.next_id = snapshot.next_id;
        self.next_group_id = snapshot.next_group_id;
        self.style = snapshot.style;
        self.in_progress = None;
        self.editing_text_id = None;
        self.status = None;
    }

    pub(super) fn push_undo(&mut self) {
        const LIMIT: usize = 200;
        self.history.push(self.snapshot());
        if self.history.len() > LIMIT {
            let overflow = self.history.len() - LIMIT;
            self.history.drain(0..overflow);
        }
        self.future.clear();
    }

    pub(super) fn undo(&mut self) {
        let Some(prev) = self.history.pop() else {
            return;
        };
        let current = self.snapshot();
        self.future.push(current);
        self.restore(prev);
    }

    pub(super) fn redo(&mut self) {
        let Some(next) = self.future.pop() else {
            return;
        };
        let current = self.snapshot();
        self.history.push(current);
        self.restore(next);
    }

    pub(super) fn element_index_by_id(&self, id: u64) -> Option<usize> {
        self.doc.elements.iter().position(|e| e.id == id)
    }

    pub(super) fn group_of(&self, id: u64) -> Option<u64> {
        self.doc
            .elements
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.group_id)
    }

    pub(super) fn group_members(&self, group_id: u64) -> Vec<u64> {
        self.doc
            .elements
            .iter()
            .filter_map(|e| (e.group_id == Some(group_id)).then_some(e.id))
            .collect()
    }

    pub(super) fn topmost_hit(&self, world_pos: egui::Pos2, threshold_world: f32) -> Option<u64> {
        for element in self.doc.elements.iter().rev() {
            if hit_test_element(&self.doc, element, world_pos, threshold_world) {
                return Some(element.id);
            }
        }
        None
    }

    pub(super) fn clear_selection(&mut self) {
        self.selected.clear();
        self.editing_text_id = None;
    }

    pub(super) fn set_selection_single(&mut self, id: u64) {
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

    pub(super) fn toggle_selection(&mut self, id: u64) {
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

    pub(super) fn delete_selected(&mut self) {
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

    pub(super) fn bring_selected_to_front(&mut self) {
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

    pub(super) fn translate_selected(&mut self, delta_world: egui::Vec2) {
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

    pub(super) fn try_bind_line_endpoint(
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

    pub(super) fn send_selected_to_back(&mut self) {
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

    pub(super) fn move_selected_layer_by(&mut self, delta: i32) {
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

    pub(super) fn group_selected(&mut self) {
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

    pub(super) fn ungroup_selected(&mut self) {
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

    pub(super) fn duplicate_selected(&mut self) {
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

    pub(super) fn copy_selected(&mut self, _ctx: &egui::Context) {
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

    pub(super) fn cut_selected(&mut self, ctx: &egui::Context) {
        self.copy_selected(ctx);
        self.delete_selected();
    }

    pub(super) fn paste_from_payload(&mut self, payload: &ClipboardPayload) {
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

    pub(super) fn paste(&mut self) {
        let Some(payload) = self.clipboard.clone() else {
            self.status = Some("Nothing in clipboard to paste".to_string());
            return;
        };
        self.status = Some(format!("Pasted {} element(s)", payload.elements.len()));
        self.paste_from_payload(&payload);
    }

    pub(super) fn save_to_path(&mut self) {
        match serde_json::to_string_pretty(&self.doc) {
            Ok(json) => match std::fs::write(&self.file_path, json) {
                Ok(()) => self.status = Some(format!("Saved {}", self.file_path)),
                Err(e) => self.status = Some(format!("Save failed: {e}")),
            },
            Err(e) => self.status = Some(format!("Serialize failed: {e}")),
        }
    }

    pub(super) fn load_from_path(&mut self) {
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
}

use crate::model;
use eframe::egui;
use std::collections::{HashMap, HashSet};

use super::geometry::{
    compute_binding_for_target, hit_test_element, resolve_binding_point,
    resolved_line_endpoints_world, snap_element_to_grid, topmost_bind_target_id, translate_element,
};
use super::{settings, svg};
use super::{ClipboardPayload, DiagramApp, LineEndpoint, Snapshot};

impl DiagramApp {
    pub(super) fn get_theme_colors(&self) -> Vec<egui::Color32> {
        if let Some(idx) = self.active_color_theme {
            if let Some(theme) = self.color_themes.get(idx) {
                return theme.colors.values()
                    .filter_map(|hex| {
                        let hex = hex.trim_start_matches('#');
                        if hex.len() >= 6 {
                            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                            Some(egui::Color32::from_rgb(r, g, b))
                        } else {
                            None
                        }
                    })
                    .collect();
            }
        }
        Vec::new()
    }

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
            style: self.style.clone(),
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

    pub(super) fn group_parent(&self, group_id: u64) -> Option<u64> {
        self.doc
            .groups
            .iter()
            .find(|g| g.id == group_id)
            .and_then(|g| g.parent_id)
    }

    pub(super) fn ensure_group_exists(&mut self, group_id: u64) {
        if self.doc.groups.iter().any(|g| g.id == group_id) {
            return;
        }
        self.doc.groups.push(model::Group {
            id: group_id,
            parent_id: None,
        });
    }

    pub(super) fn set_group_parent(&mut self, group_id: u64, parent_id: Option<u64>) {
        self.ensure_group_exists(group_id);
        if let Some(g) = self.doc.groups.iter_mut().find(|g| g.id == group_id) {
            g.parent_id = parent_id;
        }
    }

    pub(super) fn group_root(&self, mut group_id: u64) -> u64 {
        for _ in 0..256 {
            let Some(parent) = self.group_parent(group_id) else {
                return group_id;
            };
            group_id = parent;
        }
        group_id
    }

    pub(super) fn root_group_of_element(&self, element_id: u64) -> Option<u64> {
        self.group_of(element_id).map(|g| self.group_root(g))
    }

    pub(super) fn group_descendants_inclusive(&self, group_id: u64) -> Vec<u64> {
        let mut out = Vec::new();
        let mut stack = vec![group_id];
        while let Some(g) = stack.pop() {
            out.push(g);
            for child in self
                .doc
                .groups
                .iter()
                .filter(|gr| gr.parent_id == Some(g))
                .map(|gr| gr.id)
            {
                stack.push(child);
            }
        }
        out
    }

    pub(super) fn group_members_recursive(&self, group_id: u64) -> Vec<u64> {
        let groups: HashSet<u64> = self.group_descendants_inclusive(group_id).into_iter().collect();
        self.doc
            .elements
            .iter()
            .filter_map(|e| e.group_id.is_some_and(|g| groups.contains(&g)).then_some(e.id))
            .collect()
    }

    pub(super) fn normalize_groups(&mut self) {
        let mut used: HashSet<u64> = self
            .doc
            .elements
            .iter()
            .filter_map(|e| e.group_id)
            .collect();
        for g in &self.doc.groups {
            used.insert(g.id);
            if let Some(pid) = g.parent_id {
                used.insert(pid);
            }
        }
        let mut stack: Vec<u64> = used.iter().copied().collect();
        while let Some(gid) = stack.pop() {
            if let Some(parent) = self.group_parent(gid) {
                if used.insert(parent) {
                    stack.push(parent);
                }
            }
        }
        for gid in used.clone() {
            self.ensure_group_exists(gid);
        }
        self.doc.groups.retain(|g| used.contains(&g.id));
        let ids: HashSet<u64> = self.doc.groups.iter().map(|g| g.id).collect();
        for g in &mut self.doc.groups {
            if let Some(pid) = g.parent_id {
                if !ids.contains(&pid) {
                    g.parent_id = None;
                }
            }
        }
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
        if let Some(group_id) = self.root_group_of_element(id) {
            for id in self.group_members_recursive(group_id) {
                self.selected.insert(id);
            }
        } else {
            self.selected.insert(id);
        }
        self.editing_text_id = None;
    }

    pub(super) fn toggle_selection(&mut self, id: u64) {
        if let Some(group_id) = self.root_group_of_element(id) {
            let members = self.group_members_recursive(group_id);
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
        self.normalize_groups();
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

    fn element_center_world(&self, element_id: u64) -> Option<egui::Pos2> {
        let element = self.doc.elements.iter().find(|e| e.id == element_id)?;
        match &element.kind {
            model::ElementKind::Rect { rect, .. }
            | model::ElementKind::Ellipse { rect, .. }
            | model::ElementKind::Triangle { rect, .. }
            | model::ElementKind::Parallelogram { rect, .. }
            | model::ElementKind::Trapezoid { rect, .. } => Some(rect.to_rect().center()),
            model::ElementKind::Text { pos, .. } => Some(pos.to_pos2()),
            _ => Some(element.bounds().center()),
        }
    }

    pub(super) fn auto_connect_selected(&mut self, arrow_style: model::ArrowStyle) {
        if self.selected.len() != 2 {
            self.status = Some("Select exactly 2 objects to connect".to_string());
            return;
        }
        let mut it = self.selected.iter().copied();
        let a = it.next().unwrap();
        let b = it.next().unwrap();
        self.auto_connect_between(a, b, arrow_style);
    }

    pub(super) fn auto_connect_between(
        &mut self,
        a_id: u64,
        b_id: u64,
        arrow_style: model::ArrowStyle,
    ) {
        let Some(ca) = self.element_center_world(a_id) else {
            return;
        };
        let Some(cb) = self.element_center_world(b_id) else {
            return;
        };
        let mut dir = cb - ca;
        if dir.length() <= f32::EPSILON {
            dir = egui::vec2(1.0, 0.0);
        } else {
            dir /= dir.length();
        }
        let probe_a = ca + dir * 1_000_000.0;
        let probe_b = cb - dir * 1_000_000.0;

        let Some(start_binding) = compute_binding_for_target(&self.doc, a_id, probe_a) else {
            self.status = Some("Cannot bind start endpoint to selected object".to_string());
            return;
        };
        let Some(end_binding) = compute_binding_for_target(&self.doc, b_id, probe_b) else {
            self.status = Some("Cannot bind end endpoint to selected object".to_string());
            return;
        };
        let a_world = resolve_binding_point(&self.doc, &start_binding).unwrap_or(ca);
        let b_world = resolve_binding_point(&self.doc, &end_binding).unwrap_or(cb);

        self.push_undo();
        let id = self.allocate_id();
        let mut style = self.style.clone();
        style.fill = None;
        self.doc.elements.push(model::Element {
            id,
            group_id: None,
            rotation: 0.0,
            snap_enabled: true,
            kind: model::ElementKind::Line {
                a: model::Point::from_pos2(a_world),
                b: model::Point::from_pos2(b_world),
                arrow: false,
                arrow_style,
                start_binding: Some(start_binding),
                end_binding: Some(end_binding),
            },
            style,
        });
        self.set_selection_single(id);
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
        let mut selected_roots: HashSet<u64> = HashSet::new();
        let mut selected_ungrouped: HashSet<u64> = HashSet::new();
        for id in &self.selected {
            if let Some(root) = self.root_group_of_element(*id) {
                selected_roots.insert(root);
            } else {
                selected_ungrouped.insert(*id);
            }
        }
        let item_count = selected_roots.len() + selected_ungrouped.len();
        if item_count < 2 {
            return;
        }
        self.push_undo();
        let group_id = self.next_group_id;
        self.next_group_id += 1;
        self.doc.groups.push(model::Group {
            id: group_id,
            parent_id: None,
        });
        for root in selected_roots {
            self.set_group_parent(root, Some(group_id));
        }
        for element in &mut self.doc.elements {
            if selected_ungrouped.contains(&element.id) {
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
            if let Some(g) = self.root_group_of_element(*id) {
                groups.insert(g);
            }
        }
        if groups.is_empty() {
            return;
        }
        self.push_undo();
        for g in &groups {
            let parent = self.group_parent(*g);
            for element in &mut self.doc.elements {
                if element.group_id == Some(*g) {
                    element.group_id = parent;
                }
            }
            for gr in &mut self.doc.groups {
                if gr.parent_id == Some(*g) {
                    gr.parent_id = parent;
                }
            }
        }
        self.doc.groups.retain(|gr| !groups.contains(&gr.id));
        self.normalize_groups();
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
        let mut group_ids: HashSet<u64> = elements.iter().filter_map(|e| e.group_id).collect();
        let mut stack: Vec<u64> = group_ids.iter().copied().collect();
        while let Some(g) = stack.pop() {
            if let Some(parent) = self.group_parent(g) {
                if group_ids.insert(parent) {
                    stack.push(parent);
                }
            }
        }
        let groups: Vec<model::Group> = self
            .doc
            .groups
            .iter()
            .filter(|g| group_ids.contains(&g.id))
            .copied()
            .collect();
        let payload = ClipboardPayload { elements, groups };
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
        for g in &payload.groups {
            group_map.entry(g.id).or_insert_with(|| {
                let ng = self.next_group_id;
                self.next_group_id += 1;
                ng
            });
        }
        for g in &payload.groups {
            let new_id = *group_map.get(&g.id).unwrap();
            let new_parent = g.parent_id.and_then(|pid| group_map.get(&pid).copied());
            self.doc.groups.push(model::Group {
                id: new_id,
                parent_id: new_parent,
            });
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
        self.normalize_groups();
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

    pub(super) fn save_json_dialog(&mut self) {
        let default_name = format!("{}.json", self.diagram_name);
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("JSON", &["json"])
            .save_file()
        {
            let path_str = path.display().to_string();
            match serde_json::to_string_pretty(&self.doc) {
                Ok(json) => match std::fs::write(&path, json) {
                    Ok(()) => {
                        self.file_path = path_str.clone();
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            self.diagram_name = stem.to_string();
                        }
                        self.status = Some(format!("Saved {}", path_str));
                    }
                    Err(e) => self.status = Some(format!("Save failed: {e}")),
                },
                Err(e) => self.status = Some(format!("Serialize failed: {e}")),
            }
        }
    }

    pub(super) fn save_svg_to_path(&mut self) {
        let svg = svg::document_to_svg(&self.doc);
        match std::fs::write(&self.svg_path, svg) {
            Ok(()) => self.status = Some(format!("Saved {}", self.svg_path)),
            Err(e) => self.status = Some(format!("SVG save failed: {e}")),
        }
    }

    pub(super) fn save_svg_dialog(&mut self) {
        let default_name = format!("{}.svg", self.diagram_name);
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("SVG", &["svg"])
            .save_file()
        {
            let path_str = path.display().to_string();
            let svg = svg::document_to_svg(&self.doc);
            match std::fs::write(&path, svg) {
                Ok(()) => {
                    self.svg_path = path_str.clone();
                    self.status = Some(format!("Saved {}", path_str));
                }
                Err(e) => self.status = Some(format!("SVG save failed: {e}")),
            }
        }
    }

    pub(super) fn open_json_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            let path_str = path.display().to_string();
            match std::fs::read_to_string(&path) {
                Ok(json) => match serde_json::from_str::<model::Document>(&json) {
                    Ok(doc) => {
                        self.push_undo();
                        self.doc = doc;
                        self.file_path = path_str.clone();
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            self.diagram_name = stem.to_string();
                        }
                        self.selected.clear();
                        self.next_id = self.doc.elements.iter().map(|e| e.id).max().unwrap_or(0) + 1;
                        self.status = Some(format!("Loaded {}", path_str));
                    }
                    Err(e) => self.status = Some(format!("Parse failed: {e}")),
                },
                Err(e) => self.status = Some(format!("Read failed: {e}")),
            }
        }
    }

    pub(super) fn settings_snapshot(&self) -> settings::AppSettings {
        settings::AppSettings {
            file_path: self.file_path.clone(),
            svg_path: self.svg_path.clone(),
            snap_to_grid: self.snap_to_grid,
            grid_size: self.grid_size,
            move_step: self.move_step,
            move_step_fast: self.move_step_fast,
            apply_style_to_selection: self.apply_style_to_selection,
            color_themes: self.color_themes.clone(),
            active_color_theme: self.active_color_theme,
            font_directory: self.font_directory.clone(),
        }
    }

    pub(super) fn lookup_color_by_name(&self, name: &str) -> Option<model::Rgba> {
        if let Some(idx) = self.active_color_theme {
            if let Some(theme) = self.color_themes.get(idx) {
                if let Some(color) = theme.get_color(name) {
                    return Some(color);
                }
            }
        }
        for theme in &self.color_themes {
            if let Some(color) = theme.get_color(name) {
                return Some(color);
            }
        }
        None
    }

    pub(super) fn all_color_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for theme in &self.color_themes {
            for name in theme.colors.keys() {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
        }
        names
    }

    pub(super) fn persist_settings(&mut self) {
        let snapshot = self.settings_snapshot();
        if let Err(e) = settings::save_settings(&self.settings_path, &snapshot) {
            self.status = Some(format!("Settings save failed: {e}"));
        }
    }

    pub(super) fn reload_settings(&mut self, ctx: &egui::Context) {
        let settings = settings::load_settings(&self.settings_path)
            .or_else(|| settings::load_settings("settings.json"))
            .unwrap_or_default();

        self.file_path = settings.file_path;
        self.svg_path = settings.svg_path;
        self.snap_to_grid = settings.snap_to_grid;
        self.grid_size = settings.grid_size;
        self.move_step = settings.move_step;
        self.move_step_fast = settings.move_step_fast;
        self.apply_style_to_selection = settings.apply_style_to_selection;
        self.color_themes = settings.color_themes;
        self.active_color_theme = settings.active_color_theme;
        self.font_directory = settings.font_directory.clone();

        if let Some(ref font_dir) = settings.font_directory {
            let loaded = super::DiagramApp::load_custom_fonts(ctx, font_dir);
            if loaded.is_empty() {
                self.status = Some(format!("Settings reloaded (no fonts found in {})", font_dir));
            } else {
                self.status = Some(format!("Settings reloaded, loaded {} font(s): {}", loaded.len(), loaded.join(", ")));
            }
            self.loaded_fonts = loaded;
        } else {
            self.loaded_fonts.clear();
            self.status = Some("Settings reloaded".to_string());
        }
    }
}

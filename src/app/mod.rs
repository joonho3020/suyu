use crate::model;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

mod actions;
mod command_palette;
mod doc_ops;
mod geometry;
mod interaction;
mod render;
mod settings;
mod svg;
mod update;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tool {
    Select,
    Rectangle,
    Ellipse,
    Triangle,
    Parallelogram,
    Trapezoid,
    Line,
    Arrow,
    BidirectionalArrow,
    Polyline,
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
        arrow_style: model::ArrowStyle,
    },
    Polyline {
        points: Vec<egui::Pos2>,
        current: egui::Pos2,
        arrow_style: model::ArrowStyle,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShapeAdjustKind {
    TriangleApex,
    ParallelogramSkew,
    TrapezoidTopInset,
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
    ShapeAdjust {
        element_id: u64,
        kind: ShapeAdjustKind,
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
    #[serde(default)]
    groups: Vec<model::Group>,
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
    svg_path: String,
    settings_path: String,
    status: Option<String>,
    editing_text_id: Option<u64>,
    inline_text_editing: bool,
    apply_style_to_selection: bool,
    snap_to_grid: bool,
    grid_size: f32,
    move_step: f32,
    move_step_fast: f32,
    palettes: Vec<settings::StylePalette>,
    active_palette: Option<usize>,
    new_palette_name: String,
    space_pan_happened: bool,
    command_palette: command_palette::CommandPalette,
}

impl DiagramApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let settings_path = "settings.toml".to_string();
        let settings = settings::load_settings(&settings_path)
            .or_else(|| settings::load_settings("settings.json"))
            .unwrap_or_default();
        let mut style = model::Style::default_for_shapes();
        if let Some(idx) = settings.active_palette {
            if let Some(p) = settings.palettes.get(idx) {
                style = p.style;
            }
        }
        Self {
            doc: model::Document::default(),
            selected: HashSet::new(),
            tool: Tool::Select,
            tool_before_pan: None,
            view: View::default(),
            next_id: 1,
            next_group_id: 1,
            style,
            in_progress: None,
            context_world_pos: None,
            context_hit: None,
            last_pointer_world: None,
            history: Vec::new(),
            future: Vec::new(),
            clipboard: None,
            drag_transform_recorded: false,
            active_transform: None,
            file_path: settings.file_path,
            svg_path: settings.svg_path,
            settings_path,
            status: None,
            editing_text_id: None,
            inline_text_editing: false,
            apply_style_to_selection: settings.apply_style_to_selection,
            snap_to_grid: settings.snap_to_grid,
            grid_size: settings.grid_size,
            move_step: settings.move_step,
            move_step_fast: settings.move_step_fast,
            palettes: settings.palettes,
            active_palette: settings.active_palette,
            new_palette_name: String::new(),
            space_pan_happened: false,
            command_palette: command_palette::CommandPalette::default(),
        }
    }
}

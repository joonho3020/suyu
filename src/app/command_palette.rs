use crate::model;
use eframe::egui;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use super::doc_ops::{AbutMode, AlignMode, DistributeMode, abut_selected, align_selected, distribute_selected};
use super::DiagramApp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CommandId {
    ToolSelect,
    ToolRectangle,
    ToolEllipse,
    ToolTriangle,
    ToolParallelogram,
    ToolTrapezoid,
    ToolLine,
    ToolArrow,
    ToolBidirectionalArrow,
    ToolPolyline,
    ToolPen,
    ToolText,
    Undo,
    Redo,
    Duplicate,
    Delete,
    Group,
    Ungroup,
    BringFront,
    SendBack,
    LayerUp,
    LayerDown,
    AlignLeft,
    AlignHCenter,
    AlignRight,
    AlignTop,
    AlignVCenter,
    AlignBottom,
    DistributeH,
    DistributeV,
    AbutH,
    AbutV,
    ConnectLine,
    ConnectArrow,
    ConnectBidirectional,
    SaveJson,
    LoadJson,
    ExportSvg,
    ToggleSnap,
    SnapSelectionToGrid,
    SetTextSize,
    SetStrokeWidth,
    SetStrokeColor,
    SetFillColor,
    SetFillNone,
    SetWidth,
    SetHeight,
    FontProportional,
    FontMonospace,
    TextAlignLeft,
    TextAlignCenter,
    TextAlignRight,
    LineStyleSolid,
    LineStyleDashed,
    LineStyleDotted,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    SetZoom,
    SelectAll,
    DeselectAll,
}

pub(super) struct CommandSpec {
    pub id: CommandId,
    pub name: &'static str,
    pub search: &'static str,
}

const COMMANDS: &[CommandSpec] = &[
    CommandSpec { id: CommandId::ToolSelect, name: "Tool: Select", search: "select tool v" },
    CommandSpec { id: CommandId::ToolRectangle, name: "Tool: Rectangle", search: "rectangle rect tool r" },
    CommandSpec { id: CommandId::ToolEllipse, name: "Tool: Ellipse", search: "ellipse oval circle tool o" },
    CommandSpec { id: CommandId::ToolTriangle, name: "Tool: Triangle", search: "triangle tool shift t" },
    CommandSpec { id: CommandId::ToolParallelogram, name: "Tool: Parallelogram", search: "parallelogram tool shift p" },
    CommandSpec { id: CommandId::ToolTrapezoid, name: "Tool: Trapezoid", search: "trapezoid tool shift z" },
    CommandSpec { id: CommandId::ToolLine, name: "Tool: Line", search: "line tool l" },
    CommandSpec { id: CommandId::ToolArrow, name: "Tool: Arrow", search: "arrow tool a" },
    CommandSpec { id: CommandId::ToolBidirectionalArrow, name: "Tool: Bidirectional Arrow", search: "bidirectional arrow both tool shift a" },
    CommandSpec { id: CommandId::ToolPolyline, name: "Tool: Polyline", search: "polyline multi line tool shift l" },
    CommandSpec { id: CommandId::ToolPen, name: "Tool: Pen", search: "pen freehand tool p" },
    CommandSpec { id: CommandId::ToolText, name: "Tool: Text", search: "text tool t" },
    CommandSpec { id: CommandId::Undo, name: "Edit: Undo", search: "undo" },
    CommandSpec { id: CommandId::Redo, name: "Edit: Redo", search: "redo" },
    CommandSpec { id: CommandId::Duplicate, name: "Edit: Duplicate", search: "duplicate clone" },
    CommandSpec { id: CommandId::Delete, name: "Edit: Delete", search: "delete remove" },
    CommandSpec { id: CommandId::SelectAll, name: "Edit: Select All", search: "select all" },
    CommandSpec { id: CommandId::DeselectAll, name: "Edit: Deselect All", search: "deselect clear selection" },
    CommandSpec { id: CommandId::Group, name: "Group: Group", search: "group" },
    CommandSpec { id: CommandId::Ungroup, name: "Group: Ungroup", search: "ungroup" },
    CommandSpec { id: CommandId::BringFront, name: "Layer: Bring to front", search: "front bring layer" },
    CommandSpec { id: CommandId::SendBack, name: "Layer: Send to back", search: "back send layer" },
    CommandSpec { id: CommandId::LayerUp, name: "Layer: Move up", search: "layer up move" },
    CommandSpec { id: CommandId::LayerDown, name: "Layer: Move down", search: "layer down move" },
    CommandSpec { id: CommandId::AlignLeft, name: "Align: Left", search: "align left" },
    CommandSpec { id: CommandId::AlignHCenter, name: "Align: Center (Horizontal)", search: "align center horizontal" },
    CommandSpec { id: CommandId::AlignRight, name: "Align: Right", search: "align right" },
    CommandSpec { id: CommandId::AlignTop, name: "Align: Top", search: "align top" },
    CommandSpec { id: CommandId::AlignVCenter, name: "Align: Middle (Vertical)", search: "align middle vertical" },
    CommandSpec { id: CommandId::AlignBottom, name: "Align: Bottom", search: "align bottom" },
    CommandSpec { id: CommandId::DistributeH, name: "Distribute: Horizontal", search: "distribute horizontal" },
    CommandSpec { id: CommandId::DistributeV, name: "Distribute: Vertical", search: "distribute vertical" },
    CommandSpec { id: CommandId::AbutH, name: "Abut: Horizontal", search: "abut horizontal pack" },
    CommandSpec { id: CommandId::AbutV, name: "Abut: Vertical", search: "abut vertical pack" },
    CommandSpec { id: CommandId::ConnectLine, name: "Connect: Line", search: "connect line auto connection" },
    CommandSpec { id: CommandId::ConnectArrow, name: "Connect: Arrow", search: "connect arrow auto connection" },
    CommandSpec { id: CommandId::ConnectBidirectional, name: "Connect: Bidirectional", search: "connect bidirectional both auto connection" },
    CommandSpec { id: CommandId::SaveJson, name: "File: Save", search: "save file json" },
    CommandSpec { id: CommandId::LoadJson, name: "File: Load", search: "load open file json" },
    CommandSpec { id: CommandId::ExportSvg, name: "File: Export SVG", search: "export svg save" },
    CommandSpec { id: CommandId::ToggleSnap, name: "Grid: Toggle snap", search: "grid snap toggle" },
    CommandSpec { id: CommandId::SnapSelectionToGrid, name: "Grid: Snap selection", search: "grid snap selection" },
    CommandSpec { id: CommandId::SetTextSize, name: "Format: Set Text Size...", search: "text size font pt" },
    CommandSpec { id: CommandId::SetStrokeWidth, name: "Format: Set Stroke Width...", search: "stroke width px line thickness" },
    CommandSpec { id: CommandId::SetStrokeColor, name: "Format: Set Stroke Color...", search: "stroke color rgb hex" },
    CommandSpec { id: CommandId::SetFillColor, name: "Format: Set Fill Color...", search: "fill color rgb hex background" },
    CommandSpec { id: CommandId::SetFillNone, name: "Format: No Fill", search: "fill none transparent clear" },
    CommandSpec { id: CommandId::SetWidth, name: "Object: Set Width...", search: "width size resize horizontal" },
    CommandSpec { id: CommandId::SetHeight, name: "Object: Set Height...", search: "height size resize vertical" },
    CommandSpec { id: CommandId::FontProportional, name: "Format: Font Proportional", search: "font proportional sans serif" },
    CommandSpec { id: CommandId::FontMonospace, name: "Format: Font Monospace", search: "font monospace code" },
    CommandSpec { id: CommandId::TextAlignLeft, name: "Format: Text Align Left", search: "text align left" },
    CommandSpec { id: CommandId::TextAlignCenter, name: "Format: Text Align Center", search: "text align center" },
    CommandSpec { id: CommandId::TextAlignRight, name: "Format: Text Align Right", search: "text align right" },
    CommandSpec { id: CommandId::LineStyleSolid, name: "Format: Line Solid", search: "line style solid" },
    CommandSpec { id: CommandId::LineStyleDashed, name: "Format: Line Dashed", search: "line style dashed" },
    CommandSpec { id: CommandId::LineStyleDotted, name: "Format: Line Dotted", search: "line style dotted" },
    CommandSpec { id: CommandId::ZoomIn, name: "View: Zoom In", search: "zoom in larger" },
    CommandSpec { id: CommandId::ZoomOut, name: "View: Zoom Out", search: "zoom out smaller" },
    CommandSpec { id: CommandId::ZoomReset, name: "View: Reset Zoom", search: "zoom reset 100" },
    CommandSpec { id: CommandId::SetZoom, name: "View: Set Zoom...", search: "zoom set percent" },
];

#[derive(Clone, Debug, Default)]
pub(super) enum InputMode {
    #[default]
    None,
    TextSize,
    StrokeWidth,
    StrokeColor,
    FillColor,
    Width,
    Height,
    Zoom,
}

#[derive(Default)]
pub(super) struct CommandPalette {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    request_focus: bool,
    input_mode: InputMode,
    input_value: String,
}

#[derive(Clone, Copy)]
pub(super) struct CommandContext {
    pub selected_len: usize,
    pub has_undo: bool,
    pub has_redo: bool,
    pub can_ungroup: bool,
    pub snap_to_grid: bool,
    pub has_resizable: bool,
}

impl CommandPalette {
    pub fn open(&mut self, query: impl Into<String>) {
        self.open = true;
        self.query = query.into();
        self.selected = 0;
        self.request_focus = true;
        self.input_mode = InputMode::None;
        self.input_value.clear();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
        self.request_focus = false;
        self.input_mode = InputMode::None;
        self.input_value.clear();
    }

    fn is_enabled(cx: CommandContext, id: CommandId) -> bool {
        match id {
            CommandId::Undo => cx.has_undo,
            CommandId::Redo => cx.has_redo,
            CommandId::Duplicate | CommandId::Delete => cx.selected_len > 0,
            CommandId::Group => cx.selected_len >= 2,
            CommandId::Ungroup => cx.can_ungroup,
            CommandId::AlignLeft
            | CommandId::AlignHCenter
            | CommandId::AlignRight
            | CommandId::AlignTop
            | CommandId::AlignVCenter
            | CommandId::AlignBottom
            | CommandId::DistributeH
            | CommandId::DistributeV
            | CommandId::AbutH
            | CommandId::AbutV => cx.selected_len >= 2,
            CommandId::ConnectLine | CommandId::ConnectArrow | CommandId::ConnectBidirectional => cx.selected_len == 2,
            CommandId::SnapSelectionToGrid => cx.selected_len > 0 && cx.snap_to_grid,
            CommandId::SetWidth | CommandId::SetHeight => cx.has_resizable,
            _ => true,
        }
    }

    fn apply_style_change<F>(app: &mut DiagramApp, f: F)
    where
        F: Fn(&mut model::Style),
    {
        app.push_undo();
        f(&mut app.style);
        if !app.selected.is_empty() {
            for e in &mut app.doc.elements {
                if app.selected.contains(&e.id) {
                    f(&mut e.style);
                }
            }
        }
    }

    fn apply_size_change(app: &mut DiagramApp, set_width: Option<f32>, set_height: Option<f32>) {
        if app.selected.is_empty() {
            return;
        }
        app.push_undo();
        for e in &mut app.doc.elements {
            if !app.selected.contains(&e.id) {
                continue;
            }
            match &mut e.kind {
                model::ElementKind::Rect { rect, .. }
                | model::ElementKind::Ellipse { rect, .. }
                | model::ElementKind::Triangle { rect, .. }
                | model::ElementKind::Parallelogram { rect, .. }
                | model::ElementKind::Trapezoid { rect, .. } => {
                    if let Some(w) = set_width {
                        rect.max.x = rect.min.x + w;
                    }
                    if let Some(h) = set_height {
                        rect.max.y = rect.min.y + h;
                    }
                }
                _ => {}
            }
        }
    }

    fn parse_color(s: &str) -> Option<model::Rgba> {
        let s = s.trim().trim_start_matches('#');
        if s.len() == 6 {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            return Some(model::Rgba { r, g, b, a: 255 });
        }
        if s.len() == 3 {
            let r = u8::from_str_radix(&s[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&s[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&s[2..3], 16).ok()? * 17;
            return Some(model::Rgba { r, g, b, a: 255 });
        }
        let parts: Vec<&str> = s.split(&[',', ' ']).filter(|p| !p.is_empty()).collect();
        if parts.len() == 3 {
            let r: u8 = parts[0].trim().parse().ok()?;
            let g: u8 = parts[1].trim().parse().ok()?;
            let b: u8 = parts[2].trim().parse().ok()?;
            return Some(model::Rgba { r, g, b, a: 255 });
        }
        match s.to_lowercase().as_str() {
            "black" => Some(model::Rgba { r: 0, g: 0, b: 0, a: 255 }),
            "white" => Some(model::Rgba { r: 255, g: 255, b: 255, a: 255 }),
            "red" => Some(model::Rgba { r: 200, g: 40, b: 40, a: 255 }),
            "green" => Some(model::Rgba { r: 40, g: 140, b: 60, a: 255 }),
            "blue" => Some(model::Rgba { r: 40, g: 90, b: 200, a: 255 }),
            "yellow" => Some(model::Rgba { r: 200, g: 200, b: 40, a: 255 }),
            "orange" => Some(model::Rgba { r: 200, g: 140, b: 40, a: 255 }),
            "purple" => Some(model::Rgba { r: 130, g: 60, b: 180, a: 255 }),
            "gray" | "grey" => Some(model::Rgba { r: 128, g: 128, b: 128, a: 255 }),
            _ => None,
        }
    }

    fn execute_with_value(app: &mut DiagramApp, mode: &InputMode, value: &str) -> bool {
        match mode {
            InputMode::TextSize => {
                if let Ok(size) = value.trim().parse::<f32>() {
                    if size > 0.0 && size <= 200.0 {
                        Self::apply_style_change(app, |s| s.text_size = size);
                        return true;
                    }
                }
            }
            InputMode::StrokeWidth => {
                if let Ok(width) = value.trim().parse::<f32>() {
                    if width > 0.0 && width <= 50.0 {
                        Self::apply_style_change(app, |s| s.stroke.width = width);
                        return true;
                    }
                }
            }
            InputMode::StrokeColor => {
                if let Some(color) = Self::parse_color(value) {
                    Self::apply_style_change(app, |s| s.stroke.color = color);
                    return true;
                }
            }
            InputMode::FillColor => {
                if let Some(color) = Self::parse_color(value) {
                    Self::apply_style_change(app, |s| s.fill = Some(color));
                    return true;
                }
            }
            InputMode::Width => {
                if let Ok(w) = value.trim().parse::<f32>() {
                    if w > 0.0 {
                        Self::apply_size_change(app, Some(w), None);
                        return true;
                    }
                }
            }
            InputMode::Height => {
                if let Ok(h) = value.trim().parse::<f32>() {
                    if h > 0.0 {
                        Self::apply_size_change(app, None, Some(h));
                        return true;
                    }
                }
            }
            InputMode::Zoom => {
                if let Ok(z) = value.trim().trim_end_matches('%').parse::<f32>() {
                    let zoom = (z / 100.0).clamp(0.1, 8.0);
                    app.view.zoom = zoom;
                    return true;
                }
            }
            InputMode::None => {}
        }
        false
    }

    pub(super) fn execute(app: &mut DiagramApp, ctx: &egui::Context, id: CommandId) {
        match id {
            CommandId::ToolSelect => app.tool = super::Tool::Select,
            CommandId::ToolRectangle => app.tool = super::Tool::Rectangle,
            CommandId::ToolEllipse => app.tool = super::Tool::Ellipse,
            CommandId::ToolTriangle => app.tool = super::Tool::Triangle,
            CommandId::ToolParallelogram => app.tool = super::Tool::Parallelogram,
            CommandId::ToolTrapezoid => app.tool = super::Tool::Trapezoid,
            CommandId::ToolLine => app.tool = super::Tool::Line,
            CommandId::ToolArrow => app.tool = super::Tool::Arrow,
            CommandId::ToolBidirectionalArrow => app.tool = super::Tool::BidirectionalArrow,
            CommandId::ToolPolyline => app.tool = super::Tool::Polyline,
            CommandId::ToolPen => app.tool = super::Tool::Pen,
            CommandId::ToolText => app.tool = super::Tool::Text,
            CommandId::Undo => app.undo(),
            CommandId::Redo => app.redo(),
            CommandId::Duplicate => app.duplicate_selected(),
            CommandId::Delete => app.delete_selected(),
            CommandId::SelectAll => {
                for e in &app.doc.elements {
                    app.selected.insert(e.id);
                }
            }
            CommandId::DeselectAll => app.clear_selection(),
            CommandId::Group => app.group_selected(),
            CommandId::Ungroup => app.ungroup_selected(),
            CommandId::BringFront => app.bring_selected_to_front(),
            CommandId::SendBack => app.send_selected_to_back(),
            CommandId::LayerUp => app.move_selected_layer_by(1),
            CommandId::LayerDown => app.move_selected_layer_by(-1),
            CommandId::AlignLeft => {
                app.push_undo();
                align_selected(&mut app.doc, &app.selected, AlignMode::Left);
            }
            CommandId::AlignHCenter => {
                app.push_undo();
                align_selected(&mut app.doc, &app.selected, AlignMode::HCenter);
            }
            CommandId::AlignRight => {
                app.push_undo();
                align_selected(&mut app.doc, &app.selected, AlignMode::Right);
            }
            CommandId::AlignTop => {
                app.push_undo();
                align_selected(&mut app.doc, &app.selected, AlignMode::Top);
            }
            CommandId::AlignVCenter => {
                app.push_undo();
                align_selected(&mut app.doc, &app.selected, AlignMode::VCenter);
            }
            CommandId::AlignBottom => {
                app.push_undo();
                align_selected(&mut app.doc, &app.selected, AlignMode::Bottom);
            }
            CommandId::DistributeH => {
                app.push_undo();
                distribute_selected(&mut app.doc, &app.selected, DistributeMode::Horizontal);
            }
            CommandId::DistributeV => {
                app.push_undo();
                distribute_selected(&mut app.doc, &app.selected, DistributeMode::Vertical);
            }
            CommandId::AbutH => {
                app.push_undo();
                abut_selected(&mut app.doc, &app.selected, AbutMode::Horizontal);
            }
            CommandId::AbutV => {
                app.push_undo();
                abut_selected(&mut app.doc, &app.selected, AbutMode::Vertical);
            }
            CommandId::ConnectLine => app.auto_connect_selected(model::ArrowStyle::None),
            CommandId::ConnectArrow => app.auto_connect_selected(model::ArrowStyle::End),
            CommandId::ConnectBidirectional => app.auto_connect_selected(model::ArrowStyle::Both),
            CommandId::SaveJson => app.save_to_path(),
            CommandId::LoadJson => app.load_from_path(),
            CommandId::ExportSvg => app.save_svg_to_path(),
            CommandId::ToggleSnap => {
                app.snap_to_grid = !app.snap_to_grid;
                app.persist_settings();
            }
            CommandId::SnapSelectionToGrid => {
                app.push_undo();
                app.snap_selected_to_grid();
            }
            CommandId::SetFillNone => {
                Self::apply_style_change(app, |s| s.fill = None);
            }
            CommandId::FontProportional => {
                Self::apply_style_change(app, |s| s.font_family = model::FontFamily::Proportional);
            }
            CommandId::FontMonospace => {
                Self::apply_style_change(app, |s| s.font_family = model::FontFamily::Monospace);
            }
            CommandId::TextAlignLeft => {
                Self::apply_style_change(app, |s| s.text_align = model::TextAlign::Left);
            }
            CommandId::TextAlignCenter => {
                Self::apply_style_change(app, |s| s.text_align = model::TextAlign::Center);
            }
            CommandId::TextAlignRight => {
                Self::apply_style_change(app, |s| s.text_align = model::TextAlign::Right);
            }
            CommandId::LineStyleSolid => {
                Self::apply_style_change(app, |s| s.stroke.line_style = model::LineStyle::Solid);
            }
            CommandId::LineStyleDashed => {
                Self::apply_style_change(app, |s| s.stroke.line_style = model::LineStyle::Dashed);
            }
            CommandId::LineStyleDotted => {
                Self::apply_style_change(app, |s| s.stroke.line_style = model::LineStyle::Dotted);
            }
            CommandId::ZoomIn => {
                app.view.zoom = (app.view.zoom * 1.25).min(8.0);
            }
            CommandId::ZoomOut => {
                app.view.zoom = (app.view.zoom / 1.25).max(0.1);
            }
            CommandId::ZoomReset => {
                app.view.zoom = 1.0;
            }
            CommandId::SetTextSize
            | CommandId::SetStrokeWidth
            | CommandId::SetStrokeColor
            | CommandId::SetFillColor
            | CommandId::SetWidth
            | CommandId::SetHeight
            | CommandId::SetZoom => {}
        }
        ctx.request_repaint();
    }

    fn filtered(&self) -> Vec<(&'static CommandSpec, i64)> {
        let matcher = SkimMatcherV2::default();
        let q = self.query.trim();
        if q.is_empty() {
            return COMMANDS.iter().map(|c| (c, 0)).collect();
        }
        let mut out = Vec::new();
        for c in COMMANDS {
            if let Some(score) = matcher.fuzzy_match(c.search, q) {
                out.push((c, score));
            }
        }
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.name.cmp(b.0.name)));
        out
    }

    fn input_mode_for_command(id: CommandId) -> Option<InputMode> {
        match id {
            CommandId::SetTextSize => Some(InputMode::TextSize),
            CommandId::SetStrokeWidth => Some(InputMode::StrokeWidth),
            CommandId::SetStrokeColor => Some(InputMode::StrokeColor),
            CommandId::SetFillColor => Some(InputMode::FillColor),
            CommandId::SetWidth => Some(InputMode::Width),
            CommandId::SetHeight => Some(InputMode::Height),
            CommandId::SetZoom => Some(InputMode::Zoom),
            _ => None,
        }
    }

    fn input_prompt(mode: &InputMode) -> &'static str {
        match mode {
            InputMode::TextSize => "Enter text size (e.g. 16):",
            InputMode::StrokeWidth => "Enter stroke width (e.g. 2):",
            InputMode::StrokeColor => "Enter color (hex #ff0000, rgb 255,0,0, or name):",
            InputMode::FillColor => "Enter fill color (hex #ff0000, rgb 255,0,0, or name):",
            InputMode::Width => "Enter width:",
            InputMode::Height => "Enter height:",
            InputMode::Zoom => "Enter zoom % (e.g. 100):",
            InputMode::None => "",
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, cx: CommandContext) -> Option<CommandId> {
        if !self.open {
            return None;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if matches!(self.input_mode, InputMode::None) {
                self.close();
            } else {
                self.input_mode = InputMode::None;
                self.input_value.clear();
                self.request_focus = true;
            }
            return None;
        }

        let screen = ctx.content_rect();
        let width = 560.0;
        let height = 320.0;
        let pos = egui::pos2(screen.center().x - width * 0.5, screen.top() + 48.0);
        let area_id = egui::Id::new("command_palette");

        let mut result: Option<CommandId> = None;

        egui::Area::new(area_id)
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let frame = egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 20, 240))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 160, 255)))
                    .inner_margin(10.0)
                    .corner_radius(egui::CornerRadius::same(8));
                frame.show(ui, |ui| {
                    ui.set_min_size(egui::vec2(width, height));

                    if matches!(self.input_mode, InputMode::None) {
                        let matches = self.filtered();
                        if self.selected >= matches.len() {
                            self.selected = matches.len().saturating_sub(1);
                        }

                        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) && !matches.is_empty() {
                            self.selected = (self.selected + 1).min(matches.len() - 1);
                        }
                        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) && !matches.is_empty() {
                            self.selected = self.selected.saturating_sub(1);
                        }
                        let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));

                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.query)
                                .desired_width(f32::INFINITY)
                                .hint_text("Search commands..."),
                        );
                        if self.request_focus {
                            resp.request_focus();
                            self.request_focus = false;
                        }
                        ui.separator();

                        let scroll_id = ui.id().with("cmd_scroll");
                        egui::ScrollArea::vertical()
                            .id_salt(scroll_id)
                            .max_height(height - 64.0)
                            .show(ui, |ui| {
                                for (idx, (spec, _score)) in matches.iter().enumerate() {
                                    let enabled = CommandPalette::is_enabled(cx, spec.id);
                                    let selected = idx == self.selected;
                                    let resp = ui.add_enabled(
                                        enabled,
                                        egui::Button::new(spec.name).selected(selected),
                                    );
                                    if selected {
                                        resp.scroll_to_me(Some(egui::Align::Center));
                                    }
                                    if resp.clicked() {
                                        self.selected = idx;
                                        if let Some(mode) = Self::input_mode_for_command(spec.id) {
                                            self.input_mode = mode;
                                            self.input_value.clear();
                                            self.request_focus = true;
                                        } else {
                                            result = Some(spec.id);
                                            self.close();
                                        }
                                    }
                                }
                            });

                        if enter_pressed {
                            if let Some((spec, _)) = matches.get(self.selected) {
                                if CommandPalette::is_enabled(cx, spec.id) {
                                    if let Some(mode) = Self::input_mode_for_command(spec.id) {
                                        self.input_mode = mode;
                                        self.input_value.clear();
                                        self.request_focus = true;
                                    } else {
                                        result = Some(spec.id);
                                        self.close();
                                    }
                                }
                            }
                        }
                    } else {
                        ui.label(Self::input_prompt(&self.input_mode));
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.input_value)
                                .desired_width(f32::INFINITY)
                                .hint_text("Enter value..."),
                        );
                        if self.request_focus {
                            resp.request_focus();
                            self.request_focus = false;
                        }

                        ui.separator();
                        ui.label("Press Enter to apply, Escape to cancel");
                    }
                });
            });

        result
    }

    pub fn take_input_data(&mut self) -> Option<(InputMode, String)> {
        if matches!(self.input_mode, InputMode::None) {
            return None;
        }
        let mode = std::mem::take(&mut self.input_mode);
        let value = std::mem::take(&mut self.input_value);
        Some((mode, value))
    }

    pub fn execute_input(app: &mut DiagramApp, mode: InputMode, value: &str) -> bool {
        Self::execute_with_value(app, &mode, value)
    }

    pub fn is_awaiting_input(&self) -> bool {
        !matches!(self.input_mode, InputMode::None)
    }
}

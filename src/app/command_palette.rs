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
];

#[derive(Default)]
pub(super) struct CommandPalette {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    request_focus: bool,
}

#[derive(Clone, Copy)]
pub(super) struct CommandContext {
    pub selected_len: usize,
    pub has_undo: bool,
    pub has_redo: bool,
    pub can_ungroup: bool,
    pub snap_to_grid: bool,
}

impl CommandPalette {
    pub fn open(&mut self, query: impl Into<String>) {
        self.open = true;
        self.query = query.into();
        self.selected = 0;
        self.request_focus = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
        self.request_focus = false;
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
            _ => true,
        }
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

    pub fn ui(&mut self, ctx: &egui::Context, cx: CommandContext) -> Option<CommandId> {
        if !self.open {
            return None;
        }
        let matches = self.filtered();
        if self.selected >= matches.len() {
            self.selected = matches.len().saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close();
            return None;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) && !matches.is_empty() {
            self.selected = (self.selected + 1).min(matches.len() - 1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) && !matches.is_empty() {
            self.selected = self.selected.saturating_sub(1);
        }
        let mut run_selected = ctx.input(|i| i.key_pressed(egui::Key::Enter));

        let screen = ctx.content_rect();
        let width = 560.0;
        let height = 320.0;
        let pos = egui::pos2(screen.center().x - width * 0.5, screen.top() + 48.0);
        let area_id = egui::Id::new("command_palette");
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
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .desired_width(f32::INFINITY)
                            .hint_text("Search commands"),
                    );
                    if self.request_focus {
                        resp.request_focus();
                        self.request_focus = false;
                    }
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(height - 64.0).show(ui, |ui| {
                        for (idx, (spec, _score)) in matches.iter().take(24).enumerate() {
                            let enabled = CommandPalette::is_enabled(cx, spec.id);
                            let selected = idx == self.selected;
                            let resp = ui.add_enabled(
                                enabled,
                                egui::Button::new(spec.name).selected(selected),
                            );
                            if resp.clicked() {
                                self.selected = idx;
                                run_selected = true;
                            }
                        }
                    });
                });
            });

        if run_selected {
            if let Some((spec, _)) = matches.get(self.selected) {
                if CommandPalette::is_enabled(cx, spec.id) {
                    let cmd = spec.id;
                    self.close();
                    return Some(cmd);
                }
            }
        }
        None
    }
}

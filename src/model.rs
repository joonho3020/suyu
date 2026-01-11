use eframe::egui;
use serde::{Deserialize, Serialize};
use crate::text_format;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn from_pos2(p: egui::Pos2) -> Self {
        Self { x: p.x, y: p.y }
    }

    pub fn to_pos2(self) -> egui::Pos2 {
        egui::pos2(self.x, self.y)
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct RectF {
    pub min: Point,
    pub max: Point,
}

impl RectF {
    pub fn from_min_max(a: egui::Pos2, b: egui::Pos2) -> Self {
        let min = egui::pos2(a.x.min(b.x), a.y.min(b.y));
        let max = egui::pos2(a.x.max(b.x), a.y.max(b.y));
        Self {
            min: Point::from_pos2(min),
            max: Point::from_pos2(max),
        }
    }

    pub fn to_rect(self) -> egui::Rect {
        egui::Rect::from_min_max(self.min.to_pos2(), self.max.to_pos2())
    }

    pub fn is_valid(self) -> bool {
        self.max.x > self.min.x && self.max.y > self.min.y
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub fn to_color32(self) -> egui::Color32 {
        egui::Color32::from_rgba_premultiplied(self.r, self.g, self.b, self.a)
    }

    pub fn from_color32(c: egui::Color32) -> Self {
        let [r, g, b, a] = c.to_array();
        Self { r, g, b, a }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum LineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum FontFamily {
    #[default]
    Proportional,
    Monospace,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct StrokeStyle {
    pub color: Rgba,
    pub width: f32,
    #[serde(default)]
    pub line_style: LineStyle,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            color: Rgba {
                r: 30,
                g: 30,
                b: 30,
                a: 255,
            },
            width: 2.0,
            line_style: LineStyle::Solid,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Style {
    pub stroke: StrokeStyle,
    pub fill: Option<Rgba>,
    pub text_color: Rgba,
    pub text_size: f32,
    #[serde(default)]
    pub text_align: TextAlign,
    #[serde(default)]
    pub font_family: FontFamily,
}

impl Style {
    pub fn default_for_shapes() -> Self {
        Self {
            stroke: StrokeStyle::default(),
            fill: Some(Rgba {
                r: 255,
                g: 255,
                b: 255,
                a: 0,
            }),
            text_color: Rgba {
                r: 30,
                g: 30,
                b: 30,
                a: 255,
            },
            text_size: 16.0,
            text_align: TextAlign::Center,
            font_family: FontFamily::Proportional,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub elements: Vec<Element>,
    #[serde(default)]
    pub groups: Vec<Group>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            elements: vec![],
            groups: vec![],
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Group {
    pub id: u64,
    #[serde(default)]
    pub parent_id: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Element {
    pub id: u64,
    #[serde(default)]
    pub group_id: Option<u64>,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default = "default_snap_enabled")]
    pub snap_enabled: bool,
    pub kind: ElementKind,
    pub style: Style,
}

fn default_snap_enabled() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Binding {
    pub element_id: u64,
    pub norm: Point,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum ArrowStyle {
    #[default]
    None,
    End,
    Start,
    Both,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ElementKind {
    Rect {
        rect: RectF,
        #[serde(default)]
        label: String,
    },
    Ellipse {
        rect: RectF,
        #[serde(default)]
        label: String,
    },
    Triangle {
        rect: RectF,
        #[serde(default)]
        label: String,
        #[serde(default)]
        apex_ratio: f32,
    },
    Parallelogram {
        rect: RectF,
        #[serde(default)]
        label: String,
        #[serde(default = "default_parallelogram_skew_ratio")]
        skew_ratio: f32,
    },
    Trapezoid {
        rect: RectF,
        #[serde(default)]
        label: String,
        #[serde(default = "default_trapezoid_inset_ratio")]
        top_inset_ratio: f32,
    },
    Line {
        a: Point,
        b: Point,
        #[serde(default)]
        arrow: bool,
        #[serde(default)]
        arrow_style: ArrowStyle,
        #[serde(default)]
        start_binding: Option<Binding>,
        #[serde(default)]
        end_binding: Option<Binding>,
    },
    Polyline {
        points: Vec<Point>,
        #[serde(default)]
        arrow_style: ArrowStyle,
    },
    Pen {
        points: Vec<Point>,
    },
    Text {
        pos: Point,
        text: String,
    },
}

fn default_parallelogram_skew_ratio() -> f32 {
    0.25
}

fn default_trapezoid_inset_ratio() -> f32 {
    0.25
}

impl Element {
    pub fn bounds(&self) -> egui::Rect {
        match &self.kind {
            ElementKind::Rect { rect, .. }
            | ElementKind::Triangle { rect, .. }
            | ElementKind::Parallelogram { rect, .. }
            | ElementKind::Trapezoid { rect, .. } => {
                rotated_rect_aabb(rect.to_rect(), self.rotation).expand(self.style.stroke.width)
            }
            ElementKind::Ellipse { rect, .. } => {
                rotated_ellipse_aabb(rect.to_rect(), self.rotation).expand(self.style.stroke.width)
            }
            ElementKind::Line { a, b, .. } => {
                egui::Rect::from_two_pos(a.to_pos2(), b.to_pos2()).expand(self.style.stroke.width)
            }
            ElementKind::Polyline { points, .. } | ElementKind::Pen { points } => {
                let mut it = points.iter();
                let Some(first) = it.next() else {
                    return egui::Rect::NOTHING;
                };
                let mut min = first.to_pos2();
                let mut max = first.to_pos2();
                for p in it {
                    let p = p.to_pos2();
                    min.x = min.x.min(p.x);
                    min.y = min.y.min(p.y);
                    max.x = max.x.max(p.x);
                    max.y = max.y.max(p.y);
                }
                egui::Rect::from_min_max(min, max).expand(self.style.stroke.width)
            }
            ElementKind::Text { pos, text } => {
                let pos = pos.to_pos2();
                let w = (text_format::visual_char_count(text) as f32).max(1.0)
                    * self.style.text_size
                    * 0.6;
                let h = self.style.text_size * 1.2;
                let x = match self.style.text_align {
                    TextAlign::Left => pos.x,
                    TextAlign::Center => pos.x - w * 0.5,
                    TextAlign::Right => pos.x - w,
                };
                egui::Rect::from_min_size(egui::pos2(x, pos.y), egui::vec2(w, h))
            }
        }
    }
}

fn rotated_rect_aabb(rect: egui::Rect, rotation: f32) -> egui::Rect {
    if rotation.abs() <= f32::EPSILON {
        return rect;
    }
    let center = rect.center();
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    let sin = rotation.sin();
    let cos = rotation.cos();
    let mut min = egui::pos2(f32::INFINITY, f32::INFINITY);
    let mut max = egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY);
    for p in corners {
        let v = p - center;
        let r = egui::vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
        let p = center + r;
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    egui::Rect::from_min_max(min, max)
}

fn rotated_ellipse_aabb(rect: egui::Rect, rotation: f32) -> egui::Rect {
    let center = rect.center();
    let rx = rect.width() * 0.5;
    let ry = rect.height() * 0.5;
    if rx <= f32::EPSILON || ry <= f32::EPSILON {
        return rect;
    }
    if rotation.abs() <= f32::EPSILON {
        return rect;
    }
    let sin = rotation.sin();
    let cos = rotation.cos();
    let mut min = egui::pos2(f32::INFINITY, f32::INFINITY);
    let mut max = egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY);
    let steps = 32;
    for i in 0..steps {
        let t = (i as f32) / (steps as f32) * std::f32::consts::TAU;
        let x = t.cos() * rx;
        let y = t.sin() * ry;
        let r = egui::vec2(x * cos - y * sin, x * sin + y * cos);
        let p = center + r;
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    egui::Rect::from_min_max(min, max)
}

pub fn distance_to_segment(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let ab_len2 = ab.x * ab.x + ab.y * ab.y;
    if ab_len2 <= f32::EPSILON {
        return (p - a).length();
    }
    let t = (ap.x * ab.x + ap.y * ab.y) / ab_len2;
    let t = t.clamp(0.0, 1.0);
    let closest = a + ab * t;
    (p - closest).length()
}

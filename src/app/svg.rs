use crate::{model, text_format};
use eframe::egui;
use std::collections::HashMap;

use super::geometry::{
    resolved_line_endpoints_world, rotated_ellipse_points_world, rotated_parallelogram_points_world,
    rotated_rect_points_world, rotated_trapezoid_points_world, rotated_triangle_points_world,
};

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn rgba_to_svg_rgb(rgba: model::Rgba) -> (String, f32) {
    let opacity = (rgba.a as f32) / 255.0;
    (format!("rgb({},{},{})", rgba.r, rgba.g, rgba.b), opacity)
}

fn dasharray(line_style: model::LineStyle, stroke_width: f32) -> Option<String> {
    match line_style {
        model::LineStyle::Solid => None,
        model::LineStyle::Dashed => Some(format!("{} {}", stroke_width * 4.0, stroke_width * 2.5)),
        model::LineStyle::Dotted => Some(format!("{} {}", stroke_width * 0.5, stroke_width * 2.0)),
    }
}

fn points_attr(points: &[egui::Pos2]) -> String {
    let mut out = String::new();
    for (i, p) in points.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&format!("{:.3},{:.3}", p.x, p.y));
    }
    out
}

fn marker_id(stroke: model::Rgba) -> String {
    format!("arrow_{}_{}_{}_{}", stroke.r, stroke.g, stroke.b, stroke.a)
}

fn rich_text_line_content(spans: &[text_format::Span], font_size: f32) -> String {
    let mut out = String::new();
    for s in spans {
        match s.script {
            text_format::Script::Normal => out.push_str(&escape_xml(&s.text)),
            text_format::Script::Sup => out.push_str(&format!(
                r#"<tspan baseline-shift="super" font-size="{:.3}">{}</tspan>"#,
                font_size * 0.7,
                escape_xml(&s.text)
            )),
            text_format::Script::Sub => out.push_str(&format!(
                r#"<tspan baseline-shift="sub" font-size="{:.3}">{}</tspan>"#,
                font_size * 0.7,
                escape_xml(&s.text)
            )),
        }
    }
    out
}

fn rich_text_tspans(text: &str, font_size: f32, x: f32) -> String {
    let lines = text_format::parse_rich_text_lines(text);
    let line_height = font_size * 1.2;
    let mut out = String::new();
    for (i, line_spans) in lines.iter().enumerate() {
        let dy = if i == 0 { 0.0 } else { line_height };
        let content = rich_text_line_content(line_spans, font_size);
        if i == 0 {
            out.push_str(&content);
        } else {
            out.push_str(&format!(
                r#"<tspan x="{:.3}" dy="{:.3}">{}</tspan>"#,
                x, dy, content
            ));
        }
    }
    out
}

fn svg_text_anchor(align: model::TextAlign) -> &'static str {
    match align {
        model::TextAlign::Left => "start",
        model::TextAlign::Center => "middle",
        model::TextAlign::Right => "end",
    }
}

fn svg_font_family(family: model::FontFamily) -> &'static str {
    match family {
        model::FontFamily::Proportional => "sans-serif",
        model::FontFamily::Monospace => "monospace",
    }
}

fn label_anchor_in_rect(rect: egui::Rect, align: model::TextAlign) -> egui::Pos2 {
    let c = rect.center();
    let padding = 8.0_f32.min(rect.width() * 0.25);
    match align {
        model::TextAlign::Left => egui::pos2(rect.left() + padding, c.y),
        model::TextAlign::Center => c,
        model::TextAlign::Right => egui::pos2(rect.right() - padding, c.y),
    }
}

pub(super) fn document_to_svg(doc: &model::Document) -> String {
    let mut bounds: Option<egui::Rect> = None;
    for e in &doc.elements {
        let b = e.bounds();
        bounds = Some(bounds.map(|r| r.union(b)).unwrap_or(b));
    }
    let bounds = bounds.unwrap_or_else(|| egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0)));
    let padding = 24.0;
    let min_x = bounds.min.x - padding;
    let min_y = bounds.min.y - padding;
    let width = bounds.width() + padding * 2.0;
    let height = bounds.height() + padding * 2.0;

    let mut markers: HashMap<String, model::Rgba> = HashMap::new();
    for e in &doc.elements {
        let needs_marker = match &e.kind {
            model::ElementKind::Line { arrow, arrow_style, .. } => {
                *arrow || !matches!(arrow_style, model::ArrowStyle::None)
            }
            model::ElementKind::Polyline { arrow_style, .. } => !matches!(arrow_style, model::ArrowStyle::None),
            _ => false,
        };
        if needs_marker {
            let id = marker_id(e.style.stroke.color);
            markers.entry(id).or_insert(e.style.stroke.color);
        }
    }

    let mut out = String::new();
    out.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    out.push('\n');
    out.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{:.3} {:.3} {:.3} {:.3}" width="{:.3}" height="{:.3}">"#,
        min_x, min_y, width, height, width, height
    ));
    out.push('\n');
    out.push_str("<defs>\n");
    for (id, stroke) in &markers {
        let (rgb, opacity) = rgba_to_svg_rgb(*stroke);
        out.push_str(&format!(
            r#"<marker id="{}" markerWidth="10" markerHeight="10" refX="10" refY="5" orient="auto-start-reverse" markerUnits="strokeWidth">"#,
            id
        ));
        out.push('\n');
        out.push_str(&format!(
            r#"<path d="M 0 0 L 10 5 L 0 10 z" fill="{}" fill-opacity="{:.3}"/>"#,
            rgb, opacity
        ));
        out.push('\n');
        out.push_str("</marker>\n");
    }
    out.push_str("</defs>\n");

    for e in &doc.elements {
        let stroke_width = e.style.stroke.width;
        let (stroke_rgb, stroke_opacity) = rgba_to_svg_rgb(e.style.stroke.color);
        let dash = dasharray(e.style.stroke.line_style, stroke_width);
        let stroke_attrs = if let Some(dash) = dash {
            format!(
                r#"stroke="{}" stroke-opacity="{:.3}" stroke-width="{:.3}" fill="none" stroke-dasharray="{}""#,
                stroke_rgb, stroke_opacity, stroke_width, dash
            )
        } else {
            format!(
                r#"stroke="{}" stroke-opacity="{:.3}" stroke-width="{:.3}" fill="none""#,
                stroke_rgb, stroke_opacity, stroke_width
            )
        };

        match &e.kind {
            model::ElementKind::Rect { rect, label } => {
                let pts = rotated_rect_points_world(rect.to_rect(), e.rotation);
                let fill_attrs = match e.style.fill {
                    Some(rgba) if rgba.a > 0 => {
                        let (rgb, opacity) = rgba_to_svg_rgb(rgba);
                        format!(r#"fill="{}" fill-opacity="{:.3}""#, rgb, opacity)
                    }
                    _ => r#"fill="none""#.to_string(),
                };
                out.push_str(&format!(
                    r#"<polygon points="{}" {} {} />"#,
                    points_attr(&pts),
                    stroke_attrs,
                    fill_attrs
                ));
                out.push('\n');
                if !label.is_empty() {
                    let r = rect.to_rect();
                    let c = r.center();
                    let p = label_anchor_in_rect(r, e.style.text_align);
                    let (rgb, opacity) = rgba_to_svg_rgb(e.style.text_color);
                    let content = rich_text_tspans(label, e.style.text_size, p.x);
                    out.push_str(&format!(
                        r#"<text x="{:.3}" y="{:.3}" font-size="{:.3}" font-family="{}" fill="{}" fill-opacity="{:.3}" text-anchor="{}" dominant-baseline="middle" transform="rotate({:.3} {:.3} {:.3})">{}</text>"#,
                        p.x,
                        p.y,
                        e.style.text_size,
                        svg_font_family(e.style.font_family),
                        rgb,
                        opacity,
                        svg_text_anchor(e.style.text_align),
                        e.rotation.to_degrees(),
                        c.x,
                        c.y,
                        content
                    ));
                    out.push('\n');
                }
            }
            model::ElementKind::Ellipse { rect, label } => {
                let pts = rotated_ellipse_points_world(rect.to_rect(), e.rotation);
                let fill_attrs = match e.style.fill {
                    Some(rgba) if rgba.a > 0 => {
                        let (rgb, opacity) = rgba_to_svg_rgb(rgba);
                        format!(r#"fill="{}" fill-opacity="{:.3}""#, rgb, opacity)
                    }
                    _ => r#"fill="none""#.to_string(),
                };
                out.push_str(&format!(
                    r#"<polygon points="{}" {} {} />"#,
                    points_attr(&pts),
                    stroke_attrs,
                    fill_attrs
                ));
                out.push('\n');
                if !label.is_empty() {
                    let r = rect.to_rect();
                    let c = r.center();
                    let p = label_anchor_in_rect(r, e.style.text_align);
                    let (rgb, opacity) = rgba_to_svg_rgb(e.style.text_color);
                    let content = rich_text_tspans(label, e.style.text_size, p.x);
                    out.push_str(&format!(
                        r#"<text x="{:.3}" y="{:.3}" font-size="{:.3}" font-family="{}" fill="{}" fill-opacity="{:.3}" text-anchor="{}" dominant-baseline="middle" transform="rotate({:.3} {:.3} {:.3})">{}</text>"#,
                        p.x,
                        p.y,
                        e.style.text_size,
                        svg_font_family(e.style.font_family),
                        rgb,
                        opacity,
                        svg_text_anchor(e.style.text_align),
                        e.rotation.to_degrees(),
                        c.x,
                        c.y,
                        content
                    ));
                    out.push('\n');
                }
            }
            model::ElementKind::Triangle {
                rect,
                label,
                apex_ratio,
            } => {
                let pts = rotated_triangle_points_world(rect.to_rect(), e.rotation, *apex_ratio);
                let fill_attrs = match e.style.fill {
                    Some(rgba) if rgba.a > 0 => {
                        let (rgb, opacity) = rgba_to_svg_rgb(rgba);
                        format!(r#"fill="{}" fill-opacity="{:.3}""#, rgb, opacity)
                    }
                    _ => r#"fill="none""#.to_string(),
                };
                out.push_str(&format!(
                    r#"<polygon points="{}" {} {} />"#,
                    points_attr(&pts),
                    stroke_attrs,
                    fill_attrs
                ));
                out.push('\n');
                if !label.is_empty() {
                    let r = rect.to_rect();
                    let c = r.center();
                    let p = label_anchor_in_rect(r, e.style.text_align);
                    let (rgb, opacity) = rgba_to_svg_rgb(e.style.text_color);
                    let content = rich_text_tspans(label, e.style.text_size, p.x);
                    out.push_str(&format!(
                        r#"<text x="{:.3}" y="{:.3}" font-size="{:.3}" font-family="{}" fill="{}" fill-opacity="{:.3}" text-anchor="{}" dominant-baseline="middle" transform="rotate({:.3} {:.3} {:.3})">{}</text>"#,
                        p.x,
                        p.y,
                        e.style.text_size,
                        svg_font_family(e.style.font_family),
                        rgb,
                        opacity,
                        svg_text_anchor(e.style.text_align),
                        e.rotation.to_degrees(),
                        c.x,
                        c.y,
                        content
                    ));
                    out.push('\n');
                }
            }
            model::ElementKind::Parallelogram {
                rect,
                label,
                skew_ratio,
            } => {
                let pts = rotated_parallelogram_points_world(rect.to_rect(), e.rotation, *skew_ratio);
                let fill_attrs = match e.style.fill {
                    Some(rgba) if rgba.a > 0 => {
                        let (rgb, opacity) = rgba_to_svg_rgb(rgba);
                        format!(r#"fill="{}" fill-opacity="{:.3}""#, rgb, opacity)
                    }
                    _ => r#"fill="none""#.to_string(),
                };
                out.push_str(&format!(
                    r#"<polygon points="{}" {} {} />"#,
                    points_attr(&pts),
                    stroke_attrs,
                    fill_attrs
                ));
                out.push('\n');
                if !label.is_empty() {
                    let r = rect.to_rect();
                    let c = r.center();
                    let p = label_anchor_in_rect(r, e.style.text_align);
                    let (rgb, opacity) = rgba_to_svg_rgb(e.style.text_color);
                    let content = rich_text_tspans(label, e.style.text_size, p.x);
                    out.push_str(&format!(
                        r#"<text x="{:.3}" y="{:.3}" font-size="{:.3}" font-family="{}" fill="{}" fill-opacity="{:.3}" text-anchor="{}" dominant-baseline="middle" transform="rotate({:.3} {:.3} {:.3})">{}</text>"#,
                        p.x,
                        p.y,
                        e.style.text_size,
                        svg_font_family(e.style.font_family),
                        rgb,
                        opacity,
                        svg_text_anchor(e.style.text_align),
                        e.rotation.to_degrees(),
                        c.x,
                        c.y,
                        content
                    ));
                    out.push('\n');
                }
            }
            model::ElementKind::Trapezoid {
                rect,
                label,
                top_inset_ratio,
            } => {
                let pts =
                    rotated_trapezoid_points_world(rect.to_rect(), e.rotation, *top_inset_ratio);
                let fill_attrs = match e.style.fill {
                    Some(rgba) if rgba.a > 0 => {
                        let (rgb, opacity) = rgba_to_svg_rgb(rgba);
                        format!(r#"fill="{}" fill-opacity="{:.3}""#, rgb, opacity)
                    }
                    _ => r#"fill="none""#.to_string(),
                };
                out.push_str(&format!(
                    r#"<polygon points="{}" {} {} />"#,
                    points_attr(&pts),
                    stroke_attrs,
                    fill_attrs
                ));
                out.push('\n');
                if !label.is_empty() {
                    let r = rect.to_rect();
                    let c = r.center();
                    let p = label_anchor_in_rect(r, e.style.text_align);
                    let (rgb, opacity) = rgba_to_svg_rgb(e.style.text_color);
                    let content = rich_text_tspans(label, e.style.text_size, p.x);
                    out.push_str(&format!(
                        r#"<text x="{:.3}" y="{:.3}" font-size="{:.3}" font-family="{}" fill="{}" fill-opacity="{:.3}" text-anchor="{}" dominant-baseline="middle" transform="rotate({:.3} {:.3} {:.3})">{}</text>"#,
                        p.x,
                        p.y,
                        e.style.text_size,
                        svg_font_family(e.style.font_family),
                        rgb,
                        opacity,
                        svg_text_anchor(e.style.text_align),
                        e.rotation.to_degrees(),
                        c.x,
                        c.y,
                        content
                    ));
                    out.push('\n');
                }
            }
            model::ElementKind::Line {
                a,
                b,
                arrow,
                arrow_style,
                start_binding,
                end_binding,
            } => {
                let (a, b) = resolved_line_endpoints_world(doc, *a, *b, start_binding, end_binding);
                let mut attrs = stroke_attrs;
                let has_end_arrow = *arrow || matches!(arrow_style, model::ArrowStyle::End | model::ArrowStyle::Both);
                let has_start_arrow = matches!(arrow_style, model::ArrowStyle::Start | model::ArrowStyle::Both);
                if has_start_arrow || has_end_arrow {
                    let mid = marker_id(e.style.stroke.color);
                    if has_start_arrow {
                        attrs.push_str(&format!(r#" marker-start="url(#{})""#, mid));
                    }
                    if has_end_arrow {
                        attrs.push_str(&format!(r#" marker-end="url(#{})""#, mid));
                    }
                }
                out.push_str(&format!(
                    r#"<line x1="{:.3}" y1="{:.3}" x2="{:.3}" y2="{:.3}" {} />"#,
                    a.x, a.y, b.x, b.y, attrs
                ));
                out.push('\n');
            }
            model::ElementKind::Polyline {
                points,
                arrow_style,
            } => {
                if points.len() >= 2 {
                    let pts: Vec<egui::Pos2> = points.iter().map(|p| p.to_pos2()).collect();
                    let mut attrs = stroke_attrs;
                    let has_end_arrow = matches!(arrow_style, model::ArrowStyle::End | model::ArrowStyle::Both);
                    let has_start_arrow = matches!(arrow_style, model::ArrowStyle::Start | model::ArrowStyle::Both);
                    if has_start_arrow || has_end_arrow {
                        let mid = marker_id(e.style.stroke.color);
                        if has_start_arrow {
                            attrs.push_str(&format!(r#" marker-start="url(#{})""#, mid));
                        }
                        if has_end_arrow {
                            attrs.push_str(&format!(r#" marker-end="url(#{})""#, mid));
                        }
                    }
                    out.push_str(&format!(
                        r#"<polyline points="{}" {} />"#,
                        points_attr(&pts),
                        attrs
                    ));
                    out.push('\n');
                }
            }
            model::ElementKind::Pen { points } => {
                if points.len() >= 2 {
                    let pts: Vec<egui::Pos2> = points.iter().map(|p| p.to_pos2()).collect();
                    out.push_str(&format!(
                        r#"<polyline points="{}" {} />"#,
                        points_attr(&pts),
                        stroke_attrs
                    ));
                    out.push('\n');
                }
            }
            model::ElementKind::Text { pos, text } => {
                let p = pos.to_pos2();
                let (rgb, opacity) = rgba_to_svg_rgb(e.style.text_color);
                let content = rich_text_tspans(text, e.style.text_size, p.x);
                out.push_str(&format!(
                    r#"<text x="{:.3}" y="{:.3}" font-size="{:.3}" font-family="{}" fill="{}" fill-opacity="{:.3}" text-anchor="{}" dominant-baseline="hanging">{}</text>"#,
                    p.x,
                    p.y,
                    e.style.text_size,
                    svg_font_family(e.style.font_family),
                    rgb,
                    opacity,
                    svg_text_anchor(e.style.text_align),
                    content
                ));
                out.push('\n');
            }
        }
    }

    out.push_str("</svg>\n");
    out
}

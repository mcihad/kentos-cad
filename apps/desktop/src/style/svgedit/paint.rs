//! How the canvas paints a shape (what the browser does with the web's
//! `elementOf` elements): rectangles and ellipses as the SVG core's paths,
//! paths with their fill rule, strokes with their width, ends, corners and
//! dashes scaled by the view, texts in their typeface (Arimo for Arial, the
//! system's serif for Times) turned about their anchor; the symbol's colour
//! and its second colour as the preview's, the shape's opacity on both.

use iced::widget::canvas::{
    Fill, Frame, LineCap, LineDash, LineJoin, Path, Stroke, Style, Text, fill,
};
use iced::{Color, Pixels, Point, Radians, Vector};
use kentos_svg_core::model::to_path;
use kentos_svg_core::shape::{Obj, Pt, SubPath};
use kentos_ui::widget::color::parse_hex;

use super::state::Options;

/// Where a drawing unit lands on the screen: `p · zoom + offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub zoom: f64,
    pub ox: f64,
    pub oy: f64,
}

impl View {
    pub fn point(&self, p: Pt) -> Point {
        Point::new(
            (p[0] * self.zoom + self.ox) as f32,
            (p[1] * self.zoom + self.oy) as f32,
        )
    }

    /// The same view moved by (dx, dy) drawing units (the tile preview's copies).
    pub fn shifted(&self, dx: f64, dy: f64) -> View {
        View {
            zoom: self.zoom,
            ox: self.ox + dx * self.zoom,
            oy: self.oy + dy * self.zoom,
        }
    }
}

/// A paint as the preview shows it: the symbol's colour, its second colour, a colour, or nothing.
pub fn paint_color(paint: &str, o: &Options) -> Option<Color> {
    match paint {
        "none" | "" => None,
        "fill" => parse_hex(&o.ink),
        "stroke" => parse_hex(&o.second),
        other => parse_hex(other).or_else(|| named(other)),
    }
}

/// The few colour names files keep after import (the importer writes hex otherwise).
fn named(name: &str) -> Option<Color> {
    match name.to_ascii_lowercase().as_str() {
        "black" => Some(Color::BLACK),
        "white" => Some(Color::WHITE),
        "currentcolor" => None,
        _ => None,
    }
}

/// Sub-paths as a path in screen space (`screenPath`).
pub fn subs_path(subs: &[SubPath], view: &View) -> Path {
    Path::new(|b| {
        for sp in subs {
            let n = sp.nodes.len();
            if n == 0 {
                continue;
            }
            b.move_to(view.point(sp.nodes[0].pt()));
            let end = n + usize::from(sp.closed);
            for i in 1..end {
                let a = &sp.nodes[i - 1];
                let c = &sp.nodes[i % n];
                if a.out.is_some() || c.in_.is_some() {
                    b.bezier_curve_to(
                        view.point(a.out.unwrap_or(a.pt())),
                        view.point(c.in_.unwrap_or(c.pt())),
                        view.point(c.pt()),
                    );
                } else {
                    b.line_to(view.point(c.pt()));
                }
            }
            if sp.closed {
                b.close();
            }
        }
    })
}

/// A shape's outline as sub-paths (a text has none).
pub fn outline(s: &Obj) -> Option<Vec<SubPath>> {
    match s.kind() {
        "path" => s.subs().ok(),
        "rect" | "ellipse" => to_path(s).subs().ok(),
        _ => None,
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

/// The stroke a shape draws with at this zoom, or none.
fn stroke_of<'a>(
    s: &Obj,
    o: &Options,
    zoom: f64,
    alpha: f32,
    dash: &'a [f32],
) -> Option<Stroke<'a>> {
    let color = paint_color(s.text("stroke").unwrap_or("none"), o)?;
    let width = s.num("strokeWidth");
    if width.is_nan() || width <= 0.0 {
        return None;
    }
    let path = s.kind() == "path";
    let cap = match s.text("cap").unwrap_or(if path { "round" } else { "butt" }) {
        "round" => LineCap::Round,
        "square" => LineCap::Square,
        _ => LineCap::Butt,
    };
    let join = match s
        .text("join")
        .unwrap_or(if path { "round" } else { "miter" })
    {
        "round" => LineJoin::Round,
        "bevel" => LineJoin::Bevel,
        _ => LineJoin::Miter,
    };
    Some(Stroke {
        style: Style::Solid(with_alpha(color, alpha)),
        width: (width * zoom) as f32,
        line_cap: cap,
        line_join: join,
        line_dash: LineDash {
            segments: dash,
            offset: 0,
        },
    })
}

/// A shape's dash pattern in screen pixels (none for a solid line).
fn dash_of(s: &Obj, zoom: f64) -> Vec<f32> {
    match s.get("dash") {
        kentos_geometry_core::api::json::Json::Arr(d) if !d.is_empty() => d
            .iter()
            .map(|v| match v {
                kentos_geometry_core::api::json::Json::Num(x) => (x * zoom).max(0.0) as f32,
                _ => 0.0,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Paints one shape; `alpha` dims it (ghosts, the tile preview, a draft).
pub fn paint_shape(frame: &mut Frame, s: &Obj, view: &View, o: &Options, alpha: f32) {
    let opacity = s.opt_num("opacity").map_or(1.0, |v| v.clamp(0.0, 1.0)) as f32;
    let alpha = alpha * opacity;
    if s.kind() == "text" {
        paint_text(frame, s, view, o, alpha);
        return;
    }
    let Some(subs) = outline(s) else {
        return;
    };
    let path = subs_path(&subs, view);
    if let Some(c) = paint_color(s.text("fill").unwrap_or("none"), o) {
        let even_odd = match s.text("fillRule") {
            Some("nonzero") => false,
            Some("evenodd") => true,
            // Paths fill even-odd unless told (`elementOf`), the others have no holes.
            _ => s.kind() == "path",
        };
        frame.fill(
            &path,
            Fill {
                style: Style::Solid(with_alpha(c, alpha)),
                rule: if even_odd {
                    fill::Rule::EvenOdd
                } else {
                    fill::Rule::NonZero
                },
            },
        );
    }
    let dash = dash_of(s, view.zoom);
    if let Some(stroke) = stroke_of(s, o, view.zoom, alpha, &dash) {
        frame.stroke(&path, stroke);
    }
}

/// Arimo (Arial's metric twin) puts the top of a one-line box this far above the baseline, in ems.
const BASELINE: f32 = 0.8465;

/// A text shape: its anchor, size, weight and typeface, turned about (x, y).
fn paint_text(frame: &mut Frame, s: &Obj, view: &View, o: &Options, alpha: f32) {
    let size = (s.num("size") * view.zoom) as f32;
    if size.is_nan() || size <= 0.1 {
        return;
    }
    let at = view.point([s.num("x"), s.num("y")]);
    let weight = s.num("weight");
    let font = if s.text("font") == Some("serif") {
        iced::Font {
            family: iced::font::Family::Serif,
            weight: if weight >= 700.0 {
                iced::font::Weight::Bold
            } else {
                iced::font::Weight::Normal
            },
            ..iced::Font::DEFAULT
        }
    } else {
        crate::drawing_fonts::font(
            kentos_contracts::DrawingFont::Arimo,
            if weight >= 700.0 { 700 } else { 400 },
            false,
        )
    };
    let align_x = match s.text("anchor") {
        Some("middle") => iced::advanced::text::Alignment::Center,
        Some("end") => iced::advanced::text::Alignment::Right,
        _ => iced::advanced::text::Alignment::Left,
    };
    let fill = paint_color(s.text("fill").unwrap_or("none"), o);
    let dash = dash_of(s, view.zoom);
    let stroke = stroke_of(s, o, view.zoom, alpha, &dash);
    let text = Text {
        content: s.text("text").unwrap_or("").to_owned(),
        position: Point::new(0.0, -BASELINE * size),
        color: fill.map_or(Color::TRANSPARENT, |c| with_alpha(c, alpha)),
        size: Pixels(size),
        line_height: iced::widget::text::LineHeight::Relative(1.0),
        font,
        align_x,
        align_y: iced::alignment::Vertical::Top,
        ..Text::default()
    };
    let turn = s.opt_num("rotate").unwrap_or(0.0);
    frame.with_save(|f| {
        f.translate(Vector::new(at.x, at.y));
        if turn != 0.0 {
            f.rotate(Radians((turn.to_radians()) as f32));
        }
        // Outlines, not the text layer: a shape drawn later stays over the text.
        if fill.is_some() {
            text.draw_with(|glyph, color| f.fill(&glyph, color));
        }
        if let Some(stroke) = stroke {
            text.draw_with(|glyph, _| f.stroke(&glyph, stroke));
        }
    });
}

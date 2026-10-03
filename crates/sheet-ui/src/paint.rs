//! A display list painted on an Iced canvas frame (design §8): the core laid
//! everything out, so this only turns micrometres into pixels. The same code
//! paints the stage, the gallery's thumbnails and a PNG export.
//!
//! - Paths are flattened here (an arc into chords no farther than a quarter
//!   pixel from it), so a clip can cut them: Iced's software renderer does
//!   not clip canvas geometry, the GPU one does; cutting it ourselves makes
//!   both draw the same. Text is clipped by the frame (both renderers do).
//! - A group's opacity multiplies into its primitives' colours: overlapping
//!   primitives of one group show a little darker where they overlap.
//! - A picture is drawn pixel by pixel: a cell for each pixel of the screen
//!   (or of the file) it covers, its colour the average of the picture's
//!   pixels under it, a cell for each of the picture's own pixels when it is
//!   magnified, runs of one colour as one rectangle. Iced draws no raster in
//!   a canvas without its image codecs (the `image` crate, not in this
//!   build); the cells give the same pixels on both renderers, with no
//!   smoothing when magnified.
//! - A map's content is the host's ([`MapPainter`]): the stage paints maps on
//!   canvases of their own, kept while their view does not change
//!   ([`Plan`]); a thumbnail draws a placeholder; an export paints them in
//!   place.

use std::f64::consts::TAU;

use iced::widget::canvas::{self, Frame, LineCap, LineDash, LineJoin, Path, Text};
use iced::widget::text::{Alignment, LineHeight, Shaping};
use iced::{Color, Pixels, Point, Radians, Rectangle, Size, Vector, alignment};
use kentos_sheet::display::{ArcSeg, DisplayList, MapPrim, Prim, Seg, TextPrim};
use kentos_sheet::style::{self as sheet_style, Stroke, TextStyle};
use kentos_sheet::units::{Mdeg, PointUm, RectUm, Um};

use crate::fonts;
use crate::painter::{MapPainter, MapRequest};
use crate::pictures::Raster;

/// Micrometres on the paper to pixels: `px = origin + µm × k`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xf {
    /// Pixels per micrometre.
    pub k: f64,
    /// Where the paper's top-left corner is, in the frame's pixels.
    pub origin: Point,
}

impl Xf {
    pub fn new(k: f64, origin: Point) -> Self {
        Xf { k, origin }
    }

    pub fn p(&self, x: f64, y: f64) -> Point {
        Point::new(
            (f64::from(self.origin.x) + x * self.k) as f32,
            (f64::from(self.origin.y) + y * self.k) as f32,
        )
    }

    pub fn pt(&self, p: PointUm) -> Point {
        self.p(f64::from(p[0]), f64::from(p[1]))
    }

    pub fn len(&self, um: f64) -> f32 {
        (um * self.k) as f32
    }

    pub fn rect(&self, r: &RectUm) -> Rectangle {
        let a = self.p(f64::from(r.left), f64::from(r.top));
        Rectangle::new(
            a,
            Size::new(self.len(f64::from(r.width)), self.len(f64::from(r.height))),
        )
    }

    /// A pixel's place on the paper, micrometres.
    pub fn paper(&self, px: Point) -> [f64; 2] {
        [
            (f64::from(px.x) - f64::from(self.origin.x)) / self.k,
            (f64::from(px.y) - f64::from(self.origin.y)) / self.k,
        ]
    }

    /// Pixels per millimetre.
    pub fn px_per_mm(&self) -> f64 {
        self.k * 1000.0
    }
}

/// How the paint is done: the thinnest line (the stage keeps hairlines visible; an export draws them as they are).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    pub min_line: f32,
}

impl Options {
    pub const SCREEN: Options = Options { min_line: 0.75 };
    pub const EXPORT: Options = Options { min_line: 0.0 };
}

/// A `#rrggbb` or `#rrggbbaa` colour (a 3-digit `#rgb` too); none for anything else (`none`, a name).
pub fn color(s: &str) -> Option<Color> {
    let h = s.strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        6 => Some(Color::from_rgb8(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color::from_rgba8(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            f32::from(byte(6)?) / 255.0,
        )),
        3 => {
            let d = |i: usize| {
                u8::from_str_radix(h.get(i..i + 1)?, 16)
                    .ok()
                    .map(|v| v * 17)
            };
            Some(Color::from_rgb8(d(0)?, d(1)?, d(2)?))
        }
        _ => None,
    }
}

fn faded(c: Color, alpha: f32) -> Color {
    Color {
        a: c.a * alpha,
        ..c
    }
}

// ── The plan: the list cut at its maps ───────────────────────────────────

/// A primitive to paint, with the clip it is in and its groups' opacity.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub prim: usize,
    pub clip: Option<ClipBox>,
    pub alpha: f32,
}

/// A clip on the paper: a rectangle and its turn about its own middle (a
/// turned map frame's box, a turned picture's), as the core pushes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipBox {
    pub rect: RectUm,
    pub rotation: Mdeg,
}

/// A clip on the screen: its rectangle before the turn and the turn about the
/// rectangle's middle (radians, clockwise). Neither renderer cuts a canvas's
/// shapes to a turned box, so lines and fills are cut here: taken into the
/// clip's own upright frame, cut to its rectangle, turned back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Clip {
    pub(crate) rect: Rectangle,
    pub(crate) turn: f32,
}

impl Clip {
    fn of(c: &ClipBox, xf: &Xf) -> Clip {
        Clip {
            rect: xf.rect(&c.rect),
            turn: rad(c.rotation) as f32,
        }
    }

    fn middle(&self) -> Point {
        Point::new(
            self.rect.x + self.rect.width / 2.0,
            self.rect.y + self.rect.height / 2.0,
        )
    }

    /// `p` turned by `turn` about the clip's middle.
    fn turned(&self, p: Point, turn: f32) -> Point {
        let c = self.middle();
        let (s, k) = turn.sin_cos();
        let (dx, dy) = (p.x - c.x, p.y - c.y);
        Point::new(c.x + dx * k - dy * s, c.y + dx * s + dy * k)
    }

    /// The part of a line inside the clip.
    pub(crate) fn line(&self, a: Point, b: Point) -> Option<(Point, Point)> {
        if self.turn == 0.0 {
            return clip_line(a, b, Some(self.rect));
        }
        let (a, b) = (self.turned(a, -self.turn), self.turned(b, -self.turn));
        clip_line(a, b, Some(self.rect))
            .map(|(p, q)| (self.turned(p, self.turn), self.turned(q, self.turn)))
    }

    /// A closed polygon cut to the clip.
    pub(crate) fn polygon(&self, pts: &[Point]) -> Vec<Point> {
        if self.turn == 0.0 {
            return clip_polygon(pts, self.rect);
        }
        let upright: Vec<Point> = pts.iter().map(|p| self.turned(*p, -self.turn)).collect();
        clip_polygon(&upright, self.rect)
            .into_iter()
            .map(|p| self.turned(p, self.turn))
            .collect()
    }

    /// The upright box around the clip: what a renderer's own clip can do
    /// (text is cut by the renderer, to this box).
    pub(crate) fn bounds(&self) -> Rectangle {
        if self.turn == 0.0 {
            return self.rect;
        }
        let r = self.rect;
        let corners = [
            Point::new(r.x, r.y),
            Point::new(r.x + r.width, r.y),
            Point::new(r.x + r.width, r.y + r.height),
            Point::new(r.x, r.y + r.height),
        ]
        .map(|p| self.turned(p, self.turn));
        let (x0, y0, x1, y1) = corners.iter().fold(
            (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
            |(x0, y0, x1, y1), p| (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y)),
        );
        Rectangle::new(Point::new(x0, y0), Size::new(x1 - x0, y1 - y0))
    }

    /// Whether a point is inside the clip.
    pub(crate) fn holds(&self, p: Point) -> bool {
        self.rect.contains(self.turned(p, -self.turn))
    }
}

/// The paper's primitives between its maps: `layers[0]` under the first
/// map, `layers[i]` between map `i - 1` and map `i`, the last over every
/// map. The stage paints each layer on its own canvas and each map on one
/// of its own between them, so a map's content keeps its place in the
/// drawing order and is painted again only when its own view changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub layers: Vec<Vec<Entry>>,
    pub maps: Vec<Entry>,
}

/// The intersection of two clip rectangles (an empty one when they do not meet).
fn meet(a: &RectUm, b: &RectUm) -> RectUm {
    let l = i64::from(a.left).max(i64::from(b.left));
    let t = i64::from(a.top).max(i64::from(b.top));
    let r = a.right().min(b.right()).max(l);
    let bo = a.bottom().min(b.bottom()).max(t);
    RectUm::from_edges(l, t, r, bo)
}

/// The list's plan: every primitive with its clip and opacity, cut at the maps and the
/// pictures. A map or a picture stands between two layers (`maps`, after the layer of its
/// index): a renderer draws a canvas's pictures over its shapes, so a picture drawn in a layer
/// of its own keeps its place in the drawing order (what comes after it is over it).
pub fn plan(list: &DisplayList) -> Plan {
    let mut out = Plan {
        layers: vec![Vec::new()],
        maps: Vec::new(),
    };
    // The clips in force: upright ones meet; a turned one (a turned map's or picture's box) is
    // the innermost turned clip, whole (two turned boxes do not meet in a rectangle; the core
    // does not nest them).
    let mut clips: Vec<ClipBox> = Vec::new();
    let mut alphas: Vec<f32> = Vec::new();
    for (i, p) in list.prims.iter().enumerate() {
        let clip = match clips.iter().rev().find(|c| c.rotation % 360_000 != 0) {
            Some(turned) => Some(*turned),
            None => clips
                .iter()
                .map(|c| c.rect)
                .reduce(|a, c| meet(&a, &c))
                .map(|rect| ClipBox { rect, rotation: 0 }),
        };
        let alpha = alphas.iter().product::<f32>();
        match p {
            Prim::PushClip(c) => clips.push(ClipBox {
                rect: c.rect,
                rotation: c.rotation,
            }),
            Prim::PopClip(_) => {
                clips.pop();
            }
            Prim::PushGroup(g) => alphas.push(f32::from(g.opacity.min(100)) / 100.0),
            Prim::PopGroup(_) => {
                alphas.pop();
            }
            Prim::Map(_) | Prim::Image(_) => {
                out.maps.push(Entry {
                    prim: i,
                    clip,
                    alpha,
                });
                out.layers.push(Vec::new());
            }
            _ => {
                if let Some(layer) = out.layers.last_mut() {
                    layer.push(Entry {
                        prim: i,
                        clip,
                        alpha,
                    });
                }
            }
        }
    }
    out
}

// ── Painting ─────────────────────────────────────────────────────────────

/// What a map frame is filled with when it is painted in place.
#[derive(Clone, Copy)]
pub enum Maps<'a> {
    /// Nothing (the stage paints maps on canvases of their own).
    Skip,
    /// A light box saying “Harita” (a thumbnail, a frame with no place yet).
    Placeholder,
    /// The host's content; `export` asks for the printed look.
    Painter(&'a dyn MapPainter, bool),
}

/// The pictures of the book, decoded.
pub type Pictures<'a> = &'a dyn Fn(&str) -> Option<std::rc::Rc<Raster>>;

/// The paper itself: its colour over the whole sheet.
pub fn paper(frame: &mut Frame, list: &DisplayList, xf: &Xf) {
    let r = xf.rect(&RectUm::new(0, 0, list.size.width, list.size.height));
    let c = color(&list.paper).unwrap_or(Color::WHITE);
    frame.fill_rectangle(r.position(), r.size(), c);
}

/// Paints the whole list in order: the paper, then every primitive.
pub fn paint_all(
    frame: &mut Frame,
    list: &DisplayList,
    xf: &Xf,
    pictures: Pictures<'_>,
    maps: Maps<'_>,
    opts: Options,
) {
    paper(frame, list, xf);
    let plan = plan(list);
    for (i, layer) in plan.layers.iter().enumerate() {
        paint_entries(frame, list, layer, xf, pictures, opts);
        let Some(m) = plan.maps.get(i) else {
            continue;
        };
        match list.prims.get(m.prim) {
            Some(Prim::Map(mp)) => paint_map_in_place(frame, mp, xf, maps, m.alpha),
            Some(Prim::Image(_)) => {
                paint_entries(frame, list, std::slice::from_ref(m), xf, pictures, opts);
            }
            _ => {}
        }
    }
}

/// Paints some of the list's primitives (a layer of a [`Plan`]).
pub fn paint_entries(
    frame: &mut Frame,
    list: &DisplayList,
    entries: &[Entry],
    xf: &Xf,
    pictures: Pictures<'_>,
    opts: Options,
) {
    for e in entries {
        let Some(p) = list.prims.get(e.prim) else {
            continue;
        };
        let clip = e.clip.map(|c| Clip::of(&c, xf));
        if clip.is_some_and(|c| c.rect.width <= 0.0 || c.rect.height <= 0.0) {
            continue;
        }
        paint_prim(frame, p, xf, clip, e.alpha, pictures, opts);
    }
}

fn paint_prim(
    frame: &mut Frame,
    p: &Prim,
    xf: &Xf,
    clip: Option<Clip>,
    alpha: f32,
    pictures: Pictures<'_>,
    opts: Options,
) {
    match p {
        Prim::Rect(r) => {
            let polys = vec![rect_poly(&r.rect, r.rotation, r.radius, xf)];
            draw_polys(
                frame,
                &polys,
                r.fill.as_deref(),
                r.stroke.as_ref(),
                false,
                xf,
                clip,
                alpha,
                opts,
            );
        }
        Prim::Path(p) => {
            let polys = flatten(&p.segments, xf);
            draw_polys(
                frame,
                &polys,
                p.fill.as_deref(),
                p.stroke.as_ref(),
                p.even_odd,
                xf,
                clip,
                alpha,
                opts,
            );
        }
        Prim::Text(t) => text(frame, t, xf, clip, alpha),
        Prim::Image(img) => {
            let rect = xf.rect(&img.rect);
            let opacity = alpha * f32::from(img.opacity.min(100)) / 100.0;
            match pictures(&img.asset) {
                Some(raster) => picture(frame, &raster, rect, img.rotation, opacity, clip),
                None => missing_picture(frame, rect, img.rotation, opacity),
            }
        }
        Prim::Map(_)
        | Prim::PushClip(_)
        | Prim::PopClip(_)
        | Prim::PushGroup(_)
        | Prim::PopGroup(_) => {}
    }
}

/// Straight RGBA pixels `width` × `height`, row by row, drawn over `rect` smoothed: a picture a
/// map's painter drew itself (a map whose drawing has something no vector writes). A renderer
/// draws a canvas's pictures over its shapes and under its text.
pub fn draw_pixels(frame: &mut Frame, rect: Rectangle, width: u32, height: u32, rgba: Vec<u8>) {
    if width == 0 || height == 0 || rgba.len() != width as usize * height as usize * 4 {
        return;
    }
    frame.draw_image(
        rect,
        canvas::Image::new(iced::advanced::image::Handle::from_rgba(
            width, height, rgba,
        ))
        .filter_method(iced::advanced::image::FilterMethod::Linear),
    );
}

/// A map frame painted in the frame itself (a thumbnail, an export).
pub fn paint_map_in_place(frame: &mut Frame, m: &MapPrim, xf: &Xf, maps: Maps<'_>, alpha: f32) {
    let rect = xf.rect(&m.clip);
    if rect.width < 1.0 || rect.height < 1.0 {
        return;
    }
    match maps {
        Maps::Skip => {}
        Maps::Placeholder => placeholder(frame, rect, alpha),
        Maps::Painter(painter, export) => {
            let request = MapRequest::of(m, rect.size(), xf.k, export);
            // The painter keeps to the frame's own box; a turned frame reaches past the upright one.
            let around = xf.rect(&kentos_sheet::units::rotated_bounds(&m.clip, m.rotation));
            let painted = frame.with_clip(around, |f| {
                f.translate(Vector::new(rect.x, rect.y));
                if m.rotation != 0 {
                    turn_about(f, rect.size(), m.rotation);
                }
                request.view.center.is_some() && painter.paint(&request, f)
            });
            if !painted {
                placeholder(frame, rect, alpha);
            }
        }
    }
}

/// Turns the frame about the middle of a box of `size` at its origin.
pub(crate) fn turn_about(f: &mut Frame, size: Size, rotation: Mdeg) {
    let c = Vector::new(size.width / 2.0, size.height / 2.0);
    f.translate(c);
    f.rotate(Radians(rad(rotation) as f32));
    f.translate(Vector::new(-c.x, -c.y));
}

/// The light box of a map with no content: the web's and the SVG's “harita”.
pub fn placeholder(frame: &mut Frame, rect: Rectangle, alpha: f32) {
    frame.fill_rectangle(
        rect.position(),
        rect.size(),
        faded(Color::from_rgb8(0xee, 0xf1, 0xf4), alpha),
    );
    let ink = faded(Color::from_rgb8(0x9a, 0xa4, 0xb0), alpha);
    let step = 14.0_f32;
    let mut lines = Vec::new();
    let mut t = -rect.height;
    while t < rect.width {
        lines.push([
            Point::new(rect.x + t, rect.y + rect.height),
            Point::new(rect.x + t + rect.height, rect.y),
        ]);
        t += step;
    }
    let clip = Some(rect);
    let path = Path::new(|b| {
        for [a, c] in &lines {
            if let Some((a, c)) = clip_line(*a, *c, clip) {
                b.move_to(a);
                b.line_to(c);
            }
        }
    });
    frame.stroke(
        &path,
        canvas::Stroke {
            width: 1.0,
            ..canvas::Stroke::default().with_color(faded(Color::from_rgb8(0xe2, 0xe7, 0xec), alpha))
        },
    );
    if rect.width > 48.0 && rect.height > 20.0 {
        frame.fill_text(Text {
            content: "Harita".to_owned(),
            position: rect.center(),
            color: ink,
            size: Pixels((rect.height * 0.12).clamp(9.0, 16.0)),
            font: fonts::font("barlow", 500, false),
            align_x: Alignment::Center,
            align_y: alignment::Vertical::Center,
            ..Text::default()
        });
    }
}

// ── Geometry ─────────────────────────────────────────────────────────────

/// A flattened subpath in pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Poly {
    pub pts: Vec<Point>,
    pub closed: bool,
}

pub fn rad(a: Mdeg) -> f64 {
    f64::from(a) / 1000.0 * std::f64::consts::PI / 180.0
}

/// A rectangle turned about its centre, its corners rounded by `radius`.
fn rect_poly(r: &RectUm, rotation: Mdeg, radius: Um, xf: &Xf) -> Poly {
    let c = r.center();
    let (l, t, rr, b) = (
        f64::from(r.left),
        f64::from(r.top),
        r.right() as f64,
        r.bottom() as f64,
    );
    let rad_um = f64::from(radius.max(0)).min(f64::from(r.width.min(r.height)) / 2.0);
    let mut local: Vec<[f64; 2]> = Vec::new();
    if rad_um <= 0.0 {
        local.extend([[l, t], [rr, t], [rr, b], [l, b]]);
    } else {
        // Clockwise on the paper from the top edge: each corner's quarter circle.
        let corners = [
            ([rr - rad_um, t + rad_um], -90.0_f64),
            ([rr - rad_um, b - rad_um], 0.0),
            ([l + rad_um, b - rad_um], 90.0),
            ([l + rad_um, t + rad_um], 180.0),
        ];
        let n = ((rad_um * xf.k).sqrt().ceil() as usize).clamp(2, 16);
        for (cc, a0) in corners {
            for i in 0..=n {
                let a = (a0 + 90.0 * i as f64 / n as f64).to_radians();
                local.push([cc[0] + rad_um * a.cos(), cc[1] + rad_um * a.sin()]);
            }
        }
    }
    let (s, co) = (rad(rotation).sin(), rad(rotation).cos());
    let pts = local
        .into_iter()
        .map(|p| {
            if rotation == 0 {
                xf.p(p[0], p[1])
            } else {
                let (x, y) = (p[0] - c[0], p[1] - c[1]);
                xf.p(c[0] + x * co - y * s, c[1] + x * s + y * co)
            }
        })
        .collect();
    Poly { pts, closed: true }
}

/// The points of an arc, its start first (the path draws a line to it first, as Canvas does).
fn arc_points(a: &ArcSeg, xf: &Xf) -> Vec<Point> {
    let (cx, cy) = (f64::from(a.center[0]), f64::from(a.center[1]));
    let (rx, ry) = (f64::from(a.radius[0]), f64::from(a.radius[1]));
    let (s, c) = (rad(a.rotation).sin(), rad(a.rotation).cos());
    let start = rad(a.start);
    let sweep = rad(a.sweep);
    let r_px = rx.max(ry) * xf.k;
    // Chords at most a quarter pixel from the curve: θ = 2·acos(1 − e/r).
    let step = if r_px > 0.25 {
        2.0 * (1.0 - 0.25 / r_px).acos()
    } else {
        TAU
    };
    let n = ((sweep.abs() / step.max(1e-3)).ceil() as usize).clamp(1, 720);
    (0..=n)
        .map(|i| {
            let th = start + sweep * i as f64 / n as f64;
            let (x, y) = (rx * th.cos(), ry * th.sin());
            xf.p(cx + x * c - y * s, cy + x * s + y * c)
        })
        .collect()
}

/// A path's segments as subpaths in pixels.
pub(crate) fn flatten(segments: &[Seg], xf: &Xf) -> Vec<Poly> {
    let mut out: Vec<Poly> = Vec::new();
    let mut cur = Poly::default();
    for s in segments {
        match s {
            Seg::M(p) => {
                if cur.pts.len() > 1 || cur.closed {
                    out.push(std::mem::take(&mut cur));
                }
                cur = Poly {
                    pts: vec![xf.pt(*p)],
                    closed: false,
                };
            }
            Seg::L(p) => cur.pts.push(xf.pt(*p)),
            Seg::A(a) => cur.pts.extend(arc_points(a, xf)),
            Seg::Z => {
                cur.closed = true;
                let start = cur.pts.first().copied();
                out.push(std::mem::take(&mut cur));
                // A path may go on from where a closed one started.
                if let Some(p) = start {
                    cur.pts.push(p);
                }
            }
        }
    }
    if cur.pts.len() > 1 {
        out.push(cur);
    }
    out
}

/// Liang–Barsky: the part of a line inside the rectangle.
pub(crate) fn clip_line(a: Point, b: Point, clip: Option<Rectangle>) -> Option<(Point, Point)> {
    let Some(r) = clip else {
        return Some((a, b));
    };
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (mut t0, mut t1) = (0.0_f32, 1.0_f32);
    for (p, q) in [
        (-dx, a.x - r.x),
        (dx, r.x + r.width - a.x),
        (-dy, a.y - r.y),
        (dy, r.y + r.height - a.y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                if t > t1 {
                    return None;
                }
                t0 = t0.max(t);
            } else {
                if t < t0 {
                    return None;
                }
                t1 = t1.min(t);
            }
        }
    }
    Some((
        Point::new(a.x + t0 * dx, a.y + t0 * dy),
        Point::new(a.x + t1 * dx, a.y + t1 * dy),
    ))
}

/// A side of the clip: which points are inside it, and where a line crosses it.
type ClipEdge = (
    fn(Point, Rectangle) -> bool,
    fn(Point, Point, Rectangle) -> Point,
);

/// Sutherland–Hodgman: a closed polygon cut to the rectangle.
pub(crate) fn clip_polygon(pts: &[Point], r: Rectangle) -> Vec<Point> {
    let edges: [ClipEdge; 4] = [
        (|p, r| p.x >= r.x, |a, b, r| cut_x(a, b, r.x)),
        (
            |p, r| p.x <= r.x + r.width,
            |a, b, r| cut_x(a, b, r.x + r.width),
        ),
        (|p, r| p.y >= r.y, |a, b, r| cut_y(a, b, r.y)),
        (
            |p, r| p.y <= r.y + r.height,
            |a, b, r| cut_y(a, b, r.y + r.height),
        ),
    ];
    let mut poly = pts.to_vec();
    for (inside, cut) in edges {
        if poly.is_empty() {
            break;
        }
        let input = std::mem::take(&mut poly);
        let mut prev = input[input.len() - 1];
        for &p in &input {
            match (inside(p, r), inside(prev, r)) {
                (true, true) => poly.push(p),
                (true, false) => {
                    poly.push(cut(prev, p, r));
                    poly.push(p);
                }
                (false, true) => poly.push(cut(prev, p, r)),
                (false, false) => {}
            }
            prev = p;
        }
    }
    poly
}

fn cut_x(a: Point, b: Point, x: f32) -> Point {
    let t = if (b.x - a.x).abs() > f32::EPSILON {
        (x - a.x) / (b.x - a.x)
    } else {
        0.0
    };
    Point::new(x, a.y + t * (b.y - a.y))
}

fn cut_y(a: Point, b: Point, y: f32) -> Point {
    let t = if (b.y - a.y).abs() > f32::EPSILON {
        (y - a.y) / (b.y - a.y)
    } else {
        0.0
    };
    Point::new(a.x + t * (b.x - a.x), y)
}

pub(crate) fn stroke_of<'a>(
    s: &Stroke,
    xf: &Xf,
    alpha: f32,
    dash: &'a [f32],
    opts: Options,
) -> Option<canvas::Stroke<'a>> {
    let c = faded(color(&s.color)?, alpha);
    let width = xf.len(f64::from(s.width)).max(opts.min_line);
    Some(canvas::Stroke {
        width,
        line_cap: match s.cap {
            sheet_style::LineCap::Butt => LineCap::Butt,
            sheet_style::LineCap::Round => LineCap::Round,
            sheet_style::LineCap::Square => LineCap::Square,
        },
        line_join: match s.join {
            sheet_style::LineJoin::Miter => LineJoin::Miter,
            sheet_style::LineJoin::Round => LineJoin::Round,
            sheet_style::LineJoin::Bevel => LineJoin::Bevel,
        },
        line_dash: LineDash {
            segments: dash,
            offset: 0,
        },
        ..canvas::Stroke::default().with_color(c)
    })
}

#[allow(clippy::too_many_arguments)]
fn draw_polys(
    frame: &mut Frame,
    polys: &[Poly],
    fill: Option<&str>,
    stroke: Option<&Stroke>,
    even_odd: bool,
    xf: &Xf,
    clip: Option<Clip>,
    alpha: f32,
    opts: Options,
) {
    if let Some(fc) = fill.and_then(color) {
        let rings: Vec<Vec<Point>> = polys
            .iter()
            .filter(|p| p.pts.len() > 2)
            .map(|p| match clip {
                Some(c) => c.polygon(&p.pts),
                None => p.pts.clone(),
            })
            .filter(|r| r.len() > 2)
            .collect();
        if !rings.is_empty() {
            let path = Path::new(|b| {
                for r in &rings {
                    b.move_to(r[0]);
                    for p in &r[1..] {
                        b.line_to(*p);
                    }
                    b.close();
                }
            });
            frame.fill(
                &path,
                canvas::Fill {
                    style: canvas::Style::Solid(faded(fc, alpha)),
                    rule: if even_odd {
                        canvas::fill::Rule::EvenOdd
                    } else {
                        canvas::fill::Rule::NonZero
                    },
                },
            );
        }
    }
    if let Some(s) = stroke {
        let dash: Vec<f32> = s
            .dash
            .iter()
            .map(|d| xf.len(f64::from(*d)).max(0.5))
            .collect();
        let Some(st) = stroke_of(s, xf, alpha, &dash, opts) else {
            return;
        };
        let path = Path::new(|b| {
            for p in polys {
                let mut pts = p.pts.clone();
                if p.closed && pts.len() > 1 {
                    pts.push(pts[0]);
                }
                if clip.is_none() {
                    if let Some(first) = pts.first() {
                        b.move_to(*first);
                        for q in &pts[1..] {
                            b.line_to(*q);
                        }
                        if p.closed {
                            b.close();
                        }
                    }
                    continue;
                }
                // Cut: each piece inside the clip is its own run.
                let mut last: Option<Point> = None;
                for w in pts.windows(2) {
                    if let Some((a, c)) = clip.and_then(|k| k.line(w[0], w[1])) {
                        if last != Some(a) {
                            b.move_to(a);
                        }
                        b.line_to(c);
                        last = Some(c);
                    } else {
                        last = None;
                    }
                }
            }
        });
        frame.stroke(&path, st);
    }
}

// ── Text ─────────────────────────────────────────────────────────────────

/// A face's ascent and descent as fractions of the em (the core's own table).
fn em_metrics(font: &str, weight: u16, italic: bool) -> (f64, f64) {
    let style = TextStyle {
        font: font.to_owned(),
        size: 1000,
        weight,
        italic,
        color: String::new(),
    };
    match kentos_sheet::text::face(&style) {
        Some(f) if f.units_per_em > 0 => {
            let upm = f64::from(f.units_per_em);
            (f64::from(f.ascent) / upm, f64::from(f.descent.abs()) / upm)
        }
        _ => (0.9, 0.25),
    }
}

fn text(frame: &mut Frame, t: &TextPrim, xf: &Xf, clip: Option<Clip>, alpha: f32) {
    let size = xf.len(f64::from(t.size));
    if size < 0.6 || t.text.is_empty() {
        return;
    }
    let Some(c) = color(&t.color) else {
        return;
    };
    let at = xf.pt(t.at);
    // The pieces the drawing's faces draw, as the PDF writes them (design §6): a letter the
    // face lacks in the face that has it, each piece where the core's widths put it.
    let runs = kentos_sheet::text::text_runs(&t.font, t.weight, t.italic, &t.text);
    let mut x = 0.0f32;
    let mut bodies: Vec<Text> = Vec::with_capacity(runs.len().max(1));
    for r in &runs {
        let (asc, desc) = em_metrics(&r.font, r.weight, r.italic);
        bodies.push(Text {
            content: r.text.clone(),
            position: Point::new(x, -(asc as f32) * size),
            color: faded(c, alpha),
            size: Pixels(size),
            line_height: LineHeight::Absolute(Pixels(((asc + desc) as f32 * size).max(1.0))),
            font: fonts::font(&r.font, r.weight, r.italic),
            align_x: Alignment::Left,
            align_y: alignment::Vertical::Top,
            shaping: Shaping::Advanced,
            max_width: f32::INFINITY,
        });
        let style = TextStyle {
            font: r.font.clone(),
            size: t.size,
            weight: r.weight,
            italic: r.italic,
            color: String::new(),
        };
        x += xf.len(kentos_sheet::text::text_width(&r.text, &style) as f64);
    }
    let halo = t.halo.as_deref().and_then(color).map(|h| faded(h, alpha));
    let draw = |f: &mut Frame| {
        f.with_save(|f| {
            f.translate(Vector::new(at.x, at.y));
            if t.rotation != 0 {
                f.rotate(Radians(rad(t.rotation) as f32));
            }
            if let Some(h) = halo {
                let d = (size * 0.09).clamp(0.5, 2.0);
                for body in &bodies {
                    for (dx, dy) in [
                        (-d, 0.0),
                        (d, 0.0),
                        (0.0, -d),
                        (0.0, d),
                        (-d, -d),
                        (d, d),
                        (-d, d),
                        (d, -d),
                    ] {
                        f.fill_text(Text {
                            position: Point::new(body.position.x + dx, body.position.y + dy),
                            color: h,
                            ..body.clone()
                        });
                    }
                }
            }
            for body in &bodies {
                f.fill_text(body.clone());
            }
        });
    };
    match clip {
        // A turned box: a text whose foot lies outside it is left out; the renderer cuts the
        // rest to the box's upright bounds.
        Some(c) if c.turn != 0.0 && !c.holds(at) => {}
        Some(c) => frame.with_clip(c.bounds(), draw),
        None => draw(frame),
    }
}

// ── Pictures ─────────────────────────────────────────────────────────────

/// A picture whose bytes are not here: a light box with a cross.
fn missing_picture(frame: &mut Frame, rect: Rectangle, rotation: Mdeg, alpha: f32) {
    frame.with_save(|f| {
        f.translate(Vector::new(rect.x, rect.y));
        if rotation != 0 {
            turn_about(f, rect.size(), rotation);
        }
        let r = Rectangle::new(Point::ORIGIN, rect.size());
        f.fill_rectangle(
            r.position(),
            r.size(),
            faded(Color::from_rgb8(0xf1, 0xf3, 0xf5), alpha),
        );
        let ink = faded(Color::from_rgb8(0xb0, 0xb8, 0xc2), alpha);
        let cross = Path::new(|b| {
            b.move_to(Point::ORIGIN);
            b.line_to(Point::new(r.width, r.height));
            b.move_to(Point::new(r.width, 0.0));
            b.line_to(Point::new(0.0, r.height));
            b.rectangle(Point::ORIGIN, r.size());
        });
        f.stroke(
            &cross,
            canvas::Stroke::default().with_color(ink).with_width(1.0),
        );
    });
}

/// A picture as a texture, smoothed when magnified and drawn from the halving near its size when
/// small (design, open question 6), or an SVG one drawn by resvg at the size shown; turned about
/// its middle, at its opacity, inside the clip's box. A renderer draws a canvas's pictures over
/// its shapes: the plan gives each picture a layer of its own ([`plan`]). A magnified picture
/// is drawn from a texture magnified here by a whole factor ([`Raster::texture_for`]): the
/// software renderer (a PNG export's, the screenshots') places a picture only to its texture's
/// pixels, so the screen and a PNG agree.
fn picture(
    frame: &mut Frame,
    raster: &Raster,
    rect: Rectangle,
    rotation: Mdeg,
    alpha: f32,
    clip: Option<Clip>,
) {
    if rect.width < 0.5 || rect.height < 0.5 {
        return;
    }
    let turn = Radians(rad(rotation) as f32);
    let draw = |f: &mut Frame| {
        if let Some(svg) = raster.svg() {
            f.draw_svg(
                rect,
                iced::advanced::svg::Svg::new(svg.clone())
                    .rotation(turn)
                    .opacity(alpha),
            );
        } else if let Some(t) = raster.texture_for(rect.width, rect.height) {
            f.draw_image(
                rect,
                canvas::Image::new(t)
                    .filter_method(iced::advanced::image::FilterMethod::Linear)
                    .rotation(turn)
                    .opacity(alpha),
            );
        }
    };
    match clip {
        Some(c) => frame.with_clip(c.bounds(), draw),
        None => draw(frame),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One drawing on a white canvas `w` × `h`, rendered by the software renderer (a PNG
    /// export's): its pixels, row by row, RGB.
    fn shot(w: usize, h: usize, draw: impl Fn(&mut Frame)) -> impl Fn(usize, usize) -> [u8; 3] {
        struct Draw<F>(F);
        impl<F: Fn(&mut Frame)> canvas::Program<()> for Draw<F> {
            type State = ();
            fn draw(
                &self,
                _: &(),
                renderer: &iced::Renderer,
                _: &iced::Theme,
                b: Rectangle,
                _: iced::mouse::Cursor,
            ) -> Vec<canvas::Geometry> {
                let mut f = Frame::new(renderer, b.size());
                f.fill_rectangle(Point::ORIGIN, b.size(), Color::WHITE);
                (self.0)(&mut f);
                vec![f.into_geometry()]
            }
        }
        let view: iced::Element<'_, ()> = iced::widget::canvas(Draw(draw))
            .width(iced::Fill)
            .height(iced::Fill)
            .into();
        let image = kentos_ui::snapshot::Snapshot::software(Size::new(w as f32, h as f32))
            .expect("a renderer")
            .render(view, &iced::Theme::Light);
        assert_eq!((image.width as usize, image.height as usize), (w, h));
        move |x, y| {
            let i = (y * w + x) * 4;
            [image.rgba[i], image.rgba[i + 1], image.rgba[i + 2]]
        }
    }

    fn near(a: [u8; 3], b: [u8; 3], by: u8) -> bool {
        a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= by)
    }

    /// A picture is a texture (design, open question 6): magnified, smoothed — across the edge
    /// between two of its pixels the colours blend, no squares — and shrunk, drawn from the
    /// halving near its size: a fine checker comes out its average grey.
    #[test]
    fn a_picture_is_smoothed_when_magnified_and_averaged_when_shrunk() {
        // 4 × 4: four quadrants of colour.
        let quads = [
            [200u8, 30, 30],
            [30, 160, 60],
            [40, 60, 200],
            [250, 200, 20],
        ];
        let mut rgba = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                let q = quads[(y / 2) * 2 + x / 2];
                rgba.extend_from_slice(&[q[0], q[1], q[2], 255]);
            }
        }
        let small = Raster::from_rgba(4, 4, rgba).expect("a picture");
        // 64 × 64 one-pixel checker.
        let mut checker = Vec::new();
        for y in 0..64 {
            for x in 0..64 {
                let v = if (x + y) % 2 == 0 { 0 } else { 255 };
                checker.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let checker = Raster::from_rgba(64, 64, checker).expect("a picture");
        let at = shot(60, 40, |f| {
            let square = |x, w| Rectangle::new(Point::new(x, 0.0), Size::new(w, w));
            picture(f, &small, square(0.0, 40.0), 0, 1.0, None);
            picture(f, &checker, square(50.0, 8.0), 0, 1.0, None);
        });
        // Inside each quadrant its own colour.
        for (q, (x, y)) in [(2, 2), (37, 2), (2, 37), (37, 37)].into_iter().enumerate() {
            assert!(near(at(x, y), quads[q], 4), "({x}, {y}): {:?}", at(x, y));
        }
        // On the edge between the first two: between them, neither (smoothed, not squares).
        let edge = at(20, 4);
        assert!(
            !near(edge, quads[0], 30) && !near(edge, quads[1], 30),
            "{edge:?}"
        );
        for c in 0..3 {
            let (a, b) = (quads[0][c].min(quads[1][c]), quads[0][c].max(quads[1][c]));
            assert!((a..=b).contains(&edge[c]), "{edge:?}");
        }
        // The checker shrunk eightfold: grey all over.
        for x in 51..57 {
            for y in 1..7 {
                let p = at(x, y);
                assert!(
                    p.iter().all(|v| (108..=148).contains(v)),
                    "({x}, {y}): {p:?}"
                );
            }
        }
    }

    /// No cap on a picture's detail: a picture of a thousand one-pixel stripes drawn at its own
    /// size keeps every stripe (the cells of before stopped at 400 000 and made it grey).
    #[test]
    fn a_large_picture_keeps_its_detail() {
        let (w, h) = (1000usize, 500usize);
        let mut rgba = Vec::with_capacity(w * h * 4);
        for _y in 0..h {
            for x in 0..w {
                let v = if x % 2 == 0 { 0 } else { 255 };
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let stripes = Raster::from_rgba(w as u32, h as u32, rgba).expect("a picture");
        let at = shot(w, h, |f| {
            picture(
                f,
                &stripes,
                Rectangle::new(Point::ORIGIN, Size::new(w as f32, h as f32)),
                0,
                1.0,
                None,
            );
        });
        for x in [100, 101, 500, 501, 998, 999] {
            let want = if x % 2 == 0 {
                [0, 0, 0]
            } else {
                [255, 255, 255]
            };
            assert!(near(at(x, 250), want, 24), "{x}: {:?}", at(x, 250));
        }
    }

    /// An SVG picture is drawn (by resvg), at the size it is shown.
    #[test]
    fn an_svg_picture_is_drawn() {
        let svg = crate::pictures::decode(
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10"><rect x="0" y="0" width="5" height="10" fill="#c00000"/></svg>"##,
        )
        .expect("an SVG");
        let at = shot(60, 40, |f| {
            picture(
                f,
                &svg,
                Rectangle::new(Point::new(10.0, 0.0), Size::new(40.0, 40.0)),
                0,
                1.0,
                None,
            );
        });
        assert!(near(at(15, 20), [192, 0, 0], 12), "{:?}", at(15, 20));
        assert!(near(at(45, 20), [255, 255, 255], 6), "{:?}", at(45, 20));
        assert!(near(at(5, 20), [255, 255, 255], 6), "{:?}", at(5, 20));
    }

    /// A turned picture turns as the shapes do: its left half where a turned shape's left half is.
    #[test]
    fn a_turned_picture_turns_as_the_shapes_do() {
        // Left half red, right half blue (20 × 20: its smoothed edge about a pixel wide).
        let mut rgba = Vec::new();
        for _y in 0..20 {
            for x in 0..20 {
                rgba.extend_from_slice(&if x < 10 {
                    [220, 0, 0, 255]
                } else {
                    [0, 0, 220, 255]
                });
            }
        }
        let two = Raster::from_rgba(20, 20, rgba).expect("a picture");
        let turned = 30_000;
        let at = shot(80, 40, |f| {
            picture(
                f,
                &two,
                Rectangle::new(Point::new(5.0, 5.0), Size::new(30.0, 30.0)),
                turned,
                1.0,
                None,
            );
            // The same drawn as shapes, 40 to the right.
            f.with_save(|f| {
                f.translate(Vector::new(45.0, 5.0));
                turn_about(f, Size::new(30.0, 30.0), turned);
                f.fill_rectangle(
                    Point::ORIGIN,
                    Size::new(15.0, 30.0),
                    Color::from_rgb8(220, 0, 0),
                );
                f.fill_rectangle(
                    Point::new(15.0, 0.0),
                    Size::new(15.0, 30.0),
                    Color::from_rgb8(0, 0, 220),
                );
            });
        });
        let red = |x0: usize| {
            let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
            for y in 0..40 {
                for x in x0..x0 + 40 {
                    let p = at(x, y);
                    if p[0] > 150 && p[2] < 80 {
                        sx += x as f64 - x0 as f64;
                        sy += y as f64;
                        n += 1.0;
                    }
                }
            }
            (sx / n, sy / n)
        };
        let (a, b) = (red(0), red(40));
        assert!(
            (a.0 - b.0).abs() < 1.5 && (a.1 - b.1).abs() < 1.5,
            "picture's red at {a:?}, the shapes' at {b:?}"
        );
    }

    /// A picture stands between two layers, as a map does: what is drawn after it is over it.
    #[test]
    fn a_picture_has_a_layer_of_its_own() {
        use kentos_sheet::display::{ImagePrim, RectPrim};
        let rect = |item: &str| {
            Prim::Rect(RectPrim {
                item: item.into(),
                rect: RectUm::new(0, 0, 10_000, 10_000),
                rotation: 0,
                radius: 0,
                fill: Some("#ff0000".into()),
                stroke: None,
            })
        };
        let list = DisplayList {
            prims: vec![
                rect("a"),
                Prim::Image(ImagePrim {
                    item: "p".into(),
                    asset: "x".into(),
                    rect: RectUm::new(0, 0, 10_000, 10_000),
                    rotation: 0,
                    opacity: 100,
                }),
                rect("b"),
            ],
            sheet: "s1".into(),
            size: kentos_sheet::units::SizeUm {
                width: 100_000,
                height: 100_000,
            },
            paper: "#ffffff".into(),
            master_items: Vec::new(),
        };
        let p = plan(&list);
        let prims = |l: &Vec<Entry>| l.iter().map(|e| e.prim).collect::<Vec<_>>();
        assert_eq!(
            p.layers.iter().map(prims).collect::<Vec<_>>(),
            [vec![0], vec![2]]
        );
        assert_eq!(p.maps.iter().map(|e| e.prim).collect::<Vec<_>>(), [1]);
    }

    /// A turned clip (a turned map frame's box, where its karelaj is drawn)
    /// cuts a fill and a line to the turned box: not to the upright one, nor
    /// to the box around it.
    #[test]
    fn a_turned_clip_cuts_to_the_turned_box() {
        use kentos_sheet::display::{ClipPrim, ItemTag, RectPrim};
        // 1 px a millimetre; a 60 × 40 mm box about (100, 80) mm, turned 30°.
        let clip = RectUm::new(70_000, 60_000, 60_000, 40_000);
        let tag = || "m".to_owned();
        let list = DisplayList {
            sheet: "s".into(),
            size: kentos_sheet::units::SizeUm {
                width: 200_000,
                height: 160_000,
            },
            paper: "#ffffff".into(),
            prims: vec![
                Prim::PushClip(ClipPrim {
                    item: tag(),
                    rect: clip,
                    rotation: 30_000,
                }),
                // Far larger than the box, filled black.
                Prim::Rect(RectPrim {
                    item: tag(),
                    rect: RectUm::new(10_000, 10_000, 180_000, 140_000),
                    rotation: 0,
                    radius: 0,
                    fill: Some("#000000".into()),
                    stroke: None,
                }),
                Prim::PopClip(ItemTag { item: tag() }),
            ],
            master_items: Vec::new(),
        };
        let plan = plan(&list);
        assert_eq!(
            plan.layers[0][0].clip,
            Some(ClipBox {
                rect: clip,
                rotation: 30_000
            })
        );
        struct Paper(DisplayList, Plan);
        impl canvas::Program<()> for Paper {
            type State = ();
            fn draw(
                &self,
                _: &(),
                renderer: &iced::Renderer,
                _: &iced::Theme,
                b: Rectangle,
                _: iced::mouse::Cursor,
            ) -> Vec<canvas::Geometry> {
                let mut f = Frame::new(renderer, b.size());
                let xf = Xf::new(0.001, Point::ORIGIN);
                paper(&mut f, &self.0, &xf);
                paint_entries(
                    &mut f,
                    &self.0,
                    &self.1.layers[0],
                    &xf,
                    &|_| None,
                    Options::SCREEN,
                );
                vec![f.into_geometry()]
            }
        }
        let view: iced::Element<'_, ()> = iced::widget::canvas(Paper(list, plan))
            .width(iced::Fill)
            .height(iced::Fill)
            .into();
        let image = kentos_ui::snapshot::Snapshot::software(Size::new(200.0, 160.0))
            .expect("a renderer")
            .render(view, &iced::Theme::Light);
        let ink = |x: usize, y: usize| image.rgba[(y * 200 + x) * 4] < 128;
        // The middle: inside.
        assert!(ink(100, 80));
        // The upright box's corners lie outside the turned one: left white.
        for (x, y) in [(72, 62), (128, 62), (72, 98), (128, 98)] {
            assert!(!ink(x, y), "({x}, {y}) is past the turned box");
        }
        // Past the upright box but inside the turned one (its corners reach out): filled.
        for (x, y) in [(112, 104), (88, 56), (131, 80)] {
            assert!(ink(x, y), "({x}, {y}) is inside the turned box");
        }
        // Nothing beyond the box around it.
        assert!(!ink(20, 20) && !ink(180, 140));
    }

    #[test]
    fn colours_read_as_the_core_writes_them() {
        assert_eq!(color("#ff0000"), Some(Color::from_rgb8(255, 0, 0)));
        assert_eq!(
            color("#00000080").map(|c| (c.a * 255.0).round()),
            Some(128.0)
        );
        assert_eq!(color("#fff"), Some(Color::WHITE));
        assert_eq!(color("none"), None);
        assert_eq!(color("#12"), None);
    }

    #[test]
    fn a_line_is_cut_to_the_clip() {
        let r = Some(Rectangle::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0)));
        let (a, b) = clip_line(Point::new(-5.0, 5.0), Point::new(15.0, 5.0), r).unwrap();
        assert_eq!((a, b), (Point::new(0.0, 5.0), Point::new(10.0, 5.0)));
        assert!(clip_line(Point::new(-5.0, -5.0), Point::new(-1.0, 20.0), r).is_none());
        let square = [
            Point::new(-5.0, -5.0),
            Point::new(5.0, -5.0),
            Point::new(5.0, 5.0),
            Point::new(-5.0, 5.0),
        ];
        let cut = clip_polygon(
            &square,
            Rectangle::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0)),
        );
        assert!(
            cut.iter()
                .all(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= 5.0 && p.y <= 5.0),
            "{cut:?}"
        );
        assert_eq!(cut.len(), 4);
    }

    #[test]
    fn an_arc_keeps_within_a_quarter_pixel_and_starts_where_it_should() {
        let xf = Xf::new(0.01, Point::ORIGIN);
        let a = ArcSeg {
            center: [10_000, 10_000],
            radius: [5_000, 5_000],
            rotation: 0,
            start: 0,
            sweep: 90_000,
        };
        let pts = arc_points(&a, &xf);
        assert_eq!(pts[0], Point::new(150.0, 100.0));
        let last = pts[pts.len() - 1];
        assert!(
            (last.x - 100.0).abs() < 1e-3 && (last.y - 150.0).abs() < 1e-3,
            "{last:?}"
        );
        for w in pts.windows(2) {
            let m = Point::new((w[0].x + w[1].x) / 2.0, (w[0].y + w[1].y) / 2.0);
            let d = ((m.x - 100.0).powi(2) + (m.y - 100.0).powi(2)).sqrt();
            assert!(50.0 - d <= 0.26, "{d}");
        }
    }

    #[test]
    fn the_plan_cuts_the_list_at_its_maps_and_carries_clips_and_opacity() {
        use kentos_sheet::display::{ClipPrim, GroupPrim, ItemTag, MapViewPrim, RectPrim};
        use kentos_sheet::kinds::MapLayers;
        let rect = |id: &str| {
            Prim::Rect(RectPrim {
                item: id.into(),
                rect: RectUm::new(0, 0, 10, 10),
                rotation: 0,
                radius: 0,
                fill: Some("#000000".into()),
                stroke: None,
            })
        };
        let map = Prim::Map(MapPrim {
            item: "m".into(),
            clip: RectUm::new(0, 0, 100, 100),
            rotation: 0,
            view: MapViewPrim {
                center: None,
                scale: 1000,
                rotation: 0,
            },
            layers: MapLayers::default(),
            crs: None,
            extent: None,
            clip_feature: None,
        });
        let list = DisplayList {
            sheet: "s".into(),
            size: kentos_sheet::units::SizeUm {
                width: 1000,
                height: 1000,
            },
            paper: "#ffffff".into(),
            prims: vec![
                rect("a"),
                Prim::PushGroup(GroupPrim {
                    item: "g".into(),
                    opacity: 50,
                }),
                map,
                Prim::PushClip(ClipPrim {
                    item: "m".into(),
                    rect: RectUm::new(10, 10, 50, 50),
                    rotation: 0,
                }),
                rect("grid"),
                Prim::PopClip(ItemTag { item: "m".into() }),
                Prim::PopGroup(ItemTag { item: "g".into() }),
                rect("b"),
            ],
            master_items: Vec::new(),
        };
        let p = plan(&list);
        assert_eq!(p.maps.len(), 1);
        assert_eq!(p.layers.len(), 2);
        assert_eq!(p.layers[0].iter().map(|e| e.prim).collect::<Vec<_>>(), [0]);
        assert_eq!(p.maps[0].alpha, 0.5);
        let after: Vec<_> = p.layers[1]
            .iter()
            .map(|e| (e.prim, e.clip.map(|c| c.rect), e.alpha))
            .collect();
        assert_eq!(
            after,
            [(4, Some(RectUm::new(10, 10, 50, 50)), 0.5), (7, None, 1.0)]
        );
    }
}

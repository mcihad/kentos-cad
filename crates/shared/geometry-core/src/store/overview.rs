//! Genel bakış's picture (docs/adr/0181 §3): the extent of what the visible
//! layers hold and a small picture of it, drawn here from the store's shapes
//! by exact pixel rules, so both platforms show the same pixels
//! (`fixtures/navigation/v1/cases.json`, from scripts/fixtures/navigation_cases.py):
//!
//! - an object whose box is under 2 pixels both ways, a text, a dimension or a
//!   leader is a dot (d × d pixels, d the device pixel ratio rounded) at its
//!   box's centre; a point object is a dot at each of its points;
//! - an area (a closed area, a circle, a whole ellipse, a hatch, with its
//!   parts and holes) is filled even-odd at the pixels' centres in its layer's
//!   colour at 64 of 255, over pixels that are not opaque; then every edge is
//!   drawn opaque, from pixel to pixel by Bresenham's rule;
//! - circles, arcs and arc edges in n = max(4, ⌈|sweep|·r·k / 4⌉) equal steps
//!   (at most 720), ellipses and fit-point curves as their outlines;
//!   construction lines are not drawn and do not count in the extent.
//!
//! Objects go in the document's order. Nothing here walks the drawing for a
//! view change: the host draws the picture again when the drawing, its
//! layers' visibility or colours, or the card change.

use std::collections::HashMap;

use super::{Item, Store};
use crate::entity::{Shape, area_parts, is_multi_part};
use crate::geom::bulge::{bulge_arc, bulge_at};
use crate::geom::curve_outline::{ellipse_outline, spline_outline};
use crate::geom::ellipse::is_full_ellipse;
use crate::geometry::Bounds;
use crate::jsmath::{PI, cos, js_max, js_min, sin};
use crate::tools::navigation::{Fit, fit};
use crate::vec2::Vec2;

/// The fill's opacity, of 255.
pub const FILL_ALPHA: u8 = 64;

/// What the host asks for: the picture's size in logical pixels, its device
/// pixel ratio and each layer's colour by its id (a layer without one is not
/// drawn).
pub struct OverviewRequest<'a> {
    pub width: f64,
    pub height: f64,
    pub dpr: f64,
    pub colors: &'a HashMap<String, [u8; 3]>,
}

/// The picture: device pixels, RGBA with straight alpha, rows top down, and
/// the fit it was drawn at.
#[derive(Clone, Debug, PartialEq)]
pub struct OverviewPicture {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
    pub fit: Fit,
}

/// How an object shows in the overview.
enum Look {
    /// Not drawn and not in the extent (construction lines).
    Never,
    /// A dot at its box's centre, however large.
    Dot,
    Drawn,
}

fn look(shape: &Shape) -> Look {
    match shape {
        Shape::Xline { .. } | Shape::Ray { .. } => Look::Never,
        Shape::Text { .. } | Shape::Dimension { .. } | Shape::Leader { .. } => Look::Dot,
        _ => Look::Drawn,
    }
}

fn finite(b: &Bounds) -> bool {
    b.min_x.is_finite()
        && b.min_y.is_finite()
        && b.max_x.is_finite()
        && b.max_y.is_finite()
        && b.min_x <= b.max_x
        && b.min_y <= b.max_y
}

impl Store {
    fn shown_items(&self) -> impl Iterator<Item = &Item> {
        self.ordered
            .iter()
            .filter_map(|&e| self.ordered_item(e))
            .filter(|it| self.flags.get(it.layer as usize).is_none_or(|f| f.visible))
            .filter(|it| !matches!(look(&it.shape), Look::Never) && finite(&it.bounds))
    }

    /// The extent the overview shows: the union of the boxes of the objects
    /// on visible layers, construction lines left out; none without any.
    pub fn overview_extent(&self) -> Option<Bounds> {
        self.shown_items().fold(None, |acc: Option<Bounds>, it| {
            let b = it.bounds;
            Some(match acc {
                None => b,
                Some(a) => Bounds {
                    min_x: js_min(a.min_x, b.min_x),
                    min_y: js_min(a.min_y, b.min_y),
                    max_x: js_max(a.max_x, b.max_x),
                    max_y: js_max(a.max_y, b.max_y),
                },
            })
        })
    }

    /// The overview's picture of the extent (§3); none for an empty drawing.
    pub fn overview_picture(&self, request: &OverviewRequest<'_>) -> Option<OverviewPicture> {
        let extent = self.overview_extent()?;
        let f = fit(&extent, (request.width, request.height));
        let mut colors: Vec<Option<[u8; 3]>> = vec![None; self.flags.len()];
        for (id, &l) in &self.layer_ids {
            if let (Some(slot), Some(c)) = (colors.get_mut(l as usize), request.colors.get(id)) {
                *slot = Some(*c);
            }
        }
        let mut canvas = Canvas::new(request.width, request.height, request.dpr, f);
        for it in self.shown_items() {
            let Some(Some(color)) = colors.get(it.layer as usize) else {
                continue;
            };
            canvas.object(it, *color);
        }
        Some(OverviewPicture {
            width: canvas.w,
            height: canvas.h,
            pixels: canvas.px,
            fit: f,
        })
    }
}

/// The pixels being drawn and how drawing points fall on them.
struct Canvas {
    w: usize,
    h: usize,
    px: Vec<u8>,
    cx: f64,
    cy: f64,
    kp: f64,
    dot: i64,
}

impl Canvas {
    fn new(width: f64, height: f64, dpr: f64, f: Fit) -> Canvas {
        let w = (width * dpr + 0.5).floor() as usize;
        let h = (height * dpr + 0.5).floor() as usize;
        Canvas {
            w,
            h,
            px: vec![0; w * h * 4],
            cx: f.cx,
            cy: f.cy,
            kp: f.k * dpr,
            dot: js_max(1.0, (dpr + 0.5).floor()) as i64,
        }
    }

    /// A drawing point on the picture, device pixels (not yet whole).
    fn at(&self, p: Vec2) -> (f64, f64) {
        (
            self.w as f64 / 2.0 + (p.x - self.cx) * self.kp,
            self.h as f64 / 2.0 - (p.y - self.cy) * self.kp,
        )
    }

    fn cell(&self, p: Vec2) -> (i64, i64) {
        let (u, v) = self.at(p);
        (u.floor() as i64, v.floor() as i64)
    }

    fn put(&mut self, i: i64, j: i64, c: [u8; 3], alpha: u8) {
        if i < 0 || j < 0 || i as usize >= self.w || j as usize >= self.h {
            return;
        }
        let at = (j as usize * self.w + i as usize) * 4;
        if alpha < 255 && self.px[at + 3] == 255 {
            return;
        }
        self.px[at..at + 4].copy_from_slice(&[c[0], c[1], c[2], alpha]);
    }

    fn dot_at(&mut self, p: Vec2, c: [u8; 3]) {
        let (i, j) = self.cell(p);
        for dj in 0..self.dot {
            for di in 0..self.dot {
                self.put(i + di, j + dj, c, 255);
            }
        }
    }

    /// Bresenham's line from cell to cell, both ends drawn.
    fn segment(&mut self, (mut x0, mut y0): (i64, i64), (x1, y1): (i64, i64), c: [u8; 3]) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.put(x0, y0, c, 255);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn path(&mut self, pts: &[Vec2], closed: bool, c: [u8; 3]) {
        let cells: Vec<(i64, i64)> = pts.iter().map(|&p| self.cell(p)).collect();
        for w in cells.windows(2) {
            self.segment(w[0], w[1], c);
        }
        if closed && let (Some(&first), Some(&last)) = (cells.first(), cells.last()) {
            self.segment(last, first, c);
        }
    }

    /// The rings filled even-odd at the pixels' centres.
    fn fill(&mut self, rings: &[Vec<Vec2>], c: [u8; 3]) {
        let mut edges: Vec<((f64, f64), (f64, f64))> = Vec::new();
        for ring in rings {
            let pts: Vec<(f64, f64)> = ring.iter().map(|&p| self.at(p)).collect();
            for (i, &a) in pts.iter().enumerate() {
                edges.push((a, pts[(i + 1) % pts.len()]));
            }
        }
        // Each edge's crossings with the rows' centre lines: min ≤ j + 0.5 < max.
        let mut rows: Vec<Vec<f64>> = vec![Vec::new(); self.h];
        for &((ax, ay), (bx, by)) in &edges {
            if ay == by {
                continue;
            }
            let lo = (js_min(ay, by) - 0.5).ceil();
            let hi = (js_max(ay, by) - 0.5).ceil();
            let first = js_max(lo, 0.0) as usize;
            let last = js_min(hi, self.h as f64) as usize;
            for (j, row) in rows.iter_mut().enumerate().take(last).skip(first) {
                let yc = j as f64 + 0.5;
                if (ay <= yc) != (by <= yc) {
                    row.push(ax + (yc - ay) * (bx - ax) / (by - ay));
                }
            }
        }
        for (j, xs) in rows.iter_mut().enumerate() {
            xs.sort_by(f64::total_cmp);
            for pair in xs.chunks_exact(2) {
                let from = js_max((pair[0] - 0.5).ceil(), 0.0) as i64;
                let to = js_min((pair[1] - 0.5).ceil(), self.w as f64) as i64;
                for i in from..to {
                    self.put(i, j as i64, c, FILL_ALPHA);
                }
            }
        }
    }

    /// Circle and arc points: n equal steps from `a0` over `sweep`, both ends.
    fn arc(&self, c: Vec2, r: f64, a0: f64, sweep: f64) -> Vec<Vec2> {
        let n = js_min(js_max(4.0, (sweep.abs() * r * self.kp / 4.0).ceil()), 720.0) as usize;
        (0..=n)
            .map(|i| {
                let a = a0 + sweep * i as f64 / n as f64;
                Vec2::new(c.x + r * cos(a), c.y + r * sin(a))
            })
            .collect()
    }

    /// A bulged path's points, its arc edges in the arcs' steps.
    fn bulged(&self, pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> Vec<Vec2> {
        let n = pts.len();
        let mut out = Vec::with_capacity(n);
        let edges = if closed { n } else { n.saturating_sub(1) };
        if n == 0 {
            return out;
        }
        out.push(pts[0]);
        for i in 0..edges {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            if let Some(arc) = bulge_arc(a, b, bulge_at(bulges, i)) {
                let inner = self.arc(arc.c, arc.r, arc.a0, arc.sweep);
                out.extend_from_slice(&inner[1..inner.len() - 1]);
            }
            if !(closed && i + 1 == n) {
                out.push(b);
            }
        }
        out
    }

    fn object(&mut self, it: &Item, c: [u8; 3]) {
        let b = it.bounds;
        let small = (b.max_x - b.min_x) * self.kp < 2.0 && (b.max_y - b.min_y) * self.kp < 2.0;
        if small || matches!(look(&it.shape), Look::Dot) {
            self.dot_at(
                Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0),
                c,
            );
            return;
        }
        let shapes: Vec<&Shape> = it.shapes().collect();
        for s in shapes {
            self.shape(s, c);
        }
    }

    fn shape(&mut self, s: &Shape, c: [u8; 3]) {
        if is_multi_part(s) && !matches!(s, Shape::Point { .. }) {
            for part in area_parts(s).iter() {
                self.shape(part, c);
            }
            return;
        }
        match s {
            Shape::Point { p, parts, .. } => {
                self.dot_at(*p, c);
                for q in parts.iter().flatten() {
                    self.dot_at(q.p, c);
                }
            }
            Shape::Line { a, b } => self.path(&[*a, *b], false, c),
            Shape::Polyline { pts, bulges, .. } => {
                let pts = self.bulged(pts, bulges.as_deref(), false);
                self.path(&pts, false, c);
            }
            Shape::Polygon {
                pts, bulges, holes, ..
            } => {
                let mut rings = vec![self.bulged(pts, bulges.as_deref(), true)];
                for h in holes.iter().flatten() {
                    rings.push(self.bulged(&h.pts, h.bulges.as_deref(), true));
                }
                self.area(&rings, c);
            }
            Shape::Circle { c: o, r } => {
                let mut ring = self.arc(*o, *r, 0.0, 2.0 * PI);
                ring.pop();
                self.area(&[ring], c);
            }
            Shape::Arc { c: o, r, a0, a1 } => {
                let mut sweep = a1 - a0;
                while sweep <= 0.0 {
                    sweep += 2.0 * PI;
                }
                let pts = self.arc(*o, *r, *a0, sweep);
                self.path(&pts, false, c);
            }
            Shape::Ellipse {
                c: o,
                major,
                ratio,
                t0,
                t1,
            } => {
                let g = crate::entity::ellipse_geom(*o, *major, *ratio, *t0, *t1);
                let pts = ellipse_outline(&g);
                if is_full_ellipse(&g) {
                    self.area(&[pts], c);
                } else {
                    self.path(&pts, false, c);
                }
            }
            Shape::Spline { pts, closed } => {
                let pts = spline_outline(pts, *closed);
                self.path(&pts, *closed, c);
            }
            Shape::Hatch { ring, holes, .. } => {
                let mut rings = vec![ring.clone()];
                rings.extend(holes.iter().flatten().cloned());
                self.area(&rings, c);
            }
            // Pieces of a block are their own shapes; the rest show as dots (`object`).
            Shape::Insert { p, .. } => self.dot_at(*p, c),
            // A picture's part shown, filled (docs/adr/0192 §3).
            Shape::Image { .. } => self.area(&[crate::geom::image::shown(s)], c),
            // A raster's frame, filled (docs/adr/0204 §5).
            Shape::Raster { .. } => self.area(&[crate::entity::entity_vertices(s)], c),
            // A cloud's plan, filled (docs/adr/0207 §6).
            Shape::PointCloud { .. } => self.area(&[crate::entity::entity_vertices(s)], c),
            // A table's outline (docs/adr/0184 §2).
            Shape::Table { .. } => {
                if let Some(t) = crate::geom::table::table_geom(s) {
                    self.path(&t.outline(), true, c);
                }
            }
            Shape::Text { .. }
            | Shape::Dimension { .. }
            | Shape::Leader { .. }
            | Shape::Xline { .. }
            | Shape::Ray { .. } => {}
        }
    }

    fn area(&mut self, rings: &[Vec<Vec2>], c: [u8; 3]) {
        self.fill(rings, c);
        for ring in rings {
            self.path(ring, true, c);
        }
    }
}

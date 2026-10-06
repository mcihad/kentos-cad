//! What is drawn of each object (docs/adr/0008, S2), for a whole layer in
//! one call: the style engine's geometry (`styledGeometry`: a marker, lines,
//! or an area with its outer ring counter-clockwise and its holes
//! clockwise) or, unoriented, the highlight layers' (`buildSceneLayer`).
//! Curves are tessellated here as the TypeScript did; straight geometry is
//! not copied: a path or ring that is the object's own points goes as a
//! reference (`SOURCE`, or `REVERSED` when its orientation had to turn),
//! so a layer of a million contour segments does not cross the boundary.
//!
//! Records, one per id in order:
//! - `NONE`: nothing drawn (text, an unknown id, a construction line
//!   outside the clip box, a dimension without a layout);
//! - `MARKER, x, y`;
//! - `MARKERS, n, x0, y0, …`: a multi-point object's points (docs/adr/0174),
//!   each drawn as a `MARKER` would be;
//! - `LINE, paths`, then per path `closed` (0 or 1) and its points;
//! - `FILL, rings`, then per ring its points (outer ring first);
//! - `FILLS, parts`, then per part `rings` and per ring its points, a
//!   multi-part area's (docs/adr/0143): each part is drawn as a `FILL`
//!   would be, its points always given (no reference);
//! - `GROUP, n`, then `n` times a piece's place among its block's pieces
//!   and the piece's own record (never `NONE` nor another `GROUP`), a
//!   block's insert (docs/adr/0144): its pieces placed, their points always
//!   given. The host draws each with the piece's own colour and line weight
//!   when it has them (`Store::block_pieces_json`), else the insert's;
//! - `MIXED`, then a `LINE` record's paths and a `FILL` record's rings (0
//!   for none), their points always given: a leader's (docs/adr/0146 §5),
//!   its line on to its landing's end and an open arrowhead's sides, then
//!   its filled arrowhead or dot; a dimension's layout lines and its filled
//!   arrowheads or dots (docs/adr/0183 §3). The host strokes the paths and
//!   fills each ring solid in the object's colour, each an area of its own.
//!
//! Points are `n, x0, y0, …`, or `SOURCE` / `REVERSED`: the object's own
//! points for that path or ring (a line's two ends; a polyline's or
//! polygon's vertices, hole k's for ring k + 1; a hatch's ring and holes).
//! A multi-part polyline's paths, one per part, always give their points
//! (docs/adr/0174).
//! A dimension's layout lines come as open two-point paths.

use super::Store;
use crate::api::Op;
use crate::entity::{
    Entity, Shape, area_parts, dimension_geom, entity_anchor, entity_area, entity_length,
    entity_outline, is_closed_outline, is_multi_part, polygon_ring,
};
use crate::geom::bulge::has_bulges;
use crate::geom::dimension::layout_dimension;
use crate::geom::leader::{self, Head};
use crate::geometry::{Bounds, signed_area};
use crate::jsmath::{js_max, js_min};
use crate::op;
use crate::vec2::Vec2;

pub const NONE: f64 = 0.0;
pub const MARKER: f64 = 1.0;
pub const LINE: f64 = 2.0;
pub const FILL: f64 = 3.0;
/// A multi-part area: its parts' fills in one record (docs/adr/0143).
pub const FILLS: f64 = 4.0;
/// A block's insert: its pieces' records, each after its place (docs/adr/0144).
pub const GROUP: f64 = 5.0;
/// Lines and a solid area in one record: a leader's (docs/adr/0146).
pub const MIXED: f64 = 6.0;
/// A multi-point object's points (docs/adr/0174).
pub const MARKERS: f64 = 7.0;
/// The object's own points, as they are.
pub const SOURCE: f64 = -1.0;
/// The object's own points, last to first.
pub const REVERSED: f64 = -2.0;

/// Segments of a full turn in drawn outlines (`entityOutline`'s default).
const OUTLINE_SEGMENTS: f64 = 72.0;

fn points(out: &mut Vec<f64>, pts: &[Vec2]) {
    out.push(pts.len() as f64);
    for p in pts {
        out.push(p.x);
        out.push(p.y);
    }
}

/// A ring, turned to counter-clockwise (`Some(true)`) or clockwise by its
/// signed area as `oriented` in `apps/web/src/style/geometry.ts` turned it; `own`:
/// these are the object's own points (sent as a reference).
fn ring(out: &mut Vec<f64>, pts: &[Vec2], own: bool, ccw: Option<bool>) {
    let turn = ccw.is_some_and(|ccw| (signed_area(pts) > 0.0) != ccw);
    match (own, turn) {
        (true, false) => out.push(SOURCE),
        (true, true) => out.push(REVERSED),
        (false, false) => points(out, pts),
        (false, true) => {
            let back: Vec<Vec2> = pts.iter().rev().copied().collect();
            points(out, &back);
        }
    }
}

/// Part of the line `p + dir·t` (t ≥ 0 for a ray) inside the box, or None
/// (`clipLine`: construction lines are drawn clipped to the view).
pub fn clip_line(p: Vec2, dir: Vec2, ray: bool, r: &Bounds) -> Option<[Vec2; 2]> {
    let mut t0 = if ray { 0.0 } else { f64::NEG_INFINITY };
    let mut t1 = f64::INFINITY;
    for (pp, d, min, max) in [
        (p.x, dir.x, r.min_x, r.max_x),
        (p.y, dir.y, r.min_y, r.max_y),
    ] {
        if d.abs() < 1e-15 {
            if pp < min || pp > max {
                return None;
            }
            continue;
        }
        let a = (min - pp) / d;
        let b = (max - pp) / d;
        t0 = js_max(t0, js_min(a, b));
        t1 = js_min(t1, js_max(a, b));
    }
    if !(t1 > t0) {
        return None;
    }
    Some([
        Vec2::new(p.x + dir.x * t0, p.y + dir.y * t0),
        Vec2::new(p.x + dir.x * t1, p.y + dir.y * t1),
    ])
}

/// One path of a `LINE` record.
fn path(out: &mut Vec<f64>, closed: bool, pts: &[Vec2]) {
    out.push(if closed { 1.0 } else { 0.0 });
    points(out, pts);
}

/// The drawn geometry of one object (see the module comment). `oriented`:
/// rings turned for the style engine; otherwise as the object has them.
/// `clip`: the box construction lines are clipped to (none: not drawn).
pub fn drawn(s: &Shape, oriented: bool, clip: Option<&Bounds>, out: &mut Vec<f64>) {
    drawn_record(s, oriented, clip, true, out);
}

/// `drawn`, with the object's own points referred to (`refs`) or written
/// out: a block's piece is no object of the drawing, so its points go as they are.
fn drawn_record(s: &Shape, oriented: bool, clip: Option<&Bounds>, refs: bool, out: &mut Vec<f64>) {
    match s {
        // Every point of a multi-point object (docs/adr/0174).
        Shape::Point { .. } if is_multi_part(s) => {
            let parts = area_parts(s);
            out.extend([MARKERS, parts.len() as f64]);
            for part in parts.iter() {
                let p = match part {
                    Shape::Point { p, .. } => *p,
                    _ => Vec2::new(f64::NAN, f64::NAN),
                };
                out.extend([p.x, p.y]);
            }
        }
        // An insert shows its insertion point until the store expands its block (docs/adr/0144).
        Shape::Point { p, .. } | Shape::Insert { p, .. } => out.extend([MARKER, p.x, p.y]),
        Shape::Text { .. } => out.push(NONE),
        // Its layout lines; with filled arrowheads or dots, those areas after them, each a ring of
        // its own (docs/adr/0183 §3).
        Shape::Dimension { .. } => match dimension_geom(s).and_then(|d| layout_dimension(&d)) {
            Some(l) => {
                let fills = l.fills.unwrap_or_default();
                out.extend([
                    if fills.is_empty() { LINE } else { MIXED },
                    l.lines.len() as f64,
                ]);
                for [a, b] in l.lines {
                    path(out, false, &[a, b]);
                }
                if !fills.is_empty() {
                    out.push(fills.len() as f64);
                    for r in &fills {
                        ring(out, r, false, oriented.then_some(true));
                    }
                }
            }
            None => out.push(NONE),
        },
        // The part shown: its outline as a line, then as an area the style engine paints with
        // the picture (docs/adr/0192 §3).
        Shape::Image { .. } => {
            let shown = crate::geom::image::shown(s);
            if shown.len() < 3 {
                out.push(NONE);
            } else {
                out.extend([MIXED, 1.0]);
                path(out, true, &shown);
                out.push(1.0);
                ring(out, &shown, false, oriented.then_some(true));
            }
        }
        // Its lines, then its frame's band, four strips each a ring of its own; its words are
        // labels (docs/adr/0184 §2).
        Shape::Table { .. } => {
            let lines = crate::geom::table::lines_of(s);
            let band = crate::geom::table::band_of(s);
            if lines.is_empty() && band.is_empty() {
                out.push(NONE);
            } else {
                out.extend([
                    if band.is_empty() { LINE } else { MIXED },
                    lines.len() as f64,
                ]);
                for [a, b] in lines {
                    path(out, false, &[a, b]);
                }
                if !band.is_empty() {
                    out.push(band.len() as f64);
                    for strip in &band {
                        ring(out, strip, false, oriented.then_some(true));
                    }
                }
            }
        }
        Shape::Line { a, b } => {
            out.extend([LINE, 1.0]);
            if refs {
                out.extend([0.0, SOURCE]);
            } else {
                path(out, false, &[*a, *b]);
            }
        }
        // Its line on to its landing's end, an open arrowhead's sides, a filled one's or a dot's area (docs/adr/0146 §5).
        Shape::Leader { pts, .. } => match leader::layout_of(s) {
            Some(l) => {
                let open = match &l.head {
                    Head::Open { lines } => Some(lines),
                    _ => None,
                };
                out.extend([MIXED, if open.is_some() { 2.0 } else { 1.0 }]);
                path(out, false, &leader::drawn_path(pts, &l));
                if let Some(lines) = open {
                    path(out, false, lines);
                }
                match leader::head_ring(&l.head) {
                    Some(r) => {
                        out.push(1.0);
                        ring(out, &r, false, oriented.then_some(true));
                    }
                    None => out.push(0.0),
                }
            }
            None => out.push(NONE),
        },
        // Every part of a multi-part polyline, its points always given (docs/adr/0174).
        Shape::Polyline { .. } if is_multi_part(s) => {
            let parts = area_parts(s);
            out.extend([LINE, parts.len() as f64]);
            for part in parts.iter() {
                match part {
                    Shape::Polyline {
                        bulges: Some(_), ..
                    } => path(out, false, &entity_outline(part, OUTLINE_SEGMENTS)),
                    Shape::Polyline { pts, .. } => path(out, false, pts),
                    _ => path(out, false, &[]),
                }
            }
        }
        Shape::Polyline { pts, bulges, .. } => {
            out.extend([LINE, 1.0]);
            // As the TypeScript tested it: any bulge list (even all zero) is tessellated.
            if bulges.is_some() {
                path(out, false, &entity_outline(s, OUTLINE_SEGMENTS));
            } else if refs {
                out.extend([0.0, SOURCE]);
            } else {
                path(out, false, pts);
            }
        }
        Shape::Polygon { .. } if is_multi_part(s) => {
            let parts = area_parts(s);
            out.extend([FILLS, parts.len() as f64]);
            for part in parts.iter() {
                let Shape::Polygon {
                    pts, bulges, holes, ..
                } = part
                else {
                    continue;
                };
                out.push((1 + holes.as_ref().map_or(0, Vec::len)) as f64);
                let ring_of =
                    |out: &mut Vec<f64>, pts: &[Vec2], bulges: Option<&[f64]>, ccw: bool| {
                        // Always the points themselves: a reference could not say which part.
                        ring(
                            out,
                            &polygon_ring(pts, bulges),
                            false,
                            oriented.then_some(ccw),
                        );
                    };
                ring_of(out, pts, bulges.as_deref(), true);
                for h in holes.iter().flatten() {
                    ring_of(out, &h.pts, h.bulges.as_deref(), false);
                }
            }
        }
        Shape::Polygon {
            pts, bulges, holes, ..
        } => {
            out.extend([FILL, (1 + holes.as_ref().map_or(0, Vec::len)) as f64]);
            let outer = |out: &mut Vec<f64>, pts: &[Vec2], bulges: Option<&[f64]>, ccw: bool| {
                let ccw = oriented.then_some(ccw);
                if has_bulges(bulges) {
                    ring(out, &polygon_ring(pts, bulges), false, ccw);
                } else {
                    ring(out, pts, refs, ccw);
                }
            };
            outer(out, pts, bulges.as_deref(), true);
            for h in holes.iter().flatten() {
                outer(out, &h.pts, h.bulges.as_deref(), false);
            }
        }
        Shape::Circle { .. } => {
            out.extend([LINE, 1.0]);
            path(out, true, &entity_outline(s, OUTLINE_SEGMENTS));
        }
        Shape::Arc { .. } => {
            out.extend([LINE, 1.0]);
            path(out, false, &entity_outline(s, OUTLINE_SEGMENTS));
        }
        Shape::Ellipse { .. } => {
            out.extend([LINE, 1.0]);
            path(
                out,
                is_closed_outline(s),
                &entity_outline(s, OUTLINE_SEGMENTS),
            );
        }
        Shape::Xline { p, dir } | Shape::Ray { p, dir } => {
            let ray = matches!(s, Shape::Ray { .. });
            match clip.and_then(|c| clip_line(*p, *dir, ray, c)) {
                Some(seg) => {
                    out.extend([LINE, 1.0]);
                    path(out, false, &seg);
                }
                None => out.push(NONE),
            }
        }
        // Drawn open even when closed: the curve itself comes back to its start.
        Shape::Spline { .. } => {
            out.extend([LINE, 1.0]);
            path(out, false, &entity_outline(s, OUTLINE_SEGMENTS));
        }
        Shape::Hatch { ring: r, holes, .. } => {
            out.extend([FILL, (1 + holes.as_ref().map_or(0, Vec::len)) as f64]);
            ring(out, r, refs, oriented.then_some(true));
            for h in holes.iter().flatten() {
                ring(out, h, refs, oriented.then_some(false));
            }
        }
    }
}

/// One piece in a pieces list: its fields, its own colour and line weight,
/// and an attribute's tag (the insert's value under it is shown, else the text).
fn piece_json(
    out: &mut String,
    i: usize,
    s: &Shape,
    color: Option<&str>,
    weight: Option<f64>,
    attribute: Option<&str>,
) {
    if i > 0 {
        out.push(',');
    }
    out.push('{');
    let mut first = true;
    s.write_fields(out, &mut first);
    if let Some(c) = color {
        crate::api::json::field(out, &mut first, "color", c);
    }
    if let Some(w) = &weight {
        crate::api::json::field(out, &mut first, "lineWeight", w);
    }
    if let Some(tag) = attribute {
        crate::api::json::field(out, &mut first, "attribute", tag);
    }
    out.push('}');
}

/// A `GROUP` record: every piece that draws something, after its place.
fn group(pieces: &[Shape], oriented: bool, clip: Option<&Bounds>, out: &mut Vec<f64>) {
    let at = out.len();
    out.extend([GROUP, 0.0]);
    let mut n = 0.0;
    for (i, s) in pieces.iter().enumerate() {
        let before = out.len();
        out.push(i as f64);
        drawn_record(s, oriented, clip, false, out);
        if out.get(before + 1) == Some(&NONE) {
            out.truncate(before);
        } else {
            n += 1.0;
        }
    }
    out[at + 1] = n;
}

/// Bits of a `measures` record: which values the object has.
pub const HAS_LENGTH: u32 = 1;
pub const HAS_AREA: u32 = 2;
pub const HAS_ANCHOR: u32 = 4;
/// Numbers per `measures` record.
pub const MEASURE_STRIDE: usize = 6;

/// The geometry values of expressions (`$uzunluk`, `$alan`, `$y`, `$x`):
/// `flags, length, area, anchor x, anchor y, 0` (missing values 0).
pub fn measure_record(s: Option<&Shape>, out: &mut Vec<f64>) {
    let Some(s) = s else {
        out.extend([0.0; MEASURE_STRIDE]);
        return;
    };
    let length = entity_length(s);
    let area = entity_area(s);
    let anchor = entity_anchor(s);
    let flags = if length.is_some() { HAS_LENGTH } else { 0 }
        | if area.is_some() { HAS_AREA } else { 0 }
        | if anchor.is_some() { HAS_ANCHOR } else { 0 };
    let a = anchor.unwrap_or(Vec2::new(0.0, 0.0));
    out.extend([
        f64::from(flags),
        length.unwrap_or(0.0),
        area.unwrap_or(0.0),
        a.x,
        a.y,
        0.0,
    ]);
}

/// One object's drawn geometry by name, for callers without a store (symbol previews).
pub(crate) static OPS: &[Op] = &[op!(
    "drawnGeometry",
    |e: Entity, oriented: bool, clip: Option<Bounds>| {
        let mut out = Vec::new();
        drawn(&e.shape, oriented, clip.as_ref(), &mut out);
        out
    }
)];

impl Store {
    /// Drawn geometry of these objects, one record each (see `drawn`).
    pub fn drawn(&self, ids: &[f64], oriented: bool, clip: Option<&Bounds>) -> Vec<f64> {
        let mut out = Vec::new();
        for &id in ids {
            match self.get(id) {
                Some(it) => match &it.expanded {
                    Some(x) => group(&x.shapes, oriented, clip, &mut out),
                    None => drawn(&it.shape, oriented, clip, &mut out),
                },
                None => out.push(NONE),
            }
        }
        out
    }

    /// A block's pieces as the host draws them (docs/adr/0144): each piece
    /// of the flattened definition in order, its fields (relative to the
    /// base point) with its own `color` and `lineWeight` when it has them,
    /// and an attribute's `attribute` tag (§7: the insert's value under it
    /// is shown, else the piece's text); what `GROUP` records and
    /// `LABEL_PIECE_*` labels refer to by place. None for a block the drawing
    /// does not define.
    pub fn block_pieces_json(&self, block: &str) -> Option<String> {
        let flat = self.blocks().get(block)?;
        let mut out = String::from("[");
        for (i, piece) in flat.pieces.iter().enumerate() {
            piece_json(
                &mut out,
                i,
                &piece.shape,
                piece.color.as_deref(),
                piece.line_weight,
                piece.attribute.as_deref(),
            );
        }
        out.push(']');
        Some(out)
    }

    /// An insert's pieces as placed (`block_pieces_json`'s layout, in the
    /// drawing's coordinates): what a host reads of a piece that changes
    /// with the placing, such as a hatch piece's pattern. None for any
    /// object but an insert of a known block.
    pub fn insert_pieces_json(&self, id: f64) -> Option<String> {
        let x = self.get(id)?.expanded.as_ref()?;
        let mut out = String::from("[");
        for (i, s) in x.shapes.iter().enumerate() {
            piece_json(
                &mut out,
                i,
                s,
                x.colors.get(i).and_then(|c| c.as_deref()),
                x.weights.get(i).copied().flatten(),
                None,
            );
        }
        out.push(']');
        Some(out)
    }

    /// Geometry values of these objects for expressions (see `measure_record`).
    pub fn measures(&self, ids: &[f64]) -> Vec<f64> {
        let mut out = Vec::with_capacity(ids.len() * MEASURE_STRIDE);
        for &id in ids {
            measure_record(self.get(id).map(|it| &it.shape), &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::{FromJson, Json};

    fn shape(json: &str) -> Shape {
        Entity::from_json(&Json::parse(json).unwrap())
            .unwrap()
            .shape
    }

    fn record(json: &str, oriented: bool, clip: Option<&Bounds>) -> Vec<f64> {
        let mut out = Vec::new();
        drawn(&shape(json), oriented, clip, &mut out);
        out
    }

    #[test]
    fn straight_geometry_is_referred_to_not_copied() {
        let line = r#"{"kind":"line","a":{"x":0,"y":0},"b":{"x":3,"y":4}}"#;
        assert_eq!(record(line, true, None), [LINE, 1.0, 0.0, SOURCE]);
        let path = r#"{"kind":"polyline","pts":[{"x":0,"y":0},{"x":1,"y":0},{"x":1,"y":1}]}"#;
        assert_eq!(record(path, true, None), [LINE, 1.0, 0.0, SOURCE]);
        // A clockwise square with a clockwise hole: the outer ring turns for the style engine, the hole stays.
        let cw = r#"{"kind":"polygon","pts":[{"x":0,"y":0},{"x":0,"y":4},{"x":4,"y":4},{"x":4,"y":0}],
            "holes":[{"pts":[{"x":1,"y":1},{"x":1,"y":2},{"x":2,"y":2}]}]}"#;
        assert_eq!(record(cw, true, None), [FILL, 2.0, REVERSED, SOURCE]);
        // Unoriented (the highlight layers): both as they are.
        assert_eq!(record(cw, false, None), [FILL, 2.0, SOURCE, SOURCE]);
        let hatch = r#"{"kind":"hatch","ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],
            "holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]],"pattern":{"type":"solid","angle":0,"spacing":1}}"#;
        assert_eq!(record(hatch, true, None), [FILL, 2.0, SOURCE, REVERSED]);
    }

    #[test]
    fn a_leader_draws_its_lines_and_its_arrowhead_in_one_record() {
        // Up 4 m from the tip, the note 3 m high: the landing runs 6 m east from the last vertex.
        let leader = |extra: &str| {
            format!(
                r#"{{"kind":"leader","pts":[{{"x":0,"y":0}},{{"x":0,"y":4}}],"height":3,"rotation":0{extra}}}"#
            )
        };
        let line = [0.0, 3.0, 0.0, 0.0, 0.0, 4.0, 6.0, 4.0];
        // A filled arrow: its triangle (tip, base corners) turned counter-clockwise for the style engine.
        let filled = record(&leader(r#","text":"Not""#), true, None);
        assert_eq!(&filled[..2], [MIXED, 1.0]);
        assert_eq!(&filled[2..10], line);
        assert_eq!(&filled[10..], [1.0, 3.0, 0.5, 3.0, -0.5, 3.0, 0.0, 0.0]);
        // Unoriented (the highlight layers): as the layout has it.
        let plain = record(&leader(r#","text":"Not""#), false, None);
        assert_eq!(&plain[10..], [1.0, 3.0, 0.0, 0.0, -0.5, 3.0, 0.5, 3.0]);
        // An open arrow: its two sides a path of their own, no area.
        let open = record(&leader(r#","text":"Not","arrow":"open""#), true, None);
        assert_eq!(&open[..2], [MIXED, 2.0]);
        assert_eq!(&open[10..], [0.0, 3.0, -0.5, 3.0, 0.0, 0.0, 0.5, 3.0, 0.0]);
        // Without a note no landing; a dot is its outline, a full turn's 72 points about the tip.
        let dot = record(&leader(r#","arrow":"dot""#), true, None);
        assert_eq!(&dot[..9], [MIXED, 1.0, 0.0, 2.0, 0.0, 0.0, 0.0, 4.0, 1.0]);
        assert_eq!((dot[9], dot.len()), (72.0, 10 + 2 * 72));
        assert_eq!((dot[10], dot[11]), (0.75, 0.0));
        let bare = record(&leader(r#","arrow":"none""#), true, None);
        assert_eq!(bare, [MIXED, 1.0, 0.0, 2.0, 0.0, 0.0, 0.0, 4.0, 0.0]);
    }

    #[test]
    fn curves_come_tessellated() {
        let circle = record(r#"{"kind":"circle","c":{"x":10,"y":0},"r":2}"#, true, None);
        assert_eq!(&circle[..4], [LINE, 1.0, 1.0, 72.0]);
        assert_eq!(circle.len(), 4 + 2 * 72);
        assert_eq!((circle[4], circle[5]), (12.0, 0.0));
        // A bulge makes the ring explicit: 1 + the arc's points.
        let bulged = record(
            r#"{"kind":"polygon","pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"bulges":[0,0.5,0]}"#,
            true,
            None,
        );
        assert_eq!(&bulged[..2], [FILL, 1.0]);
        assert!(bulged[2] > 3.0 && bulged.len() == 3 + 2 * bulged[2] as usize);
        // Any bulge list (even all zero) tessellates a polyline, as the TypeScript tested it.
        let flat = record(
            r#"{"kind":"polyline","pts":[{"x":0,"y":0},{"x":1,"y":0}],"bulges":[0]}"#,
            true,
            None,
        );
        assert_eq!(flat, [LINE, 1.0, 0.0, 2.0, 0.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn construction_lines_only_clipped_and_nothing_for_text() {
        let xline = r#"{"kind":"xline","p":{"x":0,"y":0},"dir":{"x":1,"y":0}}"#;
        assert_eq!(record(xline, true, None), [NONE]);
        let clip = Bounds {
            min_x: -5.0,
            min_y: -1.0,
            max_x: 5.0,
            max_y: 1.0,
        };
        assert_eq!(
            record(xline, true, Some(&clip)),
            [LINE, 1.0, 0.0, 2.0, -5.0, 0.0, 5.0, 0.0]
        );
        let ray = r#"{"kind":"ray","p":{"x":0,"y":0},"dir":{"x":1,"y":0}}"#;
        assert_eq!(
            record(ray, true, Some(&clip)),
            [LINE, 1.0, 0.0, 2.0, 0.0, 0.0, 5.0, 0.0]
        );
        let text = r#"{"kind":"text","p":{"x":0,"y":0},"text":"A","height":1,"rotation":0}"#;
        assert_eq!(record(text, true, None), [NONE]);
        let point = r#"{"kind":"point","p":{"x":7,"y":8}}"#;
        assert_eq!(record(point, false, None), [MARKER, 7.0, 8.0]);
    }

    #[test]
    fn measures_say_what_an_object_has() {
        let mut out = Vec::new();
        measure_record(
            Some(&shape(
                r#"{"kind":"line","a":{"x":0,"y":0},"b":{"x":3,"y":4}}"#,
            )),
            &mut out,
        );
        assert_eq!(
            out,
            [f64::from(HAS_LENGTH | HAS_ANCHOR), 5.0, 0.0, 1.5, 2.0, 0.0]
        );
        out.clear();
        // A path without vertices has a length but no anchor; an unknown object nothing.
        measure_record(Some(&shape(r#"{"kind":"polyline","pts":[]}"#)), &mut out);
        measure_record(None, &mut out);
        assert_eq!(out[0], f64::from(HAS_LENGTH));
        assert_eq!(&out[MEASURE_STRIDE..], [0.0; MEASURE_STRIDE]);
    }
}

//! `cad.entities.fillet` v1 and `cad.entities.chamfer` v1 (docs/adr/0250):
//! a corner rounded (Köşe yuvarla) or cut (Pah), from what was asked (the
//! corner and its size, Kırp), as one undo step “Köşe yuvarla” (“Pah”).
//! The desktop's handler over the native document, run by the corner tools
//! and Tüm köşeleri yuvarla and Tüm köşelere pah (their preview is
//! [`plan_in`], their click [`execute`]), the headless server, Python and
//! MCP. The shared cases are `fixtures/commands/v1/cad.entities.fillet.json`
//! and `.chamfer.json`.
//!
//! Nothing is computed twice: the corner (`path_corner_at`,
//! `lines_corner_at`), the sides and the arc or the cut (`corner_in_path`,
//! `fillet_lines`, `chamfer_lines`, `fillet_arc`, `chamfer_line`) and every
//! corner of a path (`all_corners`) are the shared core's, as the tools
//! always took them; they are written through `cad.entities.edit`'s
//! `fillet` and `chamfer` operations ([`crate::edit`]).
//!
//! Two objects (docs/adr/0295): each side is a line or an open polyline's
//! first or last straight edge, whose free end moves to the arc or the cut;
//! the first and last edges of one open polyline close it, the arc or the
//! cut its closing edge.
//!
//! The checks, in order: `all` names objects; the ids; the size and the
//! picks finite; the size (not negative; above zero where something is
//! added); the expected revision; the objects; their layers unlocked; the
//! corner is one; the size fits it.

use kentos_commands::{
    CommandResult, CommandWarning, CornerPick, EditOperation, EntitiesChamfer, EntitiesCornerPlan,
    EntitiesCornered, EntitiesEdit, EntitiesFillet, Entity, EntityEdit,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::geom::intersect::closest_on_edge;
use kentos_geometry_core::jsmath::{PI, acos, cos, js_hypot, js_max, js_min, sin, tan};
use kentos_geometry_core::ops::curve_cuts::Geometry;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::ops::fillet::{
    Chamfer, Corner as Joined, CornerOp, Fillet, Seg, chamfer_lines, corner_in_path, fillet_lines,
};
use kentos_geometry_core::ops::reshape::all_corners;
use kentos_geometry_core::tools::editing::{
    CornerGeom, chamfer_line, fillet_arc, lines_corner_at, path_corner_at,
};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::edit;
use crate::geometry::{edit_geometry, shape};

/// A size: a fillet's radius, or a chamfer's two distances.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    Radius(f64),
    Cut(f64, f64),
}

impl Size {
    pub fn core(self) -> CornerOp {
        match self {
            Size::Radius(r) => CornerOp::Radius(r),
            Size::Cut(d1, d2) => CornerOp::Chamfer(d1, d2),
        }
    }

    fn operation(self) -> EditOperation {
        match self {
            Size::Radius(_) => EditOperation::Fillet,
            Size::Cut(..) => EditOperation::Chamfer,
        }
    }

    fn failed(self) -> &'static str {
        match self {
            Size::Radius(_) => codes::FILLET_FAILED,
            Size::Cut(..) => codes::CHAMFER_FAILED,
        }
    }

    /// Whether it adds nothing: a zero radius, two zero distances.
    fn nothing(self) -> bool {
        match self {
            Size::Radius(r) => r == 0.0,
            Size::Cut(d1, d2) => d1 == 0.0 && d2 == 0.0,
        }
    }
}

/// A command's request, whichever of the two.
pub struct Request<'a> {
    pub corner: &'a CornerPick,
    pub size: Size,
    pub trim: bool,
    pub expected_revision: Option<&'a str>,
}

impl<'a> From<&'a EntitiesFillet> for Request<'a> {
    fn from(f: &'a EntitiesFillet) -> Self {
        Request {
            corner: &f.corner,
            size: Size::Radius(f.radius),
            trim: f.trim.unwrap_or(true),
            expected_revision: f.expected_revision.as_deref(),
        }
    }
}

impl<'a> From<&'a EntitiesChamfer> for Request<'a> {
    fn from(c: &'a EntitiesChamfer) -> Self {
        Request {
            corner: &c.corner,
            size: Size::Cut(c.distance, c.second.unwrap_or(c.distance)),
            trim: c.trim.unwrap_or(true),
            expected_revision: c.expected_revision.as_deref(),
        }
    }
}

/// Where the corner is, in the drawing (the tools' `Site`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Site {
    /// Vertex `vertex` of the path at `slot`.
    Vertex { slot: Slot, vertex: usize },
    /// Two lines; each keeps the side of its pick point.
    Lines {
        first: Slot,
        pick1: Vec2,
        second: Slot,
        pick2: Vec2,
    },
}

/// A corner: where it is and its geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    pub site: Site,
    pub geom: CornerGeom,
}

impl Corner {
    pub fn slots(&self) -> Vec<Slot> {
        match self.site {
            Site::Vertex { slot, .. } => vec![slot],
            Site::Lines { first, second, .. } => vec![first, second],
        }
    }
}

/// A line object's segment.
pub fn line_of(e: &Entity) -> Option<Seg> {
    match e {
        Entity::Line(l) => Some(Seg {
            a: Vec2::new(l.a.x, l.a.y),
            b: Vec2::new(l.b.x, l.b.y),
        }),
        _ => None,
    }
}

/// A path's corner at vertex `i`, an area's vertices counted part after
/// part (docs/adr/0143): none at an open path's end, beside an arc or on a
/// straight run (the core's `path_corner_at`).
pub fn path_corner(slot: Slot, e: &Entity, i: usize) -> Option<Corner> {
    if !matches!(e, Entity::Polyline(_) | Entity::Polygon(_)) {
        return None;
    }
    let geom = path_corner_at(&shape(e), i)?;
    Some(Corner {
        site: Site::Vertex { slot, vertex: i },
        geom,
    })
}

/// A straight side of a two-object corner (docs/adr/0295): a line, or an open polyline's edge `edge` (from vertex
/// `edge` to `edge + 1`), as a segment in its own direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Side {
    pub seg: Seg,
    pub edge: Option<usize>,
}

/// Why a pick is not a side.
pub const NOT_STRAIGHT: &str = "İki düz kenar verin: çizgi ya da açık çoklu çizginin ilk ya da son kenarı; çoklu çizginin ve alanın iç köşesi kendi köşesiyle (vertex) verilir.";
const INNER_EDGE: &str = "Çoklu çizgide yalnız ilk ya da son kenar başka bir kenarla köşe yapar; iç köşe için köşenin kendisini (vertex) verin.";
const ARC_EDGE: &str = "Seçilen kenar yay; yalnız düz kenar köşe yapar.";
const INNER_END: &str = "Köşe çoklu çizginin serbest ucunda değil, iç köşesinde kalıyor; kenarın öbür yanına tıklayın ya da köşeyi kendi köşesiyle (vertex) verin.";
const SAME_END: &str = "Aynı uç iki kez seçildi; yolu kapatmak için ilk ve son kenarını seçin.";
const SAME_LINE: &str = "İkinci çizgi ilkinden farklı olmalı.";
/// Tüm köşeler's kinds.
const ALL_KINDS: &str =
    "Tüm köşeler çoklu çizgide, alanda ve uçları birbirine değen çizgilerde işlenir.";
const LINES_NO_CORNER: &str = "Çizgilerin buluştuğu yerde köşe yok: ya aynı doğrultudalar ya da bir noktada ikiden çok uç buluşuyor.";
const FITS_NONE: &str = "Hiçbir köşeye sığmadı; daha küçük bir değer verin ya da kenarları uzatın.";
const NO_CORNER: &str = "Çizgiler paralel ya da seçilen tarafta çizgi yok.";

/// The side `pick` names on `e`: a line, or an open polyline's edge nearest the pick, which must be its first or
/// last and straight.
pub fn side_of(e: &Entity, pick: Vec2) -> Result<Side, &'static str> {
    if let Some(seg) = line_of(e) {
        return Ok(Side { seg, edge: None });
    }
    let Entity::Polyline(path) = e else {
        return Err(NOT_STRAIGHT);
    };
    if path.closed || path.parts.is_some() || path.pts.len() < 2 {
        return Err(NOT_STRAIGHT);
    }
    let edges = entity_edges(&shape(e));
    let nearest = edges
        .iter()
        .enumerate()
        .map(|(i, edge)| (i, closest_on_edge(edge, pick).d))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
        .ok_or(NOT_STRAIGHT)?;
    let last = path.pts.len() - 2;
    if nearest != 0 && nearest != last {
        return Err(INNER_EDGE);
    }
    let bulge = path
        .bulges
        .as_ref()
        .and_then(|b| b.get(nearest))
        .copied()
        .unwrap_or(0.0);
    if bulge != 0.0 {
        return Err(ARC_EDGE);
    }
    let (a, b) = (path.pts[nearest], path.pts[nearest + 1]);
    Ok(Side {
        seg: Seg {
            a: Vec2::new(a.x, a.y),
            b: Vec2::new(b.x, b.y),
        },
        edge: Some(nearest),
    })
}

/// The vertex of a polyline side that the corner moves: the end the core does not keep (the kept end is the one
/// farthest along the side's direction away from the corner). Its free end, or why not.
fn moved_vertex(
    e: &Entity,
    side: Side,
    geom: &CornerGeom,
    u: Vec2,
) -> Result<Option<usize>, &'static str> {
    let Some(i) = side.edge else { return Ok(None) };
    let along = |p: Vec2| (p.x - geom.at.x) * u.x + (p.y - geom.at.y) * u.y;
    // As the core's `kept_side`: the kept end is `a` when it reaches at least as far.
    let moved = if along(side.seg.a) >= along(side.seg.b) {
        i + 1
    } else {
        i
    };
    let n = match e {
        Entity::Polyline(path) => path.pts.len(),
        _ => 0,
    };
    if moved == 0 || moved + 1 == n {
        Ok(Some(moved))
    } else {
        Err(INNER_END)
    }
}

/// The corner of two straight sides, each keeping the side of its pick (docs/adr/0295); why not when they are one
/// line, the same end of one path, parallel, or not straight sides.
pub fn lines_corner(
    doc: &Document,
    a: (Slot, Vec2),
    b: (Slot, Vec2),
) -> Result<Corner, &'static str> {
    let (ea, eb) = (
        doc.get(a.0).ok_or(NOT_STRAIGHT)?,
        doc.get(b.0).ok_or(NOT_STRAIGHT)?,
    );
    let s1 = side_of(ea, a.1)?;
    let s2 = side_of(eb, b.1)?;
    if a.0 == b.0 && (s1.edge.is_none() || s1.edge == s2.edge && ea_edges(ea) > 1) {
        return Err(if s1.edge.is_none() {
            SAME_LINE
        } else {
            SAME_END
        });
    }
    let geom =
        lines_corner_at(s1.seg.a, s1.seg.b, a.1, s2.seg.a, s2.seg.b, b.1).ok_or(NO_CORNER)?;
    let m1 = moved_vertex(ea, s1, &geom, geom.u1)?;
    let m2 = moved_vertex(eb, s2, &geom, geom.u2)?;
    if a.0 == b.0 && m1 == m2 {
        return Err(SAME_END);
    }
    Ok(Corner {
        site: Site::Lines {
            first: a.0,
            pick1: a.1,
            second: b.0,
            pick2: b.1,
        },
        geom,
    })
}

/// How many edges an open polyline has.
fn ea_edges(e: &Entity) -> usize {
    match e {
        Entity::Polyline(path) => path.pts.len().saturating_sub(1),
        _ => 0,
    }
}

/// What a size does to a corner: the sides' new geometry and the piece
/// added (the arc or the cut), made from the first side.
pub struct Done {
    pub updates: Vec<(Slot, Shape)>,
    pub add: Option<(Slot, Shape)>,
}

/// What `size` does to the corner with Kırp, or why it cannot.
pub fn done(c: &Corner, size: Size, doc: &Document) -> Result<Done, String> {
    match c.site {
        Site::Vertex { slot, vertex } => {
            let e = doc.get(slot).ok_or_else(String::new)?;
            // The whole geometry is written (docs/adr/0047): the corner's part keeps its
            // holes, the area its other parts (docs/adr/0143).
            match corner_in_path(&shape(e), vertex, &size.core())? {
                Geometry::Error(error) => Err(error),
                Geometry::Ok(done) => Ok(Done {
                    updates: vec![(slot, done.shape)],
                    add: None,
                }),
            }
        }
        Site::Lines {
            first,
            pick1,
            second,
            pick2,
        } => {
            let (e1, e2) = (
                doc.get(first).ok_or_else(String::new)?,
                doc.get(second).ok_or_else(String::new)?,
            );
            let s1 = side_of(e1, pick1).map_err(String::from)?;
            let s2 = side_of(e2, pick2).map_err(String::from)?;
            let (joined, piece) = match size {
                Size::Radius(r) => {
                    let Fillet(j) = fillet_lines(s1.seg, pick1, s2.seg, pick2, r);
                    match j {
                        Joined::Error(e) => return Err(e),
                        Joined::Ok { line1, line2, join } => (
                            (line1, line2),
                            join.map(|a| Shape::Arc {
                                c: a.c,
                                r: a.r,
                                a0: a.a0,
                                a1: a.a1,
                            }),
                        ),
                    }
                }
                Size::Cut(d1, d2) => {
                    let Chamfer(j) = chamfer_lines(s1.seg, pick1, s2.seg, pick2, d1, d2);
                    match j {
                        Joined::Error(e) => return Err(e),
                        Joined::Ok { line1, line2, join } => {
                            ((line1, line2), join.map(|s| Shape::Line { a: s.a, b: s.b }))
                        }
                    }
                }
            };
            if first == second {
                return Ok(Done {
                    updates: vec![(
                        first,
                        closed_path(e1, (s1, joined.0), (s2, joined.1), piece)?,
                    )],
                    add: None,
                });
            }
            Ok(Done {
                updates: vec![
                    (first, side_after(e1, s1, joined.0)?),
                    (second, side_after(e2, s2, joined.1)?),
                ],
                add: piece.map(|s| (first, s)),
            })
        }
    }
}

/// A side's new geometry from the core's kept segment (the kept end first, then the arc's or the cut's end): a line
/// keeps its direction, a polyline's free end moves.
fn side_after(e: &Entity, side: Side, kept: Seg) -> Result<Shape, String> {
    let Some(_) = side.edge else {
        // A side keeps its direction: the core gives the kept end first, which is the line's end when the corner
        // was at its start.
        let was = side.seg;
        let s = if kept.a == was.b && kept.a != was.a {
            Seg {
                a: kept.b,
                b: kept.a,
            }
        } else {
            kept
        };
        return Ok(Shape::Line { a: s.a, b: s.b });
    };
    let mut path = shape(e);
    let Shape::Polyline { pts, .. } = &mut path else {
        return Err(NOT_STRAIGHT.into());
    };
    let i = moved_index(side, kept, pts.len()).ok_or_else(|| String::from(INNER_END))?;
    pts[i] = kept.b;
    Ok(path)
}

/// Which vertex of a polyline side the kept segment moved: the one that is not its kept end.
fn moved_index(side: Side, kept: Seg, n: usize) -> Option<usize> {
    let i = side.edge?;
    let moved = if kept.a == side.seg.a { i + 1 } else { i };
    (moved == 0 || moved + 1 == n).then_some(moved)
}

/// One open polyline's first and last edges joined (docs/adr/0295): both free ends move, and the path closes over
/// the arc (its closing edge's bulge) or the cut; with no piece the ends meet and the last vertex goes.
fn closed_path(
    e: &Entity,
    one: (Side, Seg),
    two: (Side, Seg),
    piece: Option<Shape>,
) -> Result<Shape, String> {
    let mut path = shape(e);
    let Shape::Polyline {
        pts,
        bulges,
        closed,
        widths,
        ..
    } = &mut path
    else {
        return Err(NOT_STRAIGHT.into());
    };
    let n = pts.len();
    for (side, kept) in [one, two] {
        let i = moved_index(side, kept, n).ok_or_else(|| String::from(INNER_END))?;
        pts[i] = kept.b;
    }
    let mut bs = bulges.clone().unwrap_or_else(|| vec![0.0; n]);
    bs.resize(n, 0.0);
    match piece {
        Some(Shape::Arc { c, r, a0, a1 }) => {
            // The closing edge runs from the last vertex to the first: counter-clockwise when the arc starts there.
            let start = Vec2::new(c.x + r * cos(a0), c.y + r * sin(a0));
            let last = pts[n - 1];
            let first = pts[0];
            let near = |p: Vec2, q: Vec2| js_hypot(p.x - q.x, p.y - q.y);
            let ccw = near(start, last) <= near(start, first);
            let sweep = norm_angle(a1 - a0);
            let b = tan(sweep / 4.0);
            bs[n - 1] = if ccw { b } else { -b };
        }
        Some(_) => bs[n - 1] = 0.0,
        None => {
            // The ends met: the last vertex is the first.
            pts.pop();
            bs.truncate(n - 1);
            if let Some(w) = widths {
                w.truncate(n - 1);
            }
        }
    }
    *bulges = bs.iter().any(|b| *b != 0.0).then_some(bs);
    *closed = true;
    Ok(path)
}

/// What Tüm köşeleri yuvarla and Tüm köşelere pah do to lines whose ends meet (docs/adr/0295): every point where
/// exactly two of the lines' ends meet bit for bit, and they are not in line, is a corner; each line's end there moves
/// back by the corner's size, and the arc or the cut is a new object from the line given first. A corner that does
/// not fit its lines, or that with the corner at the line's other end needs more than the line, is left (both are
/// then); the counts are of corners.
#[derive(Clone, Debug, Default)]
pub struct LineCorners {
    /// Each line that changes, its new geometry.
    pub updates: Vec<(Slot, Shape)>,
    /// Each arc or cut, with the line it is made from.
    pub pieces: Vec<(Slot, Shape)>,
    pub done: usize,
    pub skipped: usize,
    /// The lines with no end meeting another's: no corner at all.
    pub alone: Vec<Slot>,
}

/// [`LineCorners`] of `lines` (in the order given, lines only) with `size`, every value above zero.
pub fn line_corners(doc: &Document, lines: &[Slot], size: Size) -> LineCorners {
    let segs: Vec<Seg> = lines
        .iter()
        .filter_map(|s| doc.get(*s).and_then(line_of))
        .collect();
    let ends = |k: usize| [segs[k].a, segs[k].b];
    // Every end, and the others that meet it bit for bit.
    let mut corners: Vec<((usize, usize), (usize, usize))> = Vec::new();
    let mut met = vec![[false; 2]; segs.len()];
    for (k, met_k) in met.iter_mut().enumerate() {
        for (e, met_ke) in met_k.iter_mut().enumerate() {
            let p = ends(k)[e];
            let mut at = Vec::new();
            for (j, _) in segs.iter().enumerate() {
                for f in 0..2 {
                    if (j, f) != (k, e) && ends(j)[f] == p {
                        at.push((j, f));
                    }
                }
            }
            *met_ke = !at.is_empty();
            // Two ends: one corner, named once from the earlier line.
            if let [(j, f)] = at[..]
                && (k, e) < (j, f)
                && j != k
            {
                corners.push(((k, e), (j, f)));
            }
        }
    }
    let mut out = LineCorners::default();
    for (k, m) in met.iter().enumerate() {
        if !m[0] && !m[1] {
            out.alone.push(lines[k]);
        }
    }
    // Each corner's geometry and sizes along its lines, or none when the lines are in line.
    let away = |k: usize, e: usize| {
        let (p, q) = (ends(k)[e], ends(k)[1 - e]);
        let l = js_hypot(q.x - p.x, q.y - p.y);
        (Vec2::new((q.x - p.x) / l, (q.y - p.y) / l), l)
    };
    let planned: Vec<Option<(CornerGeom, f64, f64)>> = corners
        .iter()
        .map(|&((k, e), (j, f))| {
            let ((u1, l1), (u2, l2)) = (away(k, e), away(j, f));
            let phi = acos(js_max(-1.0, js_min(1.0, u1.x * u2.x + u1.y * u2.y)));
            if !(1e-6..=PI - 1e-6).contains(&phi) {
                return None;
            }
            let (d1, d2) = match size {
                Size::Radius(r) => {
                    let d = r / tan(phi / 2.0);
                    (d, d)
                }
                Size::Cut(a, b) => (a, b),
            };
            let geom = CornerGeom {
                at: ends(k)[e],
                u1,
                u2,
                reach: js_min(l1, l2),
                phi,
            };
            Some((geom, d1, d2))
        })
        .collect();
    // What each line's end gives to its corner; a line whose two corners together need more than it leaves both.
    let mut take: Vec<bool> = planned
        .iter()
        .zip(&corners)
        .map(|(p, &((k, e), (j, f)))| {
            p.is_some_and(|(_, d1, d2)| {
                let (l1, l2) = (away(k, e).1, away(j, f).1);
                d1 > 0.0 && d2 > 0.0 && d1 <= l1 + 1e-9 && d2 <= l2 + 1e-9
            })
        })
        .collect();
    let mut need = vec![[0.0f64; 2]; segs.len()];
    let mut by = vec![[None::<usize>; 2]; segs.len()];
    for (c, &((k, e), (j, f))) in corners.iter().enumerate() {
        if let (true, Some((_, d1, d2))) = (take[c], planned[c]) {
            need[k][e] = d1;
            need[j][f] = d2;
            by[k][e] = Some(c);
            by[j][f] = Some(c);
        }
    }
    for k in 0..segs.len() {
        let l = js_hypot(segs[k].b.x - segs[k].a.x, segs[k].b.y - segs[k].a.y);
        if let [Some(c0), Some(c1)] = by[k]
            && need[k][0] + need[k][1] > l + 1e-9
        {
            take[c0] = false;
            take[c1] = false;
        }
    }
    let mut moved: Vec<[Option<Vec2>; 2]> = vec![[None; 2]; segs.len()];
    for (c, &((k, e), (j, f))) in corners.iter().enumerate() {
        let Some((geom, d1, d2)) = planned[c] else {
            continue;
        };
        if !take[c] {
            out.skipped += 1;
            continue;
        }
        let at = geom.at;
        moved[k][e] = Some(Vec2::new(at.x + geom.u1.x * d1, at.y + geom.u1.y * d1));
        moved[j][f] = Some(Vec2::new(at.x + geom.u2.x * d2, at.y + geom.u2.y * d2));
        let piece = match size {
            Size::Radius(r) => fillet_arc(&geom, r).map(|a| Shape::Arc {
                c: a.c,
                r: a.r,
                a0: norm_angle(a.a0),
                a1: norm_angle(a.a1),
            }),
            Size::Cut(..) => chamfer_line(&geom, d1, d2).map(|p| Shape::Line { a: p.a, b: p.b }),
        };
        if let Some(piece) = piece {
            out.pieces.push((lines[k], piece));
        }
        out.done += 1;
    }
    for (k, m) in moved.iter().enumerate() {
        if m[0].is_none() && m[1].is_none() {
            continue;
        }
        out.updates.push((
            lines[k],
            Shape::Line {
                a: m[0].unwrap_or(segs[k].a),
                b: m[1].unwrap_or(segs[k].b),
            },
        ));
    }
    out
}

/// The arc or the cut alone, without Kırp.
pub fn piece(c: &Corner, size: Size) -> Option<Shape> {
    match size {
        // Its angles in [0, 2π), as the trimmed corner's arc (`fillet_lines`).
        Size::Radius(r) => fillet_arc(&c.geom, r).map(|a| Shape::Arc {
            c: a.c,
            r: a.r,
            a0: norm_angle(a.a0),
            a1: norm_angle(a.a1),
        }),
        Size::Cut(d1, d2) => chamfer_line(&c.geom, d1, d2).map(|p| Shape::Line { a: p.a, b: p.b }),
    }
}

/// Checks the request against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, request: Request<'_>) -> CommandResult<()> {
    match check(cx.doc, &request) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, request: Request<'_>) -> CommandResult<EntitiesCornerPlan> {
    plan_in(cx.doc, request)
}

/// [`plan`] over the document alone (the tools' preview).
pub fn plan_in(doc: &Document, request: Request<'_>) -> CommandResult<EntitiesCornerPlan> {
    let checked = match check(doc, &request) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    match edit::plan_in(doc, &checked.edit) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: EntitiesCornerPlan {
                changed: output.changed,
                created: output.created,
                done: checked.done,
                skipped: checked.skipped,
                revision: output.revision,
            },
            warnings: [checked.warnings, warnings].concat(),
        },
        other => relay(other),
    }
}

/// Checks the request again and writes it as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    request: Request<'_>,
) -> CommandResult<EntitiesCornered> {
    let checked = match check(cx.doc, &request) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    match edit::execute(cx, checked.edit) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: EntitiesCornered {
                changed: output.changed,
                created: output.created,
                done: checked.done,
                skipped: checked.skipped,
                revision: output.revision,
            },
            warnings: [checked.warnings, warnings].concat(),
        },
        other => relay(other),
    }
}

fn relay<T, U>(result: CommandResult<T>) -> CommandResult<U> {
    match result {
        CommandResult::Completed { .. } | CommandResult::Cancelled => CommandResult::Cancelled,
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::Failed { error } => CommandResult::Failed { error },
    }
}

struct Checked {
    edit: EntitiesEdit,
    done: u32,
    skipped: u32,
    warnings: Vec<CommandWarning>,
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn uid_of(doc: &Document, slot: Slot) -> String {
    doc.uid(slot).map(|u| u.to_string()).unwrap_or_default()
}

fn check(doc: &Document, r: &Request<'_>) -> Result<Checked, Stop> {
    let size = r.size;
    // The ids, with where each is named.
    let named: Vec<(&str, String)> = match r.corner {
        CornerPick::Vertex { uid, .. } => vec![(uid.as_str(), "corner.uid".into())],
        CornerPick::Lines { first, second } => vec![
            (first.uid.as_str(), "corner.first.uid".into()),
            (second.uid.as_str(), "corner.second.uid".into()),
        ],
        CornerPick::All { uids } => uids
            .iter()
            .enumerate()
            .map(|(i, u)| (u.as_str(), format!("corner.uids[{i}]")))
            .collect(),
    };
    if named.is_empty() {
        return Err(refuse(
            codes::NO_ENTITIES,
            "Köşeleri işlenecek nesne yok. En az bir çoklu çizgi ya da alan verin.".into(),
            "corner.uids",
        ));
    }
    for (uid, path) in &named {
        if !is_uid_text(uid) {
            return Err(refuse(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                ),
                path,
            ));
        }
    }
    let (what, field) = match size {
        Size::Radius(_) => ("Yarıçap", "radius"),
        Size::Cut(..) => ("Pah mesafesi", "distance"),
    };
    let values: Vec<(f64, &str)> = match size {
        Size::Radius(radius) => vec![(radius, field)],
        Size::Cut(d1, d2) => vec![(d1, field), (d2, "second")],
    };
    for (v, path) in &values {
        checks::finite(*v, what, "Sonlu bir sayı yazın.", path)?;
    }
    if let CornerPick::Lines { first, second } = r.corner {
        checks::point(
            first.at,
            "Birinci çizginin tıklanan noktasının",
            "corner.first.at",
        )?;
        checks::point(
            second.at,
            "İkinci çizginin tıklanan noktasının",
            "corner.second.at",
        )?;
    }
    let all = matches!(r.corner, CornerPick::All { .. });
    for (v, path) in &values {
        if *v < 0.0 {
            return Err(refuse(
                codes::INVALID_DISTANCE,
                format!("{what} eksi olamaz; sıfır ya da sıfırdan büyük bir değer yazın."),
                path,
            ));
        }
    }
    // Every corner needs every value above zero (the core's `all_corners`).
    let positive = match size {
        Size::Radius(radius) => radius > 0.0,
        Size::Cut(d1, d2) => d1 > 0.0 && d2 > 0.0,
    };
    if (all && !positive) || (size.nothing() && !r.trim) {
        let why = if all {
            "Tüm köşelerde her değer sıfırdan büyük olmalı; sıfır hiçbir köşeyi değiştirmez."
        } else {
            "Kırpmadan çalışırken sıfırdan büyük bir boyut verin; yoksa eklenecek bir şey yok."
        };
        return Err(refuse(codes::INVALID_DISTANCE, why.into(), field));
    }
    checks::revision(doc, r.expected_revision)?;
    let mut slots = Vec::with_capacity(named.len());
    for (uid, path) in &named {
        let found = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u));
        let Some(slot) = found else {
            return Err(refuse(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                path,
            ));
        };
        slots.push(slot);
    }
    let layers = doc.layers();
    for (slot, (_, path)) in slots.iter().zip(&named) {
        let Some(e) = doc.get(*slot) else { continue };
        let layer = &e.base().layer_id;
        if layers.is_locked(layer) {
            let name = layers
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(refuse(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."
                ),
                path,
            ));
        }
    }
    let operation = size.operation();
    let failed = size.failed();
    let mut warnings = Vec::new();
    let (mut changes, mut done_n, mut skipped) = (Vec::new(), 0u32, 0u32);
    match r.corner {
        CornerPick::All { .. } => {
            let mut first_why: Option<String> = None;
            // Lines among them: the corners where their ends meet (docs/adr/0295).
            let lines: Vec<Slot> = slots
                .iter()
                .copied()
                .filter(|s| doc.get(*s).and_then(line_of).is_some())
                .collect();
            let found = line_corners(doc, &lines, size);
            if let Some(alone) = found.alone.first() {
                let at = slots.iter().position(|s| s == alone).unwrap_or(0);
                return Err(refuse(codes::NOT_A_CORNER, ALL_KINDS.into(), &named[at].1));
            }
            if !lines.is_empty() && found.done == 0 && found.skipped == 0 {
                let at = slots.iter().position(|s| *s == lines[0]).unwrap_or(0);
                return Err(refuse(
                    codes::NOT_A_CORNER,
                    LINES_NO_CORNER.into(),
                    &named[at].1,
                ));
            }
            if !lines.is_empty() {
                skipped += found.skipped as u32;
                done_n += found.done as u32;
                if found.done == 0 {
                    let why = FITS_NONE.to_owned();
                    first_why.get_or_insert_with(|| why.clone());
                    let at = slots.iter().position(|s| *s == lines[0]).unwrap_or(0);
                    warnings.push(CommandWarning {
                        code: failed.into(),
                        message: why,
                        path: Some(named[at].1.clone()),
                    });
                }
                for (slot, s) in &found.updates {
                    if let Some(geometry) = edit_geometry(s.clone()) {
                        changes.push(EntityEdit::Update {
                            uid: uid_of(doc, *slot),
                            geometry,
                        });
                    }
                }
                for (slot, s) in &found.pieces {
                    if let Some(geometry) = edit_geometry(s.clone()) {
                        changes.push(EntityEdit::add(uid_of(doc, *slot), geometry, None));
                    }
                }
            }
            for (slot, (_, path)) in slots.iter().zip(&named) {
                if lines.contains(slot) {
                    continue;
                }
                let e = doc
                    .get(*slot)
                    .ok_or_else(|| refuse(codes::NOT_A_CORNER, String::new(), path))?;
                let reshaped = matches!(e, Entity::Polyline(_) | Entity::Polygon(_))
                    .then(|| all_corners(&CoreEntity::new(shape(e)), &size.core()))
                    .flatten();
                let Some(rs) = reshaped else {
                    return Err(refuse(codes::NOT_A_CORNER, ALL_KINDS.into(), path));
                };
                skipped += rs.skipped as u32;
                if rs.done == 0 {
                    let why = FITS_NONE.to_owned();
                    first_why.get_or_insert_with(|| why.clone());
                    warnings.push(CommandWarning {
                        code: failed.into(),
                        message: why,
                        path: Some(path.clone()),
                    });
                    continue;
                }
                done_n += rs.done as u32;
                if let Some(geometry) = edit_geometry(rs.entity.shape) {
                    changes.push(EntityEdit::Update {
                        uid: uid_of(doc, *slot),
                        geometry,
                    });
                }
            }
            if changes.is_empty() {
                let why = first_why.unwrap_or_default();
                return Err(refuse(failed, why, "corner.uids"));
            }
        }
        CornerPick::Vertex { index, .. } => {
            let slot = slots[0];
            let corner = doc
                .get(slot)
                .and_then(|e| path_corner(slot, e, *index as usize))
                .ok_or_else(|| {
                    refuse(
                        codes::NOT_A_CORNER,
                        "Bu köşe işlenemez: köşe yolun dışında, açık yolun ucunda ya da bir yayın yanında; yalnızca düz kenarlar arasındaki köşe işlenir.".into(),
                        "corner.index",
                    )
                })?;
            add_corner(doc, &corner, size, r.trim, &mut changes)
                .map_err(|why| refuse(failed, why, "corner"))?;
            done_n = 1;
        }
        CornerPick::Lines { first, second } => {
            let a = (slots[0], Vec2::new(first.at.x, first.at.y));
            let b = (slots[1], Vec2::new(second.at.x, second.at.y));
            let corner = lines_corner(doc, a, b)
                .map_err(|why| refuse(codes::NOT_A_CORNER, why.into(), "corner"))?;
            add_corner(doc, &corner, size, r.trim, &mut changes)
                .map_err(|why| refuse(failed, why, "corner"))?;
            done_n = 1;
        }
    }
    Ok(Checked {
        edit: EntitiesEdit {
            operation,
            changes,
            expected_revision: None,
        },
        done: done_n,
        skipped,
        warnings,
    })
}

/// The changes that round or cut one corner (the corner tools' `commit`).
fn add_corner(
    doc: &Document,
    c: &Corner,
    size: Size,
    trim: bool,
    changes: &mut Vec<EntityEdit>,
) -> Result<(), String> {
    let done = done(c, size, doc)?;
    if !trim {
        let Some(piece) = piece(c, size) else {
            return Err("Ölçü köşeye sığmıyor; daha küçük bir değer verin.".into());
        };
        let from = uid_of(doc, c.slots()[0]);
        if let Some(geometry) = edit_geometry(piece) {
            changes.push(EntityEdit::add(from, geometry, None));
        }
        return Ok(());
    }
    for (slot, s) in &done.updates {
        if let Some(geometry) = edit_geometry(s.clone()) {
            changes.push(EntityEdit::Update {
                uid: uid_of(doc, *slot),
                geometry,
            });
        }
    }
    if let Some((like, s)) = &done.add
        && let Some(geometry) = edit_geometry(s.clone())
    {
        changes.push(EntityEdit::add(uid_of(doc, *like), geometry, None));
    }
    Ok(())
}

fn norm_angle(a: f64) -> f64 {
    let r = a % core::f64::consts::TAU;
    if r < 0.0 {
        r + core::f64::consts::TAU
    } else {
        r
    }
}

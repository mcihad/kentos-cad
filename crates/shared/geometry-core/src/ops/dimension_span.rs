//! Façade, wall-group and space dimension rows (docs/adr/0729): the points a persistent recipe measures along one
//! direction: the walls a user chose (with the walls at their ends and the walls meeting them), their openings, the
//! grids across them, or a space's outline. Pure: the host gives the candidates and maps source numbers to its
//! identities; the rows' spans are ordinary linear dimensions along the direction.

use crate::geom::wall::{self, Line, Spec};
use crate::geom::wall_join::JOIN_TOLERANCE;
use crate::jsmath::{PI, atan2, js_hypot, js_max, js_min};
use crate::ops::dimension_set::{Openings, Point, Row, Span};
use crate::ops::subref::Place;
use crate::predicates::orient2d;
use crate::vec2::Vec2;

/// How square an edge must be to count as parallel or perpendicular to the direction: the sine of its angle off.
pub const SQUARE: f64 = 1e-6;
/// Points this close along the direction are one.
pub const SAME: f64 = 1e-9;

/// Which of a wall's lines a recipe measures (docs/adr/0729 §1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Reference {
    /// Its two faces: its first and last line.
    #[default]
    Faces,
    /// Its core: the two boundaries of its strongest components' band; its faces without components.
    Core,
}

/// A wall the host gives: the ones the user chose (`chosen`) and the walls near them on their level.
#[derive(Clone, Copy, Debug)]
pub struct WallIn<'a> {
    pub source: usize,
    pub pts: &'a [Vec2],
    pub bulges: Option<&'a [f64]>,
    pub spec: &'a Spec,
    pub chosen: bool,
}

/// An opening: its jambs (start-left, start-right, end-left, end-right along its wall; docs/adr/0694), its wall (an
/// index into the walls given: a chosen wall's openings are measured), a flue (never measured), a niche's face
/// (`Some(true)`: its wall's right face).
#[derive(Clone, Copy, Debug)]
pub struct OpeningIn {
    pub source: usize,
    pub host: Option<usize>,
    pub jambs: [Vec2; 4],
    pub flue: bool,
    pub niche_right: Option<bool>,
}

/// A grid: its two ends and its bulge (a curved grid is never measured).
#[derive(Clone, Copy, Debug)]
pub struct GridIn<'a> {
    pub source: usize,
    pub pts: &'a [Vec2],
    pub bulges: Option<&'a [f64]>,
}

/// A space's boundary: per part its outer ring, then its holes, each its vertices and their bulges.
#[derive(Clone, Debug)]
pub struct SpaceIn {
    pub source: usize,
    pub parts: Vec<Vec<(Vec<Vec2>, Vec<f64>)>>,
}

/// What the recipe asks: the direction measured, the lines, the openings' points, the rows (from the source out), the
/// first row's place, the rows' spacing (world) and the rows placed by hand.
#[derive(Clone, Copy, Debug)]
pub struct Recipe<'a> {
    pub direction: Vec2,
    pub reference: Reference,
    pub openings: Openings,
    pub rows: &'a [Row],
    pub at: Vec2,
    pub gap: f64,
    pub row_places: &'a [(Row, Vec2)],
}

/// Why a chosen wall or the space cannot be measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// An edge neither parallel nor perpendicular to the direction.
    Oblique,
    /// A curved edge.
    Arc,
    /// It cannot be drawn, or a line of it has no vertex per path vertex (a bevelled corner).
    Drawn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The direction is zero or not finite, a place or the spacing not finite.
    Direction,
    /// A chosen wall or the space (its source number) cannot be measured.
    Unsupported(usize, Why),
    /// Nothing to measure: no wall chosen, no space.
    Nothing,
}

/// A row's account: its points, the objects they come from, the spans drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct RowNote {
    pub row: Row,
    pub points: usize,
    pub sources: usize,
    pub spans: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Planned {
    /// The rows' spans, row by row in the recipe's order, each row's along the direction.
    pub spans: Vec<Span>,
    pub rows: Vec<RowNote>,
    /// The walls near the chosen ones left unmeasured (an oblique or curved edge where they meet): their sources.
    pub skipped: Vec<usize>,
    /// The side the rows go to: +1 to the direction's left, −1 to its right.
    pub side: f64,
    /// The frame the rows are placed in: its origin, the direction (a unit vector) and the bodies' reach across it
    /// (the least and the greatest `o`), the faces a row's distance is counted from.
    pub origin: Vec2,
    pub direction: Vec2,
    pub body: (f64, f64),
}

/// What a point is, its order among points at one place along the direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Line,
    Opening,
    Grid,
}

#[derive(Clone, Copy, Debug)]
struct Pt {
    point: Point,
    t: f64,
    o: f64,
    kind: Kind,
}

impl Pt {
    /// The order of points at one place: lines, openings, grids; then the source, then the place.
    fn key(&self) -> (Kind, usize, [usize; 3]) {
        let place = match self.point.place {
            Place::Vertex { part, ring, index } => [part, ring, index],
            Place::Start => [usize::MAX - 2, 0, 0],
            Place::End => [usize::MAX - 1, 0, 0],
            _ => [usize::MAX, 0, 0],
        };
        (self.kind, self.point.source, place)
    }
}

/// The direction's frame from an origin near the drawing: along (`t`) and across (`o`, to the left).
#[derive(Clone, Copy, Debug)]
struct Frame {
    origin: Vec2,
    d: Vec2,
    n: Vec2,
}

impl Frame {
    fn t(&self, p: Vec2) -> f64 {
        (p.x - self.origin.x) * self.d.x + (p.y - self.origin.y) * self.d.y
    }
    fn o(&self, p: Vec2) -> f64 {
        (p.x - self.origin.x) * self.n.x + (p.y - self.origin.y) * self.n.y
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Parallel,
    Perpendicular,
    Oblique,
    Arc,
}

fn classify(a: Vec2, b: Vec2, bulge: f64, frame: &Frame) -> Edge {
    if bulge != 0.0 {
        return Edge::Arc;
    }
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l = js_hypot(dx, dy);
    if !l.is_finite() || l <= 0.0 {
        return Edge::Oblique;
    }
    let (ux, uy) = (dx / l, dy / l);
    if (ux * frame.n.x + uy * frame.n.y).abs() <= SQUARE {
        Edge::Parallel
    } else if (ux * frame.d.x + uy * frame.d.y).abs() <= SQUARE {
        Edge::Perpendicular
    } else {
        Edge::Oblique
    }
}

/// A wall read for measuring: its lines, the reference lines, its path's edges and its straight edges' bands.
struct Wall<'a> {
    w: WallIn<'a>,
    lines: Vec<Line>,
    refs: [usize; 2],
    /// Each path edge: its first and second vertex and its kind.
    edges: Vec<(usize, usize, Edge)>,
    /// Each straight edge's band between the faces: its edge number and its corners.
    quads: Vec<(usize, [Vec2; 4])>,
}

/// The reference lines of a wall of these lines (docs/adr/0729 §1): its faces, or its strongest components' band.
fn reference_lines(spec: &Spec, lines: usize, reference: Reference) -> [usize; 2] {
    let last = lines - 1;
    if reference == Reference::Faces || lines != spec.components.len() + 1 {
        return [0, last];
    }
    let ranks: Vec<u8> = spec.components.iter().map(|c| c.rank()).collect();
    let Some(&least) = ranks.iter().min() else {
        return [0, last];
    };
    let first = ranks.iter().position(|r| *r == least).unwrap_or(0);
    let end = ranks.iter().rposition(|r| *r == least).unwrap_or(last - 1);
    [first, end + 1]
}

fn wall_of<'a>(w: WallIn<'a>, frame: &Frame, reference: Reference) -> Result<Wall<'a>, Why> {
    let n = w.pts.len();
    let lines = wall::lines(w.pts, w.bulges, w.spec).map_err(|_| Why::Drawn)?;
    if n < 2 || lines.len() < 2 || lines.iter().any(|l| l.pts.len() != n) {
        return Err(Why::Drawn);
    }
    let refs = reference_lines(w.spec, lines.len(), reference);
    let count = if w.spec.closed { n } else { n - 1 };
    let bulge = |k: usize| w.bulges.and_then(|b| b.get(k)).copied().unwrap_or(0.0);
    let mut edges = Vec::with_capacity(count);
    let mut quads = Vec::new();
    let (left, right) = (&lines[0], &lines[lines.len() - 1]);
    for k in 0..count {
        let to = (k + 1) % n;
        let kind = classify(w.pts[k], w.pts[to], bulge(k), frame);
        edges.push((k, to, kind));
        if kind != Edge::Arc {
            quads.push((k, [left.pts[k], left.pts[to], right.pts[to], right.pts[k]]));
        }
    }
    Ok(Wall {
        w,
        lines,
        refs,
        edges,
        quads,
    })
}

/// Whether `p` is in the convex quad (on its sides too, within `SAME`).
fn in_quad(p: Vec2, q: &[Vec2; 4]) -> bool {
    let area = (0..4)
        .map(|i| {
            let (a, b) = (q[i], q[(i + 1) % 4]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    (0..4).all(|i| {
        let (a, b) = (q[i], q[(i + 1) % 4]);
        let l = js_hypot(b.x - a.x, b.y - a.y);
        l <= 0.0 || sign * orient2d(a, b, p) / l >= -SAME
    })
}

/// The edges of `x` a point touches: an end of its path within the join tolerance (its end edge), or its body's
/// band of an edge (that edge).
fn touching(p: Vec2, x: &Wall<'_>) -> Vec<usize> {
    let mut out = Vec::new();
    let n = x.w.pts.len();
    if !x.w.spec.closed {
        for (end, edge) in [(0, 0), (n - 1, n - 2)] {
            let e = x.w.pts[end];
            if js_hypot(p.x - e.x, p.y - e.y) <= JOIN_TOLERANCE && !out.contains(&edge) {
                out.push(edge);
            }
        }
    }
    for (k, q) in &x.quads {
        if in_quad(p, q) && !out.contains(k) {
            out.push(*k);
        }
    }
    out
}

/// Whether two segments cross at a point inside both.
fn crosses(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let (o1, o2) = (orient2d(a, b, c), orient2d(a, b, d));
    let (o3, o4) = (orient2d(c, d, a), orient2d(c, d, b));
    o1 * o2 < 0.0 && o3 * o4 < 0.0
}

/// The points of an edge `k` of `x`'s reference lines (docs/adr/0729 §2): each line's end of the edge farther on `side`.
fn edge_points(x: &Wall<'_>, k: usize, frame: &Frame, side: f64) -> Vec<Pt> {
    let (from, to, _) = x.edges[k];
    x.refs
        .iter()
        .map(|&part| {
            let (a, b) = (x.lines[part].pts[from], x.lines[part].pts[to]);
            let (sa, sb) = (side * frame.o(a), side * frame.o(b));
            let (index, at) = if sb > sa || (sb == sa && to < from) {
                (to, b)
            } else {
                (from, a)
            };
            Pt {
                point: Point {
                    source: x.w.source,
                    place: Place::Vertex {
                        part,
                        ring: 0,
                        index,
                    },
                    at,
                },
                t: frame.t(at),
                o: frame.o(at),
                kind: Kind::Line,
            }
        })
        .collect()
}

/// A point of a wall's lines at its path vertex `v`: the reference line's vertex farther on `side`.
fn end_point(x: &Wall<'_>, v: usize, frame: &Frame, side: f64) -> Pt {
    let mut best: Option<Pt> = None;
    for &part in &x.refs {
        let at = x.lines[part].pts[v];
        let pt = Pt {
            point: Point {
                source: x.w.source,
                place: Place::Vertex {
                    part,
                    ring: 0,
                    index: v,
                },
                at,
            },
            t: frame.t(at),
            o: frame.o(at),
            kind: Kind::Line,
        };
        if best.is_none_or(|b| side * pt.o > side * b.o) {
            best = Some(pt);
        }
    }
    // `refs` is never empty: a wall has two lines at least.
    best.unwrap_or(Pt {
        point: Point {
            source: x.w.source,
            place: Place::Vertex {
                part: 0,
                ring: 0,
                index: v,
            },
            at: x.w.pts[v],
        },
        t: frame.t(x.w.pts[v]),
        o: frame.o(x.w.pts[v]),
        kind: Kind::Line,
    })
}

/// Whether a point is hidden from a viewer far off on `side`: a band other than `own` holds its place along the
/// direction strictly inside and reaches farther toward the viewer.
fn hidden(p: &Pt, bands: &[[Vec2; 4]], own: &[[Vec2; 4]], frame: &Frame, side: f64) -> bool {
    bands.iter().filter(|q| !own.contains(q)).any(|q| {
        let ts: Vec<f64> = q.iter().map(|c| frame.t(*c)).collect();
        let (lo, hi) = ts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), t| {
            (js_min(lo, *t), js_max(hi, *t))
        });
        if !(p.t > lo + SAME && p.t < hi - SAME) {
            return false;
        }
        // The band's reach across at the point's place along: where its sides meet that cross line.
        let mut far = f64::NEG_INFINITY;
        for i in 0..4 {
            let (a, b) = (q[i], q[(i + 1) % 4]);
            let (ta, tb) = (ts[i], ts[(i + 1) % 4]);
            if (ta - p.t) * (tb - p.t) > 0.0 {
                continue;
            }
            let (oa, ob) = (frame.o(a), frame.o(b));
            if ta == tb {
                far = js_max(js_max(far, side * oa), side * ob);
            } else {
                let o = oa + (p.t - ta) / (tb - ta) * (ob - oa);
                far = js_max(far, side * o);
            }
        }
        far > side * p.o + SAME
    })
}

/// The points along, sorted, one per place (the first in the order of lines, openings, grids, then sources).
fn settle(mut pts: Vec<Pt>) -> Vec<Pt> {
    pts.sort_by(|a, b| a.t.total_cmp(&b.t).then_with(|| a.key().cmp(&b.key())));
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last().is_some_and(|q| p.t - q.t <= SAME) {
            continue;
        }
        out.push(p);
    }
    out
}

/// The rows' points of a façade (docs/adr/0729 §2): rows `openings`, `walls`, `grids`, `overall`.
struct Rows {
    openings: Vec<Pt>,
    walls: Vec<Pt>,
    extent: Vec<Pt>,
    grids: Vec<Pt>,
    skipped: Vec<usize>,
    body: (f64, f64),
}

fn facade(
    recipe: &Recipe<'_>,
    walls: &[WallIn<'_>],
    openings: &[OpeningIn],
    frame: &Frame,
    side_of: impl Fn((f64, f64)) -> f64,
) -> Result<(Rows, f64), Fault> {
    let mut read: Vec<Option<Wall<'_>>> = Vec::with_capacity(walls.len());
    for w in walls {
        match wall_of(*w, frame, recipe.reference) {
            Ok(x) => {
                if w.chosen
                    && let Some((_, _, kind)) =
                        x.edges.iter().find(|(_, _, k)| matches!(k, Edge::Oblique | Edge::Arc))
                {
                    let why = if *kind == Edge::Arc { Why::Arc } else { Why::Oblique };
                    return Err(Fault::Unsupported(w.source, why));
                }
                read.push(Some(x));
            }
            Err(why) if w.chosen => return Err(Fault::Unsupported(w.source, why)),
            Err(_) => read.push(None),
        }
    }
    let chosen: Vec<usize> = (0..read.len())
        .filter(|&i| read[i].as_ref().is_some_and(|x| x.w.chosen))
        .collect();
    if chosen.is_empty() {
        return Err(Fault::Nothing);
    }
    // The bodies' reach across: the side the rows go to.
    let mut body = (f64::INFINITY, f64::NEG_INFINITY);
    for &i in &chosen {
        if let Some(x) = &read[i] {
            for l in [&x.lines[0], &x.lines[x.lines.len() - 1]] {
                for p in &l.pts {
                    let o = frame.o(*p);
                    body = (js_min(body.0, o), js_max(body.1, o));
                }
            }
        }
    }
    let side = side_of(body);
    let mut lines: Vec<Pt> = Vec::new();
    let mut free: Vec<Pt> = Vec::new();
    let mut walls_row: Vec<Pt> = Vec::new();
    // The bands seen from the side: the chosen walls' and the end walls' edges where they meet a chosen end.
    let mut bands: Vec<[Vec2; 4]> = Vec::new();
    let mut skipped: Vec<usize> = Vec::new();
    let mut own_of: Vec<(Pt, Vec<[Vec2; 4]>)> = Vec::new();
    for &i in &chosen {
        let Some(x) = &read[i] else { continue };
        for (k, q) in &x.quads {
            bands.push(*q);
            if x.edges[*k].2 == Edge::Perpendicular {
                for p in edge_points(x, *k, frame, side) {
                    own_of.push((p, vec![*q]));
                    lines.push(p);
                }
            }
        }
    }
    // A chosen wall's open end: on another chosen wall nothing; at a wall perpendicular there that wall's lines;
    // else a free end.
    let mut ends_at: Vec<(usize, usize)> = Vec::new();
    for &i in &chosen {
        let Some(x) = &read[i] else { continue };
        if x.w.spec.closed {
            continue;
        }
        let n = x.w.pts.len();
        for (v, edge) in [(0, 0), (n - 1, n - 2)] {
            if x.edges[edge].2 != Edge::Parallel {
                continue;
            }
            let e = x.w.pts[v];
            let on_chosen = chosen
                .iter()
                .filter(|&&j| j != i)
                .any(|&j| read[j].as_ref().is_some_and(|y| !touching(e, y).is_empty()));
            if on_chosen {
                continue;
            }
            let mut walled = false;
            for (j, y) in read.iter().enumerate() {
                let Some(y) = y else { continue };
                if y.w.chosen {
                    continue;
                }
                let at = touching(e, y);
                if at.is_empty() {
                    continue;
                }
                if at
                    .iter()
                    .any(|&k| matches!(y.edges[k].2, Edge::Oblique | Edge::Arc))
                {
                    if !skipped.contains(&y.w.source) {
                        skipped.push(y.w.source);
                    }
                    continue;
                }
                for &k in &at {
                    if y.edges[k].2 == Edge::Perpendicular {
                        walled = true;
                        ends_at.push((j, k));
                        let own = y.quads.iter().find(|(m, _)| *m == k).map(|(_, q)| *q);
                        if let Some(q) = own {
                            bands.push(q);
                        }
                        for p in edge_points(y, k, frame, side) {
                            own_of.push((p, own.into_iter().collect()));
                            lines.push(p);
                        }
                    }
                }
            }
            if !walled {
                let p = end_point(x, v, frame, side);
                let own: Vec<[Vec2; 4]> = x
                    .quads
                    .iter()
                    .filter(|(m, _)| *m == edge)
                    .map(|(_, q)| *q)
                    .collect();
                own_of.push((p, own));
                free.push(p);
            }
        }
    }
    // The walls meeting the chosen ones: an end on one, or a path across one; their touching edges.
    for (j, y) in read.iter().enumerate() {
        let Some(y) = y else { continue };
        if y.w.chosen {
            continue;
        }
        let mut meet: Vec<usize> = ends_at
            .iter()
            .filter(|(m, _)| *m == j)
            .map(|(_, k)| *k)
            .collect();
        let n = y.w.pts.len();
        if !y.w.spec.closed {
            for (v, edge) in [(0, 0), (n - 1, n - 2)] {
                let e = y.w.pts[v];
                if chosen
                    .iter()
                    .any(|&i| read[i].as_ref().is_some_and(|x| !touching(e, x).is_empty()))
                    && !meet.contains(&edge)
                {
                    meet.push(edge);
                }
            }
        }
        for (k, (from, to, kind)) in y.edges.iter().enumerate() {
            if *kind == Edge::Arc || meet.contains(&k) {
                continue;
            }
            let (a, b) = (y.w.pts[*from], y.w.pts[*to]);
            let across = chosen.iter().any(|&i| {
                read[i].as_ref().is_some_and(|x| {
                    x.edges.iter().any(|(f, t, kd)| {
                        *kd != Edge::Arc && crosses(a, b, x.w.pts[*f], x.w.pts[*t])
                    })
                })
            });
            if across {
                meet.push(k);
            }
        }
        if meet.is_empty() {
            continue;
        }
        if meet
            .iter()
            .any(|&k| matches!(y.edges[k].2, Edge::Oblique | Edge::Arc))
        {
            if !skipped.contains(&y.w.source) {
                skipped.push(y.w.source);
            }
            continue;
        }
        for k in meet {
            if y.edges[k].2 == Edge::Perpendicular {
                walls_row.extend(edge_points(y, k, frame, side));
            }
        }
    }
    let mut extent: Vec<Pt> = lines.clone();
    extent.extend(free.iter().copied());
    let settled = settle(extent.clone());
    let (lo, hi) = match (settled.first(), settled.last()) {
        (Some(a), Some(b)) => (a.t, b.t),
        _ => return Err(Fault::Nothing),
    };
    // The openings of the chosen walls, along the direction, their jambs on the face toward the side.
    let mut opening_pts: Vec<(Pt, Vec<[Vec2; 4]>)> = Vec::new();
    for o in openings {
        let Some(h) = o.host else { continue };
        let Some(Some(x)) = read.get(h) else { continue };
        if !x.w.chosen || o.flue || !o.jambs.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
            continue;
        }
        if classify(o.jambs[0], o.jambs[2], 0.0, frame) != Edge::Parallel {
            continue;
        }
        let left_near = side * frame.o(o.jambs[0]) >= side * frame.o(o.jambs[1]);
        if let Some(right) = o.niche_right
            && right == left_near
        {
            continue;
        }
        let mid = Vec2::new(
            (o.jambs[0].x + o.jambs[1].x + o.jambs[2].x + o.jambs[3].x) * 0.25,
            (o.jambs[0].y + o.jambs[1].y + o.jambs[2].y + o.jambs[3].y) * 0.25,
        );
        let own: Vec<[Vec2; 4]> = x
            .quads
            .iter()
            .filter(|(_, q)| in_quad(mid, q))
            .map(|(_, q)| *q)
            .collect();
        let mut add = |place: Place, at: Vec2| {
            opening_pts.push((
                Pt {
                    point: Point {
                        source: o.source,
                        place,
                        at,
                    },
                    t: frame.t(at),
                    o: frame.o(at),
                    kind: Kind::Opening,
                },
                own.clone(),
            ));
        };
        match recipe.openings {
            Openings::Ends => {
                for j in if left_near { [0, 2] } else { [1, 3] } {
                    add(
                        Place::Vertex {
                            part: 0,
                            ring: 0,
                            index: j,
                        },
                        o.jambs[j],
                    );
                }
            }
            Openings::Centers => add(Place::Center, mid),
        }
    }
    let seen = |(p, own): &(Pt, Vec<[Vec2; 4]>)| !hidden(p, &bands, own, frame, side);
    let mut openings_row: Vec<Pt> = own_of.iter().filter(|e| seen(e)).map(|(p, _)| *p).collect();
    openings_row.extend(opening_pts.iter().filter(|e| seen(e)).map(|(p, _)| *p));
    let mut walls_all = lines;
    walls_all.extend(free.iter().copied());
    walls_all.extend(
        walls_row
            .into_iter()
            .filter(|p| p.t >= lo - SAME && p.t <= hi + SAME),
    );
    Ok((
        Rows {
            openings: openings_row,
            walls: walls_all,
            extent,
            grids: Vec::new(),
            skipped,
            body,
        },
        side,
    ))
}

fn space_rows(
    recipe: &Recipe<'_>,
    space: &SpaceIn,
    openings: &[OpeningIn],
    frame: &Frame,
    side_of: impl Fn((f64, f64)) -> f64,
) -> Result<(Rows, f64), Fault> {
    let mut body = (f64::INFINITY, f64::NEG_INFINITY);
    for rings in &space.parts {
        for (ring, (pts, bulges)) in rings.iter().enumerate() {
            let n = pts.len();
            if n < 3 {
                return Err(Fault::Unsupported(space.source, Why::Drawn));
            }
            for k in 0..n {
                let kind = classify(pts[k], pts[(k + 1) % n], bulges.get(k).copied().unwrap_or(0.0), frame);
                match kind {
                    Edge::Arc => return Err(Fault::Unsupported(space.source, Why::Arc)),
                    Edge::Oblique => return Err(Fault::Unsupported(space.source, Why::Oblique)),
                    _ => {}
                }
                if ring == 0 {
                    let o = frame.o(pts[k]);
                    body = (js_min(body.0, o), js_max(body.1, o));
                }
            }
        }
    }
    if !body.0.is_finite() {
        return Err(Fault::Nothing);
    }
    let side = side_of(body);
    let mut outer: Vec<Pt> = Vec::new();
    let mut holes: Vec<Pt> = Vec::new();
    for (part, rings) in space.parts.iter().enumerate() {
        for (ring, (pts, _)) in rings.iter().enumerate() {
            let n = pts.len();
            for k in 0..n {
                let (from, to) = (k, (k + 1) % n);
                if classify(pts[from], pts[to], 0.0, frame) != Edge::Perpendicular {
                    continue;
                }
                let (sa, sb) = (side * frame.o(pts[from]), side * frame.o(pts[to]));
                let index = if sb > sa || (sb == sa && to < from) { to } else { from };
                let at = pts[index];
                let p = Pt {
                    point: Point {
                        source: space.source,
                        place: Place::Vertex { part, ring, index },
                        at,
                    },
                    t: frame.t(at),
                    o: frame.o(at),
                    kind: Kind::Line,
                };
                if ring == 0 { outer.push(p) } else { holes.push(p) }
            }
        }
    }
    // The openings on an outer edge facing the side (its outward normal, the ring counter-clockwise, to its right).
    let mut opening_pts: Vec<Pt> = Vec::new();
    for o in openings {
        if o.flue || !o.jambs.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
            continue;
        }
        let on = |a: Vec2, b: Vec2| -> bool {
            space.parts.iter().any(|rings| {
                let Some((pts, _)) = rings.first() else { return false };
                let n = pts.len();
                let ccw = (0..n)
                    .map(|i| {
                        let (p, q) = (pts[i], pts[(i + 1) % n]);
                        p.x * q.y - q.x * p.y
                    })
                    .sum::<f64>()
                    > 0.0;
                (0..n).any(|k| {
                    let (u, v) = (pts[k], pts[(k + 1) % n]);
                    if classify(u, v, 0.0, frame) != Edge::Parallel {
                        return false;
                    }
                    let l = js_hypot(v.x - u.x, v.y - u.y);
                    let near = |p: Vec2| {
                        (orient2d(u, v, p) / l).abs() <= 1e-6 && {
                            let (tu, tv, tp) = (frame.t(u), frame.t(v), frame.t(p));
                            tp >= js_min(tu, tv) - SAME && tp <= js_max(tu, tv) + SAME
                        }
                    };
                    // Outward: right of the edge on a counter-clockwise ring.
                    let (ex, ey) = ((v.x - u.x) / l, (v.y - u.y) / l);
                    let out = if ccw { Vec2::new(ey, -ex) } else { Vec2::new(-ey, ex) };
                    let facing = side * (out.x * frame.n.x + out.y * frame.n.y) > 0.0;
                    facing && near(a) && near(b)
                })
            })
        };
        let pair = if on(o.jambs[0], o.jambs[2]) {
            Some([0, 2])
        } else if on(o.jambs[1], o.jambs[3]) {
            Some([1, 3])
        } else {
            None
        };
        let Some(pair) = pair else { continue };
        if let Some(right) = o.niche_right
            && right != (pair == [1, 3])
        {
            continue;
        }
        let mut add = |place: Place, at: Vec2| {
            opening_pts.push(Pt {
                point: Point {
                    source: o.source,
                    place,
                    at,
                },
                t: frame.t(at),
                o: frame.o(at),
                kind: Kind::Opening,
            });
        };
        match recipe.openings {
            Openings::Ends => {
                for j in pair {
                    add(
                        Place::Vertex {
                            part: 0,
                            ring: 0,
                            index: j,
                        },
                        o.jambs[j],
                    );
                }
            }
            Openings::Centers => add(
                Place::Center,
                Vec2::new(
                    (o.jambs[0].x + o.jambs[1].x + o.jambs[2].x + o.jambs[3].x) * 0.25,
                    (o.jambs[0].y + o.jambs[1].y + o.jambs[2].y + o.jambs[3].y) * 0.25,
                ),
            ),
        }
    }
    let mut openings_row = outer.clone();
    openings_row.extend(opening_pts);
    let mut walls_row = outer.clone();
    walls_row.extend(holes);
    Ok((
        Rows {
            openings: openings_row,
            walls: walls_row,
            extent: outer,
            grids: Vec::new(),
            skipped: Vec::new(),
            body,
        },
        side,
    ))
}

/// The recipe's spans (docs/adr/0729 §2): a façade of the walls given (`space` none) or a space's outline; `drop`
/// says a span the user removed (it is not drawn, and the row's other spans stay in place).
pub fn plan(
    recipe: &Recipe<'_>,
    walls: &[WallIn<'_>],
    openings: &[OpeningIn],
    grids: &[GridIn<'_>],
    space: Option<&SpaceIn>,
    drop: &dyn Fn(Row, &Point, &Point) -> bool,
) -> Result<Planned, Fault> {
    let d = recipe.direction;
    let l = js_hypot(d.x, d.y);
    if !l.is_finite()
        || l < 1e-12
        || !recipe.at.x.is_finite()
        || !recipe.at.y.is_finite()
        || !recipe.gap.is_finite()
        || recipe.gap <= 0.0
        || recipe
            .row_places
            .iter()
            .any(|(_, p)| !p.x.is_finite() || !p.y.is_finite())
    {
        return Err(Fault::Direction);
    }
    let d = Vec2::new(d.x / l, d.y / l);
    let origin = match (space, walls.iter().find(|w| w.chosen)) {
        (Some(s), _) => s
            .parts
            .first()
            .and_then(|r| r.first())
            .and_then(|(pts, _)| pts.first().copied())
            .ok_or(Fault::Nothing)?,
        (None, Some(w)) => *w.pts.first().ok_or(Fault::Nothing)?,
        (None, None) => return Err(Fault::Nothing),
    };
    let frame = Frame {
        origin,
        d,
        n: Vec2::new(-d.y, d.x),
    };
    let at_o = frame.o(recipe.at);
    let side_of = |(lo, hi): (f64, f64)| if at_o >= (lo + hi) * 0.5 { 1.0 } else { -1.0 };
    let (mut rows, side) = match space {
        Some(s) => space_rows(recipe, s, openings, &frame, side_of)?,
        None => facade(recipe, walls, openings, &frame, side_of)?,
    };
    // The grids square to the direction across the bodies, between the outermost points.
    let extent = settle(rows.extent.clone());
    let (lo, hi) = match (extent.first(), extent.last()) {
        (Some(a), Some(b)) => (*a, *b),
        _ => return Err(Fault::Nothing),
    };
    for g in grids {
        let [a, b] = match g.pts {
            [a, b] => [*a, *b],
            _ => continue,
        };
        if g.bulges.and_then(|k| k.first()).is_some_and(|k| *k != 0.0)
            || classify(a, b, 0.0, &frame) != Edge::Perpendicular
        {
            continue;
        }
        let (oa, ob) = (frame.o(a), frame.o(b));
        if js_max(oa, ob) < rows.body.0 - SAME || js_min(oa, ob) > rows.body.1 + SAME {
            continue;
        }
        let t = frame.t(a);
        if t < lo.t - SAME || t > hi.t + SAME {
            continue;
        }
        let (place, at) = if side * ob > side * oa {
            (Place::End, b)
        } else {
            (Place::Start, a)
        };
        rows.grids.push(Pt {
            point: Point {
                source: g.source,
                place,
                at,
            },
            t: frame.t(at),
            o: frame.o(at),
            kind: Kind::Grid,
        });
    }
    let mut grid_row = vec![lo, hi];
    grid_row.extend(rows.grids.iter().copied());
    let angle = atan2(d.y, d.x) * 180.0 / PI;
    let mut spans: Vec<Span> = Vec::new();
    let mut notes: Vec<RowNote> = Vec::new();
    let mut drawn: Vec<(Point, Point)> = Vec::new();
    let mut slot = 0usize;
    for &row in recipe.rows {
        let pts = settle(match row {
            Row::Openings => rows.openings.clone(),
            Row::Walls => rows.walls.clone(),
            Row::Grids => grid_row.clone(),
            Row::Overall => vec![lo, hi],
        });
        let mut sources: Vec<usize> = pts.iter().map(|p| p.point.source).collect();
        sources.sort_unstable();
        sources.dedup();
        let row_at = recipe
            .row_places
            .iter()
            .find(|(r, _)| *r == row)
            .map(|(_, p)| *p)
            .unwrap_or_else(|| {
                let target = at_o + side * recipe.gap * slot as f64;
                let delta = target - at_o;
                Vec2::new(recipe.at.x + frame.n.x * delta, recipe.at.y + frame.n.y * delta)
            });
        let mut count = 0;
        for pair in pts.windows(2) {
            let (a, b) = (pair[0].point, pair[1].point);
            let same = |p: &Point, q: &Point| p.source == q.source && p.place == q.place;
            if drop(row, &a, &b) || drawn.iter().any(|(x, y)| same(x, &a) && same(y, &b)) {
                continue;
            }
            drawn.push((a, b));
            let offset = (row_at.x - a.at.x) * frame.n.x + (row_at.y - a.at.y) * frame.n.y;
            spans.push(Span {
                row,
                a,
                b,
                at: row_at,
                angle,
                offset,
            });
            count += 1;
        }
        if count > 0 {
            slot += 1;
        }
        notes.push(RowNote {
            row,
            points: pts.len(),
            sources: sources.len(),
            spans: count,
        });
    }
    rows.skipped.sort_unstable();
    Ok(Planned {
        spans,
        rows: notes,
        skipped: rows.skipped,
        side,
        origin,
        direction: d,
        body: rows.body,
    })
}

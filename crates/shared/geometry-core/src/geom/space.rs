//! Spaces (mahal, docs/adr/0342 §2): a space's boundary found from the walls round a point, moved to the walls'
//! centre lines or outer faces by its method, measured, and drawn with its label.
//!
//! - From the walls: the walls are joined (`geom::wall_join`) without their openings, so that a door's gap leaves the
//!   room closed; each wall's two faces and its ends are line work; `FaceIndex` finds the face round the seed with its
//!   holes (docs/adr/0328 §3).
//! - By the method: each edge of the net face that runs along a wall's face moves into that wall, half its width
//!   (centre line) or its whole width (outer face); an edge on no wall's face stays. The outer ring runs counter-
//!   clockwise, the holes clockwise, and every edge moves to its right; consecutive moved edges meet where their lines
//!   (an arc's tangent at the vertex) cross; two that stay parallel take a short step between them.
//! - Measured: the rings' exact area, holes off; their length; the label's place at the largest part's pole of
//!   inaccessibility.

use crate::api::json::{FromJson, Json, ToJson, field};
use crate::entity::{Entity, Shape};
use crate::geom::arrangement::{Area, Ring};
use crate::geom::bulge::{bulge_path_length, segment_mid, segment_tangent};
use crate::geom::intersect::closest_on_edge;
use crate::geom::region::{net_area, orient_ring, ring_edges};
use crate::geom::wall_join::{WallIn, solve};
use crate::jsmath::js_max;
use crate::vec2::Vec2;

/// How near a face's edge must run along a wall's face to be on it, metres.
pub const ON_FACE: f64 = 1e-6;
/// The label's height on paper, millimetres.
pub const LABEL_MM: f64 = 2.5;

/// What a space's rings stand for (the contract's `SpaceMethod`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    #[default]
    Net,
    Centerline,
    Gross,
}

impl Method {
    pub fn name(self) -> &'static str {
        match self {
            Method::Net => "net",
            Method::Centerline => "centerline",
            Method::Gross => "gross",
        }
    }

    pub fn of(name: &str) -> Option<Method> {
        match name {
            "net" => Some(Method::Net),
            "centerline" => Some(Method::Centerline),
            "gross" => Some(Method::Gross),
            _ => None,
        }
    }

    /// How far into its wall an edge on a wall `width` wide moves.
    fn reach(self, width: f64) -> f64 {
        match self {
            Method::Net => 0.0,
            Method::Centerline => width / 2.0,
            Method::Gross => width,
        }
    }
}

/// A space's fields beside its rings: what its label and its following need. Its usage, level, finishes and the rest
/// are the contract's and pass through untouched.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spec {
    pub seed: Option<Vec2>,
    pub method: Method,
    pub number: Option<String>,
    pub name: Option<String>,
    /// A void (docs/adr/0708): drawn crossed.
    pub void: bool,
}

fn word(v: &Json, name: &str) -> Option<String> {
    match v.get(name) {
        Json::Str(s) => Some(s.clone()),
        _ => None,
    }
}

impl crate::api::json::Flat for Spec {
    const NAMES: &'static [&'static str] = &["seed", "method", "number", "name", "void"];

    fn read_flat(v: &Json) -> Result<Self, String> {
        let method = match word(v, "method").as_deref() {
            None => Method::Net,
            Some(s) => Method::of(s).ok_or_else(|| format!("bilinmeyen ölçme yöntemi “{s}”"))?,
        };
        Ok(Spec {
            seed: Option::<Vec2>::from_json(v.get("seed"))?,
            method,
            number: word(v, "number"),
            name: word(v, "name"),
            void: matches!(v.get("void"), Json::Bool(true)),
        })
    }

    fn write_flat(&self, out: &mut String, first: &mut bool) {
        if let Some(s) = &self.seed {
            field(out, first, "seed", s);
        }
        if self.method != Method::Net {
            field(out, first, "method", self.method.name());
        }
        if let Some(n) = &self.number {
            field(out, first, "number", n.as_str());
        }
        if let Some(n) = &self.name {
            field(out, first, "name", n.as_str());
        }
        if self.void {
            field(out, first, "void", &true);
        }
    }
}

impl ToJson for Spec {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        crate::api::json::Flat::write_flat(self, out, &mut first);
        out.push('}');
    }
}

/// The walls' faces as line work, each with its wall's width: every wall's first and last line (its faces) as the
/// joins leave them without openings, and its ends.
pub fn wall_faces(walls: &[WallIn<'_>]) -> Vec<(Shape, f64)> {
    let solved = solve(walls, None);
    let mut out = Vec::new();
    for (w, drawn) in walls.iter().zip(solved.drawn) {
        let Some(d) = drawn else { continue };
        let last = d.lines.len().saturating_sub(1);
        for (k, line) in d.lines.iter().enumerate() {
            if k != 0 && k != last {
                continue;
            }
            for s in line {
                out.push((s.line.shape(s.closed), w.spec.width));
            }
        }
        for (a, b) in &d.caps {
            // An end is no wall's face: what runs along it stays.
            out.push((Shape::Line { a: *a, b: *b }, 0.0));
        }
    }
    out
}

/// The face the `faces` close round `seed`, with its holes; none when they close nothing round it.
pub fn net_face(faces: &[(Shape, f64)], seed: Vec2) -> Option<Area> {
    let lines: Vec<Entity> = faces.iter().map(|(s, _)| Entity::new(s.clone())).collect();
    let index = crate::geom::region::FaceIndex::new(&[crate::ops::areas::line_source(&lines)]);
    index.at(seed, true)
}

fn right(t: Vec2) -> Vec2 {
    Vec2::new(t.y, -t.x)
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

/// Where the lines through `p` along `u` and through `q` along `v` cross; none when they are parallel.
fn meet(p: Vec2, u: Vec2, q: Vec2, v: Vec2) -> Option<Vec2> {
    let d = cross(u, v);
    if d.abs() <= 1e-12 {
        return None;
    }
    let t = cross(Vec2::new(q.x - p.x, q.y - p.y), v) / d;
    Some(Vec2::new(p.x + u.x * t, p.y + u.y * t))
}

/// The reach of each of the ring's edges: the wall whose face its middle lies on, else 0.
fn reaches(r: &Ring, faces: &[(Shape, f64)], method: Method) -> Vec<f64> {
    let edges: Vec<(Vec<crate::geom::intersect::Edge>, f64)> = faces
        .iter()
        .map(|(s, w)| (crate::ops::edges::entity_edges(s), *w))
        .collect();
    let n = r.pts.len();
    (0..n)
        .map(|i| {
            let b = r.bulges.as_ref().map_or(0.0, |b| b[i]);
            let m = segment_mid(r.pts[i], r.pts[(i + 1) % n], b);
            edges
                .iter()
                .find(|(es, w)| *w > 0.0 && es.iter().any(|e| closest_on_edge(e, m).d <= ON_FACE))
                .map_or(0.0, |(_, w)| method.reach(*w))
        })
        .collect()
}

/// The ring with each edge moved right by its reach.
fn moved_ring(r: &Ring, reach: &[f64]) -> Ring {
    let n = r.pts.len();
    let bulge = |i: usize| r.bulges.as_ref().map_or(0.0, |b| b[i % n]);
    let mut pts = Vec::with_capacity(n);
    let mut bulges = Vec::with_capacity(n);
    for i in 0..n {
        // The vertex between edge i−1 (ending here) and edge i (starting here).
        let prev = (i + n - 1) % n;
        let p = r.pts[i];
        let (dp, di) = (reach[prev], reach[i]);
        let tp = segment_tangent(r.pts[prev], p, bulge(prev), true);
        let ti = segment_tangent(p, r.pts[(i + 1) % n], bulge(i), false);
        let (np, ni) = (right(tp), right(ti));
        let ap = Vec2::new(p.x + np.x * dp, p.y + np.y * dp);
        let ai = Vec2::new(p.x + ni.x * di, p.y + ni.y * di);
        if dp == 0.0 && di == 0.0 {
            pts.push(p);
            bulges.push(bulge(i));
            continue;
        }
        match meet(ap, tp, ai, ti) {
            Some(q) => {
                pts.push(q);
                bulges.push(bulge(i));
            }
            // Parallel: one point when they move alike, else a step between them.
            None if (dp - di).abs() <= 1e-12 => {
                pts.push(ai);
                bulges.push(bulge(i));
            }
            None => {
                pts.push(ap);
                bulges.push(0.0);
                pts.push(ai);
                bulges.push(bulge(i));
            }
        }
    }
    let curved = bulges.iter().any(|b| *b != 0.0);
    Ring {
        pts,
        bulges: curved.then_some(bulges),
    }
}

/// The net face moved by `method` (the outer ring counter-clockwise, the holes clockwise).
pub fn by_method(net: &Area, faces: &[(Shape, f64)], method: Method) -> Area {
    let outer = orient_ring(&net.outer, true);
    let holes: Vec<Ring> = net.holes.iter().map(|h| orient_ring(h, false)).collect();
    if method == Method::Net {
        return Area { outer, holes };
    }
    let move_ring = |r: &Ring| moved_ring(r, &reaches(r, faces, method));
    Area {
        outer: move_ring(&outer),
        holes: holes.iter().map(move_ring).collect(),
    }
}

/// A space's boundary from the walls round `seed` by `method`; none when they close nothing round it.
pub fn found(walls: &[WallIn<'_>], seed: Vec2, method: Method) -> Option<Area> {
    let faces = wall_faces(walls);
    let net = net_face(&faces, seed)?;
    Some(by_method(&net, &faces, method))
}

/// The area of `parts`, holes off, m².
pub fn area(parts: &[Area]) -> f64 {
    parts.iter().map(net_area).sum()
}

/// The length of every ring of `parts`, metres.
pub fn perimeter(parts: &[Area]) -> f64 {
    parts
        .iter()
        .flat_map(|a| std::iter::once(&a.outer).chain(&a.holes))
        .map(|r| bulge_path_length(&r.pts, r.bulges.as_deref(), true))
        .sum()
}

/// Where a space's label stands: the largest part's pole of inaccessibility (its rings' vertices).
pub fn label_at(parts: &[Area]) -> Option<Vec2> {
    let largest = parts
        .iter()
        .max_by(|a, b| net_area(a).total_cmp(&net_area(b)))?;
    let rings: Vec<Vec<Vec2>> = std::iter::once(&largest.outer)
        .chain(&largest.holes)
        .map(|r| crate::entity::polygon_ring(&r.pts, r.bulges.as_deref()))
        .collect();
    let size = ring_edges(&largest.outer)
        .iter()
        .fold(0.0, |m, e| js_max(m, crate::ops::edges::edge_length(e)));
    crate::labels::geom::pole(&rings, js_max(size / 100.0, 1e-3)).map(|(p, _)| p)
}

/// The label's lines: its number, its name, its area (two decimals, m²).
pub fn label_lines(parts: &[Area], spec: &Spec) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(spec.number.clone());
    out.extend(spec.name.clone());
    out.push(format!("{:.2} m²", area(parts)));
    out
}

fn text(p: Vec2, s: String, height: f64) -> Shape {
    Shape::Text {
        p,
        text: s,
        height,
        rotation: 0.0,
        align: Some(crate::text::TextAlign::MiddleCenter),
        width_factor: None,
        mask: None,
        box_width: None,
        line_spacing: None,
        runs: None,
        flow: None,
        face: Default::default(),
        path: None,
    }
}

/// A void's cross (docs/adr/0708): for each part the two diagonals of the rectangle round it along its longest edge,
/// kept where they run inside the part (its holes left out). A rectangular shaft's run corner to corner.
pub fn void_cross(parts: &[Area]) -> Vec<Shape> {
    let mut out = Vec::new();
    for part in parts {
        let outer = crate::entity::polygon_ring(&part.outer.pts, part.outer.bulges.as_deref());
        let holes: Vec<Vec<Vec2>> = part
            .holes
            .iter()
            .map(|h| crate::entity::polygon_ring(&h.pts, h.bulges.as_deref()))
            .collect();
        let n = outer.len();
        if n < 3 {
            continue;
        }
        // The longest edge's direction, the first of equals.
        let mut along = Vec2::new(0.0, 0.0);
        let mut longest = 0.0;
        for i in 0..n {
            let (a, b) = (outer[i], outer[(i + 1) % n]);
            let l = crate::jsmath::js_hypot(b.x - a.x, b.y - a.y);
            if l > longest {
                longest = l;
                along = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
            }
        }
        if !(longest > 0.0) {
            continue;
        }
        let across = Vec2::new(-along.y, along.x);
        let o = outer[0];
        let (mut u0, mut u1, mut v0, mut v1) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for p in &outer {
            let (dx, dy) = (p.x - o.x, p.y - o.y);
            let (u, v) = (dx * along.x + dy * along.y, dx * across.x + dy * across.y);
            u0 = crate::jsmath::js_min(u0, u);
            u1 = js_max(u1, u);
            v0 = crate::jsmath::js_min(v0, v);
            v1 = js_max(v1, v);
        }
        let at = |u: f64, v: f64| Vec2::new(o.x + along.x * u + across.x * v, o.y + along.y * u + across.y * v);
        for (a, b) in [(at(u0, v0), at(u1, v1)), (at(u1, v0), at(u0, v1))] {
            let mut ts = vec![0.0, 1.0];
            for ring in std::iter::once(&outer).chain(&holes) {
                let m = ring.len();
                for i in 0..m {
                    if let Some(h) =
                        crate::geom::intersect::seg_seg(a, b, ring[i], ring[(i + 1) % m], 1e-12)
                    {
                        ts.push(h.t.clamp(0.0, 1.0));
                    }
                }
            }
            ts.sort_by(f64::total_cmp);
            ts.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
            for w in ts.windows(2) {
                let (t0, t1) = (w[0], w[1]);
                let mid = Vec2::new(a.x + (b.x - a.x) * (t0 + t1) / 2.0, a.y + (b.y - a.y) * (t0 + t1) / 2.0);
                let inside = crate::geometry::point_in_polygon(mid, &outer)
                    && !holes.iter().any(|h| crate::geometry::point_in_polygon(mid, h));
                if inside {
                    out.push(Shape::Line {
                        a: Vec2::new(a.x + (b.x - a.x) * t0, a.y + (b.y - a.y) * t0),
                        b: Vec2::new(a.x + (b.x - a.x) * t1, a.y + (b.y - a.y) * t1),
                    });
                }
            }
        }
    }
    out
}

/// A space's pieces at 1:`scale`: its rings, a void's cross and its label's lines, centred one under another.
pub fn pieces(parts: &[Area], spec: &Spec, scale: f64) -> Vec<Shape> {
    pieces_with(parts, spec, scale, true)
}

/// `pieces`, its label left out when not `label`: a tagged space's tag is its label (docs/adr/0344's 27 October
/// addendum).
pub fn pieces_with(parts: &[Area], spec: &Spec, scale: f64, label: bool) -> Vec<Shape> {
    let mut out: Vec<Shape> = parts
        .iter()
        .flat_map(|a| std::iter::once(&a.outer).chain(&a.holes))
        .map(|r| Shape::Polyline {
            pts: r.pts.clone(),
            bulges: r.bulges.clone(),
            holes: None,
            parts: None,
            closed: true,
            widths: None,
        })
        .collect();
    if spec.void {
        out.extend(void_cross(parts));
    }
    // One text of its lines, centred on the pole: its lines grow together as the view asks (docs/adr/0205 §5).
    if label && let Some(at) = label_at(parts) {
        let h = LABEL_MM * scale / 1000.0;
        out.push(text(at, label_lines(parts, spec).join("\n"), h));
    }
    out
}

/// The space's rings as one area object (a multi-part area with holes): what picking, outlines and edges take.
pub fn as_polygon(parts: &[Area]) -> Shape {
    let ring_of = |a: &Area| {
        let holes: Vec<Ring> = a.holes.clone();
        (
            a.outer.pts.clone(),
            a.outer.bulges.clone(),
            (!holes.is_empty()).then_some(holes),
        )
    };
    let mut it = parts.iter();
    let (pts, bulges, holes) = it.next().map(ring_of).unwrap_or_default();
    let rest: Vec<crate::entity::Part> = it
        .map(|a| {
            let (pts, bulges, holes) = ring_of(a);
            crate::entity::Part { pts, bulges, holes }
        })
        .collect();
    Shape::Polygon {
        pts,
        bulges,
        holes,
        parts: (!rest.is_empty()).then_some(rest),
    }
}

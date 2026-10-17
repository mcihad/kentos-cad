//! The standard relations (docs/adr/0501, plan/gis.md CBS-22): an object's DE-9IM matrix against another, exact up to
//! the floating point (not Konuma göre seç's 1 mm), and the OGC/PostGIS predicates read from it: intersects, disjoint,
//! contains, within, covers, coveredBy, touches, crosses, overlaps, equals, and any `ST_Relate` pattern. An object is
//! read as a query reads it (`spatial_query::Geometry`): its areas with their holes (arcs exact), or its open paths, or
//! its points; a line's boundary is its ends by the mod-2 rule, a closed path has none. Z is not read (2D, as
//! PostGIS's ST_Relate).
//!
//! The matrix is built by noding: each object's edges cut where the other's edges meet them, at the other's vertices
//! and points on them; each piece's middle placed in the other (interior, boundary or exterior) gives a dimension-1
//! entry, each node a dimension-0 entry, and for two areas the points just either side of every boundary piece give
//! the dimension-2 entries (every face of the two boundaries lies beside some piece). The independent reference is GEOS
//! through QGIS (scripts/fixtures/relate_cases.py).

use crate::entity::{Shape as Kind, area_parts};
use crate::geom::intersect::point_at as at;
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges, point_at};
use crate::geometry::{Bounds, dist};
use crate::jsmath::{js_max, js_min};
use crate::ops::edges::entity_edges;
use crate::ops::spatial_query::{Geometry, geometry_of, inside};
use crate::vec2::Vec2;

/// Where a point lies against an object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loc {
    Interior = 0,
    Boundary = 1,
    Exterior = 2,
}

/// A DE-9IM matrix: each entry the dimension of where `a`'s interior, boundary and exterior meet `b`'s (−1 none),
/// row by row: II, IB, IE, BI, BB, BE, EI, EB, EE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Matrix(pub [i8; 9]);

impl Matrix {
    fn empty() -> Matrix {
        Matrix([-1; 9])
    }

    fn set(&mut self, a: Loc, b: Loc, dim: i8) {
        let i = a as usize * 3 + b as usize;
        if self.0[i] < dim {
            self.0[i] = dim;
        }
    }

    pub fn get(&self, a: Loc, b: Loc) -> i8 {
        self.0[a as usize * 3 + b as usize]
    }

    /// Its text as `ST_Relate` writes it (`F`, `0`, `1`, `2`).
    pub fn text(&self) -> String {
        self.0
            .iter()
            .map(|&d| match d {
                0 => '0',
                1 => '1',
                2 => '2',
                _ => 'F',
            })
            .collect()
    }

    /// Whether it matches a nine-letter pattern of `T`, `F`, `*`, `0`, `1`, `2` (any case); none for a bad pattern.
    pub fn matches(&self, pattern: &str) -> Option<bool> {
        let letters: Vec<char> = pattern.chars().collect();
        if letters.len() != 9 {
            return None;
        }
        let mut ok = true;
        for (d, c) in self.0.iter().zip(letters) {
            let hit = match c.to_ascii_uppercase() {
                '*' => true,
                'T' => *d >= 0,
                'F' => *d < 0,
                '0' => *d == 0,
                '1' => *d == 1,
                '2' => *d == 2,
                _ => return None,
            };
            ok &= hit;
        }
        Some(ok)
    }

    /// The matrix of `b` against `a`.
    pub fn transpose(&self) -> Matrix {
        let m = self.0;
        Matrix([m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]])
    }
}

/// A predicate of the standard profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Predicate {
    Intersects,
    Disjoint,
    Contains,
    Within,
    Covers,
    CoveredBy,
    Touches,
    Crosses,
    Overlaps,
    Equals,
}

impl Predicate {
    pub const ALL: [Predicate; 10] = [
        Predicate::Intersects,
        Predicate::Disjoint,
        Predicate::Contains,
        Predicate::Within,
        Predicate::Covers,
        Predicate::CoveredBy,
        Predicate::Touches,
        Predicate::Crosses,
        Predicate::Overlaps,
        Predicate::Equals,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Predicate::Intersects => "intersects",
            Predicate::Disjoint => "disjoint",
            Predicate::Contains => "contains",
            Predicate::Within => "within",
            Predicate::Covers => "covers",
            Predicate::CoveredBy => "coveredBy",
            Predicate::Touches => "touches",
            Predicate::Crosses => "crosses",
            Predicate::Overlaps => "overlaps",
            Predicate::Equals => "equals",
        }
    }

    pub fn from_key(key: &str) -> Option<Predicate> {
        Predicate::ALL.into_iter().find(|p| p.key() == key)
    }

    /// Whether `m`, the matrix of an object of dimension `da` against one of dimension `db`, says it (OGC SFA 1.2.1
    /// §6.1.15.3, as PostGIS reads it).
    pub fn holds(self, m: &Matrix, da: i8, db: i8) -> bool {
        let t = |p: &str| m.matches(p).unwrap_or(false);
        match self {
            Predicate::Intersects => !t("FF*FF****"),
            Predicate::Disjoint => t("FF*FF****"),
            Predicate::Contains => t("T*****FF*"),
            Predicate::Within => t("T*F**F***"),
            Predicate::Covers => {
                t("T*****FF*") || t("*T****FF*") || t("***T**FF*") || t("****T*FF*")
            }
            Predicate::CoveredBy => {
                t("T*F**F***") || t("*TF**F***") || t("**FT*F***") || t("**F*TF***")
            }
            Predicate::Touches => t("FT*******") || t("F**T*****") || t("F***T****"),
            Predicate::Crosses => match (da, db) {
                (1, 1) => t("0********"),
                (a, b) if a < b => t("T*T******"),
                (a, b) if a > b => t("T*****T**"),
                _ => false,
            },
            Predicate::Overlaps => match (da, db) {
                (1, 1) => t("1*T***T**"),
                (a, b) if a == b => t("T*T***T**"),
                _ => false,
            },
            Predicate::Equals => t("T*F**FFF*"),
        }
    }
}

/// An object as the standard profile reads it (docs/adr/0501 §1): a polygon, a circle, a whole ellipse and a hatch are
/// areas; a line, a polyline (closed too: a closed line, docs/adr/0258), an arc, an elliptic arc, a curve and a NURBS
/// curve (closed too) are lines; the rest as a query reads them (points, an insert's or a text's place). None for a
/// construction line or a ray.
pub fn geometry_of_standard(s: &Kind) -> Option<Geometry> {
    let line = matches!(
        s,
        Kind::Line { .. }
            | Kind::Polyline { .. }
            | Kind::Arc { .. }
            | Kind::Spline { .. }
            | Kind::Nurbs { .. }
    ) || matches!(s, Kind::Ellipse { .. } if crate::ops::areas::areas_of_entity(s).is_empty());
    if !line {
        return geometry_of(s);
    }
    let mut edges = Vec::new();
    let mut starts = Vec::new();
    for part in area_parts(s).iter() {
        let es = entity_edges(part);
        if let Some(e) = es.first() {
            starts.push(at(e, 0.0));
        }
        edges.extend(es);
    }
    let mut b = crate::geometry::empty_bounds();
    for e in &edges {
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            crate::geometry::extend_bounds(&mut b, at(e, t), 0.0);
        }
        if let Edge::Arc { c, r, .. } = *e {
            crate::geometry::extend_bounds(&mut b, c, r);
        }
    }
    if edges.is_empty() || crate::geometry::is_empty_bounds(&b) {
        return None;
    }
    Some(Geometry {
        areas: Vec::new(),
        area_edges: Vec::new(),
        edges,
        points: Vec::new(),
        starts,
        center: None,
        bounds: b,
    })
}

/// An object's dimension: 2 with areas, 1 with paths, 0 with points only.
pub fn dimension(g: &Geometry) -> i8 {
    if !g.areas.is_empty() {
        2
    } else if !g.edges.is_empty() {
        1
    } else {
        0
    }
}

/// How near two points must be to be one: a few units in the last place of the largest coordinate, at least 10⁻⁹ m.
pub fn tolerance(a: &Bounds, b: &Bounds) -> f64 {
    let big = [
        a.min_x, a.min_y, a.max_x, a.max_y, b.min_x, b.min_y, b.max_x, b.max_y,
    ]
    .iter()
    .filter(|v| v.is_finite())
    .fold(0.0_f64, |m, v| js_max(m, v.abs()));
    js_max(1e-9, big * 8.0 * f64::EPSILON)
}

/// What a query reads of an object for its matrix: its dimension, the edges it is noded along (an area's boundary, a
/// line's paths), its points, and a line's boundary points.
struct Shape<'a> {
    g: &'a Geometry,
    dim: i8,
    edges: &'a [Edge],
    ends: Vec<Vec2>,
}

impl<'a> Shape<'a> {
    fn new(g: &'a Geometry, eps: f64) -> Shape<'a> {
        let dim = dimension(g);
        let edges: &[Edge] = match dim {
            2 => &g.area_edges,
            1 => &g.edges,
            _ => &[],
        };
        let mut ends = Vec::new();
        if dim == 1 {
            // The mod-2 rule: a point that ends an odd number of edges is on the boundary.
            let mut seen: Vec<(Vec2, u32)> = Vec::new();
            for e in edges {
                for p in [point_at(e, 0.0), point_at(e, 1.0)] {
                    match seen.iter_mut().find(|(q, _)| dist(*q, p) <= eps) {
                        Some((_, n)) => *n += 1,
                        None => seen.push((p, 1)),
                    }
                }
            }
            ends = seen
                .into_iter()
                .filter(|(_, n)| n % 2 == 1)
                .map(|(p, _)| p)
                .collect();
        }
        Shape {
            g,
            dim,
            edges,
            ends,
        }
    }

    /// Where `p` lies.
    fn locate(&self, p: Vec2, eps: f64) -> Loc {
        match self.dim {
            2 => {
                if self.edges.iter().any(|e| closest_on_edge(e, p).d <= eps) {
                    Loc::Boundary
                } else if inside(&self.g.areas, p) {
                    Loc::Interior
                } else {
                    Loc::Exterior
                }
            }
            1 => {
                if self.ends.iter().any(|q| dist(*q, p) <= eps) {
                    Loc::Boundary
                } else if self.edges.iter().any(|e| closest_on_edge(e, p).d <= eps) {
                    Loc::Interior
                } else {
                    Loc::Exterior
                }
            }
            _ => {
                if self.g.points.iter().any(|q| dist(*q, p) <= eps) {
                    Loc::Interior
                } else {
                    Loc::Exterior
                }
            }
        }
    }

    /// The location its edges' points have in itself: an area's boundary, a line's interior (or boundary at its ends).
    fn own(&self, p: Vec2, eps: f64) -> Loc {
        if self.dim == 2 {
            Loc::Boundary
        } else {
            self.locate(p, eps)
        }
    }
}

/// The places along `e` where the other object meets it, as parameters 0 … 1, sorted, ends included.
fn cuts(e: &Edge, other: &Shape, eps: f64) -> Vec<f64> {
    let mut ts = vec![0.0, 1.0];
    for f in other.edges {
        for h in intersect_edges(e, f) {
            ts.push(h.t.clamp(0.0, 1.0));
        }
        for q in [point_at(f, 0.0), point_at(f, 1.0)] {
            let c = closest_on_edge(e, q);
            if c.d <= eps {
                ts.push(c.t);
            }
        }
    }
    for &q in &other.g.points {
        let c = closest_on_edge(e, q);
        if c.d <= eps {
            ts.push(c.t);
        }
    }
    ts.sort_by(f64::total_cmp);
    ts.dedup_by(|x, y| (*x - *y).abs() <= 1e-15);
    ts
}

/// A unit normal of `e` at parameter `t`.
fn normal(e: &Edge, t: f64) -> Vec2 {
    match *e {
        Edge::Seg { a, b } => {
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let l = (dx * dx + dy * dy).sqrt();
            Vec2::new(-dy / l, dx / l)
        }
        Edge::Arc { c, r, .. } => {
            let p = point_at(e, t);
            Vec2::new((p.x - c.x) / r, (p.y - c.y) / r)
        }
    }
}

/// `x`'s pieces and nodes placed in `y`; `x_is_a` says which row they fill. For two areas, the points either side of
/// each piece fill the dimension-2 entries.
fn node(x: &Shape, y: &Shape, x_is_a: bool, both_areas: bool, eps: f64, m: &mut Matrix) {
    let mut put = |lx: Loc, ly: Loc, d: i8| {
        if x_is_a {
            m.set(lx, ly, d)
        } else {
            m.set(ly, lx, d)
        }
    };
    for e in x.edges {
        let ts = cuts(e, y, eps);
        for &t in &ts {
            let p = point_at(e, t);
            put(x.own(p, eps), y.locate(p, eps), 0);
        }
        for w in ts.windows(2) {
            if w[1] - w[0] <= 1e-12 {
                continue;
            }
            let mid = (w[0] + w[1]) / 2.0;
            let p = point_at(e, mid);
            let lx = if x.dim == 2 {
                Loc::Boundary
            } else {
                Loc::Interior
            };
            put(lx, y.locate(p, eps), 1);
            if both_areas {
                let n = normal(e, mid);
                let len = dist(point_at(e, w[0]), point_at(e, w[1]));
                let d = js_max(eps * 16.0, js_min(len * 1e-3, 1e-6));
                for s in [1.0, -1.0] {
                    let q = Vec2::new(p.x + s * d * n.x, p.y + s * d * n.y);
                    let side = |g: &Shape| {
                        if inside(&g.g.areas, q) {
                            Loc::Interior
                        } else {
                            Loc::Exterior
                        }
                    };
                    let (lx, ly) = (side(x), side(y));
                    put(lx, ly, 2);
                }
            }
        }
    }
    for &p in &x.g.points {
        if x.dim == 0 {
            put(Loc::Interior, y.locate(p, eps), 0);
        }
    }
}

/// `a`'s DE-9IM matrix against `b`.
pub fn relate(a: &Geometry, b: &Geometry) -> Matrix {
    let eps = tolerance(&a.bounds, &b.bounds);
    let (sa, sb) = (Shape::new(a, eps), Shape::new(b, eps));
    let mut m = Matrix::empty();
    let both = sa.dim == 2 && sb.dim == 2;
    node(&sa, &sb, true, both, eps, &mut m);
    node(&sb, &sa, false, both, eps, &mut m);
    // An area's interior is wider than anything of a lower dimension.
    if sa.dim == 2 && sb.dim < 2 {
        m.set(Loc::Interior, Loc::Exterior, 2);
    }
    if sb.dim == 2 && sa.dim < 2 {
        m.set(Loc::Exterior, Loc::Interior, 2);
    }
    // A line's interior and boundary are wider than a point set; a point set's boundary is empty.
    if sa.dim == 1 && sb.dim == 0 {
        m.set(Loc::Interior, Loc::Exterior, 1);
    }
    if sb.dim == 1 && sa.dim == 0 {
        m.set(Loc::Exterior, Loc::Interior, 1);
    }
    // Two point sets: a point of one the other lacks lies in its exterior.
    if sa.dim == 0 && sb.dim == 0 {
        for &p in &a.points {
            m.set(Loc::Interior, sb.locate(p, eps), 0);
        }
        for &q in &b.points {
            m.set(sa.locate(q, eps), Loc::Interior, 0);
        }
    }
    // A line's ends that the other does not hold.
    for &p in &sa.ends {
        m.set(Loc::Boundary, sb.locate(p, eps), 0);
    }
    for &q in &sb.ends {
        m.set(sa.locate(q, eps), Loc::Boundary, 0);
    }
    // Bounded objects: their exteriors always meet in an area.
    m.set(Loc::Exterior, Loc::Exterior, 2);
    m
}

/// The dimension of an object's boundary: an area's 1, a line's 0 (none when every path is closed), points' none.
pub fn boundary_dimension(g: &Geometry) -> i8 {
    match dimension(g) {
        2 => 1,
        1 if !Shape::new(g, tolerance(&g.bounds, &g.bounds))
            .ends
            .is_empty() =>
        {
            0
        }
        _ => -1,
    }
}

/// Whether the boxes of two objects are apart, so nothing of them meets.
pub fn boxes_apart(a: &Geometry, b: &Geometry) -> bool {
    let eps = tolerance(&a.bounds, &b.bounds);
    a.bounds.min_x - eps > b.bounds.max_x
        || b.bounds.min_x - eps > a.bounds.max_x
        || a.bounds.min_y - eps > b.bounds.max_y
        || b.bounds.min_y - eps > a.bounds.max_y
}

/// [`relate`], without noding when the boxes are apart: then nothing is in common.
pub fn relate_fast(a: &Geometry, b: &Geometry) -> Matrix {
    if !boxes_apart(a, b) {
        return relate(a, b);
    }
    Matrix([
        -1,
        -1,
        dimension(a),
        -1,
        -1,
        boundary_dimension(a),
        dimension(b),
        boundary_dimension(b),
        2,
    ])
}

/// Whether `a` stands in `p` to `b`.
pub fn holds(a: &Geometry, b: &Geometry, p: Predicate) -> bool {
    p.holds(&relate_fast(a, b), dimension(a), dimension(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Shape as S;

    fn poly(pts: &[(f64, f64)]) -> Geometry {
        geometry_of(&S::Polygon {
            pts: pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
            bulges: None,
            holes: None,
            parts: None,
        })
        .expect("geometry")
    }

    fn point(x: f64, y: f64) -> Geometry {
        geometry_of(&S::Point {
            p: Vec2::new(x, y),
            z: None,
            parts: None,
        })
        .expect("geometry")
    }

    #[test]
    fn a_point_on_a_squares_edge_is_covered_but_not_contained() {
        let sq = poly(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        let p = point(10.0, 5.0);
        let m = relate(&sq, &p);
        assert_eq!(m.text(), "FF20F1FF2");
        assert!(holds(&sq, &p, Predicate::Covers));
        assert!(!holds(&sq, &p, Predicate::Contains));
        assert!(holds(&p, &sq, Predicate::Touches));
    }

    #[test]
    fn squares_sharing_an_edge_touch() {
        let a = poly(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        let b = poly(&[(10.0, 0.0), (20.0, 0.0), (20.0, 10.0), (10.0, 10.0)]);
        assert_eq!(relate(&a, &b).text(), "FF2F11212");
        assert!(holds(&a, &b, Predicate::Touches));
    }
}

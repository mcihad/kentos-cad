//! Topoloji kuralları (docs/adr/0202): a project's rules over its layers,
//! what each finds (the problem, its objects, its place, its measure and
//! shape), which findings are exceptions, and what each fix writes. Areas
//! meet areas in the core's overlay (`geom::overlay`); how far a stretch of
//! an edge lies within the tolerance of other edges is worked out against
//! their capsules (`near`). The web calls the operations in `calls.rs`; the
//! desktop calls `check` and `fix`. The independent reference is
//! scripts/fixtures/topology_rules_cases.py.

mod areas;
pub mod calls;
mod fixes;
mod lines;

pub use fixes::{Change, fix};

use crate::entity::Shape;
use crate::geom::arrangement::{Area, edge_box, edge_len};
use crate::geom::bulge::bulge_arc;
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges, point_at};
use crate::geometry::{Bounds, empty_bounds, extend_bounds};
use crate::jsmath::{PI, js_hypot, js_max, js_min, stable_sort};
use crate::ops::geoprocess::{Class, class_of};
use crate::vec2::Vec2;

/// The thirteen kinds (the contract's `TopologyRuleKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    MustNotOverlap,
    MustNotHaveGaps,
    MustNotHaveSlivers,
    MustNotHaveDuplicates,
    MustNotHaveDangles,
    MustNotHaveShortEdges,
    MustNotHaveSmallAngles,
    MustBeValid,
    MustNotHaveMissingVertices,
    MustNotOverlapWith,
    MustBeCoveredBy,
    BoundaryMustBeCoveredBy,
    MustBeOnEndOf,
}

/// What a kind's value is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Length,
    Angle,
}

impl Kind {
    pub const ALL: [Kind; 13] = [
        Kind::MustNotOverlap,
        Kind::MustNotHaveGaps,
        Kind::MustNotHaveSlivers,
        Kind::MustNotHaveDuplicates,
        Kind::MustNotHaveDangles,
        Kind::MustNotHaveShortEdges,
        Kind::MustNotHaveSmallAngles,
        Kind::MustBeValid,
        Kind::MustNotHaveMissingVertices,
        Kind::MustNotOverlapWith,
        Kind::MustBeCoveredBy,
        Kind::BoundaryMustBeCoveredBy,
        Kind::MustBeOnEndOf,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Kind::MustNotOverlap => "mustNotOverlap",
            Kind::MustNotHaveGaps => "mustNotHaveGaps",
            Kind::MustNotHaveSlivers => "mustNotHaveSlivers",
            Kind::MustNotHaveDuplicates => "mustNotHaveDuplicates",
            Kind::MustNotHaveDangles => "mustNotHaveDangles",
            Kind::MustNotHaveShortEdges => "mustNotHaveShortEdges",
            Kind::MustNotHaveSmallAngles => "mustNotHaveSmallAngles",
            Kind::MustBeValid => "mustBeValid",
            Kind::MustNotHaveMissingVertices => "mustNotHaveMissingVertices",
            Kind::MustNotOverlapWith => "mustNotOverlapWith",
            Kind::MustBeCoveredBy => "mustBeCoveredBy",
            Kind::BoundaryMustBeCoveredBy => "boundaryMustBeCoveredBy",
            Kind::MustBeOnEndOf => "mustBeOnEndOf",
        }
    }

    pub fn of_key(key: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.key() == key)
    }

    /// Its name in the rules window; a rule between layers writes the other
    /// layer's name for “…” (`named`).
    pub fn label(self) -> &'static str {
        match self {
            Kind::MustNotOverlap => "Çakışmamalı",
            Kind::MustNotHaveGaps => "Boşluk olmamalı",
            Kind::MustNotHaveSlivers => "İnce alan olmamalı",
            Kind::MustNotHaveDuplicates => "Yinelenmemeli",
            Kind::MustNotHaveDangles => "Sarkan uç olmamalı",
            Kind::MustNotHaveShortEdges => "Kısa kenar olmamalı",
            Kind::MustNotHaveSmallAngles => "Küçük açı olmamalı",
            Kind::MustBeValid => "Geçerli olmalı",
            Kind::MustNotHaveMissingVertices => "Ortak sınırda köşe eksik olmamalı",
            Kind::MustNotOverlapWith => "… ile çakışmamalı",
            Kind::MustBeCoveredBy => "… içinde kalmalı",
            Kind::BoundaryMustBeCoveredBy => "Sınırı … sınırlarında olmalı",
            Kind::MustBeOnEndOf => "… çizgilerinin ucunda olmalı",
        }
    }

    /// What it looks at, as the rules window says it.
    pub fn takes(self) -> &'static str {
        match self {
            Kind::MustNotOverlap
            | Kind::MustNotHaveGaps
            | Kind::MustNotHaveSlivers
            | Kind::MustNotHaveMissingVertices
            | Kind::MustNotOverlapWith
            | Kind::BoundaryMustBeCoveredBy => "alanlar",
            Kind::MustNotHaveDuplicates => "çizgiler ve noktalar",
            Kind::MustNotHaveDangles => "çizgiler",
            Kind::MustNotHaveShortEdges | Kind::MustNotHaveSmallAngles | Kind::MustBeValid => {
                "çizgiler ve alanlar"
            }
            Kind::MustBeCoveredBy => "alanlar, çizgiler ve noktalar",
            Kind::MustBeOnEndOf => "noktalar",
        }
    }

    /// Whether the rule is between its layer and another.
    pub fn between(self) -> bool {
        matches!(
            self,
            Kind::MustNotOverlapWith
                | Kind::MustBeCoveredBy
                | Kind::BoundaryMustBeCoveredBy
                | Kind::MustBeOnEndOf
        )
    }

    pub fn value(self) -> Option<ValueKind> {
        match self {
            Kind::MustNotHaveSlivers | Kind::MustNotHaveShortEdges => Some(ValueKind::Length),
            Kind::MustNotHaveSmallAngles => Some(ValueKind::Angle),
            _ => None,
        }
    }

    /// The value when the rule names none: 0.1 m, 0.05 m, 5°.
    pub fn default_value(self) -> Option<f64> {
        match self {
            Kind::MustNotHaveSlivers => Some(0.1),
            Kind::MustNotHaveShortEdges => Some(0.05),
            Kind::MustNotHaveSmallAngles => Some(5.0 * PI / 180.0),
            _ => None,
        }
    }

    /// What the value is called in the rules window.
    pub fn value_label(self) -> Option<&'static str> {
        match self {
            Kind::MustNotHaveSlivers => Some("En az genişlik"),
            Kind::MustNotHaveShortEdges => Some("En kısa kenar"),
            Kind::MustNotHaveSmallAngles => Some("En küçük açı"),
            _ => None,
        }
    }

    /// The rule as a sentence: “…” the other layer's name.
    pub fn named(self, other: Option<&str>) -> String {
        match other {
            Some(name) if self.between() => self.label().replace('…', name),
            _ => self.label().to_owned(),
        }
    }
}

/// A problem's name, by its key.
pub fn problem_label(key: &str) -> &'static str {
    match key {
        "overlap" => "Çakışma",
        "gap" => "Boşluk",
        "sliver" => "İnce alan",
        "duplicateEdge" => "Yinelenen kenar",
        "duplicatePoint" => "Yinelenen nokta",
        "dangle" => "Sarkan uç",
        "shortEdge" => "Kısa kenar",
        "smallAngle" => "Küçük açı",
        "missingVertex" => "Komşuda eksik köşe",
        "outside" => "Dışarıda kalan",
        "uncoveredBoundary" => "Sınırda olmayan kenar",
        "notOnEnd" => "Çizgi ucunda değil",
        other => crate::ops::geoprocess::validity::Kind::of_key(other)
            .map_or("Sorun", crate::ops::geoprocess::validity::Kind::text),
    }
}

/// A fix's name, by its key.
pub fn fix_label(key: &str) -> &'static str {
    match key {
        "subtractFirst" => "Birinci nesneden çıkar",
        "subtractSecond" => "İkinci nesneden çıkar",
        "mergeNeighbour" => "Komşuya kat",
        "deletePart" => "Sil",
        "deleteDuplicate" => "Yineleneni sil",
        "snapEnd" => "Ucu en yakın çizgiye taşı",
        "removeVertex" => "Köşeyi sil",
        "deleteObject" => "Sil",
        "repair" => "Onar",
        "addVertex" => "Komşuya köşe ekle",
        "clipOutside" => "Dışarıda kalanı kes",
        "snapToEnd" => "En yakın uca taşı",
        _ => "Düzelt",
    }
}

/// What a finding's measure is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasureKind {
    Area,
    Length,
    Angle,
    Distance,
}

impl MeasureKind {
    pub fn key(self) -> &'static str {
        match self {
            MeasureKind::Area => "area",
            MeasureKind::Length => "length",
            MeasureKind::Angle => "angle",
            MeasureKind::Distance => "distance",
        }
    }

    pub fn of_key(key: &str) -> Option<MeasureKind> {
        [
            MeasureKind::Area,
            MeasureKind::Length,
            MeasureKind::Angle,
            MeasureKind::Distance,
        ]
        .into_iter()
        .find(|k| k.key() == key)
    }
}

/// A rule as the check takes it: the layers by id.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub id: String,
    pub kind: Kind,
    pub layer: String,
    pub other: Option<String>,
    pub value: Option<f64>,
}

impl Rule {
    /// Its value: the rule's, or the kind's default.
    pub fn value(&self) -> f64 {
        self.value.or(self.kind.default_value()).unwrap_or_default()
    }
}

/// A finding marked as left on purpose: its rule's id, its objects' uids in
/// the finding's order, its place.
#[derive(Clone, Debug, PartialEq)]
pub struct Exception {
    pub rule: String,
    pub objects: Vec<String>,
    pub at: Vec2,
}

/// The objects the check looks at: each one's shape, layer and uid.
pub struct Objects<'a> {
    pub shapes: &'a [Shape],
    pub layers: &'a [String],
    pub uids: &'a [String],
}

/// Where a fix works on an object's vertices: a part, a ring (0 its outer,
/// 1… its holes; a path's 0) and a vertex or an edge in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spot {
    pub part: usize,
    pub ring: usize,
    pub index: usize,
}

/// One finding (docs/adr/0202 §3).
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    /// The rule's place in the rules.
    pub rule: usize,
    pub problem: &'static str,
    /// The objects, by their place among the objects; the order a finding says them in.
    pub objects: Vec<usize>,
    pub at: Vec2,
    /// What is shown: the problem's regions and its edges.
    pub bounds: Bounds,
    pub regions: Vec<Area>,
    pub edges: Vec<Edge>,
    pub measure: Option<f64>,
    pub measure_kind: Option<MeasureKind>,
    pub fixes: Vec<&'static str>,
    pub exception: bool,
    /// What a fix works on: the vertex a fix removes, the edge a vertex is
    /// added to, the part of an area, the end or the point moved (§4).
    pub spot: Option<Spot>,
    /// Where a vertex, an end or a point goes.
    pub target: Option<Vec2>,
    /// The object a fix acts on besides the finding's first: the neighbour
    /// Komşuya kat merges into, the duplicate Yineleneni sil deletes.
    pub subject: Option<usize>,
}

impl Finding {
    fn new(rule: usize, problem: &'static str, objects: Vec<usize>, at: Vec2) -> Finding {
        let mut bounds = empty_bounds();
        extend_bounds(&mut bounds, at, 0.0);
        Finding {
            rule,
            problem,
            objects,
            at,
            bounds,
            regions: Vec::new(),
            edges: Vec::new(),
            measure: None,
            measure_kind: None,
            fixes: Vec::new(),
            exception: false,
            spot: None,
            target: None,
            subject: None,
        }
    }

    fn measured(mut self, kind: MeasureKind, value: f64) -> Finding {
        self.measure = Some(value);
        self.measure_kind = Some(kind);
        self
    }

    fn fixes(mut self, keys: &[&'static str]) -> Finding {
        self.fixes = keys.to_vec();
        self
    }

    /// The box of what is shown, its place in it.
    fn shown(mut self, regions: Vec<Area>, edges: Vec<Edge>) -> Finding {
        let mut b = empty_bounds();
        extend_bounds(&mut b, self.at, 0.0);
        for a in &regions {
            for e in crate::ops::geoprocess::boundary_edges(std::slice::from_ref(a)) {
                grow(&mut b, &edge_box(&e));
            }
        }
        for e in &edges {
            grow(&mut b, &edge_box(e));
        }
        self.bounds = b;
        self.regions = regions;
        self.edges = edges;
        self
    }
}

/// The check's answer: the findings in order, and how many objects each rule looked at.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Checked {
    pub findings: Vec<Finding>,
    pub looked: Vec<usize>,
}

// ── Shared helpers ─────────────────────────────────────────────────────

fn grow(b: &mut Bounds, by: &Bounds) {
    b.min_x = js_min(b.min_x, by.min_x);
    b.min_y = js_min(b.min_y, by.min_y);
    b.max_x = js_max(b.max_x, by.max_x);
    b.max_y = js_max(b.max_y, by.max_y);
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    js_hypot(b.x - a.x, b.y - a.y)
}

/// A ring or a path of an object as written: its part, its place in the
/// part (0 the outer ring or the path, 1… holes), its vertices and bulges,
/// whether it closes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Written {
    pub part: usize,
    pub ring: usize,
    pub pts: Vec<Vec2>,
    pub bulges: Vec<f64>,
    pub closed: bool,
}

impl Written {
    /// How many edges: a ring's every vertex starts one, a path's but the last.
    pub fn edge_count(&self) -> usize {
        if self.closed {
            self.pts.len()
        } else {
            self.pts.len().saturating_sub(1)
        }
    }

    /// Edge `k` from vertex k to the next, an arc where it bulges.
    pub fn edge(&self, k: usize) -> Edge {
        let n = self.pts.len();
        let (a, b) = (self.pts[k], self.pts[(k + 1) % n]);
        match bulge_arc(a, b, self.bulges.get(k).copied().unwrap_or(0.0)) {
            Some(arc) => Edge::Arc {
                c: arc.c,
                r: arc.r,
                a0: arc.a0,
                sweep: arc.sweep,
            },
            None => Edge::Seg { a, b },
        }
    }
}

/// An object's rings and paths as written: a line's one edge, a polyline's
/// parts, an area's parts with their holes; none for other kinds.
pub(crate) fn written(s: &Shape) -> Vec<Written> {
    let mut out = Vec::new();
    match s {
        Shape::Line { a, b } => out.push(Written {
            part: 0,
            ring: 0,
            pts: vec![*a, *b],
            bulges: vec![0.0],
            closed: false,
        }),
        Shape::Polyline { .. } => {
            for (k, part) in crate::entity::area_parts(s).iter().enumerate() {
                if let Shape::Polyline { pts, bulges, .. } = part {
                    out.push(Written {
                        part: k,
                        ring: 0,
                        pts: pts.clone(),
                        bulges: bulges_of(pts.len(), bulges.as_deref()),
                        closed: false,
                    });
                }
            }
        }
        Shape::Polygon { .. } => {
            for (k, part) in crate::entity::area_parts(s).iter().enumerate() {
                if let Shape::Polygon {
                    pts, bulges, holes, ..
                } = part
                {
                    out.push(Written {
                        part: k,
                        ring: 0,
                        pts: pts.clone(),
                        bulges: bulges_of(pts.len(), bulges.as_deref()),
                        closed: true,
                    });
                    for (h, ring) in holes.iter().flatten().enumerate() {
                        out.push(Written {
                            part: k,
                            ring: h + 1,
                            pts: ring.pts.clone(),
                            bulges: bulges_of(ring.pts.len(), ring.bulges.as_deref()),
                            closed: true,
                        });
                    }
                }
            }
        }
        _ => {}
    }
    out
}

fn bulges_of(n: usize, b: Option<&[f64]>) -> Vec<f64> {
    (0..n)
        .map(|i| b.and_then(|b| b.get(i)).copied().unwrap_or(0.0))
        .collect()
}

/// The pieces of the boundary of the band within `t` of an edge (its
/// capsule): the two sides and the two ends' circles (as half circles).
fn capsule(f: &Edge, t: f64) -> Vec<Edge> {
    let circle = |c: Vec2| {
        [
            Edge::Arc {
                c,
                r: t,
                a0: 0.0,
                sweep: PI,
            },
            Edge::Arc {
                c,
                r: t,
                a0: PI,
                sweep: PI,
            },
        ]
    };
    let (a, b) = (point_at(f, 0.0), point_at(f, 1.0));
    let mut out: Vec<Edge> = Vec::with_capacity(6);
    match *f {
        Edge::Seg { a, b } => {
            let l = dist(a, b);
            if l > 0.0 {
                let n = Vec2::new(-(b.y - a.y) / l * t, (b.x - a.x) / l * t);
                out.push(Edge::Seg {
                    a: Vec2::new(a.x + n.x, a.y + n.y),
                    b: Vec2::new(b.x + n.x, b.y + n.y),
                });
                out.push(Edge::Seg {
                    a: Vec2::new(a.x - n.x, a.y - n.y),
                    b: Vec2::new(b.x - n.x, b.y - n.y),
                });
            }
        }
        Edge::Arc { c, r, a0, sweep } => {
            out.push(Edge::Arc {
                c,
                r: r + t,
                a0,
                sweep,
            });
            if r > t {
                out.push(Edge::Arc {
                    c,
                    r: r - t,
                    a0,
                    sweep,
                });
            }
        }
    }
    out.extend(circle(a));
    if dist(a, b) > 0.0 {
        out.extend(circle(b));
    }
    out
}

/// The stretches of `e` (parameters, sorted and apart) within `t` of `f`:
/// `e` cut where it crosses the capsule's boundary, a piece kept by its middle.
fn near(e: &Edge, f: &Edge, t: f64) -> Vec<(f64, f64)> {
    let mut cuts = vec![0.0, 1.0];
    for g in capsule(f, t) {
        for h in intersect_edges(e, &g) {
            if h.t > 0.0 && h.t < 1.0 {
                cuts.push(h.t);
            }
        }
    }
    stable_sort(&mut cuts, &mut |a, b| a.total_cmp(b));
    let mut out: Vec<(f64, f64)> = Vec::new();
    for w in cuts.windows(2) {
        let (s0, s1) = (w[0], w[1]);
        if s1 - s0 <= 1e-15 {
            continue;
        }
        if closest_on_edge(f, point_at(e, (s0 + s1) / 2.0)).d <= t {
            match out.last_mut() {
                Some(last) if last.1 >= s0 => last.1 = s1,
                _ => out.push((s0, s1)),
            }
        }
    }
    out
}

/// The union of `e`'s stretches within `t` of any of `edges` (sorted, apart).
fn near_all<'a>(e: &Edge, edges: impl IntoIterator<Item = &'a Edge>, t: f64) -> Vec<(f64, f64)> {
    let bx = edge_box(e);
    let mut all: Vec<(f64, f64)> = Vec::new();
    for f in edges {
        let fb = edge_box(f);
        if fb.min_x > bx.max_x + t
            || fb.max_x < bx.min_x - t
            || fb.min_y > bx.max_y + t
            || fb.max_y < bx.min_y - t
        {
            continue;
        }
        all.extend(near(e, f, t));
    }
    stable_sort(&mut all, &mut |a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f64)> = Vec::new();
    for (s0, s1) in all {
        match out.last_mut() {
            Some(last) if s0 <= last.1 + 1e-12 => last.1 = js_max(last.1, s1),
            _ => out.push((s0, s1)),
        }
    }
    out
}

/// Whether every point of `e` is within `t` of the edges.
fn within_all<'a>(e: &Edge, edges: impl IntoIterator<Item = &'a Edge>, t: f64) -> bool {
    let s = near_all(e, edges, t);
    s.len() == 1 && s[0].0 <= 1e-9 && s[0].1 >= 1.0 - 1e-9
}

/// The piece of an edge between two parameters.
fn sub_edge(e: &Edge, s0: f64, s1: f64) -> Edge {
    match *e {
        Edge::Seg { .. } => Edge::Seg {
            a: point_at(e, s0),
            b: point_at(e, s1),
        },
        Edge::Arc { c, r, a0, sweep } => Edge::Arc {
            c,
            r,
            a0: a0 + sweep * s0,
            sweep: sweep * (s1 - s0),
        },
    }
}

/// The place half way along a run of edges.
fn middle_of(run: &[Edge]) -> Vec2 {
    let total: f64 = run.iter().map(edge_len).sum();
    let mut left = total / 2.0;
    for e in run {
        let l = edge_len(e);
        if left <= l {
            return point_at(e, if l > 0.0 { left / l } else { 0.0 });
        }
        left -= l;
    }
    run.last().map_or(Vec2::default(), |e| point_at(e, 1.0))
}

/// Pairs of boxes that meet (grown by `t`): `a` against `b`, or `a` against
/// itself (`b` none: i < j); in the order of `a`, then of `b`.
fn pairs(a: &[Bounds], b: Option<&[Bounds]>, t: f64) -> Vec<(usize, usize)> {
    let meet = |p: &Bounds, q: &Bounds| {
        p.min_x <= q.max_x + t
            && q.min_x <= p.max_x + t
            && p.min_y <= q.max_y + t
            && q.min_y <= p.max_y + t
    };
    let other = b.unwrap_or(a);
    // A sweep over x: each box of `other` by its least x.
    let mut order: Vec<usize> = (0..other.len()).collect();
    stable_sort(&mut order, &mut |&i, &j| {
        other[i].min_x.total_cmp(&other[j].min_x)
    });
    let mut out = Vec::new();
    for (i, p) in a.iter().enumerate() {
        let mut found: Vec<usize> = Vec::new();
        for &j in &order {
            let q = &other[j];
            if q.min_x > p.max_x + t {
                break;
            }
            if (b.is_some() || j > i) && meet(p, q) {
                found.push(j);
            }
        }
        found.sort_unstable();
        out.extend(found.into_iter().map(|j| (i, j)));
    }
    out
}

/// An object's class and box.
pub(crate) struct Taken {
    pub class: Class,
    pub bounds: Bounds,
}

pub(crate) fn taken(s: &Shape) -> Taken {
    let class = class_of(s);
    let bounds = crate::ops::geoprocess::clip::class_box(&class).unwrap_or_else(empty_bounds);
    Taken { class, bounds }
}

/// The objects of a layer whose class a rule looks at, in their order.
fn of_layer(
    objects: &Objects<'_>,
    taken: &[Taken],
    layer: &str,
    want: &[fn(&Class) -> bool],
) -> Vec<usize> {
    (0..objects.shapes.len())
        .filter(|&i| objects.layers[i] == layer && want.iter().any(|w| w(&taken[i].class)))
        .collect()
}

pub(crate) fn is_areas(c: &Class) -> bool {
    matches!(c, Class::Areas(a) if !a.is_empty())
}

pub(crate) fn is_paths(c: &Class) -> bool {
    matches!(c, Class::Paths(p) if !p.is_empty())
}

pub(crate) fn is_points(c: &Class) -> bool {
    matches!(c, Class::Points(p) if !p.is_empty())
}

/// Every rule run over the objects (docs/adr/0202 §2), in the rules' order;
/// each finding marked when an exception names it (§3).
pub fn check(
    objects: &Objects<'_>,
    rules: &[Rule],
    tolerance: f64,
    exceptions: &[Exception],
) -> Checked {
    let taken: Vec<Taken> = objects.shapes.iter().map(taken).collect();
    let t = tolerance;
    let mut out = Checked::default();
    for (r, rule) in rules.iter().enumerate() {
        let layer = rule.layer.as_str();
        let other = rule.other.as_deref().unwrap_or("");
        let (found, looked) = match rule.kind {
            Kind::MustNotOverlap => {
                let idx = of_layer(objects, &taken, layer, &[is_areas]);
                (areas::overlaps(&taken, r, &idx, None, t), idx.len())
            }
            Kind::MustNotOverlapWith => {
                let idx = of_layer(objects, &taken, layer, &[is_areas]);
                let with = of_layer(objects, &taken, other, &[is_areas]);
                (areas::overlaps(&taken, r, &idx, Some(&with), t), idx.len())
            }
            Kind::MustNotHaveGaps => {
                let idx = of_layer(objects, &taken, layer, &[is_areas]);
                (areas::gaps(&taken, r, &idx, t), idx.len())
            }
            Kind::MustNotHaveSlivers => {
                let idx = of_layer(objects, &taken, layer, &[is_areas]);
                (areas::slivers(&taken, r, &idx, rule.value(), t), idx.len())
            }
            Kind::MustNotHaveMissingVertices => {
                let idx: Vec<usize> = of_layer(objects, &taken, layer, &[is_areas])
                    .into_iter()
                    .filter(|&i| matches!(objects.shapes[i], Shape::Polygon { .. }))
                    .collect();
                (
                    areas::missing_vertices(objects.shapes, &taken, r, &idx, t),
                    idx.len(),
                )
            }
            Kind::MustBeCoveredBy => {
                let idx = of_layer(objects, &taken, layer, &[is_areas, is_paths, is_points]);
                let by = of_layer(objects, &taken, other, &[is_areas]);
                (areas::covered_by(&taken, r, &idx, &by, t), idx.len())
            }
            Kind::MustNotHaveDuplicates => {
                let idx = of_layer(objects, &taken, layer, &[is_paths, is_points]);
                (
                    lines::duplicates(objects.shapes, &taken, r, &idx, t),
                    idx.len(),
                )
            }
            Kind::MustNotHaveDangles => {
                let idx = of_layer(objects, &taken, layer, &[is_paths]);
                let areas = of_layer(objects, &taken, layer, &[is_areas]);
                (
                    lines::dangles(objects.shapes, &taken, r, &idx, &areas, t),
                    idx.len(),
                )
            }
            Kind::MustNotHaveShortEdges => {
                let idx = of_layer(objects, &taken, layer, &[is_paths, is_areas]);
                (
                    lines::short_edges(objects.shapes, r, &idx, rule.value()),
                    idx.len(),
                )
            }
            Kind::MustNotHaveSmallAngles => {
                let idx = of_layer(objects, &taken, layer, &[is_paths, is_areas]);
                (
                    lines::small_angles(objects.shapes, r, &idx, rule.value(), t),
                    idx.len(),
                )
            }
            Kind::MustBeValid => {
                let idx = of_layer(objects, &taken, layer, &[is_paths, is_areas]);
                (lines::validity(objects.shapes, r, &idx), idx.len())
            }
            Kind::BoundaryMustBeCoveredBy => {
                let idx = of_layer(objects, &taken, layer, &[is_areas]);
                let by = of_layer(objects, &taken, other, &[is_areas, is_paths]);
                (lines::boundary_covered(&taken, r, &idx, &by, t), idx.len())
            }
            Kind::MustBeOnEndOf => {
                let idx = of_layer(objects, &taken, layer, &[is_points]);
                let by = of_layer(objects, &taken, other, &[is_paths]);
                (lines::on_end_of(&taken, r, &idx, &by, t), idx.len())
            }
        };
        out.looked.push(looked);
        for mut f in found {
            f.exception = exceptions.iter().any(|x| {
                x.rule == rule.id
                    && x.objects.len() == f.objects.len()
                    && x.objects
                        .iter()
                        .zip(&f.objects)
                        .all(|(u, &i)| *u == objects.uids[i])
                    && dist(x.at, f.at) <= t
            });
            out.findings.push(f);
        }
    }
    out
}

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
pub mod incremental;
mod lines;

pub use fixes::{Change, fix};

use crate::entity::Shape;
use crate::geom::arrangement::{Area, edge_box, edge_len};
use crate::geom::bulge::bulge_arc;
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges, point_at};
use crate::geometry::{Bounds, empty_bounds, extend_bounds};
use crate::jsmath::{PI, js_hypot, js_max, js_min, stable_sort};
use crate::ops::geoprocess::{Class, class_of};
use crate::store::rtree::PackedTree;
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

/// What a long check tells and asks whoever watches it (docs/adr/0756): how far it has come, and whether to stop. The
/// desktop's Topoloji tab watches it on its own thread, a command run as a job through its progress; a check no one
/// watches is [`Unwatched`].
pub trait Watch {
    /// The share of the check done, 0–1, said now and then.
    fn progress(&self, done: f64);
    /// A stretch whose progress is not measured (one overlay of a whole layer) from the share `from` to `until`,
    /// expected to take about `seconds`: a bar may move on by the time that went by.
    fn stretch(&self, from: f64, until: f64, seconds: f64) {
        let _ = (until, seconds);
        self.progress(from);
    }
    /// Whether the check is to stop: it then gives up and answers nothing.
    fn stopped(&self) -> bool;
}

/// A check no one watches: it never stops and says nothing.
pub struct Unwatched;

impl Watch for Unwatched {
    fn progress(&self, _: f64) {}

    fn stopped(&self) -> bool {
        false
    }
}

/// A stretch of a watched check, a rule's or a part of it: where in the whole it starts and how much of it it is.
#[derive(Clone, Copy)]
pub(crate) struct Pace<'a> {
    watch: &'a dyn Watch,
    from: f64,
    share: f64,
}

impl<'a> Pace<'a> {
    /// A stretch within this one, from `a` to `b` of it (0–1).
    pub(crate) fn within(self, a: f64, b: f64) -> Pace<'a> {
        Pace {
            watch: self.watch,
            from: self.from + self.share * a,
            share: self.share * (b - a),
        }
    }

    /// A stretch not measured from `a` to `b` of this one, expected to take about `seconds`; whether to go on.
    pub(crate) fn stretch(&self, a: f64, b: f64, seconds: f64) -> bool {
        if self.watch.stopped() {
            return false;
        }
        self.watch.stretch(
            self.from + self.share * a,
            self.from + self.share * b,
            seconds,
        );
        true
    }

    /// Whether to go on, `done` of `of` gone by: the stop is asked and the share said at the first and every
    /// sixteenth after it.
    pub(crate) fn go(&self, done: usize, of: usize) -> bool {
        if !done.is_multiple_of(16) {
            return true;
        }
        if self.watch.stopped() {
            return false;
        }
        let part = if of == 0 {
            1.0
        } else {
            js_min(done as f64 / of as f64, 1.0)
        };
        self.watch.progress(self.from + self.share * part);
        true
    }
}

/// Boxes asked which of them may meet another (docs/adr/0756): the finite ones in a packed tree, those that are not set
/// apart and always asked, the empty ones never (they meet nothing). It only narrows: each caller applies its own
/// exact test after it, so what it keeps is what a walk over every box keeps, in the boxes' order.
pub(crate) struct Boxes {
    tree: PackedTree,
    odd: Vec<u32>,
}

impl Boxes {
    pub(crate) fn new(boxes: impl IntoIterator<Item = Bounds>) -> Boxes {
        let mut entries = Vec::new();
        let mut odd = Vec::new();
        for (i, b) in boxes.into_iter().enumerate() {
            let finite = b.min_x.is_finite()
                && b.min_y.is_finite()
                && b.max_x.is_finite()
                && b.max_y.is_finite();
            if finite && b.min_x <= b.max_x && b.min_y <= b.max_y {
                entries.push((i as u32, b));
            } else if !(b.min_x > b.max_x || b.min_y > b.max_y) {
                odd.push(i as u32);
            }
        }
        Boxes {
            tree: PackedTree::build(&entries),
            odd,
        }
    }

    /// The places of the boxes that may meet `b` grown by `t`, in order: never fewer than meet, a few more (the
    /// query grows by a margin over the rounding of a test that grows the other box instead).
    pub(crate) fn near(&self, b: &Bounds, t: f64, out: &mut Vec<u32>) {
        out.clear();
        let m = js_max(
            js_max(b.min_x.abs(), b.max_x.abs()),
            js_max(b.min_y.abs(), b.max_y.abs()),
        );
        let grown = t + (m + t.abs()) * 1e-12;
        let q = Bounds {
            min_x: b.min_x - grown,
            min_y: b.min_y - grown,
            max_x: b.max_x + grown,
            max_y: b.max_y + grown,
        };
        self.tree.search(&q, out);
        out.extend_from_slice(&self.odd);
        out.sort_unstable();
    }
}

/// The distance from `p` to a box (0 inside).
pub(crate) fn box_dist(b: &Bounds, p: Vec2) -> f64 {
    let dx = js_max(js_max(b.min_x - p.x, p.x - b.max_x), 0.0);
    let dy = js_max(js_max(b.min_y - p.y, p.y - b.max_y), 0.0);
    js_hypot(dx, dy)
}

/// A distance's lower bound with room for its rounding: what a best-first search may order a box by when what it
/// holds is measured by another computation (`closest_on_edge`, `dist`).
pub(crate) fn below(d: f64, p: Vec2) -> f64 {
    js_max(d - (p.x.abs() + p.y.abs() + d) * 1e-12, 0.0)
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
    // The boxes of `other` near each of `a` from a tree (docs/adr/0756), then the test itself.
    let near = Boxes::new(other.iter().copied());
    let mut found = Vec::new();
    let mut out = Vec::new();
    for (i, p) in a.iter().enumerate() {
        near.near(p, t, &mut found);
        out.extend(
            found
                .iter()
                .map(|&j| j as usize)
                .filter(|&j| (b.is_some() || j > i) && meet(p, &other[j]))
                .map(|j| (i, j)),
        );
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
    check_watched(objects, rules, tolerance, exceptions, &Unwatched).unwrap_or_default()
}

/// What a rule looks at: its layer's objects of the classes it takes, and those of the other layer (or, for Sarkan uç
/// olmamalı, its layer's areas the ends may meet).
fn looks(objects: &Objects<'_>, taken: &[Taken], rule: &Rule) -> (Vec<usize>, Vec<usize>) {
    let layer = rule.layer.as_str();
    let other = rule.other.as_deref().unwrap_or("");
    let of = |l: &str, want: &[fn(&Class) -> bool]| of_layer(objects, taken, l, want);
    match rule.kind {
        Kind::MustNotOverlap | Kind::MustNotHaveGaps | Kind::MustNotHaveSlivers => {
            (of(layer, &[is_areas]), Vec::new())
        }
        Kind::MustNotOverlapWith => (of(layer, &[is_areas]), of(other, &[is_areas])),
        Kind::MustNotHaveMissingVertices => (
            of(layer, &[is_areas])
                .into_iter()
                .filter(|&i| matches!(objects.shapes[i], Shape::Polygon { .. }))
                .collect(),
            Vec::new(),
        ),
        Kind::MustBeCoveredBy => (
            of(layer, &[is_areas, is_paths, is_points]),
            of(other, &[is_areas]),
        ),
        Kind::MustNotHaveDuplicates => (of(layer, &[is_paths, is_points]), Vec::new()),
        Kind::MustNotHaveDangles => (of(layer, &[is_paths]), of(layer, &[is_areas])),
        Kind::MustNotHaveShortEdges | Kind::MustNotHaveSmallAngles | Kind::MustBeValid => {
            (of(layer, &[is_paths, is_areas]), Vec::new())
        }
        Kind::BoundaryMustBeCoveredBy => (of(layer, &[is_areas]), of(other, &[is_areas, is_paths])),
        Kind::MustBeOnEndOf => (of(layer, &[is_points]), of(other, &[is_paths])),
    }
}

/// How much of a check a rule is, by what it looks at and the time its kind takes for each (docs/adr/0756: about that
/// many microseconds an object at 100 000 parcels): Boşluk olmamalı's overlay of the whole layer the most.
fn weight(kind: Kind, looked: usize, with: usize) -> f64 {
    let each = match kind {
        Kind::MustNotHaveGaps => 20.0,
        Kind::MustNotHaveDangles => 8.0,
        Kind::MustNotOverlapWith
        | Kind::MustNotHaveMissingVertices
        | Kind::MustBeCoveredBy
        | Kind::BoundaryMustBeCoveredBy => 5.0,
        Kind::MustNotHaveDuplicates => 4.0,
        Kind::MustNotOverlap | Kind::MustBeOnEndOf => 2.0,
        Kind::MustNotHaveSlivers
        | Kind::MustNotHaveShortEdges
        | Kind::MustNotHaveSmallAngles
        | Kind::MustBeValid => 1.0,
    };
    1.0 + each * (looked + with) as f64
}

/// `check` watched (docs/adr/0756): how far it has come said as the rules go, each its share by its [`weight`];
/// none when it was asked to stop.
pub fn check_watched(
    objects: &Objects<'_>,
    rules: &[Rule],
    tolerance: f64,
    exceptions: &[Exception],
    watch: &dyn Watch,
) -> Option<Checked> {
    let taken: Vec<Taken> = objects.shapes.iter().map(taken).collect();
    let t = tolerance;
    let lists: Vec<(Vec<usize>, Vec<usize>)> =
        rules.iter().map(|r| looks(objects, &taken, r)).collect();
    let weights: Vec<f64> = rules
        .iter()
        .zip(&lists)
        .map(|(r, (idx, with))| weight(r.kind, idx.len(), with.len()))
        .collect();
    let whole: f64 = weights.iter().sum();
    let mut from = 0.0;
    let mut out = Checked::default();
    for (r, (rule, (idx, with))) in rules.iter().zip(&lists).enumerate() {
        if watch.stopped() {
            return None;
        }
        let share = if whole > 0.0 { weights[r] / whole } else { 0.0 };
        let pace = Pace { watch, from, share };
        from += share;
        let found = match rule.kind {
            Kind::MustNotOverlap => areas::overlaps(&taken, r, idx, None, t, pace),
            Kind::MustNotOverlapWith => areas::overlaps(&taken, r, idx, Some(with), t, pace),
            Kind::MustNotHaveGaps => areas::gaps(&taken, r, idx, t, pace),
            Kind::MustNotHaveSlivers => areas::slivers(&taken, r, idx, rule.value(), t, pace),
            Kind::MustNotHaveMissingVertices => {
                areas::missing_vertices(objects.shapes, &taken, r, idx, t, pace)
            }
            Kind::MustBeCoveredBy => areas::covered_by(&taken, r, idx, with, t, pace),
            Kind::MustNotHaveDuplicates => {
                lines::duplicates(objects.shapes, &taken, r, idx, t, pace)
            }
            Kind::MustNotHaveDangles => {
                lines::dangles(objects.shapes, &taken, r, idx, with, t, pace)
            }
            Kind::MustNotHaveShortEdges => {
                lines::short_edges(objects.shapes, r, idx, rule.value(), pace)
            }
            Kind::MustNotHaveSmallAngles => {
                lines::small_angles(objects.shapes, r, idx, rule.value(), t, pace)
            }
            Kind::MustBeValid => lines::validity(objects.shapes, r, idx, pace),
            Kind::BoundaryMustBeCoveredBy => lines::boundary_covered(&taken, r, idx, with, t, pace),
            Kind::MustBeOnEndOf => lines::on_end_of(&taken, r, idx, with, t, pace),
        };
        out.looked.push(idx.len());
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
    if watch.stopped() {
        return None;
    }
    watch.progress(1.0);
    Some(out)
}

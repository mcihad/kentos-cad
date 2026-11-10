//! A roof's plan (docs/adr/8044 §2; M44, ADV-20). Its eaves line (saçak çizgisi) and each edge's slope give its
//! planes: an edge with a slope (eğim: rise over run, per cent) lifts a plane from its line into the roof at that slope;
//! an edge without one is a gable end (kalkan), where the roof stops at a wall standing up to it. Over a point inside
//! the eaves the roof stands at the lowest of the planes there: their lower envelope, which for a convex eaves line is
//! its straight skeleton's roof (Revit's roof by footprint: each edge "defines slope" or not).
//!
//! - **Faces.** One for each sloping edge, where its plane is the lowest. Every edge sloping is a hip roof (kırma
//!   çatı), two opposite ones a gable roof (beşik), one a shed (tek eğimli), none a flat roof (düz çatı).
//! - **Lines.** The eaves (saçak) a face starts at, the gable ends (kalkan), and where two faces meet: a hip (kırma)
//!   down to a corner the eaves turn out at, a valley (dere) down to one they turn in at, else a ridge (mahya): level,
//!   or sloping between two of the roof's inner points.
//! - **Plan** (Mimarlar Odası, Mimarlık Hizmetleri Şartnamesi 14.3.3 u, 14.5.3 w: "Meyiller, su toplama yerleri,
//!   dereler ... gösterilir ve gerekli kotlar verilir"): the eaves and gable ends at the object's pen, ridges, hips and
//!   valleys narrow, an arrow down each face with its slope ("%33"), the ridge's height ("+8.45").
//! - **Not yet** (docs/adr/8044 §9): an eaves line that turns in (an L or a T): its straight skeleton needs split events.
//!   [`faces`] refuses it ([`Problem::Concave`]).

use crate::Vec2;
use crate::block::{Piece, PieceStroke};
use crate::entity::Shape;
use crate::geom::arch::{HEAD_MM, LABEL_MM, Level};
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::text::TextAlign;

/// Lengths, heights and areas closer than this are one, metres.
const EPS: f64 = 1e-9;
/// A slope arrow's length on paper, millimetres.
pub const ARROW_MM: f64 = 12.0;
/// The gap between an arrow and its slope, millimetres on paper.
const WORDS_GAP_MM: f64 = 1.0;
/// Ridges, hips and valleys: the narrow pen's share of the object's (ISO 128-20's narrow line, half the wide).
pub const NARROW: f64 = 0.5;
/// A snow guard's ticks on paper, millimetres: across it, and apart along it.
const GUARD_TICK_MM: f64 = 1.5;
const GUARD_STEP_MM: f64 = 5.0;

/// A roof's fields beside its eaves line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoofSpec {
    /// Each edge's slope, per cent (`pts[i]` to `pts[i + 1]`, the last back to the first); none: a gable end.
    pub slopes: Vec<Option<f64>>,
    /// The eaves' height over ±0.00, metres: where the ridge's height is counted from.
    pub base: f64,
    /// Drawn as the plan of its own storey shows it (docs/adr/8044 §3): over the plan's cut, its eaves line alone,
    /// dashed and narrow, as Mimarlar Odası asks of what lies between the plan's cut and the ceiling ("Saçak … nokta nokta
    /// işlenir"). A view's, never read from nor written to JSON (as a stair's `upper`, a slab's `overhead`).
    pub overhead: bool,
    /// Its chimneys, vents and roof windows (docs/adr/8044 §6).
    pub parts: Vec<Part>,
    /// Its gutter's width outside every sloping eaves edge (docs/adr/8044 §8); none: no gutter.
    pub gutter: Option<f64>,
    /// Its snow guard's distance in from every sloping eaves edge (docs/adr/8044 §8); none: no snow guard.
    pub snow_guard: Option<f64>,
}

/// What a roof's part is (docs/adr/8044 §6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PartKind {
    /// Duman bacası.
    #[default]
    Chimney,
    /// Hava bacası.
    Vent,
    /// Çatı penceresi.
    Window,
    /// İniş borusu (docs/adr/8044 §8): under the gutter, its width its diameter.
    Downpipe,
}

impl PartKind {
    const ALL: [PartKind; 4] = [
        PartKind::Chimney,
        PartKind::Vent,
        PartKind::Window,
        PartKind::Downpipe,
    ];

    fn name(self) -> &'static str {
        match self {
            PartKind::Chimney => "chimney",
            PartKind::Vent => "vent",
            PartKind::Window => "window",
            PartKind::Downpipe => "downpipe",
        }
    }
}

impl crate::api::json::ToJson for PartKind {
    fn write_json(&self, out: &mut String) {
        self.name().write_json(out);
    }
}

impl crate::api::json::FromJson for PartKind {
    fn from_json(v: &crate::api::json::Json) -> Result<Self, String> {
        match v {
            crate::api::json::Json::Str(s) => PartKind::ALL
                .into_iter()
                .find(|k| k.name() == s)
                .ok_or_else(|| format!("çatı parçası türü değil: {s}")),
            _ => Err("çatı parçasının türü bekleniyordu".into()),
        }
    }
}

/// A roof's part (docs/adr/8044 §6): a chimney or a vent standing through the roof, a window in its face.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part {
    pub kind: PartKind,
    /// Its middle in plan.
    pub at: Vec2,
    /// A window's along its face's eaves; a chimney's or a vent's along its turn.
    pub width: f64,
    /// A window's down its face's slope, in the roof's plane; a chimney's or a vent's across its width.
    pub depth: f64,
    /// A chimney's or a vent's turn in plan, radians; none: 0. A window takes its face's.
    pub rotation: Option<f64>,
    /// A chimney's top over the ridge, a vent's over the roof at its highest corner, metres.
    pub rise: f64,
}

crate::json_struct!(Part {
    kind,
    at,
    width,
    depth,
    rotation,
    rise
});

/// The part's outline in plan, counter-clockwise (docs/adr/8044 §6): a chimney or a vent its rectangle turned about its
/// middle; a window its rectangle in the face holding its middle, its width along that face's eaves, its depth down the
/// fall line as the plan shows it (its depth in the roof's plane over √(1 + slope²)). None for a window whose middle is
/// on no face (a flat roof's, or outside the eaves).
pub fn part_ring(pts: &[Vec2], spec: &RoofSpec, part: &Part) -> Option<Vec<Vec2>> {
    let (along, down, w, d) = match part.kind {
        // A downpipe is round and hangs off the gutter: no ring on the roof.
        PartKind::Downpipe => return None,
        PartKind::Chimney | PartKind::Vent => {
            let r = part.rotation.unwrap_or(0.0);
            let (sin, cos) = (crate::jsmath::sin(r), crate::jsmath::cos(r));
            (
                Vec2::new(cos, sin),
                Vec2::new(-sin, cos),
                part.width,
                part.depth,
            )
        }
        PartKind::Window => {
            let (ring, edges) = checked(pts, spec).ok()?;
            let n = ring.len();
            let face = faces(pts, spec)
                .ok()?
                .into_iter()
                .find(|f| inside(&f.ring, part.at))?;
            let k = edges.iter().position(|e| e.0 == face.edge)?;
            let (a, b) = (ring[k], ring[(k + 1) % n]);
            let l = js_hypot(b.x - a.x, b.y - a.y);
            let along = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
            // Up the slope: the eaves' inward normal (counter-clockwise: its left).
            let up = Vec2::new(-along.y, along.x);
            let s = face.slope / 100.0;
            (along, up, part.width, part.depth / js_hypot(1.0, s))
        }
    };
    let (hw, hd) = (w / 2.0, d / 2.0);
    let c = part.at;
    let corner = |sa: f64, sd: f64| {
        Vec2::new(
            c.x + along.x * sa * hw + down.x * sd * hd,
            c.y + along.y * sa * hw + down.y * sd * hd,
        )
    };
    Some(vec![
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
    ])
}

/// Whether `p` lies inside the convex counter-clockwise `ring` or on its outline.
fn inside(ring: &[Vec2], p: Vec2) -> bool {
    let n = ring.len();
    n >= 3
        && (0..n).all(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x) >= -EPS
        })
}

/// A chimney's or a vent's top over ±0.00 (docs/adr/8044 §6): a chimney's its rise over the ridge's top (the roof's
/// highest point), a vent's over the roof's highest point under its outline; none for a window.
pub fn part_top(pts: &[Vec2], spec: &RoofSpec, part: &Part) -> Option<f64> {
    match part.kind {
        PartKind::Window | PartKind::Downpipe => None,
        PartKind::Chimney => Some(spec.base + ridge_rise(pts, spec).ok()? + part.rise),
        PartKind::Vent => {
            let ring = part_ring(pts, spec, part)?;
            let high = ring
                .iter()
                .map(|q| height_at(pts, spec, *q).unwrap_or(0.0))
                .fold(f64::NEG_INFINITY, js_max);
            Some(spec.base + high + part.rise)
        }
    }
}

/// The face a window lies in: its eaves edge's index as given, the face its middle is on; none for a chimney, a vent
/// or a window on no face.
pub fn part_face(pts: &[Vec2], spec: &RoofSpec, part: &Part) -> Option<usize> {
    if part.kind != PartKind::Window {
        return None;
    }
    faces(pts, spec)
        .ok()?
        .into_iter()
        .find(|f| inside(&f.ring, part.at))
        .map(|f| f.edge)
}

/// Whether a window lies wholly in its face (not across a hip, a ridge or a valley, nor past the eaves).
pub fn window_fits(pts: &[Vec2], spec: &RoofSpec, part: &Part) -> bool {
    let Some(ring) = part_ring(pts, spec, part) else {
        return false;
    };
    faces(pts, spec)
        .ok()
        .and_then(|fs| fs.into_iter().find(|f| inside(&f.ring, part.at)))
        .is_some_and(|f| ring.iter().all(|q| inside(&f.ring, *q)))
}

/// What a line of a roof's plan is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    /// Saçak: where a face starts.
    Eaves,
    /// Kalkan: an edge without a slope.
    Gable,
    /// Mahya: two faces meeting level.
    Ridge,
    /// Kırma: two faces meeting sloping, down to a corner the eaves turn out at.
    Hip,
    /// Dere: two faces meeting sloping, down to a corner the eaves turn in at.
    Valley,
}

/// A face: the sloping edge it starts at (its index in the eaves as given), its outline counter-clockwise and its slope
/// (per cent).
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub edge: usize,
    pub ring: Vec<Vec2>,
    pub slope: f64,
}

/// A line of a roof's plan from `a` to `b`.
#[derive(Clone, Debug, PartialEq)]
pub struct RoofLine {
    pub kind: Line,
    pub a: Vec2,
    pub b: Vec2,
}

/// Why a roof's faces cannot be had.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// Fewer than three points, or a slope count other than the edges'.
    Shape,
    /// No area.
    Degenerate,
    /// The eaves turn in somewhere (docs/adr/8044 §9).
    Concave,
}

fn area2(pts: &[Vec2]) -> f64 {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (p, q) = (pts[i], pts[(i + 1) % n]);
            p.x * q.y - q.x * p.y
        })
        .sum()
}

/// The eaves counter-clockwise, each edge with its slope and its index as given.
/// Each edge of the eaves counter-clockwise: its index as given and its slope.
type Edges = Vec<(usize, Option<f64>)>;

fn ccw(pts: &[Vec2], slopes: &[Option<f64>]) -> (Vec<Vec2>, Edges) {
    let n = pts.len();
    if area2(pts) >= 0.0 {
        return (pts.to_vec(), slopes.iter().copied().enumerate().collect());
    }
    // Reversed, the edge from the k-th point is the given edge (n - 2 - k) mod n run backwards.
    let ring: Vec<Vec2> = pts.iter().rev().copied().collect();
    let edges = (0..n)
        .map(|k| {
            let i = (2 * n - 2 - k) % n;
            (i, slopes[i])
        })
        .collect();
    (ring, edges)
}

/// A sloping edge's plane over its eaves: `z = c + dx·x + dy·y`, rising from the edge's line into the roof (its left,
/// the eaves counter-clockwise) at `slope` per cent.
fn plane(a: Vec2, b: Vec2, slope: f64) -> (f64, f64, f64) {
    let len = js_hypot(b.x - a.x, b.y - a.y);
    let n = Vec2::new(-(b.y - a.y) / len, (b.x - a.x) / len);
    let s = slope / 100.0;
    (-s * (a.x * n.x + a.y * n.y), s * n.x, s * n.y)
}

fn at(p: (f64, f64, f64), q: Vec2) -> f64 {
    p.0 + p.1 * q.x + p.2 * q.y
}

/// The part of the convex `ring` where `f` is not below zero.
fn clip(ring: &[Vec2], f: impl Fn(Vec2) -> f64) -> Vec<Vec2> {
    let n = ring.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let (fp, fq) = (f(p), f(q));
        if fp >= -EPS {
            out.push(p);
        }
        if (fp > EPS && fq < -EPS) || (fp < -EPS && fq > EPS) {
            let t = fp / (fp - fq);
            out.push(Vec2::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t));
        }
    }
    let mut clean: Vec<Vec2> = Vec::new();
    for p in out {
        if clean
            .last()
            .is_none_or(|l| js_hypot(p.x - l.x, p.y - l.y) > EPS)
        {
            clean.push(p);
        }
    }
    while clean.len() > 1 {
        let (f0, l) = (clean[0], clean[clean.len() - 1]);
        if js_hypot(f0.x - l.x, f0.y - l.y) <= EPS {
            clean.pop();
        } else {
            break;
        }
    }
    clean
}

/// The eaves counter-clockwise with their edges, checked: at least three points, a slope for each edge, an area, no
/// corner turning in.
fn checked(pts: &[Vec2], spec: &RoofSpec) -> Result<(Vec<Vec2>, Edges), Problem> {
    if pts.len() < 3 || spec.slopes.len() != pts.len() {
        return Err(Problem::Shape);
    }
    if area2(pts).abs() / 2.0 <= EPS {
        return Err(Problem::Degenerate);
    }
    let (ring, edges) = ccw(pts, &spec.slopes);
    let n = ring.len();
    for i in 0..n {
        let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross
            < -EPS
                * js_max(
                    1.0,
                    js_hypot(b.x - a.x, b.y - a.y) * js_hypot(c.x - b.x, c.y - b.y),
                )
        {
            return Err(Problem::Concave);
        }
    }
    Ok((ring, edges))
}

/// Its faces, one for each sloping edge whose plane is the lowest somewhere, in the edges' order as given; none for a
/// flat roof (no edge sloping).
pub fn faces(pts: &[Vec2], spec: &RoofSpec) -> Result<Vec<Face>, Problem> {
    let (ring, edges) = checked(pts, spec)?;
    let n = ring.len();
    let planes: Vec<(usize, f64, (f64, f64, f64))> = (0..n)
        .filter_map(|k| {
            let (i, s) = edges[k];
            s.filter(|s| *s > 0.0)
                .map(|s| (i, s, plane(ring[k], ring[(k + 1) % n], s)))
        })
        .collect();
    let mut out: Vec<Face> = Vec::new();
    for (i, s, p) in &planes {
        let mut face = ring.clone();
        for (j, _, q) in &planes {
            if j == i {
                continue;
            }
            face = clip(&face, |x| at(*q, x) - at(*p, x));
            if face.len() < 3 {
                break;
            }
        }
        if face.len() >= 3 && area2(&face) / 2.0 > EPS {
            out.push(Face {
                edge: *i,
                ring: face,
                slope: *s,
            });
        }
    }
    out.sort_by_key(|f| f.edge);
    Ok(out)
}

/// A face's plane: `(c, dx, dy)`, its top `c + dx·x + dy·y` metres over ±0.00.
pub type Plane = (f64, f64, f64);

/// Each face with its plane (docs/adr/8044 §5), the eaves at `spec.base`, in the faces' order.
pub fn face_planes(pts: &[Vec2], spec: &RoofSpec) -> Result<Vec<(Face, Plane)>, Problem> {
    let (ring, edges) = checked(pts, spec)?;
    let n = ring.len();
    Ok(faces(pts, spec)?
        .into_iter()
        .filter_map(|f| {
            let k = edges.iter().position(|e| e.0 == f.edge)?;
            let (c, dx, dy) = plane(ring[k], ring[(k + 1) % n], f.slope);
            Some((f, (c + spec.base, dx, dy)))
        })
        .collect())
}

/// The roof's height over its eaves at `p`: the lowest sloping plane's there; 0 for a flat roof.
pub fn height_at(pts: &[Vec2], spec: &RoofSpec, p: Vec2) -> Result<f64, Problem> {
    let (ring, edges) = checked(pts, spec)?;
    let n = ring.len();
    Ok((0..n)
        .filter_map(|k| {
            edges[k]
                .1
                .filter(|s| *s > 0.0)
                .map(|s| at(plane(ring[k], ring[(k + 1) % n], s), p))
        })
        .fold(None, |m: Option<f64>, z| {
            Some(m.map_or(z, |m| js_min(m, z)))
        })
        .unwrap_or(0.0))
}

/// Its highest point over its eaves (its ridge's, a hip roof's apex), metres; 0 for a flat roof.
pub fn ridge_rise(pts: &[Vec2], spec: &RoofSpec) -> Result<f64, Problem> {
    let faces = faces(pts, spec)?;
    let mut top = 0.0_f64;
    for f in &faces {
        for q in &f.ring {
            top = js_max(top, height_at(pts, spec, *q)?);
        }
    }
    Ok(top)
}

fn same(a: Vec2, b: Vec2) -> bool {
    js_hypot(a.x - b.x, a.y - b.y) <= 1e-7
}

/// Whether `p` lies on an edge of the ring.
fn on_ring(ring: &[Vec2], p: Vec2) -> bool {
    let n = ring.len();
    (0..n).any(|i| {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len2 = dx * dx + dy * dy;
        if len2 <= EPS {
            return false;
        }
        let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2;
        if !(-1e-9..=1.0 + 1e-9).contains(&t) {
            return false;
        }
        let (qx, qy) = (a.x + dx * t, a.y + dy * t);
        js_hypot(p.x - qx, p.y - qy) <= 1e-7
    })
}

/// Its plan's lines: the eaves and the gable ends edge by edge in the order given, then where faces meet (ridges, hips
/// and valleys), each once, from its higher end.
pub fn lines(pts: &[Vec2], spec: &RoofSpec) -> Result<Vec<RoofLine>, Problem> {
    let (ring, _) = checked(pts, spec)?;
    let n = pts.len();
    let mut out: Vec<RoofLine> = (0..n)
        .map(|i| RoofLine {
            kind: if spec.slopes[i].is_some_and(|s| s > 0.0) {
                Line::Eaves
            } else {
                Line::Gable
            },
            a: pts[i],
            b: pts[(i + 1) % n],
        })
        .collect();
    let faces = faces(pts, spec)?;
    let z = |p: Vec2| height_at(pts, spec, p).unwrap_or(0.0);
    let mut inner: Vec<RoofLine> = Vec::new();
    for f in &faces {
        let m = f.ring.len();
        for k in 0..m {
            let (a, b) = (f.ring[k], f.ring[(k + 1) % m]);
            if js_hypot(b.x - a.x, b.y - a.y) <= 1e-7 {
                continue;
            }
            let mid = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            if on_ring(&ring, mid) {
                continue;
            }
            if inner
                .iter()
                .any(|l| (same(l.a, a) && same(l.b, b)) || (same(l.a, b) && same(l.b, a)))
            {
                continue;
            }
            let (za, zb) = (z(a), z(b));
            let (hi, lo) = if za >= zb { (a, b) } else { (b, a) };
            // Down to a corner of the eaves: a hip where they turn out, a valley where they turn in (none on a convex
            // line); level, or sloping between two of the roof's inner points: a ridge.
            let n = ring.len();
            let turn = (0..n).find(|k| same(ring[*k], lo)).map(|k| {
                let (p, c, q) = (ring[(k + n - 1) % n], ring[k], ring[(k + 1) % n]);
                (c.x - p.x) * (q.y - c.y) - (c.y - p.y) * (q.x - c.x)
            });
            let kind = match turn {
                _ if (za - zb).abs() <= 1e-7 => Line::Ridge,
                Some(t) if t < 0.0 => Line::Valley,
                Some(_) => Line::Hip,
                None => Line::Ridge,
            };
            inner.push(RoofLine { kind, a: hi, b: lo });
        }
    }
    out.extend(inner);
    Ok(out)
}

/// Its eaves as one shape: a closed outline.
pub fn outline(pts: &[Vec2]) -> Shape {
    Shape::Polygon {
        pts: pts.to_vec(),
        bulges: None,
        holes: None,
        parts: None,
    }
}

fn piece(shape: Shape) -> Piece {
    Piece {
        shape,
        color: None,
        line_weight: None,
        by_layer: Vec::new(),
        gates: Vec::new(),
        stroke: PieceStroke::default(),
        attribute: None,
        annotation_origin: None,
    }
}

fn segment(a: Vec2, b: Vec2) -> Shape {
    Shape::Line { a, b }
}

/// A slope as a plan writes it: "%33", "%32,5".
pub fn slope_words(s: f64) -> String {
    let r = crate::jsmath::js_round(s * 10.0) / 10.0;
    if r.fract() == 0.0 {
        format!("%{r:.0}")
    } else {
        format!("%{r:.1}").replace('.', ",")
    }
}

/// A height as a level mark writes it (EK-4 4.27): "+8.45", "±0.00".
pub fn level_words(z: f64) -> String {
    let r = crate::jsmath::js_round(z * 100.0) / 100.0;
    if r == 0.0 {
        "±0.00".into()
    } else {
        format!("{r:+.2}")
    }
}

fn text(p: Vec2, words: String, height: f64, rotation: f64, align: TextAlign) -> Shape {
    Shape::Text {
        p,
        text: words,
        height,
        rotation,
        align: Some(align),
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

/// Its pieces at 1:`scale`: its eaves and gable ends, its ridges, hips and valleys narrow; an arrow down each face
/// from its middle with its slope beside it; its ridge's height at the highest point of its highest line. A roof that
/// cannot be had draws its eaves alone.
pub(crate) fn pieces_at(pts: &[Vec2], spec: &RoofSpec, scale: f64, _level: Level) -> Vec<Piece> {
    if spec.overhead {
        let mut p = piece(outline(pts));
        p.stroke = PieceStroke {
            line_type: Some("dashed".to_owned()),
            weight_scale: Some(crate::geom::slab::OVERHEAD_PEN),
            ..Default::default()
        };
        return vec![p];
    }
    let Ok(lines) = lines(pts, spec) else {
        return vec![piece(Shape::Polyline {
            pts: pts.to_vec(),
            bulges: None,
            holes: None,
            parts: None,
            closed: true,
            widths: None,
        })];
    };
    let mut out = Vec::new();
    for l in &lines {
        let mut p = piece(segment(l.a, l.b));
        if matches!(l.kind, Line::Ridge | Line::Hip | Line::Valley) {
            p.stroke.weight_scale = Some(NARROW);
        }
        out.push(p);
    }
    let h = LABEL_MM * scale / 1000.0;
    let gap = WORDS_GAP_MM * scale / 1000.0;
    let len = ARROW_MM * scale / 1000.0;
    let head = HEAD_MM * scale / 1000.0;
    for f in faces(pts, spec).unwrap_or_default() {
        let m = f.ring.len() as f64;
        let c = Vec2::new(
            f.ring.iter().map(|q| q.x).sum::<f64>() / m,
            f.ring.iter().map(|q| q.y).sum::<f64>() / m,
        );
        // Down the face: from its edge's inward normal, backwards.
        let n = pts.len();
        let (a, b) = (pts[f.edge], pts[(f.edge + 1) % n]);
        let l = js_hypot(b.x - a.x, b.y - a.y);
        let inward = if area2(pts) >= 0.0 {
            Vec2::new(-(b.y - a.y) / l, (b.x - a.x) / l)
        } else {
            Vec2::new((b.y - a.y) / l, -(b.x - a.x) / l)
        };
        let down = Vec2::new(-inward.x, -inward.y);
        let tail = Vec2::new(c.x - down.x * len / 2.0, c.y - down.y * len / 2.0);
        let tip = Vec2::new(c.x + down.x * len / 2.0, c.y + down.y * len / 2.0);
        out.push(piece(segment(tail, tip)));
        for w in [std::f64::consts::FRAC_PI_6, -std::f64::consts::FRAC_PI_6] {
            let (sin, cos) = (crate::jsmath::sin(w), crate::jsmath::cos(w));
            let back = Vec2::new(-down.x * cos + down.y * sin, -down.x * sin - down.y * cos);
            out.push(piece(segment(
                tip,
                Vec2::new(tip.x + back.x * head, tip.y + back.y * head),
            )));
        }
        // Its slope beside the arrow, along it, read from below or from the right.
        let mut turn = crate::jsmath::atan2(down.y, down.x);
        if turn > std::f64::consts::FRAC_PI_2 + 1e-9 {
            turn -= std::f64::consts::PI;
        } else if turn <= -std::f64::consts::FRAC_PI_2 + 1e-9 {
            turn += std::f64::consts::PI;
        }
        let up = Vec2::new(-crate::jsmath::sin(turn), crate::jsmath::cos(turn));
        out.push(piece(text(
            Vec2::new(c.x + up.x * gap, c.y + up.y * gap),
            slope_words(f.slope),
            h,
            turn.to_degrees(),
            TextAlign::BottomCenter,
        )));
    }
    // The ridge's height (mahya kotu) at the middle of the highest level line, else at the highest point.
    let z = |p: Vec2| height_at(pts, spec, p).unwrap_or(0.0);
    let top = lines
        .iter()
        .filter(|l| l.kind == Line::Ridge)
        .max_by(|a, b| z(a.a).total_cmp(&z(b.a)))
        .map(|l| Vec2::new((l.a.x + l.b.x) / 2.0, (l.a.y + l.b.y) / 2.0))
        .or_else(|| {
            lines
                .iter()
                .filter(|l| matches!(l.kind, Line::Hip | Line::Valley))
                .map(|l| l.a)
                .max_by(|a, b| z(*a).total_cmp(&z(*b)))
        });
    if let Some(p) = top {
        out.push(piece(text(
            Vec2::new(p.x, p.y + gap),
            level_words(spec.base + z(p)),
            h,
            0.0,
            TextAlign::BottomCenter,
        )));
    }
    out.extend(edge_pieces(pts, spec, scale, h));
    out.extend(part_pieces(pts, spec, h));
    out
}

/// Lines `d` out of every sloping eaves edge (in from it when negative): mitred where two sloping edges meet, squared
/// off where a gable end meets one (it has no gutter and no snow guard: the water and the snow come off the eaves).
/// Each with its edge's index.
pub fn along_eaves(pts: &[Vec2], spec: &RoofSpec, d: f64) -> Vec<(usize, [Vec2; 2])> {
    let n = pts.len();
    if n < 3 || spec.slopes.len() != n {
        return Vec::new();
    }
    let ccw = area2(pts) >= 0.0;
    // Each edge's outward normal.
    let normal = |i: usize| {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let l = js_max(js_hypot(b.x - a.x, b.y - a.y), EPS);
        if ccw {
            Vec2::new((b.y - a.y) / l, -(b.x - a.x) / l)
        } else {
            Vec2::new(-(b.y - a.y) / l, (b.x - a.x) / l)
        }
    };
    let moved = |i: usize, p: Vec2| {
        let m = normal(i);
        Vec2::new(p.x + m.x * d, p.y + m.y * d)
    };
    // Where edge `i`'s line moved meets edge `j`'s, `j` following it; its own end when they run in one line.
    let meet = |i: usize, j: usize| {
        let (a, b) = (moved(i, pts[i]), moved(i, pts[(i + 1) % n]));
        let (c, e) = (moved(j, pts[j]), moved(j, pts[(j + 1) % n]));
        let (r, s) = (
            Vec2::new(b.x - a.x, b.y - a.y),
            Vec2::new(e.x - c.x, e.y - c.y),
        );
        let den = r.x * s.y - r.y * s.x;
        if den.abs() <= EPS * js_hypot(r.x, r.y) * js_hypot(s.x, s.y) {
            return b;
        }
        let t = ((c.x - a.x) * s.y - (c.y - a.y) * s.x) / den;
        Vec2::new(a.x + r.x * t, a.y + r.y * t)
    };
    let sloping = |i: usize| spec.slopes[i].is_some();
    (0..n)
        .filter(|&i| sloping(i))
        .map(|i| {
            let (prev, next) = ((i + n - 1) % n, (i + 1) % n);
            let start = if sloping(prev) {
                meet(prev, i)
            } else {
                moved(i, pts[i])
            };
            let end = if sloping(next) {
                meet(i, next)
            } else {
                moved(i, pts[next])
            };
            (i, [start, end])
        })
        .collect()
}

/// Its gutter and snow guard as the roof plan draws them (docs/adr/8044 §8; Mimarlık Hizmetleri Şartnamesi 14.3.3 u
/// "su toplama yerleri", Sivas İmar Yönetmeliği 42/5): the gutter's outer edge outside the eaves, the snow guard a line
/// in from them with short ticks across it, both narrow; each named once, along the longest of its lines.
fn edge_pieces(pts: &[Vec2], spec: &RoofSpec, scale: f64, h: f64) -> Vec<Piece> {
    let mut out = Vec::new();
    let tick = GUARD_TICK_MM * scale / 1000.0;
    let step = GUARD_STEP_MM * scale / 1000.0;
    let gap = WORDS_GAP_MM * scale / 1000.0;
    let narrow = |shape: Shape| {
        let mut p = piece(shape);
        p.stroke.weight_scale = Some(NARROW);
        p
    };
    // Its words along the longest line, read from below or from the right, on the side `side` (1 out, -1 in).
    let name = |lines: &[(usize, [Vec2; 2])], words: &str, side: f64| -> Option<Piece> {
        let (_, [a, b]) = lines.iter().max_by(|(_, p), (_, q)| {
            js_hypot(p[1].x - p[0].x, p[1].y - p[0].y)
                .total_cmp(&js_hypot(q[1].x - q[0].x, q[1].y - q[0].y))
        })?;
        let l = js_max(js_hypot(b.x - a.x, b.y - a.y), EPS);
        let along = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
        let ccw = area2(pts) >= 0.0;
        let outward = if ccw {
            Vec2::new(along.y, -along.x)
        } else {
            Vec2::new(-along.y, along.x)
        };
        let mut turn = crate::jsmath::atan2(along.y, along.x);
        if turn > std::f64::consts::FRAC_PI_2 + 1e-9 {
            turn -= std::f64::consts::PI;
        } else if turn <= -std::f64::consts::FRAC_PI_2 + 1e-9 {
            turn += std::f64::consts::PI;
        }
        let up = Vec2::new(-crate::jsmath::sin(turn), crate::jsmath::cos(turn));
        // On the line's own side, a gap off it: over it when that side is up, under it when down.
        let toward = Vec2::new(outward.x * side, outward.y * side);
        let above = up.x * toward.x + up.y * toward.y >= 0.0;
        let mid = Vec2::new(
            (a.x + b.x) / 2.0 + toward.x * gap,
            (a.y + b.y) / 2.0 + toward.y * gap,
        );
        Some(piece(text(
            mid,
            words.to_owned(),
            h,
            turn.to_degrees(),
            if above {
                TextAlign::BottomCenter
            } else {
                TextAlign::TopCenter
            },
        )))
    };
    if let Some(g) = spec.gutter.filter(|g| *g > 0.0) {
        let lines = along_eaves(pts, spec, g);
        out.extend(lines.iter().map(|(_, [a, b])| narrow(segment(*a, *b))));
        out.extend(name(&lines, "Oluk", 1.0));
    }
    if let Some(d) = spec.snow_guard {
        let lines = along_eaves(pts, spec, -d);
        for (_, [a, b]) in &lines {
            out.push(narrow(segment(*a, *b)));
            let l = js_hypot(b.x - a.x, b.y - a.y);
            if l <= EPS || step <= EPS {
                continue;
            }
            let along = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
            let across = Vec2::new(-along.y * tick / 2.0, along.x * tick / 2.0);
            let count = (l / step).floor() as usize;
            let first = (l - count as f64 * step) / 2.0;
            for k in 0..=count {
                let t = first + k as f64 * step;
                let c = Vec2::new(a.x + along.x * t, a.y + along.y * t);
                out.push(narrow(segment(
                    Vec2::new(c.x - across.x, c.y - across.y),
                    Vec2::new(c.x + across.x, c.y + across.y),
                )));
            }
        }
        out.extend(name(&lines, "Kar siperliği", -1.0));
    }
    out
}

/// Its parts as the roof plan draws them (docs/adr/8044 §6; Mimarlar Odası 14.3.3 u: "bacalar ve çatıya çıkış delikleri
/// gösterilir", 14.3.4 g: "bacalar kotlandırılır"): a chimney its outline and both diagonals, a vent its outline and one,
/// each with its top's level beside it; a window its outline and its frame 5 cm in.
fn part_pieces(pts: &[Vec2], spec: &RoofSpec, h: f64) -> Vec<Piece> {
    let mut out = Vec::new();
    for part in &spec.parts {
        // A downpipe at its real size, its diameter written beside it in millimetres (Mimarlık Hizmetleri Şartnamesi
        // 14.5.3 x: "Yağmur iniş boruları gerçek boyutlarında çizilir ve ölçüleri yazılır").
        if part.kind == PartKind::Downpipe {
            let r = part.width / 2.0;
            out.push(piece(Shape::Circle { c: part.at, r }));
            out.push(piece(text(
                Vec2::new(part.at.x + r + h * 0.5, part.at.y),
                format!("Ø{}", crate::jsmath::js_round(part.width * 1000.0)),
                h,
                0.0,
                TextAlign::MiddleLeft,
            )));
            continue;
        }
        let Some(ring) = part_ring(pts, spec, part) else {
            continue;
        };
        out.push(piece(outline(&ring)));
        match part.kind {
            PartKind::Chimney => {
                out.push(piece(segment(ring[0], ring[2])));
                out.push(piece(segment(ring[1], ring[3])));
            }
            PartKind::Vent => out.push(piece(segment(ring[0], ring[2]))),
            PartKind::Downpipe => {}
            PartKind::Window => {
                let frame = 0.05;
                // The frame: each corner moved 5 cm in along both of the window's own sides.
                let along = Vec2::new(ring[1].x - ring[0].x, ring[1].y - ring[0].y);
                let down = Vec2::new(ring[3].x - ring[0].x, ring[3].y - ring[0].y);
                let (la, ld) = (js_hypot(along.x, along.y), js_hypot(down.x, down.y));
                if la > 2.0 * frame && ld > 2.0 * frame {
                    let (ua, ud) = (
                        Vec2::new(along.x / la * frame, along.y / la * frame),
                        Vec2::new(down.x / ld * frame, down.y / ld * frame),
                    );
                    let inner = vec![
                        Vec2::new(ring[0].x + ua.x + ud.x, ring[0].y + ua.y + ud.y),
                        Vec2::new(ring[1].x - ua.x + ud.x, ring[1].y - ua.y + ud.y),
                        Vec2::new(ring[2].x - ua.x - ud.x, ring[2].y - ua.y - ud.y),
                        Vec2::new(ring[3].x + ua.x - ud.x, ring[3].y + ua.y - ud.y),
                    ];
                    out.push(piece(outline(&inner)));
                }
            }
        }
        if let Some(z) = part_top(pts, spec, part) {
            let right = ring.iter().map(|q| q.x).fold(f64::NEG_INFINITY, js_max);
            let mid = ring.iter().map(|q| q.y).sum::<f64>() / ring.len() as f64;
            out.push(piece(text(
                Vec2::new(right + h * 0.5, mid),
                level_words(z),
                h,
                0.0,
                TextAlign::MiddleLeft,
            )));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: f64, d: f64) -> Vec<Vec2> {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(w, 0.0),
            Vec2::new(w, d),
            Vec2::new(0.0, d),
        ]
    }

    fn spec(slopes: [Option<f64>; 4]) -> RoofSpec {
        RoofSpec {
            slopes: slopes.to_vec(),
            base: 6.0,
            ..Default::default()
        }
    }

    #[test]
    fn a_downpipe_is_drawn_at_its_size_with_its_diameter() {
        let s = RoofSpec {
            gutter: Some(0.15),
            parts: vec![Part {
                kind: PartKind::Downpipe,
                at: Vec2::new(6.0, -0.075),
                width: 0.1,
                depth: 0.1,
                ..Default::default()
            }],
            ..spec([Some(33.0); 4])
        };
        let p = pieces_at(&rect(12.0, 9.0), &s, 100.0, Level::Whole);
        assert!(
            p.iter().any(|q| matches!(&q.shape, Shape::Circle { c, r }
                if (c.x - 6.0).abs() < 1e-12 && (c.y + 0.075).abs() < 1e-12 && (r - 0.05).abs() < 1e-12)),
            "its circle"
        );
        assert!(
            p.iter().any(|q| matches!(&q.shape, Shape::Text { text, .. } if text == "Ø100")),
            "its diameter written"
        );
        // No ring on the roof, no top: no prism in a section.
        assert_eq!(part_ring(&rect(12.0, 9.0), &s, &s.parts[0]), None);
        assert_eq!(part_top(&rect(12.0, 9.0), &s, &s.parts[0]), None);
    }

    #[test]
    fn a_gutter_and_a_snow_guard_run_along_the_sloping_eaves() {
        let pts = rect(12.0, 9.0);
        let close = |p: Vec2, x: f64, y: f64| (p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9;
        // A hip roof: four gutter lines 15 cm out, mitred into a rectangle.
        let hip = spec([Some(33.0); 4]);
        let g = along_eaves(&pts, &hip, 0.15);
        assert_eq!(g.len(), 4);
        assert!(
            close(g[0].1[0], -0.15, -0.15) && close(g[0].1[1], 12.15, -0.15),
            "{g:?}"
        );
        assert!(
            close(g[2].1[0], 12.15, 9.15) && close(g[2].1[1], -0.15, 9.15),
            "{g:?}"
        );
        // A gable roof: the south and the north eaves only, squared off at the gable ends.
        let gable = spec([Some(40.0), None, Some(40.0), None]);
        let g = along_eaves(&pts, &gable, 0.15);
        assert_eq!(g.iter().map(|(i, _)| *i).collect::<Vec<_>>(), [0, 2]);
        assert!(
            close(g[0].1[0], 0.0, -0.15) && close(g[0].1[1], 12.0, -0.15),
            "{g:?}"
        );
        // A snow guard 30 cm in.
        let s = along_eaves(&pts, &hip, -0.3);
        assert!(
            close(s[0].1[0], 0.3, 0.3) && close(s[0].1[1], 11.7, 0.3),
            "{s:?}"
        );
        // Drawn and named once each; the gable's gable ends have neither.
        let words = |s: &RoofSpec| -> Vec<String> {
            pieces_at(&pts, s, 100.0, Level::Whole)
                .into_iter()
                .filter_map(|p| match p.shape {
                    Shape::Text { text, .. } => Some(text),
                    _ => None,
                })
                .collect()
        };
        let with = RoofSpec {
            gutter: Some(0.15),
            snow_guard: Some(0.3),
            ..hip.clone()
        };
        let w = words(&with);
        assert_eq!(w.iter().filter(|t| *t == "Oluk").count(), 1, "{w:?}");
        assert_eq!(
            w.iter().filter(|t| *t == "Kar siperliği").count(),
            1,
            "{w:?}"
        );
        assert!(
            !words(&hip)
                .iter()
                .any(|t| t == "Oluk" || t == "Kar siperliği")
        );
        // On its own storey's plan only the dashed eaves.
        let mut own = with.clone();
        own.overhead = true;
        assert_eq!(pieces_at(&pts, &own, 100.0, Level::Whole).len(), 1);
    }

    fn kinds(lines: &[RoofLine]) -> Vec<Line> {
        lines.iter().map(|l| l.kind).collect()
    }

    #[test]
    fn a_hip_roof_on_a_rectangle_has_four_faces_a_ridge_and_four_hips() {
        // 12 × 9 m at %33 all round: the ridge 4.5 m in from the long sides, 1.485 m up, 3 m long.
        let pts = rect(12.0, 9.0);
        let s = spec([Some(33.0); 4]);
        let f = faces(&pts, &s).unwrap();
        assert_eq!(f.iter().map(|f| f.edge).collect::<Vec<_>>(), [0, 1, 2, 3]);
        let l = lines(&pts, &s).unwrap();
        let mut inner = kinds(&l[4..]);
        inner.sort_by_key(|k| *k as u8);
        assert_eq!(
            inner,
            [Line::Ridge, Line::Hip, Line::Hip, Line::Hip, Line::Hip]
        );
        let ridge = l.iter().find(|l| l.kind == Line::Ridge).unwrap();
        let (x0, x1) = (js_min(ridge.a.x, ridge.b.x), js_max(ridge.a.x, ridge.b.x));
        assert!(
            (x0 - 4.5).abs() < 1e-9 && (x1 - 7.5).abs() < 1e-9 && (ridge.a.y - 4.5).abs() < 1e-9,
            "{ridge:?}"
        );
        assert!((ridge_rise(&pts, &s).unwrap() - 1.485).abs() < 1e-9);
    }

    #[test]
    fn a_gable_roof_runs_its_ridge_from_gable_to_gable() {
        let pts = rect(12.0, 9.0);
        let s = spec([Some(40.0), None, Some(40.0), None]);
        let l = lines(&pts, &s).unwrap();
        assert_eq!(
            kinds(&l),
            [
                Line::Eaves,
                Line::Gable,
                Line::Eaves,
                Line::Gable,
                Line::Ridge
            ]
        );
        let r = &l[4];
        assert!(
            (r.a.y - 4.5).abs() < 1e-9
                && (r.b.y - 4.5).abs() < 1e-9
                && (r.a.x - r.b.x).abs() > 11.999
        );
        assert!((ridge_rise(&pts, &s).unwrap() - 1.8).abs() < 1e-9);
    }

    #[test]
    fn a_shed_rises_from_its_one_eaves_to_the_far_side() {
        let pts = rect(12.0, 9.0);
        let s = spec([Some(10.0), None, None, None]);
        assert_eq!(faces(&pts, &s).unwrap().len(), 1);
        assert!((ridge_rise(&pts, &s).unwrap() - 0.9).abs() < 1e-9);
        assert_eq!(
            kinds(&lines(&pts, &s).unwrap()),
            [Line::Eaves, Line::Gable, Line::Gable, Line::Gable]
        );
    }

    #[test]
    fn a_flat_roof_has_no_face_and_clockwise_eaves_keep_their_slopes() {
        let pts = rect(12.0, 9.0);
        assert!(faces(&pts, &spec([None; 4])).unwrap().is_empty());
        // The same gable roof given clockwise: the south edge (now from (12,0) to (0,0)) still slopes.
        let cw = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 9.0),
            Vec2::new(12.0, 9.0),
            Vec2::new(12.0, 0.0),
        ];
        let s = spec([None, Some(40.0), None, Some(40.0)]);
        let f = faces(&cw, &s).unwrap();
        assert_eq!(f.iter().map(|f| f.edge).collect::<Vec<_>>(), [1, 3]);
        assert!((ridge_rise(&cw, &s).unwrap() - 1.8).abs() < 1e-9);
    }

    #[test]
    fn each_face_has_its_plane_over_the_eaves() {
        let s = spec([Some(33.0); 4]);
        let planes = face_planes(&rect(12.0, 9.0), &s).unwrap();
        assert_eq!(planes.len(), 4);
        // The south face (edge 0, along y = 0) rises north at 33 % from the eaves at +6.00.
        let (f, (c, dx, dy)) = &planes[0];
        assert_eq!(f.edge, 0);
        assert!((c - 6.0).abs() < 1e-12 && dx.abs() < 1e-12 && (dy - 0.33).abs() < 1e-12);
        // Each face's plane gives the roof's height over its own ring.
        for (f, (c, dx, dy)) in &planes {
            for q in &f.ring {
                let z = c + dx * q.x + dy * q.y;
                assert!((z - s.base - height_at(&rect(12.0, 9.0), &s, *q).unwrap()).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn a_roofs_parts_stand_where_they_are_given() {
        // 12 × 9 m at 33 %, its eaves at +6.00: its ridge 1.485 m over them.
        let pts = rect(12.0, 9.0);
        let s = spec([Some(33.0); 4]);
        let chimney = Part {
            kind: PartKind::Chimney,
            at: Vec2::new(9.0, 6.0),
            width: 0.4,
            depth: 0.4,
            rotation: None,
            rise: 0.8,
        };
        let ring = part_ring(&pts, &s, &chimney).unwrap();
        assert!((ring[0].x - 8.8).abs() < 1e-12 && (ring[0].y - 5.8).abs() < 1e-12);
        assert!((ring[2].x - 9.2).abs() < 1e-12 && (ring[2].y - 6.2).abs() < 1e-12);
        // 0.80 m over the ridge's top (BYKHY 58/2).
        let top = part_top(&pts, &s, &chimney).unwrap();
        assert!((top - (6.0 + 0.33 * 4.5 + 0.8)).abs() < 1e-9, "{top}");
        assert_eq!(part_face(&pts, &s, &chimney), None);
        // A vent: 0.60 m over the roof's highest point under it.
        let vent = Part {
            kind: PartKind::Vent,
            at: Vec2::new(3.0, 2.0),
            width: 0.6,
            depth: 0.6,
            rotation: None,
            rise: 0.6,
        };
        let high = part_ring(&pts, &s, &vent)
            .unwrap()
            .iter()
            .map(|q| height_at(&pts, &s, *q).unwrap())
            .fold(f64::NEG_INFINITY, js_max);
        assert!((part_top(&pts, &s, &vent).unwrap() - (6.0 + high + 0.6)).abs() < 1e-12);
        // A window on the south face: its width along the eaves, its 1.18 m down the slope 1.13 m in plan.
        let window = Part {
            kind: PartKind::Window,
            at: Vec2::new(6.0, 1.5),
            width: 0.78,
            depth: 1.18,
            rotation: None,
            rise: 0.0,
        };
        assert_eq!(part_face(&pts, &s, &window), Some(0));
        let ring = part_ring(&pts, &s, &window).unwrap();
        assert!((ring[1].x - ring[0].x - 0.78).abs() < 1e-12);
        let plan = 1.18 / (1.0 + 0.33_f64 * 0.33).sqrt();
        assert!(
            (ring[3].y - ring[0].y - plan).abs() < 1e-12,
            "{}",
            ring[3].y - ring[0].y
        );
        assert!(window_fits(&pts, &s, &window));
        // Near the corner it runs across the hip; past the eaves it has no face.
        let across = Part {
            at: Vec2::new(1.0, 0.7),
            ..window.clone()
        };
        assert_eq!(part_face(&pts, &s, &across), Some(0));
        assert!(!window_fits(&pts, &s, &across));
        let outside = Part {
            at: Vec2::new(6.0, -1.0),
            ..window.clone()
        };
        assert_eq!(part_face(&pts, &s, &outside), None);
        assert!(part_ring(&pts, &s, &outside).is_none());
        // A flat roof has no face for a window.
        assert_eq!(part_face(&pts, &spec([None; 4]), &window), None);
    }

    #[test]
    fn the_roof_plan_draws_its_parts_and_their_tops() {
        let mut s = spec([Some(33.0); 4]);
        let bare = pieces_at(&rect(12.0, 9.0), &s, 100.0, Level::Whole).len();
        s.parts = vec![
            Part {
                kind: PartKind::Chimney,
                at: Vec2::new(9.0, 6.0),
                width: 0.4,
                depth: 0.4,
                rotation: None,
                rise: 0.8,
            },
            Part {
                kind: PartKind::Window,
                at: Vec2::new(6.0, 1.5),
                width: 0.78,
                depth: 1.18,
                rotation: None,
                rise: 0.0,
            },
        ];
        let p = pieces_at(&rect(12.0, 9.0), &s, 100.0, Level::Whole);
        // The chimney: its outline, both diagonals and its top's level; the window: its outline and its frame.
        assert_eq!(p.len(), bare + 4 + 2);
        let words: Vec<String> = p
            .iter()
            .filter_map(|q| match &q.shape {
                Shape::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(words.iter().any(|w| w == "+8.29"), "{words:?}");
        // Over its own storey's plan: the eaves alone, no parts.
        s.overhead = true;
        assert_eq!(
            pieces_at(&rect(12.0, 9.0), &s, 100.0, Level::Whole).len(),
            1
        );
    }

    #[test]
    fn over_its_own_storeys_plan_a_roof_is_its_eaves_dashed() {
        let mut s = spec([Some(33.0); 4]);
        s.overhead = true;
        let p = pieces_at(&rect(12.0, 9.0), &s, 100.0, Level::Whole);
        assert_eq!(p.len(), 1, "the eaves alone: no hips, arrows or words");
        assert!(matches!(&p[0].shape, Shape::Polygon { pts, .. } if pts.len() == 4));
        assert_eq!(p[0].stroke.line_type.as_deref(), Some("dashed"));
        assert_eq!(
            p[0].stroke.weight_scale,
            Some(crate::geom::slab::OVERHEAD_PEN)
        );
    }

    #[test]
    fn an_eaves_line_turning_in_is_not_drawn_yet() {
        let l = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 4.0),
            Vec2::new(4.0, 4.0),
            Vec2::new(4.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let s = RoofSpec {
            slopes: vec![Some(33.0); 6],
            base: 0.0,
            ..Default::default()
        };
        assert_eq!(faces(&l, &s), Err(Problem::Concave));
        // Its plan draws its eaves alone.
        assert_eq!(pieces_at(&l, &s, 100.0, Level::Whole).len(), 1);
    }

    #[test]
    fn its_plan_has_an_arrow_and_a_slope_on_each_face_and_the_ridge_s_height() {
        let pts = rect(12.0, 9.0);
        let s = spec([Some(33.0); 4]);
        let pieces = pieces_at(&pts, &s, 100.0, Level::Whole);
        let words: Vec<&str> = pieces
            .iter()
            .filter_map(|p| match &p.shape {
                Shape::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(words, ["%33", "%33", "%33", "%33", "+7.49"]);
        // Four eaves, a ridge and four hips, then each face's arrow and its head's two wings.
        let lines = pieces
            .iter()
            .filter(|p| matches!(p.shape, Shape::Line { .. }))
            .count();
        assert_eq!(lines, 9 + 4 * 3);
        assert_eq!(
            pieces
                .iter()
                .filter(|p| p.stroke.weight_scale == Some(NARROW))
                .count(),
            5
        );
        assert_eq!(slope_words(32.5), "%32,5");
        assert_eq!(level_words(0.0), "±0.00");
        assert_eq!(level_words(-2.754), "-2.75");
    }
}

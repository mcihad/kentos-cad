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
            overhead: false,
        }
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
            overhead: false,
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

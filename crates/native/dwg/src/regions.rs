//! A REGION drawn by its boundary (plan/render.md RM-22, docs/adr/9227).
//!
//! A DWG keeps a region's shape as ACIS data. The DXF bridge carries that data only in its data section (ACDSDATA),
//! where KentOS's DXF reader does not look, so a region was left out ("3B katı, yüzey ve bölge nesneleri alınmaz"):
//! the SİVAS AVM DWG's 33 area outlines (gross, building and net areas) among them. A CAD draws a region by its
//! edges. Here each planar face becomes a closed LWPOLYLINE in the region's place (its handle, layer, colour, owner):
//! its outer loop; each other loop one more, marked as a hole of it with KentOS's own extended data (`hole`, as
//! KentOS's DXF writer marks a polygon's rings), so the reader makes them one area with its holes.
//!
//! Straight edges stay straight and circular arcs keep their bulge. An elliptic edge is drawn by points within a
//! thousandth of its semi-major axis and a spline edge by points along it (eight a control point); any other curve by
//! its chord. All three are counted and said.

use acadrust::entities::acis::{
    SabReader, SatCoedge, SatDocument, SatEdge, SatEllipseCurve, SatFace, SatIntCurve, SatLoop,
    SatLump, SatParser, SatPoint, SatPointer, SatRecord, SatShell, SatStraightCurve, SatTransform,
    SatVertex, Sense,
};
use acadrust::entities::{EntityType, LwPolyline, LwVertex};
use acadrust::xdata::{ExtendedDataRecord, XDataValue};
use acadrust::{CadDocument, Handle, Vector2};

/// KentOS's own extended data's application (kentos-formats' `dxf::xdata::APP`).
const KENTOS: &str = "KENTOS";
/// An elliptic edge's points: the chord's distance from the curve at most this share of its semi-major axis.
const ELLIPSE_SHARE: f64 = 1e-3;

/// What the regions became.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Regions {
    /// Regions drawn by their boundaries.
    pub taken: usize,
    /// Their rings that are holes of another.
    pub holes: usize,
    /// Edges drawn by points (an ellipse's, a spline's) and by their chord (another curve's).
    pub by_points: usize,
    pub chords: usize,
    /// Loops with no area left out.
    pub flat: usize,
    /// Regions left out, and why (Turkish, for the report).
    pub left: Vec<(&'static str, usize)>,
}

impl Regions {
    fn leave(&mut self, why: &'static str) {
        match self.left.iter_mut().find(|(w, _)| *w == why) {
            Some((_, n)) => *n += 1,
            None => self.left.push((why, 1)),
        }
    }
}

/// A closed ring: its points (x, y) with each point's bulge to the next, and its height.
#[derive(Debug, Clone, PartialEq)]
struct Ring {
    pts: Vec<(f64, f64)>,
    bulges: Vec<f64>,
    z: f64,
}

impl Ring {
    /// Twice its signed area by its points (the bulges aside: enough to tell the outer loop).
    fn area2(&self) -> f64 {
        let n = self.pts.len();
        (0..n)
            .map(|i| {
                let (a, b) = (self.pts[i], self.pts[(i + 1) % n]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum()
    }
}

/// An ACIS transform: a point's image is `scale · (x·row0 + y·row1 + z·row2) + translation`.
#[derive(Debug, Clone, Copy)]
struct Tf {
    rows: [(f64, f64, f64); 3],
    t: (f64, f64, f64),
    scale: f64,
}

impl Tf {
    /// A `transform` record's numbers: the matrix by rows (each the image of a unit vector), the translation, the
    /// scale. Read as its numbers in order, the coordinate triples opened and a leading pointer passed: SAT text holds
    /// a pointer before the thirteen numbers, SAB its triples and no pointer, and the codec's `SatTransform` reads both
    /// by SAT's slots, which turned a SAB identity into (x, y) ↦ (y, 0).
    fn of(t: &SatRecord) -> Option<Tf> {
        let mut n: Vec<f64> = Vec::with_capacity(13);
        for token in &t.tokens {
            if let Some((c, len)) = token.coordinate_components() {
                n.extend_from_slice(&c[..len]);
            } else if token.as_pointer().is_none()
                && let Some(v) = token.as_float()
            {
                n.push(v);
            }
            if n.len() >= 13 {
                break;
            }
        }
        if n.len() < 13 {
            return None;
        }
        let s = n[12];
        Some(Tf {
            rows: [(n[0], n[1], n[2]), (n[3], n[4], n[5]), (n[6], n[7], n[8])],
            t: (n[9], n[10], n[11]),
            scale: if s.is_finite() && s != 0.0 { s } else { 1.0 },
        })
    }

    fn vector(&self, (x, y, z): (f64, f64, f64)) -> (f64, f64, f64) {
        let [a, b, c] = self.rows;
        (
            self.scale * (x * a.0 + y * b.0 + z * c.0),
            self.scale * (x * a.1 + y * b.1 + z * c.1),
            self.scale * (x * a.2 + y * b.2 + z * c.2),
        )
    }

    fn point(&self, p: (f64, f64, f64)) -> (f64, f64, f64) {
        let v = self.vector(p);
        (v.0 + self.t.0, v.1 + self.t.1, v.2 + self.t.2)
    }
}

fn apply(tf: Option<Tf>, p: (f64, f64, f64)) -> (f64, f64, f64) {
    tf.map_or(p, |t| t.point(p))
}

fn turn(tf: Option<Tf>, v: (f64, f64, f64)) -> (f64, f64, f64) {
    tf.map_or(v, |t| t.vector(v))
}

fn sub(a: (f64, f64, f64), b: (f64, f64, f64)) -> (f64, f64, f64) {
    (a.0 - b.0, a.1 - b.1, a.2 - b.2)
}

fn dot(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}

fn cross(a: (f64, f64, f64), b: (f64, f64, f64)) -> (f64, f64, f64) {
    (
        a.1 * b.2 - a.2 * b.1,
        a.2 * b.0 - a.0 * b.2,
        a.0 * b.1 - a.1 * b.0,
    )
}

/// The angle in (0, 2π] from `from` to `to`, going up.
fn up(from: f64, to: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let d = (to - from).rem_euclid(tau);
    if d <= 1e-12 { tau } else { d }
}

/// A region's ACIS data read: SAT text or SAB bytes.
fn document(sat: &str, sab: &[u8]) -> Option<SatDocument> {
    if !sat.is_empty() {
        SatParser::parse(sat).ok()
    } else if !sab.is_empty() {
        SabReader::read(sab).ok()
    } else {
        None
    }
}

/// The faces' rings of an ACIS document, each face's in order; an error says why it cannot be drawn.
fn faces(doc: &SatDocument, out: &mut Regions) -> Result<Vec<Vec<Ring>>, &'static str> {
    let at = |p: SatPointer| -> Option<&SatRecord> { p.index().and_then(|i| doc.record(i)) };
    // Every walk is bounded by the records' count: a broken file's loop of pointers ends.
    let most = doc.records.len() + 1;
    let mut faces = Vec::new();
    for body in doc.bodies() {
        let tf = at(body.transform())
            .filter(|r| SatTransform::from_record(r).is_some())
            .and_then(Tf::of);
        let (mut lump, mut n) = (body.lump(), 0);
        while let Some(l) = at(lump).and_then(SatLump::from_record) {
            let mut shell = l.shell();
            while let Some(s) = at(shell).and_then(SatShell::from_record) {
                let mut face = s.face();
                while let Some(f) = at(face).and_then(SatFace::from_record) {
                    let mut rings = Vec::new();
                    let mut lp = f.first_loop();
                    while let Some(lo) = at(lp).and_then(SatLoop::from_record) {
                        // A loop with no area (two edges between the same two points) draws nothing: left out.
                        match ring(doc, lo.first_coedge(), tf, out)? {
                            Some(r) => rings.push(r),
                            None => out.flat += 1,
                        }
                        lp = lo.next_loop();
                        n += 1;
                        if n > most {
                            return Err("ACIS verisi döngüsel");
                        }
                    }
                    if !rings.is_empty() {
                        faces.push(rings);
                    }
                    face = f.next_face();
                    n += 1;
                    if n > most {
                        return Err("ACIS verisi döngüsel");
                    }
                }
                shell = s.next_shell();
            }
            lump = l.next_lump();
        }
    }
    if faces.is_empty() {
        Err("ACIS verisinde yüz yok")
    } else {
        Ok(faces)
    }
}

/// One loop as a ring, its coedges in order from `first`; none when it has no area.
fn ring(
    doc: &SatDocument,
    first: SatPointer,
    tf: Option<Tf>,
    out: &mut Regions,
) -> Result<Option<Ring>, &'static str> {
    let at = |p: SatPointer| -> Option<&SatRecord> { p.index().and_then(|i| doc.record(i)) };
    let point = |v: SatPointer| -> Option<(f64, f64, f64)> {
        let v = at(v).and_then(SatVertex::from_record)?;
        let p = at(v.point()).and_then(SatPoint::from_record)?;
        Some(apply(tf, p.position()))
    };
    let mut ring = Ring {
        pts: Vec::new(),
        bulges: Vec::new(),
        z: f64::NAN,
    };
    let mut c = first;
    for _ in 0..=doc.records.len() {
        let coedge = at(c)
            .and_then(SatCoedge::from_record)
            .ok_or("ACIS kenar bağı okunamadı")?;
        let edge = at(coedge.edge())
            .and_then(SatEdge::from_record)
            .ok_or("ACIS kenarı okunamadı")?;
        let (a, b) = (point(edge.start_vertex()), point(edge.end_vertex()));
        let (Some(a), Some(b)) = (a, b) else {
            return Err("ACIS köşesi okunamadı");
        };
        let along = coedge.sense() == Sense::Forward;
        let (from, to) = if along { (a, b) } else { (b, a) };
        if ring.z.is_nan() {
            ring.z = from.2;
        }
        let off = |z: f64| (z - ring.z).abs() > 1e-6 * (1.0 + ring.z.abs());
        if off(from.2) || off(to.2) {
            return Err("bölge yatay bir düzlemde değil");
        }
        let curve = at(edge.curve());
        // Along the curve's own direction: the edge's sense and the coedge's.
        let forward = (edge.sense() == Sense::Forward) == along;
        if curve.and_then(SatStraightCurve::from_record).is_some() {
            ring.pts.push((from.0, from.1));
            ring.bulges.push(0.0);
        } else if let Some(e) = curve.and_then(SatEllipseCurve::from_record) {
            let centre = apply(tf, e.center());
            let normal = turn(tf, e.normal());
            let major = turn(tf, e.major_axis());
            let ratio = e.ratio();
            let minor = {
                let n = (dot(normal, normal)).sqrt().max(f64::MIN_POSITIVE);
                let u = (normal.0 / n, normal.1 / n, normal.2 / n);
                let m = cross(u, major);
                (m.0 * ratio, m.1 * ratio, m.2 * ratio)
            };
            // The curve's parameter at a point: its share along the major and the minor axis.
            let param = |p: (f64, f64, f64)| {
                let d = sub(p, centre);
                (dot(d, minor) / dot(minor, minor)).atan2(dot(d, major) / dot(major, major))
            };
            let (t0, t1) = (param(from), param(to));
            let sweep = if forward { up(t0, t1) } else { -up(t1, t0) };
            if (ratio - 1.0).abs() < 1e-9 {
                // A circular arc: its bulge in the plan, the parameter turning the normal's way round.
                let plan = if normal.2 >= 0.0 { sweep } else { -sweep };
                if (plan.abs() - std::f64::consts::TAU).abs() < 1e-9 {
                    // A whole circle: four quarters, a bulge cannot say a whole turn.
                    for q in 0..4 {
                        let m = t0 + sweep * f64::from(q) / 4.0;
                        ring.pts.push((
                            centre.0 + major.0 * m.cos() + minor.0 * m.sin(),
                            centre.1 + major.1 * m.cos() + minor.1 * m.sin(),
                        ));
                        ring.bulges.push((plan / 16.0).tan());
                    }
                } else {
                    ring.pts.push((from.0, from.1));
                    ring.bulges.push((plan / 4.0).tan());
                }
            } else {
                // An elliptic arc: points, the chord within its share of the semi-major axis.
                let step = 2.0 * (1.0 - ELLIPSE_SHARE).acos();
                let n = ((sweep.abs() / step).ceil() as usize).clamp(2, 4096);
                for k in 0..n {
                    let t = t0 + sweep * k as f64 / n as f64;
                    ring.pts.push((
                        centre.0 + major.0 * t.cos() + minor.0 * t.sin(),
                        centre.1 + major.1 * t.cos() + minor.1 * t.sin(),
                    ));
                    ring.bulges.push(0.0);
                }
                out.by_points += 1;
            }
        } else if let Some(points) = curve
            .and_then(SatIntCurve::from_record)
            .and_then(|c| spline_points(doc, &c, &edge))
        {
            // A spline edge: points along it (its own span), from this coedge's start.
            let mut points: Vec<(f64, f64, f64)> =
                points.into_iter().map(|p| apply(tf, p)).collect();
            let gap =
                |p: (f64, f64, f64), q: (f64, f64, f64)| kentos_num::hypot(p.0 - q.0, p.1 - q.1);
            if gap(points[0], from) > gap(points[points.len() - 1], from) {
                points.reverse();
            }
            ring.pts.push((from.0, from.1));
            ring.bulges.push(0.0);
            for p in &points[1..points.len() - 1] {
                ring.pts.push((p.0, p.1));
                ring.bulges.push(0.0);
            }
            out.by_points += 1;
        } else {
            // Another curve: its chord.
            ring.pts.push((from.0, from.1));
            ring.bulges.push(0.0);
            out.chords += 1;
        }
        c = coedge.next();
        if c == first {
            at_least_three(&mut ring);
            return Ok((ring.pts.len() >= 3 && ring.area2().abs() > 0.0).then_some(ring));
        }
    }
    Err("ACIS verisi döngüsel")
}

/// A spline edge's points over its own span, evenly in its parameter: eight a control point, 32 to 1 024. The
/// B-spline from the record or its shared definition (`bspline_in`); an intcurve saved `reversed` runs against its
/// spline (curve parameter t is spline parameter −t). None when it cannot be read.
fn spline_points(
    doc: &SatDocument,
    curve: &SatIntCurve<'_>,
    edge: &SatEdge<'_>,
) -> Option<Vec<(f64, f64, f64)>> {
    let (degree, knots, ctrl) = curve.bspline_in(doc)?;
    if ctrl.len() < 2 || knots.len() != ctrl.len() + degree + 1 {
        return None;
    }
    let (lo, hi) = (knots[degree], knots[ctrl.len()]);
    let (t0, t1) = if curve.sense() == Sense::Reversed {
        (-edge.end_param(), -edge.start_param())
    } else {
        (edge.start_param(), edge.end_param())
    };
    let (a, b) = (t0.min(t1).max(lo), t0.max(t1).min(hi));
    // Nothing left of the span, or a parameter that is not a number.
    if b.partial_cmp(&a) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    let n = (ctrl.len() * 8).clamp(32, 1024);
    let points: Vec<(f64, f64, f64)> = (0..=n)
        .map(|k| {
            let p = de_boor(degree, &knots, &ctrl, a + (b - a) * k as f64 / n as f64);
            let w = if p[3].abs() > 1e-12 { p[3] } else { 1.0 };
            (p[0] / w, p[1] / w, p[2] / w)
        })
        .collect();
    points
        .iter()
        .all(|p| p.0.is_finite() && p.1.is_finite() && p.2.is_finite())
        .then_some(points)
}

/// A B-spline of homogeneous control points (x·w, y·w, z·w, w) at `t` (de Boor).
fn de_boor(degree: usize, knots: &[f64], ctrl: &[[f64; 4]], t: f64) -> [f64; 4] {
    let n = ctrl.len();
    let mut k = degree;
    while k + 1 < n && t >= knots[k + 1] {
        k += 1;
    }
    let mut d: Vec<[f64; 4]> = (0..=degree).map(|j| ctrl[j + k - degree]).collect();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = j + k - degree;
            let span = knots[i + degree + 1 - r] - knots[i];
            let a = if span.abs() < 1e-300 {
                0.0
            } else {
                (t - knots[i]) / span
            };
            for c in 0..4 {
                d[j][c] = (1.0 - a) * d[j - 1][c] + a * d[j][c];
            }
        }
    }
    d[degree]
}

/// A ring of fewer than three points made of arcs (a whole circle's two halves, two half circles) given more: its
/// widest arc halved at its middle until it has three. KentOS's reader takes a closed polyline of fewer points for a
/// closed line, not an area, and only an area takes holes.
fn at_least_three(ring: &mut Ring) {
    while ring.pts.len() < 3 {
        let Some(k) = (0..ring.pts.len())
            .filter(|&k| ring.bulges[k] != 0.0)
            .max_by(|&a, &b| ring.bulges[a].abs().total_cmp(&ring.bulges[b].abs()))
        else {
            return;
        };
        let (a, b) = (ring.pts[k], ring.pts[(k + 1) % ring.pts.len()]);
        let bulge = ring.bulges[k];
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        // The arc's middle: the chord's middle moved its sagitta to the right of the chord (a positive bulge turns
        // counter-clockwise, its arc lies to the right).
        let sagitta = bulge / 2.0;
        let mid = (
            (a.0 + b.0) / 2.0 + dy * sagitta,
            (a.1 + b.1) / 2.0 - dx * sagitta,
        );
        let half = (bulge.atan() / 2.0).tan();
        ring.bulges[k] = half;
        ring.pts.insert(k + 1, mid);
        ring.bulges.insert(k + 1, half);
    }
}

/// A closed LWPOLYLINE of `ring`, as the region `from` (its handle, layer, colour, owner) or a new object beside it.
fn polyline(ring: &Ring, from: &acadrust::entities::EntityCommon) -> LwPolyline {
    let mut p = LwPolyline::new();
    p.common = from.clone();
    p.is_closed = true;
    p.elevation = ring.z;
    p.vertices = ring
        .pts
        .iter()
        .zip(&ring.bulges)
        .map(|(&(x, y), &bulge)| LwVertex::with_bulge(Vector2::new(x, y), bulge))
        .collect();
    p
}

/// The document's regions drawn by their boundaries, in place (see the module).
pub fn outlines(doc: &mut CadDocument) -> Regions {
    let mut out = Regions::default();
    // Each region's faces, read first; the document is changed after.
    let mut plans: Vec<(Handle, Vec<Vec<Ring>>)> = Vec::new();
    for e in doc.entities() {
        let EntityType::Region(r) = e else {
            continue;
        };
        let Some(acis) = document(&r.acis_data.sat_data, &r.acis_data.sab_data) else {
            out.leave("ACIS verisi okunamadı");
            continue;
        };
        match faces(&acis, &mut out) {
            Ok(f) => plans.push((r.common.handle, f)),
            Err(why) => out.leave(why),
        }
    }
    if plans.is_empty() {
        return out;
    }
    // The new objects, in order: a further face's outer loop, then the holes of the face before them.
    let mut extra: Vec<(LwPolyline, Hole)> = Vec::new();
    let mut by_handle: std::collections::HashMap<Handle, Vec<Vec<Ring>>> =
        plans.into_iter().collect();
    for e in doc.entities_mut() {
        let EntityType::Region(r) = e else {
            continue;
        };
        let Some(faces) = by_handle.remove(&r.common.handle) else {
            continue;
        };
        let common = r.common.clone();
        let mut first: Option<LwPolyline> = None;
        for mut rings in faces {
            // The outer loop: the largest.
            let outer = (0..rings.len())
                .max_by(|&a, &b| rings[a].area2().abs().total_cmp(&rings[b].area2().abs()))
                .unwrap_or(0);
            let outer_ring = rings.remove(outer);
            let mut shape = polyline(&outer_ring, &common);
            let of = if first.is_none() {
                Hole::Of(common.handle)
            } else {
                Hole::OfLastNew
            };
            let holes = rings.iter().map(|ring| {
                let mut hole = polyline(ring, &common);
                hole.common.handle = Handle::NULL;
                hole.common.extended_data = Default::default();
                hole
            });
            out.holes += rings.len();
            if first.is_none() {
                first = Some(shape);
            } else {
                // A further face: a polygon of its own beside the region.
                shape.common.handle = Handle::NULL;
                shape.common.extended_data = Default::default();
                extra.push((shape, Hole::No));
            }
            extra.extend(holes.map(|h| (h, of)));
        }
        if let Some(shape) = first {
            *e = EntityType::LwPolyline(shape);
            out.taken += 1;
        }
    }
    let mut last = Handle::NULL;
    for (mut p, hole) in extra {
        let owner = match hole {
            Hole::No => None,
            Hole::Of(h) => Some(h),
            Hole::OfLastNew => Some(last),
        };
        if let Some(owner) = owner.filter(|h| !h.is_null()) {
            p.common.extended_data.add_record(hole_of(owner));
        }
        if let Ok(h) = doc.add_entity(EntityType::LwPolyline(p))
            && matches!(hole, Hole::No)
        {
            last = h;
        }
    }
    out
}

/// Whose hole a new ring is.
#[derive(Debug, Clone, Copy)]
enum Hole {
    /// None: a further face's outer loop.
    No,
    /// The region's own (its first face's outer loop keeps its handle).
    Of(Handle),
    /// The further face's outer loop added just before.
    OfLastNew,
}

/// The KENTOS item naming `owner` as the polygon a ring is a hole of.
fn hole_of(owner: Handle) -> ExtendedDataRecord {
    let mut r = ExtendedDataRecord::new(KENTOS);
    r.add_value(XDataValue::ControlString("{".into()));
    r.add_value(XDataValue::String("hole".into()));
    r.add_value(XDataValue::Handle(owner));
    r.add_value(XDataValue::ControlString("}".into()));
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use acadrust::entities::{AcisData, Region};

    /// A planar region as ACIS 7.0 SAT text (the form of the codec's own tests): one face, a 10 × 10 square from
    /// (0, 0) and, when `hole`, a circle of radius 2 round (5, 5) as its hole, the body moved by `shift`.
    fn sat(shift: [f64; 2], hole: bool) -> String {
        let square: [[f64; 2]; 4] = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let n = 4;
        let at = |i: usize, k: usize| 7 + 5 * (i % n) + k;
        let second = 7 + 5 * n;
        let mut records = vec![
            "body $-1 -1 $-1 $1 $-1 $2".to_string(),
            "lump $-1 -1 $-1 $-1 $3 $0".to_string(),
            format!(
                "transform $-1 -1 1 0 0 0 1 0 0 0 1 {} {} 0 1 no_rotate no_reflect no_shear",
                shift[0], shift[1]
            ),
            "shell $-1 -1 $-1 $-1 $-1 $4 $-1 $1".to_string(),
            "face $-1 -1 $-1 $-1 $5 $3 $-1 $6 forward single out".to_string(),
            format!(
                "loop $-1 -1 $-1 ${} ${} $4",
                if hole { second as i64 } else { -1 },
                at(0, 0)
            ),
            "plane-surface $-1 -1 $-1 0 0 0 0 0 1 1 0 0 forward_v I I I I".to_string(),
        ];
        for i in 0..n {
            let (a, b) = (square[i], square[(i + 1) % n]);
            let d = [b[0] - a[0], b[1] - a[1]];
            let length = kentos_num::hypot(d[0], d[1]);
            records.push(format!(
                "coedge $-1 -1 $-1 ${} ${} $-1 ${} forward $5 $-1",
                at(i + 1, 0),
                at(i + n - 1, 0),
                at(i, 1)
            ));
            records.push(format!(
                "edge $-1 -1 $-1 ${} 0 ${} {length} ${} ${} forward @7 unknown",
                at(i, 2),
                at(i + 1, 2),
                at(i, 0),
                at(i, 4)
            ));
            records.push(format!("vertex $-1 -1 $-1 ${} ${}", at(i, 1), at(i, 3)));
            records.push(format!("point $-1 -1 $-1 {} {} 0", a[0], a[1]));
            records.push(format!(
                "straight-curve $-1 -1 $-1 {} {} 0 {} {} 0 I I",
                a[0],
                a[1],
                d[0] / length,
                d[1] / length
            ));
        }
        if hole {
            // The hole: one coedge round a whole circle, reversed (a hole runs clockwise).
            let (l, c) = (second, second + 1);
            records.push(format!("loop $-1 -1 $-1 $-1 ${c} $4"));
            records.push(format!(
                "coedge $-1 -1 $-1 ${c} ${c} $-1 ${} reversed ${l} $-1",
                c + 1
            ));
            records.push(format!(
                "edge $-1 -1 $-1 ${} 0 ${} 6.283185307179586 ${c} ${} forward @7 unknown",
                c + 2,
                c + 2,
                c + 4
            ));
            records.push(format!("vertex $-1 -1 $-1 ${} ${}", c + 1, c + 3));
            records.push("point $-1 -1 $-1 7 5 0".to_string());
            records.push("ellipse-curve $-1 -1 $-1 5 5 0 0 0 1 2 0 0 1 I I".to_string());
        }
        let mut text = String::from(
            "700 0 1 0\n@8 acadrust @8 ACIS 7.0 @24 Thu Jan 01 00:00:00 2026\n1e-06 9.9999999999999995e-07\n",
        );
        for (i, record) in records.iter().enumerate() {
            text.push_str(&format!("-{i} {record} #\n"));
        }
        text.push_str("End-of-ACIS-data\n");
        text
    }

    fn faces_of(text: &str) -> (Vec<Vec<Ring>>, Regions) {
        let doc = SatParser::parse(text).expect("the SAT reads");
        let mut out = Regions::default();
        (faces(&doc, &mut out).expect("its faces"), out)
    }

    #[test]
    fn a_region_s_loops_are_its_rings_moved_by_its_transform() {
        let (faces, _) = faces_of(&sat([100.0, 200.0], false));
        assert_eq!(faces.len(), 1);
        let ring = &faces[0][0];
        // The transform's translation, read from its own slots (the codec's accessor starts a slot late).
        assert_eq!(
            ring.pts,
            vec![
                (100.0, 200.0),
                (110.0, 200.0),
                (110.0, 210.0),
                (100.0, 210.0)
            ]
        );
        assert!(ring.bulges.iter().all(|b| *b == 0.0));
        assert_eq!(ring.z, 0.0);
    }

    #[test]
    fn a_circle_hole_is_four_quarter_arcs_turning_its_own_way() {
        let (faces, _) = faces_of(&sat([0.0, 0.0], true));
        assert_eq!(faces[0].len(), 2, "the square and its hole");
        let hole = &faces[0][1];
        assert_eq!(hole.pts.len(), 4);
        // Reversed: clockwise, each quarter's bulge −tan(π/8).
        let q = (std::f64::consts::PI / 8.0).tan();
        assert!(
            hole.bulges.iter().all(|b| (b + q).abs() < 1e-12),
            "{:?}",
            hole.bulges
        );
        for (x, y) in &hole.pts {
            assert!(
                (kentos_num::hypot(x - 5.0, y - 5.0) - 2.0).abs() < 1e-9,
                "on the circle: ({x}, {y})"
            );
        }
        assert!(hole.area2() < 0.0, "clockwise");
    }

    #[test]
    fn fewer_than_three_points_on_arcs_are_given_a_middle() {
        // Two half circles of radius 1 round the origin: their middles at (0, −1) and (0, 1) turning counter-clockwise.
        let mut ring = Ring {
            pts: vec![(1.0, 0.0), (-1.0, 0.0)],
            bulges: vec![1.0, 1.0],
            z: 0.0,
        };
        at_least_three(&mut ring);
        // The second half (equal bulges: the last) from (−1, 0) back to (1, 0) passes (0, −1): halved there.
        assert_eq!(ring.pts.len(), 3);
        let (x, y) = ring.pts[2];
        assert!(
            x.abs() < 1e-12 && (y + 1.0).abs() < 1e-12,
            "the arc's middle: ({x}, {y})"
        );
        let q = (std::f64::consts::PI / 8.0).tan();
        assert_eq!(ring.bulges[0], 1.0, "the first half as it was");
        assert!((ring.bulges[1] - q).abs() < 1e-12 && (ring.bulges[2] - q).abs() < 1e-12);
        assert!(ring.area2() > 0.0, "counter-clockwise");
    }

    #[test]
    fn a_region_becomes_its_outer_polyline_and_a_hole_marked_as_its() {
        let mut doc = CadDocument::new();
        let mut region = Region::new();
        region.common.layer = "ALAN".into();
        region.acis_data = AcisData::from_sat(&sat([0.0, 0.0], true));
        let handle = doc.add_entity(EntityType::Region(region)).expect("added");
        let out = outlines(&mut doc);
        assert_eq!((out.taken, out.holes), (1, 1));
        let polylines: Vec<&LwPolyline> = doc
            .entities()
            .filter_map(|e| match e {
                EntityType::LwPolyline(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(polylines.len(), 2);
        assert!(
            !doc.entities().any(|e| matches!(e, EntityType::Region(_))),
            "the region is gone"
        );
        let outer = polylines
            .iter()
            .find(|p| p.common.handle == handle)
            .expect("the region's handle kept");
        assert!(outer.is_closed && outer.vertices.len() == 4 && outer.common.layer == "ALAN");
        let hole = polylines
            .iter()
            .find(|p| p.common.handle != handle)
            .expect("the hole");
        assert!(hole.is_closed && hole.vertices.len() == 4 && hole.common.layer == "ALAN");
        // KentOS's reader takes them as one area with its hole (the bridge's text, read as the import reads it).
        let dxf = acadrust::DxfWriter::new(&doc)
            .with_version(acadrust::DxfVersion::AC1032)
            .write_to_vec()
            .expect("DXF");
        let options = kentos_contracts::DxfReadOptions {
            unit: Some(kentos_contracts::DrawingUnit::M),
            ..Default::default()
        };
        let got = kentos_formats::dxf::read(&dxf, &options).expect("read");
        let polygons: Vec<_> = got
            .entities
            .iter()
            .filter_map(|e| match e {
                kentos_contracts::Entity::Polygon(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(polygons.len(), 1, "{:?}", got.entities.len());
        assert_eq!(polygons[0].holes.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            polygons[0].holes.as_ref().unwrap()[0]
                .bulges
                .as_ref()
                .map(Vec::len),
            Some(4)
        );
    }
}

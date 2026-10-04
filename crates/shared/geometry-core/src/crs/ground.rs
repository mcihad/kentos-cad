//! Plane, ellipsoid and ground (docs/adr/0171): what a path or an area
//! measures in the project's plane, on its ellipsoid and on the ground at
//! the project's mean ellipsoidal height; the projection's scale at a point
//! and along a line, and the height's factor. The points stay on the
//! project's datum: nothing is shifted to another.
//!
//! On the ellipsoid a straight segment is the geodesic between its ends and
//! an area the region its geodesics bound (Karney 2013, `geographiclib-rs`).
//! An arc is drawn in the plane: its length is the sum of the geodesics
//! between the pieces of docs/adr/0167 §2 (sagitta at most 0.1 mm), taken to
//! the arc (times its exact length over its chords' sum in the plane); its
//! area beside its chord is the plane's area per ellipsoid area integrated
//! over the region (Gauss's rules on a fan of triangles), less the sliver
//! between the plane chord and the geodesic chord the vertices' polygon
//! takes. A ring's area is its vertices' geodesic polygon's (GeographicLib's
//! `PolygonArea`) with its arcs' regions. Cutting the arcs into the 0.1 mm
//! pieces for `PolygonArea` would not do: its area is a sum of each edge's
//! area to the equator, and its rounding grows with the edges (one ulp in a
//! vertex moves a 400 m² square by up to 3·10⁻⁶ m²; a 20 m circle in 1 000
//! pieces by 3·10⁻⁴ m²). On the ground a piece is its geodesic times
//! (R + h)/R, R Euler's radius in its direction at its middle; an area its
//! ellipsoid area times (M + h)(N + h)/(MN) at its middle latitude.
//!
//! GeographicLib comes as `crates/shared/geographiclib-rs`, the crates.io
//! release with `libm`'s functions (scripts/vendor/geographiclib.py): the
//! release's own float methods parted the desktop from the web far beyond
//! the last bits; now both give the same bits (`ground-answers.json`). The
//! independent reference is `scripts/fixtures/ground_cases.py` (PROJ, and
//! GeographicLib's C library, through pyproj; mpmath; areas on PROJ's
//! equal-area azimuthal projection).

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic, PolygonArea, Winding};

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::geodesy::tm_scale;
use crate::geom::bulge::{BulgeArc, bulge_arc, bulge_at, bulge_ring_area};
use crate::jsmath::{PI, cos, js_hypot, js_max, js_min, sin};
use crate::op;
use crate::vec2::Vec2;

use super::measure::{Ring, arc_pieces};
use super::{Plane, System};

/// What a path or an area measures. A path's area values are none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundMeasures {
    /// In the project's plane: the path's length or the rings' perimeter,
    /// and the area's (outer ring less holes); none for a geographic system.
    pub plane_length: Option<f64>,
    pub plane_area: Option<f64>,
    /// On the ellipsoid.
    pub ellipsoid_length: f64,
    pub ellipsoid_area: Option<f64>,
    /// On the ground at the given height; none without one.
    pub ground_length: Option<f64>,
    pub ground_area: Option<f64>,
    /// The plane's scale against the ellipsoid: the lengths' ratio for a
    /// path, the areas' ratio's square root for an area; none without a
    /// plane or without a measure.
    pub scale: Option<f64>,
    /// The ellipsoid's against the ground: a path's R/(R + h) as its pieces
    /// give it together (their lengths' ratio), an area's
    /// √(MN/((M + h)(N + h))); none without a height.
    pub height_factor: Option<f64>,
}

/// A line's factors between grid and ground (docs/adr/0171 §3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineFactors {
    /// The geodesic's length between the line's ends (m).
    pub ellipsoid_length: f64,
    /// The projection's scale along the line, Simpson's (k₁ + 4kₘ + k₂)/6;
    /// none where the system has no one scale at a point.
    pub scale: Option<f64>,
    /// R/(R + h) at the geodesic's middle in its direction; none without a
    /// height.
    pub height_factor: Option<f64>,
}

/// Why nothing was measured: a point the system's projection does not take
/// back to the ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unreachable;

/// The project's system and its mean ellipsoidal height (m): what the survey
/// windows take measured lengths between the ground and the grid with
/// (docs/adr/0171 §4).
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub system: System,
    pub height: f64,
}

crate::json_struct!(Grid { system, height });

/// A line's factors from the ground to the grid: the projection's scale
/// along it and the height's factor; a grid length is a ground length times
/// both.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridFactor {
    pub scale: f64,
    pub height_factor: f64,
}

impl GridFactor {
    /// Ground length × this = grid length.
    pub fn combined(&self) -> f64 {
        self.scale * self.height_factor
    }
}

/// Why a system takes no lengths to its grid.
pub const NO_SCALE: &str = "Projenin sisteminde uzunluklar projeksiyona indirilemez: bu sistemde bir noktanın tek ölçeği yok (coğrafi sistem, Pseudo-Mercator ya da afinle bağlı yerel sistem).";

/// Why a line's factors could not be had.
pub const BEYOND: &str = "Projenin sistemi bu çizgiye ulaşmıyor; uzunluk projeksiyona indirilemez.";

/// The factors of the line from `a` to `b` on the grid (docs/adr/0171 §3).
pub fn grid_factor(grid: &Grid, a: Vec2, b: Vec2) -> Result<GridFactor, String> {
    let f = line_factors(&grid.system, a, b, Some(grid.height)).map_err(|_| BEYOND.to_owned())?;
    match (f.scale, f.height_factor) {
        (Some(scale), Some(height_factor)) => Ok(GridFactor {
            scale,
            height_factor,
        }),
        (None, _) => Err(NO_SCALE.to_owned()),
        (_, None) => Err(BEYOND.to_owned()),
    }
}

/// The ellipsoid's semi-major axis and flattening.
fn ellipsoid(system: &System) -> (f64, f64) {
    let (a, inverse_flattening) = system.datum().ellipsoid();
    (a, 1.0 / inverse_flattening)
}

/// The radii of curvature at a latitude (degrees): along the meridian (M)
/// and across it (N).
fn radii(a: f64, f: f64, lat: f64) -> (f64, f64) {
    let e2 = f * (2.0 - f);
    let s = libm::sin(lat * (PI / 180.0));
    let w2 = 1.0 - e2 * s * s;
    (a * (1.0 - e2) / (w2 * libm::sqrt(w2)), a / libm::sqrt(w2))
}

/// Euler's radius of curvature in a direction: MN/(N cos²α + M sin²α), α
/// the azimuth (degrees).
fn euler_radius(a: f64, f: f64, lat: f64, azimuth: f64) -> f64 {
    let (m, n) = radii(a, f, lat);
    let t = azimuth * (PI / 180.0);
    let (c, s) = (libm::cos(t), libm::sin(t));
    m * n / (n * c * c + m * s * s)
}

/// A height's factor R/(R + h); none for a height that makes no ground.
fn factor(r: f64, h: f64) -> Option<f64> {
    let k = r / (r + h);
    (k.is_finite() && k > 0.0).then_some(k)
}

/// The geodesic from `p` to `q` (latitude, longitude): its length and its
/// middle's latitude and azimuth.
fn geodesic(g: &Geodesic, p: (f64, f64), q: (f64, f64)) -> (f64, f64, f64) {
    let (s12, azi1, _azi2, _a12): (f64, f64, f64, f64) = g.inverse(p.0, p.1, q.0, q.1);
    let (lat, _lon, azi): (f64, f64, f64) = g.direct(p.0, p.1, azi1, s12 / 2.0);
    (s12, lat, azi)
}

/// The projection's scale at a point of the system's plane (docs/adr/0171
/// §3): a transverse Mercator's; a local system's on a similarity, its
/// base's over the plane's scale. None for a geographic system, for the
/// Pseudo-Mercator (on the ellipsoid its scale along a meridian is not its
/// scale along a parallel) and for an affine local plane (another in each
/// direction), and where the projection does not take the point back.
pub fn point_scale(system: &System, p: Vec2) -> Option<f64> {
    match system {
        System::Tm { .. } => {
            let (lat, lon) = system.unproject(p)?;
            tm_scale(&system.tm()?, lat, lon)
        }
        System::Local {
            base,
            plane: plane @ Plane::Similarity { scale, .. },
        } if *scale > 0.0 => Some(point_scale(base, plane.forward(p))? / scale),
        _ => None,
    }
}

/// The projection's scale along the line from `a` to `b`: Simpson's rule
/// over its ends and its middle.
pub fn line_scale(system: &System, a: Vec2, b: Vec2) -> Option<f64> {
    let m = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let (ka, km, kb) = (
        point_scale(system, a)?,
        point_scale(system, m)?,
        point_scale(system, b)?,
    );
    Some((ka + 4.0 * km + kb) / 6.0)
}

/// The line from `a` to `b`: its geodesic, its scale and, with the project's
/// mean ellipsoidal `height` (m), its height's factor.
pub fn line_factors(
    system: &System,
    a: Vec2,
    b: Vec2,
    height: Option<f64>,
) -> Result<LineFactors, Unreachable> {
    let (ea, ef) = ellipsoid(system);
    let g = Geodesic::new(ea, ef);
    let p = system.unproject(a).ok_or(Unreachable)?;
    let q = system.unproject(b).ok_or(Unreachable)?;
    let (s12, lat, azi) = geodesic(&g, p, q);
    Ok(LineFactors {
        ellipsoid_length: s12,
        scale: line_scale(system, a, b),
        height_factor: height.and_then(|h| factor(euler_radius(ea, ef, lat, azi), h)),
    })
}

/// The plane's area per area on the ellipsoid at a point: a conformal
/// grid's k²; a local system's base's over its plane's determinant; the
/// Pseudo-Mercator's R²/(MN cos²φ) (R its sphere's radius); a geographic
/// system's square degrees, 1/(MN cos φ (π/180)²).
fn areal_scale(system: &System, p: Vec2) -> Option<f64> {
    let m = match system {
        System::Tm { .. } => {
            let k = point_scale(system, p)?;
            k * k
        }
        System::Local { base, plane } => {
            let [a, b, _, d, e, _] = plane.coefficients();
            areal_scale(base, plane.forward(p))? / (a * e - b * d).abs()
        }
        System::Mercator {} => {
            let (lat, _) = system.unproject(p)?;
            let (a, f) = ellipsoid(system);
            let (m, n) = radii(a, f, lat);
            let c = cos(lat * (PI / 180.0));
            a * a / (m * n * c * c)
        }
        System::Geographic { .. } => {
            let (a, f) = ellipsoid(system);
            let (m, n) = radii(a, f, p.y);
            let rad = PI / 180.0;
            1.0 / (m * n * cos(p.y * rad) * rad * rad)
        }
    };
    (m.is_finite() && m > 0.0).then_some(m)
}

/// Whether the system's plane turns as latitude and longitude do (east
/// right of north): 1, or −1 for a local plane that mirrors its base's.
fn orientation(system: &System) -> f64 {
    match system {
        System::Local { base, plane } => {
            let [a, b, _, d, e, _] = plane.coefficients();
            let sign = if a * e - b * d < 0.0 { -1.0 } else { 1.0 };
            sign * orientation(base)
        }
        _ => 1.0,
    }
}

/// Gauss–Legendre's four nodes and weights on [0, 1].
const GAUSS: [(f64, f64); 4] = [
    (0.069_431_844_202_973_7, 0.173_927_422_568_726_9),
    (0.330_009_478_207_571_87, 0.326_072_577_431_273_05),
    (0.669_990_521_792_428_1, 0.326_072_577_431_273_05),
    (0.930_568_155_797_026_3, 0.173_927_422_568_726_9),
];

/// ∫∫ f over the triangle (p0, p1, p2), signed by its turn: Duffy's
/// collapse of the square, Gauss 4 × 4 (exact for f of degree 6).
fn triangle_integral(p0: Vec2, p1: Vec2, p2: Vec2, f: &dyn Fn(Vec2) -> Option<f64>) -> Option<f64> {
    let e1 = Vec2::new(p1.x - p0.x, p1.y - p0.y);
    let e2 = Vec2::new(p2.x - p1.x, p2.y - p1.y);
    let cross = e1.x * e2.y - e1.y * e2.x;
    if cross == 0.0 {
        return Some(0.0);
    }
    let mut sum = 0.0;
    for &(u, wu) in &GAUSS {
        for &(v, wv) in &GAUSS {
            let p = Vec2::new(p0.x + u * (e1.x + v * e2.x), p0.y + u * (e1.y + v * e2.y));
            sum += wu * wv * u * f(p)?;
        }
    }
    Some(sum * cross)
}

/// The largest turn of an arc's cuts for its area (5°).
const TURN: f64 = PI / 36.0;

/// ∫∫ f over the region between an arc's chord (from `a` to `b`) and the
/// arc, signed as the ring's area takes it ((r²/2)(θ − sin θ) for f = 1): a
/// fan of triangles from `a` over the arc cut into turns of at most 5° and
/// lengths of at most `cut`, and each cut's own sliver, (r²/2)(δ − sin δ),
/// at its centroid (two fifths of its sagitta from its chord). A sliver's
/// f taken at its centroid misses half f's curvature times the sliver's
/// spread (its chord's square over 20): the length bound keeps that below
/// 10⁻⁷ m² for an arc of any radius (a flat arc of 98 km and a 3.8 km chord
/// in one sliver missed 5·10⁻⁴ m²).
fn arc_integral(
    arc: &BulgeArc,
    a: Vec2,
    b: Vec2,
    cut: f64,
    f: &dyn Fn(Vec2) -> Option<f64>,
) -> Option<f64> {
    // At most 2¹⁶ cuts: an arc thousands of kilometres long is not measured finer.
    let turns = arc.sweep.abs() / TURN;
    let n = (libm::ceil(js_max(turns, arc.r * arc.sweep.abs() / cut)) as usize).clamp(1, 1 << 16);
    let delta = arc.sweep / n as f64;
    let point = |i: usize| match i {
        0 => a,
        _ if i == n => b,
        _ => {
            let t = arc.a0 + delta * i as f64;
            Vec2::new(arc.c.x + arc.r * cos(t), arc.c.y + arc.r * sin(t))
        }
    };
    let mut total = 0.0;
    for i in 1..n {
        total += triangle_integral(a, point(i), point(i + 1), f)?;
    }
    // δ − sin δ without its cancellation for a short cut.
    let d2 = delta * delta;
    let excess = if delta.abs() < 1e-2 {
        delta * d2 / 6.0 * (1.0 - d2 / 20.0 * (1.0 - d2 / 42.0))
    } else {
        delta - sin(delta)
    };
    let sliver = arc.r * arc.r / 2.0 * excess;
    let h = sin(delta / 4.0);
    // The cut's chord is r·cos(δ/2) from the centre, its sagitta 2r·sin²(δ/4).
    let to_centroid = arc.r * (cos(delta / 2.0) + 0.4 * 2.0 * h * h);
    for i in 0..n {
        let t = arc.a0 + delta * (i as f64 + 0.5);
        let p = Vec2::new(
            arc.c.x + to_centroid * cos(t),
            arc.c.y + to_centroid * sin(t),
        );
        total += sliver * f(p)?;
    }
    Some(total)
}

/// The plane area between the straight chord from `a` to `b` and the image
/// of the geodesic between them, signed as an arc that bulges right of the
/// chord adds to a ring: (2/3)·L·s, s the geodesic's middle's offset to the
/// chord's right (its image is a parabola's arc to the curvature's change).
fn geodesic_lens(
    system: &System,
    g: &Geodesic,
    a: Vec2,
    b: Vec2,
    p: (f64, f64),
    q: (f64, f64),
) -> Option<f64> {
    let (s12, azi1, _azi2, _a12): (f64, f64, f64, f64) = g.inverse(p.0, p.1, q.0, q.1);
    let (lat, lon, _azi): (f64, f64, f64) = g.direct(p.0, p.1, azi1, s12 / 2.0);
    let m = system.project(lat, lon)?;
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = js_hypot(dx, dy);
    if len == 0.0 {
        return Some(0.0);
    }
    let (cx, cy) = (m.x - (a.x + b.x) / 2.0, m.y - (a.y + b.y) / 2.0);
    let right = (dy * cx - dx * cy) / len;
    Some(2.0 / 3.0 * len * right)
}

/// A path (`closed` false: the first ring) or an area's rings (the first
/// the outer, the others its holes), in the project's `system`, in its
/// plane, on its ellipsoid and, with the project's mean ellipsoidal
/// `height` (m), on the ground.
pub fn ground_measures(
    system: &System,
    rings: &[Ring],
    closed: bool,
    height: Option<f64>,
) -> Result<GroundMeasures, Unreachable> {
    let (ea, ef) = ellipsoid(system);
    let g = Geodesic::new(ea, ef);
    let rings = if closed {
        rings
    } else {
        &rings[..rings.len().min(1)]
    };
    let has_plane = !matches!(system, System::Geographic { .. });
    // A height that makes a ground at all (above the ellipsoid's centre).
    let height = height.filter(|&h| factor(ea, h).is_some());
    let (mut plane_len, mut plane_ar) = (0.0, 0.0);
    let (mut ell_len, mut ell_ar, mut ground_len) = (0.0, 0.0, 0.0);
    // The outer ring's latitudes' bounds: the area's middle latitude.
    let (mut lat_min, mut lat_max) = (f64::INFINITY, f64::NEG_INFINITY);
    // The plane's area per ellipsoid area, inverted: what a plane m² is on the ellipsoid.
    let per_plane = |p: Vec2| areal_scale(system, p).map(|m| 1.0 / m);
    // An arc's longest cut for its area: 100 m, or a thousandth of a degree.
    let cut = if has_plane { 100.0 } else { 1e-3 };
    for (i, ring) in rings.iter().enumerate() {
        let pts = &ring.pts;
        let Some(&first) = pts.first() else {
            continue;
        };
        let n = pts.len();
        let bulges = ring.bulges.as_deref();
        // The vertices and the pieces' ends on the ellipsoid; each segment's
        // last is the next one's first.
        let start = system.unproject(first).ok_or(Unreachable)?;
        let mut vertices = vec![start];
        let mut geo = vec![start];
        let mut arcs = Vec::new();
        for k in 0..if closed { n } else { n - 1 } {
            let (a, b) = (pts[k], pts[(k + 1) % n]);
            let bulge = bulge_at(bulges, k);
            let (mut chords, mut s, mut ground) = (0.0, 0.0, 0.0);
            for w in arc_pieces(a, b, bulge).windows(2) {
                let p = geo[geo.len() - 1];
                let q = system.unproject(w[1]).ok_or(Unreachable)?;
                let (s12, lat, azi) = geodesic(&g, p, q);
                chords += js_hypot(w[1].x - w[0].x, w[1].y - w[0].y);
                s += s12;
                if let Some(k) = height.and_then(|h| factor(euler_radius(ea, ef, lat, azi), h)) {
                    ground += s12 / k;
                }
                geo.push(q);
            }
            vertices.push(geo[geo.len() - 1]);
            // An arc's chords taken to the arc: its exact length over theirs.
            let (exact, ratio) = match bulge_arc(a, b, bulge) {
                Some(arc) if chords > 0.0 => {
                    arcs.push((k, arc));
                    let exact = arc.r * arc.sweep.abs();
                    (exact, exact / chords)
                }
                _ => (chords, 1.0),
            };
            plane_len += exact;
            ell_len += s * ratio;
            ground_len += ground * ratio;
        }
        if !closed {
            continue;
        }
        // The closing piece ends on the ring's first point again.
        geo.pop();
        vertices.pop();
        if i == 0 {
            for &(lat, _) in &geo {
                lat_min = js_min(lat_min, lat);
                lat_max = js_max(lat_max, lat);
            }
        }
        // The vertices' geodesic polygon, turned as the plane turns.
        let mut signed = 0.0;
        if vertices.len() > 2 {
            let mut polygon = PolygonArea::new(&g, Winding::CounterClockwise);
            for &(lat, lon) in &vertices {
                polygon.add_point(lat, lon);
            }
            let (_perimeter, area, _count) = polygon.compute(true);
            signed = orientation(system) * area;
        }
        // Each arc's region beside its chord, less the sliver between the
        // plane chord and the geodesic chord the polygon took.
        for (k, arc) in arcs {
            let (a, b) = (pts[k], pts[(k + 1) % n]);
            let (p, q) = (vertices[k], vertices[(k + 1) % vertices.len()]);
            let lens = geodesic_lens(system, &g, a, b, p, q).ok_or(Unreachable)?;
            let mid = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            signed += arc_integral(&arc, a, b, cut, &per_plane).ok_or(Unreachable)?
                - lens * per_plane(mid).ok_or(Unreachable)?;
        }
        let sign = if i == 0 { 1.0 } else { -1.0 };
        plane_ar += sign * bulge_ring_area(pts, bulges).abs();
        ell_ar += sign * signed.abs();
    }
    // The area's height factor MN/((M + h)(N + h)) at its middle latitude.
    let area_factor = height
        .filter(|_| closed && lat_min <= lat_max)
        .and_then(|h| {
            let (m, n) = radii(ea, ef, (lat_min + lat_max) / 2.0);
            Some(factor(m, h)? * factor(n, h)?)
        });
    let scale = match (has_plane, closed) {
        (false, _) => None,
        (true, true) => (ell_ar > 0.0).then(|| libm::sqrt(plane_ar / ell_ar)),
        (true, false) => (ell_len > 0.0).then(|| plane_len / ell_len),
    };
    let height_factor = if closed {
        area_factor.map(libm::sqrt)
    } else {
        height
            .filter(|_| ground_len > 0.0)
            .map(|_| ell_len / ground_len)
    };
    Ok(GroundMeasures {
        plane_length: has_plane.then_some(plane_len),
        plane_area: (has_plane && closed).then_some(plane_ar),
        ellipsoid_length: ell_len,
        ellipsoid_area: closed.then_some(ell_ar),
        ground_length: height.map(|_| ground_len),
        ground_area: area_factor.map(|k| ell_ar / k),
        scale,
        height_factor,
    })
}

/// The op's answer: the measures, or why there are none.
struct Answer(Result<GroundMeasures, Unreachable>);

impl ToJson for Answer {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok(m) => {
                field(out, &mut first, "planeLength", &m.plane_length);
                field(out, &mut first, "planeArea", &m.plane_area);
                field(out, &mut first, "ellipsoidLength", &m.ellipsoid_length);
                field(out, &mut first, "ellipsoidArea", &m.ellipsoid_area);
                field(out, &mut first, "groundLength", &m.ground_length);
                field(out, &mut first, "groundArea", &m.ground_area);
                field(out, &mut first, "scale", &m.scale);
                field(out, &mut first, "heightFactor", &m.height_factor);
            }
            Err(Unreachable) => field(out, &mut first, "why", "unreachable"),
        }
        out.push('}');
    }
}

/// A line's factors, or why there are none.
struct Factors(Result<LineFactors, Unreachable>);

impl ToJson for Factors {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok(l) => {
                field(out, &mut first, "ellipsoidLength", &l.ellipsoid_length);
                field(out, &mut first, "scale", &l.scale);
                field(out, &mut first, "heightFactor", &l.height_factor);
            }
            Err(Unreachable) => field(out, &mut first, "why", "unreachable"),
        }
        out.push('}');
    }
}

pub(crate) static OPS: &[Op] = &[
    op!(
        "crsGroundMeasures",
        |system: System, rings: Vec<Ring>, closed: bool, height: Option<f64>| {
            Answer(ground_measures(&system, &rings, closed, height))
        }
    ),
    op!("crsPointScale", |system: System, p: Vec2| {
        point_scale(&system, p)
    }),
    op!(
        "crsLineFactors",
        |system: System, a: Vec2, b: Vec2, height: Option<f64>| {
            Factors(line_factors(&system, a, b, height))
        }
    ),
];

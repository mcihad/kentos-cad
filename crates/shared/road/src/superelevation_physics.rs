//! A superelevation's axis of rotation and its physics (docs/adr/0931, plan/road.md Y27; ROAD-DEVER-01, 07, 11).
//!
//! **The axis of rotation.** The traveled way turns about a line that keeps its place (AASHTO's methods of attaining
//! superelevation, Civil 3D's axis of rotation, OpenRoads' pivot methods). About the centreline the profile grade is
//! the axis's elevation as before. About an edge the traveled way's edge on that side keeps the elevation it has at the
//! normal crown, `z_profile + W·n`, so the axis rises by `W·(n − s)` (W that side's lanes' width, n its normal slope,
//! s its slope now; slopes per one). The inside edge is the side whose slope is the lower (toward the curve's centre);
//! where both slopes are equal the lift is the same either way at a normal crown, and the left is taken.
//!
//! **The relative gradient.** The rise of a traveled way's edge over the axis of rotation per metre along the road:
//! `Δ = |d(z_edge − z_axis_line)/ds|`. About the centreline the edge is `W·s` over the axis; about an edge, the other
//! edge is `W_o·s_o − W_p·s_p` over the pivot (the whole width turns). The slopes and widths are piecewise linear in
//! the station, so on each stretch between break points `d/ds (W·s)` is linear and its largest size is at an end:
//! exact. The design speed's largest relative gradient is the standard's (AASHTO's table runs 1:128 at 25 km/h to
//! 1:286 at 130 km/h; KTŞ's own); KentOS fills in none.
//!
//! **The side friction.** A vehicle on a curve of radius R at speed V is held by the superelevation e and the side
//! friction f: `V²/(gR) = (e + f)/(1 − e·f)` (the point-mass balance along and across the slope), so the friction it
//! asks for is `f = (V²/(gR) − e)/(1 + e·V²/(gR))`. The usual `f ≈ V²/(127·R) − e` (V in km/h, 127 ≈ 3.6²·9.81) drops
//! the product e·f. e is the slope toward the centre of the side that has the least of it (an adverse crown on the
//! outside is negative); g is 9.80665 m/s². The largest friction to allow is the standard's.

/// The line the traveled way turns about.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pivot {
    #[default]
    Centerline,
    LeftEdge,
    RightEdge,
    /// The edge on the curve's inside: the side with the lower slope.
    InsideEdge,
    /// The edge on the curve's outside: the side with the higher slope.
    OutsideEdge,
}

/// Standard gravity (m/s²).
pub const G: f64 = 9.80665;

/// Which side a pivot holds at slopes `left`, `right` (percent): `Some(true)` the left, `Some(false)` the right, none the
/// centreline.
pub fn pivot_side(pivot: Pivot, left: f64, right: f64) -> Option<bool> {
    match pivot {
        Pivot::Centerline => None,
        Pivot::LeftEdge => Some(true),
        Pivot::RightEdge => Some(false),
        Pivot::InsideEdge => Some(left <= right),
        Pivot::OutsideEdge => Some(left > right),
    }
}

/// How much the axis rises over the profile grade at a station (m): `W·(n − s)/100` of the held side, 0 about the
/// centreline. `slopes`, `normal` (left, right) in percent; `widths` (left, right) the traveled way's widths there.
pub fn axis_lift(pivot: Pivot, slopes: (f64, f64), normal: (f64, f64), widths: (f64, f64)) -> f64 {
    match pivot_side(pivot, slopes.0, slopes.1) {
        None => 0.0,
        Some(true) => widths.0 * (normal.0 - slopes.0) / 100.0,
        Some(false) => widths.1 * (normal.1 - slopes.1) / 100.0,
    }
}

/// A traveled way's description along the road: each side's slope (percent) and width (m), piecewise linear in the
/// station; break points (station, value) in station order, a single point a constant, none the normal.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Way {
    pub left_slope: Vec<[f64; 2]>,
    pub right_slope: Vec<[f64; 2]>,
    pub left_width: Vec<[f64; 2]>,
    pub right_width: Vec<[f64; 2]>,
}

fn along(points: &[[f64; 2]], s: f64) -> f64 {
    let (Some(first), Some(last)) = (points.first(), points.last()) else { return 0.0 };
    if s <= first[0] {
        return first[1];
    }
    if s >= last[0] {
        return last[1];
    }
    let i = points.partition_point(|p| p[0] <= s);
    let (a, b) = (points[i - 1], points[i]);
    if b[0] == a[0] { b[1] } else { a[1] + (b[1] - a[1]) * (s - a[0]) / (b[0] - a[0]) }
}

/// The derivative of a piecewise linear function on the stretch holding `(a, b)`.
fn rate(points: &[[f64; 2]], a: f64, b: f64) -> f64 {
    let m = 0.5 * (a + b);
    if points.len() < 2 || m <= points[0][0] || m >= points[points.len() - 1][0] {
        return 0.0;
    }
    let i = points.partition_point(|p| p[0] <= m);
    let (p, q) = (points[i - 1], points[i]);
    if q[0] == p[0] { 0.0 } else { (q[1] - p[1]) / (q[0] - p[0]) }
}

/// A stretch's relative gradients (per one): each side's edge over the axis of rotation, the largest size on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gradient {
    pub from: f64,
    pub to: f64,
    pub left: f64,
    pub right: f64,
}

/// The relative gradients of `way` about `pivot` from `from` to `to`: one row per stretch between the slopes' and
/// widths' break points (where neither changes, both are 0).
pub fn relative_gradients(way: &Way, pivot: Pivot, from: f64, to: f64) -> Vec<Gradient> {
    let mut stops: Vec<f64> = vec![from, to];
    for pts in [&way.left_slope, &way.right_slope, &way.left_width, &way.right_width] {
        stops.extend(pts.iter().map(|p| p[0]).filter(|s| *s > from && *s < to));
    }
    stops.sort_by(f64::total_cmp);
    stops.dedup();
    let mut out = Vec::new();
    for w in stops.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b <= a {
            continue;
        }
        // d(W·s/100)/ds = (W'·s + W·s')/100: linear on the stretch, its largest size at an end.
        let side = |slope: &[[f64; 2]], width: &[[f64; 2]], x: f64| {
            (rate(width, a, b) * along(slope, x) + along(width, x) * rate(slope, a, b)) / 100.0
        };
        let left = |x: f64| side(&way.left_slope, &way.left_width, x);
        let right = |x: f64| side(&way.right_slope, &way.right_width, x);
        // The pivot's side at the stretch's middle (it changes only where the slopes cross, a break point or between).
        let m = 0.5 * (a + b);
        let held = pivot_side(pivot, along(&way.left_slope, m), along(&way.right_slope, m));
        let edge = |x: f64, is_left: bool| {
            let own = if is_left { left(x) } else { right(x) };
            match held {
                None => own,
                Some(h) if h == is_left => 0.0,
                // The other edge over the held one: W_o·s_o − W_p·s_p (each measured outward from the axis).
                Some(h) => own - if h { left(x) } else { right(x) },
            }
        };
        let most = |is_left: bool| edge(a, is_left).abs().max(edge(b, is_left).abs());
        out.push(Gradient { from: a, to: b, left: most(true), right: most(false) });
    }
    out
}

/// The side friction a vehicle at `speed` (km/h) asks for on a curve of radius `radius` (m) with the superelevation
/// `e` toward the centre (per one): `(V²/(gR) − e)/(1 + e·V²/(gR))`.
pub fn friction(speed: f64, radius: f64, e: f64) -> f64 {
    let v = speed / 3.6;
    let a = v * v / (G * radius);
    (a - e) / (1.0 + e * a)
}

/// The superelevation toward the centre (per one) at slopes `left`, `right` (percent) on a curve turning left
/// (`left_turn`) or right: the side with the least of it; an adverse crown is negative.
pub fn toward_centre(left: f64, right: f64, left_turn: bool) -> f64 {
    // A left curve's centre is on the left: the left side falls toward it when its outward slope is negative, the right
    // side when its outward slope is positive.
    let (l, r) = if left_turn { (-left, right) } else { (left, -right) };
    l.min(r) / 100.0
}

/// A curve's side friction at a speed: where it asks for the most, the radius and superelevation there.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveFriction {
    pub pi: String,
    /// The curve's TS and ST.
    pub from: f64,
    pub to: f64,
    pub most: f64,
    pub at: f64,
    pub radius: f64,
    pub e: f64,
}

/// The side friction along each curve of `al` at `speed` (km/h) with `schedule`'s slopes: read at the curve's TS, SC,
/// CS, ST, the schedule's break points on it and every `step` metres. On the circular part R is constant and e linear
/// between break points, so f (monotone in e) is largest at one of them: exact there; on a clothoid R changes and the
/// reading is every `step` metres.
pub fn curve_friction(
    al: &crate::alignment::Alignment,
    schedule: &crate::superelevation::Schedule,
    normal: crate::superelevation::Crossfall,
    speed: f64,
    step: f64,
) -> Vec<CurveFriction> {
    let mut out = Vec::new();
    for c in &al.curves {
        let (from, to) = (c.stations[0], c.stations[3]);
        if !(to > from) {
            continue;
        }
        let mut at: Vec<f64> = c.stations.to_vec();
        for pts in [&schedule.left, &schedule.right] {
            at.extend(pts.iter().map(|p| p[0]).filter(|s| *s > from && *s < to));
        }
        let n = ((to - from) / step.max(1e-3)).ceil() as usize;
        at.extend((0..=n).map(|i| (from + i as f64 * step).min(to)));
        at.sort_by(f64::total_cmp);
        at.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        let mut best: Option<CurveFriction> = None;
        for s in at {
            let Some(p) = al.at(s) else { continue };
            // At an element's end the curvature is the element's own: read just inside the curve.
            let k = if p.curvature == 0.0 { al.at((s + 1e-6).min(to)).map_or(0.0, |q| q.curvature) } else { p.curvature };
            if k == 0.0 {
                continue;
            }
            let slopes = schedule.at(s, normal);
            let e = toward_centre(slopes.left, slopes.right, k > 0.0);
            let radius = 1.0 / k.abs();
            let f = friction(speed, radius, e);
            // Full superelevation on the circular part is a plateau of one friction: within 1e-12 the first
            // station stays, so the place does not hang on the last bit of the curvature.
            if best.as_ref().is_none_or(|b| f > b.most + 1e-12) {
                best = Some(CurveFriction { pi: c.pi.clone(), from, to, most: f, at: s, radius, e });
            }
        }
        out.extend(best);
    }
    out
}

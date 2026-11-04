//! A design vehicle's swept path (plan/road.md Y17, docs/adr/0915; ROAD-JUNC-06, 07): its front axle's centre driven
//! along a path, its rear axle (and a trailer's axle behind its kingpin) following as a rigid bar does, the body's
//! corners' tracks, how far the rear runs inside the front (off-tracking), and the least clearance of the body to
//! obstacle lines (curbs). The vehicle's sizes are the user's (the agency's design vehicle); nothing here is a
//! standard's number.
//!
//! - **Kinematic model:** a rigid bar of the wheelbase L between the front and the rear axle (the bicycle model at low
//!   speed: no slip). While the front moves straight a distance s along u, the bar's angle φ to u follows the tractrix:
//!   tan(φ/2) = tan(φ₀/2) e^(−s/L), exactly; a path is taken as its chords, so the only approximation is the chords'
//!   (their sagitta, the sampling tolerance said back). A trailer's axle follows its kingpin (a point on the tractor's
//!   axis, `kingpin` behind its rear axle) by the same rule over the kingpin's chords.
//! - **Body:** a rectangle of the unit's width from its rear overhang behind its rear axle to its front overhang ahead
//!   of its front axle (a trailer's from its kingpin), placed at every sample.
//! - **Clearance:** the least distance from a body's edges to an obstacle segment at any sample; zero where they meet.

/// A towed unit behind the tractor's kingpin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trailer {
    /// The kingpin's place behind the tractor's rear axle on its axis, m (negative: ahead of it).
    pub kingpin: f64,
    /// From the kingpin to the trailer's axle, m.
    pub wheelbase: f64,
    /// The trailer's body ahead of the kingpin and behind its axle, and its width, m.
    pub front: f64,
    pub rear: f64,
    pub width: f64,
}

/// A design vehicle: the tractor's wheelbase, overhangs and width, m; an optional trailer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vehicle {
    pub wheelbase: f64,
    pub front: f64,
    pub rear: f64,
    pub width: f64,
    pub trailer: Option<Trailer>,
}

/// Why a vehicle does not drive a path.
#[derive(Clone, Debug, PartialEq)]
pub enum SweptError {
    /// A size not finite, or a wheelbase or width not positive, an overhang negative.
    Vehicle,
    /// Fewer than two places, or a step not positive.
    Path,
}

impl std::fmt::Display for SweptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SweptError::Vehicle => "aracın dingil aralığı ve genişliği sıfırdan büyük, çıkıntıları eksi olmayan sayılar olmalı",
            SweptError::Path => "yolun en az iki ayrı yeri ve sıfırdan büyük bir örnekleme aralığı olmalı",
        })
    }
}

/// One sample of the drive: how far the front has gone, and each unit's axles and body corners (front left, front
/// right, rear right, rear left).
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub distance: f64,
    pub front: [f64; 2],
    pub rear: [f64; 2],
    pub trailer: Option<[f64; 2]>,
    pub bodies: Vec<[[f64; 2]; 4]>,
}

/// A drive: its samples, the largest off-tracking (the rear axle's distance from the front's path, and the trailer's),
/// and the chords' largest distance from the path (the sampling tolerance; zero for a polyline driven as it is).
#[derive(Clone, Debug, PartialEq)]
pub struct Swept {
    pub samples: Vec<Sample>,
    pub offtracking: f64,
    pub trailer_offtracking: Option<f64>,
    pub step: f64,
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn norm(a: [f64; 2]) -> f64 {
    a[0].hypot(a[1])
}

fn unit(a: [f64; 2]) -> [f64; 2] {
    let n = norm(a);
    [a[0] / n, a[1] / n]
}

/// The follower `r` of a leader moving straight from `f` to `g` on a bar of length `l` (exact tractrix).
fn follow(r: [f64; 2], f: [f64; 2], g: [f64; 2], l: f64) -> [f64; 2] {
    let d = sub(g, f);
    let s = norm(d);
    if s == 0.0 {
        return r;
    }
    let u = [d[0] / s, d[1] / s];
    let n = [-u[1], u[0]];
    let b = sub(f, r);
    let (c, sn) = ((b[0] * u[0] + b[1] * u[1]) / l, (b[0] * n[0] + b[1] * n[1]) / l);
    let phi = sn.atan2(c);
    // tan(φ/2) e^(−s/L); φ = ±π (the bar pointing back) stays.
    let half = (phi / 2.0).tan() * (-s / l).exp();
    let phi2 = if half.is_finite() { 2.0 * half.atan() } else { phi };
    [g[0] - l * (phi2.cos() * u[0] + phi2.sin() * n[0]), g[1] - l * (phi2.cos() * u[1] + phi2.sin() * n[1])]
}

/// A body rectangle from `back` to `ahead` along the unit's heading, `width` wide.
fn body(back: [f64; 2], ahead: [f64; 2], width: f64) -> [[f64; 2]; 4] {
    let h = unit(sub(ahead, back));
    let n = [-h[1] * width / 2.0, h[0] * width / 2.0];
    [
        [ahead[0] + n[0], ahead[1] + n[1]],
        [ahead[0] - n[0], ahead[1] - n[1]],
        [back[0] - n[0], back[1] - n[1]],
        [back[0] + n[0], back[1] + n[1]],
    ]
}

/// An arc's places: from `start` (radians) through `sweep` (radians, counter-clockwise positive) round `centre` at
/// `radius`, in n = ⌈|sweep|·radius / step⌉ equal pieces (at least one), both ends included.
pub fn arc_places(centre: [f64; 2], radius: f64, start: f64, sweep: f64, step: f64) -> Vec<[f64; 2]> {
    let n = ((sweep.abs() * radius / step).ceil() as usize).max(1);
    (0..=n)
        .map(|k| {
            let a = start + sweep * k as f64 / n as f64;
            [centre[0] + radius * a.cos(), centre[1] + radius * a.sin()]
        })
        .collect()
}

/// A polyline's edge from `a` to `b` with `bulge` (tan of a quarter of its sweep, positive counter-clockwise) as places
/// by [`arc_places`]; a straight edge is its two ends.
pub fn bulge_places(a: [f64; 2], b: [f64; 2], bulge: f64, step: f64) -> Vec<[f64; 2]> {
    if bulge == 0.0 {
        return vec![a, b];
    }
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let c = dx.hypot(dy);
    if c == 0.0 {
        return vec![a, b];
    }
    // The centre off the chord's middle, to the left of the chord by (c/2)(1 − b²)/(2b).
    let h = c / 2.0 * (1.0 - bulge * bulge) / (2.0 * bulge);
    let centre = [(a[0] + b[0]) / 2.0 - dy / c * h, (a[1] + b[1]) / 2.0 + dx / c * h];
    let radius = (a[0] - centre[0]).hypot(a[1] - centre[1]);
    let start = (a[1] - centre[1]).atan2(a[0] - centre[0]);
    let mut out = arc_places(centre, radius, start, 4.0 * bulge.atan(), step);
    if let Some(last) = out.last_mut() {
        *last = b;
    }
    out
}

/// The places of `path` every `step` metres along it (and its corners), with how far along each is.
pub fn resample(path: &[[f64; 2]], step: f64) -> Vec<(f64, [f64; 2])> {
    let mut out = vec![(0.0, path[0])];
    let mut at = 0.0;
    for w in path.windows(2) {
        let len = norm(sub(w[1], w[0]));
        if len == 0.0 {
            continue;
        }
        let n = (len / step).ceil().max(1.0) as usize;
        for k in 1..=n {
            let t = k as f64 / n as f64;
            out.push((at + t * len, [w[0][0] + t * (w[1][0] - w[0][0]), w[0][1] + t * (w[1][1] - w[0][1])]));
        }
        at += len;
    }
    out
}

/// The distance of `p` to the path's stretch from `from` to `to` metres along it (`along` its places' distances).
fn distance_near(path: &[[f64; 2]], along: &[f64], from: f64, to: f64, p: [f64; 2]) -> f64 {
    let start = along.partition_point(|d| *d < from).saturating_sub(1);
    let mut best = f64::INFINITY;
    for k in start..path.len().saturating_sub(1) {
        if along[k] > to {
            break;
        }
        best = best.min(segment_point(path[k], path[k + 1], p));
    }
    best
}

fn segment_point(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    let (d, w) = (sub(b, a), sub(p, a));
    let l2 = d[0] * d[0] + d[1] * d[1];
    let t = if l2 > 0.0 { ((w[0] * d[0] + w[1] * d[1]) / l2).clamp(0.0, 1.0) } else { 0.0 };
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}

/// The front axle driven along `path` (its places in order, sampled every `step` metres along its chords), the
/// vehicle aligned with the path's first chord at the start.
pub fn drive(vehicle: &Vehicle, path: &[[f64; 2]], step: f64) -> Result<Swept, SweptError> {
    let ok = |v: f64| v.is_finite();
    let units = [vehicle.wheelbase, vehicle.width];
    let mut bad = !(units.iter().all(|v| ok(*v) && *v > 0.0)
        && ok(vehicle.front)
        && vehicle.front >= 0.0
        && ok(vehicle.rear)
        && vehicle.rear >= 0.0);
    if let Some(t) = vehicle.trailer {
        bad |= !(ok(t.kingpin) && ok(t.wheelbase) && t.wheelbase > 0.0 && ok(t.width) && t.width > 0.0 && ok(t.front) && t.front >= 0.0 && ok(t.rear) && t.rear >= 0.0);
    }
    if bad {
        return Err(SweptError::Vehicle);
    }
    let mut places: Vec<[f64; 2]> = path.to_vec();
    places.dedup();
    if places.len() < 2 || !(step.is_finite() && step > 0.0) || places.iter().any(|p| !(ok(p[0]) && ok(p[1]))) {
        return Err(SweptError::Path);
    }
    let l = vehicle.wheelbase;
    let first = unit(sub(places[1], places[0]));
    let mut f = places[0];
    let mut r = [f[0] - l * first[0], f[1] - l * first[1]];
    let kingpin = |f: [f64; 2], r: [f64; 2], k: f64| {
        let h = unit(sub(f, r));
        [r[0] - k * h[0], r[1] - k * h[1]]
    };
    let mut trailer = vehicle.trailer.map(|t| {
        let k = kingpin(f, r, t.kingpin);
        [k[0] - t.wheelbase * first[0], k[1] - t.wheelbase * first[1]]
    });
    let sample = |d: f64, f: [f64; 2], r: [f64; 2], trailer: Option<[f64; 2]>| {
        let h = unit(sub(f, r));
        let mut bodies = vec![body(
            [r[0] - vehicle.rear * h[0], r[1] - vehicle.rear * h[1]],
            [f[0] + vehicle.front * h[0], f[1] + vehicle.front * h[1]],
            vehicle.width,
        )];
        if let (Some(t), Some(ta)) = (vehicle.trailer, trailer) {
            let k = kingpin(f, r, t.kingpin);
            let ht = unit(sub(k, ta));
            bodies.push(body(
                [ta[0] - t.rear * ht[0], ta[1] - t.rear * ht[1]],
                [k[0] + t.front * ht[0], k[1] + t.front * ht[1]],
                t.width,
            ));
        }
        Sample { distance: d, front: f, rear: r, trailer, bodies }
    };
    let mut samples = vec![sample(0.0, f, r, trailer)];
    for (d, g) in resample(&places, step).into_iter().skip(1) {
        let r2 = follow(r, f, g, l);
        if let (Some(t), Some(ta)) = (vehicle.trailer, trailer) {
            let (k1, k2) = (kingpin(f, r, t.kingpin), kingpin(g, r2, t.kingpin));
            trailer = Some(follow(ta, k1, k2, t.wheelbase));
        }
        f = g;
        r = r2;
        samples.push(sample(d, f, r, trailer));
    }
    // The off-tracking against the path with its lead-in (the line the vehicle stood on at the start), each axle
    // against the stretch within the vehicle's length behind the front.
    let reach = l + vehicle.trailer.map_or(0.0, |t| t.kingpin.abs() + t.wheelbase);
    let lead = [places[0][0] - reach * first[0], places[0][1] - reach * first[1]];
    let mut line = vec![lead];
    line.extend(&places);
    let mut along = vec![-reach, 0.0];
    for w in places.windows(2) {
        let last = along.last().copied().unwrap_or(0.0);
        along.push(last + norm(sub(w[1], w[0])));
    }
    let off = |p: [f64; 2], d: f64| distance_near(&line, &along, d - 2.0 * reach - step, d + step, p);
    let offtracking = samples.iter().map(|s| off(s.rear, s.distance)).fold(0.0, f64::max);
    let trailer_offtracking =
        vehicle.trailer.map(|_| samples.iter().filter_map(|s| s.trailer.map(|t| off(t, s.distance))).fold(0.0, f64::max));
    Ok(Swept { samples, offtracking, trailer_offtracking, step })
}

/// Whether two segments cross or touch (collinear ones only where they overlap).
fn segments_meet(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let o = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    // `r` on the segment from `p` to `q`, given it is on its line.
    let on = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        r[0] >= p[0].min(q[0]) && r[0] <= p[0].max(q[0]) && r[1] >= p[1].min(q[1]) && r[1] <= p[1].max(q[1])
    };
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    if d1 != 0.0 && d2 != 0.0 && d3 != 0.0 && d4 != 0.0 {
        return (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0);
    }
    (d1 == 0.0 && on(c, d, a)) || (d2 == 0.0 && on(c, d, b)) || (d3 == 0.0 && on(a, b, c)) || (d4 == 0.0 && on(a, b, d))
}

fn segments_distance(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> f64 {
    if segments_meet(a, b, c, d) {
        return 0.0;
    }
    segment_point(a, b, c).min(segment_point(a, b, d)).min(segment_point(c, d, a)).min(segment_point(c, d, b))
}

/// The least clearance found: its distance, the sample, the body (0 the tractor, 1 the trailer) and the obstacle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clearance {
    pub distance: f64,
    pub sample: usize,
    pub body: usize,
    pub obstacle: usize,
}

/// The least clearance of the drive's bodies to the obstacles (polylines); zero where a body's edge meets an obstacle.
/// A point of an obstacle inside a body is a meeting too. The first least found (sample, body, obstacle in order).
pub fn clearance(swept: &Swept, obstacles: &[Vec<[f64; 2]>]) -> Option<Clearance> {
    let mut best: Option<Clearance> = None;
    for (k, s) in swept.samples.iter().enumerate() {
        for (j, b) in s.bodies.iter().enumerate() {
            for (o, line) in obstacles.iter().enumerate() {
                let mut d = f64::INFINITY;
                for w in line.windows(2) {
                    for i in 0..4 {
                        d = d.min(segments_distance(b[i], b[(i + 1) % 4], w[0], w[1]));
                    }
                }
                if line.iter().any(|p| inside(b, *p)) {
                    d = 0.0;
                }
                if d.is_finite() && best.is_none_or(|c| d < c.distance) {
                    best = Some(Clearance { distance: d, sample: k, body: j, obstacle: o });
                }
            }
        }
    }
    best
}

fn inside(r: &[[f64; 2]; 4], p: [f64; 2]) -> bool {
    (0..4).all(|i| {
        let (a, b) = (r[i], r[(i + 1) % 4]);
        (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) <= 0.0
    })
}

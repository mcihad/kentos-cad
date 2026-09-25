//! Where symbols go on a geometry (formerly in the TypeScript
//! `style/geometry.ts` and the helpers of `style/compile.ts`): marker
//! places along a path, waves laid along a path, a point inside an area,
//! an area's centre of mass and the middle of a line. All in float64 world
//! coordinates.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::centroid;
use kentos_geometry_core::jsmath::{
    PI, atan2, cos, js_cmp, js_floor, js_hypot, js_max, js_min, sin, stable_sort,
};

/// A marker place: where, and the path's direction there (radians).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub at: Vec2,
    pub angle: f64,
}

/// Greater than zero (NaN is not): the former TypeScript's `!(x > 0)` tests.
pub(crate) fn positive(x: f64) -> bool {
    x > 0.0
}

/// No path gets more markers than this (a tiny interval would hang the page).
pub const MAX_MARKERS_PER_PATH: usize = 50_000;

const SHARP_TURN: f64 = PI / 6.0;

fn dir_of(a: Vec2, b: Vec2) -> f64 {
    atan2(b.y - a.y, b.x - a.x)
}

/// A path's straight piece with its length, its start along the path and its direction.
#[derive(Clone, Copy)]
struct Seg {
    a: Vec2,
    b: Vec2,
    len: f64,
    start: f64,
    angle: f64,
}

/// The pieces of a path longer than 1e-12, and the path's length.
fn segments(pts: &[Vec2], closed: bool) -> (Vec<Seg>, f64) {
    let n = pts.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    let mut segs = Vec::with_capacity(count);
    let mut total = 0.0;
    for i in 0..count {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let len = js_hypot(b.x - a.x, b.y - a.y);
        if len < 1e-12 {
            continue;
        }
        segs.push(Seg {
            a,
            b,
            len,
            start: total,
            angle: dir_of(a, b),
        });
        total += len;
    }
    (segs, total)
}

/// The piece containing distance `s` (the last one starting at or before it).
fn find(segs: &[Seg], s: f64) -> usize {
    let mut lo = 0;
    let mut hi = segs.len() - 1;
    while lo < hi {
        let mid = (lo + hi + 1) >> 1;
        if segs[mid].start <= s {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

/// The point and direction at distance `s` along the pieces (`s` clamped to
/// the piece; `pick` is where to look for it).
fn point_on(g: &Seg, s: f64) -> Placed {
    let t = js_min(1.0, js_max(0.0, (s - g.start) / g.len));
    Placed {
        at: Vec2::new(g.a.x + (g.b.x - g.a.x) * t, g.a.y + (g.b.y - g.a.y) * t),
        angle: g.angle,
    }
}

/// Distances along the path of the corners that turn more than 30° (a closed path's start included).
fn sharp_corners(segs: &[Seg], closed: bool) -> Vec<f64> {
    let mut out = Vec::new();
    let tau = 2.0 * PI;
    for i in usize::from(!closed)..segs.len() {
        let prev = segs[(i + segs.len() - 1) % segs.len()];
        let mut turn = (segs[i].angle - prev.angle).abs() % tau;
        if turn > PI {
            turn = tau - turn;
        }
        if turn > SHARP_TURN {
            out.push(segs[i].start);
        }
    }
    out
}

/// The bisector direction at a corner: the mean of the directions in and out.
fn mean_angle(a: f64, b: f64) -> f64 {
    atan2(sin(a) + sin(b), cos(a) + cos(b))
}

/// `x` wrapped into [0, m) as `((x % m) + m) % m` does in JavaScript.
fn wrap(x: f64, m: f64) -> f64 {
    ((x % m) + m) % m
}

/// Several markers per place, `spacing` apart along the path and centred on it.
#[derive(Clone, Copy, Debug)]
pub struct PlaceGroup {
    pub count: f64,
    pub spacing: f64,
}

/// Marker positions along a path. `interval` places one every `interval`
/// from `offset_along` (a closed path does not repeat its start at the
/// end); vertices face the bisector of their corner. A `group` puts `count`
/// markers at each place instead of one, centred on it (dots in a dash
/// gap); on a closed path they wrap around, on an open one those past an
/// end are left out. `clear` keeps interval places that far from sharp
/// corners (turns over 30°): a code written along a boundary is left out
/// rather than bent around a corner.
pub fn place_along(
    pts: &[Vec2],
    closed: bool,
    placement: &str,
    interval: f64,
    offset_along: f64,
    group: Option<PlaceGroup>,
    clear: f64,
) -> Vec<Placed> {
    let n = pts.len();
    if n < 2 {
        return if n == 1 {
            vec![Placed {
                at: pts[0],
                angle: 0.0,
            }]
        } else {
            Vec::new()
        };
    }
    let (segs, total) = segments(pts, closed);
    if segs.is_empty() {
        return Vec::new();
    }
    let at = |s: f64| point_on(&segs[find(&segs, s)], s);
    // Where along the path each place is (vertices keep their bisector unless grouped).
    let mut places: Vec<(f64, Option<Placed>)> = Vec::new();
    match placement {
        "interval" => {
            if !positive(interval) {
                return Vec::new();
            }
            let first = wrap(offset_along, interval);
            let end = if closed { total - 1e-9 } else { total + 1e-9 };
            let mut s = if closed { first } else { offset_along };
            while s <= end && places.len() < MAX_MARKERS_PER_PATH {
                if s >= -1e-9 {
                    places.push((js_max(0.0, s), None));
                }
                s += interval;
            }
            if clear > 0.0 {
                let corners = sharp_corners(&segs, closed);
                if !corners.is_empty() {
                    let gap = |s: f64, c: f64| {
                        if closed {
                            js_min((s - c).abs(), total - (s - c).abs())
                        } else {
                            (s - c).abs()
                        }
                    };
                    places.retain(|p| corners.iter().all(|&c| gap(p.0, c) >= clear));
                }
            }
        }
        "center" => places.push((total / 2.0, None)),
        "segmentCenter" => {
            for g in &segs {
                places.push((
                    g.start + g.len / 2.0,
                    Some(Placed {
                        at: Vec2::new((g.a.x + g.b.x) / 2.0, (g.a.y + g.b.y) / 2.0),
                        angle: g.angle,
                    }),
                ));
            }
        }
        "first" => places.push((
            0.0,
            Some(Placed {
                at: segs[0].a,
                angle: segs[0].angle,
            }),
        )),
        "last" => {
            let g = segs[segs.len() - 1];
            places.push((
                total,
                Some(Placed {
                    at: g.b,
                    angle: g.angle,
                }),
            ));
        }
        "vertex" | "innerVertex" => {
            let inner = placement == "innerVertex";
            for i in 0..segs.len() {
                let prev = if i > 0 {
                    Some(segs[i - 1])
                } else if closed {
                    Some(segs[segs.len() - 1])
                } else {
                    None
                };
                if prev.is_none() && inner {
                    continue;
                }
                places.push((
                    segs[i].start,
                    Some(Placed {
                        at: segs[i].a,
                        angle: prev.map_or(segs[i].angle, |p| mean_angle(p.angle, segs[i].angle)),
                    }),
                ));
            }
            if !closed && !inner {
                let g = segs[segs.len() - 1];
                places.push((
                    total,
                    Some(Placed {
                        at: g.b,
                        angle: g.angle,
                    }),
                ));
            }
        }
        _ => {}
    }
    let count = group.map_or(1.0, |g| js_max(1.0, js_floor(g.count)));
    let spacing = group.map_or(f64::NAN, |g| g.spacing);
    if count == 1.0 || !positive(spacing) {
        return places
            .into_iter()
            .map(|(s, placed)| placed.unwrap_or_else(|| at(s)))
            .collect();
    }
    let mut out = Vec::new();
    let half = ((count - 1.0) * spacing) / 2.0;
    for &(ps, _) in &places {
        let mut i = 0.0;
        while i < count && out.len() < MAX_MARKERS_PER_PATH {
            let mut s = ps - half + i * spacing;
            i += 1.0;
            if closed {
                s = wrap(s, total);
            } else if s < -1e-9 || s > total + 1e-9 {
                continue;
            }
            out.push(at(js_min(total, js_max(0.0, s))));
        }
    }
    out
}

// ── Waves ──────────────────────────────────────────────────────────────

/// Points per wave: enough for a smooth sine at any zoom a symbol is drawn at.
const WAVE_STEPS: usize = 16;

/// A wave laid along a path (world units).
#[derive(Clone, Debug)]
pub struct WaveSpec {
    pub shape: String,
    /// One wave's length along the path, its height to each side, and the repeat.
    pub length: f64,
    pub amplitude: f64,
    pub spacing: f64,
    /// Straight line between waves when the repeat is longer than a wave.
    pub connect: bool,
    /// Start of the first wave from the path's start; None: waves centred on the path.
    pub offset_along: Option<f64>,
}

/// Height of a wave at t ∈ [0, 1] of its length, in amplitudes (starts and ends on the line).
fn wave_at(shape: &str, t: f64) -> f64 {
    match shape {
        "sine" => sin(t * 2.0 * PI),
        "zigzag" => {
            if t < 0.25 {
                t * 4.0
            } else if t < 0.75 {
                2.0 - t * 4.0
            } else {
                t * 4.0 - 4.0
            }
        }
        _ => {
            if t <= 0.0 || t >= 1.0 {
                0.0
            } else if t < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
    }
}

/// Arc-length access to a path: point and direction at a distance, and the corners inside a stretch.
struct Walker {
    segs: Vec<Seg>,
    total: f64,
}

impl Walker {
    fn new(pts: &[Vec2], closed: bool) -> Option<Walker> {
        let (segs, total) = segments(pts, closed);
        (!segs.is_empty()).then_some(Walker { segs, total })
    }

    fn at(&self, s: f64) -> Placed {
        point_on(
            &self.segs[find(&self.segs, js_min(self.total, js_max(0.0, s)))],
            s,
        )
    }

    fn corners_between(&self, s0: f64, s1: f64, out: &mut Vec<Vec2>) {
        for g in &self.segs {
            if g.start > s0 + 1e-9 && g.start < s1 - 1e-9 {
                out.push(g.a);
            }
        }
    }
}

/// A path drawn as waves: each wave is laid along the path (following its
/// bends) and pushed to the left by the wave height. Returns the pieces to
/// stroke: one continuous path when the waves connect, one per wave when the
/// line between them is left out. Waves that would pass an open path's end
/// are cut short there.
pub fn wave_paths(pts: &[Vec2], closed: bool, w: &WaveSpec) -> Vec<Vec<Vec2>> {
    if !positive(w.length) || pts.len() < 2 {
        return vec![pts.to_vec()];
    }
    let spacing = js_max(w.length, w.spacing);
    let Some(along) = Walker::new(pts, closed) else {
        return Vec::new();
    };
    let total = along.total;
    // Anchored: waves from `offset_along` on, as many as fit (a closed path wraps the start into its first repeat).
    let first = match w.offset_along {
        None => 0.0,
        Some(o) if closed => wrap(o, spacing),
        Some(o) => js_max(0.0, o),
    };
    let fit = if w.offset_along.is_some() {
        js_floor((total - first - w.length + 1e-9) / spacing) + 1.0
    } else {
        js_floor((total + 1e-9) / spacing)
    };
    let count = js_min(MAX_MARKERS_PER_PATH as f64, fit);
    // As the TypeScript compared: a NaN count draws no wave rather than the plain path.
    if count < 1.0 {
        return vec![pts.to_vec()];
    }
    let mut out = Vec::new();
    let mut current: Vec<Vec2> = Vec::new();
    let push = |current: &mut Vec<Vec2>, s: f64, h: f64| {
        let p = along.at(s);
        current.push(Vec2::new(
            p.at.x - sin(p.angle) * h,
            p.at.y + cos(p.angle) * h,
        ));
    };
    // Waves centred along the path, the rest shared at both ends (or from the anchor on).
    let start = if w.offset_along.is_some() {
        first
    } else {
        (total - count * spacing) / 2.0 + (spacing - w.length) / 2.0
    };
    let square = w.shape == "square";
    let mut k = 0.0;
    while k < count {
        let s0 = start + k * spacing;
        if w.connect {
            if k == 0.0 {
                push(&mut current, 0.0, 0.0);
            }
            // The corners of the path between waves stay corners.
            let from = if k == 0.0 {
                0.0
            } else {
                js_max(0.0, s0 - (spacing - w.length))
            };
            along.corners_between(from, s0, &mut current);
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        if square {
            let a = w.amplitude;
            push(&mut current, s0, 0.0);
            push(&mut current, s0, a);
            push(&mut current, s0 + w.length / 2.0, a);
            push(&mut current, s0 + w.length / 2.0, -a);
            push(&mut current, s0 + w.length, -a);
            push(&mut current, s0 + w.length, 0.0);
        } else {
            for i in 0..=WAVE_STEPS {
                let f = i as f64;
                push(
                    &mut current,
                    s0 + (w.length * f) / WAVE_STEPS as f64,
                    wave_at(&w.shape, f / WAVE_STEPS as f64) * w.amplitude,
                );
            }
        }
        k += 1.0;
    }
    if w.connect {
        along.corners_between(
            start + (count - 1.0) * spacing + w.length,
            total,
            &mut current,
        );
        push(&mut current, if closed { 0.0 } else { total }, 0.0);
    }
    if current.len() > 1 {
        out.push(current);
    }
    out
}

// ── Inside point ───────────────────────────────────────────────────────

fn inside_rings(rings: &[Vec<Vec2>], p: Vec2) -> bool {
    let mut inside = false;
    for r in rings {
        let n = r.len();
        let mut j = n.wrapping_sub(1);
        for i in 0..n {
            let a = r[i];
            let b = r[j];
            if (a.y > p.y) != (b.y > p.y) && p.x < ((b.x - a.x) * (p.y - a.y)) / (b.y - a.y) + a.x {
                inside = !inside;
            }
            j = i;
        }
    }
    inside
}

/// A point inside the area where a symbol or text sits well: the centroid
/// when it is inside, else the middle of the widest inside stretch of a
/// few horizontal scan lines (GEOS "point on surface", simplified).
pub fn interior_point(rings: &[Vec<Vec2>]) -> Option<Vec2> {
    let outer = rings.first()?;
    if outer.len() < 3 {
        return None;
    }
    let c = centroid(outer);
    if inside_rings(rings, c) {
        return Some(c);
    }
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in outer {
        min_y = js_min(min_y, p.y);
        max_y = js_max(max_y, p.y);
    }
    let mut best: Option<(f64, f64, f64)> = None;
    let mut xs: Vec<f64> = Vec::new();
    for f in [0.5, 0.35, 0.65, 0.2, 0.8, 0.1, 0.9] {
        let y = min_y + (max_y - min_y) * f;
        xs.clear();
        for r in rings {
            let n = r.len();
            let mut j = n.wrapping_sub(1);
            for i in 0..n {
                let a = r[i];
                let b = r[j];
                if (a.y > y) != (b.y > y) {
                    xs.push(a.x + ((y - a.y) * (b.x - a.x)) / (b.y - a.y));
                }
                j = i;
            }
        }
        // `(p, q) => p - q`, as JavaScript sorts: equal crossings keep their order.
        stable_sort(&mut xs, &mut |p, q| js_cmp(*p - *q, 0.0));
        let mut k = 0;
        while k + 1 < xs.len() {
            let w = xs[k + 1] - xs[k];
            if best.is_none_or(|b| w > b.2) {
                best = Some(((xs[k] + xs[k + 1]) / 2.0, y, w));
            }
            k += 2;
        }
    }
    Some(best.map_or(c, |b| Vec2::new(b.0, b.1)))
}

/// An area's centre of mass by the shoelace sums, or its first point when
/// it has no area (the centroid marker's "centroid" position).
pub fn centroid_of(ring: &[Vec2]) -> Option<Vec2> {
    if ring.is_empty() {
        return None;
    }
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let f = ring[j].x * ring[i].y - ring[i].x * ring[j].y;
        a += f;
        cx += (ring[j].x + ring[i].x) * f;
        cy += (ring[j].y + ring[i].y) * f;
        j = i;
    }
    Some(if a.abs() < 1e-12 {
        ring[0]
    } else {
        Vec2::new(cx / (3.0 * a), cy / (3.0 * a))
    })
}

/// The point halfway along the longest path of a line geometry.
pub fn line_middle(paths: &[(Vec<Vec2>, bool)]) -> Option<Vec2> {
    let mut best: Option<(&[Vec2], bool, f64)> = None;
    for (pts, closed) in paths {
        let n = pts.len();
        let count = if *closed { n } else { n.saturating_sub(1) };
        let mut len = 0.0;
        for i in 0..count {
            len += js_hypot(pts[(i + 1) % n].x - pts[i].x, pts[(i + 1) % n].y - pts[i].y);
        }
        if best.is_none_or(|b| len > b.2) {
            best = Some((pts, *closed, len));
        }
    }
    let (pts, closed, _) = best?;
    place_along(pts, closed, "center", 0.0, 0.0, None, 0.0)
        .first()
        .map(|p| p.at)
        .or_else(|| pts.first().copied())
}

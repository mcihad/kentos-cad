//! Object snap tracking, "nesne izleme" (`apps/web/src/viewport/objectTracking.ts`).
//! Points acquired by resting the cursor on a snap emit alignment lines
//! (horizontal/vertical, plus polar steps when polar tracking is on). The
//! cursor locks onto the nearest line, or onto the crossing of two lines
//! from different origins — "above this corner and level with that one".
//! The viewport feeds it world coordinates and a world tolerance.

use crate::api::Op;
use crate::jsmath::{PI, cos, js_hypot, js_round, sin, truthy};
use crate::op;
use crate::vec2::Vec2;

/// An alignment line: its origin and direction in degrees, counter-clockwise from east.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackLine {
    pub origin: Vec2,
    pub angle: f64,
}

crate::json_struct!(TrackLine { origin, angle });

/// Where the cursor locks: one line, or two when the point is where two alignments cross.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackHit {
    pub point: Vec2,
    pub lines: Vec<TrackLine>,
}

crate::json_struct!(TrackHit { point, lines });

/// Alignment directions: always 0/90/180/270, plus every polar step (a Set in insertion order).
pub fn track_angles(polar_step: Option<f64>) -> Vec<f64> {
    let mut out = vec![0.0, 90.0, 180.0, 270.0];
    if let Some(step) = polar_step
        && truthy(step)
        && step > 0.0
    {
        let mut a = 0.0;
        while a < 360.0 - 1e-9 {
            let v = js_round(a * 1e6) / 1e6;
            if !out.contains(&v) {
                out.push(v);
            }
            a += step;
        }
    }
    out
}

/// Unit direction; exact for the axes so "straight above" keeps the very same X.
fn dir(deg: f64) -> Vec2 {
    let a = ((deg % 360.0) + 360.0) % 360.0;
    if a == 0.0 {
        return Vec2::new(1.0, 0.0);
    }
    if a == 90.0 {
        return Vec2::new(0.0, 1.0);
    }
    if a == 180.0 {
        return Vec2::new(-1.0, 0.0);
    }
    if a == 270.0 {
        return Vec2::new(0.0, -1.0);
    }
    let r = (a * PI) / 180.0;
    Vec2::new(cos(r), sin(r))
}

struct Candidate {
    line: TrackLine,
    d: Vec2,
    off: f64,
}

struct Ray {
    o: Vec2,
    i: usize,
    angle: f64,
    d: Vec2,
}

/// Where the cursor `p` locks to, if anywhere within `tol`. `acquired` are
/// the tracking points; `from` (the command's last point) only takes part
/// in crossings, since polar tracking already covers lines through it.
pub fn track_point(
    p: Vec2,
    acquired: &[Vec2],
    from: Option<Vec2>,
    angles: &[f64],
    tol: f64,
) -> Option<TrackHit> {
    if acquired.is_empty() {
        return None;
    }
    let mut origins = acquired.to_vec();
    if let Some(f) = from {
        origins.push(f);
    }
    let mut lines = Vec::new();
    for (i, &o) in origins.iter().enumerate() {
        for &angle in angles {
            let d = dir(angle);
            let vx = p.x - o.x;
            let vy = p.y - o.y;
            // Rays point away from their origin; the opposite angle covers the other side.
            if vx * d.x + vy * d.y <= 0.0 {
                continue;
            }
            let extra = if i >= acquired.len() {
                f64::INFINITY
            } else {
                0.0
            };
            lines.push(Candidate {
                line: TrackLine { origin: o, angle },
                d,
                off: (d.x * vy - d.y * vx).abs() + extra,
            });
        }
    }

    // Crossings of two lines from different origins near the cursor win over a single line.
    let mut best: Option<(TrackHit, f64)> = None;
    let all: Vec<Ray> = origins
        .iter()
        .enumerate()
        .flat_map(|(i, &o)| {
            angles.iter().map(move |&angle| Ray {
                o,
                i,
                angle,
                d: dir(angle),
            })
        })
        .collect();
    for a in 0..all.len() {
        for b in a + 1..all.len() {
            let (ra, rb) = (&all[a], &all[b]);
            if ra.i == rb.i {
                continue;
            }
            let den = ra.d.x * rb.d.y - ra.d.y * rb.d.x;
            if den.abs() < 1e-9 {
                continue;
            }
            let t = ((rb.o.x - ra.o.x) * rb.d.y - (rb.o.y - ra.o.y) * rb.d.x) / den;
            let u = ((rb.o.x - ra.o.x) * ra.d.y - (rb.o.y - ra.o.y) * ra.d.x) / den;
            if t <= 1e-9 || u <= 1e-9 {
                continue;
            }
            let x = Vec2::new(ra.o.x + ra.d.x * t, ra.o.y + ra.d.y * t);
            let err = js_hypot(x.x - p.x, x.y - p.y);
            if err <= tol && best.as_ref().is_none_or(|(_, e)| err < *e) {
                let lines = vec![
                    TrackLine {
                        origin: ra.o,
                        angle: ra.angle,
                    },
                    TrackLine {
                        origin: rb.o,
                        angle: rb.angle,
                    },
                ];
                best = Some((TrackHit { point: x, lines }, err));
            }
        }
    }
    if let Some((hit, _)) = best {
        return Some(hit);
    }

    let mut single: Option<&Candidate> = None;
    for l in &lines {
        if l.off <= tol && single.is_none_or(|s| l.off < s.off) {
            single = Some(l);
        }
    }
    let s = single?;
    let o = s.line.origin;
    let t = (p.x - o.x) * s.d.x + (p.y - o.y) * s.d.y;
    Some(TrackHit {
        point: Vec2::new(o.x + s.d.x * t, o.y + s.d.y * t),
        lines: vec![s.line],
    })
}

/// Point `distance` along a single tracking line from its origin (typed distance while tracking).
pub fn along_track(hit: &TrackHit, distance: f64) -> Option<Vec2> {
    if hit.lines.len() != 1 {
        return None;
    }
    let TrackLine { origin, angle } = hit.lines[0];
    let d = dir(angle);
    Some(Vec2::new(
        origin.x + d.x * distance,
        origin.y + d.y * distance,
    ))
}

pub(crate) static OPS: &[Op] = &[
    op!("trackAngles", |polar_step: Option<f64>| track_angles(
        polar_step
    )),
    op!("trackPoint", |p: Vec2,
                       acquired: Vec<Vec2>,
                       from: Option<Vec2>,
                       angles: Vec<f64>,
                       tol: f64| {
        track_point(p, &acquired, from, &angles, tol)
    }),
    op!("alongTrack", |hit: TrackHit, distance: f64| along_track(
        &hit, distance
    )),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_above_keeps_the_very_same_x() {
        let p = Vec2::new(486512.34, 4420118.9);
        let hit = track_point(
            Vec2::new(p.x + 0.3, p.y + 250.0),
            &[p],
            None,
            &track_angles(Some(15.0)),
            1.0,
        );
        assert_eq!(hit.map(|h| h.point.x), Some(p.x));
    }

    #[test]
    fn polar_steps_join_the_axes_once_each() {
        assert_eq!(
            track_angles(Some(45.0)),
            vec![0.0, 90.0, 180.0, 270.0, 45.0, 135.0, 225.0, 315.0]
        );
        assert_eq!(track_angles(None), vec![0.0, 90.0, 180.0, 270.0]);
        assert_eq!(track_angles(Some(-15.0)).len(), 4);
    }
}

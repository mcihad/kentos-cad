//! Rotayı kalibre et (docs/adr/0224 §4): a route's measures made anew from
//! points with known measures along it.

use super::{Fault, ON, Route, RoutePart, Walk, dist, steps};
use crate::vec2::Vec2;

/// Calibrate's rules.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrateRules {
    pub radius: f64,
    pub before: Option<bool>,
    pub after: Option<bool>,
    pub ignore_gaps: Option<bool>,
    /// Vertices put where the points fall (true when absent); false: only
    /// the route's own vertices take measures (a line keeps its two).
    pub split: Option<bool>,
}

crate::json_struct!(CalibrateRules {
    radius,
    before,
    after,
    ignore_gaps => "ignoreGaps",
    split
});

/// A calibration point: where it is and its measure.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub p: Vec2,
    pub m: f64,
}

crate::json_struct!(Mark { p, m });

/// A route calibrated, and which points were used.
#[derive(Clone, Debug, PartialEq)]
pub struct Calibrated {
    pub parts: Vec<RoutePart>,
    pub used: Vec<bool>,
}

crate::json_struct!(Calibrated { parts, used });

/// The route with a vertex put at each place (part, edge, t) not within
/// [`ON`] of a vertex; a new vertex's measure is the edge's there.
fn split_at(route: &Route, places: &[(usize, usize, f64)]) -> Route {
    let mut sorted = places.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.total_cmp(&b.2)));
    let parts = route
        .parts
        .iter()
        .enumerate()
        .map(|(k, part)| {
            let all = steps(part);
            let mut pts = vec![part.pts[0]];
            let mut bulges = Vec::new();
            let mut ms = vec![part.ms.first().copied().flatten()];
            for (i, s) in all.iter().enumerate() {
                let from = sorted.partition_point(|q| (q.0, q.1) < (k, i));
                let upto = sorted.partition_point(|q| (q.0, q.1) <= (k, i));
                let mut cuts: Vec<f64> = sorted[from..upto].iter().map(|q| q.2).collect();
                cuts.dedup();
                let mut ts = vec![0.0];
                for t in cuts {
                    let p = s.at(t);
                    let prev = s.at(ts[ts.len() - 1]);
                    if dist(p, prev) > ON && dist(p, s.b) > ON {
                        ts.push(t);
                    }
                }
                ts.push(1.0);
                for w in ts.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    pts.push(s.at(b));
                    bulges.push(s.piece_bulge(a, b));
                    ms.push(if b == 1.0 {
                        part.ms.get(i + 1).copied().flatten()
                    } else {
                        s.m(b)
                    });
                }
            }
            RoutePart {
                pts,
                bulges: bulges.iter().any(|&b| b != 0.0).then_some(bulges),
                ms,
            }
        })
        .collect();
    Route { parts }
}

/// The length walked to each vertex of each part.
fn walk_route(route: &Route, ignore: bool) -> Vec<Vec<f64>> {
    let mut out = Vec::with_capacity(route.parts.len());
    let mut cum = 0.0;
    let mut last: Option<Vec2> = None;
    for part in &route.parts {
        if let (Some(e), Some(&first)) = (last, part.pts.first())
            && !ignore
        {
            cum += dist(e, first);
        }
        let mut walked = vec![cum];
        for s in steps(part) {
            cum += s.len;
            walked.push(cum);
        }
        out.push(walked);
        last = part.pts.last().copied();
    }
    out
}

/// The route's measures made from `marks`: the marks within the radius at
/// their closest places, at least two; vertices put where they fall; each
/// vertex's measure linear in the length walked between the marks around
/// it, before the first and after the last by the end intervals when asked,
/// else as it was.
pub fn calibrate(
    route: &Route,
    marks: &[Mark],
    rules: &CalibrateRules,
) -> Result<Calibrated, (Fault, Vec<bool>)> {
    let ignore = rules.ignore_gaps.unwrap_or(true);
    let walk = Walk::new(route);
    // The gaps walked before each part when they count.
    let mut gaps = Vec::with_capacity(route.parts.len());
    let mut gap = 0.0;
    let mut last: Option<Vec2> = None;
    for part in &route.parts {
        if let (Some(e), Some(&first)) = (last, part.pts.first())
            && !ignore
        {
            gap += dist(e, first);
        }
        gaps.push(gap);
        last = part.pts.last().copied();
    }
    let mut used = vec![false; marks.len()];
    let mut found: Vec<(f64, f64, usize, usize, f64, usize)> = Vec::new();
    for (k, mark) in marks.iter().enumerate() {
        let Some(place) = walk.locate(mark.p) else {
            continue;
        };
        if place.distance.abs() > rules.radius {
            continue;
        }
        used[k] = true;
        found.push((
            place.along + gaps[place.part],
            mark.m,
            place.part,
            place.edge,
            place.t,
            k,
        ));
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.5.cmp(&b.5)));
    // One measure a place: a later mark at the same place is left out.
    let mut kept: Vec<(f64, f64, usize, usize, f64, usize)> = Vec::new();
    for f in found {
        if kept.last().is_some_and(|k| k.0 == f.0) {
            used[f.5] = false;
            continue;
        }
        kept.push(f);
    }
    if kept.len() < 2 {
        return Err((Fault::FewPoints, used));
    }
    let places: Vec<(usize, usize, f64)> = if rules.split.unwrap_or(true) {
        kept.iter().map(|f| (f.2, f.3, f.4)).collect()
    } else {
        Vec::new()
    };
    let split = split_at(route, &places);
    let walked = walk_route(&split, ignore);
    let value = |a: &(f64, f64, usize, usize, f64, usize),
                 b: &(f64, f64, usize, usize, f64, usize),
                 s: f64| { a.1 + (b.1 - a.1) * (s - a.0) / (b.0 - a.0) };
    let (first, last) = (kept[0].0, kept[kept.len() - 1].0);
    let n = kept.len();
    let parts = split
        .parts
        .into_iter()
        .zip(walked)
        .map(|(part, ss)| {
            let ms = part
                .ms
                .iter()
                .zip(&ss)
                .map(|(&old, &s)| {
                    if s < first - ON {
                        if rules.before.unwrap_or(true) {
                            Some(value(&kept[0], &kept[1], s))
                        } else {
                            old
                        }
                    } else if s > last + ON {
                        if rules.after.unwrap_or(true) {
                            Some(value(&kept[n - 2], &kept[n - 1], s))
                        } else {
                            old
                        }
                    } else {
                        let mut k = 0;
                        while k + 2 < n && s > kept[k + 1].0 {
                            k += 1;
                        }
                        Some(value(&kept[k], &kept[k + 1], s))
                    }
                })
                .collect();
            RoutePart { ms, ..part }
        })
        .collect();
    Ok(Calibrated { parts, used })
}

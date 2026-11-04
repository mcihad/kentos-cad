//! A surface's figures and its section along a line (plan/road.md Y2.3): what the terrain's information panel shows
//! (OpenRoads' terrain statistics: the highest, the lowest elevation) and the heights along a line where it crosses the
//! triangles' edges (Civil 3D's `SampleElevations`), the ground line of a profile or a section.

use std::collections::BTreeSet;

use serde::Serialize;

use super::{Place, Surface};

/// A surface's figures: its corners, kept triangles, constrained edges, plan area (m²) and heights' range.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Figures {
    pub vertices: usize,
    pub triangles: usize,
    pub constrained: usize,
    pub area: f64,
    /// The lowest and highest heights of the kept triangles' corners; none without a triangle.
    pub min_z: Option<f64>,
    pub max_z: Option<f64>,
}

/// A place of a section: its distance along the line from its start, where it is and its height (none off the
/// surface). `gap`: the line from the previous place to this one is off the surface (outside or in a void).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionPoint {
    pub station: f64,
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub gap: bool,
}

/// Two places closer along a segment than this (m) are one.
const SAME: f64 = 1e-9;
/// A segment and an edge whose directions' sine is below this are parallel: their crossing is rounding's.
const PARALLEL: f64 = 1e-12;

impl Surface {
    /// Its figures.
    pub fn figures(&self) -> Figures {
        let mut area = 0.0;
        let mut used = BTreeSet::new();
        for t in &self.triangles {
            let [a, b, c] = t.map(|i| self.vertices[i as usize]);
            area += ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])) / 2.0;
            used.extend(t.iter().copied());
        }
        let heights = used.iter().map(|&i| self.vertices[i as usize][2]);
        Figures {
            vertices: self.vertices.len(),
            triangles: self.triangles.len(),
            constrained: self.constrained.len(),
            area,
            min_z: heights.clone().reduce(f64::min),
            max_z: heights.reduce(f64::max),
        }
    }

    /// The section along a line of two places or more: its corners, and every place it crosses a kept triangle's
    /// edge, in order; a crossing's height is the edge's own (linear along it), another place's its triangle's.
    pub fn section(&self, line: &[[f64; 2]]) -> Vec<SectionPoint> {
        let edges: BTreeSet<[u32; 2]> = self
            .triangles
            .iter()
            .flat_map(|t| [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]])
            .map(|[a, b]| if a < b { [a, b] } else { [b, a] })
            .collect();
        let mut out: Vec<SectionPoint> = Vec::new();
        let mut station = 0.0;
        for w in line.windows(2) {
            let (p, q) = (w[0], w[1]);
            let d = [q[0] - p[0], q[1] - p[1]];
            let length = d[0].hypot(d[1]);
            if length == 0.0 {
                continue;
            }
            // (t along the segment, a height from an edge if it is a crossing)
            let mut at: Vec<(f64, Option<f64>)> = vec![(0.0, None), (1.0, None)];
            let (lo, hi) = ([p[0].min(q[0]), p[1].min(q[1])], [p[0].max(q[0]), p[1].max(q[1])]);
            for e in &edges {
                let (a, b) = (self.vertices[e[0] as usize], self.vertices[e[1] as usize]);
                if a[0].max(b[0]) < lo[0] || a[0].min(b[0]) > hi[0] || a[1].max(b[1]) < lo[1] || a[1].min(b[1]) > hi[1] {
                    continue;
                }
                let r = [b[0] - a[0], b[1] - a[1]];
                let denom = d[0] * r[1] - d[1] * r[0];
                let ap = [a[0] - p[0], a[1] - p[1]];
                let rl = r[0].hypot(r[1]);
                if denom.abs() <= PARALLEL * length * rl {
                    // Parallel, or so nearly that the crossing is lost in rounding (a line along an edge whose
                    // direction is rounded): on the same line, the edge's ends within the segment are places.
                    if (ap[0] * d[1] - ap[1] * d[0]).abs() > SAME * length.max(1.0) {
                        continue;
                    }
                    for c in [a, b] {
                        let t = ((c[0] - p[0]) * d[0] + (c[1] - p[1]) * d[1]) / (length * length);
                        if (0.0..=1.0).contains(&t) {
                            at.push((t, Some(c[2])));
                        }
                    }
                    continue;
                }
                let t = (ap[0] * r[1] - ap[1] * r[0]) / denom;
                let u = (ap[0] * d[1] - ap[1] * d[0]) / denom;
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                    at.push((t, Some(a[2] + u * (b[2] - a[2]))));
                }
            }
            at.sort_by(|x, y| x.0.total_cmp(&y.0));
            let mut kept: Vec<(f64, Option<f64>)> = Vec::new();
            for (t, z) in at {
                match kept.last_mut() {
                    Some(last) if (t - last.0) * length < SAME => {
                        if last.1.is_none() {
                            last.1 = z;
                        }
                    }
                    _ => kept.push((t, z)),
                }
            }
            for (i, (t, z)) in kept.iter().enumerate() {
                if !out.is_empty() && i == 0 {
                    continue;
                }
                let (x, y) = (p[0] + t * d[0], p[1] + t * d[1]);
                let z = z.or_else(|| self.height(x, y).z);
                // The line from the previous place: off the surface where its middle is.
                let gap = i > 0 && {
                    let mid = (t + kept[i - 1].0) / 2.0;
                    self.height(p[0] + mid * d[0], p[1] + mid * d[1]).place != Place::Inside
                };
                out.push(SectionPoint {
                    station: station + t * length,
                    x,
                    y,
                    z,
                    gap,
                });
            }
            station += length;
        }
        out
    }
}

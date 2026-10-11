//! Kartografik genelleştirme (docs/adr/0221): the Genelleştirme tools'
//! smoothing, squaring, building simplification, aggregation, part removal,
//! elimination and coverage simplification, one rule for both platforms: the
//! desktop's tools call the functions with shapes, the web's
//! (`model/ops/generalize.ts`) call the operations in `calls` with whole
//! lists. The independent reference is scripts/fixtures/generalize_cases.py.
//!
//! - `smooth`: Yumuşat, Chaikin's corner cutting (§2).
//! - `orthogonal`: Dik açılandır, the iD and QGIS rule (§3).
//! - `building`: Bina sadeleştir, Ruas's short edges (§4).
//! - `aggregate`: Alanları kümele, the closing's bridges (§5).
//! - `parts`: Küçük parçaları ayıkla (§6).
//! - `eliminate`: Küçükleri komşuya kat (§7).
//! - `coverage` and `simplify`: the coverage's edges, Ortak kenarları
//!   koruyarak sadeleştir (§8) and Yumuşat's shared edges (§2).

pub mod aggregate;
pub mod building;
pub mod calls;
pub mod coverage;
pub mod eliminate;
pub mod orthogonal;
pub mod parts;
pub mod simplify;
pub mod smooth;
#[cfg(test)]
mod tests;

use crate::entity::Shape;
use crate::ops::convert::{Elevations, PathOf, Zs, paths};
use crate::vec2::Vec2;

pub(crate) use calls::OPS;

/// A path as the tools change it: its vertices, the bulge of the edge from
/// each (an open path's last is 0) and each vertex's elevation.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub pts: Vec<Vec2>,
    pub bulges: Vec<f64>,
    pub zs: Vec<Option<f64>>,
    pub closed: bool,
}

impl Track {
    pub fn empty(closed: bool) -> Track {
        Track {
            pts: Vec::new(),
            bulges: Vec::new(),
            zs: Vec::new(),
            closed,
        }
    }

    pub fn len(&self) -> usize {
        self.pts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pts.is_empty()
    }

    /// How many edges it has.
    pub fn edges(&self) -> usize {
        let n = self.pts.len();
        match (self.closed, n) {
            (_, 0 | 1) => 0,
            (true, _) => n,
            (false, _) => n - 1,
        }
    }

    /// Whether vertex `i` ends an arc edge.
    pub fn on_arc(&self, i: usize) -> bool {
        let n = self.pts.len();
        let into = if i > 0 {
            self.bulges[i - 1] != 0.0
        } else {
            self.closed && n > 1 && self.bulges[n - 1] != 0.0
        };
        let out = (self.closed || i + 1 < n) && self.bulges[i] != 0.0;
        into || out
    }

    /// Adds a vertex; one equal to the last merges with it, the edge from it
    /// the new one's (the zero-length edge between them goes).
    pub fn push(&mut self, p: Vec2, bulge: f64, z: Option<f64>) {
        if let Some(last) = self.pts.last()
            && *last == p
        {
            let k = self.pts.len() - 1;
            self.bulges[k] = bulge;
            return;
        }
        self.pts.push(p);
        self.bulges.push(bulge);
        self.zs.push(z);
    }

    /// A ring whose last vertex came back to its first: the last goes.
    pub fn close_up(&mut self) {
        if self.closed && self.pts.len() > 1 && self.pts.last() == self.pts.first() {
            self.pts.pop();
            self.bulges.pop();
            self.zs.pop();
        }
    }

    fn of(p: &PathOf, zs: Option<&Vec<Option<f64>>>) -> Track {
        let n = p.pts.len();
        let mut bulges = p.bulges.clone().unwrap_or_else(|| vec![0.0; n]);
        bulges.resize(n, 0.0);
        if !p.closed && n > 0 {
            bulges[n - 1] = 0.0;
        }
        Track {
            pts: p.pts.clone(),
            bulges,
            zs: (0..n)
                .map(|i| zs.and_then(|z| z.get(i)).copied().flatten())
                .collect(),
            closed: p.closed,
        }
    }
}

/// An object's paths (`ops::convert::paths`' order) with their elevations.
pub fn tracks_of(s: &Shape, zs: Option<&Zs>) -> Vec<Track> {
    paths(s)
        .iter()
        .enumerate()
        .map(|(k, p)| Track::of(p, zs.and_then(|z| z.get(k))))
        .collect()
}

/// The paths' parts and rings (`paths`' order): which part (from 1) and ring (0 the outer one) each is.
pub fn places_of(s: &Shape) -> Vec<(u32, u32)> {
    paths(s).iter().map(|p| (p.part, p.ring)).collect()
}

/// `s` with its paths replaced, in `tracks_of`' order, and their elevations.
pub fn with_tracks(s: &Shape, tracks: &[Track]) -> Elevations {
    let new = tracks
        .iter()
        .map(|t| (t.pts.clone(), t.bulges.clone()))
        .collect();
    let zs: Zs = tracks.iter().map(|t| t.zs.clone()).collect();
    Elevations {
        shape: crate::ops::convert::paths::rebuild(s, new),
        zs: crate::ops::convert::kept(zs),
    }
}

/// An area's rings as a polygon: the outer ring, its holes, then the other
/// parts with theirs; their elevations in `paths`' order.
pub fn polygon_of(parts: &[(Track, Vec<Track>)]) -> Option<Elevations> {
    use crate::entity::Part;
    use crate::geom::arrangement::Ring;
    let ((outer, holes), rest) = parts.split_first()?;
    let bulged = |t: &Track| t.bulges.iter().any(|&b| b != 0.0).then(|| t.bulges.clone());
    let ring = |t: &Track| Ring {
        pts: t.pts.clone(),
        bulges: bulged(t),
    };
    let mut zs: Zs = vec![outer.zs.clone()];
    zs.extend(holes.iter().map(|h| h.zs.clone()));
    let mut others = Vec::new();
    for (o, hs) in rest {
        zs.push(o.zs.clone());
        zs.extend(hs.iter().map(|h| h.zs.clone()));
        others.push(Part {
            pts: o.pts.clone(),
            bulges: bulged(o),
            holes: (!hs.is_empty()).then(|| hs.iter().map(ring).collect()),
        });
    }
    Some(Elevations {
        shape: Shape::Polygon {
            pts: outer.pts.clone(),
            bulges: bulged(outer),
            holes: (!holes.is_empty()).then(|| holes.iter().map(ring).collect()),
            parts: (!others.is_empty()).then_some(others),
        },
        zs: crate::ops::convert::kept(zs),
    })
}

/// An object's tracks grouped by part: each part's outer ring and its holes.
pub fn parts_of(s: &Shape, zs: Option<&Zs>) -> Vec<(Track, Vec<Track>)> {
    let mut out: Vec<(Track, Vec<Track>)> = Vec::new();
    for (t, (_, ring)) in tracks_of(s, zs).into_iter().zip(places_of(s)) {
        match out.last_mut() {
            Some((_, holes)) if ring > 0 => holes.push(t),
            _ => out.push((t, Vec::new())),
        }
    }
    out
}

/// The vertices a shape is written with (every part's, holes too).
pub fn vertex_count(s: &Shape) -> usize {
    crate::ops::geoprocess::written_vertices(s)
}

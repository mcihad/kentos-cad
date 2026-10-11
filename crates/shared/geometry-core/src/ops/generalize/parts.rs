//! Küçük parçaları ayıkla (docs/adr/0221 §6): ArcGIS's Eliminate Polygon
//! Part. A part or a hole smaller than the least area, or than the share of
//! the object's outer rings' area, goes; an object whose parts are all small
//! keeps its largest. What stays is unchanged, elevations too.

use super::Track;
use crate::geom::arrangement::Ring;
use crate::geom::region::ring_area;

/// Küçük parçaları ayıkla's settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartRule {
    /// The least area, m².
    pub min_area: f64,
    /// The least share of the outer rings' area, percent (0: not used).
    pub percent: f64,
    pub parts: bool,
    pub holes: bool,
}

/// What stays of an object, and how many parts and holes went.
#[derive(Clone, Debug, PartialEq)]
pub struct Kept {
    pub parts: Vec<(Track, Vec<Track>)>,
    pub parts_gone: usize,
    pub holes_gone: usize,
}

/// A ring's area, unsigned (its arcs exact).
pub fn track_area(t: &Track) -> f64 {
    ring_area(&Ring {
        pts: t.pts.clone(),
        bulges: t.bulges.iter().any(|&b| b != 0.0).then(|| t.bulges.clone()),
    })
    .abs()
}

/// An object's parts with the small ones gone.
pub fn drop_parts(parts: &[(Track, Vec<Track>)], rule: &PartRule) -> Kept {
    let total: f64 = parts.iter().map(|(o, _)| track_area(o)).sum();
    let small =
        |a: f64| a < rule.min_area || (rule.percent > 0.0 && a < total * rule.percent / 100.0);
    let net: Vec<f64> = parts
        .iter()
        .map(|(o, hs)| track_area(o) - hs.iter().map(track_area).sum::<f64>())
        .collect();
    let mut keep: Vec<bool> = net.iter().map(|&a| !rule.parts || !small(a)).collect();
    if !keep.iter().any(|&k| k) {
        // All small: the largest stays (the first of equal ones).
        let mut best = 0;
        for (i, &a) in net.iter().enumerate() {
            if a > net[best] {
                best = i;
            }
        }
        keep[best] = true;
    }
    let mut out = Vec::new();
    let mut holes_gone = 0;
    for ((outer, holes), &k) in parts.iter().zip(&keep) {
        if !k {
            continue;
        }
        let kept: Vec<Track> = holes
            .iter()
            .filter(|h| !rule.holes || !small(track_area(h)))
            .cloned()
            .collect();
        holes_gone += holes.len() - kept.len();
        out.push((outer.clone(), kept));
    }
    Kept {
        parts_gone: parts.len() - out.len(),
        holes_gone,
        parts: out,
    }
}

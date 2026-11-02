//! A sample line (docs/adr/0903 §1, ROAD-SEC-01, 02): at an alignment's station, square to it, from `left` metres on
//! its left to `right` metres on its right; the terrain along it is the surface's section (ADR 0900, exact where the
//! line crosses a triangle's edge), each place by its offset from the alignment, right positive.

use crate::alignment::Alignment;
use crate::surface::Surface;

/// A sample line: its station, its swath widths and its ends on the plane, left first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampleLine {
    pub station: f64,
    pub left: f64,
    pub right: f64,
    /// The left end and the right end.
    pub ends: [[f64; 2]; 2],
    /// The alignment's place and direction there.
    pub at: [f64; 2],
    pub direction: f64,
}

/// The sample line at `station`, none off the alignment or with a width not above 0.
pub fn sample_line(al: &Alignment, station: f64, left: f64, right: f64) -> Option<SampleLine> {
    if !(left > 0.0 && right > 0.0 && left.is_finite() && right.is_finite()) {
        return None;
    }
    let p = al.at(station)?;
    // Right of the direction θ is (sin θ, −cos θ).
    let (rx, ry) = (p.direction.sin(), -p.direction.cos());
    Some(SampleLine {
        station,
        left,
        right,
        ends: [[p.x - rx * left, p.y - ry * left], [p.x + rx * right, p.y + ry * right]],
        at: [p.x, p.y],
        direction: p.direction,
    })
}

/// A place of the terrain along a sample line: its offset (right positive), its height (none off the surface), and
/// whether the stretch from the place before is off the surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundPoint {
    pub offset: f64,
    pub z: Option<f64>,
    pub gap: bool,
    /// Where the line crosses a triangle's edge (an end of the line is not, unless it lies on one).
    pub crossing: bool,
}

/// The terrain along `line`, from its left end to its right.
pub fn ground(surface: &Surface, line: &SampleLine) -> Vec<GroundPoint> {
    let points = surface.section(&line.ends);
    let n = points.len();
    points
        .into_iter()
        .enumerate()
        .map(|(i, p)| GroundPoint {
            offset: p.station - line.left,
            z: p.z,
            gap: p.gap,
            crossing: i > 0 && i + 1 < n,
        })
        .collect()
}

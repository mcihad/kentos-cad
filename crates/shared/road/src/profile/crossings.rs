//! What a road crosses in its profile (docs/adr/0946, plan/road.md Y40; ROAD-PROFILE-07): a structure over the road
//! (a bridge, an overpass, a power line) the grade line must pass under by a clearance, or one under it (a culvert, a
//! pipe) it must pass over by a cover. Each is a bound on the grade line's elevation at its station: under a
//! structure z ≤ its underside − the clearance, over one z ≥ its top + the cover. The values are the project's.

use super::propose::Bound;
use super::vertical::Vertical;

/// Where the structure is against the road.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossingKind {
    /// Over the road (its `elevation` its underside): the road passes under.
    Above,
    /// Under the road (its `elevation` its top): the road passes over.
    Below,
}

/// A crossing: its station on the road, which side the structure is, its elevation (underside or top) and the
/// clearance or cover (m).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crossing {
    pub station: f64,
    pub kind: CrossingKind,
    pub elevation: f64,
    pub clearance: f64,
}

impl Crossing {
    /// The grade line's limit there.
    pub fn limit(&self) -> f64 {
        match self.kind {
            CrossingKind::Above => self.elevation - self.clearance,
            CrossingKind::Below => self.elevation + self.clearance,
        }
    }

    /// The bound a proposal keeps.
    pub fn bound(&self) -> Bound {
        match self.kind {
            CrossingKind::Above => Bound { station: self.station, low: None, high: Some(self.limit()) },
            CrossingKind::Below => Bound { station: self.station, low: Some(self.limit()), high: None },
        }
    }
}

/// A crossing against a grade line: the line's elevation there (none off the profile), the limit, and the margin
/// (positive clear, negative short).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Margin {
    pub z: Option<f64>,
    pub limit: f64,
    pub margin: Option<f64>,
}

/// Each crossing against `line`.
pub fn margins(line: &Vertical, crossings: &[Crossing]) -> Vec<Margin> {
    crossings
        .iter()
        .map(|c| {
            let z = line.at(c.station).map(|r| r.0);
            let limit = c.limit();
            let margin = z.map(|z| match c.kind {
                CrossingKind::Above => limit - z,
                CrossingKind::Below => z - limit,
            });
            Margin { z, limit, margin }
        })
        .collect()
}

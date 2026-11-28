//! What stands against the neighbour's building (docs/adr/8142; Planlı Alanlar İmar Yönetmeliği 39/4, Binaların
//! Yangından Korunması Hakkında Yönetmelik 25): along each edge of the site plan's parcel its kind says is a party edge
//! (the building stands against the neighbouring parcel's building there), on each storey, the walls, the openings and
//! the stretches with neither, each with how long it runs. The standards package reads the walls' declared classes (a
//! fire wall of 90 minutes) and the openings (none in a party wall).
//!
//! - **Read** 5 cm inside the parcel, every 25 cm along the edge and never within 30 cm of its corners (the walls of the
//!   next edges meet there), as a separation between two rooms is read (docs/adr/8081): an opening whose outline holds
//!   the point (a door, a window, a gap; a niche or a flue is not one), else a wall whose body holds it, else nothing.
//!   The change between two readings is found to a tenth of a millimetre: a window's length is its width.
//! - **A storey** is reported when something stands on the edge there; a storey with nothing on it is not attached there.
//! - **In ayrık nizam** a party edge is a side garden (the site plan says the footprint's distance to it): nothing is read
//!   along it.

use std::collections::BTreeMap;

use kentos_geometry_core::vec2::Vec2;

use crate::Document;
use crate::separations::{At, CORNER, element, spans};

/// How far inside the parcel the edge is read, metres: within a wall drawn to the boundary.
const INSIDE: f64 = 0.05;

/// What stands along a party edge on a storey, metres along it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartyWall {
    /// The edge's place in the parcel's boundary, from 1, as the site plan says it.
    pub edge: usize,
    pub level: Option<String>,
    /// The walls along it, by persistent id, each with its length there.
    pub walls: Vec<(String, f64)>,
    /// The openings along it (a door, a window, a gap), by persistent id, each with its length there.
    pub openings: Vec<(String, f64)>,
    /// Its length with neither a wall nor an opening.
    pub open: f64,
}

impl Document {
    /// What stands along each party edge of the site plan's parcel, on each storey where something does (docs/adr/8142),
    /// in the edges' order then the storeys' order of their names; none without a site plan, without a party edge, or in
    /// ayrık nizam.
    pub fn party_walls(&self) -> Vec<PartyWall> {
        let Some(arch) = self.settings().arch.as_ref() else {
            return Vec::new();
        };
        if arch.layout == Some(kentos_contracts::BuildingLayout::Detached)
            || !arch.has_party_edges()
        {
            return Vec::new();
        }
        let Some(report) = self.site_report() else {
            return Vec::new();
        };
        let (walls, openings) = self.bodies();
        let mut levels: Vec<Option<String>> = walls.iter().map(|(_, l, _)| l.clone()).collect();
        levels.extend(openings.iter().map(|(_, l, _)| l.clone()));
        levels.sort();
        levels.dedup();
        // The parcel's corners as it was drawn: its inside is on the left when they run anticlockwise.
        let ring = &report.parcel;
        let twice: f64 = 2.0 * kentos_geometry_core::geometry::ring_area(ring);
        let left = if twice >= 0.0 { 1.0 } else { -1.0 };
        let mut out = Vec::new();
        for (i, e) in report.edges.iter().enumerate() {
            if e.kind != kentos_contracts::SiteEdgeKind::Party {
                continue;
            }
            let (a, b) = (e.a, e.b);
            let len = (b.x - a.x).hypot(b.y - a.y);
            if len <= 2.0 * CORNER {
                continue;
            }
            let (dx, dy) = ((b.x - a.x) / len, (b.y - a.y) / len);
            let inward = Vec2::new(-dy * left, dx * left);
            for level in &levels {
                let mut read = |s: f64| -> At {
                    let p = Vec2::new(
                        a.x + dx * s + inward.x * INSIDE,
                        a.y + dy * s + inward.y * INSIDE,
                    );
                    element(&walls, &openings, level, p)
                };
                let mut found: BTreeMap<At, f64> = BTreeMap::new();
                for (at, length) in spans(CORNER, len - CORNER, &mut read) {
                    *found.entry(at).or_default() += length;
                }
                if found.keys().all(|k| *k == At::Open) {
                    continue;
                }
                let mut wall = PartyWall {
                    edge: i + 1,
                    level: level.clone(),
                    ..Default::default()
                };
                for (at, length) in found {
                    match at {
                        At::Wall(uid) => wall.walls.push((uid, length)),
                        At::Opening(uid) => wall.openings.push((uid, length)),
                        At::Open => wall.open += length,
                    }
                }
                out.push(wall);
            }
        }
        out
    }
}

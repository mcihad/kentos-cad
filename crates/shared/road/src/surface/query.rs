//! A place's height on a surface (ADR 0900 §8): its triangle's plane, nothing outside or in a void (ROAD-TIN-11).

use serde::Serialize;

use super::{Kind, Surface};

/// Where a place is on a surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Place {
    /// On a kept triangle (or its edge).
    Inside,
    /// Outside the points' hull, the boundary or the edge method's cut.
    Outside,
    /// In a void.
    Void,
}

/// A place's height: none outside and in a void; the kept triangle it is on.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Height {
    pub z: Option<f64>,
    pub place: Place,
    /// Its index in [`Surface::triangles`].
    pub triangle: Option<u32>,
}

impl Surface {
    /// The height at (x, y).
    pub fn height(&self, x: f64, y: f64) -> Height {
        self.height_from(x, y, &mut 0)
    }

    /// The heights of many places, the walk of each starting where the last ended (a section's, a profile's
    /// places are near each other).
    pub fn heights(&self, places: &[[f64; 2]]) -> Vec<Height> {
        let mut start = 0;
        places
            .iter()
            .map(|p| self.height_from(p[0], p[1], &mut start))
            .collect()
    }

    fn height_from(&self, x: f64, y: f64, start: &mut u32) -> Height {
        let none = |place| Height {
            z: None,
            place,
            triangle: None,
        };
        if !(x.is_finite() && y.is_finite()) {
            return none(Place::Outside);
        }
        let (xy, z) = (&self.xy[..], &self.z[..]);
        let Some(t) = self.mesh.locate(xy, [x, y], *start) else {
            return none(Place::Outside);
        };
        *start = t;
        match self.kinds[t as usize] {
            Kind::Kept => Height {
                z: Some(self.mesh.interpolate(xy, z, t, [x, y])),
                place: Place::Inside,
                triangle: Some(self.kept_index[t as usize]),
            },
            Kind::Void => none(Place::Void),
            Kind::Outside | Kind::Trimmed => none(Place::Outside),
        }
    }
}

//! From a place to its address (docs/adr/0229 §8): the nearest door within
//! the reach, else the nearest street's line, and the neighbourhood,
//! district and province areas the place is in (their edges too; the first
//! in the drawing's order). Distances are the project's units; the reach and
//! the answer's distance are metres.

use super::format::label;
use super::locator::{Level, Locator, Texts};
use crate::vec2::Vec2;

/// What the address of a place was found from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlaceKind {
    /// Kapı.
    Door,
    /// Yol.
    Street,
    /// Bölge: only an area holds it.
    Area,
}

impl PlaceKind {
    pub fn word(self) -> &'static str {
        match self {
            PlaceKind::Door => "Kapı",
            PlaceKind::Street => "Yol",
            PlaceKind::Area => "Bölge",
        }
    }
}

/// A place's address.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub kind: PlaceKind,
    /// The door, the nearest point of the street's line, or the place asked about.
    pub at: Vec2,
    /// Metres to the door or the line; none for an area alone.
    pub distance: Option<f64>,
    pub texts: Texts,
    pub label: String,
}

impl Locator {
    /// The address of `p` within `reach` metres (§8); none when nothing is near and no area holds it.
    pub fn reverse(&self, p: Vec2, reach: f64) -> Option<Place> {
        let units = reach * self.per_metre;
        let door = self.nearest_door(p, units);
        let line = if door.is_none() {
            self.nearest_line(p, units)
        } else {
            None
        };
        let hood = self.area_at(Level::Neighbourhood, p);
        let district = self.area_at(Level::District, p);
        let province = self.area_at(Level::Province, p);
        if door.is_none()
            && line.is_none()
            && hood.is_none()
            && district.is_none()
            && province.is_none()
        {
            return None;
        }
        let first = |a: &str, b: &str| {
            if a.is_empty() {
                b.to_owned()
            } else {
                a.to_owned()
            }
        };
        let hood_name = hood.map_or("", |a| a.name_text.as_str());
        let hood_district = hood.map_or("", |a| a.district_text.as_str());
        let district_name = district.map_or("", |a| a.name_text.as_str());
        let province_name = province.map_or("", |a| a.name_text.as_str());
        let (kind, at, distance, mut texts) = match (door, line) {
            (Some((i, d)), _) => {
                let door = &self.doors[i];
                (
                    PlaceKind::Door,
                    door.at,
                    Some(d / self.per_metre),
                    door.texts.clone(),
                )
            }
            (None, Some((i, d, q))) => {
                let l = &self.lines[i];
                (
                    PlaceKind::Street,
                    q,
                    Some(d / self.per_metre),
                    l.texts.clone(),
                )
            }
            (None, None) => (PlaceKind::Area, p, None, Texts::default()),
        };
        texts.neighbourhood = first(&texts.neighbourhood, hood_name);
        texts.district = first(&first(&texts.district, hood_district), district_name);
        texts.province = first(&texts.province, province_name);
        Some(Place {
            kind,
            at,
            distance,
            label: label(&texts),
            texts,
        })
    }
}

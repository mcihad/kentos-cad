//! Kütle (docs/adr/8178): a massing study's volume, a prism over its footprint from its bottom to its top: what a design
//! is weighed by before its walls and floors are drawn (its storeys' gross areas, its emsal and taban alanı against the
//! parcel). Never an element of the building: the IFC does not write it (EK-5's classes have none for it), the
//! building's audits, quantities and joins do not see it.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::ArchInfo;
use crate::entity::EntityBase;
use crate::space::{MAX_SPACE_POINTS, SpacePart, ring_problem_of};

/// A mass's height, metres, at most: a 300 m tower.
pub const MAX_MASS_HEIGHT: f64 = 300.0;
/// Its bottom over its level's floor, metres, either way at most.
pub const MAX_MASS_OFFSET: f64 = 100.0;
/// Its name's characters, at most.
pub const MAX_MASS_NAME: usize = 80;

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// A mass's own fields.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MassFields {
    /// Its footprint: one part or more, each an outer ring and its holes (a courtyard).
    pub parts: Vec<SpacePart>,
    /// Its height, metres: above 0, at most 300.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: f64,
    /// Its bottom above its level's elevation, metres (negative: below it); at most 100 either way. Absent: 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub offset: f64,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArchInfo>", optional))]
    pub arch: ArchInfo,
    /// Its name ("Blok A", "Podyum"); absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
}

/// A mass: its base and its own fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct MassEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub mass: Box<MassFields>,
}

impl MassFields {
    /// Its bottom and top over `floor` (its level's elevation), metres.
    pub fn span(&self, floor: f64) -> (f64, f64) {
        let bottom = floor + self.offset;
        (bottom, bottom + self.height)
    }

    /// What is wrong with it, the field and the words; none when nothing is.
    pub fn problem(&self) -> Option<(String, String)> {
        if self.parts.is_empty() {
            return Some((
                "parts".into(),
                "Kütlenin oturumu yok: en az bir halka verin.".into(),
            ));
        }
        let mut total = 0;
        for (i, p) in self.parts.iter().enumerate() {
            if let Some(found) = ring_problem_of(&p.outer, &format!("parts[{i}].outer"), "Kütlenin")
            {
                return Some(found);
            }
            total += p.outer.pts.len();
            for (k, h) in p.holes.iter().enumerate() {
                if let Some(found) =
                    ring_problem_of(h, &format!("parts[{i}].holes[{k}]"), "Kütlenin")
                {
                    return Some(found);
                }
                total += h.pts.len();
            }
        }
        if total > MAX_SPACE_POINTS {
            return Some((
                "parts".into(),
                format!("Kütlenin halkalarında {total} köşe var; en çok {MAX_SPACE_POINTS}."),
            ));
        }
        if !(self.height.is_finite() && self.height > 0.0 && self.height <= MAX_MASS_HEIGHT) {
            return Some((
                "height".into(),
                format!(
                    "Kütlenin yüksekliği sıfırdan büyük, en çok {MAX_MASS_HEIGHT} m olmalı; {} verildi.",
                    self.height
                ),
            ));
        }
        if !(self.offset.is_finite() && self.offset.abs() <= MAX_MASS_OFFSET) {
            return Some((
                "offset".into(),
                format!(
                    "Kütlenin kata göre tabanı sonlu, en çok ±{MAX_MASS_OFFSET} m olmalı; {} verildi.",
                    self.offset
                ),
            ));
        }
        if let Some(name) = self.name.as_deref()
            && (name.trim().is_empty()
                || name.trim() != name
                || name.chars().any(char::is_control)
                || name.chars().count() > MAX_MASS_NAME)
        {
            return Some((
                "name".into(),
                format!(
                    "Kütlenin adı boş, kenarı boşluklu ya da denetim karakterli olamaz; en çok {MAX_MASS_NAME} karakter."
                ),
            ));
        }
        None
    }
}

impl MassEntity {
    /// Its rings as one polygon object (picking, exports, anchors).
    pub fn as_polygon(&self) -> crate::PathEntity {
        crate::space::parts_polygon(&self.base, &self.mass.parts)
    }
}

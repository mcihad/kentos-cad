//! A beam (kiriş, docs/adr/3041 §1): its axis, its web's width and its height, how it lies on its axis, its top. A
//! plan draws it dashed and narrow, as what lies over the cut (ISO 128-23 02.1); from 1:50 its number and size
//! ("K101 25/50") beside it, as a construction plan writes them.
//!
//! - Its height is its whole depth, the slab within it: TBDY 2018 7.4.1.1 (b) measures a beam's height against the
//!   slab's thickness, so the slab is part of it.
//! - Its top is where it hangs: the structure's top of the slab over its storey (docs/adr/3040 §2), else its storey's
//!   top; its offset moves it from there. A wall under it stops at its underside.
//! - A lintel or a bond beam (docs/adr/3041 §3) is one too, of its own role: it sits in a wall at its base over its
//!   storey's floor (a lintel at its opening's head), and the wall runs on round it.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::entity::{EntityBase, Vec2};
use crate::{ArchInfo, WallJustify};

/// The most vertices a beam's axis holds.
pub const MAX_BEAM_POINTS: usize = 1000;
/// The widest a beam's web is, metres.
pub const MAX_BEAM_WIDTH: f64 = 3.0;
/// The deepest a beam is, metres.
pub const MAX_BEAM_HEIGHT: f64 = 5.0;
/// How far a beam's top may move from where it hangs, metres.
pub const MAX_BEAM_OFFSET: f64 = 10.0;
/// The longest a beam's number or material is, in characters.
pub const MAX_BEAM_TEXT: usize = 120;

fn is_center(j: &WallJustify) -> bool {
    *j == WallJustify::Center
}

/// What a beam is (docs/adr/3041 §3): a beam of the frame (kiriş), a lintel over an opening (lento, TBDY 2018 11.5.3) or
/// a bond beam in a wall (hatıl, TBDY 2018 7.12, 11.3.7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BeamRole {
    #[default]
    Beam,
    Lintel,
    BondBeam,
}

impl BeamRole {
    pub const ALL: [BeamRole; 3] = [BeamRole::Beam, BeamRole::Lintel, BeamRole::BondBeam];

    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            BeamRole::Beam => "Kiriş",
            BeamRole::Lintel => "Lento",
            BeamRole::BondBeam => "Hatıl",
        }
    }

    fn is_beam(&self) -> bool {
        *self == BeamRole::Beam
    }
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// A beam's own fields (docs/adr/3041 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BeamFields {
    /// Its axis: straight stretches from point to point, at least two points.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// Its web's width (gövde genişliği), metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// Its height (kiriş yüksekliği), metres: its top to its underside, the slab within it (TBDY 2018 7.4.1.1 b).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: f64,
    /// How it lies on its axis: its middle (absent), its left face or its right face, as a wall's.
    #[serde(default, skip_serializing_if = "is_center")]
    #[cfg_attr(feature = "ts", ts(as = "Option<WallJustify>", optional))]
    pub justify: WallJustify,
    /// Its top over where it hangs (the structure's top of the slab over its storey, else its storey's top), metres;
    /// absent: 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub offset: f64,
    /// What it is (docs/adr/3041 §3): a beam (absent), a lintel or a bond beam.
    #[serde(default, skip_serializing_if = "BeamRole::is_beam")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BeamRole>", optional))]
    pub role: BeamRole,
    /// Its underside's height over its storey's floor, metres (docs/adr/3041 §3): a lintel's at its opening's head;
    /// absent: it hangs from where its top is (a beam's). Not with an offset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub base_offset: Option<f64>,
    /// Its number ("K101").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tag: Option<String>,
    /// Its material; absent: Betonarme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArchInfo>", optional))]
    pub arch: ArchInfo,
}

impl BeamFields {
    /// Its material's name: its own, else Betonarme.
    pub fn material_name(&self) -> &str {
        self.material.as_deref().unwrap_or("Betonarme")
    }

    /// Its size as a construction plan writes it beside its number: its width over its height in centimetres
    /// ("25/50").
    pub fn size_text(&self) -> String {
        let cm = |m: f64| {
            let c = (m * 1000.0).round() / 10.0;
            if c.fract() == 0.0 {
                format!("{c:.0}")
            } else {
                format!("{c:.1}")
            }
        };
        format!("{}/{}", cm(self.width), cm(self.height))
    }

    /// The length of its axis, metres.
    pub fn length(&self) -> f64 {
        self.pts
            .windows(2)
            .map(|p| ((p[1].x - p[0].x).powi(2) + (p[1].y - p[0].y).powi(2)).sqrt())
            .sum()
    }

    /// What is wrong with the beam, said so a reader can fix it: the field and the words; none: it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        if self.pts.len() < 2 || self.pts.len() > MAX_BEAM_POINTS {
            return Some((
                "pts".into(),
                format!(
                    "Kirişin ekseni en az iki, en çok {MAX_BEAM_POINTS} noktadır; {} verildi.",
                    self.pts.len()
                ),
            ));
        }
        if self
            .pts
            .iter()
            .any(|p| !(p.x.is_finite() && p.y.is_finite()))
        {
            return Some((
                "pts".into(),
                "Kirişin ekseninin noktaları sonlu olmalı.".into(),
            ));
        }
        if self.pts.windows(2).any(|p| p[0] == p[1]) {
            return Some((
                "pts".into(),
                "Kirişin ekseninde art arda iki aynı nokta var; birini kaldırın.".into(),
            ));
        }
        if !(self.width.is_finite() && self.width > 0.0 && self.width <= MAX_BEAM_WIDTH) {
            return Some((
                "width".into(),
                format!(
                    "Kirişin genişliği {}; sıfırdan büyük, en çok {MAX_BEAM_WIDTH} m olmalı.",
                    self.width
                ),
            ));
        }
        if !(self.height.is_finite() && self.height > 0.0 && self.height <= MAX_BEAM_HEIGHT) {
            return Some((
                "height".into(),
                format!(
                    "Kirişin yüksekliği {}; sıfırdan büyük, en çok {MAX_BEAM_HEIGHT} m olmalı.",
                    self.height
                ),
            ));
        }
        if let Some(b) = self.base_offset {
            if !(b.is_finite() && b.abs() <= MAX_BEAM_OFFSET) {
                return Some((
                    "baseOffset".into(),
                    format!("Kirişin kattan yüksekliği {b}; en çok ±{MAX_BEAM_OFFSET} m olmalı."),
                ));
            }
            if self.offset != 0.0 {
                return Some((
                    "offset".into(),
                    "Kattan yüksekliği olan kirişin kot farkı olmaz: biri ya da öbürü verilir.".into(),
                ));
            }
        }
        if !(self.offset.is_finite() && self.offset.abs() <= MAX_BEAM_OFFSET) {
            return Some((
                "offset".into(),
                format!(
                    "Kirişin kot farkı {}; en çok ±{MAX_BEAM_OFFSET} m olmalı.",
                    self.offset
                ),
            ));
        }
        for (name, v) in [("tag", &self.tag), ("material", &self.material)] {
            if let Some(t) = v
                && (t.trim().is_empty()
                    || t.trim() != t
                    || t.chars().any(char::is_control)
                    || t.chars().count() > MAX_BEAM_TEXT)
            {
                return Some((
                    name.into(),
                    format!(
                        "Boş, kenarı boşluklu ya da denetim karakterli metin yazılmaz; en çok {MAX_BEAM_TEXT} karakter."
                    ),
                ));
            }
        }
        None
    }
}

/// Whether `e` is a beam with a role or a base of its own (docs/adr/3041 §3): what `.kcad` schema 224 adds.
pub fn needs_beam_roles(e: &crate::Entity) -> bool {
    matches!(e, crate::Entity::Beam(b) if b.beam.role != BeamRole::Beam || b.beam.base_offset.is_some())
}

/// A beam (docs/adr/3041 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BeamEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub beam: BeamFields,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beam() -> BeamFields {
        BeamFields {
            pts: vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 5.0, y: 0.0 }],
            width: 0.25,
            height: 0.5,
            ..Default::default()
        }
    }

    #[test]
    fn a_beam_says_its_size_and_what_is_wrong_with_it() {
        let b = beam();
        assert_eq!(b.problem(), None);
        assert_eq!(b.size_text(), "25/50");
        assert_eq!(b.material_name(), "Betonarme");
        assert_eq!(b.length(), 5.0);
        let odd = BeamFields {
            width: 0.225,
            ..beam()
        };
        assert_eq!(odd.size_text(), "22.5/50");
        for (field, b) in [
            (
                "pts",
                BeamFields {
                    pts: vec![Vec2 { x: 0.0, y: 0.0 }],
                    ..beam()
                },
            ),
            (
                "width",
                BeamFields {
                    width: 0.0,
                    ..beam()
                },
            ),
            (
                "height",
                BeamFields {
                    height: 6.0,
                    ..beam()
                },
            ),
            (
                "offset",
                BeamFields {
                    offset: f64::NAN,
                    ..beam()
                },
            ),
            (
                "tag",
                BeamFields {
                    tag: Some(" K1".into()),
                    ..beam()
                },
            ),
        ] {
            assert_eq!(b.problem().map(|p| p.0), Some(field.to_owned()), "{b:?}");
        }
        // A lintel sits at its base; an offset beside it is refused (docs/adr/3041 §3).
        let lintel = BeamFields {
            role: BeamRole::Lintel,
            base_offset: Some(2.1),
            height: 0.2,
            ..beam()
        };
        assert_eq!(lintel.problem(), None);
        assert_eq!(
            BeamFields { offset: 0.1, ..lintel.clone() }.problem().map(|p| p.0),
            Some("offset".to_owned())
        );
        assert_eq!(
            serde_json::to_value(&lintel).unwrap()["role"],
            serde_json::json!("lintel")
        );
        assert_eq!(BeamRole::BondBeam.label(), "Hatıl");
        // A beam's JSON: the defaults left out.
        let json = serde_json::to_value(beam()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"pts": [{"x": 0.0, "y": 0.0}, {"x": 5.0, "y": 0.0}], "width": 0.25, "height": 0.5})
        );
    }
}

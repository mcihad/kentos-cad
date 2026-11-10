//! A roof (çatı, docs/adr/8044 §1): its eaves line (saçak çizgisi) and, for each edge, the slope the roof rises at from
//! it (eğim, per cent) or none, a gable end (kalkan), as Revit's roof by footprint marks each edge "defines slope". Its
//! faces, ridges, hips and valleys follow (docs/adr/8044 §2): every edge sloping is a hip roof (kırma), two opposite
//! ones a gable roof (beşik), one a shed (tek eğimli), none a flat roof (düz).
//!
//! - Its eaves stand at its storey's top (the next storey's floor) and its offset; without a storey at its offset over
//!   ±0.00.
//! - Its overhang (saçak) is how far its eaves were drawn past the walls' outer faces: PAİY 42 allows 1.00 m.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::ArchInfo;
use crate::entity::{EntityBase, Vec2};

/// The most points a roof's eaves line holds.
pub const MAX_ROOF_POINTS: usize = 1000;
/// The steepest slope a roof's edge takes, per cent (a 79° pitch).
pub const MAX_ROOF_SLOPE: f64 = 500.0;
/// The thickest a roof's build-up is, metres.
pub const MAX_ROOF_THICKNESS: f64 = 2.0;
/// How far a roof's eaves may stand from its storey's top, metres.
pub const MAX_ROOF_OFFSET: f64 = 10.0;
/// The widest overhang a roof's eaves take, metres.
pub const MAX_ROOF_OVERHANG: f64 = 5.0;
/// The longest a roof's material is, in characters.
pub const MAX_ROOF_TEXT: usize = 120;
/// A roof's build-up when none is given, metres: tiles on battens, boarding and rafters.
pub const DEFAULT_ROOF_THICKNESS: f64 = 0.25;

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// A roof's own fields (docs/adr/8044 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoofFields {
    /// Its eaves line, closed: at least three points, either way round.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// Each edge's slope from its eaves, per cent (rise ÷ run × 100), the edge from `pts[i]` to the next point (the last
    /// back to the first); none: a gable end (kalkan). As many as the points.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub slopes: Vec<Option<f64>>,
    /// Its eaves' height over its storey's top (the next storey's floor), else over ±0.00, metres; absent: 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub offset: f64,
    /// Its build-up's thickness (kaplama, çıta, tahta, mertek), metres, measured upright; absent: 0.25.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub thickness: Option<f64>,
    /// How far its eaves were drawn past the walls' outer faces (saçak), metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub overhang: Option<f64>,
    /// Its covering (çatı kaplaması); absent: Kiremit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArchInfo>", optional))]
    pub arch: ArchInfo,
}

impl RoofFields {
    /// Its covering's name: its own, else Kiremit.
    pub fn material_name(&self) -> &str {
        self.material.as_deref().unwrap_or("Kiremit")
    }

    /// Its build-up's thickness, metres.
    pub fn thickness_m(&self) -> f64 {
        self.thickness.unwrap_or(DEFAULT_ROOF_THICKNESS)
    }

    /// What it is by its slopes (Revit's roof by footprint): every edge sloping a hip roof, two a gable roof, one a
    /// shed, none a flat roof.
    pub fn kind_name(&self) -> &'static str {
        let n = self.slopes.iter().filter(|s| s.is_some()).count();
        match n {
            0 => "Düz çatı",
            1 => "Tek eğimli çatı",
            _ if n == self.slopes.len() => "Kırma çatı",
            2 => "Beşik çatı",
            _ => "Çatı",
        }
    }

    /// What is wrong with the roof, said so a reader can fix it: the field and the words; none: it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        let n = self.pts.len();
        if !(3..=MAX_ROOF_POINTS).contains(&n) {
            return Some((
                "pts".into(),
                format!(
                    "Çatının saçak çizgisi en az üç, en çok {MAX_ROOF_POINTS} noktadır; {n} verildi."
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
                "Çatının saçak çizgisinin noktaları sonlu olmalı.".into(),
            ));
        }
        if (0..n).any(|i| self.pts[i] == self.pts[(i + 1) % n]) {
            return Some((
                "pts".into(),
                "Çatının saçak çizgisinde art arda iki aynı nokta var; birini kaldırın.".into(),
            ));
        }
        if self.slopes.len() != n {
            return Some((
                "slopes".into(),
                format!(
                    "Çatının her kenarının bir eğimi (ya da kalkanı) olur: {n} kenar, {} eğim verildi.",
                    self.slopes.len()
                ),
            ));
        }
        if let Some(s) = self
            .slopes
            .iter()
            .flatten()
            .find(|s| !(s.is_finite() && **s > 0.0 && **s <= MAX_ROOF_SLOPE))
        {
            return Some((
                "slopes".into(),
                format!(
                    "Çatının eğimi %{s}; sıfırdan büyük, en çok %{MAX_ROOF_SLOPE} olmalı (kalkan: eğimsiz)."
                ),
            ));
        }
        if !(self.offset.is_finite() && self.offset.abs() <= MAX_ROOF_OFFSET) {
            return Some((
                "offset".into(),
                format!(
                    "Çatının kot farkı {}; en çok ±{MAX_ROOF_OFFSET} m olmalı.",
                    self.offset
                ),
            ));
        }
        if let Some(t) = self.thickness
            && !(t.is_finite() && t > 0.0 && t <= MAX_ROOF_THICKNESS)
        {
            return Some((
                "thickness".into(),
                format!(
                    "Çatının kalınlığı {t}; sıfırdan büyük, en çok {MAX_ROOF_THICKNESS} m olmalı."
                ),
            ));
        }
        if let Some(o) = self.overhang
            && !(o.is_finite() && (0.0..=MAX_ROOF_OVERHANG).contains(&o))
        {
            return Some((
                "overhang".into(),
                format!("Çatının saçağı {o}; sıfırla {MAX_ROOF_OVERHANG} m arasında olmalı."),
            ));
        }
        if let Some(t) = &self.material
            && (t.trim().is_empty()
                || t.trim() != t
                || t.chars().any(char::is_control)
                || t.chars().count() > MAX_ROOF_TEXT)
        {
            return Some((
                "material".into(),
                format!(
                    "Boş, kenarı boşluklu ya da denetim karakterli metin yazılmaz; en çok {MAX_ROOF_TEXT} karakter."
                ),
            ));
        }
        None
    }
}

/// A roof (docs/adr/8044 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoofEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub roof: RoofFields,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roof(slopes: Vec<Option<f64>>) -> RoofFields {
        RoofFields {
            pts: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 12.0, y: 0.0 },
                Vec2 { x: 12.0, y: 9.0 },
                Vec2 { x: 0.0, y: 9.0 },
            ],
            slopes,
            ..Default::default()
        }
    }

    #[test]
    fn a_roof_is_named_by_its_slopes_and_checked() {
        assert_eq!(roof(vec![Some(33.0); 4]).kind_name(), "Kırma çatı");
        assert_eq!(
            roof(vec![Some(40.0), None, Some(40.0), None]).kind_name(),
            "Beşik çatı"
        );
        assert_eq!(
            roof(vec![Some(10.0), None, None, None]).kind_name(),
            "Tek eğimli çatı"
        );
        assert_eq!(roof(vec![None; 4]).kind_name(), "Düz çatı");
        assert_eq!(roof(vec![Some(33.0); 4]).problem(), None);
        assert_eq!(
            roof(vec![Some(33.0); 3]).problem().map(|p| p.0),
            Some("slopes".into())
        );
        assert_eq!(
            roof(vec![Some(0.0), None, None, None])
                .problem()
                .map(|p| p.0),
            Some("slopes".into())
        );
        let mut r = roof(vec![Some(33.0); 4]);
        r.overhang = Some(-0.1);
        assert_eq!(r.problem().map(|p| p.0), Some("overhang".into()));
        // Its fields as JSON: camelCase, a gable end null, defaults left out.
        let v = serde_json::to_value(roof(vec![Some(40.0), None, Some(40.0), None])).unwrap();
        assert_eq!(v["slopes"], serde_json::json!([40.0, null, 40.0, null]));
        assert!(v.get("offset").is_none() && v.get("thickness").is_none());
    }
}

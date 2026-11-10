//! A roof (çatı, docs/adr/8044 §1): its eaves line (saçak çizgisi) and, for each edge, the slope the roof rises at from
//! it (eğim, per cent) or none, a gable end (kalkan), as Revit's roof by footprint marks each edge "defines slope". Its
//! faces, ridges, hips and valleys follow (docs/adr/8044 §2): every edge sloping is a hip roof (kırma), two opposite
//! ones a gable roof (beşik), one a shed (tek eğimli), none a flat roof (düz).
//!
//! - Its eaves stand at its storey's top (the next storey's floor) and its offset; without a storey at its offset over
//!   ±0.00.
//! - Its overhang (saçak) is how far its eaves were drawn past the walls' outer faces: PAİY 42 allows 1.00 m.
//! - Its parts (docs/adr/8044 §6): chimneys (duman bacası), vents (hava bacası) and roof windows (çatı penceresi), each
//!   by its middle in plan and its size; a chimney's top 0.80 m over the ridge unless given (Binaların Yangından
//!   Korunması Hakkında Yönetmelik 58/2), a vent's 0.60 m over the roof at its place (PAİY 40/5: at most that), a window
//!   in its face's plane, its depth down the slope (Sivas İmar Yönetmeliği 40: at most 1.20 × 1.50 m).
//! - Its gutter and snow guard (docs/adr/8044 §8) run along every eaves edge that slopes, where the water and the snow
//!   come off (a gable end has neither): the gutter outside the eaves by its width (Mimarlık Hizmetleri Şartnamesi
//!   14.5.3 w, x: "su toplama yerleri", "yağmur iniş boruları"), the snow guard on the roof by its distance in from
//!   the eaves (Sivas İmar Yönetmeliği 42/5: "saçak uçlarında kar siperliklerinin yapılması mecburidir").

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
/// The most parts a roof holds.
pub const MAX_ROOF_PARTS: usize = 100;
/// The largest a roof's part is across, metres.
pub const MAX_ROOF_PART: f64 = 10.0;
/// A chimney's top over the ridge when none is given, metres: Binaların Yangından Korunması Hakkında Yönetmelik 58/2
/// ("ait olduğu bina mahyasının en az 0.8 m üzerine kadar çıkarılması gerekir").
pub const CHIMNEY_OVER_RIDGE: f64 = 0.8;
/// A vent's top over the roof at its place when none is given, metres: PAİY 40/5 allows "hava bacaları … en fazla 0.60
/// metre" over the roof's planes.
pub const VENT_OVER_ROOF: f64 = 0.6;
/// The widest a roof's gutter is, metres.
pub const MAX_ROOF_GUTTER: f64 = 0.5;
/// The farthest a roof's snow guard stands in from its eaves, metres.
pub const MAX_SNOW_GUARD: f64 = 2.0;
/// A gutter's width when the tool gives one, metres: a half-round gutter of 15 cm.
pub const DEFAULT_ROOF_GUTTER: f64 = 0.15;
/// A snow guard's distance in from the eaves when the tool gives one, metres: at the eaves' tip (Sivas 42/5).
pub const DEFAULT_SNOW_GUARD: f64 = 0.3;

/// What a roof's part is (docs/adr/8044 §6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RoofPartKind {
    /// Duman bacası: its top 0.80 m over the ridge unless given.
    #[default]
    Chimney,
    /// Hava bacası: its top 0.60 m over the roof at its place unless given.
    Vent,
    /// Çatı penceresi: in its face's plane, turned with its face's slope.
    Window,
}

impl RoofPartKind {
    /// Its name in Turkish, as the drawing and Öznitelikler say it.
    pub fn label(self) -> &'static str {
        match self {
            RoofPartKind::Chimney => "Duman bacası",
            RoofPartKind::Vent => "Hava bacası",
            RoofPartKind::Window => "Çatı penceresi",
        }
    }

    /// Its name in the genitive: "Duman bacasının", "Çatı penceresinin".
    pub fn whose(self) -> &'static str {
        match self {
            RoofPartKind::Chimney => "Duman bacasının",
            RoofPartKind::Vent => "Hava bacasının",
            RoofPartKind::Window => "Çatı penceresinin",
        }
    }
}

/// A part of a roof (docs/adr/8044 §6): a chimney, a vent or a roof window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoofPart {
    pub kind: RoofPartKind,
    /// Its middle in plan.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// Its width, metres: a window's along its face's eaves (across the slope), a chimney's or a vent's along its turn.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// Its depth, metres: a window's down its face's slope, in the roof's plane (Sivas's 1.50 m is measured so); a
    /// chimney's or a vent's across its width.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: f64,
    /// A chimney's or a vent's turn in plan, radians counter-clockwise; absent: 0. A window takes its face's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: Option<f64>,
    /// A chimney's top over the roof's ridge, a vent's over the roof at its highest corner, metres; absent: 0.80, 0.60.
    /// A window has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rise: Option<f64>,
}

impl RoofPart {
    /// Its top's rise: its own, else its kind's (a window's none).
    pub fn rise_m(&self) -> Option<f64> {
        match self.kind {
            RoofPartKind::Chimney => Some(self.rise.unwrap_or(CHIMNEY_OVER_RIDGE)),
            RoofPartKind::Vent => Some(self.rise.unwrap_or(VENT_OVER_ROOF)),
            RoofPartKind::Window => None,
        }
    }
}

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
    /// Its gutter (oluk, docs/adr/8044 §8): its width outside every sloping eaves edge, metres; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub gutter: Option<f64>,
    /// Its snow guard (kar siperliği, docs/adr/8044 §8): its distance in from every sloping eaves edge, metres; absent:
    /// none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub snow_guard: Option<f64>,
    /// Its chimneys, vents and roof windows (docs/adr/8044 §6).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<RoofPart>>"))]
    pub parts: Vec<RoofPart>,
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
        if let Some(g) = self.gutter
            && !(g.is_finite() && g > 0.0 && g <= MAX_ROOF_GUTTER)
        {
            return Some((
                "gutter".into(),
                format!("Çatının oluğu {g}; sıfırdan büyük, en çok {MAX_ROOF_GUTTER} m olmalı."),
            ));
        }
        if let Some(d) = self.snow_guard
            && !(d.is_finite() && (0.0..=MAX_SNOW_GUARD).contains(&d))
        {
            return Some((
                "snowGuard".into(),
                format!(
                    "Kar siperliğinin saçaktan uzaklığı {d}; sıfırla {MAX_SNOW_GUARD} m arasında olmalı."
                ),
            ));
        }
        if self.parts.len() > MAX_ROOF_PARTS {
            return Some((
                "parts".into(),
                format!(
                    "Bir çatıda en çok {MAX_ROOF_PARTS} baca ya da pencere olur; {} verildi.",
                    self.parts.len()
                ),
            ));
        }
        for (i, part) in self.parts.iter().enumerate() {
            let at = |field: &str| format!("parts[{i}].{field}");
            if !(part.at.x.is_finite() && part.at.y.is_finite()) {
                return Some((
                    at("at"),
                    format!("{} yeri sonlu olmalı.", part.kind.whose()),
                ));
            }
            for (v, field, words) in [
                (part.width, "width", "eni"),
                (part.depth, "depth", "derinliği"),
            ] {
                if !(v.is_finite() && v > 0.0 && v <= MAX_ROOF_PART) {
                    return Some((
                        at(field),
                        format!(
                            "{} {words} {v}; sıfırdan büyük, en çok {MAX_ROOF_PART} m olmalı.",
                            part.kind.whose()
                        ),
                    ));
                }
            }
            if let Some(r) = part.rotation
                && !r.is_finite()
            {
                return Some((at("rotation"), "Dönüş sonlu olmalı.".into()));
            }
            match (part.kind, part.rise) {
                (RoofPartKind::Window, Some(_)) => {
                    return Some((
                        at("rise"),
                        "Çatı penceresi çatının düzlemindedir; yüksekliği verilmez.".into(),
                    ));
                }
                (_, Some(r)) if !(r.is_finite() && r.abs() <= MAX_ROOF_OFFSET) => {
                    return Some((
                        at("rise"),
                        format!(
                            "{} yüksekliği {r}; en çok ±{MAX_ROOF_OFFSET} m olmalı.",
                            part.kind.whose()
                        ),
                    ));
                }
                _ => {}
            }
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

/// Whether `e` is a roof with parts: a file of schema 238 holds them (docs/adr/8044 §6).
pub fn needs_roof_parts(e: &crate::Entity) -> bool {
    matches!(e, crate::Entity::Roof(r) if !r.roof.parts.is_empty())
}

/// Whether `e` is a roof with a gutter or a snow guard: a file of schema 239 holds them (docs/adr/8044 §8).
pub fn needs_roof_edges(e: &crate::Entity) -> bool {
    matches!(e, crate::Entity::Roof(r) if r.roof.gutter.is_some() || r.roof.snow_guard.is_some())
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

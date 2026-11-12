//! A railing (korkuluk, docs/adr/8045 §3): its line, the middle of its top rail (küpeşte), open or closed round a
//! gallery's void; its height over its floor, its top rail's width, its infill and its offset. A plan draws its top rail
//! as two thin lines its width apart, as a balcony's railing (docs/adr/0722).
//!
//! - PAİY 38/1 (RG 3.7.2017/30113): "balkon, teras, galeri boşluğu, sahanlıkların boş kenarları … döşeme kotundan
//!   itibaren en az 1.10 metre yüksekliğe kadar … korkuluk yapılması mecburidir": 1.10 m when none is given.
//! - PAİY 38/4: "Boşluklarda, yük altındaki deformasyonlar da dâhil, en fazla 0.10 metre çapında geçişe izin verilir":
//!   a balustered railing's clear gap is 0.10 m when none is given; a glass or solid infill has none.
//! - Its floor is its storey's, moved by its offset (a landing's or a mezzanine's edge).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::ArchInfo;
use crate::entity::{EntityBase, Vec2};

/// The most vertices a railing's line holds.
pub const MAX_RAILING_POINTS: usize = 1000;
/// The highest railing, metres.
pub const MAX_RAILING_HEIGHT: f64 = 3.0;
/// The widest top rail, metres.
pub const MAX_RAILING_WIDTH: f64 = 0.5;
/// The widest clear gap between a railing's balusters, metres.
pub const MAX_RAILING_GAP: f64 = 1.0;
/// How far a railing's floor may stand from its storey's, metres, either way.
pub const MAX_RAILING_OFFSET: f64 = 10.0;
/// A railing's height when none is given, metres: PAİY 38/1's least.
pub const RAILING_HEIGHT: f64 = 1.1;
/// A balustered railing's clear gap when none is given, metres: PAİY 38/4's most.
pub const RAILING_GAP: f64 = 0.1;

/// What fills a railing under its top rail (docs/adr/8045 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RailingInfill {
    /// Dikey çubuk: upright balusters, their clear gap `gap`.
    #[default]
    Balusters,
    /// Cam: a glass panel.
    Glass,
    /// Dolu: a solid panel (sheet, board).
    Solid,
}

impl RailingInfill {
    pub const ALL: [RailingInfill; 3] = [
        RailingInfill::Balusters,
        RailingInfill::Glass,
        RailingInfill::Solid,
    ];

    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            RailingInfill::Balusters => "Dikey çubuk",
            RailingInfill::Glass => "Cam",
            RailingInfill::Solid => "Dolu",
        }
    }

    /// Its name in JSON.
    pub fn name(self) -> &'static str {
        match self {
            RailingInfill::Balusters => "balusters",
            RailingInfill::Glass => "glass",
            RailingInfill::Solid => "solid",
        }
    }

    /// EK-4 Tablo 4.1's third field for its layer (MM-KORK-CUBUK).
    pub fn layer_word(self) -> &'static str {
        match self {
            RailingInfill::Balusters => "CUBUK",
            RailingInfill::Glass => "CAM",
            RailingInfill::Solid => "DOLU",
        }
    }

    fn is_balusters(&self) -> bool {
        *self == RailingInfill::Balusters
    }
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// A railing's own fields (docs/adr/8045 §3).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RailingFields {
    /// Its line, the middle of its top rail: straight stretches from point to point, at least two points (three when
    /// closed).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// Its line runs on from its last point back to its first: round a gallery's void.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub closed: bool,
    /// Its top over its floor, metres (PAİY 38/1: at least 1.10).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: f64,
    /// Its top rail's width (küpeşte), metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// What fills it under its top rail: upright balusters (absent), glass or a solid panel.
    #[serde(default, skip_serializing_if = "RailingInfill::is_balusters")]
    #[cfg_attr(feature = "ts", ts(as = "Option<RailingInfill>", optional))]
    pub infill: RailingInfill,
    /// The clear gap between its balusters, metres; absent: 0.10 (PAİY 38/4's most). Read with balusters alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub gap: Option<f64>,
    /// Its floor over its storey's, metres; absent: 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub offset: f64,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArchInfo>", optional))]
    pub arch: ArchInfo,
}

impl RailingFields {
    /// The clear gap between its balusters: its own, else PAİY 38/4's 0.10 m; none with glass or a solid panel.
    pub fn baluster_gap(&self) -> Option<f64> {
        (self.infill == RailingInfill::Balusters).then(|| self.gap.unwrap_or(RAILING_GAP))
    }

    /// The length of its line, metres, round to its first point when closed.
    pub fn length(&self) -> f64 {
        let d = |a: &Vec2, b: &Vec2| ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        let open: f64 = self.pts.windows(2).map(|p| d(&p[0], &p[1])).sum();
        match (self.closed, self.pts.first(), self.pts.last()) {
            (true, Some(a), Some(b)) => open + d(b, a),
            _ => open,
        }
    }

    /// What is wrong with the railing, said so a reader can fix it: the field and the words; none: it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        let least = if self.closed { 3 } else { 2 };
        if self.pts.len() < least || self.pts.len() > MAX_RAILING_POINTS {
            return Some((
                "pts".into(),
                format!(
                    "Korkuluğun çizgisi {} en az {least}, en çok {MAX_RAILING_POINTS} noktadır; {} verildi.",
                    if self.closed {
                        "kapalıyken"
                    } else {
                        "açıkken"
                    },
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
                "Korkuluğun çizgisinin noktaları sonlu olmalı.".into(),
            ));
        }
        if self.pts.windows(2).any(|p| p[0] == p[1])
            || (self.closed && self.pts.first() == self.pts.last())
        {
            return Some((
                "pts".into(),
                "Korkuluğun çizgisinde art arda iki aynı nokta var; birini kaldırın (kapalı korkuluğun ilk noktası sonda \
                 yinelenmez)."
                    .into(),
            ));
        }
        if !(self.height.is_finite() && self.height > 0.0 && self.height <= MAX_RAILING_HEIGHT) {
            return Some((
                "height".into(),
                format!(
                    "Korkuluğun yüksekliği {}; sıfırdan büyük, en çok {MAX_RAILING_HEIGHT} m olmalı.",
                    self.height
                ),
            ));
        }
        if !(self.width.is_finite() && self.width > 0.0 && self.width <= MAX_RAILING_WIDTH) {
            return Some((
                "width".into(),
                format!(
                    "Korkuluğun küpeşte eni {}; sıfırdan büyük, en çok {MAX_RAILING_WIDTH} m olmalı.",
                    self.width
                ),
            ));
        }
        if let Some(g) = self.gap
            && !(g.is_finite() && g > 0.0 && g <= MAX_RAILING_GAP)
        {
            return Some((
                "gap".into(),
                format!(
                    "Korkuluğun çubuk aralığı {g}; sıfırdan büyük, en çok {MAX_RAILING_GAP} m olmalı."
                ),
            ));
        }
        if !(self.offset.is_finite() && self.offset.abs() <= MAX_RAILING_OFFSET) {
            return Some((
                "offset".into(),
                format!(
                    "Korkuluğun kot farkı {}; en çok ±{MAX_RAILING_OFFSET} m olmalı.",
                    self.offset
                ),
            ));
        }
        None
    }
}

/// A railing (docs/adr/8045 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RailingEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub railing: RailingFields,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn railing() -> RailingFields {
        RailingFields {
            pts: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 4.0, y: 0.0 },
                Vec2 { x: 4.0, y: 3.0 },
            ],
            height: RAILING_HEIGHT,
            width: crate::RAILING_WIDTH,
            ..Default::default()
        }
    }

    #[test]
    fn a_railing_says_its_length_its_gap_and_what_is_wrong_with_it() {
        let r = railing();
        assert_eq!(r.problem(), None);
        assert_eq!(r.length(), 7.0);
        assert_eq!(r.baluster_gap(), Some(0.1));
        let closed = RailingFields {
            closed: true,
            ..railing()
        };
        assert_eq!(closed.length(), 12.0);
        let glass = RailingFields {
            infill: RailingInfill::Glass,
            gap: Some(0.12),
            ..railing()
        };
        assert_eq!(glass.baluster_gap(), None);
        for (field, r) in [
            (
                "pts",
                RailingFields {
                    pts: vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 1.0, y: 0.0 }],
                    closed: true,
                    ..railing()
                },
            ),
            (
                "pts",
                RailingFields {
                    pts: vec![
                        Vec2 { x: 0.0, y: 0.0 },
                        Vec2 { x: 1.0, y: 0.0 },
                        Vec2 { x: 1.0, y: 1.0 },
                        Vec2 { x: 0.0, y: 0.0 },
                    ],
                    closed: true,
                    ..railing()
                },
            ),
            (
                "height",
                RailingFields {
                    height: 0.0,
                    ..railing()
                },
            ),
            (
                "width",
                RailingFields {
                    width: 0.6,
                    ..railing()
                },
            ),
            (
                "gap",
                RailingFields {
                    gap: Some(-0.1),
                    ..railing()
                },
            ),
            (
                "offset",
                RailingFields {
                    offset: f64::INFINITY,
                    ..railing()
                },
            ),
        ] {
            assert_eq!(r.problem().map(|p| p.0), Some(field.to_owned()), "{r:?}");
        }
        assert_eq!(RailingInfill::Glass.label(), "Cam");
        // A railing's JSON: the defaults left out.
        let json = serde_json::to_value(railing()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"pts": [{"x": 0.0, "y": 0.0}, {"x": 4.0, "y": 0.0}, {"x": 4.0, "y": 3.0}],
                               "height": 1.1, "width": 0.05})
        );
        let json = serde_json::to_value(RailingFields {
            closed: true,
            infill: RailingInfill::Solid,
            ..railing()
        })
        .unwrap();
        assert_eq!(
            (json["closed"].clone(), json["infill"].clone()),
            (serde_json::json!(true), serde_json::json!("solid"))
        );
    }
}

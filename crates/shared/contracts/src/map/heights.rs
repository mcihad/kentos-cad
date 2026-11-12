//! What a project's elevations and coordinates rest on beyond the horizontal system (docs/adr/0519 §1): whether a
//! drawing's elevations are ellipsoidal or orthometric heights, the vertical system and the geoid model they rest on,
//! and the coordinate epoch of a dynamic system. Kept with the project's settings (`heights`, `coordinateEpoch`;
//! `.kcad` schema 85) and given to `cad.crs.transform`; none of them moves a coordinate by itself. The vertical
//! systems KentOS knows are [`VERTICAL_SYSTEMS`], from EPSG's registry (the independent reference
//! `scripts/fixtures/crs_vertical_cases.py` checks them against PROJ's database).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// What a drawing's elevations are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HeightKind {
    /// Heights above the horizontal datum's ellipsoid (a GNSS receiver's): they move with the datum's shift.
    Ellipsoidal,
    /// Heights above a geoid or a mean sea level (levelling's): a horizontal datum's shift leaves them; between them
    /// and ellipsoidal heights stands a geoid model.
    Orthometric,
    /// Not said: what a call says to mean that a side's heights are not known, and what `cad.settings.set` takes to
    /// forget the project's. A project never keeps it (its settings leave `heights` out instead).
    Unknown,
}

/// A geoid model by its name and, when the grid file is at hand, the SHA-256 of its bytes (lower-case hex).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeoidModel {
    /// “TG-20”, “EGM2008 (2.5′)”.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sha256: Option<String>,
}

/// What a drawing's elevations rest on: their kind, and for orthometric heights the vertical system (one of
/// [`VERTICAL_SYSTEMS`] by its EPSG code) and the geoid model. An ellipsoidal height names neither: its ellipsoid is
/// the horizontal system's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HeightReference {
    pub kind: HeightKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub vertical_srid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geoid: Option<GeoidModel>,
}

/// A vertical system of EPSG's registry: its code, name, datum (what its heights are measured from), unit and the
/// metres of one unit, where it is used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerticalSystem {
    pub srid: u32,
    pub name: &'static str,
    pub datum: &'static str,
    /// “m”, “ft”.
    pub unit: &'static str,
    pub metres: f64,
    /// EPSG's area of use: its name and box (west, south, east, north; degrees).
    pub area: (&'static str, [f64; 4]),
}

const WORLD: (&str, [f64; 4]) = ("World.", [-180.0, -90.0, 180.0, 90.0]);

/// The vertical systems a project may name, as EPSG's registry has them.
pub const VERTICAL_SYSTEMS: &[VerticalSystem] = &[
    VerticalSystem {
        srid: 5775,
        name: "Antalya height",
        datum: "Antalya",
        unit: "m",
        metres: 1.0,
        area: ("Türkiye (Turkey) - onshore.", [25.62, 35.81, 44.83, 42.15]),
    },
    VerticalSystem {
        srid: 3855,
        name: "EGM2008 height",
        datum: "EGM2008 geoid",
        unit: "m",
        metres: 1.0,
        area: WORLD,
    },
    VerticalSystem {
        srid: 5773,
        name: "EGM96 height",
        datum: "EGM96 geoid",
        unit: "m",
        metres: 1.0,
        area: WORLD,
    },
    VerticalSystem {
        srid: 5714,
        name: "MSL height",
        datum: "Mean Sea Level",
        unit: "m",
        metres: 1.0,
        area: WORLD,
    },
    VerticalSystem {
        srid: 8050,
        name: "MSL height (ft)",
        datum: "Mean Sea Level",
        unit: "ft",
        metres: 0.3048,
        area: WORLD,
    },
    VerticalSystem {
        srid: 5735,
        name: "Black Sea height",
        datum: "Black Sea",
        unit: "m",
        metres: 1.0,
        area: ("Georgia - onshore.", [39.99, 41.04, 46.72, 43.59]),
    },
];

/// The vertical system of an EPSG code, if KentOS knows it.
pub fn vertical_system(srid: u32) -> Option<&'static VerticalSystem> {
    VERTICAL_SYSTEMS.iter().find(|v| v.srid == srid)
}

/// The earliest and latest coordinate epoch a project may name (decimal years).
pub const MIN_EPOCH: f64 = 1900.0;
pub const MAX_EPOCH: f64 = 2100.0;

/// What is wrong with a coordinate epoch, or none.
pub fn epoch_problem(epoch: f64) -> Option<String> {
    (!(epoch.is_finite() && (MIN_EPOCH..=MAX_EPOCH).contains(&epoch)))
        .then(|| format!("koordinat epoku {MIN_EPOCH} ile {MAX_EPOCH} arasında ondalık bir yıl olmalı (2023.5 gibi)"))
}

fn sha256_text(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl HeightReference {
    /// What is wrong with the reference, or none; the path names the field.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        if self.kind != HeightKind::Orthometric
            && (self.vertical_srid.is_some() || self.geoid.is_some())
        {
            return Some((
                "kind",
                "düşey sistem ve geoit modeli yalnız ortometrik yüksekliğin olur; elipsoit yüksekliğinin elipsoidi yatay sistemindir".into(),
            ));
        }
        if let Some(srid) = self.vertical_srid
            && vertical_system(srid).is_none()
        {
            return Some((
                "verticalSrid",
                format!(
                    "EPSG:{srid} KentOS'un bildiği düşey sistemlerden değil (Antalya, EGM2008, EGM96, MSL, Karadeniz)"
                ),
            ));
        }
        if let Some(g) = &self.geoid {
            if g.name.trim().is_empty() || g.name.trim() != g.name || g.name.chars().count() > 128 {
                return Some((
                    "geoid.name",
                    "geoit modelinin adı kırpılmış, boş olmayan ve en çok 128 harf olmalı".into(),
                ));
            }
            if g.sha256.as_deref().is_some_and(|s| !sha256_text(s)) {
                return Some((
                    "geoid.sha256",
                    "geoit dosyasının özeti küçük harfli 64 onaltılık rakamla SHA-256 olmalı"
                        .into(),
                ));
            }
        }
        None
    }

    /// The vertical system it names.
    pub fn vertical(&self) -> Option<&'static VerticalSystem> {
        self.vertical_srid.and_then(vertical_system)
    }

    /// The metres of one unit of its heights: the vertical system's, metres otherwise.
    pub fn metres(&self) -> f64 {
        self.vertical().map_or(1.0, |v| v.metres)
    }

    /// As a sentence names it: “ortometrik, Antalya height (EPSG:5775), geoit TG-20”, “elipsoit yüksekliği”.
    pub fn title(&self) -> String {
        match self.kind {
            HeightKind::Ellipsoidal => "elipsoit yüksekliği".into(),
            HeightKind::Unknown => "bilinmiyor".into(),
            HeightKind::Orthometric => {
                let mut t = String::from("ortometrik");
                match self.vertical() {
                    Some(v) => t.push_str(&format!(", {} (EPSG:{})", v.name, v.srid)),
                    None => t.push_str(", düşey sistemi belirtilmemiş"),
                }
                if let Some(g) = &self.geoid {
                    t.push_str(&format!(", geoit {}", g.name));
                }
                t
            }
        }
    }
}

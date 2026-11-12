//! The terrain surfaces' definitions: their sources, edits, exclusions and point clouds. A part of the road project's settings (docs/adr/8054 §9).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[allow(unused_imports)]
use super::*;

impl SurfaceSources {
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty() && self.uids.is_empty()
    }
}

/// A terrain surface's definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainSurfaceDef {
    /// Its id, unique in the project; what a profile or a section names.
    pub id: String,
    /// Its name (“Mevcut zemin”), unique in the project.
    pub name: String,
    /// The points: each point object's places with an elevation.
    #[serde(default, skip_serializing_if = "SurfaceSources::is_empty")]
    pub points: SurfaceSources,
    /// The breaklines: each line's and polyline's path, every vertex with an elevation.
    #[serde(default, skip_serializing_if = "SurfaceSources::is_empty")]
    pub breaklines: SurfaceSources,
    /// The outer boundary: a closed polyline's or an area's id; its vertices take the surface's heights.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundary: Option<String>,
    /// The voids: closed polylines' or areas' ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub voids: Vec<String>,
    /// Two records at one place are one vertex when their heights are this close, m; absent: 0.001.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: Option<f64>,
    /// The edge method: an outer triangle whose outer edge is longer than this is left out, m; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub max_edge: Option<f64>,
    /// The digest (64 lower-case hex digits) of the sources it was last made and accepted from; absent: never.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub built: Option<String>,
    /// The ground points taken from point clouds into its point layers (docs/adr/0928 §2), each layer's record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SurfaceCloudDef>>", optional))]
    pub clouds: Vec<SurfaceCloudDef>,
    /// The point records left out of it (docs/adr/0916 §6; a repair: a spike, a shot twice): each place, exactly as the
    /// record's; the drawing's points stay as they are.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SurfaceExclusion>>", optional))]
    pub excluded: Vec<SurfaceExclusion>,
    /// The edits made on it, in their order (docs/adr/1560; `.kcad` schema 196): a point added, a point's height set,
    /// an edge put in between two corners, a triangle taken out. Made again with the surface from its sources: an
    /// edit that no longer fits is said back, not dropped.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SurfaceEdit>>", optional))]
    pub edits: Vec<SurfaceEdit>,
}

/// An edit of a terrain surface (docs/adr/1560), as Civil 3D's surface definition keeps them: Nokta ekle (a point at
/// a place no record has), Kot değiştir (every point record at the place takes the height), Kenar ekle (a TIN edge
/// between the corners at two places; Kenar çevir is the other diagonal of two triangles) and Üçgen sil (the kept
/// triangle at the place goes).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "op", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SurfaceEdit {
    #[serde(rename_all = "camelCase")]
    AddPoint {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        x: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        y: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
        z: f64,
    },
    #[serde(rename_all = "camelCase")]
    SetHeight {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        x: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        y: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
        z: f64,
    },
    #[serde(rename_all = "camelCase")]
    AddEdge {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        ax: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        ay: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        bx: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        by: f64,
    },
    #[serde(rename_all = "camelCase")]
    RemoveTriangle {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        x: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        y: f64,
    },
    /// Yüzey yapıştır (docs/adr/3920; `.kcad` schema 214): another surface of the project, by its id, pasted in as Civil
    /// 3D's Paste Surface and OpenRoads' Merge do: where its kept triangles lie, this surface's records made so far go
    /// and its corners and edges come in; the rest is triangulated anew round it.
    #[serde(rename_all = "camelCase")]
    Paste { terrain: String },
}

/// The most edits one surface keeps.
pub const MOST_SURFACE_EDITS: usize = 10_000;

impl SurfaceEdit {
    /// Why it cannot be kept (a number not finite, an edge of no length), in the user's words; none when it can.
    pub fn problem(&self) -> Option<&'static str> {
        let finite = |v: &[f64]| v.iter().all(|x| x.is_finite());
        match self {
            Self::AddPoint { x, y, z } | Self::SetHeight { x, y, z } if !finite(&[*x, *y, *z]) => {
                Some("düzenlemenin yeri ve kotu sonlu sayılar olmalı")
            }
            Self::AddEdge { ax, ay, bx, by } if !finite(&[*ax, *ay, *bx, *by]) => {
                Some("kenarın iki ucu sonlu sayılar olmalı")
            }
            Self::AddEdge { ax, ay, bx, by } if ax == bx && ay == by => Some("kenarın iki ucu aynı yerde"),
            Self::RemoveTriangle { x, y } if !finite(&[*x, *y]) => Some("silinen üçgenin yeri sonlu iki sayı olmalı"),
            Self::Paste { terrain } if terrain.trim().is_empty() => Some("yapıştırılacak yüzeyin kimliği boş"),
            _ => None,
        }
    }
}

/// A point record left out of a surface: its place (docs/adr/0916 §6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurfaceExclusion {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
}

/// The most point records one surface leaves out.
pub const MOST_SURFACE_EXCLUSIONS: usize = 100_000;

/// Which of a pulse's returns a cloud's points are taken from (docs/adr/0928): all, the first, the last (nearest the
/// ground), or a pulse's only return.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CloudReturns {
    #[default]
    All,
    First,
    Last,
    Single,
}

/// What a surface's points taken from point clouds are (docs/adr/0928 §2): the layer they were written to, the clouds,
/// classes, returns and area they were chosen by, the thinning, and what it cost: the ground points read and kept,
/// their coverage (0–1) and density (per m²) of the area, and the vertical error of the points left out against the
/// surface made then (absent when none was measured), and how many were outside it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SurfaceCloudDef {
    pub layer: String,
    pub clouds: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub classes: Vec<u8>,
    #[serde(default)]
    pub returns: CloudReturns,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub area: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cell: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub z_tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub near: Option<f64>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kept: u64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub coverage: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub density: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub error_max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub error_rms: Option<f64>,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    #[cfg_attr(feature = "ts", ts(type = "number", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u64,
    /// The clouds' vertical system as their files name it (docs/adr/1029; schema 195): a WKT's `VERT_CS`/`VERTCRS`
    /// name or `EPSG:n` (a GeoTIFF key); absent: not known (a file naming none, sources naming different ones, or a
    /// record made before the rule).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vertical: Option<String>,
    /// The clouds' heights as the call said them and the project's they were written in (docs/adr/1032; schema 196);
    /// absent: the call said none, the points' heights are the files'.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub heights: Option<CloudHeights>,
}

/// What the ground taken from clouds rests on when the call says the clouds' heights (docs/adr/1032): `from`, the
/// clouds' as said; `to`, the project's the points were written in. Where the two differ the heights went from one to
/// the other by the geoid models (H = h − N), as `cad.crs.transform` turns them; where they are one, unchanged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CloudHeights {
    pub from: crate::HeightReference,
    pub to: crate::HeightReference,
}

pub(super) fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

/// The most cloud records a surface keeps, and the most clouds and classes one names.
pub const MAX_SURFACE_CLOUDS: usize = 64;
/// The most characters of a cloud record's vertical system as its file names it (docs/adr/1029).
pub const MAX_CLOUD_VERTICAL_CHARS: usize = 200;

/// What is wrong with a surface's cloud records (docs/adr/0928 §2): a record names its layer once and at least one
/// cloud and class; the thinning's values are positive; the counts agree and the shares are shares.
pub(super) fn clouds_problem(list: &[SurfaceCloudDef]) -> Option<String> {
    if list.len() > MAX_SURFACE_CLOUDS {
        return Some(format!("en çok {MAX_SURFACE_CLOUDS} bulut kaydı olur"));
    }
    let mut layers = std::collections::BTreeSet::new();
    for (k, c) in list.iter().enumerate() {
        let at = |what: &str| format!("{}. bulut kaydının {what}", k + 1);
        if !word_ok(&c.layer) || !layers.insert(c.layer.as_str()) {
            return Some(at("katmanı bir katmanın kimliği olmalı ve bir kez geçmeli"));
        }
        if c.clouds.is_empty() || c.clouds.len() > MAX_SURFACE_CLOUDS || c.clouds.iter().any(|u| !word_ok(u)) {
            return Some(at("bulutları 1 ile 64 arası nesne kimliği olmalı"));
        }
        if c.classes.is_empty() || c.classes.len() > 256 {
            return Some(at("sınıfları en az bir sınıf olmalı"));
        }
        if c.area.as_deref().is_some_and(|a| !word_ok(a)) {
            return Some(at("alanı bir nesnenin kimliği olmalı"));
        }
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !positive(c.cell) || c.z_tolerance.is_some_and(|v| !positive(v)) || c.near.is_some_and(|v| !positive(v)) {
            return Some(at("hücresi, kot toleransı ve kırık hat erimi sıfırdan büyük olmalı"));
        }
        if c.kept > c.read
            || !(0.0..=1.0).contains(&c.coverage)
            || !(c.density.is_finite() && c.density >= 0.0)
            || [c.error_max, c.error_rms].iter().flatten().any(|v| !(v.is_finite() && *v >= 0.0))
        {
            return Some(at("sayıları tutarlı olmalı (kalan okunandan çok değil, kapsama 0–1)"));
        }
        if let Some((side, why)) = c.heights.as_ref().and_then(|h| {
            [("from", &h.from), ("to", &h.to)].into_iter().find_map(|(side, r)| {
                if r.kind == crate::HeightKind::Unknown {
                    return Some((side, "bilinen bir yükseklik olmalı".to_owned()));
                }
                r.problem().map(|(_, why)| (side, why))
            })
        }) {
            return Some(at(&format!("kotlarının {side} tarafı: {why}")));
        }
        if c.vertical.as_deref().is_some_and(|v| {
            v.is_empty() || v.trim() != v || v.chars().count() > MAX_CLOUD_VERTICAL_CHARS || v.chars().any(char::is_control)
        }) {
            return Some(at("düşey sistemi boş olmayan, kenarında boşluk olmayan, en çok 200 harf olmalı"));
        }
    }
    None
}

pub(super) fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

pub(super) fn is_true(v: &bool) -> bool {
    *v
}

pub(super) fn yes() -> bool {
    true
}

pub(super) fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

/// A PI of an alignment (docs/adr/0901 §2): its persistent id, its place, and the curve on it. The first and the last
/// are the alignment's start and end and carry no curve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AlignmentPi {
    /// Unique in the alignment; the elements' ids come from it (`p3/kurp`).
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub y: f64,
    /// The curve's radius; absent or 0: no curve (a kink).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
    /// The entry clothoid's length; absent or 0: none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral_in: f64,
    /// The exit clothoid's length; absent or 0: none.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub spiral_out: f64,
    /// The curve runs into the next PI's through an ove clothoid whose A is solved on the PIs (docs/adr/0979; KCAD
    /// schema 192): both curves turn the same way, of different radii, with no clothoid on the side they share.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub ove: bool,
    /// The design speed at this PI's curve (km/h), when it is not the alignment's (docs/adr/0985; KCAD schema 193):
    /// KGM's 2019 sample sheets write V=110 for one curve and V=50 for a junction's on the same road.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "km/h")))]
    pub speed: Option<f64>,
}

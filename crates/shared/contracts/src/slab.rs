//! A slab (döşeme, docs/adr/0722): a floor, a balcony or a terrace. Its outline is a space's rings (parts of an outer
//! ring and holes, docs/adr/0342 §1), drawn or found from the walls; its top stands `offset` above its level's elevation
//! and it is `thickness` deep. Along its free sides (those no wall's face runs along) it may carry an edge, a railing or
//! a parapet, whose runs the document derives from the walls on its level. Its quantities are summed unless it is
//! `excluded`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::ArchInfo;
use crate::entity::{EntityBase, Vec2};
use crate::space::{MAX_SPACE_POINTS, SpacePart, ring_problem_of};

/// A slab's thickness when none is given, metres.
pub const SLAB_THICKNESS: f64 = 0.15;
/// The thickest slab, metres.
pub const MAX_SLAB_THICKNESS: f64 = 5.0;
/// The farthest a slab's top stands from its level's elevation, metres, either way.
pub const MAX_SLAB_OFFSET: f64 = 100.0;
/// A railing's and a parapet's width when none is given, metres.
pub const RAILING_WIDTH: f64 = 0.05;
pub const PARAPET_WIDTH: f64 = 0.15;
/// The widest edge, metres.
pub const MAX_SLAB_EDGE: f64 = 2.0;
/// The most characters a slab's material has.
pub const MAX_SLAB_MATERIAL: usize = 120;

/// What a slab is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SlabRole {
    /// Döşeme: a storey's floor.
    #[default]
    Floor,
    /// Balkon.
    Balcony,
    /// Teras.
    Terrace,
}

impl SlabRole {
    pub const ALL: [SlabRole; 3] = [SlabRole::Floor, SlabRole::Balcony, SlabRole::Terrace];

    pub fn label(self) -> &'static str {
        match self {
            SlabRole::Floor => "Döşeme",
            SlabRole::Balcony => "Balkon",
            SlabRole::Terrace => "Teras",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SlabRole::Floor => "floor",
            SlabRole::Balcony => "balcony",
            SlabRole::Terrace => "terrace",
        }
    }

    pub fn is_floor(&self) -> bool {
        *self == SlabRole::Floor
    }
}

/// What runs along a slab's free sides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SlabEdgeKind {
    /// Korkuluk: two thin lines `width` apart.
    #[default]
    Railing,
    /// Parapet: a filled band `width` wide.
    Parapet,
}

impl SlabEdgeKind {
    pub const ALL: [SlabEdgeKind; 2] = [SlabEdgeKind::Railing, SlabEdgeKind::Parapet];

    pub fn label(self) -> &'static str {
        match self {
            SlabEdgeKind::Railing => "Korkuluk",
            SlabEdgeKind::Parapet => "Parapet",
        }
    }

    /// Its name as the owner of what follows (“Korkuluğun eni”).
    pub fn whose(self) -> &'static str {
        match self {
            SlabEdgeKind::Railing => "Korkuluğun",
            SlabEdgeKind::Parapet => "Parapetin",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SlabEdgeKind::Railing => "railing",
            SlabEdgeKind::Parapet => "parapet",
        }
    }

    /// Its width when none is given, metres.
    pub fn width(self) -> f64 {
        match self {
            SlabEdgeKind::Railing => RAILING_WIDTH,
            SlabEdgeKind::Parapet => PARAPET_WIDTH,
        }
    }
}

/// A run of a slab's free sides: an open polyline along its outline, one bulge per edge when an edge is an arc.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabRun {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
}

/// The edge a slab carries along its free sides: its kind, its width inward from the outline and the runs it is drawn
/// along (the document's: every edge of the outline whose middle no wall's face on the slab's level runs through).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabEdge {
    pub kind: SlabEdgeKind,
    /// Metres, above 0 and at most 2.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SlabRun>>", optional))]
    pub runs: Vec<SlabRun>,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// What a bound slab's outline comes from (docs/adr/0771 §4): the walls round a point (their outer faces, a storey's
/// floor; or a room's inner faces), or a space's rings (a floor finish, a wet room's slab).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SlabSourceKind {
    /// Duvarlar: the walls round the seed.
    #[default]
    Walls,
    /// Mahal: the space's rings.
    Space,
}

impl SlabSourceKind {
    pub fn label(self) -> &'static str {
        match self {
            SlabSourceKind::Walls => "Duvarlar",
            SlabSourceKind::Space => "Mahal",
        }
    }
}

/// Which of the walls' faces a bound slab runs along (the command's `SlabBoundary`, kept with the slab).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SlabFace {
    /// Dış yüzler.
    #[default]
    Outline,
    /// Oda: the room's inner faces.
    Room,
}

/// Why a bound slab's outline is not what its source says: the last one kept as it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SlabBreak {
    /// The walls no longer close round its seed (or the outline moved by its offset turns over).
    LostEnclosure,
    /// Its space is gone (deleted, or no longer a space).
    SourceGone,
}

impl SlabBreak {
    pub fn words(self) -> &'static str {
        match self {
            SlabBreak::LostEnclosure => {
                "Duvarlar döşemenin iç noktasının çevresini artık kapatmıyor; son sınırı korundu."
            }
            SlabBreak::SourceGone => "Döşemenin bağlı olduğu mahal yok; son sınırı korundu.",
        }
    }
}

/// A slab whose outline follows its source (Kaynağa bağlı sınır, docs/adr/0771 §4): what it comes from, the seed and
/// the face rule (walls) or the space's persistent id, the edge offset, and, the document's, the walls that bound it
/// when it was last found and why it is broken. Absent on a slab: its outline is its own (Bağımsız kontur).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabBound {
    pub source: SlabSourceKind,
    /// The point the walls are read round (walls).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub seed: Option<Vec2>,
    #[serde(default, skip_serializing_if = "is_outline")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SlabFace>", optional))]
    pub face: SlabFace,
    /// The space, by its persistent id (space).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub space: Option<String>,
    /// Every edge moved by this much, out when positive; at most 10 m either way. Absent: 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: f64,
    /// The walls whose faces bound it when it was last found, by their persistent ids (the document's).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<SlabBreak>,
}

fn is_outline(f: &SlabFace) -> bool {
    *f == SlabFace::Outline
}

/// The farthest a bound slab's edges move, metres, either way (the command's edge offset's).
pub const MAX_BOUND_OFFSET: f64 = 10.0;

impl SlabBound {
    /// What is wrong with it: the field (relative to the slab) and the words.
    pub fn problem(&self) -> Option<(String, String)> {
        match self.source {
            SlabSourceKind::Walls if self.seed.is_none() || self.space.is_some() => {
                return Some((
                    "bound".into(),
                    "Duvarlara bağlı döşemenin iç noktası (seed) verilir, mahali verilmez.".into(),
                ));
            }
            SlabSourceKind::Space if self.space.is_none() || self.seed.is_some() => {
                return Some((
                    "bound".into(),
                    "Mahale bağlı döşemenin mahali (space) verilir, iç noktası verilmez.".into(),
                ));
            }
            _ => {}
        }
        if self.seed.is_some_and(|p| !(p.x.is_finite() && p.y.is_finite())) {
            return Some(("bound.seed".into(), "Döşemenin iç noktası sonlu olmalı.".into()));
        }
        if !(self.offset.is_finite() && self.offset.abs() <= MAX_BOUND_OFFSET) {
            return Some((
                "bound.offset".into(),
                format!("Kenar ötelemesi en çok ±{MAX_BOUND_OFFSET} m olur; {} verildi.", self.offset),
            ));
        }
        None
    }
}

/// A slab's own fields: what a command writes (the edge's runs the document's).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabFields {
    /// Its outline: one part or more, each an outer ring and its holes (a courtyard, a hole for a lowered wet room).
    pub parts: Vec<SpacePart>,
    #[serde(default, skip_serializing_if = "SlabRole::is_floor")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SlabRole>", optional))]
    pub role: SlabRole,
    /// Metres, above 0 and at most 5.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub thickness: f64,
    /// Its top above its level's elevation, metres (negative: below it); at most 100 either way. Absent: 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub offset: f64,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArchInfo>", optional))]
    pub arch: ArchInfo,
    /// What it is made of (Betonarme); absent: not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub edge: Option<SlabEdge>,
    /// Left out of the quantities' totals: listed, not summed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub excluded: bool,
    /// Kaynağa bağlı sınır (docs/adr/0771 §4): its outline found again from its source whenever that changes; absent:
    /// an outline of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bound: Option<SlabBound>,
}

/// A slab (docs/adr/0722).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub slab: SlabFields,
}

/// A text's problem as a slab's material: blank, edged with spaces, a control character, too long.
pub fn material_problem(m: &str) -> Option<String> {
    if m.trim().is_empty() || m.trim() != m || m.chars().any(char::is_control) || m.chars().count() > MAX_SLAB_MATERIAL {
        return Some(format!(
            "Döşemenin malzemesi boş, kenarı boşluklu ya da denetim karakterli olamaz; en çok {MAX_SLAB_MATERIAL} karakter."
        ));
    }
    None
}

impl SlabFields {
    /// What is wrong with the slab, said so a reader can fix it: the field and the words; none: it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        if self.parts.is_empty() {
            return Some(("parts".into(), "Döşemenin sınırı yok: en az bir halka verin.".into()));
        }
        let mut total = 0;
        for (i, p) in self.parts.iter().enumerate() {
            if let Some(found) = ring_problem_of(&p.outer, &format!("parts[{i}].outer"), "Döşemenin") {
                return Some(found);
            }
            total += p.outer.pts.len();
            for (k, h) in p.holes.iter().enumerate() {
                if let Some(found) = ring_problem_of(h, &format!("parts[{i}].holes[{k}]"), "Döşemenin") {
                    return Some(found);
                }
                total += h.pts.len();
            }
        }
        if total > MAX_SPACE_POINTS {
            return Some((
                "parts".into(),
                format!("Döşemenin halkalarında {total} köşe var; en çok {MAX_SPACE_POINTS}."),
            ));
        }
        if !(self.thickness.is_finite() && self.thickness > 0.0 && self.thickness <= MAX_SLAB_THICKNESS) {
            return Some((
                "thickness".into(),
                format!(
                    "Döşemenin kalınlığı sıfırdan büyük, en çok {MAX_SLAB_THICKNESS} m olmalı; {} verildi.",
                    self.thickness
                ),
            ));
        }
        if !(self.offset.is_finite() && self.offset.abs() <= MAX_SLAB_OFFSET) {
            return Some((
                "offset".into(),
                format!("Döşemenin kata göre kotu sonlu, en çok ±{MAX_SLAB_OFFSET} m olmalı; {} verildi.", self.offset),
            ));
        }
        if let Some(words) = self.material.as_deref().and_then(material_problem) {
            return Some(("material".into(), words));
        }
        if let Some(problem) = self.bound.as_ref().and_then(SlabBound::problem) {
            return Some(problem);
        }
        if let Some(e) = &self.edge {
            if !(e.width.is_finite() && e.width > 0.0 && e.width <= MAX_SLAB_EDGE) {
                return Some((
                    "edge.width".into(),
                    format!(
                        "{} eni sıfırdan büyük, en çok {MAX_SLAB_EDGE} m olmalı; {} verildi.",
                        e.kind.whose(),
                        e.width
                    ),
                ));
            }
            for (i, r) in e.runs.iter().enumerate() {
                let n = r.pts.len();
                if n < 2 || r.pts.iter().any(|p| !(p.x.is_finite() && p.y.is_finite())) {
                    return Some((
                        format!("edge.runs[{i}]"),
                        "Kenarın her parçası en az iki sonlu noktadır.".into(),
                    ));
                }
                if let Some(b) = &r.bulges
                    && (b.len() + 1 != n || b.iter().any(|x| !x.is_finite()))
                {
                    return Some((
                        format!("edge.runs[{i}].bulges"),
                        "Kenar parçasının her kenarının bir sonlu bükümü olmalı.".into(),
                    ));
                }
            }
        }
        None
    }
}

impl SlabEntity {
    /// Its rings as one polygon object (picking, exports, anchors).
    pub fn as_polygon(&self) -> crate::PathEntity {
        crate::space::parts_polygon(&self.base, &self.slab.parts)
    }
}

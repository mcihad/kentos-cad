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
/// The thickest layer besides a slab's structure, metres (docs/adr/8040 §2).
pub const MAX_SLAB_LAYER: f64 = 1.0;
/// The most layers over or under a slab's structure.
pub const MAX_SLAB_LAYERS: usize = 8;
/// The steepest slab, rise over run (docs/adr/8071 §6): 100 %, 45°; steeper is a wall's or a roof's.
pub const MAX_SLAB_GRADE: f64 = 1.0;

/// A sloped slab's slope arrow (docs/adr/8071 §6): its top rises evenly along the arrow, from its tail `from` (where
/// the top is the slab's `offset` over its level) to its head `to`, by `rise` metres there (negative: down); the same
/// plane runs on past both ends. Its thicknesses are measured square to its top; its sides stand upright on its outline.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlabSlope {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rise: f64,
}

impl SlabSlope {
    /// Its arrow's length in plan, metres.
    pub fn run(&self) -> f64 {
        ((self.to.x - self.from.x).powi(2) + (self.to.y - self.from.y).powi(2)).sqrt()
    }

    /// Its grade, rise over run (0 for an arrow of no length).
    pub fn grade(&self) -> f64 {
        let run = self.run();
        if run > 0.0 { self.rise / run } else { 0.0 }
    }

    /// Its top's height over its tail's at plan point `p`, metres.
    pub fn over(&self, p: Vec2) -> f64 {
        let (dx, dy) = (self.to.x - self.from.x, self.to.y - self.from.y);
        let l2 = dx * dx + dy * dy;
        if l2 > 0.0 {
            self.rise * ((p.x - self.from.x) * dx + (p.y - self.from.y) * dy) / l2
        } else {
            0.0
        }
    }

    /// How tall upright a thickness square to its top stands: √(1 + grade²).
    pub fn upright(&self) -> f64 {
        let g = self.grade();
        (1.0 + g * g).sqrt()
    }

    /// What is wrong with it, said so a reader can fix it: the field (under `slope`) and the words; none: it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        let finite = |p: Vec2| p.x.is_finite() && p.y.is_finite();
        if !finite(self.from) || !finite(self.to) || !self.rise.is_finite() {
            return Some((
                "slope".into(),
                "Eğim okunun noktaları ve yükselişi sonlu olmalı.".into(),
            ));
        }
        if self.run() < 0.001 {
            return Some((
                "slope.to".into(),
                "Eğim okunun başı kuyruğundan en az 1 mm uzakta olmalı.".into(),
            ));
        }
        if self.rise.abs() > MAX_SLAB_OFFSET {
            return Some((
                "slope.rise".into(),
                format!(
                    "Eğimin yükselişi en çok ±{MAX_SLAB_OFFSET} m olur; {} verildi.",
                    self.rise
                ),
            ));
        }
        if self.grade().abs() > MAX_SLAB_GRADE {
            return Some((
                "slope.rise".into(),
                format!(
                    "Döşemenin eğimi en çok %{:.0} olur (45°); %{:.1} verildi. Daha dik bir yüzey duvar ya da çatıdır.",
                    MAX_SLAB_GRADE * 100.0,
                    self.grade().abs() * 100.0
                ),
            ));
        }
        None
    }
}

/// What a layer of a slab besides its structure is (docs/adr/8040 §2): TS 825's floor build-up (a storey's floor: its
/// finish, screed, insulation; a terrace's waterproofing too) and the ceiling's plaster under it. The finish is PAİY
/// 4(1)(ıı)'s "taban döşeme kaplaması", the plaster its "tavan kaplaması".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum SlabLayerKind {
    /// Kaplama.
    Finish,
    /// Şap.
    Screed,
    /// Isı yalıtımı.
    Insulation,
    /// Su yalıtımı.
    Waterproofing,
    /// Sıva.
    Plaster,
}

impl SlabLayerKind {
    pub const ALL: [SlabLayerKind; 5] = [
        SlabLayerKind::Finish,
        SlabLayerKind::Screed,
        SlabLayerKind::Insulation,
        SlabLayerKind::Waterproofing,
        SlabLayerKind::Plaster,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SlabLayerKind::Finish => "Kaplama",
            SlabLayerKind::Screed => "Şap",
            SlabLayerKind::Insulation => "Isı yalıtımı",
            SlabLayerKind::Waterproofing => "Su yalıtımı",
            SlabLayerKind::Plaster => "Sıva",
        }
    }
}

/// A layer of a slab besides its structure (docs/adr/8040 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlabLayer {
    pub kind: SlabLayerKind,
    /// Metres, above 0 and at most 1.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub thickness: f64,
    /// What it is made of (Seramik, Laminat parke …); absent: not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
}

/// A slab's layers besides its structure (its `thickness`, docs/adr/8040 §2), each list from the top: those over the
/// structure (kaplama, şap, yalıtım) and those under it (sıva). Its top (its level's elevation and offset) is the top of
/// the first over the structure: the finished floor the storey's height is measured from (PAİY 4(1)(rr)).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlabLayers {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SlabLayer>>", optional))]
    pub over: Vec<SlabLayer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SlabLayer>>", optional))]
    pub under: Vec<SlabLayer>,
}

/// The build-ups a slab is given whole (docs/adr/8040 §2), each its name and its layers; typical thicknesses, each to
/// be changed: a storey's floor (kaplama 2, şap 5 over the concrete, sıva 2 under), one on the ground (kaplama 2, şap 5,
/// ısı yalıtımı 5, su yalıtımı 1; no ceiling under it), a terrace (TS 825's terrace build-up from the top: its finish 3,
/// su yalıtımı 1, şap as the slope's concrete 5, ısı yalıtımı 8; sıva 2 under) and the concrete alone.
pub fn slab_build_ups() -> Vec<(&'static str, SlabLayers)> {
    let layer = |kind, thickness| SlabLayer {
        kind,
        thickness,
        material: None,
    };
    use SlabLayerKind::*;
    vec![
        ("Yalnız taşıyıcı", SlabLayers::default()),
        (
            "Ara kat döşemesi",
            SlabLayers {
                over: vec![layer(Finish, 0.02), layer(Screed, 0.05)],
                under: vec![layer(Plaster, 0.02)],
            },
        ),
        (
            "Zemine oturan döşeme",
            SlabLayers {
                over: vec![
                    layer(Finish, 0.02),
                    layer(Screed, 0.05),
                    layer(Insulation, 0.05),
                    layer(Waterproofing, 0.01),
                ],
                under: Vec::new(),
            },
        ),
        (
            "Teras döşemesi",
            SlabLayers {
                over: vec![
                    layer(Finish, 0.03),
                    layer(Waterproofing, 0.01),
                    layer(Screed, 0.05),
                    layer(Insulation, 0.08),
                ],
                under: vec![layer(Plaster, 0.02)],
            },
        ),
    ]
}

impl SlabLayers {
    pub fn is_empty(&self) -> bool {
        self.over.is_empty() && self.under.is_empty()
    }

    /// The layers over the structure, metres.
    pub fn over_thickness(&self) -> f64 {
        self.over.iter().map(|l| l.thickness).sum()
    }

    /// The layers under the structure, metres.
    pub fn under_thickness(&self) -> f64 {
        self.under.iter().map(|l| l.thickness).sum()
    }

    /// What is wrong with them, said so a reader can fix it: the field and the words.
    pub fn problem(&self) -> Option<(String, String)> {
        for (side, list) in [("over", &self.over), ("under", &self.under)] {
            if list.len() > MAX_SLAB_LAYERS {
                return Some((
                    format!("layers.{side}"),
                    format!(
                        "Döşemenin bir yanında en çok {MAX_SLAB_LAYERS} katman olur; {} verildi.",
                        list.len()
                    ),
                ));
            }
            for (i, l) in list.iter().enumerate() {
                if !(l.thickness.is_finite() && l.thickness > 0.0 && l.thickness <= MAX_SLAB_LAYER)
                {
                    return Some((
                        format!("layers.{side}[{i}].thickness"),
                        format!(
                            "{} katmanının kalınlığı sıfırdan büyük, en çok {MAX_SLAB_LAYER} m olmalı; {} verildi.",
                            l.kind.label(),
                            l.thickness
                        ),
                    ));
                }
                if let Some(words) = l.material.as_deref().and_then(material_problem) {
                    return Some((format!("layers.{side}[{i}].material"), words));
                }
            }
        }
        None
    }
}

/// What a slab is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum SlabRole {
    /// Döşeme: a storey's floor.
    #[default]
    Floor,
    /// Balkon.
    Balcony,
    /// Teras.
    Terrace,
    /// Radye (docs/adr/8172 §6): a raft, the building's foundation slab; IFC's `IfcSlab` BASESLAB.
    Raft,
}

impl SlabRole {
    pub const ALL: [SlabRole; 4] = [
        SlabRole::Floor,
        SlabRole::Balcony,
        SlabRole::Terrace,
        SlabRole::Raft,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SlabRole::Floor => "Döşeme",
            SlabRole::Balcony => "Balkon",
            SlabRole::Terrace => "Teras",
            SlabRole::Raft => "Radye",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SlabRole::Floor => "floor",
            SlabRole::Balcony => "balcony",
            SlabRole::Terrace => "terrace",
            SlabRole::Raft => "raft",
        }
    }

    pub fn is_floor(&self) -> bool {
        *self == SlabRole::Floor
    }

    /// Its `IfcSlab`'s predefined type (EK-6 Tablo 6.56): a raft's BASESLAB (IFC 4.3: a slab foundation is no
    /// `IfcFooting`), every other FLOOR.
    pub fn ifc_type(self) -> &'static str {
        if self == SlabRole::Raft {
            "BASESLAB"
        } else {
            "FLOOR"
        }
    }

    /// Its IFC name's words (EK-5 §5.5): a raft's Radye, every other Döşeme.
    pub fn ifc_words(self) -> &'static str {
        if self == SlabRole::Raft {
            "Radye"
        } else {
            "Döşeme"
        }
    }
}

/// What runs along a slab's free sides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
pub enum SlabSourceKind {
    /// Duvarlar: the walls round the seed.
    #[default]
    Walls,
    /// Mahal: the space's rings.
    Space,
    /// Kenarlar: individual component faces and an independent reference perimeter.
    Edges,
}

impl SlabSourceKind {
    pub fn label(self) -> &'static str {
        match self {
            SlabSourceKind::Walls => "Duvarlar",
            SlabSourceKind::Space => "Mahal",
            SlabSourceKind::Edges => "Duvar bileşenlerine bağlı kenarlar",
        }
    }
}

/// Which of the walls' faces a bound slab runs along (the command's `SlabBoundary`, kept with the slab).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
pub enum SlabBreak {
    /// The walls no longer close round its seed (or the outline moved by its offset turns over).
    LostEnclosure,
    /// Its space is gone (deleted, or no longer a space).
    SourceGone,
    EdgeSourceGone,
    EdgeShapeInvalid,
}

impl SlabBreak {
    pub fn words(self) -> &'static str {
        match self {
            SlabBreak::LostEnclosure => {
                "Duvarlar döşemenin iç noktasının çevresini artık kapatmıyor; son sınırı korundu."
            }
            SlabBreak::SourceGone => "Döşemenin bağlı olduğu mahal yok; son sınırı korundu.",
            SlabBreak::EdgeSourceGone => {
                "Bağlı duvar, doğrusal parça veya bileşen yüzü artık bulunamıyor; son sınır korundu."
            }
            SlabBreak::EdgeShapeInvalid => {
                "Bağlı kenarlar kapalı, geçerli bir kontur oluşturmuyor; son sınır korundu."
            }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub edges: Option<SlabEdgeBindings>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlabEdgeBindings {
    pub reference: SpacePart,
    pub links: Vec<SlabEdgeLink>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlabEdgeLink {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub edge: u32,
    pub wall: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub segment: u32,
    /// Wall component boundary index from its left face. No index means the outer right face.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub face: Option<u32>,
    /// Outwards relative to the slab contour, in metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: f64,
    pub locked: bool,
}

fn is_outline(f: &SlabFace) -> bool {
    *f == SlabFace::Outline
}

/// The farthest a bound slab's edges move, metres, either way (the command's edge offset's).
pub const MAX_BOUND_OFFSET: f64 = 10.0;

impl SlabBound {
    /// What is wrong with it: the field (relative to the slab) and the words.
    pub fn problem(&self) -> Option<(String, String)> {
        if self.source == SlabSourceKind::Edges {
            let invalid = || {
                ("bound.edges".into(), "Kenar bağlantıları geçerli, tek parçalı ve doğrusal bir döşeme konturuna ait olmalı.".into())
            };
            let Some(edges) = &self.edges else {
                return Some(invalid());
            };
            let ring = &edges.reference.outer;
            let mut seen = std::collections::BTreeSet::new();
            if self.seed.is_some()
                || self.space.is_some()
                || self.offset != 0.0
                || !(3..=512).contains(&ring.pts.len())
                || ring
                    .bulges
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|v| *v != 0.0))
                || ring_problem_of(ring, "bound.edges.reference.outer", "Döşemenin").is_some()
                || edges.links.is_empty()
                || edges.links.iter().any(|l| {
                    l.edge as usize >= ring.pts.len()
                        || !seen.insert(l.edge)
                        || l.wall.trim().is_empty()
                        || !l.offset.is_finite()
                        || l.offset.abs() > MAX_BOUND_OFFSET
                })
            {
                return Some(invalid());
            }
            for (i, hole) in edges.reference.holes.iter().enumerate() {
                if let Some(failure) = ring_problem_of(
                    hole,
                    &format!("bound.edges.reference.holes[{i}]"),
                    "Döşemenin",
                ) {
                    return Some(failure);
                }
            }
        } else if self.edges.is_some() {
            return Some((
                "bound.edges".into(),
                "Tekil kenar bağları yalnız Kenarlar kaynağında verilir.".into(),
            ));
        }
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
        if self
            .seed
            .is_some_and(|p| !(p.x.is_finite() && p.y.is_finite()))
        {
            return Some((
                "bound.seed".into(),
                "Döşemenin iç noktası sonlu olmalı.".into(),
            ));
        }
        if !(self.offset.is_finite() && self.offset.abs() <= MAX_BOUND_OFFSET) {
            return Some((
                "bound.offset".into(),
                format!(
                    "Kenar ötelemesi en çok ±{MAX_BOUND_OFFSET} m olur; {} verildi.",
                    self.offset
                ),
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
    /// The project material it binds to (docs/adr/0785), by its id: its `material` is the material's name and follows
    /// it. Absent: its own text material.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material_id: Option<String>,
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
    /// Its layers besides its structure (docs/adr/8040 §2): kaplama, şap, yalıtım over it, sıva under it; absent: the
    /// structure alone, its top the finished floor.
    #[serde(default, skip_serializing_if = "SlabLayers::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SlabLayers>", optional))]
    pub layers: SlabLayers,
    /// A covering (docs/adr/8070 §10): a plate covering another element (a suspended ceiling, a floor covering drawn by
    /// hand), written as an `IfcCovering` of its type; no floor of the building. Absent: a slab.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub covering: Option<crate::Covering>,
    /// Its slope (docs/adr/8071 §6): a sloped slab rises along its arrow; absent: level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub slope: Option<SlabSlope>,
}

/// A slab (docs/adr/0722).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SlabEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub slab: Box<SlabFields>,
}

/// A text's problem as a slab's material: blank, edged with spaces, a control character, too long.
pub fn material_problem(m: &str) -> Option<String> {
    if m.trim().is_empty()
        || m.trim() != m
        || m.chars().any(char::is_control)
        || m.chars().count() > MAX_SLAB_MATERIAL
    {
        return Some(format!(
            "Döşemenin malzemesi boş, kenarı boşluklu ya da denetim karakterli olamaz; en çok {MAX_SLAB_MATERIAL} karakter."
        ));
    }
    None
}

impl SlabFields {
    /// Its whole thickness, its layers with its structure, metres (docs/adr/8040 §2).
    pub fn whole_thickness(&self) -> f64 {
        self.layers.over_thickness() + self.thickness + self.layers.under_thickness()
    }

    /// Its structure's top over `floor` (its level's elevation), metres: its top less the layers over it.
    pub fn structure_top(&self, floor: f64) -> f64 {
        floor + self.offset - self.layers.over_thickness()
    }

    /// How tall upright its thicknesses stand (docs/adr/8071 §6): √(1 + grade²) on a sloped slab, 1 on a level one.
    pub fn upright(&self) -> f64 {
        self.slope.map_or(1.0, |s| s.upright())
    }

    /// Its top over `floor` at plan point `p`, metres: its offset, and its slope's rise there.
    pub fn top_at(&self, floor: f64, p: Vec2) -> f64 {
        floor + self.offset + self.slope.map_or(0.0, |s| s.over(p))
    }

    /// Its structure's top over `floor` at plan point `p`, metres: its top there less the layers over it, upright.
    pub fn structure_top_at(&self, floor: f64, p: Vec2) -> f64 {
        self.top_at(floor, p) - self.layers.over_thickness() * self.upright()
    }

    /// Its structure's underside over `floor` at plan point `p`, metres: where a wall under it stops there.
    pub fn structure_underside_at(&self, floor: f64, p: Vec2) -> f64 {
        self.structure_top_at(floor, p) - self.thickness * self.upright()
    }

    /// Its structure's underside over `floor`, metres: where a wall under it stops (docs/adr/8040 §1, §2).
    pub fn structure_underside(&self, floor: f64) -> f64 {
        self.structure_top(floor) - self.thickness
    }

    /// The ceiling it gives the storey under it, over `floor`, metres: its structure's underside less the layers under
    /// it (PAİY 4(1)(ıı)'s "tavan kaplaması").
    pub fn ceiling(&self, floor: f64) -> f64 {
        self.structure_underside(floor) - self.layers.under_thickness()
    }

    /// What is wrong with the slab, said so a reader can fix it: the field and the words; none: it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        if self.parts.is_empty() {
            return Some((
                "parts".into(),
                "Döşemenin sınırı yok: en az bir halka verin.".into(),
            ));
        }
        let mut total = 0;
        for (i, p) in self.parts.iter().enumerate() {
            if let Some(found) =
                ring_problem_of(&p.outer, &format!("parts[{i}].outer"), "Döşemenin")
            {
                return Some(found);
            }
            total += p.outer.pts.len();
            for (k, h) in p.holes.iter().enumerate() {
                if let Some(found) =
                    ring_problem_of(h, &format!("parts[{i}].holes[{k}]"), "Döşemenin")
                {
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
        if !(self.thickness.is_finite()
            && self.thickness > 0.0
            && self.thickness <= MAX_SLAB_THICKNESS)
        {
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
                format!(
                    "Döşemenin kata göre kotu sonlu, en çok ±{MAX_SLAB_OFFSET} m olmalı; {} verildi.",
                    self.offset
                ),
            ));
        }
        if let Some(words) = self.material.as_deref().and_then(material_problem) {
            return Some(("material".into(), words));
        }
        if let Some(words) = crate::material_ref_problem(&self.material_id) {
            return Some(("materialId".into(), words));
        }
        if let Some(problem) = self.bound.as_ref().and_then(SlabBound::problem) {
            return Some(problem);
        }
        if let Some(problem) = self.layers.problem() {
            return Some(problem);
        }
        // A sloped slab (docs/adr/8071 §6): its arrow holds, and no hole (the section draws a sloped face whole).
        if let Some(s) = &self.slope {
            if let Some(problem) = s.problem() {
                return Some(problem);
            }
            if self.parts.iter().any(|p| !p.holes.is_empty()) {
                return Some((
                    "slope".into(),
                    "Delikli döşemeye eğim verilmez: deliği kaldırın ya da döşemeyi deliğin çevresinde parçalara bölün."
                        .into(),
                ));
            }
        }
        // A raft (docs/adr/8172 §6): the building's level base, no railing or parapet along it.
        if self.role == SlabRole::Raft {
            if self.slope.is_some() {
                return Some((
                    "slope".into(),
                    "Radyeye eğim verilmez: radye binanın düz tabanıdır.".into(),
                ));
            }
            if self.edge.is_some() {
                return Some((
                    "edge".into(),
                    "Radyenin kenarı (korkuluk, parapet) olmaz.".into(),
                ));
            }
        }
        // A covering is a plate of its own (docs/adr/8070 §10): no layers, no edge, no balcony, terrace or raft.
        if let Some(c) = &self.covering {
            if let Some(problem) = c.problem() {
                return Some(problem);
            }
            if !self.layers.is_empty() {
                return Some((
                    "covering".into(),
                    "Kaplamanın katmanları olmaz: kalınlığı kendisinindir; katmanları kaldırın."
                        .into(),
                ));
            }
            if self.edge.is_some() {
                return Some((
                    "covering".into(),
                    "Kaplamanın kenarı (korkuluk, parapet) olmaz.".into(),
                ));
            }
            if self.role == SlabRole::Raft {
                return Some((
                    "covering".into(),
                    "Radye kaplama olamaz; kaplamanın türü Döşeme olmalı.".into(),
                ));
            }
            if self.role != SlabRole::Floor {
                return Some((
                    "covering".into(),
                    "Balkon ve teras kaplama olamaz; kaplamanın türü Döşeme olmalı.".into(),
                ));
            }
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

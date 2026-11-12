//! The kitchen run (“Mutfak dizisi”, docs/adr/0706, 0727): `cad.casework.run`'s input (a run's recipe), its modules,
//! legs and fridge, and the drawing's record of a run (its recipe, its legs' walls and its objects, kept as
//! `casework`). The commands' plans and answers and `cad.casework.edit` and `.query` are `kentos-commands`'.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;
use crate::identity::{CaseworkId, EntityId};

/// A base cabinet's width when none is given (docs/adr/0706 §1): KentOS's default, the library's 60 cm.
pub const DEFAULT_CASEWORK_MODULE: f64 = 0.60;
/// The countertop's depth when none is given.
pub const DEFAULT_CASEWORK_DEPTH: f64 = 0.60;
/// The sink module's width: the library's 80 cm sink on a 90 cm base cabinet.
pub const CASEWORK_SINK_MODULE: f64 = 0.90;
/// An appliance's module (docs/adr/0727 §2): the dishwasher's, the oven's, the fridge's place; the cooktop's at least.
pub const CASEWORK_APPLIANCE: f64 = 0.60;
/// The most corners a run turns (a U).
pub const CASEWORK_CORNERS_MAX: usize = 2;
/// The most runs a drawing keeps.
pub const CASEWORK_RUNS_MAX: usize = 10_000;
/// The most objects a run holds.
pub const CASEWORK_MEMBERS_MAX: usize = 10_000;

/// Which side of its path a run stands on, as one walks from `from` (the room's side).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkSide {
    #[default]
    Left,
    Right,
}

/// How a run turns a corner (docs/adr/0727 §1): the leg before keeps the corner (a blind corner, a 5 cm filler after
/// it), or both legs stop a cabinet's depth from it (an open corner, its square a filler).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkCorner {
    #[default]
    Blind,
    Open,
}

/// An end of a run's path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkEnd {
    Start,
    End,
}

/// The side a single door hinges on, as one faces the cabinet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkDoor {
    Left,
    Right,
}

/// A module's own values (docs/adr/0727 §2): its width instead of the run's default, its door's hand.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkModuleSetting {
    /// The module, counted from `from` along the path (1 the first).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub module: u32,
    /// Its width, one of the cabinet block's widths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: Option<f64>,
    /// Its door's hinge; a double door (over 0.60 m) and an appliance have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hand: Option<CaseworkDoor>,
}

/// Input of `cad.casework.run` v1 (“Mutfak dizisi”, docs/adr/0706, 0727): base cabinets along the path where their
/// backs stand (a wall's face): `from`, the corners `through` (none: straight, one: an L, two: a U), `to`, on its
/// `side`; modules of `module` width numbered from `from`, the sink's module (`sink`) 0.90 m, the cooktop's
/// (`cooktop`) at least 0.60 m, the dishwasher's (`dishwasher`) and the oven under the cooktop (`oven`) 0.60 m, a
/// module's own width and door (`perModule`); the fridge at an end (`fridge`); what is left on a leg a filler. The
/// countertop covers the path but the fridge, `depth` deep, mitred at the corners, cut for the sink and the cooktop;
/// `upper` adds wall cabinets over the modules. KentOS's library blocks (Alt dolap, Evye, Ocak, Bulaşık makinesi,
/// Fırın, Buzdolabı, Üst dolap) are brought in the same step when the drawing has none of their names. The run is
/// kept as the drawing's record (`casework`): its legs tied to the wall faces they stand on, it follows them.
///
/// Refusals, in this order: `not_finite`, `invalid_casework` (too many corners, a leg with no length, a depth out of
/// its range, a corner not square or turning away from the cabinets, a module's own values given twice, a block
/// without its width parameter, a width not one of the cabinet's, a leg too short for a module, a sink, cooktop,
/// dishwasher or module's values outside the run, two of them in one module, an oven without a cooktop, an
/// appliance's width, a door given an appliance or a double door, a sink or cooktop that does not fit its module or
/// the countertop, the wall cabinet's widths), `invalid_line_weight`, `invalid_revision`, `revision_conflict`,
/// `layer_not_found`, `not_a_layer`, `layer_locked`; on the desktop also `slots_exhausted`. Warnings: `layer_hidden`,
/// `casework_filler`, `casework_blind_corner`, `casework_clash`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkRun {
    /// The layer the run goes on, a layer's id.
    pub layer_id: String,
    /// Where the run starts, on the line its backs stand on.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: Vec2,
    /// Its corners in order: one an L, two a U; absent: a straight run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub through: Option<Vec<Vec2>>,
    /// Where its path ends.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: Vec2,
    /// The side it stands on; absent: left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub side: Option<CaseworkSide>,
    /// A base cabinet's width (one of the cabinet block's widths: 0.30 to 1.20 m by 0.15); absent: 0.60.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub module: Option<f64>,
    /// The countertop's depth, from 0.30 to 1.20 m; absent: 0.60.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<f64>,
    /// How it turns its corners; absent: blind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corner: Option<CaseworkCorner>,
    /// The sink's module, counted from `from` (1 the first); absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub sink: Option<u32>,
    /// The cooktop's module; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub cooktop: Option<u32>,
    /// The dishwasher's module; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub dishwasher: Option<u32>,
    /// An oven under the cooktop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub oven: Option<bool>,
    /// The fridge at an end of the path; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fridge: Option<CaseworkEnd>,
    /// Modules' own widths and doors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub per_module: Option<Vec<CaseworkModuleSetting>>,
    /// Wall cabinets over the modules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub upper: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

impl CaseworkRun {
    /// Its path: `from`, its corners, `to`.
    pub fn path(&self) -> Vec<Vec2> {
        let mut out = vec![self.from];
        out.extend(self.through.iter().flatten().copied());
        out.push(self.to);
        out
    }

    /// The same run along `path` (its first point `from`, its last `to`, the others its corners).
    pub fn along(&self, path: &[Vec2]) -> CaseworkRun {
        let mut out = self.clone();
        if let (Some(first), Some(last)) = (path.first(), path.last()) {
            out.from = *first;
            out.to = *last;
            out.through = (path.len() > 2).then(|| path[1..path.len() - 1].to_vec());
        }
        out
    }
}

/// What a module of the run holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkModuleKind {
    Base,
    Sink,
    Cooktop,
    Dishwasher,
}

/// A module's door: single, hinged on a side, or double (wider than 0.60 m).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkHand {
    Left,
    Right,
    Double,
}

/// One module of a run, along its leg from the leg's start.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkModule {
    /// Its number, from `from` along the path.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub number: u32,
    pub kind: CaseworkModuleKind,
    /// Where it starts along its leg.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub start: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// Its leg, from 0; absent: the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub leg: Option<u32>,
    /// Its door; an appliance has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hand: Option<CaseworkHand>,
    /// The cooktop's module with an oven under it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub oven: Option<bool>,
    /// In a blind corner: the next leg's cabinets stand before part of its front (docs/adr/0727 §1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub blind: Option<bool>,
}

/// One leg of a run: its length, the span its modules and filler take, its filler and the wall it stands on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkLeg {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub start: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub end: f64,
    /// What its modules leave of its span (0: none).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub filler: f64,
    /// The wall whose face it stands on, by its persistent id; absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wall: Option<String>,
}

/// The fridge's place: along its leg, from the leg's start.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkFridge {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub start: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub leg: Option<u32>,
}

// ── The drawing's record ────────────────────────────────────────────────────

/// What an object of a run is in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkPart {
    /// A module's floor unit: a base cabinet, the dishwasher, the oven.
    Module,
    /// The sink on its module.
    Sink,
    /// The cooktop on its module.
    Cooktop,
    Fridge,
    /// A wall cabinet over a module.
    Upper,
    /// A leg's filler.
    Filler,
    /// A corner's filler.
    Corner,
    /// The countertop.
    Counter,
}

/// An object of a run: its persistent id, its part and, for modules, uppers, fillers and corners, its number (a
/// module's, a leg's from 0, a corner's from 1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkMember {
    pub uid: EntityId,
    pub part: CaseworkPart,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: Option<u32>,
}

/// A wall's face, relative to its path's direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkFace {
    Left,
    Right,
}

/// The wall face edge a leg stands on (docs/adr/0727 §5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkTie {
    pub wall: EntityId,
    /// The face's edge, from 0 along the wall's path.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub edge: u32,
    pub face: CaseworkFace,
}

/// Why a run did not follow its walls: one is gone, or the path they give lays no run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkBreak {
    WallGone,
    NotBuilt,
}

impl CaseworkBreak {
    pub fn name(self) -> &'static str {
        match self {
            Self::WallGone => "wallGone",
            Self::NotBuilt => "notBuilt",
        }
    }
}

/// A kitchen run as the drawing keeps it (docs/adr/0727 §5): its recipe (`cad.casework.run`'s input without an
/// expected revision), each leg's wall face (none: it stands free), its objects and why it no longer follows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkRecord {
    pub id: CaseworkId,
    pub recipe: CaseworkRun,
    pub ties: Vec<Option<CaseworkTie>>,
    pub members: Vec<CaseworkMember>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<CaseworkBreak>,
}

/// What is wrong with a drawing's runs, by the first one's index; none when they hold: at most
/// [`CASEWORK_RUNS_MAX`], ids neither nil nor twice, a recipe with finite numbers, at most two corners and no expected
/// revision, a tie for each leg, at most [`CASEWORK_MEMBERS_MAX`] objects, an object in one run at most.
pub fn casework_problem(runs: &[CaseworkRecord]) -> Option<(usize, String)> {
    use std::collections::BTreeSet;
    if runs.len() > CASEWORK_RUNS_MAX {
        return Some((0, "Çok fazla mutfak dizisi var.".into()));
    }
    let mut ids = BTreeSet::new();
    let mut members = BTreeSet::new();
    for (i, run) in runs.iter().enumerate() {
        let fail = |message: &str| Some((i, message.to_owned()));
        if run.id.is_nil() || !ids.insert(run.id) {
            return fail("Mutfak dizisinin kimliği boş ya da yineleniyor.");
        }
        let r = &run.recipe;
        if r.layer_id.is_empty() {
            return fail("Mutfak dizisinin katmanı boş.");
        }
        let corners = r.through.as_ref().map_or(0, Vec::len);
        if corners > CASEWORK_CORNERS_MAX {
            return fail("Mutfak dizisi en çok iki köşelidir.");
        }
        let numbers = r
            .path()
            .iter()
            .flat_map(|p| [p.x, p.y])
            .chain(r.module)
            .chain(r.depth)
            .chain(r.line_weight)
            .chain(r.per_module.iter().flatten().filter_map(|m| m.width))
            .all(f64::is_finite);
        if !numbers {
            return fail("Mutfak dizisinin sayıları sonlu olmalı.");
        }
        if r.expected_revision.is_some() {
            return fail("Mutfak dizisinin tarifi beklenen sürüm taşımaz.");
        }
        if run.ties.len() != corners + 1 {
            return fail("Mutfak dizisinin her kolunun bir duvar bağı yeri olmalı.");
        }
        if run.ties.iter().flatten().any(|t| t.wall.is_nil()) {
            return fail("Mutfak dizisinin duvar bağının kimliği boş.");
        }
        if run.members.len() > CASEWORK_MEMBERS_MAX {
            return fail("Mutfak dizisinin nesne sınırı aşıldı.");
        }
        for m in &run.members {
            if m.uid.is_nil() || !members.insert(m.uid) {
                return fail("Mutfak dizisinin nesnesi boş ya da iki dizide.");
            }
        }
    }
    None
}


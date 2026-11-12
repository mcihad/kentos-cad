//! Sheet sets, guardrails, culverts, retaining walls, platforms, junctions and the pay book. A part of the road project's settings (docs/adr/8054 §9).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[allow(unused_imports)]
use super::*;

pub const MAX_ROAD_SHEETS: usize = 500;

/// A sheet of a plan–profile set: the book's sheet, the stations it shows (internal, m) and the profile view drawn for
/// it (its frame's lower left and upper right), when the set has a profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadSheetDef {
    pub sheet: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub view: Option<[[f64; 2]; 2]>,
}

/// A plan–profile sheet set (docs/adr/0908 §10; Civil 3D's view frame group): sheets along an alignment, each showing
/// its stretch in plan at the set's scale (1:`scale`) and, with a profile, the profile view drawn for it; the sources'
/// revisions it was made from, so the sheets say when the alignment or the profile changed since.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadSheetSetDef {
    pub id: String,
    pub name: String,
    pub alignment: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub profile_revision: u32,
    /// A section sheet set's sample line set (docs/adr/0908 §11) and its revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sections: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub sections_revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: u32,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub overlap: f64,
    pub sheets: Vec<RoadSheetDef>,
}

pub(super) fn sheet_sets_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.sheet_sets;
    if list.len() > MAX_SHEET_SETS {
        return fail("sheetSets".into(), &format!("en çok {MAX_SHEET_SETS} pafta takımı olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, t) in list.iter().enumerate() {
        let at = |field: &str| format!("sheetSets[{i}].{field}");
        if !word_ok(&t.id) {
            return fail(at("id"), "pafta takımının kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(t.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir pafta takımı var");
        }
        let name = t.name.trim();
        if name.is_empty() || name != t.name || t.name.chars().count() > 120 {
            return fail(at("name"), "pafta takımının adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir pafta takımı var");
        }
        match road.alignments.iter().find(|a| a.id == t.alignment) {
            None => return fail(at("alignment"), "pafta takımının aksı projede yok"),
            Some(a) if a.offset_of.is_some() => return fail(at("alignment"), "pafta takımı paralel aksta olmaz"),
            Some(_) => {}
        }
        if let Some(p) = &t.profile
            && road.profiles.iter().all(|q| q.id != *p || q.alignment != t.alignment)
        {
            return fail(at("profile"), "pafta takımının boykesiti projede aynı aksta yok");
        }
        if let Some(k) = &t.sections
            && road.sections.iter().all(|q| q.id != *k || q.alignment != t.alignment)
        {
            return fail(at("sections"), "pafta takımının kesit takımı projede aynı aksta yok");
        }
        if !(t.from.is_finite() && t.to.is_finite() && t.from <= t.to) {
            return fail(at("to"), "aralığın başı sonundan sonra olamaz, ikisi sonlu olmalı");
        }
        if t.scale == 0 || t.scale > 1_000_000 {
            return fail(at("scale"), "ölçek 1 ile 1 000 000 arası olmalı");
        }
        if !(t.overlap.is_finite() && t.overlap >= 0.0) {
            return fail(at("overlap"), "bindirme 0 ya da büyük, sonlu olmalı");
        }
        if t.sheets.is_empty() || t.sheets.len() > MAX_ROAD_SHEETS {
            return fail(at("sheets"), &format!("pafta takımında 1 ile {MAX_ROAD_SHEETS} arası pafta olur"));
        }
        let mut sheets = std::collections::BTreeSet::new();
        for (j, sh) in t.sheets.iter().enumerate() {
            let path = at(&format!("sheets[{j}]"));
            if !word_ok(&sh.sheet) || !sheets.insert(sh.sheet.as_str()) {
                return fail(path, "paftanın kimliği tek, boş olmayan bir sözcük olmalı");
            }
            if !(sh.from.is_finite() && sh.to.is_finite() && sh.from <= sh.to) {
                return fail(path, "paftanın aralığının başı sonundan sonra olamaz");
            }
            if sh.view.is_some_and(|v| v.iter().flatten().any(|c| !c.is_finite())) {
                return fail(path, "boykesit görünümünün çerçevesi sonlu olmalı");
            }
        }
    }
    None
}

/// The most guardrails a project holds, and rows a guardrail's rule holds.
pub const MAX_GUARDRAILS: usize = 256;
pub const MAX_GUARDRAIL_WARRANTS: usize = 32;

/// A row of a guardrail rule (docs/adr/0909 §1): a barrier where the fill is at least `height` high and, with `slope`,
/// steeper than 1:`slope` (its run per rise at most that).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailWarrantDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub slope: Option<f64>,
}

/// A guardrail rule (docs/adr/0909 §1): the standard's table it comes from (`source`, required: KentOS has no number of
/// its own), its rows, the lengths added before (the approach end) and after a warranted stretch, the least gap left
/// between two runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailRuleDef {
    pub source: String,
    pub warrants: Vec<GuardrailWarrantDef>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub approach: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub leaving: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub gap: f64,
}

/// A guardrail (docs/adr/0909 §1): its corridor and the corridor's revision it was made from, its rule, the step its
/// crosses are read at, and how far outward from the hinge the rail stands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GuardrailDef {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub corridor_revision: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub rule: GuardrailRuleDef,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub offset: f64,
}

/// The most culverts a project holds, and barrels a culvert carries.
pub const MAX_CULVERTS: usize = 1000;
pub const MAX_CULVERT_BARRELS: u32 = 20;

/// A culvert's barrel cross (docs/adr/0909 §2): pipes side by side (`spacing` their clear gap) or a box of cells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CulvertShapeDef {
    Pipe {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        diameter: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        thickness: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
        #[serde(default, skip_serializing_if = "is_zero_f64")]
        #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        spacing: f64,
    },
    Box {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        width: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        height: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        thickness: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
        slab: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        cells: u32,
    },
}

/// The side a culvert's water runs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CulvertFlow {
    Right,
    Left,
}

/// A culvert across a corridor (docs/adr/0909 §2): its corridor and that corridor's revision it was made from, its
/// station and skew (degrees from the cross, positive ahead on the right), its barrel, its flow line under the axis and
/// its fall (a grade, 0.02 is 2 %; docs/adr/0977) toward `flow`, the length added beyond each toe, the least cover asked, its material and Manning
/// roughness (typed, kept).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CulvertDef {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub corridor_revision: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub station: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub skew: f64,
    pub shape: CulvertShapeDef,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub invert: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub slope: f64,
    pub flow: CulvertFlow,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub extend: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cover: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub roughness: Option<f64>,
}

pub(super) fn culverts_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.culverts;
    if list.len() > MAX_CULVERTS {
        return fail("culverts".into(), &format!("en çok {MAX_CULVERTS} menfez olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    let size = |v: f64| v.is_finite() && v > 0.0 && v <= 100.0;
    for (i, c) in list.iter().enumerate() {
        let at = |field: &str| format!("culverts[{i}].{field}");
        if !word_ok(&c.id) {
            return fail(at("id"), "menfezin kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(c.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir menfez var");
        }
        let name = c.name.trim();
        if name.is_empty() || name != c.name || c.name.chars().count() > 120 {
            return fail(at("name"), "menfezin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir menfez var");
        }
        if road.corridors.iter().all(|x| x.id != c.corridor) {
            return fail(at("corridor"), "menfezin koridoru projede yok");
        }
        if !c.station.is_finite() {
            return fail(at("station"), "menfezin km'si sonlu olmalı");
        }
        if !(c.skew.is_finite() && c.skew.abs() < 90.0) {
            return fail(at("skew"), "menfezin açısı -90° ile 90° arası olur (sınırlar dışında)");
        }
        match c.shape {
            CulvertShapeDef::Pipe { diameter, thickness, count, spacing } => {
                if !size(diameter) || !size(thickness) {
                    return fail(at("shape"), "borunun çapı ve et kalınlığı 0'dan büyük, en çok 100 m olur");
                }
                if !(1..=MAX_CULVERT_BARRELS).contains(&count) {
                    return fail(at("shape.count"), &format!("boru sayısı 1 ile {MAX_CULVERT_BARRELS} arası olur"));
                }
                if !(spacing.is_finite() && (0.0..=100.0).contains(&spacing)) {
                    return fail(at("shape.spacing"), "borular arası boşluk 0 ile 100 m arası olur");
                }
            }
            CulvertShapeDef::Box { width, height, thickness, slab, cells } => {
                if !size(width) || !size(height) || !size(thickness) || !size(slab) {
                    return fail(at("shape"), "kutunun genişliği, yüksekliği, duvar ve döşeme kalınlığı 0'dan büyük, en çok 100 m olur");
                }
                if !(1..=MAX_CULVERT_BARRELS).contains(&cells) {
                    return fail(at("shape.cells"), &format!("göz sayısı 1 ile {MAX_CULVERT_BARRELS} arası olur"));
                }
            }
        }
        if !(c.invert.is_finite() && c.invert.abs() <= 10_000.0) {
            return fail(at("invert"), "akış kotu sonlu, en çok 10 000 m olur");
        }
        if !(c.slope.is_finite() && (0.0..=1.0).contains(&c.slope)) {
            return fail(at("slope"), "akış eğimi oranla 0 ile 1 (%0–100) arası olur");
        }
        if !(c.extend.is_finite() && (0.0..=100.0).contains(&c.extend)) {
            return fail(at("extend"), "topuğun ötesine boy 0 ile 100 m arası olur");
        }
        if c.cover.is_some_and(|v| !(v.is_finite() && (0.0..=100.0).contains(&v))) {
            return fail(at("cover"), "en az örtü 0 ile 100 m arası olur");
        }
        if c.material.as_ref().is_some_and(|m| m.trim().is_empty() || m.trim() != m || m.chars().count() > 120) {
            return fail(at("material"), "malzeme boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if c.roughness.is_some_and(|n| !(n.is_finite() && n > 0.0 && n <= 1.0)) {
            return fail(at("roughness"), "Manning pürüzlülüğü 0'dan büyük, en çok 1 olur");
        }
    }
    None
}

/// The most retaining walls a project holds, and stations a wall is read at.
pub const MAX_RETAINING_WALLS: usize = 1000;
pub const MAX_RETAINING_WALL_STATIONS: usize = 5000;

/// A corridor's side a wall stands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WallSide {
    Left,
    Right,
}

/// A retaining wall's cross (docs/adr/0909 §3): the stem's top thickness and its batter (m/m), the footing's width,
/// thickness and toe, the ground over the footing and what stands above the higher of the road's edge and the ground.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallShapeDef {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub top: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub batter: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub footing: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub base: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub toe: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub cover: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub above: f64,
}

/// A retaining wall along a corridor's side (docs/adr/0909 §3): its corridor and that corridor's revision it was made
/// from, its side and stations, the step it is read at, its cross, the highest stem asked (a warning, not a refusal) and
/// its material (typed, kept).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RetainingWallDef {
    pub id: String,
    pub name: String,
    pub corridor: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub corridor_revision: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub side: WallSide,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: f64,
    pub shape: WallShapeDef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub highest: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
}

pub(super) fn walls_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.walls;
    if list.len() > MAX_RETAINING_WALLS {
        return fail("walls".into(), &format!("en çok {MAX_RETAINING_WALLS} istinat duvarı olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    let size = |v: f64| v.is_finite() && v > 0.0 && v <= 100.0;
    let room = |v: f64| v.is_finite() && (0.0..=100.0).contains(&v);
    for (i, w) in list.iter().enumerate() {
        let at = |field: &str| format!("walls[{i}].{field}");
        if !word_ok(&w.id) {
            return fail(at("id"), "duvarın kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(w.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir istinat duvarı var");
        }
        let name = w.name.trim();
        if name.is_empty() || name != w.name || w.name.chars().count() > 120 {
            return fail(at("name"), "duvarın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir istinat duvarı var");
        }
        if road.corridors.iter().all(|x| x.id != w.corridor) {
            return fail(at("corridor"), "duvarın koridoru projede yok");
        }
        if !(w.from.is_finite() && w.to.is_finite() && w.from < w.to) {
            return fail(at("to"), "duvarın bitiş km'si başlangıcından büyük olmalı");
        }
        if !(w.step.is_finite() && w.step > 0.0 && w.step <= 1000.0) {
            return fail(at("step"), "duvarın okunduğu aralık 0'dan büyük, en çok 1000 m olur");
        }
        if (w.to - w.from) / w.step > MAX_RETAINING_WALL_STATIONS as f64 {
            return fail(at("step"), &format!("duvar en çok {MAX_RETAINING_WALL_STATIONS} km'de okunur; aralığı büyütün"));
        }
        let s = &w.shape;
        if !size(s.top) || !size(s.footing) || !size(s.base) {
            return fail(at("shape"), "gövdenin üst kalınlığı, temelin genişliği ve kalınlığı 0'dan büyük, en çok 100 m olur");
        }
        if !(s.batter.is_finite() && (0.0..=10.0).contains(&s.batter)) {
            return fail(at("shape.batter"), "gövdenin eğimi 0 ile 10 arası olur (metrede yatay metre)");
        }
        if !room(s.toe) || !room(s.cover) || !room(s.above) {
            return fail(at("shape"), "temelin önü, örtü ve korkuluk payı 0 ile 100 m arası olur");
        }
        if s.toe + s.top > s.footing + 1e-9 {
            return fail(at("shape.toe"), "temelin önü ve gövdenin üst kalınlığı temelin genişliğine sığmalı");
        }
        if w.highest.is_some_and(|v| !size(v)) {
            return fail(at("highest"), "en yüksek gövde 0'dan büyük, en çok 100 m olur");
        }
        if w.material.as_ref().is_some_and(|m| m.trim().is_empty() || m.trim() != m || m.chars().count() > 120) {
            return fail(at("material"), "malzeme boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
    }
    None
}

/// The most graded pads a project holds, and corners a pad's boundary has.
pub const MAX_PLATFORMS: usize = 1000;
pub const MAX_PLATFORM_CORNERS: usize = 1000;

/// A graded pad (docs/adr/0927): its boundary's corners, the plane through `base` rising `grade` (0.02 is 2 %; docs/adr/0977) towards
/// `direction` (degrees counterclockwise from +x), its cut and fill slopes (horizontal per vertical), the spacing of
/// its slope lines along an edge, a corner's widest fan step (degrees), the longest slope line and the terrain it is
/// graded to. The slopes are the user's: no soil's default is guessed (ROAD-GRADE-02).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformDef {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub corners: Vec<[f64; 2]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub base: [f64; 3],
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "grade")))]
    pub grade: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub direction: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub cut: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub fill: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub step: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub fan: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub reach: f64,
    pub surface: String,
}

pub(super) fn platforms_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.platforms;
    if list.len() > MAX_PLATFORMS {
        return fail("platforms".into(), &format!("en çok {MAX_PLATFORMS} platform olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, p) in list.iter().enumerate() {
        let at = |field: &str| format!("platforms[{i}].{field}");
        if !word_ok(&p.id) {
            return fail(at("id"), "platformun kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(p.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir platform var");
        }
        let name = p.name.trim();
        if name.is_empty() || name != p.name || p.name.chars().count() > 120 {
            return fail(at("name"), "platformun adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir platform var");
        }
        if p.corners.len() < 3 || p.corners.len() > MAX_PLATFORM_CORNERS {
            return fail(at("corners"), &format!("platformun sınırı 3 ile {MAX_PLATFORM_CORNERS} arası köşedir"));
        }
        if !p.corners.iter().flatten().chain(p.base.iter()).all(|v| v.is_finite()) {
            return fail(at("corners"), "köşeler ve taban sonlu sayılar olmalı");
        }
        if !(p.grade.is_finite() && p.grade.abs() <= 1.0) {
            return fail(at("grade"), "platformun eğimi oranla yazılır (0,02 %2'dir), en çok 1 (%100) olur");
        }
        if !(p.direction.is_finite() && (-360.0..=360.0).contains(&p.direction)) {
            return fail(at("direction"), "eğimin yönü −360 ile 360 derece arası olur");
        }
        for (field, v, top) in [("cut", p.cut, 100.0), ("fill", p.fill, 100.0), ("step", p.step, 1000.0), ("reach", p.reach, 10_000.0)] {
            if !(v.is_finite() && v > 0.0 && v <= top) {
                return fail(at(field), &format!("0'dan büyük, en çok {top} olur"));
            }
        }
        if !(p.fan.is_finite() && p.fan > 0.0 && p.fan <= 90.0) {
            return fail(at("fan"), "köşe yelpazesinin açısı 0'dan büyük, en çok 90 derece olur");
        }
        if road.surfaces.iter().all(|s| s.id != p.surface) {
            return fail(at("surface"), "platformun arazi yüzeyi projede yok");
        }
    }
    None
}

/// The most junctions a project holds.
pub const MAX_JUNCTIONS: usize = 1000;

/// How a junction's legs meet: at grade (joined, the curb returns built) or separated (one over the other, not joined).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum JunctionKind {
    AtGrade,
    Separated,
}

/// A junction's leg: its alignment and that alignment's revision it was made from, and its edges' offsets from the
/// alignment (left and right, metres, positive).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionLegDef {
    pub alignment: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub alignment_revision: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub left: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub right: f64,
}

/// A junction of two alignments (docs/adr/0910 §2): its legs, which of their crossings (`at`, 0 the first along the
/// first leg), how they meet, and at grade the curb returns' radius (`radius`, every quadrant) and, when given, each
/// quadrant's own (`radii`, four: first leg's right with the second's right, right with left, left with left, left with
/// right).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct JunctionDef {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub kind: JunctionKind,
    pub legs: Vec<JunctionLegDef>,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub at: u32,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radius: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub radii: Option<Vec<f64>>,
}

pub(super) fn junctions_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.junctions;
    if list.len() > MAX_JUNCTIONS {
        return fail("junctions".into(), &format!("en çok {MAX_JUNCTIONS} kavşak olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    let size = |v: f64| v.is_finite() && v > 0.0 && v <= 1000.0;
    for (i, j) in list.iter().enumerate() {
        let at = |field: &str| format!("junctions[{i}].{field}");
        if !word_ok(&j.id) {
            return fail(at("id"), "kavşağın kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(j.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir kavşak var");
        }
        let name = j.name.trim();
        if name.is_empty() || name != j.name || j.name.chars().count() > 120 {
            return fail(at("name"), "kavşağın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir kavşak var");
        }
        if j.legs.len() != 2 {
            return fail(at("legs"), "kavşağın iki kolu olur");
        }
        if j.legs[0].alignment == j.legs[1].alignment {
            return fail(at("legs"), "kavşağın iki kolu ayrı akslarda olur");
        }
        for (k, leg) in j.legs.iter().enumerate() {
            match road.alignments.iter().find(|a| a.id == leg.alignment) {
                None => return fail(format!("junctions[{i}].legs[{k}].alignment"), "kolun aksı projede yok"),
                Some(a) if a.offset_of.is_some() => {
                    return fail(format!("junctions[{i}].legs[{k}].alignment"), "kolun aksı PI'larıyla kurulan bir aks olur");
                }
                _ => {}
            }
            if !(size(leg.left) && size(leg.right)) || leg.left > 100.0 || leg.right > 100.0 {
                return fail(format!("junctions[{i}].legs[{k}].left"), "kolun kenarları 0'dan büyük, en çok 100 m olur");
            }
        }
        if j.at > 64 {
            return fail(at("at"), "kesişimin sırası en çok 64 olur");
        }
        match j.kind {
            JunctionKind::AtGrade => {
                if !size(j.radius) {
                    return fail(at("radius"), "hemzemin kavşağın dönüş yarıçapı 0'dan büyük, en çok 1000 m olur");
                }
                if j.radii.as_ref().is_some_and(|r| r.len() != 4 || r.iter().any(|v| !size(*v))) {
                    return fail(at("radii"), "çeyreklerin yarıçapı dört olur, her biri 0'dan büyük, en çok 1000 m");
                }
            }
            JunctionKind::Separated => {
                if j.radius != 0.0 || j.radii.is_some() {
                    return fail(at("radius"), "katlı kavşakta dönüş yayı olmaz");
                }
            }
        }
    }
    None
}

/// The most pay items and rules a book holds.
pub const MAX_PAY_ITEMS: usize = 2000;
pub const MAX_PAY_RULES: usize = 2000;

/// A quantity KentOS reads from a road project's structures and earthwork (docs/adr/0909 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PayMeasure {
    /// A guardrail's runs, both sides (m).
    GuardrailLength,
    /// A culvert's barrel (m).
    CulvertLength,
    /// A culvert's pipes: count × barrel (m).
    CulvertPipes,
    /// A culvert's barrel concrete (m³).
    CulvertConcrete,
    /// A retaining wall's face in plan (m).
    WallLength,
    /// A retaining wall's stem face (m²).
    WallFace,
    /// A retaining wall's concrete (m³).
    WallConcrete,
    /// An earthwork volume's cut (m³).
    VolumeCut,
    /// An earthwork volume's fill (m³).
    VolumeFill,
    /// An earthwork volume's cut times its swell factors (m³).
    VolumeFactoredCut,
    /// An earthwork volume's fill times its shrink factors (m³).
    VolumeFactoredFill,
}

impl PayMeasure {
    /// Whether it reads a culvert or a wall (which have a material).
    pub fn has_material(self) -> bool {
        matches!(
            self,
            Self::CulvertLength | Self::CulvertPipes | Self::CulvertConcrete | Self::WallLength | Self::WallFace | Self::WallConcrete
        )
    }
}

/// An agency's pay item: its code (as the agency writes it), name, unit (as written) and optional unit price.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayItemDef {
    pub code: String,
    pub name: String,
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub price: Option<f64>,
}

/// A rule giving an item a quantity: KentOS's `measure` (of the culverts or walls of `material` only, when given)
/// times `factor`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayRuleDef {
    pub measure: PayMeasure,
    pub item: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub factor: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
}

/// The project's pay item book (docs/adr/0909 §4): its source (the agency's list and year, required: KentOS gives no
/// item), its version, its currency, its items and its rules.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayBookDef {
    pub source: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub currency: Option<String>,
    pub items: Vec<PayItemDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<PayRuleDef>>", optional))]
    pub rules: Vec<PayRuleDef>,
}

pub(super) fn pay_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let Some(b) = &road.pay else { return None };
    let text = |t: &str, most: usize| !t.trim().is_empty() && t.trim() == t && t.chars().count() <= most;
    if !text(&b.source, 200) {
        return fail("pay.source".into(), "poz defterinin kaynağı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 200 karakter");
    }
    if !text(&b.version, 64) {
        return fail("pay.version".into(), "poz defterinin sürümü boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 64 karakter");
    }
    if b.currency.as_ref().is_some_and(|c| !text(c, 12)) {
        return fail("pay.currency".into(), "para birimi boş olmamalı, en çok 12 karakter");
    }
    if b.items.is_empty() || b.items.len() > MAX_PAY_ITEMS {
        return fail("pay.items".into(), &format!("poz defterinde 1 ile {MAX_PAY_ITEMS} arası poz olur"));
    }
    if b.rules.len() > MAX_PAY_RULES {
        return fail("pay.rules".into(), &format!("poz defterinde en çok {MAX_PAY_RULES} kural olur"));
    }
    let mut codes = std::collections::BTreeSet::new();
    for (i, it) in b.items.iter().enumerate() {
        let at = |field: &str| format!("pay.items[{i}].{field}");
        if !text(&it.code, 64) {
            return fail(at("code"), "pozun numarası boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 64 karakter");
        }
        if !codes.insert(it.code.as_str()) {
            return fail(at("code"), "bu numarada başka bir poz var");
        }
        if !text(&it.name, 300) {
            return fail(at("name"), "pozun adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 300 karakter");
        }
        if !text(&it.unit, 16) {
            return fail(at("unit"), "pozun birimi boş olmamalı, en çok 16 karakter");
        }
        if it.price.is_some_and(|p| !(0.0..=1e12).contains(&p)) {
            return fail(at("price"), "pozun birim fiyatı 0 ya da büyük, sonlu olmalı");
        }
    }
    for (i, r) in b.rules.iter().enumerate() {
        let at = |field: &str| format!("pay.rules[{i}].{field}");
        if !codes.contains(r.item.as_str()) {
            return fail(at("item"), "kuralın pozu defterde yok");
        }
        if !(r.factor.is_finite() && r.factor > 0.0 && r.factor <= 1e6) {
            return fail(at("factor"), "kuralın çarpanı 0'dan büyük, en çok 1 000 000 olur");
        }
        if let Some(m) = &r.material {
            if !r.measure.has_material() {
                return fail(at("material"), "malzeme yalnız menfez ve istinat duvarı ölçülerinde olur");
            }
            if !text(m, 120) {
                return fail(at("material"), "kuralın malzemesi boş olmamalı, en çok 120 karakter");
            }
        }
    }
    None
}

pub(super) fn guardrails_problem(road: &RoadSettings) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_string()));
    let list = &road.guardrails;
    if list.len() > MAX_GUARDRAILS {
        return fail("guardrails".into(), &format!("en çok {MAX_GUARDRAILS} otokorkuluk olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (i, g) in list.iter().enumerate() {
        let at = |field: &str| format!("guardrails[{i}].{field}");
        if !word_ok(&g.id) {
            return fail(at("id"), "otokorkuluğun kimliği boş olmayan, en çok 64 karakterlik bir sözcük olmalı");
        }
        if !ids.insert(g.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir otokorkuluk var");
        }
        let name = g.name.trim();
        if name.is_empty() || name != g.name || g.name.chars().count() > 120 {
            return fail(at("name"), "otokorkuluğun adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter");
        }
        if !names.insert(name.to_lowercase()) {
            return fail(at("name"), "bu adda başka bir otokorkuluk var");
        }
        if road.corridors.iter().all(|c| c.id != g.corridor) {
            return fail(at("corridor"), "otokorkuluğun koridoru projede yok");
        }
        let r = &g.rule;
        let source = r.source.trim();
        if source.is_empty() || source != r.source || r.source.chars().count() > 200 {
            return fail(at("rule.source"), "kuralın kaynağı boş olmamalı (standart ve tablosu), başında ve sonunda boşluk olmamalı, en çok 200 karakter");
        }
        if r.warrants.is_empty() || r.warrants.len() > MAX_GUARDRAIL_WARRANTS {
            return fail(at("rule.warrants"), &format!("kuralın 1 ile {MAX_GUARDRAIL_WARRANTS} arası satırı olur"));
        }
        for (j, w) in r.warrants.iter().enumerate() {
            if !(w.height.is_finite() && (0.0..=1000.0).contains(&w.height)) {
                return fail(at(&format!("rule.warrants[{j}].height")), "dolgu yüksekliği 0 ile 1000 m arası olur");
            }
            if w.slope.is_some_and(|s| !(s.is_finite() && s > 0.0 && s <= 1000.0)) {
                return fail(at(&format!("rule.warrants[{j}].slope")), "şev (bir düşeye yatay) 0'dan büyük, en çok 1000 olur");
            }
        }
        for (field, v) in [("approach", r.approach), ("leaving", r.leaving), ("gap", r.gap)] {
            if !(v.is_finite() && (0.0..=10_000.0).contains(&v)) {
                return fail(at(&format!("rule.{field}")), "uzunluk 0 ile 10 000 m arası olur");
            }
        }
        if !(g.step.is_finite() && g.step > 0.0 && g.step <= 1000.0) {
            return fail(at("step"), "okuma aralığı 0'dan büyük, en çok 1000 m olur");
        }
        if !(g.offset.is_finite() && g.offset.abs() <= 100.0) {
            return fail(at("offset"), "korkuluğun şev başından uzaklığı en çok 100 m olur");
        }
    }
    None
}

//! A building's program (docs/adr/0773): its buildings and their wings, its groups of spaces — independent units
//! (bağımsız bölüm), common areas and outdoor areas — and the design brief its spaces are read against. A space joins
//! a group by the group's id (`SpaceFields::unit`); a group is not bound to one level (a duplex spans two), nor a level to
//! one group, and no layer name stands for any of it. A brief is the designer's target, never a legal limit: its
//! findings say the brief's own version and the rule they were measured by.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The rule the program's numbers come from: areas from the spaces' own rings by their own method, net width the
/// diameter of the largest circle inside a space, net passage the widest a path between its doors stays.
pub const PROGRAM_RULE: &str = "kentos.program/1";
/// The most buildings, groups, briefs and clearances a program holds, each.
pub const MAX_PROGRAM_ITEMS: usize = 1000;
/// The most usages a brief's adjacencies or reaches name.
pub const MAX_BRIEF_USAGES: usize = 32;
/// The most letters a name, number or usage holds.
pub const MAX_PROGRAM_TEXT: usize = 120;
/// How far apart two spaces' facing sides may lie and still be neighbours (a wall between), metres.
pub const ADJACENT_GAP: f64 = 0.6;
/// How long two spaces' facing sides must run along each other to be neighbours, or to open into each other, metres.
pub const ADJACENT_OVERLAP: f64 = 0.3;
/// How far apart two spaces' sides may lie and still open into each other (a room separator, no wall), metres.
pub const OPEN_GAP: f64 = 0.001;

/// A group of spaces' kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum UnitKind {
    /// Bağımsız bölüm: a flat, a shop, an office; its own spaces summed for it alone.
    #[default]
    Unit,
    /// Ortak alan: a shared corridor, stairs, a shaft; summed apart, never into a unit's total.
    Common,
    /// Dış alan: a garden, a car park; summed apart.
    Outdoor,
    /// Eklenti (docs/adr/8031; `.kcad` schema 223; Kat Mülkiyeti Kanunu 2/a, 6): a place outside an independent unit,
    /// allocated to it (a storage room, a garage, a coal room; `annexOf`); its gross counts into its unit's total gross
    /// (PAİY 4/ğ-4), never into the common areas.
    Annex,
}

impl UnitKind {
    pub const ALL: [UnitKind; 4] = [
        UnitKind::Unit,
        UnitKind::Common,
        UnitKind::Outdoor,
        UnitKind::Annex,
    ];

    pub fn label(self) -> &'static str {
        match self {
            UnitKind::Unit => "Bağımsız bölüm",
            UnitKind::Common => "Ortak alan",
            UnitKind::Outdoor => "Dış alan",
            UnitKind::Annex => "Eklenti",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            UnitKind::Unit => "unit",
            UnitKind::Common => "common",
            UnitKind::Outdoor => "outdoor",
            UnitKind::Annex => "annex",
        }
    }
}

fn is_unit(k: &UnitKind) -> bool {
    *k == UnitKind::Unit
}

/// A wing of a building.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramWing {
    pub id: String,
    pub name: String,
    /// The levels it spans (docs/adr/0784 §1), by their ids, each once; within its building's when the building names
    /// any. None: not said.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub levels: Vec<String>,
}

/// A building and its wings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramBuilding {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProgramWing>>", optional))]
    pub wings: Vec<ProgramWing>,
    /// The levels it spans (docs/adr/0784 §1), by their ids, each once; a level may be another building's too.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub levels: Vec<String>,
    /// Its building code (docs/adr/8072 §1; EK-2 §2.3, Tablo 2.1): two capital letters or digits, its model files'
    /// names carry it; "00" only for a project of one building. None: not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub code: Option<String>,
    /// What it is used for (Kullanım türü; EK-6 Tablo 6.7 `OccupancyType`), as the project writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub usage: Option<String>,
    /// Its ±0.00 over the project's ±0.00, metres (docs/adr/8072 §1): a block entered higher or lower than another;
    /// its storeys' levels are read from it. None: not said (the project's when it is the only building).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub zero: Option<f64>,
    /// Its own Bölge (docs/adr/8144; `.kcad` Mimari data 30): where the parcel's blocks stand on decisions of different
    /// uses, PAİY 28/1 reads its storeys' heights by it. None: the project's (Proje ayarları › Mimari › İmar durumu).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zone: Option<crate::Zone>,
    /// Its own structural system and traditional elements (docs/adr/8145; `.kcad` Mimari data 31): a block built
    /// otherwise than the others, PAİY 22/1-m, 28/1-d, 5/8 and 20/8-j read its storeys by it. None: the project's
    /// (Proje ayarları › Mimari › Taşıyıcı sistem).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub structure: Option<crate::BuildingStructure>,
}

/// Whether a storey's name says it is the ground storey: “Zemin” in any case (Turkish İ lower-cases to i and a dot; the
/// dot is dropped for the comparison).
pub fn is_ground_name(name: &str) -> bool {
    name.to_lowercase().replace('\u{307}', "").contains("zemin")
}

/// A level two or more buildings span (a common basement, docs/adr/8072 §1): the building it counts as a storey of, its
/// area once.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SharedLevel {
    pub level: String,
    pub building: String,
}

/// The farthest a building's ±0.00 may stand from the project's, metres.
pub const MAX_BUILDING_ZERO: f64 = 100.0;

/// A group of spaces: an independent unit, a common area or an outdoor area.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramUnit {
    pub id: String,
    /// Its number, as the project writes it (“D1”, “1”, “Ortak”); one per group.
    pub number: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "is_unit")]
    #[cfg_attr(feature = "ts", ts(as = "Option<UnitKind>", optional))]
    pub kind: UnitKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub building: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wing: Option<String>,
    /// Its recorded (official) area, m², as the user gave it (a deed's, a permit's): read beside the measured one,
    /// never written by a measurement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub recorded_area: Option<f64>,
    /// Its weight in the common areas' shares when the program shares them by weight (docs/adr/0784 §2): a decimal
    /// written as given (“120”, “1.5”), at most 12 digits before the point and 6 after; read exactly, never as a float.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = r"^[0-9]{1,12}(\.[0-9]{1,6})?$")))]
    pub share_weight: Option<String>,
    /// Niteliği (docs/adr/8031; `.kcad` schema 223; Kat Mülkiyeti Kanunu 12/a: "kat, daire, iş bürosu gibi nevi"):
    /// what the deed calls it (“Daire”, “Dükkân”, “İş bürosu”; an annex's “Depo”, “Garaj”).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub nature: Option<String>,
    /// An annex's independent unit (docs/adr/8031): the id of the group of kind `unit` it is allocated to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub annex_of: Option<String>,
    /// An independent unit's value factor in the land shares (docs/adr/8031; Kat Mülkiyeti Kanunu 3: values by
    /// location and size): a decimal as written (“1.15”), read exactly; 1 when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = r"^[0-9]{1,12}(\.[0-9]{1,6})?$")))]
    pub land_factor: Option<String>,
}

/// The design brief for the spaces of one usage: what the designer aims at, never a legal limit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpaceBrief {
    /// The usage it is for, as the spaces write it (`SpaceFields::usage`).
    pub usage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area_max: Option<f64>,
    /// The least net width: the largest circle inside the space, its diameter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub min_width: Option<f64>,
    /// The least net passage: the widest a body may be and still go between all the space's doors (a corridor's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub min_passage: Option<f64>,
    /// Usages one of its neighbours (a wall or an opening between) should have, each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub adjacent: Vec<String>,
    /// Usages it should open into, each: by a door, a gap or a room separator.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub reach: Vec<String>,
    /// An outdoor space of a unit (a balcony, a terrace): summed apart from its unit's rooms.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub outdoor: bool,
    /// How its rooms count for the floor area ratio (docs/adr/8030; `.kcad` schema 222). Absent: by their name and
    /// use (`emsal_by_use`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub emsal: Option<crate::EmsalCount>,
}

impl SpaceBrief {
    /// How far an area lies outside its range: below the least minus, above the most plus, inside nothing; none when
    /// it gives no range.
    pub fn area_difference(&self, area: f64) -> Option<f64> {
        if self.area_min.is_none() && self.area_max.is_none() {
            return None;
        }
        if let Some(min) = self.area_min
            && area < min
        {
            return Some(area - min);
        }
        if let Some(max) = self.area_max
            && area > max
        {
            return Some(area - max);
        }
        Some(0.0)
    }
}

/// The free room a fixture needs in front of it: a rectangle `width` wide, `depth` deep, centred on the front of the
/// block's own box (its +y side).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FixtureClearance {
    /// The block's name.
    pub block: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: f64,
}

/// A building's program (docs/adr/0773, `.kcad` schema 138).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BuildingProgram {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProgramBuilding>>", optional))]
    pub buildings: Vec<ProgramBuilding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProgramUnit>>", optional))]
    pub units: Vec<ProgramUnit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SpaceBrief>>", optional))]
    pub briefs: Vec<SpaceBrief>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<FixtureClearance>>", optional))]
    pub clearances: Vec<FixtureClearance>,
    /// The brief's own name and version (“Ofis konut brief'i 2026-1”), said with its findings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub version: Option<String>,
    /// How the common areas are shared out to the independent units (docs/adr/0784 §2); none: they are not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub share_rule: Option<crate::ShareRule>,
    /// Arsa payı paydası (docs/adr/8031; `.kcad` schema 223): the whole the land shares are written over; none: the
    /// parcel's area in whole m² when the project has a zoning, else the units' weighted areas in whole m² summed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 100000000)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub land_total: Option<u64>,
    /// The levels two or more buildings span, each with the building it counts as a storey of (docs/adr/8072 §1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SharedLevel>>", optional))]
    pub shared: Vec<SharedLevel>,
}

/// The largest land share whole (payda) a program may give (docs/adr/8031).
pub const MAX_LAND_TOTAL: u64 = 100_000_000;

fn text_problem(v: &str, at: &str, what: &str) -> Option<(String, String)> {
    let t = v.trim();
    if t.is_empty() {
        return Some((at.into(), format!("{what} boş olamaz; bir değer yazın.")));
    }
    if t != v {
        return Some((at.into(), format!("“{v}” {what} başında ya da sonunda boşluk var; silin.")));
    }
    if v.chars().count() > MAX_PROGRAM_TEXT {
        return Some((at.into(), format!("{what} en çok {MAX_PROGRAM_TEXT} harftir.")));
    }
    None
}

fn levels_problem(levels: &[String], at: &str) -> Option<(String, String)> {
    if levels.len() > MAX_PROGRAM_ITEMS {
        return Some((at.into(), format!("En çok {MAX_PROGRAM_ITEMS} kat verilir.")));
    }
    for (k, l) in levels.iter().enumerate() {
        if let Some(p) = text_problem(l, &format!("{at}[{k}]"), "Katın kimliği") {
            return Some(p);
        }
        if levels[..k].contains(l) {
            return Some((format!("{at}[{k}]"), format!("“{l}” katı iki kez yazıldı.")));
        }
    }
    None
}

fn positive(v: Option<f64>, at: &str, what: &str, unit: &str) -> Option<(String, String)> {
    let v = v?;
    (!(v.is_finite() && v > 0.0 && v <= 1.0e6))
        .then(|| (at.into(), format!("{what} {v} {unit}; sıfırdan büyük, sonlu olmalı.")))
}

impl BuildingProgram {
    /// Whether a brief says how its rooms count for the floor area ratio (KCAD 222, docs/adr/8030).
    pub fn has_emsal(&self) -> bool {
        self.briefs.iter().any(|b| b.emsal.is_some())
    }

    pub fn is_empty(&self) -> bool {
        self == &BuildingProgram::default()
    }

    /// Whether it says what architecture data 9 brings (docs/adr/8072 §1): a building's code, use or ±0.00, a shared
    /// level's building.
    pub fn has_building_places(&self) -> bool {
        !self.shared.is_empty()
            || self.buildings.iter().any(|b| b.code.is_some() || b.usage.is_some() || b.zero.is_some())
    }

    /// Whether a building says its own Bölge (Mimari data 30, docs/adr/8144).
    pub fn has_building_zones(&self) -> bool {
        self.buildings.iter().any(|b| b.zone.is_some())
    }

    /// Whether a building says its own structural system (Mimari data 31, docs/adr/8145).
    pub fn has_building_structures(&self) -> bool {
        self.buildings.iter().any(|b| b.structure.is_some())
    }

    /// The building a level is a storey of (docs/adr/8072 §1): the one building that spans it, else the one its shared
    /// entry names; none when no building or several without a choice span it.
    pub fn building_of_level(&self, level: &str) -> Option<&ProgramBuilding> {
        let mut spanning = self.buildings.iter().filter(|b| b.levels.iter().any(|l| l == level));
        let first = spanning.next()?;
        if spanning.next().is_none() {
            return Some(first);
        }
        let chosen = self.shared.iter().find(|s| s.level == level)?;
        self.buildings.iter().find(|b| b.id == chosen.building)
    }

    /// The levels a building counts as its storeys (docs/adr/8072 §4), in its order: its own, a shared one only in the
    /// building chosen for it; none for a building the program does not have.
    pub fn counted_levels(&self, building: &str) -> Vec<&str> {
        let Some(b) = self.building(building) else {
            return Vec::new();
        };
        b.levels
            .iter()
            .filter(|l| self.building_of_level(l).is_some_and(|x| x.id == b.id))
            .map(String::as_str)
            .collect()
    }

    /// A building's ground storey (PAİY 4/1-bbbb, docs/adr/8072 §4): among the levels it counts the one named Zemin,
    /// else the one nearest its own ±0.00; `levels` the project's.
    pub fn ground_of(&self, building: &str, levels: &[crate::ArchLevel]) -> Option<String> {
        let zero = self.building(building)?.zero.unwrap_or(0.0);
        let mine: Vec<&crate::ArchLevel> = self
            .counted_levels(building)
            .into_iter()
            .filter_map(|id| levels.iter().find(|l| l.id == id))
            .collect();
        mine.iter()
            .find(|l| is_ground_name(&l.name))
            .or_else(|| mine.iter().min_by(|a, b| (a.elevation - zero).abs().total_cmp(&(b.elevation - zero).abs())))
            .map(|l| l.id.clone())
    }

    /// The levels two or more buildings span with no building chosen for them, in the buildings' order.
    pub fn unshared_levels(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for b in &self.buildings {
            for l in &b.levels {
                let many = self.buildings.iter().filter(|x| x.levels.contains(l)).count() > 1;
                if many && !out.contains(&l.as_str()) && !self.shared.iter().any(|s| &s.level == l) {
                    out.push(l);
                }
            }
        }
        out
    }

    /// Whether it says what schema 223 brings (docs/adr/8031): an annex, a unit's nature or value factor, the land
    /// shares' whole.
    pub fn has_land_shares(&self) -> bool {
        self.land_total.is_some()
            || self.units.iter().any(|u| {
                u.kind == UnitKind::Annex
                    || u.nature.is_some()
                    || u.annex_of.is_some()
                    || u.land_factor.is_some()
            })
    }

    /// The annexes of the unit of the id given (docs/adr/8031), in the program's order.
    pub fn annexes_of(&self, id: &str) -> Vec<&ProgramUnit> {
        self.units
            .iter()
            .filter(|u| u.kind == UnitKind::Annex && u.annex_of.as_deref() == Some(id))
            .collect()
    }

    /// Whether it says what schema 147 brings (docs/adr/0784): a share rule, a unit's weight, a building's or a wing's
    /// levels.
    pub fn has_shares(&self) -> bool {
        self.share_rule.is_some()
            || self.units.iter().any(|u| u.share_weight.is_some())
            || self.buildings.iter().any(|b| !b.levels.is_empty() || b.wings.iter().any(|w| !w.levels.is_empty()))
    }

    /// The building of the id given.
    pub fn building(&self, id: &str) -> Option<&ProgramBuilding> {
        self.buildings.iter().find(|b| b.id == id)
    }

    /// The groups that name a building or a wing of the id given (its users), in the program's order.
    pub fn users_of(&self, id: &str) -> Vec<&ProgramUnit> {
        self.units
            .iter()
            .filter(|u| u.building.as_deref() == Some(id) || u.wing.as_deref() == Some(id))
            .collect()
    }

    /// The levels a group's spaces should lie on: its wing's when the wing names any, else its building's; empty: any.
    pub fn levels_of(&self, unit: &ProgramUnit) -> &[String] {
        let Some(b) = unit.building.as_deref().and_then(|b| self.building(b)) else {
            return &[];
        };
        if let Some(w) = unit.wing.as_deref().and_then(|w| b.wings.iter().find(|x| x.id == w))
            && !w.levels.is_empty()
        {
            return &w.levels;
        }
        &b.levels
    }

    /// The group of the id given.
    pub fn unit(&self, id: &str) -> Option<&ProgramUnit> {
        self.units.iter().find(|u| u.id == id)
    }

    /// The brief for a usage (exact, as written).
    pub fn brief(&self, usage: &str) -> Option<&SpaceBrief> {
        self.briefs.iter().find(|b| b.usage == usage)
    }

    /// The clearance for a block's name.
    pub fn clearance(&self, block: &str) -> Option<&FixtureClearance> {
        self.clearances.iter().find(|c| c.block == block)
    }

    /// The rule and the brief's version, as a finding says them.
    pub fn rule(&self) -> String {
        match &self.version {
            Some(v) => format!("{PROGRAM_RULE} · {v}"),
            None => PROGRAM_RULE.into(),
        }
    }

    /// What is wrong with it, said so a reader can fix it: the field and the words; none when it holds.
    pub fn problem(&self) -> Option<(String, String)> {
        for (key, n) in [
            ("buildings", self.buildings.len()),
            ("units", self.units.len()),
            ("briefs", self.briefs.len()),
            ("clearances", self.clearances.len()),
        ] {
            if n > MAX_PROGRAM_ITEMS {
                return Some((key.into(), format!("Programda en çok {MAX_PROGRAM_ITEMS} öğe olur; {n} verildi.")));
            }
        }
        if let Some(v) = &self.version
            && let Some(p) = text_problem(v, "version", "Hedeflerin sürümü")
        {
            return Some(p);
        }
        let mut ids: Vec<&str> = Vec::new();
        for (i, b) in self.buildings.iter().enumerate() {
            if let Some(p) = text_problem(&b.id, &format!("buildings[{i}].id"), "Binanın kimliği") {
                return Some(p);
            }
            if let Some(p) = text_problem(&b.name, &format!("buildings[{i}].name"), "Binanın adı") {
                return Some(p);
            }
            if ids.contains(&b.id.as_str()) {
                return Some((format!("buildings[{i}].id"), format!("“{}” kimliği iki kez var; her bina ve kanadın kimliği ayrı olmalı.", b.id)));
            }
            ids.push(&b.id);
            if let Some(p) = levels_problem(&b.levels, &format!("buildings[{i}].levels")) {
                return Some(p);
            }
            // Its own structural system (docs/adr/8145): traditional elements only of cut stone masonry.
            if let Some((path, why)) = b.structure.as_ref().and_then(crate::BuildingStructure::problem) {
                return Some((format!("buildings[{i}].{path}"), why));
            }
            // Its code, use and ±0.00 (docs/adr/8072 §1; EK-2 §2.3).
            if let Some(c) = &b.code {
                let well = c.chars().count() == 2 && c.chars().all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit());
                if !well {
                    return Some((
                        format!("buildings[{i}].code"),
                        format!("“{c}” bina kodu olamaz; bina kodu iki büyük harf ya da rakamdır (A1, B2, 01)."),
                    ));
                }
                if c == "00" && self.buildings.len() > 1 {
                    return Some((
                        format!("buildings[{i}].code"),
                        "“00” tek binalı projenin kodudur; birden çok binada her binaya kendi kodunu verin.".into(),
                    ));
                }
                if self.buildings[..i].iter().any(|x| x.code.as_deref() == Some(c.as_str())) {
                    return Some((format!("buildings[{i}].code"), format!("“{c}” bina kodu iki binada var; her binanın kodu ayrı olmalı.")));
                }
            }
            if let Some(u) = &b.usage
                && let Some(p) = text_problem(u, &format!("buildings[{i}].usage"), "Binanın kullanım türü")
            {
                return Some(p);
            }
            if let Some(z) = b.zero
                && !(z.is_finite() && z.abs() <= MAX_BUILDING_ZERO)
            {
                return Some((
                    format!("buildings[{i}].zero"),
                    format!("Binanın ±0.00'ı projenin ±0.00'ından en çok {MAX_BUILDING_ZERO} m uzakta olur; {z} verildi."),
                ));
            }
            for (k, w) in b.wings.iter().enumerate() {
                let at = format!("buildings[{i}].wings[{k}]");
                if let Some(p) = text_problem(&w.id, &format!("{at}.id"), "Kanadın kimliği") {
                    return Some(p);
                }
                if let Some(p) = text_problem(&w.name, &format!("{at}.name"), "Kanadın adı") {
                    return Some(p);
                }
                if ids.contains(&w.id.as_str()) {
                    return Some((format!("{at}.id"), format!("“{}” kimliği iki kez var; her bina ve kanadın kimliği ayrı olmalı.", w.id)));
                }
                ids.push(&w.id);
                if let Some(p) = levels_problem(&w.levels, &format!("{at}.levels")) {
                    return Some(p);
                }
                if !b.levels.is_empty()
                    && let Some((k, l)) = w.levels.iter().enumerate().find(|(_, l)| !b.levels.contains(l))
                {
                    return Some((
                        format!("{at}.levels[{k}]"),
                        format!("“{l}” katı “{}” binasının katlarında yok; kanadın katları binasının katlarından seçilir.", b.name),
                    ));
                }
            }
        }
        // The shared levels (docs/adr/8072 §1): each once, spanned by two or more buildings, counted in one of them.
        for (i, sh) in self.shared.iter().enumerate() {
            let at = format!("shared[{i}]");
            if self.shared[..i].iter().any(|x| x.level == sh.level) {
                return Some((format!("{at}.level"), format!("“{}” katının sayıldığı bina iki kez verildi.", sh.level)));
            }
            let spanning: Vec<&ProgramBuilding> =
                self.buildings.iter().filter(|b| b.levels.contains(&sh.level)).collect();
            if spanning.len() < 2 {
                return Some((
                    format!("{at}.level"),
                    format!("“{}” katını birden çok bina kaplamıyor; ortak kat değil, sayıldığı bina seçilmez.", sh.level),
                ));
            }
            if !spanning.iter().any(|b| b.id == sh.building) {
                return Some((
                    format!("{at}.building"),
                    format!("“{}” katı “{}” binasının katı değil; onu kaplayan binalardan birini seçin.", sh.level, sh.building),
                ));
            }
        }
        let mut unit_ids: Vec<&str> = Vec::new();
        let mut numbers: Vec<&str> = Vec::new();
        for (i, u) in self.units.iter().enumerate() {
            let at = format!("units[{i}]");
            if let Some(p) = text_problem(&u.id, &format!("{at}.id"), "Bölümün kimliği") {
                return Some(p);
            }
            if let Some(p) = text_problem(&u.number, &format!("{at}.number"), "Bölümün numarası") {
                return Some(p);
            }
            if let Some(n) = &u.name
                && let Some(p) = text_problem(n, &format!("{at}.name"), "Bölümün adı")
            {
                return Some(p);
            }
            if unit_ids.contains(&u.id.as_str()) {
                return Some((format!("{at}.id"), format!("“{}” kimlikli bölüm iki kez var.", u.id)));
            }
            if numbers.contains(&u.number.as_str()) {
                return Some((format!("{at}.number"), format!("“{}” numarası iki bölümde var; her bölümün numarası ayrı olmalı.", u.number)));
            }
            unit_ids.push(&u.id);
            numbers.push(&u.number);
            let building = match &u.building {
                Some(b) => match self.buildings.iter().find(|x| &x.id == b) {
                    Some(x) => Some(x),
                    None => {
                        return Some((format!("{at}.building"), format!("“{b}” binası programda yok; önce binayı ekleyin.")));
                    }
                },
                None => None,
            };
            if let Some(w) = &u.wing {
                match building {
                    Some(b) if b.wings.iter().any(|x| &x.id == w) => {}
                    Some(b) => {
                        return Some((format!("{at}.wing"), format!("“{w}” kanadı “{}” binasında yok.", b.name)));
                    }
                    None => {
                        return Some((format!("{at}.wing"), "Kanat yalnız binası verilen bölümde seçilir.".into()));
                    }
                }
            }
            if let Some(p) = positive(u.recorded_area, &format!("{at}.recordedArea"), "Kayıtlı alan", "m²") {
                return Some(p);
            }
            if let Some(w) = &u.share_weight
                && let Err(why) = crate::program_share::weight_units(w)
            {
                return Some((format!("{at}.shareWeight"), why));
            }
            // Kat irtifakı (docs/adr/8031): the nature, an annex's unit, a unit's value factor.
            if let Some(n) = &u.nature
                && let Some(p) = text_problem(n, &format!("{at}.nature"), "Bölümün niteliği")
            {
                return Some(p);
            }
            match (u.kind, &u.annex_of) {
                (UnitKind::Annex, None) => {
                    return Some((
                        format!("{at}.annexOf"),
                        format!(
                            "“{}” eklentisinin hangi bağımsız bölüme ait olduğu seçilmeli.",
                            u.number
                        ),
                    ));
                }
                (UnitKind::Annex, Some(_)) | (_, None) => {}
                (_, Some(_)) => {
                    return Some((
                        format!("{at}.annexOf"),
                        "Ait olduğu bölüm yalnız eklentide seçilir.".into(),
                    ));
                }
            }
            if let Some(f) = &u.land_factor {
                if u.kind != UnitKind::Unit {
                    return Some((
                        format!("{at}.landFactor"),
                        "Değer katsayısı yalnız bağımsız bölümde verilir.".into(),
                    ));
                }
                match crate::program_share::weight_units(f) {
                    Err(why) => {
                        return Some((
                            format!("{at}.landFactor"),
                            why.replacen("Pay ağırlığı", "Değer katsayısı", 1),
                        ));
                    }
                    Ok(0) => {
                        return Some((
                            format!("{at}.landFactor"),
                            "Değer katsayısı sıfırdan büyük olmalı.".into(),
                        ));
                    }
                    Ok(_) => {}
                }
            }
        }
        for (i, u) in self.units.iter().enumerate() {
            if let Some(of) = &u.annex_of
                && !self
                    .units
                    .iter()
                    .any(|x| &x.id == of && x.kind == UnitKind::Unit)
            {
                return Some((
                    format!("units[{i}].annexOf"),
                    format!(
                        "“{}” eklentisinin ait olduğu “{of}” programda bir bağımsız bölüm değil.",
                        u.number
                    ),
                ));
            }
        }
        if let Some(t) = self.land_total
            && !(1..=MAX_LAND_TOTAL).contains(&t)
        {
            return Some((
                "landTotal".into(),
                format!(
                    "Arsa payı paydası {t}; 1 ile {MAX_LAND_TOTAL} arasında bir tam sayı olmalı."
                ),
            ));
        }
        let mut usages: Vec<&str> = Vec::new();
        for (i, b) in self.briefs.iter().enumerate() {
            let at = format!("briefs[{i}]");
            if let Some(p) = text_problem(&b.usage, &format!("{at}.usage"), "Hedefin kullanımı") {
                return Some(p);
            }
            if usages.contains(&b.usage.as_str()) {
                return Some((format!("{at}.usage"), format!("“{}” kullanımının hedefi iki kez var.", b.usage)));
            }
            usages.push(&b.usage);
            for (key, v, what, unit) in [
                ("areaMin", b.area_min, "En az alan", "m²"),
                ("areaMax", b.area_max, "En çok alan", "m²"),
                ("minWidth", b.min_width, "Net en", "m"),
                ("minPassage", b.min_passage, "Net geçiş", "m"),
            ] {
                if let Some(p) = positive(v, &format!("{at}.{key}"), what, unit) {
                    return Some(p);
                }
            }
            if let (Some(lo), Some(hi)) = (b.area_min, b.area_max)
                && lo > hi
            {
                return Some((format!("{at}.areaMax"), format!("En çok alan ({hi} m²) en az alandan ({lo} m²) küçük.")));
            }
            for (key, list) in [("adjacent", &b.adjacent), ("reach", &b.reach)] {
                if list.len() > MAX_BRIEF_USAGES {
                    return Some((format!("{at}.{key}"), format!("En çok {MAX_BRIEF_USAGES} kullanım verilir.")));
                }
                for (k, u) in list.iter().enumerate() {
                    if let Some(p) = text_problem(u, &format!("{at}.{key}[{k}]"), "Kullanım") {
                        return Some(p);
                    }
                    if list[..k].contains(u) {
                        return Some((format!("{at}.{key}[{k}]"), format!("“{u}” iki kez yazıldı.")));
                    }
                }
            }
        }
        let mut blocks: Vec<&str> = Vec::new();
        for (i, c) in self.clearances.iter().enumerate() {
            let at = format!("clearances[{i}]");
            if let Some(p) = text_problem(&c.block, &format!("{at}.block"), "Bloğun adı") {
                return Some(p);
            }
            if blocks.contains(&c.block.as_str()) {
                return Some((format!("{at}.block"), format!("“{}” bloğunun kullanım alanı iki kez var.", c.block)));
            }
            blocks.push(&c.block);
            for (key, v, what) in [("width", c.width, "Kullanım alanının eni"), ("depth", c.depth, "Kullanım alanının derinliği")] {
                if let Some(p) = positive(Some(v), &format!("{at}.{key}"), what, "m") {
                    return Some(p);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_brief_says_how_far_an_area_is_out() {
        let b = SpaceBrief { usage: "Yatak odası".into(), area_min: Some(12.0), area_max: Some(16.0), ..Default::default() };
        assert_eq!(b.area_difference(10.5), Some(-1.5));
        assert_eq!(b.area_difference(14.0), Some(0.0));
        assert_eq!(b.area_difference(17.25), Some(1.25));
        assert_eq!(SpaceBrief::default().area_difference(3.0), None);
    }

    #[test]
    fn a_program_names_what_does_not_hold() {
        let mut p = BuildingProgram {
            units: vec![
                ProgramUnit { id: "d1".into(), number: "1".into(), ..Default::default() },
                ProgramUnit { id: "d2".into(), number: "1".into(), ..Default::default() },
            ],
            ..Default::default()
        };
        assert_eq!(p.problem().map(|p| p.0), Some("units[1].number".into()));
        p.units[1].number = "2".into();
        assert_eq!(p.problem(), None);
        p.units[1].wing = Some("a".into());
        assert_eq!(p.problem().map(|p| p.0), Some("units[1].wing".into()));
    }

    /// Two blocks over a common basement (docs/adr/8072 §1): the basement is the building's its shared entry names, the
    /// others' storeys their own; without the entry the basement is no one's and is said; a code in a wrong form, a
    /// repeated one, "00" beside another building, a shared entry for a level only one building spans or naming a
    /// building that does not span it are refused.
    #[test]
    fn a_shared_basement_counts_in_the_building_chosen() {
        let b = |id: &str, levels: &[&str], code: &str| ProgramBuilding {
            id: id.into(),
            name: format!("{id} Blok"),
            levels: levels.iter().map(|l| (*l).into()).collect(),
            code: Some(code.into()),
            ..Default::default()
        };
        let mut p = BuildingProgram {
            buildings: vec![b("A", &["bodrum", "zemin-a"], "A1"), b("B", &["bodrum", "zemin-b"], "B1")],
            ..Default::default()
        };
        assert_eq!(p.problem(), None);
        assert_eq!(p.unshared_levels(), vec!["bodrum"]);
        assert!(p.building_of_level("bodrum").is_none());
        assert_eq!(p.building_of_level("zemin-b").map(|x| x.id.as_str()), Some("B"));
        p.shared = vec![SharedLevel { level: "bodrum".into(), building: "B".into() }];
        assert_eq!(p.problem(), None);
        assert!(p.unshared_levels().is_empty());
        assert_eq!(p.building_of_level("bodrum").map(|x| x.id.as_str()), Some("B"));
        assert!(p.has_building_places());
        let refused = |q: &BuildingProgram| q.problem().map(|x| x.0);
        let mut q = p.clone();
        q.buildings[1].code = Some("b1".into());
        assert_eq!(refused(&q), Some("buildings[1].code".into()));
        q.buildings[1].code = Some("A1".into());
        assert_eq!(refused(&q), Some("buildings[1].code".into()));
        q.buildings[1].code = Some("00".into());
        assert_eq!(refused(&q), Some("buildings[1].code".into()));
        let mut q = p.clone();
        q.shared = vec![SharedLevel { level: "zemin-a".into(), building: "A".into() }];
        assert_eq!(refused(&q), Some("shared[0].level".into()));
        q.shared = vec![SharedLevel { level: "bodrum".into(), building: "C".into() }];
        assert_eq!(refused(&q), Some("shared[0].building".into()));
        let mut q = p.clone();
        q.buildings[0].zero = Some(f64::NAN);
        assert_eq!(refused(&q), Some("buildings[0].zero".into()));
        let one = BuildingProgram { buildings: vec![b("A", &[], "00")], ..Default::default() };
        assert_eq!(one.problem(), None, "a single building's 00");
    }

    /// Each building counts its own levels and the shared one where it is chosen (docs/adr/8072 §4); its ground storey
    /// is the one named Zemin, else the one nearest its own ±0.00.
    #[test]
    fn a_building_counts_its_levels_and_stands_on_its_ground() {
        let level = |id: &str, name: &str, elevation: f64| crate::ArchLevel {
            id: id.into(),
            name: name.into(),
            elevation,
            height: None,
        };
        let levels = [
            level("bodrum", "Bodrum kat", -3.0),
            level("zemin-a", "ZEMİN KAT", 0.0),
            level("giris-b", "Giriş", 1.2),
            level("k1-b", "1. kat", 4.2),
        ];
        let p = BuildingProgram {
            buildings: vec![
                ProgramBuilding {
                    id: "A".into(),
                    name: "A Blok".into(),
                    levels: vec!["bodrum".into(), "zemin-a".into()],
                    ..Default::default()
                },
                ProgramBuilding {
                    id: "B".into(),
                    name: "B Blok".into(),
                    levels: vec!["bodrum".into(), "giris-b".into(), "k1-b".into()],
                    zero: Some(1.2),
                    ..Default::default()
                },
            ],
            shared: vec![SharedLevel { level: "bodrum".into(), building: "A".into() }],
            ..Default::default()
        };
        assert_eq!(p.counted_levels("A"), vec!["bodrum", "zemin-a"]);
        assert_eq!(p.counted_levels("B"), vec!["giris-b", "k1-b"]);
        assert!(p.counted_levels("C").is_empty());
        // A's by its name (the dotted İ folded), B's by its ±0.00 at +1.20.
        assert_eq!(p.ground_of("A", &levels).as_deref(), Some("zemin-a"));
        assert_eq!(p.ground_of("B", &levels).as_deref(), Some("giris-b"));
        assert!(is_ground_name("Zemin kat") && is_ground_name("ZEMİN") && !is_ground_name("Bodrum"));
    }
}

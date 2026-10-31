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
}

impl UnitKind {
    pub const ALL: [UnitKind; 3] = [UnitKind::Unit, UnitKind::Common, UnitKind::Outdoor];

    pub fn label(self) -> &'static str {
        match self {
            UnitKind::Unit => "Bağımsız bölüm",
            UnitKind::Common => "Ortak alan",
            UnitKind::Outdoor => "Dış alan",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            UnitKind::Unit => "unit",
            UnitKind::Common => "common",
            UnitKind::Outdoor => "outdoor",
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
}

/// A building and its wings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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
}

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
}

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

fn positive(v: Option<f64>, at: &str, what: &str, unit: &str) -> Option<(String, String)> {
    let v = v?;
    (!(v.is_finite() && v > 0.0 && v <= 1.0e6))
        .then(|| (at.into(), format!("{what} {v} {unit}; sıfırdan büyük, sonlu olmalı.")))
}

impl BuildingProgram {
    pub fn is_empty(&self) -> bool {
        self == &BuildingProgram::default()
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
}

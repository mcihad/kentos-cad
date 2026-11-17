//! A drawing's building facts: the `building` section of `kentos.auditFacts` 1 (docs/adr/8078). What the regulation's
//! rules count a building by that the architecture section does not carry: every storey of the project with its
//! elevation, the rooms it holds by their program group's kind (an independent unit's, a common area's, an annex's, none)
//! and as a technical or a lived-in room, its elevators; the program's independent units and its dwellings; the
//! municipality's regulation the project reads over the national one. A rule decides what a storey counts as for its
//! article and its version (Planlı Alanlar İmar Yönetmeliği 34/1 before and after 1.7.2026 count a basement apart);
//! the facts say what is drawn, never what it means.
//!
//! An optional section of version 1, as [`super::architecture`]: absent when the project has no storey, so the facts
//! of a drawing that never had one stay byte for byte what they were. The storeys lowest first (by elevation, then
//! id), each elevation to 1 mm.

use serde::{Deserialize, Serialize};

use super::{AREA, LENGTH, round};

/// The building as the regulation's rules count it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct BuildingFacts {
    /// Every storey of the project, lowest first.
    pub storeys: Vec<StoreyHolds>,
    /// The building program's independent units (bağımsız bölüm).
    pub units: u32,
    /// Those of them whose nature is a dwelling's (Daire, Mesken, Konut) or not given.
    pub dwellings: u32,
    /// The municipality's regulation the project reads over the national one (`sivas`: Sivas Belediyesi İmar
    /// Yönetmeliği, RG 10.11.2022/32009); absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<String>,
    /// The rooms that carry people as the escape rules count them (BYKHY Ek-5/A, docs/adr/8035), by persistent id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rooms: Vec<RoomPeople>,
    /// The doors, by persistent id (docs/adr/8078 §8).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub doors: Vec<DoorFacts>,
    /// The uses Planlı Alanlar İmar Yönetmeliği 22/1 lets out of katlar alanı only up to a limit, as Emsal ve TAKS
    /// tablosu counts them (docs/adr/8079): a unit's balconies, its storage annexes, the caretaker flats, the library.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limits: Vec<EmsalLimit>,
    /// What the program's buildings are used for, as the project writes it (Kullanım türü), in the program's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uses: Vec<String>,
}

/// A use 22/1 lets out of katlar alanı only up to a limit (docs/adr/8079), as Emsal ve TAKS tablosu counts it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct EmsalLimit {
    /// `balcony` (22/1-l: a unit's open balconies, 20 % of its net area), `storage` (22/1-k: a unit's storage annexes
    /// on a storey under the ground, 50 % of its net area), `caretaker` (22/1-ç: the caretaker flats, 70 m² together) or
    /// `library` (22/1-f: the library rooms, 50 m² together).
    pub limit: String,
    /// The independent unit whose net area the limit is a share of, by program id (a balcony's, a storage annex's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Its rooms, by persistent id.
    pub rooms: Vec<String>,
    /// Their area as katlar alanı counts it (each room's outer contour, PAİY 4/1-ğ-1), m².
    pub area: f64,
    /// What the limit lets out, m².
    pub allowed: f64,
    /// What passed it, counted in, m².
    pub over: f64,
    /// Every room of it is a common area's (a program group of kind `common`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub common: bool,
}

/// A room's people: its use's square metres a person over its area, rounded up (BYKHY Ek-5/A as Mimari denetim reads
/// it).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RoomPeople {
    pub uid: String,
    pub level: Option<String>,
    /// Its name (and number).
    pub name: String,
    pub persons: f64,
}

/// A door: the rooms on its two sides, the one its leaf swings into, how it opens, its declared fire class, and whether
/// it opens into an escape stair's room or a fire safety lobby.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DoorFacts {
    pub uid: String,
    pub level: Option<String>,
    /// The rooms on its two sides by persistent id, the path's left first; none: no room there (outdoors, a wall's
    /// other face with nothing drawn).
    pub between: [Option<String>; 2],
    /// The room its leaf swings into; none for a door that does not swing or swings where no room is.
    pub opens_into: Option<String>,
    /// `swing`, `sliding`, `folding`, `revolving`, `pocket` … as the opening's operation is named.
    pub operation: String,
    /// The fire class the author declares, the door's own else its type's (docs/adr/8068): `EI2 60-S200C5`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_rating: Option<String>,
    /// One of its sides is an escape stair's room (a room named or used as Merdiven that is no dwelling unit's own).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stair: bool,
    /// One of its sides is a fire safety lobby (a room named Yangın güvenlik holü).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lobby: bool,
}

/// A storey and what stands on it. A room is one that is neither a void (a shaft, a gallery) nor outdoors, counted by
/// its program group's kind and, apart, as a technical or a lived-in room; an elevator shaft is a void and counted with
/// the elevators.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StoreyHolds {
    /// The storey's id (`architecture.levels[].id`).
    pub level: String,
    /// Its name.
    pub name: String,
    /// Its elevation over ±0.00, m.
    pub elevation: f64,
    /// Rooms of an independent unit on it.
    pub units: u32,
    /// Rooms of a common area on it that are not technical rooms.
    pub common: u32,
    /// Rooms of an annex (eklenti, a place allocated to an independent unit) on it.
    pub annexes: u32,
    /// Rooms in no program group on it that are not technical rooms.
    pub other: u32,
    /// Technical rooms on it, whatever their group: a heat centre, a boiler room, an electricity room, a water tank, a
    /// climate plant, a generator room, a pump or a transformer room, by their name or use.
    pub technical: u32,
    /// Rooms of a dwelling's living kind on it, whatever their group (a living room, a bedroom, a kitchen, by their name
    /// or use): lived in.
    pub inhabited: u32,
    /// Elevators on it: elevator shafts (a space named Asansör, not its lobby) and Asansör blocks standing on it.
    pub elevators: u32,
    /// The rooms counted on it, by their persistent ids in order: what a finding about the storey names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rooms: Vec<String>,
    /// An escape stair's room stands on it: the stair serves it (BYKHY 47/3 counts them).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stair: bool,
}

impl BuildingFacts {
    /// These facts in the section's order and rounding. Doing it again changes nothing.
    pub fn normalized(mut self) -> BuildingFacts {
        for s in &mut self.storeys {
            s.elevation = round(s.elevation, LENGTH);
            s.rooms.sort();
            s.rooms.dedup();
        }
        self.storeys
            .sort_by(|a, b| a.elevation.total_cmp(&b.elevation).then_with(|| a.level.cmp(&b.level)));
        self.local = self.local.filter(|l| !l.is_empty());
        for r in &mut self.rooms {
            r.persons = round(r.persons, LENGTH);
        }
        self.rooms.sort_by(|a, b| a.uid.cmp(&b.uid));
        self.doors.sort_by(|a, b| a.uid.cmp(&b.uid));
        for l in &mut self.limits {
            l.rooms.sort();
            l.area = round(l.area, AREA);
            l.allowed = round(l.allowed, AREA);
            l.over = round(l.over, AREA);
        }
        self.uses.retain(|u| !u.trim().is_empty());
        self
    }

    /// Whether there is nothing to say: no storey.
    pub fn is_empty(&self) -> bool {
        self.storeys.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storey(level: &str, elevation: f64) -> StoreyHolds {
        StoreyHolds { level: level.into(), name: level.into(), elevation, ..StoreyHolds::default() }
    }

    #[test]
    fn storeys_are_lowest_first_and_their_elevations_rounded() {
        let facts = BuildingFacts {
            storeys: vec![storey("k1", 3.000_4), storey("b1", -3.000_6), storey("k0", 0.0), storey("a", 0.0)],
            units: 6,
            dwellings: 6,
            local: Some(String::new()),
            ..BuildingFacts::default()
        }
        .normalized();
        let order: Vec<&str> = facts.storeys.iter().map(|s| s.level.as_str()).collect();
        assert_eq!(order, ["b1", "a", "k0", "k1"]);
        assert_eq!(facts.storeys[0].elevation, -3.001);
        assert_eq!(facts.storeys[3].elevation, 3.0);
        assert_eq!(facts.local, None);
        assert_eq!(facts.clone().normalized(), facts);
        let json = serde_json::to_string(&facts).unwrap_or_default();
        assert!(json.starts_with(r#"{"storeys":[{"level":"b1","name":"b1","elevation":-3.001,"units":0"#), "{json}");
        assert!(!json.contains("local"), "{json}");
    }
}

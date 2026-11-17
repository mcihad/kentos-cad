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

use super::{LENGTH, round};

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

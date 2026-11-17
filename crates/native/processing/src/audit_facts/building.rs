//! The drawing's building facts for the audit facts (`building`, docs/adr/8078): every storey of the project with the
//! rooms it holds by their program group's kind and as technical or lived-in rooms, and its elevators; the program's
//! independent units and dwellings; the municipality's regulation the project reads. Their order and rounding are
//! `kentos_extensions::audit_facts::building`'s; the rules that read them are the standards package's Python
//! (catalog/standards/mimari-mevzuat, bim-plan K10), which decides what a storey counts as for an article's version.
//!
//! - **A room** is a space that is neither a void nor outdoors (its use's brief, docs/adr/0773), on a storey; a room
//!   whose outline is broken is still drawn there and counted.
//! - **Technical, lived in:** the domain's readings of a room's name or use (`kentos_domain::room_names::technical`,
//!   `kind_of`: a living room, a bedroom or a kitchen is lived in), whatever its group.
//! - **An elevator** is a space named Asansör (not its lobby, `regulation_audit::shaft_of`), a void or not, or an
//!   Asansör block standing in no such space of its storey (`elevator_cabins`, docs/adr/8045 §5): a cabin in a shaft is
//!   the shaft's elevator, counted once.

use kentos_commands::{Entity, LocalRules, UnitKind};
use kentos_domain::Document;
use kentos_domain::room_names::{RoomKind, dwelling_nature, kind_of, technical};
use kentos_extensions::audit_facts::building::{BuildingFacts, StoreyHolds};

/// The drawing's building facts; empty (no storey) for a project without Mimari's storeys.
pub fn measure(doc: &Document) -> BuildingFacts {
    let Some(arch) = doc.settings().arch.as_ref() else {
        return BuildingFacts::default();
    };
    let program = doc.program();
    let mut storeys: Vec<StoreyHolds> = arch
        .levels
        .iter()
        .map(|l| StoreyHolds {
            level: l.id.clone(),
            name: l.name.clone(),
            elevation: l.elevation,
            ..StoreyHolds::default()
        })
        .collect();
    let units: Vec<_> = program.units.iter().filter(|u| u.kind == UnitKind::Unit).collect();
    let dwellings = units.iter().filter(|u| dwelling_nature(u.nature.as_deref())).count();
    let shafts = elevators(doc, &mut storeys);
    for e in doc.entities() {
        let Entity::Space(s) = e else { continue };
        let x = &s.space;
        let Some(at) = e.own_level().and_then(|id| storeys.iter().position(|l| l.level == id)) else {
            continue;
        };
        let (name, usage) = (x.name.as_deref(), x.usage.as_deref());
        let outdoor = usage.and_then(|u| program.brief(u)).is_some_and(|b| b.outdoor);
        if x.void || outdoor || shafts.contains(&s.base.id) {
            continue;
        }
        let storey = &mut storeys[at];
        let tech = technical(name, usage);
        match x.unit.as_deref().and_then(|u| program.unit(u)).map(|u| u.kind) {
            Some(UnitKind::Unit) => storey.units += 1,
            Some(UnitKind::Annex) => storey.annexes += 1,
            Some(UnitKind::Common) if !tech => storey.common += 1,
            None if !tech => storey.other += 1,
            _ => {}
        }
        if tech {
            storey.technical += 1;
        }
        if matches!(kind_of(name, usage), Some(RoomKind::Living | RoomKind::Bedroom | RoomKind::Kitchen)) {
            storey.inhabited += 1;
        }
    }
    BuildingFacts {
        storeys,
        units: units.len() as u32,
        dwellings: dwellings as u32,
        local: arch.local_rules.map(|l| match l {
            LocalRules::Sivas => "sivas".to_owned(),
        }),
    }
}

/// Each storey's elevators counted into it: its elevator shafts and the Asansör blocks standing in none of them; the
/// shafts' ids, which are no rooms.
#[cfg(feature = "mod-arch")]
fn elevators(doc: &Document, storeys: &mut [StoreyHolds]) -> Vec<u32> {
    use kentos_geometry_core::geom::region::inside_area;
    use kentos_native_application::regulation_audit::{ShaftKind, shaft_of};
    let mut shafts = Vec::new();
    let mut areas: Vec<(String, Vec<kentos_geometry_core::geom::arrangement::Area>)> = Vec::new();
    for e in doc.entities() {
        let Entity::Space(s) = e else { continue };
        let x = &s.space;
        if shaft_of(x.name.as_deref(), x.usage.as_deref()) != Some(ShaftKind::Elevator) {
            continue;
        }
        shafts.push(s.base.id);
        let Some(level) = e.own_level() else { continue };
        if let Some(storey) = storeys.iter_mut().find(|l| l.level == level) {
            storey.elevators += 1;
            areas.push((level.to_owned(), kentos_domain::arch_values::core_areas(&x.parts)));
        }
    }
    for cabin in kentos_native_application::elevator_cabins::cabins(doc) {
        let Some(level) = cabin.level.as_deref() else { continue };
        let at = kentos_geometry_core::Vec2::new(cabin.at.x, cabin.at.y);
        let in_shaft = areas.iter().any(|(l, a)| l == level && a.iter().any(|a| inside_area(a, at)));
        if !in_shaft && let Some(storey) = storeys.iter_mut().find(|l| l.level == level) {
            storey.elevators += 1;
        }
    }
    shafts
}

/// Without Mimari's building parts there is no elevator to read.
#[cfg(not(feature = "mod-arch"))]
fn elevators(_: &Document, _: &mut [StoreyHolds]) -> Vec<u32> {
    Vec::new()
}

#[cfg(test)]
mod tests;

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
//! - **People** (docs/adr/8078 §8): each room's as the escape rules count them (`fire_audit::room_people`: BYKHY
//!   Ek-5/A, its use's square metres a person over its area, rounded up); a dwelling's room carries none.
//! - **A door** (a door that opens: not a fixed one): the rooms on its wall's two faces on its wall's storey, the path's
//!   left first, and the one its leaf swings into (its `side`, the rule the room list reads a door's room by, docs/adr
//!   8028); its operation; its fire class, its own else its type's (docs/adr/8068); whether a side is an escape stair's
//!   room (a room named or used as Merdiven that is no dwelling unit's own, as the escape routes read it) or a fire
//!   safety lobby (named Yangın güvenlik holü). A storey holding an escape stair's room is the stair's.
//! - **The limited uses** (docs/adr/8079): what Emsal ve TAKS tablosu lets out of katlar alanı only up to a limit of
//!   22/1 (`Document::zoning_limits`), with or without the parcel's zoning; and the program's buildings' uses.
//! - **Under the stairs** (docs/adr/8080): each stair's places lower than 1.80 m (`Document::under_stairs`) and the rooms
//!   its spaces lie in (`Document::under_stair_rooms`), each named by its name and number.

use kentos_commands::{Entity, LocalRules, UnitKind};
use kentos_domain::Document;
use kentos_domain::room_names::{RoomKind, dwelling_nature, kind_of, technical};
use kentos_extensions::audit_facts::building::{
    BuildingFacts, DoorFacts, EmsalLimit, RoomPeople, StoreyHolds, UnderStairFacts, UnderStairRoom,
};

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
    for (uid, e) in doc.entities_with_uids() {
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
        storey.rooms.push(uid.to_string());
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
    let (rooms, doors) = people_and_doors(doc, &mut storeys);
    let limits = doc
        .zoning_limits()
        .into_iter()
        .map(|l| EmsalLimit {
            limit: l.limit.name().to_owned(),
            unit: l.unit,
            rooms: l.rooms,
            area: l.area,
            allowed: l.allowed,
            over: l.over,
            common: l.common,
        })
        .collect();
    let names: std::collections::HashMap<String, String> = doc
        .entities_with_uids()
        .filter_map(|(uid, e)| match e {
            Entity::Space(s) => Some((uid.to_string(), room_name(&s.space))),
            _ => None,
        })
        .collect();
    let under_stairs = doc
        .under_stairs()
        .into_iter()
        .map(|u| {
            let area = |a: &[kentos_geometry_core::geom::arrangement::Area]| kentos_geometry_core::geom::space::area(a);
            UnderStairFacts {
                rooms: doc
                    .under_stair_rooms(&u)
                    .into_iter()
                    .map(|(uid, area)| UnderStairRoom { name: names.get(&uid).cloned().unwrap_or_default(), uid, area })
                    .collect(),
                stair: u.uid,
                level: u.level,
                low: area(&u.low),
                open: area(&u.open),
                waist: u.waist,
            }
        })
        .collect();
    BuildingFacts {
        storeys,
        units: units.len() as u32,
        dwellings: dwellings as u32,
        local: arch.local_rules.map(|l| match l {
            LocalRules::Sivas => "sivas".to_owned(),
        }),
        rooms,
        doors,
        limits,
        uses: program.buildings.iter().filter_map(|b| b.usage.clone()).collect(),
        under_stairs,
    }
}

/// A room's name as a finding says it: its name, else its use, else "Mahal"; its number after it.
fn room_name(x: &kentos_commands::SpaceFields) -> String {
    let name = x.name.clone().or_else(|| x.usage.clone()).unwrap_or_else(|| "Mahal".to_owned());
    match x.number.as_deref().filter(|n| !n.is_empty()) {
        Some(n) => format!("{name} ({n})"),
        None => name,
    }
}

/// The rooms' people and the doors; each storey holding an escape stair's room marked.
#[cfg(feature = "mod-arch")]
fn people_and_doors(doc: &Document, storeys: &mut [StoreyHolds]) -> (Vec<RoomPeople>, Vec<DoorFacts>) {
    use kentos_commands::{OpeningForm, OpeningOperation, OpeningSide};
    use kentos_domain::room_names::{has, words};
    use kentos_geometry_core::geom::region::inside_area;
    let people = kentos_native_application::fire_audit::room_people(doc)
        .into_iter()
        .filter_map(|(slot, level, name, persons)| {
            Some(RoomPeople { uid: doc.uid(slot)?.to_string(), level, name, persons })
        })
        .collect();
    let program = doc.program();
    // The rooms each storey's points are found in: id, storey, outline, an escape stair's, a fire safety lobby.
    let mut rooms: Vec<(String, Option<String>, Vec<kentos_geometry_core::geom::arrangement::Area>, bool, bool)> =
        Vec::new();
    for (uid, e) in doc.entities_with_uids() {
        let Entity::Space(s) = e else { continue };
        let x = &s.space;
        if x.void {
            continue;
        }
        let (name, usage) = (x.name.as_deref(), x.usage.as_deref());
        let own_dwelling = x
            .unit
            .as_deref()
            .and_then(|u| program.unit(u))
            .is_some_and(|u| u.kind == UnitKind::Unit && dwelling_nature(u.nature.as_deref()));
        let stair = kentos_domain::escape::stair_name(name, usage) && !own_dwelling;
        let w = words(name, usage);
        let lobby = has(&w, "GUVENLIK") && has(&w, "HOL");
        let level = e.own_level().map(str::to_owned);
        if stair && let Some(storey) = storeys.iter_mut().find(|l| Some(&l.level) == level.as_ref()) {
            storey.stair = true;
        }
        rooms.push((uid.to_string(), level, kentos_domain::arch_values::core_areas(&x.parts), stair, lobby));
    }
    let at = |level: &Option<String>, p: kentos_geometry_core::Vec2| {
        rooms.iter().find(|(_, l, areas, _, _)| l == level && areas.iter().any(|a| inside_area(a, p)))
    };
    let mut doors = Vec::new();
    for (uid, e) in doc.entities_with_uids() {
        let Entity::Opening(o) = e else { continue };
        let f = &o.opening;
        if f.form != OpeningForm::Door || f.operation == OpeningOperation::Fixed {
            continue;
        }
        let Some((_, left, right)) = kentos_domain::room_openings::opening_sides(f) else { continue };
        let level = doc.wall_of(&f.host).and_then(|w| w.arch.level);
        let (l, r) = (at(&level, left), at(&level, right));
        let opens_into = match (f.operation, f.side) {
            (OpeningOperation::Swing, OpeningSide::Right) => r.map(|x| x.0.clone()),
            (OpeningOperation::Swing, OpeningSide::Left) => l.map(|x| x.0.clone()),
            _ => None,
        };
        let fire_rating = f
            .fire_rating
            .as_ref()
            .or_else(|| {
                f.style
                    .as_deref()
                    .and_then(|t| doc.opening_types().iter().find(|x| x.id == t))
                    .and_then(|t| t.fire_rating.as_ref())
            })
            .map(|d| d.value.clone());
        doors.push(DoorFacts {
            uid: uid.to_string(),
            level,
            between: [l.map(|x| x.0.clone()), r.map(|x| x.0.clone())],
            opens_into,
            operation: serde_json::to_value(f.operation)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            fire_rating,
            stair: l.is_some_and(|x| x.3) || r.is_some_and(|x| x.3),
            lobby: l.is_some_and(|x| x.4) || r.is_some_and(|x| x.4),
        });
    }
    (people, doors)
}

/// Without Mimari's audits there are no people or doors to read.
#[cfg(not(feature = "mod-arch"))]
fn people_and_doors(_: &Document, _: &mut [StoreyHolds]) -> (Vec<RoomPeople>, Vec<DoorFacts>) {
    (Vec::new(), Vec::new())
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

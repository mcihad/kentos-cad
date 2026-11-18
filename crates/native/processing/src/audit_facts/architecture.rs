//! The drawing's architecture for the audit facts (`architecture`, docs/adr/8057 §39): its storeys, every room's net
//! area and outline to the walls' axes, the building program's article 4 areas (`Document::paiy_areas`) and the
//! escape routes (`kentos_domain::escape`), as KentOS Mimari works them out. Their order, rounding and bounds are
//! `kentos_extensions::audit_facts::architecture`'s.
//!
//! - **Only where there is architecture:** a drawing without a room gives no section.
//! - **The same bytes in any order:** the escape routes pick among equal walks, and the article's sums add, in the
//!   drawing's order; the measure reads a copy whose objects stand in persistent-id order, so the same objects give the
//!   same facts whatever order they were drawn or read in.
//! - **A room's areas as the article measures them:** its net area between the walls' inner faces and its outline to
//!   the walls' axes, each found again round its seed, else its drawn rings; its narrow side and its kind as Mimari
//!   denetim reads them (`kentos_domain::room_width`, `room_names`; docs/adr/8057 §53).

use std::collections::BTreeMap;

use kentos_commands::{Entity, SpaceFields, SpaceMethod, UnitKind};
use kentos_domain::arch_values::core_areas;
use kentos_domain::escape::{self, ExitRoute, Route, dwelling_nature, stair_name};
use kentos_domain::room_names::{self, kind_of};
use kentos_domain::room_width::{net_parts, width_of};
use kentos_domain::space_follow::SpaceSearch;
use kentos_domain::{Document, Slot};
use kentos_extensions::audit_facts::architecture::{
    ArchitectureFacts, DoorEscape, EscapeWalk, GroupKind, Occupancy, RoomEscape, RoomFacts,
    RoomKind, Storey, StoreyExits, UnitFacts, UnitLevel, ZoningFacts,
};
use kentos_geometry_core::geom::space::area;

/// The measure was asked to stop.
pub(super) struct Stopped;

/// `doc`'s architecture: none when it has no room.
pub(super) fn measure(
    doc: &Document,
    stopped: &dyn Fn() -> bool,
) -> Result<Option<ArchitectureFacts>, Stopped> {
    if !doc.entities().any(|e| matches!(e, Entity::Space(_))) {
        return Ok(None);
    }
    let ordered = by_persistent_id(doc);
    let doc = ordered.as_ref().unwrap_or(doc);
    let go = || if stopped() { Err(Stopped) } else { Ok(()) };
    go()?;
    let program = doc.program();
    let mut search = SpaceSearch::new(doc);
    let mut rooms: Vec<(Slot, RoomFacts)> = Vec::new();
    for (uid, e) in doc.entities_with_uids() {
        let Entity::Space(s) = e else { continue };
        let x = &s.space;
        let group = x.unit.as_deref().and_then(|u| program.unit(u));
        let dwelling =
            group.is_some_and(|u| u.kind == UnitKind::Unit && dwelling_nature(u.nature.as_deref()));
        let drawn = area(&core_areas(&x.parts));
        let broken = x.broken.is_some();
        // Its narrow side as Mimari denetim measures it (docs/adr/8057 §53): the largest circle in its net outline.
        let narrow = if broken {
            None
        } else {
            match width_of(&net_parts(&mut search, x)) {
                (w, Some(_)) => Some(w),
                (_, None) => None,
            }
        };
        rooms.push((
            Slot(s.base.id),
            RoomFacts {
                uid: uid.to_string(),
                number: x.number.clone(),
                name: x.name.clone(),
                usage: x.usage.clone(),
                kind: kind_of(x.name.as_deref(), x.usage.as_deref()).map(facts_kind),
                occupancy: kentos_domain::fire_use::use_of(x.name.as_deref(), x.usage.as_deref())
                    .filter(|u| *u != kentos_domain::fire_use::Use::Dwelling)
                    .map(row_of),
                level: e.own_level().map(str::to_owned),
                unit: x.unit.clone(),
                void: x.void,
                outdoor: x
                    .usage
                    .as_deref()
                    .and_then(|u| program.brief(u))
                    .is_some_and(|b| b.outdoor),
                broken,
                stair: !dwelling && stair_name(x.name.as_deref(), x.usage.as_deref()),
                net: search.net_area(x).unwrap_or(drawn),
                axis: outline(&mut search, x, SpaceMethod::Centerline),
                narrow,
                escape: None,
            },
        ));
    }
    go()?;
    let units = units(doc, &program);
    go()?;
    let walks = escape::escape(doc);
    go()?;
    let mut of_room: BTreeMap<Slot, RoomEscape> = walks
        .rooms
        .into_iter()
        .map(|r| {
            (
                r.room,
                RoomEscape {
                    exit: r.exit.map(exit_walk),
                    storey_exits: r.storey_exits,
                    entrance: r.entrance.map(walk),
                    has_entrance: r.has_entrance,
                    passages: r.passages as u64,
                },
            )
        })
        .collect();
    let rooms = rooms
        .into_iter()
        .map(|(slot, mut room)| {
            room.escape = of_room.remove(&slot);
            room
        })
        .collect();
    let doors = walks
        .doors
        .into_iter()
        .map(|d| DoorEscape {
            uid: doc.uid(d.door).map(|u| u.to_string()).unwrap_or_default(),
            unit: d.unit,
            level: d.level,
            at: [d.at.x, d.at.y],
            exit: d.exit.map(exit_walk),
            into_stair: d.into_stair,
        })
        .collect();
    let levels = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| {
            a.levels
                .iter()
                .map(|l| Storey {
                    id: l.id.clone(),
                    name: l.name.clone(),
                    elevation: l.elevation,
                    height: l.height,
                })
                .collect()
        })
        .unwrap_or_default();
    let exits = walks
        .exits
        .into_iter()
        .map(|(level, count)| StoreyExits {
            level,
            count: count as u64,
        })
        .collect();
    let sprinklers = doc
        .settings()
        .arch
        .as_ref()
        .is_some_and(|a| a.has_sprinklers());
    // The parcel's zoning and the areas the Emsal ve TAKS table reads (docs/adr/8030, 8057 §92), with where its emsal
    // and TAKS are read from (docs/adr/8140).
    let typed = doc.settings().arch.as_ref().and_then(|a| a.zoning.as_ref());
    let far_source = typed.and_then(|z| z.far_source.clone());
    let coverage_source = typed.and_then(|z| z.coverage_source.clone());
    let zoning = doc.zoning_report().map(|z| ZoningFacts {
        parcel: z.parcel,
        far: z.far_limit,
        coverage: z.coverage_limit,
        floors: z.floors,
        included: z.included,
        exempt: z.exempt,
        outside: z.outside,
        cap: z.cap,
        over: z.over,
        footprint: z.footprint,
        far_source,
        coverage_source,
    });
    Ok(Some(ArchitectureFacts {
        levels,
        exits,
        rooms,
        units,
        doors,
        sprinklers,
        zoning,
    }))
}

/// A room's kind in the facts' words, from KentOS Mimari's reading.
fn facts_kind(kind: room_names::RoomKind) -> RoomKind {
    match kind {
        room_names::RoomKind::Living => RoomKind::Living,
        room_names::RoomKind::Bedroom => RoomKind::Bedroom,
        room_names::RoomKind::Kitchen => RoomKind::Kitchen,
        room_names::RoomKind::Bath => RoomKind::Bath,
        room_names::RoomKind::Toilet => RoomKind::Toilet,
        room_names::RoomKind::Corridor => RoomKind::Corridor,
    }
}

/// A copy of `doc` with its objects in persistent-id order; none when the copy cannot be read back (the drawing's own
/// order is measured then).
fn by_persistent_id(doc: &Document) -> Option<Document> {
    let mut snapshot = doc.to_snapshot_v2_of(|_| true);
    let mut objects: Vec<_> = std::mem::take(&mut snapshot.uids)
        .into_iter()
        .zip(std::mem::take(&mut snapshot.entities))
        .collect();
    objects.sort_by_key(|(uid, _)| *uid);
    (snapshot.uids, snapshot.entities) = objects.into_iter().unzip();
    Document::from_snapshot_v2(snapshot).ok()
}

/// `s`'s outline to its walls' `method` line, m²: found again round its seed, else its drawn rings (as article 4
/// measures a room).
fn outline(search: &mut SpaceSearch<'_>, s: &SpaceFields, method: SpaceMethod) -> f64 {
    if s.seed.is_some() && s.method != method {
        let mut other = s.clone();
        other.method = method;
        let found = search.found_preserving_legacy(&other);
        if found.broken.is_none() && !found.parts.is_empty() {
            return area(&core_areas(&found.parts));
        }
    }
    area(&core_areas(&s.parts))
}

/// A unit's use as Ek-5/B's rows name it, read as Mimari denetim's fire rules read it (`kentos_domain::fire_use`): a
/// dwelling's nature the apartments' row, another nature by its words; an annex by its unit's; none for a common or
/// outdoor group and for words that name no use (docs/adr/8057 §79).
fn occupancy_of(
    program: &kentos_commands::BuildingProgram,
    u: &kentos_commands::ProgramUnit,
) -> Option<Occupancy> {
    use kentos_domain::fire_use::{Use, use_of};
    let owner = match u.kind {
        UnitKind::Unit => u,
        UnitKind::Annex => u.annex_of.as_deref().and_then(|o| program.unit(o))?,
        UnitKind::Common | UnitKind::Outdoor => return None,
    };
    let read = if dwelling_nature(owner.nature.as_deref()) {
        Use::Dwelling
    } else {
        use_of(owner.nature.as_deref(), None)?
    };
    Some(row_of(read))
}

/// A use as the facts name Ek-5/B's row.
fn row_of(read: kentos_domain::fire_use::Use) -> Occupancy {
    use kentos_domain::fire_use::Use;
    match read {
        Use::HighHazard => Occupancy::HighHazard,
        Use::Industry => Occupancy::Industry,
        Use::Dormitory => Occupancy::Dormitory,
        Use::Shop => Occupancy::Shop,
        Use::Office => Occupancy::Office,
        Use::Parking => Occupancy::Parking,
        Use::School => Occupancy::School,
        Use::Assembly => Occupancy::Assembly,
        Use::Hospital => Occupancy::Hospital,
        Use::Hotel => Occupancy::Hotel,
        Use::Dwelling => Occupancy::Dwelling,
    }
}

/// The program's groups with rooms and their article 4 areas, in the program's order.
fn units(doc: &Document, program: &kentos_commands::BuildingProgram) -> Vec<UnitFacts> {
    doc.paiy_areas()
        .into_iter()
        .filter_map(|(id, g)| {
            let u = program.unit(&id)?;
            Some(UnitFacts {
                number: u.number.clone(),
                name: u.name.clone(),
                kind: match u.kind {
                    UnitKind::Unit => GroupKind::Unit,
                    UnitKind::Common => GroupKind::Common,
                    UnitKind::Outdoor => GroupKind::Outdoor,
                    UnitKind::Annex => GroupKind::Annex,
                },
                nature: u.nature.clone(),
                annex_of: u.annex_of.clone(),
                dwelling: u.kind == UnitKind::Unit && dwelling_nature(u.nature.as_deref()),
                occupancy: occupancy_of(program, u),
                total: g.total(),
                general: g.general(),
                gross: g.gross,
                net: g.net,
                annex_gross: g.annex_gross,
                annex_net: g.annex_net,
                share: g.share,
                levels: g
                    .levels
                    .iter()
                    .map(|(level, gross, net)| UnitLevel {
                        level: level.clone(),
                        gross: *gross,
                        net: *net,
                    })
                    .collect(),
                id,
            })
        })
        .collect()
}

fn exit_walk(r: ExitRoute) -> EscapeWalk {
    EscapeWalk {
        one_way: Some(r.one_way),
        exits: Some(r.exits as u64),
        ..walk(r.route)
    }
}

fn walk(r: Route) -> EscapeWalk {
    EscapeWalk {
        length: r.length,
        one_way: None,
        exits: None,
        path: r.path.iter().map(|p| [p.x, p.y]).collect(),
    }
}

#[cfg(test)]
mod tests;

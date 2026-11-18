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
//! - **Compartments' parts** (docs/adr/8081): every room the storeys count with its regulation gross area, and what
//!   separates each two rooms of a storey (`Document::separations`): walls and openings with their declared fire classes
//!   (`arch_performance::fire_rating_of`: its own, else its type's), and the open stretches; the project's sprinklers.
//! - **Ek-3's elements** (docs/adr/8082): the frame's columns and beams, the walls (exterior or not), the floors
//!   (`Document::fire_elements`) with their declared classes; the building's height as the fire rules measure it and
//!   its basements' depth.
//! - **Storeys and doors as 28/1-c and 39/1 read them** (docs/adr/8083): each storey's height floor to floor (its own,
//!   else up to the storey over it) and its bedrooms (`kind_of`); the program's common areas; each door's net clear
//!   width (`OpeningSizes::clear`) and height (its head over its sill), what stands on its two sides (a room by its
//!   program group, a void, nothing) and whether one of them is a toilet (`kind_of`) or a balcony (Balkon).
//! - **The parcel's plan notes** (docs/adr/8084): the project's `planNotes` as written, the measure by its name.
//! - **The zone** (docs/adr/8085): the project's `zone` by its name; each storey's rooms of a dwelling unit and whether
//!   its name says Asma kat (a mezzanine, part of the ground storey).
//! - **Openings between storeys** (docs/adr/8086): `Document::vertical_links` (floor slabs' holes, void spaces that are
//!   no shaft, stairs) and each room's being an escape stair's.
//! - **Shafts, elevators' doors, roofs** (docs/adr/8087): each elevator's or service shaft's enclosure
//!   (`Document::shaft_enclosures`) with its walls' and openings' declared classes, its area, the shaft through the
//!   storeys it belongs to and a lift shaft's declared vent (docs/adr/8135); a door's being an elevator's (a side
//!   in an elevator's shaft) and opening onto a passage (a corridor, a hall, an entrance, a lobby, a landing, a stair's
//!   room); the structure's height (`fire_audit::heights`); roofs and terraces among Ek-3's elements.
//! - **Coverings** (docs/adr/8091): each covering's IFC type, material and the material's declared reaction class.
//! - **The structure** (docs/adr/8094): the project's declared structural system by its name and whether the building
//!   holds traditional architectural elements; the limited uses carry 22/1-m's traditional rooms. A traditional cut
//!   stone masonry building's stone walls (docs/adr/8095): their parts as Emsal ve TAKS tablosu counts them
//!   and the thick walls with no load-bearing declaration; with the parcel's emsal, the 30 % of 5/8 and the emsal harici
//!   counted within it. Emsal ve TAKS tablosu's count is read once (`Document::zoning_counted`).

use kentos_commands::{Entity, LocalRules, UnitKind};
use kentos_domain::Document;
use kentos_domain::room_names::{RoomKind, dwelling_nature, kind_of, technical};
use kentos_extensions::audit_facts::building::{
    BuildingFacts, CoveringFacts, DoorFacts, ElementFacts, EmsalLimit, ExemptCap, LinkFacts, MasonryFacts, PlanNoteFacts,
    RoofFacts, RoofLayerFacts, RoomPeople, SeparatingElement, SeparationFacts, ShaftFacts, ShaftVentFacts, SpaceFacts,
    StoreyHolds, UnderStairFacts, UnderStairRoom,
};

/// The drawing's building facts; empty (no storey) for a project without Mimari's storeys.
pub fn measure(doc: &Document) -> BuildingFacts {
    let Some(arch) = doc.settings().arch.as_ref() else {
        return BuildingFacts::default();
    };
    let program = doc.program();
    // Each storey's height floor to floor: its own, else up to the storey over it (28/1-c, docs/adr/8083).
    let mut by_elevation: Vec<&kentos_commands::ArchLevel> = arch.levels.iter().collect();
    by_elevation.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    let height_of = |id: &str| {
        let i = by_elevation.iter().position(|l| l.id == id)?;
        let l = by_elevation[i];
        let over = by_elevation.get(i + 1).map(|n| n.elevation - l.elevation).filter(|h| *h > 0.0);
        l.height.or(over)
    };
    let mut storeys: Vec<StoreyHolds> = arch
        .levels
        .iter()
        .map(|l| StoreyHolds {
            level: l.id.clone(),
            name: l.name.clone(),
            elevation: l.elevation,
            height: height_of(&l.id),
            // A mezzanine by its name (docs/adr/8085), as the inside heights read it (28/4).
            mezzanine: kentos_domain::room_names::has(&kentos_domain::room_names::words(Some(&l.name), None), "ASMA"),
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
        let kind = kind_of(name, usage);
        if matches!(kind, Some(RoomKind::Living | RoomKind::Bedroom | RoomKind::Kitchen)) {
            storey.inhabited += 1;
        }
        if kind == Some(RoomKind::Bedroom) {
            storey.bedrooms += 1;
        }
        let unit = x.unit.as_deref().and_then(|u| program.unit(u));
        if unit.is_some_and(|u| u.kind == UnitKind::Unit && dwelling_nature(u.nature.as_deref())) {
            storey.dwelling_rooms += 1;
        }
    }
    let (rooms, doors) = people_and_doors(doc, &mut storeys);
    // Emsal ve TAKS tablosu's count once: its limited uses, the rooms' gross, the stone walls and the 30 %.
    let counted = doc.zoning_counted();
    let limits = counted
        .limits
        .iter()
        .cloned()
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
    let gross: std::collections::HashMap<String, f64> =
        counted.rooms.iter().map(|r| (r.uid.clone(), r.gross)).collect();
    // An escape stair's room: named or used as Merdiven, no dwelling unit's own (the escape routes' reading).
    let stair_rooms: std::collections::HashSet<String> = doc
        .entities_with_uids()
        .filter_map(|(uid, e)| match e {
            Entity::Space(s) => {
                let x = &s.space;
                let own_dwelling = x
                    .unit
                    .as_deref()
                    .and_then(|u| program.unit(u))
                    .is_some_and(|u| u.kind == UnitKind::Unit && dwelling_nature(u.nature.as_deref()));
                (kentos_domain::escape::stair_name(x.name.as_deref(), x.usage.as_deref()) && !own_dwelling)
                    .then(|| uid.to_string())
            }
            _ => None,
        })
        .collect();
    // A hall's or a corridor's least width (docs/adr/8110): its narrowest neck in its net outline, the width a corridor
    // is by; the other rooms' narrow side is their largest circle (the architecture facts').
    let mut search = kentos_domain::space_follow::SpaceSearch::new(doc);
    let least: std::collections::HashMap<String, f64> = doc
        .entities_with_uids()
        .filter_map(|(uid, e)| match e {
            Entity::Space(s)
                if s.space.broken.is_none()
                    && kind_of(s.space.name.as_deref(), s.space.usage.as_deref()) == Some(RoomKind::Corridor) =>
            {
                let parts = kentos_domain::room_width::net_parts(&mut search, &s.space);
                kentos_domain::room_width::least_of(&parts).map(|w| (uid.to_string(), w.width))
            }
            _ => None,
        })
        .collect();
    // Each room's net floor area, between its walls' inner faces (docs/adr/8113; 54/5).
    let net: std::collections::HashMap<String, f64> = doc
        .entities_with_uids()
        .filter_map(|(uid, e)| match e {
            Entity::Space(s) if s.space.broken.is_none() => Some((uid.to_string(), search.net_area(&s.space)?)),
            _ => None,
        })
        .collect();
    // Each room's walk from its farthest point to its own nearest door, inside it (docs/adr/8111; 50/3).
    let to_door: std::collections::HashMap<String, f64> = kentos_domain::escape::escape(doc)
        .rooms
        .into_iter()
        .filter_map(|r| Some((doc.uid(r.room)?.to_string(), r.door?.length)))
        .collect();
    let spaces = storeys
        .iter()
        .flat_map(|st| st.rooms.iter().map(move |uid| (st.level.clone(), uid.clone())))
        .map(|(level, uid)| SpaceFacts {
            name: names.get(&uid).cloned().unwrap_or_default(),
            area: gross.get(&uid).copied().unwrap_or(0.0),
            level: Some(level),
            stair: stair_rooms.contains(&uid),
            least: least.get(&uid).copied(),
            to_door: to_door.get(&uid).copied(),
            net: net.get(&uid).copied(),
            uid,
        })
        .collect();
    let links = doc
        .vertical_links()
        .into_iter()
        .map(|l| LinkFacts {
            uid: l.uid,
            kind: l.kind.to_owned(),
            below: l.below,
            above: l.above,
            rooms_below: l.rooms_below,
            rooms_above: l.rooms_above,
            area: l.area,
            stair_well: l.stair_well,
            stack: l.stack,
        })
        .collect();
    let separations = separations(doc);
    let (elements, (height, structure_height)) = elements(doc);
    let enclosed = shaft_facts(doc, &names);
    let basement_depth = arch.levels.iter().map(|l| -l.elevation).fold(0.0_f64, f64::max);
    BuildingFacts {
        storeys,
        units: units.len() as u32,
        dwellings: dwellings as u32,
        commons: program.units.iter().filter(|u| u.kind == UnitKind::Common).count() as u32,
        local: arch.local_rules.map(|l| match l {
            LocalRules::Sivas => "sivas".to_owned(),
        }),
        plan_notes: arch
            .plan_notes
            .iter()
            .map(|n| PlanNoteFacts { key: n.key.name().to_owned(), value: n.value, source: n.source.clone() })
            .collect(),
        zone: arch.zone.map(|z| z.name().to_owned()),
        structural_system: arch.structure.map(|s| s.system.name().to_owned()),
        traditional: arch.structure.is_some_and(|s| s.traditional),
        masonry: arch.traditional_cut_stone().then(|| MasonryFacts {
            walls: counted.masonry,
            footprint: counted.masonry_footprint,
            thick: counted.masonry_walls.clone(),
            undeclared: counted.masonry_undeclared.clone(),
        }),
        exempt_cap: counted.cap.map(|cap| ExemptCap { cap, exempt: counted.exempt }),
        rooms,
        doors,
        limits,
        uses: program.buildings.iter().filter_map(|b| b.usage.clone()).collect(),
        under_stairs,
        spaces,
        separations,
        links,
        sprinklers: arch.has_sprinklers(),
        elements,
        height,
        basement_depth,
        structure_height,
        shafts: enclosed,
        coverings: coverings(doc),
        roofs: roofs(doc),
    }
}

/// Ek-3's elements with their declared classes, and the building's height as the fire rules measure it (docs/adr/8082).
#[cfg(feature = "mod-arch")]
fn elements(doc: &Document) -> (Vec<ElementFacts>, (f64, f64)) {
    use kentos_native_application::arch_performance::{exit_of, fire_rating_of, load_bearing_of};
    let by_uid: std::collections::HashMap<String, &Entity> =
        doc.entities_with_uids().map(|(u, e)| (u.to_string(), e)).collect();
    let elements = doc
        .fire_elements()
        .into_iter()
        .map(|x| ElementFacts {
            fire_rating: by_uid
                .get(&x.uid)
                .and_then(|e| fire_rating_of(doc.settings(), e))
                .map(|(d, _)| d.value.clone()),
            // The author's roles (docs/adr/8090): a wall's load-bearing, a roof's escape route. A column's declared
            // role is its `frame` (docs/adr/8124).
            load_bearing: (x.kind == "wall")
                .then(|| by_uid.get(&x.uid).and_then(|e| load_bearing_of(e)).map(|b| b.bearing))
                .flatten(),
            escape: x.kind == "roof" && by_uid.get(&x.uid).and_then(|e| exit_of(e)).is_some_and(|d| d.exit),
            uid: x.uid,
            kind: x.kind.to_owned(),
            level: x.level,
            frame: x.frame,
            exterior: x.exterior,
        })
        .collect();
    (elements, kentos_native_application::fire_audit::heights(doc))
}

/// Without Mimari's performance declarations there are no classes to read.
#[cfg(not(feature = "mod-arch"))]
fn elements(_: &Document) -> (Vec<ElementFacts>, (f64, f64)) {
    (Vec::new(), (0.0, 0.0))
}

/// What separates each two rooms of a storey, its walls' and openings' declared classes read as the performance command
/// writes them (docs/adr/8081).
#[cfg(feature = "mod-arch")]
fn separations(doc: &Document) -> Vec<SeparationFacts> {
    use kentos_commands::OpeningForm;
    use kentos_native_application::arch_performance::fire_rating_of;
    let by_uid: std::collections::HashMap<String, &Entity> =
        doc.entities_with_uids().map(|(u, e)| (u.to_string(), e)).collect();
    let element = |uid: &str, length: f64| {
        let e = by_uid.get(uid);
        let kind = match e {
            Some(Entity::Opening(o)) => match o.opening.form {
                OpeningForm::Door => "door",
                OpeningForm::Window => "window",
                _ => "gap",
            },
            _ => "wall",
        };
        SeparatingElement {
            uid: uid.to_owned(),
            kind: kind.to_owned(),
            fire_rating: e.and_then(|e| fire_rating_of(doc.settings(), e)).map(|(d, _)| d.value.clone()),
            length,
        }
    };
    doc.separations()
        .into_iter()
        .map(|s| SeparationFacts {
            walls: s.walls.iter().map(|(u, l)| element(u, *l)).collect(),
            openings: s.openings.iter().map(|(u, l)| element(u, *l)).collect(),
            level: s.level,
            rooms: s.rooms,
            open: s.open,
        })
        .collect()
}

/// The coverings with their materials' declared reaction to fire classes (docs/adr/8091; BYKHY 26/2, 26/4): a material
/// named by its id or its name (case aside) in the project's materials.
fn coverings(doc: &Document) -> Vec<CoveringFacts> {
    let materials = doc.settings().arch.as_ref().map_or(&[][..], |a| a.materials.as_slice());
    doc.entities_with_uids()
        .filter_map(|(uid, e)| {
            let Entity::Slab(s) = e else { return None };
            let c = s.slab.covering.as_ref()?;
            let material = s.slab.material.clone().filter(|m| !m.trim().is_empty());
            let reaction = material.as_deref().and_then(|name| {
                let folded = name.to_lowercase();
                materials
                    .iter()
                    .find(|m| m.id == name || m.name.to_lowercase() == folded)
                    .and_then(|m| m.reaction.as_ref())
                    .map(|d| d.value.clone())
            });
            Some(CoveringFacts {
                uid: uid.to_string(),
                level: e.own_level().map(str::to_owned),
                predefined: c.predefined.clone(),
                material,
                reaction,
            })
        })
        .collect()
}

/// The roofs with their layers' materials and those materials' declared classes (docs/adr/8134; BYKHY 28/2, 28/3): a
/// material named by its id or its name (case aside) in the project's materials; a roof without layers one covering of
/// its own material.
fn roofs(doc: &Document) -> Vec<RoofFacts> {
    let materials = doc.settings().arch.as_ref().map_or(&[][..], |a| a.materials.as_slice());
    let layer = |kind: &str, material: Option<String>| {
        let found = material.as_deref().and_then(|name| {
            let folded = name.to_lowercase();
            materials.iter().find(|m| m.id == name || m.name.to_lowercase() == folded)
        });
        RoofLayerFacts {
            kind: kind.to_owned(),
            reaction: found.and_then(|m| m.reaction.as_ref()).map(|d| d.value.clone()),
            roof_class: found.and_then(|m| m.roof_class.as_ref()).map(|d| d.value.clone()),
            material,
        }
    };
    doc.entities_with_uids()
        .filter_map(|(uid, e)| {
            let Entity::Roof(r) = e else { return None };
            let f = &r.roof;
            let layers = if f.layers.is_empty() {
                vec![layer("covering", Some(f.material_name().to_owned()))]
            } else {
                // Each layer's kind as the file spells it (`covering`, `battens` …).
                f.layers
                    .iter()
                    .map(|l| {
                        let kind = serde_json::to_value(l.kind).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default();
                        layer(&kind, l.material.clone().filter(|m| !m.trim().is_empty()))
                    })
                    .collect()
            };
            Some(RoofFacts { uid: uid.to_string(), level: e.own_level().map(str::to_owned), layers })
        })
        .collect()
}

/// Each shaft and what encloses it (docs/adr/8087), the walls' and openings' declared classes read as the separations'.
#[cfg(feature = "mod-arch")]
fn shaft_facts(doc: &Document, names: &std::collections::HashMap<String, String>) -> Vec<ShaftFacts> {
    use kentos_commands::OpeningForm;
    use kentos_native_application::arch_performance::fire_rating_of;
    let by_uid: std::collections::HashMap<String, &Entity> =
        doc.entities_with_uids().map(|(u, e)| (u.to_string(), e)).collect();
    let element = |uid: &str, length: f64| {
        let e = by_uid.get(uid);
        let kind = match e {
            Some(Entity::Opening(o)) => match o.opening.form {
                OpeningForm::Door => "door",
                OpeningForm::Window => "window",
                _ => "gap",
            },
            _ => "wall",
        };
        SeparatingElement {
            uid: uid.to_owned(),
            kind: kind.to_owned(),
            fire_rating: e.and_then(|e| fire_rating_of(doc.settings(), e)).map(|(d, _)| d.value.clone()),
            length,
        }
    };
    // A lift shaft's declared vent (docs/adr/8135), as its room holds it.
    let vent = |uid: &str| {
        let v = by_uid.get(uid)?.arch_info()?.shaft_vent.as_ref()?;
        Some(ShaftVentFacts { area: v.area, pressurized: v.pressurized })
    };
    doc.shaft_enclosures()
        .into_iter()
        .map(|x| ShaftFacts {
            name: names.get(&x.uid).cloned().unwrap_or_default(),
            walls: x.walls.iter().map(|(u, l)| element(u, *l)).collect(),
            openings: x.openings.iter().map(|(u, l)| element(u, *l)).collect(),
            kind: x.kind.to_owned(),
            level: x.level,
            open: x.open,
            area: x.area,
            vent: if x.kind == "elevator" { vent(&x.uid) } else { None },
            stack: x.stack,
            uid: x.uid,
        })
        .collect()
}

/// Without Mimari's performance declarations there are no classes to read.
#[cfg(not(feature = "mod-arch"))]
fn shaft_facts(_: &Document, _: &std::collections::HashMap<String, String>) -> Vec<ShaftFacts> {
    Vec::new()
}

/// Without Mimari's performance declarations there is nothing to read the separations' classes by.
#[cfg(not(feature = "mod-arch"))]
fn separations(_: &Document) -> Vec<SeparationFacts> {
    Vec::new()
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
    // What a door's side is as 39/1 reads it (docs/adr/8083): by uid, its program group's kind (or `room`, `void`), a
    // toilet, a balcony; the voids' outlines apart, found when no room is.
    let mut what: std::collections::HashMap<String, (&'static str, bool, bool)> = std::collections::HashMap::new();
    let mut voids: Vec<(Option<String>, Vec<kentos_geometry_core::geom::arrangement::Area>)> = Vec::new();
    // The elevators' shafts (BYKHY 62/4, Ek-3/B 12) and the passages an elevator may open onto (docs/adr/8087).
    let mut elevators: Vec<(Option<String>, Vec<kentos_geometry_core::geom::arrangement::Area>)> = Vec::new();
    let mut passages: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (uid, e) in doc.entities_with_uids() {
        let Entity::Space(s) = e else { continue };
        let x = &s.space;
        let (name, usage) = (x.name.as_deref(), x.usage.as_deref());
        let balcony = has(&words(name, usage), "BALKON");
        if x.void {
            let areas = kentos_domain::arch_values::core_areas(&x.parts);
            if kentos_domain::room_names::shaft_of(name, usage) == Some(kentos_domain::room_names::ShaftKind::Elevator) {
                elevators.push((e.own_level().map(str::to_owned), areas.clone()));
            }
            voids.push((e.own_level().map(str::to_owned), areas));
            continue;
        }
        let group = match x.unit.as_deref().and_then(|u| program.unit(u)).map(|u| u.kind) {
            Some(UnitKind::Unit) => "unit",
            Some(UnitKind::Common) => "common",
            Some(UnitKind::Annex) => "annex",
            Some(UnitKind::Outdoor) => "outdoor",
            None => "room",
        };
        what.insert(uid.to_string(), (group, kind_of(name, usage) == Some(RoomKind::Toilet), balcony));
        let own_dwelling = x
            .unit
            .as_deref()
            .and_then(|u| program.unit(u))
            .is_some_and(|u| u.kind == UnitKind::Unit && dwelling_nature(u.nature.as_deref()));
        let stair = kentos_domain::escape::stair_name(name, usage) && !own_dwelling;
        let w = words(name, usage);
        if kind_of(name, usage) == Some(RoomKind::Corridor)
            || kentos_domain::room_names::passage(name, usage)
            || has(&w, "LOBI")
            || has(&w, "SAHANLIK")
            || kentos_domain::escape::stair_name(name, usage)
        {
            passages.insert(uid.to_string());
        }
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
        // A side's room by its group, else a void there, else nothing.
        let side = |room: Option<&String>, p: kentos_geometry_core::Vec2| match room {
            Some(uid) => what.get(uid).map(|w| w.0.to_owned()),
            None => voids
                .iter()
                .any(|(l, areas)| *l == level && areas.iter().any(|a| inside_area(a, p)))
                .then(|| "void".to_owned()),
        };
        let sides = [side(l.map(|x| &x.0), left), side(r.map(|x| &x.0), right)];
        let sides_what: Vec<&(&str, bool, bool)> = [l, r].iter().flatten().filter_map(|x| what.get(&x.0)).collect();
        let elevator = [left, right]
            .iter()
            .any(|p| elevators.iter().any(|(lv, areas)| *lv == level && areas.iter().any(|a| inside_area(a, *p))));
        let passage = [l, r].iter().flatten().any(|x| passages.contains(&x.0));
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
        let sizes = f.sizes();
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
            clear: sizes.clear,
            height: f.head.map(|head| head - f.sill.unwrap_or(0.0)),
            // A double door's leaves and its first leaf's net clear (docs/adr/8093; PAİY 39/1-b).
            leaves: if sizes.leaves.len() > 1 { sizes.leaves.clone() } else { Vec::new() },
            clear_first: sizes.clear_first,
            sides,
            toilet: sides_what.iter().any(|w| w.1),
            balcony: sides_what.iter().any(|w| w.2),
            elevator,
            passage,
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

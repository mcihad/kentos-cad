//! The KentOS project file, `.kcad` v2 (docs/specs/kcad-v2.md, docs/adr/0025):
//! a versioned binary snapshot for saving and loading only, never a database,
//! index or tile store (docs/adr/0011).
//!
//! - **Container** (`header`): the `\x89KCAD\r\n\x1a\n` signature, versions,
//!   the payload's encoding and codec, lengths, required extensions and a
//!   SHA-256 trailer that detects corruption (it is not a signature).
//! - **KentOS CBOR profile 1** (`cbor`): definite lengths, shortest integers,
//!   binary64 floats only (NaN and infinities refused, −0 kept), text keys in
//!   RFC 8949 §4.2.1 order without duplicates, no tags; depth, length and size
//!   limits. Written here: no CBOR crate (owner's decision, 2026-09-26).
//! - **Document schemas 2 to 7** (`encode`, `decode`): the contract
//!   `DocumentSnapshotV2`, every object with its 16-byte persistent id
//!   (docs/adr/0014); the open document's slots never enter the file. Schema 3
//!   adds an object's own line weight, schema 4 vertex elevations, schema 5
//!   multi-part areas, schema 6 blocks, schema 7 a text's alignment, width
//!   factor and mask; a writer writes the oldest schema that holds what the
//!   drawing has.
//!
//! One implementation for every platform (CLAUDE.md §14): the desktop app and
//! the server call it natively, the browser through the formats WASM module
//! (`kentos-formats-wasm`). An independent reader in Python
//! (`tools/kcad/kcad.py`) and the byte-level fixtures (`fixtures/kcad/v2`)
//! hold it to the specification.
//!
//! Large drawings (docs/adr/0030): every read and write can be watched and
//! stopped (`watch`: the project first, then the objects, a few thousand at a
//! time); the browser hands its drawings to and from the codec as typed
//! columns (`columns`), not as JSON text.
//!
//! Rules: nothing a file holds may crash the reader or make it reserve more
//! memory than the file itself could fill; a drawing is written only if a
//! reader would take it back; the same drawing always gives the same bytes.
#![forbid(unsafe_code)]
// A panic traps the WASM module; user data must never cause one (docs/adr/0008).
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod blocks;
mod cbor;
pub mod columns;
mod decode;
mod encode;
mod error;
mod header;
mod opening;
pub mod salvage;
mod schemas;
mod space;
mod watch;

use kentos_contracts::DocumentSnapshotV2;
use serde::Serialize;

pub use cbor::{MAX_DEPTH, MAX_ITEMS, MAX_STRING};
pub use columns::Columns;
pub use error::{Code, KcadError};
pub use header::{
    CODEC_NONE, ENCODING_CBOR_PROFILE_1, FIXED_HEADER, HASH_SIZE, Header, MAGIC, MAJOR,
    MAX_PAYLOAD, MINOR,
};
pub use kentos_contracts as contracts;
pub use watch::{Quiet, Step, Watch};

/// The file extension (with the dot).
pub const EXTENSION: &str = ".kcad";
/// The media type: no registered KCAD type exists, so none is claimed (TODOS.md FILE-13).
pub const MIME: &str = "application/octet-stream";

// The document schemas, each the feature it brought (`schemas.rs`).
pub use schemas::*;

/// The modules a root's `modules` may name: key, name for people, and the version of its data this build reads (0: not
/// raised). A file with a newer one, an unknown module or an extension's data is refused before its document is read,
/// as written by a newer KentOS (docs/adr/8054 §10).
pub const MODULE_VERSIONS: [(&str, &str, u16); 4] = [
    ("arch", "mimari", ARCH_WITH_GROUND),
    ("gis", "CBS", GIS_WITH_LAYER_ORIGINS),
    ("map", "harita", 0),
    ("road", "yol", 0),
];

/// Architecture data version 1 (docs/adr/8068): the fire resistance classes (an object's `arch.fireRating`, an
/// opening's `fireRating`, a wall style's and a door or window type's), the exit roles (`exit`), a material's
/// `strength` and the building's `performance`. The document schema stays as it is.
pub const ARCH_WITH_PERFORMANCE: u16 = 1;

/// Whether `doc` holds architecture data 1 ([`ARCH_WITH_PERFORMANCE`]): what its writer names and an older reader
/// refuses.
pub fn uses_performance(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_performance_in(doc, doc.entities.iter())
}

fn uses_performance_in<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    use kentos_contracts::Entity;
    let stamped = objects.any(|e| match e {
        Entity::Opening(o) => o.opening.fire_rating.is_some() || o.opening.exit.is_some(),
        _ => e.arch_info().is_some_and(|a| a.fire_rating.is_some() || a.exit.is_some()),
    });
    let styles = doc.settings.wall_styles.iter().any(|s| s.fire_rating.is_some());
    let arch = doc.settings.arch.as_ref().is_some_and(|a| {
        a.performance.as_ref().is_some_and(|p| !p.is_empty())
            || a.opening_types.iter().any(|t| t.fire_rating.is_some())
            || a.materials.iter().any(|m| m.strength.is_some())
    });
    stamped || styles || arch
}

/// Architecture data version 2 (docs/adr/8070 §5): a block card's building element (`card.element`: its class, type and
/// body). Version 2 holds version 1's data too. The document schema stays as it is.
pub const ARCH_WITH_ELEMENTS: u16 = 2;

/// Whether `doc` holds architecture data 2 ([`ARCH_WITH_ELEMENTS`]): a block whose card says what building element it
/// is.
pub fn uses_elements(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.blocks
        .iter()
        .any(|b| b.card.as_ref().is_some_and(|c| c.element.is_some()))
}

/// Architecture data version 3 (docs/adr/8070 §7): a curtain wall's grid (a wall's `curtain`). Version 3 holds versions
/// 1's and 2's data too. The document schema stays as it is.
pub const ARCH_WITH_CURTAIN_WALLS: u16 = 3;

/// Whether `doc` holds architecture data 3 ([`ARCH_WITH_CURTAIN_WALLS`]): a curtain wall.
pub fn uses_curtain_walls(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_curtain_walls_in(doc, doc.entities.iter())
}

fn uses_curtain_walls_in<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    let wall = |e: &kentos_contracts::Entity| matches!(e, kentos_contracts::Entity::Wall(w) if w.wall.curtain.is_some());
    objects.any(wall) || doc.blocks.iter().any(|b| b.entities.iter().any(wall))
}

/// Architecture data version 4 (docs/adr/8070 §8): a post or a rail (a column's or a beam's `member`) and a column's own
/// height. Version 4 holds versions 1–3's data too. The document schema stays as it is.
pub const ARCH_WITH_MEMBERS: u16 = 4;

/// Whether `doc` holds architecture data 4 ([`ARCH_WITH_MEMBERS`]): a post, a rail or a column of its own height, in
/// the drawing or a block.
pub fn uses_members(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_members_in(doc, doc.entities.iter())
}

fn uses_members_in<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    let member = |e: &kentos_contracts::Entity| match e {
        kentos_contracts::Entity::Column(c) => c.column.member.is_some() || c.column.height.is_some(),
        kentos_contracts::Entity::Beam(b) => b.beam.member.is_some(),
        _ => false,
    };
    objects.any(member) || doc.blocks.iter().any(|b| b.entities.iter().any(member))
}

/// Architecture data version 5 (docs/adr/8070 §10): a covering (a slab's `covering`). Version 5 holds versions 1–4's data
/// too. The document schema stays as it is.
pub const ARCH_WITH_COVERINGS: u16 = 5;

/// Whether `doc` holds architecture data 5 ([`ARCH_WITH_COVERINGS`]): a covering.
pub fn uses_coverings(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_coverings_in(doc.entities.iter())
}

fn uses_coverings_in<'a>(
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    objects.any(kentos_contracts::is_covering)
}

/// Architecture data version 6 (docs/adr/8070 §11): a schedule of beams or of coverings. Version 6 holds versions 1–5's
/// data too. The document schema stays as it is.
pub const ARCH_WITH_ELEMENT_SCHEDULES: u16 = 6;

/// Whether `doc` holds architecture data 6 ([`ARCH_WITH_ELEMENT_SCHEDULES`]): a beams' or coverings' schedule.
pub fn uses_element_schedules(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_element_schedules_in(doc, doc.entities.iter())
}

fn uses_element_schedules_in<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    use kentos_contracts::{Entity, ScheduleOf, TableSource};
    let schedule = |e: &Entity| {
        matches!(e, Entity::Table(t) if matches!(
            t.source,
            Some(TableSource::Schedule { of: ScheduleOf::Beams | ScheduleOf::Coverings, .. })
        ))
    };
    objects.any(schedule) || doc.blocks.iter().any(|b| b.entities.iter().any(schedule))
}

/// Architecture data version 7 (docs/adr/8071 §6): a sloped slab (a slab's `slope`). Version 7 holds versions 1–6's
/// data too. The document schema stays as it is.
pub const ARCH_WITH_SLAB_SLOPES: u16 = 7;

/// Whether `doc` holds architecture data 7 ([`ARCH_WITH_SLAB_SLOPES`]): a sloped slab, in the drawing or a block.
pub fn uses_slab_slopes(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_slab_slopes_in(doc, doc.entities.iter())
}

fn uses_slab_slopes_in<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    let sloped = |e: &kentos_contracts::Entity| matches!(e, kentos_contracts::Entity::Slab(s) if s.slab.slope.is_some());
    objects.any(sloped) || doc.blocks.iter().any(|b| b.entities.iter().any(sloped))
}

/// Architecture data version 8 (docs/adr/8071 §8): a roof's layers. Version 8 holds versions 1–7's data too. The
/// document schema stays as it is.
pub const ARCH_WITH_ROOF_LAYERS: u16 = 8;

/// Whether `doc` holds architecture data 8 ([`ARCH_WITH_ROOF_LAYERS`]): a roof with layers, in the drawing or a block.
pub fn uses_roof_layers(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_roof_layers_in(doc, doc.entities.iter())
}

fn uses_roof_layers_in<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    let layered = |e: &kentos_contracts::Entity| matches!(e, kentos_contracts::Entity::Roof(r) if !r.roof.layers.is_empty());
    objects.any(layered) || doc.blocks.iter().any(|b| b.entities.iter().any(layered))
}

/// Architecture data version 9 (docs/adr/8072 §1): a program building's code, use or ±0.00, a shared level's building.
/// Version 9 holds versions 1–8's data too. The document schema stays as it is.
pub const ARCH_WITH_BUILDING_PLACES: u16 = 9;

/// Whether `doc` holds architecture data 9 ([`ARCH_WITH_BUILDING_PLACES`]).
pub fn uses_building_places(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(|a| a.program.has_building_places())
}

/// Architecture data version 10 (docs/adr/8077): the author's answers to the model quality control form
/// (`settings.arch.formAnswers`). Version 10 holds versions 1–9's data too. The document schema stays as it is.
pub const ARCH_WITH_FORM_ANSWERS: u16 = 10;

/// Whether `doc` holds architecture data 10 ([`ARCH_WITH_FORM_ANSWERS`]).
pub fn uses_form_answers(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(|a| !a.form_answers.is_empty())
}

/// Architecture data version 11 (docs/adr/8084): the parcel's plan notes the regulation's rules read
/// (`settings.arch.planNotes`). Version 11 holds versions 1–10's data too. The document schema stays as it is.
pub const ARCH_WITH_PLAN_NOTES: u16 = 11;

/// Whether `doc` holds architecture data 11 ([`ARCH_WITH_PLAN_NOTES`]).
pub fn uses_plan_notes(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_plan_notes)
}

/// Architecture data version 12 (docs/adr/8085): the parcel's area's use in the plan (`settings.arch.zone`). Version 12
/// holds versions 1–11's data too. The document schema stays as it is.
pub const ARCH_WITH_ZONE: u16 = 12;

/// Whether `doc` holds architecture data 12 ([`ARCH_WITH_ZONE`]).
pub fn uses_zone(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_zone)
}

/// Architecture data version 13 (docs/adr/8090): a wall's declared load-bearing role (`arch.loadBearing`) and a roof's
/// escape route role (a roof's `arch.exit`). Version 13 holds versions 1–12's data too. The document schema stays as it
/// is.
pub const ARCH_WITH_DECLARED_ROLES: u16 = 13;

/// Whether `doc` holds architecture data 13 ([`ARCH_WITH_DECLARED_ROLES`]).
pub fn uses_declared_roles(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_declared_roles_in(doc.entities.iter())
}

fn uses_declared_roles_in<'a>(
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    use kentos_contracts::Entity;
    objects.any(|e| {
        e.arch_info().is_some_and(|a| a.load_bearing.is_some() || (matches!(e, Entity::Roof(_)) && a.exit.is_some()))
    })
}

/// Architecture data version 14 (docs/adr/8091): a material's reaction to fire class (`settings.arch.materials[].reaction`).
/// Version 14 holds versions 1–13's data too. The document schema stays as it is.
pub const ARCH_WITH_MATERIAL_REACTIONS: u16 = 14;

/// Whether `doc` holds architecture data 14 ([`ARCH_WITH_MATERIAL_REACTIONS`]).
pub fn uses_material_reactions(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(|a| a.materials.iter().any(|m| m.reaction.is_some()))
}

/// Architecture data version 15 (docs/adr/8094): the building's declared structural system (`settings.arch.structure`).
/// Version 15 holds versions 1–14's data too. The document schema stays as it is.
pub const ARCH_WITH_STRUCTURE: u16 = 15;

/// Whether `doc` holds architecture data 15 ([`ARCH_WITH_STRUCTURE`]).
pub fn uses_structure(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_structure)
}

/// Architecture data version 16 (docs/adr/8098): a plan note of the plan's building height or storey count
/// (`settings.arch.planNotes[].key`: `buildingHeight`, `storeys`). Version 16 holds versions 1–15's data too. The document
/// schema stays as it is.
pub const ARCH_WITH_PLAN_LIMITS: u16 = 16;

/// Whether `doc` holds architecture data 16 ([`ARCH_WITH_PLAN_LIMITS`]).
pub fn uses_plan_limits(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_plan_limits)
}

/// Architecture data version 17 (docs/adr/8099): the delivery record (`settings.arch.deliveries`). Version 17 holds
/// versions 1–16's data too. The document schema stays as it is.
pub const ARCH_WITH_DELIVERIES: u16 = 17;

/// Whether `doc` holds architecture data 17 ([`ARCH_WITH_DELIVERIES`]).
pub fn uses_deliveries(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_deliveries)
}

/// Architecture data version 18 (docs/adr/8101): the underlay models (`settings.arch.underlays`). Version 18 holds
/// versions 1–17's data too. The document schema stays as it is.
pub const ARCH_WITH_UNDERLAYS: u16 = 18;

/// Whether `doc` holds architecture data 18 ([`ARCH_WITH_UNDERLAYS`]).
pub fn uses_underlays(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_underlays)
}

/// Architecture data version 19 (docs/adr/8106): the project's element types (`settings.arch.elementTypes`) and the
/// type an element follows (`arch.elementType`). Version 19 holds versions 1–18's data too. The document schema stays
/// as it is.
pub const ARCH_WITH_ELEMENT_TYPES: u16 = 19;

/// Whether `doc` holds architecture data 19 ([`ARCH_WITH_ELEMENT_TYPES`]).
pub fn uses_element_types(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_element_types_in(doc, doc.entities.iter())
}

fn uses_element_types_in<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_element_types)
        || objects.any(|e| e.arch_info().is_some_and(|a| a.element_type.is_some()))
}

/// Architecture data version 20 (docs/adr/8124): a wall's or a slab's declared exterior role (`arch.external`) and a
/// column's declared load-bearing role (its `arch.loadBearing`), over what the drawing says of them. Version 20 holds
/// versions 1–19's data too. The document schema stays as it is.
pub const ARCH_WITH_DECLARED_READINGS: u16 = 20;

/// Whether `doc` holds architecture data 20 ([`ARCH_WITH_DECLARED_READINGS`]).
pub fn uses_declared_readings(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_declared_readings_in(doc.entities.iter())
}

fn uses_declared_readings_in<'a>(
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    use kentos_contracts::Entity;
    objects.any(|e| {
        e.arch_info().is_some_and(|a| a.external.is_some() || (matches!(e, Entity::Column(_)) && a.load_bearing.is_some()))
    })
}

/// Architecture data version 21 (docs/adr/8125): a room's own emsal state (`emsal`: included, exempt, outside or an
/// emsal increase with its basis). Version 21 holds versions 1–20's data too. The document schema stays as it is.
pub const ARCH_WITH_ROOM_EMSAL: u16 = 21;

/// Whether `doc` holds architecture data 21 ([`ARCH_WITH_ROOM_EMSAL`]).
pub fn uses_room_emsal(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_room_emsal_in(doc.entities.iter())
}

fn uses_room_emsal_in<'a>(
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    objects.any(|e| matches!(e, kentos_contracts::Entity::Space(s) if s.space.emsal.is_some()))
}

/// Architecture data version 22 (docs/adr/8130): a sloped member (a beam's `rise`). Version 22 holds versions 1–21's
/// data too. The document schema stays as it is.
pub const ARCH_WITH_SLOPED_MEMBERS: u16 = 22;

/// Whether `doc` holds architecture data 22 ([`ARCH_WITH_SLOPED_MEMBERS`]): a sloped member, in the drawing or a block.
pub fn uses_sloped_members(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_sloped_members_in(doc, doc.entities.iter())
}

fn uses_sloped_members_in<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    objects.any(kentos_contracts::is_sloped_member)
        || doc.blocks.iter().any(|b| b.entities.iter().any(kentos_contracts::is_sloped_member))
}

/// Architecture data version 23 (docs/adr/8134): a material's external fire performance class as a roof covering
/// (`settings.arch.materials[].roofClass`, TS EN 13501-5). Version 23 holds versions 1–22's data too. The document
/// schema stays as it is.
pub const ARCH_WITH_ROOF_CLASSES: u16 = 23;

/// Whether `doc` holds architecture data 23 ([`ARCH_WITH_ROOF_CLASSES`]).
pub fn uses_roof_classes(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(|a| a.materials.iter().any(|m| m.roof_class.is_some()))
}

/// Architecture data version 35 (docs/adr/8163): a building of the program giving its own plan notes
/// (`settings.arch.program.buildings[].planNotes`). Version 35 holds versions 1–34's data too. The document schema stays
/// as it is.
pub const ARCH_WITH_BUILDING_PLAN_NOTES: u16 = 35;

/// Architecture data version 36: individually bound slab edges, in the drawing or a block.
pub const ARCH_WITH_SLAB_EDGE_BINDINGS: u16 = 36;
/// Architecture data version 37: an upward view interval in a named recipe or view template.
pub const ARCH_WITH_CEILING_RECIPE: u16 = 37;

/// First architecture reader that preserves electrical appliance cards.
pub const ARCH_WITH_ELECTRIC_APPLIANCES: u16 = 38;
/// Storey and per-leg filler choice in a kitchen recipe.
pub const ARCH_WITH_CASEWORK_PLACEMENT: u16 = 39;
/// Architecture data version 40 (docs/adr/8172): a footing, a beam of a pad or strip footing role (`role`
/// `padFooting`, `stripFooting`), in the drawing or a block. Version 40 holds versions 1–39's data too. The document
/// schema stays as it is.
pub const ARCH_WITH_FOOTINGS: u16 = 40;

/// Architecture data version 41 (docs/adr/8172 §6): a raft, a slab of the raft role (`role` `raft`), in the drawing or
/// a block. Version 41 holds versions 1–40's data too. The document schema stays as it is.
pub const ARCH_WITH_RAFTS: u16 = 41;

/// Whether `doc` holds architecture data 40 ([`ARCH_WITH_FOOTINGS`]): a footing.
pub fn uses_footings(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_footings_in(doc, doc.entities.iter())
}

/// Whether `doc` holds architecture data 41 ([`ARCH_WITH_RAFTS`]): a raft.
pub fn uses_rafts(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_rafts_in(doc, doc.entities.iter())
}

/// Architecture data version 42 (docs/adr/8173): a profile, a beam's band covering a wall (`covering`: a facade
/// moulding, a coping), in the drawing. Version 42 holds versions 1–41's data too. The document schema stays as it is.
pub const ARCH_WITH_PROFILES: u16 = 42;

/// Architecture data version 43 (docs/adr/8174): the site plan's natural and graded ground (`naturalGround`,
/// `gradedGround`: each its survey's layers). Version 43 holds versions 1–42's data too.
pub const ARCH_WITH_GROUND: u16 = 43;

/// Whether `doc` holds architecture data 43 ([`ARCH_WITH_GROUND`]): a site ground.
pub fn uses_ground(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings
        .arch
        .as_ref()
        .and_then(|a| a.site.as_ref())
        .is_some_and(|s| s.natural_ground.is_some() || s.graded_ground.is_some())
}

/// Whether `doc` holds architecture data 42 ([`ARCH_WITH_PROFILES`]): a profile.
pub fn uses_profiles(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_profiles_in(doc.entities.iter())
}

fn uses_profiles_in<'a>(mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>) -> bool {
    objects.any(|e| matches!(e, kentos_contracts::Entity::Beam(b) if b.beam.covering.is_some()))
}

fn uses_rafts_in<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    let raft = |e: &kentos_contracts::Entity| {
        matches!(e, kentos_contracts::Entity::Slab(s) if s.slab.role == kentos_contracts::SlabRole::Raft)
    };
    objects.any(raft) || doc.blocks.iter().any(|b| b.entities.iter().any(raft))
}

fn uses_footings_in<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    let footing = |e: &kentos_contracts::Entity| matches!(e, kentos_contracts::Entity::Beam(b) if b.beam.role.is_footing());
    objects.any(footing) || doc.blocks.iter().any(|b| b.entities.iter().any(footing))
}

pub fn uses_casework_placement(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.casework.iter().map(|r| &r.recipe)
        .chain(doc.settings.arch.iter().flat_map(|a| &a.modules).flat_map(|m| &m.runs).map(|r| &r.recipe))
        .any(|r| r.level.is_some() || !r.filler_ends.is_empty())
}

pub fn uses_electric_appliances(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.blocks.iter().filter_map(|b| b.card.as_ref()?.element.as_ref())
        .any(|e| e.class == kentos_contracts::BlockElementClass::ElectricAppliance)
}

pub fn uses_ceiling_recipe(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.named_views.iter().any(|v| v.recipe.as_ref().is_some_and(|r| r.ceiling.is_some()))
        || doc.settings.view_templates.iter().any(|t| t.recipe.ceiling.is_some())
}

pub fn uses_slab_edge_bindings(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_slab_edge_bindings_in(doc, doc.entities.iter())
}

fn uses_slab_edge_bindings_in<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    let mixed = |e: &kentos_contracts::Entity| matches!(e, kentos_contracts::Entity::Slab(s) if s.slab.bound.as_ref().is_some_and(|b| b.edges.is_some()));
    objects.any(mixed) || doc.blocks.iter().any(|b| b.entities.iter().any(mixed))
}

/// Whether `doc` holds architecture data 35 ([`ARCH_WITH_BUILDING_PLAN_NOTES`]).
pub fn uses_building_plan_notes(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(|a| a.program.has_building_plan_notes())
}

/// Architecture data version 34 (docs/adr/8162): a plan note giving the plan's projection or basement limit
/// (`settings.arch.planNotes[]` of `projectionMost`, `basementsMost`). Version 34 holds versions 1–33's data too. The
/// document schema stays as it is.
pub const ARCH_WITH_PLAN_MASS: u16 = 34;

/// Whether `doc` holds architecture data 34 ([`ARCH_WITH_PLAN_MASS`]).
pub fn uses_plan_mass(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_plan_mass)
}

/// Architecture data version 33 (docs/adr/8160): a plan note giving the plan's roof ridge, slope or eaves
/// (`settings.arch.planNotes[]` of `ridgeMost`, `roofSlopeMost`, `eavesMost`). Version 33 holds versions 1–32's data
/// too. The document schema stays as it is.
pub const ARCH_WITH_PLAN_ROOF: u16 = 33;

/// Whether `doc` holds architecture data 33 ([`ARCH_WITH_PLAN_ROOF`]).
pub fn uses_plan_roof(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_plan_roof)
}

/// Architecture data version 32 (docs/adr/8158): a plan note giving the plan's subasman (`settings.arch.planNotes[]`
/// of `plinthMost` or `plinthLeast`). Version 32 holds versions 1–31's data too. The document schema stays as it is.
pub const ARCH_WITH_PLAN_PLINTH: u16 = 32;

/// Whether `doc` holds architecture data 32 ([`ARCH_WITH_PLAN_PLINTH`]).
pub fn uses_plan_plinth(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_plan_plinth)
}

/// Architecture data version 31 (docs/adr/8145): a building of the program saying its own structural system
/// (`settings.arch.program.buildings[].structure`). Version 31 holds versions 1–30's data too. The document schema stays
/// as it is.
pub const ARCH_WITH_BUILDING_STRUCTURES: u16 = 31;

/// Whether `doc` holds architecture data 31 ([`ARCH_WITH_BUILDING_STRUCTURES`]).
pub fn uses_building_structures(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(|a| a.program.has_building_structures())
}

/// Architecture data version 30 (docs/adr/8144): a building of the program saying its own Bölge
/// (`settings.arch.program.buildings[].zone`). Version 30 holds versions 1–29's data too. The document schema stays as it
/// is.
pub const ARCH_WITH_BUILDING_ZONES: u16 = 30;

/// Whether `doc` holds architecture data 30 ([`ARCH_WITH_BUILDING_ZONES`]).
pub fn uses_building_zones(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(|a| a.program.has_building_zones())
}

/// Architecture data version 29 (docs/adr/8142): a site plan's edge against the neighbour's building
/// (`settings.arch.site.edges[].kind` `party`). Version 29 holds versions 1–28's data too. The document schema stays as
/// it is.
pub const ARCH_WITH_PARTY_EDGES: u16 = 29;

/// Whether `doc` holds architecture data 29 ([`ARCH_WITH_PARTY_EDGES`]).
pub fn uses_party_edges(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_party_edges)
}

/// Architecture data version 28 (docs/adr/8141): the building's layout (yapı nizamı, `settings.arch.layout`). Version 28
/// holds versions 1–27's data too. The document schema stays as it is.
pub const ARCH_WITH_LAYOUT: u16 = 28;

/// Whether `doc` holds architecture data 28 ([`ARCH_WITH_LAYOUT`]).
pub fn uses_layout(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().is_some_and(kentos_contracts::ArchSettings::has_layout)
}

/// Architecture data version 27 (docs/adr/8140): where İmar durumu's emsal and TAKS are read from
/// (`settings.arch.zoning.farSource`, `coverageSource`). Version 27 holds versions 1–26's data too. The document schema
/// stays as it is.
pub const ARCH_WITH_ZONING_SOURCES: u16 = 27;

/// Whether `doc` holds architecture data 27 ([`ARCH_WITH_ZONING_SOURCES`]).
pub fn uses_zoning_sources(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.arch.as_ref().and_then(|a| a.zoning.as_ref()).is_some_and(kentos_contracts::Zoning::has_sources)
}

/// Architecture data version 26 (docs/adr/8137): a wall style's declared load-bearing role
/// (`settings.wallStyles[].loadBearing`). Version 26 holds versions 1–25's data too. The document schema stays as it is.
pub const ARCH_WITH_STYLE_BEARING: u16 = 26;

/// Whether `doc` holds architecture data 26 ([`ARCH_WITH_STYLE_BEARING`]).
pub fn uses_style_bearing(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    doc.settings.wall_styles.iter().any(|s| s.load_bearing.is_some())
}

/// Architecture data version 25 (docs/adr/8136): a shaft room's declared protected role and a wall's or a slab's declared
/// fire stop role (`arch.protected`, `arch.fireStop`; BYKHY Ek-3/B 8, 10). Version 25 holds versions 1–24's data too.
/// The document schema stays as it is.
pub const ARCH_WITH_FIRE_STOPS: u16 = 25;

/// Whether `doc` holds architecture data 25 ([`ARCH_WITH_FIRE_STOPS`]).
pub fn uses_fire_stops(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_fire_stops_in(doc.entities.iter())
}

fn uses_fire_stops_in<'a>(
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    objects.any(|e| e.arch_info().is_some_and(|a| a.protected.is_some() || a.fire_stop.is_some()))
}

/// Architecture data version 24 (docs/adr/8135): a lift shaft room's declared vent or pressurisation
/// (`arch.shaftVent` of a room, BYKHY 62/4). Version 24 holds versions 1–23's data too. The document schema stays as it
/// is.
pub const ARCH_WITH_SHAFT_VENTS: u16 = 24;

/// Whether `doc` holds architecture data 24 ([`ARCH_WITH_SHAFT_VENTS`]).
pub fn uses_shaft_vents(doc: &kentos_contracts::DocumentSnapshotV2) -> bool {
    uses_shaft_vents_in(doc.entities.iter())
}

fn uses_shaft_vents_in<'a>(
    mut objects: impl Iterator<Item = &'a kentos_contracts::Entity>,
) -> bool {
    objects.any(|e| e.arch_info().is_some_and(|a| a.shaft_vent.is_some()))
}

/// The architecture data version `doc` needs: 43 for a site ground, 42 for a profile, 41 for a raft, 40 for a footing, 39 for kitchen storey or filler placement, 38 for an electrical
/// appliance card, 37 for a ceiling view recipe, 36 for individual slab edge links,
/// 35 for a building's own plan notes, 34 for a plan note of the
/// projections or basements, 33 for one of the
/// roof, 32 for one of the subasman, 31 for a
/// building's own structural
/// system, 30 for a building's own
/// Bölge, 29 for a party edge, 28 for the building's
/// layout, 27 for İmar durumu's
/// sources, 26 for a wall
/// style's load-bearing role, 25
/// for a protected shaft or a fire
/// stop, 24 for a lift shaft's vent, 23 for a material's roof class, 22 for a sloped member, 21 for a room's own emsal state, 20 for a declared exterior or column load-bearing role, 19 for an element type, 18 for an underlay model, 17 for a delivery record, 16 for a plan note of the building's height or
/// storey count, 15 for the
/// building's structural system, 14 for a material's reaction class, 13 for a declared load-bearing or escape route
/// role, 12 for the parcel's
/// area's use, 11 for its plan notes, 10 for the form's answers, 9 for a
/// building's code, use or ±0.00 or a shared level, 8 for a roof's
/// layers, 7 for a sloped slab, 6 for a beams' or coverings'
/// schedule, 5 for a covering, 4 for a post, a rail or a column's own height, 3 for a curtain wall, 2 for a block's
/// building element, 1 for the performance declarations, else 0 (not raised).
pub fn arch_version(doc: &kentos_contracts::DocumentSnapshotV2) -> u16 {
    arch_version_of(doc, doc.entities.iter())
}

/// Architecture requirements of the persisted objects, including desktop saves with a separate object store.
fn arch_version_of<'a>(
    doc: &kentos_contracts::DocumentSnapshotV2,
    objects: impl Iterator<Item = &'a kentos_contracts::Entity> + Clone,
) -> u16 {
    if uses_ground(doc) {
        ARCH_WITH_GROUND
    } else if uses_profiles_in(objects.clone()) {
        ARCH_WITH_PROFILES
    } else if uses_rafts_in(doc, objects.clone()) {
        ARCH_WITH_RAFTS
    } else if uses_footings_in(doc, objects.clone()) {
        ARCH_WITH_FOOTINGS
    } else if uses_casework_placement(doc) {
        ARCH_WITH_CASEWORK_PLACEMENT
    } else if uses_electric_appliances(doc) {
        ARCH_WITH_ELECTRIC_APPLIANCES
    } else if uses_ceiling_recipe(doc) {
        ARCH_WITH_CEILING_RECIPE
    } else if uses_slab_edge_bindings_in(doc, objects.clone()) {
        ARCH_WITH_SLAB_EDGE_BINDINGS
    } else if uses_building_plan_notes(doc) {
        ARCH_WITH_BUILDING_PLAN_NOTES
    } else if uses_plan_mass(doc) {
        ARCH_WITH_PLAN_MASS
    } else if uses_plan_roof(doc) {
        ARCH_WITH_PLAN_ROOF
    } else if uses_plan_plinth(doc) {
        ARCH_WITH_PLAN_PLINTH
    } else if uses_building_structures(doc) {
        ARCH_WITH_BUILDING_STRUCTURES
    } else if uses_building_zones(doc) {
        ARCH_WITH_BUILDING_ZONES
    } else if uses_party_edges(doc) {
        ARCH_WITH_PARTY_EDGES
    } else if uses_layout(doc) {
        ARCH_WITH_LAYOUT
    } else if uses_zoning_sources(doc) {
        ARCH_WITH_ZONING_SOURCES
    } else if uses_style_bearing(doc) {
        ARCH_WITH_STYLE_BEARING
    } else if uses_fire_stops_in(objects.clone()) {
        ARCH_WITH_FIRE_STOPS
    } else if uses_shaft_vents_in(objects.clone()) {
        ARCH_WITH_SHAFT_VENTS
    } else if uses_roof_classes(doc) {
        ARCH_WITH_ROOF_CLASSES
    } else if uses_sloped_members_in(doc, objects.clone()) {
        ARCH_WITH_SLOPED_MEMBERS
    } else if uses_room_emsal_in(objects.clone()) {
        ARCH_WITH_ROOM_EMSAL
    } else if uses_declared_readings_in(objects.clone()) {
        ARCH_WITH_DECLARED_READINGS
    } else if uses_element_types_in(doc, objects.clone()) {
        ARCH_WITH_ELEMENT_TYPES
    } else if uses_underlays(doc) {
        ARCH_WITH_UNDERLAYS
    } else if uses_deliveries(doc) {
        ARCH_WITH_DELIVERIES
    } else if uses_plan_limits(doc) {
        ARCH_WITH_PLAN_LIMITS
    } else if uses_structure(doc) {
        ARCH_WITH_STRUCTURE
    } else if uses_material_reactions(doc) {
        ARCH_WITH_MATERIAL_REACTIONS
    } else if uses_declared_roles_in(objects.clone()) {
        ARCH_WITH_DECLARED_ROLES
    } else if uses_zone(doc) {
        ARCH_WITH_ZONE
    } else if uses_plan_notes(doc) {
        ARCH_WITH_PLAN_NOTES
    } else if uses_form_answers(doc) {
        ARCH_WITH_FORM_ANSWERS
    } else if uses_building_places(doc) {
        ARCH_WITH_BUILDING_PLACES
    } else if uses_roof_layers_in(doc, objects.clone()) {
        ARCH_WITH_ROOF_LAYERS
    } else if uses_slab_slopes_in(doc, objects.clone()) {
        ARCH_WITH_SLAB_SLOPES
    } else if uses_element_schedules_in(doc, objects.clone()) {
        ARCH_WITH_ELEMENT_SCHEDULES
    } else if uses_coverings_in(objects.clone()) {
        ARCH_WITH_COVERINGS
    } else if uses_members_in(doc, objects.clone()) {
        ARCH_WITH_MEMBERS
    } else if uses_curtain_walls_in(doc, objects.clone()) {
        ARCH_WITH_CURTAIN_WALLS
    } else if uses_elements(doc) {
        ARCH_WITH_ELEMENTS
    } else if uses_performance_in(doc, objects.clone()) {
        ARCH_WITH_PERFORMANCE
    } else {
        0
    }
}

/// GIS data version 1 (docs/adr/8057 §15): a layer's derivation names the extension whose tool made it and where its
/// code ran (`extension`, `environment`). The first module version raised: the document schema stays as it is.
pub const GIS_WITH_EXTENSION_DERIVATIONS: u16 = 1;
/// GIS data version 2 (docs/adr/9101 §3): a data layer (`data` of a layer node): a table of a GeoPackage or a database
/// drawn as it is now, its rows never in the file. The document schema stays as it is.
pub const GIS_WITH_DATA_LAYERS: u16 = 2;
/// GIS data version 3 (docs/adr/9104 §2): how the data layers' edit sessions go together (the settings'
/// `editGrouping`: `layer`, `source` or `all`). The document schema stays as it is.
pub const GIS_WITH_EDIT_GROUPING: u16 = 3;
/// GIS data version 4 (docs/adr/9105 §4): a layer whose tree row says how many objects it and each of its style's
/// classes hold (`classCounts` of a layer node, only `true`). The document schema stays as it is.
pub const GIS_WITH_CLASS_COUNTS: u16 = 4;
/// GIS data version 5 (docs/adr/9114 §1): a data layer whose rows are texts (`text` of a data layer: which columns hold
/// their words, height, angle, alignment and width factor). The document schema stays as it is.
pub const GIS_WITH_TEXT_TABLES: u16 = 5;
/// GIS data version 6 (docs/adr/9135): a relation by a parent key (`parentKey` of a relation: the parent field whose
/// value the child field holds). The document schema stays as it is.
pub const GIS_WITH_RELATION_KEYS: u16 = 6;
/// GIS data version 7 (docs/adr/9192): where a layer's objects were copied from (`origin` of a layer node: the format,
/// file or database address, table, the file's SHA-256 and when an import read it). The document schema stays as it is.
pub const GIS_WITH_LAYER_ORIGINS: u16 = 7;

/// A payload root's `modules` (schema 254): the versions above 0 of each module's and each `.kek` extension's data.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModuleVersions {
    /// By module key (`arch`, `gis`, `map`, `road`).
    pub modules: Vec<(String, u16)>,
    /// By extension id.
    pub extensions: Vec<(String, u16)>,
}

impl ModuleVersions {
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty() && self.extensions.is_empty()
    }

    /// A module's version; 0 when it is not written (not raised).
    pub fn module(&self, key: &str) -> u16 {
        self.modules
            .iter()
            .find(|(k, _)| k == key)
            .map_or(0, |(_, v)| *v)
    }
}
/// The largest drawing order a file holds: 2³¹ − 1; the smallest is −2³¹.
pub const MAX_ORDER: u64 = i32::MAX as u64;

/// The document schemas this codec reads, oldest first.
pub const SCHEMAS: [u32; 255] = [
    kentos_contracts::DOCUMENT_VERSION_2,
    SCHEMA_WITH_LINE_WEIGHTS,
    SCHEMA_WITH_ELEVATIONS,
    SCHEMA_WITH_PARTS,
    SCHEMA_WITH_BLOCKS,
    SCHEMA_WITH_TEXT_EXTRAS,
    SCHEMA_WITH_LEADERS,
    SCHEMA_WITH_DIMENSIONS,
    SCHEMA_WITH_LAYER_SNAP,
    SCHEMA_WITH_DRAWING_UNIT,
    SCHEMA_WITH_SECOND_SRID,
    SCHEMA_WITH_CUSTOM_CRS,
    SCHEMA_WITH_SURVEY,
    SCHEMA_WITH_TRAVERSE_TOLERANCES,
    SCHEMA_WITH_GROUND,
    SCHEMA_WITH_LINE_PARTS,
    SCHEMA_WITH_LINKED_TEXTS,
    SCHEMA_WITH_LAYER_STATES,
    SCHEMA_WITH_PARAGRAPHS,
    SCHEMA_WITH_STYLES,
    SCHEMA_WITH_TABLES,
    SCHEMA_WITH_HATCH_PATTERNS,
    SCHEMA_WITH_IMAGES,
    SCHEMA_WITH_TEXT_PATHS,
    SCHEMA_WITH_LAYER_FIELDS,
    SCHEMA_WITH_TOPOLOGY,
    SCHEMA_WITH_SURVEY_SIGMAS,
    SCHEMA_WITH_RASTERS,
    SCHEMA_WITH_ANNOTATION,
    SCHEMA_WITH_POINT_CLOUDS,
    SCHEMA_WITH_SERVICES,
    SCHEMA_WITH_NETWORKS,
    SCHEMA_WITH_TEMPORAL,
    SCHEMA_WITH_FILTERS,
    SCHEMA_WITH_LABELS,
    SCHEMA_WITH_VARIABLES,
    SCHEMA_WITH_MULTIDIM,
    SCHEMA_WITH_MEASURES,
    SCHEMA_WITH_ATTACHMENTS,
    SCHEMA_WITH_COMPOSE,
    SCHEMA_WITH_ADDRESSES,
    SCHEMA_WITH_DIMENSION_REFS,
    SCHEMA_WITH_SHEETS,
    SCHEMA_WITH_REQUESTS,
    SCHEMA_WITH_POLYLINE_EXTRAS,
    SCHEMA_WITH_NURBS,
    SCHEMA_WITH_LOOK,
    SCHEMA_WITH_UCS,
    SCHEMA_WITH_IMPERIAL,
    SCHEMA_WITH_SUBREF_EXTRAS,
    SCHEMA_WITH_VIEWS,
    SCHEMA_WITH_ARRAYS,
    SCHEMA_WITH_WIPEOUTS,
    SCHEMA_WITH_FROZEN,
    SCHEMA_WITH_PATTERNS,
    SCHEMA_WITH_BLOCK_EXTRAS,
    SCHEMA_WITH_BLOCK_CARDS,
    SCHEMA_WITH_DYNAMIC_BLOCKS,
    SCHEMA_WITH_DIMENSION_RULES,
    SCHEMA_WITH_DIMENSION_OPTIONS,
    SCHEMA_WITH_DIMENSION_MARKS,
    SCHEMA_WITH_OVERRIDES,
    SCHEMA_WITH_ANNOTATIVE,
    SCHEMA_WITH_PARAGRAPH_LAYOUT,
    SCHEMA_WITH_TEXT_FIELDS,
    SCHEMA_WITH_MULTILEADER,
    SCHEMA_WITH_TABLE_CELLS,
    SCHEMA_WITH_HATCH_BOUNDARIES,
    SCHEMA_WITH_HATCH_GAPS,
    SCHEMA_WITH_HATCH_EXTRAS,
    SCHEMA_WITH_FORMS,
    SCHEMA_WITH_CONSTRAINTS,
    SCHEMA_WITH_SOURCES,
    SCHEMA_WITH_WALLS,
    SCHEMA_WITH_WALL_JOINS,
    SCHEMA_WITH_OPENINGS,
    SCHEMA_WITH_SPACES,
    SCHEMA_WITH_ARCH_ELEMENTS,
    SCHEMA_WITH_TAGS,
    SCHEMA_WITH_VIEWMARKS,
    SCHEMA_WITH_PHASES,
    SCHEMA_WITH_XREFS,
    SCHEMA_WITH_XREF_PATHS,
    SCHEMA_WITH_CLIPPING,
    SCHEMA_WITH_HEIGHTS,
    SCHEMA_WITH_NETWORK_TURNS,
    SCHEMA_WITH_GEOREF,
    SCHEMA_WITH_JOIN_VIEWS,
    SCHEMA_WITH_RECORDED_TIME,
    SCHEMA_WITH_SOURCE_BASES,
    SCHEMA_WITH_SOURCE_FIELD_BASES,
    SCHEMA_WITH_NETWORK_TERMINALS,
    SCHEMA_WITH_NETWORK_CONFIGS,
    SCHEMA_WITH_NETWORK_PROFILES,
    SCHEMA_WITH_TIMED_CLOSURES,
    SCHEMA_WITH_LINEAR_EVENTS,
    SCHEMA_WITH_SOURCE_CRS,
    SCHEMA_WITH_EXCEPTION_DIGESTS,
    SCHEMA_WITH_GEOREF_CRS,
    SCHEMA_WITH_LINEAGE,
    SCHEMA_WITH_LAYER_GEOMETRY,
    SCHEMA_WITH_RASTER_TILES,
    SCHEMA_WITH_FIELD_KINDS,
    SCHEMA_WITH_JOIN_SOURCES,
    SCHEMA_WITH_DATASETS,
    SCHEMA_WITH_WITHHELD_FIELDS,
    SCHEMA_WITH_SPACE_COLUMNS,
    SCHEMA_WITH_SPACE_SCOPE,
    SCHEMA_WITH_SPACE_SOURCES,
    SCHEMA_WITH_SPACE_SEPARATORS,
    SCHEMA_WITH_SPACE_CHOICES,
    SCHEMA_WITH_DIMENSION_SETS,
    SCHEMA_WITH_DIMENSION_SET_EDGES,
    SCHEMA_WITH_RAILINGS,
    SCHEMA_WITH_SPACE_VOIDS,
    SCHEMA_WITH_STAIR_ARCS,
    SCHEMA_WITH_DIMENSION_CROSS,
    SCHEMA_WITH_RECESSES,
    SCHEMA_WITH_LEVEL_STATE,
    SCHEMA_WITH_DIMENSION_LEVELS,
    SCHEMA_WITH_SCHEDULE_COLUMNS,
    SCHEMA_WITH_SLABS,
    SCHEMA_WITH_RAMP_LEVELS,
    SCHEMA_WITH_STAIR_LEVELS,
    SCHEMA_WITH_GRID_COLUMN_FORMS,
    SCHEMA_WITH_CASEWORK,
    SCHEMA_WITH_DIMENSION_SET_SPANS,
    SCHEMA_WITH_DIMENSION_SET_SCALES,
    SCHEMA_WITH_AUDIT_EXCEPTIONS,
    SCHEMA_WITH_FIXTURES,
    SCHEMA_WITH_PROFILE_FILES,
    SCHEMA_WITH_FIELD_COLUMNS,
    SCHEMA_WITH_TABLE_CONSTRAINTS,
    SCHEMA_WITH_CAD_WORK,
    SCHEMA_WITH_LAYER_INDEXES,
    SCHEMA_WITH_ARCH_BONDS,
    SCHEMA_WITH_BUILDING_PARTS,
    SCHEMA_WITH_OPENING_TYPES,
    SCHEMA_WITH_BUILDING_PROGRAM,
    SCHEMA_WITH_VIEW_RECIPES,
    SCHEMA_WITH_MODULES,
    SCHEMA_WITH_RECIPE_DETAIL,
    SCHEMA_WITH_SPECIAL_DOORS,
    SCHEMA_WITH_MODULE_UNITS,
    SCHEMA_WITH_VIEW_TEMPLATES,
    SCHEMA_WITH_PROGRAM_SHARES,
    SCHEMA_WITH_MATERIALS,
    SCHEMA_WITH_OPTIONS,
    SCHEMA_WITH_MULTI_SLIDE,
    SCHEMA_WITH_UNIT_SCHEDULES,
    SCHEMA_WITH_MODULE_STOREYS,
    SCHEMA_WITH_SLIDE_MORE,
    SCHEMA_WITH_OPTION_COMBOS,
    SCHEMA_WITH_INSTANCE_STOREYS,
    SCHEMA_WITH_ARC_EDGE_BONDS,
    SCHEMA_WITH_BLOCK_PUBLICATIONS,
    SCHEMA_WITH_BLOCK_PARAMETER_GROUPS,
    SCHEMA_WITH_BLOCK_TYPE_VALUES,
    SCHEMA_WITH_LATTICE_ARRAYS,
    SCHEMA_WITH_ROAD_WORK,
    SCHEMA_WITH_TEXT_ENGINE,
    SCHEMA_WITH_SHAPE_RESOURCES,
    SCHEMA_WITH_STANDARD_DOCS,
    SCHEMA_WITH_ROAD_PAVEMENT,
    SCHEMA_WITH_ROAD_PIVOT,
    SCHEMA_WITH_SECTION_BREAKS,
    SCHEMA_WITH_VOLUME_CURVE,
    SCHEMA_WITH_CORRIDOR_SAMPLING,
    SCHEMA_WITH_HAUL_SORTING,
    SCHEMA_WITH_PROFILE_CROSSINGS,
    SCHEMA_WITH_VOLUME_HISTORY,
    SCHEMA_WITH_HAUL_APPROVAL,
    SCHEMA_WITH_HAUL_NETWORK,
    SCHEMA_WITH_VOLUME_STATES,
    SCHEMA_WITH_CUT_BENCHES,
    SCHEMA_WITH_SLOPE_TRANSITIONS,
    SCHEMA_WITH_VOLUME_GRID,
    SCHEMA_WITH_REVIEW_NOTES,
    SCHEMA_WITH_LONG_PATTERNS,
    SCHEMA_WITH_EXTRACTION,
    SCHEMA_WITH_APPROVED_SHEET_TEMPLATES,
    SCHEMA_WITH_SHX_FONTS,
    SCHEMA_WITH_TABLE_STYLES,
    SCHEMA_WITH_DIMENSION_TEXT_STYLES,
    SCHEMA_WITH_LEADER_TEXT_STYLES,
    SCHEMA_WITH_MAP_WORK,
    SCHEMA_WITH_TOPOLOGY_REVIEWS,
    SCHEMA_WITH_STANDARD_EXCEPTIONS,
    SCHEMA_WITH_ROAD_GRADES,
    SCHEMA_WITH_ROAD_OVE,
    SCHEMA_WITH_MAP_WORK_RELATIVE,
    SCHEMA_WITH_MAP_SURVEY,
    SCHEMA_WITH_MAP_SURFACES,
    SCHEMA_WITH_SURFACE_EDITS,
    SCHEMA_WITH_MAP_PLATFORMS,
    SCHEMA_WITH_MAP_DRAWINGS,
    SCHEMA_WITH_GNSS_NETWORK,
    SCHEMA_WITH_ROAD_SPEED,
    SCHEMA_WITH_ROAD_PLACES,
    SCHEMA_WITH_ROAD_CLOUD_HEIGHTS,
    SCHEMA_WITH_ROAD_CLOUD_CONVERSIONS,
    SCHEMA_WITH_ROAD_APPROVAL_SOURCES,
    SCHEMA_WITH_CUTFILL_DRAWINGS,
    SCHEMA_WITH_WORK_STAMPS,
    SCHEMA_WITH_KNOWN_VELOCITIES,
    SCHEMA_WITH_BOUNDARY_BONDS,
    SCHEMA_WITH_DRAWING_PROVENANCE,
    SCHEMA_WITH_APPROVED_OBJECTS,
    SCHEMA_WITH_APPROVED_PARTS,
    SCHEMA_WITH_PIPE_PROFILE_DRAWINGS,
    SCHEMA_WITH_APPROVED_VALUES,
    SCHEMA_WITH_SURFACE_PASTES,
    SCHEMA_WITH_REVIEW_DECISIONS,
    SCHEMA_WITH_PARCEL_REGISTER,
    SCHEMA_WITH_PLAN_CUT,
    SCHEMA_WITH_LINE_GROUPS,
    SCHEMA_WITH_ELLIPSE_TAGS,
    SCHEMA_WITH_OPENING_CODES,
    SCHEMA_WITH_PAIY_AREAS,
    SCHEMA_WITH_ZONING,
    SCHEMA_WITH_LAND_SHARES,
    SCHEMA_WITH_SITE_PLAN,
    SCHEMA_WITH_LOCAL_RULES,
    SCHEMA_WITH_SPRINKLERS,
    SCHEMA_WITH_PROVISION_TABLES,
    SCHEMA_WITH_ROOM_LIST,
    SCHEMA_WITH_OPENING_SPECS,
    SCHEMA_WITH_OPENING_PHASES,
    SCHEMA_WITH_RENOVATION_VIEW,
    SCHEMA_WITH_SLAB_LAYERS,
    SCHEMA_WITH_ROOM_HEIGHTS,
    SCHEMA_WITH_BEAMS,
    SCHEMA_WITH_COLUMN_MATERIAL,
    SCHEMA_WITH_BEAM_ROLES,
    SCHEMA_WITH_ROOFS,
    SCHEMA_WITH_ROOF_PARTS,
    SCHEMA_WITH_ROOF_EDGES,
    SCHEMA_WITH_ROOF_DOWNPIPES,
    SCHEMA_WITH_STAIR_WINDERS,
    SCHEMA_WITH_RAILING_OBJECTS,
    SCHEMA_WITH_GRID_BONDS,
    SCHEMA_WITH_COLUMN_SIDE_BONDS,
    SCHEMA_WITH_MODULE_SCOPE,
    SCHEMA_WITH_SHEET_CONTOUR_DRAWINGS,
    SCHEMA_WITH_WORK_PROFILES,
    SCHEMA_WITH_GNSS_SESSIONS,
    SCHEMA_WITH_PARCEL_DRAFTS,
    SCHEMA_WITH_ZONING_DRAFTS,
    SCHEMA_WITH_CLOUD_POSES,
    SCHEMA_WITH_KEPT_ADJUSTMENTS,
    SCHEMA_WITH_MONITORING_REVIEWS,
    SCHEMA_WITH_MODULE_VERSIONS,
    SCHEMA_WITH_TOOL_LAYERS,
    SCHEMA_WITH_LAYER_MOBILE,
    SCHEMA_WITH_NAME_LINKS,
];

/// The file a drawing is saved as.
pub fn encode(doc: &DocumentSnapshotV2) -> Result<Vec<u8>, KcadError> {
    encode_watched(doc, &mut Quiet)
}

/// `encode`, the objects reported to `watch` as they are written; it may stop the writing.
pub fn encode_watched(
    doc: &DocumentSnapshotV2,
    watch: &mut dyn Watch,
) -> Result<Vec<u8>, KcadError> {
    header::write(&encode::payload(
        doc,
        encode::Objs::Plain(&doc.entities),
        watch,
    )?)
}

/// The file a drawing is saved as, read back before it is handed out: the
/// bytes must decode to the same drawing, every float64 bit for bit (only
/// the objects' slots, which the file does not keep, may differ). A save
/// writes only verified bytes, so a file is never replaced by one that does
/// not open again (TODOS.md FILE-16, FILE-21).
pub fn encode_verified(doc: &DocumentSnapshotV2) -> Result<Vec<u8>, KcadError> {
    encode_verified_watched(doc, &mut Quiet)
}

/// `encode_verified` with its stages reported to `watch` (writing, then
/// `Verifying` and the reading back), which may stop it.
pub fn encode_verified_watched(
    doc: &DocumentSnapshotV2,
    watch: &mut dyn Watch,
) -> Result<Vec<u8>, KcadError> {
    verified(doc, encode::Objs::Plain(&doc.entities), watch)
}

/// `encode_verified_watched` of a drawing whose objects are held apart, as the open document shares them
/// (docs/adr/0654): `head` is the drawing with no objects and with their ids (`uids`), `objects` the objects in the
/// same order. Nothing is copied: the writer reads the objects where they are, and the bytes are read back object by
/// object, each compared with its original and dropped, so a large drawing is never in memory twice.
pub fn encode_verified_parts_watched(
    head: &DocumentSnapshotV2,
    objects: &[std::sync::Arc<contracts::Entity>],
    watch: &mut dyn Watch,
) -> Result<Vec<u8>, KcadError> {
    if !head.entities.is_empty() {
        return Err(KcadError::new(
            Code::BadValue,
            "Çizimin başı nesne taşımamalı; nesneler ayrı verilir. Bu bir yazılım hatasıdır; durumu bildirin.",
        ));
    }
    verified(head, encode::Objs::Shared(objects), watch)
}

/// The bytes of `doc` with `objects`, read back before they are handed out: the objects compared one by one as they
/// are read (`columns::Same`), then everything else.
fn verified(
    doc: &DocumentSnapshotV2,
    objects: encode::Objs<'_>,
    watch: &mut dyn Watch,
) -> Result<Vec<u8>, KcadError> {
    let bytes = header::write(&encode::payload(doc, objects, watch)?)?;
    watch::report(watch, Step::Verifying)?;
    let mut check = Verify {
        objects,
        uids: &doc.uids,
        same: columns::Same::default(),
    };
    let (_, payload) = header::read_watched(&bytes, watch).map_err(not_read_back)?;
    let back = decode::payload_checked(payload, watch, &mut check).map_err(not_read_back)?;
    if back.uids.len() != objects.len() {
        return Err(unverified("nesne sayısı"));
    }
    if let Some(what) = head_difference(doc, &back) {
        return Err(unverified(&what));
    }
    Ok(bytes)
}

/// A verifying read's comparison of each object read with the one written.
struct Verify<'a> {
    objects: encode::Objs<'a>,
    uids: &'a [contracts::EntityId],
    same: columns::Same,
}

impl decode::Check for Verify<'_> {
    fn object(
        &mut self,
        i: usize,
        entity: &contracts::Entity,
        uid: &contracts::EntityId,
    ) -> Option<String> {
        let differs = match (self.objects.get(i), self.uids.get(i)) {
            (Some(a), Some(a_uid)) => self.same.differs(a, a_uid, entity, uid),
            _ => true,
        };
        differs.then(|| format!("nesne {}", i + 1))
    }
}

/// Bytes that did not read back: a verification failure, unless the watcher stopped the reading.
fn not_read_back(e: KcadError) -> KcadError {
    if e.code == Code::Cancelled {
        e
    } else {
        unverified(&e.message)
    }
}

fn unverified(what: &str) -> KcadError {
    KcadError::new(
        Code::VerifyFailed,
        format!(
            "Çizim kaydedilmedi: yazılacak KCAD baytları geri okununca çizimle aynı çıkmadı ({what}). Bu bir yazılım hatasıdır; eski dosyaya dokunulmadı. Çizimi başka bir yere kaydetmeyi deneyin ve durumu bildirin."
        ),
    )
}

/// The drawing in a file; the whole file is checked before anything is returned.
pub fn decode(data: &[u8]) -> Result<DocumentSnapshotV2, KcadError> {
    decode_watched(data, &mut Quiet)
}

/// `decode` with its stages reported to `watch` (the integrity check, the
/// project, the objects), which may stop it: then nothing is returned, so
/// half a file never becomes a drawing (TODOS.md FILE-20).
pub fn decode_watched(data: &[u8], watch: &mut dyn Watch) -> Result<DocumentSnapshotV2, KcadError> {
    let (_, payload) = header::read_watched(data, watch)?;
    decode::payload(payload, watch)
}

/// The header and the drawing of a file (`kcad inspect`).
pub fn read(data: &[u8]) -> Result<(Header, DocumentSnapshotV2), KcadError> {
    let (head, payload) = header::read(data)?;
    Ok((head, decode::payload(payload, &mut Quiet)?))
}

// ── The browser's typed boundary (docs/adr/0030) ────────────────────────

/// A drawing taken apart for the browser's page: the contract's JSON of
/// everything but the objects (`entities` and `uids` empty), and the objects
/// as columns. Each object is dropped once packed, so a large drawing is in
/// memory about once.
pub fn split(mut doc: DocumentSnapshotV2) -> Result<(String, Columns), KcadError> {
    let entities = std::mem::take(&mut doc.entities);
    let uids = std::mem::take(&mut doc.uids);
    let head = head_json(&doc)?;
    Ok((head, columns::pack(entities, uids)))
}

fn head_json(doc: &DocumentSnapshotV2) -> Result<String, KcadError> {
    serde_json::to_string(doc).map_err(|e| {
        KcadError::new(
            Code::BadColumns,
            format!(
                "Çizimin proje bilgisi yazılamadı ({e}). Bu bir yazılım hatasıdır; durumu bildirin."
            ),
        )
    })
}

/// The drawing the browser's page packed, as a `.kcad` v2 file's bytes,
/// verified before they are handed out: `head` is the contract's JSON of
/// everything but the objects, `cols` the objects. The bytes are read back
/// and the objects compared with the columns as they were sent, every field
/// and every float bit for bit (`columns::differs`). Returns the bytes and
/// the head as the file holds it (JSON), for the page's side to compare with
/// the head it sent: a field the contract does not read would show there.
pub fn encode_columns(
    head: &str,
    cols: &Columns,
    watch: &mut dyn Watch,
) -> Result<(Vec<u8>, String), KcadError> {
    let mut doc: DocumentSnapshotV2 = serde_json::from_str(head).map_err(|e| {
        KcadError::new(
            Code::BadColumns,
            format!(
                "Kaydedilecek çizimin proje bilgisi okunamadı ({e}); uygulama ile dosya biçimi paketi uyuşmuyor olabilir (pnpm wasm). Dosya yazılmadı."
            ),
        )
    })?;
    let (entities, uids) = columns::unpack(cols)?;
    doc.entities = entities;
    doc.uids = uids;
    let bytes = encode_watched(&doc, watch)?;
    // The drawing goes before it is read back: a large one is not in memory twice.
    drop(doc);
    watch::report(watch, Step::Verifying)?;
    let mut back = decode_watched(&bytes, watch).map_err(not_read_back)?;
    if let Some(what) = columns::differs(cols, &back.entities, &back.uids) {
        return Err(unverified(&what));
    }
    back.entities = Vec::new();
    back.uids = Vec::new();
    Ok((bytes, head_json(&back)?))
}

/// What a file is by its content, not its name (docs/specs/kcad-v2.md §8, TODOS.md FILE-03).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sniff {
    /// The full KCAD signature: the v2 reader decides the rest.
    Kcad,
    /// “\x89KCAD” with the rest of the signature changed: a KCAD file a text transfer damaged.
    KcadDamaged,
    /// A JSON object: the v1 reader (`kentos.document` 1) decides.
    Json,
    /// Nothing but a byte order mark and white space.
    Empty,
    /// Anything else (DXF, a picture, another program's file).
    Foreign,
}

impl Sniff {
    /// As the specification and the fixtures write it.
    pub fn as_str(self) -> &'static str {
        match self {
            Sniff::Kcad => "kcad",
            Sniff::KcadDamaged => "kcad-damaged",
            Sniff::Json => "json",
            Sniff::Empty => "empty",
            Sniff::Foreign => "foreign",
        }
    }
}

pub fn sniff(data: &[u8]) -> Sniff {
    if data.starts_with(&MAGIC) {
        return Sniff::Kcad;
    }
    if data.starts_with(&header::MAGIC_PREFIX) {
        return Sniff::KcadDamaged;
    }
    let rest = data.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(data);
    match rest
        .iter()
        .find(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
    {
        None => Sniff::Empty,
        Some(b'{') => Sniff::Json,
        Some(_) => Sniff::Foreign,
    }
}

/// Where two drawings differ but for their objects: the small parts as their JSON text (every float64 bit for bit,
/// `-0.0` apart from `0.0`); the objects' ids are compared, the objects are not.
fn head_difference(a: &DocumentSnapshotV2, b: &DocumentSnapshotV2) -> Option<String> {
    fn same<T: Serialize + ?Sized>(x: &T, y: &T) -> bool {
        match (serde_json::to_vec(x), serde_json::to_vec(y)) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        }
    }
    let parts: [(&str, bool); 11] = [
        ("format", a.format == b.format && a.version == b.version),
        ("name", a.name == b.name),
        ("settings", same(&a.settings, &b.settings)),
        ("origin", same(&a.origin, &b.origin)),
        ("homeView", same(&a.home_view, &b.home_view)),
        ("layers", same(&a.layers, &b.layers)),
        ("activeLayer", a.active_layer == b.active_layer),
        ("uids", a.uids == b.uids),
        ("styles", same(&a.styles, &b.styles)),
        ("projectId", a.project_id == b.project_id),
        ("migratedFrom", a.migrated_from == b.migrated_from),
    ];
    parts
        .iter()
        .find(|(_, same)| !same)
        .map(|(name, _)| (*name).to_owned())
}

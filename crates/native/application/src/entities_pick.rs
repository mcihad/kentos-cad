//! `cad.entities.pick` v1 (docs/adr/0276): what a click can mean, the most specific first (the selection query's
//! `point` region, `entities_select`), each object with the sub-object the click falls on: its nearest point (an end,
//! a vertex, a point, an insertion point) before its edges, an edge's nearest point as an `along` place, inside an
//! insert the piece's path too (`ops::subref::place_near`, `place_near_in`). Sıradakini seç's Ctrl tıklaması reads it
//! the same. It writes nothing. The shared cases are `fixtures/commands/v1/cad.entities.pick.json`
//! (`scripts/fixtures/entities_pick_cases.py`).

use kentos_contracts::{
    CommandResult, EntitiesPick, EntitiesPicked, EntitiesSelect, Entity, PickedEntity,
    SelectRegion,
};
use kentos_domain::dimension_ties::{contract_place, core_blocks};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::subref::{Place, place_near, place_near_in};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::blocks::core_entity;
use crate::checks::{self, Stop, error};
use crate::codes;

/// Checks `input`.
pub fn validate(_cx: &ExecutionContext<'_>, input: &EntitiesPick) -> CommandResult<()> {
    match check(input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The objects; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesPick) -> CommandResult<EntitiesPicked> {
    answer(cx.doc, cx.store, input)
}

/// The objects. Writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: EntitiesPick) -> CommandResult<EntitiesPicked> {
    answer(cx.doc, cx.store, &input)
}

/// [`execute`] over the document and the host's store of it, when it has one.
pub fn answer(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesPick,
) -> CommandResult<EntitiesPicked> {
    if let Err(stop) = check(input) {
        return stop.into();
    }
    let own;
    let store = match store {
        Some(s) => s,
        None => {
            own = crate::geometry_store::store_of(doc);
            &own
        }
    };
    let query = EntitiesSelect {
        region: SelectRegion::Point {
            at: input.at,
            tolerance: input.tolerance,
        },
        kinds: input.kinds.clone(),
        layers: None,
    };
    let found = match crate::entities_select::select(doc, store, &query) {
        Ok(found) => found,
        Err(e) => {
            return CommandResult::Failed { error: *e };
        }
    };
    let at = Vec2::new(input.at.x, input.at.y);
    let picks = found
        .slots
        .iter()
        .filter_map(|s| picked(doc, *s, at, input.tolerance))
        .collect();
    CommandResult::Completed {
        output: EntitiesPicked {
            picks,
            left_out: found.left_out,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// The sub-object a click at `at` within `tol` falls on: its place and, inside an insert, the piece's path. A point
/// object's points are searched half as far again, as the click reaches it (docs/adr/0187 §1).
pub fn sub_object(doc: &Document, e: &Entity, at: Vec2, tol: f64) -> Option<(Vec<u32>, Place)> {
    let core = core_entity(e);
    let tol = if matches!(e, Entity::Point(_)) { tol * 1.5 } else { tol };
    if matches!(e, Entity::Insert(_)) {
        place_near_in(&core_blocks(doc.blocks()), &core, at, tol).map(|(path, p, _)| (path, p))
    } else {
        place_near(&core, at, tol).map(|(p, _)| (Vec::new(), p))
    }
}

fn picked(doc: &Document, slot: Slot, at: Vec2, tol: f64) -> Option<PickedEntity> {
    let e = doc.get(slot)?;
    let uid = doc.uid(slot)?.to_string();
    let (path, place) = match sub_object(doc, e, at, tol) {
        Some((path, p)) => (path, Some(contract_place(p))),
        None => (Vec::new(), None),
    };
    Some(PickedEntity { uid, place, path })
}

fn check(input: &EntitiesPick) -> Result<(), Stop> {
    checks::point(input.at, "Tıklamanın", "at")?;
    checks::finite(input.tolerance, "Tolerans", "Metre olarak bir sayı verin.", "tolerance")?;
    if input.tolerance < 0.0 {
        return Err(Stop::failed(error(
            codes::INVALID_DISTANCE,
            "Tolerans sıfırdan küçük olamaz: tıklamanın yakınında aranacak uzaklığı metre olarak verin.".into(),
            Some("tolerance".into()),
        )));
    }
    Ok(())
}

//! `cad.entities.array` with `associative` (docs/adr/0300): the objects
//! become a block “Dizi n”, its base point the middle of their box, and every
//! place of the layout, theirs the first, gets an insert of it; the inserts
//! are the items of one associative array the document keeps. One undo step
//! named after the tool: the block, the objects' removal, the items and the
//! array together. The places are the core's (`ops::assoc_array`), the same
//! maps the independent copies take, so the plan's copies are what the items
//! show.

use kentos_contracts::{
    ArrayId, ArrayItem, ArrayLayout, AssocArray, BlockDefinition, BlockId, CommandError, Entity,
    EntityId, Vec2,
};
use kentos_domain::arrays::{item_insert, placements};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::entity_bounds_in;
use kentos_geometry_core::jsmath::{js_max, js_min};

use crate::checks::{Stop, error};
use crate::codes;
use crate::geometry::{drawing_font, shape};

/// The first free name “Dizi n” (n from 1), as block names compare (Turkish case folded).
fn free_name(doc: &Document) -> String {
    (1..)
        .map(|n| format!("Dizi {n}"))
        .find(|name| doc.block_named(name).is_none())
        .unwrap_or_else(|| "Dizi".into())
}

/// The middle of the objects' box, as the polar array's copies that do not turn are placed by.
fn middle(doc: &Document, sources: &[&Entity]) -> Option<Vec2> {
    let font = drawing_font(doc.settings().drawing_font);
    let (mut lo_x, mut lo_y, mut hi_x, mut hi_y) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for e in sources {
        let b = entity_bounds_in(&shape(e), font);
        lo_x = js_min(lo_x, b.min_x);
        lo_y = js_min(lo_y, b.min_y);
        hi_x = js_max(hi_x, b.max_x);
        hi_y = js_max(hi_y, b.max_y);
    }
    (lo_x <= hi_x && lo_y <= hi_y).then(|| Vec2 {
        x: (lo_x + hi_x) / 2.0,
        y: (lo_y + hi_y) / 2.0,
    })
}

/// What the associative write made: the items' ids, its source's first, and the array's.
pub(crate) struct Made {
    pub created: Vec<String>,
    pub array: ArrayId,
}

/// Writes the associative array of the objects named by `sources` (their unlocked ones, checked) in `layout`,
/// under `label`; the source objects are removed.
pub(crate) fn write(
    doc: &mut Document,
    sources: &[String],
    layout: &ArrayLayout,
    label: &str,
) -> Result<Made, Stop> {
    let found: Vec<(Slot, Entity)> = sources
        .iter()
        .filter_map(|u| Uuid::parse_str(u).ok())
        .filter_map(|u| doc.slot_of(u))
        .filter_map(|slot| doc.get(slot).map(|e| (slot, e.clone())))
        .collect();
    let objects: Vec<&Entity> = found.iter().map(|(_, e)| e).collect();
    let Some(base) = middle(doc, &objects) else {
        return Err(Stop::failed(error(
            codes::NOT_FINITE,
            "Nesnelerin kutusu bulunamadı; dizi kurulamadı.".into(),
            Some("uids".into()),
        )));
    };
    let layer = objects
        .first()
        .map(|e| e.base().layer_id.clone())
        .unwrap_or_default();
    let block = BlockDefinition {
        dynamic: None,
        card: None,
        id: BlockId(*Uuid::now_v7().as_bytes()),
        name: free_name(doc),
        base,
        entities: crate::blocks_define::copies(objects.iter().copied()),
        units: None,
        attributes: Vec::new(),
        description: None,
    };
    let mut array = AssocArray {
        id: ArrayId(*Uuid::now_v7().as_bytes()),
        block: block.id,
        layer_id: layer,
        p: base,
        scale: 1.0,
        rotation: 0.0,
        mirror: false,
        layout: layout.clone(),
        items: Vec::new(),
    };
    let path = match layout {
        ArrayLayout::Path { path, .. } => Uuid::parse_str(path)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|slot| doc.get(slot).cloned()),
        _ => None,
    };
    let Some(places) = placements(&array, path.as_ref()) else {
        return Err(Stop::failed(error(
            codes::NOT_FINITE,
            "Dizinin yerleri kurulamadı.".into(),
            Some("layout".into()),
        )));
    };
    let inserts: Vec<Entity> = places
        .iter()
        .map(|at| Entity::Insert(item_insert(&array, at, None)))
        .collect();
    let gone: Vec<Slot> = found.iter().map(|(slot, _)| *slot).collect();
    enum Failed {
        Refused(String),
        Full(String),
    }
    let written = doc.transact(label, |doc| {
        doc.add_block(block)
            .map_err(|r| Failed::Refused(r.to_string()))?;
        doc.remove(&gone);
        let slots = doc
            .add_many(inserts, label)
            .map_err(|full| Failed::Full(full.to_string()))?;
        let uids: Vec<Uuid> = slots.iter().filter_map(|s| doc.uid(*s)).collect();
        array.items = uids
            .iter()
            .map(|u| ArrayItem {
                uid: Some(EntityId(u.into_bytes())),
                kept: false,
            })
            .collect();
        let id = array.id;
        doc.add_array(array)
            .map_err(|r| Failed::Refused(r.to_string()))?;
        Ok((uids, id))
    });
    match written {
        Ok((uids, array)) => Ok(Made {
            created: uids.iter().map(|u| u.to_string()).collect(),
            array,
        }),
        Err(Failed::Refused(message)) => {
            Err(Stop::failed(error(codes::BLOCK_REFUSED, message, None)))
        }
        Err(Failed::Full(message)) => Err(Stop::failed(CommandError {
            code: codes::SLOTS_EXHAUSTED.into(),
            message,
            ..Default::default()
        })),
    }
}

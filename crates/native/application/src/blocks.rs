//! Blocks between the contract and the shared core (docs/adr/0144): the
//! drawing's definitions as the core flattens them (`kentos_geometry_core::block`),
//! and an insert's placed pieces back as the drawing's own kind of objects,
//! for the hosts that draw objects rather than the store's records (the
//! desktop's selection highlight and plain scene).

use std::sync::Arc;

use kentos_contracts::{BlockDefinition, Entity, EntityBase};
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::block::{Blocks, Definition};
use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::store::Expanded;

use crate::geometry::{edit_geometry, entity_of, shape};

/// The drawing's definitions for the core.
pub fn core_blocks(blocks: &[Arc<BlockDefinition>]) -> Blocks {
    Blocks::new(
        blocks
            .iter()
            .map(|b| Definition {
                id: b.id.to_text(),
                base: kentos_geometry_core::Vec2::new(b.base.x, b.base.y),
                entities: b.entities.iter().map(core_entity).collect(),
            })
            .collect(),
    )
}

/// An object as the core takes it: its shape, and the fields a block's
/// flattening reads (its layer, its own colour and line weight).
pub fn core_entity(e: &Entity) -> CoreEntity {
    let base = e.base();
    let mut rest = vec![
        ("id".to_owned(), Json::Num(f64::from(base.id))),
        ("layerId".to_owned(), Json::Str(base.layer_id.clone())),
    ];
    if let Some(c) = &base.color {
        rest.push(("color".to_owned(), Json::Str(c.clone())));
    }
    if let Some(w) = base.line_weight {
        rest.push(("lineWeight".to_owned(), Json::Num(w)));
    }
    CoreEntity {
        shape: shape(e),
        rest,
    }
}

/// An insert's pieces as objects: each its placed shape, on the insert's
/// layer, with its own colour and line weight, else the insert's; the
/// insert's slot, no attributes. A piece the contract cannot hold is left out.
pub fn piece_entities(insert: &Entity, x: &Expanded) -> Vec<Entity> {
    let own = insert.base();
    x.shapes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let geometry = edit_geometry(s.clone())?;
            let base = EntityBase {
                id: own.id,
                layer_id: own.layer_id.clone(),
                color: x
                    .colors
                    .get(i)
                    .cloned()
                    .flatten()
                    .or_else(|| own.color.clone()),
                attrs: Default::default(),
                label: None,
                symbol: None,
                line_weight: x.weights.get(i).copied().flatten().or(own.line_weight),
            };
            Some(entity_of(&geometry, base))
        })
        .collect()
}

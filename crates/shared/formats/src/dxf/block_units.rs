//! Definitions keep their own physical size, including after a file-unit override (ADR 0589).
//! INSERT's stored factors already position the raw definition in its owner's coordinates.
//! Changing that definition's unit therefore requires compensating every reference to it.

use std::collections::HashMap;

use kentos_contracts::{BlockDefinition, BlockId, Entity};

use crate::units::{self, Scale};

pub(super) fn normalize(
    entities: &mut [Entity],
    blocks: &mut [BlockDefinition],
    drawing: Scale,
    records: &[(String, i64)],
) {
    let scales: HashMap<BlockId, Scale> = blocks
        .iter()
        .map(|b| {
            let own = records
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(&b.name))
                .and_then(|(_, code)| units::insunits(*code))
                .map_or(drawing, |(scale, _)| scale);
            (b.id, own)
        })
        .collect();
    for e in entities {
        units::entity(e, drawing);
        compensate(e, drawing, &scales);
    }
    for b in blocks {
        let own = scales.get(&b.id).copied().unwrap_or(drawing);
        units::block(b, own);
        for e in &mut b.entities {
            compensate(e, own, &scales);
        }
    }
}

fn compensate(e: &mut Entity, owner: Scale, scales: &HashMap<BlockId, Scale>) {
    if let Entity::Insert(i) = e
        && let Some(target) = scales.get(&i.block)
    {
        let ratio = owner.mul * target.div / (owner.div * target.mul);
        i.scale *= ratio;
        if let Some(y) = &mut i.y_scale {
            *y *= ratio;
        }
    }
}

//! CAD extents use visible curve spans and placed definitions, with cached local boxes rather than instance expansion.
use std::collections::{HashMap, HashSet};
use kentos_contracts::{BlockDefinition, BlockId, Bounds, Entity};
use kentos_geometry_core::{block::insert_matrix_xy, geom::affine::apply, Vec2 as Point};
pub(crate) struct ModelBounds<'a> {
    definitions: HashMap<BlockId, &'a BlockDefinition>,
    cached: HashMap<BlockId, Option<Bounds>>,
    active: HashSet<BlockId>,
}
impl<'a> ModelBounds<'a> {
    pub fn new(blocks: impl IntoIterator<Item = &'a BlockDefinition>) -> Self {
        Self {
            definitions: blocks.into_iter().map(|b| (b.id, b)).collect(),
            cached: HashMap::new(),
            active: HashSet::new(),
        }
    }
    pub fn entity(&mut self, e: &Entity) -> Option<Bounds> {
        self.at(e, 0)
    }
    fn at(&mut self, e: &Entity, depth: usize) -> Option<Bounds> {
        if let Entity::Insert(i) = e {
            let Some(definition) = self.definitions.get(&i.block).copied() else {
                return crate::import::defining_bounds(e);
            };
            if depth >= kentos_geometry_core::block::MAX_DEPTH || !self.active.insert(i.block) {
                return crate::import::defining_bounds(e);
            }
            let local = if let Some(b) = self.cached.get(&i.block) {
                *b
            } else {
                let mut b = None;
                for child in &definition.entities {
                    b = join(b, self.at(child, depth + 1));
                }
                self.cached.insert(i.block, b);
                b
            };
            self.active.remove(&i.block);
            let Some(b) = local else {
                return crate::import::defining_bounds(e);
            };
            let matrix = insert_matrix_xy(
                Point {
                    x: definition.base.x,
                    y: definition.base.y,
                },
                Point { x: i.p.x, y: i.p.y },
                (i.scale, i.y_scale.unwrap_or(i.scale)),
                i.rotation,
                i.mirror,
            );
            let mut transformed = None;
            for (x, y) in [
                (b.min_x, b.min_y),
                (b.max_x, b.min_y),
                (b.max_x, b.max_y),
                (b.min_x, b.max_y),
            ] {
                let p = apply(&matrix, Point { x, y });
                transformed = join(
                    transformed,
                    Some(Bounds {
                        min_x: p.x,
                        min_y: p.y,
                        max_x: p.x,
                        max_y: p.y,
                    }),
                );
            }
            return transformed;
        }
        if matches!(
            e,
            Entity::Arc(_) | Entity::Ellipse(_) | Entity::Polyline(_) | Entity::Polygon(_)
        ) {
            let b = kentos_geometry_core::entity::entity_bounds(&crate::blocks::shape(e));
            if [b.min_x, b.min_y, b.max_x, b.max_y]
                .iter()
                .all(|x| x.is_finite())
                && b.min_x <= b.max_x
                && b.min_y <= b.max_y
            {
                return Some(Bounds {
                    min_x: b.min_x,
                    min_y: b.min_y,
                    max_x: b.max_x,
                    max_y: b.max_y,
                });
            }
        }
        crate::import::defining_bounds(e)
    }
}
pub(crate) fn join(a: Option<Bounds>, b: Option<Bounds>) -> Option<Bounds> {
    match (a, b) {
        (Some(a), Some(b)) => Some(Bounds {
            min_x: a.min_x.min(b.min_x),
            min_y: a.min_y.min(b.min_y),
            max_x: a.max_x.max(b.max_x),
            max_y: a.max_y.max(b.max_y),
        }),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}

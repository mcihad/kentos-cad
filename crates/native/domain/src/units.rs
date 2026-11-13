//! What a unit change writes besides the objects and the block definitions
//! (docs/adr/0256 §1): the drawing unit, the home view and the drawing's
//! anchor and current/named UCS origins. The unit is a project setting, which is no undo step
//! (docs/adr/0049); a unit change that keeps the numbers scales the objects
//! with it, and undo must give both back, so it writes them as an op of its
//! own in its step.

use kentos_contracts::{Bounds, DrawingUnit, NamedUcs, Ucs, Vec2};

use crate::document::Document;
use crate::history::Op;

/// The drawing unit (none: metres), where the view opens and the anchor the
/// GPU works from, together with the drawing's current and named UCS frames.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitState {
    pub drawing_unit: Option<DrawingUnit>,
    pub home_view: Option<Bounds>,
    pub origin: Vec2,
    pub ucs: Option<Ucs>,
    pub named_ucs: Vec<NamedUcs>,
}

impl Document {
    /// Correct the model's unit as one batch, including the recipes owning its objects.
    /// Their exact child writes are authorized together; a correction is never a manual
    /// array-item move or an illegal edit of an automatically generated dimension.
    pub fn scale_unit_objects(&mut self, scale: kentos_contracts::units::Scale, label: &str) {
        use crate::store::Stored;
        use kentos_contracts::units::{
            scale_dimension_look, scale_entity, scale_insert_details, scale_insert_parameters,
        };
        use kentos_contracts::{ArrayLayout, Entity};
        use std::sync::Arc;
        let mut ops = Vec::new();
        for stored in self
            .store
            .iter()
            .filter(|stored| !matches!(&*stored.entity, Entity::PointCloud(_)))
        {
            let e = &*stored.entity;
            let before = stored.clone();
            let mut entity = e.clone();
            scale_entity(&mut entity, scale);
            if let Entity::Insert(i) = &mut entity {
                scale_insert_details(i, scale);
                if let Some(d) = self.block(i.block).and_then(|b| b.dynamic.as_ref()) {
                    scale_insert_parameters(i, d, scale);
                }
            }
            if entity != *e {
                ops.push(Op::Update {
                    before: before.clone(),
                    after: Stored {
                        uid: before.uid,
                        entity: Arc::new(entity),
                    },
                });
            }
        }
        let point = |p: &mut Vec2| {
            p.x = scale.apply(p.x);
            p.y = scale.apply(p.y);
        };
        for before in &self.arrays {
            let mut after = (**before).clone();
            point(&mut after.p);
            match &mut after.layout {
                ArrayLayout::Grid { dx, dy, .. } | ArrayLayout::Lattice { dx, dy, .. } | ArrayLayout::Line { dx, dy, .. } => {
                    *dx = scale.apply(*dx);
                    *dy = scale.apply(*dy);
                }
                ArrayLayout::Polar { center, .. } => point(center),
                ArrayLayout::Path { spacing, .. } => *spacing = spacing.map(|v| scale.apply(v)),
            }
            // Also record an origin-centred polar or fitted path recipe whose numbers
            // did not change: it owns this batch's item changes all the same.
            ops.push(Op::ArrayUpdate {
                before: before.clone(),
                after: Arc::new(after),
            });
        }
        let writes: std::collections::HashMap<_, _> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Update { before, after } => {
                    Some((after.uid, (before.entity.clone(), after.entity.clone())))
                }
                _ => None,
            })
            .collect();
        for before in &self.dimension_sets {
            let mut after = (**before).clone();
            point(&mut after.at);
            after.height = scale.apply(after.height);
            scale_dimension_look(&mut after.look, scale);
            for row in &mut after.row_places {
                point(&mut row.at);
            }
            for member in &mut after.members {
                if let Some(shift) = &mut member.shift {
                    shift.along = scale.apply(shift.along);
                    shift.across = scale.apply(shift.across);
                }
            }
            // Only this method's exact member updates confer permission, never another
            // operation in a script's enclosing transaction (ADR 0730).
            let owned = before
                .members
                .iter()
                .filter_map(|member| {
                    let uid = crate::Uuid::from_bytes(member.uid.0);
                    writes.get(&uid).map(|(b, a)| (uid, b.clone(), a.clone()))
                })
                .collect();
            ops.push(Op::DimensionSetUpdate {
                before: before.clone(),
                after: Arc::new(after),
                writes: owned,
            });
        }
        self.record(ops, label);
    }

    /// The plot scale written as a change of the open step or group (its annotations follow in the same one, so one
    /// undo gives both back); outside one, an undo step of its own named `label`.
    pub fn set_plot_scale_in_step(&mut self, scale: f64, label: &str) -> bool {
        let before = self.settings.plot_scale;
        if !(scale.is_finite() && scale > 0.0) || before == scale {
            return false;
        }
        self.record(vec![Op::PlotScale { before, after: scale }], label);
        // What reads the scale, as `set_settings` refreshes it.
        self.refresh_viewmarks();
        self.refresh_dimension_set_rows();
        true
    }

    /// The drawing unit, the home view and the anchor as they are.
    pub fn unit_state(&self) -> UnitState {
        UnitState {
            drawing_unit: self.settings.drawing_unit,
            home_view: self.home_view,
            origin: self.origin,
            ucs: self.settings.ucs,
            named_ucs: self.settings.named_ucs.clone(),
        }
    }

    /// Writes them as one change named `label`: into the open transaction or
    /// step, else as an undo step of its own. Metres are written as none (a
    /// project's settings keep them so). Whether anything changed.
    pub fn set_unit_state(&mut self, next: UnitState, label: &str) -> bool {
        let next = UnitState {
            drawing_unit: next.drawing_unit.filter(|u| *u != DrawingUnit::M),
            ..next
        };
        let before = self.unit_state();
        if before == next {
            return false;
        }
        self.record(
            vec![Op::Units {
                before,
                after: next,
            }],
            label,
        );
        true
    }
}

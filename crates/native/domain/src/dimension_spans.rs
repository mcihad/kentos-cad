//! Façade and space dimension recipes (docs/adr/0729): the objects a recipe reads from the drawing, given to the
//! shared rule (`kentos_formats::dimension_spans`, which the server checks with), and its plan with each row's account.

use kentos_contracts::{DimensionSet, DimensionSetBreak, DimensionSetRow, DimensionSetRowPlace, Entity, EntityId, Vec2};
use kentos_geometry_core::ops::dimension_set as core;
use kentos_geometry_core::ops::dimension_span as span;
use kentos_geometry_core::vec2::Vec2 as CoreVec2;

use crate::document::Document;
use crate::identity::Uuid;

pub use kentos_formats::dimension_spans::{Box4, REACH, contract_row, core_row, meets, reach_box};

/// Why a recipe cannot be planned, for a command to say: the break it gets, and the source and why when one is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpanRefusal {
    pub broken: DimensionSetBreak,
    pub source: Option<EntityId>,
    pub why: Option<span::Why>,
}

/// A row's account (docs/adr/0729 §5): its points, the objects they come from, the spans it draws.
#[derive(Clone, Debug, PartialEq)]
pub struct SpanRow {
    pub row: DimensionSetRow,
    pub points: usize,
    pub sources: usize,
    pub spans: usize,
}

/// A planned recipe's account: its rows, the walls near it left unmeasured, the side its rows go to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpanNotes {
    pub rows: Vec<SpanRow>,
    pub skipped: Vec<EntityId>,
    pub side: f64,
}

/// A recipe's spans with the identities of the source numbers their points name, its account and its rows' frame.
pub(crate) struct SpanPlan {
    pub spans: Vec<core::Span>,
    pub sources: Vec<EntityId>,
    pub notes: SpanNotes,
    pub origin: CoreVec2,
    pub direction: CoreVec2,
    pub body: (f64, f64),
}

impl SpanPlan {
    /// Across the direction (to its left) from the frame's origin.
    fn across(&self, q: CoreVec2) -> f64 {
        (q.x - self.origin.x) * -self.direction.y + (q.y - self.origin.y) * self.direction.x
    }

    /// The bodies' outermost face on the rows' side, across.
    fn face(&self) -> f64 {
        if self.notes.side > 0.0 { self.body.1 } else { self.body.0 }
    }

    /// Where a row's line runs: its place by hand, else its drawn spans' line, else the recipe's place.
    fn row_at(&self, set: &DimensionSet, which: DimensionSetRow) -> Result<CoreVec2, DimensionSetBreak> {
        if !set.rows.contains(&which) {
            return Err(DimensionSetBreak::UnsupportedWall);
        }
        if let Some(place) = set.row_places.iter().find(|r| r.row == which) {
            return Ok(CoreVec2::new(place.at.x, place.at.y));
        }
        let row = core_row(which);
        Ok(self
            .spans
            .iter()
            .find(|s| s.row == row)
            .map_or(CoreVec2::new(set.at.x, set.at.y), |s| s.at))
    }
}

impl Document {
    fn entity_of(&self, id: EntityId) -> Option<&Entity> {
        self.slot_of(Uuid::from_bytes(id.0))
            .and_then(|slot| self.get(slot))
    }

    /// The box a recipe reads in: its sources' bodies, grown by [`REACH`]; none when no source is left.
    pub fn dimension_span_reach(&self, set: &DimensionSet) -> Option<Box4> {
        kentos_formats::dimension_spans::reach(set, &|id| self.entity_of(id))
    }

    /// A façade or space recipe's spans (docs/adr/0729 §2): the drawing's walls, openings and grids in its box given to
    /// the shared rule.
    pub(crate) fn dimension_span_spans(&self, set: &DimensionSet) -> Result<SpanPlan, SpanRefusal> {
        let refused = |r: kentos_formats::dimension_spans::Refusal| SpanRefusal {
            broken: r.broken,
            source: r.source,
            why: r.why,
        };
        let reach = self.dimension_span_reach(set);
        let sources: Vec<String> = set.sources().iter().map(|id| id.to_string()).collect();
        // Every grid, the openings of the sources wherever they are (a misplaced one breaks the recipe), the walls and
        // the other openings in the box.
        let near: Vec<kentos_formats::dimension_spans::Near<'_>> = match reach {
            Some(b) => self
                .entities_with_uids()
                .filter(|(_, e)| match e {
                    Entity::Grid(_) => true,
                    Entity::Opening(o) if sources.contains(&o.opening.host) => true,
                    Entity::Wall(_) | Entity::Opening(_) => reach_box(e).is_some_and(|r| meets(&r, &b)),
                    _ => false,
                })
                .map(|(uid, e)| kentos_formats::dimension_spans::Near {
                    id: EntityId(uid.into_bytes()),
                    entity: e,
                    shown: self.layers.is_visible(&e.base().layer_id),
                })
                .collect(),
            None => Vec::new(),
        };
        let planned = kentos_formats::dimension_spans::plan(
            set,
            &|id| self.entity_of(id),
            &near,
            self.settings.plot_scale,
        )
        .map_err(refused)?;
        Ok(SpanPlan {
            notes: SpanNotes {
                rows: planned
                    .rows
                    .iter()
                    .map(|r| SpanRow {
                        row: r.row,
                        points: r.points,
                        sources: r.sources,
                        spans: r.spans,
                    })
                    .collect(),
                skipped: planned.skipped,
                side: planned.side,
            },
            spans: planned.spans,
            sources: planned.sources,
            origin: planned.origin,
            direction: planned.direction,
            body: planned.body,
        })
    }

    /// A façade's or a space's row's distance (docs/adr/0729 §6): from its bodies' outermost face on the rows' side,
    /// outward positive.
    pub fn dimension_span_row_offset(&self, set: &DimensionSet, which: DimensionSetRow) -> Result<f64, DimensionSetBreak> {
        let plan = self.dimension_span_spans(set).map_err(|r| r.broken)?;
        let at = plan.row_at(set, which)?;
        Ok(plan.notes.side * (plan.across(at) - plan.face()))
    }

    /// A façade's or a space's row placed `offset` out from its bodies' outermost face, where it runs along.
    pub fn set_dimension_span_row_offset(
        &self,
        set: &mut DimensionSet,
        which: DimensionSetRow,
        offset: f64,
    ) -> Result<(), DimensionSetBreak> {
        let plan = self.dimension_span_spans(set).map_err(|r| r.broken)?;
        let now = plan.row_at(set, which)?;
        let delta = plan.face() + plan.notes.side * offset - plan.across(now);
        let (nx, ny) = (-plan.direction.y, plan.direction.x);
        let at = Vec2 {
            x: now.x + nx * delta,
            y: now.y + ny * delta,
        };
        match set.row_places.iter_mut().find(|r| r.row == which) {
            Some(place) => place.at = at,
            None => set.row_places.push(DimensionSetRowPlace { row: which, at }),
        }
        Ok(())
    }
}

//! What a unit change writes besides the objects and the block definitions
//! (docs/adr/0256 §1): the drawing unit, the home view and the drawing's
//! anchor. The unit is a project setting, which is no undo step
//! (docs/adr/0049); a unit change that keeps the numbers scales the objects
//! with it, and undo must give both back, so it writes them as an op of its
//! own in its step.

use kentos_contracts::{Bounds, DrawingUnit, Vec2};

use crate::document::Document;
use crate::history::Op;

/// The drawing unit (none: metres), where the view opens and the anchor the
/// GPU works from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitState {
    pub drawing_unit: Option<DrawingUnit>,
    pub home_view: Option<Bounds>,
    pub origin: Vec2,
}

impl Document {
    /// The drawing unit, the home view and the anchor as they are.
    pub fn unit_state(&self) -> UnitState {
        UnitState {
            drawing_unit: self.settings.drawing_unit,
            home_view: self.home_view,
            origin: self.origin,
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

//! A zoning plan's kept symbol choices and plan change drafts as an undo step (plan/map-cad.md MAP-09, docs/adr/5040).
//! They live in the project's settings, which are otherwise outside the model's undo.

use std::sync::Arc;

use kentos_contracts::ZoningPlanData;

use crate::{Document, history::Op};

impl Document {
    /// The drawing's zoning plan data; none in a drawing without any.
    pub fn zoning_plan(&self) -> Option<&ZoningPlanData> {
        self.settings.zoning_plan.as_ref()
    }

    /// Replaces the data as one undo step named `label` (inside an open transaction or group, part of it); empty data
    /// is none. Nothing happens when it is the same.
    pub fn set_zoning_plan(&mut self, next: Option<ZoningPlanData>, label: &str) {
        let next = next.filter(|d| !d.is_empty());
        if self.settings.zoning_plan == next {
            return;
        }
        let before = Arc::new(std::mem::replace(
            &mut Arc::make_mut(&mut self.settings).zoning_plan,
            next,
        ));
        let after = Arc::new(self.settings.zoning_plan.clone());
        self.record(vec![Op::ZoningPlan { before, after }], label);
    }
}

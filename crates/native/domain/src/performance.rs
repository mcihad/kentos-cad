//! The building's declared fire protection class (docs/adr/8068): a setting the drawing keeps in its undo history, a
//! step of its own (`cad.arch.performance`'s building). The objects' and types' declarations are their own fields.

use kentos_contracts::BuildingPerformance;

use crate::Refusal;
use crate::document::Document;
use crate::history::ArchOp;
use crate::history::Op;

impl Document {
    /// The building's declared performance, none when nothing is declared.
    pub fn building_performance(&self) -> Option<&BuildingPerformance> {
        self.settings.arch.as_ref().and_then(|a| a.performance.as_ref())
    }

    /// Gives the building `next` as one change named `label` (into the open transaction, else an undo step of its own);
    /// an empty one is none. Refused with nothing changed: a declaration with a problem. Whether anything changed.
    pub fn set_building_performance(
        &mut self,
        next: Option<BuildingPerformance>,
        label: &str,
    ) -> Result<bool, Refusal> {
        let next = next.filter(|p| !p.is_empty());
        if let Some(why) = next
            .as_ref()
            .and_then(|p| p.fire_protection_class.as_ref())
            .and_then(|d| d.problem("binanın yangın koruma sınıfı"))
        {
            return Err(Refusal(why));
        }
        let before = self.building_performance().cloned();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::Arch(ArchOp::Performance { before, after: next })], label);
            Ok(true)
        })
    }
}

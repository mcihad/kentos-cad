//! The project's design options (docs/adr/0788): a setting the drawing keeps in its undo history, written in the step
//! with the objects an option copies or deletes, so undo gives both back. What a view leaves out of the drawing is
//! worked out here once: every set's options but the one it shows (the primary unless the view chooses another).

use std::collections::{BTreeSet, HashMap};

use kentos_contracts::{OptionChoice, OptionSet, option_sets_problem};

use crate::Refusal;
use crate::document::Document;
use crate::history::Op;
use crate::{Slot, Uuid};

impl Document {
    /// The project's design option sets, in the order they were made.
    pub fn arch_options(&self) -> &[OptionSet] {
        self.settings
            .arch
            .as_ref()
            .map_or(&[], |a| a.options.as_slice())
    }

    /// Gives the project the option sets `next` as one change named `label` (into the open transaction, else an undo
    /// step of its own). Refused with nothing changed: sets with a problem (`option_sets_problem`). Whether anything
    /// changed.
    pub fn set_arch_options(&mut self, next: Vec<OptionSet>, label: &str) -> Result<bool, Refusal> {
        if let Some((path, problem)) = option_sets_problem(&next) {
            return Err(Refusal(format!("{path}: {problem}")));
        }
        let before = self.arch_options().to_vec();
        if before == next {
            return Ok(false);
        }
        self.transact(label, |doc| {
            doc.record(vec![Op::ArchOptions { before, after: next }], label);
            Ok(true)
        })
    }

    /// The objects a view showing `choices` (each set's primary for the sets it does not name) leaves out, by their
    /// slots; none when the project has no options.
    pub fn option_hidden_slots(&self, choices: &[OptionChoice]) -> Vec<Slot> {
        let sets = self.arch_options();
        if sets.is_empty() {
            return Vec::new();
        }
        kentos_contracts::option_hidden_uids(sets, choices)
            .into_iter()
            .filter_map(|u| Uuid::parse_str(u).ok())
            .filter_map(|u| self.slot_of(u))
            .collect()
    }

    /// [`option_hidden_slots`] as a set, for the primary view.
    pub fn option_out(&self) -> BTreeSet<Slot> {
        self.option_hidden_slots(&[]).into_iter().collect()
    }

    /// The view an object of an option sees (docs/adr/0788 §4): its own option for its set, each other set's primary;
    /// an object of the main model sees the primaries.
    pub fn option_view_of(&self, uid: Uuid) -> Vec<OptionChoice> {
        let text = uid.to_string();
        match kentos_contracts::option_of(self.arch_options(), &text) {
            Some((s, o)) => vec![OptionChoice {
                set: s.id.clone(),
                option: o.id.clone(),
            }],
            None => Vec::new(),
        }
    }

    /// Each option's object by its slot: the set's and the option's places in the project's list.
    pub fn option_groups(&self) -> HashMap<Slot, (usize, usize)> {
        let mut out = HashMap::new();
        for (i, s) in self.arch_options().iter().enumerate() {
            for (j, o) in s.options.iter().enumerate() {
                for m in &o.members {
                    if let Some(slot) = Uuid::parse_str(m).ok().and_then(|u| self.slot_of(u)) {
                        out.insert(slot, (i, j));
                    }
                }
            }
        }
        out
    }
}

/// Whether two objects may meet (docs/adr/0788 §4): not two options of one set; the main model meets every option.
pub fn options_meet(groups: &HashMap<Slot, (usize, usize)>, a: Slot, b: Slot) -> bool {
    match (groups.get(&a), groups.get(&b)) {
        (Some(x), Some(y)) => x.0 != y.0 || x.1 == y.1,
        _ => true,
    }
}

/// The persistent ids an option-set change touched: every member before and after.
pub(crate) fn touched_members(before: &[OptionSet], after: &[OptionSet]) -> BTreeSet<Uuid> {
    before
        .iter()
        .chain(after)
        .flat_map(|s| s.options.iter())
        .flat_map(|o| o.members.iter())
        .filter_map(|m| Uuid::parse_str(m).ok())
        .collect()
}

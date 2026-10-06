//! The selection: which objects are selected, in the order they were, and
//! the one under the pointer (`apps/web/src/model/selection.ts`). It is
//! session state, not the drawing's (CLAUDE.md §4.4): nothing here is saved
//! or undone. A command never reads it: the tool that acts on it gives its
//! objects to the command explicitly (TODOS.md CMD-07, `cad.entities.delete`).

use std::collections::HashSet;

use kentos_domain::Slot;

use crate::Vec2;

/// Sıradakini seç's chip (docs/adr/0187 §1): the objects a click could mean,
/// the most specific first, which of them is chosen, and where the click was
/// (the drawing's coordinates). It holds while the selection is the one it made.
#[derive(Clone, Debug, PartialEq)]
pub struct Cycle {
    pub at: Vec2,
    pub candidates: Vec<Slot>,
    pub index: usize,
}

/// The selected objects and the hovered one.
#[derive(Clone, Debug, Default)]
pub struct Selection {
    ids: Vec<Slot>,
    set: HashSet<Slot>,
    hover: Option<Slot>,
    /// Counts changes to the selected set, so a view redraws only then.
    version: u64,
    /// Counts changes to the hovered object.
    hover_version: u64,
    /// The last selection that held something and was replaced or cleared
    /// (Önceki seçim, docs/adr/0187 §3).
    previous: Vec<Slot>,
    /// Sıradakini seç's chip; none while the selection is not a click's among several objects.
    cycle: Option<Cycle>,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    /// The selected objects, in the order they were selected.
    pub fn ids(&self) -> &[Slot] {
        &self.ids
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn contains(&self, id: Slot) -> bool {
        self.set.contains(&id)
    }

    /// Selects exactly these (the web's `set`); the selection it replaces
    /// becomes the previous one.
    pub fn set(&mut self, ids: impl IntoIterator<Item = Slot>) {
        self.cycle = None;
        let mut next = Selection::default();
        next.extend(ids);
        if next.ids != self.ids {
            // Another order of the same objects is no new selection (the web's set equality).
            if !self.ids.is_empty() && next.set != self.set {
                self.previous = std::mem::take(&mut self.ids);
            }
            self.ids = next.ids;
            self.set = next.set;
            self.version += 1;
        }
    }

    /// Adds these after the ones selected (the web's `add`: Shift with a window).
    pub fn add(&mut self, ids: impl IntoIterator<Item = Slot>) {
        self.cycle = None;
        let before = self.ids.len();
        self.extend(ids);
        if self.ids.len() != before {
            self.version += 1;
        }
    }

    /// What a selecting tool found (docs/adr/0141): it replaces the selection,
    /// or with `add` (Shift held) joins it.
    pub fn take(&mut self, ids: impl IntoIterator<Item = Slot>, add: bool) {
        if add {
            self.add(ids);
        } else {
            self.set(ids);
        }
    }

    /// Selects or deselects one (the web's `toggle`: Shift with a click).
    pub fn toggle(&mut self, id: Slot) {
        self.cycle = None;
        if self.set.remove(&id) {
            self.ids.retain(|s| *s != id);
        } else {
            self.set.insert(id);
            self.ids.push(id);
        }
        self.version += 1;
    }

    /// Selects nothing; the selection it clears becomes the previous one.
    pub fn clear(&mut self) {
        self.cycle = None;
        if !self.ids.is_empty() {
            self.previous = std::mem::take(&mut self.ids);
            self.set.clear();
            self.version += 1;
        }
    }

    /// The last selection that held something and was replaced or cleared,
    /// in its order (Önceki seçim, docs/adr/0187 §3): what the caller brings
    /// back of it.
    pub fn previous(&self) -> &[Slot] {
        &self.previous
    }

    /// Sıradakini seç's chip, while it holds.
    pub fn cycle(&self) -> Option<&Cycle> {
        self.cycle.as_ref()
    }

    /// Opens the chip after a click among several objects (the first chosen).
    pub fn start_cycle(&mut self, at: Vec2, candidates: Vec<Slot>) {
        self.cycle = (candidates.len() > 1).then_some(Cycle {
            at,
            candidates,
            index: 0,
        });
    }

    /// Closes the chip (a command started, Esc).
    pub fn end_cycle(&mut self) {
        self.cycle = None;
    }

    /// Sıradakini seç: the chip's `index`th candidate takes the place of the
    /// one chosen, the rest of the selection kept; the previous selection
    /// stays what it was.
    pub fn cycle_to(&mut self, index: usize) {
        let Some(c) = &self.cycle else {
            return;
        };
        let n = c.candidates.len();
        if n == 0 {
            return;
        }
        let i = index % n;
        let (old, new) = (c.candidates[c.index], c.candidates[i]);
        if self.set.remove(&old) {
            self.ids.retain(|s| *s != old);
        }
        if self.set.insert(new) {
            self.ids.push(new);
        }
        self.version += 1;
        if let Some(c) = &mut self.cycle {
            c.index = i;
        }
    }

    /// Drops the objects that no longer exist (an undo, a delete), and the
    /// hovered one with them (the web's `retain`).
    pub fn retain(&mut self, exists: impl Fn(Slot) -> bool) {
        let before = self.ids.len();
        self.ids.retain(|s| exists(*s));
        if self.ids.len() != before {
            self.cycle = None;
            self.set = self.ids.iter().copied().collect();
            self.version += 1;
        }
        if self.hover.is_some_and(|h| !exists(h)) {
            self.set_hover(None);
        }
    }

    /// The object under the pointer, highlighted unless it is selected.
    pub fn hover(&self) -> Option<Slot> {
        self.hover
    }

    pub fn set_hover(&mut self, hover: Option<Slot>) {
        if self.hover != hover {
            self.hover = hover;
            self.hover_version += 1;
        }
    }

    /// Changes to the selected set so far.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Changes to the hovered object so far.
    pub fn hover_version(&self) -> u64 {
        self.hover_version
    }

    fn extend(&mut self, ids: impl IntoIterator<Item = Slot>) {
        for id in ids {
            if self.set.insert(id) {
                self.ids.push(id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_keeps_the_order_and_counts_its_changes() {
        let mut s = Selection::new();
        s.set([Slot(3), Slot(1), Slot(3)]);
        assert_eq!(s.ids(), [Slot(3), Slot(1)]);
        let v = s.version();
        s.set([Slot(3), Slot(1)]);
        assert_eq!(s.version(), v, "the same set is no change");
        s.add([Slot(1), Slot(7)]);
        assert_eq!(s.ids(), [Slot(3), Slot(1), Slot(7)]);
        s.toggle(Slot(1));
        assert_eq!(s.ids(), [Slot(3), Slot(7)]);
        s.toggle(Slot(1));
        assert_eq!(s.ids(), [Slot(3), Slot(7), Slot(1)]);
        s.set_hover(Some(Slot(7)));
        s.retain(|id| id != Slot(7));
        assert_eq!((s.ids(), s.hover()), (&[Slot(3), Slot(1)][..], None));
        s.clear();
        assert!(s.is_empty() && !s.contains(Slot(3)));
    }

    #[test]
    fn a_replaced_or_cleared_selection_is_the_previous_one() {
        let mut s = Selection::new();
        s.set([Slot(1), Slot(2)]);
        assert!(s.previous().is_empty());
        s.add([Slot(3)]);
        s.toggle(Slot(1));
        assert!(s.previous().is_empty(), "adding and taking out keep it");
        s.set([Slot(9)]);
        assert_eq!(s.previous(), [Slot(2), Slot(3)]);
        s.set([Slot(9)]);
        assert_eq!(
            s.previous(),
            [Slot(2), Slot(3)],
            "the same set is no change"
        );
        s.clear();
        assert_eq!(s.previous(), [Slot(9)]);
        s.clear();
        assert_eq!(
            s.previous(),
            [Slot(9)],
            "an empty selection is never the previous"
        );
    }

    #[test]
    fn the_chip_cycles_the_one_chosen_and_closes_with_any_other_change() {
        let mut s = Selection::new();
        s.set([Slot(7), Slot(1)]);
        let before = s.previous().to_vec();
        s.start_cycle(Vec2::new(1.0, 2.0), vec![Slot(1), Slot(4), Slot(5)]);
        s.cycle_to(1);
        assert_eq!(s.ids(), [Slot(7), Slot(4)]);
        s.cycle_to(5);
        assert_eq!(
            (s.ids(), s.cycle().map(|c| c.index)),
            (&[Slot(7), Slot(5)][..], Some(2))
        );
        assert_eq!(s.previous(), before, "cycling is not a new selection");
        s.add([Slot(8)]);
        assert!(s.cycle().is_none());
        s.start_cycle(Vec2::new(0.0, 0.0), vec![Slot(1)]);
        assert!(s.cycle().is_none(), "one candidate has no chip");
    }
}

//! The selection: which objects are selected, in the order they were, and
//! the one under the pointer (`apps/web/src/model/selection.ts`). It is
//! session state, not the drawing's (CLAUDE.md §4.4): nothing here is saved
//! or undone. A command never reads it: the tool that acts on it gives its
//! objects to the command explicitly (TODOS.md CMD-07, `cad.entities.delete`).

use std::collections::HashSet;

use kentos_domain::Slot;

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

    /// Selects exactly these (the web's `set`).
    pub fn set(&mut self, ids: impl IntoIterator<Item = Slot>) {
        let mut next = Selection::default();
        next.extend(ids);
        if next.ids != self.ids {
            self.ids = next.ids;
            self.set = next.set;
            self.version += 1;
        }
    }

    /// Adds these after the ones selected (the web's `add`: Shift with a window).
    pub fn add(&mut self, ids: impl IntoIterator<Item = Slot>) {
        let before = self.ids.len();
        self.extend(ids);
        if self.ids.len() != before {
            self.version += 1;
        }
    }

    /// Selects or deselects one (the web's `toggle`: Shift with a click).
    pub fn toggle(&mut self, id: Slot) {
        if self.set.remove(&id) {
            self.ids.retain(|s| *s != id);
        } else {
            self.set.insert(id);
            self.ids.push(id);
        }
        self.version += 1;
    }

    pub fn clear(&mut self) {
        if !self.ids.is_empty() {
            self.ids.clear();
            self.set.clear();
            self.version += 1;
        }
    }

    /// Drops the objects that no longer exist (an undo, a delete), and the
    /// hovered one with them (the web's `retain`).
    pub fn retain(&mut self, exists: impl Fn(Slot) -> bool) {
        let before = self.ids.len();
        self.ids.retain(|s| exists(*s));
        if self.ids.len() != before {
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
}

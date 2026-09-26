//! Which objects changed, for readers that follow the document without
//! rebuilding from it (the web's `touched` event, apps/web/src/model/document.ts):
//! the desktop's geometry store for picking, snapping and window selection
//! (docs/adr/0029), later the drawing area's layers (CLAUDE.md §4.9).
//!
//! Every applied op that adds, changes or removes an object (an edit, undo,
//! redo, a rolled back transaction, a cancelled group) notes its slot in a
//! journal. A reader keeps a [`ChangeMark`] and asks what changed since:
//! the slots, oldest first, or [`Changes::All`] when the journal no longer
//! reaches back that far. The journal is bounded: once it outgrows the
//! drawing it is emptied, since reading every object again then costs no
//! more than following it. Layer changes are not objects; a reader compares
//! the layer tree itself (it is small).

use crate::document::Document;
use crate::identity::Slot;

/// Entries kept at least, whatever the drawing's size.
const KEEP_AT_LEAST: usize = 1 << 16;

/// Where a reader of the document's changes stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChangeMark(u64);

/// What changed since a mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Changes<'a> {
    /// The objects at these slots were added, changed or removed, oldest
    /// first; a slot may appear more than once. Look each one up in the
    /// document: what is there now is what counts, and a slot it no longer
    /// has was removed.
    Slots(&'a [Slot]),
    /// More changed than the journal keeps: read every object again.
    All,
}

/// The slots applied ops touched, oldest first, after `dropped` earlier ones.
#[derive(Clone, Debug, Default)]
pub(crate) struct Journal {
    slots: Vec<Slot>,
    dropped: u64,
}

impl Journal {
    pub(crate) fn touch(&mut self, slot: Slot, objects: usize) {
        if self.slots.len() >= KEEP_AT_LEAST.max(objects.saturating_mul(2)) {
            self.dropped += self.slots.len() as u64;
            self.slots.clear();
        }
        self.slots.push(slot);
    }

    fn mark(&self) -> ChangeMark {
        ChangeMark(self.dropped + self.slots.len() as u64)
    }

    fn since(&self, mark: ChangeMark) -> Changes<'_> {
        match mark.0.checked_sub(self.dropped) {
            Some(start) => {
                let start = usize::try_from(start).unwrap_or(usize::MAX);
                self.slots.get(start..).map_or(Changes::All, Changes::Slots)
            }
            None => Changes::All,
        }
    }
}

impl Document {
    /// A reader's mark now: `changes_since` it later says what changed meanwhile.
    pub fn change_mark(&self) -> ChangeMark {
        self.history.journal.mark()
    }

    /// The objects added, changed or removed since `mark`, or `All` when the
    /// journal no longer reaches back to it (or the mark is not this
    /// document's: a reader of another drawing reads everything again).
    pub fn changes_since(&self, mark: ChangeMark) -> Changes<'_> {
        self.history.journal.since(mark)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reader_gets_what_was_touched_since_its_mark() {
        let mut journal = Journal::default();
        let start = journal.mark();
        journal.touch(Slot(3), 10);
        journal.touch(Slot(5), 10);
        let middle = journal.mark();
        journal.touch(Slot(3), 10);
        assert_eq!(
            journal.since(start),
            Changes::Slots(&[Slot(3), Slot(5), Slot(3)])
        );
        assert_eq!(journal.since(middle), Changes::Slots(&[Slot(3)]));
        assert_eq!(journal.since(journal.mark()), Changes::Slots(&[]));
        // A mark from the future (another document's) reads everything again.
        assert_eq!(journal.since(ChangeMark(99)), Changes::All);
    }

    #[test]
    fn the_journal_is_emptied_once_it_outgrows_the_drawing() {
        let mut journal = Journal::default();
        let start = journal.mark();
        for i in 0..KEEP_AT_LEAST as u32 {
            journal.touch(Slot(i), 10);
        }
        let late = journal.mark();
        assert!(matches!(journal.since(start), Changes::Slots(s) if s.len() == KEEP_AT_LEAST));
        journal.touch(Slot(7), 10);
        assert_eq!(journal.since(start), Changes::All);
        // A reader that had followed up to the drop loses nothing.
        assert_eq!(journal.since(late), Changes::Slots(&[Slot(7)]));
        assert_eq!(journal.since(journal.mark()), Changes::Slots(&[]));
        let after = journal.mark();
        journal.touch(Slot(8), 10);
        assert_eq!(journal.since(after), Changes::Slots(&[Slot(8)]));
    }
}

//! After a door, window or bare opening is placed (usage review UR-008): no preview and no warning until the
//! pointer moves, so the success is not followed at once by the next one's overlap warning; a move back onto the
//! one just placed shows the overlap, and a click there is refused.

use crate::common::Bench;
use kentos_interaction::{Level, Tone};

/// A 30 m wall along x, `tool` started.
fn wall(tool: &str) -> Bench {
    let mut b = Bench::new("wall");
    b.click(0.0, 0.0);
    b.click(30.0, 0.0);
    b.confirm();
    b.confirm();
    b.start(tool);
    b
}

#[test]
fn the_next_opening_waits_for_the_pointer_and_an_overlap_is_still_refused() {
    for (tool, label) in [("door", "Kapı"), ("window", "Pencere"), ("opening", "Boşluk")] {
        let mut b = wall(tool);
        b.move_to(5.0, 0.3);
        b.click(5.0, 0.3);
        let count = b.doc.len();
        assert_eq!(b.last_level(), Some(Level::Success), "{tool}");
        assert!(b.last_text().is_some_and(|t| t.starts_with(label)), "{tool}: {:?}", b.last_text());
        let still = b.session.preview(&Default::default()).unwrap_or_default();
        assert!(still.tag.is_none(), "{tool}: nothing beside the cursor until it moves");
        assert_ne!(still.tag_tone, Tone::Danger, "{tool}");

        // Moved, back onto the one just placed: the overlap is shown and a click there adds nothing.
        b.move_to(5.0, 0.3);
        let over = b.session.preview(&Default::default()).unwrap_or_default();
        assert!(over.tag.is_some(), "{tool}");
        assert_eq!(over.tag_tone, Tone::Danger, "{tool}: the overlap is said");
        b.click(5.0, 0.3);
        assert_eq!(b.doc.len(), count, "{tool}: no second opening on the first");

        // Elsewhere on the wall: the next one goes in, its undo the one step.
        b.move_to(15.0, 0.3);
        b.click(15.0, 0.3);
        assert_eq!(b.doc.len(), count + 1, "{tool}");
        assert!(b.undo_step() || b.doc.undo().is_some(), "{tool}: undone");
    }
}

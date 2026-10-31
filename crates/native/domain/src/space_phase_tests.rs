//! Which phase a space's boundary sources stand in (docs/adr/0798 §3).

use kentos_contracts::ArchPhase::{Demolish, Existing, New, Temporary};

use crate::space_follow::phase_bounds;

#[test]
fn a_room_before_the_works_is_bounded_by_what_stands_then_and_a_new_room_by_what_stands_after() {
    // Existing and to-be-demolished rooms: the existing and the demolished walls, not the new ones.
    for space in [Existing, Demolish] {
        assert!(phase_bounds(Some(space), Some(Existing)));
        assert!(phase_bounds(Some(space), Some(Demolish)));
        assert!(!phase_bounds(Some(space), Some(New)));
        assert!(!phase_bounds(Some(space), Some(Temporary)));
    }
    // A new room: the existing and the new walls, not the demolished ones nor the temporary.
    assert!(phase_bounds(Some(New), Some(Existing)));
    assert!(phase_bounds(Some(New), Some(New)));
    assert!(!phase_bounds(Some(New), Some(Demolish)));
    assert!(!phase_bounds(Some(New), Some(Temporary)));
    // Without a phase on either side, or a temporary room: every source, as before.
    for p in [None, Some(Existing), Some(Demolish), Some(New), Some(Temporary)] {
        assert!(phase_bounds(None, p));
        assert!(phase_bounds(Some(Temporary), p));
        assert!(phase_bounds(p, None));
    }
}

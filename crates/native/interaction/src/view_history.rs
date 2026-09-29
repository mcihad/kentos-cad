//! The view history behind Önceki görünüm and Sonraki görünüm (docs/adr/0141).
//!
//! A view is where the camera stands: the world point in the middle of the
//! drawing area and the zoom. The history is the session's, not the drawing's:
//! it never enters the document, undo or a file, and each window has its own.
//!
//! - A view is recorded as the user leaves it by navigating (the host decides
//!   which changes are navigations; the rules are the ADR's). The same view as
//!   the last record is not recorded again, and at most [`LIMIT`] are kept:
//!   the oldest goes first.
//! - [`ViewHistory::back`] puts the view being left on the “next” stack and
//!   returns the last record; [`ViewHistory::forward`] is the reverse.
//! - A new navigation ([`ViewHistory::record`]) empties the “next” stack.
//! - The wheel is one pass, however many steps it takes: only the first step
//!   after a pause of [`WHEEL_PAUSE`] records ([`ViewHistory::record_wheel`]).
//!   The time is the host's monotonic clock as a duration, so tests keep their own.
//! - A window resizing is no navigation and records nothing (the host does not call).
//!
//! Pure data, so that the rules are held by unit tests and the web's history
//! (the same rules, its own camera) can be compared with it.

use std::time::Duration;

use crate::Vec2;

/// The most views kept; the oldest is dropped past it.
pub const LIMIT: usize = 30;

/// How long the wheel must have rested for its next step to start a new pass.
pub const WHEEL_PAUSE: Duration = Duration::from_millis(500);

/// Where the camera stands: the world point in the middle of the area, and
/// logical pixels per world unit. The area's size is not part of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewpoint {
    pub center: Vec2,
    pub scale: f64,
}

/// The views left behind and the ones gone back from.
#[derive(Clone, Debug, Default)]
pub struct ViewHistory {
    /// The views recorded, the newest last.
    back: Vec<Viewpoint>,
    /// The views gone back from, the nearest last.
    next: Vec<Viewpoint>,
    /// When the wheel last recorded or stepped: the pass it is in.
    wheel: Option<Duration>,
}

impl ViewHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Keeps `view`, the one a navigation is leaving. A new navigation empties
    /// the views that “next” would go to, even when `view` is the last record
    /// already (that is kept once), and ends the wheel's pass: its next step
    /// starts another.
    pub fn record(&mut self, view: Viewpoint) {
        self.wheel = None;
        self.push(view);
    }

    fn push(&mut self, view: Viewpoint) {
        self.next.clear();
        if self.back.last() == Some(&view) {
            return;
        }
        self.back.push(view);
        if self.back.len() > LIMIT {
            self.back.remove(0);
        }
    }

    /// A step of the wheel at `at`: the first after a rest of [`WHEEL_PAUSE`]
    /// (or the first ever) records `view`, the one it leaves; the steps that
    /// follow soon after it do not. Whether it recorded.
    pub fn record_wheel(&mut self, view: Viewpoint, at: Duration) -> bool {
        let rested = self
            .wheel
            .is_none_or(|last| at.saturating_sub(last) >= WHEEL_PAUSE);
        self.wheel = Some(at);
        if rested {
            self.push(view);
        }
        rested
    }

    /// Önceki görünüm: the last record, with `current` (the view now) put on the
    /// “next” stack. None when there is nothing to go back to. A record that
    /// is the view already standing is skipped, so that the command always moves.
    pub fn back(&mut self, current: Viewpoint) -> Option<Viewpoint> {
        self.wheel = None;
        Self::step(&mut self.back, &mut self.next, current)
    }

    /// Sonraki görünüm: what [`ViewHistory::back`] left, with `current` put
    /// back on the records.
    pub fn forward(&mut self, current: Viewpoint) -> Option<Viewpoint> {
        self.wheel = None;
        Self::step(&mut self.next, &mut self.back, current)
    }

    fn step(
        from: &mut Vec<Viewpoint>,
        to: &mut Vec<Viewpoint>,
        current: Viewpoint,
    ) -> Option<Viewpoint> {
        while let Some(view) = from.pop() {
            if view == current {
                continue;
            }
            to.push(current);
            if to.len() > LIMIT {
                to.remove(0);
            }
            return Some(view);
        }
        None
    }

    /// Empties both stacks and the wheel's pass: another drawing was opened.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Whether Önceki görünüm has a view to go to.
    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    /// Whether Sonraki görünüm has a view to go to.
    pub fn can_forward(&self) -> bool {
        !self.next.is_empty()
    }

    /// How many views are recorded, for tests and the picture of the history.
    pub fn len(&self) -> usize {
        self.back.len()
    }

    pub fn is_empty(&self) -> bool {
        self.back.is_empty() && self.next.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The n-th view: distinct in the middle and the zoom.
    fn view(n: u32) -> Viewpoint {
        Viewpoint {
            center: Vec2::new(f64::from(n) * 10.0, f64::from(n) * -3.0),
            scale: 1.0 + f64::from(n) / 8.0,
        }
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_new_history_has_nowhere_to_go() {
        let mut h = ViewHistory::new();
        assert!(!h.can_back() && !h.can_forward() && h.is_empty());
        assert_eq!(h.back(view(0)), None);
        assert_eq!(h.forward(view(0)), None);
    }

    /// The ADR's case: 31 navigations keep 30 views, 30 steps back reach the
    /// second view (the first was dropped), and 30 forward come home.
    #[test]
    fn thirty_one_navigations_go_back_thirty_steps_and_forward_again() {
        let mut h = ViewHistory::new();
        // The user stands at view(k) and leaves it: navigation k records view(k).
        for k in 0..31 {
            h.record(view(k));
        }
        assert_eq!(h.len(), LIMIT);
        let mut here = view(31);
        for k in (1..31).rev() {
            let went = h.back(here).expect("a view to go back to");
            assert_eq!(went, view(k), "step back to view {k}");
            here = went;
        }
        assert_eq!(here, view(1), "view 0 was dropped");
        assert!(!h.can_back());
        assert_eq!(h.back(here), None);
        assert!(h.can_forward());
        for k in 2..=31 {
            let went = h.forward(here).expect("a view to go forward to");
            assert_eq!(went, view(k), "step forward to view {k}");
            here = went;
        }
        assert_eq!(here, view(31));
        assert!(!h.can_forward());
        assert_eq!(h.forward(here), None);
        assert!(h.can_back(), "and back again");
    }

    #[test]
    fn a_new_navigation_deletes_the_views_ahead() {
        let mut h = ViewHistory::new();
        for k in 0..3 {
            h.record(view(k));
        }
        let mut here = view(3);
        here = h.back(here).expect("2");
        here = h.back(here).expect("1");
        assert_eq!(here, view(1));
        assert!(h.can_forward());
        // From view 1 the user navigates: view 1 is recorded, nothing is ahead.
        h.record(here);
        assert!(!h.can_forward());
        assert_eq!(h.back(view(9)), Some(view(1)));
        assert_eq!(h.back(view(1)), Some(view(0)));
    }

    #[test]
    fn the_view_last_recorded_is_not_recorded_again() {
        let mut h = ViewHistory::new();
        h.record(view(1));
        h.record(view(1));
        h.record(view(2));
        h.record(view(1));
        assert_eq!(h.len(), 3, "1, 2, 1: only the same one in a row is dropped");
        // Even so a navigation empties what is ahead.
        let mut h = ViewHistory::new();
        h.record(view(1));
        assert_eq!(h.back(view(2)), Some(view(1)));
        assert!(h.can_forward());
        h.record(view(1));
        assert!(!h.can_forward());
    }

    #[test]
    fn going_back_skips_a_record_that_is_where_the_view_stands() {
        let mut h = ViewHistory::new();
        h.record(view(1));
        h.record(view(2));
        // The user came back to view 2 by hand: the record of view 2 is no step.
        assert_eq!(h.back(view(2)), Some(view(1)));
        assert_eq!(h.back(view(1)), None, "nothing else is left");
        // The skipped record is gone; the view left is ahead.
        assert_eq!(h.forward(view(1)), Some(view(2)));
    }

    #[test]
    fn the_wheel_is_one_pass_until_it_rests() {
        let mut h = ViewHistory::new();
        // A pass of steps 100 ms apart: only the first records.
        assert!(h.record_wheel(view(0), ms(1_000)));
        for (i, t) in [1_100, 1_200, 1_300, 1_400, 1_500].into_iter().enumerate() {
            assert!(!h.record_wheel(view(1 + i as u32), ms(t)), "step at {t}");
        }
        assert_eq!(h.len(), 1);
        // Just under the pause since the last step: still the same pass.
        assert!(!h.record_wheel(view(7), ms(1_999)));
        // 500 ms of rest starts another.
        assert!(h.record_wheel(view(8), ms(2_499)));
        assert_eq!(h.len(), 2);
        // The pass is counted from the last step, not from the first.
        assert!(!h.record_wheel(view(9), ms(2_900)));
        assert!(h.record_wheel(view(10), ms(3_400)));
        assert_eq!(h.back(view(11)), Some(view(10)));
        assert_eq!(h.back(view(10)), Some(view(8)));
        assert_eq!(h.back(view(8)), Some(view(0)));
    }

    /// A command or a step back or forward ends the wheel's pass: the wheel's
    /// next step, however soon, records the view it leaves.
    #[test]
    fn a_command_ends_the_wheels_pass() {
        let mut h = ViewHistory::new();
        assert!(h.record_wheel(view(0), ms(1_000)));
        assert!(!h.record_wheel(view(1), ms(1_100)));
        h.record(view(2));
        assert!(h.record_wheel(view(3), ms(1_200)), "after a command");
        assert!(!h.record_wheel(view(4), ms(1_300)));
        assert_eq!(h.back(view(5)), Some(view(3)));
        assert!(h.record_wheel(view(3), ms(1_400)), "after going back");
    }

    #[test]
    fn opening_another_drawing_empties_it_all() {
        let mut h = ViewHistory::new();
        h.record(view(1));
        h.record(view(2));
        let _ = h.back(view(3));
        assert!(h.can_back() && h.can_forward());
        assert!(h.record_wheel(view(4), ms(50)));
        h.clear();
        assert!(!h.can_back() && !h.can_forward() && h.is_empty());
        // The wheel's pass is forgotten too: its next step records at once.
        assert!(h.record_wheel(view(5), ms(60)));
    }

    /// Ahead is bounded like the records: a long walk back and forth keeps the limit.
    #[test]
    fn neither_stack_grows_past_the_limit() {
        let mut h = ViewHistory::new();
        for k in 0..100 {
            h.record(view(k));
        }
        let mut here = view(100);
        for _ in 0..LIMIT {
            here = h.back(here).expect("a view");
        }
        assert_eq!(h.len(), 0);
        assert!(h.can_forward());
        for _ in 0..LIMIT {
            here = h.forward(here).expect("a view");
        }
        assert_eq!(h.forward(here), None);
        assert_eq!(h.len(), LIMIT, "the views walked over are records again");
    }
}

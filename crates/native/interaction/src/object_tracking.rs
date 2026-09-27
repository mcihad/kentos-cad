//! Nesne izleme, object snap tracking (the web's `ViewportController`
//! tracking and `viewport/objectTracking.ts`): resting the cursor on an
//! object snap for a moment acquires that point; alignment lines leave the
//! acquired points (horizontal and vertical, and every polar step when polar
//! tracking is on), and with no object snap under it the cursor locks onto
//! the nearest line, or onto the crossing of two lines from different
//! points. Resting on an acquired point again releases it; at most three
//! are kept, the oldest going first; they belong to one command.
//!
//! The lines and the crossing are the shared core's
//! (`kentos_geometry_core::tools::object_tracking`); this keeps the state
//! the web keeps in its viewport. The waiting itself is the host's: it
//! calls [`ObjectTracking::dwell_due`] once the dwell has passed with the
//! number [`ObjectTracking::dwell`] gave.

use kentos_geometry_core::store::snap::{SnapHit, SnapKind};
pub use kentos_geometry_core::tools::object_tracking::{TrackHit, TrackLine};
use kentos_geometry_core::tools::object_tracking::{along_track, track_angles, track_point};

use crate::Vec2;

/// Resting on a snap this long acquires (or releases) it, milliseconds (the web's `TRACK_DWELL_MS`).
pub const DWELL_MS: u64 = 350;
/// Points kept at most (the web's `MAX_TRACK_POINTS`).
pub const MAX_POINTS: usize = 3;
/// How close the cursor must come to an alignment line to lock onto it,
/// logical pixels (the web's `TRACK_PX`).
pub const TRACK_PX: f64 = 8.0;

/// The snaps that make sense as tracking points (the web's `TRACKABLE`).
fn trackable(kind: SnapKind) -> bool {
    matches!(
        kind,
        SnapKind::Endpoint
            | SnapKind::Midpoint
            | SnapKind::Center
            | SnapKind::Node
            | SnapKind::Quadrant
            | SnapKind::Intersection
    )
}

/// The snap point being rested on; `done` once it toggled, so resting
/// longer does not toggle it back.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Dwell {
    p: Vec2,
    number: u64,
    done: bool,
}

/// The tracking state of the drawing area.
#[derive(Clone, Debug, Default)]
pub struct ObjectTracking {
    acquired: Vec<Vec2>,
    track: Option<TrackHit>,
    dwell: Option<Dwell>,
    /// Counts the dwells begun, so a late wait is told from the current one.
    dwells: u64,
}

impl ObjectTracking {
    pub fn new() -> Self {
        Self::default()
    }

    /// After the object snap was taken for the cursor at `raw`: the rest on
    /// a trackable snap begins (or goes on), and with no snap the cursor's
    /// lock is found. `on`: tracking is on and the running command snaps;
    /// `from`: the command's last point (it takes part in crossings only);
    /// `polar`: polar tracking's step when it is on; `tol`: the world length
    /// of [`TRACK_PX`].
    pub fn update(
        &mut self,
        on: bool,
        snap: Option<&SnapHit>,
        raw: Vec2,
        from: Option<Vec2>,
        polar: Option<f64>,
        tol: f64,
    ) {
        if !on {
            self.track = None;
            self.dwell = None;
            return;
        }
        match snap.filter(|s| trackable(s.kind)) {
            Some(s) => self.rest_on(s.point),
            None => self.dwell = None,
        }
        self.track = match snap {
            Some(_) => None,
            None => track_point(raw, &self.acquired, from, &track_angles(polar), tol),
        };
    }

    fn rest_on(&mut self, p: Vec2) {
        if self.dwell.is_some_and(|d| d.p == p) {
            return;
        }
        self.dwells += 1;
        self.dwell = Some(Dwell {
            p,
            number: self.dwells,
            done: false,
        });
    }

    /// The rest waiting for its dwell, by its number: the host calls
    /// [`ObjectTracking::dwell_due`] with it once the dwell has passed.
    pub fn dwell(&self) -> Option<u64> {
        self.dwell.filter(|d| !d.done).map(|d| d.number)
    }

    /// The dwell has passed: if the cursor still rests where it did, that
    /// point is acquired, or released when it was. Returns whether it toggled.
    pub fn dwell_due(&mut self, number: u64) -> bool {
        let Some(dwell) = self
            .dwell
            .as_mut()
            .filter(|d| d.number == number && !d.done)
        else {
            return false;
        };
        dwell.done = true;
        let p = dwell.p;
        match self.acquired.iter().position(|q| *q == p) {
            Some(i) => {
                self.acquired.remove(i);
            }
            None => {
                self.acquired.push(p);
                if self.acquired.len() > MAX_POINTS {
                    self.acquired.remove(0);
                }
            }
        }
        true
    }

    /// Another command: its points go (they belong to one command).
    pub fn clear(&mut self) {
        self.acquired.clear();
        self.track = None;
        self.dwell = None;
    }

    /// The acquired points, oldest first.
    pub fn acquired(&self) -> &[Vec2] {
        &self.acquired
    }

    /// The alignment the cursor is locked to now, if any.
    pub fn track(&self) -> Option<&TrackHit> {
        self.track.as_ref()
    }

    /// The point `distance` along the lock's single line from its origin:
    /// a distance typed while the cursor is on an alignment (the web's `trackAlong`).
    pub fn along(&self, distance: f64) -> Option<Vec2> {
        along_track(self.track.as_ref()?, distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(kind: SnapKind, x: f64, y: f64) -> SnapHit {
        SnapHit {
            kind,
            point: Vec2::new(x, y),
            id: 1.0,
        }
    }

    fn acquire(t: &mut ObjectTracking, x: f64, y: f64) {
        t.update(
            true,
            Some(&snap(SnapKind::Endpoint, x, y)),
            Vec2::new(x, y),
            None,
            None,
            1.0,
        );
        let number = t.dwell().expect("a rest waits");
        assert!(t.dwell_due(number));
    }

    #[test]
    fn a_rest_acquires_a_second_releases_and_three_are_kept() {
        let mut t = ObjectTracking::new();
        acquire(&mut t, 0.0, 0.0);
        assert_eq!(t.acquired(), &[Vec2::new(0.0, 0.0)]);
        // Resting on and on does not toggle it back.
        t.update(
            true,
            Some(&snap(SnapKind::Endpoint, 0.0, 0.0)),
            Vec2::new(0.0, 0.0),
            None,
            None,
            1.0,
        );
        assert_eq!(t.dwell(), None);
        // Level with it, the cursor locks onto its horizontal.
        t.update(true, None, Vec2::new(10.0, 0.3), None, None, 1.0);
        let hit = t.track().expect("locked");
        assert_eq!(hit.point, Vec2::new(10.0, 0.0));
        assert_eq!(t.along(4.0), Some(Vec2::new(4.0, 0.0)));
        // Away from it, and back: it goes.
        acquire(&mut t, 0.0, 0.0);
        assert!(t.acquired().is_empty());
        for (x, y) in [(1.0, 1.0), (2.0, 2.0), (3.0, 3.0), (4.0, 4.0)] {
            t.update(true, None, Vec2::new(50.0, 50.0), None, None, 1.0);
            acquire(&mut t, x, y);
        }
        assert_eq!(
            t.acquired(),
            &[
                Vec2::new(2.0, 2.0),
                Vec2::new(3.0, 3.0),
                Vec2::new(4.0, 4.0)
            ],
            "the oldest goes first"
        );
    }

    #[test]
    fn a_late_wait_a_nearest_snap_or_tracking_off_acquires_nothing() {
        let mut t = ObjectTracking::new();
        t.update(
            true,
            Some(&snap(SnapKind::Endpoint, 0.0, 0.0)),
            Vec2::new(0.0, 0.0),
            None,
            None,
            1.0,
        );
        let first = t.dwell().expect("waits");
        t.update(
            true,
            Some(&snap(SnapKind::Endpoint, 5.0, 0.0)),
            Vec2::new(5.0, 0.0),
            None,
            None,
            1.0,
        );
        assert!(!t.dwell_due(first), "the cursor moved on");
        t.update(
            true,
            Some(&snap(SnapKind::Nearest, 7.0, 0.0)),
            Vec2::new(7.0, 0.0),
            None,
            None,
            1.0,
        );
        assert_eq!(t.dwell(), None, "a nearest point is no tracking point");
        t.update(
            false,
            Some(&snap(SnapKind::Endpoint, 0.0, 0.0)),
            Vec2::new(0.0, 0.0),
            None,
            None,
            1.0,
        );
        assert_eq!(t.dwell(), None);
        assert!(t.acquired().is_empty());
    }
}

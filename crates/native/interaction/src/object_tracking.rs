//! Nesne izleme, object snap tracking (the web's `ViewportController`
//! tracking and `viewport/objectTracking.ts`): resting the cursor on an
//! object snap for a moment acquires that point; alignment lines leave the
//! acquired points (horizontal and vertical, and every polar step when polar
//! tracking is on), and with no object snap under it the cursor locks onto
//! the nearest line, or onto the crossing of two lines from different
//! points. Resting on an acquired point again releases it; at most three
//! are kept, the oldest going first; they belong to one command.
//!
//! The same rests acquire for the snap additions (docs/adr/0163 §2), in the
//! same list: an end rested on with Uzantı on brings the extensions of its
//! edges, a straight edge rested on with Paralel on gives its direction;
//! each works with tracking off too. A typed distance goes along the
//! extension or the parallel snapped to.
//!
//! The lines and the crossing are the shared core's
//! (`kentos_geometry_core::tools::object_tracking`); this keeps the state
//! the web keeps in its viewport. The waiting itself is the host's: it
//! calls [`ObjectTracking::dwell_due`] once the dwell has passed with the
//! number [`ObjectTracking::dwell`] gave.

use kentos_geometry_core::store::snap::{Extension, SnapHit, SnapKind};
pub use kentos_geometry_core::tools::object_tracking::{TrackHit, TrackLine};
use kentos_geometry_core::tools::object_tracking::{along_track, track_angles, track_point};

use crate::Vec2;

/// Resting on a snap this long acquires (or releases) it, milliseconds (the web's `TRACK_DWELL_MS`).
pub const DWELL_MS: u64 = 350;
/// Acquisitions kept at most (the web's `MAX_TRACK_POINTS`).
pub const MAX_POINTS: usize = 3;
/// How close the cursor must come to an alignment line to lock onto it,
/// logical pixels (the web's `TRACK_PX`).
pub const TRACK_PX: f64 = 8.0;
/// How near a point lies to a parallel line through the last point to be on it, metres.
const ON: f64 = 1e-6;

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

/// What rests acquire (docs/adr/0085, 0163 §2): each aid on its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Aids {
    /// Nesne izleme: a snap rested on becomes a tracking point.
    pub tracking: bool,
    /// Uzantı: an end rested on brings the extensions of its edges.
    pub extension: bool,
    /// Paralel: a straight edge rested on gives its direction.
    pub parallel: bool,
}

impl Aids {
    fn any(self) -> bool {
        self.tracking || self.extension || self.parallel
    }
}

/// An acquisition, oldest first in the list (docs/adr/0085, 0163 §2).
#[derive(Clone, Debug, PartialEq)]
pub enum Acquired {
    /// A point rested on: a tracking point, and an end's extensions when Uzantı was on.
    Point {
        at: Vec2,
        extensions: Vec<Extension>,
    },
    /// A straight edge rested on (Paralel): where, and its direction, unit.
    Edge { at: Vec2, dir: Vec2 },
}

/// What the cursor rests on.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Rest {
    /// A snap point; `track` when it is a tracking point, `end` the object
    /// whose end it is when Uzantı takes its extensions.
    Point {
        p: Vec2,
        track: bool,
        end: Option<f64>,
    },
    /// A straight edge, by its direction; `at` where the rest began.
    Edge { at: Vec2, dir: Vec2 },
}

impl Rest {
    /// The same rest goes on: the same point, or an edge of the same direction.
    fn same(&self, other: &Rest) -> bool {
        match (self, other) {
            (Rest::Point { p, .. }, Rest::Point { p: q, .. }) => p == q,
            (Rest::Edge { dir, .. }, Rest::Edge { dir: d, .. }) => dir == d,
            _ => false,
        }
    }
}

/// The rest being waited on; `done` once it toggled, so resting longer does
/// not toggle it back.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Dwell {
    rest: Rest,
    number: u64,
    done: bool,
}

/// The tracking state of the drawing area.
#[derive(Clone, Debug, Default)]
pub struct ObjectTracking {
    acquired: Vec<Acquired>,
    track: Option<TrackHit>,
    dwell: Option<Dwell>,
    /// Counts the dwells begun, so a late wait is told from the current one.
    dwells: u64,
    /// The snap and the command's last point as of the last update: a typed
    /// distance goes along the extension or the parallel snapped to.
    snap: Option<SnapHit>,
    from: Option<Vec2>,
}

impl ObjectTracking {
    pub fn new() -> Self {
        Self::default()
    }

    /// After the object snap was taken for the cursor at `raw`: a rest on
    /// what the aids take begins (or goes on), and with no snap the cursor's
    /// lock is found. `aids`: what is on, for a command that snaps (all off
    /// otherwise); `from`: the command's last point (it takes part in
    /// crossings only); `polar`: polar tracking's step when it is on; `tol`:
    /// the world length of [`TRACK_PX`]; `edge`: the direction of the
    /// straight edge under the cursor (Paralel's rest), when the host found
    /// one with no point snapped there.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        aids: Aids,
        snap: Option<&SnapHit>,
        raw: Vec2,
        from: Option<Vec2>,
        polar: Option<f64>,
        tol: f64,
        edge: Option<Vec2>,
    ) {
        self.snap = snap.copied();
        self.from = from;
        if !aids.any() {
            self.track = None;
            self.dwell = None;
            return;
        }
        let rest = match snap {
            Some(s) => {
                let track = aids.tracking && trackable(s.kind);
                // An end of the drawing's own objects (the path being drawn has none to give).
                let end =
                    (aids.extension && s.kind == SnapKind::Endpoint && s.id >= 0.0).then_some(s.id);
                let edge = (s.kind == SnapKind::Nearest).then_some(edge).flatten();
                if track || end.is_some() {
                    Some(Rest::Point {
                        p: s.point,
                        track,
                        end,
                    })
                } else {
                    edge.filter(|_| aids.parallel)
                        .map(|dir| Rest::Edge { at: raw, dir })
                }
            }
            None => edge
                .filter(|_| aids.parallel)
                .map(|dir| Rest::Edge { at: raw, dir }),
        };
        match rest {
            Some(rest) => self.rest_on(rest),
            None => self.dwell = None,
        }
        self.track = match snap {
            None if aids.tracking => {
                track_point(raw, &self.points(), from, &track_angles(polar), tol)
            }
            _ => None,
        };
    }

    fn rest_on(&mut self, rest: Rest) {
        if self.dwell.is_some_and(|d| d.rest.same(&rest)) {
            return;
        }
        self.dwells += 1;
        self.dwell = Some(Dwell {
            rest,
            number: self.dwells,
            done: false,
        });
    }

    /// The rest waiting for its dwell, by its number: the host calls
    /// [`ObjectTracking::dwell_due`] with it once the dwell has passed.
    pub fn dwell(&self) -> Option<u64> {
        self.dwell.filter(|d| !d.done).map(|d| d.number)
    }

    /// The dwell has passed: if the cursor still rests where it did, that is
    /// acquired, or released when it was. `ends` gives the extensions of an
    /// object's edges ending at a point (`Spatial::extensions_at`). Returns
    /// whether it toggled.
    pub fn dwell_due(
        &mut self,
        number: u64,
        ends: impl FnOnce(f64, Vec2) -> Vec<Extension>,
    ) -> bool {
        let Some(dwell) = self
            .dwell
            .as_mut()
            .filter(|d| d.number == number && !d.done)
        else {
            return false;
        };
        dwell.done = true;
        match dwell.rest {
            Rest::Point { p, track, end } => {
                let held = self
                    .acquired
                    .iter()
                    .position(|a| matches!(a, Acquired::Point { at, .. } if *at == p));
                match held {
                    Some(i) => {
                        self.acquired.remove(i);
                    }
                    None => {
                        let extensions = end.map(|id| ends(id, p)).unwrap_or_default();
                        if !track && extensions.is_empty() {
                            return false;
                        }
                        self.push(Acquired::Point { at: p, extensions });
                    }
                }
            }
            Rest::Edge { at, dir } => {
                let held = self
                    .acquired
                    .iter()
                    .position(|a| matches!(a, Acquired::Edge { dir: d, .. } if *d == dir));
                match held {
                    Some(i) => {
                        self.acquired.remove(i);
                    }
                    None => self.push(Acquired::Edge { at, dir }),
                }
            }
        }
        true
    }

    fn push(&mut self, a: Acquired) {
        self.acquired.push(a);
        if self.acquired.len() > MAX_POINTS {
            self.acquired.remove(0);
        }
    }

    /// Another command: its acquisitions go (they belong to one command).
    pub fn clear(&mut self) {
        self.acquired.clear();
        self.track = None;
        self.dwell = None;
    }

    /// The acquisitions, oldest first.
    pub fn acquired(&self) -> &[Acquired] {
        &self.acquired
    }

    /// The acquired points (tracking points and ends), oldest first.
    pub fn points(&self) -> Vec<Vec2> {
        self.acquired
            .iter()
            .filter_map(|a| match a {
                Acquired::Point { at, .. } => Some(*at),
                Acquired::Edge { .. } => None,
            })
            .collect()
    }

    /// The acquired ends' extensions, for the snap (Uzantı).
    pub fn extensions(&self) -> Vec<Extension> {
        self.acquired
            .iter()
            .flat_map(|a| match a {
                Acquired::Point { extensions, .. } => extensions.clone(),
                Acquired::Edge { .. } => Vec::new(),
            })
            .collect()
    }

    /// The acquired edges' directions, for the snap (Paralel).
    pub fn parallels(&self) -> Vec<Vec2> {
        self.acquired
            .iter()
            .filter_map(|a| match a {
                Acquired::Edge { dir, .. } => Some(*dir),
                Acquired::Point { .. } => None,
            })
            .collect()
    }

    /// The acquired extensions `p` lies on, each with how far along from
    /// its end (a snap's tag: “Uzantı 12.063 m”, or a crossing of two).
    pub fn extensions_through(&self, p: Vec2) -> Vec<(Extension, f64)> {
        self.extensions()
            .into_iter()
            .filter_map(|x| x.along(p).map(|d| (x, d)))
            .collect()
    }

    /// The acquired direction whose line through `from` holds `p`.
    pub fn parallel_through(&self, p: Vec2, from: Vec2) -> Option<Vec2> {
        self.parallels()
            .into_iter()
            .find(|u| ((p.x - from.x) * u.y - (p.y - from.y) * u.x).abs() <= ON)
    }

    /// The alignment the cursor is locked to now, if any.
    pub fn track(&self) -> Option<&TrackHit> {
        self.track.as_ref()
    }

    /// The point `distance` along what the cursor is on: the lock's single
    /// line from its origin (the web's `trackAlong`), or the one extension
    /// snapped to from its end, or the parallel snapped to from the last
    /// point toward the cursor (docs/adr/0163 §2).
    pub fn along(&self, distance: f64) -> Option<Vec2> {
        if let Some(t) = &self.track {
            return along_track(t, distance);
        }
        let s = self.snap?;
        match s.kind {
            SnapKind::Extension => match self.extensions_through(s.point).as_slice() {
                [(x, _)] => x.at(distance),
                _ => None,
            },
            SnapKind::Parallel => {
                let from = self.from?;
                let u = self.parallel_through(s.point, from)?;
                let side = if (s.point.x - from.x) * u.x + (s.point.y - from.y) * u.y < 0.0 {
                    -1.0
                } else {
                    1.0
                };
                Some(Vec2::new(
                    from.x + u.x * side * distance,
                    from.y + u.y * side * distance,
                ))
            }
            _ => None,
        }
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

    const TRACKING: Aids = Aids {
        tracking: true,
        extension: false,
        parallel: false,
    };

    fn acquire(t: &mut ObjectTracking, x: f64, y: f64) {
        t.update(
            TRACKING,
            Some(&snap(SnapKind::Endpoint, x, y)),
            Vec2::new(x, y),
            None,
            None,
            1.0,
            None,
        );
        let number = t.dwell().expect("a rest waits");
        assert!(t.dwell_due(number, |_, _| Vec::new()));
    }

    #[test]
    fn a_rest_acquires_a_second_releases_and_three_are_kept() {
        let mut t = ObjectTracking::new();
        acquire(&mut t, 0.0, 0.0);
        assert_eq!(t.points(), &[Vec2::new(0.0, 0.0)]);
        // Resting on and on does not toggle it back.
        t.update(
            TRACKING,
            Some(&snap(SnapKind::Endpoint, 0.0, 0.0)),
            Vec2::new(0.0, 0.0),
            None,
            None,
            1.0,
            None,
        );
        assert_eq!(t.dwell(), None);
        // Level with it, the cursor locks onto its horizontal.
        t.update(TRACKING, None, Vec2::new(10.0, 0.3), None, None, 1.0, None);
        let hit = t.track().expect("locked");
        assert_eq!(hit.point, Vec2::new(10.0, 0.0));
        assert_eq!(t.along(4.0), Some(Vec2::new(4.0, 0.0)));
        // Away from it, and back: it goes.
        acquire(&mut t, 0.0, 0.0);
        assert!(t.points().is_empty());
        for (x, y) in [(1.0, 1.0), (2.0, 2.0), (3.0, 3.0), (4.0, 4.0)] {
            t.update(TRACKING, None, Vec2::new(50.0, 50.0), None, None, 1.0, None);
            acquire(&mut t, x, y);
        }
        assert_eq!(
            t.points(),
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
            TRACKING,
            Some(&snap(SnapKind::Endpoint, 0.0, 0.0)),
            Vec2::new(0.0, 0.0),
            None,
            None,
            1.0,
            None,
        );
        let first = t.dwell().expect("waits");
        t.update(
            TRACKING,
            Some(&snap(SnapKind::Endpoint, 5.0, 0.0)),
            Vec2::new(5.0, 0.0),
            None,
            None,
            1.0,
            None,
        );
        assert!(
            !t.dwell_due(first, |_, _| Vec::new()),
            "the cursor moved on"
        );
        t.update(
            TRACKING,
            Some(&snap(SnapKind::Nearest, 7.0, 0.0)),
            Vec2::new(7.0, 0.0),
            None,
            None,
            1.0,
            None,
        );
        assert_eq!(t.dwell(), None, "a nearest point is no tracking point");
        t.update(
            Aids::default(),
            Some(&snap(SnapKind::Endpoint, 0.0, 0.0)),
            Vec2::new(0.0, 0.0),
            None,
            None,
            1.0,
            None,
        );
        assert_eq!(t.dwell(), None);
        assert!(t.points().is_empty());
    }

    /// Uzantı and Paralel acquire with tracking off (docs/adr/0163 §2): an
    /// end of an object brings its extensions, an edge under the cursor its
    /// direction; the same edge again releases it; a typed distance goes
    /// along the extension or the parallel snapped to.
    #[test]
    fn extensions_and_parallels_are_acquired_and_typed_along() {
        use kentos_geometry_core::store::snap::NO_OBJECT;
        let aids = Aids {
            tracking: false,
            extension: true,
            parallel: true,
        };
        let mut t = ObjectTracking::new();
        let ext = Extension::Line {
            end: Vec2::new(10.0, 0.0),
            dir: Vec2::new(1.0, 0.0),
        };
        let end = snap(SnapKind::Endpoint, 10.0, 0.0);
        t.update(
            aids,
            Some(&end),
            Vec2::new(10.0, 0.1),
            None,
            None,
            1.0,
            None,
        );
        let n = t.dwell().expect("a rest on the end waits");
        assert!(t.dwell_due(n, |id, at| {
            assert_eq!((id, at), (1.0, Vec2::new(10.0, 0.0)));
            vec![ext]
        }));
        assert_eq!(t.extensions(), [ext]);
        assert!(t.track().is_none(), "no alignments with tracking off");
        // A middle snap is no end: nothing to rest on with tracking off.
        t.update(
            aids,
            Some(&snap(SnapKind::Midpoint, 5.0, 0.0)),
            Vec2::new(5.0, 0.1),
            None,
            None,
            1.0,
            None,
        );
        assert_eq!(t.dwell(), None);
        // An edge with no point under the cursor: its direction; moving along it is the same rest.
        let u = Vec2::new(0.6, 0.8);
        t.update(aids, None, Vec2::new(3.0, 4.1), None, None, 1.0, Some(u));
        let n = t.dwell().expect("a rest on the edge waits");
        t.update(aids, None, Vec2::new(6.0, 8.1), None, None, 1.0, Some(u));
        assert_eq!(t.dwell(), Some(n), "the same edge goes on resting");
        assert!(t.dwell_due(n, |_, _| Vec::new()));
        assert_eq!(t.parallels(), [u]);
        // Typed along the extension snapped to, from its end.
        let on = SnapHit {
            kind: SnapKind::Extension,
            point: Vec2::new(15.0, 0.0),
            id: NO_OBJECT,
        };
        t.update(aids, Some(&on), Vec2::new(15.0, 0.2), None, None, 1.0, None);
        assert_eq!(t.along(6.0), Some(Vec2::new(16.0, 0.0)));
        // Typed along the parallel from the last point, toward the cursor's side.
        let from = Vec2::new(0.0, 10.0);
        let par = SnapHit {
            kind: SnapKind::Parallel,
            point: Vec2::new(-3.0, 6.0),
            id: NO_OBJECT,
        };
        t.update(
            aids,
            Some(&par),
            Vec2::new(-3.1, 6.0),
            Some(from),
            None,
            1.0,
            None,
        );
        assert_eq!(t.along(10.0), Some(Vec2::new(-6.0, 2.0)));
        // The same edge rested on again releases its direction.
        t.update(aids, None, Vec2::new(9.0, 12.1), None, None, 1.0, Some(u));
        let n = t.dwell().expect("a new rest");
        assert!(t.dwell_due(n, |_, _| Vec::new()));
        assert!(t.parallels().is_empty());
        // An end whose object gives no extension, tracking off: nothing is kept.
        t.update(
            aids,
            Some(&snap(SnapKind::Endpoint, 0.0, 0.0)),
            Vec2::new(0.0, 0.0),
            None,
            None,
            1.0,
            None,
        );
        let n = t.dwell().expect("a rest");
        assert!(!t.dwell_due(n, |_, _| Vec::new()));
        assert_eq!(t.points(), [Vec2::new(10.0, 0.0)]);
    }
}

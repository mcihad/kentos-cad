//! Tab through the snap candidates (docs/adr/0271): where the pointer stands, Tab takes the next snap point the
//! core offers there, Shift+Tab the one before, best first and round again; moving the pointer lets the cycle
//! go. The marker says the object's kind and, while cycling, which candidate of how many.

use iced::Point;
use kentos_interaction::SnapHit;

use crate::app::App;
use crate::input::CameraView;

/// How far the pointer may move, logical pixels, before the cycle goes: a hand on the mouse trembles.
const STILL: f32 = 0.75;

/// The candidates where the pointer stood when Tab was pressed, and which one is taken.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapCycle {
    pub at: Point,
    pub candidates: Vec<SnapHit>,
    pub pick: usize,
}

impl App {
    /// Tab (`back`: Shift+Tab) over the drawing while a point is taken: the next snap candidate. False when
    /// there is none to step to (the key goes on as before).
    pub(crate) fn cycle_snap(&mut self, back: bool) -> bool {
        let Some(world) = self.viewport.cursor else {
            return false;
        };
        let [x, y] = self.viewport.camera.world_to_screen(world);
        let at = Point::new(x as f32, y as f32);
        let fresh = !self
            .snap_cycle
            .as_ref()
            .is_some_and(|c| c.at.distance(at) <= STILL);
        if fresh {
            let Some(doc) = &self.document else {
                return false;
            };
            self.spatial.sync(&doc.model);
            let view = CameraView(&self.viewport.camera);
            let candidates = self.session.snap_candidates(
                &self.spatial,
                world,
                &view,
                &self.snap_draft(),
                &self.tracking,
            );
            if candidates.len() < 2 {
                return false;
            }
            self.snap_cycle = Some(SnapCycle {
                at,
                candidates,
                pick: 0,
            });
        }
        let Some(cycle) = self.snap_cycle.as_mut() else {
            return false;
        };
        let n = cycle.candidates.len();
        cycle.pick = if back {
            (cycle.pick + n - 1) % n
        } else {
            (cycle.pick + 1) % n
        };
        self.repoint();
        true
    }

    /// The snap the cycle holds for a pointer at `at`; the cycle goes once the pointer moved away.
    pub(crate) fn cycled_snap(&mut self, at: Point) -> Option<SnapHit> {
        let cycle = self.snap_cycle.as_ref()?;
        if cycle.at.distance(at) > STILL {
            self.snap_cycle = None;
            return None;
        }
        cycle.candidates.get(cycle.pick).copied()
    }

    /// What the snap marker says after its kind: the object's kind, and while cycling “2/5”.
    pub(crate) fn snap_marker_note(&self) -> Option<String> {
        let hit = self.snap?;
        let object = (hit.id >= 0.0)
            .then(|| {
                let doc = self.document.as_ref()?;
                let slot = kentos_domain::Slot(u32::try_from(hit.id as i64).ok()?);
                doc.model
                    .get(slot)
                    .map(|e| crate::view::kind_name(e.kind()))
            })
            .flatten();
        let step = self
            .snap_cycle
            .as_ref()
            .filter(|c| c.candidates.get(c.pick) == Some(&hit))
            .map(|c| format!("{}/{} (Tab)", c.pick + 1, c.candidates.len()));
        match (object, step) {
            (Some(o), Some(s)) => Some(format!("{o} · {s}")),
            (Some(o), None) => Some(o.to_owned()),
            (None, s) => s,
        }
    }
}

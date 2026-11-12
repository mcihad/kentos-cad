//! Nesne izleme on the desktop (docs/adr/0085): the web's object snap
//! tracking over the tool session's state
//! (`kentos_interaction::object_tracking`). What the app adds:
//!
//! - the dwell: resting on a snap starts a 350 ms wait on a thread of its
//!   own ([`crate::hover_card::after`]); if the pointer still rests there
//!   when it ends, the point is acquired, or released;
//! - the points belong to one command: another command clears them;
//! - the marks: a cross on every acquired point, the dashed alignment the
//!   cursor is locked to and “İzleme 12.500 m < 0°” (or “İzleme: kesişim”)
//!   beside it, in the snap colour (marks.rs);
//! - the snap additions' (docs/adr/0163 §2): two short strokes on an
//!   acquired edge, the dashed extension from its end to a snap on it (the
//!   marker says “Uzantı 12.063 m”, or “Uzantı: kesişim” where two cross),
//!   the parallel's dashed line through the last point.
//!
//! Shift+F3 (`draft.tracking`, `drafting.tracking`) turns it off and on;
//! the points are kept meanwhile, as on the web.

use std::time::Duration;

use iced::Task;
use kentos_geometry_core::jsmath::{cos, js_sign, sin};
use kentos_geometry_core::store::snap::Extension;
use kentos_interaction::object_tracking::{Acquired, DWELL_MS};
use kentos_interaction::{Format, SnapKind, Vec2, dist};

use crate::app::{App, Message};
use crate::marks::{Guide, TrackingMarks};

impl App {
    /// After every message: another command clears the points, and a new
    /// rest on a snap starts its wait.
    pub(crate) fn follow_tracking(&mut self) -> Task<Message> {
        let tool = self.session.tool_id();
        if tool != self.tracking_tool {
            self.tracking_tool = tool;
            self.tracking.clear();
        }
        match self.tracking.dwell() {
            Some(number) if number != self.tracking_waited && self.dwell_on_time => {
                self.tracking_waited = number;
                crate::hover_card::after(
                    Duration::from_millis(DWELL_MS),
                    Message::TrackDwell(number),
                )
            }
            _ => Task::none(),
        }
    }

    /// What the marks draw of object tracking and the snap additions'
    /// acquisitions; none while nothing is acquired.
    pub(crate) fn tracking_marks(&self, format: &Format) -> Option<TrackingMarks> {
        let acquired = self.tracking.acquired();
        if acquired.is_empty() {
            return None;
        }
        let edges = acquired
            .iter()
            .filter_map(|a| match a {
                Acquired::Edge { at, dir } => Some((*at, *dir)),
                Acquired::Point { .. } => None,
            })
            .collect();
        let (paths, snap_label) = self.snap_guides(format);
        // A snap wins over the lock: then only the points show.
        let track = self
            .tracking
            .track()
            .filter(|_| self.snap.is_none())
            .cloned();
        let label = track.as_ref().map(|t| match t.lines.as_slice() {
            // The angle as JavaScript writes the number: 0, 90, 22.5.
            [line] => format!(
                "İzleme {} < {}°",
                format.length(dist(line.origin, t.point)),
                line.angle + 0.0
            ),
            _ => "İzleme: kesişim".to_owned(),
        });
        Some(TrackingMarks {
            acquired: self.tracking.points(),
            edges,
            track,
            label,
            paths,
            snap_label,
        })
    }

    /// The extensions or the parallel the snap lies on (docs/adr/0163 §2),
    /// and what its marker says then: “Uzantı 12.063 m”, “Uzantı: kesişim”.
    fn snap_guides(&self, format: &Format) -> (Vec<Guide>, Option<String>) {
        let Some(hit) = self.snap else {
            return (Vec::new(), None);
        };
        match hit.kind {
            SnapKind::Extension => {
                let on = self.tracking.extensions_through(hit.point);
                let label = match on.as_slice() {
                    [] => None,
                    [(_, d)] => Some(format!("Uzantı {}", format.length(*d))),
                    _ => Some("Uzantı: kesişim".to_owned()),
                };
                (on.iter().map(|(x, d)| guide(x, *d)).collect(), label)
            }
            SnapKind::Parallel => {
                let guide = self.session.snap_from().and_then(|from| {
                    self.tracking
                        .parallel_through(hit.point, from)
                        .map(|dir| Guide::Line { through: from, dir })
                });
                (guide.into_iter().collect(), None)
            }
            _ => (Vec::new(), None),
        }
    }
}

/// An extension from its end to `d` along it: the segment, or the arc's
/// points at most 5° apart.
fn guide(x: &Extension, d: f64) -> Guide {
    match *x {
        Extension::Line { end, dir } => {
            Guide::Path(vec![end, Vec2::new(end.x + dir.x * d, end.y + dir.y * d)])
        }
        Extension::Arc { c, r, a0, sweep } => {
            let turn = d / r;
            let steps = (turn / 5f64.to_radians()).ceil().max(1.0) as usize;
            Guide::Path(
                (0..=steps)
                    .map(|i| {
                        let a = a0 + js_sign(sweep) * turn * i as f64 / steps as f64;
                        Vec2::new(c.x + r * cos(a), c.y + r * sin(a))
                    })
                    .collect(),
            )
        }
    }
}

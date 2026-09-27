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
//!   beside it, in the snap colour (marks.rs).
//!
//! Shift+F3 (`draft.tracking`, `drafting.tracking`) turns it off and on;
//! the points are kept meanwhile, as on the web.

use std::time::Duration;

use iced::Task;
use kentos_interaction::object_tracking::DWELL_MS;
use kentos_interaction::{Format, dist};

use crate::app::{App, Message};
use crate::marks::TrackingMarks;

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

    /// What the marks draw of object tracking; none without acquired points.
    pub(crate) fn tracking_marks(&self, format: &Format) -> Option<TrackingMarks> {
        let acquired = self.tracking.acquired();
        if acquired.is_empty() {
            return None;
        }
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
            acquired: acquired.to_vec(),
            track,
            label,
        })
    }
}

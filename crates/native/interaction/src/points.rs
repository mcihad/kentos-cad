//! What the point-driven tools share: the web's `PointInputTool` base
//! (`apps/web/src/tools/drawTools.ts`) without its DOM side. The effective
//! cursor (ortho, polar tracking), the echo of a taken point, and how a
//! product command's answer reaches the user.

use kentos_contracts::CommandResult;
use kentos_geometry_core::tools::point_input::{Tracking, constrain_cursor};
use kentos_native_application::codes;

use crate::Vec2;
use crate::log::Level;
use crate::tool::{Context, Pointer};

/// How near the cursor must be to a polar ray to lock onto it, logical pixels (web `CAPTURE_PX`).
const CAPTURE_PX: f64 = 10.0;
/// Two points closer than this are the same point.
pub(crate) const SAME: f64 = 1e-9;

/// The effective cursor for the next point after `from`: ortho (Shift turns
/// it over) and polar tracking, by the shared core; with the tracking ray it
/// locked onto. The pointer's own point when there is no point yet, and an
/// object snap's point exactly: ortho and polar never move it (the web's
/// `constrainPoint`, docs/adr/0029).
pub(crate) fn constrain(
    from: Option<Vec2>,
    p: &Pointer,
    cx: &Context<'_>,
) -> (Vec2, Option<Tracking>) {
    let Some(from) = from else {
        return (p.world, None);
    };
    let c = constrain_cursor(
        Some(from),
        p.world,
        p.snap.is_some(),
        cx.draft.ortho != p.shift,
        cx.draft.polar,
        cx.view.world_length(CAPTURE_PX),
    );
    (c.point, c.tracking)
}

/// A taken point, said as the web says it: `  Y 487012.000  X 4420000.000`.
pub(crate) fn echo(p: Vec2, cx: &mut Context<'_>) {
    let line = format!("  {}", cx.format().point(p));
    cx.say(Level::Info, line);
}

/// A product command's answer, as the tool reports it (docs/adr/0022,
/// 0027): the warnings of a completed write, then its output; a refusal's
/// message and `None`. A document out of slots is not the user's to fix
/// here, so it is an error; the rest are warnings, word for word the
/// command's (the locked and hidden layer texts are the tools' own).
pub(crate) fn written<T>(result: CommandResult<T>, cx: &mut Context<'_>) -> Option<T> {
    match result {
        CommandResult::Completed { output, warnings } => {
            for warning in warnings {
                cx.say(Level::Warn, warning.message);
            }
            Some(output)
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => {
            let level = if error.code == codes::SLOTS_EXHAUSTED {
                Level::Error
            } else {
                Level::Warn
            };
            cx.say(level, error.message);
            None
        }
        CommandResult::Queued { .. } | CommandResult::Cancelled => None,
    }
}

/// `Vec2` as the contracts carry it.
pub(crate) fn wire(p: Vec2) -> kentos_contracts::Vec2 {
    kentos_contracts::Vec2 { x: p.x, y: p.y }
}

/// A closed shape a tool built (a rectangle, a regular polygon) written
/// through the product command `cad.polygon.create` (docs/adr/0032): the
/// active layer explicit in its input (CMD-07); the desktop has no current
/// colour. Whether it was written; the object is noted on the draft for Ctrl+Z.
pub(crate) fn write_ring(
    d: &mut Taken,
    pts: &[Vec2],
    bulges: Option<Vec<f64>>,
    cx: &mut Context<'_>,
) -> bool {
    use kentos_native_application::{ExecutionContext, polygon};
    let input = kentos_contracts::PolygonCreate {
        layer_id: cx.doc.layers().active().to_owned(),
        pts: pts.iter().map(|p| wire(*p)).collect(),
        bulges,
        holes: None,
        color: None,
        attrs: None,
        expected_revision: None,
    };
    let result = polygon::execute(&mut ExecutionContext::new(cx.doc), input);
    match written(result, cx) {
        Some(output) => {
            d.note(output.id, cx);
            true
        }
        None => false,
    }
}

/// The web's `PointInputTool.draw`: the points so far and the cursor as one
/// solid line, the length and bearing of its last segment beside the cursor,
/// and the polar ray the cursor is locked to.
pub(crate) fn chain_preview(d: &Taken, format: &crate::format::Format) -> crate::tool::Preview {
    use kentos_geometry_core::geometry::{bearing_grad, dist};
    let path: Vec<Vec2> = d.pts.iter().copied().chain(d.hover).collect();
    let tag = match (d.last(), d.hover) {
        (Some(last), Some(hover)) => Some(crate::tool::Tag {
            at: hover,
            lines: vec![
                format.length(dist(last, hover)),
                format!("Semt {}", format.bearing(bearing_grad(last, hover))),
            ],
        }),
        _ => None,
    };
    crate::tool::Preview {
        path,
        tracking: tag.as_ref().and(d.tracking),
        tag,
        ..crate::tool::Preview::default()
    }
}

/// Typed text that is a number and nothing else: no separator, relative or
/// polar mark (the web's `!/[,;@<]/.test(text)` beside `parseNumber`).
pub(crate) fn plain_number(text: &str) -> Option<f64> {
    if text.contains([',', ';', '@', '<']) {
        return None;
    }
    kentos_geometry_core::tools::point_text::parse_number(text)
}

/// What every point-input tool keeps (the web's `PointInputTool` fields,
/// docs/adr/0032): its points, the effective cursor, and the objects written
/// for the object being drawn, so Ctrl+Z can take the newest back as an
/// undo while the drawing has not changed since.
#[derive(Clone, Debug, Default)]
pub(crate) struct Taken {
    pub pts: Vec<Vec2>,
    /// The effective cursor from the last pointer move.
    pub hover: Option<Vec2>,
    pub tracking: Option<Tracking>,
    made: Vec<kentos_domain::Slot>,
    made_at: Option<u64>,
}

impl Taken {
    pub fn last(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    /// The effective cursor for the pointer, from the last point (ortho,
    /// polar tracking; a snapped point exact): the web's `constrain`.
    pub fn constrain(&mut self, p: &Pointer, cx: &Context<'_>) -> Vec2 {
        let (point, tracking) = constrain(self.last(), p, cx);
        self.tracking = tracking;
        point
    }

    /// The web's `accept` before the tool's `onPoint`: the point is echoed,
    /// and a first point starts a new object (what was written before is the
    /// drawing's to undo).
    pub fn begin(&mut self, p: Vec2, cx: &mut Context<'_>) {
        echo(p, cx);
        if self.pts.is_empty() {
            self.made.clear();
        }
    }

    /// An object written for the object being drawn (the web's `noteMade`).
    pub fn note(&mut self, slot: u32, cx: &Context<'_>) {
        self.made.push(kentos_domain::Slot(slot));
        self.made_at = Some(cx.doc.revision());
    }

    /// Takes back the newest object written for the object being drawn, as
    /// an undo, when the drawing has not changed since (the web's `undoLastMade`).
    pub fn undo_last_made(&mut self, cx: &mut Context<'_>) -> bool {
        if self.made.is_empty() || self.made_at != Some(cx.doc.revision()) {
            return false;
        }
        self.made.pop();
        cx.doc.undo();
        self.made_at = Some(cx.doc.revision());
        true
    }

    /// Drops the draft's points and what it wrote (the web's `reset`).
    pub fn reset(&mut self) {
        self.pts.clear();
        self.made.clear();
    }

    /// Ctrl+Z for a tool without Geri (G) (the web's `undoStep`): the object
    /// written for the draft (as an undo), else the draft's last point when
    /// its step follows from its points alone, else the draft starts over.
    /// False when there was nothing: the drawing is undone then.
    pub fn undo_step(&mut self, steps_from_points: bool, cx: &mut Context<'_>) -> bool {
        if self.undo_last_made(cx) {
            return true;
        }
        if self.pts.is_empty() {
            return false;
        }
        if steps_from_points {
            self.pts.pop();
        } else {
            self.reset();
        }
        true
    }
}

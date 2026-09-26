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

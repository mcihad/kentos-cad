//! The pictures of docs/adr/0284 (Komut geçmişi's calls): a circle drawn with Daire and a call refused, the circle's
//! Python shown under it; and the same history with the circle undone. `tools_screens` takes them in both themes at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=gecmis-python,gecmis-geri-alindi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::advanced::widget::operation;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::history::Event;
use crate::history::tests::{circle_drawn, circle_refused, prepare};
use crate::tools_screens::Pointed;

/// The calls and the circle's Python open; the panel tall enough for them.
fn python(app: &mut App) {
    prepare(app);
    app.command_expanded = true;
    app.bottom_log = Some(360.0);
    circle_drawn(app);
    circle_refused(app);
    let id = app.history.entries[0].id;
    let _ = app.update(Message::History(Event::Toggle(id)));
}

/// The same, the circle undone: its call faint, “geri alındı”.
fn undone(app: &mut App) {
    python(app);
    let _ = app.update(Message::History(Event::Toggle(app.history.entries[0].id)));
    let _ = app.update(Message::Run("edit.undo"));
}

/// Komut geçmişi at its end.
fn to_end(s: &mut Snapshot, app: &mut App) {
    s.operate(
        app.view(),
        Box::new(operation::scrollable::snap_to(
            crate::message_log::LIST.into(),
            iced::widget::scrollable::RelativeOffset::END.into(),
        )),
    );
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        ("gecmis-python", python, to_end),
        ("gecmis-geri-alindi", undone, to_end),
    ]
}

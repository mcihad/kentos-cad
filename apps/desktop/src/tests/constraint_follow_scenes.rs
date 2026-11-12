//! The pictures of docs/adr/0337 (kısıtlar düzenlemelerde): the room of `constraint_scenes` made a rectangle by its
//! constraints (a held corner, its height 8, no width). Taşı on its right edge: the edge moved alone in the preview;
//! written, the rectangle widens and stays a rectangle, its top and bottom following. The top edge moved up: the
//! held corner and the height contradict it, nothing written, the solver's word below. Gevşeterek düzenle on the
//! Kısıtlar panel, then the same move: written, the two corners it breaks taken away and said. `tools_screens` takes
//! them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kisit-izle-onizleme,kisit-izle,kisit-izle-celiski,kisit-gevset cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::Entity;
use kentos_domain::Slot;

use crate::app::{App, Message};
use crate::constraint_scenes::{drawn, rectangle};
use crate::tools_scenes::{E, N, click, hover, run};
use crate::tools_screens::Scene;

/// The middle of a line of the drawing as the scenes place points (from the drawing's corner).
fn middle(app: &App, slot: u32) -> [f64; 2] {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let Some(Entity::Line(l)) = doc.get(Slot(slot)) else {
        panic!("a line")
    };
    [(l.a.x + l.b.x) / 2.0 - E, (l.a.y + l.b.y) / 2.0 - N]
}

fn by(p: [f64; 2], dx: f64, dy: f64) -> [f64; 2] {
    [p[0] + dx, p[1] + dy]
}

/// The right edge picked, Taşı from its middle, the pointer four metres to the right.
fn preview(app: &mut App) {
    drawn(app);
    rectangle(app);
    let right = middle(app, 2);
    click(app, right);
    run(app, "tool.move");
    click(app, right);
    hover(app, by(right, 4.0, 0.0));
}

/// Written: the rectangle widens, its top and bottom follow the edge.
fn followed(app: &mut App) {
    preview(app);
    let right = middle(app, 2);
    click(app, by(right, 4.0, 0.0));
    hover(app, [24.0, 10.0]);
}

/// The top edge moved up: the held corner and the height refuse it.
fn refused(app: &mut App) {
    drawn(app);
    rectangle(app);
    let top = middle(app, 3);
    click(app, top);
    run(app, "tool.move");
    click(app, top);
    click(app, by(top, 0.0, 2.0));
    hover(app, [24.0, 10.0]);
}

/// Gevşeterek düzenle, then the same move: written, the corners it breaks said.
fn relaxed(app: &mut App) {
    drawn(app);
    rectangle(app);
    let _ = app.update(Message::RibbonTab("modify"));
    let _ = app.update(Message::Run("constraint.relaxNext"));
    let top = middle(app, 3);
    click(app, top);
    run(app, "tool.move");
    click(app, top);
    click(app, by(top, 0.0, 2.0));
    hover(app, [24.0, 10.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kisit-izle-onizleme", preview),
        ("kisit-izle", followed),
        ("kisit-izle-celiski", refused),
        ("kisit-gevset", relaxed),
    ]
}

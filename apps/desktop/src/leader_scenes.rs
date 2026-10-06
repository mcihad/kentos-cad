//! The pictures of docs/adr/0146 §7, Kılavuz at work over a parcel with a
//! building and a road: the tip on the building's corner, a vertex, the
//! pointer where the landing starts (the arrowhead, the line, the landing
//! and the note's place); the note's field open past the landing, the note
//! typed; Ok's menu from the command line's chip; the leaders written (a
//! filled arrow with its note, an open one with a masked note going left,
//! a dot); the first's note edited in place after a double click. The web's
//! are `shots.mjs leaders` (`leader-tool`, `leader-note`, `leader-arrows`,
//! `leader-written`, `leader-edit`). `tools_screens` takes them in the dark
//! and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kilavuz-arac cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::keyboard::key::Named;
use kentos_ui::snapshot::Input;

use crate::app::{App, Message};
use crate::files_testing::find_text;
use crate::text_field::Event as FieldEvent;
use crate::tools_scenes::{Objects, click, forget, hover, open, run, typed};
use crate::tools_screens::{Pointed, Scene, press_caption};

/// A parcel, a building in it and a road below: what the leaders point at.
fn ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]],
        true,
    );
    o.path(
        "cizim",
        &[[8.0, 6.0], [22.0, 6.0], [22.0, 18.0], [8.0, 18.0]],
        true,
    );
    o.line("yol", [-6.0, -4.0], [60.0, -4.0]);
    o
}

/// Kılavuz running: the tip on the building's corner, a vertex up and to the
/// right, the pointer where the landing will start.
fn tool(app: &mut App) {
    open(app, ground());
    run(app, "tool.leader");
    click(app, [22.0, 18.0]);
    click(app, [28.0, 24.0]);
    forget(app);
    hover(app, [36.0, 25.0]);
}

/// The last vertex given and Enter: the note's field past the landing, the note typed.
fn note(app: &mut App) {
    tool(app);
    click(app, [36.0, 25.0]);
    typed(app, "");
    let _ = app.update(Message::TextField(FieldEvent::Input("Mevcut bina".into())));
}

/// The note kept, then two leaders more: an open arrow from the road going
/// left with a masked note, and a dot on the building's other corner without one.
fn written(app: &mut App) {
    note(app);
    let _ = app.update(Message::TextField(FieldEvent::Keep));
    let _ = app.update(Message::PromptChoice("O", "açık".into()));
    typed(app, "Z");
    click(app, [50.0, -4.0]);
    click(app, [46.0, 4.0]);
    click(app, [38.0, 6.0]);
    typed(app, "");
    let _ = app.update(Message::TextField(FieldEvent::Input("Ø150 PVC".into())));
    let _ = app.update(Message::TextField(FieldEvent::Keep));
    let _ = app.update(Message::PromptChoice("O", "nokta".into()));
    typed(app, "Z");
    click(app, [8.0, 18.0]);
    click(app, [3.0, 23.0]);
    typed(app, "");
    let _ = app.update(Message::TextField(FieldEvent::Keep));
    forget(app);
    hover(app, [52.0, 12.0]);
}

/// Kılavuz left, a double click on the first leader's note: its field over
/// it, the note's words changed (the building is to be pulled down).
fn edit(app: &mut App) {
    written(app);
    run(app, "tool.cancel");
    run(app, "tool.cancel");
    click(app, [46.0, 25.0]);
    click(app, [46.0, 25.0]);
    let _ = app.update(Message::TextField(FieldEvent::Input("Yıkılacak bina".into())));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kilavuz-arac", tool),
        ("kilavuz-not", note),
        ("kilavuz-sonuc", written),
        ("kilavuz-duzenle", edit),
    ]
}

/// Ok's menu from the command line's chip (in a narrow window, Diğer's menu
/// with Ok's open over it).
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![("kilavuz-ok-menusu", tool, |s, app| {
        if find_text(s, app, "Ok: dolu").is_some() {
            press_caption(s, app, "Ok: dolu", false, false);
            return;
        }
        press_caption(s, app, "Diğer", false, false);
        let mut update = |app: &mut App, message: Message| {
            let _ = app.update(message);
        };
        s.settle(app, App::view, &mut update);
        // Ok, Diğer's first row, opened from the keyboard as a user may: ↓ to it, → into its menu.
        for key in [Named::ArrowDown, Named::ArrowRight] {
            s.input(app, App::view, &mut update, Input::Key(key));
        }
    })]
}

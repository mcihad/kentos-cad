//! The drawing area's visual conformance on CAD's own tools (plan/render.md RM-16, docs/adr/9224; the check is
//! render_testing.rs): a line drawn, erased, moved and undone, a circle, a text, and a line drawn in the deep view;
//! each picture right after the edit, nothing moved, against an app that opens the edited drawing afresh. Mimari's
//! walls, doors and windows are checked in its package (kentos-desktop-arch, render_conformance_tests.rs).
//!
//! ```text
//! KENTOS_GPU_TESTS=1 cargo test -p kentos-desktop --lib render_conformance -- --nocapture --test-threads=1
//! ```

use kentos_domain::Slot;

use crate::app::{App, Message};
use crate::text_field::Event as TextField;
use crate::render_testing::{Edit, all_show_at_once, nothing, on_gpu};
use crate::tools_scenes::{E, N, Objects, click, run, typed};
use crate::wall_scenes::{fit, opened};

/// Off the drawing, below and right of it.
const AWAY: [f64; 2] = [9.0, -1.0];

fn empty(app: &mut App) {
    opened(app, Objects::new(), 50.0);
}

/// One line on Çizim (slot 1), from (1, 1) to (6, 4).
fn a_line(app: &mut App) {
    let mut o = Objects::new();
    o.line("cizim", [1.0, 1.0], [6.0, 4.0]);
    opened(app, o, 50.0);
}

fn framed(app: &mut App) {
    fit(app, 7.0, 5.0);
}

/// Close, in the deep view (500 px/m and more, docs/adr/9220): 1 200 px a metre round (2, 2).
fn close(app: &mut App) {
    app.viewport.camera.center.x = E + 2.0;
    app.viewport.camera.center.y = N + 2.0;
    app.viewport.camera.scale = 1_200.0;
}

fn line_tool(app: &mut App, a: [f64; 2], b: [f64; 2]) {
    run(app, "tool.line");
    click(app, a);
    click(app, b);
    typed(app, "");
}

#[test]
fn an_edit_with_cad_s_tools_shows_at_once_as_a_fresh_app_draws_it() {
    if !on_gpu() {
        eprintln!("render_conformance: KENTOS_GPU_TESTS=1 is not set; the GPU cases did not run");
        return;
    }
    all_show_at_once(&[
        Edit {
            name: "cizgi",
            start: &empty,
            view: &framed,
            drawn: &nothing,
            edit: &|app| line_tool(app, [1.0, 1.0], [6.0, 4.0]),
            away: AWAY,
        },
        Edit {
            name: "cizgi-silindi",
            start: &a_line,
            view: &framed,
            drawn: &nothing,
            edit: &|app| {
                app.selection.set([Slot(1)]);
                run(app, "tool.erase");
            },
            away: AWAY,
        },
        Edit {
            name: "cizgi-tasindi",
            start: &a_line,
            view: &framed,
            drawn: &nothing,
            edit: &|app| {
                app.selection.set([Slot(1)]);
                run(app, "tool.move");
                click(app, [1.0, 1.0]);
                click(app, [1.0, 0.0]);
            },
            away: AWAY,
        },
        Edit {
            name: "cizgi-geri-alindi",
            start: &empty,
            view: &framed,
            drawn: &|app| {
                line_tool(app, [1.0, 1.0], [6.0, 4.0]);
                line_tool(app, [1.0, 4.0], [6.0, 1.0]);
            },
            edit: &|app| run(app, "edit.undo"),
            away: AWAY,
        },
        Edit {
            name: "daire",
            start: &empty,
            view: &framed,
            drawn: &nothing,
            edit: &|app| {
                run(app, "tool.circle");
                click(app, [3.5, 2.5]);
                click(app, [5.5, 2.5]);
            },
            away: AWAY,
        },
        Edit {
            name: "yazi",
            start: &a_line,
            view: &framed,
            drawn: &nothing,
            edit: &|app| {
                // Yazı asks for the text in the field over the drawing where it starts (text_field.rs).
                run(app, "tool.text");
                click(app, [1.0, 2.5]);
                let _ = app.update(Message::TextField(TextField::Input("Salon".into())));
                let _ = app.update(Message::TextField(TextField::Keep));
            },
            away: AWAY,
        },
        Edit {
            name: "cizgi-derin",
            start: &a_line,
            view: &close,
            drawn: &nothing,
            edit: &|app| line_tool(app, [1.5, 2.2], [2.5, 1.8]),
            away: [2.0, 1.75],
        },
    ]);
}

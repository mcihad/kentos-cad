//! The drawing area's visual conformance on Mimari's walls, doors and windows (plan/render.md RM-16, docs/adr/9224;
//! the check is the core's render_testing.rs). The owner, 24 November 2026: a window or a door put on a wall did not
//! show until the view was zoomed or moved (docs/adr/9221). Each picture right after the edit, nothing moved, against
//! an app that opens the edited drawing afresh with the same camera.
//!
//! ```text
//! KENTOS_GPU_TESTS=1 cargo test -p kentos-desktop-arch render_conformance -- --nocapture --test-threads=1
//! ```

use kentos_commands::Entity;
use kentos_domain::Slot;

use crate::app::{App, Message};
use crate::render_testing::{Edit, all_show_at_once, nothing, on_gpu};
use crate::tools_scenes::{E, N, Objects, click, run, typed};
use crate::wall_scenes::{fit, opened};

/// Off the drawing, below and right of the room.
const AWAY: [f64; 2] = [9.0, -1.0];

/// An empty drawing with KentOS Mimari's layer key on (docs/adr/8008): walls, doors and windows each go on their
/// kind's layer (MM-DUVA-…, MM-KAPI-GENEL, MM-PENC-…), as in the owner's Mimari project where the bug showed. On one
/// layer a door's own change would build the wall's part again too.
fn empty(app: &mut App) {
    opened(app, Objects::new(), 50.0);
    let _ = app.update(Message::LayerKeys(true));
}

/// Duvar with a wall style chosen by name.
fn wall_tool(app: &mut App, style: &str) {
    app.command_expanded = true;
    run(app, "tool.wall");
    typed(app, "S");
    typed(app, style);
}

/// The room of wall_scenes, drawn with Duvar: 7 × 5 m of Dış duvar 25 closed with K and an inner wall of İç duvar 15
/// at 4.2 m.
fn the_room(app: &mut App) {
    wall_tool(app, "Dış duvar 25");
    for p in [[0.0, 0.0], [7.0, 0.0], [7.0, 5.0], [0.0, 5.0]] {
        click(app, p);
    }
    typed(app, "K");
    wall_tool(app, "İç duvar 15");
    click(app, [4.2, 0.0]);
    click(app, [4.2, 5.0]);
    typed(app, "");
    typed(app, "");
}

/// The room with a door on its lower wall at 1.6 m.
fn the_room_with_a_door(app: &mut App) {
    the_room(app);
    run(app, "tool.door");
    click(app, [1.6, 0.0]);
    typed(app, "");
    typed(app, "");
}

/// The slot of the first object `which` picks.
fn slot_where(app: &App, which: impl Fn(&Entity) -> bool) -> Slot {
    let model = &app.document.as_ref().expect("the drawing is open").model;
    model
        .entities_with_uids()
        .find(|(_, e)| which(e))
        .and_then(|(uid, _)| model.slot_of(uid))
        .expect("the object is in the drawing")
}

/// The room's door.
fn the_door(app: &App) -> Slot {
    slot_where(app, |e| matches!(e, Entity::Opening(_)))
}

/// The room's inner wall, at 4.2 m.
fn the_inner_wall(app: &App) -> Slot {
    slot_where(app, |e| match e {
        Entity::Wall(w) => w.wall.pts.iter().all(|p| (p.x - E - 4.2).abs() < 1e-6),
        _ => false,
    })
}

fn framed(app: &mut App) {
    fit(app, 7.0, 5.0);
}

/// Close, in the deep view (500 px/m and more, docs/adr/9220): 1 200 px a metre round the lower wall at 2 m.
fn close(app: &mut App) {
    app.viewport.camera.center.x = E + 2.0;
    app.viewport.camera.center.y = N;
    app.viewport.camera.scale = 1_200.0;
}

/// The room's middle on screen at 1:`denominator`.
fn at_scale(app: &mut App, denominator: f64) {
    app.viewport.camera.center.x = E + 3.5;
    app.viewport.camera.center.y = N + 2.5;
    app.viewport.camera.scale = kentos_geometry_core::scale::px_per_metre(denominator);
}

fn window_at(app: &mut App, p: [f64; 2]) {
    run(app, "tool.window");
    click(app, p);
    typed(app, "");
}

#[test]
fn an_edit_of_walls_doors_and_windows_shows_at_once_as_a_fresh_app_draws_it() {
    if !on_gpu() {
        eprintln!("render_conformance: KENTOS_GPU_TESTS=1 is not set; the GPU cases did not run");
        return;
    }
    all_show_at_once(&[
        Edit {
            name: "duvar",
            start: &empty,
            view: &framed,
            drawn: &nothing,
            edit: &|app| {
                wall_tool(app, "Dış duvar 25");
                for p in [[0.0, 0.0], [7.0, 0.0], [7.0, 5.0]] {
                    click(app, p);
                }
                typed(app, "");
            },
            away: AWAY,
        },
        Edit {
            name: "pencere",
            start: &empty,
            view: &framed,
            drawn: &the_room,
            edit: &|app| window_at(app, [2.0, 0.0]),
            away: AWAY,
        },
        Edit {
            name: "kapi",
            start: &empty,
            view: &framed,
            drawn: &the_room,
            edit: &|app| {
                run(app, "tool.door");
                click(app, [5.6, 5.0]);
                typed(app, "");
            },
            away: AWAY,
        },
        Edit {
            name: "kapi-silindi",
            start: &empty,
            view: &framed,
            drawn: &the_room_with_a_door,
            edit: &|app| {
                let door = the_door(app);
                app.selection.set([door]);
                run(app, "tool.erase");
            },
            away: AWAY,
        },
        Edit {
            name: "duvar-tasindi",
            start: &empty,
            view: &framed,
            drawn: &the_room,
            edit: &|app| {
                // The inner wall a metre to the right: the outer wall it meets is joined again.
                let wall = the_inner_wall(app);
                app.selection.set([wall]);
                run(app, "tool.move");
                click(app, [4.2, 2.5]);
                click(app, [5.2, 2.5]);
            },
            away: AWAY,
        },
        Edit {
            name: "kapi-geri-alindi",
            start: &empty,
            view: &framed,
            drawn: &the_room_with_a_door,
            edit: &|app| run(app, "edit.undo"),
            away: AWAY,
        },
        Edit {
            name: "pencere-derin",
            start: &empty,
            view: &framed,
            // The room drawn with the whole of it in view, then the camera close.
            drawn: &|app| {
                the_room(app);
                close(app);
            },
            edit: &|app| window_at(app, [2.0, 0.0]),
            away: [2.0, -0.3],
        },
        // The walls' detail follows the zoom (docs/adr/0339 §4): schematic at 1:120, detailed at 1:20. A door and a
        // window on layers of their own take the new detail with their walls (docs/adr/9224; they kept the old one).
        Edit {
            name: "ayrinti-yakinlasma",
            start: &empty,
            view: &framed,
            drawn: &|app| {
                the_room_with_a_door(app);
                window_at(app, [5.6, 5.0]);
                at_scale(app, 120.0);
            },
            edit: &|app| at_scale(app, 20.0),
            away: AWAY,
        },
        Edit {
            name: "ayrinti-uzaklasma",
            start: &empty,
            view: &framed,
            drawn: &|app| {
                the_room_with_a_door(app);
                window_at(app, [5.6, 5.0]);
                at_scale(app, 20.0);
            },
            edit: &|app| at_scale(app, 120.0),
            away: AWAY,
        },
    ]);
}

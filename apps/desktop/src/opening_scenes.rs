//! The pictures of docs/adr/0341 (openings): the joined room of docs/adr/0340 with two doors and a double door in its
//! bottom wall, two windows in its top wall, a sliding door in its inner wall and a bare opening in its left wall, all
//! put with the Kapı, Pencere and Boşluk tools; CAD's Mimari tab with Açıklık; the room at 1:50 and a door near at
//! 1:20; Kapı under the cursor (the command's plan); Pencere with Uzaklık; a door's Öznitelikler; Açıklığı taşı with
//! the door picked; the inner wall deleted, its sliding door left with its warning. `tools_screens` takes them in both
//! themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=acik-serit,acik-oda,acik-ayrintili,acik-kapi,acik-pencere,acik-oznitelikler,acik-tasi,acik-duvar-yok cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;

use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::joined_room;

/// The joined room with its openings: slots 6 (Kapı 90), 7 (İki kanatlı kapı 140), 8 and 9 (Pencere 120), 10 (Sürme
/// kapı 160), 11 (Boşluk 100).
fn room(app: &mut App, scale: f64) {
    joined_room(app, scale);
    app.command_expanded = true;
    run(app, "tool.door");
    click(app, [1.6, 0.4]);
    typed(app, "T");
    typed(app, "İki kanatlı kapı 140");
    click(app, [5.6, 0.4]);
    typed(app, "");
    run(app, "tool.window");
    click(app, [1.8, 4.6]);
    click(app, [5.6, 4.6]);
    typed(app, "");
    run(app, "tool.door");
    typed(app, "T");
    typed(app, "Sürme kapı 160");
    click(app, [4.4, 2.6]);
    typed(app, "");
    run(app, "tool.opening");
    click(app, [0.4, 2.5]);
    typed(app, "");
    hover(app, [9.0, -1.0]);
}

fn ribbon(app: &mut App) {
    room(app, 50.0);
    app.tab = "arch";
}

fn written(app: &mut App) {
    room(app, 50.0);
}

/// At 1:20, the bottom left door near: its frame at each jamb, its leaf and swing, the wall's plaster cut at the jambs.
fn detailed(app: &mut App) {
    room(app, 20.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + 0.6,
            min_y: N - 0.6,
            max_x: E + 2.8,
            max_y: N + 1.4,
        },
        24.0,
    );
    hover(app, [2.7, -0.5]);
}

/// Kapı with the cursor inside the room by its bottom wall: the door the command would put there.
fn door_tool(app: &mut App) {
    joined_room(app, 50.0);
    app.command_expanded = true;
    run(app, "tool.door");
    hover(app, [2.2, 0.5]);
}

/// Pencere with its near jamb 0,6 m from the top wall's near corner.
fn window_tool(app: &mut App) {
    joined_room(app, 50.0);
    app.command_expanded = true;
    run(app, "tool.window");
    typed(app, "U");
    typed(app, "0.6");
    hover(app, [1.2, 4.6]);
}

/// The bottom left door selected: Öznitelikler's opening rows.
fn props(app: &mut App) {
    room(app, 50.0);
    app.selection.set([Slot(6)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Açıklığı taşı: the bottom left door picked, the cursor where it would go.
fn mover(app: &mut App) {
    room(app, 50.0);
    run(app, "tool.openingMove");
    click(app, [1.6, 0.1]);
    hover(app, [3.0, 0.4]);
}

/// The inner wall deleted: its sliding door stays where it was, said with a warning triangle.
fn host_gone(app: &mut App) {
    room(app, 50.0);
    run(app, "tool.erase");
    click(app, [4.2, 1.0]);
    typed(app, "");
    hover(app, [9.0, -1.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("acik-serit", ribbon),
        ("acik-oda", written),
        ("acik-ayrintili", detailed),
        ("acik-kapi", door_tool),
        ("acik-pencere", window_tool),
        ("acik-oznitelikler", props),
        ("acik-tasi", mover),
        ("acik-duvar-yok", host_gone),
    ]
}

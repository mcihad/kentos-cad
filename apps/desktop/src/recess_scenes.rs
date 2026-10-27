//! The pictures of docs/adr/0719 (niches, flues, service holes): the joined room of docs/adr/0340 with a window in its
//! top wall and a radiator niche under it, a niche in its right wall, a flue in its bottom wall and a service hole high
//! in its inner wall, all put with the Pencere, Niş, Baca and Boşluk tools; the room at 1:50; the flue and the niche
//! near at 1:20; the window and its radiator niche near; Niş under the cursor with its tag; a niche too deep for its
//! wall, refused beside the cursor; a niche's and a flue's Öznitelikler; Mimari's Boşluk ▾. `tools_screens` takes them
//! in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=nis-oda,nis-ayrintili,nis-pencere-alti,nis-arac,nis-derin,nis-oznitelikler,baca-oznitelikler,nis-serit cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, click, hover, run, typed};
use crate::tools_screens::{Pointed, Scene, press_caption};
use crate::wall_scenes::joined_room;

/// The joined room with slots 6 (Pencere 120, top wall), 7 (Kalorifer nişi 100 under it), 8 (Niş 60, right wall), 9
/// (Baca 20, bottom wall) and 10 (Tesisat boşluğu 20, inner wall).
pub(crate) fn room(app: &mut App, scale: f64) {
    joined_room(app, scale);
    app.command_expanded = true;
    run(app, "tool.window");
    click(app, [2.0, 4.6]);
    typed(app, "");
    run(app, "tool.niche");
    typed(app, "T");
    typed(app, "Kalorifer nişi 100");
    click(app, [2.0, 4.6]);
    typed(app, "T");
    typed(app, "Niş 60");
    click(app, [6.6, 2.5]);
    typed(app, "");
    run(app, "tool.flue");
    click(app, [5.6, 0.4]);
    typed(app, "");
    run(app, "tool.opening");
    typed(app, "T");
    typed(app, "Tesisat boşluğu 20");
    click(app, [4.4, 1.2]);
    typed(app, "");
    hover(app, [9.0, -1.0]);
}

fn written(app: &mut App) {
    room(app, 50.0);
}

fn near(app: &mut App, min: [f64; 2], max: [f64; 2]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + min[0],
            min_y: N + min[1],
            max_x: E + max[0],
            max_y: N + max[1],
        },
        24.0,
    );
}

/// At 1:20, the bottom right corner: the flue's cavity crossed inside the outer wall, the niche cut from the right
/// wall's inner face with its back.
fn detailed(app: &mut App) {
    room(app, 20.0);
    near(app, [4.9, -0.5], [7.5, 3.1]);
    hover(app, [7.4, 3.0]);
}

/// At 1:20, the window in the top wall and its radiator niche under it, below the plan's cut: dashed.
fn under_window(app: &mut App) {
    room(app, 20.0);
    near(app, [0.9, 3.9], [3.1, 5.6]);
    hover(app, [3.0, 3.95]);
}

/// Niş with the cursor inside the room by its right wall: the niche the command would put there and its tag.
fn niche_tool(app: &mut App) {
    room(app, 50.0);
    run(app, "tool.niche");
    hover(app, [6.6, 3.8]);
}

/// Niş 0,30 m deep by the inner wall (0,15 m): refused beside the cursor, nothing written.
fn too_deep(app: &mut App) {
    room(app, 50.0);
    run(app, "tool.niche");
    typed(app, "D");
    typed(app, "0.30");
    hover(app, [4.4, 3.4]);
}

fn props(app: &mut App, slot: u32) {
    room(app, 50.0);
    app.selection.set([Slot(slot)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// The right wall's niche selected: Öznitelikler's Yüz, Derinlik, Alt kot, Üst kot and Planda.
fn niche_props(app: &mut App) {
    props(app, 8);
}

/// The flue selected: its Yüzden uzaklık (Ortada).
fn flue_props(app: &mut App) {
    props(app, 9);
}

fn ribbon(app: &mut App) {
    room(app, 50.0);
    app.tab = "arch";
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("nis-oda", written),
        ("nis-ayrintili", detailed),
        ("nis-pencere-alti", under_window),
        ("nis-arac", niche_tool),
        ("nis-derin", too_deep),
        ("nis-oznitelikler", niche_props),
        ("baca-oznitelikler", flue_props),
    ]
}

/// Mimari's Boşluk ▾ open: Boşluk, Niş and Baca.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![("nis-serit", ribbon, |s, app| press_caption(s, app, "Boşluk", false, true))]
}

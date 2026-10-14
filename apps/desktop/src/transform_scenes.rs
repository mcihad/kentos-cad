//! The pictures of docs/adr/0291 (Taşı, Kopyala, Döndür, Ölçekle with AutoCAD's rules): Kopyala's Dizi with five
//! places along the displacement, the copies' ghosts; the same with Sığdır, the last on the pointer; Taşı's Yer
//! değiştirme after Enter at the base point, the last displacement offered; Döndür in a CBS project, the turn in
//! grads clockwise beside the pointer; Döndür's Referans with Noktalar, the new direction shown by two points;
//! Kopyala's Kip in its prompt. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=donusum-dizi,donusum-sigdir,donusum-yer-degistirme,donusum-dondur-cbs,donusum-noktalar,donusum-kip cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// A building and a tree by a road, the view on them; the building selected.
fn ground(app: &mut App, cad: bool) {
    let mut o = Objects::new();
    o.path("parsel", &[[0.0, 0.0], [8.0, 0.0], [8.0, 6.0], [0.0, 6.0]], true);
    o.circle("cizim", [12.0, 3.0], 1.0);
    o.line("yol", [-4.0, -4.0], [40.0, -4.0]);
    open(app, o);
    if cad {
        make_cad(app);
    }
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 6.0,
            min_y: N - 12.0,
            max_x: E + 42.0,
            max_y: N + 18.0,
        },
        24.0,
    );
}

fn select_building(app: &mut App) {
    click(app, [4.0, 0.0]);
}

/// Kopyala, Dizi: five places along the displacement, the ghosts of the copies.
fn array(app: &mut App) {
    ground(app, true);
    select_building(app);
    run(app, "tool.copy");
    click(app, [0.0, 0.0]);
    typed(app, "A");
    typed(app, "5");
    hover(app, [9.0, 1.0]);
}

/// The same with Sığdır: the last copy on the pointer, the others sharing the way.
fn fitted(app: &mut App) {
    ground(app, true);
    select_building(app);
    run(app, "tool.copy");
    click(app, [0.0, 0.0]);
    typed(app, "A");
    typed(app, "5");
    typed(app, "S");
    hover(app, [30.0, 8.0]);
}

/// Taşı: a move typed, then Enter at the base point offers it again.
fn displacement(app: &mut App) {
    ground(app, true);
    select_building(app);
    run(app, "tool.move");
    typed(app, "Y");
    typed(app, "2,1");
    click(app, [4.0, 1.0]);
    run(app, "tool.move");
    let _ = app.update(Message::CommandSubmitted);
    hover(app, [20.0, 10.0]);
}

/// Döndür in a CBS project: the turn shown in grads, clockwise.
fn rotate_cbs(app: &mut App) {
    ground(app, false);
    select_building(app);
    run(app, "tool.rotate");
    click(app, [0.0, 0.0]);
    hover(app, [6.0, -6.0]);
}

/// Döndür, Referans, Noktalar: the building's edge as the reference, the road's way shown by two points.
fn new_points(app: &mut App) {
    ground(app, true);
    select_building(app);
    run(app, "tool.rotate");
    click(app, [0.0, 0.0]);
    typed(app, "R");
    click(app, [0.0, 0.0]);
    click(app, [8.0, 0.0]);
    typed(app, "N");
    click(app, [20.0, 2.0]);
    hover(app, [30.0, 10.0]);
}

/// Kopyala: Kip in its prompt, Tek chosen.
fn mode(app: &mut App) {
    ground(app, true);
    select_building(app);
    run(app, "tool.copy");
    typed(app, "P");
    hover(app, [20.0, 10.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("donusum-dizi", array),
        ("donusum-sigdir", fitted),
        ("donusum-yer-degistirme", displacement),
        ("donusum-dondur-cbs", rotate_cbs),
        ("donusum-noktalar", new_points),
        ("donusum-kip", mode),
    ]
}

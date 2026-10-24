//! The pictures of docs/adr/0339 (walls): CAD's Mimari tab; Duvar drawing a wall of KentOS's Dış duvar 25 (its
//! options in the command line, the faces and components from the command's plan under the cursor); a room drawn with
//! it, closed (K), and an inner wall, at 1:50 (standard: the components' hatches) and at 1:20 (detailed: the plaster
//! too); a wall selected with Öznitelikler's wall rows; Duvar stilleri with the section; Çizgiden duvar's Çiftler over
//! two pairs of lines; Proje ayarları › Mimari with two levels. `tools_screens` takes them in both themes at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=duvar-serit,duvar-ciz,duvar-yazildi,duvar-ayrintili,duvar-sematik,duvar-oznitelikler,duvar-stilleri,duvar-cizgiden,duvar-katlar cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! And docs/adr/0340's: a room of four separate walls at mitred corners with an inner wall teeing in, at 1:50 and its
//! tee at 1:20; two walls crossing and a corner too sharp, its mark; the Mimari tab's Duvar düzenle; Duvar birleşimi
//! with an end picked; Birleşimleri denetle; Öznitelikler's join rows; Duvarı böl:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=duvar-birlesim,duvar-birlesim-ayrintili,duvar-birlesim-x,duvar-duzenle-serit,duvar-birlesim-araci,duvar-birlesim-denetle,duvar-birlesim-oznitelikler,duvar-bol cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::json;

use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open_edited, run, typed};
use crate::tools_screens::Scene;

pub(crate) fn fit(app: &mut App, w: f64, h: f64) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.5,
            min_y: N - 1.5,
            max_x: E + w + 1.5,
            max_y: N + h + 1.5,
        },
        24.0,
    );
}

/// A CAD drawing at 1:`scale`, of `objects`.
pub(crate) fn opened(app: &mut App, objects: Objects, scale: f64) {
    open_edited(app, objects, |d| {
        d["settings"]["plotScale"] = json!(scale);
    });
    make_cad(app);
}

/// Duvar with KentOS's Dış duvar 25 chosen by name.
pub(crate) fn wall_tool(app: &mut App, style: &str) {
    app.command_expanded = true;
    run(app, "tool.wall");
    typed(app, "S");
    typed(app, style);
}

/// A 7 × 5 m room of Dış duvar 25, closed with K, and an inner wall of İç duvar 15 across it.
fn room(app: &mut App, scale: f64) {
    opened(app, Objects::new(), scale);
    fit(app, 7.0, 5.0);
    wall_tool(app, "Dış duvar 25");
    for p in [[0.0, 0.0], [7.0, 0.0], [7.0, 5.0], [0.0, 5.0]] {
        click(app, p);
    }
    typed(app, "K");
    wall_tool(app, "İç duvar 15");
    click(app, [4.2, 0.0]);
    click(app, [4.2, 5.0]);
    // Enter writes the wall, the next leaves the tool.
    typed(app, "");
    typed(app, "");
    hover(app, [9.0, -1.0]);
}

/// CAD's Mimari tab over the room.
fn ribbon(app: &mut App) {
    room(app, 50.0);
    app.tab = "arch";
}

/// Duvar drawing: three corners clicked, the cursor at the fourth, the wall's chips in the command line.
fn drawing(app: &mut App) {
    opened(app, Objects::new(), 50.0);
    fit(app, 7.0, 5.0);
    wall_tool(app, "Dış duvar 25");
    click(app, [0.0, 0.0]);
    click(app, [7.0, 0.0]);
    click(app, [7.0, 5.0]);
    hover(app, [1.5, 5.0]);
}

fn written(app: &mut App) {
    room(app, 50.0);
}

/// At 1:20 the plaster shows too: the room's corner where the inner wall meets the outer one, near.
fn detailed(app: &mut App) {
    room(app, 20.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + 3.0,
            min_y: N - 0.6,
            max_x: E + 7.6,
            max_y: N + 1.6,
        },
        24.0,
    );
    hover(app, [7.4, -0.4]);
}

/// At 1:200 a wall is its fill alone.
fn schematic(app: &mut App) {
    room(app, 200.0);
}

/// The room's outer wall selected: Öznitelikler's wall rows.
fn props(app: &mut App) {
    room(app, 50.0);
    app.selection.set([Slot(1)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Duvar stilleri over the room, the project's Dış duvar 25 chosen.
fn styles(app: &mut App) {
    room(app, 50.0);
    run(app, "arch.wallStyles");
}

/// Çizgiden duvar's Çiftler over two pairs of parallel lines, the walls to come from the command's plan.
fn from_lines(app: &mut App) {
    let mut o = Objects::new();
    o.line("cizim", [0.0, 0.0], [7.0, 0.0]);
    o.line("cizim", [0.0, 0.2], [7.0, 0.2]);
    o.line("cizim", [0.0, 3.0], [5.0, 3.0]);
    o.line("cizim", [0.0, 3.25], [5.0, 3.25]);
    o.line("cizim", [6.0, 1.0], [6.0, 4.0]);
    opened(app, o, 50.0);
    fit(app, 7.0, 4.5);
    app.command_expanded = true;
    app.selection
        .set([Slot(1), Slot(2), Slot(3), Slot(4), Slot(5)]);
    run(app, "tool.wallFromLines");
    typed(app, "Ç");
    hover(app, [6.6, -0.8]);
}

/// Proje ayarları › Mimari with two levels and the detail by the scale.
fn levels(app: &mut App) {
    room(app, 50.0);
    run(app, "arch.settings");
    let add = || crate::project::settings_message(crate::project::SettingsEvent::AddLevel);
    let _ = app.update(add());
    let _ = app.update(add());
}

/// One open wall of `style` from `a` to `b`, drawn with Duvar and written with Enter.
pub(crate) fn wall_between(app: &mut App, style: &str, a: [f64; 2], b: [f64; 2]) {
    wall_tool(app, style);
    click(app, a);
    click(app, b);
    typed(app, "");
    typed(app, "");
}

/// A 7 × 5 m room of four separate walls of Dış duvar 25 meeting at mitred corners (docs/adr/0340 §2), and an inner
/// wall of İç duvar 15 teeing into both long walls; the walls' cores pass the plaster.
pub(crate) fn joined_room(app: &mut App, scale: f64) {
    opened(app, Objects::new(), scale);
    fit(app, 7.0, 5.0);
    let c = [[0.0, 0.0], [7.0, 0.0], [7.0, 5.0], [0.0, 5.0]];
    for k in 0..4 {
        wall_between(app, "Dış duvar 25", c[k], c[(k + 1) % 4]);
    }
    wall_between(app, "İç duvar 15", [4.2, 0.0], [4.2, 5.0]);
    hover(app, [9.0, -1.0]);
}

fn joins_written(app: &mut App) {
    joined_room(app, 50.0);
}

/// At 1:20, the inner wall's tee into the outer wall near: its brick passes the plaster, the plaster lines broken.
fn joins_detailed(app: &mut App) {
    joined_room(app, 20.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + 3.4,
            min_y: N - 0.6,
            max_x: E + 5.0,
            max_y: N + 0.9,
        },
        24.0,
    );
    hover(app, [4.9, -0.5]);
}

/// Two walls crossing and a corner too sharp to mitre: the cross cleans up, the corner is said with its mark.
fn joins_cross(app: &mut App) {
    opened(app, Objects::new(), 50.0);
    fit(app, 8.0, 5.0);
    wall_between(app, "İç duvar 15", [0.0, 2.0], [6.0, 2.0]);
    wall_between(app, "Dış duvar 25", [3.0, -0.5], [3.0, 4.5]);
    wall_between(app, "İç duvar 15", [5.0, 4.5], [8.0, 4.5]);
    wall_between(app, "İç duvar 15", [8.0, 4.5], [5.0625, 5.25]);
    hover(app, [9.0, -1.0]);
}

/// CAD's Mimari tab with Duvar düzenle over the joined room.
fn edit_ribbon(app: &mut App) {
    joined_room(app, 50.0);
    app.tab = "arch";
}

/// Duvar birleşimi with the room's top right corner picked: the joins in the command line.
fn joins_tool(app: &mut App) {
    joined_room(app, 50.0);
    app.command_expanded = true;
    run(app, "tool.wallJoins");
    click(app, [6.9, 5.0]);
    hover(app, [6.2, 4.2]);
}

/// The sharp corner's issue in Birleşimleri denetle.
fn joins_check(app: &mut App) {
    joins_cross(app);
    run(app, "arch.wallJoinCheck");
}

/// The room's bottom wall selected: Öznitelikler's joins rows.
fn joins_props(app: &mut App) {
    joined_room(app, 50.0);
    app.selection.set([Slot(1)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Duvarı böl: the room's bottom wall split at the door's place, the two parts joined inline.
fn split(app: &mut App) {
    joined_room(app, 50.0);
    app.command_expanded = true;
    run(app, "tool.wallSplit");
    click(app, [2.0, 0.0]);
    hover(app, [2.6, -0.4]);
}

/// The new rectangle's inner clear dimensions, its actual wall body and reference ring.
fn rectangle_drawing(app: &mut App) {
    opened(app, Objects::new(), 100.0);
    fit(app, 4.0, 3.0);
    app.tab = "arch";
    app.command_expanded = true;
    run(app, "tool.wallRectangle");
    typed(app, "E");
    typed(app, "0.2m");
    click(app, [0.0, 0.0]);
    hover(app, [4.0, 3.0]);
}

fn rectangle_written(app: &mut App) {
    rectangle_drawing(app);
    click(app, [4.0, 3.0]);
    typed(app, "");
    hover(app, [5.0, 3.5]);
}

fn rectangle_dimensions(app: &mut App) {
    opened(app, Objects::new(), 100.0);
    fit(app, 4.0, 3.0);
    app.tab = "arch";
    app.command_expanded = true;
    run(app, "tool.wallRectangle");
    typed(app, "R");
    typed(app, "Dış ölçü");
    click(app, [0.0, 0.0]);
    typed(app, "B");
    typed(app, "4m,3m");
    hover(app, [4.0, 3.0]);
}

/// The same active wall preview and options at the user's 18 px interface text size.
fn rectangle_large_text(app: &mut App) {
    let _ = app.settings.choose(&[("appearance.textSize", json!(18))]);
    app.apply_settings();
    rectangle_dimensions(app);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("duvar-dikdortgen-onizleme", rectangle_drawing),
        ("duvar-dikdortgen-yazildi", rectangle_written),
        ("duvar-dikdortgen-boyutlar", rectangle_dimensions),
        ("duvar-dikdortgen-buyuk-yazi", rectangle_large_text),
        ("duvar-birlesim", joins_written),
        ("duvar-birlesim-ayrintili", joins_detailed),
        ("duvar-birlesim-x", joins_cross),
        ("duvar-duzenle-serit", edit_ribbon),
        ("duvar-birlesim-araci", joins_tool),
        ("duvar-birlesim-denetle", joins_check),
        ("duvar-birlesim-oznitelikler", joins_props),
        ("duvar-bol", split),
        ("duvar-serit", ribbon),
        ("duvar-ciz", drawing),
        ("duvar-yazildi", written),
        ("duvar-ayrintili", detailed),
        ("duvar-sematik", schematic),
        ("duvar-oznitelikler", props),
        ("duvar-stilleri", styles),
        ("duvar-cizgiden", from_lines),
        ("duvar-katlar", levels),
    ]
}

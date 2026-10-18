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
//! Test code only.

use kentos_domain::Slot;
use serde_json::json;

use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open_edited, run, typed};
use crate::tools_screens::Scene;

fn fit(app: &mut App, w: f64, h: f64) {
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
fn opened(app: &mut App, objects: Objects, scale: f64) {
    open_edited(app, objects, |d| {
        d["settings"]["plotScale"] = json!(scale);
    });
    make_cad(app);
}

/// Duvar with KentOS's Dış duvar 25 chosen by name.
fn wall_tool(app: &mut App, style: &str) {
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

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
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

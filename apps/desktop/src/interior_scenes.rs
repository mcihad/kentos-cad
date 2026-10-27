//! The pictures of docs/adr/0711 (İç ölçü zinciri): a 12 × 6 m plan of three rooms (Dış duvar 25 closed, two İç
//! duvar 15 partitions) and a column; the cut drawn across them, the chain previewed on it from the command's plan;
//! the chain following the pointer; written and selected; every line of the walls, the column's centre and the total;
//! a partition moved by its temporary distance, the chain's values following, its line in place; the Mimari tab's
//! Belgele group. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=ic-olcu-onizleme,ic-olcu-yer,ic-olcu-yazildi,ic-olcu-ayarlar,ic-olcu-kaydirildi,ic-olcu-serit cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;

use crate::app::{App, Message};
use crate::tools_scenes::{Objects, click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::{fit, opened, wall_tool};

/// Three rooms across 0..12 × 0..6 (the outer wall slot 1, the partitions 2 and 3) and a column at (10, 3) (slot 4).
fn plan(app: &mut App) {
    opened(app, Objects::new(), 50.0);
    fit(app, 12.0, 6.0);
    app.tab = "arch";
    wall_tool(app, "Dış duvar 25");
    for p in [[0.0, 0.0], [12.0, 0.0], [12.0, 6.0], [0.0, 6.0]] {
        click(app, p);
    }
    typed(app, "K");
    // Typed, not clicked: no snap moves them off 4 and 8.
    for x in [4.0, 8.0] {
        use crate::tools_scenes::{E, N};
        wall_tool(app, "İç duvar 15");
        typed(app, &format!("{},{}", E + x, N));
        typed(app, &format!("{},{}", E + x, N + 6.0));
        typed(app, "");
        typed(app, "");
    }
    run(app, "tool.column");
    click(app, [10.0, 3.0]);
    typed(app, "");
    app.selection.clear();
}

/// The cut drawn from west of the plan; its end at the pointer east of it: the chain on the cut, the faces marked.
fn previewed(app: &mut App) {
    plan(app);
    run(app, "tool.interiorDimensions");
    click(app, [-1.2, 3.0]);
    hover(app, [13.2, 3.0]);
}

/// The cut ended: the chain follows the pointer along it.
fn placing(app: &mut App) {
    previewed(app);
    click(app, [13.2, 3.0]);
    hover(app, [6.0, 4.6]);
}

/// The third click writes it: nine dimensions, one step, selected; the tool waits for the next cut.
fn written(app: &mut App) {
    placing(app);
    click(app, [6.0, 4.6]);
    hover(app, [6.0, -1.0]);
}

/// Bütün katmanlar, Kolonlarda: Merkezler and Toplam: the outer wall's plaster lines too, the column's centre, the
/// overall dimension one row further.
fn settings(app: &mut App) {
    plan(app);
    run(app, "tool.interiorDimensions");
    for (key, value) in [("F", "skins"), ("K", "centers"), ("T", "yes")] {
        let _ = app.tool_setting(crate::architecture_settings::Commit {
            tool: "interiorDimensions",
            key,
            value: value.into(),
        });
    }
    click(app, [-1.2, 3.0]);
    click(app, [13.2, 3.0]);
    hover(app, [6.0, 4.4]);
}

/// The first partition moved by its temporary distance (3.00 to the second): its two dimensions' values change, the
/// chain's line stays where it was placed.
fn moved(app: &mut App) {
    written(app);
    typed(app, "");
    app.selection.set([Slot(2)]);
    hover(app, [4.6, 1.2]);
    let numbers = app.session.temporary_numbers();
    let east = numbers
        .iter()
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .copied()
        .expect("a number");
    click(
        app,
        [
            east.x - crate::tools_scenes::E,
            east.y - crate::tools_scenes::N,
        ],
    );
    let _ = app.update(Message::TextField(crate::text_field::Event::Input(
        "3.00".into(),
    )));
    let _ = app.update(Message::TextField(crate::text_field::Event::Keep));
    hover(app, [6.0, -1.0]);
}

/// The two western rooms near enough to read the values (1:50, their real 2.5 mm height).
fn near(app: &mut App) {
    use crate::tools_scenes::{E, N};
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.0,
            min_y: N + 1.8,
            max_x: E + 8.6,
            max_y: N + 5.4,
        },
        24.0,
    );
}

/// The written chain, near.
fn written_near(app: &mut App) {
    written(app);
    near(app);
    hover(app, [4.0, 2.2]);
}

/// The moved partition, near: 4.625 and 3.000 where 3.800 and 3.850 were, the line where it was.
fn moved_near(app: &mut App) {
    moved(app);
    app.selection.clear();
    near(app);
    hover(app, [4.0, 2.2]);
}

/// The Mimari tab over the plan: Belgele's İç ölçü zinciri beside Duvar ölçüleri.
fn ribbon(app: &mut App) {
    written(app);
    typed(app, "");
    app.selection.clear();
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("ic-olcu-onizleme", previewed),
        ("ic-olcu-yer", placing),
        ("ic-olcu-yazildi", written),
        ("ic-olcu-ayarlar", settings),
        ("ic-olcu-kaydirildi", moved),
        ("ic-olcu-yakin", written_near),
        ("ic-olcu-kaydirildi-yakin", moved_near),
        ("ic-olcu-serit", ribbon),
    ]
}

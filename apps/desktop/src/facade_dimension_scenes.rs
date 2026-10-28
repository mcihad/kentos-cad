//! The pictures of docs/adr/0729 (façade, wall group and space dimension recipes): a 10 × 8 m house of four Dış duvar
//! 25 walls drawn corner to corner at 1:50, an İç duvar 15 partition at 4 m, two doors in its south wall, a window in
//! its north wall, grids A–C. Duvar ölçüleri as Cephe on the south wall with every row, the values that do not fit in
//! the danger tone; written with one Enter and a member's Öznitelikler; a door added afterwards and the rows following
//! it; the four walls selected, Dört yan, previewed and written; Çekirdek; a room's outline as Mahal. `tools_screens`
//! takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=cephe-olcu-onizleme,cephe-olcu-yazildi,cephe-olcu-oznitelikler,cephe-olcu-izleme,cephe-olcu-dort-yan,cephe-olcu-dort-yan-yazildi,cephe-olcu-cekirdek,mahal-olcu cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;

use crate::app::App;
use crate::tools_scenes::{E, N, Objects, click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::{opened, wall_between};

/// The camera on a part of the drawing, `[west, south, east, north]` from (E, N): the values readable at 1:50.
fn look(app: &mut App, b: [f64; 4]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + b[0],
            min_y: N + b[1],
            max_x: E + b[2],
            max_y: N + b[3],
        },
        24.0,
    );
}

/// The south façade and its rows.
const SOUTH: [f64; 4] = [-1.2, -3.4, 11.2, 1.4];
/// The whole house and its four sides' rows.
const WHOLE: [f64; 4] = [-3.5, -3.2, 13.5, 11.2];

/// The house: walls slots 1–4 (south, east, north, west), the partition 5, then its doors, window and grids.
fn house(app: &mut App) {
    opened(app, Objects::new(), 50.0);
    look(app, WHOLE);
    app.tab = "arch";
    app.command_expanded = true;
    let c = [[0.0, 0.0], [10.0, 0.0], [10.0, 8.0], [0.0, 8.0]];
    for k in 0..4 {
        wall_between(app, "Dış duvar 25", c[k], c[(k + 1) % 4]);
    }
    wall_between(app, "İç duvar 15", [4.0, 0.0], [4.0, 8.0]);
    for x in [1.5, 6.5] {
        run(app, "tool.door");
        click(app, [x, -0.05]);
        typed(app, "");
    }
    run(app, "tool.window");
    click(app, [7.0, 8.05]);
    typed(app, "");
    run(app, "tool.grid");
    typed(app, "S");
    typed(app, "4 6");
    typed(app, "E");
    typed(app, "A");
    click(app, [0.0, -2.5]);
    click(app, [0.0, 10.5]);
    click(app, [5.0, 5.0]);
    typed(app, "");
    app.selection.clear();
}

fn set(app: &mut App, key: &'static str, value: &str) {
    let _ = app.tool_setting(crate::architecture_settings::Commit {
        tool: "wallDimensions",
        key,
        value: value.into(),
    });
}

/// Cephe on the south wall, every row: openings, walls, grids and the overall length; the rows south of it.
fn preview(app: &mut App) {
    house(app);
    run(app, "tool.wallDimensions");
    set(app, "M", "facade");
    set(app, "W", "yes");
    set(app, "G", "yes");
    click(app, [5.0, 0.0]);
    look(app, SOUTH);
    hover(app, [5.0, -1.6]);
}

/// One Enter writes the recipe and its dimensions.
fn written(app: &mut App) {
    preview(app);
    typed(app, "");
    app.selection.clear();
    hover(app, [10.8, 1.2]);
}

/// A member selected: Öznitelikler shows the recipe's source, direction, rows and lines.
fn properties(app: &mut App) {
    written(app);
    let member = app
        .document
        .as_ref()
        .and_then(|d| {
            let set = d.model.dimension_sets().first()?;
            d.model.slot_of(kentos_domain::Uuid::from_bytes(set.members.first()?.uid.0))
        })
        .expect("a member");
    app.selection.set([member]);
}

/// A door added afterwards at 3 m: the openings row measures it in the same step.
fn followed(app: &mut App) {
    written(app);
    look(app, WHOLE);
    run(app, "tool.door");
    click(app, [3.0, -0.05]);
    typed(app, "");
    app.selection.clear();
    look(app, SOUTH);
    hover(app, [10.8, 1.2]);
}

/// The four walls selected before the tool: a façade of four walls, Dört yan.
fn four_sides(app: &mut App) {
    house(app);
    app.selection.set([Slot(1), Slot(2), Slot(3), Slot(4)]);
    run(app, "tool.wallDimensions");
    set(app, "N", "all");
    hover(app, [5.0, -1.6]);
}

fn four_sides_written(app: &mut App) {
    four_sides(app);
    typed(app, "");
    app.selection.clear();
    hover(app, [13.0, 11.0]);
}

/// Ölçülen çizgi → Çekirdek: the brick's faces instead of the plaster's.
fn core(app: &mut App) {
    house(app);
    run(app, "tool.wallDimensions");
    set(app, "M", "facade");
    set(app, "W", "yes");
    set(app, "Y", "core");
    click(app, [5.0, 0.0]);
    look(app, SOUTH);
    hover(app, [5.0, -1.6]);
}

/// The west room's space, then Mahal across its long edge: its door and its net length.
fn space(app: &mut App) {
    house(app);
    run(app, "tool.space");
    click(app, [2.0, 4.0]);
    typed(app, "");
    app.selection.clear();
    run(app, "tool.wallDimensions");
    set(app, "M", "space");
    click(app, [2.0, 4.0]);
    set(app, "O", "across");
    look(app, [-1.2, -3.4, 6.8, 1.4]);
    hover(app, [2.0, -1.6]);
}

/// The member's Öznitelikler scrolled to the recipe's rows: source, direction, rows, lines, spacing.
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    fn scroll(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App) {
        let mut update = |app: &mut App, message: crate::app::Message| {
            let _ = app.update(message);
        };
        let at = crate::tools_screens::caption_at(s, app, "Genel", false);
        s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(at, -9.0));
    }
    vec![("cephe-olcu-oznitelikler", properties, scroll)]
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("cephe-olcu-onizleme", preview),
        ("cephe-olcu-yazildi", written),
        ("cephe-olcu-izleme", followed),
        ("cephe-olcu-dort-yan", four_sides),
        ("cephe-olcu-dort-yan-yazildi", four_sides_written),
        ("cephe-olcu-cekirdek", core),
        ("mahal-olcu", space),
    ]
}

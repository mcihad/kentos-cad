//! The pictures of docs/adr/0322: Merkez işareti on a column and an arched door, Eksen çizgisi down a corridor and
//! Kot on a slab's corner, drawn with their tools, the level selected with Öznitelikler's rows (olcu-isaretler); the
//! centre mark's preview over a column (olcu-isaret-onizleme); the column moved, a wall moved and the corner given an
//! elevation, the marks following (olcu-isaret-izleme). `tools_screens` takes them in both themes at 1440×900 and
//! 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=olcu-isaretler,olcu-isaret-onizleme,olcu-isaret-izleme cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::{CommandResult, EntitiesEdit, EntitiesTransform};
use kentos_domain::Slot;
use kentos_native_application::ExecutionContext;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, layer, open_edited, run, typed};
use crate::tools_screens::Scene;

/// The plan: two columns, an arched door, a corridor's two walls and a slab's edge with an elevation at its corner.
fn drawing(app: &mut App) {
    let mut o = Objects::new();
    o.circle("kolon", [1.0, 5.0], 0.3);
    o.circle("kolon", [7.0, 5.0], 0.3);
    o.arc("kapi", [4.0, 0.0], 0.9, 0.0, std::f64::consts::PI);
    o.line("duvar", [0.0, 2.0], [8.0, 2.0]);
    o.line("duvar", [8.0, 3.2], [0.0, 3.2]);
    o.path_z(
        "doseme",
        &[[9.0, 0.0], [9.0, 6.0], [12.0, 6.0]],
        false,
        &[Some(3.2), None, Some(3.35)],
        &[],
    );
    open_edited(app, o, |d| {
        d["layers"] = serde_json::json!([
            layer("kolon", "Kolon", "#3E63DD", 0.35),
            layer("kapi", "Kapı", "#8E4EC6", 0.25),
            layer("duvar", "Duvar", "#3E63DD", 0.5),
            layer("doseme", "Döşeme", "#12A594", 0.25),
            layer("olcu", "Ölçü", "#E5484D", 0.18),
        ]);
        d["activeLayer"] = serde_json::json!("olcu");
        d["settings"]["plotScale"] = serde_json::json!(100);
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.0,
            min_y: N - 1.5,
            max_x: E + 13.5,
            max_y: N + 7.0,
        },
        24.0,
    );
}

/// The marks drawn with their tools: both columns and the door's arch, the corridor's axis, the slab's corner.
fn marked(app: &mut App) {
    drawing(app);
    run(app, "tool.centerMark");
    click(app, [1.3, 5.0]);
    click(app, [7.0, 5.3]);
    click(app, [4.0, 0.9]);
    run(app, "tool.dimCenterLine");
    click(app, [2.0, 2.0]);
    click(app, [6.0, 3.2]);
    run(app, "tool.dimElevation");
    click(app, [9.0, 0.0]);
    click(app, [10.0, 0.7]);
    // A corner with no elevation of its own: the level is typed.
    click(app, [9.0, 6.0]);
    typed(app, "3.35");
    click(app, [7.6, 6.6]);
    run(app, "tool.select");
}

fn listed(app: &mut App) {
    marked(app);
    let last = app.document.as_ref().map_or(0, |d| d.model.len() as u32);
    app.selection.set([Slot(last)]);
    hover(app, [13.0, 6.8]);
}

fn previewed(app: &mut App) {
    drawing(app);
    run(app, "tool.centerMark");
    click(app, [1.3, 5.0]);
    hover(app, [7.25, 5.15]);
}

fn uid(app: &App, slot: u32) -> String {
    app.document
        .as_ref()
        .expect("a drawing")
        .model
        .uid(Slot(slot))
        .expect("an object")
        .to_string()
}

/// The left column moved 0.8 m right, the far wall 0.4 m out and the slab's middle corner given 3.35 m: the marks
/// go with them.
fn followed(app: &mut App) {
    marked(app);
    let (column, wall, slab) = (uid(app, 1), uid(app, 5), uid(app, 6));
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    for (uid, dx, dy) in [(column, 0.8, 0.0), (wall, 0.0, 0.4)] {
        let input: EntitiesTransform = serde_json::from_value(serde_json::json!({
            "uids": [uid], "transform": { "kind": "move", "dx": dx, "dy": dy },
        }))
        .expect("a move");
        assert!(matches!(
            kentos_native_application::transform::execute(&mut ExecutionContext::new(doc), input),
            CommandResult::Completed { .. }
        ));
    }
    let xy = |x: f64, y: f64| serde_json::json!({ "x": E + x, "y": N + y });
    let input: EntitiesEdit = serde_json::from_value(serde_json::json!({
        "operation": "elevation",
        "changes": [{ "kind": "update", "uid": slab, "geometry": {
            "kind": "polyline", "pts": [xy(9.0, 0.0), xy(9.0, 6.0), xy(12.0, 6.0)], "zs": [3.05, 3.35, 3.35],
        } }],
    }))
    .expect("an elevation edit");
    assert!(matches!(
        kentos_native_application::edit::execute(&mut ExecutionContext::new(doc), input),
        CommandResult::Completed { .. }
    ));
    hover(app, [13.0, 6.8]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("olcu-isaretler", listed),
        ("olcu-isaret-onizleme", previewed),
        ("olcu-isaret-izleme", followed),
    ]
}

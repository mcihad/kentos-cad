//! The pictures of docs/adr/0345 (view marks and room finishes): the room of docs/adr/0342 with a section across it, an
//! elevation looking north and a detail of the kitchen's corner, pointing at the sheet A-301's frames and the named
//! view “Kesit A-A”; CAD's Mimari tab with Görünüm, Mahal's finishes and Donatı; Kesit and Detay with their marks under
//! the cursor; Başvurular; the sheet renamed, the marks' words following; Döşeme deseni and Tavan ızgarası on the rooms.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=isaret-serit,isaret-plan,isaret-kesit,isaret-detay,isaret-basvurular,isaret-yeniden-adlandir,mahal-doseme,mahal-tavan cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::{Entity, NewViewMark, ViewMarkForm, ViewMarksCreate, ViewTarget, ViewsOp};
use kentos_domain::Slot;

use crate::app::App;
use crate::tools_scenes::{click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::fit;

fn p(x: f64, y: f64) -> kentos_commands::Vec2 {
    kentos_commands::Vec2 {
        x: crate::tools_scenes::E + x,
        y: crate::tools_scenes::N + y,
    }
}

/// A sheet of the book with its map frames, as the sheet core keeps it.
fn sheet(id: &str, name: &str, frames: &[(&str, &str)]) -> serde_json::Value {
    let items: Vec<serde_json::Value> = frames
        .iter()
        .enumerate()
        .map(|(k, (item, title))| {
            serde_json::json!({
                "bindings": [], "constraints": {"h": "left", "relativeTo": "margins", "v": "top"},
                "frame": {"height": 120000, "left": 20000 + 190000 * k, "top": 10000, "width": 180000},
                "hidden": false, "id": item,
                "kind": {"clipToAtlasFeature": false, "grids": [], "labelBand": 0, "layers": {"type": "all"},
                         "overviewFrame": {"cap": "butt", "color": "#d0021b", "dash": [], "join": "miter", "width": 500},
                         "type": "map", "view": {"center": {"x": 4.0, "y": 2.5}, "rotation": 0, "scale": 50, "type": "fixed"}},
                "locked": false, "name": title, "opacity": 100, "padding": 0, "printable": true, "rotation": 0
            })
        })
        .collect();
    serde_json::json!({
        "export": {"dpi": 300, "fileName": "[% @pafta_adi %]", "format": "svg"}, "guides": [], "id": id, "items": items,
        "name": name,
        "page": {"margins": {"bottom": 10000, "left": 20000, "right": 10000, "top": 10000}, "orientation": "landscape",
                 "paper": "a3", "size": {"height": 297000, "width": 420000}},
        "snapGrid": {"enabled": false, "spacing": 5000, "visible": false}, "variables": [], "variants": []
    })
}

fn sheets_edit(app: &mut App, ops: serde_json::Value) {
    let Some(doc) = app.document.as_mut() else {
        return;
    };
    let Ok(input) = serde_json::from_value(serde_json::json!({ "ops": ops })) else {
        return;
    };
    let _ = kentos_native_application::sheets_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        input,
    );
}

/// The room with its spaces, the sheet A-301 (Kesitler, Detaylar) and the named view “Kesit A-A”.
fn targets(app: &mut App) {
    crate::space_scenes::spaces(app);
    fit(app, 7.0, 5.0);
    sheets_edit(
        app,
        serde_json::json!([{ "op": "addSheet", "sheet": sheet("pafta-1", "A-301", &[("harita-1", "Kesitler"), ("harita-2", "Detaylar")]) }]),
    );
    let _ = app.edit_views(ViewsOp::Save {
        name: "Kesit A-A".into(),
        center: p(3.5, 2.5),
        width: 10.0,
        height: 7.0,
        twist: None,
    });
}

/// The marks: section A to Kesitler, the elevation to the named view, the kitchen's detail to Detaylar (its number the
/// frame's order).
fn marked(app: &mut App) {
    targets(app);
    let view = app
        .named_views()
        .first()
        .map(kentos_commands::view_key)
        .unwrap_or_default();
    let mark = |form, points, mark: &str, target| NewViewMark {
        form,
        points,
        radius: None,
        at: None,
        angle: None,
        flip: false,
        mark: mark.into(),
        target: Some(target),
        size: None,
    };
    let marks = vec![
        mark(ViewMarkForm::Section, vec![p(-1.6, 2.2), p(8.6, 2.2)], "A", ViewTarget::sheet("pafta-1", "harita-1")),
        NewViewMark {
            angle: Some(90.0),
            ..mark(ViewMarkForm::Elevation, vec![p(3.0, -1.4)], "1", ViewTarget::view(&view))
        },
        NewViewMark {
            radius: Some(0.9),
            at: Some(p(8.9, 6.2)),
            ..mark(ViewMarkForm::Detail, vec![p(6.6, 4.3)], "", ViewTarget::sheet("pafta-1", "harita-2"))
        },
    ];
    if let Some(doc) = app.document.as_mut() {
        let layer = doc.model.layers().active().to_owned();
        let _ = kentos_native_application::viewmarks_create::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            ViewMarksCreate {
                layer_id: layer,
                marks,
                color: None,
                line_weight: None,
                expected_revision: None,
            },
        );
    }
    hover(app, [9.5, -1.5]);
}

fn ribbon(app: &mut App) {
    marked(app);
    app.tab = "arch";
}

fn plan(app: &mut App) {
    marked(app);
}

/// Kesit: the cut line's first point placed, its second under the cursor, numbered B and pointing at Detaylar.
fn section_tool(app: &mut App) {
    targets(app);
    app.command_expanded = true;
    run(app, "tool.viewSection");
    typed(app, "N");
    typed(app, "B");
    typed(app, "H");
    typed(app, "H");
    typed(app, "H");
    click(app, [4.4, -1.2]);
    hover(app, [4.4, 6.4]);
}

/// Detay: the middle and the radius given, the bubble under the cursor.
fn detail_tool(app: &mut App) {
    targets(app);
    app.command_expanded = true;
    run(app, "tool.viewDetail");
    typed(app, "N");
    typed(app, "3");
    click(app, [0.8, 4.2]);
    click(app, [1.6, 4.2]);
    hover(app, [-0.8, 6.6]);
}

/// Başvurular with the detail chosen.
fn references(app: &mut App) {
    marked(app);
    let detail = app.document.as_ref().and_then(|d| {
        d.model
            .entities()
            .find(|e| matches!(e, Entity::ViewMark(m) if m.mark.form == ViewMarkForm::Detail))
            .map(|e| Slot(e.base().id))
    });
    if let Some(s) = detail {
        app.selection.set([s]);
    }
    run(app, "arch.viewRefs");
}

/// The sheet renamed A-302 and the view “Kesit B-B”: every mark's bottom half follows, no undo step.
fn renamed(app: &mut App) {
    marked(app);
    sheets_edit(app, serde_json::json!([{ "op": "renameSheet", "id": "pafta-1", "name": "A-302" }]));
    let _ = app.edit_views(ViewsOp::Rename {
        name: "Kesit A-A".into(),
        to: "Kesit B-B".into(),
    });
    let all: Vec<Slot> = app
        .document
        .as_ref()
        .map(|d| {
            d.model
                .entities()
                .filter(|e| matches!(e, Entity::ViewMark(_)))
                .map(|e| Slot(e.base().id))
                .collect()
        })
        .unwrap_or_default();
    app.selection.set(all);
}

/// Döşeme deseni: the living room done with NET, the kitchen's under the cursor with BRICK.
fn floor(app: &mut App) {
    crate::space_scenes::spaces(app);
    fit(app, 7.0, 5.0);
    app.command_expanded = true;
    run(app, "tool.spaceFloor");
    typed(app, "D");
    typed(app, "NET");
    click(app, [2.0, 2.5]);
    typed(app, "D");
    typed(app, "BRICK");
    hover(app, [5.6, 2.5]);
}

/// Tavan ızgarası: 60 × 120 from the middle in the living room, the kitchen's under the cursor.
fn ceiling(app: &mut App) {
    crate::space_scenes::spaces(app);
    fit(app, 7.0, 5.0);
    app.command_expanded = true;
    run(app, "tool.spaceCeiling");
    typed(app, "A");
    typed(app, "0.6x1.2");
    click(app, [2.0, 2.5]);
    typed(app, "A");
    typed(app, "0.6");
    hover(app, [5.6, 2.5]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("isaret-serit", ribbon),
        ("isaret-plan", plan),
        ("isaret-kesit", section_tool),
        ("isaret-detay", detail_tool),
        ("isaret-basvurular", references),
        ("isaret-yeniden-adlandir", renamed),
        ("mahal-doseme", floor),
        ("mahal-tavan", ceiling),
    ]
}

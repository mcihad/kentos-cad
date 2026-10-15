//! The pictures of docs/adr/0312 (the drawing standard, the look's sources): Çizim standardı on an office drawing
//! with old layers, wrong looks, a foreign font and an object with its own line type (the findings, the parts of the
//! fix and its preview); the same window after Uygula (the A- layers in Katmanlar, what is left to do by hand); the
//! Yönet tab's Standart panel; a door block drawn through an insert with its own dashed line type and half its
//! transparency, a piece that follows its layer keeping its continuous line. `tools_screens` takes them in both
//! themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=standart-bulgular,standart-uygulandi,standart-serit,standart-blok cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited, run};
use crate::tools_screens::Scene;

fn styled(id: &str, name: &str, color: &str, weight: f64, more: Value) -> Value {
    let mut l = layer(id, name, color, weight);
    if let (Some(l), Some(more)) = (l.as_object_mut(), more.as_object()) {
        for (k, v) in more {
            l.insert(k.clone(), v.clone());
        }
    }
    l
}

/// A small plan on old layers: walls on `duvar` and `Wall`, a door on `kapı_eski`, furniture on `mobilya1`, an axis
/// on a dotted layer, a room name on `Kat1`; one wall with its own red colour.
fn ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "duvar",
        &[[0.0, 0.0], [12.0, 0.0], [12.0, 8.0], [0.0, 8.0]],
        true,
    );
    o.path(
        "wall",
        &[[0.25, 0.25], [11.75, 0.25], [11.75, 7.75], [0.25, 7.75]],
        true,
    );
    o.line("kapi", [5.0, 0.25], [5.0, 1.25]);
    o.arc("kapi", [5.0, 0.25], 1.0, 0.0, std::f64::consts::FRAC_PI_2);
    o.path(
        "mobilya",
        &[[1.0, 4.5], [3.0, 4.5], [3.0, 7.0], [1.0, 7.0]],
        true,
    );
    o.circle("mobilya", [8.0, 4.0], 0.9);
    o.line("aks", [-1.5, 4.0], [13.5, 4.0]);
    o.line("kat1", [14.0, 0.0], [14.0, 8.0]);
    o
}

fn tree(d: &mut Value) {
    d["layers"] = json!([
        styled("duvar", "A-DUVAR", "#E5484D", 0.35, json!({})),
        styled("wall", "Wall", "#30A46C", 0.35, json!({})),
        styled("kapi", "kapı_eski", "#C8A165", 0.18, json!({})),
        styled("mobilya", "mobilya1", "#8A94A6", 0.13, json!({})),
        styled(
            "aks",
            "Aks",
            "#E06C75",
            0.18,
            json!({ "style": { "color": "#E06C75", "lineType": "dotted", "lineWeight": 0.18 } })
        ),
        styled("kat1", "Kat1", "fg", 0.25, json!({})),
    ]);
    d["activeLayer"] = json!("kat1");
    d["settings"]["drawingFont"] = json!("quicksand");
    if let Some(list) = d["entities"].as_array_mut()
        && let Some(first) = list.first_mut()
    {
        first["color"] = json!("#FF0000");
    }
}

fn drawn(app: &mut App) {
    open_edited(app, ground(), tree);
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 3.0,
            min_y: N - 3.0,
            max_x: E + 17.0,
            max_y: N + 11.0,
        },
        24.0,
    );
    hover(app, [16.0, -2.0]);
}

/// The findings, the four parts of the fix and its preview.
fn findings(app: &mut App) {
    drawn(app);
    run(app, "standards.check");
    // The first finding pressed: its whole message under the table, its layer chosen in Katmanlar.
    let _ = app.update(Message::DrawingStandard(
        crate::drawing_standard::Event::Pick(0),
    ));
}

/// After Uygula: the standard's layers in Katmanlar, what is left (the furniture's name, the dotted axis, the
/// wall's own colour) said.
fn applied(app: &mut App) {
    drawn(app);
    run(app, "standards.apply");
    let _ = app.update(Message::DrawingStandard(
        crate::drawing_standard::Event::Apply,
    ));
}

/// The Yönet tab with its Standart panel.
fn ribbon(app: &mut App) {
    drawn(app);
    let _ = app.update(Message::RibbonTab("manage"));
}

/// A door block through an insert with its own dashed line type and transparency: the leaf and its swing dashed and
/// half seen, the frame (on its own layer, which it follows for its line type) continuous.
fn block(app: &mut App) {
    let mut o = Objects::new();
    o.path(
        "cizim",
        &[[0.0, 0.0], [12.0, 0.0], [12.0, 8.0], [0.0, 8.0]],
        true,
    );
    let id = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0003";
    o.insert("cizim", id, [3.0, 0.0], 1.0, 0.0, false);
    o.insert("cizim", id, [8.0, 0.0], 1.0, 0.0, false);
    open_edited(app, o, |d| {
        d["blocks"] = json!([{
            "id": id, "name": "Kapı", "base": { "x": 0, "y": 0 },
            "entities": [
                { "kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": { "x": 0, "y": 0 }, "b": { "x": 0, "y": 1.6 } },
                { "kind": "arc", "id": 2, "layerId": "", "attrs": {}, "c": { "x": 0, "y": 0 }, "r": 1.6, "a0": 0, "a1": std::f64::consts::FRAC_PI_2 },
                { "kind": "line", "id": 3, "layerId": "parsel", "attrs": {}, "byLayer": ["color", "lineType", "transparency"],
                  "a": { "x": 1.6, "y": 0 }, "b": { "x": 1.6, "y": -0.3 } }
            ]
        }]);
        // A door's scale: its lines as they are on paper at 1:50.
        d["settings"]["plotScale"] = json!(50);
        // The first insert its own dashed line type and half its transparency; the second as its layer draws it.
        if let Some(list) = d["entities"].as_array_mut()
            && let Some(first) = list.get_mut(1)
        {
            first["lineType"] = json!("dashed");
            first["lineTypeScale"] = json!(0.5);
            first["transparency"] = json!(0.5);
            first["color"] = json!("#4AA3DF");
        }
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + 1.0,
            min_y: N - 1.5,
            max_x: E + 11.0,
            max_y: N + 3.0,
        },
        24.0,
    );
    hover(app, [11.5, 2.5]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("standart-bulgular", findings),
        ("standart-uygulandi", applied),
        ("standart-serit", ribbon),
        ("standart-blok", block),
    ]
}

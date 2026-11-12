//! The pictures of docs/adr/0306 (Denetle by its command) in a CAD project: a site plan put together from other
//! drawings, carrying what that leaves behind: a parcel number linked to a parcel that was not brought along, a hatch
//! whose boundary is gone, a dimension bound to a corner that is gone, a title naming a text style and a dimension
//! naming a dimension style the project lacks, a note on a locked layer linked to nothing; unused an empty layer, a
//! group with an empty layer, a tree block nothing places, a text and a dimension style. Shown first (the broken
//! objects dashed, the counts on the command line, the unused by name in the log); then with both kinds of styles to
//! remove; and what the write left. `tools_screens` takes them in the dark and the light theme at 1440×900 and
//! 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=denetle-bulunan,denetle-stiller,denetle-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::{Value, json};

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited, run, typed};
use crate::tools_screens::Scene;

const GONE: &str = "0192f5a2-0000-7000-8000-00000000ffff";
const TREE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e02";
const MANHOLE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e01";
const TITLE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101";
const OLD_TITLE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0102";
const PLAN: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5001";
const OLD_PLAN: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5002";
const MISSING: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999";

fn xy(p: [f64; 2]) -> Value {
    json!({ "x": E + p[0], "y": N + p[1] })
}

fn plan() -> Objects {
    let mut o = Objects::new();
    // The road and the parcels.
    o.line("yol", [-2.0, 0.0], [42.0, 0.0]);
    o.line("yol", [-2.0, -8.0], [42.0, -8.0]);
    o.path(
        "parsel",
        &[[0.0, 2.0], [18.0, 2.0], [18.0, 20.0], [0.0, 20.0]],
        true,
    );
    o.path(
        "parsel",
        &[[20.0, 2.0], [40.0, 2.0], [40.0, 20.0], [20.0, 20.0]],
        true,
    );
    // The parcel number of a parcel that was not brought along.
    let number = o.text("parsel", [8.0, 10.0], "101", 1.8, 0.0);
    o.fields(number, json!({ "labelOf": GONE, "labelScale": 500 }));
    // The title in a style the project lacks, the note in one it has.
    let title = o.text("cizim", [0.0, 24.0], "Ada 412 vaziyet planı", 1.6, 0.0);
    o.fields(title, json!({ "textStyle": MISSING }));
    let note = o.text("cizim", [20.0, 24.0], "Yapı yaklaşma sınırı 5 m", 1.0, 0.0);
    o.fields(note, json!({ "textStyle": TITLE }));
    // A green area whose boundary is gone.
    let green = o.hatch(
        "cizim",
        &[[24.0, 6.0], [36.0, 6.0], [36.0, 16.0], [24.0, 16.0]],
        json!({ "type": "lines", "angle": 45, "spacing": 1 }),
    );
    o.fields(
        green,
        json!({ "assoc": { "outer": GONE, "seed": xy([30.0, 11.0]) } }),
    );
    // A frontage bound to a corner that is gone, another in a style the project lacks.
    let front = o.dimension(
        "cizim",
        None,
        [[0.0, 2.0], [18.0, 2.0]],
        None,
        -1.0,
        json!({}),
    );
    o.fields(
        front,
        json!({ "refs": { "a": { "uid": GONE, "place": { "kind": "start" } } }, "dimStyle": PLAN }),
    );
    let side = o.dimension(
        "cizim",
        None,
        [[40.0, 2.0], [40.0, 20.0]],
        None,
        -1.5,
        json!({}),
    );
    o.fields(side, json!({ "dimStyle": MISSING }));
    o.insert("cizim", MANHOLE, [19.0, -4.0], 1.0, 0.0, false);
    // A note on the locked layer, linked to nothing.
    let old = o.text("eski", [0.0, -12.0], "Eski parsel 99", 1.0, 0.0);
    o.fields(old, json!({ "labelOf": GONE, "labelScale": 500 }));
    o
}

fn drawn(app: &mut App) {
    open_edited(app, plan(), |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        layers.push(layer("bos", "Boş", "fg", 0.25));
        let mut group = layer("ithal", "İthal", "fg", 0.25);
        group["type"] = json!("group");
        group["children"] = json!([layer("ithal-yol", "Eski yol", "fg", 0.25)]);
        layers.push(group);
        let mut locked = layer("eski", "Eski", "#8B8D98", 0.25);
        locked["locked"] = json!(true);
        layers.push(locked);
        d["layers"] = Value::Array(layers);
        d["settings"]["textStyles"] = json!([
            { "id": TITLE, "name": "Not", "font": "arimo" },
            { "id": OLD_TITLE, "name": "Eski başlık", "font": "overpass", "bold": true },
        ]);
        d["settings"]["dimensionStyles"] = json!([
            { "id": PLAN, "name": "Vaziyet", "height": 2.5 },
            { "id": OLD_PLAN, "name": "Eski ölçü", "height": 3.5 },
        ]);
        d["blocks"] = json!([
            { "id": MANHOLE, "name": "Rögar", "base": { "x": 0, "y": 0 },
              "entities": [{ "kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": { "x": 0, "y": 0 }, "r": 0.6 }] },
            { "id": TREE, "name": "Ağaç", "base": { "x": 0, "y": 0 },
              "entities": [{ "kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": { "x": 0, "y": 0 }, "r": 1.5 }] },
        ]);
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 15.0,
            max_x: E + 44.0,
            max_y: N + 27.0,
        },
        24.0,
    );
}

/// What the audit found: five broken objects, one on the locked layer, and the unused.
fn found(app: &mut App) {
    drawn(app);
    run(app, "tool.audit");
    hover(app, [46.0, -14.0]);
}

/// Yazı stilleri and Ölçü stilleri on: the unused styles go too.
fn styles(app: &mut App) {
    found(app);
    typed(app, "Y");
    typed(app, "Ö");
    hover(app, [46.0, -14.0]);
}

/// Enter: the repaired drawing and the message.
fn written(app: &mut App) {
    styles(app);
    typed(app, "");
    hover(app, [46.0, -14.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("denetle-bulunan", found),
        ("denetle-stiller", styles),
        ("denetle-yazildi", written),
    ]
}

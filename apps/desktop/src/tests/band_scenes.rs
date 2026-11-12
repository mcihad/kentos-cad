//! The rows round a part (docs/adr/0578 §1) in pictures and in a test: the table `qgis.gpkg`'s parcels (the first, a
//! hundred metres wide with its hole, and the second ten metres wide a metre or two past the part's east edge), a part
//! holding only the first, survey points named just inside the part's east edge. The second parcel is no object of the
//! layer, but the layer blocks the labels (it is an obstacle layer): with the band read, the names keep off it;
//! without, they lie over it. `tools_screens` takes them in the dark and the light theme:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=bant-cevre,bant-yalniz-parca,bant-park,bant-denetle cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! The file is a copy in a temporary folder. Test code only.

use kentos_commands::CallOp;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};
use crate::tools_screens::Scene;

const AREA: [f64; 4] = [486_990.0, 4_419_990.0, 487_098.0, 4_420_006.0];

fn command(doc: &mut kentos_domain::Document, name: &str, input: Value) -> Value {
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        name,
        CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer["output"].clone()
}

/// The drawing: survey points along the part's east edge, the part of the table bound, its layer an obstacle layer.
pub(crate) fn world(tag: &str) -> Document {
    let file = scratch(tag).join("qgis.gpkg");
    std::fs::copy(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/geopackage/v1/qgis.gpkg"),
        &file,
    )
    .expect("a copy of the file");
    let style = json!({ "color": "fg", "lineType": "continuous", "lineWeight": 0.25 });
    let mut nokta = style.clone();
    nokta["label"] = json!({ "placement": "beside", "size": 16, "point": "around" });
    nokta["point"] = json!({ "symbol": "ring", "size": 6 });
    // Where the second parcel lies, drawn as a plain layer of its own so a picture shows it: in the drawing it is a row of
    // the table beside the part, not an object, and holds room for the labels without being drawn.
    let neighbour = json!({ "kind": "polygon", "id": 20, "layerId": "komsu", "attrs": {},
        "pts": [{ "x": 487_100.0, "y": 4_420_000.0 }, { "x": 487_110.0, "y": 4_420_000.0 },
                { "x": 487_110.0, "y": 4_420_010.0 }, { "x": 487_100.0, "y": 4_420_010.0 }] });
    let mut points: Vec<Value> = (0..5)
        .map(|i| {
            json!({ "kind": "point", "id": 1 + i, "layerId": "nokta", "attrs": {},
                    "label": format!("P.{}", 101 + i),
                    "p": { "x": 487_097.5, "y": 4_419_993.0 + f64::from(i) * 3.0 } })
        })
        .collect();
    points.push(neighbour);
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Parça",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 487_000.0, "y": 4_420_000.0 },
        "layers": [{ "id": "nokta", "name": "Nokta", "type": "layer", "visible": true, "locked": false,
                     "expanded": true, "style": nokta, "children": [] },
                   { "id": "komsu", "name": "Komşu satır", "type": "layer", "visible": true, "locked": false,
                     "expanded": true, "style": style, "children": [] }],
        "activeLayer": "nokta", "entities": points, "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    let mut d = Document::new(snapshot, None).expect("opens");
    command(
        &mut d.model,
        "cad.sources.edit",
        json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": "parsel", "extent": AREA } }),
    );
    command(
        &mut d.model,
        "cad.layers.labels",
        json!({ "layer": "import-parsel", "labels": { "mode": "off", "obstacle": { "weight": 8 } } }),
    );
    d
}

/// The drawing open, the view on the part and its neighbours, the band read.
pub(crate) fn open(app: &mut App, tag: &str) {
    // The band is asked for after the message that opened the drawing and arrives on a thread.
    let task = app.update(Message::Opened(Some(Ok(Box::new(world(tag))))));
    drive(app, task);
    app.tab = "map";
    app.command_expanded = false;
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
        iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(900.0, 560.0)),
    )));
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: 487_060.0,
            min_y: 4_419_978.0,
            max_x: 487_130.0,
            max_y: 4_420_022.0,
        },
        24.0,
    );
    let task = app.update(Message::DialogClosed);
    drive(app, task);
}

/// Where each labelled point's name lies: its middle, east and north of the point, in centimetres, by the point's north.
pub(crate) fn sides(app: &App) -> Vec<(i64, i64)> {
    use kentos_geometry_core::vec2::Vec2;
    use kentos_interaction::LabelSpot;
    let spots = app.spatial.labels(
        Vec2::new(487_060.0, 4_419_978.0),
        Vec2::new(487_130.0, 4_420_022.0),
        app.viewport.camera.scale,
        Default::default(),
    );
    let doc = app.document.as_ref().expect("a drawing");
    let mut out: Vec<(f64, (i64, i64))> = Vec::new();
    for spot in spots {
        if let LabelSpot::Placed { slot, at, .. } = spot
            && let Some(e) = doc.model.get(slot)
            && e.base().layer_id == "nokta"
            && let kentos_commands::Entity::Point(p) = e
        {
            out.push((
                p.p.y,
                (
                    ((at.x - p.p.x) * 100.0).round() as i64,
                    ((at.y - p.p.y) * 100.0).round() as i64,
                ),
            ));
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out.into_iter().map(|(_, d)| d).collect()
}

/// Etiketler over the bound layer, Denetle pressed: what the band and the parked pins mean for its labels.
fn report(app: &mut App) {
    app.open_labelling(Some("import-parsel".to_owned()));
    let _ = app.update(Message::Labelling(crate::labelling::Event::Report));
}

/// A pin on the part's parcel, then a part that does not hold it any more: its pins are parked in the binding.
pub(crate) fn park(app: &mut App, tag: &str) {
    open(app, tag);
    let doc = app.document.as_mut().expect("a drawing");
    let uid = doc
        .model
        .by_layer_with_uids("import-parsel")
        .next()
        .map(|(u, _)| u.to_string())
        .expect("the parcel");
    command(
        &mut doc.model,
        "cad.labels.pin",
        json!({ "pins": [{ "uid": uid, "pin": { "at": { "x": 3.0, "y": -4.0 } } }] }),
    );
    command(
        &mut doc.model,
        "cad.sources.edit",
        json!({ "edit": { "kind": "extent", "id": "kaynak-1", "extent": [487_090.0, 4_419_990.0, 487_098.0, 4_420_006.0] } }),
    );
    let task = app.update(Message::DialogClosed);
    drive(app, task);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        // The band read: the names keep off parcels B and C beside the part (they are not objects of the layer).
        ("bant-cevre", |app| open(app, "bant-cevre")),
        // Denetle over the layer: the row beside the part holds room, no pin parked.
        ("bant-denetle", |app| {
            open(app, "bant-denetle");
            report(app);
        }),
        // A part that does not hold the pinned parcel any more: the pin is parked, Denetle says so.
        ("bant-park", |app| {
            park(app, "bant-park");
            report(app);
        }),
        // The part alone, as before: the names lie over the neighbours.
        ("bant-yalniz-parca", |app| {
            open(app, "bant-yalniz-parca");
            app.bands.forget();
            app.spatial.clear_all_ghosts();
            let _ = app.update(Message::DialogClosed);
        }),
    ]
}

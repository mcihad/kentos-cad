//! Hidroloji in the desktop's window (docs/adr/0235) over the valley with
//! a road and two outlets (fixtures/interaction/v1/hydrology.kcad): Dere ağı
//! writes its links with their orders on a new layer right above the
//! elevation model's, undone whole; Akış birikimi writes its file and puts
//! its raster right above the model; Noktadan havza moves the outlets onto
//! their streams and writes their watersheds; Havzalar finds the streams
//! the road crosses.

use kentos_contracts::{Entity, RasterSample};
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::hydrology_scenes::open_hydrology;

fn value(app: &mut App, name: &str, v: Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

fn run(app: &mut App) -> RunStatus {
    super::surface_tests::run(app)
}

fn valley() -> App {
    let (mut app, _) = App::boot(None);
    open_hydrology(&mut app);
    app
}

fn layer_named(app: &App, name: &str) -> (String, usize) {
    let layers = app.document.as_ref().expect("open").model.layers();
    let id = layers
        .leaves()
        .into_iter()
        .find(|l| l.name == name)
        .map(|l| l.id.clone())
        .unwrap_or_else(|| panic!("{name}"));
    let (_, at) = layers.place_of(&id).expect("placed");
    (id, at)
}

fn open(app: &mut App, tool: &'static str) {
    let _ = app.update(Message::Run(tool));
    value(app, "input", json!({ "scope": "layer", "layerId": "dem" }));
}

#[test]
fn the_streams_go_right_above_the_model_in_one_step() {
    let mut app = valley();
    open(&mut app, "processing.run.hydrology.streams");
    value(&mut app, "threshold", json!(2000));
    let s = run(&mut app);
    let RunStatus::Ok { text, undo, .. } = &s else {
        panic!("{s:?}");
    };
    assert!(
        text.contains(" kol yazıldı; en büyük Strahler sırası "),
        "{text}"
    );
    assert!(text.ends_with(", eşik 2000 m²."), "{text}");
    assert!(undo);
    let (made, at) = layer_named(&app, "Dere ağı");
    let (_, model) = layer_named(&app, "Yükseklik modeli");
    assert_eq!(at + 1, model);
    let doc = &mut app.document.as_mut().expect("open").model;
    let links: Vec<_> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Polyline(p) if p.base.layer_id == made => Some(p),
            _ => None,
        })
        .collect();
    assert!(links.len() > 20, "{}", links.len());
    // Every link numbered in order, its orders, its length, drop and slope, the area at its end.
    for (k, p) in links.iter().enumerate() {
        let a = &p.base.attrs;
        assert_eq!(
            a.get("Bağ").map(String::as_str),
            Some((k + 1).to_string().as_str())
        );
        for name in ["Sıra", "Shreve", "Uzunluk", "Düşü", "Eğim", "Alan", "Aşağı"] {
            assert!(a.contains_key(name), "{name}: {a:?}");
        }
        let area: f64 = a["Alan"].parse().expect("a number");
        assert!(area >= 2000.0, "{area}");
    }
    assert_eq!(doc.undo().as_deref(), Some("Dere ağı"));
    assert!(doc.layers().leaves().iter().all(|l| l.name != "Dere ağı"));
}

#[test]
fn the_accumulation_is_written_beside_the_model() {
    let dir = crate::files_testing::scratch("hid-birikim");
    let mut app = valley();
    open(&mut app, "processing.run.hydrology.flowAccumulation");
    value(&mut app, "unit", json!("area"));
    let out = dir.join("birikim.tif");
    value(&mut app, "output", json!(out.to_string_lossy()));
    let s = run(&mut app);
    let RunStatus::Ok { text, .. } = &s else {
        panic!("{s:?}");
    };
    assert!(text.starts_with("480 × 324 hücrelik raster; “"), "{text}");
    assert!(text.contains("En büyük birikim "), "{text}");
    let bytes = std::fs::read(&out).expect("the result");
    let reader = kentos_formats::raster::source::open_bytes(&bytes, None, 1 << 26).expect("reads");
    assert_eq!(reader.info.sample, RasterSample::F32);
    let (made, at) = layer_named(&app, "Akış birikimi");
    let (_, model) = layer_named(&app, "Yükseklik modeli");
    assert_eq!(at + 1, model);
    let doc = &app.document.as_ref().expect("open").model;
    assert!(doc.entities().any(|e| matches!(e, Entity::Raster(r)
        if r.base.layer_id == made && r.raster.style.ramp.as_deref() == Some("Viridis"))));
}

#[test]
fn the_outlets_watersheds_and_the_roads_crossings() {
    let mut app = valley();
    open(&mut app, "processing.run.hydrology.watershed");
    value(
        &mut app,
        "points",
        json!({ "scope": "layer", "layerId": "cikis" }),
    );
    value(&mut app, "snap", json!(10));
    let s = run(&mut app);
    let RunStatus::Ok { text, .. } = &s else {
        panic!("{s:?}");
    };
    assert_eq!(text, "2 havza yazıldı.");
    let (made, _) = layer_named(&app, "Havzalar");
    let doc = &app.document.as_ref().expect("open").model;
    let basins: Vec<_> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Polygon(p) if p.base.layer_id == made => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(basins.len(), 2);
    for (k, p) in basins.iter().enumerate() {
        assert_eq!(
            p.base.attrs.get("Havza").map(String::as_str),
            Some((k + 1).to_string().as_str())
        );
        let area: f64 = p.base.attrs["Alan"].parse().expect("a number");
        // A watershed is more than the outlet's cell and less than the model.
        assert!(area > 1000.0 && area < 768.0 * 518.4, "{area}");
    }
    let _ = app.update(Message::Processing(Event::Close));
    // The road crosses the eastern streams: a basin each, by km on the road.
    open(&mut app, "processing.run.hydrology.basins");
    value(&mut app, "mode", json!("route"));
    value(&mut app, "threshold", json!(2000));
    value(
        &mut app,
        "routes",
        json!({ "scope": "layer", "layerId": "yol" }),
    );
    let s = run(&mut app);
    let RunStatus::Ok { text, .. } = &s else {
        panic!("{s:?}");
    };
    assert!(text.ends_with(" havza yazıldı."), "{text}");
    let doc = &app.document.as_ref().expect("open").model;
    let km: Vec<f64> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Polygon(p) if p.base.attrs.contains_key("Km") => {
                p.base.attrs["Km"].parse().ok()
            }
            _ => None,
        })
        .collect();
    assert!(km.len() >= 3, "{km:?}");
    assert!(km.windows(2).all(|w| w[0] <= w[1]), "{km:?}");
}

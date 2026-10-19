//! Doğrusal referans in the desktop's window (docs/adr/0224 §5): every
//! shared case run from the window as a user fills it, on the shared drawing
//! (fixtures/processing/v1/lrs.kcad; an event table chosen as the dialog
//! reads it), and the pictures for the owner on the pictures' road
//! (fixtures/interaction/v1/lrs.kcad and lrs/, scripts/fixtures/lrs_scene.py),
//! the routes' measures in Öznitelikler, Koordinat listesi and the rollover
//! card among them.

use std::path::PathBuf;

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Slot;
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const DRAWING: &str = include_str!("../../../../fixtures/processing/v1/lrs.kcad");
const CASES: &str = include_str!("../../../../fixtures/processing/v1/lrs.json");
/// The event tables the cases choose, by name.
const FILES: [(&str, &[u8]); 3] = [
    (
        "lrs-noktalar.csv",
        include_bytes!("../../../../fixtures/processing/v1/lrs-noktalar.csv"),
    ),
    (
        "lrs-cizgiler.csv",
        include_bytes!("../../../../fixtures/processing/v1/lrs-cizgiler.csv"),
    ),
    (
        "lrs-hizlar.csv",
        include_bytes!("../../../../fixtures/processing/v1/lrs-hizlar.csv"),
    ),
];
/// What a value's field depends on comes first: the objects, then where the rows come from, then the files.
const FIRST: [&str; 12] = [
    "input",
    "routes",
    "points",
    "sourceKind",
    "aSourceKind",
    "bSourceKind",
    "file",
    "aFile",
    "bFile",
    "source",
    "aSource",
    "bSource",
];
/// The pictures' road: D-100 and D-200 as measured routes, their pieces, km stones, accidents.
const SCENE: &str = include_str!("../../../../fixtures/interaction/v1/lrs.kcad");
/// Its event tables, by name.
const SCENE_FILES: [(&str, &[u8]); 4] = [
    (
        "kaplama.csv",
        include_bytes!("../../../../fixtures/interaction/v1/lrs/kaplama.csv"),
    ),
    (
        "hiz.csv",
        include_bytes!("../../../../fixtures/interaction/v1/lrs/hiz.csv"),
    ),
    (
        "korkuluk.csv",
        include_bytes!("../../../../fixtures/interaction/v1/lrs/korkuluk.csv"),
    ),
    (
        "levhalar.csv",
        include_bytes!("../../../../fixtures/interaction/v1/lrs/levhalar.csv"),
    ),
];
/// The pictures' D-100: four vertices, a curve between the second and the third, M 12+000 to 13+700.
const D100: Slot = Slot(1);
/// The roads with the accidents beside them.
const ROAD: [f64; 4] = [486_080.0, 4_419_830.0, 487_730.0, 4_420_500.0];

/// The app with the drawing open.
fn app_with_drawing() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("the drawing opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Processing(e));
}

fn status(app: &App) -> RunStatus {
    app.processing
        .dialog
        .as_ref()
        .map_or(RunStatus::Idle, |w| w.status.clone())
}

fn cases() -> Vec<Value> {
    let file: Value = serde_json::from_str(CASES).expect("the cases read");
    file["cases"].as_array().expect("cases").clone()
}

/// A case's tool opened and filled as a user does.
fn fill(app: &mut App, c: &Value) {
    let tool = c["run"]["tool"].as_str().expect("a tool");
    open(app, tool, &c["values"], &FILES);
}

/// `tool`'s window filled with `values` as a user does: the objects first,
/// then where the rows come from, a table file (one of `files`) chosen as
/// the dialog reads it, then the rest.
fn open(app: &mut App, tool: &str, values: &Value, files: &[(&str, &[u8])]) {
    let command: &'static str = Box::leak(format!("processing.run.{tool}").into_boxed_str());
    let _ = app.update(Message::Run(command));
    let values = values.as_object().expect("values");
    let rest = values.keys().filter(|k| !FIRST.contains(&k.as_str()));
    for name in FIRST.iter().copied().chain(rest.map(String::as_str)) {
        let Some(v) = values.get(name) else {
            continue;
        };
        match (name.ends_with("ile"), v.as_str()) {
            (true, Some(file)) => {
                let bytes = files
                    .iter()
                    .find(|(n, _)| *n == file)
                    .map(|(_, b)| b.to_vec())
                    .expect("a table's file");
                let path = PathBuf::from("/veri").join(file);
                event(
                    app,
                    Event::FileChosen(name.into(), Some((file.to_owned(), path, bytes))),
                );
            }
            _ => event(app, Event::Value(name.into(), v.clone())),
        }
    }
}

#[test]
fn every_shared_case_runs_from_the_window_as_it_says() {
    let mut seen = 0;
    for c in cases() {
        let mut app = app_with_drawing();
        fill(&mut app, &c);
        event(&mut app, Event::Run);
        let want = &c["expect"];
        let id = c["id"].as_str().unwrap_or("?");
        match (want["status"].as_str(), status(&app)) {
            (Some("ok"), RunStatus::Ok { text, table, .. }) => {
                assert_eq!(text, want["summary"], "{id}");
                if let Some(rows) = want["outputs"]["table"]["rows"].as_array() {
                    let table = table.expect("the table");
                    assert_eq!(
                        json!(table.columns),
                        want["outputs"]["table"]["columns"],
                        "{id}"
                    );
                    assert_eq!(table.rows.len(), rows.len(), "{id}");
                }
                // The objects the run added are on the output layer, as many as the case says.
                let doc = &app.document.as_ref().expect("open").model;
                if let Some(layer) = want["layers"][0]["id"].as_str() {
                    let on = doc
                        .entities()
                        .filter(|e| e.base().layer_id == layer)
                        .count();
                    assert_eq!(on, want["added"].as_array().map_or(0, Vec::len), "{id}");
                }
                for u in want["updated"].as_array().into_iter().flatten() {
                    let slot = Slot(u["id"].as_u64().expect("an id") as u32);
                    let e = doc.get(slot).expect("the object");
                    if let Some(attrs) = u.get("attrs") {
                        assert_eq!(&json!(e.base().attrs), attrs, "{id}: {slot:?}");
                    }
                }
            }
            (Some("error"), RunStatus::Error(text)) => assert_eq!(text, want["message"], "{id}"),
            (_, got) => panic!("{id}: {got:?}"),
        }
        seen += 1;
    }
    assert!(seen >= 14, "{seen} cases");
}

/// The app with the pictures' road open.
fn app_with_scene() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(SCENE).expect("the drawing reads");
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/interaction/v1/lrs.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("the drawing opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn layer(id: &str) -> Value {
    json!({ "scope": "layer", "layerId": id })
}

/// What the pictures run, on the road's layers and tables: the tool and its values.
fn picture(name: &str) -> (&'static str, Value) {
    match name {
        "create" => (
            "linearRef.createRoutes",
            json!({ "input": layer("yol"), "idField": "Yol", "method": "length" }),
        ),
        "calibrate" => (
            "linearRef.calibrateRoutes",
            json!({
                "routes": layer("rota"), "routeField": "Rota", "points": layer("km"),
                "pointRoute": "Rota", "measure": "Km", "radius": 1
            }),
        ),
        "signs" => (
            "linearRef.routeEvents",
            json!({
                "routes": layer("rota"), "routeField": "Rota", "sourceKind": "file",
                "file": "levhalar.csv", "eventRoute": "Rota", "eventKind": "point",
                "measure": "Km", "offset": "Ofset", "side": "right"
            }),
        ),
        "rails" => (
            "linearRef.routeEvents",
            json!({
                "routes": layer("rota"), "routeField": "Rota", "sourceKind": "file",
                "file": "korkuluk.csv", "eventRoute": "Rota", "eventKind": "line",
                "from": "Bas", "to": "Son", "offset": "Ofset", "side": "right"
            }),
        ),
        "locate" => (
            "linearRef.locateAlong",
            json!({
                "input": layer("kaza"), "routes": layer("rota"), "routeField": "Rota",
                "radius": 20, "all": false, "side": "right", "write": false
            }),
        ),
        "overlay" => (
            "linearRef.overlayEvents",
            json!({
                "aSourceKind": "file", "aFile": "kaplama.csv", "aRoute": "Rota", "aFrom": "Bas",
                "aTo": "Son", "bSourceKind": "file", "bFile": "hiz.csv", "bRoute": "Rota",
                "bFrom": "Bas", "bTo": "Son", "how": "union"
            }),
        ),
        "drift" | "drift-fix" => (
            "linearRef.eventDrift",
            json!({
                "events": layer("islem-olaylar"), "routes": layer("rota"), "routeField": "Rota",
                "eventRoute": "Rota", "eventKind": "point", "measure": "Km", "offset": "Ofset",
                "side": "right", "tolerance": 0.01, "fix": name == "drift-fix"
            }),
        ),
        _ => (
            "linearRef.extractMeasures",
            json!({ "input": layer("rota"), "prefix": "M_" }),
        ),
    }
}

/// D-100 corrected after its events were written: its last vertex 40 m north, its measures as they were.
fn corrected(app: &mut App) {
    let doc = &mut app.document.as_mut().expect("open").model;
    let mut e = doc.get(D100).expect("D-100").clone();
    if let kentos_contracts::Entity::Polyline(path) = &mut e
        && let Some(last) = path.pts.last_mut()
    {
        last.y += 40.0;
    }
    doc.update_many(vec![(D100, e)], "Köşeyi taşı");
}

/// Olay kaymasını denetle (docs/adr/0526) on the shared case's drawing: the drifted sign reported, a fixed line
/// event moved onto its route and a line that cannot take a bent path left where it is.
#[test]
fn drifted_events_are_found_and_moved() {
    let c = cases()
        .into_iter()
        .find(|c| c["id"] == "drift-lines-fix")
        .expect("the case");
    let mut app = app_with_drawing();
    fill(&mut app, &c);
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, table, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, c["expect"]["summary"]);
    let rows = table.expect("the table").rows;
    assert_eq!(rows.len(), 4);
    let doc = &app.document.as_ref().expect("open").model;
    let Some(kentos_contracts::Entity::Line(l)) = doc.get(Slot(78)) else {
        panic!("the line");
    };
    assert_eq!((l.a.x, l.b.x), (488_300.0, 488_300.0));
    let Some(kentos_contracts::Entity::Line(l)) = doc.get(Slot(79)) else {
        panic!("the bent one");
    };
    assert_eq!((l.b.x, l.b.y), (488_100.0, 4_421_020.0), "left where it is");
    let _ = app.update(Message::Run("edit.undo"));
    let doc = &app.document.as_ref().expect("open").model;
    let Some(kentos_contracts::Entity::Line(l)) = doc.get(Slot(78)) else {
        panic!("the line");
    };
    assert_eq!(l.a.x, 488_301.0, "one undo step");
}

/// Doğrusal referans for the owner, on the pictures' road: CBS's Analiz tab
/// with Ağ analizi's ▾ open on the six, each tool's window and what it wrote
/// (the routes made from the road pieces with their measures in
/// Öznitelikler, the guardrails and the signs beside the routes), the
/// tables of Rota boyunca konumla and Olayları çakıştır, D-100's measures
/// in Öznitelikler, in Koordinat listesi's M column and on the rollover
/// card. Not run by default:
/// `cargo test -p kentos-desktop processing::lrs_tests::screens -- --ignored --nocapture --test-threads=1`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = [
        "serit",
        "olustur",
        "olustur-cizim",
        "kalibre",
        "olay",
        "olay-cizim",
        "konum",
        "cakistir",
        "m-degerleri",
        "oznitelik",
        "koordinat",
        "kart",
        "kayma",
        "kayma-tasindi",
    ];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in views {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| x == name))
                {
                    continue;
                }
                let mut app = app_with_scene();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                // The roads in view.
                let on_road = |app: &mut App| {
                    app.viewport.camera.fit(
                        &kentos_render_wgpu::Bounds {
                            min_x: ROAD[0],
                            min_y: ROAD[1],
                            max_x: ROAD[2],
                            max_y: ROAD[3],
                        },
                        24.0,
                    );
                };
                let opened = |app: &mut App, what: &str| {
                    let (tool, values) = picture(what);
                    open(app, tool, &values, &SCENE_FILES);
                };
                // A tool run in its window, the form scrolled to its end: the summary and the table under it.
                let ran = |s: &mut Snapshot, app: &mut App, what: &str| {
                    opened(app, what);
                    event(app, Event::Run);
                    let mut update = |app: &mut App, message| {
                        let _ = app.update(message);
                    };
                    s.settle(app, App::view, &mut update);
                    s.operate(app.view(), Box::new(crate::files_testing::SnapAll));
                };
                // Tools run and their windows closed: what they wrote on the drawing.
                let drawn = |app: &mut App, whats: &[&str]| {
                    for what in whats {
                        opened(app, what);
                        event(app, Event::Run);
                        event(app, Event::Close);
                    }
                };
                match name {
                    // CBS's Analiz tab, Ağ analizi's ▾ open on Doğrusal referans's six.
                    "serit" => {
                        let _ = app.update(Message::RibbonTab("analysis"));
                        snapshot.settle(&mut app, App::view, &mut update);
                        use iced::futures::StreamExt as _;
                        let task: iced::Task<Message> = kentos_ui::widget::context_menu::open_menu(
                            crate::ribbon_keys::more_id("Ağ analizi"),
                        );
                        if let Some(mut stream) = iced_runtime::task::into_stream(task) {
                            while let Some(action) =
                                iced::futures::executor::block_on(stream.next())
                            {
                                if let iced_runtime::Action::Widget(operation) = action {
                                    snapshot.operate(app.view(), operation);
                                }
                            }
                        }
                    }
                    "olustur" => {
                        on_road(&mut app);
                        opened(&mut app, "create");
                    }
                    // The routes made: Ölçülü rotalar hidden, the new D-100 selected, its measures in Öznitelikler.
                    "olustur-cizim" => {
                        drawn(&mut app, &["create"]);
                        let _ = app.update(Message::LayerVisible("rota".into()));
                        on_road(&mut app);
                        let made = app.document.as_ref().and_then(|d| {
                            d.model
                                .entities()
                                .find(|e| {
                                    e.base().layer_id == "islem-rotalar"
                                        && e.base().attrs.get("Yol").map(String::as_str)
                                            == Some("D-100")
                                })
                                .map(|e| Slot(e.base().id))
                        });
                        crate::elevation_scenes::select(
                            &mut app,
                            &made.into_iter().collect::<Vec<_>>(),
                        );
                        crate::elevation_scenes::panel_room(&mut app);
                    }
                    "kalibre" => {
                        on_road(&mut app);
                        ran(&mut snapshot, &mut app, "calibrate");
                    }
                    "olay" => {
                        on_road(&mut app);
                        ran(&mut snapshot, &mut app, "rails");
                    }
                    "olay-cizim" => {
                        drawn(&mut app, &["rails", "signs"]);
                        on_road(&mut app);
                    }
                    "konum" => {
                        on_road(&mut app);
                        ran(&mut snapshot, &mut app, "locate");
                    }
                    "cakistir" => {
                        on_road(&mut app);
                        ran(&mut snapshot, &mut app, "overlay");
                    }
                    "m-degerleri" => {
                        on_road(&mut app);
                        ran(&mut snapshot, &mut app, "extract");
                    }
                    // The signs written, then D-100's last vertex moved 40 m north (a corrected axis): Olay
                    // kaymasını denetle finds the signs that no longer sit where their Km puts them.
                    "kayma" | "kayma-tasindi" => {
                        drawn(&mut app, &["signs"]);
                        corrected(&mut app);
                        on_road(&mut app);
                        if name == "kayma" {
                            ran(&mut snapshot, &mut app, "drift");
                        } else {
                            drawn(&mut app, &["drift-fix"]);
                        }
                    }
                    // D-100 selected: M (başlangıç) and M (bitiş) among its geometry's rows.
                    "oznitelik" => {
                        on_road(&mut app);
                        crate::elevation_scenes::select(&mut app, &[D100]);
                        crate::elevation_scenes::panel_room(&mut app);
                    }
                    // D-100 in Koordinat listesi: its M column.
                    "koordinat" => {
                        on_road(&mut app);
                        app.selection.set([D100]);
                        let _ = app.update(Message::Run("view.coords"));
                        let _ = app.update(Message::BottomResized(300.0));
                    }
                    // The pointer resting on D-100's first straight: the card's M.
                    _ => {
                        on_road(&mut app);
                        snapshot.settle(&mut app, App::view, &mut update);
                        crate::elevation_scenes::rest(&mut app, [-600.0, -140.0]);
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("islem-dr-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

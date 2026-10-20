//! Tablodan nokta and the tables' files in the desktop (docs/adr/0223), on
//! the shared drawing (fixtures/processing/v1/table-points.kcad): every
//! shared case run from the window as a user fills it (the file chosen as
//! the dialog reads it, a workbook on its first sheet), and the pictures for
//! the owner.

use std::path::PathBuf;

use kentos_commands::DocumentSnapshotV1;
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const DRAWING: &str = include_str!("../../../../fixtures/processing/v1/table-points.kcad");
const CASES: &str = include_str!("../../../../fixtures/processing/v1/table-points.json");
const PLANE: &[u8] = include_bytes!("../../../../fixtures/processing/v1/nokta-duzlem.csv");
const WGS84: &[u8] = include_bytes!("../../../../fixtures/processing/v1/nokta-wgs84.ods");

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

fn case(id: &str) -> Value {
    cases()
        .into_iter()
        .find(|c| c["id"] == id)
        .expect("the case")
}

/// A case's tool opened and filled as a user does: the kind of source before
/// the file, a file chosen as the dialog reads it.
fn fill(app: &mut App, c: &Value) {
    let tool = c["run"]["tool"].as_str().expect("a tool");
    let command: &'static str = Box::leak(format!("processing.run.{tool}").into_boxed_str());
    let _ = app.update(Message::Run(command));
    let values = c["values"].as_object().expect("values");
    if let Some(kind) = values.get("sourceKind") {
        event(app, Event::Value("sourceKind".into(), kind.clone()));
    }
    for (name, v) in values {
        if name == "sourceKind" {
            continue;
        }
        if name == "file" {
            let file = v.as_str().expect("a file's name");
            let bytes = if file.ends_with(".ods") { WGS84 } else { PLANE };
            let path = PathBuf::from("/veri").join(file);
            event(
                app,
                Event::FileChosen(name.clone(), Some((file.to_owned(), path, bytes.to_vec()))),
            );
        } else {
            event(app, Event::Value(name.clone(), v.clone()));
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
        match status(&app) {
            RunStatus::Ok { text, table, .. } => {
                assert_eq!(text, want["summary"], "{id}");
                if let Some(t) = want["outputs"].get("table") {
                    let table = table.expect("the report");
                    assert_eq!(
                        json!({ "columns": table.columns, "rows": table.rows }),
                        *t,
                        "{id}"
                    );
                }
            }
            RunStatus::Error(text) => assert_eq!(text, want["message"], "{id}"),
            other => panic!("{id}: {other:?}"),
        }
        // The objects the run added are on the output layer, as many as the case says.
        let doc = &app.document.as_ref().expect("open").model;
        let added = want["added"].as_array().map_or(0, Vec::len);
        if let Some(layer) = want["layers"][0]["id"].as_str() {
            let on = doc
                .entities()
                .filter(|e| e.base().layer_id == layer)
                .count();
            assert_eq!(on, added, "{id}");
        }
        seen += 1;
    }
    assert!(seen >= 4, "{seen} cases");
}

/// A workbook's other sheet chosen in the window is the one read: the ODS
/// file's Notlar has no Boylam column, so the run says so.
#[test]
fn the_sheet_chosen_is_the_one_read() {
    let mut app = app_with_drawing();
    fill(&mut app, &case("wgs84-ods"));
    event(&mut app, Event::Sheet("file".into(), "Notlar".into()));
    event(&mut app, Event::Run);
    let RunStatus::Error(text) = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert!(text.starts_with("Kaynakta “Boylam” alanı yok"), "{text}");
    event(&mut app, Event::Sheet("file".into(), "Noktalar".into()));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, "7 satırdan 4 nokta yazıldı.");
}

/// Where a caption is seen: its bounds less the translation of the
/// scrollable it is found in (operations see a scrolled body unscrolled).
fn seen_at(
    snapshot: &mut kentos_ui::snapshot::Snapshot,
    app: &App,
    caption: &str,
) -> Vec<iced::Rectangle> {
    use std::sync::{Arc, Mutex};

    use iced::advanced::widget::operation::Scrollable;
    use iced::advanced::widget::{Id, Operation};

    struct Find {
        caption: String,
        shift: iced::Vector,
        found: Arc<Mutex<Vec<iced::Rectangle>>>,
    }
    impl Operation for Find {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn scrollable(
            &mut self,
            _id: Option<&Id>,
            _bounds: iced::Rectangle,
            _content_bounds: iced::Rectangle,
            translation: iced::Vector,
            _state: &mut dyn Scrollable,
        ) {
            self.shift = translation;
        }
        fn text(&mut self, _id: Option<&Id>, bounds: iced::Rectangle, text: &str) {
            if text == self.caption
                && let Ok(mut found) = self.found.lock()
            {
                found.push(bounds - self.shift);
            }
        }
    }
    let found = Arc::new(Mutex::new(Vec::new()));
    snapshot.operate(
        app.view(),
        Box::new(Find {
            caption: caption.to_owned(),
            shift: iced::Vector::ZERO,
            found: found.clone(),
        }),
    );
    found.lock().map(|f| f.clone()).unwrap_or_default()
}

/// GIS-27 for the owner: CBS's Veri tab with Toplu çevir ▾ open on
/// Tablodan nokta, its window on the ODS file (Sayfa, its list open, the
/// report and Kaydet ▾), the points drawn, Öznitelik tablosu's Dışa aktar ▾
/// on them and a workbook in Koordinat listesi içe aktar. Not run by default:
/// `cargo test -p kentos-desktop processing::table_points_tests::screens -- --ignored --nocapture --test-threads=1`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size, mouse};
    use kentos_ui::snapshot::Snapshot;

    use crate::files_testing::drive;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = [
        "serit",
        "nokta",
        "sayfa",
        "kaydet",
        "nokta-cizim",
        "disa-aktar",
        "koordinat",
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
                let mut app = app_with_drawing();
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
                // A click on the last place a caption is drawn: the window's, over the rest.
                let click = |snapshot: &mut Snapshot, app: &mut App, caption: &str| {
                    let mut update = |app: &mut App, message| {
                        let _ = app.update(message);
                    };
                    snapshot.settle(app, App::view, &mut update);
                    let at = seen_at(snapshot, app, caption)
                        .pop()
                        .unwrap_or_else(|| panic!("“{caption}” on screen"));
                    let at = Point::new(at.x + at.width / 2.0, at.y + at.height / 2.0);
                    snapshot.step(
                        app,
                        App::view,
                        &mut update,
                        &[
                            iced::Event::Mouse(mouse::Event::CursorMoved { position: at }),
                            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                        ],
                    );
                };
                // Both files' points drawn, the window closed, the view on them (A4, 3.7 km south, aside).
                let drawn = |app: &mut App| {
                    for id in ["plane-csv", "wgs84-ods"] {
                        fill(app, &case(id));
                        event(app, Event::Run);
                        event(app, Event::Close);
                    }
                    let doc = &app.document.as_ref().expect("open").model;
                    let near: Vec<kentos_domain::Slot> = doc
                        .entities()
                        .filter(|e| {
                            e.base().layer_id != "olcum"
                                && e.base().attrs.get("Ad").is_none_or(|a| a != "A4")
                        })
                        .map(|e| kentos_domain::Slot(e.base().id))
                        .collect();
                    app.selection.set(near);
                    let _ = app.update(Message::Run("view.zoomSelection"));
                    app.selection.set(std::iter::empty());
                };
                match name {
                    // CBS's Veri tab, Toplu çevir's ▾ open: Tablodan nokta after the other tables' tools.
                    "serit" => {
                        let _ = app.update(Message::RibbonTab("data"));
                        click(&mut snapshot, &mut app, "Toplu çevir");
                    }
                    // The run's report under the settings: the skipped rows and their reasons.
                    "nokta" => {
                        fill(&mut app, &case("wgs84-ods"));
                        event(&mut app, Event::Run);
                        snapshot.settle(&mut app, App::view, &mut update);
                        snapshot.operate(app.view(), Box::new(crate::files_testing::SnapDown));
                    }
                    // The workbook's sheets listed.
                    "sayfa" => {
                        fill(&mut app, &case("wgs84-ods"));
                        click(&mut snapshot, &mut app, "Noktalar");
                    }
                    "kaydet" => {
                        fill(&mut app, &case("wgs84-ods"));
                        event(&mut app, Event::Run);
                        snapshot.settle(&mut app, App::view, &mut update);
                        snapshot.operate(app.view(), Box::new(crate::files_testing::SnapDown));
                        click(&mut snapshot, &mut app, "Kaydet");
                    }
                    "nokta-cizim" => drawn(&mut app),
                    // Öznitelik tablosu on the points, Dışa aktar ▾ open.
                    "disa-aktar" => {
                        drawn(&mut app);
                        let _ = app.update(Message::Run("data.featureTable"));
                        let _ = app.update(Message::Features(crate::features::Event::Layer(
                            "islem-noktalar".into(),
                        )));
                        click(&mut snapshot, &mut app, "Dışa aktar");
                    }
                    // A workbook in Koordinat listesi içe aktar: nothing read, Tablodan nokta named.
                    _ => {
                        app.picker = crate::app::Picker::File(
                            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                .join("../../fixtures/processing/v1/nokta-wgs84.ods"),
                        );
                        let task = app.run("file.import.ncn");
                        drive(&mut app, task);
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("islem-tablo-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

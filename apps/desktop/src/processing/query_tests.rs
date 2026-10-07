//! The query tools in the desktop's window (docs/adr/0200), on the shared
//! query drawing (fixtures/processing/v1/queries.kcad): Özet istatistik's
//! table under the form, a file chosen for Anahtarla birleştir (its columns
//! the fields, its rows not kept in the last values), a value a layer's field
//! does not take, and the pictures for the owner.

use std::path::PathBuf;

use kentos_contracts::DocumentSnapshotV1;
use serde_json::json;

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const QUERIES: &str = include_str!("../../../../fixtures/processing/v1/queries.kcad");
const OWNERS: &[u8] = include_bytes!("../../../../fixtures/processing/v1/malikler.csv");

/// The app with the query drawing open.
fn app_with_queries() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(QUERIES).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("the drawing opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Processing(e));
}

fn value(app: &mut App, name: &str, v: serde_json::Value) {
    event(app, Event::Value(name.into(), v));
}

fn status(app: &App) -> RunStatus {
    app.processing
        .dialog
        .as_ref()
        .map_or(RunStatus::Idle, |w| w.status.clone())
}

fn layer(id: &str) -> serde_json::Value {
    json!({ "scope": "layer", "layerId": id })
}

/// The summary of the buildings' ground areas by storeys (queries.json's `summary-grouped`).
fn summary(app: &mut App) {
    let _ = app.update(Message::Run("processing.run.statistics.summary"));
    value(app, "input", layer("yapi"));
    value(app, "field", json!("Taban alanı"));
    value(app, "group", json!("Kat"));
}

/// Anahtarla birleştir with the owners' CSV chosen (queries.json's `join-file`).
fn join(app: &mut App) {
    let _ = app.update(Message::Run("processing.run.attributes.joinByField"));
    value(app, "target", layer("parsel"));
    value(app, "targetKey", json!("Parsel"));
    value(app, "sourceKind", json!("file"));
    let path = PathBuf::from("/veri/malikler.csv");
    event(
        app,
        Event::FileChosen(
            "file".into(),
            Some(("malikler.csv".into(), path, OWNERS.to_vec())),
        ),
    );
    value(app, "prefix", json!("Tapu "));
}

#[test]
fn the_summary_shows_its_table_under_the_form_until_a_value_changes() {
    let mut app = app_with_queries();
    summary(&mut app);
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, table, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, "5 nesnede “Taban alanı”: 4 değer okundu, 3 grup.");
    let table = table.expect("the table");
    assert_eq!(
        table.columns,
        [
            "Grup",
            "Nesne",
            "Değer",
            "Toplam",
            "Ortalama",
            "En az",
            "En çok",
            "Std. sapma"
        ]
    );
    assert_eq!(
        table.rows,
        [
            ["1", "2", "1", "100", "100.000", "100", "100", ""],
            ["2", "2", "2", "95.5", "47.750", "25.5", "70", "31.466"],
            ["(boş)", "1", "1", "36", "36.000", "36", "36", ""],
            [
                "Toplam", "5", "4", "231.5", "57.875", "25.5", "100", "33.903"
            ],
        ]
    );
    assert!(
        !app.document.as_ref().expect("open").model.can_undo(),
        "the drawing does not change"
    );
    // Panoya kopyala says so; the table goes when a value changes.
    event(&mut app, Event::CopyTable);
    assert_eq!(
        crate::files_testing::last_said(&app),
        "Tablo panoya kopyalandı."
    );
    value(&mut app, "group", json!(""));
    assert_eq!(status(&app), RunStatus::Idle);
}

#[test]
fn a_chosen_file_gives_its_columns_as_fields_and_is_kept_by_its_name_and_path() {
    let mut app = app_with_queries();
    join(&mut app);
    let window = app.processing.dialog.as_ref().expect("open");
    let file = window.inputs.get("file").expect("the file's table");
    assert!(file.rows);
    assert_eq!(
        file.fields,
        [
            ("Parsel".to_owned(), 4),
            ("Malik".to_owned(), 4),
            ("Hisse".to_owned(), 4)
        ]
    );
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, "2 nesne eşleşti; 2 nesnede 2 alan aktarıldı.");
    let doc = &app.document.as_ref().expect("open").model;
    let owner = doc
        .get(kentos_domain::Slot(1))
        .and_then(|e| e.base().attrs.get("Tapu Malik").cloned());
    assert_eq!(owner.as_deref(), Some("Ayşe Yılmaz"));
    // The last values keep the file's name and path, not its rows.
    let kept = app
        .processing
        .memory
        .last_values("attributes.joinByField")
        .and_then(|v| v.get("file"))
        .cloned();
    assert_eq!(
        kept,
        Some(json!({ "name": "malikler.csv", "path": "/veri/malikler.csv" }))
    );
    // Opened again, a file that is not where it was must be chosen again.
    event(&mut app, Event::Close);
    let _ = app.update(Message::Run("processing.run.attributes.joinByField"));
    event(&mut app, Event::Run);
    let window = app.processing.dialog.as_ref().expect("open");
    let issue = window.issue_of("file").map(|i| i.message.clone());
    assert_eq!(
        issue.as_deref(),
        Some("“Dosya”: “malikler.csv” dosyasını yeniden seçin; dosyanın içeriği saklanmaz.")
    );
}

#[test]
fn a_file_without_the_key_column_and_a_value_a_field_refuses_end_the_run_unchanged() {
    let mut app = app_with_queries();
    join(&mut app);
    value(&mut app, "sourceKey", json!("Ada"));
    event(&mut app, Event::Run);
    assert_eq!(
        status(&app),
        RunStatus::Error("Kaynakta “Ada” alanı yok; alanları: Parsel, Malik, Hisse.".into())
    );
    event(&mut app, Event::Close);
    let _ = app.update(Message::Run("processing.run.attributes.calculate"));
    value(&mut app, "input", layer("parsel"));
    value(&mut app, "field", json!("Ağaç sayısı"));
    value(&mut app, "value", json!("'çok'"));
    event(&mut app, Event::Run);
    assert_eq!(
        status(&app),
        RunStatus::Error(
            "Öznitelik yazılamadı (#1): “Ağaç sayısı” alanı tam sayı ister; “çok” verildi. Rakamlarla, ondalıksız yazın."
                .into()
        )
    );
    assert!(!app.document.as_ref().expect("open").model.can_undo());
}

/// Where a text is drawn on the screen inside a scrolled pane: the text's
/// place less the translation of the last scrollable passed before it (the
/// operation sees a pane's contents unscrolled).
fn scrolled_text(
    snapshot: &mut kentos_ui::snapshot::Snapshot,
    app: &App,
    caption: &str,
) -> Option<iced::Rectangle> {
    use std::sync::{Arc, Mutex};

    use iced::advanced::widget::operation::Scrollable;
    use iced::advanced::widget::{Id, Operation};

    struct Find {
        caption: String,
        shift: iced::Vector,
        found: Arc<Mutex<Option<iced::Rectangle>>>,
    }
    impl Operation for Find {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn scrollable(
            &mut self,
            _id: Option<&Id>,
            _bounds: iced::Rectangle,
            _content: iced::Rectangle,
            translation: iced::Vector,
            _state: &mut dyn Scrollable,
        ) {
            self.shift = translation;
        }
        fn text(&mut self, _id: Option<&Id>, bounds: iced::Rectangle, text: &str) {
            if text == self.caption
                && let Ok(mut found) = self.found.lock()
                && found.is_none()
            {
                *found = Some(bounds - self.shift);
            }
        }
    }
    let found = Arc::new(Mutex::new(None));
    snapshot.operate(
        app.view(),
        Box::new(Find {
            caption: caption.to_owned(),
            shift: iced::Vector::ZERO,
            found: found.clone(),
        }),
    );
    found.lock().ok().and_then(|f| *f)
}

/// The query tools' windows for the owner, after a run: Konuma göre seç,
/// İçindekinden and Çevreleyenden bilgi al, Özet istatistik with its table,
/// Anahtarla birleştir with a file and its fields' list open. Not run by default:
/// `cargo test -p kentos-desktop processing::query_tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = [
        "konum",
        "icindeki",
        "cevreleyen",
        "ozet",
        "birlestir",
        "birlestir-alanlar",
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
                let mut app = app_with_queries();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "konum" => {
                        let _ = app.update(Message::Run("processing.run.selection.byLocation"));
                        value(&mut app, "input", layer("parsel"));
                        value(&mut app, "reference", layer("yol"));
                        event(&mut app, Event::Run);
                    }
                    "icindeki" => {
                        let _ = app.update(Message::Run("processing.run.attributes.fromInside"));
                        value(&mut app, "target", layer("parsel"));
                        value(&mut app, "source", layer("agac"));
                        value(&mut app, "output", json!("Ağaç sayısı"));
                        event(&mut app, Event::Run);
                    }
                    "cevreleyen" => {
                        let _ = app.update(Message::Run("processing.run.attributes.fromEnclosing"));
                        value(&mut app, "target", layer("yapi"));
                        value(&mut app, "source", layer("parsel"));
                        value(&mut app, "field", json!("Parsel"));
                        event(&mut app, Event::Run);
                    }
                    "ozet" => {
                        summary(&mut app);
                        event(&mut app, Event::Run);
                    }
                    _ => {
                        join(&mut app);
                        value(&mut app, "fields", json!("Malik, Hisse"));
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                // The form scrolled to its end: the table, or the file and its fields.
                if matches!(name, "ozet" | "birlestir" | "birlestir-alanlar") {
                    snapshot.step(
                        &mut app,
                        App::view,
                        &mut update,
                        &[
                            iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                                position: iced::Point::new(width / 2.0 - 120.0, height / 2.0),
                            }),
                            iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                                delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y: -40.0 },
                            }),
                        ],
                    );
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                if name == "birlestir-alanlar" {
                    let at = scrolled_text(&mut snapshot, &app, "Malik, Hisse")
                        .expect("the fields' list");
                    // The list's button at the end of the field's row: the window is 940 wide, centred.
                    let open = iced::Point::new((width - 940.0) / 2.0 + 583.5, at.center_y());
                    snapshot.step(
                        &mut app,
                        App::view,
                        &mut update,
                        &[
                            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position: open }),
                            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                                iced::mouse::Button::Left,
                            )),
                            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                                iced::mouse::Button::Left,
                            )),
                        ],
                    );
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("islem-sorgu-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

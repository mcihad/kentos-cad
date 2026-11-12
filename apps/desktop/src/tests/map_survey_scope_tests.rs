//! Each map's own survey quality in the desktop's sheet mode (plan/map-cad.md MAP-18, docs/adr/2800): the drawing and
//! the sheet of fixtures/sheet/v1/survey-scope.json (scripts/fixtures/sheet_survey_scope_cases.py, no KentOS code)
//! opened. The preview's texts say each map's words as the reference does: the texts tied to a map its own points', the
//! untied one the first map's, the map over no point its sentence and the placeholder. And its pictures.

use kentos_sheet_ui::Message as Sheet;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};

fn opened(name: &str) -> (App, Value) {
    let case: Value = serde_json::from_str(include_str!("../../../../fixtures/sheet/v1/survey-scope.json")).expect("the cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["drawing"].clone()).expect("a snapshot");
    let mut model = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let ops: Vec<kentos_sheet::ops::Op> =
        serde_json::from_value(json!([{"op": "addSheet", "sheet": case["sheet"]}])).expect("ops");
    let book = kentos_sheet::ops::apply_all(&kentos_sheet::model::SheetBook::default(), &ops).expect("the sheet").book;
    model.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).expect("kept")).expect("the book");
    let (mut app, _) = App::boot(None);
    app.sheet_store = Some(kentos_sheet_ui::Store::open(&scratch(name)).expect("a store"));
    let doc = Document::from_v2(model.to_snapshot_v2(), None).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    let task = app.update(Message::Sheet(Sheet::Tab(1)));
    drive(&mut app, task);
    (app, case)
}

/// The preview's texts, each text's lines joined.
fn texts(app: &App) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for p in app.sheets.display_list().map(|l| l.prims.as_slice()).unwrap_or_default() {
        if let kentos_sheet::display::Prim::Text(t) = p {
            match out.iter_mut().find(|(id, _)| *id == t.item) {
                Some((_, words)) => {
                    words.push(' ');
                    words.push_str(&t.text);
                }
                None => out.push((t.item.clone(), t.text.clone())),
            }
        }
    }
    out
}

#[test]
fn the_preview_says_each_map_s_own_points() {
    let (app, case) = opened("harita-kalitesi");
    let all = texts(&app);
    for (id, want) in case["texts"].as_object().expect("texts") {
        let got = all.iter().find(|(i, _)| i == id).map_or("", |(_, w)| w.as_str());
        assert_eq!(got, want.as_str().unwrap_or(""), "{id}");
    }
}

/// `.run/gui-evidence/harita-kalitesi-2800/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib sheets::map_survey_scope_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    crate::drawing_fonts::load();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/harita-kalitesi-2800");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let (mut app, _) = opened(&format!("harita-kalitesi-{name}"));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-pafta-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

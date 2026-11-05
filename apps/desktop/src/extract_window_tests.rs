//! Veri çıkarımı (tables/extract.rs, docs/adr/1030 §8): the window says the shared case's recipe, previews the rows
//! `cad.extract.run` gives without writing, keeps a recipe in a file and reads it back, and places the table through
//! `cad.extract.table`; the pictures for the owner (`screens`).

use std::path::PathBuf;

use crate::app::{App, Dialog, Message};
use crate::tables::extract::{Col, Event as X};

const RUN: &str = include_str!("../../../fixtures/commands/v1/cad.extract.run.json");

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/commands/v1/extract")
}

/// The shared cases' open drawing, as if saved beside kat2.kcad and aciklama.csv.
fn extract_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value = serde_json::from_str(RUN).expect("cases");
    let text = cases["setup"].to_string().replace(
        "$fixture:commands/v1/extract",
        &folder().display().to_string(),
    );
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(&text).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, Some(folder().join("kat1.kcad")))
        .expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: X) {
    let _ = app.update(Message::Extract(e));
}

/// The parts list of the shared case: two drawings, nested blocks, external references, a description table.
fn parts(app: &mut App) {
    let _ = app.update(Message::Run("table.extract"));
    send(app, X::Name("Mobilya listesi".into()));
    send(app, X::Drawings("kat2.kcad".into()));
    send(app, X::Xrefs(true));
    send(app, X::Column(Col::Attribute, true));
    send(app, X::Tag("KOD".into()));
    send(app, X::Column(Col::Joined, true));
    send(app, X::JoinPath("aciklama.csv".into()));
    send(app, X::JoinKey("Blok".into()));
    send(app, X::JoinFields("Açıklama, Birim fiyat".into()));
}

#[test]
fn the_window_previews_the_shared_recipe_without_writing_and_places_its_table() {
    let mut app = extract_app();
    let before = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .revision()
        .to_string();
    parts(&mut app);
    assert!(matches!(app.dialog, Some(Dialog::Extract)));
    let x = app.extract.as_ref().expect("open");
    let out = x
        .result
        .as_ref()
        .expect("a preview")
        .as_ref()
        .expect("rows");
    let cases: serde_json::Value = serde_json::from_str(RUN).expect("cases");
    let want = &cases["cases"][0]["steps"][0]["result"]["output"];
    let rows: Vec<Vec<String>> = out.rows.iter().map(|r| r.cells.clone()).collect();
    let wanted: Vec<Vec<String>> = want["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|r| {
            r["cells"]
                .as_array()
                .expect("cells")
                .iter()
                .map(|c| c.as_str().unwrap_or("").to_owned())
                .collect()
        })
        .collect();
    assert_eq!(rows, wanted, "the window's recipe is the shared case's");
    assert_eq!(out.total.as_ref().expect("a total")[4], "31");
    assert_eq!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .revision()
            .to_string(),
        before,
        "a preview writes nothing"
    );
    // A recipe file reads back as the window wrote it.
    let recipe = app.extract.as_ref().expect("open").recipe();
    let text = kentos_formats::extract::recipe_file(&recipe);
    let mut other = crate::tables::extract::Extract::default();
    other.take(&kentos_formats::extract::read_recipe_file(&text).expect("reads"));
    assert_eq!(other.recipe(), recipe);
    // Tabloyu yerleştir: the window closes and the placement runs.
    send(&mut app, X::Depth("1.5".into()));
    send(&mut app, X::Place);
    assert!(app.dialog.is_none());
}

/// The window with the parts list, the placed table broken into parts with its heading again, wide and narrow, light
/// and dark; `.run/gui-evidence/cad-adv-13/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib extract_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/cad-adv-13");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let themed = |theme: &str| {
        let mut app = extract_app();
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        app
    };
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let shot = |snapshot: &mut Snapshot, app: &mut App, file: &str| {
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("cad-adv-13-{file}-{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        // The parts list's window, then its table placed in parts, its heading atop each (one renderer: the drawing's
        // lines are uploaded while the window was shown).
        let mut app = themed(theme);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        parts(&mut app);
        shot(&mut snapshot, &mut app, "pencere");
        send(&mut app, X::Close);
        let doc = app.document.as_mut().expect("a drawing");
        let recipe = app.extract.as_ref().expect("the window's").recipe();
        let input = serde_json::json!({
            "recipe": recipe, "at": {"x": 26.0, "y": 6.0}, "height": 0.35,
            "breaks": {"height": 2.6, "gap": 1.0}, "base": folder().display().to_string(),
        });
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            "cad.extract.table",
            kentos_commands::CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut snapshot, &mut app, "tablo-parcalar");
        // The metraj in a window of its own, its recipe as a recipe file gives it before the window opens: wall
        // lengths, the dimensions' measure apart from the override's words, a weight per metre.
        let mut app = themed(theme);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut metraj = crate::tables::extract::Extract::default();
        metraj.name = "Metraj".into();
        metraj.kinds = vec!["line", "dimension"];
        metraj.layers = "Duvar, Ölçü".into();
        metraj.columns = vec![
            Col::Layer,
            Col::Kind,
            Col::Length,
            Col::Measured,
            Col::Shown,
            Col::Overridden,
            Col::Computed,
        ];
        metraj.factor = "7.5".into();
        metraj.unit = "kg".into();
        metraj.sort = false;
        app.extract = Some(metraj);
        let _ = app.update(Message::Run("table.extract"));
        shot(&mut snapshot, &mut app, "metraj");
    }
}

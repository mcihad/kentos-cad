//! Atlası PDF yap on the desktop (docs/adr/0534): the window previews the command's plan, writes the PDF through
//! `file.export.atlas` with the pin of the preview, refuses a drawing that moved on since and asks for the preview again,
//! and edits the sheet's atlas with `cad.sheets.edit`. The drawing is the one of `fixtures/atlas/v1/cases.json`
//! (written without KentOS code): 105 parcels, five outside the layer's filter, an owner each a form marks `redact`.
//! Pictures: `atlas-*` (`screens`, ignored).

use std::path::PathBuf;

use super::atlas_export::Event as A;
use super::tests::{run, send};
use super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{drive, scratch};
use kentos_commands::DocumentSnapshotV1;
use kentos_native_application::ExecutionContext;
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../fixtures/atlas/v1/cases.json");

/// The app with the atlas drawing and its two sheets (the book goes in as a script puts it: `addSheet`).
fn app_with_atlas() -> App {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    let setup = &file["setups"]["main"];
    let snapshot = DocumentSnapshotV1::from_json(&setup["drawing"].to_string()).expect("reads");
    let mut doc = crate::document::Document::new(snapshot, None).expect("opens");
    for sheet in setup["book"]["sheets"].as_array().expect("sheets") {
        let input =
            serde_json::from_value(json!({ "ops": [{ "op": "addSheet", "sheet": sheet }] }))
                .expect("an input");
        let answer = kentos_native_application::sheets_edit::execute(
            &mut ExecutionContext::new(&mut doc.model),
            input,
        );
        assert!(
            matches!(answer, kentos_commands::CommandResult::Completed { .. }),
            "{answer:?}"
        );
    }
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The app with no parcels of its own: the fixtures' GeoPackage (written by GDAL) bound as two parts, the parcels
/// the sheet's coverage (docs/adr/0638 §3).
fn app_with_source() -> App {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    let setup = &file["setups"]["source"];
    let snapshot = DocumentSnapshotV1::from_json(&setup["drawing"].to_string()).expect("reads");
    let mut doc = crate::document::Document::new(snapshot, None).expect("opens");
    let gpkg = scratch("atlas-kaynak").join("atlas.gpkg");
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/atlas/v1/atlas.gpkg"),
        &gpkg,
    )
    .expect("copied");
    let mut call = |name: &str, input: Value| {
        let answer = kentos_native_application::registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            name,
            kentos_commands::CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        answer
    };
    for sheet in setup["book"]["sheets"].as_array().expect("sheets") {
        call(
            "cad.sheets.edit",
            json!({ "ops": [{ "op": "addSheet", "sheet": sheet }] }),
        );
    }
    let part = [399_999.0, 4_399_999.0, 400_095.0, 4_400_154.0];
    let parsel = call(
        "cad.sources.edit",
        json!({ "edit": { "kind": "link", "path": gpkg, "table": "parsel", "extent": part } }),
    )["output"]["link"]["layer"]
        .clone();
    call(
        "cad.sources.edit",
        json!({ "edit": { "kind": "link", "path": gpkg, "table": "bina", "extent": part } }),
    );
    call(
        "cad.sheets.edit",
        json!({ "ops": [{ "op": "setAtlas", "sheet": "atlas-paftasi", "atlas": {
            "layer": parsel, "filter": "", "sort": [{"expression": "Parsel", "descending": false}],
            "pageName": "Ada || '/' || Parsel" } }] }),
    );
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The pages drawn in another system and the coverage read from its source (docs/adr/0638 §1, §3): the window
/// plans them as the command does.
#[test]
fn the_window_draws_in_another_system_and_reads_the_coverage_from_its_source() {
    let mut app = app_with_atlas();
    run(&mut app, "file.export.atlas");
    let tm30 = window(&app)
        .systems
        .iter()
        .position(|(code, _)| *code == 5254)
        .expect("TM30 offered");
    send(&mut app, Event::Atlas(A::Crs(tm30 + 1)));
    let planned = window(&app).plan.as_ref().expect("planned");
    assert_eq!(planned.bindings.srid, 5254);
    assert_eq!(planned.bindings.project_srid, Some(5253));
    // Degrees are not offered: a map's scale is of lengths.
    assert!(!window(&app).systems.iter().any(|(code, _)| *code == 4326));
    let mut app = app_with_source();
    run(&mut app, "file.export.atlas");
    let from_drawing = window(&app).plan.as_ref().expect("planned").pages.len();
    send(&mut app, Event::Atlas(A::FromSource(true)));
    let planned = window(&app).plan.as_ref().expect("planned");
    assert!(from_drawing < 105, "{from_drawing}: the part's parcels");
    assert_eq!(planned.pages.len(), 105, "the table's");
    assert!(window(&app).from_source);
    // The server reads no file of this computer.
    assert!(App::atlas_server_work(window(&app)).is_none());
}

fn window(app: &App) -> &super::atlas_export::State {
    match &app.exchange {
        Some(Window::AtlasExport(s)) => s,
        _ => panic!("the atlas window"),
    }
}

#[test]
fn the_window_previews_a_hundred_pages_and_writes_them_from_the_pinned_state() {
    let mut app = app_with_atlas();
    run(&mut app, "file.export.atlas");
    let planned = window(&app).plan.as_ref().expect("planned");
    assert_eq!(planned.pages.len(), 100);
    assert_eq!(planned.filtered_out, 5);
    assert_eq!(planned.fields, ["Ada", "Parsel", "Durum"]);
    assert!(planned.withheld.contains(&"Malik".to_owned()));
    let dir = scratch("atlas-disa");
    let path = dir.join("atlas.pdf");
    send(&mut app, Event::Atlas(A::SavePicked(Some(path.clone()))));
    assert!(app.exchange.is_none(), "written and closed");
    let bytes = std::fs::read(&path).expect("written");
    assert!(bytes.starts_with(b"%PDF-"));
}

#[test]
fn a_drawing_that_moved_on_since_the_preview_is_refused_and_the_preview_asked_again() {
    let mut app = app_with_atlas();
    run(&mut app, "file.export.atlas");
    // The drawing changes after the preview: a polygon more on the coverage layer.
    {
        let doc = app.document.as_mut().expect("open");
        let entity: kentos_commands::Entity = serde_json::from_value(json!({
            "kind": "polygon", "id": 0, "layerId": "parsel", "attrs": {},
            "pts": [{"x": 1.0, "y": 1.0}, {"x": 5.0, "y": 1.0}, {"x": 5.0, "y": 4.0}]
        }))
        .expect("a polygon");
        doc.model.add(entity).expect("added");
    }
    let path = scratch("atlas-degisti").join("atlas.pdf");
    send(&mut app, Event::Atlas(A::SavePicked(Some(path.clone()))));
    let s = window(&app);
    let (error, text) = s.status.as_ref().expect("said");
    assert!(*error, "{text}");
    assert!(text.contains("sabitlenen durumdan değişti"), "{text}");
    assert!(
        !path.exists(),
        "nothing written from a state the preview did not show"
    );
    // The preview again: the new state, and the file is written.
    send(&mut app, Event::Atlas(A::Refresh));
    send(&mut app, Event::Atlas(A::SavePicked(Some(path.clone()))));
    assert!(app.exchange.is_none());
    assert!(path.exists());
}

#[test]
fn the_atlas_of_a_sheet_is_edited_in_the_window() {
    let mut app = app_with_atlas();
    run(&mut app, "file.export.atlas");
    // The second sheet has no atlas.
    send(&mut app, Event::Atlas(A::Sheet(1)));
    assert!(window(&app).plan.is_err(), "no atlas yet");
    let layer = window(&app)
        .layers
        .iter()
        .position(|(id, _)| id == "parsel")
        .expect("the parcel layer")
        + 1;
    send(&mut app, Event::Atlas(A::Layer(layer)));
    send(&mut app, Event::Atlas(A::Filter("Parsel <= 10".into())));
    send(
        &mut app,
        Event::Atlas(A::PageName("'Parsel ' || Parsel".into())),
    );
    send(&mut app, Event::Atlas(A::Apply));
    let pages = &window(&app).plan.as_ref().expect("planned now").pages;
    // Ten parcels (the fifth is outside the layer's filter).
    assert_eq!(pages.len(), 9);
    assert_eq!(pages[0].name, "Parsel 1");
    let sheets = app
        .document
        .as_ref()
        .and_then(|d| d.model.sheets())
        .expect("a book");
    assert!(sheets.book.to_string().contains("Parsel <= 10"));
}

#[test]
fn the_sheet_modes_atlas_button_opens_the_window() {
    let mut app = app_with_atlas();
    let task = app.sheet_effects(vec![kentos_sheet_ui::Effect::HostAction(
        "sheet.atlas".into(),
    )]);
    drive(&mut app, task);
    assert!(matches!(app.exchange, Some(Window::AtlasExport(_))));
}

#[test]
#[ignore = "pictures, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").unwrap_or_default();
    let wanted = |name: &str| only.is_empty() || only.split(',').any(|w| name.contains(w));
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
            if !wanted(name) {
                return;
            }
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, App::view, &mut update);
            let _ = snapshot.render(app.view(), &app.theme());
            let file = out.join(format!("{name}-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        for mode in ["dark", "light"] {
            let fresh = || {
                let mut app = app_with_atlas();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", Value::from(mode))]);
                app.apply_settings();
                app
            };
            let mut app = fresh();
            run(&mut app, "file.export.atlas");
            picture(&mut app, &format!("atlas-{mode}-1-onizleme"));
            // The atlas of the sheet without one: asked to be defined.
            send(&mut app, Event::Atlas(A::Sheet(1)));
            picture(&mut app, &format!("atlas-{mode}-2-atlassiz"));
            // A moment and a theme that is not there: said, nothing planned.
            let mut app = fresh();
            run(&mut app, "file.export.atlas");
            send(&mut app, Event::Atlas(A::Time("dün".into())));
            send(&mut app, Event::Atlas(A::Refresh));
            picture(&mut app, &format!("atlas-{mode}-3-zaman-hatasi"));
            // A drawing that moved on since the preview.
            let mut app = fresh();
            run(&mut app, "file.export.atlas");
            {
                let doc = app.document.as_mut().expect("open");
                let entity: kentos_commands::Entity = serde_json::from_value(json!({
                    "kind": "polygon", "id": 0, "layerId": "parsel", "attrs": {},
                    "pts": [{"x": 1.0, "y": 1.0}, {"x": 5.0, "y": 1.0}, {"x": 5.0, "y": 4.0}]
                }))
                .expect("a polygon");
                doc.model.add(entity).expect("added");
            }
            app.picker = Picker::File(scratch("atlas-resim").join("atlas.pdf"));
            send(&mut app, Event::Atlas(A::Save));
            picture(&mut app, &format!("atlas-{mode}-4-degisti"));
            // Pages drawn in TUREF / TM30 (docs/adr/0638 §1).
            let mut app = fresh();
            run(&mut app, "file.export.atlas");
            let tm30 = window(&app)
                .systems
                .iter()
                .position(|(code, _)| *code == 5254)
                .expect("TM30");
            send(&mut app, Event::Atlas(A::Crs(tm30 + 1)));
            picture(&mut app, &format!("atlas-{mode}-5-sistem"));
            // The coverage read from its source (docs/adr/0638 §3).
            let mut app = app_with_source();
            let _ = app
                .settings
                .choose(&[("appearance.theme", Value::from(mode))]);
            app.apply_settings();
            run(&mut app, "file.export.atlas");
            send(&mut app, Event::Atlas(A::FromSource(true)));
            picture(&mut app, &format!("atlas-{mode}-6-kaynak"));
        }
    }
}

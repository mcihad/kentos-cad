//! MAP-18 on the desktop (docs/adr/1320): a parcel set's delivery, one parcel moved, the window's Durum telling which
//! pages went stale and why, and Güncelle making only those again into a new folder while the others are carried byte
//! for byte. The parcels and sheets are the shared cases' (`fixtures/commands/v1/file.publish.sheets.update.json`).
use super::*;
use crate::exchange::{Window, sheet_publish::Event as E};

const CASES: &str = include_str!("../../../fixtures/commands/v1/file.publish.sheets.update.json");

fn parcel_set_app(theme: &str, text: u32) -> App {
    let fx: serde_json::Value = serde_json::from_str(CASES).unwrap();
    let mut model = kentos_domain::Document::from_snapshot(
        serde_json::from_value(fx["setup"].clone()).unwrap(),
    )
    .unwrap();
    let book: kentos_sheet::SheetBook = serde_json::from_value(fx["book"].clone()).unwrap();
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let (mut app, _store) = app_with_store("map18-update");
    app.settings.choose(&[
        ("appearance.theme", serde_json::json!(theme)),
        ("appearance.textSize", serde_json::json!(text)),
    ]);
    app.apply_settings();
    let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    app
}

/// Publishes the whole set into `path`, then moves parcel 37 (east, row 2) half a metre.
fn published_then_moved(app: &mut App, path: &std::path::Path) {
    let _ = app.sheet_publish_open();
    for e in [
        E::Path(path.display().to_string()),
        E::All(true),
        E::Separate(true),
        E::Plan,
    ] {
        let _ = app.sheet_publish_event(e);
    }
    wait_sheet_publication(app);
    let _ = app.sheet_publish_event(E::Publish);
    wait_sheet_publication(app);
    assert!(path.join("yayin.json").is_file());
    let model = &mut app.document.as_mut().unwrap().model;
    let parcel = model.uid(kentos_domain::Slot(37)).unwrap().to_string();
    let answer = kentos_native_application::call::call(
        model,
        None,
        envelope(
            "cad.entities.transform",
            serde_json::json!({"uids": [parcel], "transform": {"kind": "move", "dx": 0.5, "dy": 0.0}}),
        ),
    );
    assert!(
        matches!(
            answer.result,
            kentos_commands::CommandResult::Completed { .. }
        ),
        "{answer:?}"
    );
}

#[test]
fn durum_names_the_moved_parcel_and_guncelle_carries_the_unchanged_pages() {
    let mut app = parcel_set_app("light", 13);
    let path = scratch("map18-update").join("teslim");
    published_then_moved(&mut app, &path);
    let _ = app.sheet_publish_event(E::Status);
    wait_sheet_publication(&mut app);
    let Some(Window::SheetPublish(s)) = &app.exchange else {
        panic!("window")
    };
    let f = s.folder.as_ref().unwrap();
    assert_eq!(f.stale, ["dogu-500", "butun-5000"], "{}", s.status);
    let east = f.sheets.iter().find(|x| x.sheet == "dogu-500").unwrap();
    assert_eq!(east.changes.as_ref().unwrap().changed.len(), 1);
    let task = app.sheet_publish_event(E::Update);
    drive(&mut app, task);
    wait_sheet_publication(&mut app);
    let Some(Window::SheetPublish(s)) = &app.exchange else {
        panic!("window")
    };
    let plan = s.report.as_ref().unwrap();
    assert!(plan.ready, "{:?}", plan.issues);
    assert_eq!(plan.carried, ["bati-500", "genel-1000"]);
    let again = std::path::PathBuf::from(&s.input.path);
    assert_ne!(again, path);
    let _ = app.sheet_publish_event(E::Publish);
    wait_sheet_publication(&mut app);
    let Some(Window::SheetPublish(s)) = &app.exchange else {
        panic!("window")
    };
    let done = s.report.as_ref().unwrap();
    assert_eq!(done.carried, ["bati-500", "genel-1000"], "{}", s.status);
    for id in &done.carried {
        let name = |m: &kentos_commands::SheetsPublished| {
            m.files
                .iter()
                .find(|f| f.sheets == [id.clone()])
                .unwrap()
                .name
                .clone()
        };
        assert_eq!(
            std::fs::read(again.join(name(done))).unwrap(),
            std::fs::read(path.join(name(done))).unwrap()
        );
    }
}

#[test]
#[ignore = "GPU publication window: stale pages, their causes and the update, run alone"]
fn map18_update_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).unwrap();
    for (theme, text) in [("light", 13), ("dark", 13), ("dark", 18)] {
        let mut app = parcel_set_app(theme, text);
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let path = scratch(&format!("map18-shots-{theme}-{text}")).join("teslim");
        published_then_moved(&mut app, &path);
        let _ = app.sheet_publish_event(E::Status);
        wait_sheet_publication(&mut app);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(1100., 720.)).unwrap();
        snap.settle(&mut app, App::view, &mut update);
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Scroll(iced::Point::new(550., 420.), -40.),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("map18-status-{theme}-{text}.png")))
            .unwrap();
        let task = app.sheet_publish_event(E::Update);
        drive(&mut app, task);
        wait_sheet_publication(&mut app);
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("map18-update-plan-{theme}-{text}.png")))
            .unwrap();
        // Down to the earlier delivery's line and the pages, each carried or made again.
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Scroll(iced::Point::new(550., 420.), -10.),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("map18-update-pages-{theme}-{text}.png")))
            .unwrap();
        let _ = app.sheet_publish_event(E::Publish);
        wait_sheet_publication(&mut app);
        snap.settle(&mut app, App::view, &mut update);
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Scroll(iced::Point::new(550., 420.), 100.),
        );
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Scroll(iced::Point::new(550., 420.), -10.),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("map18-updated-{theme}-{text}.png")))
            .unwrap();
    }
}

//! Sunucuya içe aktar and Atlası sunucuda yap, message by message, without a server (docs/adr/0561 §4): the button an
//! import or atlas window gets is there for an open cloud database project (off, with why, when the account may not
//! ask), each window says its choices as the work it asks (the command's input without its path, the file, the pin the
//! preview showed), the window asks the server for the job after the unsent edits went and keeps the request id until
//! the server answers, and the pictures:
//!
//! ```text
//! cargo test -p kentos-desktop exchange::server_work_tests::screens -- --ignored --nocapture
//! ```
//!
//! They go to `.run/shots/islem-sunucu-ice-aktar-*.png`, `…-atlas-*.png`.

use std::path::PathBuf;

use kentos_cloud::ApiFailure;
use kentos_commands::{
    ImportFormat, ProjectPermission, ProjectState, ProjectStorage, ServerJob, ServerJobState,
};
use serde_json::{Value, json};

use super::server_work::{Event as W, Source, Work};
use super::tests::{fixture, run, send};
use super::{Event, Window};
use crate::app::{App, Dialog, Message, Picker};
use crate::cloud::catalog::Tab;
use crate::cloud::tests::{info, open, open_with, signed_in};
use crate::files_testing::app_with_drawing;
use kentos_native_application::ExecutionContext;

const CASES: &str = include_str!("../../../../fixtures/atlas/v1/cases.json");

fn gpkg_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geopackage/v1/kentos-genis.gpkg")
}

/// A cloud database project open, the account holding exactly these rights, signed in.
fn cloud_app(rights: &[ProjectPermission]) -> App {
    let mut app = signed_in();
    let mut i = info(ProjectStorage::Database, true, ProjectState::Active);
    i.access.permissions = rights.to_vec();
    open(&mut app, i);
    app
}

fn all() -> Vec<ProjectPermission> {
    use ProjectPermission::*;
    vec![Read, FeatureWrite, Edit, Download, JobsRun]
}

/// The atlas drawing of `fixtures/atlas/v1/cases.json` with its sheets, as the server's content of a cloud project.
fn atlas_document() -> kentos_domain::Document {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    let setup = &file["setups"]["main"];
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&setup["drawing"].to_string())
        .expect("reads");
    let mut doc = kentos_domain::Document::from_snapshot(snapshot).expect("opens");
    for sheet in setup["book"]["sheets"].as_array().expect("sheets") {
        let input =
            serde_json::from_value(json!({ "ops": [{ "op": "addSheet", "sheet": sheet }] }))
                .expect("an input");
        let answer = kentos_native_application::sheets_edit::execute(
            &mut ExecutionContext::new(&mut doc),
            input,
        );
        assert!(
            matches!(answer, kentos_commands::CommandResult::Completed { .. }),
            "{answer:?}"
        );
    }
    doc
}

fn job_of(kind: &str, input: Value) -> ServerJob {
    ServerJob {
        id: "0199bbbb-0000-7000-8000-000000000002".into(),
        project: crate::cloud::tests::PROJECT.into(),
        kind: kind.into(),
        input,
        input_sha256: "0".repeat(64),
        base_revision: "7".into(),
        state: ServerJobState::Queued,
        progress: 0.0,
        progress_text: None,
        attempts: 0,
        max_attempts: 3,
        timeout_seconds: 600,
        cancel_requested: false,
        created_by: "0199aaaa-0000-7000-8000-00000000a1a1".into(),
        created_at: "2026-10-20T10:00:00Z".into(),
        started_at: None,
        finished_at: None,
        result: None,
        artifact: None,
        error: None,
        cached_from: None,
    }
}

fn window(app: &App) -> &super::server_work::State {
    match &app.exchange {
        Some(Window::ServerWork(s)) => s,
        _ => panic!("the server work window"),
    }
}

/// A message handled without running the tasks it makes (the ask goes nowhere here).
fn step(app: &mut App, event: Event) {
    let _ = app.update(Message::Exchange(Box::new(event)));
}

fn import_work() -> Work {
    Work::import(
        ImportFormat::Gpkg,
        "kentos-genis.gpkg",
        Source::Path(gpkg_fixture()),
        json!({ "path": "/yok.gpkg", "tables": ["parsel"], "reproject": false }),
        vec!["Tablolar: parsel.".into()],
    )
}

#[test]
fn the_button_is_for_an_open_cloud_database_project_and_off_with_why_without_the_right() {
    use super::server_work::format_name;
    use crate::processing::ServerPlace;
    // A local drawing and a file project: no button.
    let app = app_with_drawing();
    assert_eq!(app.server_work_place(false), ServerPlace::Absent);
    assert!(app.server_work_button(import_work()).is_none());
    let mut app = signed_in();
    open(
        &mut app,
        info(ProjectStorage::File, true, ProjectState::Active),
    );
    assert_eq!(app.server_work_place(false), ServerPlace::Absent);
    // An account that may run jobs, write objects and download: both are ready.
    let app = cloud_app(&all());
    assert_eq!(app.server_work_place(false), ServerPlace::Ready);
    assert_eq!(app.server_work_place(true), ServerPlace::Ready);
    assert!(app.server_work_button(import_work()).is_some());
    // An import writes objects, an atlas reads: each needs its own right beside `project.jobs.run`.
    use ProjectPermission::*;
    let app = cloud_app(&[Read, Download, JobsRun]);
    assert_eq!(
        app.server_work_place(false),
        ServerPlace::Off("yetkiniz yok (feature.write)".into())
    );
    assert_eq!(app.server_work_place(true), ServerPlace::Ready);
    let app = cloud_app(&[Read, FeatureWrite, JobsRun]);
    assert_eq!(app.server_work_place(false), ServerPlace::Ready);
    assert_eq!(
        app.server_work_place(true),
        ServerPlace::Off("yetkiniz yok (project.download)".into())
    );
    let app = cloud_app(&[Read, FeatureWrite, Download]);
    assert_eq!(
        app.server_work_place(false),
        ServerPlace::Off("yetkiniz yok (project.jobs.run)".into())
    );
    // An off place still has its button, pressed by nobody.
    assert!(app.server_work_button(import_work()).is_some());
    assert_eq!(format_name(ImportFormat::Dxf), "DXF");
}

#[test]
fn each_import_window_says_its_choices_as_the_work_it_asks() {
    // GeoPackage: the tables chosen and the move into the project's system.
    let mut app = app_with_drawing();
    app.picker = Picker::File(gpkg_fixture());
    run(&mut app, "file.import.gpkg");
    let Some(Window::Gpkg(s)) = &app.exchange else {
        panic!("the GeoPackage window");
    };
    let Work::Import {
        format,
        name,
        source,
        options,
        lines,
    } = App::gpkg_server_work(s)
    else {
        panic!("an import");
    };
    assert_eq!(format, ImportFormat::Gpkg);
    assert_eq!(name, "kentos-genis.gpkg");
    assert!(matches!(source, Source::Path(p) if p == gpkg_fixture()));
    assert!(!options.contains_key("path"), "the server gives the file");
    assert_eq!(options["reproject"], false);
    assert!(options["tables"].as_array().is_some_and(|t| !t.is_empty()));
    assert!(lines[0].starts_with("Tablolar: "), "{lines:?}");
    // FlatGeobuf, GML and CSV: the command's input, the path left out.
    let fgb = super::fgb_exchange::ImportState {
        path: PathBuf::from("/veri/ada.fgb"),
        reproject: true,
        merge: super::merge_form::MergeForm::new(&PathBuf::from("/veri/ada.fgb"), true),
        plan: Err(String::new()),
        status: None,
    };
    let Work::Import {
        format,
        name,
        options,
        lines,
        ..
    } = App::fgb_server_work(&fgb)
    else {
        panic!("an import");
    };
    assert_eq!((format, name.as_str()), (ImportFormat::Fgb, "ada.fgb"));
    assert_eq!(Value::Object(options), json!({ "reproject": true }));
    assert_eq!(lines, ["Başka sistemdeyse projenin sistemine çevrilir."]);
    // A merge goes to the server; the rows refused stay on its side: no file of this computer is named.
    let mut merging = fgb;
    merging.merge.on = true;
    merging.merge.rejects = true;
    let Work::Import { options, .. } = App::fgb_server_work(&merging) else {
        panic!("an import");
    };
    assert_eq!(options["merge"]["matched"], "update");
    assert!(!options.contains_key("rejects"), "{options:?}");
    let gml = super::gml_exchange::ImportState {
        path: PathBuf::from("/veri/binalar.gml"),
        reproject: false,
        merge: super::merge_form::MergeForm::new(&PathBuf::from("/veri/binalar.gml"), true),
        plan: Err(String::new()),
        status: None,
    };
    let Work::Import {
        format,
        options,
        lines,
        ..
    } = App::gml_server_work(&gml)
    else {
        panic!("an import");
    };
    assert_eq!(format, ImportFormat::Gml);
    assert_eq!(options["reproject"], false);
    assert!(lines.is_empty());
    // DXF: the file the window read goes as bytes, with the layers left out and Blokları patlat.
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("entities.dxf"));
    run(&mut app, "file.import.dxf");
    send(
        &mut app,
        Event::DrawingImport(super::drawing_import::Event::Explode),
    );
    let Some(Window::DrawingImport(s)) = &app.exchange else {
        panic!("the DXF window");
    };
    let Some(Work::Import {
        format,
        source,
        options,
        lines,
        ..
    }) = app.dxf_server_work(s)
    else {
        panic!("a DXF import");
    };
    assert_eq!(format, ImportFormat::Dxf);
    assert!(matches!(&source, Source::Bytes(b) if !b.is_empty()));
    assert_eq!(options["explodeBlocks"], true);
    assert!(!options.contains_key("path"));
    assert!(
        lines.iter().any(|l| l.starts_with("Bloklar patlatılır")),
        "{lines:?}"
    );
}

#[test]
fn the_atlas_window_asks_for_the_pages_of_the_state_its_preview_pinned() {
    let mut app = signed_in();
    open_with(
        &mut app,
        info(ProjectStorage::Database, true, ProjectState::Active),
        atlas_document(),
    );
    run(&mut app, "file.export.atlas");
    let Some(Window::AtlasExport(s)) = &app.exchange else {
        panic!("the atlas window");
    };
    let planned = s.plan.as_ref().expect("planned");
    assert_eq!(planned.pages.len(), 100);
    let pin = planned.pin.clone();
    let Some(Work::Atlas {
        input,
        sheet,
        pages,
        lines,
    }) = App::atlas_server_work(s)
    else {
        panic!("an atlas");
    };
    assert_eq!(pages, 100);
    assert_eq!(
        input.pin.as_ref(),
        Some(&pin),
        "the job carries the preview's pin"
    );
    assert_eq!(input.sheet, s.sheets[s.sheet].id);
    assert_eq!(input.label.as_deref(), Some(sheet.as_str()));
    assert!(input.theme.is_none() && input.time.is_none());
    assert!(lines[0].starts_with("100 sayfa"), "{lines:?}");
    // A moment chosen is a moment asked.
    send(
        &mut app,
        Event::Atlas(super::atlas_export::Event::Time("2024-06-30".into())),
    );
    let Some(Window::AtlasExport(s)) = &app.exchange else {
        panic!("the atlas window");
    };
    // The preview is of the last choices: a time edited since is the next preview's.
    if let Some(Work::Atlas { input, .. }) = App::atlas_server_work(s) {
        assert_eq!(input.time.as_deref(), Some("2024-06-30"));
    }
}

#[test]
fn the_window_asks_the_server_once_keeps_the_request_id_for_a_retry_and_takes_the_way_to_the_job() {
    let mut app = cloud_app(&all());
    send(
        &mut app,
        Event::ServerWork(W::Open(Box::new(import_work()))),
    );
    assert!(matches!(&app.exchange, Some(Window::ServerWork(_))));
    let request = window(&app).request.clone();
    assert!(request.starts_with("desktop-"));
    assert!(window(&app).asked.is_none() && window(&app).job.is_none());
    // Gönder: the ask is on its way (the unsent edits had nothing to send); nothing is written to the drawing.
    let before = app.document.as_ref().expect("open").model.revision();
    step(&mut app, Event::ServerWork(W::Send));
    let asked = window(&app).asked.expect("the ask is on its way");
    assert_eq!(
        window(&app).status,
        Some((false, "Dosya sunucuya gönderiliyor…".to_owned()))
    );
    assert_eq!(
        app.document.as_ref().expect("open").model.revision(),
        before
    );
    // A second press while it is on its way asks nothing more.
    step(&mut app, Event::ServerWork(W::Send));
    assert_eq!(window(&app).asked, Some(asked));
    // The server refuses: its words, the request id kept.
    step(
        &mut app,
        Event::ServerWork(W::Asked {
            id: asked,
            result: Box::new(Err(ApiFailure::new(
                429,
                "quota_exceeded",
                "Kurumunuzun sırada ve çalışan iş sınırına (50) ulaşıldı.",
            ))),
        }),
    );
    let (error, text) = window(&app).status.clone().expect("said");
    assert!(
        error && text.starts_with("Sunucu işi istenemedi: "),
        "{text}"
    );
    assert!(text.contains("(50)"), "{text}");
    assert!(window(&app).asked.is_none());
    assert_eq!(
        window(&app).request,
        request,
        "asking again gives the same job"
    );
    // Again, and the server makes the job.
    step(&mut app, Event::ServerWork(W::Send));
    let asked = window(&app).asked.expect("asked again");
    // An answer to an ask that is no longer the window's is let go.
    step(
        &mut app,
        Event::ServerWork(W::Asked {
            id: asked + 99,
            result: Box::new(Err(ApiFailure::new(500, "internal", "Başka bir iş."))),
        }),
    );
    assert_eq!(window(&app).asked, Some(asked));
    step(
        &mut app,
        Event::ServerWork(W::Asked {
            id: asked,
            result: Box::new(Ok(job_of(
                "project.import",
                json!({ "upload": "x", "label": "kentos-genis.gpkg" }),
            ))),
        }),
    );
    let (error, text) = window(&app).status.clone().expect("said");
    assert!(!error);
    assert!(
        text.starts_with("Sunucu işi istendi: İçe aktar: kentos-genis.gpkg."),
        "{text}"
    );
    assert!(window(&app).job.is_some());
    // Once made, it is not asked for again.
    step(&mut app, Event::ServerWork(W::Send));
    assert!(window(&app).asked.is_none());
    // Sunucu işleri: the catalog on the project's jobs.
    send(&mut app, Event::ServerWork(W::Jobs));
    assert!(app.exchange.is_none());
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    assert_eq!(
        app.cloud.catalog.as_ref().expect("the catalog").tab,
        Tab::Jobs
    );
}

#[test]
fn an_account_without_the_right_is_told_so_and_nothing_is_asked() {
    use ProjectPermission::*;
    let mut app = cloud_app(&[Read, FeatureWrite, Download]);
    send(
        &mut app,
        Event::ServerWork(W::Open(Box::new(import_work()))),
    );
    step(&mut app, Event::ServerWork(W::Send));
    assert!(window(&app).asked.is_none());
    let (error, text) = window(&app).status.clone().expect("said");
    assert!(error && text.contains("project.jobs.run"), "{text}");
    // A staged ask answered after the window closed is nobody's.
    send(&mut app, Event::Close);
    send(
        &mut app,
        Event::ServerWork(W::Asked {
            id: 1,
            result: Box::new(Err(ApiFailure::new(500, "internal", "Geç gelen."))),
        }),
    );
    assert!(app.exchange.is_none());
}

/// The windows with the button, the shared window before and after the ask, and off for lack of the right.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = [
        "gpkg-pencere",
        "secildi",
        "istendi",
        "yetkisiz",
        "dxf-pencere",
        "atlas-pencere",
        "atlas-secildi",
        "atlas-istendi",
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
                let atlas = name.starts_with("atlas");
                let mut app = if atlas {
                    let mut app = signed_in();
                    open_with(
                        &mut app,
                        {
                            let mut i = info(ProjectStorage::Database, true, ProjectState::Active);
                            i.access.permissions = all();
                            i
                        },
                        atlas_document(),
                    );
                    app
                } else if name == "yetkisiz" {
                    use ProjectPermission::*;
                    cloud_app(&[Read, FeatureWrite])
                } else {
                    cloud_app(&all())
                };
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
                    "gpkg-pencere" => {
                        app.picker = Picker::File(gpkg_fixture());
                        run(&mut app, "file.import.gpkg");
                    }
                    "dxf-pencere" => {
                        app.picker = Picker::File(fixture("entities.dxf"));
                        run(&mut app, "file.import.dxf");
                    }
                    "atlas-pencere" | "atlas-secildi" | "atlas-istendi" => {
                        run(&mut app, "file.export.atlas");
                        if name != "atlas-pencere" {
                            let work = match &app.exchange {
                                Some(Window::AtlasExport(s)) => App::atlas_server_work(s),
                                _ => None,
                            }
                            .expect("the atlas work");
                            send(&mut app, Event::ServerWork(W::Open(Box::new(work))));
                        }
                    }
                    _ => {
                        send(&mut app, Event::ServerWork(W::Open(Box::new(shot_work()))));
                    }
                }
                if matches!(name, "istendi" | "atlas-istendi") {
                    step(&mut app, Event::ServerWork(W::Send));
                    let asked = window(&app).asked.expect("asked");
                    let job = if atlas {
                        job_of(
                            "project.atlas",
                            json!({ "sheet": "atlas-paftasi", "label": "Parsel atlası" }),
                        )
                    } else {
                        job_of(
                            "project.import",
                            json!({ "upload": "x", "label": "parseller.gpkg" }),
                        )
                    };
                    step(
                        &mut app,
                        Event::ServerWork(W::Asked {
                            id: asked,
                            result: Box::new(Ok(job)),
                        }),
                    );
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!(
                    "islem-sunucu-ice-aktar-{name}-{width}x{height}{suffix}.png"
                ));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

/// The work the shared window's pictures show: a GeoPackage of parcels with its choices.
fn shot_work() -> Work {
    Work::import(
        ImportFormat::Gpkg,
        "parseller.gpkg",
        Source::Path(gpkg_fixture()),
        json!({ "tables": ["parsel", "bina"], "reproject": true }),
        vec![
            "Tablolar: parsel, bina.".into(),
            "Başka sistemdeyse projenin sistemine çevrilir.".into(),
        ],
    )
}

//! Sunucuda çalıştır in the tool's window, message by message, without a server (docs/adr/0559 §5): “KentOS sunucusunda”
//! is a place of Nerede çalışır for an open cloud database project (off, with why, when the account may not run jobs
//! there; not listed for a local drawing, a file project or a model), Çalıştır asks the server for a
//! `project.processing` job with the window's values, the objects chosen and the box on screen, the unsent edits of
//! the open project going first, the window says the job is asked and takes the user to its row in the catalog, a
//! refusal is said in the server's words and asking again keeps the request id; and the pictures:
//!
//! ```text
//! cargo test -p kentos-desktop processing::server_tests::screens -- --ignored --nocapture
//! ```
//!
//! They go to `.run/shots/islem-sunucu-*.png`.

use kentos_cloud::ApiFailure;
use kentos_commands::{
    ProcessingJobMode, ProjectPermission, ProjectState, ProjectStorage, ServerJob, ServerJobState,
};
use serde_json::json;

use super::plan::{ServerPlace, targets_view};
use super::{Event, RunStatus};
use crate::app::{App, Dialog, Message};
use crate::cloud::catalog::Tab;
use crate::cloud::tests::{info, open, signed_in};

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Processing(e));
}

fn status(app: &App) -> RunStatus {
    app.processing
        .dialog
        .as_ref()
        .map_or(RunStatus::Idle, |w| w.status.clone())
}

/// A cloud database project open, the account allowed to run jobs in it or not, signed in.
fn cloud_app(jobs: bool) -> App {
    let mut app = signed_in();
    let mut i = info(ProjectStorage::Database, true, ProjectState::Active);
    if jobs {
        i.access.permissions.push(ProjectPermission::JobsRun);
    }
    open(&mut app, i);
    app
}

/// A local drawing: the sample, not a cloud project.
fn local_app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
        "../../../../fixtures/document/v1/sample.json"
    ))
    .expect("the sample reads");
    let doc = crate::document::Document::new(snapshot, None).expect("the sample opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn tool(app: &mut App, id: &'static str) {
    let _ = app.update(Message::Run(id));
}

/// The rows of Nerede çalışır as the window shows them: (label, note, disabled).
fn rows(app: &App) -> Vec<(String, Option<String>, bool)> {
    let window = app.processing.dialog.as_ref().expect("a window");
    let (options, _, _) = targets_view(
        &window.targets(&app.processing.registry, &app.server_place()),
        super::plan::Choice::read(app.processing.memory.target(&window.tool.id)),
    );
    options
        .into_iter()
        .map(|o| (o.label.to_owned(), o.note, o.disabled))
        .collect()
}

fn job_of(mode: &str) -> ServerJob {
    ServerJob {
        id: "0199bbbb-0000-7000-8000-000000000001".into(),
        project: crate::cloud::tests::PROJECT.into(),
        kind: "project.processing".into(),
        input: json!({ "tool": "annotation.edgeLengths", "label": "Kenar uzunluklarını yaz", "mode": mode }),
        input_sha256: "0".repeat(64),
        base_revision: "4".into(),
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

#[test]
fn the_server_is_a_place_for_an_open_cloud_database_project_and_off_with_why_without_the_right() {
    // A local drawing: not listed.
    let mut app = local_app();
    tool(&mut app, "processing.run.annotation.edgeLengths");
    assert_eq!(app.server_place(), ServerPlace::Absent);
    assert!(
        rows(&app)
            .iter()
            .all(|(label, _, _)| label != "KentOS sunucusunda"),
        "{:?}",
        rows(&app)
    );
    // An account that may run jobs: listed, on.
    let mut app = cloud_app(true);
    tool(&mut app, "processing.run.annotation.edgeLengths");
    assert_eq!(app.server_place(), ServerPlace::Ready);
    let listed = rows(&app);
    let server = listed
        .iter()
        .find(|(label, _, _)| label == "KentOS sunucusunda")
        .expect("the server's row");
    assert!(!server.2, "{listed:?}");
    // One that may not: listed, off, with why in the row's note.
    let mut app = cloud_app(false);
    tool(&mut app, "processing.run.annotation.edgeLengths");
    assert_eq!(
        app.server_place(),
        ServerPlace::Off("yetkiniz yok (project.jobs.run)".into())
    );
    let listed = rows(&app);
    let server = listed
        .iter()
        .find(|(label, _, _)| label == "KentOS sunucusunda")
        .expect("the server's row");
    assert!(server.2);
    assert_eq!(server.1.as_deref(), Some("yetkiniz yok (project.jobs.run)"));
    // Off, it is never the place a run goes to, whatever was kept for the tool.
    event(&mut app, Event::Target("server".into()));
    assert!(!app.processing_at_server());
    // A file project keeps its revisions as files: the tools run here.
    let mut app = signed_in();
    open(
        &mut app,
        info(ProjectStorage::File, true, ProjectState::Active),
    );
    assert_eq!(app.server_place(), ServerPlace::Absent);
    // Signed out: the project cannot be reached.
    let mut app = cloud_app(true);
    app.cloud.client = None;
    assert!(matches!(app.server_place(), ServerPlace::Off(w) if w.contains("oturum")));
}

#[test]
fn chosen_the_run_asks_the_server_for_a_job_with_the_windows_values() {
    let mut app = cloud_app(true);
    // Two objects chosen: the job names them by their persistent ids.
    let chosen: Vec<_> = app
        .document
        .as_ref()
        .expect("open")
        .model
        .entities()
        .take(2)
        .map(|e| kentos_domain::Slot(e.base().id))
        .collect();
    app.selection.set(chosen.iter().copied());
    tool(&mut app, "processing.run.annotation.edgeLengths");
    event(&mut app, Event::Target("server".into()));
    assert!(app.processing_at_server());
    // The button says what it will do; the result goes to the project unless the window says a file.
    let window = app.processing.dialog.as_ref().expect("a window");
    assert_eq!(
        super::plan::footer_at(&window.status, true).run,
        "Sunucuda çalıştır"
    );
    let input = app.processing_server_input().expect("a job's input");
    assert_eq!(input.tool, "annotation.edgeLengths");
    assert_eq!(input.label.as_deref(), Some("Kenar uzunluklarını yaz"));
    assert_eq!(input.mode, ProcessingJobMode::Commit);
    assert_eq!(input.selection.len(), 2);
    assert!(input.view.is_some());
    assert!(!input.parameters.is_empty(), "the window's values go too");
    event(&mut app, Event::ServerSink(true));
    assert_eq!(
        app.processing_server_input().expect("input").mode,
        ProcessingJobMode::Sink
    );
    // Çalıştır: the job is asked for (the unsent edits had nothing to send), nothing runs here.
    let before = app.document.as_ref().expect("open").model.revision();
    event(&mut app, Event::Run);
    assert_eq!(
        status(&app),
        RunStatus::Asked("Sunucu işi isteniyor…".into())
    );
    let asked = app.processing.server_asked.expect("the ask is on its way");
    let request = app.processing.server_request.clone().expect("a request id");
    assert_eq!(
        app.document.as_ref().expect("open").model.revision(),
        before,
        "the drawing is not touched by the ask"
    );
    // The server refuses: its words, the request id kept for the retry.
    crate::cloud::tests::cloud(
        &mut app,
        crate::cloud::Event::ProcessingJobAsked {
            id: asked,
            result: Err(ApiFailure::new(
                429,
                "quota_exceeded",
                "Kurumunuzun sırada ve çalışan iş sınırına (50) ulaşıldı.",
            )),
        },
    );
    let RunStatus::Error(text) = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert!(text.starts_with("Sunucu işi istenemedi: "), "{text}");
    assert!(text.contains("(50)"), "{text}");
    assert_eq!(
        app.processing.server_request.as_deref(),
        Some(request.as_str())
    );
    // Asking again sends the same request id; the server's answer is said, with the way to the job's row.
    event(&mut app, Event::Run);
    let asked = app.processing.server_asked.expect("asked again");
    assert_eq!(
        app.processing.server_request.as_deref(),
        Some(request.as_str())
    );
    crate::cloud::tests::cloud(
        &mut app,
        crate::cloud::Event::ProcessingJobAsked {
            id: asked,
            result: Ok(job_of("sink")),
        },
    );
    let RunStatus::Asked(text) = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert!(
        text.starts_with("Sunucu işi istendi: İşlem (dosyaya): Kenar uzunluklarını yaz."),
        "{text}"
    );
    assert!(
        app.processing.server_request.is_none(),
        "the next ask is a new request"
    );
    let line = app.processing.dialog.as_ref().expect("a window").line();
    assert_eq!(line.actions, vec![super::plan::Action::Jobs]);
    // An answer to an ask that is no longer the window's is let go.
    crate::cloud::tests::cloud(
        &mut app,
        crate::cloud::Event::ProcessingJobAsked {
            id: asked + 99,
            result: Err(ApiFailure::new(500, "internal", "Başka bir iş.")),
        },
    );
    assert!(matches!(status(&app), RunStatus::Asked(_)));
    // Sunucu işleri: the catalog on the project's jobs.
    event(&mut app, Event::ServerJobs);
    assert!(app.processing.dialog.is_none());
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    let catalog = app.cloud.catalog.as_ref().expect("the catalog");
    assert_eq!(catalog.tab, Tab::Jobs);
}

#[test]
fn an_edit_of_the_window_takes_the_asked_line_back() {
    let mut app = cloud_app(true);
    tool(&mut app, "processing.run.annotation.edgeLengths");
    event(&mut app, Event::Target("server".into()));
    event(&mut app, Event::Run);
    assert!(matches!(status(&app), RunStatus::Asked(_)));
    // An edit of a value puts the line back.
    let first = app
        .processing
        .dialog
        .as_ref()
        .expect("a window")
        .tool
        .parameters
        .iter()
        .find(|p| p.type_name() == "number")
        .map(|p| p.name.clone());
    if let Some(name) = first {
        event(&mut app, Event::Number(name, "3".into()));
        assert_eq!(status(&app), RunStatus::Idle);
    }
}

/// The window with the server chosen, after the ask, a tool off for lack of the right, and the catalog's tab with
/// the jobs the ask made: for the owner.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = ["secildi", "istendi", "yetkisiz", "dosya"];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in views {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| x == name))
                {
                    continue;
                }
                let mut app = cloud_app(name != "yetkisiz");
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                tool(&mut app, "processing.run.annotation.edgeLengths");
                event(&mut app, Event::Target("server".into()));
                if name == "dosya" {
                    event(&mut app, Event::ServerSink(true));
                }
                if name == "istendi" {
                    event(&mut app, Event::Run);
                    let asked = app.processing.server_asked.expect("asked");
                    crate::cloud::tests::cloud(
                        &mut app,
                        crate::cloud::Event::ProcessingJobAsked {
                            id: asked,
                            result: Ok(job_of("commit")),
                        },
                    );
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("islem-sunucu-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

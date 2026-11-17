//! Çalışan işler (mod.rs, docs/adr/8057 §102): read where each work runs, who started it, its own Durdur.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use kentos_commands::JobPhase;
use kentos_python_env::python::Progress;
use serde_json::{Value, json};

use super::{By, Event, Key};
use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::extensions::environment::{EnvBusy, EnvState};
use crate::files_testing::app_with_drawing;
use crate::python_personal::Busy;

const SQUARE: &str = r#"[{"x": 423500, "y": 4512300}, {"x": 423520, "y": 4512300}, {"x": 423520, "y": 4512312.5}, {"x": 423500, "y": 4512312.5}]"#;

fn objects(app: &App) -> usize {
    app.document.as_ref().map_or(0, |d| d.model.len())
}

/// An agent's job started on the drawing in front, as `who` asks for it: its id.
fn start(app: &mut App, who: &str, op: &str) -> String {
    let layer = app
        .document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .expect("a layer");
    let input: Value =
        serde_json::from_str(&format!(r#"{{"layerId": "{layer}", "pts": {SQUARE}}}"#))
            .expect("input");
    let started = app
        .agent_request(
            &json!({ "method": "start",
                     "params": { "command": "cad.polygon.create", "op": op, "input": input } }),
            who,
        )
        .expect("started");
    started["id"].as_str().expect("its id").to_owned()
}

/// Until the catalog's titles are read (on their own thread: `kentos_headless::command_title`).
fn titles_read() {
    let began = Instant::now();
    while kentos_headless::command_title("cad.polygon.create").is_none() {
        assert!(
            began.elapsed() < Duration::from_secs(60),
            "the titles never came"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Until the job's computation is in, as the list reads it: without asking it, which would write it.
fn staged(app: &App, id: &str) {
    let began = Instant::now();
    while app.python.jobs.peek(id).map(|s| s.phase) != Some(JobPhase::Staged) {
        assert!(
            began.elapsed() < Duration::from_secs(30),
            "{id} never computed"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// An agent's job computed and not asked for yet is listed by who started it and what it waits for; Durdur drops it
/// unwritten, the list empties and the agent hears `cancelled` when it asks.
#[test]
fn an_agents_job_is_listed_by_who_started_it_and_stopped_without_being_written() {
    let mut app = app_with_drawing();
    assert!(app.works().is_empty());
    assert_eq!(app.works_words(false), None);
    let before = objects(&app);
    let id = start(&mut app, "Ajan", "execute");
    staged(&app, &id);
    titles_read();
    let works = app.works();
    assert_eq!(works.len(), 1);
    let work = &works[0];
    assert_eq!(work.key, Key::Job(id.clone()));
    assert_eq!((work.what, work.by), ("Ajan işi", By::Agent));
    // Named by the command's title in the catalog, not its id.
    assert_eq!(work.title, "Kapalı alan oluştur");
    assert_eq!(
        work.state.as_deref(),
        Some("Hesaplandı; yazmak için ajanı bekliyor")
    );
    assert_eq!(work.share, Some(1.0), "computed: its bar full");
    assert!(work.can_stop && work.can_show && work.since.is_some());
    assert_eq!(
        app.works_words(false).as_deref(),
        Some("Kapalı alan oluştur")
    );
    let _ = app.update(Message::Works(Event::Stop(Key::Job(id.clone()))));
    assert!(app.works().is_empty());
    assert_eq!(objects(&app), before, "nothing written");
    let status = app
        .agent_request(&json!({ "method": "job", "params": { "id": id } }), "Ajan")
        .expect("its status");
    assert_eq!(status["phase"], "cancelled", "{status:#}");
    assert_eq!(objects(&app), before, "nor when the agent asks");
}

/// Asistan's job and an agent's say who started them and what each waits for (a plan read, an execution written); the
/// cell counts them; a job asked after it ended leaves the list.
#[test]
fn asistans_and_an_agents_jobs_say_who_and_the_cell_counts_them() {
    let mut app = app_with_drawing();
    let plan = start(&mut app, "Asistan", "plan");
    let execute = start(&mut app, "Ajan", "execute");
    staged(&app, &plan);
    staged(&app, &execute);
    let works = app.works();
    let said: Vec<(By, Option<&str>)> = works.iter().map(|w| (w.by, w.state.as_deref())).collect();
    assert_eq!(
        said,
        [
            (By::Assistant, Some("Plan hazır; ajanı bekliyor")),
            (By::Agent, Some("Hesaplandı; yazmak için ajanı bekliyor")),
        ]
    );
    // Asistan's shows in its panel; the link's in the Python tab.
    assert!(!works[0].can_show && works[1].can_show);
    assert_eq!(app.works_words(false).as_deref(), Some("2 iş çalışıyor"));
    assert_eq!(app.works_words(true).as_deref(), Some("2"));
    let status = app
        .agent_request(
            &json!({ "method": "job", "params": { "id": execute } }),
            "Ajan",
        )
        .expect("its status");
    assert_eq!(status["phase"], "done", "{status:#}");
    let left: Vec<Key> = app.works().into_iter().map(|w| w.key).collect();
    assert_eq!(left, [Key::Job(plan)]);
    // Göster on the link's job: the Python tab.
    let _ = app.update(Message::Works(Event::Show(Key::Job(execute))));
    assert_eq!(app.bottom_tab, BottomTab::Python);
}

/// KentOS's Python being installed, the personal environment's packages and an extension's environment are listed with
/// how far they are; each one's Durdur is its own stop, and a work that waits for its stop says so.
#[test]
fn pythons_install_and_environments_are_listed_with_their_own_durdur() {
    let (mut app, _) = App::boot(None);
    let install = Arc::new(AtomicBool::new(false));
    app.python_setup.installing = Some(Arc::clone(&install));
    app.python_setup.progress = Some(Progress::Downloading {
        done: 12 * 1024 * 1024,
        total: Some(30 * 1024 * 1024),
    });
    let packages = Arc::new(AtomicBool::new(false));
    app.python_setup.personal.busy = Some(Busy::Applying);
    app.python_setup.personal.stop = Some(Arc::clone(&packages));
    let environment = Arc::new(AtomicBool::new(false));
    app.extensions.envs.insert(
        "tr.ornek.deneme".to_owned(),
        EnvState {
            busy: Some(EnvBusy::Preparing),
            stop: Some(Arc::clone(&environment)),
            ..EnvState::default()
        },
    );
    let works = app.works();
    let keys: Vec<Key> = works.iter().map(|w| w.key.clone()).collect();
    assert_eq!(
        keys,
        [
            Key::Environment("tr.ornek.deneme".to_owned()),
            Key::Install,
            Key::Personal,
        ]
    );
    assert_eq!(
        works[0].title,
        "tr.ornek.deneme: Python ortamı hazırlanıyor"
    );
    assert!(works.iter().all(|w| w.by == By::You && w.can_stop));
    let share = works[1].share.expect("how far");
    assert!((share - 0.4).abs() < 1e-6, "{share}");
    assert!(
        works[1]
            .state
            .as_deref()
            .is_some_and(|s| s.starts_with("İndiriliyor: 12,0 MB / 30,0 MB")),
        "{:?}",
        works[1].state
    );
    for (key, flag) in [
        (Key::Install, &install),
        (Key::Personal, &packages),
        (Key::Environment("tr.ornek.deneme".to_owned()), &environment),
    ] {
        let _ = app.update(Message::Works(Event::Stop(key.clone())));
        assert!(flag.load(Ordering::Relaxed), "{key:?}");
    }
    assert!(app.works().iter().all(|w| w.stopping));
    // Looking for newer versions has no Durdur of its own: none is offered.
    app.python_setup.personal.busy = Some(Busy::Checking);
    let checking = app
        .works()
        .into_iter()
        .find(|w| w.key == Key::Personal)
        .expect("listed");
    assert!(!checking.can_stop);
    assert_eq!(checking.title, "Paketlerin yeni sürümleri aranıyor");
}

/// The status bar's cell is there only while something runs; it opens the tab (every program has it:
/// module_surface_tests).
#[test]
fn the_cell_shows_while_something_runs_and_opens_the_tab() {
    let mut app = app_with_drawing();
    assert!(app.works_cell(false).is_none());
    let id = start(&mut app, "Ajan", "execute");
    staged(&app, &id);
    assert!(app.works_cell(false).is_some());
    let _ = app.update(Message::BottomTab(BottomTab::Works));
    assert!(app.command_expanded);
    assert_eq!(app.bottom_tab, BottomTab::Works);
}

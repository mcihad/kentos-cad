//! Komut geçmişi's calls (docs/adr/0284) on the desktop.

use kentos_contracts::{ReplayJournal, ReplayOutcome};
use kentos_native_application::ExecutionContext;
use serde_json::json;

use super::{Call, Event};
use crate::app::{App, Message};
use crate::tools_scenes::{E, N, Objects, click, open, run, typed};

/// A drawing on Çizim, Yol and Parsel, the view on the first 30 m.
pub(crate) fn drawing() -> App {
    let mut app = crate::files_testing::app_with_drawing();
    prepare(&mut app);
    app
}

/// [`drawing`] in `app`.
pub(crate) fn prepare(app: &mut App) {
    open(app, Objects::new());
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 2.0,
            max_x: E + 30.0,
            max_y: N + 20.0,
        },
        24.0,
    );
}

/// A circle drawn with Daire: its centre clicked, its radius typed.
pub(crate) fn circle_drawn(app: &mut App) {
    run(app, "tool.circle");
    click(app, [10.0, 8.0]);
    typed(app, "7.5");
    let _ = app.update(Message::LogFrame);
}

/// A circle refused by the command: its radius below zero.
pub(crate) fn circle_refused(app: &mut App) {
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let input = serde_json::from_value(json!({
        "layerId": "yol", "c": {"x": E + 20.0, "y": N + 4.0}, "r": -2.0
    }))
    .expect("the input reads");
    let _ = kentos_native_application::circle::execute(&mut ExecutionContext::new(doc), input);
    let _ = app.update(Message::LogFrame);
}

#[test]
fn a_tool_s_command_is_in_the_history_with_its_python_and_replay() {
    let mut app = drawing();
    circle_drawn(&mut app);
    let calls: Vec<_> = app
        .history
        .entries
        .iter()
        .filter_map(|e| match &e.call {
            Call::Row(r) => Some((e.id, r.command.clone())),
            Call::Refused(_) => None,
        })
        .collect();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].1.as_deref(), Some("cad.circle.create"));
    let id = calls[0].0;
    let entry = app.history.get(id).expect("the call").clone();

    let row = app.row_now(&entry).expect("its row");
    let layer = |id: &str| (id == "cizim").then(|| "Çizim".to_owned());
    let words = super::text::inputs(row.input.as_ref().expect("its input"), &layer);
    assert!(words.contains("Merkez: 487010, 4420008"), "{words}");
    assert!(words.contains("Yarıçap: 7.5"), "{words}");
    assert_eq!(super::text::outcome(row), "1 nesne eklendi");

    let python = app.python_of(&entry).expect("its Python");
    assert!(
        python.starts_with("# Daire oluştur — cad.circle.create v1\n"),
        "{python}"
    );
    assert!(python.contains("command = cad.circle.create\n"), "{python}");
    assert!(python.contains("\"r\": 7.5"), "{python}");

    // Shown under it, then copied: the log says where it went.
    let _ = app.update(Message::History(Event::Toggle(id)));
    assert_eq!(app.history.open, Some(id));
    let _ = app.update(Message::History(Event::CopyPython(id)));
    assert!(app.log.said(
        kentos_interaction::Level::Info,
        "Python kodu panoya kopyalandı"
    ));

    // The replay input runs the same command again on the drawing as it was.
    let replay: ReplayJournal =
        serde_json::from_str(&app.replay_of(&entry).expect("its replay input"))
            .expect("kentos.replay/1");
    assert_eq!(replay.format, "kentos.replay/1");
    assert_eq!(replay.calls.len(), 1);
    let _ = app.run("edit.undo");
    let undone = app.row_now(&entry).expect("its row").status;
    assert_eq!(
        undone,
        kentos_contracts::CallStatus::Undone,
        "read as the drawing has it now"
    );
    assert!(super::text::outcome(app.row_now(&entry).expect("row")).ends_with("geri alındı"));
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let report = kentos_native_application::replay::replay(doc, None, &replay);
    assert_eq!(report.steps[0].outcome, ReplayOutcome::Same, "{report:?}");
}

#[test]
fn a_refused_call_says_its_code_and_gives_its_python() {
    let mut app = drawing();
    circle_refused(&mut app);
    let entry = app
        .history
        .entries
        .iter()
        .find(|e| matches!(e.call, Call::Refused(_)))
        .expect("the refused call")
        .clone();
    let Call::Refused(r) = &entry.call else {
        unreachable!()
    };
    let said = super::text::refusal(r);
    assert!(
        said.starts_with("Reddedildi, hiçbir şey yazılmadı · "),
        "{said}"
    );
    assert!(
        said.contains(&r.error.as_ref().expect("its error").code),
        "{said}"
    );
    assert!(app.python_of(&entry).is_some());
    let replay: ReplayJournal =
        serde_json::from_str(&app.replay_of(&entry).expect("its replay input")).expect("reads");
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let report = kentos_native_application::replay::replay(doc, None, &replay);
    assert_eq!(
        report.steps[0].outcome,
        ReplayOutcome::Failed,
        "refused again"
    );
}

#[test]
fn the_replay_input_is_saved_where_asked_and_cleared_with_the_history() {
    let mut app = drawing();
    circle_drawn(&mut app);
    let id = app.history.entries[0].id;
    let dir = std::env::temp_dir().join(format!("kentos-history-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("komut.json");
    let _ = app.update(Message::History(Event::SaveTo(id, Some(path.clone()))));
    let text = std::fs::read_to_string(&path).expect("written");
    assert!(text.contains("\"format\": \"kentos.replay/1\""));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = app.update(Message::HistoryCleared);
    assert!(app.history.entries.is_empty());
    // A call after the clearing is listed; the ones before do not come back.
    circle_drawn(&mut app);
    assert_eq!(app.history.entries.len(), 1);
}

use super::*;
use crate::catalog::PORTED;

#[test]
fn every_ported_command_has_a_handler() {
    for id in PORTED {
        let (mut app, _) = App::boot(None);
        let _ = app.run(id);
        assert!(
            !app.log.said(Level::Error, "işleyicisi eksik"),
            "{id} is listed as ported but has no handler"
        );
    }
}

#[test]
fn a_command_not_ported_says_so_and_changes_nothing() {
    // One the web runs and the desktop not yet, whichever is left as they are ported.
    let Some(id) = catalog()
        .commands()
        .iter()
        .find(|c| c.standing == Standing::OnTheWeb)
        .map(|c| c.id)
    else {
        return;
    };
    let (mut app, _) = App::boot(None);
    let before = app.log.len();
    let _ = app.run(id);
    assert_eq!(app.log.len(), before + 1);
    assert!(
        app.log.last().is_some_and(
            |l| l.level == Level::Info && l.text.contains("masaüstüne henüz taşınmadı")
        )
    );
}

fn with_demo() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
        "../../../../../fixtures/document/v1/sample.json"
    ))
    .expect("the web's demo file reads");
    app.document = Some(Document::new(snapshot, None).expect("opens"));
    app
}

/// A tool's method from its ribbon menu starts the tool and gives it the
/// method's option, as the web's `runEntry` does (docs/adr/0032).
#[test]
fn a_method_from_the_ribbon_starts_its_tool_with_its_option() {
    let mut app = with_demo();
    let _ = app.update(Message::RunMethod {
        id: "tool.circle",
        option: "TTT",
        label: "Teğet, teğet, teğet",
    });
    assert_eq!(app.session.tool_id(), "circle");
    assert_eq!(
        app.session.prompt().text(),
        "Daire: birinci teğet çizgi, yay ya da daireyi seçin"
    );
    let _ = app.update(Message::RunMethod {
        id: "tool.arc",
        option: "M",
        label: "Merkez, başlangıç, bitiş",
    });
    assert_eq!(app.session.tool_id(), "arc");
    assert_eq!(app.session.prompt().text(), "Yay: yayın merkezini belirtin");
    assert!(
        !app.log.said(Level::Warn, "başlatılamadı"),
        "both methods started"
    );

    // With no drawing open the tool does not start; that says why, once.
    let (mut closed, _) = App::boot(None);
    let before = closed.log.len();
    let _ = closed.update(Message::RunMethod {
        id: "tool.circle",
        option: "2N",
        label: "2 nokta",
    });
    assert!(!closed.session.is_running());
    assert_eq!(closed.log.len(), before + 1);
    assert_eq!(
        last_output(&closed),
        crate::keys::shown("Açık çizim yok. Önce bir çizim açın (Ctrl+O).")
    );
}

fn last_output(app: &App) -> &str {
    match app.log.last() {
        Some(line) if line.level == Level::Info || line.level == Level::Success => &line.text,
        other => panic!("expected an output line, found {other:?}"),
    }
}

#[test]
fn undo_and_redo_run_through_the_document_and_say_what_they_did() {
    let mut app = with_demo();
    assert!(!app.available("edit.undo") && !app.available("edit.redo"));
    let _ = app.run("edit.undo");
    assert_eq!(last_output(&app), "Geri alınacak değişiklik yok.");

    // An edit made on the document directly.
    let doc = app.document.as_mut().expect("open");
    let first = doc.model.entities().next().expect("an object").clone();
    let count = doc.entity_count();
    doc.model.add(first).expect("a slot");
    assert!(app.available("edit.undo"));

    let _ = app.run("edit.undo");
    assert_eq!(last_output(&app), "Geri alındı: Ekle");
    let doc = app.document.as_ref().expect("open");
    assert_eq!(doc.entity_count(), count);
    assert!(doc.dirty(), "an undo is a change to save");
    assert!(app.available("edit.redo") && !app.available("edit.undo"));

    let _ = app.run("edit.redo");
    assert_eq!(last_output(&app), "Yinelendi: Ekle");
    assert_eq!(
        app.document.as_ref().map(Document::entity_count),
        Some(count + 1)
    );
    let _ = app.run("edit.redo");
    assert_eq!(last_output(&app), "Yinelenecek değişiklik yok.");
}

#[test]
fn a_save_that_finishes_after_another_drawing_was_opened_leaves_that_one_alone() {
    let mut app = with_demo();
    let first = app.document.as_ref().map(|doc| doc.session).expect("open");
    // The first drawing's save is still running when another is opened.
    let second = with_demo().document.expect("open");
    assert_ne!(second.session, first);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(second)))));
    app.document.as_mut().expect("open").model.mark_unsaved();
    let _ = app.update(Message::Saved(Some(Ok(Written {
        session: first,
        path: PathBuf::from("ilk.kcad"),
        revision: 0,
    }))));
    let doc = app.document.as_ref().expect("open");
    assert_eq!(
        doc.path, None,
        "Ctrl+S must not write this drawing over the first one's file"
    );
    assert!(doc.dirty());
    assert_eq!(last_output(&app), "Kaydedildi: ilk.kcad.");
}

/// Another editor's deletion comes in from outside (docs/adr/0040): the
/// revision stays, so nothing is to save, but the store, the selection
/// and the screen follow the generation and let the object go.
#[test]
fn changes_from_outside_reach_the_store_and_the_selection() {
    let (mut app, _) = App::boot(None);
    let demo = with_demo().document.expect("open");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(demo)))));
    let doc = app.document.as_ref().expect("open");
    let slot = kentos_domain::Slot(doc.model.entities().next().expect("an object").base().id);
    let uid = doc.model.uid(slot).expect("a persistent id");
    let (revision, objects) = (doc.model.revision(), app.spatial.len());
    assert!(objects > 0, "the store follows the drawing");
    app.selection.set([slot]);
    let _ = app.update(Message::Modifiers(keyboard::Modifiers::default()));
    assert_eq!(app.selection.len(), 1);

    let doc = app.document.as_mut().expect("open");
    doc.model
        .apply_external(kentos_domain::External {
            remove: vec![uid],
            ..Default::default()
        })
        .expect("taken in");
    let _ = app.update(Message::Modifiers(keyboard::Modifiers::default()));
    assert!(app.selection.is_empty(), "the selection lets it go");
    assert_eq!(app.spatial.len(), objects - 1, "the store follows");
    let doc = app.document.as_ref().expect("open");
    assert_eq!(doc.model.revision(), revision);
    assert!(!doc.dirty(), "a change from outside is nothing to save");
}

#[test]
fn layer_changes_are_edits_but_not_undo_steps_and_folding_is_neither() {
    let mut app = with_demo();
    let group = app
        .document
        .as_ref()
        .and_then(|doc| doc.layers().iter().find(|n| !n.children.is_empty()))
        .map(|n| n.id.clone())
        .expect("the demo has a group");
    let _ = app.update(Message::LayerExpanded(group.clone()));
    let doc = app.document.as_ref().expect("open");
    assert!(doc.find(&group).is_some_and(|n| !n.expanded));
    assert!(!doc.dirty(), "folding is not an edit (web)");

    let _ = app.update(Message::LayerVisible(group.clone()));
    let _ = app.update(Message::LayerLocked(group));
    let _ = app.run("layer.showAll");
    let doc = app.document.as_ref().expect("open");
    assert!(doc.dirty());
    assert!(
        !app.available("edit.undo"),
        "visibility and lock are not undo steps (web)"
    );
}

#[test]
fn the_web_keys_undo_and_redo() {
    use iced::Event;
    use keyboard::key::{Code, Physical};
    use keyboard::{Key, Location, Modifiers};
    let chord = |modifiers: Modifiers, letter: &str| {
        let message = keys::key_event(
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Character(letter.into()),
                modified_key: Key::Character(letter.into()),
                physical_key: Physical::Code(Code::KeyZ),
                location: Location::Standard,
                modifiers,
                text: None,
                repeat: false,
            }),
            event::Status::Ignored,
            window::Id::unique(),
        );
        let Some(Message::Key(press)) = message else {
            panic!("a key press for the router, not {message:?}");
        };
        keys::chord(&press).and_then(|chord| App::boot(None).0.shortcut(&chord))
    };
    assert_eq!(chord(Modifiers::CTRL, "z"), Some("edit.undo"));
    assert_eq!(chord(Modifiers::CTRL, "y"), Some("edit.redo"));
    assert_eq!(
        chord(Modifiers::CTRL | Modifiers::SHIFT, "Z"),
        Some("edit.redo")
    );
}

#[test]
fn typed_names_find_commands_whatever_the_case_and_turkish_letters() {
    let (mut app, _) = App::boot(None);
    app.mode = Mode::Light;
    let _ = app.update(Message::CommandRun("temayı değiştir".into()));
    assert_eq!(app.mode, Mode::Dark, "the title works as a name");
    let _ = app.update(Message::CommandRun("OLMAYAN".into()));
    assert_eq!(
        app.log.last().map(|l| (l.level, l.text.as_str())),
        Some((
            Level::Error,
            "“OLMAYAN” adında bir komut yok. Tüm komutlar ve kısayollar için F1’e basın."
        ))
    );
    assert_eq!(
        app.typed,
        [Entry::Input("temayı değiştir".into())],
        "an unknown name is not brought back by ↑"
    );
}

#[test]
fn a_typed_tool_is_said_once_by_its_own_name() {
    let mut app = with_demo();
    let before = app.log.len();
    let _ = app.update(Message::CommandRun("çizgi".into()));
    assert_eq!(app.session.tool_id(), "line");
    // The tool's name, once (the web logs the tool, not the typed text).
    assert_eq!(app.log.len(), before + 1);
    assert_eq!(
        app.log.last().map(|l| (l.level, l.text.as_str())),
        Some((Level::Command, "Çizgi"))
    );
    // A command that is not a tool says nothing of what was typed (the web's
    // CommandLine); the line's ↑ brings both back, newest first.
    let _ = app.run("tool.cancel");
    let before = app.log.len();
    let _ = app.update(Message::CommandRun("ZE".into()));
    assert_eq!(app.log.len(), before);
    assert_eq!(
        app.typed,
        [Entry::Input("çizgi".into()), Entry::Input("ZE".into())]
    );
}

/// A value or an option given to the running command is echoed as typed
/// (the web's `› X`), never taken for a command's short name: X is also
/// Patlat's (docs/adr/0061).
#[test]
fn a_value_or_an_option_is_echoed_as_typed() {
    let mut app = with_demo();
    let _ = app.run("tool.dimension");
    assert_eq!(app.session.tool_id(), "dimension");
    let _ = app.prompt_option("D");
    assert_eq!(
        app.log.last().map(|l| (l.level, l.text.as_str())),
        Some((Level::Command, "› D"))
    );
    // The typed point, then the tool's echo of it; the line's ↑ brings back
    // what was typed there, not the option's letter.
    let before = app.log.last_id();
    let _ = app.submit_line("0,0");
    assert_eq!(
        app.log
            .lines()
            .find(|l| l.id > before)
            .map(|l| (l.level, l.text.as_str())),
        Some((Level::Command, "› 0,0"))
    );
    assert_eq!(app.typed, [Entry::Input("0,0".into())]);
    assert_eq!(
        app.session.prompt().text(),
        "Ölçü: ikinci ölçü noktasını belirtin"
    );
}

#[test]
fn a_tool_needs_an_open_drawing_and_says_so() {
    let (mut app, _) = App::boot(None);
    let _ = app.run("tool.polygon");
    assert!(!app.session.is_running());
    assert_eq!(
        last_output(&app),
        crate::keys::shown("Açık çizim yok. Önce bir çizim açın (Ctrl+O).")
    );
    let mut app = with_demo();
    let _ = app.run("tool.polygon");
    assert_eq!(app.session.tool_id(), "polygon");
    assert_eq!(
        app.log.last().map(|l| (l.level, l.text.as_str())),
        Some((Level::Command, "Kapalı alan"))
    );
    // Opening another drawing drops the command and its draft.
    let other = with_demo().document.expect("open");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(other)))));
    assert!(!app.session.is_running());
}

/// The line and polyline tools (docs/adr/0027) start from their web
/// shortcuts (L, P) and from their names in the command line.
#[test]
fn the_line_and_polyline_tools_start_from_their_keys_and_names() {
    use keyboard::key::{Code, Physical};
    use keyboard::{Key, Modifiers};
    let letter = |c: &str, code: Code| {
        Message::Key(KeyPress {
            key: Key::Character(c.into()),
            physical: Physical::Code(code),
            modifiers: Modifiers::empty(),
            text: Some(c.to_owned()),
            repeat: false,
        })
    };
    let mut app = with_demo();
    let _ = app.update(letter("l", Code::KeyL));
    assert_eq!(app.session.tool_id(), "line");
    let _ = app.run("tool.cancel");
    let _ = app.update(letter("p", Code::KeyP));
    assert_eq!(app.session.tool_id(), "polyline");
    for (name, tool) in [
        ("çizgi", "line"),
        ("PL", "polyline"),
        ("coklucizgi", "polyline"),
    ] {
        let _ = app.run("tool.cancel");
        let _ = app.update(Message::CommandRun(name.into()));
        assert_eq!(app.session.tool_id(), tool, "{name}");
    }
}

/// While a command runs the command line suggests its options, never
/// another command: a name typed there goes to the tool, as on the web
/// (docs/adr/0027), and the draft stays.
#[test]
fn a_running_command_owns_what_is_typed() {
    use kentos_ui::widget::command_line::{Suggested, suggested};
    let mut app = with_demo();
    assert!(!app.line_commands().is_empty());
    let _ = app.run("tool.polygon");
    assert!(app.line_commands().is_empty());
    let _ = app.update(Message::Viewport(viewport::Event::Pressed(
        iced::Point::new(10.0, 10.0),
    )));
    assert_eq!(app.session.point_count(), 1);
    // After the first corner the prompt has options; they are still suggested.
    let offered = suggested(&app.line_commands(), app.line_prompt().as_ref(), "Ya");
    assert!(
        matches!(offered.as_slice(), [Suggested::Option { label, .. }] if label == "Yay"),
        "{offered:?}"
    );
    let _ = app.update(Message::CommandInput("KA".into()));
    let _ = app.update(Message::CommandSubmitted);
    assert_eq!(app.session.tool_id(), "polygon");
    assert_eq!(app.session.point_count(), 1, "the draft stays");
    assert!(
        app.log
            .last()
            .is_some_and(|l| l.level == Level::Warn && l.text.contains("“KA” anlaşılamadı"))
    );
}

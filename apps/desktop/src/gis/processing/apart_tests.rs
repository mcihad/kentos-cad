//! İşlemler's tool window in a window of its own (apart.rs, docs/adr/8060): taken out and brought back, closed by its
//! close button and by Kapat, the choice kept for the next tool, its keys kept from the main window, a pick on the
//! drawing handing the keyboard over, and the main window's close taking it along.

use iced::futures::{FutureExt as _, StreamExt as _};
use iced::keyboard::key::{Code, Named, NativeCode, Physical};
use iced::keyboard::{Key, Modifiers};
use iced::{Task, window};
use kentos_domain::Slot;
use serde_json::json;

use crate::app::{App, Dialog, Message, Then};
use crate::gis::GisDialogKind;
use crate::keys::KeyPress;
use crate::processing::Event as Tool;
use crate::windows::{Event, Surface};

const TOOL: &str = "processing.run.annotation.edgeLengths";
const OVER_DRAWING: Option<Dialog> = Some(Dialog::Gis(GisDialogKind::Processing));

/// What a task asks of the runtime, as far as the windows go.
#[derive(Debug, PartialEq)]
enum Did {
    Open(window::Id),
    Close(window::Id),
    Focus(window::Id),
    Exit,
    Other,
}

/// The actions a task gives at once; one waiting for the world (a file, the network) ends the list.
fn did(task: Task<Message>) -> Vec<Did> {
    let Some(mut stream) = iced_runtime::task::into_stream(task) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    while let Some(Some(action)) = stream.next().now_or_never() {
        // Each action dropped as it is read: an open's answer channel goes with it and the stream ends.
        out.push(match action {
            iced_runtime::Action::Window(iced_runtime::window::Action::Open(id, ..)) => {
                Did::Open(id)
            }
            iced_runtime::Action::Window(iced_runtime::window::Action::Close(id)) => Did::Close(id),
            iced_runtime::Action::Window(iced_runtime::window::Action::GainFocus(id)) => {
                Did::Focus(id)
            }
            iced_runtime::Action::Exit => Did::Exit,
            _ => Did::Other,
        });
    }
    out
}

/// The window actions of a task, in a stable order (a batch gives them in any).
fn windows(task: Task<Message>) -> Vec<Did> {
    let mut out: Vec<Did> = did(task).into_iter().filter(|d| *d != Did::Other).collect();
    out.sort_by_key(|d| match d {
        Did::Open(_) => 0,
        Did::Close(_) => 1,
        Did::Focus(_) => 2,
        Did::Exit => 3,
        Did::Other => 4,
    });
    out
}

fn tool(app: &mut App, e: Tool) -> Task<Message> {
    app.update(Message::Gis(crate::gis::Event::Processing(e)))
}

fn press(key: Key, physical: Physical, modifiers: Modifiers, text: Option<&str>) -> KeyPress {
    KeyPress {
        key,
        physical,
        modifiers,
        text: text.map(str::to_owned),
        repeat: false,
    }
}

fn named(named: Named, modifiers: Modifiers) -> KeyPress {
    press(
        Key::Named(named),
        Physical::Unidentified(NativeCode::Unidentified),
        modifiers,
        None,
    )
}

fn letter_l() -> KeyPress {
    press(
        Key::Character("l".into()),
        Physical::Code(Code::KeyL),
        Modifiers::empty(),
        Some("l"),
    )
}

/// The parcels with the main window known, as the daemon has it, three parcels selected and a tool open over the
/// drawing.
fn app_with_tool() -> (App, window::Id) {
    let mut app = crate::processing::tests::app_with_parcels();
    let main = window::Id::unique();
    app.windows.set_main(main);
    app.selection.set([Slot(1), Slot(2), Slot(7)]);
    let _ = app.update(Message::Run(TOOL));
    (app, main)
}

fn ran(app: &App) -> bool {
    matches!(
        app.gis.processing.dialog.as_ref().map(|w| &w.status),
        Some(crate::processing::RunStatus::Ok { .. })
    )
}

/// The tool taken out into its own window, the platform having made it.
fn apart(app: &mut App) -> window::Id {
    let asked = windows(app.update(Message::Windows(Event::Apart(Surface::ProcessingTool, true))));
    let w = app.windows.of(Surface::ProcessingTool).expect("its window");
    assert_eq!(
        asked,
        [Did::Open(w.id)],
        "the window is asked of the platform"
    );
    assert!(!w.opened);
    let _ = app.update(Message::Windows(Event::Opened(w.id)));
    w.id
}

#[test]
fn the_tool_window_goes_into_its_own_window_and_comes_back() {
    let (mut app, main) = app_with_tool();
    assert_eq!(app.dialog, OVER_DRAWING);
    let id = apart(&mut app);
    // The drawing is free: no window over it; the tool is the same, in its own window, named after it.
    assert_eq!(app.dialog, None);
    assert!(app.gis.processing.dialog.is_some());
    assert!(app.processing_is_apart());
    assert_eq!(app.window_title(id), "Kenar uzunluklarını yaz — KentOS CAD");
    assert_eq!(app.window_title(main), app.title());
    let settings = app.window_settings(Surface::ProcessingTool);
    assert!(!settings.exit_on_close_request, "its close is Kapat");
    assert!(settings.min_size.is_some());
    let _ = app.window_view(id);
    let _ = app.window_view(main);
    // A window no longer known shows nothing, never the main window's view.
    let _ = app.window_view(window::Id::unique());

    // Running it from its window: the run goes on as over the drawing, the window stays.
    let _ = tool(&mut app, Tool::Run);
    assert!(ran(&app));
    assert_eq!(
        app.windows.of(Surface::ProcessingTool).map(|w| w.id),
        Some(id)
    );
    // Sonuçları seç keeps it: it covers nothing of the drawing.
    let _ = tool(&mut app, Tool::Results);
    assert!(app.gis.processing.dialog.is_some());
    assert!(!app.selection.is_empty(), "what it made is selected");

    // Ana pencereye al: back over the drawing, its window closed, the main window given the keyboard.
    let back = windows(app.update(Message::Windows(Event::Apart(Surface::ProcessingTool, false))));
    assert_eq!(app.dialog, OVER_DRAWING);
    assert!(app.gis.processing.dialog.is_some());
    assert_eq!(back, [Did::Close(id), Did::Focus(main)]);
    assert_eq!(app.windows.of(Surface::ProcessingTool), None);
    assert!(!app.processing_is_apart());
}

#[test]
fn closing_the_tool_s_window_is_kapat_and_the_next_tool_opens_apart() {
    let (mut app, _) = app_with_tool();
    let id = apart(&mut app);
    let closed = windows(app.update(Message::Windows(Event::CloseRequested(id))));
    assert!(app.gis.processing.dialog.is_none(), "as Kapat");
    assert_eq!(closed, [Did::Close(id)]);
    assert_eq!(app.windows.of(Surface::ProcessingTool), None);
    // The platform's word that it closed comes after: nothing more happens.
    assert!(windows(app.update(Message::Windows(Event::Closed(id)))).is_empty());

    // The choice is kept: the next tool opens in its own window, the drawing left free.
    let opened = windows(app.update(Message::Run("processing.run.points.numberVertices")));
    let w = app
        .windows
        .of(Surface::ProcessingTool)
        .expect("its window again");
    assert_eq!(opened, [Did::Open(w.id)]);
    assert_eq!(app.dialog, None);
    assert_eq!(app.processing_window_title(), "Köşe noktalarını numarala");

    // Kapat from the footer (or Esc) closes the window too.
    let _ = app.update(Message::Windows(Event::Opened(w.id)));
    let closed = windows(tool(&mut app, Tool::Close));
    assert_eq!(closed, [Did::Close(w.id)]);
    // A window closed before the platform made it goes when it is made.
    let _ = app.update(Message::Run(TOOL));
    let late = app.windows.of(Surface::ProcessingTool).expect("asked").id;
    let _ = tool(&mut app, Tool::Close);
    assert_eq!(
        app.windows.of(Surface::ProcessingTool).map(|w| w.id),
        Some(late),
        "kept until it is made"
    );
    assert_eq!(
        windows(app.update(Message::Windows(Event::Opened(late)))),
        [Did::Close(late)]
    );
    assert_eq!(app.windows.of(Surface::ProcessingTool), None);
}

#[test]
fn keys_typed_in_the_tool_s_window_never_reach_the_main_window() {
    let (mut app, _) = app_with_tool();
    let id = apart(&mut app);
    // L in its window: no line tool, nothing typed into the command line.
    let _ = app.update(Message::Windows(Event::Key(id, letter_l())));
    assert!(!app.session.is_running(), "{}", app.session.tool_id());
    assert!(app.command_input.is_empty());
    // Delete in its window: the selection stays.
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Windows(Event::Key(
        id,
        named(Named::Delete, Modifiers::empty()),
    )));
    assert!(
        app.document
            .as_ref()
            .expect("open")
            .model
            .get(Slot(1))
            .is_some()
    );
    assert_eq!(app.selection.len(), 1);
    // The same L in the main window, the tool still open apart: the drawing takes it as usual.
    let _ = app.update(Message::Key(letter_l()));
    assert_eq!(app.session.tool_id(), "line");
    let _ = app.run("tool.cancel");

    // Its own keys: Ctrl+Enter runs it, Esc is Kapat.
    app.selection.set([Slot(1), Slot(2), Slot(7)]);
    let _ = app.update(Message::Windows(Event::Key(
        id,
        named(Named::Enter, Modifiers::CTRL),
    )));
    assert!(ran(&app));
    let _ = app.update(Message::Windows(Event::Key(
        id,
        named(Named::Escape, Modifiers::empty()),
    )));
    assert!(app.gis.processing.dialog.is_none());
    // A key of a window no longer known does nothing.
    let _ = app.update(Message::Windows(Event::Key(id, letter_l())));
    assert!(!app.session.is_running());
}

#[test]
fn a_pick_on_the_drawing_gives_the_main_window_the_keyboard_and_takes_it_back() {
    let mut app = crate::processing::tests::app_with_parcels();
    let main = window::Id::unique();
    app.windows.set_main(main);
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("processing.run.points.numberVertices"));
    let id = apart(&mut app);
    let _ = tool(&mut app, Tool::Value("start".into(), json!("point")));
    let away = windows(tool(&mut app, Tool::Pick("startPoint".into())));
    assert_eq!(away, [Did::Focus(main)], "the drawing has the work");
    assert_eq!(app.dialog, None, "no window comes over the drawing");
    assert_eq!(
        app.windows.of(Surface::ProcessingTool).map(|w| w.id),
        Some(id),
        "its window waits"
    );
    let _ = app.window_view(id);
    // Esc in its window gives the pick up: the form comes back in its window, which takes the keyboard back.
    let back = windows(app.update(Message::Windows(Event::Key(
        id,
        named(Named::Escape, Modifiers::empty()),
    ))));
    assert!(app.gis.processing.picking.is_none());
    assert!(app.gis.processing.dialog.is_some());
    assert_eq!(app.dialog, None);
    assert_eq!(back, [Did::Focus(id)]);
    // A point given in the main window comes back into the form in its own window.
    let _ = tool(&mut app, Tool::Pick("startPoint".into()));
    let _ = app.submit_line("487010,4420010");
    let window = app.gis.processing.dialog.as_ref().expect("back");
    assert_eq!(
        window.values.get("startPoint"),
        Some(&json!({ "x": 487010.0, "y": 4420010.0 }))
    );
    assert_eq!(app.dialog, None);
    assert_eq!(
        app.windows.of(Surface::ProcessingTool).map(|w| w.id),
        Some(id)
    );
}

#[test]
fn closing_the_main_window_closes_the_others_and_ends_the_program() {
    let (mut app, main) = app_with_tool();
    let id = apart(&mut app);
    // Nothing unsaved: the question is not asked, the windows go and the program ends.
    let gone = windows(app.proceed(Then::Close(main)));
    assert_eq!(gone, [Did::Close(id), Did::Close(main), Did::Exit]);
    assert!(app.windows.closing());
    assert_eq!(app.windows.of(Surface::ProcessingTool), None);
    // Nothing opens any more while it ends.
    assert!(windows(app.update(Message::Run(TOOL))).is_empty());

    // Through the close button: its unsaved changes are asked about first, the tool's window waits.
    let (mut app, main) = app_with_tool();
    let id = apart(&mut app);
    let _ = tool(&mut app, Tool::Run);
    assert!(
        app.document.as_ref().expect("open").dirty(),
        "the run wrote"
    );
    let asked = windows(app.update(Message::CloseRequested(main)));
    assert!(asked.is_empty(), "{asked:?}");
    assert!(matches!(app.dialog, Some(Dialog::Unsaved(Then::Close(_)))));
    assert_eq!(
        app.windows.of(Surface::ProcessingTool).map(|w| w.id),
        Some(id)
    );
    let gone = windows(app.update(Message::DialogConfirmed));
    assert_eq!(gone, [Did::Close(id), Did::Close(main), Did::Exit]);
}

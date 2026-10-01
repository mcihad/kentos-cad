//! Real keyboard and pointer events exercise the composed native surfaces.

use iced::keyboard::{self, key};
use iced::{Element, Event, Point, Size};

use crate::snapshot::{Input, Snapshot};

use super::python::{CompletionEvent, CompletionItem, EditorState, PythonEditor, SymbolKind};

fn enter(modifiers: keyboard::Modifiers) -> Event {
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(key::Named::Enter),
        modified_key: keyboard::Key::Named(key::Named::Enter),
        physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    })
}

#[derive(Clone)]
enum PythonEvent {
    Edit(iced::widget::text_editor::Action),
    Complete(CompletionEvent),
    Run,
}
#[derive(Default)]
struct PythonState {
    editor: EditorState,
    runs: usize,
}
fn python_view(state: &PythonState) -> Element<'_, PythonEvent> {
    PythonEditor::new(&state.editor.content, PythonEvent::Edit)
        .header(false)
        .footer(false)
        .completions(&state.editor.completion, PythonEvent::Complete)
        .on_run(PythonEvent::Run)
        .into()
}
fn python_update(state: &mut PythonState, event: PythonEvent) {
    match event {
        PythonEvent::Edit(action) => state.editor.perform(action),
        PythonEvent::Complete(event) => {
            state.editor.complete(event);
        }
        PythonEvent::Run => state.runs += 1,
    }
}

#[test]
fn editor_keyboard_indents_unicode_and_run_shortcut_is_owned_by_the_host() {
    let mut snapshot = Snapshot::software(Size::new(420.0, 210.0)).unwrap();
    let mut state = PythonState::default();
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Click(Point::new(85.0, 20.0)),
    );
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Type("def ölç():".into()),
    );
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Key(key::Named::Enter),
    );
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Type("return 42".into()),
    );
    assert_eq!(state.editor.content.text(), "def ölç():\n    return 42");
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Key(key::Named::F5),
    );
    assert_eq!(state.runs, 1);
}

#[test]
fn editor_reveals_long_lines_and_scrolled_rows_before_pointer_selection() {
    let mut snapshot = Snapshot::software(Size::new(420.0, 210.0)).unwrap();
    let mut state = PythonState {
        editor: EditorState::with_text(&"a".repeat(220)),
        runs: 0,
    };
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Click(Point::new(85.0, 20.0)),
    );
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Key(key::Named::End),
    );
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Click(Point::new(380.0, 20.0)),
    );
    assert!(
        state.editor.content.cursor().position.column > 180,
        "the visible end of a long line must remain clickable"
    );
    state.editor =
        EditorState::with_text(&(0..80).map(|i| format!("line_{i}\n")).collect::<String>());
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Scroll(Point::new(160.0, 50.0), -6.0),
    );
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Click(Point::new(100.0, 24.0)),
    );
    assert!(state.editor.content.cursor().position.line >= 5);
}

#[test]
fn completion_tab_enter_and_pointer_replace_the_correct_range_and_undo_once() {
    use iced::widget::text_editor::{Action, Motion};
    let mut snapshot = Snapshot::software(Size::new(700.0, 450.0)).unwrap();
    let mut state = PythonState::default();
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Click(Point::new(85.0, 20.0)),
    );
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Type("pri".into()),
    );
    assert_eq!(state.editor.completion.selected().unwrap().name, "print");
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Key(key::Named::Tab),
    );
    assert_eq!(state.editor.content.text(), "print");
    state.editor.undo();
    assert_eq!(state.editor.content.text(), "pri");

    state.editor = EditorState::with_text("import ma");
    state
        .editor
        .content
        .perform(Action::Move(Motion::DocumentEnd));
    state.editor.complete(CompletionEvent::Request);
    let request = state.editor.completion.request.clone().unwrap();
    let mut item = CompletionItem::new("math", SymbolKind::Module);
    item.detail = "math".into();
    item.documentation = "Mathematical functions.".into();
    state
        .editor
        .completion
        .receive(request.generation, vec![item]);
    snapshot.settle(&mut state, python_view, &mut python_update);
    snapshot.step(
        &mut state,
        python_view,
        &mut python_update,
        &[enter(keyboard::Modifiers::empty())],
    );
    assert_eq!(state.editor.content.text(), "import math");

    state.editor = EditorState::with_text("pri");
    state
        .editor
        .content
        .perform(Action::Move(Motion::DocumentEnd));
    state.editor.complete(CompletionEvent::Request);
    snapshot.settle(&mut state, python_view, &mut python_update);
    // The same real pointer events reach the overlay and keep editor focus.
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Click(Point::new(155.0, 85.0)),
    );
    assert_eq!(state.editor.content.text(), "print");
    snapshot.input(
        &mut state,
        python_view,
        &mut python_update,
        Input::Type("(42)".into()),
    );
    assert_eq!(state.editor.content.text(), "print(42)");
}

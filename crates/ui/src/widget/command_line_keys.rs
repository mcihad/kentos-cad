//! Actual composed widget events: command chords must reach the host without typing.
use iced::advanced::{
    Clipboard, Layout, Shell, layout,
    widget::{Tree, operation},
};
use iced::keyboard::{
    self, Key, Modifiers,
    key::{Code, Physical},
};
use iced::{Element, Event, Point, Rectangle, Size, event, mouse};

use super::CommandLine;
use crate::snapshot::Snapshot;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Input(String),
    Focus(bool),
}
#[derive(Default)]
struct Memory(String);
impl Clipboard for Memory {
    fn read(&self, _: iced::advanced::clipboard::Kind) -> Option<String> {
        Some(self.0.clone())
    }
    fn write(&mut self, _: iced::advanced::clipboard::Kind, text: String) {
        self.0 = text;
    }
}
struct Harness {
    snapshot: Snapshot,
    view: Element<'static, Message>,
    tree: Tree,
    node: layout::Node,
    clipboard: Memory,
    cursor: Point,
}
impl Harness {
    fn new() -> Self {
        let snapshot = Snapshot::software(Size::new(800., 100.)).expect("software renderer");
        let id = iced::advanced::widget::Id::from("command-chord-test");
        let mut view: Element<'static, Message> = CommandLine::new(&[], "room")
            .id(id.clone())
            .on_input(Message::Input)
            .on_focus(Message::Focus)
            .into();
        let mut tree = Tree::new(view.as_widget());
        let node = view.as_widget_mut().layout(
            &mut tree,
            snapshot.renderer(),
            &layout::Limits::new(Size::ZERO, Size::new(800., 100.)),
        );
        // This is the same focus operation delivered after Tab, without a pointer press.
        view.as_widget_mut().operate(
            &mut tree,
            Layout::new(&node),
            snapshot.renderer(),
            &mut operation::focusable::focus(id),
        );
        Self {
            snapshot,
            view,
            tree,
            node,
            clipboard: Memory("copied".into()),
            cursor: Point::new(700., 30.),
        }
    }
    fn cell() -> Self {
        let snapshot = Snapshot::software(Size::new(800., 100.)).expect("software renderer");
        let mut view: Element<'static, Message> =
            crate::widget::EditCell::new("room", Message::Input).into();
        let mut tree = Tree::new(view.as_widget());
        let node = view.as_widget_mut().layout(
            &mut tree,
            snapshot.renderer(),
            &layout::Limits::new(Size::ZERO, Size::new(800., 100.)),
        );
        let mut h = Self {
            snapshot,
            view,
            tree,
            node,
            clipboard: Memory("copied".into()),
            cursor: Point::new(700., 12.),
        };
        h.send(Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        )));
        h
    }
    fn send(&mut self, event: Event) -> (event::Status, Vec<Message>) {
        let mut messages = vec![];
        let mut shell = Shell::new(&mut messages);
        self.view.as_widget_mut().update(
            &mut self.tree,
            &event,
            Layout::new(&self.node),
            mouse::Cursor::Available(self.cursor),
            self.snapshot.renderer(),
            &mut self.clipboard,
            &mut shell,
            &Rectangle::with_size(Size::new(800., 100.)),
        );
        (shell.event_status(), messages)
    }
    fn key(&mut self, c: &str, code: Code, modifiers: Modifiers) -> (event::Status, Vec<Message>) {
        self.send(Event::Keyboard(keyboard::Event::ModifiersChanged(
            modifiers,
        )));
        self.key_from_event(c, code, modifiers)
    }
    fn key_from_event(
        &mut self,
        c: &str,
        code: Code,
        modifiers: Modifiers,
    ) -> (event::Status, Vec<Message>) {
        self.send(Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Character(c.into()),
            modified_key: Key::Character(c.into()),
            physical_key: Physical::Code(code),
            location: keyboard::Location::Standard,
            modifiers,
            text: Some(c.into()),
            repeat: false,
        }))
    }
}

#[test]
fn clipboard_uses_the_key_event_when_focus_missed_the_modifier_notification() {
    for (mut h, property) in [(Harness::cell(), true), (Harness::new(), false)] {
        h.key_from_event("a", Code::KeyA, Modifiers::COMMAND);
        h.key_from_event("c", Code::KeyC, Modifiers::COMMAND);
        assert_eq!(
            h.clipboard.0, "room",
            "Copy after fresh focus; property={property}"
        );
        h.clipboard.0 = "pasted".into();
        let (_, messages) = h.key_from_event("v", Code::KeyV, Modifiers::COMMAND);
        if property {
            assert!(
                messages.is_empty(),
                "Clipboard must not commit a property draft"
            );
            let (_, messages) = h.send(Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(keyboard::key::Named::Enter),
                modified_key: Key::Named(keyboard::key::Named::Enter),
                physical_key: Physical::Code(Code::Enter),
                location: keyboard::Location::Standard,
                modifiers: Modifiers::empty(),
                text: None,
                repeat: false,
            }));
            assert_eq!(messages, [Message::Input("pasted".into())]);
        } else {
            assert!(
                messages.contains(&Message::Input("pasted".into())),
                "{messages:?}"
            );
        }
    }
}

#[test]
fn a_plain_key_does_not_reuse_a_previous_command_modifier() {
    let mut h = Harness::new();
    h.key("a", Code::KeyA, Modifiers::COMMAND);
    assert_eq!(
        h.key_from_event("v", Code::KeyV, Modifiers::empty()),
        (event::Status::Captured, vec![Message::Input("v".into())])
    );
}

#[test]
fn command_save_in_an_editing_property_cell_is_not_text_or_a_commit() {
    for modifiers in [Modifiers::LOGO, Modifiers::CTRL] {
        let (status, messages) = Harness::cell().key("s", Code::KeyS, modifiers);
        assert_eq!(
            status,
            event::Status::Ignored,
            "host must receive save from a property cell"
        );
        assert!(
            messages.is_empty(),
            "save must not commit its buffer: {messages:?}"
        );
    }
}

#[test]
fn command_save_after_keyboard_focus_is_not_printable_text() {
    for (text, modifiers) in [
        ("s", Modifiers::CTRL),
        ("s", Modifiers::LOGO),
        ("S", Modifiers::LOGO | Modifiers::SHIFT),
    ] {
        let (status, messages) = Harness::new().key(text, Code::KeyS, modifiers);
        assert_eq!(
            status,
            event::Status::Ignored,
            "the host must receive save: {messages:?}"
        );
        assert!(
            !messages.iter().any(|m| matches!(m, Message::Input(_))),
            "{messages:?}"
        );
    }
}

#[test]
fn a_property_edit_buffer_and_its_clipboard_keys_survive_host_save() {
    let mut h = Harness::cell();
    h.key("a", Code::KeyA, Modifiers::COMMAND);
    assert_eq!(
        h.key("c", Code::KeyC, Modifiers::COMMAND).0,
        event::Status::Captured
    );
    assert_eq!(h.clipboard.0, "room");
    h.clipboard.0 = "copied".into();
    assert_eq!(
        h.key("v", Code::KeyV, Modifiers::COMMAND).0,
        event::Status::Captured
    );
    assert_eq!(
        h.key("s", Code::KeyS, Modifiers::COMMAND).0,
        event::Status::Ignored
    );
    h.send(Event::Keyboard(keyboard::Event::ModifiersChanged(
        Modifiers::empty(),
    )));
    let (_, messages) = h.send(Event::Keyboard(keyboard::Event::KeyPressed {
        key: Key::Named(keyboard::key::Named::Enter),
        modified_key: Key::Named(keyboard::key::Named::Enter),
        physical_key: Physical::Code(Code::Enter),
        location: keyboard::Location::Standard,
        modifiers: Modifiers::empty(),
        text: None,
        repeat: false,
    }));
    assert_eq!(messages, [Message::Input("copied".into())]);
}

#[test]
fn a_newly_focused_line_reports_focus_before_leaving_the_chord_to_its_host() {
    let (status, messages) = Harness::new().send(Event::Keyboard(keyboard::Event::KeyPressed {
        key: Key::Character("s".into()),
        modified_key: Key::Character("s".into()),
        physical_key: Physical::Code(Code::KeyS),
        location: keyboard::Location::Standard,
        modifiers: Modifiers::LOGO,
        text: Some("s".into()),
        repeat: false,
    }));
    assert_eq!(status, event::Status::Ignored);
    assert_eq!(
        messages,
        [Message::Focus(true)],
        "a host must see the actual field focus"
    );
}

#[test]
fn plain_shift_option_and_altgr_input_remain_text() {
    for (text, code, modifiers) in [
        ("s", Code::KeyS, Modifiers::empty()),
        ("S", Code::KeyS, Modifiers::SHIFT),
        ("ş", Code::Semicolon, Modifiers::empty()),
        ("@", Code::KeyQ, Modifiers::CTRL | Modifiers::ALT),
        ("#", Code::Digit3, Modifiers::CTRL | Modifiers::ALT),
        ("€", Code::KeyE, Modifiers::ALT),
    ] {
        let (status, messages) = Harness::new().key(text, code, modifiers);
        assert_eq!(status, event::Status::Captured, "{text} {messages:?}");
        assert!(
            messages
                .iter()
                .any(|m| matches!(m, Message::Input(v) if v.contains(text))),
            "{text} {messages:?}"
        );
    }
}

#[test]
fn platform_select_all_copy_cut_and_paste_still_belong_to_the_field() {
    let mut h = Harness::new();
    assert_eq!(
        h.key("a", Code::KeyA, Modifiers::COMMAND).0,
        event::Status::Captured
    );
    let (status, copied) = h.key("c", Code::KeyC, Modifiers::COMMAND);
    assert_eq!(status, event::Status::Captured);
    assert_eq!(h.clipboard.0, "room");
    assert!(!copied.iter().any(|m| matches!(m, Message::Input(_))));
    assert_eq!(
        h.key("x", Code::KeyX, Modifiers::COMMAND),
        (event::Status::Captured, vec![Message::Input(String::new())])
    );
    let mut h = Harness::new();
    h.key("a", Code::KeyA, Modifiers::COMMAND);
    assert_eq!(
        h.key("v", Code::KeyV, Modifiers::COMMAND),
        (
            event::Status::Captured,
            vec![Message::Input("copied".into())]
        )
    );
}

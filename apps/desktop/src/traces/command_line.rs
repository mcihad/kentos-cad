//! The command line as view.rs configures it, as the player follows it: the
//! KentOS UI command line's own state that the app does not hold (whether
//! its text box has the keyboard, the focus it last reported), what it sends
//! for a key press while it has the keyboard, and what a widget operation
//! from the app's tasks does to it. `the_command_line_model_is_the_widget`
//! holds this model to the real widget, key by key and operation by operation.
//!
//! The suggestion list is the widget's own: its order comes from
//! `kentos_ui::widget::command_line::suggested`, fed with what view.rs gives
//! the widget (`line_commands`, `App::line_prompt`); this model only says
//! when the list is open and what its keys do. With `escape_clears` Esc never
//! leaves the list closed over a text, so the list is open whenever the text
//! box has the keyboard, the text is not blank and something matches.

use iced::Rectangle;
use iced::advanced::widget::Id;
use iced::advanced::widget::operation::{Focusable, Operation, Outcome, TextInput};
use iced::keyboard::key::Named;
use iced::keyboard::{self, Key};
use kentos_ui::widget::command_line::{Suggested, suggested};

use crate::app::{App, COMMAND_INPUT, Message};
use crate::view::line_commands;

/// The widget's state the player follows.
#[derive(Debug, Default)]
pub struct CommandLine {
    /// Whether the text box has the keyboard.
    focused: bool,
    /// The focus the widget last reported to the app (`on_focus`). It
    /// reports a change at the next event it sees; the runtime redraws after
    /// every widget operation, so a change from a task reaches the app before
    /// any key does.
    reported: bool,
}

impl CommandLine {
    /// Whether the text box has the keyboard.
    pub fn has_keyboard(&self) -> bool {
        self.focused
    }

    /// A click on the text box: it takes the keyboard, and says so.
    pub fn click(&mut self) -> Option<Message> {
        self.focused = true;
        self.report()
    }

    /// A click anywhere else: it lets the keyboard go, and says so.
    pub fn click_elsewhere(&mut self) -> Option<Message> {
        self.focused = false;
        self.report()
    }

    /// A widget operation from the app's tasks, run on the text box as the
    /// runtime runs it on the window's widgets (chained operations too): a
    /// focus change, reported. The command line's text box is the only
    /// widget with an id the player follows; an operation that does not act
    /// on its focus (a text selection, a custom operation) cannot be
    /// followed, and stops the trace rather than let the widget's state go
    /// wrong unseen.
    pub fn operate(&mut self, operation: &mut dyn Operation) -> Result<Option<Message>, String> {
        let mut text_box = TextBox {
            focused: self.focused,
            touched: false,
            unfollowed: None,
        };
        let id = Id::new(COMMAND_INPUT);
        let mut visit = |operation: &mut dyn Operation| {
            let bounds = Rectangle::default();
            operation.container(None, bounds);
            operation.traverse(&mut |operation| {
                operation.focusable(Some(&id), bounds, &mut text_box);
                operation.text_input(Some(&id), bounds, &mut text_box);
            });
            match operation.finish() {
                Outcome::Chain(next) => Some(next),
                _ => None,
            }
        };
        let mut next = visit(operation);
        while let Some(mut operation) = next.take() {
            next = visit(operation.as_mut());
        }
        if let Some(what) = text_box.unfollowed {
            return Err(format!(
                "iz, oynatıcının izleyemediği bir widget işlemi üretti: komut satırında {what}"
            ));
        }
        if !text_box.touched {
            return Err(
                "iz, oynatıcının izleyemediği bir widget işlemi üretti: komut satırının odağına dokunmuyor"
                    .to_owned(),
            );
        }
        self.focused = text_box.focused;
        Ok(self.report())
    }

    /// What the widget sends for a key press while its text box has the
    /// keyboard; `None` when it lets the key through to the subscription.
    /// `app` is the app before the key.
    pub fn key(&mut self, app: &App, event: &iced::Event) -> Option<Vec<Message>> {
        let iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modifiers,
            text,
            ..
        }) = event
        else {
            return None;
        };
        let value = app.command_input.as_str();
        let plain = !modifiers.command() && !modifiers.alt();
        let suggestions = suggested(&line_commands(), app.line_prompt().as_ref(), value);
        let open = !value.trim().is_empty() && !suggestions.is_empty();
        if let Key::Named(named) = key {
            // Esc on an empty line with the list closed ends the command and lets the keyboard go.
            if *named == Named::Escape && plain && !open && value.is_empty() {
                self.focused = false;
                let mut messages = vec![Message::CommandCancelled];
                messages.extend(self.report());
                return Some(messages);
            }
            if plain && let Some(messages) = list_key(*named, open, value, &suggestions) {
                return Some(messages);
            }
            // The text box's own keys.
            match named {
                Named::Enter => return Some(vec![Message::CommandSubmitted]),
                Named::Backspace => {
                    let mut value = value.to_owned();
                    value.pop();
                    return Some(vec![Message::CommandInput(value)]);
                }
                _ => {}
            }
        }
        text.as_deref()
            .and_then(|t| t.chars().next())
            .filter(|c| !c.is_control())
            .map(|c| vec![Message::CommandInput(format!("{value}{c}"))])
    }

    /// The widget's `on_focus`: the focus when it differs from the last report.
    fn report(&mut self) -> Option<Message> {
        (self.reported != self.focused).then(|| {
            self.reported = self.focused;
            Message::CommandFocus(self.focused)
        })
    }
}

/// A key the widget takes before its text box: the suggestion list's, and
/// Space and Esc as view.rs configures them (`space_submits`,
/// `escape_clears`). The list's arrows are not in the traces' keys.
fn list_key(
    named: Named,
    open: bool,
    value: &str,
    suggestions: &[Suggested<Message>],
) -> Option<Vec<Message>> {
    let first = suggestions.first().filter(|_| open);
    match (named, first) {
        // Tab writes the highlighted suggestion's name into the line.
        (Named::Tab, Some(first)) => Some(vec![Message::CommandInput(match first {
            Suggested::Option { label, .. } => label.clone(),
            Suggested::Command(name) => name.clone(),
        })]),
        // Enter, and Space as a second Enter, run the highlighted suggestion.
        (Named::Enter | Named::Space, Some(first)) => Some(match first {
            Suggested::Option { message, .. } => {
                vec![message.clone(), Message::CommandInput(String::new())]
            }
            Suggested::Command(name) => vec![Message::CommandRun(name.clone())],
        }),
        (Named::Space, None) => Some(vec![Message::CommandSubmitted]),
        // Esc clears what is typed, the list or not (ADR 0018).
        (Named::Escape, _) if !value.is_empty() => Some(vec![Message::CommandInput(String::new())]),
        _ => None,
    }
}

/// The command line's text box as a widget operation sees it.
struct TextBox {
    focused: bool,
    /// The operation focused it or let it go.
    touched: bool,
    /// What the operation did that the model cannot follow.
    unfollowed: Option<&'static str>,
}

impl Focusable for TextBox {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
        self.touched = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
        self.touched = true;
    }
}

impl TextInput for TextBox {
    fn text(&self) -> &str {
        ""
    }

    fn move_cursor_to_front(&mut self) {
        self.unfollowed = Some("imleç başa taşındı");
    }

    /// Where typing goes on anyway: the model always adds at the end.
    fn move_cursor_to_end(&mut self) {}

    fn move_cursor_to(&mut self, _position: usize) {
        self.unfollowed = Some("imleç taşındı");
    }

    fn select_all(&mut self) {
        self.unfollowed = Some("yazının tamamı seçildi");
    }

    fn select_range(&mut self, _start: usize, _end: usize) {
        self.unfollowed = Some("yazının bir kısmı seçildi");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size as IcedSize;
    use iced::futures::StreamExt;
    use iced::time::Instant;
    use iced::{Point, Task, event, mouse, window};
    use kentos_ui::snapshot::Snapshot;

    use crate::keys;
    use crate::traces::keyboard::{Layout, VARIANTS, chord_stroke};
    use crate::traces::player::{AREA, Player};
    use crate::traces::{Trace, scratch_file};

    /// The real widget and the model side by side over one app.
    struct Twin<'a> {
        app: &'a mut App,
        ui: Snapshot,
        model: CommandLine,
        /// What went wrong, step by step.
        problems: Vec<String>,
    }

    impl Twin<'_> {
        /// One key: the model's prediction against the real widget's messages
        /// and whether it took the key; then, as the window would, the
        /// subscription gets what the widget let through, and the app's tasks run.
        fn key(&mut self, name: &str) {
            let stroke = chord_stroke(name, Layout::TurkishQ).expect("a key");
            let event = stroke.event();
            let want = if self.model.has_keyboard() {
                self.model.key(self.app, &event)
            } else {
                None
            };
            let (got, status) = self.ui.deliver(self.app.command_line(), &event);
            match &want {
                Some(want) => {
                    if format!("{got:?}") != format!("{want:?}") {
                        self.problems
                            .push(format!("{name}: widget {got:?}, model {want:?}"));
                    }
                    if status != event::Status::Captured {
                        self.problems
                            .push(format!("{name}: the widget let it through"));
                    }
                }
                None => {
                    if !got.is_empty() || status != event::Status::Ignored {
                        self.problems.push(format!(
                            "{name}: the widget took it ({got:?}), the model lets it through"
                        ));
                    }
                }
            }
            for message in got {
                self.update(message);
            }
            if status == event::Status::Ignored
                && let Some(message) = keys::key_event(event, status, window::Id::unique())
            {
                self.update(message);
            }
        }

        /// A click on the text box.
        fn click_line(&mut self) {
            let height = kentos_ui::widget::command_line::height(
                kentos_ui::widget::command_line::LINES,
                false,
            );
            let input = Point::new(1200.0, height - 16.0);
            let want = self.model.click();
            let _ = self.ui.deliver(
                self.app.command_line(),
                &iced::Event::Mouse(mouse::Event::CursorMoved { position: input }),
            );
            let (got, _) = self.ui.deliver(
                self.app.command_line(),
                &iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            );
            if format!("{got:?}") != format!("{:?}", want.iter().collect::<Vec<_>>()) {
                self.problems
                    .push(format!("click: widget {got:?}, model {want:?}"));
            }
            for message in got {
                self.update(message);
            }
            let _ = self.ui.deliver(
                self.app.command_line(),
                &iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            );
        }

        fn update(&mut self, message: Message) {
            let task = self.app.update(message);
            self.run(task);
        }

        /// Runs a task: its messages go back to the app; a widget operation
        /// runs on the model and on the real widget, which reports its focus
        /// at the redraw that follows, as the runtime redraws after one.
        fn run(&mut self, task: Task<Message>) {
            let Some(mut stream) = iced_runtime::task::into_stream(task) else {
                return;
            };
            // One action at a time, as the player and the runtime take them.
            while let Some(action) = iced::futures::executor::block_on(stream.next()) {
                match action {
                    iced_runtime::Action::Output(message) => self.update(message),
                    iced_runtime::Action::Widget(mut operation) => {
                        let want = self
                            .model
                            .operate(operation.as_mut())
                            .expect("an operation the model follows");
                        self.ui.operate(self.app.command_line(), operation);
                        let redraw =
                            iced::Event::Window(window::Event::RedrawRequested(Instant::now()));
                        let (got, _) = self.ui.deliver(self.app.command_line(), &redraw);
                        if format!("{got:?}") != format!("{:?}", want.iter().collect::<Vec<_>>()) {
                            self.problems
                                .push(format!("operation: widget {got:?}, model {want:?}"));
                        }
                        for message in got {
                            self.update(message);
                        }
                    }
                    _ => {}
                }
            }
        }

        fn keys(&mut self, names: &[&str]) {
            for name in names {
                self.key(name);
            }
        }
    }

    /// Keys typed and pressed on the real KentOS UI command line, as
    /// `view.rs` configures it, and the widget operations of the app's own
    /// tasks, give what the model says: the messages, whether the widget
    /// takes the key (the others reach the subscription), and the focus it
    /// reports. Letters open the suggestion list; Tab, Enter, Space and Esc
    /// act on it, for commands and for the running command's options.
    #[test]
    fn the_command_line_model_is_the_widget() {
        let (mut app, _) = App::boot(None);
        let trace = Trace::by_id("polygon-keys").expect("reads");
        let mut player = Player::new(
            &mut app,
            &trace,
            VARIANTS[1],
            AREA,
            scratch_file(&trace, VARIANTS[1]),
        )
        .expect("opens");
        let _ = player.apply(Message::Run("tool.polygon"));
        let size = IcedSize::new(
            1600.0,
            kentos_ui::widget::command_line::height(kentos_ui::widget::command_line::LINES, false),
        );
        let mut twin = Twin {
            app: &mut *player.app,
            ui: Snapshot::software(size).expect("the software renderer"),
            model: CommandLine::default(),
            problems: Vec::new(),
        };

        // A value typed in the line while a command runs: no suggestion matches.
        twin.click_line();
        twin.keys(&[
            "1",
            "2",
            "Esc",
            "4",
            "8",
            "7",
            "Backspace",
            "7",
            ",",
            "4",
            "@",
            "+",
            "-",
            ".",
            "Space",
            "Tab",
            "Ctrl+Z",
            "Esc",
        ]);
        assert!(
            !twin.app.line_focused,
            "Esc on the empty line let the keyboard go"
        );
        assert_eq!(
            twin.app.session.tool_id(),
            "select",
            "and ended the command"
        );

        // Idle: a letter on the drawing opens the line (a focus operation), the
        // list suggests commands; Esc clears, Tab completes, Enter runs the
        // first, and the keyboard goes back to the drawing (an unfocus operation).
        twin.keys(&["k", "a", "Esc", "k", "a", "Tab", "Backspace", "a"]);
        assert_eq!(twin.app.command_input, "Ka");
        twin.key("Enter");
        assert_eq!(twin.app.session.tool_id(), "polygon", "KA is Kapalı alan");
        assert!(!twin.model.has_keyboard() && !twin.app.line_focused);

        // A command runs: the list suggests its options first.
        let _ = twin.app.with_tool(|s, cx| s.input("487000,4420000", cx));
        twin.click_line();
        twin.keys(&["u", "Tab", "Esc", "y", "Space"]);
        assert_eq!(
            twin.app.session.prompt().keys().first().copied(),
            Some("D"),
            "Space ran the Yay option: arc mode"
        );
        twin.keys(&["1", "2", "Enter", "Esc", "Esc"]);
        assert_eq!(twin.app.session.tool_id(), "select");

        assert!(
            twin.problems.is_empty(),
            "the model is not the widget:\n{}",
            twin.problems.join("\n")
        );
    }
}

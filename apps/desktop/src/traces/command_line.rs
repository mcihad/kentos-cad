//! The command line as view.rs configures it: what the KentOS UI command
//! line does with a key press while its text box has the keyboard. A test
//! holds this model to the real widget.

use iced::keyboard::key::Named;
use iced::keyboard::{self, Key};

use crate::app::Message;

/// What the KentOS UI command line sends for a key press while its text box
/// has the keyboard and no suggestion list is open (`value` is its text):
/// `None` when it lets the key through to the subscription. The widget's
/// own behaviour is the reference: `the_command_line_model_is_the_widget`.
pub fn line_messages(value: &str, event: &iced::Event) -> Option<Vec<Message>> {
    let iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key,
        modifiers,
        text,
        ..
    }) = event
    else {
        return None;
    };
    let plain = !modifiers.command() && !modifiers.alt();
    match key {
        Key::Named(Named::Escape) if plain => Some(if value.is_empty() {
            vec![Message::CommandCancelled, Message::CommandFocus(false)]
        } else {
            vec![Message::CommandInput(String::new())]
        }),
        Key::Named(Named::Enter) => Some(vec![Message::CommandSubmitted]),
        Key::Named(Named::Space) if plain => Some(vec![Message::CommandSubmitted]),
        Key::Named(Named::Backspace) => {
            let mut value = value.to_owned();
            value.pop();
            Some(vec![Message::CommandInput(value)])
        }
        _ => text
            .as_deref()
            .and_then(|t| t.chars().next())
            .filter(|c| !c.is_control())
            .map(|c| vec![Message::CommandInput(format!("{value}{c}"))]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size as IcedSize;
    use iced::{Point, event, mouse};
    use kentos_ui::snapshot::Snapshot;

    use crate::app::App;
    use crate::traces::keyboard::{Layout, VARIANTS, chord_stroke};
    use crate::traces::player::{AREA, Player};
    use crate::traces::{Trace, scratch_file};

    /// Keys typed and pressed on the real KentOS UI command line, as `view.rs`
    /// configures it, give what `line_messages` says, and the widget takes
    /// exactly those keys: the others reach the subscription.
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
        let app = &mut *player.app;
        let size = IcedSize::new(
            1600.0,
            kentos_ui::widget::command_line::height(kentos_ui::widget::command_line::LINES, false),
        );
        let mut ui = Snapshot::software(size).expect("the software renderer");
        let mut deliver = |app: &mut App, event: iced::Event| {
            let (messages, status) = ui.deliver(app.command_line(), &event);
            for m in &messages {
                let _ = app.update(m.clone());
            }
            (messages, status)
        };
        // Click the text box: it takes the keyboard and says so.
        let input = Point::new(1200.0, size.height - 16.0);
        let _ = deliver(
            app,
            iced::Event::Mouse(mouse::Event::CursorMoved { position: input }),
        );
        let (messages, _) = deliver(
            app,
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        );
        assert!(
            matches!(messages.as_slice(), [Message::CommandFocus(true)]),
            "the click focuses the text box: {messages:?}"
        );
        let _ = deliver(
            app,
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        );
        let keys = [
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
        ];
        for key in keys {
            let stroke = chord_stroke(key, Layout::TurkishQ).expect("a key");
            let event = stroke.event();
            let want = line_messages(&app.command_input, &event);
            let (got, status) = deliver(app, event);
            match want {
                Some(want) => {
                    assert_eq!(format!("{got:?}"), format!("{want:?}"), "{key}");
                    assert_eq!(status, event::Status::Captured, "{key} is the text box's");
                }
                None => {
                    assert!(got.is_empty(), "{key}: {got:?}");
                    assert_eq!(
                        status,
                        event::Status::Ignored,
                        "{key} goes on to the subscription"
                    );
                }
            }
        }
        assert!(
            !app.line_focused,
            "Esc on the empty line let the keyboard go"
        );
        assert_eq!(app.session.tool_id(), "select", "and ended the command");
    }
}

//! The strip over the drawing while a command runs (the web's
//! `ui/shell/CommandBar.ts`; DESIGN.md §7.4.1): the tool, the step it waits
//! for, its options as buttons with their values and keys, and what the right
//! button and Esc do, over the drawing, besides the command line at the bottom,
//! which offers the same options. A preference (`drafting.commandBar`, off by
//! default: the owner found the command line's options neater) turns it on.
//!
//! It floats at the top centre of the drawing and takes the presses on it: a
//! click on the strip never reaches the drawing under it. A long step gives
//! way first: it wraps in what the mouse hints leave, and the options go on
//! to another line, so the hints stay whole at the end of the strip.

use iced::widget::{Row, button, container, opaque, row, text};
use iced::{Center, Element, Fill, Padding};

use kentos_ui::icon::{Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::vertical_divider;

use crate::app::{App, Message};
use crate::catalog::catalog;

/// The strip's distance from the top of the drawing (the web's `top: 10px`).
const TOP: f32 = 10.0;
/// Room the strip leaves beside it (the web's `max-width: calc(100% - 320px)`).
const BESIDE: f32 = 320.0;
/// The strip's padding left and right, and the gap on each side of its divider.
const PAD_LEFT: f32 = 12.0;
const PAD_RIGHT: f32 = 8.0;
const GAP: f32 = 12.0;
/// A key's padding either side and its border.
const KEY_PAD: f32 = 2.0 * 4.0 + 2.0;
/// The gap between the mouse hints.
const HINT_GAP: f32 = 5.0;
/// The least the step and the options get, however narrow the drawing.
const MAIN_MIN: f32 = 120.0;

/// How wide the mouse hints are, “Sağ tık onayla Esc çık” with their keys and
/// gaps: the typography's estimates, which are never short of the text.
fn hints_width() -> f32 {
    let size = typography::caption();
    let key = |name: &str| typography::mono_width(name, size) + KEY_PAD;
    key("Sağ tık")
        + typography::text_width("onayla", size)
        + key("Esc")
        + typography::text_width("çık", size)
        + 3.0 * HINT_GAP
}

/// What the tool, its step and its options may take of a strip `strip` wide:
/// what the hints, the divider and the padding leave. A long step wraps in it,
/// the options go on to another line, and the hints stay whole at the end.
fn main_room(strip: f32) -> f32 {
    (strip - PAD_LEFT - PAD_RIGHT - 2.0 * GAP - 1.0 - hints_width()).max(MAIN_MIN)
}

impl App {
    /// The strip, while a command runs and the preference has it on.
    pub(crate) fn command_bar(&self) -> Option<Element<'_, Message>> {
        if !self.command_bar {
            return None;
        }
        let p = self.session.prompt();
        let tool = p.tool?;

        let mut name = Row::new().spacing(6).align_y(Center);
        if let Some(command) = catalog().get(&format!("tool.{}", self.session.tool_id())) {
            name = name.push(icon(command.icon).size(16.0).tone(Tone::Highlight));
        }
        let name = name.push(
            text(tool)
                .font(typography::ui_strong())
                .size(typography::body()),
        );
        let options = p.options.iter().map(|o| {
            let mut face = row![label::caption(o.label)].spacing(6).align_y(Center);
            if let Some(value) = &o.value {
                face = face.push(text(value.clone()).size(typography::caption()).style(
                    |theme: &iced::Theme| text::Style {
                        color: Some(Tokens::of(theme).accent_hover),
                    },
                ));
            }
            let face = face.push(key(o.key));
            button(face)
                .on_press(Message::PromptOption(o.key))
                .padding(Padding {
                    top: 2.0,
                    right: 4.0,
                    bottom: 2.0,
                    left: 9.0,
                })
                .style(style::button::keyword)
                .into()
        });
        // The step and its notes (a current value, a hint), as the web's `cmdbar__note`: smaller, muted.
        let step = p.notes.iter().fold(
            row![label::body(p.step.clone().into_owned())]
                .spacing(8)
                .align_y(Center),
            |step, note| step.push(label::caption(note.clone())),
        );
        let main = Row::new()
            .push(name)
            .push(step)
            .push(
                Row::with_children(options)
                    .spacing(4)
                    .align_y(Center)
                    .wrap()
                    .vertical_spacing(4),
            )
            .spacing(12)
            .align_y(Center)
            .wrap()
            .vertical_spacing(6);

        // What the mouse does; the desktop has no command menu on a held right button yet.
        let mouse = row![
            key("Sağ tık"),
            label::caption("onayla"),
            key("Esc"),
            label::caption("çık"),
        ]
        .spacing(HINT_GAP)
        .align_y(Center);

        let width = (self.viewport.camera.width as f32 - BESIDE).max(240.0);
        // A long step gives way first: it wraps in what the hints leave, so they are never pushed out.
        let main = container(main).max_width(main_room(width));
        let strip = container(
            // A divider of its own height: one that fills would stretch the strip down the drawing.
            row![main, container(vertical_divider()).height(22), mouse]
                .spacing(GAP)
                .align_y(Center),
        )
        .padding(Padding {
            top: 5.0,
            right: PAD_RIGHT,
            bottom: 5.0,
            left: PAD_LEFT,
        })
        .max_width(width)
        .style(style::container::popover);

        Some(
            container(opaque(strip))
                .width(Fill)
                .align_x(Center)
                .padding(Padding {
                    top: TOP,
                    ..Padding::ZERO
                })
                .into(),
        )
    }
}

/// A key as the keyboard shows it: `S`, `Enter`, `Sağ tık`.
fn key(name: &str) -> Element<'_, Message> {
    container(label::mono_caption(name))
        .padding([0, 4])
        .style(style::container::keycap)
        .into()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use iced::advanced::widget::{Id, Operation};
    use iced::{Point, Rectangle, Size};
    use kentos_ui::snapshot::{Input, Snapshot};
    use serde_json::Value;

    use super::{BESIDE, TOP};
    use crate::app::{App, Message};
    use crate::files_testing::app_with_drawing;
    use crate::settings_view::Edit;

    /// Every text of a view and where it is laid out.
    struct Texts(Arc<Mutex<Vec<(String, Rectangle)>>>);

    impl Operation for Texts {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }

        fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, text: &str) {
            if let Ok(mut found) = self.0.lock() {
                found.push((text.to_owned(), bounds));
            }
        }
    }

    /// A long step gives way: at the narrowest the shell goes, Dizi's
    /// spacing step (the longest there is) wraps, and the mouse hints stay
    /// whole inside the strip instead of being pushed out of it.
    #[test]
    fn a_long_step_never_pushes_the_hints_out() {
        let mut app = app_with_drawing();
        strip(&mut app, true);
        let _ = app.run("edit.selectAll");
        let _ = app.run("tool.array");
        let _ = app.run("tool.confirm");
        assert!(
            app.session
                .prompt()
                .step
                .starts_with("sütun ve satır aralığını"),
            "{:?}",
            app.session.prompt()
        );
        let mut snapshot = Snapshot::software(Size::new(1100.0, 600.0)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let found = Arc::new(Mutex::new(Vec::new()));
        snapshot.operate(app.view(), Box::new(Texts(found.clone())));
        let found = found.lock().expect("the texts").clone();
        let area = app.viewport.bounds;
        let right = area.center_x() + (area.width - BESIDE) / 2.0;
        let at = |text: &str| {
            found
                .iter()
                .find(|(t, _)| t == text)
                .map(|(_, b)| *b)
                .unwrap_or_else(|| panic!("“{text}” is shown: {found:?}"))
        };
        // Squeezed out, a hint is laid out 0 px wide; whole, it ends inside the strip.
        for text in ["Sağ tık", "onayla", "Esc", "çık"] {
            let b = at(text);
            assert!(b.width > 5.0, "“{text}” is squeezed to {} px", b.width);
            assert!(
                b.x + b.width <= right + 0.5,
                "“{text}” ends at {} past the strip's {right}",
                b.x + b.width
            );
        }
        let esc = at("Esc");
        let step = at(&app.session.prompt().step);
        assert!(
            step.x + step.width <= esc.x,
            "the step wraps before the hints"
        );
        assert!(step.height > esc.height * 1.5, "the step takes two lines");
        // `KENTOS_STRIP_SHOT=out` keeps what was laid out, dark and light
        // (`out.png`, `out-acik.png`), for a look; the software renderer
        // leaves the drawing area itself empty.
        if let Ok(path) = std::env::var("KENTOS_STRIP_SHOT") {
            for (theme, name) in [("view.theme.dark", ""), ("view.theme.light", "-acik")] {
                let _ = app.run(theme);
                snapshot.settle(&mut app, App::view, &mut update);
                snapshot
                    .render(app.view(), &app.theme())
                    .save(format!("{path}{name}.png"))
                    .expect("the image is written");
            }
        }
    }

    fn update(app: &mut App, message: Message) {
        let _ = app.update(message);
    }

    fn strip(app: &mut App, on: bool) {
        let _ = app.run("tools.options");
        update(
            app,
            Message::Settings(Edit::Value("drafting.commandBar", Value::Bool(on))),
        );
        update(app, Message::Settings(Edit::Save));
    }

    #[test]
    fn the_strip_is_off_by_default_and_its_preference_turns_it_on() {
        let mut app = app_with_drawing();
        let _ = app.run("tool.regularPolygon");
        // Off by default: the command line keeps the step and the options.
        assert!(!app.command_bar && app.command_bar().is_none());
        assert!(app.line_prompt().is_some());
        assert!(!app.session.prompt().options.is_empty());

        let _ = app.run("tools.options");
        update(
            &mut app,
            Message::Settings(Edit::Value("drafting.commandBar", Value::Bool(true))),
        );
        assert!(app.command_bar().is_none(), "nothing changes before Kaydet");
        update(&mut app, Message::Settings(Edit::Save));
        assert!(app.command_bar().is_some());
        // The idle select tool has nothing to say, strip or not.
        let _ = app.run("tool.cancel");
        assert!(app.command_bar().is_none());
    }

    /// A press on the strip is the strip's: the drawing under it gets no
    /// point. With the strip off, the same press is a point.
    #[test]
    fn a_press_on_the_strip_never_reaches_the_drawing_under_it() {
        let mut app = app_with_drawing();
        strip(&mut app, true);
        let _ = app.run("tool.regularPolygon");
        let mut snapshot = Snapshot::software(Size::new(1440.0, 900.0)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Move(Point::new(700.0, 500.0)),
        );
        let area = app.viewport.bounds;
        assert!(area.width > 0.0, "the drawing area reported its place");
        // The strip's top padding, in the middle of the drawing: no button there.
        let on_strip = Point::new(area.center_x(), area.y + TOP + 2.0);
        let first = app.session.prompt();
        snapshot.input(&mut app, App::view, &mut update, Input::Click(on_strip));
        assert_eq!(app.session.prompt(), first, "the drawing got no point");

        strip(&mut app, false);
        snapshot.input(&mut app, App::view, &mut update, Input::Click(on_strip));
        assert_ne!(
            app.session.prompt(),
            first,
            "without the strip the press is the centre"
        );
    }
}

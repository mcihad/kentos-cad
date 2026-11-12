//! Sıradakini seç's chip (docs/adr/0187 §1), the web's `SelectionChip`
//! (`apps/web/src/ui/shell/SelectionChip.ts`): beside a click that found
//! several objects, “1/3 ▾”. Shift+Boşluk takes the next
//! (`edit.cycleSelection`); a click opens their list, kind and layer, the
//! chosen one marked, the one under the pointer highlighted in the drawing.
//! It follows the clicked place as the view moves and goes with the cycle
//! (the selection changing otherwise, a command starting, Esc).

use iced::widget::{container, row, tooltip};
use iced::{Background, Border, Center, Element, Point, Theme, Vector};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::theme::Tokens;
use kentos_ui::theme::shape::{self, Level};
use kentos_ui::widget::context_menu::{Menu, MenuButton};
use kentos_ui::widget::{Tip, beside, tip};

use crate::app::{App, Message};
use crate::selecting::kind_title;
use crate::selection_commands::kind_icon;

/// The chip's distance right of and below the click, logical pixels (the web's `OFFSET`).
const OFFSET: f32 = 10.0;

impl App {
    /// The chip over the drawing, while a click's cycle holds and no command runs.
    pub(crate) fn selection_chip(&self) -> Option<Element<'_, Message>> {
        if self.session.is_running() || self.session.grip_active() {
            return None;
        }
        let c = self.selection.cycle()?;
        let n = c.candidates.len();
        let [x, y] = self.viewport.camera.world_to_screen(c.at);
        let pill = container(
            row![
                label::caption(format!("{}/{n}", c.index + 1)),
                icon(Icon::ChevronDown).size(12.0),
            ]
            .spacing(2)
            .align_y(Center),
        )
        .padding(iced::Padding {
            top: 2.0,
            right: 4.0,
            bottom: 2.0,
            left: 8.0,
        })
        .style(|theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: Some(Background::Color(t.surface)),
                text_color: Some(t.text),
                border: Border {
                    color: t.accent_line(),
                    width: 1.0,
                    radius: 999.0.into(),
                },
                shadow: shape::shadow(Level::Float, &t),
                ..container::Style::default()
            }
        });
        let shortcut = crate::catalog::catalog()
            .get("edit.cycleSelection")
            .and_then(|c| c.shortcuts.first())
            .map(|s| crate::shortcuts::chord(s));
        let words = format!(
            "Burada üst üste {n} nesne var. {}Tıklayın: listeden seçin.",
            shortcut.map_or(String::new(), |s| format!("{s}: sıradakini seç. "))
        );
        let chip = MenuButton::new(pill, move || self.cycle_menu());
        Some(beside(
            tip(chip, Tip::new(words), tooltip::Position::Bottom),
            Point::new(x as f32, y as f32),
            Vector::new(OFFSET, OFFSET),
        ))
    }

    /// The chip's list: each candidate by its kind and layer, the chosen one
    /// marked; resting on one highlights it in the drawing.
    fn cycle_menu(&self) -> Menu<Message> {
        let mut menu = Menu::new()
            .header("Burada üst üste binenler")
            .on_unhighlight(Message::CycleHover(None));
        let (Some(c), Some(doc)) = (self.selection.cycle(), self.document.as_ref()) else {
            return menu;
        };
        for (i, slot) in c.candidates.iter().enumerate() {
            let Some(e) = doc.model.get(*slot) else {
                continue;
            };
            let layer = doc
                .model
                .layers()
                .get(&e.base().layer_id)
                .map(|n| n.name.clone());
            let text = match layer {
                Some(name) => format!("{} · {name}", kind_title(e.kind())),
                None => kind_title(e.kind()).to_owned(),
            };
            menu = menu
                .radio(text, i == c.index, Message::CycleTo(i))
                .icon(crate::icons::from_web(Some(kind_icon(e.kind()))))
                .highlight(Message::CycleHover(Some(*slot)));
        }
        menu
    }
}

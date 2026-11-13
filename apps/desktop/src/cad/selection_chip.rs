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
        // Its place out of the view (the view moved away): no chip saying “burada” over what is there now.
        if !cycle_in_view(&self.viewport.camera, c.at) {
            return None;
        }
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

/// Whether the place the overlapping objects were picked at is inside the view.
pub(crate) fn cycle_in_view(camera: &kentos_render_wgpu::Camera, at: kentos_interaction::Vec2) -> bool {
    let b = camera.visible_bounds();
    at.x >= b.min_x && at.x <= b.max_x && at.y >= b.min_y && at.y <= b.max_y
}

#[cfg(test)]
mod view_tests {
    /// UR-099: the chip of a place moved out of the view is not shown; a place in it is.
    #[test]
    fn a_cycle_out_of_the_view_has_no_chip() {
        let mut app = crate::files_testing::app_with_drawing();
        let _ = app.update(crate::app::Message::WindowResized(iced::Size::new(1200.0, 800.0)));
        app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds { min_x: 0.0, min_y: 0.0, max_x: 100.0, max_y: 100.0 },
            24.0,
        );
        let inside = kentos_interaction::Vec2::new(50.0, 50.0);
        let far = kentos_interaction::Vec2::new(5000.0, 50.0);
        assert!(super::cycle_in_view(&app.viewport.camera, inside));
        assert!(!super::cycle_in_view(&app.viewport.camera, far));
        app.selection.start_cycle(far, vec![kentos_domain::Slot(1), kentos_domain::Slot(2)]);
        assert!(app.selection_chip().is_none(), "no chip for a place out of view");
        app.selection.start_cycle(inside, vec![kentos_domain::Slot(1), kentos_domain::Slot(2)]);
        assert!(app.selection_chip().is_some());
    }
}

//! The status bar's scale selector (docs/adr/0165 §5; the web's
//! `StatusBar` scale cell, QGIS's scale box): the view's 96 dpi screen scale,
//! 1:N, chosen from the project's type's scales or typed. The view zooms
//! about its middle to it, a navigation step Önceki görünüm goes back from.

use iced::widget::{Id, operation, row, text_input};
use iced::{Center, Element, Task};
use kentos_contracts::Workspace;
use kentos_interaction::Level;
use kentos_project::wizard::{CAD_SCALES, GIS_SCALES, scale_text};
use kentos_ui::widget::Menu;
use kentos_ui::{label, style, theme::typography};

use crate::app::{App, Message};

/// The typed scale's field.
const FIELD: &str = "ekran-olcegi";

#[derive(Debug, Clone)]
pub enum Event {
    /// A scale chosen: 1:N.
    Choose(f64),
    /// Ölçek yaz…: the field opens with the view's scale.
    Type,
    Edit(String),
    /// Enter in the field; Esc or a press on the drawing closes it (input.rs).
    Submit,
}

fn event(e: Event) -> Message {
    Message::ScreenScale(e)
}

/// A scale typed as 1:N, N, or N with dots between its digits: a whole number over 0.
pub fn typed_scale(text: &str) -> Option<f64> {
    let t = text.trim();
    let t = t.strip_prefix("1:").unwrap_or(t);
    let digits: String = t
        .chars()
        .filter(|c| *c != '.' && !c.is_whitespace())
        .collect();
    let n: u64 = digits.parse().ok()?;
    (n > 0 && n < 1_000_000_000).then_some(n as f64)
}

impl App {
    pub(crate) fn screen_scale_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Choose(n) => self.zoom_to_scale(n),
            Event::Type => {
                let n = self.viewport.camera.screen_scale().round();
                self.scale_field = Some(format!("{n}"));
                return Task::batch([
                    operation::focus(Id::new(FIELD)),
                    operation::select_all(Id::new(FIELD)),
                ]);
            }
            Event::Edit(text) => self.scale_field = Some(text),
            Event::Submit => {
                let text = self.scale_field.take().unwrap_or_default();
                match typed_scale(&text) {
                    Some(n) => self.zoom_to_scale(n),
                    None => self.say(
                        Level::Warn,
                        format!(
                            "“{text}” bir ölçek değil. 1:N biçiminde bir tam sayı yazın, ör. 1:500."
                        ),
                    ),
                }
            }
        }
        Task::none()
    }

    /// The view zoomed about its middle to 1:N, a navigation step.
    fn zoom_to_scale(&mut self, n: f64) {
        self.navigating(|app| app.viewport.camera.zoom_to_screen_scale(n));
    }

    /// The scales the selector offers: the project's type's (the new project
    /// wizard's), and a wider one for a map.
    pub(crate) fn offered_scales(&self) -> Vec<f64> {
        if self.work_mode() == Workspace::Cad {
            CAD_SCALES.to_vec()
        } else {
            GIS_SCALES.iter().copied().chain([100_000.0]).collect()
        }
    }

    /// The field the scale is typed in, in the cell's place while it is open:
    /// “Ekran 1:” and the number, as the cell reads.
    pub(crate) fn scale_field_view(&self) -> Option<Element<'_, Message>> {
        let text = self.scale_field.as_ref()?;
        let field = kentos_ui::widget::focus_ring(
            text_input("N", text)
                .id(Id::new(FIELD))
                .on_input(|t| event(Event::Edit(t)))
                .on_submit(event(Event::Submit))
                .padding([1, 6])
                .size(typography::body())
                .width(typography::from_default(84.0))
                .style(style::field::input),
        );
        Some(
            row![label::muted("Ekran 1:"), field]
                .spacing(4)
                .align_y(Center)
                .into(),
        )
    }
}

/// The selector's menu: the type's scales, then Ölçek yaz….
pub(crate) fn scale_menu(scales: &[f64]) -> Menu<Message> {
    scales
        .iter()
        .fold(Menu::new().header("Ekran ölçeği"), |menu, &n| {
            menu.item(scale_text(n), event(Event::Choose(n)))
        })
        .separator()
        .item("Ölçek yaz…", event(Event::Type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;

    fn send(app: &mut App, e: Event) {
        let _ = app.update(Message::ScreenScale(e));
    }

    #[test]
    fn a_scale_chosen_or_typed_zooms_the_view_to_it() {
        assert_eq!(
            ["1:500", "2.500", " 1:25.000 ", "0", "abc", "1:"].map(typed_scale),
            [Some(500.0), Some(2500.0), Some(25_000.0), None, None, None]
        );
        let mut app = app_with_drawing();
        let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0)),
        )));
        send(&mut app, Event::Choose(1000.0));
        assert!((app.viewport.camera.screen_scale() - 1000.0).abs() < 1e-6);
        assert!(app.view_history.can_back(), "a navigation step");
        send(&mut app, Event::Type);
        assert_eq!(app.scale_field.as_deref(), Some("1000"));
        send(&mut app, Event::Edit("1:250".into()));
        send(&mut app, Event::Submit);
        assert_eq!(app.scale_field, None);
        assert!((app.viewport.camera.screen_scale() - 250.0).abs() < 1e-6);
        // What is no scale says so and leaves the view.
        send(&mut app, Event::Type);
        send(&mut app, Event::Edit("büyük".into()));
        send(&mut app, Event::Submit);
        assert!((app.viewport.camera.screen_scale() - 250.0).abs() < 1e-6);
        assert!(crate::files_testing::last_said(&app).contains("bir ölçek değil"));
    }

    #[test]
    fn the_menu_offers_the_types_scales() {
        let mut app = app_with_drawing();
        assert!(app.offered_scales().contains(&25_000.0), "a map's");
        app.choose_mode("workspace.cad");
        assert_eq!(app.offered_scales().first(), Some(&1.0), "a drawing's");
    }

    /// The selector's menu and its typed field, a CAD and a CBS project;
    /// `.run/shots/ekran-olcegi-*`:
    ///
    /// ```text
    /// cargo test -p kentos-desktop screen_scale::tests::screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use iced::{Point, Size};
        use kentos_ui::snapshot::{Input, Snapshot};

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        for (mode, cad) in [("light", true), ("dark", false)] {
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            if cad {
                app.choose_mode("workspace.cad");
            }
            let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let name = if cad { "cad" } else { "cbs" };
            // The cell: right of the drafting aids, left of the type's cell.
            let x = if cad { 1165.0 } else { 1160.0 };
            snapshot.input(
                &mut app,
                App::view,
                &mut update,
                Input::Click(Point::new(x, 888.0)),
            );
            let file = out.join(format!("ekran-olcegi-{name}-menu.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
            send(&mut app, Event::Type);
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("ekran-olcegi-{name}-yaz.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

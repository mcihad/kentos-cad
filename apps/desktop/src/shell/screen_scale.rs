//! The status bar's scale selector (docs/adr/0165 §5): the view's 96 dpi screen scale, 1:N, chosen from the project's
//! type's scales or typed. The view zooms about its middle to it, a navigation step Önceki görünüm goes back from.
//!
//! Every scale readout says a view's scale in one way ([`scale_words`], docs/adr/9327): the status bar, a split
//! view's badge and the typed field. Closer in than full size it is the enlargement N:1; neither number is ever 0.

use iced::widget::{Id, operation, row};
use iced::{Center, Element, Task};
use kentos_commands::Workspace;
use kentos_interaction::Level;
use kentos_project::wizard::{CAD_SCALES, GIS_SCALES, scale_text};
use kentos_ui::widget::Menu;
use kentos_ui::theme::typography;

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

/// A whole number over 0 typed with dots between its digits, or none.
fn whole(text: &str) -> Option<f64> {
    let digits: String = text.chars().filter(|c| *c != '.' && !c.is_whitespace()).collect();
    let n: u64 = digits.parse().ok()?;
    (n > 0 && n < 1_000_000_000).then_some(n as f64)
}

/// A scale typed as 1:N, N, or an enlargement N:1 (the N of 1:N then 1/N), each N a whole number over 0 that may have
/// dots between its digits.
pub fn typed_scale(text: &str) -> Option<f64> {
    let t = text.trim();
    if let Some(n) = t.strip_suffix(":1").filter(|n| !n.trim().starts_with("1:")) {
        return whole(n).map(|n| 1.0 / n);
    }
    whole(t.strip_prefix("1:").unwrap_or(t))
}

/// A view's scale, the N of 1:N, as every readout says it (docs/adr/9327): 1:N at N of one or more; closer in than
/// full size, the enlargement N:1 (the N of 1:N rounded to 0 read “1:0”, and a view's badge and the status bar
/// disagreed); none readable, a dash. Either way the number is a whole one of at least 1.
pub(crate) fn scale_words(n: f64) -> String {
    if !n.is_finite() || n <= 0.0 {
        return "—".to_owned();
    }
    if n >= 1.0 {
        return scale_text(n.round());
    }
    format!("{}:1", scale_text((1.0 / n).round()).trim_start_matches("1:"))
}

impl App {
    pub(crate) fn screen_scale_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Choose(n) => self.zoom_to_scale(n),
            Event::Type => {
                self.scale_field = Some(scale_words(self.view_scale()));
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
                            "“{text}” bir ölçek değil. 1:N biçiminde bir tam sayı yazın, ör. 1:500; büyütmeyi N:1 biçiminde, ör. 2:1."
                        ),
                    ),
                }
            }
        }
        Task::none()
    }

    /// The view zoomed about its middle to 1:N, a navigation step; in a geographic project N is of metres on the
    /// ground, a degree being as many metres as at the view's middle (GIS-040).
    fn zoom_to_scale(&mut self, n: f64) {
        let per_unit = self.metres_per_unit();
        self.navigating(|app| app.viewport.camera.zoom_to_screen_scale(n / per_unit));
    }

    /// Metres a drawing unit is at the view's middle: 1 but in a geographic project (map_marks.rs).
    pub(crate) fn metres_per_unit(&self) -> f64 {
        self.document
            .as_ref()
            .map_or(1.0, |d| crate::map_marks::metres_per_unit(d.model.settings(), &self.viewport.camera))
    }

    /// The view's 1:N as the status bar says it: of metres on the ground (GIS-040).
    pub(crate) fn view_scale(&self) -> f64 {
        self.scale_of(&self.viewport.camera)
    }

    /// A view's 1:N through its own camera, as the status bar says the active one's (docs/adr/9327): a split view's
    /// badge and the status bar say the same scale, in a geographic project too.
    pub(crate) fn scale_of(&self, camera: &kentos_render_wgpu::Camera) -> f64 {
        let per_unit =
            self.document.as_ref().map_or(1.0, |d| crate::map_marks::metres_per_unit(d.model.settings(), camera));
        camera.screen_scale() * per_unit
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

    /// The field the scale is typed in, in the cell's place while it is open: the scale whole, as the cell reads
    /// (docs/adr/0626 §3, 9327), 1:N or N:1.
    pub(crate) fn scale_field_view(&self) -> Option<Element<'_, Message>> {
        let text = self.scale_field.as_ref()?;
        let field = kentos_ui::widget::text_field("1:N", text)
            .padding(kentos_ui::theme::metrics::padding(
                kentos_ui::theme::metrics::inline(),
                6.0,
            ))
            .id(Id::new(FIELD))
            .on_input(|t| event(Event::Edit(t)))
            .on_submit(event(Event::Submit))
            .width(typography::from_default(104.0));
        Some(row![field].align_y(Center).into())
    }
}

/// The selector's menu: the type's scales, then Ölçek yaz….
pub(crate) fn scale_menu(scales: &[f64], summary: &[kentos_interaction::scale_summary::ScaleRow]) -> Menu<Message> {
    let menu = scales
        .iter()
        .fold(Menu::new().header("Görünümün ölçeği"), |menu, &n| {
            menu.item(scale_text(n), event(Event::Choose(n)))
        })
        .separator()
        .item("Ölçek yaz…", event(Event::Type));
    crate::annotation_scale::with_summary(menu, summary)
}

impl App {
    /// Ölçeklerin özeti at the screen's middle (docs/adr/1231): a project with a coordinate system's (a map's, a road's,
    /// a GIS's); none for a building on its own coordinates.
    pub(crate) fn scale_summary(&self) -> Vec<kentos_interaction::scale_summary::ScaleRow> {
        let Some(doc) = &self.document else { return Vec::new() };
        let settings = doc.settings();
        if kentos_project::systems::own(settings).is_none() {
            return Vec::new();
        }
        let cam = &self.viewport.camera;
        let center = cam.screen_to_world(cam.width / 2.0, cam.height / 2.0);
        kentos_interaction::scale_summary::rows(settings, center, Some(cam.screen_scale()))
    }
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
            ["1:500", "2.500", " 1:25.000 ", "0", "abc", "1:", "2:1", "2.646:1", "0:1", "1:1"].map(typed_scale),
            [
                Some(500.0),
                Some(2500.0),
                Some(25_000.0),
                None,
                None,
                None,
                Some(0.5),
                Some(1.0 / 2646.0),
                None,
                Some(1.0)
            ]
        );
        let mut app = app_with_drawing();
        let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0)),
        )));
        send(&mut app, Event::Choose(1000.0));
        assert!((app.viewport.camera.screen_scale() - 1000.0).abs() < 1e-6);
        assert!(app.view_history.can_back(), "a navigation step");
        send(&mut app, Event::Type);
        assert_eq!(app.scale_field.as_deref(), Some("1:1.000"));
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

    /// UR-098, docs/adr/9327: a view zoomed in past full size says its enlargement, never “1:0”; at 1:1 the two ways
    /// meet, and neither number is ever 0.
    #[test]
    fn a_scale_reads_one_way_on_both_sides_of_full_size() {
        assert_eq!(scale_words(68.4), "1:68");
        assert_eq!(scale_words(2646.0), "1:2.646");
        assert_eq!(scale_words(1.0), "1:1");
        assert_eq!(scale_words(1.4), "1:1");
        assert_eq!(scale_words(0.99), "1:1", "just closer than full size");
        assert_eq!(scale_words(0.5), "2:1", "half the paper's metre: twice the size");
        assert_eq!(scale_words(0.2), "5:1");
        assert_eq!(scale_words(0.4), "3:1");
        assert_eq!(scale_words(1.0 / 2646.0), "2.646:1", "the deepest zoom");
        assert_eq!(scale_words(0.0), "—");
        assert_eq!(scale_words(f64::NAN), "—");
        // Within the camera's reach: 2 646:1 closest in.
        for n in [0.3, 0.51, 0.7, 1.0 / 2646.0, 1e-3, 0.999, 1.0, 1.5, 68.4, 2.5e8] {
            let words = scale_words(n);
            assert!(!words.starts_with("0:") && !words.ends_with(":0"), "{n}: {words}");
            // What a readout says, typed back, is the scale it said (to its rounding).
            let back = typed_scale(&words).expect("typed back");
            let said = if n >= 1.0 { n.round() } else { 1.0 / (1.0 / n).round() };
            assert!((back - said).abs() <= said * 1e-12, "{n}: {words} read {back}");
        }
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

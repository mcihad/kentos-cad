//! The project's windows (the web's `ui/settings/NewProjectDialog.ts` and
//! `ProjectSettingsDialog.ts`): Yeni proje and Proje ayarları. Nothing
//! changes until the window's button. A new project replaces the drawing;
//! unsaved work is asked about first, over the window, and Vazgeç there
//! comes back to it. Project settings are assigned to the drawing: an edit,
//! not an undo step, and a new coordinate system is assigned, never a
//! transformation of the coordinates (CLAUDE.md §5).

mod content;
mod new;
mod settings;

use std::fmt;

use iced::widget::{Column, Row, button, column, container, row, text};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::Workspace;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::typography;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::{label, style};

pub use content::PLOT_SCALES;

use crate::app::{App, Dialog, Message};
use crate::catalog::catalog;
use crate::crs::grouped;

/// The open project window.
#[derive(Debug)]
pub enum Window {
    New(Box<new::State>),
    Settings(Box<settings::State>),
}

#[derive(Debug, Clone)]
pub enum Event {
    New(new::Event),
    Settings(settings::Event),
    Close,
}

fn message(event: Event) -> Message {
    Message::Project(Box::new(event))
}

/// The web command ids this module runs; Koordinat sistemi… is Proje
/// ayarları on its coordinate system page (the web's `openProjectSettings('crs')`).
pub const COMMANDS: [&str; 3] = ["file.new", "file.settings", "crs.set"];

impl App {
    pub(crate) fn project_command(&mut self, id: &'static str) -> Task<Message> {
        match id {
            "file.new" => {
                let state = new::State::new(self);
                self.open_project_window(Window::New(Box::new(state)));
            }
            "file.settings" | "crs.set" => match &self.document {
                Some(doc) => {
                    let mut state = settings::State::new(doc);
                    if id == "crs.set" {
                        state.section = settings::Section::Crs;
                    }
                    self.open_project_window(Window::Settings(Box::new(state)));
                }
                None => self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O)."),
            },
            _ => {}
        }
        Task::none()
    }

    fn open_project_window(&mut self, window: Window) {
        self.project = Some(window);
        self.dialog = Some(Dialog::Project);
    }

    pub(crate) fn project_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::New(e) => self.new_project_event(e),
            Event::Settings(e) => {
                self.project_settings_event(e);
                Task::none()
            }
            Event::Close => {
                self.close_project_window();
                Task::none()
            }
        }
    }

    pub(crate) fn close_project_window(&mut self) {
        self.project = None;
        if self.dialog == Some(Dialog::Project) {
            self.dialog = None;
        }
    }

    pub(crate) fn project_view(&self) -> Element<'_, Message> {
        match &self.project {
            Some(Window::New(s)) => self.new_project_view(s),
            Some(Window::Settings(s)) => self.project_settings_view(s),
            None => text("").into(),
        }
    }
}

/// A drawing typeface's name; a project that names none draws in Barlow.
pub fn font_label(font: Option<kentos_contracts::DrawingFont>) -> &'static str {
    let font = font.unwrap_or(kentos_contracts::DrawingFont::Barlow);
    settings::FONTS
        .iter()
        .find(|(f, _)| *f == font)
        .map_or("Barlow", |(_, name)| name)
}

/// A plot scale as the web writes it: `1:1.000`.
pub fn scale_label(scale: f64) -> String {
    format!("1:{}", grouped(scale))
}

/// A plot scale choice of the segmented control.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Scale(f64);

impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&scale_label(self.0))
    }
}

/// The plot scales as a segmented control.
fn scales<'a, M: Clone + 'a>(value: f64, on: impl Fn(f64) -> M) -> Element<'a, M> {
    Segmented::new(PLOT_SCALES.map(Scale), Scale(value), move |s| on(s.0)).into()
}

/// The work modes as cards (the web's `workspacePicker`): the ones that can
/// be chosen, then the announced ones, dimmed, with “Yakında”. `compact`
/// leaves out what each is for (Proje ayarları).
fn modes<'a, M: Clone + 'a>(
    value: Workspace,
    compact: bool,
    on: impl Fn(Workspace) -> M,
) -> Element<'a, M> {
    let card = |m: &'static crate::catalog::Mode| -> Element<'a, M> {
        let on_it = m.id == value;
        let mark: Element<'a, M> = if !m.ready {
            container(label::caption("Yakında"))
                .padding([0, 5])
                .style(style::container::badge)
                .into()
        } else if on_it {
            icon(Icon::Check).size(12.0).tone(Tone::Accent).into()
        } else {
            text("").into()
        };
        let mut body = Column::new().spacing(4).push(
            row![
                text(m.label)
                    .font(typography::ui_strong())
                    .size(typography::body()),
                mark
            ]
            .spacing(8)
            .align_y(Center),
        );
        body = body.push(label::caption(m.title));
        if !compact {
            body = body.push(label::muted(m.description));
            for point in m.highlights {
                body = body.push(label::caption(format!("• {point}")));
            }
        }
        button(container(body).width(Fill))
            .on_press_maybe(m.ready.then(|| on(m.id)))
            .padding(10)
            .width(Fill)
            .style(style::button::navigation(on_it))
            .into()
    };
    let all = catalog().modes();
    let ready: Vec<Element<'a, M>> = all.iter().filter(|m| m.ready).map(card).collect();
    let soon: Vec<Element<'a, M>> = all.iter().filter(|m| !m.ready).map(card).collect();
    let mut out = Column::new()
        .spacing(8)
        .push(Row::with_children(ready).spacing(8));
    if !soon.is_empty() && !compact {
        out = out
            .push(label::caption("Yakında"))
            .push(Row::with_children(soon).spacing(8));
    }
    out.into()
}

/// A titled group of a window (the web's `group`).
fn group<'a, M: 'a>(title: &'a str, content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    column![
        text(title)
            .font(typography::ui_strong())
            .size(typography::body()),
        content.into()
    ]
    .spacing(8)
    .into()
}

/// A labelled row: the name and what it does on the left, the control on the right (the web's `settingRow`).
fn setting<'a, M: 'a>(
    name: &'a str,
    hint: Option<&'a str>,
    control: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    let mut words = Column::new().spacing(2).push(label::body(name));
    if let Some(hint) = hint {
        words = words.push(label::caption(hint));
    }
    row![container(words).width(Fill), control.into()]
        .spacing(16)
        .align_y(Center)
        .into()
}

#[cfg(test)]
mod tests {
    use kentos_contracts::{AreaUnit, DrawingFont, Workspace};

    use super::Event;
    use super::new::Event as New;
    use super::settings::Event as Settings;
    use crate::app::{App, Dialog, Message};
    use crate::files_testing::{app_with_drawing, last_said};

    fn send(app: &mut App, event: Event) {
        let _ = app.update(Message::Project(Box::new(event)));
    }

    #[test]
    fn a_new_project_replaces_a_clean_drawing_with_the_standard_layers() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("file.new"));
        assert_eq!(app.dialog, Some(Dialog::Project));
        send(&mut app, Event::New(New::Name("  Ada 7  ".into())));
        send(&mut app, Event::New(New::Scale(500.0)));
        send(&mut app, Event::New(New::Mode(Workspace::Cad)));
        send(&mut app, Event::New(New::Crs(5254)));
        let _ = app.update(Message::Project(Box::new(Event::New(New::Create))));
        assert_eq!(app.dialog, None);
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.name(), "Ada 7");
        assert_eq!(doc.settings().srid, 5254);
        assert_eq!(doc.settings().plot_scale, 500.0);
        assert_eq!(doc.settings().workspace, Some(Workspace::Cad));
        assert_eq!(doc.settings().drawing_font, Some(DrawingFont::Barlow));
        assert_eq!(doc.entity_count(), 0);
        assert_eq!(doc.model.layers().active(), "taslak");
        assert!(doc.path.is_none() && !doc.dirty());
        assert!(
            last_said(&app)
                .starts_with("“Ada 7” yeni projesi açıldı: TUREF / TM30 (EPSG:5254), 1:500."),
            "{}",
            last_said(&app)
        );
    }

    #[test]
    fn unsaved_work_is_asked_about_and_vazgec_comes_back_to_the_window() {
        let mut app = app_with_drawing();
        let layer = app.document.as_ref().expect("open").layers()[0].id.clone();
        let _ = app.update(Message::LayerLocked(layer));
        assert!(app.document.as_ref().expect("open").dirty());
        let _ = app.update(Message::Run("file.new"));
        let _ = app.update(Message::Project(Box::new(Event::New(New::Create))));
        assert!(matches!(app.dialog, Some(Dialog::Unsaved(_))));
        let _ = app.update(Message::DialogClosed);
        assert_eq!(app.dialog, Some(Dialog::Project), "Vazgeç comes back here");
        assert_eq!(
            app.document.as_ref().expect("open").name(),
            "Örnek pafta.kcad"
        );
        // Asked again, and this time the work is left.
        let _ = app.update(Message::Project(Box::new(Event::New(New::Create))));
        let _ = app.update(Message::DialogConfirmed);
        assert_eq!(app.document.as_ref().expect("open").name(), "Yeni proje");
    }

    #[test]
    fn an_empty_name_is_refused() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("file.new"));
        send(&mut app, Event::New(New::Name("   ".into())));
        let _ = app.update(Message::Project(Box::new(Event::New(New::Create))));
        assert_eq!(app.dialog, Some(Dialog::Project));
        assert_ne!(app.document.as_ref().expect("open").name(), "   ");
    }

    /// Pictures of the windows for the owner, dark and light, written to
    /// `.run/shots` (never committed); not run by default:
    ///
    /// ```text
    /// cargo test -p kentos-desktop project::tests::screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use iced::Size;
        use kentos_ui::snapshot::Snapshot;

        use super::settings::Section;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let picture = |app: &mut App, name: &str| {
            let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, App::view, &mut update);
            let _ = snapshot.render(app.view(), &app.theme());
            let file = out.join(format!("{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        for mode in ["dark", "light"] {
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let _ = app.update(Message::Run("file.new"));
            picture(&mut app, &format!("proje-{mode}-1-yeni"));
            send(&mut app, Event::Close);
            let _ = app.update(Message::Run("file.settings"));
            picture(&mut app, &format!("proje-{mode}-2-ayarlar-genel"));
            send(&mut app, Event::Settings(Settings::Section(Section::Crs)));
            send(&mut app, Event::Settings(Settings::Crs(2322)));
            picture(&mut app, &format!("proje-{mode}-3-ayarlar-sistem"));
            send(&mut app, Event::Settings(Settings::Section(Section::Units)));
            send(
                &mut app,
                Event::Settings(Settings::AreaUnit(AreaUnit::Donum)),
            );
            picture(&mut app, &format!("proje-{mode}-4-ayarlar-birimler"));
        }
    }

    #[test]
    fn project_settings_work_on_a_draft_and_kaydet_assigns_them() {
        let mut app = app_with_drawing();
        let before = app.document.as_ref().expect("open").settings().clone();
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, Event::Settings(Settings::Name("Pafta 12".into())));
        send(
            &mut app,
            Event::Settings(Settings::AreaUnit(AreaUnit::Donum)),
        );
        send(&mut app, Event::Settings(Settings::Crs(5255)));
        assert_eq!(
            app.document.as_ref().expect("open").settings(),
            &before,
            "nothing before Kaydet"
        );
        send(&mut app, Event::Close);
        assert_eq!(
            app.document.as_ref().expect("open").settings(),
            &before,
            "Vazgeç changes nothing"
        );

        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, Event::Settings(Settings::Name("Pafta 12".into())));
        send(
            &mut app,
            Event::Settings(Settings::AreaUnit(AreaUnit::Donum)),
        );
        send(&mut app, Event::Settings(Settings::Crs(5255)));
        send(&mut app, Event::Settings(Settings::Save));
        let doc = app.document.as_ref().expect("open");
        assert_eq!(doc.name(), "Pafta 12");
        assert_eq!(doc.settings().area_unit, AreaUnit::Donum);
        assert_eq!(doc.settings().srid, 5255);
        assert!(doc.dirty() && !doc.model.can_undo());
        assert_eq!(
            last_said(&app),
            "Proje ayarları kaydedildi. Proje dosyasıyla birlikte saklanacak."
        );
        assert!(app.history.iter().any(|e| matches!(e,
            kentos_ui::widget::command_line::Entry::Output(t)
                if t == "Proje koordinat sistemi TUREF / TM33 (EPSG:5255) olarak atandı. Koordinat değerleri değiştirilmedi.")));
    }
}

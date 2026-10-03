//! The project's windows (the web's `ui/settings/NewProjectWizard.ts`,
//! `ProjectSettingsDialog.ts` and `ProjectTypeDialog.ts`): Yeni proje's
//! wizard, Proje ayarları and the type question of a project opened without
//! one. Nothing changes until the window's button. A new project replaces the drawing;
//! unsaved work is asked about first, over the window, and Vazgeç there
//! comes back to it. Project settings are assigned to the drawing: an edit,
//! not an undo step, and a new coordinate system is assigned, never a
//! transformation of the coordinates (CLAUDE.md §5).

mod ask_type;
mod content;
pub(crate) mod settings;
mod wizard;

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
    New(Box<wizard::State>),
    Settings(Box<settings::State>),
    Type(Box<ask_type::State>),
}

#[derive(Debug, Clone)]
pub enum Event {
    New(wizard::Event),
    Settings(settings::Event),
    Type(ask_type::Event),
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
                let state = wizard::State::new(self);
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
            Event::Type(e) => {
                self.project_type_event(e);
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

    /// Esc or × closed a project window (its state goes): the type question
    /// says what that leaves.
    pub(crate) fn project_dismissed(&mut self) {
        if let Some(Window::Type(s)) = self.project.take() {
            self.type_left_unasked(&s.name);
        }
    }

    pub(crate) fn project_view(&self) -> Element<'_, Message> {
        match &self.project {
            Some(Window::New(s)) => self.new_project_view(s),
            Some(Window::Settings(s)) => self.project_settings_view(s),
            Some(Window::Type(s)) => self.project_type_view(s),
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

/// The project types as cards (the web's `workspacePicker`): the ones that
/// can be chosen, then, with `soon`, the announced ones, dimmed, with
/// “Yakında”. `compact` leaves out what each is for (Proje ayarları) and the
/// announced ones.
fn modes<'a, M: Clone + 'a>(
    value: Workspace,
    compact: bool,
    soon: bool,
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
    let announced: Vec<Element<'a, M>> = all.iter().filter(|m| !m.ready).map(card).collect();
    let mut out = Column::new()
        .spacing(8)
        .push(Row::with_children(ready).spacing(8));
    if !announced.is_empty() && soon && !compact {
        out = out
            .push(label::caption("Yakında"))
            .push(Row::with_children(announced).spacing(8));
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
    use kentos_contracts::{AreaUnit, DrawingUnit, Workspace};

    use super::Event;
    use super::settings::Event as Settings;
    use super::wizard::Event as New;
    use crate::app::{App, Dialog, Message};
    use crate::files_testing::{app_with_drawing, last_said};

    fn send(app: &mut App, event: Event) {
        let _ = app.update(Message::Project(Box::new(event)));
    }

    /// Yeni proje's note about the drawing on screen: the web's cases
    /// (newProjectNote.test.ts; docs/inventory/parity-audit.md N1).
    #[test]
    fn yeni_proje_says_what_becomes_of_the_drawing_on_screen() {
        use super::wizard::{Note, note};
        let cloud = |database: bool, autosaves: bool| Some(("Ada 101", autosaves, database));
        // A database project that saves by itself is closed, what waits sent.
        assert_eq!(
            note("Ada 101", cloud(true, true), true),
            Some(Note {
                warn: false,
                text: "“Ada 101” bulut projesi kapanır. Bekleyen değişiklikleri buluta gönderilir; gönderilemeyenler bu cihazda kalır ve proje yeniden açılınca geri gelir.".into(),
            })
        );
        // One that does not save: its edits are not sent, what to do is asked.
        assert_eq!(
            note("Ada 101", cloud(true, false), true),
            Some(Note {
                warn: true,
                text: "“Ada 101” projesindeki değişiklikleriniz buluta kaydedilmiyor; Oluştur’a basınca ne yapılacağı sorulur.".into(),
            })
        );
        // A file project's edits are saved by Kaydet: the unsaved question, as a local drawing's.
        let local = note("Ada 101", None, true);
        assert_eq!(note("Ada 101", cloud(false, false), true), local);
        assert_eq!(
            local,
            Some(Note {
                warn: true,
                text: "“Ada 101” içinde kaydedilmemiş değişiklikler var; Oluştur’a basınca önce sorulur."
                    .into(),
            })
        );
        // Nothing over a clean drawing that does not save by itself.
        assert_eq!(
            [
                note("Ada 101", None, false),
                note("Ada 101", cloud(false, false), false),
                note("Ada 101", cloud(true, false), false),
            ],
            [None, None, None]
        );
    }

    /// Made on the start screen, before the drawing area has its size, a new
    /// project opens on its sheet at its scale once the area has it (it
    /// opened at about 1:500 000, docs/adr/0165 §3).
    #[test]
    fn a_new_project_made_before_the_area_has_its_size_opens_at_its_scale() {
        use iced::{Point, Rectangle, Size};

        let (mut app, _) = App::boot(None);
        let _ = app.update(Message::Run("file.new"));
        send(&mut app, Event::New(New::Go(2)));
        send(&mut app, Event::New(New::Next));
        assert!(app.document.is_some());
        let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
            Rectangle::new(Point::ORIGIN, Size::new(1000.0, 750.0)),
        )));
        // The sheet (500 × 375 m at 1:1000) fills the area: about 2 pixels a metre.
        let scale = app.viewport.camera.scale;
        assert!((1.5..2.1).contains(&scale), "{scale}");
    }

    #[test]
    fn unsaved_work_is_asked_about_and_vazgec_comes_back_to_the_window() {
        let mut app = app_with_drawing();
        let layer = app.document.as_ref().expect("open").layers()[0].id.clone();
        let _ = app.update(Message::LayerLocked(layer));
        assert!(app.document.as_ref().expect("open").dirty());
        let _ = app.update(Message::Run("file.new"));
        send(&mut app, Event::New(New::Go(2)));
        send(&mut app, Event::New(New::Next));
        assert!(matches!(app.dialog, Some(Dialog::Unsaved(_))));
        let _ = app.update(Message::DialogClosed);
        assert_eq!(app.dialog, Some(Dialog::Project), "Vazgeç comes back here");
        assert_eq!(
            app.document.as_ref().expect("open").name(),
            "Örnek pafta.kcad"
        );
        // Asked again, and this time the work is left.
        send(&mut app, Event::New(New::Next));
        let _ = app.update(Message::DialogConfirmed);
        assert_eq!(app.document.as_ref().expect("open").name(), "Yeni proje");
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
            send(&mut app, Event::Close);
            // The type question of a project opened without one (docs/adr/0165 §1).
            app.ask_project_type();
            picture(&mut app, &format!("proje-{mode}-5-tur-sorusu"));
            send(&mut app, Event::Close);
            // A local project in millimetres (docs/adr/0165 §2): a new one without a coordinate
            // system, at 1:1 (at 1:1000 a 0.25 mm pen would be 25 cm wide on a part), its unit in
            // Proje ayarları, then a plate 120 × 80 mm with a hole, selected.
            let _ = app.update(Message::Run("file.new"));
            send(&mut app, Event::New(New::Type(Workspace::Cad)));
            send(&mut app, Event::New(New::Next));
            send(&mut app, Event::New(New::Unit(DrawingUnit::Mm)));
            send(&mut app, Event::New(New::Next));
            send(&mut app, Event::New(New::Name("Mil plakası".into())));
            send(&mut app, Event::New(New::Next));
            if matches!(app.dialog, Some(Dialog::Unsaved(_))) {
                let _ = app.update(Message::DialogConfirmed);
            }
            let _ = app.update(Message::Run("file.settings"));
            send(&mut app, Event::Settings(Settings::Section(Section::Units)));
            send(
                &mut app,
                Event::Settings(Settings::DrawingUnit(DrawingUnit::Mm)),
            );
            picture(&mut app, &format!("proje-{mode}-6-yerel-birim"));
            send(&mut app, Event::Settings(Settings::Save));
            let plate = local_plate(&mut app);
            app.selection.set(vec![plate]);
            let _ = app.update(Message::Run("view.zoomSelection"));
            // Genel closed, so that the plate's measures show.
            let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
                "general",
            )));
            picture(&mut app, &format!("proje-{mode}-7-yerel-cizim"));
        }
    }

    /// A plate 120 × 80 mm with a hole of 25 mm radius at the origin of a
    /// local drawing (kept in metres); the plate's slot.
    fn local_plate(app: &mut App) -> kentos_domain::Slot {
        use kentos_contracts::{CircleEntity, Entity, EntityBase, PathEntity, Vec2};
        let base = || EntityBase {
            id: 0,
            layer_id: app
                .document
                .as_ref()
                .expect("open")
                .model
                .layers()
                .active()
                .to_owned(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
        };
        let pts = [(0.0, 0.0), (0.12, 0.0), (0.12, 0.08), (0.0, 0.08)];
        let plate = Entity::Polygon(PathEntity {
            base: base(),
            pts: pts.iter().map(|&(x, y)| Vec2 { x, y }).collect(),
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
        });
        let hole = Entity::Circle(CircleEntity {
            base: base(),
            c: Vec2 { x: 0.06, y: 0.04 },
            r: 0.025,
        });
        let model = &mut app.document.as_mut().expect("open").model;
        let slot = model.add(plate).expect("a slot");
        model.add(hole).expect("a slot");
        slot
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
        assert!(app.log.lines().any(|l| l.text
            == "Proje koordinat sistemi TUREF / TM33 (EPSG:5255) olarak atandı. Koordinat değerleri değiştirilmedi."));
    }
}

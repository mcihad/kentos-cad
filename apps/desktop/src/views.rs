//! Görünümler (docs/adr/0285): the project's named views and the view's
//! twist. A named view keeps where the camera looks: its middle, the world
//! width and height it shows and its twist; it is kept in the project's
//! settings through `cad.views.edit` (an edit, no undo step), as Python and
//! MCP keep them. Going to one is the camera's: a navigation Önceki görünüm
//! comes back from, never a change of the drawing.
//!
//! The window lists the views. A view is chosen by a click (its name comes
//! into Ad), shown with Göster (or a double click), saved again from the
//! view as it is with Güncelle, removed with Sil. Yeni görünüm kaydet keeps
//! the view as it is under the name in Ad; Yeniden adlandır gives the chosen
//! view that name. The window stays open; Kapat closes it.
//!
//! Görünümü UCS'e döndür lays the UCS's x axis horizontal (AutoCAD's PLAN),
//! Kuzey yukarı turns the view back; Görünümü döndür is the tool
//! (`kentos_interaction::view_twist`).

use iced::widget::{Column, button, container, row, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{NamedView, ViewsEdit, ViewsEdited, ViewsOp};
use kentos_interaction::{Level, ViewChange};
use kentos_native_application::{ExecutionContext, views_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, Elided, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const VIEWS_TITLE: &str = "Görünümler";
/// Why the named views may not change here.
pub const VIEWS_LOCKED: &str = "Bu projede adlı görünümleri değiştirme yetkiniz yok (project.edit); proje sahibinden ya da yöneticisinden isteyin.";
const NAME: &str = "Ad";
const SHOW: &str = "Göster";
const UPDATE: &str = "Güncelle";
const REMOVE: &str = "Sil";
const SAVE: &str = "Yeni görünüm kaydet";
const RENAME: &str = "Yeniden adlandır";
const CLOSE: &str = "Kapat";

/// The commands this module runs.
pub const COMMANDS: [&str; 3] = ["view.namedViews", "view.twistUcs", "view.northUp"];

/// The first “Görünüm n” no view is named.
pub fn next_view_name(views: &[NamedView]) -> String {
    (1..)
        .map(|n| format!("Görünüm {n}"))
        .find(|name| !views.iter().any(|v| &v.name == name))
        .unwrap_or_default()
}

/// The window: the chosen view and the name field.
#[derive(Debug, Clone)]
pub struct Window {
    chosen: Option<String>,
    name: String,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A view's row: it is chosen; a second press shows it.
    Choose(String),
    /// Shows a view: Göster.
    Show(String),
    Update,
    Remove,
    Save,
    Rename,
    Name(String),
    Close,
}

fn msg(event: Event) -> Message {
    Message::Views(event)
}

/// A view's size and twist in words: “60 × 40 m, 30°”.
fn view_caption(app: &App, v: &NamedView) -> String {
    let f = app.format();
    let mut out = format!("{} × {}", f.length(v.width), f.length(v.height));
    if v.twist != 0.0 {
        out.push_str(&format!(", {}", f.angle(v.twist)));
    }
    out
}

impl App {
    pub(crate) fn views_command(&mut self, id: &str) -> Task<Message> {
        match id {
            "view.namedViews" => self.open_views(),
            "view.twistUcs" => {
                let angle = self
                    .document
                    .as_ref()
                    .and_then(|d| d.model.settings().ucs)
                    .map_or(0.0, |u| u.angle);
                self.twist_view(angle);
            }
            "view.northUp" => self.twist_view(0.0),
            _ => {}
        }
        Task::none()
    }

    /// The view turned to `angle` (radians, counter-clockwise from east) about its middle: a navigation.
    pub(crate) fn twist_view(&mut self, angle: f64) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.navigating(|app| app.viewport.change(ViewChange::Twist(angle)));
        let said = if self.viewport.camera.twist == 0.0 {
            "Görünüm: kuzey yukarı".to_owned()
        } else {
            format!(
                "Görünüm döndürüldü: {}",
                self.format().angle(self.viewport.camera.twist)
            )
        };
        self.say(Level::Success, said);
    }

    /// The open drawing's named views.
    pub(crate) fn named_views(&self) -> &[NamedView] {
        self.document
            .as_ref()
            .map_or(&[], |d| d.model.settings().named_views.as_slice())
    }

    /// Why the named views may not change in the open drawing, or none.
    fn views_locked(&self) -> Option<&'static str> {
        self.tree_locked().map(|_| VIEWS_LOCKED)
    }

    /// `cad.views.edit` on the open drawing.
    pub(crate) fn edit_views(&mut self, edit: ViewsOp) -> Option<ViewsEdited> {
        if let Some(locked) = self.views_locked() {
            self.warn(locked);
            return None;
        }
        let doc = self.document.as_mut()?;
        let result = views_edit::execute(
            &mut ExecutionContext::new(&mut doc.model),
            ViewsEdit {
                edit,
                expected_revision: None,
            },
        );
        self.answered(result)
    }

    /// The view as it is kept under `name` (Kaydet, Güncelle).
    pub(crate) fn save_view(&mut self, name: &str) -> Option<NamedView> {
        let v = self.viewport.named(name.trim());
        let done = self.edit_views(ViewsOp::Save {
            name: v.name,
            center: v.center,
            width: v.width,
            height: v.height,
            twist: Some(v.twist),
        })?;
        let kept = done.view?;
        self.say(
            Level::Success,
            format!(
                "“{}” görünümü kaydedildi: {}.",
                kept.name,
                view_caption(self, &kept)
            ),
        );
        Some(kept)
    }

    /// Goes to the named view `name`: a navigation (Önceki görünüm comes back).
    pub(crate) fn show_named_view(&mut self, name: &str) {
        let Some(v) = self.named_views().iter().find(|v| v.name == name).cloned() else {
            self.warn(format!("“{name}” adlı bir görünüm yok."));
            return;
        };
        self.navigating(|app| app.viewport.show_view(&v));
        self.say(Level::Info, format!("Görünüm: {}", v.name));
    }

    /// Görünümler (`view.namedViews`): the window.
    pub(crate) fn open_views(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.views_window = Some(Window {
            chosen: None,
            name: next_view_name(self.named_views()),
        });
        self.dialog = Some(Dialog::Views);
    }

    pub(crate) fn views_event(&mut self, event: Event) -> Task<Message> {
        let chosen = self.views_window.as_ref().and_then(|w| w.chosen.clone());
        let name = self
            .views_window
            .as_ref()
            .map(|w| w.name.clone())
            .unwrap_or_default();
        match event {
            Event::Choose(id) => {
                if chosen.as_deref() == Some(id.as_str()) {
                    self.show_named_view(&id);
                }
                if let Some(w) = self.views_window.as_mut() {
                    w.name = id.clone();
                    w.chosen = Some(id);
                }
            }
            Event::Show(id) => self.show_named_view(&id),
            Event::Update => {
                if let Some(id) = chosen {
                    self.save_view(&id);
                }
            }
            Event::Remove => {
                if let Some(id) = chosen
                    && self
                        .edit_views(ViewsOp::Delete { name: id.clone() })
                        .is_some()
                {
                    self.say(Level::Success, format!("“{id}” görünümü silindi."));
                    let next = next_view_name(self.named_views());
                    if let Some(w) = self.views_window.as_mut() {
                        w.chosen = None;
                        w.name = next;
                    }
                }
            }
            Event::Save => {
                if let Some(kept) = self.save_view(&name)
                    && let Some(w) = self.views_window.as_mut()
                {
                    w.chosen = Some(kept.name.clone());
                    w.name = kept.name;
                }
            }
            Event::Rename => {
                if let Some(id) = chosen {
                    let to = name.trim().to_owned();
                    if to != id
                        && self
                            .edit_views(ViewsOp::Rename {
                                name: id.clone(),
                                to: to.clone(),
                            })
                            .is_some()
                    {
                        self.say(
                            Level::Success,
                            format!("“{id}” görünümünün adı “{to}” oldu."),
                        );
                        if let Some(w) = self.views_window.as_mut() {
                            w.chosen = Some(to);
                        }
                    }
                }
            }
            Event::Name(text) => {
                if let Some(w) = self.views_window.as_mut() {
                    w.name = text;
                }
            }
            Event::Close => {
                self.views_window = None;
                self.dialog = None;
            }
        }
        // A view the window had chosen and that is gone (removed elsewhere) is chosen no more.
        let gone = self
            .views_window
            .as_ref()
            .and_then(|w| w.chosen.clone())
            .is_some_and(|id| !self.named_views().iter().any(|v| v.name == id));
        if gone && let Some(w) = self.views_window.as_mut() {
            w.chosen = None;
        }
        Task::none()
    }

    pub(crate) fn views_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.views_window else {
            return iced::widget::text("").into();
        };
        let locked = self.views_locked();
        let chosen = w.chosen.as_deref();
        let mut list = Column::new().spacing(1).padding(2);
        if self.named_views().is_empty() {
            list = list.push(
                container(label::caption(
                    "Kayıtlı görünüm yok. Çizimi istediğiniz gibi gösterin, aşağıya bir ad yazıp Yeni görünüm kaydet’e basın.",
                ))
                .padding(10),
            );
        }
        for v in self.named_views() {
            let name = Elided::new(v.name.clone())
                .size(typography::body())
                .font(typography::ui())
                .width(Fill);
            let caption = label::caption(view_caption(self, v)).style(|theme: &iced::Theme| {
                iced::widget::text::Style {
                    color: Some(Tokens::of(theme).muted),
                }
            });
            let face = row![name, caption].spacing(6).align_y(Center);
            list = list.push(
                button(face)
                    .on_press(msg(Event::Choose(v.name.clone())))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(chosen == Some(v.name.as_str()))),
            );
        }
        let list = container(iced::widget::scrollable(list).height(typography::scaled(200.0)))
            .style(style::container::field_box)
            .width(Fill);
        let editable = locked.is_none();
        let actions = row![
            words::secondary(SHOW, chosen.map(|id| msg(Event::Show(id.to_owned())))),
            words::secondary(
                UPDATE,
                chosen.filter(|_| editable).map(|_| msg(Event::Update))
            ),
            words::secondary(
                REMOVE,
                chosen.filter(|_| editable).map(|_| msg(Event::Remove))
            ),
        ]
        .spacing(8);
        let name = text_input("Görünümün adı", &w.name)
            .on_input(|t| msg(Event::Name(t)))
            .on_submit(msg(Event::Save))
            .padding([5, 8])
            .style(style::field::input);
        let saving = row![
            words::primary(SAVE, editable.then(|| msg(Event::Save))),
            words::secondary(
                RENAME,
                chosen.filter(|_| editable).map(|_| msg(Event::Rename))
            ),
        ]
        .spacing(8);
        let mut body = Column::new().spacing(12);
        if let Some(why) = locked {
            body = body.push(words::summary(vec![words::text_line(Kind::Warn, why)]));
        }
        let body = body
            .push(words::field(
                "Kayıtlı görünümler",
                list,
                Some("Bir görünüme iki kez tıklamak onu gösterir; Önceki görünüm geri getirir.".to_owned()),
            ))
            .push(actions)
            .push(words::field(NAME, focus_ring(name), None))
            .push(saving);
        overlay::modal(
            Frame::new(VIEWS_TITLE)
                .push(body)
                .action(words::secondary(CLOSE, Some(msg(Event::Close))))
                .width(520.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step): a view's row by its name, Ad and the buttons.
    pub(crate) fn views_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.views_window else {
            return Err(format!("{VIEWS_TITLE} penceresi açık değil"));
        };
        let editable = self.views_locked().is_none();
        let chosen = w.chosen.clone();
        Ok(match control {
            Control::Fill(NAME, text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Press(SHOW) => chosen.map(|id| msg(Event::Show(id))),
            Control::Press(UPDATE) => chosen.filter(|_| editable).map(|_| msg(Event::Update)),
            Control::Press(REMOVE) => chosen.filter(|_| editable).map(|_| msg(Event::Remove)),
            Control::Press(SAVE) => editable.then(|| msg(Event::Save)),
            Control::Press(RENAME) => chosen.filter(|_| editable).map(|_| msg(Event::Rename)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(words) => match self.named_views().iter().find(|v| v.name == words) {
                Some(v) => Some(msg(Event::Choose(v.name.clone()))),
                None => {
                    return Err(format!("“{VIEWS_TITLE}” penceresinde “{words}” düğmesi yok"));
                }
            },
            other => return Err(format!("“{VIEWS_TITLE}” penceresinde {other} yok")),
        })
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Rectangle, Size};

    use crate::app::{App, Message};
    use crate::files_testing::{app_with_drawing, last_said};
    use crate::viewport::Event as Area;

    use super::Event;

    fn open() -> App {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Viewport(Area::Resized(Rectangle::new(
            Point::ORIGIN,
            Size::new(1000.0, 800.0),
        ))));
        app
    }

    fn run(app: &mut App, id: &'static str) {
        let _ = app.update(Message::Run(id));
    }

    #[test]
    fn a_view_is_kept_shown_renamed_and_removed_and_showing_it_is_a_navigation() {
        let mut app = open();
        run(&mut app, "view.namedViews");
        assert!(app.views_window.is_some());
        // The view as it is, turned 30°, under “Giriş”.
        app.twist_view(std::f64::consts::FRAC_PI_6);
        let kept = app.viewport.viewpoint();
        let _ = app.update(Message::Views(Event::Name("Giriş".into())));
        let _ = app.update(Message::Views(Event::Save));
        assert!(last_said(&app).starts_with("“Giriş” görünümü kaydedildi"), "{}", last_said(&app));
        let views = app.named_views().to_vec();
        assert_eq!(views.len(), 1);
        assert!((views[0].twist - std::f64::consts::FRAC_PI_6).abs() < 1e-12);
        // Elsewhere, north up; Göster brings the view back, Önceki görünüm the one before.
        run(&mut app, "view.northUp");
        run(&mut app, "view.zoomIn");
        let before = app.viewport.viewpoint();
        let _ = app.update(Message::Views(Event::Show("Giriş".into())));
        let shown = app.viewport.viewpoint();
        assert!((shown.center.x - kept.center.x).abs() < 1e-6 && (shown.center.y - kept.center.y).abs() < 1e-6);
        assert!((shown.scale / kept.scale - 1.0).abs() < 1e-9);
        assert!((shown.twist - kept.twist).abs() < 1e-12);
        run(&mut app, "view.previous");
        assert_eq!(app.viewport.viewpoint(), before);
        // Renamed, then removed.
        let _ = app.update(Message::Views(Event::Choose("Giriş".into())));
        let _ = app.update(Message::Views(Event::Name("Ana giriş".into())));
        let _ = app.update(Message::Views(Event::Rename));
        assert_eq!(app.named_views()[0].name, "Ana giriş");
        let _ = app.update(Message::Views(Event::Remove));
        assert!(app.named_views().is_empty());
        assert_eq!(last_said(&app), "“Ana giriş” görünümü silindi.");
    }

    #[test]
    fn the_twist_commands_turn_the_view_about_its_middle() {
        let mut app = open();
        let middle = app.viewport.camera.center;
        app.twist_view(1.0);
        assert_eq!(app.viewport.camera.twist, 1.0);
        assert_eq!(app.viewport.camera.center, middle);
        run(&mut app, "view.northUp");
        assert_eq!(app.viewport.camera.twist, 0.0);
        assert_eq!(last_said(&app), "Görünüm: kuzey yukarı");
        // Önceki görünüm turns it back.
        run(&mut app, "view.previous");
        assert_eq!(app.viewport.camera.twist, 1.0);
    }

    fn click(app: &mut App, at: Point) {
        let _ = app.update(Message::Viewport(Area::Moved(at)));
        let _ = app.update(Message::Viewport(Area::Pressed(at)));
        let _ = app.update(Message::Viewport(Area::Released(at)));
    }

    /// docs/adr/0285 §2: in a twisted view a click places the corner under the pointer, a click on the object
    /// picks it, and a box drawn on the screen selects what the turned box holds.
    #[test]
    fn a_twisted_view_places_picks_and_selects_where_the_screen_shows() {
        let mut app = open();
        app.draft.snap = false;
        app.draft.ortho = false;
        app.draft.polar = None;
        app.twist_view(0.6);
        run(&mut app, "tool.polyline");
        let corners = [Point::new(200.0, 300.0), Point::new(320.0, 300.0), Point::new(320.0, 420.0)];
        for c in corners {
            click(&mut app, c);
        }
        run(&mut app, "tool.confirm");
        run(&mut app, "tool.cancel");
        let doc = &app.document.as_ref().expect("a drawing").model;
        let newest = doc.entities().max_by_key(|e| e.base().id).expect("the polyline").clone();
        let kentos_contracts::Entity::Polyline(p) = &newest else {
            panic!("a polyline: {newest:?}");
        };
        for (c, q) in corners.iter().zip(&p.pts) {
            let w = app.viewport.world(*c);
            assert!((w.x - q.x).abs() < 1e-6 && (w.y - q.y).abs() < 1e-6, "{w:?} {q:?}");
        }
        // A click on its middle edge, in the twisted view, picks it.
        app.selection.clear();
        click(&mut app, Point::new(320.0, 360.0));
        assert_eq!(app.selection.len(), 1);
        // A window drawn on the screen round it selects it; a window round half of it does not.
        app.selection.clear();
        let drag = |app: &mut App, a: Point, b: Point| {
            let _ = app.update(Message::Viewport(Area::Moved(a)));
            let _ = app.update(Message::Viewport(Area::Pressed(a)));
            let _ = app.update(Message::Viewport(Area::Moved(Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0))));
            let _ = app.update(Message::Viewport(Area::Moved(b)));
            let _ = app.update(Message::Viewport(Area::Released(b)));
        };
        drag(&mut app, Point::new(180.0, 280.0), Point::new(340.0, 440.0));
        assert_eq!(app.selection.len(), 1, "wholly inside the box on the screen");
        app.selection.clear();
        drag(&mut app, Point::new(180.0, 280.0), Point::new(330.0, 380.0));
        assert!(app.selection.is_empty(), "half outside a window");
    }
}

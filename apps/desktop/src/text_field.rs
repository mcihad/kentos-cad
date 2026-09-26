//! The text field over the drawing (the web's `ui/shell/InlineTextEditor.ts`),
//! for two things:
//!
//! - Yazı: the tool asks for it where the text will start
//!   (`ViewChange::Text`); what is typed goes back to the tool
//!   (`Tool::text_typed`);
//! - a double click on a text or a dimension, no command running, edits its
//!   value in place (the web's `SelectTool.maybeEditText`); the drawn text
//!   is hidden meanwhile.
//!
//! Enter keeps what is typed (trimmed), Esc drops it. As the web's blur, a
//! press on the drawing or a command from the ribbon or a menu keeps it
//! first: a click elsewhere with Yazı running keeps this text and opens the
//! next field there. The field follows the drawing when the view pans or
//! zooms. It is not turned with the text: Iced's text box does not rotate
//! (the web turns it with CSS).

use std::time::{Duration, Instant};

use iced::widget::{column, container, pin, text_input};
use iced::{Element, Fill, Task};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::{TextField, Vec2, dimension_layout};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::{label, style};

use crate::app::{App, Message};

/// The field's widget id: focused when it opens.
pub const ID: &str = "cizim-yazi-alani";

/// Two presses on the same object closer than this are a double click (the web's 450 ms).
const DOUBLE_CLICK: Duration = Duration::from_millis(450);

/// An open field.
#[derive(Debug, Clone, PartialEq)]
pub struct Open {
    /// Where its text starts (a new text, a text), or a dimension's value's centre.
    pub at: Vec2,
    /// The text's height, metres: the field's type size follows it.
    pub height: f64,
    pub centered: bool,
    pub text: String,
    pub placeholder: String,
    pub hint: &'static str,
    /// The text or dimension being edited; none for Yazı's new text.
    pub editing: Option<Slot>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Input(String),
    /// Enter.
    Keep,
}

impl App {
    /// Yazı asked for a field where its text will start.
    pub(crate) fn open_text_field(&mut self, field: TextField) {
        self.text_field = Some(Open {
            at: field.at,
            height: field.height,
            centered: false,
            text: String::new(),
            placeholder: "Yazıyı yazın".to_owned(),
            hint: "Enter: ekle · Esc: vazgeç",
            editing: None,
        });
        self.text_field_focus = true;
    }

    /// A left press on the drawing with no command running: a second one on
    /// the same text or dimension soon after opens its field (the web's 450 ms).
    pub(crate) fn maybe_edit_text(&mut self, world: Vec2) {
        let Some(doc) = &self.document else {
            return;
        };
        let tolerance = self.draft.pick_aperture / self.viewport.camera.scale;
        let Some(slot) = self.spatial.pick(world, tolerance) else {
            self.last_click = None;
            return;
        };
        let now = Instant::now();
        let double = self
            .last_click
            .is_some_and(|(last, at)| last == slot && now.duration_since(at) <= DOUBLE_CLICK);
        self.last_click = Some((slot, now));
        if !double {
            return;
        }
        let Some(entity) = doc.model.get(slot) else {
            return;
        };
        if !matches!(entity, Entity::Text(_) | Entity::Dimension(_)) {
            return;
        }
        if doc.model.layers().is_locked(&entity.base().layer_id) {
            self.warn("Kilitli katmandaki yazı düzenlenemez.");
            return;
        }
        self.last_click = None;
        let format = kentos_interaction::Format::of(doc.settings());
        let open = match entity {
            Entity::Text(t) => Open {
                at: Vec2::new(t.p.x, t.p.y),
                height: t.height,
                centered: false,
                text: t.text.clone(),
                placeholder: String::new(),
                hint: "Enter: kaydet · Esc: vazgeç",
                editing: Some(slot),
            },
            Entity::Dimension(d) => {
                let Some(layout) = dimension_layout(entity) else {
                    return;
                };
                Open {
                    at: layout.text_at,
                    height: d.height,
                    centered: true,
                    text: d.text.clone().unwrap_or_default(),
                    // The measured value as drawn, shown when the text is cleared.
                    placeholder: crate::exchange::dimension_text(
                        &format,
                        layout.prefix,
                        layout.unit,
                        layout.value,
                    ),
                    hint: "Enter: kaydet · Esc: vazgeç",
                    editing: Some(slot),
                }
            }
            _ => return,
        };
        self.text_field = Some(open);
        self.text_field_focus = true;
        self.text_field_select = true;
    }

    pub(crate) fn text_field_event(&mut self, event: Event) {
        match event {
            Event::Input(text) => {
                if let Some(open) = &mut self.text_field {
                    open.text = text;
                }
            }
            Event::Keep => self.close_text_field(true),
        }
    }

    /// Closes the field: `keep`, what is typed goes where it belongs (the
    /// web's `close(commit)`); otherwise nothing changes.
    pub(crate) fn close_text_field(&mut self, keep: bool) {
        let Some(open) = self.text_field.take() else {
            return;
        };
        self.text_field_release = true;
        let value = open.text.trim().to_owned();
        match open.editing {
            None => {
                let typed = (keep && !value.is_empty()).then_some(value.as_str());
                self.with_tool(|s, cx| s.text_typed(typed, cx));
            }
            Some(slot) if keep => {
                let Some(doc) = &mut self.document else {
                    return;
                };
                let changed = match doc.model.get(slot) {
                    Some(Entity::Text(t)) if !value.is_empty() && value != t.text => {
                        let mut t = t.clone();
                        t.text = value;
                        Some(Entity::Text(t))
                    }
                    Some(Entity::Dimension(d)) if value != d.text.clone().unwrap_or_default() => {
                        let mut d = d.clone();
                        d.text = (!value.is_empty()).then_some(value);
                        Some(Entity::Dimension(d))
                    }
                    _ => None,
                };
                // The web's `doc.update`: one undo step, “Değiştir”.
                if let Some(entity) = changed {
                    doc.model.update(slot, entity);
                }
            }
            Some(_) => {}
        }
    }

    /// The field over the drawing, at its text; `None` when closed.
    pub(crate) fn text_field_view(&self) -> Option<Element<'_, Message>> {
        let open = self.text_field.as_ref()?;
        let camera = &self.viewport.camera;
        let [x, y] = camera.world_to_screen(open.at);
        // The web's size: the text's height on screen, 13 to 48 px.
        let size = (open.height * camera.scale).clamp(13.0, 48.0) as f32;
        let chars = open.text.chars().count().max(open.placeholder.chars().count()).max(8);
        let width = (chars as f32 * size * 0.62 + 24.0).max(160.0);
        let input = text_input(&open.placeholder, &open.text)
            .id(ID)
            .on_input(|text| Message::TextField(Event::Input(text)))
            .on_submit(Message::TextField(Event::Keep))
            .font(typography::ui())
            .size(size)
            .padding([2, 6])
            .width(width)
            .style(style::field::input);
        let hint = container(label::caption(open.hint)).padding([2, 6]).style(
            |theme: &iced::Theme| container::Style {
                background: Some(Tokens::of(theme).surface.into()),
                ..container::Style::default()
            },
        );
        // Its baseline at the text's (the web's `translate(0, -85%)`); a
        // dimension's value centred on its place.
        let field_height = size * 1.25 + 4.0;
        let left = if open.centered {
            x as f32 - width / 2.0
        } else {
            x as f32
        };
        let top = y as f32 - field_height * 0.85;
        Some(
            pin(column![input, hint].spacing(2))
                .x(left.max(0.0))
                .y(top.max(0.0))
                .width(Fill)
                .height(Fill)
                .into(),
        )
    }

    /// Tasks a field that opened or closed asks for: the keyboard to it, its
    /// text chosen (an edit), or the keyboard back to the drawing.
    pub(crate) fn text_field_tasks(&mut self) -> Task<Message> {
        let mut tasks = Vec::new();
        if std::mem::take(&mut self.text_field_focus) {
            tasks.push(iced::widget::operation::focus(ID));
        }
        if std::mem::take(&mut self.text_field_select) {
            tasks.push(iced::widget::operation::select_all(ID));
        }
        if std::mem::take(&mut self.text_field_release) && self.text_field.is_none() {
            tasks.push(crate::input::release_keyboard());
        }
        Task::batch(tasks)
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use kentos_contracts::Entity;

    use super::Event;
    use crate::app::{App, Message};
    use crate::files_testing::app_with_drawing;
    use crate::viewport;

    fn press(app: &mut App, at: Point) {
        let _ = app.update(Message::Viewport(viewport::Event::Pressed(at)));
    }

    fn texts(app: &App) -> Vec<String> {
        app.document
            .as_ref()
            .expect("open")
            .model
            .entities()
            .filter_map(|e| match e {
                Entity::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn yazi_opens_the_field_where_it_is_clicked_and_enter_adds_the_text() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("tool.text"));
        press(&mut app, Point::new(300.0, 200.0));
        let open = app.text_field.clone().expect("the field opened");
        assert!(open.editing.is_none() && open.text.is_empty());
        assert_eq!(open.placeholder, "Yazıyı yazın");
        let _ = app.update(Message::TextField(Event::Input("Çınar".into())));
        let _ = app.update(Message::TextField(Event::Keep));
        assert!(app.text_field.is_none());
        assert!(texts(&app).contains(&"Çınar".to_owned()));
        assert!(app.session.is_running(), "the tool waits for the next text");
    }

    #[test]
    fn esc_drops_the_text_and_a_click_elsewhere_keeps_it_and_opens_the_next() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("tool.text"));
        let before = texts(&app).len();
        press(&mut app, Point::new(300.0, 200.0));
        let _ = app.update(Message::TextField(Event::Input("Gitmez".into())));
        app.close_text_field(false);
        assert_eq!(texts(&app).len(), before);
        press(&mut app, Point::new(300.0, 200.0));
        let _ = app.update(Message::TextField(Event::Input("Kalır".into())));
        // The web's blur: the press keeps this text, then opens a field there.
        press(&mut app, Point::new(400.0, 260.0));
        assert!(texts(&app).contains(&"Kalır".to_owned()));
        assert!(app.text_field.is_some(), "the next field is open");
    }

    #[test]
    fn a_double_click_edits_a_text_and_a_locked_one_is_said() {
        let mut app = app_with_drawing();
        let model = &app.document.as_ref().expect("open").model;
        let (slot, layer, at) = model
            .entities()
            .find_map(|e| match e {
                Entity::Text(t) => Some((
                    kentos_domain::Slot(t.base.id),
                    t.base.layer_id.clone(),
                    kentos_interaction::Vec2::new(t.p.x, t.p.y),
                )),
                _ => None,
            })
            .expect("the sample has a text");
        // Its layer shown, so it can be picked.
        let doc = app.document.as_mut().expect("open");
        if !doc.model.layers().is_visible(&layer) {
            doc.model.toggle_layer_visible(&layer);
        }
        app.spatial.sync(&app.document.as_ref().expect("open").model);
        app.maybe_edit_text(at);
        app.maybe_edit_text(at);
        let open = app.text_field.clone().expect("the field opened on the text");
        assert_eq!(open.editing, Some(slot));
        assert_eq!(open.hint, "Enter: kaydet · Esc: vazgeç");
        let _ = app.update(Message::TextField(Event::Input("Çınar caddesi".into())));
        let _ = app.update(Message::TextField(Event::Keep));
        assert!(texts(&app).contains(&"Çınar caddesi".to_owned()));
        let model = &mut app.document.as_mut().expect("open").model;
        assert_eq!(model.undo().as_deref(), Some("Değiştir"));
        model.toggle_layer_locked(&layer);
        app.spatial.sync(&app.document.as_ref().expect("open").model);
        app.maybe_edit_text(at);
        app.maybe_edit_text(at);
        assert!(app.text_field.is_none());
        assert_eq!(
            crate::files_testing::last_said(&app),
            "Kilitli katmandaki yazı düzenlenemez."
        );
    }
}

/// Pictures for the owner: Yazı's field with a text typed, and a text of the
/// sample being edited in place; `.run/shots/yazi-kutusu-*`.
/// `cargo test -p kentos-desktop text_field::screens -- --ignored --nocapture`
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["yeni", "duzenle"] {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                // The sample's texts are on its hidden Çizim layer: shown.
                if let Some(doc) = app.document.as_mut()
                    && !doc.model.layers().is_visible("cizim")
                {
                    doc.model.toggle_layer_visible("cizim");
                }
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                app.spatial.sync(&app.document.as_ref().expect("open").model);
                match name {
                    "yeni" => {
                        let _ = app.update(Message::Run("tool.text"));
                        let area = app.viewport.bounds;
                        let at = iced::Point::new(area.width * 0.6, area.height * 0.3);
                        let _ = app.update(Message::Viewport(crate::viewport::Event::Pressed(at)));
                        let _ = app.update(Message::TextField(Event::Input("Ada 101 Parsel 7".into())));
                    }
                    _ => {
                        let at = app
                            .document
                            .as_ref()
                            .and_then(|d| {
                                d.model.entities().find_map(|e| match e {
                                    Entity::Text(t) => Some(Vec2::new(t.p.x, t.p.y)),
                                    _ => None,
                                })
                            })
                            .expect("a text");
                        app.maybe_edit_text(at);
                        app.maybe_edit_text(at);
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("yazi-kutusu-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

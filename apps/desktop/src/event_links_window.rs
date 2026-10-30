//! Olay bağları (docs/adr/0569, `linearRef.eventLinks`): the project's links of event layers to their routes.
//!
//! ```text
//! ┌ Olay bağları ────────────────────────────────────────────────────────────────┐
//! │ Bağlar                │ Ad               [Kazalar                       ]     │
//! │ ● Kazalar             │ Olay katmanı     [Kaza                         ▾]     │
//! │   Kaplama kesimleri   │ Olay türü        [Nokta                        ▾]     │
//! │ [Yeni bağ]            │ Rota katmanları  [x] Yol  [ ] Eski yol                │
//! │                       │ Rota kimliği     [Yol ▾]  Olayın rotası [Yol ▾]       │
//! │ [Bağı sil]            │ Ölçü alanı       [Km ▾]   Öteleme [Sapma ▾]           │
//! │                       │ [ ] Artı öteleme solda        Tolerans [0,001]        │
//! │                                                     [Vazgeç] [Kaydet]        │
//! └──────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the links; Kaydet writes what changed through `cad.linearref.define` (removes, then
//! sets) and closes the window. A link keeps the events of its layer on their routes: every later write that touches
//! a route or an event places the layer's events again in the same undo step (docs/adr/0569 §3).

use iced::widget::{Column, button, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    EventLink, EventLinkKind, EventSide, LinearRefDefine, LinearRefDefineOperation,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, linearref_define};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Olay bağları";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni bağ";
const REMOVE: &str = "Bağı sil";
const LEFT: &str = "Artı öteleme solda";
const NONE: &str = "(yok)";

/// A link being edited, its tolerance as the text the user typed (empty: the default).
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    link: EventLink,
    tolerance: String,
}

impl Row {
    fn of(link: EventLink) -> Self {
        let tolerance = link.tolerance.map(|t| t.to_string()).unwrap_or_default();
        Self { link, tolerance }
    }
}

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    /// The copy being edited and the links as the project has them.
    rows: Vec<Row>,
    kept: Vec<EventLink>,
    chosen: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(usize),
    New,
    Remove,
    Name(String),
    Events(String),
    Kind(EventLinkKind),
    Route(String),
    RouteField(String),
    NamesRoute(String),
    Measure(String),
    From(String),
    To(String),
    /// The offset field by its place in the list (0: none).
    Offset(usize),
    Left,
    Tolerance(String),
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::EventLinks(event)
}

/// An id `base`, `base-2`, … none of `taken` has.
fn id_of(base: &str, taken: &[String]) -> String {
    (1..)
        .map(|n| {
            if n == 1 {
                base.to_owned()
            } else {
                format!("{base}-{n}")
            }
        })
        .find(|id| !taken.contains(id))
        .unwrap_or_else(|| base.to_owned())
}

/// The link with its measure fields as its kind has them (the ones it does not use go).
fn with_kind(link: &mut EventLink, kind: EventLinkKind) {
    link.kind = kind;
    match kind {
        EventLinkKind::Point => {
            link.measure = Some(String::new());
            link.from = None;
            link.to = None;
        }
        EventLinkKind::Line => {
            link.measure = None;
            link.from = Some(String::new());
            link.to = Some(String::new());
        }
    }
}

impl App {
    /// Olay bağları: the window on the project's event links.
    pub(crate) fn open_event_links(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let links = doc.model.settings().linear_events.clone();
        self.event_links_window = Some(Window {
            chosen: (!links.is_empty()).then_some(0),
            rows: links.iter().cloned().map(Row::of).collect(),
            kept: links,
        });
        self.dialog = Some(Dialog::EventLinks);
    }

    pub(crate) fn event_links_event(&mut self, event: Event) -> Task<Message> {
        let layers = self.relation_layers();
        // The offset field is picked by its place among the chosen link's event layer's attributes.
        let offset_keys = match (&event, &self.event_links_window) {
            (Event::Offset(_), Some(w)) => w
                .chosen
                .and_then(|i| w.rows.get(i))
                .map(|r| self.event_keys_of(&r.link.events))
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let Some(w) = self.event_links_window.as_mut() else {
            return Task::none();
        };
        let taken: Vec<String> = w.rows.iter().map(|r| r.link.id.clone()).collect();
        let chosen = w.chosen.and_then(|i| w.rows.get_mut(i));
        match event {
            Event::Choose(i) => w.chosen = Some(i),
            Event::New => {
                let events = layers.first().map(|(id, _)| id.clone()).unwrap_or_default();
                let mut link = EventLink {
                    id: id_of("olay-bagi", &taken),
                    name: format!("Olay bağı {}", w.rows.len() + 1),
                    events,
                    kind: EventLinkKind::Point,
                    routes: Vec::new(),
                    route_field: String::new(),
                    event_route: String::new(),
                    measure: None,
                    from: None,
                    to: None,
                    offset: None,
                    side: EventSide::Right,
                    tolerance: None,
                };
                with_kind(&mut link, EventLinkKind::Point);
                w.rows.push(Row::of(link));
                w.chosen = Some(w.rows.len() - 1);
            }
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|i| *i < w.rows.len()) {
                    w.rows.remove(i);
                    w.chosen =
                        (!w.rows.is_empty()).then_some(i.min(w.rows.len().saturating_sub(1)));
                }
            }
            Event::Name(text) => {
                if let Some(r) = chosen {
                    r.link.name = text;
                }
            }
            Event::Events(id) => {
                if let Some(r) = chosen {
                    r.link.routes.retain(|l| *l != id);
                    r.link.events = id;
                    r.link.event_route.clear();
                    let kind = r.link.kind;
                    r.link.offset = None;
                    with_kind(&mut r.link, kind);
                }
            }
            Event::Kind(kind) => {
                if let Some(r) = chosen {
                    with_kind(&mut r.link, kind);
                }
            }
            Event::Route(id) => {
                if let Some(r) = chosen {
                    match r.link.routes.iter().position(|l| *l == id) {
                        Some(at) => {
                            r.link.routes.remove(at);
                        }
                        None if id != r.link.events => r.link.routes.push(id),
                        None => {}
                    }
                    if r.link.routes.is_empty() {
                        r.link.route_field.clear();
                    }
                }
            }
            Event::RouteField(f) => {
                if let Some(r) = chosen {
                    r.link.route_field = f;
                }
            }
            Event::NamesRoute(f) => {
                if let Some(r) = chosen {
                    r.link.event_route = f;
                }
            }
            Event::Measure(f) => {
                if let Some(r) = chosen {
                    r.link.measure = Some(f);
                }
            }
            Event::From(f) => {
                if let Some(r) = chosen {
                    r.link.from = Some(f);
                }
            }
            Event::To(f) => {
                if let Some(r) = chosen {
                    r.link.to = Some(f);
                }
            }
            Event::Offset(at) => {
                if let Some(r) = chosen {
                    r.link.offset = at
                        .checked_sub(1)
                        .and_then(|k| offset_keys.get(k))
                        .map(|(k, _)| k.clone());
                }
            }
            Event::Left => {
                if let Some(r) = chosen {
                    r.link.side = match r.link.side {
                        EventSide::Right => EventSide::Left,
                        EventSide::Left => EventSide::Right,
                    };
                }
            }
            Event::Tolerance(text) => {
                if let Some(r) = chosen {
                    r.tolerance = text;
                }
            }
            Event::Cancel => {
                self.event_links_window = None;
                self.dialog = None;
            }
            Event::Save => return self.save_event_links(),
        }
        Task::none()
    }

    /// Kaydet: what changed through `cad.linearref.define`: the removed, then the changed and the new.
    fn save_event_links(&mut self) -> Task<Message> {
        let Some(w) = self.event_links_window.clone() else {
            return Task::none();
        };
        let mut links = Vec::new();
        for r in &w.rows {
            let text = r.tolerance.trim().replace(',', ".");
            let mut link = r.link.clone();
            link.tolerance = if text.is_empty() {
                None
            } else {
                match text.parse::<f64>() {
                    Ok(t) => Some(t),
                    Err(_) => {
                        self.warn(format!(
                            "“{}” bağının toleransı bir sayı olmalı (metre).",
                            r.link.name
                        ));
                        return Task::none();
                    }
                }
            };
            links.push(link);
        }
        let mut edits = Vec::new();
        for v in &w.kept {
            if !links.iter().any(|l| l.id == v.id) {
                edits.push(LinearRefDefine {
                    operation: LinearRefDefineOperation::Remove,
                    link: None,
                    id: Some(v.id.clone()),
                    expected_revision: None,
                });
            }
        }
        for l in &links {
            if w.kept.iter().find(|k| k.id == l.id) != Some(l) {
                edits.push(LinearRefDefine {
                    operation: LinearRefDefineOperation::Set,
                    link: Some(l.clone()),
                    id: None,
                    expected_revision: None,
                });
            }
        }
        for edit in edits {
            let Some(doc) = self.document.as_mut() else {
                return Task::none();
            };
            let result =
                linearref_define::execute(&mut ExecutionContext::new(&mut doc.model), edit);
            if self.answered(result).is_none() {
                if let Some(doc) = &self.document
                    && let Some(w) = self.event_links_window.as_mut()
                {
                    w.kept = doc.model.settings().linear_events.clone();
                }
                return Task::none();
            }
        }
        self.event_links_window = None;
        self.dialog = None;
        self.say(Level::Success, "Olay bağları kaydedildi.");
        Task::none()
    }

    /// A layer's attribute names for the lists: its fields (an untyped layer's attributes).
    fn event_keys_of(&self, layer: &str) -> Vec<(String, String)> {
        self.join_keys_of(layer)
            .into_iter()
            .filter(|(k, _)| k != kentos_commands::UID_KEY)
            .collect()
    }

    pub(crate) fn event_links_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.event_links_window else {
            return iced::widget::text("").into();
        };
        let layers = self.relation_layers();
        let name_of = |id: &str| {
            layers
                .iter()
                .find(|(l, _)| l == id)
                .map_or(id.to_owned(), |(_, n)| n.clone())
        };
        let mut list = Column::new().spacing(1).padding(2);
        for (i, r) in w.rows.iter().enumerate() {
            let routes: Vec<String> = r.link.routes.iter().map(|l| name_of(l)).collect();
            let face = column![
                Elided::new(r.link.name.clone())
                    .size(typography::body())
                    .font(typography::ui())
                    .width(Fill),
                label::caption(format!(
                    "{} ← {}",
                    name_of(&r.link.events),
                    if routes.is_empty() {
                        "rota yok".to_owned()
                    } else {
                        routes.join(", ")
                    }
                ))
                .style(muted),
            ]
            .spacing(1);
            list = list.push(
                button(face)
                    .on_press(msg(Event::Choose(i)))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(w.chosen == Some(i))),
            );
        }
        if w.rows.is_empty() {
            list = list.push(
                label::caption(
                    "Projede olay bağı yok. Yeni bağ ile bir olay katmanını rotalarına bağlayın.",
                )
                .style(muted),
            );
        }
        let left = column![
            label::strong("Bağlar"),
            container(scrollable(list).height(typography::scaled(240.0)))
                .style(style::container::field_box)
                .width(Fill),
            words::secondary(NEW, Some(msg(Event::New))),
        ]
        .spacing(8);
        let right: Element<'_, Message> = match w.chosen.and_then(|i| w.rows.get(i)) {
            None => container(label::caption(
                "Bir bağ seçin ya da yeni bir bağ oluşturun.",
            ))
            .padding(12)
            .into(),
            Some(r) => scrollable(self.event_link_editor(r, &layers))
                .height(typography::scaled(330.0))
                .into(),
        };
        let body = column![
            row![container(left).width(240), container(right).width(Fill)].spacing(16),
            row![
                words::ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove))),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, Some(msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center),
        ]
        .spacing(14);
        Frame::new(TITLE)
            .push(body)
            .width(720.0)
            .modal(msg(Event::Cancel))
    }

    fn event_link_editor<'a>(
        &'a self,
        r: &'a Row,
        layers: &[(String, String)],
    ) -> Element<'a, Message> {
        let l = &r.link;
        let ids: Vec<String> = layers.iter().map(|(id, _)| id.clone()).collect();
        let events = {
            let ids = ids.clone();
            Select::new(
                layers.iter().map(|(_, n)| Choice::new(n.clone())),
                ids.iter().position(|id| *id == l.events),
                move |i| msg(Event::Events(ids[i].clone())),
            )
            .placeholder("Katman")
        };
        let kinds = [EventLinkKind::Point, EventLinkKind::Line];
        let kind = Select::new(
            [Choice::new("Nokta"), Choice::new("Çizgi")],
            kinds.iter().position(|k| *k == l.kind),
            move |i| msg(Event::Kind(kinds[i])),
        );
        let mut routes = Column::new().spacing(4);
        for (id, name) in layers.iter().filter(|(id, _)| *id != l.events) {
            routes = routes.push(words::check(
                l.routes.contains(id),
                name.clone(),
                Some(msg(Event::Route(id.clone()))),
            ));
        }
        let key = |keys: Vec<(String, String)>, at: &str, on: fn(String) -> Event| {
            let names: Vec<String> = keys.iter().map(|(k, _)| k.clone()).collect();
            Select::new(
                keys.iter().map(|(_, n)| Choice::new(n.clone())),
                names.iter().position(|k| k == at),
                move |i| msg(on(names[i].clone())),
            )
            .placeholder("Alan")
        };
        let route_keys = l
            .routes
            .first()
            .map(|id| self.event_keys_of(id))
            .unwrap_or_default();
        let event_keys = self.event_keys_of(&l.events);
        let offset_keys = event_keys.clone();
        let offset = Select::new(
            std::iter::once(Choice::new(NONE))
                .chain(offset_keys.iter().map(|(_, n)| Choice::new(n.clone()))),
            Some(
                l.offset
                    .as_ref()
                    .and_then(|o| offset_keys.iter().position(|(k, _)| k == o))
                    .map_or(0, |p| p + 1),
            ),
            |i| msg(Event::Offset(i)),
        );
        let name =
            kentos_ui::widget::text_field("Bağın adı", &l.name).on_input(|t| msg(Event::Name(t)));
        let tolerance = kentos_ui::widget::text_field("0,001", &r.tolerance)
            .on_input(|t| msg(Event::Tolerance(t)));
        let mut editor = Column::new()
            .spacing(10)
            .push(words::field("Ad", name, None))
            .push(words::field("Olay katmanı", events, None))
            .push(words::field("Olay türü", kind, None))
            .push(words::field("Rota katmanları", routes, None))
            .push(words::field(
                "Rotaların kimlik alanı",
                key(route_keys, &l.route_field, Event::RouteField),
                None,
            ))
            .push(words::field(
                "Olayın rota alanı",
                key(event_keys.clone(), &l.event_route, Event::NamesRoute),
                None,
            ));
        editor = match l.kind {
            EventLinkKind::Point => editor.push(words::field(
                "Ölçü alanı",
                key(
                    event_keys,
                    l.measure.as_deref().unwrap_or(""),
                    Event::Measure,
                ),
                None,
            )),
            EventLinkKind::Line => editor
                .push(words::field(
                    "Başlangıç ölçüsü alanı",
                    key(
                        event_keys.clone(),
                        l.from.as_deref().unwrap_or(""),
                        Event::From,
                    ),
                    None,
                ))
                .push(words::field(
                    "Bitiş ölçüsü alanı",
                    key(event_keys, l.to.as_deref().unwrap_or(""), Event::To),
                    None,
                )),
        };
        editor
            .push(words::field("Öteleme alanı", offset, None))
            .push(words::check(
                l.side == EventSide::Left,
                LEFT,
                Some(msg(Event::Left)),
            ))
            .push(words::field("Tolerans (m)", tolerance, None))
            .push(
                label::caption(
                    "Rotası ya da olayı değişen her yazma, aynı adımda, bağın olaylarını rotalarının verdiği yere koyar \
                     (Olay kaymasını denetle'nin Yerine taşı'sı gibi); koyamadıkları olduğu yerde kalır ve söylenir.",
                )
                .style(muted),
            )
            .into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn event_links_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.event_links_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(LEFT) => Some(msg(Event::Left)),
            Control::Press(words) => match w.rows.iter().position(|r| r.link.name == words) {
                Some(i) => Some(msg(Event::Choose(i))),
                None => return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok")),
            },
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Fill("Tolerans (m)", text) => Some(msg(Event::Tolerance(text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}

#[cfg(test)]
mod tests {
    use super::{Event as W, id_of};
    use crate::app::{App, Dialog, Message};

    #[test]
    fn a_new_id_is_not_taken() {
        assert_eq!(id_of("olay-bagi", &[]), "olay-bagi");
        assert_eq!(
            id_of("olay-bagi", &["olay-bagi".into(), "olay-bagi-2".into()]),
            "olay-bagi-3"
        );
    }

    /// The shared cases' drawing (`fixtures/commands/v1/cad.linearref.follow.json`'s setup) without its link.
    fn app() -> App {
        let (mut app, _) = App::boot(None);
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/commands/v1/cad.linearref.follow.json"
        ))
        .expect("the cases read");
        let mut setup = cases["setup"].clone();
        setup["settings"]["linearEvents"] = serde_json::json!([]);
        setup["settings"]
            .as_object_mut()
            .expect("settings")
            .remove("linearEvents");
        let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string())
            .expect("the drawing reads");
        let doc = crate::document::Document::new(snapshot, None).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        app
    }

    fn send(app: &mut App, e: W) {
        let _ = app.update(Message::EventLinks(e));
    }

    #[test]
    fn the_window_saves_a_link_and_the_next_write_follows_it() {
        let mut app = app();
        let _ = app.update(Message::Run("linearRef.eventLinks"));
        assert_eq!(app.dialog, Some(Dialog::EventLinks));
        send(&mut app, W::New);
        send(&mut app, W::Name("Kazalar".into()));
        send(&mut app, W::Events("kaza".into()));
        send(&mut app, W::Route("yol".into()));
        send(&mut app, W::RouteField("Yol".into()));
        send(&mut app, W::NamesRoute("Yol".into()));
        send(&mut app, W::Measure("Km".into()));
        send(&mut app, W::Save);
        assert!(app.event_links_window.is_none(), "Kaydet closes the window");
        let links = app
            .document
            .as_ref()
            .expect("open")
            .model
            .settings()
            .linear_events
            .clone();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].events, "kaza");
        assert_eq!(links[0].routes, vec!["yol".to_owned()]);
        assert_eq!(links[0].measure.as_deref(), Some("Km"));
        // A write on the route's layer places the events again: the point of Km 40 follows the route moved north.
        let doc = app.document.as_mut().expect("open");
        let y_of = |doc: &crate::document::Document| {
            doc.model
                .by_layer("kaza")
                .find_map(|e| match e {
                    kentos_commands::Entity::Point(p) if p.base.id == 5 => Some(p.p.y),
                    _ => None,
                })
                .expect("the event")
        };
        let before = y_of(doc);
        let uid = doc
            .model
            .by_layer_with_uids("yol")
            .next()
            .map(|(u, _)| u)
            .expect("the route");
        let result = kentos_native_application::transform::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::EntitiesTransform {
                uids: vec![uid.to_string()],
                transform: kentos_commands::Transform::Move { dx: 0.0, dy: 10.0 },
                copy: None,
                layer_id: None,
                expected_revision: None,
            },
        );
        assert!(matches!(
            result,
            kentos_commands::CommandResult::Completed { .. }
        ));
        let after = y_of(app.document.as_ref().expect("open"));
        assert!((after - before - 10.0).abs() < 1e-6, "{before} {after}");
        let _ = app.update(Message::Run("edit.undo"));
        let undone = y_of(app.document.as_ref().expect("open"));
        assert!((undone - before).abs() < 1e-6, "one undo step");
    }

    /// The window dark and light (`.run/shots/olay-baglari-*`):
    ///
    /// ```text
    /// KENTOS_SHOTS_ONLY=pencere cargo test -p kentos-desktop event_links_window::tests::screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use iced::Size;
        use kentos_ui::snapshot::Snapshot;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
            for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
                let mut app = app();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                let _ = app.update(Message::Run("linearRef.eventLinks"));
                send(&mut app, W::New);
                send(&mut app, W::Name("Kazalar".into()));
                send(&mut app, W::Events("kaza".into()));
                send(&mut app, W::Route("yol".into()));
                send(&mut app, W::Route("yol-eski".into()));
                send(&mut app, W::RouteField("Yol".into()));
                send(&mut app, W::NamesRoute("Yol".into()));
                send(&mut app, W::Measure("Km".into()));
                send(&mut app, W::Offset(1));
                send(&mut app, W::New);
                send(&mut app, W::Name("Kaplama kesimleri".into()));
                send(&mut app, W::Events("kesim".into()));
                send(&mut app, W::Kind(kentos_commands::EventLinkKind::Line));
                send(&mut app, W::Route("yol".into()));
                send(&mut app, W::Choose(0));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("olay-baglari-pencere-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

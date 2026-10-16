//! İlişkiler (docs/adr/0405 §3, `data.relations`): the project's relations.
//!
//! ```text
//! ┌ İlişkiler ───────────────────────────────────────────────────────────────┐
//! │ İlişkiler            │ Ad          [Binanın sayaçları        ]            │
//! │ ● Binanın sayaçları  │ Üst katman  [Bina                    ▾]            │
//! │   Sayacın vanaları   │ Alt katman  [Sayaç                   ▾]            │
//! │ [Yeni ilişki]        │ Alan        [bina                    ▾]            │
//! │                      │ Üst nesne silinince [Bağı kopar      ▾]            │
//! │ [İlişkiyi sil]                                  [Vazgeç] [Kaydet]          │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the relations; Kaydet writes what changed through
//! `cad.relations.edit` (removes, updates, creates, in that order) and closes the window.

use iced::widget::{Column, button, column, container, row, scrollable, space, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{
    LayerNodeType, ProjectRelation, RelationDelete, RelationsEdit, RelationsOp,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, relations_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "İlişkiler";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni ilişki";
const REMOVE: &str = "İlişkiyi sil";

/// The policies in the order the window offers them, with their words.
const POLICIES: [(RelationDelete, &str, &str); 4] = [
    (
        RelationDelete::Unlink,
        "Bağı kopar",
        "Alt kayıtların alanı boşalır.",
    ),
    (
        RelationDelete::Cascade,
        "Alt kayıtları da sil",
        "Bileşik ilişki: alt kayıtlar üst nesneyle silinir.",
    ),
    (
        RelationDelete::Restrict,
        "Silinmesin",
        "Alt kaydı olan üst nesne silinemez.",
    ),
    (
        RelationDelete::Keep,
        "Bağ kalsın",
        "Alt kayıtlar silinmiş nesneye başvurur; form bunu söyler.",
    ),
];

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    /// The copy being edited and the relations as the project has them.
    rows: Vec<ProjectRelation>,
    kept: Vec<ProjectRelation>,
    chosen: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(usize),
    New,
    Remove,
    Name(String),
    Parent(String),
    Child(String),
    Field(String),
    Policy(RelationDelete),
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Relations(event)
}

impl App {
    /// İlişkiler: the window on the project's relations.
    pub(crate) fn open_relations(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let rows = doc.model.settings().relations.clone();
        self.relations_window = Some(Window {
            chosen: (!rows.is_empty()).then_some(0),
            kept: rows.clone(),
            rows,
        });
        self.dialog = Some(Dialog::Relations);
    }

    pub(crate) fn relations_event(&mut self, event: Event) -> Task<Message> {
        let layers = self.relation_layers();
        let Some(w) = self.relations_window.as_mut() else {
            return Task::none();
        };
        let chosen = w.chosen.and_then(|i| w.rows.get_mut(i));
        match event {
            Event::Choose(i) => w.chosen = Some(i),
            Event::New => {
                let taken: Vec<String> = w.rows.iter().map(|r| r.id.clone()).collect();
                let parent = layers.first().map(|(id, _)| id.clone()).unwrap_or_default();
                w.rows.push(ProjectRelation {
                    id: kentos_forms::item_id_of("iliski", &taken),
                    name: format!("İlişki {}", w.rows.len() + 1),
                    child: parent.clone(),
                    parent,
                    field: String::new(),
                    on_delete: RelationDelete::Unlink,
                });
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
                    r.name = text;
                }
            }
            Event::Parent(id) => {
                if let Some(r) = chosen {
                    r.parent = id;
                }
            }
            Event::Child(id) => {
                if let Some(r) = chosen {
                    r.child = id;
                    r.field.clear();
                }
            }
            Event::Field(key) => {
                if let Some(r) = chosen {
                    r.field = key;
                }
            }
            Event::Policy(p) => {
                if let Some(r) = chosen {
                    r.on_delete = p;
                }
            }
            Event::Cancel => {
                self.relations_window = None;
                self.dialog = None;
            }
            Event::Save => return self.save_relations(),
        }
        Task::none()
    }

    /// Kaydet: what changed through `cad.relations.edit`: the removed, the changed, the new.
    fn save_relations(&mut self) -> Task<Message> {
        let Some(w) = self.relations_window.clone() else {
            return Task::none();
        };
        if let Some(locked) = self.forms_locked() {
            self.warn(locked);
            return Task::none();
        }
        let mut edits = Vec::new();
        for r in &w.kept {
            if !w.rows.iter().any(|x| x.id == r.id) {
                edits.push(RelationsOp::Remove { id: r.id.clone() });
            }
        }
        for r in &w.rows {
            match w.kept.iter().find(|x| x.id == r.id) {
                Some(was) if was == r => {}
                Some(_) => edits.push(RelationsOp::Update {
                    relation: r.clone(),
                }),
                None => edits.push(RelationsOp::Create {
                    relation: r.clone(),
                }),
            }
        }
        for edit in edits {
            let Some(doc) = self.document.as_mut() else {
                return Task::none();
            };
            let result = relations_edit::execute(
                &mut ExecutionContext::new(&mut doc.model),
                RelationsEdit {
                    edit,
                    expected_revision: None,
                },
            );
            if self.answered(result).is_none() {
                // The window stays on what did not go in; the rest is written.
                if let Some(doc) = &self.document
                    && let Some(w) = self.relations_window.as_mut()
                {
                    w.kept = doc.model.settings().relations.clone();
                }
                return Task::none();
            }
        }
        self.relations_window = None;
        self.dialog = None;
        self.say(Level::Success, "İlişkiler kaydedildi.");
        Task::none()
    }

    /// The layers a relation can join, by their path.
    fn relation_layers(&self) -> Vec<(String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut out = Vec::new();
        fn walk(nodes: &[kentos_contracts::LayerNode], at: &str, out: &mut Vec<(String, String)>) {
            for n in nodes {
                let path = if at.is_empty() {
                    n.name.clone()
                } else {
                    format!("{at} / {}", n.name)
                };
                if n.kind == LayerNodeType::Layer && n.service.is_none() {
                    out.push((n.id.clone(), path.clone()));
                }
                walk(&n.children, &path, out);
            }
        }
        walk(doc.model.layers().nodes(), "", &mut out);
        out
    }

    pub(crate) fn relations_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.relations_window else {
            return iced::widget::text("").into();
        };
        let layers = self.relation_layers();
        let mut list = Column::new().spacing(1).padding(2);
        for (i, r) in w.rows.iter().enumerate() {
            let name_of = |id: &str| {
                layers
                    .iter()
                    .find(|(l, _)| l == id)
                    .map_or(id.to_owned(), |(_, n)| n.clone())
            };
            let face = column![
                Elided::new(r.name.clone())
                    .size(typography::body())
                    .font(typography::ui())
                    .width(Fill),
                label::caption(format!("{} → {}", name_of(&r.parent), name_of(&r.child)))
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
                label::caption("Projede ilişki yok. Yeni ilişki ile bir katmanı ötekine bağlayın.")
                    .style(muted),
            );
        }
        let left = column![
            label::strong("İlişkiler"),
            container(scrollable(list).height(typography::scaled(240.0)))
                .style(style::container::field_box)
                .width(Fill),
            words::secondary(NEW, Some(msg(Event::New))),
        ]
        .spacing(8);
        let right: Element<'_, Message> = match w.chosen.and_then(|i| w.rows.get(i)) {
            None => container(label::caption(
                "Bir ilişki seçin ya da yeni bir ilişki oluşturun.",
            ))
            .padding(12)
            .into(),
            Some(r) => self.relation_editor(r, &layers),
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
        overlay::modal(
            Frame::new(TITLE).push(body).width(720.0),
            msg(Event::Cancel),
        )
    }

    fn relation_editor<'a>(
        &'a self,
        r: &ProjectRelation,
        layers: &[(String, String)],
    ) -> Element<'a, Message> {
        let pick = |at: &str, on: fn(String) -> Event| {
            let ids: Vec<String> = layers.iter().map(|(id, _)| id.clone()).collect();
            Select::new(
                layers.iter().map(|(_, n)| Choice::new(n.clone())),
                ids.iter().position(|id| id == at),
                move |i| msg(on(ids[i].clone())),
            )
            .placeholder("Katman")
        };
        let fields = self.layer_fields_of(&r.child);
        let keys: Vec<String> = fields.iter().map(|f| f.key().to_owned()).collect();
        let field = Select::new(
            fields.iter().map(|f| Choice::new(f.label().to_owned())),
            keys.iter().position(|k| *k == r.field),
            move |i| msg(Event::Field(keys[i].clone())),
        )
        .placeholder("Üst nesnenin kimliğini tutan alan");
        let at = POLICIES.iter().position(|(p, _, _)| *p == r.on_delete);
        let policy = Select::new(POLICIES.iter().map(|(_, w, _)| Choice::new(*w)), at, |i| {
            msg(Event::Policy(POLICIES[i].0))
        });
        let explain = at.map_or("", |i| POLICIES[i].2);
        let name = text_input("İlişkinin adı", &r.name)
            .size(typography::body())
            .on_input(|t| msg(Event::Name(t)))
            .padding([5, 8])
            .style(style::field::input);
        let mut col = Column::new()
            .spacing(10)
            .push(words::field("Ad", focus_ring(name), None))
            .push(words::field(
                "Üst katman",
                pick(&r.parent, Event::Parent),
                None,
            ))
            .push(words::field(
                "Alt katman",
                pick(&r.child, Event::Child),
                None,
            ))
            .push(words::field("Alan", field, None));
        if fields.is_empty() {
            col = col.push(words::text_line(
                Kind::Warn,
                "Alt katmanın alanı yok: önce Alanlar penceresinden üst nesnenin kimliğini tutacak bir alan ekleyin.",
            ));
        }
        col.push(words::field("Üst nesne silinince", policy, None))
            .push(label::caption(explain).style(muted))
            .into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn relations_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.relations_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(words) => match w.rows.iter().position(|r| r.name == words) {
                Some(i) => Some(msg(Event::Choose(i))),
                None => return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok")),
            },
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}

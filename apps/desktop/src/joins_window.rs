//! Birleşik görünümler (docs/adr/0527, `data.joinViews`): the project's join views.
//!
//! ```text
//! ┌ Birleşik görünümler ─────────────────────────────────────────────────────────┐
//! │ Görünümler             │ Ad              [Parseller ve malikleri         ]     │
//! │ ● Parseller ve malikl. │ Katman          [Parsel                        ▾]     │
//! │   Malikli parseller    │ Anahtar         [Parsel no                     ▾]     │
//! │ [Yeni görünüm]         │ Birleşen katman [Malik                         ▾]     │
//! │ [İlişkiden ▾]          │ Anahtar         [parsel                        ▾]     │
//! │                        │ [ ] Yalnız eşleşen kayıtlar                          │
//! │ [Görünümü sil]                                       [Vazgeç] [Kaydet]       │
//! └──────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the views; Kaydet writes what changed through `cad.joins.edit` (removes,
//! updates, creates, in that order) and closes the window. İlişkiden fills a new view from a relation (its
//! parent layer by the object's id, its child layer by the relation's field). The rows are read in the
//! Öznitelik tablosu, which lists the views after the layers.

use iced::widget::{Column, button, column, container, row, scrollable, space, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{JoinKeep, JoinView, JoinsEdit, JoinsOp, UID_KEY};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, joins_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Birleşik görünümler";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni görünüm";
const REMOVE: &str = "Görünümü sil";
const MATCHED: &str = "Yalnız eşleşen kayıtlar";
/// The key that is the object's own id, as the lists show it.
const UID_WORDS: &str = "Nesnenin kimliği";

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    /// The copy being edited and the views as the project has them.
    rows: Vec<JoinView>,
    kept: Vec<JoinView>,
    chosen: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(usize),
    New,
    /// İlişkiden: a new view of the relation at this place.
    FromRelation(usize),
    Remove,
    Name(String),
    Layer(String),
    Key(String),
    With(String),
    WithKey(String),
    Matched,
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Joins(event)
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

impl App {
    /// Birleşik görünümler: the window on the project's join views.
    pub(crate) fn open_joins(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let rows = doc.model.settings().join_views.clone();
        self.joins_window = Some(Window {
            chosen: (!rows.is_empty()).then_some(0),
            kept: rows.clone(),
            rows,
        });
        self.dialog = Some(Dialog::Joins);
    }

    pub(crate) fn joins_event(&mut self, event: Event) -> Task<Message> {
        let layers = self.relation_layers();
        let relations = self
            .document
            .as_ref()
            .map(|d| d.model.settings().relations.clone())
            .unwrap_or_default();
        let Some(w) = self.joins_window.as_mut() else {
            return Task::none();
        };
        let taken: Vec<String> = w.rows.iter().map(|v| v.id.clone()).collect();
        let chosen = w.chosen.and_then(|i| w.rows.get_mut(i));
        match event {
            Event::Choose(i) => w.chosen = Some(i),
            Event::New => {
                let first = layers.first().map(|(id, _)| id.clone()).unwrap_or_default();
                w.rows.push(JoinView {
                    id: id_of("gorunum", &taken),
                    name: format!("Görünüm {}", w.rows.len() + 1),
                    layer: first.clone(),
                    key: UID_KEY.into(),
                    with: first,
                    with_key: UID_KEY.into(),
                    keep: JoinKeep::All,
                    fields: Vec::new(),
                });
                w.chosen = Some(w.rows.len() - 1);
            }
            Event::FromRelation(i) => {
                if let Some(r) = relations.get(i) {
                    w.rows.push(JoinView {
                        id: id_of(&r.id, &taken),
                        name: r.name.clone(),
                        layer: r.parent.clone(),
                        key: UID_KEY.into(),
                        with: r.child.clone(),
                        with_key: r.field.clone(),
                        keep: JoinKeep::All,
                        fields: Vec::new(),
                    });
                    w.chosen = Some(w.rows.len() - 1);
                }
            }
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|i| *i < w.rows.len()) {
                    w.rows.remove(i);
                    w.chosen =
                        (!w.rows.is_empty()).then_some(i.min(w.rows.len().saturating_sub(1)));
                }
            }
            Event::Name(text) => {
                if let Some(v) = chosen {
                    v.name = text;
                }
            }
            Event::Layer(id) => {
                if let Some(v) = chosen {
                    v.layer = id;
                    v.key = UID_KEY.into();
                }
            }
            Event::Key(k) => {
                if let Some(v) = chosen {
                    v.key = k;
                }
            }
            Event::With(id) => {
                if let Some(v) = chosen {
                    v.with = id;
                    v.with_key = UID_KEY.into();
                    v.fields.clear();
                }
            }
            Event::WithKey(k) => {
                if let Some(v) = chosen {
                    v.with_key = k;
                }
            }
            Event::Matched => {
                if let Some(v) = chosen {
                    v.keep = match v.keep {
                        JoinKeep::All => JoinKeep::Matched,
                        JoinKeep::Matched => JoinKeep::All,
                    };
                }
            }
            Event::Cancel => {
                self.joins_window = None;
                self.dialog = None;
            }
            Event::Save => return self.save_joins(),
        }
        Task::none()
    }

    /// Kaydet: what changed through `cad.joins.edit`: the removed, the changed, the new.
    fn save_joins(&mut self) -> Task<Message> {
        let Some(w) = self.joins_window.clone() else {
            return Task::none();
        };
        let mut edits = Vec::new();
        for v in &w.kept {
            if !w.rows.iter().any(|x| x.id == v.id) {
                edits.push(JoinsOp::Remove { id: v.id.clone() });
            }
        }
        for v in &w.rows {
            match w.kept.iter().find(|x| x.id == v.id) {
                Some(was) if was == v => {}
                Some(_) => edits.push(JoinsOp::Update { view: v.clone() }),
                None => edits.push(JoinsOp::Create { view: v.clone() }),
            }
        }
        for edit in edits {
            let Some(doc) = self.document.as_mut() else {
                return Task::none();
            };
            let result = joins_edit::execute(
                &mut ExecutionContext::new(&mut doc.model),
                JoinsEdit {
                    edit,
                    expected_revision: None,
                },
            );
            if self.answered(result).is_none() {
                if let Some(doc) = &self.document
                    && let Some(w) = self.joins_window.as_mut()
                {
                    w.kept = doc.model.settings().join_views.clone();
                }
                return Task::none();
            }
        }
        self.joins_window = None;
        self.dialog = None;
        self.features.changed();
        self.say(Level::Success, "Birleşik görünümler kaydedildi.");
        Task::none()
    }

    /// A layer's keys for the lists: the object's id, then its fields (an untyped layer's attributes).
    fn join_keys_of(&self, layer: &str) -> Vec<(String, String)> {
        let mut out = vec![(UID_KEY.to_owned(), UID_WORDS.to_owned())];
        let Some(doc) = &self.document else {
            return out;
        };
        let fields = self.layer_fields_of(layer);
        if fields.is_empty() {
            let mut names: Vec<String> = doc
                .model
                .by_layer(layer)
                .flat_map(|e| e.base().attrs.keys().cloned())
                .collect();
            names.sort();
            names.dedup();
            out.extend(names.into_iter().map(|n| (n.clone(), n)));
        } else {
            out.extend(
                fields
                    .iter()
                    .map(|f| (f.key().to_owned(), f.label().to_owned())),
            );
        }
        out
    }

    pub(crate) fn joins_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.joins_window else {
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
        for (i, v) in w.rows.iter().enumerate() {
            let face = column![
                Elided::new(v.name.clone())
                    .size(typography::body())
                    .font(typography::ui())
                    .width(Fill),
                label::caption(format!("{} ⋈ {}", name_of(&v.layer), name_of(&v.with)))
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
                label::caption("Projede birleşik görünüm yok. Yeni görünüm ile bir katmanın kayıtlarını ötekininkilerle eşleyin.")
                    .style(muted),
            );
        }
        let relations = self
            .document
            .as_ref()
            .map(|d| d.model.settings().relations.clone())
            .unwrap_or_default();
        let mut left = column![
            label::strong("Görünümler"),
            container(scrollable(list).height(typography::scaled(240.0)))
                .style(style::container::field_box)
                .width(Fill),
            words::secondary(NEW, Some(msg(Event::New))),
        ]
        .spacing(8);
        if !relations.is_empty() {
            left = left.push(
                Select::new(
                    relations.iter().map(|r| Choice::new(r.name.clone())),
                    None,
                    |i| msg(Event::FromRelation(i)),
                )
                .placeholder("İlişkiden kur"),
            );
        }
        let right: Element<'_, Message> = match w.chosen.and_then(|i| w.rows.get(i)) {
            None => container(label::caption(
                "Bir görünüm seçin ya da yeni bir görünüm oluşturun.",
            ))
            .padding(12)
            .into(),
            Some(v) => self.join_editor(v, &layers),
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

    fn join_editor<'a>(
        &'a self,
        v: &JoinView,
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
        let key = |layer: &str, at: &str, on: fn(String) -> Event| {
            let keys = self.join_keys_of(layer);
            let ids: Vec<String> = keys.iter().map(|(k, _)| k.clone()).collect();
            Select::new(
                keys.iter().map(|(_, n)| Choice::new(n.clone())),
                ids.iter().position(|k| k == at),
                move |i| msg(on(ids[i].clone())),
            )
            .placeholder("Anahtar alanı")
        };
        let name = text_input("Görünümün adı", &v.name)
            .size(typography::body())
            .on_input(|t| msg(Event::Name(t)))
            .padding([5, 8])
            .style(style::field::input);
        Column::new()
            .spacing(10)
            .push(words::field("Ad", focus_ring(name), None))
            .push(words::field("Katman", pick(&v.layer, Event::Layer), None))
            .push(words::field("Anahtar", key(&v.layer, &v.key, Event::Key), None))
            .push(words::field("Birleşen katman", pick(&v.with, Event::With), None))
            .push(words::field("Anahtar", key(&v.with, &v.with_key, Event::WithKey), None))
            .push(words::check(v.keep == JoinKeep::Matched, MATCHED, Some(msg(Event::Matched))))
            .push(
                label::caption(
                    "Anahtarlar Anahtarla birleştir'in kuralıyla eşleşir (007 ile 7 aynı). Bir kayıt birden çok kayıtla \
                     eşleşirse birden çok satırdır; satırlar her okuyuşta çizimden kurulur, kaydedilmez.",
                )
                .style(muted),
            )
            .into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn joins_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.joins_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(MATCHED) => Some(msg(Event::Matched)),
            Control::Press(words) => match w.rows.iter().position(|v| v.name == words) {
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

#[cfg(test)]
mod tests {
    use super::id_of;

    #[test]
    fn a_new_id_is_not_taken() {
        assert_eq!(id_of("gorunum", &[]), "gorunum");
        assert_eq!(
            id_of("gorunum", &["gorunum".into(), "gorunum-2".into()]),
            "gorunum-3"
        );
    }
}

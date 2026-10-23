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
use kentos_commands::{
    CommandResult, LayerNodeType, ProjectRelation, RelateOp, RelationDelete, RelationState,
    RelationsEdit, RelationsOp, SourcesRelate, SourcesRelated,
};
use kentos_interaction::Level;
use kentos_native_application::sources::relate;
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
/// The sources' side of a many-to-many relation (docs/adr/0532 §2).
const MAP: &str = "Dosyaya eşle";
const UNMAP: &str = "Eşlemeyi kaldır";
/// The same for a database's junction table (docs/adr/0590).
const MAP_DB: &str = "Bağlantı tablosu kur";
const UNMAP_DB: &str = "Bağlantı tablosunu kaldır";

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
    /// How each saved relation is kept in its bound file (`cad.sources.relate`'s `check`), by the relation's id.
    filed: std::collections::BTreeMap<String, SourcesRelated>,
}

impl Window {
    /// Whether the relation is kept in its file as `state` says (a test's question).
    #[cfg(test)]
    pub(crate) fn has_filed(&self, id: &str, state: RelationState) -> bool {
        self.filed.get(id).is_some_and(|f| f.state == state)
    }
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
    /// Dosyaya eşle and Eşlemeyi kaldır on the chosen relation (docs/adr/0532 §2): written to the file at once.
    Map,
    Unmap,
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
            filed: std::collections::BTreeMap::new(),
        });
        self.refile_relations();
        self.dialog = Some(Dialog::Relations);
    }

    /// Reads how each saved relation is kept in its file (`cad.sources.relate`'s `check`).
    fn refile_relations(&mut self) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let ids: Vec<String> = doc
            .model
            .settings()
            .relations
            .iter()
            .map(|r| r.id.clone())
            .collect();
        let mut filed = std::collections::BTreeMap::new();
        for id in ids {
            let answer = relate::execute(
                &mut ExecutionContext::new(&mut doc.model),
                SourcesRelate {
                    relation: id.clone(),
                    action: RelateOp::Check,
                    mapping: None,
                },
            );
            if let CommandResult::Completed { output, .. } = answer {
                filed.insert(id, output);
            }
        }
        if let Some(w) = self.relations_window.as_mut() {
            w.filed = filed;
        }
    }

    /// Dosyaya eşle or Eşlemeyi kaldır on the chosen, saved relation.
    fn relation_filing(&mut self, op: RelateOp) {
        let Some(w) = self.relations_window.as_ref() else {
            return;
        };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)).cloned() else {
            return;
        };
        if w.kept.iter().find(|k| k.id == r.id) != Some(&r) {
            self.warn("İlişkide kaydedilmemiş değişiklik var; önce Kaydet.");
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let answer = relate::execute(
            &mut ExecutionContext::new(&mut doc.model),
            SourcesRelate {
                relation: r.id.clone(),
                action: op,
                mapping: None,
            },
        );
        match answer {
            CommandResult::Completed { output, .. } => {
                let said = match op {
                    RelateOp::Map => format!(
                        "“{}” ilişkisi dosyada GeoPackage ilişkisi olarak eşlendi: “{}” eşleme tablosu, {} satır.",
                        r.name,
                        output.mapping.clone().unwrap_or_default(),
                        output.rows
                    ),
                    _ => format!(
                        "“{}” ilişkisinin eşlemesi dosyadan kaldırıldı; başvurular katmanda kaldı.",
                        r.name
                    ),
                };
                self.announce(Level::Success, said);
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.announce(Level::Error, error.message),
            _ => {}
        }
        self.refile_relations();
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
            Event::Map => self.relation_filing(RelateOp::Map),
            Event::Unmap => self.relation_filing(RelateOp::Unmap),
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
    pub(crate) fn relation_layers(&self) -> Vec<(String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut out = Vec::new();
        fn walk(nodes: &[kentos_commands::LayerNode], at: &str, out: &mut Vec<(String, String)>) {
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
        col = col
            .push(words::field("Üst nesne silinince", policy, None))
            .push(label::caption(explain).style(muted));
        if let Some(filed) = self
            .relations_window
            .as_ref()
            .and_then(|w| w.filed.get(&r.id))
        {
            // A database keeps it in a junction table (docs/adr/0590), a file in its relation profile.
            let database =
                self.document.as_ref().is_some_and(|d| {
                    d.model.settings().sources.iter().any(|s| {
                        s.layer == r.child && s.kind == kentos_commands::SourceKind::Postgis
                    })
                });
            col = col.push(Self::relation_filed(filed, database));
        }
        col.into()
    }

    /// How the relation is kept in its bound file, and what can be done about it (docs/adr/0532 §2).
    fn relation_filed(f: &SourcesRelated, database: bool) -> Element<'_, Message> {
        let table = f.child.clone().unwrap_or_default();
        let (holder, mapped) = if database {
            ("Veritabanı", "Bağlantı tablosu olarak tutuluyor")
        } else {
            ("Dosya", "GeoPackage ilişkisi olarak tutuluyor")
        };
        let (kind, said, action): (Kind, String, Option<(&str, Event)>) = match f.state {
            RelationState::Text => (
                Kind::Info,
                format!(
                    "Çok değerli başvuru {table} tablosunda “; ” ile birleşmiş metin sütunu olarak tutuluyor."
                ),
                Some((if database { MAP_DB } else { MAP }, Event::Map)),
            ),
            RelationState::Mapped => (
                Kind::Ok,
                format!(
                    "{mapped}: “{}” {}, {} satır.",
                    f.mapping.clone().unwrap_or_default(),
                    if database {
                        "tablosu"
                    } else {
                        "eşleme tablosu"
                    },
                    f.rows
                ),
                Some((if database { UNMAP_DB } else { UNMAP }, Event::Unmap)),
            ),
            RelationState::Foreign | RelationState::Broken => (
                Kind::Warn,
                format!(
                    "{holder} iki tabloyu başka bir temsille ilişkilendiriyor: {}. KentOS onun üzerine yazmaz ve metne çevirmez.",
                    f.why.clone().unwrap_or_default()
                ),
                None,
            ),
            RelationState::Split => (
                Kind::Info,
                "Katmanlar ayrı kaynaklarda; eşleme iki tabloyu tek işlemde ister.".to_owned(),
                None,
            ),
            RelationState::Unbound => (
                Kind::Info,
                "Katmanlardan biri bir GeoPackage'a bağlı değil; dosyada tutulmuyor.".to_owned(),
                None,
            ),
            RelationState::Single => return space::vertical().height(0).into(),
        };
        let mut col = Column::new()
            .spacing(6)
            .push(label::strong(if database {
                "Veritabanında"
            } else {
                "Dosyada"
            }))
            .push(words::text_line(kind, said));
        if let Some((name, event)) = action {
            col = col.push(words::secondary(name, Some(msg(event))));
        }
        col.into()
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
            Control::Press(MAP | MAP_DB) => Some(msg(Event::Map)),
            Control::Press(UNMAP | UNMAP_DB) => Some(msg(Event::Unmap)),
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

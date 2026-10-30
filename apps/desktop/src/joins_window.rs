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
//!
//! Birleşen (docs/adr/0632): a layer, or a source's table (a GeoPackage file or a PostgreSQL address; its tables
//! read off the UI thread through `cad.sources.describe`, the key one of the table's columns). Toplama sütunları:
//! each a heading, what it does (Sayı, Toplam, En küçük, En büyük, Ortalama) and the joined records' field.

use iced::widget::{Column, button, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Length, Task};
use kentos_commands::{
    JoinAggregate, JoinHow, JoinKeep, JoinSource, JoinView, JoinsEdit, JoinsOp,
    MOST_JOIN_AGGREGATES, SourceTable, UID_KEY,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, joins_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided};
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
const ADD_SUM: &str = "Toplama sütunu ekle";
const PICK_FILE: &str = "Dosya seç…";
const READ_TABLES: &str = "Tabloları oku";
/// What a summing column does, as the list says it.
const HOWS: [(JoinHow, &str); 5] = [
    (JoinHow::Count, "Sayı"),
    (JoinHow::Sum, "Toplam"),
    (JoinHow::Min, "En küçük"),
    (JoinHow::Max, "En büyük"),
    (JoinHow::Mean, "Ortalama"),
];

/// What the joined records are: a layer's or a source's table's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Joined {
    Layer,
    Source,
}

impl std::fmt::Display for Joined {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Joined::Layer => "Katman",
            Joined::Source => "Kaynak tablosu",
        })
    }
}

/// A source's tables as read: the path they were read from, and the tables or why not.
type Tables = (String, Result<Vec<SourceTable>, String>);

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    /// The copy being edited and the views as the project has them.
    rows: Vec<JoinView>,
    kept: Vec<JoinView>,
    chosen: Option<usize>,
    /// The chosen view's source's tables, once read (none while reading or before).
    tables: Option<Tables>,
    reading: bool,
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
    /// Birleşen: a layer or a source's table.
    Joined(Joined),
    SourcePath(String),
    /// Dosya seç…: a GeoPackage file from the disk.
    SourcePick,
    SourcePicked(Option<String>),
    /// Tabloları oku: the source's tables off the UI thread.
    SourceRead,
    SourceTables(String, Result<Vec<SourceTable>, String>),
    SourceTable(String),
    SumAdd,
    SumName(usize, String),
    SumHow(usize, JoinHow),
    SumField(usize, String),
    SumRemove(usize),
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
            tables: None,
            reading: false,
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
            Event::Choose(i) => {
                w.chosen = Some(i);
                w.tables = None;
                // A source's tables read for the lists (nothing waits on them).
                if w.rows.get(i).is_some_and(|v| v.source.is_some()) {
                    return Task::done(msg(Event::SourceRead));
                }
            }
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
                    source: None,
                    aggregates: Vec::new(),
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
                        source: None,
                        aggregates: Vec::new(),
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
                    v.source = None;
                }
            }
            Event::Joined(to) => {
                if let Some(v) = chosen {
                    match to {
                        Joined::Layer if v.source.is_some() => {
                            v.source = None;
                            v.with = layers.first().map(|(id, _)| id.clone()).unwrap_or_default();
                            v.with_key = UID_KEY.into();
                            v.fields.clear();
                        }
                        Joined::Source if v.source.is_none() => {
                            v.source = Some(JoinSource {
                                path: String::new(),
                                table: String::new(),
                            });
                            v.with.clear();
                            v.with_key = UID_KEY.into();
                            v.fields.clear();
                        }
                        _ => {}
                    }
                }
                w.tables = None;
            }
            Event::SourcePath(path) => {
                if let Some(s) = chosen.and_then(|v| v.source.as_mut()) {
                    s.path = path;
                }
                w.tables = None;
            }
            Event::SourcePick => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("GeoPackage", &["gpkg"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_string_lossy().into_owned())
                    },
                    |path| msg(Event::SourcePicked(path)),
                );
            }
            Event::SourcePicked(None) => {}
            Event::SourcePicked(Some(path)) => {
                if let Some(s) = chosen.and_then(|v| v.source.as_mut()) {
                    s.path = path;
                }
                w.tables = None;
                return Task::done(msg(Event::SourceRead));
            }
            Event::SourceRead => {
                let Some(path) = chosen
                    .and_then(|v| v.source.as_ref())
                    .map(|s| s.path.trim().to_owned())
                    .filter(|p| !p.is_empty())
                else {
                    return Task::none();
                };
                w.reading = true;
                return off_thread(move || {
                    let r = crate::exchange::list_tables(path.clone());
                    msg(Event::SourceTables(path, r))
                });
            }
            Event::SourceTables(path, tables) => {
                w.reading = false;
                let now = chosen
                    .and_then(|v| v.source.as_ref())
                    .map(|s| s.path.trim().to_owned());
                if now.as_deref() == Some(path.as_str()) {
                    w.tables = Some((path, tables));
                }
            }
            Event::SourceTable(table) => {
                if let Some(v) = chosen {
                    if let Some(s) = v.source.as_mut() {
                        s.table = table;
                    }
                    v.with_key = UID_KEY.into();
                    v.fields.clear();
                }
            }
            Event::SumAdd => {
                if let Some(v) = chosen.filter(|v| v.aggregates.len() < MOST_JOIN_AGGREGATES) {
                    let n = v.aggregates.len() + 1;
                    v.aggregates.push(JoinAggregate {
                        name: if n == 1 {
                            "Kayıt sayısı".into()
                        } else {
                            format!("Toplama {n}")
                        },
                        how: JoinHow::Count,
                        field: None,
                    });
                }
            }
            Event::SumName(i, name) => {
                if let Some(a) = chosen.and_then(|v| v.aggregates.get_mut(i)) {
                    a.name = name;
                }
            }
            Event::SumHow(i, how) => {
                if let Some(a) = chosen.and_then(|v| v.aggregates.get_mut(i)) {
                    a.how = how;
                }
            }
            Event::SumField(i, field) => {
                if let Some(a) = chosen.and_then(|v| v.aggregates.get_mut(i)) {
                    a.field = (!field.is_empty()).then_some(field);
                }
            }
            Event::SumRemove(i) => {
                if let Some(v) = chosen.filter(|v| i < v.aggregates.len()) {
                    v.aggregates.remove(i);
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
    pub(crate) fn join_keys_of(&self, layer: &str) -> Vec<(String, String)> {
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
                label::caption(format!(
                    "{} ⋈ {}",
                    name_of(&v.layer),
                    match &v.source {
                        Some(src) if src.table.is_empty() => "kaynak tablosu".to_owned(),
                        Some(src) => src.table.clone(),
                        None => name_of(&v.with),
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
            // The form keeps clear of the scroll bar.
            Some(v) => scrollable(
                container(self.join_editor(v, &layers)).padding(iced::Padding {
                    right: 16.0,
                    ..iced::Padding::ZERO
                }),
            )
            .height(typography::scaled(460.0))
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
            .width(860.0)
            .modal(msg(Event::Cancel))
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
        let name = kentos_ui::widget::text_field("Görünümün adı", &v.name)
            .on_input(|t| msg(Event::Name(t)));
        let joined = if v.source.is_some() {
            Joined::Source
        } else {
            Joined::Layer
        };
        let mut form = Column::new()
            .spacing(10)
            .push(words::field("Ad", name, None))
            .push(words::field("Katman", pick(&v.layer, Event::Layer), None))
            .push(words::field(
                "Anahtar",
                key(&v.layer, &v.key, Event::Key),
                None,
            ))
            .push(words::field(
                "Birleşen",
                Segmented::new([Joined::Layer, Joined::Source], joined, |j| {
                    msg(Event::Joined(j))
                }),
                None,
            ));
        // The joined records' keys and fields: the layer's, or the source table's columns.
        let joined_fields: Vec<(String, String)>;
        match &v.source {
            None => {
                form = form
                    .push(words::field(
                        "Birleşen katman",
                        pick(&v.with, Event::With),
                        None,
                    ))
                    .push(words::field(
                        "Anahtar",
                        key(&v.with, &v.with_key, Event::WithKey),
                        None,
                    ));
                joined_fields = self.join_keys_of(&v.with).into_iter().skip(1).collect();
            }
            Some(src) => {
                let w = self.joins_window.as_ref();
                let path = kentos_ui::widget::text_field(
                    "GeoPackage dosyası ya da postgresql://kullanıcı@sunucu/veritabanı",
                    &src.path,
                )
                .on_input(|t| msg(Event::SourcePath(t)))
                .on_submit(msg(Event::SourceRead));
                form = form.push(words::field(
                    "Kaynak",
                    row![
                        container(path).width(Fill),
                        words::secondary(PICK_FILE, Some(msg(Event::SourcePick))),
                        words::secondary(
                            READ_TABLES,
                            (!src.path.trim().is_empty()).then(|| msg(Event::SourceRead))
                        ),
                    ]
                    .spacing(6)
                    .align_y(Center),
                    None,
                ));
                let tables = w.and_then(|w| w.tables.as_ref());
                let reading = w.is_some_and(|w| w.reading);
                let read: &[SourceTable] = match tables {
                    Some((_, Ok(t))) => t,
                    _ => &[],
                };
                let names: Vec<String> = read.iter().map(|t| t.name.clone()).collect();
                let chosen = names.iter().position(|n| *n == src.table);
                let table_pick: Element<'a, Message> = match tables {
                    Some((_, Err(why))) => label::caption(why.clone())
                        .style(style::text::warning)
                        .into(),
                    _ if reading => label::caption("Tablolar okunuyor…").style(muted).into(),
                    None if !src.table.is_empty() => label::body(src.table.clone()).into(),
                    None => label::caption(
                        "Kaynağı yazın ya da seçin; Tabloları oku tablolarını getirir.",
                    )
                    .style(muted)
                    .into(),
                    Some(_) => {
                        let ids = names.clone();
                        Select::new(
                            read.iter().map(|t| {
                                Choice::new(t.name.clone()).detail(format!("{} kayıt", t.rows))
                            }),
                            chosen,
                            move |i| msg(Event::SourceTable(ids[i].clone())),
                        )
                        .placeholder("Tablo")
                        .into()
                    }
                };
                form = form.push(words::field("Tablo", table_pick, None));
                let columns: Vec<String> = read
                    .iter()
                    .find(|t| t.name == src.table)
                    .map(|t| t.columns.iter().map(|c| c.name.clone()).collect())
                    .unwrap_or_default();
                let mut keys = vec![(UID_KEY.to_owned(), "Kaydın kalıcı kimliği".to_owned())];
                keys.extend(columns.iter().map(|c| (c.clone(), c.clone())));
                if !keys.iter().any(|(k, _)| *k == v.with_key) {
                    keys.push((v.with_key.clone(), v.with_key.clone()));
                }
                let ids: Vec<String> = keys.iter().map(|(k, _)| k.clone()).collect();
                form = form.push(words::field(
                    "Anahtar",
                    Select::new(
                        keys.iter().map(|(_, n)| Choice::new(n.clone())),
                        ids.iter().position(|k| *k == v.with_key),
                        move |i| msg(Event::WithKey(ids[i].clone())),
                    )
                    .placeholder("Anahtar sütunu"),
                    None,
                ));
                joined_fields = columns.into_iter().map(|c| (c.clone(), c)).collect();
            }
        }
        form = form.push(words::check(
            v.keep == JoinKeep::Matched,
            MATCHED,
            Some(msg(Event::Matched)),
        ));
        // Toplama sütunları: a heading, what it does and the field.
        let mut sums = Column::new().spacing(6);
        for (i, a) in v.aggregates.iter().enumerate() {
            let head = kentos_ui::widget::text_field("Sütunun adı", &a.name)
                .on_input(move |t| msg(Event::SumName(i, t)));
            let how = Select::new(
                HOWS.iter().map(|(_, w)| Choice::new(*w)),
                HOWS.iter().position(|(h, _)| *h == a.how),
                move |k| msg(Event::SumHow(i, HOWS[k].0)),
            );
            let mut fields = vec![(String::new(), "Eşleşen kayıtlar".to_owned())];
            fields.extend(joined_fields.iter().cloned());
            if let Some(f) = &a.field
                && !fields.iter().any(|(k, _)| k == f)
            {
                fields.push((f.clone(), f.clone()));
            }
            let ids: Vec<String> = fields.iter().map(|(k, _)| k.clone()).collect();
            let at = a.field.clone().unwrap_or_default();
            let field = Select::new(
                fields.iter().map(|(_, n)| Choice::new(n.clone())),
                ids.iter().position(|k| *k == at),
                move |k| msg(Event::SumField(i, ids[k].clone())),
            );
            sums = sums.push(
                row![
                    container(head).width(Length::FillPortion(3)),
                    container(how).width(Length::FillPortion(2)),
                    container(field).width(Length::FillPortion(3)),
                    words::ghost("Kaldır", Some(msg(Event::SumRemove(i)))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        form = form.push(
            column![
                label::strong("Toplama sütunları"),
                sums,
                words::secondary(
                    ADD_SUM,
                    (v.aggregates.len() < MOST_JOIN_AGGREGATES).then(|| msg(Event::SumAdd))
                ),
            ]
            .spacing(6),
        );
        form
            .push(
                label::caption(
                    "Anahtarlar Anahtarla birleştir'in kuralıyla eşleşir (007 ile 7 aynı). Bir kayıt birden çok kayıtla \
                     eşleşirse birden çok satırdır; satırlar her okuyuşta çizimden ve kaynaktan kurulur, kaydedilmez. \
                     Toplama sütunları kaydın eşleşen kayıtlarını sayar ve toplar.",
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
            Control::Press(ADD_SUM) => Some(msg(Event::SumAdd)),
            Control::Press(READ_TABLES) => Some(msg(Event::SourceRead)),
            Control::Press(words) => match w.rows.iter().position(|v| v.name == words) {
                Some(i) => Some(msg(Event::Choose(i))),
                None => return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok")),
            },
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// Runs `work` on a thread of its own and brings its message back.
fn off_thread(work: impl FnOnce() -> Message + Send + 'static) -> Task<Message> {
    iced_runtime::task::blocking(
        move |mut out: iced::futures::channel::mpsc::Sender<Message>| {
            let answer = work();
            let _ =
                iced::futures::executor::block_on(iced::futures::SinkExt::send(&mut out, answer));
        },
    )
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

//! Nesne inceleyici: CAD'deki Özellikler paleti ve ArcGIS'teki öznitelik
//! bölmesi gibi, bir nesnenin özniteliklerini türlerine uygun, zengin
//! düzenleyicilerle gösterir.
//!
//! ```text
//! [⌕ Özellik ara                 ] [≡|A–Z] [◌]
//! ▾ Genel                                   6
//! ┃ T  Ad *         Kuleli İş Merkezi       ↺
//!   ≡  Kullanım     Karma                   ▾
//!   #  Kat          24                     ▴▾
//!   ✓  Asansör      [ — │ Evet │ Hayır ]
//!   ⎍  Doluluk      ━━━━━●━━━  [85   ]  %
//! ▾ Tarihler                                3
//!   ▦  Ruhsat       12.03.2019             [▦]
//!   ◷  Açılış       08:30                  [◷]
//! ▾ Konum                                   2
//!   ⛓  Şehir        İstanbul        ▾  ↗
//!   🔒 Ada/parsel   1204/7
//! ───────────────────────────────────────────
//! Yükseklik
//! Ondalık sayı, 1 basamak, birim m
//! Yapının zeminden en üst noktasına yüksekliği.
//! ```
//!
//! | Alan türü            | Düzenleyici                                        |
//! |----------------------|----------------------------------------------------|
//! | Metin                | Metin girişi; uzun metinde çok satırlı düzenleyici |
//! | Tam sayı, ondalık    | Metin girişi, birim ve artırma/azaltma okları      |
//! | Evet/hayır           | Parçalı seçim; boş bırakılabilirse üç parça        |
//! | Kodlu değer          | Aranabilir açılır liste                            |
//! | Aralık               | Kaydırıcı ve sayı girişi                           |
//! | Tarih, tarih ve saat | Metin girişi ve takvim; saat ızgarasıyla           |
//! | Saat                 | Metin girişi ve saat seçici                        |
//! | Nesne başvurusu      | Aranabilir liste, haritadan seçme, başvuruya git   |
//!
//! Üstteki araç çubuğu alanları arar, kategorili ve alfabetik görünüm
//! arasında geçer, boş alanları gizler. Kategoriler başlıklarına tıklanarak
//! daraltılır. Nesne incelenmeye başladığından beri değişen alanlar solda
//! vurgu çizgisiyle işaretlenir ve ilk değerlerine döndürülebilir. Alttaki
//! yardım bölümü imlecin üzerindeki alanın türünü, kısıtlarını ve
//! açıklamasını gösterir.
//!
//! Bileşen geçici durumunu (yazılmakta olan metinler, arama, daraltılan
//! kategoriler) uygulamanın tuttuğu bir [`State`]'te saklar. Bütün
//! etkileşimler tek bir [`Event`] olarak gelir; uygulama bunu
//! [`State::update`]'e verir ve dönen [`Action`]'ı uygular:
//!
//! ```ignore
//! Message::Inspector(event) => {
//!     if let Some(action) = self.inspector.update(event) {
//!         match action {
//!             inspector::Action::Change { id, value } => self.set_attribute(id, value),
//!             inspector::Action::Pick(id) => self.start_picking(id),
//!             inspector::Action::CancelPick => self.picking = None,
//!             inspector::Action::Navigate { id, object } => self.select_referenced(id, object),
//!         }
//!     }
//! }
//! ```

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{
    Column, Row, button, column, container, mouse_area, row, slider, space, text_editor,
    text_input, tooltip,
};
use iced::{Center, Element, Fill};

use crate::attribute::{DateTime, Field, FieldKind, ObjectId, Value, number, text};
use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::{Choice, DatePicker, Select, TimePicker, Tip, tip};

/// Alan adlarının sütun genişliği (değişiklik çizgisi ve tür ikonu dahil).
const KEY_WIDTH: f32 = 136.0;
/// Satır yüksekliği.
const ROW_HEIGHT: f32 = 26.0;
/// Boş değerin gösterimi.
const EMPTY: &str = "—";

/// Özelliğin kimliği; uygulamanın alan numarası.
pub type PropertyId = usize;

/// Bileşende olan bir şey.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Metin girişi değişti; `value` metnin alan türüne göre çözümlenmesidir.
    Input {
        id: PropertyId,
        text: String,
        value: Result<Value, String>,
    },
    /// Değer doğrudan seçildi (liste, takvim, kaydırıcı, artırma okları).
    Set { id: PropertyId, value: Value },
    /// Uzun metin düzenleyicisini verilen metinle açar.
    Edit { id: PropertyId, text: String },
    /// Uzun metin düzenleyicisinde değişiklik.
    TextAction(PropertyId, text_editor::Action),
    /// Uzun metin düzenleyicisini kapatır; `apply` ise metin uygulanır.
    CloseEditor { id: PropertyId, apply: bool },
    /// Haritadan varlık seçme isteği; seçim sürerken iptal eder.
    Pick(PropertyId),
    /// Başvurulan nesneye git.
    Navigate(PropertyId, ObjectId),
    /// Alanı incelemenin başındaki değerine döndürür.
    Revert(PropertyId),
    /// Alan araması.
    Filter(String),
    /// Alfabetik ya da kategorili görünüm.
    Alphabetical(bool),
    /// Boş alanları gizler.
    HideEmpty(bool),
    /// Kategoriyi daraltır ya da açar.
    Collapse(String),
    /// İmleç alanın üzerine geldi; yardım bölümü onu anlatır.
    Hover(PropertyId),
}

/// Uygulamanın yapması gereken.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Özelliğin değeri değişti.
    Change { id: PropertyId, value: Value },
    /// Kullanıcı özelliğin değerini haritadan seçmek istiyor.
    Pick(PropertyId),
    /// Haritadan seçim iptal edildi.
    CancelPick,
    /// Kullanıcı başvurulan nesneye gitmek istiyor.
    Navigate { id: PropertyId, object: ObjectId },
}

/// Yazılmakta olan, henüz ya da hiç geçerli olmayan metin.
#[derive(Debug, Clone, PartialEq)]
struct Draft {
    text: String,
    error: Option<String>,
}

/// İnceleyicinin geçici durumu.
#[derive(Debug, Clone, Default)]
pub struct State {
    subject: Option<u64>,
    drafts: BTreeMap<PropertyId, Draft>,
    editors: BTreeMap<PropertyId, text_editor::Content>,
    picking: Option<PropertyId>,
    hovered: Option<PropertyId>,
    /// Alanların incelemenin başındaki değerleri; ilk çizimde kaydedilir.
    originals: RefCell<BTreeMap<PropertyId, Value>>,
    // Görünüm tercihleri; incelenen nesne değişince korunur.
    filter: String,
    alphabetical: bool,
    hide_empty: bool,
    collapsed: BTreeSet<String>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    /// İncelenen nesneyi bildirir. Nesne değiştiyse yazılmakta olan
    /// metinler, açık düzenleyiciler, haritadan seçim ve ilk değerler
    /// temizlenir; arama ve görünüm tercihleri korunur.
    pub fn inspect(&mut self, subject: Option<u64>) {
        if self.subject != subject {
            *self = Self {
                subject,
                filter: std::mem::take(&mut self.filter),
                alphabetical: self.alphabetical,
                hide_empty: self.hide_empty,
                collapsed: std::mem::take(&mut self.collapsed),
                ..Self::default()
            };
        }
    }

    /// Haritadan seçim bekleyen özellik.
    pub fn picking(&self) -> Option<PropertyId> {
        self.picking
    }

    /// Haritadan seçimi bitirir (seçildi ya da iptal edildi).
    pub fn stop_picking(&mut self) {
        self.picking = None;
    }

    /// Geçersiz metin girilmiş özellik var mı.
    pub fn has_errors(&self) -> bool {
        self.drafts.values().any(|draft| draft.error.is_some())
    }

    /// Özellik incelemenin başından beri değişti mi.
    pub fn is_modified(&self, id: PropertyId, value: &Value) -> bool {
        self.originals
            .borrow()
            .get(&id)
            .is_some_and(|original| original != value)
    }

    /// İlk çizimde alanın değerini kaydeder.
    fn remember(&self, id: PropertyId, value: &Value) {
        self.originals
            .borrow_mut()
            .entry(id)
            .or_insert_with(|| value.clone());
    }

    /// Olayı uygular; uygulamanın yapması gereken bir şey varsa döndürür.
    pub fn update(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::Input { id, text, value } => match value {
                Ok(value) => {
                    self.drafts.insert(id, Draft { text, error: None });
                    Some(Action::Change { id, value })
                }
                Err(error) => {
                    self.drafts.insert(
                        id,
                        Draft {
                            text,
                            error: Some(error),
                        },
                    );
                    None
                }
            },
            Event::Set { id, value } => {
                self.drafts.remove(&id);
                Some(Action::Change { id, value })
            }
            Event::Edit { id, text } => {
                self.editors
                    .insert(id, text_editor::Content::with_text(&text));
                None
            }
            Event::TextAction(id, action) => {
                if let Some(content) = self.editors.get_mut(&id) {
                    content.perform(action);
                }

                None
            }
            Event::CloseEditor { id, apply } => {
                let content = self.editors.remove(&id)?;

                apply.then(|| {
                    let text = content.text();
                    let text = text.trim_end();

                    self.drafts.remove(&id);

                    Action::Change {
                        id,
                        value: if text.trim().is_empty() {
                            Value::Null
                        } else {
                            Value::Text(text.to_owned())
                        },
                    }
                })
            }
            Event::Pick(id) => {
                if self.picking == Some(id) {
                    self.picking = None;
                    Some(Action::CancelPick)
                } else {
                    self.picking = Some(id);
                    Some(Action::Pick(id))
                }
            }
            Event::Navigate(id, object) => Some(Action::Navigate { id, object }),
            Event::Revert(id) => {
                let original = self.originals.borrow().get(&id).cloned()?;

                self.drafts.remove(&id);
                self.editors.remove(&id);

                Some(Action::Change {
                    id,
                    value: original,
                })
            }
            Event::Filter(filter) => {
                self.filter = filter;
                None
            }
            Event::Alphabetical(alphabetical) => {
                self.alphabetical = alphabetical;
                None
            }
            Event::HideEmpty(hide_empty) => {
                self.hide_empty = hide_empty;
                None
            }
            Event::Collapse(category) => {
                if !self.collapsed.remove(&category) {
                    self.collapsed.insert(category);
                }

                None
            }
            Event::Hover(id) => {
                self.hovered = Some(id);
                None
            }
        }
    }
}

enum Entry<'a> {
    Category(String),
    Field {
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: Vec<(ObjectId, String)>,
        pickable: bool,
    },
    Fixed {
        label: Fragment<'a>,
        value: String,
        mono: bool,
    },
}

impl Entry<'_> {
    fn name(&self) -> &str {
        match self {
            Entry::Category(title) => title,
            Entry::Field { field, .. } => &field.name,
            Entry::Fixed { label, .. } => label,
        }
    }
}

/// Nesne inceleyici.
pub struct Inspector<'a, Message> {
    state: &'a State,
    entries: Vec<Entry<'a>>,
    on_event: Rc<dyn Fn(Event) -> Message + 'a>,
    now: DateTime,
    toolbar: bool,
    help: bool,
}

impl<'a, Message: Clone + 'a> Inspector<'a, Message> {
    pub fn new(state: &'a State, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        Self {
            state,
            entries: Vec::new(),
            on_event: Rc::new(on_event),
            now: DateTime::now(0),
            toolbar: true,
            help: true,
        }
    }

    /// "Bugün" ve "Şimdi"nin yerel zamanı (varsayılan UTC).
    pub fn now(mut self, now: DateTime) -> Self {
        self.now = now;
        self
    }

    /// Üstteki arama ve görünüm araç çubuğu (varsayılan: gösterilir).
    pub fn toolbar(mut self, toolbar: bool) -> Self {
        self.toolbar = toolbar;
        self
    }

    /// Alttaki yardım bölümü (varsayılan: gösterilir).
    pub fn help(mut self, help: bool) -> Self {
        self.help = help;
        self
    }

    /// Sonraki özellikleri gruplayan, daraltılabilen başlık.
    pub fn category(mut self, title: impl Into<String>) -> Self {
        self.entries.push(Entry::Category(title.into()));
        self
    }

    /// Alanın türüne uygun düzenleyiciyle bir özellik. Değiştirilemeyen
    /// alanlar salt okunur gösterilir.
    pub fn field(mut self, id: PropertyId, field: &'a Field, value: &'a Value) -> Self {
        self.state.remember(id, value);
        self.entries.push(Entry::Field {
            id,
            field,
            value,
            candidates: Vec::new(),
            pickable: false,
        });
        self
    }

    /// Nesne başvurusu: `candidates` aranabilir listede (nesne seçici)
    /// gösterilir; `pickable` ise haritadan seçme komutu da eklenir (varlık
    /// seçici). Değer varsa başvurulan nesneye gitme düğmesi gösterilir.
    pub fn object(
        mut self,
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: Vec<(ObjectId, String)>,
        pickable: bool,
    ) -> Self {
        self.state.remember(id, value);
        self.entries.push(Entry::Field {
            id,
            field,
            value,
            candidates,
            pickable,
        });
        self
    }

    /// Salt okunur, hesaplanmış bir değer (ör. uzunluk).
    pub fn fixed(mut self, label: impl IntoFragment<'a>, value: impl Into<String>) -> Self {
        self.entries.push(Entry::Fixed {
            label: label.into_fragment(),
            value: value.into(),
            mono: false,
        });
        self
    }

    /// Salt okunur sayısal değer; eş aralıklı yazılır.
    pub fn figure(mut self, label: impl IntoFragment<'a>, value: impl Into<String>) -> Self {
        self.entries.push(Entry::Fixed {
            label: label.into_fragment(),
            value: value.into(),
            mono: true,
        });
        self
    }

    fn emit(&self, event: Event) -> Message {
        (self.on_event)(event)
    }

    /// Girdinin arama ve "boşları gizle" tercihine göre görünüp görünmediği.
    fn is_visible(&self, entry: &Entry<'_>) -> bool {
        let filter = self.state.filter.trim();

        let matches = filter.is_empty() || text::contains(entry.name(), filter);

        let empty = match entry {
            Entry::Field { id, value, .. } => {
                value.is_null() && !self.state.drafts.contains_key(id)
            }
            _ => false,
        };

        matches && !(self.state.hide_empty && empty)
    }

    /// Metin girişi; yazılmakta olan metin varsa onu gösterir.
    fn input(&self, id: PropertyId, field: &'a Field, value: &Value) -> Element<'a, Message> {
        let draft = self.state.drafts.get(&id);
        let current = draft.map_or_else(|| editable_text(field, value), |draft| draft.text.clone());
        let invalid = draft.is_some_and(|draft| draft.error.is_some());
        let on_event = self.on_event.clone();

        let input = text_input(field.placeholder(), &current)
            .on_input(move |text| {
                on_event(Event::Input {
                    id,
                    value: field.parse(&text),
                    text,
                })
            })
            .size(typography::BODY)
            .padding([3, 4])
            .width(Fill)
            .style(style::field::cell(invalid));

        if field.is_numeric()
            || matches!(
                field.kind,
                FieldKind::Date | FieldKind::Time | FieldKind::DateTime
            )
        {
            input.font(typography::MONO).into()
        } else {
            input.into()
        }
    }

    /// Hücre içindeki küçük ikon düğmesi.
    fn cell_button(
        &self,
        glyph: Icon,
        description: &'a str,
        event: Event,
        active: bool,
    ) -> Element<'a, Message> {
        tip(
            button(icon(glyph).size(13.0))
                .on_press(self.emit(event))
                .padding([3, 4])
                .style(style::button::tool(active)),
            Tip::new(description),
            tooltip::Position::Left,
        )
    }

    fn editor(
        &self,
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: &[(ObjectId, String)],
        pickable: bool,
    ) -> Element<'a, Message> {
        if !field.editable {
            let shown = match (&field.kind, value) {
                (FieldKind::Object { .. }, Value::Object(object)) => {
                    object_label(candidates, *object)
                }
                _ => field.format_with_unit(value),
            };

            return container(label::muted(if shown.is_empty() {
                EMPTY.to_owned()
            } else {
                shown
            }))
            .padding([0, 4])
            .into();
        }

        let unit = field
            .unit
            .as_deref()
            .map(|unit| label::caption(unit.to_owned()));
        let on_event = self.on_event.clone();

        match &field.kind {
            FieldKind::Text if field.multiline => {
                // Uzun metin hücrede ilk satırıyla önizlenir; tıklamak
                // çok satırlı düzenleyiciyi açar.
                let text = value.as_str().unwrap_or_default();
                let mut lines = text.lines();
                let first = lines.next().unwrap_or_default().to_owned();
                let more = lines.count();

                let edit = Event::Edit {
                    id,
                    text: text.to_owned(),
                };

                let mut preview = row![
                    container(if first.is_empty() {
                        label::muted(EMPTY)
                    } else {
                        label::body(first).wrapping(iced::widget::text::Wrapping::None)
                    })
                    .width(Fill)
                    .clip(true)
                ]
                .spacing(6)
                .align_y(Center);

                if more > 0 {
                    preview = preview.push(label::caption(format!("+{more} satır")));
                }

                row![
                    button(preview)
                        .on_press(self.emit(edit.clone()))
                        .padding([3, 4])
                        .width(Fill)
                        .style(style::button::flat),
                    self.cell_button(
                        Icon::Document,
                        "Çok satırlı düzenle",
                        edit,
                        self.state.editors.contains_key(&id),
                    ),
                ]
                .spacing(2)
                .align_y(Center)
                .into()
            }
            FieldKind::Text => self.input(id, field, value),
            FieldKind::Integer { .. } | FieldKind::Real { .. } => {
                let mut editor = row![self.input(id, field, value)]
                    .spacing(4)
                    .align_y(Center);

                if let Some(unit) = unit {
                    editor = editor.push(unit);
                }

                editor.push(self.spinner(id, field, value)).into()
            }
            FieldKind::Bool => self.toggle(id, field, value),
            FieldKind::Choice(options) => {
                let selected = value
                    .as_str()
                    .and_then(|value| options.iter().position(|option| option == value));
                let choices = options.clone();
                let select_event = on_event.clone();

                let select = Select::new(options.iter().map(Choice::new), selected, move |index| {
                    select_event(Event::Set {
                        id,
                        value: Value::Text(choices[index].clone()),
                    })
                })
                .placeholder(EMPTY)
                .borderless();

                if field.required {
                    select.into()
                } else {
                    select
                        .clear(on_event(Event::Set {
                            id,
                            value: Value::Null,
                        }))
                        .into()
                }
            }
            FieldKind::Range { min, max, step } => {
                let current = value.as_f64().unwrap_or(*min);
                let mut editor = row![
                    slider(*min..=*max, current, move |amount| {
                        on_event(Event::Set {
                            id,
                            value: Value::Real(amount),
                        })
                    })
                    .step(*step),
                    container(self.input(id, field, value)).width(52),
                ]
                .spacing(6)
                .align_y(Center);

                if let Some(unit) = unit {
                    editor = editor.push(unit);
                }

                editor.into()
            }
            FieldKind::Date => {
                let set = on_event.clone();

                DatePicker::date(
                    match value {
                        Value::Date(date) => Some(*date),
                        _ => None,
                    },
                    move |date| {
                        set(Event::Set {
                            id,
                            value: date.map_or(Value::Null, Value::Date),
                        })
                    },
                )
                .anchor(self.input(id, field, value))
                .now(self.now)
                .clearable(!field.required)
                .into()
            }
            FieldKind::DateTime => {
                let set = on_event.clone();

                DatePicker::date_time(
                    match value {
                        Value::DateTime(moment) => Some(*moment),
                        _ => None,
                    },
                    move |moment| {
                        set(Event::Set {
                            id,
                            value: moment.map_or(Value::Null, Value::DateTime),
                        })
                    },
                )
                .anchor(self.input(id, field, value))
                .now(self.now)
                .clearable(!field.required)
                .into()
            }
            FieldKind::Time => {
                let set = on_event.clone();

                TimePicker::new(
                    match value {
                        Value::Time(time) => Some(*time),
                        _ => None,
                    },
                    move |time| {
                        set(Event::Set {
                            id,
                            value: time.map_or(Value::Null, Value::Time),
                        })
                    },
                )
                .anchor(self.input(id, field, value))
                .now(self.now)
                .clearable(!field.required)
                .into()
            }
            FieldKind::Object { .. } => self.object_editor(id, field, value, candidates, pickable),
        }
    }

    /// Sayının yanındaki artırma/azaltma okları.
    fn spinner(&self, id: PropertyId, field: &Field, value: &Value) -> Element<'a, Message> {
        let arrow = |glyph: Icon, direction: f64| {
            let next =
                stepped(field, value, direction).map(|value| self.emit(Event::Set { id, value }));

            button(container(icon(glyph).size(9.0)).center(Fill))
                .on_press_maybe(next)
                .width(14)
                .height(11)
                .padding(0)
                .style(style::button::flat)
        };

        column![arrow(Icon::ChevronUp, 1.0), arrow(Icon::ChevronDown, -1.0)]
            .spacing(1)
            .into()
    }

    /// Evet/hayır: parçalı seçim; boş bırakılabilen alanda "—" parçası da
    /// vardır.
    fn toggle(&self, id: PropertyId, field: &Field, value: &Value) -> Element<'a, Message> {
        let mut options = Vec::with_capacity(3);

        if !field.required {
            options.push((EMPTY, Value::Null));
        }

        options.push(("Evet", Value::Bool(true)));
        options.push(("Hayır", Value::Bool(false)));

        let segments = options.into_iter().map(|(name, option)| {
            let selected = *value == option;

            button(label::body(name))
                .on_press(self.emit(Event::Set { id, value: option }))
                .padding([1, 10])
                .style(style::button::segment(selected))
                .into()
        });

        container(Row::with_children(segments).spacing(1))
            .padding(1)
            .style(style::container::segmented)
            .into()
    }

    /// Nesne başvurusu: aranabilir liste, haritadan seçme ve başvuruya git.
    fn object_editor(
        &self,
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: &[(ObjectId, String)],
        pickable: bool,
    ) -> Element<'a, Message> {
        if self.state.picking == Some(id) {
            return row![
                label::body("Haritada seçin")
                    .style(style::text::accent)
                    .width(Fill),
                self.cell_button(
                    Icon::Target,
                    "Haritadan seçimi iptal et",
                    Event::Pick(id),
                    true
                ),
            ]
            .spacing(2)
            .align_y(Center)
            .into();
        }

        let selected = value.as_object().and_then(|object| {
            candidates
                .iter()
                .position(|(candidate, _)| *candidate == object)
        });
        let objects: Vec<ObjectId> = candidates.iter().map(|(object, _)| *object).collect();
        let on_event = self.on_event.clone();

        let mut select = Select::new(
            candidates
                .iter()
                .map(|(object, name)| Choice::new(name.clone()).detail(format!("#{object}"))),
            selected,
            move |index| {
                on_event(Event::Set {
                    id,
                    value: Value::Object(objects[index]),
                })
            },
        )
        .placeholder("Seçilmedi")
        .borderless();

        if pickable {
            select = select.action(Icon::Target, "Haritadan seç", self.emit(Event::Pick(id)));
        }

        if !field.required {
            select = select.clear(self.emit(Event::Set {
                id,
                value: Value::Null,
            }));
        }

        let mut editor = row![select].spacing(2).align_y(Center);

        if let Some(object) = value.as_object() {
            editor = editor.push(self.cell_button(
                Icon::Open,
                "Başvurulan nesneye git",
                Event::Navigate(id, object),
                false,
            ));
        }

        editor.into()
    }

    /// Alan satırı; altında varsa uzun metin düzenleyicisi ve hata satırı.
    fn field_rows(
        &self,
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: &[(ObjectId, String)],
        pickable: bool,
        rows: &mut Vec<Element<'a, Message>>,
    ) {
        let modified = self.state.is_modified(id, value);

        let name = if field.required && field.editable {
            format!("{} *", field.name)
        } else {
            field.name.clone()
        };

        let name: Element<'a, Message> = if field.editable {
            label::body(name).into()
        } else {
            row![
                label::muted(name),
                icon(Icon::Lock).size(11.0).tone(Tone::Muted)
            ]
            .spacing(4)
            .align_y(Center)
            .into()
        };

        let key = container(
            row![
                marker(modified),
                container(icon(kind_icon(field)).size(13.0).tone(Tone::Muted))
                    .width(22)
                    .center_x(22),
                name,
            ]
            .align_y(Center),
        )
        .width(KEY_WIDTH)
        .height(ROW_HEIGHT)
        .align_y(Center)
        .style(style::container::surface);

        let mut line = row![
            key,
            cell(self.editor(id, field, value, candidates, pickable))
        ]
        .spacing(1);

        if modified {
            line = line.push(
                container(tip(
                    button(icon(Icon::Undo).size(12.0))
                        .on_press(self.emit(Event::Revert(id)))
                        .padding([3, 3])
                        .style(style::button::subtle),
                    Tip::new("İlk değerine döndür").body(original_text(self.state, id, field)),
                    tooltip::Position::Left,
                ))
                .height(ROW_HEIGHT)
                .align_y(Center)
                .style(style::container::surface_alt),
            );
        }

        rows.push(
            mouse_area(line)
                .on_enter(self.emit(Event::Hover(id)))
                .into(),
        );

        if let Some(content) = self.state.editors.get(&id) {
            let on_event = self.on_event.clone();

            rows.push(full_width(
                column![
                    text_editor(content)
                        .on_action(move |action| on_event(Event::TextAction(id, action)))
                        .size(typography::BODY)
                        .height(96)
                        .style(style::field::text_area),
                    row![
                        space::horizontal(),
                        button(label::body("Vazgeç"))
                            .on_press(self.emit(Event::CloseEditor { id, apply: false }))
                            .padding([3, 10])
                            .style(style::button::secondary),
                        button(label::body("Uygula"))
                            .on_press(self.emit(Event::CloseEditor { id, apply: true }))
                            .padding([3, 10])
                            .style(style::button::primary),
                    ]
                    .spacing(6),
                ]
                .spacing(6),
            ));
        }

        if let Some(error) = self
            .state
            .drafts
            .get(&id)
            .and_then(|draft| draft.error.clone())
        {
            rows.push(
                container(
                    row![
                        icon(Icon::Warning).size(12.0).tone(Tone::Danger),
                        label::caption(error).style(style::text::danger),
                    ]
                    .spacing(6)
                    .align_y(Center),
                )
                .padding([3, 8])
                .width(Fill)
                .style(style::container::surface)
                .into(),
            );
        }
    }

    fn fixed_row(&self, name: &Fragment<'a>, value: &str, mono: bool) -> Element<'a, Message> {
        let value = if mono {
            label::mono(value.to_owned())
        } else {
            label::body(value.to_owned())
        };

        row![
            container(
                row![
                    marker(false),
                    container(icon(Icon::Lock).size(11.0).tone(Tone::Muted))
                        .width(22)
                        .center_x(22),
                    label::muted(name.clone()),
                ]
                .align_y(Center),
            )
            .width(KEY_WIDTH)
            .height(ROW_HEIGHT)
            .align_y(Center)
            .style(style::container::surface),
            cell(container(value).padding([0, 4])),
        ]
        .spacing(1)
        .into()
    }

    fn category_row(&self, title: &str, count: usize) -> Element<'a, Message> {
        let collapsed = self.state.collapsed.contains(title);

        button(
            row![
                icon(if collapsed {
                    Icon::ChevronRight
                } else {
                    Icon::ChevronDown
                })
                .size(10.0)
                .tone(Tone::Muted),
                label::caption(title.to_owned())
                    .font(typography::UI_STRONG)
                    .style(style::text::default),
                space::horizontal(),
                label::mono_caption(count.to_string()),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .on_press(self.emit(Event::Collapse(title.to_owned())))
        .padding([3, 8])
        .width(Fill)
        .style(style::button::category)
        .into()
    }

    fn toolbar_row(&self) -> Element<'a, Message> {
        let on_event = self.on_event.clone();
        let state = self.state;

        let search = container(
            row![
                icon(Icon::Search).size(13.0).tone(Tone::Muted),
                text_input("Özellik ara", &state.filter)
                    .on_input(move |filter| on_event(Event::Filter(filter)))
                    .size(typography::BODY)
                    .padding([3, 0])
                    .style(style::field::bare_input),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([0, 8])
        .width(Fill)
        .style(style::container::field_box);

        let mode = |content: Element<'a, Message>,
                    description: &'static str,
                    active: bool,
                    event: Event| {
            tip(
                button(container(content).center_y(18))
                    .on_press(self.emit(event))
                    .padding([1, 6])
                    .style(style::button::segment(active)),
                Tip::new(description),
                tooltip::Position::Bottom,
            )
        };

        row![
            search,
            container(
                row![
                    mode(
                        icon(Icon::Properties).size(13.0).into(),
                        "Kategorili",
                        !state.alphabetical,
                        Event::Alphabetical(false)
                    ),
                    mode(
                        label::caption("A–Z").style(style::text::default).into(),
                        "Alfabetik",
                        state.alphabetical,
                        Event::Alphabetical(true)
                    ),
                ]
                .spacing(1),
            )
            .padding(1)
            .style(style::container::segmented),
            tip(
                button(icon(Icon::EyeOff).size(13.0))
                    .on_press(self.emit(Event::HideEmpty(!state.hide_empty)))
                    .padding([3, 5])
                    .style(style::button::tool(state.hide_empty)),
                Tip::new(if state.hide_empty {
                    "Boş alanları göster"
                } else {
                    "Boş alanları gizle"
                }),
                tooltip::Position::Bottom,
            ),
        ]
        .spacing(6)
        .align_y(Center)
        .into()
    }

    /// Yardım bölümü: imlecin üzerindeki alanın türü, kısıtları ve
    /// açıklaması; varsa hatalı alan sayısı.
    fn help_section(&self) -> Element<'a, Message> {
        let errors = self
            .state
            .drafts
            .values()
            .filter(|draft| draft.error.is_some())
            .count();

        let hovered = self.state.hovered.and_then(|hovered| {
            self.entries.iter().find_map(|entry| match entry {
                Entry::Field { id, field, .. } if *id == hovered => Some(*field),
                _ => None,
            })
        });

        let mut content = Column::new().spacing(3);

        if errors > 0 {
            content = content.push(
                row![
                    icon(Icon::Warning).size(12.0).tone(Tone::Danger),
                    label::caption(format!(
                        "{errors} alanda geçersiz değer; düzeltilene kadar kaydedilmez."
                    ))
                    .style(style::text::danger),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }

        content = match hovered {
            Some(field) => {
                let mut flags = Vec::new();

                if field.required {
                    flags.push("Zorunlu");
                }

                if !field.editable {
                    flags.push("Salt okunur");
                }

                let mut title = row![label::strong(field.name.clone())]
                    .spacing(8)
                    .align_y(Center);

                if !flags.is_empty() {
                    title = title.push(label::caption(flags.join(", ")));
                }

                let mut content = content.push(title).push(label::caption(field.summary()));

                if let Some(description) = &field.description {
                    content = content.push(label::muted(description.clone()));
                }

                content
            }
            None => content.push(label::caption(
                "Bir alanın üzerine gelin: türü, kısıtları ve açıklaması burada görünür.",
            )),
        };

        container(content)
            .padding([8, 10])
            .width(Fill)
            .style(style::container::surface)
            .into()
    }
}

/// Alanın türünü gösteren ikon.
fn kind_icon(field: &Field) -> Icon {
    match field.kind {
        FieldKind::Text if field.multiline => Icon::Document,
        FieldKind::Text => Icon::Type,
        FieldKind::Integer { .. } | FieldKind::Real { .. } => Icon::Hash,
        FieldKind::Bool => Icon::Check,
        FieldKind::Choice(_) => Icon::Properties,
        FieldKind::Range { .. } => Icon::Slider,
        FieldKind::Date | FieldKind::DateTime => Icon::Calendar,
        FieldKind::Time => Icon::Clock,
        FieldKind::Object { .. } => Icon::Link,
    }
}

/// Değişen alanı gösteren, satırın solundaki ince çizgi.
fn marker<'a, Message: 'a>(modified: bool) -> Element<'a, Message> {
    container(space::horizontal())
        .width(2)
        .height(ROW_HEIGHT)
        .style(move |theme| container::Style {
            background: modified.then(|| Tokens::of(theme).accent.into()),
            ..container::Style::default()
        })
        .into()
}

/// İlk değerin yazımı; geri alma düğmesinin ipucunda.
fn original_text(state: &State, id: PropertyId, field: &Field) -> String {
    let original = state
        .originals
        .borrow()
        .get(&id)
        .map(|value| field.format_with_unit(value))
        .unwrap_or_default();

    if original.is_empty() {
        "İlk değer: boş".to_owned()
    } else {
        format!("İlk değer: {original}")
    }
}

/// Artırma okunun üreteceği değer; sınırların dışına çıkmaz.
fn stepped(field: &Field, value: &Value, direction: f64) -> Option<Value> {
    match field.kind {
        FieldKind::Integer { min, max } => {
            let current = match value {
                Value::Integer(value) => *value,
                _ => min.unwrap_or(0),
            };
            let next = (current + direction as i64)
                .max(min.unwrap_or(i64::MIN))
                .min(max.unwrap_or(i64::MAX));

            (next != current || value.is_null()).then_some(Value::Integer(next))
        }
        FieldKind::Real { decimals } => {
            let scale = 10f64.powi(i32::from(decimals));
            let current = value.as_f64().unwrap_or(0.0);

            Some(Value::Real(((current * scale).round() + direction) / scale))
        }
        FieldKind::Range { min, max, step } => {
            let current = value.as_f64().unwrap_or(min);
            let next = (current + step * direction).clamp(min, max);

            (next != current || value.is_null()).then_some(Value::Real(next))
        }
        _ => None,
    }
}

/// Düzenleme için metin: sayılar binlik ayraçsız yazılır ki kolay
/// düzeltilsin.
fn editable_text(field: &Field, value: &Value) -> String {
    match (&field.kind, value) {
        (_, Value::Integer(value)) => value.to_string(),
        (FieldKind::Real { decimals }, Value::Real(value)) => {
            number::real(*value, usize::from(*decimals)).replace('.', "")
        }
        (FieldKind::Range { step, .. }, Value::Real(value)) => {
            number::real(*value, number::decimals_of(*step)).replace('.', "")
        }
        _ => field.format(value),
    }
}

fn object_label(candidates: &[(ObjectId, String)], object: ObjectId) -> String {
    candidates
        .iter()
        .find(|(candidate, _)| *candidate == object)
        .map_or_else(|| format!("#{object}"), |(_, name)| name.clone())
}

fn cell<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Fill)
        .height(ROW_HEIGHT)
        .padding([0, 4])
        .align_y(Center)
        .style(style::container::surface_alt)
        .into()
}

fn full_width<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(8)
        .width(Fill)
        .style(style::container::surface)
        .into()
}

impl<'a, Message: Clone + 'a> From<Inspector<'a, Message>> for Element<'a, Message> {
    fn from(inspector: Inspector<'a, Message>) -> Self {
        let mut rows: Vec<Element<'a, Message>> = Vec::with_capacity(inspector.entries.len() + 2);

        let render = |entry: &Entry<'a>, rows: &mut Vec<Element<'a, Message>>| match entry {
            Entry::Field {
                id,
                field,
                value,
                candidates,
                pickable,
            } => inspector.field_rows(*id, field, value, candidates, *pickable, rows),
            Entry::Fixed { label, value, mono } => {
                rows.push(inspector.fixed_row(label, value, *mono));
            }
            Entry::Category(_) => {}
        };

        if inspector.state.alphabetical {
            let mut entries: Vec<&Entry<'a>> = inspector
                .entries
                .iter()
                .filter(|entry| !matches!(entry, Entry::Category(_)))
                .filter(|entry| inspector.is_visible(entry))
                .collect();

            entries.sort_by(|a, b| text::compare(a.name(), b.name()));

            for entry in entries {
                render(entry, &mut rows);
            }
        } else {
            // Kategorisiz girdiler başta, başlıksız gösterilir.
            let mut groups: Vec<(Option<&str>, Vec<&Entry<'a>>)> = vec![(None, Vec::new())];

            for entry in &inspector.entries {
                match entry {
                    Entry::Category(title) => groups.push((Some(title), Vec::new())),
                    entry => {
                        if let Some((_, entries)) = groups.last_mut() {
                            entries.push(entry);
                        }
                    }
                }
            }

            for (title, entries) in groups {
                let visible: Vec<&Entry<'a>> = entries
                    .into_iter()
                    .filter(|entry| inspector.is_visible(entry))
                    .collect();

                if let Some(title) = title {
                    if visible.is_empty() {
                        continue;
                    }

                    rows.push(inspector.category_row(title, visible.len()));

                    if inspector.state.collapsed.contains(title) {
                        continue;
                    }
                }

                for entry in visible {
                    render(entry, &mut rows);
                }
            }
        }

        if rows.is_empty() {
            rows.push(
                container(label::muted("Aramaya uyan alan yok."))
                    .padding([8, 10])
                    .width(Fill)
                    .style(style::container::surface)
                    .into(),
            );
        }

        let grid = container(Column::with_children(rows).spacing(1))
            .padding([1, 0])
            .width(Fill)
            .style(style::container::grid_lines);

        let mut content = Column::new().width(Fill);

        if inspector.toolbar {
            content = content.push(
                container(inspector.toolbar_row())
                    .padding([6, 8])
                    .width(Fill)
                    .style(style::container::surface),
            );
        }

        content = content.push(grid);

        if inspector.help {
            content = content.push(inspector.help_section());
        }

        content.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_input_changes_the_value_and_invalid_input_is_kept_as_draft() {
        let mut state = State::new();
        let field = Field::integer("Plaka").between(1, 81);

        let action = state.update(Event::Input {
            id: 3,
            text: "34".to_owned(),
            value: field.parse("34"),
        });
        assert_eq!(
            action,
            Some(Action::Change {
                id: 3,
                value: Value::Integer(34)
            })
        );
        assert!(!state.has_errors());

        let action = state.update(Event::Input {
            id: 3,
            text: "99".to_owned(),
            value: field.parse("99"),
        });
        assert_eq!(action, None);
        assert!(state.has_errors());

        state.inspect(Some(7));
        assert!(!state.has_errors());
    }

    #[test]
    fn view_preferences_survive_a_new_subject() {
        let mut state = State::new();
        state.update(Event::Filter("ad".to_owned()));
        state.update(Event::Alphabetical(true));
        state.update(Event::Collapse("Genel".to_owned()));
        state.remember(1, &Value::Integer(5));

        state.inspect(Some(2));

        assert_eq!(state.filter, "ad");
        assert!(state.alphabetical);
        assert!(state.collapsed.contains("Genel"));
        assert!(state.originals.borrow().is_empty());

        state.update(Event::Collapse("Genel".to_owned()));
        assert!(!state.collapsed.contains("Genel"));
    }

    #[test]
    fn modified_fields_revert_to_their_first_value() {
        let mut state = State::new();
        state.remember(4, &Value::from("İlk"));
        state.remember(4, &Value::from("Sonraki"));

        assert!(state.is_modified(4, &Value::from("Yeni")));
        assert!(!state.is_modified(4, &Value::from("İlk")));

        assert_eq!(
            state.update(Event::Revert(4)),
            Some(Action::Change {
                id: 4,
                value: Value::from("İlk")
            })
        );
        assert_eq!(state.update(Event::Revert(9)), None);
    }

    #[test]
    fn long_text_is_applied_from_the_editor() {
        let mut state = State::new();

        state.update(Event::Edit {
            id: 2,
            text: "Birinci satır".to_owned(),
        });
        assert_eq!(
            state.update(Event::CloseEditor { id: 2, apply: true }),
            Some(Action::Change {
                id: 2,
                value: Value::from("Birinci satır")
            })
        );
        assert_eq!(
            state.update(Event::CloseEditor { id: 2, apply: true }),
            None
        );
    }

    #[test]
    fn picking_toggles_and_navigation_is_reported() {
        let mut state = State::new();

        assert_eq!(state.update(Event::Pick(4)), Some(Action::Pick(4)));
        assert_eq!(state.picking(), Some(4));
        assert_eq!(state.update(Event::Pick(4)), Some(Action::CancelPick));
        assert_eq!(
            state.update(Event::Navigate(4, ObjectId(2))),
            Some(Action::Navigate {
                id: 4,
                object: ObjectId(2)
            })
        );
    }

    #[test]
    fn steps_stay_within_bounds() {
        let plate = Field::integer("Plaka").between(1, 81);
        assert_eq!(stepped(&plate, &Value::Integer(81), 1.0), None);
        assert_eq!(stepped(&plate, &Value::Null, 1.0), Some(Value::Integer(2)));

        let height = Field::real("Yükseklik", 1);
        assert_eq!(
            stepped(&height, &Value::Real(96.5), -1.0),
            Some(Value::Real(96.4))
        );

        let speed = Field::range("Hız", 30.0, 140.0, 10.0);
        assert_eq!(stepped(&speed, &Value::Real(140.0), 1.0), None);
    }

    #[test]
    fn integers_are_edited_without_grouping() {
        let field = Field::integer("Nüfus");
        assert_eq!(
            editable_text(&field, &Value::Integer(15_840_900)),
            "15840900"
        );

        let height = Field::real("Yükseklik", 2);
        assert_eq!(editable_text(&height, &Value::Real(1234.5)), "1234,50");
    }
}

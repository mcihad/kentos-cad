//! Nesne inceleyici: CAD'deki Özellikler paleti ve ArcGIS'teki öznitelik
//! bölmesi gibi, bir nesnenin özniteliklerini türlerine uygun
//! düzenleyicilerle gösterir.
//!
//! | Alan türü                 | Düzenleyici                                     |
//! |---------------------------|-------------------------------------------------|
//! | Metin, tam sayı, ondalık  | Metin girişi; yazarken denetlenir               |
//! | Evet/hayır                | Onay kutusu                                     |
//! | Kodlu değer (seçenekler)  | Açılır liste                                    |
//! | Aralık                    | Kaydırıcı ve değer                              |
//! | Tarih, tarih ve saat      | Metin girişi ve açılır takvim                   |
//! | Saat                      | Metin girişi ve "şimdi" düğmesi                 |
//! | Nesne başvurusu           | Aranabilir liste (nesne seçici); isteğe bağlı   |
//! |                           | olarak haritadan seçme düğmesi (varlık seçici)  |
//!
//! Bileşen, açık takvim ve yazılmakta olan metin gibi geçici durumunu
//! uygulamanın tuttuğu bir [`State`]'te saklar. Bütün etkileşimler tek bir
//! [`Event`] olarak gelir; uygulama bunu [`State::update`]'e verir ve dönen
//! [`Action`]'ı uygular:
//!
//! ```ignore
//! Message::Inspector(event) => {
//!     if let Some(action) = self.inspector.update(event) {
//!         match action {
//!             inspector::Action::Change { id, value } => self.set_attribute(id, value),
//!             inspector::Action::Pick(id) => self.start_picking(id),
//!             inspector::Action::CancelPick => self.picking = None,
//!         }
//!     }
//! }
//! ```

use std::collections::BTreeMap;
use std::rc::Rc;

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{
    Column, button, checkbox, column, container, pick_list, row, slider, text, text_input, tooltip,
};
use iced::{Center, Element, Fill, Right};

use crate::attribute::time::WEEKDAYS;
use crate::attribute::{Date, DateTime, Field, FieldKind, ObjectId, Time, Value, number};
use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::typography;
use crate::widget::{Tip, tip};

/// Alan adlarının sütun genişliği.
const KEY_WIDTH: f32 = 124.0;
/// Satır yüksekliği.
const ROW_HEIGHT: f32 = 26.0;
/// Nesne seçicide gösterilen en fazla aday.
const MAX_CANDIDATES: usize = 8;
/// Takvim günlerinin genişliği.
const DAY_WIDTH: f32 = 30.0;
/// Seçenek listesinde "boş" değeri temsil eden seçenek.
const EMPTY_CHOICE: &str = "—";

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
    /// Değer doğrudan seçildi (onay kutusu, liste, kaydırıcı, takvim, nesne).
    Set { id: PropertyId, value: Value },
    /// Takvimi ya da nesne listesini açar; aynı özellik için kapatır.
    Expand(Option<PropertyId>),
    /// Nesne seçicideki arama metni.
    Search(String),
    /// Takvimde gösterilen ay.
    Month(Date),
    /// Haritadan varlık seçme isteği; seçim sürerken iptal eder.
    Pick(PropertyId),
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
}

/// Yazılmakta olan, henüz ya da hiç geçerli olmayan metin.
#[derive(Debug, Clone, PartialEq)]
struct Draft {
    text: String,
    error: Option<String>,
}

/// İnceleyicinin geçici durumu.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    subject: Option<u64>,
    drafts: BTreeMap<PropertyId, Draft>,
    expanded: Option<PropertyId>,
    search: String,
    month: Option<Date>,
    picking: Option<PropertyId>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    /// İncelenen nesneyi bildirir. Nesne değiştiyse yazılmakta olan metinler,
    /// açık takvim ve haritadan seçim temizlenir.
    pub fn inspect(&mut self, subject: Option<u64>) {
        if self.subject != subject {
            *self = Self {
                subject,
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
                self.expanded = None;
                self.search.clear();
                Some(Action::Change { id, value })
            }
            Event::Expand(target) => {
                self.expanded = if target == self.expanded {
                    None
                } else {
                    target
                };
                self.search.clear();
                self.month = None;
                None
            }
            Event::Search(search) => {
                self.search = search;
                None
            }
            Event::Month(month) => {
                self.month = Some(month.first_of_month());
                None
            }
            Event::Pick(id) => {
                self.expanded = None;

                if self.picking == Some(id) {
                    self.picking = None;
                    Some(Action::CancelPick)
                } else {
                    self.picking = Some(id);
                    Some(Action::Pick(id))
                }
            }
        }
    }
}

enum Entry<'a> {
    Category(Fragment<'a>),
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

/// Nesne inceleyici.
pub struct Inspector<'a, Message> {
    state: &'a State,
    entries: Vec<Entry<'a>>,
    on_event: Rc<dyn Fn(Event) -> Message + 'a>,
    now: DateTime,
}

impl<'a, Message: Clone + 'a> Inspector<'a, Message> {
    pub fn new(state: &'a State, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        Self {
            state,
            entries: Vec::new(),
            on_event: Rc::new(on_event),
            now: DateTime::now(0),
        }
    }

    /// "Bugün" ve "Şimdi" düğmelerinin kullandığı yerel zaman (varsayılan
    /// UTC).
    pub fn now(mut self, now: DateTime) -> Self {
        self.now = now;
        self
    }

    /// Sonraki özellikleri gruplayan başlık.
    pub fn category(mut self, title: impl IntoFragment<'a>) -> Self {
        self.entries.push(Entry::Category(title.into_fragment()));
        self
    }

    /// Alanın türüne uygun düzenleyiciyle bir özellik. Değiştirilemeyen
    /// alanlar salt okunur gösterilir.
    pub fn field(mut self, id: PropertyId, field: &'a Field, value: &'a Value) -> Self {
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
    /// gösterilir; `pickable` ise haritadan seçme düğmesi de eklenir (varlık
    /// seçici).
    pub fn object(
        mut self,
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: Vec<(ObjectId, String)>,
        pickable: bool,
    ) -> Self {
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
            button(icon(glyph).size(14.0))
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

            return label::muted(if shown.is_empty() {
                EMPTY_CHOICE.to_owned()
            } else {
                shown
            })
            .into();
        }

        let expanded = self.state.expanded == Some(id);
        let unit = field
            .unit
            .as_deref()
            .map(|unit| label::caption(unit.to_owned()));

        match &field.kind {
            FieldKind::Text => self.input(id, field, value),
            FieldKind::Integer { .. } | FieldKind::Real { .. } => {
                let mut editor = row![self.input(id, field, value)]
                    .spacing(6)
                    .align_y(Center);

                if let Some(unit) = unit {
                    editor = editor.push(unit);
                }

                editor.into()
            }
            FieldKind::Bool => {
                let on_event = self.on_event.clone();

                checkbox(value.as_bool().unwrap_or(false))
                    .label(match value {
                        Value::Bool(true) => "Evet",
                        Value::Bool(false) => "Hayır",
                        _ => "Boş",
                    })
                    .size(13.0)
                    .text_size(typography::BODY)
                    .on_toggle(move |checked| {
                        on_event(Event::Set {
                            id,
                            value: Value::Bool(checked),
                        })
                    })
                    .into()
            }
            FieldKind::Choice(options) => {
                let on_event = self.on_event.clone();
                let mut choices: Vec<String> = Vec::with_capacity(options.len() + 1);

                if !field.required {
                    choices.push(EMPTY_CHOICE.to_owned());
                }

                choices.extend(options.iter().cloned());

                let selected = value
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| (!field.required).then(|| EMPTY_CHOICE.to_owned()));

                pick_list(choices, selected, move |choice: String| {
                    on_event(Event::Set {
                        id,
                        value: if choice == EMPTY_CHOICE {
                            Value::Null
                        } else {
                            Value::Text(choice)
                        },
                    })
                })
                .placeholder("Seçin")
                .text_size(typography::BODY)
                .padding([3, 4])
                .width(Fill)
                .style(style::field::cell_pick_list)
                .menu_style(style::field::menu)
                .into()
            }
            FieldKind::Range { min, max, step } => {
                let on_event = self.on_event.clone();
                let current = value.as_f64().unwrap_or(*min);
                let shown = if value.is_null() {
                    EMPTY_CHOICE.to_owned()
                } else {
                    field.format_with_unit(value)
                };

                row![
                    slider(*min..=*max, current, move |amount| {
                        on_event(Event::Set {
                            id,
                            value: Value::Real(amount),
                        })
                    })
                    .step(*step),
                    label::mono_caption(shown)
                        .style(style::text::default)
                        .width(72)
                        .align_x(Right),
                ]
                .spacing(8)
                .align_y(Center)
                .into()
            }
            FieldKind::Date | FieldKind::DateTime => {
                let mut editor = row![
                    self.input(id, field, value),
                    self.cell_button(Icon::Calendar, "Takvim", Event::Expand(Some(id)), expanded),
                ]
                .spacing(2)
                .align_y(Center);

                if field.kind == FieldKind::DateTime {
                    editor = editor.push(self.cell_button(
                        Icon::Clock,
                        "Şimdi",
                        Event::Set {
                            id,
                            value: Value::DateTime(self.now),
                        },
                        false,
                    ));
                }

                editor.into()
            }
            FieldKind::Time => row![
                self.input(id, field, value),
                self.cell_button(
                    Icon::Clock,
                    "Şimdi",
                    Event::Set {
                        id,
                        value: Value::Time(self.now.time),
                    },
                    false,
                ),
            ]
            .spacing(2)
            .align_y(Center)
            .into(),
            FieldKind::Object { .. } => self.object_editor(id, field, value, candidates, pickable),
        }
    }

    fn object_editor(
        &self,
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: &[(ObjectId, String)],
        pickable: bool,
    ) -> Element<'a, Message> {
        let picking = self.state.picking == Some(id);
        let selected = value.as_object();

        let caption: Element<'a, Message> = match (picking, selected) {
            (true, _) => label::body("Haritada seçin")
                .style(style::text::accent)
                .width(Fill)
                .into(),
            (false, Some(object)) => label::body(object_label(candidates, object))
                .width(Fill)
                .into(),
            (false, None) => label::muted("Seçilmedi").width(Fill).into(),
        };

        let mut editor = row![
            button(
                row![
                    caption,
                    icon(Icon::ChevronDown).size(10.0).tone(Tone::Muted)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .on_press(self.emit(Event::Expand(Some(id))))
            .padding([3, 4])
            .width(Fill)
            .style(style::button::flat),
        ]
        .spacing(2)
        .align_y(Center);

        if pickable {
            editor = editor.push(self.cell_button(
                Icon::Target,
                "Haritadan seç",
                Event::Pick(id),
                picking,
            ));
        }

        if selected.is_some() && !field.required {
            editor = editor.push(
                button(icon(Icon::Close).size(12.0))
                    .on_press(self.emit(Event::Set {
                        id,
                        value: Value::Null,
                    }))
                    .padding([3, 4])
                    .style(style::button::subtle),
            );
        }

        editor.into()
    }

    /// Nesne seçicinin açılan listesi: arama ve eşleşen adaylar.
    fn candidate_list(
        &self,
        id: PropertyId,
        value: &Value,
        candidates: &[(ObjectId, String)],
    ) -> Element<'a, Message> {
        let on_event = self.on_event.clone();
        let search = self.state.search.trim();
        let selected = value.as_object();

        let matches: Vec<&(ObjectId, String)> = candidates
            .iter()
            .filter(|(_, name)| search.is_empty() || crate::attribute::text::contains(name, search))
            .collect();

        let mut list = Column::new().spacing(1);

        for (object, name) in matches.iter().take(MAX_CANDIDATES) {
            list = list.push(
                button(
                    row![
                        label::body(name.clone()).width(Fill),
                        label::mono_caption(format!("#{object}")),
                    ]
                    .align_y(Center),
                )
                .on_press(self.emit(Event::Set {
                    id,
                    value: Value::Object(*object),
                }))
                .width(Fill)
                .padding([3, 6])
                .style(style::button::list_item(selected == Some(*object))),
            );
        }

        let footer = match matches.len() {
            0 => Some("Eşleşen nesne yok.".to_owned()),
            count if count > MAX_CANDIDATES => Some(format!(
                "{} sonuç daha; aramayı daraltın.",
                count - MAX_CANDIDATES
            )),
            _ => None,
        };

        let mut content = column![
            text_input("Ara", &self.state.search)
                .on_input(move |search| on_event(Event::Search(search)))
                .size(typography::BODY)
                .padding([3, 6])
                .style(style::field::input),
            list,
        ]
        .spacing(6);

        if let Some(footer) = footer {
            content = content.push(label::caption(footer));
        }

        content.into()
    }

    /// Açılan takvim: ay başlığı, gün adları ve altı haftalık gün ızgarası.
    fn calendar(&self, id: PropertyId, field: &Field, value: &Value) -> Element<'a, Message> {
        let selected = match value {
            Value::Date(date) => Some(*date),
            Value::DateTime(moment) => Some(moment.date),
            _ => None,
        };

        let time = match value {
            Value::DateTime(moment) => moment.time,
            _ => Time::MIDNIGHT,
        };

        let value_for = |date: Date| match field.kind {
            FieldKind::DateTime => Value::DateTime(DateTime::new(date, time)),
            _ => Value::Date(date),
        };

        let shown = self
            .state
            .month
            .unwrap_or_else(|| selected.unwrap_or(self.now.date))
            .first_of_month();

        let header = row![
            button(icon(Icon::ChevronLeft).size(14.0))
                .on_press(self.emit(Event::Month(shown.add_months(-1))))
                .padding([3, 4])
                .style(style::button::flat),
            container(label::strong(shown.month_title()))
                .width(Fill)
                .center_x(Fill),
            button(icon(Icon::ChevronRight).size(14.0))
                .on_press(self.emit(Event::Month(shown.add_months(1))))
                .padding([3, 4])
                .style(style::button::flat),
            button(label::body("Bugün"))
                .on_press(self.emit(Event::Set {
                    id,
                    value: match field.kind {
                        FieldKind::DateTime => Value::DateTime(self.now),
                        _ => Value::Date(self.now.date),
                    },
                }))
                .padding([2, 8])
                .style(style::button::secondary),
        ]
        .spacing(4)
        .align_y(Center);

        let weekdays = row(WEEKDAYS.map(|day| {
            container(label::caption(day))
                .width(DAY_WIDTH)
                .center_x(DAY_WIDTH)
                .into()
        }))
        .spacing(2);

        let start = shown.add_days(-i64::from(shown.weekday()));

        let weeks = Column::with_children((0..6).map(|week| {
            row((0..7).map(|weekday| {
                let day = start.add_days(week * 7 + weekday);
                let in_month = day.month() == shown.month();
                let is_selected = selected == Some(day);
                let is_today = day == self.now.date;

                let number = text(day.day().to_string()).size(typography::BODY);
                let number = if in_month || is_selected {
                    number
                } else {
                    number.style(style::text::disabled)
                };

                let cell = button(container(number).center_x(Fill))
                    .on_press(self.emit(Event::Set {
                        id,
                        value: value_for(day),
                    }))
                    .width(DAY_WIDTH)
                    .padding([3, 0]);

                if is_selected {
                    cell.style(style::button::segment(true)).into()
                } else if is_today {
                    cell.style(style::button::tool(true)).into()
                } else {
                    cell.style(style::button::flat).into()
                }
            }))
            .spacing(2)
            .into()
        }))
        .spacing(2);

        column![header, weekdays, weeks]
            .spacing(6)
            .max_width(DAY_WIDTH * 7.0 + 12.0 + 120.0)
            .into()
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
        _ => field.format(value),
    }
}

fn object_label(candidates: &[(ObjectId, String)], object: ObjectId) -> String {
    candidates
        .iter()
        .find(|(candidate, _)| *candidate == object)
        .map_or_else(|| format!("#{object}"), |(_, name)| name.clone())
}

fn key<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(KEY_WIDTH)
        .height(ROW_HEIGHT)
        .padding([0, 8])
        .align_y(Center)
        .style(style::container::surface)
        .into()
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

        for entry in &inspector.entries {
            match entry {
                Entry::Category(title) => rows.push(
                    container(
                        row![
                            icon(Icon::ChevronDown).size(10.0).tone(Tone::Muted),
                            label::caption(title.clone())
                                .font(typography::UI_STRONG)
                                .style(style::text::default),
                        ]
                        .spacing(6)
                        .align_y(Center),
                    )
                    .padding([3, 8])
                    .width(Fill)
                    .style(style::container::header)
                    .into(),
                ),
                Entry::Fixed {
                    label: name,
                    value,
                    mono,
                } => {
                    let value = if *mono {
                        label::mono(value.clone())
                    } else {
                        label::body(value.clone())
                    };

                    rows.push(
                        row![
                            key(label::muted(name.clone())),
                            cell(container(value).padding([0, 4]))
                        ]
                        .spacing(1)
                        .into(),
                    );
                }
                Entry::Field {
                    id,
                    field,
                    value,
                    candidates,
                    pickable,
                } => {
                    let name = if field.required && field.editable {
                        format!("{} *", field.name)
                    } else {
                        field.name.clone()
                    };

                    let name = if field.editable {
                        label::body(name)
                    } else {
                        label::muted(name)
                    };

                    rows.push(
                        row![
                            key(name),
                            cell(inspector.editor(*id, field, value, candidates, *pickable))
                        ]
                        .spacing(1)
                        .into(),
                    );

                    if inspector.state.expanded == Some(*id) && field.editable {
                        match field.kind {
                            FieldKind::Date | FieldKind::DateTime => {
                                rows.push(full_width(inspector.calendar(*id, field, value)));
                            }
                            FieldKind::Object { .. } => {
                                rows.push(full_width(
                                    inspector.candidate_list(*id, value, candidates),
                                ));
                            }
                            _ => {}
                        }
                    }

                    if let Some(error) = inspector
                        .state
                        .drafts
                        .get(id)
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
            }
        }

        container(Column::with_children(rows).spacing(1))
            .padding([1, 0])
            .width(Fill)
            .style(style::container::grid_lines)
            .into()
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
    fn expanding_toggles_and_picking_can_be_cancelled() {
        let mut state = State::new();

        state.update(Event::Expand(Some(2)));
        assert_eq!(state.expanded, Some(2));
        state.update(Event::Expand(Some(2)));
        assert_eq!(state.expanded, None);

        assert_eq!(state.update(Event::Pick(4)), Some(Action::Pick(4)));
        assert_eq!(state.picking(), Some(4));
        assert_eq!(state.update(Event::Pick(4)), Some(Action::CancelPick));
        assert_eq!(state.picking(), None);
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

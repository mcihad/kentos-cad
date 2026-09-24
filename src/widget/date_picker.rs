//! Tarih ve saat seçicileri: alanın yanındaki düğmeyle açılan takvim ve saat
//! panelleri.
//!
//! ```text
//! [24.09.2026 14:30        ][▦]
//! ┌──────────────────────────────────────────────────────┐
//! │ ‹      Eylül 2026 ▾      ›  │  14:30                  │
//! │ Hf Pt Sa Ça Pe Cu Ct Pz     │  Saat                   │
//! │ 36 31  1  2  3  4  5  6     │  00 01 02 03 04 05      │
//! │ 37  7  8  9 10 11 12 13     │  …                      │
//! │ 38 14 15 16 17 18 19 20     │  Dakika                 │
//! │ 39 21 22 23[24]25 26 27     │  00 05 10 15 20 25      │
//! │ 40 28 29 30  1  2  3  4     │  30 35 40 45 50 55      │
//! │ [Şimdi] [Temizle]                            [Tamam]  │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! [`DatePicker`] tarih ya da tarih ve saat seçer. Başlığa tıklamak ay,
//! sonra yıl görünümüne geçer; oklar görünüme göre ay, yıl ya da on yıl
//! ilerler. Hafta numaraları ISO 8601'e göredir; bugün kenarla, hafta sonu
//! sönük gösterilir. [`TimePicker`] saat seçer: önce saat, sonra dakika.
//!
//! Alan yazılabilir kalabilir: seçici uygulamanın kendi metin girişine
//! eklenir ([`DatePicker::anchor`]); verilmezse değer salt gösterilir ve
//! alana tıklamak paneli açar.
//!
//! ```ignore
//! DatePicker::date(value, Message::DateChanged)
//!     .anchor(text_input("GG.AA.YYYY", &draft).on_input(Message::DateTyped))
//!     .now(DateTime::now(180))
//! ```
//!
//! Panelde gezinmek uygulamaya mesaj üretmez; yalnızca seçilen değer
//! bildirilir.

use std::rc::Rc;

use iced::widget::{Column, Row, button, column, container, row, rule, space};
use iced::{Center, Element, Fill, Length};

use crate::attribute::time::{MONTHS_SHORT, WEEKDAYS};
use crate::attribute::{Date, DateTime, Time};
use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::style::button::CellTone;
use crate::theme::typography;
use crate::widget::dropdown::{Dropdown, Reaction};

// Hücre ölçüleri 12 piksellik gövde metninde tasarlandı ve yazı boyutuyla
// büyür; aralarındaki boşluk sabittir.

/// Takvim hücresi.
fn day_width() -> f32 {
    typography::scaled(30.0)
}

fn day_height() -> f32 {
    typography::scaled(26.0)
}

/// Hafta numarası sütunu.
fn week_width() -> f32 {
    typography::scaled(24.0)
}

/// Saat ve dakika hücresi.
fn clock_width() -> f32 {
    typography::scaled(30.0)
}

fn clock_height() -> f32 {
    typography::scaled(24.0)
}

/// Ay ve yıl hücrelerinin yüksekliği.
fn wide_cell_height() -> f32 {
    typography::scaled(40.0)
}

/// Hücreler arası boşluk.
const GAP: f32 = 2.0;
/// Dakika ızgarasının adımı.
const MINUTE_STEP: u8 = 5;

/// Seçicinin türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Date,
    DateTime,
    Time,
}

/// Takvimin görünümü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Days,
    Months,
    Years,
}

/// Panelin iç mesajları.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Pick {
    Previous,
    Next,
    /// Başlık: gün → ay → yıl görünümü.
    Zoom,
    Day(Date),
    Month(u8),
    Year(i32),
    Hour(u8),
    Minute(u8),
    /// Bugün ya da şimdi.
    Now,
    Clear,
    Done,
}

/// Açık panelin durumu.
struct State {
    /// Gösterilen ayın ilk günü.
    shown: Date,
    view: View,
    /// Paneldeki seçim; tarih-saatte saat değiştirilirken korunur.
    selection: Option<DateTime>,
}

/// Seçilen değeri bildiren fonksiyon; tarih ve saat seçicilerinde değer
/// yayınlanırken dönüştürülür.
type OnChange<'a, Message> = Rc<dyn Fn(Option<DateTime>) -> Message + 'a>;

/// Tarih ya da tarih ve saat seçici.
pub struct DatePicker<'a, Message> {
    mode: Mode,
    value: Option<DateTime>,
    on_change: OnChange<'a, Message>,
    anchor: Option<Element<'a, Message>>,
    now: DateTime,
    week_numbers: bool,
    clearable: bool,
}

impl<'a, Message: 'a> DatePicker<'a, Message> {
    fn with_mode(mode: Mode, value: Option<DateTime>, on_change: OnChange<'a, Message>) -> Self {
        Self {
            mode,
            value,
            on_change,
            anchor: None,
            now: DateTime::now(0),
            week_numbers: true,
            clearable: true,
        }
    }

    /// Tarih seçici; gün seçilince panel kapanır.
    pub fn date(value: Option<Date>, on_change: impl Fn(Option<Date>) -> Message + 'a) -> Self {
        Self::with_mode(
            Mode::Date,
            value.map(|date| DateTime::new(date, Time::MIDNIGHT)),
            Rc::new(move |value: Option<DateTime>| on_change(value.map(|value| value.date))),
        )
    }

    /// Tarih ve saat seçici; takvim ve saat ızgarası yan yana. Seçim anında
    /// bildirilir, panel "Tamam" ya da dışarı tıklamayla kapanır.
    pub fn date_time(
        value: Option<DateTime>,
        on_change: impl Fn(Option<DateTime>) -> Message + 'a,
    ) -> Self {
        Self::with_mode(Mode::DateTime, value, Rc::new(on_change))
    }

    /// Yazılabilir alan (ör. uygulamanın metin girişi); seçici yanına bir
    /// düğme olarak eklenir. Verilmezse değer salt gösterilir.
    pub fn anchor(mut self, anchor: impl Into<Element<'a, Message>>) -> Self {
        self.anchor = Some(anchor.into());
        self
    }

    /// "Bugün" ve "Şimdi"nin yerel zamanı (varsayılan UTC).
    pub fn now(mut self, now: DateTime) -> Self {
        self.now = now;
        self
    }

    /// ISO hafta numaralarını gösterir (varsayılan: gösterir).
    pub fn week_numbers(mut self, show: bool) -> Self {
        self.week_numbers = show;
        self
    }

    /// "Temizle" düğmesi; zorunlu alanlarda kapatılır (varsayılan: açık).
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }
}

/// Saat seçici: önce saat, sonra dakika seçilir; dakika seçilince panel
/// kapanır.
pub struct TimePicker<'a, Message>(DatePicker<'a, Message>);

impl<'a, Message: 'a> TimePicker<'a, Message> {
    pub fn new(value: Option<Time>, on_change: impl Fn(Option<Time>) -> Message + 'a) -> Self {
        Self(DatePicker::with_mode(
            Mode::Time,
            value.map(|time| DateTime::new(Date::from_days_since_epoch(0), time)),
            Rc::new(move |value: Option<DateTime>| on_change(value.map(|value| value.time))),
        ))
    }

    /// Yazılabilir alan; seçici yanına bir düğme olarak eklenir.
    pub fn anchor(self, anchor: impl Into<Element<'a, Message>>) -> Self {
        Self(self.0.anchor(anchor))
    }

    /// "Şimdi"nin yerel zamanı.
    pub fn now(self, now: DateTime) -> Self {
        Self(self.0.now(now))
    }

    pub fn clearable(self, clearable: bool) -> Self {
        Self(self.0.clearable(clearable))
    }
}

impl<'a, Message: 'a> From<TimePicker<'a, Message>> for Element<'a, Message> {
    fn from(picker: TimePicker<'a, Message>) -> Self {
        picker.0.into()
    }
}

/// Seçicinin değişmeyen ayarları; panel ve iç mesajlar için paylaşılır.
#[derive(Clone, Copy)]
struct Props {
    mode: Mode,
    value: Option<DateTime>,
    now: DateTime,
    week_numbers: bool,
    clearable: bool,
}

impl<'a, Message: 'a> From<DatePicker<'a, Message>> for Element<'a, Message> {
    fn from(picker: DatePicker<'a, Message>) -> Self {
        let props = Props {
            mode: picker.mode,
            value: picker.value,
            now: picker.now,
            week_numbers: picker.week_numbers,
            clearable: picker.clearable,
        };
        let on_change = picker.on_change;
        let glyph = match props.mode {
            Mode::Time => Icon::Clock,
            Mode::Date | Mode::DateTime => Icon::Calendar,
        };

        let init = move || State {
            shown: props
                .value
                .filter(|_| props.mode != Mode::Time)
                .map_or(props.now.date, |value| value.date)
                .first_of_month(),
            view: View::Days,
            selection: props.value,
        };

        let reduce = move |state: &mut State, pick: Pick| reduce(state, pick, props, &on_change);

        let dropdown = match picker.anchor {
            Some(anchor) => Dropdown::new(anchor, init, move |state| panel(state, props), reduce)
                .trigger(
                    button(icon(glyph).size(14.0))
                        .on_press(())
                        .padding([3, 4])
                        .style(style::button::flat),
                ),
            None => Dropdown::new(
                display(props, glyph),
                init,
                move |state| panel(state, props),
                reduce,
            ),
        };

        dropdown.into()
    }
}

/// Alan verilmediğinde değeri gösteren, tıklanınca paneli açan kutu.
fn display<'a, Message: 'a>(props: Props, glyph: Icon) -> Element<'a, Message> {
    let content = match (props.value, props.mode) {
        (Some(value), Mode::Date) => label::mono(value.date.to_string()),
        (Some(value), Mode::DateTime) => label::mono(value.to_string()),
        (Some(value), Mode::Time) => label::mono(value.time.to_string()),
        (None, _) => label::muted("Seçin"),
    };

    container(
        row![
            container(content).width(Fill),
            icon(glyph).size(14.0).tone(Tone::Muted)
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([3, 6])
    .width(Fill)
    .style(style::container::field)
    .into()
}

/// İç mesajı uygular; seçilen değer uygulamaya bildirilir.
fn reduce<'a, Message>(
    state: &mut State,
    pick: Pick,
    props: Props,
    on_change: &OnChange<'a, Message>,
) -> Reaction<Message> {
    let emit = |value: Option<DateTime>| on_change(value);

    match pick {
        Pick::Previous | Pick::Next => {
            let direction = if pick == Pick::Next { 1 } else { -1 };

            state.shown = match state.view {
                View::Days => state.shown.add_months(direction),
                View::Months => state.shown.add_months(12 * direction),
                View::Years => state.shown.add_months(120 * direction),
            };

            Reaction::stay()
        }
        Pick::Zoom => {
            state.view = match state.view {
                View::Days => View::Months,
                View::Months | View::Years => View::Years,
            };

            Reaction::stay()
        }
        Pick::Month(month) => {
            state.shown = Date::new(state.shown.year(), month, 1).unwrap_or(state.shown);
            state.view = View::Days;

            Reaction::stay()
        }
        Pick::Year(year) => {
            state.shown = Date::new(year, state.shown.month(), 1).unwrap_or(state.shown);
            state.view = View::Months;

            Reaction::stay()
        }
        Pick::Day(day) => {
            state.shown = day.first_of_month();

            let time = state
                .selection
                .map_or(Time::MIDNIGHT, |selection| selection.time);
            let selection = DateTime::new(day, time);
            state.selection = Some(selection);

            match props.mode {
                Mode::Date => Reaction::commit(emit(Some(selection))),
                Mode::DateTime => Reaction::publish(emit(Some(selection))),
                Mode::Time => Reaction::stay(),
            }
        }
        Pick::Hour(hour) | Pick::Minute(hour) => {
            let base = state
                .selection
                .unwrap_or(DateTime::new(props.now.date, Time::MIDNIGHT));

            let time = match pick {
                Pick::Hour(_) => Time::new(hour, base.time.minute(), 0),
                _ => Time::new(base.time.hour(), hour, 0),
            }
            .unwrap_or(base.time);

            let selection = DateTime::new(base.date, time);
            state.selection = Some(selection);

            // Saat seçicide dakika seçimi son adımdır.
            if props.mode == Mode::Time && matches!(pick, Pick::Minute(_)) {
                Reaction::commit(emit(Some(selection)))
            } else {
                Reaction::publish(emit(Some(selection)))
            }
        }
        Pick::Now => {
            let now = match props.mode {
                Mode::Date => DateTime::new(props.now.date, Time::MIDNIGHT),
                Mode::DateTime | Mode::Time => props.now,
            };

            Reaction::commit(emit(Some(now)))
        }
        Pick::Clear => Reaction::commit(emit(None)),
        Pick::Done => Reaction::close(),
    }
}

/// Takvimin genişliği: hafta numaraları ve yedi gün.
fn calendar_width(props: Props) -> f32 {
    let week_column = if props.week_numbers {
        week_width() + GAP
    } else {
        0.0
    };

    week_column + day_width() * 7.0 + GAP * 6.0
}

/// Saat ızgarasının genişliği: altı sütun.
fn clock_section() -> f32 {
    clock_width() * 6.0 + GAP * 5.0
}

/// Takvimle saat ızgarası arasındaki boşluk.
const SECTION_GAP: f32 = 20.0;

/// Açık panel: takvim, saat ızgarası ve alt düğmeler.
fn panel<'a>(state: &State, props: Props) -> Element<'a, Pick> {
    // Panelin genişliği açıkça verilir: esnek öğeler (çizgi, boşluk)
    // paneli pencere boyunca yaymasın.
    let width = match props.mode {
        Mode::Date => calendar_width(props),
        Mode::DateTime => calendar_width(props) + SECTION_GAP + clock_section(),
        Mode::Time => clock_section(),
    };

    let mut sections = Row::new().spacing(SECTION_GAP);

    if props.mode != Mode::Time {
        sections = sections.push(calendar(state, props));
    }

    if props.mode != Mode::Date {
        sections = sections.push(clock(state, props));
    }

    container(
        column![
            sections,
            rule::horizontal(1).style(style::field::hairline),
            footer(props)
        ]
        .spacing(8)
        .width(width),
    )
    .padding(10)
    .style(style::container::popover)
    .into()
}

/// Takvim bölümü: başlık ve görünüme göre gün, ay ya da yıl ızgarası.
fn calendar<'a>(state: &State, props: Props) -> Element<'a, Pick> {
    let width = calendar_width(props);

    let title = match state.view {
        View::Days => state.shown.month_title(),
        View::Months => state.shown.year().to_string(),
        View::Years => {
            let (first, last) = decade(state.shown.year());
            format!("{first} – {last}")
        }
    };

    let header = row![
        nav(Icon::ChevronLeft, Pick::Previous),
        container(
            button(
                row![
                    label::strong(title),
                    icon(Icon::ChevronDown).size(10.0).tone(Tone::Muted)
                ]
                .spacing(4)
                .align_y(Center),
            )
            .on_press_maybe((state.view != View::Years).then_some(Pick::Zoom))
            .padding([3, 8])
            .style(style::button::flat),
        )
        .center_x(Fill),
        nav(Icon::ChevronRight, Pick::Next),
    ]
    .align_y(Center)
    .width(width);

    let body = match state.view {
        View::Days => days(state, props),
        View::Months => months(state, props, width),
        View::Years => years(state, props, width),
    };

    column![header, body].spacing(6).width(width).into()
}

fn nav<'a>(glyph: Icon, pick: Pick) -> Element<'a, Pick> {
    button(icon(glyph).size(14.0))
        .on_press(pick)
        .padding([4, 6])
        .style(style::button::flat)
        .into()
}

/// Gün ızgarası: haftanın günleri, isteğe bağlı hafta numaraları ve altı
/// hafta.
fn days<'a>(state: &State, props: Props) -> Element<'a, Pick> {
    let shown = state.shown;
    let selected = state.selection.map(|selection| selection.date);
    let start = shown.add_days(-i64::from(shown.weekday()));

    let mut heading = Row::new().spacing(GAP);

    if props.week_numbers {
        heading = heading.push(
            container(label::caption("Hf"))
                .width(week_width())
                .center_x(week_width()),
        );
    }

    heading = heading.extend(WEEKDAYS.into_iter().enumerate().map(|(index, day)| {
        let name = label::caption(day);
        let name = if index >= 5 {
            name
        } else {
            name.style(style::text::default)
        };

        container(name)
            .width(day_width())
            .center_x(day_width())
            .into()
    }));

    let weeks = Column::with_children((0..6).map(|week| {
        let first = start.add_days(week * 7);
        let mut line = Row::new().spacing(GAP).align_y(Center);

        if props.week_numbers {
            line = line.push(
                container(label::mono_caption(first.iso_week().to_string()))
                    .width(week_width())
                    .center_x(week_width()),
            );
        }

        line.extend((0..7).map(|weekday| {
            let day = first.add_days(weekday);
            let tone = if day.month() != shown.month() {
                CellTone::Outside
            } else if day.is_weekend() {
                CellTone::Weekend
            } else {
                CellTone::Normal
            };

            cell(
                day.day().to_string(),
                Pick::Day(day),
                selected == Some(day),
                day == props.now.date,
                tone,
                day_width(),
                day_height(),
            )
        }))
        .into()
    }))
    .spacing(GAP);

    column![heading, weeks].spacing(4).into()
}

/// Ay ızgarası: dört satırda üçer ay.
fn months<'a>(state: &State, props: Props, width: f32) -> Element<'a, Pick> {
    let year = state.shown.year();
    let selected = state
        .selection
        .filter(|selection| selection.date.year() == year)
        .map(|selection| selection.date.month());
    let current = (props.now.date.year() == year).then(|| props.now.date.month());
    let cell_width = (width - GAP * 2.0) / 3.0;

    Column::with_children((0..4).map(|line| {
        Row::with_children((0..3).map(|column| {
            let month = (line * 3 + column + 1) as u8;

            cell(
                MONTHS_SHORT[usize::from(month) - 1].to_owned(),
                Pick::Month(month),
                selected == Some(month),
                current == Some(month),
                CellTone::Normal,
                cell_width,
                wide_cell_height(),
            )
        }))
        .spacing(GAP)
        .into()
    }))
    .spacing(GAP)
    .into()
}

/// On yıl ızgarası: on yıl ve iki yanındaki birer yıl.
fn years<'a>(state: &State, props: Props, width: f32) -> Element<'a, Pick> {
    let (first, last) = decade(state.shown.year());
    let selected = state.selection.map(|selection| selection.date.year());
    let cell_width = (width - GAP * 2.0) / 3.0;

    Column::with_children((0..4).map(|line| {
        Row::with_children((0..3).map(|column| {
            let year = first - 1 + line * 3 + column;
            let tone = if (first..=last).contains(&year) {
                CellTone::Normal
            } else {
                CellTone::Outside
            };

            cell(
                year.to_string(),
                Pick::Year(year),
                selected == Some(year),
                props.now.date.year() == year,
                tone,
                cell_width,
                wide_cell_height(),
            )
        }))
        .spacing(GAP)
        .into()
    }))
    .spacing(GAP)
    .into()
}

/// Yılın on yılı: 2026 → 2020–2029.
fn decade(year: i32) -> (i32, i32) {
    let first = year - year.rem_euclid(10);
    (first, first + 9)
}

/// Saat bölümü: seçili saat, saat ve dakika ızgaraları.
fn clock<'a>(state: &State, props: Props) -> Element<'a, Pick> {
    let selected = state.selection.map(|selection| selection.time);
    let readout = selected.map_or_else(
        || "--:--".to_owned(),
        |time| format!("{:02}:{:02}", time.hour(), time.minute()),
    );

    let hours = Column::with_children((0..4).map(|line| {
        Row::with_children((0..6).map(|column| {
            let hour = (line * 6 + column) as u8;

            cell(
                format!("{hour:02}"),
                Pick::Hour(hour),
                selected.is_some_and(|time| time.hour() == hour),
                props.now.time.hour() == hour,
                CellTone::Normal,
                clock_width(),
                clock_height(),
            )
        }))
        .spacing(GAP)
        .into()
    }))
    .spacing(GAP);

    let minutes = Column::with_children((0..2).map(|line| {
        Row::with_children((0..6).map(|column| {
            let minute = (line * 6 + column) as u8 * MINUTE_STEP;

            cell(
                format!("{minute:02}"),
                Pick::Minute(minute),
                selected.is_some_and(|time| time.minute() == minute),
                props.now.time.minute() / MINUTE_STEP * MINUTE_STEP == minute,
                CellTone::Normal,
                clock_width(),
                clock_height(),
            )
        }))
        .spacing(GAP)
        .into()
    }))
    .spacing(GAP);

    column![
        container(label::figure(readout).style(style::text::default)).center_x(Fill),
        label::caption("Saat"),
        hours,
        label::caption("Dakika"),
        minutes,
    ]
    .spacing(4)
    .width(clock_section())
    .into()
}

/// Alt düğmeler: bugün ya da şimdi, temizle ve tarih-saatte tamam.
fn footer<'a>(props: Props) -> Element<'a, Pick> {
    let now = match props.mode {
        Mode::Date => "Bugün",
        Mode::DateTime | Mode::Time => "Şimdi",
    };

    let mut footer = row![small(now, Pick::Now, false)]
        .spacing(6)
        .align_y(Center);

    if props.clearable {
        footer = footer.push(small("Temizle", Pick::Clear, false));
    }

    footer = footer.push(space::horizontal());

    if props.mode != Mode::Date {
        footer = footer.push(small("Tamam", Pick::Done, true));
    }

    footer.into()
}

fn small<'a>(content: &'a str, pick: Pick, primary: bool) -> Element<'a, Pick> {
    button(label::body(content))
        .on_press(pick)
        .padding([3, 10])
        .style(if primary {
            style::button::primary
        } else {
            style::button::secondary
        })
        .into()
}

/// Takvim ya da saat hücresi.
fn cell<'a>(
    content: String,
    pick: Pick,
    selected: bool,
    today: bool,
    tone: CellTone,
    width: f32,
    height: f32,
) -> Element<'a, Pick> {
    button(
        container(label::body(content))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .on_press(pick)
    .width(width)
    .height(height)
    .padding(0)
    .style(style::button::calendar(selected, today, tone))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(mode: Mode) -> Props {
        Props {
            mode,
            value: None,
            now: DateTime::new(
                Date::new(2026, 9, 24).expect("tarih"),
                Time::new(14, 32, 0).expect("saat"),
            ),
            week_numbers: true,
            clearable: true,
        }
    }

    fn september() -> State {
        State {
            shown: Date::new(2026, 9, 1).expect("tarih"),
            view: View::Days,
            selection: None,
        }
    }

    #[test]
    fn navigation_moves_by_view() {
        let on_change: OnChange<'_, Option<DateTime>> = Rc::new(|value| value);
        let mut state = september();

        let _ = reduce(&mut state, Pick::Next, props(Mode::Date), &on_change);
        assert_eq!(state.shown, Date::new(2026, 10, 1).expect("tarih"));

        let _ = reduce(&mut state, Pick::Zoom, props(Mode::Date), &on_change);
        let _ = reduce(&mut state, Pick::Previous, props(Mode::Date), &on_change);
        assert_eq!(state.view, View::Months);
        assert_eq!(state.shown.year(), 2025);

        let _ = reduce(&mut state, Pick::Zoom, props(Mode::Date), &on_change);
        let _ = reduce(&mut state, Pick::Year(2030), props(Mode::Date), &on_change);
        let _ = reduce(&mut state, Pick::Month(2), props(Mode::Date), &on_change);
        assert_eq!(state.view, View::Days);
        assert_eq!(state.shown, Date::new(2030, 2, 1).expect("tarih"));
    }

    #[test]
    fn date_picks_close_and_date_time_picks_keep_the_time() {
        let on_change: OnChange<'_, Option<DateTime>> = Rc::new(|value| value);
        let day = Date::new(2026, 9, 8).expect("tarih");

        let mut state = september();
        let reaction = reduce(&mut state, Pick::Day(day), props(Mode::Date), &on_change);
        assert!(reaction.close);

        let mut state = september();
        state.selection = Some(DateTime::new(day, Time::new(9, 30, 0).expect("saat")));
        let other = Date::new(2026, 9, 10).expect("tarih");
        let reaction = reduce(
            &mut state,
            Pick::Day(other),
            props(Mode::DateTime),
            &on_change,
        );
        assert!(!reaction.close);
        assert_eq!(
            reaction.publish.flatten(),
            Some(DateTime::new(other, Time::new(9, 30, 0).expect("saat")))
        );

        let reaction = reduce(&mut state, Pick::Minute(45), props(Mode::Time), &on_change);
        assert!(reaction.close);
        assert_eq!(
            state.selection.map(|selection| selection.time),
            Time::new(9, 45, 0)
        );
    }

    #[test]
    fn decades_are_aligned() {
        assert_eq!(decade(2026), (2020, 2029));
        assert_eq!(decade(2030), (2030, 2039));
    }
}

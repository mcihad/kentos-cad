//! Zaman çizelgesi: zamanlı veride ya da animasyonda oynatma başı, seçili
//! aralık, işaretler ve oynatma denetimleri.
//!
//! ```text
//! ⏮ ⏪ ▶ ⏩ ⏭  12 Mar 2025 14:30              Aralık ×   ⟲   1× ⌄   ⤢
//! ┌─────────┬─────────┬─────────┬─────────┬─────────┬─────────┐
//! │2025     Şub       Mar       Nis       May       Haz       │  eksen
//! │              ┃▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒┃                      │  aralık
//! │   ◆               ◆     ┃             ◆                   │  işaretler
//! └─────────────────────────┃────────────────────────────────┘
//!                     oynatma başı
//! ```
//!
//! Durum uygulamanındır ([`Playback`]): tüm süre, görünen pencere, oynatma
//! başı, seçili aralık, oynatılıp oynatılmadığı, hız ve döngü. Çizelge her
//! değişikliği bir [`Event`] ile bildirir; uygulama `playback.update(event)`
//! ile uygular, oynatılırken bir zamanlayıcıyla `playback.advance(geçen)`
//! çağırır.
//!
//! - **Eksen.** Tıklamak ya da sürüklemek oynatma başını taşır. Tekerlek
//!   imlecin altındaki ana göre yakınlaştırır, Shift ile kaydırır; ⤢ tümünü
//!   gösterir.
//! - **Aralık.** Aralık şeridinde sürüklemek yeni aralık seçer; uçları ve
//!   gövdesi sürüklenir, boş yere tıklamak aralığı kaldırır. Oynatma aralık
//!   varsa aralıkta, yoksa baştan sona sürer.
//! - **İşaretler.** Olaylar ya da anahtar kareler ([`Marker`]); üzerine
//!   gelince adı yazar, tıklamak oynatma başını oraya taşır.
//! - **Ölçek.** [`Scale::Number`] sayılar (kare, saniye); [`Scale::Calendar`]
//!   Unix saniyesi: yıl, ay, gün, saat ve dakika adımları, Türkçe ay adları.
//!
//! ```ignore
//! Timeline::new(&self.playback, Message::Timeline)
//!     .scale(Scale::Calendar { offset: 180 })
//!     .marker(Marker::new(permit, "Ruhsat"))
//!
//! // update
//! Message::Timeline(event) => self.playback.update(event),
//! Message::Tick(elapsed) => self.playback.advance(elapsed),
//! ```

use std::time::Duration;

use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::text::{self, Renderer as _, Text};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::alignment::Vertical;
use iced::widget::canvas::{self, Path};
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::widget::{button, column, container, row, space, tooltip};
use iced::{
    Background, Border, Center, Color, Element, Event as IcedEvent, Length, Pixels, Point,
    Rectangle, Renderer, Size, Theme, Vector, keyboard, mouse,
};

use crate::attribute::number;
use crate::attribute::time::{Date, DateTime, MONTHS, MONTHS_SHORT};
use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::context_menu::{Menu, MenuButton};
use crate::widget::{Tip, axis, tip};

/// Şeritlerin yükseklikleri ve yatay iç boşluk (12 piksellik gövde metnine
/// göre).
const AXIS: f32 = 24.0;
const RANGE: f32 = 14.0;
const MARKERS: f32 = 20.0;
const PAD: f32 = 10.0;
/// Etiketler ve küçük çizgiler arasında en az bu kadar piksel olur.
const LABEL_SPACING: f64 = 72.0;
const TICK_SPACING: f64 = 5.0;
/// Aralığın uçlarını tutmak için imlecin en fazla uzaklığı.
const EDGE: f32 = 5.0;
/// Yakınlaştırma sınırı: pencere tüm sürenin bu kadarından dar olmaz.
const MIN_WINDOW: f64 = 1e-5;
/// Hız menüsündeki çarpanlar.
const SPEEDS: [f64; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];

/// Eksenin ölçeği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scale {
    /// Sayılar (kare, saniye): 1, 2, 5 × 10ⁿ adımlar.
    #[default]
    Number,
    /// Unix saniyesi (UTC); etiketler yerel saatle, UTC'ye `offset` dakika
    /// eklenerek yazılır (Türkiye: 180).
    Calendar { offset: i32 },
}

/// Oynatma durumu.
#[derive(Debug, Clone, PartialEq)]
pub struct Playback {
    /// Verinin baştan sona süresi.
    pub extent: (f64, f64),
    /// Çizelgede görünen pencere.
    pub window: (f64, f64),
    /// Oynatma başı.
    pub current: f64,
    /// Seçili aralık; oynatma bu aralıkta döner.
    pub range: Option<(f64, f64)>,
    pub playing: bool,
    /// 1× hızda gerçek saniye başına ilerleme ve hız çarpanı.
    pub rate: f64,
    pub speed: f64,
    pub looping: bool,
    /// İleri ve geri düğmelerinin atladığı süre.
    pub step: f64,
}

/// Oynatmada yapılan değişiklik.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// Oynatma başı taşındı.
    Seek(f64),
    Play(bool),
    /// `step` kadar ileri (pozitif) ya da geri adım.
    Step(i32),
    /// Aralığın (yoksa tüm sürenin) başına ya da sonuna.
    Start,
    End,
    /// Aralık seçildi ya da kaldırıldı.
    Range(Option<(f64, f64)>),
    /// Görünen pencere değişti (yakınlaştırma, kaydırma).
    Window(f64, f64),
    Speed(f64),
    Loop(bool),
}

impl Playback {
    /// `extent` süresinde, 1× hızda gerçek saniye başına `rate` ilerleyen
    /// oynatma; oynatma başı baştadır, döngü açıktır.
    pub fn new(extent: (f64, f64), rate: f64) -> Self {
        let extent = (extent.0.min(extent.1), extent.0.max(extent.1));

        Self {
            extent,
            window: extent,
            current: extent.0,
            range: None,
            playing: false,
            rate,
            speed: 1.0,
            looping: true,
            step: (extent.1 - extent.0) / 100.0,
        }
    }

    /// İleri ve geri düğmelerinin adımı.
    pub fn with_step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Oynatmanın sınırları: aralık, yoksa tüm süre.
    pub fn bounds(&self) -> (f64, f64) {
        self.range.unwrap_or(self.extent)
    }

    fn clamp(&self, value: f64) -> f64 {
        value.clamp(self.extent.0, self.extent.1)
    }

    /// Çizelgenin bildirdiği değişikliği uygular.
    pub fn update(&mut self, event: Event) {
        match event {
            Event::Seek(value) => {
                self.current = self.clamp(value);
                self.follow();
            }
            Event::Play(playing) => {
                let (start, end) = self.bounds();

                // Sonda ya da aralığın dışındaysa baştan başlar.
                if playing && (self.current >= end || self.current < start) {
                    self.current = start;
                }

                self.playing = playing;
                self.follow();
            }
            Event::Step(steps) => {
                self.playing = false;
                self.current = self.clamp(self.current + f64::from(steps) * self.step);
                self.follow();
            }
            Event::Start => {
                self.current = self.bounds().0;
                self.follow();
            }
            Event::End => {
                self.playing = false;
                self.current = self.bounds().1;
                self.follow();
            }
            Event::Range(range) => {
                self.range = range.map(|(a, b)| {
                    let (start, end) = (self.clamp(a.min(b)), self.clamp(a.max(b)));
                    (start, end)
                });
            }
            Event::Window(start, end) => self.window = self.fit_window(start, end),
            Event::Speed(speed) => self.speed = speed.max(0.0),
            Event::Loop(looping) => self.looping = looping,
        }
    }

    /// Oynatılıyorsa geçen gerçek süre kadar ilerler. Sona gelince döngüde
    /// başa döner, değilse durur. Oynatma başı pencereden çıkarsa pencere
    /// onu izler.
    pub fn advance(&mut self, elapsed: Duration) {
        if !self.playing {
            return;
        }

        let (start, end) = self.bounds();
        let length = end - start;
        let mut next = self.current + elapsed.as_secs_f64() * self.rate * self.speed;

        if next < start {
            next = start;
        }

        if next >= end {
            if self.looping && length > 0.0 {
                next = start + (next - end) % length;
            } else {
                next = end;
                self.playing = false;
            }
        }

        self.current = next;
        self.follow();
    }

    /// Oynatma başı pencerenin dışındaysa pencere kayar: baş, pencerenin
    /// onda birine gelir.
    fn follow(&mut self) {
        let (start, end) = self.window;
        let span = end - start;

        if self.current < start || self.current > end {
            let from = self.current - span * 0.1;
            self.window = self.fit_window(from, from + span);
        }
    }

    /// Pencereyi tüm sürenin içine sığdırır; çok dar olmasına izin vermez.
    fn fit_window(&self, start: f64, end: f64) -> (f64, f64) {
        let total = self.extent.1 - self.extent.0;
        let span = (end - start)
            .abs()
            .clamp(total * MIN_WINDOW, total.max(f64::EPSILON));
        let start = start.min(end).clamp(self.extent.0, self.extent.1 - span);

        (start, start + span)
    }
}

/// Çizelgedeki işaret: olay ya da anahtar kare.
#[derive(Debug, Clone, PartialEq)]
pub struct Marker {
    pub time: f64,
    pub label: String,
    pub color: Option<Color>,
}

impl Marker {
    pub fn new(time: f64, label: impl Into<String>) -> Self {
        Self {
            time,
            label: label.into(),
            color: None,
        }
    }

    /// Kendi rengi; yoksa vurgu rengi.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// Takvim birimleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Second,
    Minute,
    Hour,
    Day,
    Month,
    Year,
}

impl Unit {
    /// Birimin (yaklaşık) saniyesi.
    fn seconds(self) -> f64 {
        match self {
            Unit::Second => 1.0,
            Unit::Minute => 60.0,
            Unit::Hour => 3_600.0,
            Unit::Day => 86_400.0,
            Unit::Month => 2_629_746.0,
            Unit::Year => 31_556_952.0,
        }
    }
}

/// Takvim adımı: birim ve kaç birim.
type Step = (Unit, i64);

/// Takvim adımları: etiketli adım ve aradaki küçük adım.
const CALENDAR: [(Step, Option<Step>); 22] = [
    ((Unit::Second, 1), None),
    ((Unit::Second, 5), Some((Unit::Second, 1))),
    ((Unit::Second, 15), Some((Unit::Second, 5))),
    ((Unit::Second, 30), Some((Unit::Second, 5))),
    ((Unit::Minute, 1), Some((Unit::Second, 15))),
    ((Unit::Minute, 5), Some((Unit::Minute, 1))),
    ((Unit::Minute, 15), Some((Unit::Minute, 5))),
    ((Unit::Minute, 30), Some((Unit::Minute, 5))),
    ((Unit::Hour, 1), Some((Unit::Minute, 15))),
    ((Unit::Hour, 3), Some((Unit::Hour, 1))),
    ((Unit::Hour, 6), Some((Unit::Hour, 1))),
    ((Unit::Hour, 12), Some((Unit::Hour, 3))),
    ((Unit::Day, 1), Some((Unit::Hour, 6))),
    ((Unit::Day, 2), Some((Unit::Hour, 12))),
    ((Unit::Day, 7), Some((Unit::Day, 1))),
    ((Unit::Month, 1), Some((Unit::Day, 7))),
    ((Unit::Month, 3), Some((Unit::Month, 1))),
    ((Unit::Month, 6), Some((Unit::Month, 1))),
    ((Unit::Year, 1), Some((Unit::Month, 1))),
    ((Unit::Year, 2), Some((Unit::Month, 3))),
    ((Unit::Year, 5), Some((Unit::Year, 1))),
    ((Unit::Year, 10), Some((Unit::Year, 1))),
];

/// Eksenin çizgileri: etiketli çizgilerin ve küçük çizgilerin anları
/// (ölçeğin biriminde) ve etiketleri.
#[derive(Debug, Clone, PartialEq)]
struct Ticks {
    major: Vec<(f64, String)>,
    minor: Vec<f64>,
}

/// Sayı ölçeğinin çizgileri.
fn number_ticks(window: (f64, f64), pixels_per_unit: f64) -> Ticks {
    let (major, parts) = axis::nice(pixels_per_unit, LABEL_SPACING, TICK_SPACING);
    let decimals = axis::decimals(major);
    let step = major / f64::from(parts);
    let first = (window.0 / step).floor() as i64;
    let last = (window.1 / step).ceil() as i64;
    let mut ticks = Ticks {
        major: Vec::new(),
        minor: Vec::new(),
    };

    for index in first..=last {
        let value = index as f64 * step;

        if index.rem_euclid(i64::from(parts)) == 0 {
            ticks.major.push((value, number::real(value, decimals)));
        } else {
            ticks.minor.push(value);
        }
    }

    ticks
}

/// Takvim ölçeğinin etiketli adımı: etiketler en az `LABEL_SPACING` piksel
/// aralıklı olacak en küçük adım.
fn calendar_step(pixels_per_second: f64) -> (Step, Option<Step>) {
    CALENDAR
        .iter()
        .copied()
        .find(|((unit, count), _)| {
            unit.seconds() * *count as f64 * pixels_per_second >= LABEL_SPACING
        })
        .unwrap_or(CALENDAR[CALENDAR.len() - 1])
}

/// Yerel saniyelerde `from`–`to` arasına düşen, `unit` × `count` adımlı
/// anlar; ay ve yıl adımları takvime, haftalar pazartesiye oturur.
fn calendar_instants(from: f64, to: f64, (unit, count): Step) -> Vec<i64> {
    let mut instants = Vec::new();
    let (from, to) = (from.floor() as i64, to.ceil() as i64);

    match unit {
        Unit::Month | Unit::Year => {
            let months = if unit == Unit::Year {
                count * 12
            } else {
                count
            };
            let start = DateTime::from_unix(from).date;
            let index = i64::from(start.year()) * 12 + i64::from(start.month()) - 1;
            let mut index = index - index.rem_euclid(months);

            loop {
                let (year, month) = (index.div_euclid(12), index.rem_euclid(12) + 1);
                let Some(date) = Date::new(year as i32, month as u8, 1) else {
                    break;
                };
                let instant = date.days_since_epoch() * 86_400;

                if instant > to {
                    break;
                }

                if instant >= from {
                    instants.push(instant);
                }

                index += months;
            }
        }
        _ => {
            let step = (unit.seconds() as i64 * count).max(1);
            // Haftalar pazartesiye oturur: 1 Ocak 1970 perşembeydi.
            let shift = if unit == Unit::Day && count == 7 {
                3 * 86_400
            } else {
                0
            };
            let mut instant = (from + shift).div_euclid(step) * step - shift;

            while instant <= to {
                if instant >= from {
                    instants.push(instant);
                }

                instant += step;
            }
        }
    }

    instants
}

/// Takvim çizgisinin etiketi.
fn calendar_label(instant: i64, unit: Unit) -> String {
    let moment = DateTime::from_unix(instant);
    let (date, time) = (moment.date, moment.time);
    let day_month = || {
        format!(
            "{} {}",
            date.day(),
            MONTHS_SHORT[usize::from(date.month()) - 1]
        )
    };

    match unit {
        Unit::Year => date.year().to_string(),
        Unit::Month if date.month() == 1 => date.year().to_string(),
        Unit::Month => MONTHS_SHORT[usize::from(date.month()) - 1].to_owned(),
        Unit::Day => day_month(),
        Unit::Hour | Unit::Minute if time.hour() == 0 && time.minute() == 0 => day_month(),
        Unit::Hour | Unit::Minute => format!("{:02}:{:02}", time.hour(), time.minute()),
        Unit::Second => format!(
            "{:02}:{:02}:{:02}",
            time.hour(),
            time.minute(),
            time.second()
        ),
    }
}

/// Takvim ölçeğinin çizgileri; anlar UTC saniyesidir.
fn calendar_ticks(window: (f64, f64), pixels_per_second: f64, offset: i32) -> Ticks {
    let shift = f64::from(offset) * 60.0;
    let (major, minor) = calendar_step(pixels_per_second);
    let (from, to) = (window.0 + shift, window.1 + shift);

    let majors = calendar_instants(from, to, major);
    let minors = minor
        .filter(|(unit, count)| unit.seconds() * *count as f64 * pixels_per_second >= TICK_SPACING)
        .map(|minor| calendar_instants(from, to, minor))
        .unwrap_or_default();

    Ticks {
        minor: minors
            .into_iter()
            .filter(|instant| !majors.contains(instant))
            .map(|instant| instant as f64 - shift)
            .collect(),
        major: majors
            .into_iter()
            .map(|instant| (instant as f64 - shift, calendar_label(instant, major.0)))
            .collect(),
    }
}

/// Oynatma başının yazısı: takvimde gün, ay, yıl ve saat; sayılarda adımın
/// basamağı kadar ondalık.
pub fn format(value: f64, scale: Scale, step: f64) -> String {
    match scale {
        Scale::Number => number::real(value, axis::decimals(step.abs().max(f64::EPSILON))),
        Scale::Calendar { offset } => {
            let moment = DateTime::from_unix((value + f64::from(offset) * 60.0).round() as i64);
            let (date, time) = (moment.date, moment.time);
            let day = format!(
                "{} {} {}",
                date.day(),
                MONTHS[usize::from(date.month()) - 1],
                date.year()
            );

            if step >= 86_400.0 {
                day
            } else if step >= 60.0 {
                format!("{day} {:02}:{:02}", time.hour(), time.minute())
            } else {
                format!(
                    "{day} {:02}:{:02}:{:02}",
                    time.hour(),
                    time.minute(),
                    time.second()
                )
            }
        }
    }
}

/// Zaman çizelgesi.
pub struct Timeline<'a, Message> {
    playback: &'a Playback,
    on_event: Box<dyn Fn(Event) -> Message + 'a>,
    scale: Scale,
    markers: Vec<Marker>,
}

impl<'a, Message: Clone + 'a> Timeline<'a, Message> {
    pub fn new(playback: &'a Playback, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        Self {
            playback,
            on_event: Box::new(on_event),
            scale: Scale::Number,
            markers: Vec::new(),
        }
    }

    pub fn scale(mut self, scale: Scale) -> Self {
        self.scale = scale;
        self
    }

    pub fn marker(mut self, marker: Marker) -> Self {
        self.markers.push(marker);
        self
    }

    pub fn markers(mut self, markers: impl IntoIterator<Item = Marker>) -> Self {
        self.markers.extend(markers);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Timeline<'a, Message>> for Element<'a, Message> {
    fn from(timeline: Timeline<'a, Message>) -> Self {
        let playback = timeline.playback;
        let on_event = &timeline.on_event;
        let scale = timeline.scale;

        let control = |glyph: Icon, name: &'static str, event: Event| -> Element<'a, Message> {
            tip(
                button(icon(glyph).size(14.0))
                    .on_press(on_event(event))
                    .padding(5)
                    .style(style::button::flat),
                Tip::new(name),
                tooltip::Position::Top,
            )
        };

        let play: Element<'a, Message> = tip(
            button(
                icon(if playback.playing {
                    Icon::Pause
                } else {
                    Icon::Play
                })
                .size(14.0),
            )
            .on_press(on_event(Event::Play(!playback.playing)))
            .padding(5)
            .style(style::button::toggle(playback.playing)),
            Tip::new(if playback.playing {
                "Duraklat"
            } else {
                "Oynat"
            }),
            tooltip::Position::Top,
        );

        let mut controls = row![
            control(Icon::SkipBack, "Başa git", Event::Start),
            control(Icon::StepBack, "Bir adım geri", Event::Step(-1)),
            play,
            control(Icon::StepForward, "Bir adım ileri", Event::Step(1)),
            control(Icon::SkipForward, "Sona git", Event::End),
            container(label::mono(format(playback.current, scale, playback.step))).padding([0, 8]),
            space::horizontal(),
        ]
        .spacing(2)
        .align_y(Center);

        if let Some((start, end)) = playback.range {
            controls = controls.push(label::caption(format!(
                "Aralık: {} – {}",
                format(start, scale, playback.step),
                format(end, scale, playback.step)
            )));
            controls = controls.push(control(Icon::Close, "Aralığı kaldır", Event::Range(None)));
        }

        let speeds: Vec<(f64, Message)> = SPEEDS
            .iter()
            .map(|speed| (*speed, on_event(Event::Speed(*speed))))
            .collect();
        let current_speed = playback.speed;
        let speed = MenuButton::new(
            container(
                row![
                    label::mono_caption(format!(
                        "{}×",
                        number::real(current_speed, 2)
                            .trim_end_matches('0')
                            .trim_end_matches(',')
                    )),
                    icon(Icon::ChevronDown).size(10.0),
                ]
                .spacing(3)
                .align_y(Center),
            )
            .padding([4, 6]),
            move || {
                speeds
                    .iter()
                    .fold(Menu::new().header("Hız"), |menu, (speed, message)| {
                        let name = format!(
                            "{}×",
                            number::real(*speed, 2)
                                .trim_end_matches('0')
                                .trim_end_matches(',')
                        );

                        menu.check(name, *speed == current_speed, message.clone())
                    })
            },
        );

        controls = controls
            .push(tip(
                button(icon(Icon::Retry).size(14.0))
                    .on_press(on_event(Event::Loop(!playback.looping)))
                    .padding(5)
                    .style(style::button::toggle(playback.looping)),
                Tip::new(if playback.looping {
                    "Döngü açık: sonda başa döner"
                } else {
                    "Döngü kapalı: sonda durur"
                }),
                tooltip::Position::Top,
            ))
            .push(speed)
            .push(control(
                Icon::ZoomExtents,
                "Tümünü göster",
                Event::Window(playback.extent.0, playback.extent.1),
            ));

        let track = Track {
            playback,
            scale,
            markers: timeline.markers,
            on_event: timeline.on_event,
        };

        column![controls, Element::new(track)].spacing(4).into()
    }
}

/// Sürükleme: oynatma başı, yeni aralık (başladığı an ve yer), aralığın
/// uçları ya da gövdesi (tutulan yerin başa uzaklığı).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    Scrub,
    NewRange { anchor: f64, x: f32, moved: bool },
    Start,
    End,
    Move { offset: f64 },
}

#[derive(Debug, Default)]
struct State {
    drag: Option<Drag>,
    hovered: Option<usize>,
    shift: bool,
    /// İmlecin eksendeki yeri; ince çizgiyle gösterilir.
    cursor: Option<f32>,
}

/// Çizelgenin şeritleri.
struct Rows {
    axis: Rectangle,
    range: Rectangle,
    markers: Rectangle,
}

fn rows(bounds: Rectangle) -> Rows {
    let axis = typography::scaled(AXIS).round();
    let range = typography::scaled(RANGE).round();

    Rows {
        axis: Rectangle::new(bounds.position(), Size::new(bounds.width, axis)),
        range: Rectangle::new(
            Point::new(bounds.x, bounds.y + axis),
            Size::new(bounds.width, range),
        ),
        markers: Rectangle::new(
            Point::new(bounds.x, bounds.y + axis + range),
            Size::new(bounds.width, (bounds.height - axis - range).max(0.0)),
        ),
    }
}

struct Track<'a, Message> {
    playback: &'a Playback,
    scale: Scale,
    markers: Vec<Marker>,
    on_event: Box<dyn Fn(Event) -> Message + 'a>,
}

impl<'a, Message> Track<'a, Message> {
    /// Anın x'i.
    fn x(&self, bounds: Rectangle, value: f64) -> f32 {
        let (start, end) = self.playback.window;
        let width = f64::from((bounds.width - PAD * 2.0).max(1.0));

        bounds.x + PAD + ((value - start) / (end - start).max(f64::EPSILON) * width) as f32
    }

    /// x'teki an; tüm sürenin içinde.
    fn value(&self, bounds: Rectangle, x: f32) -> f64 {
        let (start, end) = self.playback.window;
        let width = f64::from((bounds.width - PAD * 2.0).max(1.0));
        let value = start + f64::from(x - bounds.x - PAD) / width * (end - start);

        self.playback.clamp(value)
    }

    fn pixels_per_unit(&self, bounds: Rectangle) -> f64 {
        let (start, end) = self.playback.window;

        f64::from((bounds.width - PAD * 2.0).max(1.0)) / (end - start).max(f64::EPSILON)
    }

    fn ticks(&self, bounds: Rectangle) -> Ticks {
        let scale = self.pixels_per_unit(bounds);

        match self.scale {
            Scale::Number => number_ticks(self.playback.window, scale),
            Scale::Calendar { offset } => calendar_ticks(self.playback.window, scale, offset),
        }
    }

    /// İmlecin üzerindeki işaret.
    fn marker_at(&self, bounds: Rectangle, point: Point) -> Option<usize> {
        let markers = rows(bounds).markers;

        if !markers.contains(point) {
            return None;
        }

        self.markers
            .iter()
            .enumerate()
            .map(|(index, marker)| (index, (self.x(bounds, marker.time) - point.x).abs()))
            .filter(|(_, distance)| *distance <= 6.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }
}

impl<'a, Message> Widget<Message, Theme, Renderer> for Track<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &layout::Limits) -> Node {
        let height = (typography::scaled(AXIS).round()
            + typography::scaled(RANGE).round()
            + typography::scaled(MARKERS).round())
        .ceil();

        Node::new(limits.resolve(Length::Fill, height, Size::new(0.0, height)))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &IcedEvent,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        let rows = rows(bounds);
        let playback = self.playback;

        match event {
            IcedEvent::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.shift = modifiers.shift();
            }
            IcedEvent::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(drag) = state.drag {
                    let value = self.value(bounds, position.x);
                    let event = match drag {
                        Drag::Scrub => Some(Event::Seek(value)),
                        Drag::NewRange { anchor, x, moved } => {
                            let moved = moved || (position.x - x).abs() >= 3.0;

                            state.drag = Some(Drag::NewRange { anchor, x, moved });
                            moved.then_some(Event::Range(Some((anchor, value))))
                        }
                        Drag::Start => playback
                            .range
                            .map(|(_, end)| Event::Range(Some((value.min(end), end)))),
                        Drag::End => playback
                            .range
                            .map(|(start, _)| Event::Range(Some((start, value.max(start))))),
                        Drag::Move { offset } => playback.range.map(|(start, end)| {
                            let length = end - start;
                            let start = (value - offset)
                                .clamp(playback.extent.0, playback.extent.1 - length);

                            Event::Range(Some((start, start + length)))
                        }),
                    };

                    if let Some(event) = event {
                        shell.publish((self.on_event)(event));
                    }

                    shell.capture_event();
                    return;
                }

                let hovered = self.marker_at(bounds, *position);
                let over = cursor.position_over(bounds).map(|point| point.x);

                if hovered != state.hovered || over != state.cursor {
                    state.hovered = hovered;
                    state.cursor = over;
                    shell.request_redraw();
                }
            }
            IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(point) = cursor.position_over(bounds) else {
                    return;
                };
                let value = self.value(bounds, point.x);

                if rows.range.contains(point) {
                    state.drag = Some(match playback.range {
                        Some((start, _)) if (self.x(bounds, start) - point.x).abs() <= EDGE => {
                            Drag::Start
                        }
                        Some((_, end)) if (self.x(bounds, end) - point.x).abs() <= EDGE => {
                            Drag::End
                        }
                        Some((start, end)) if value > start && value < end => Drag::Move {
                            offset: value - start,
                        },
                        _ => Drag::NewRange {
                            anchor: value,
                            x: point.x,
                            moved: false,
                        },
                    });
                } else if let Some(marker) = self.marker_at(bounds, point) {
                    shell.publish((self.on_event)(Event::Seek(self.markers[marker].time)));
                } else {
                    state.drag = Some(Drag::Scrub);
                    shell.publish((self.on_event)(Event::Seek(value)));
                }

                shell.capture_event();
            }
            IcedEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(drag) = state.drag.take() {
                    // Boş yere tıklamak aralığı kaldırır.
                    if let Drag::NewRange { moved: false, .. } = drag
                        && playback.range.is_some()
                    {
                        shell.publish((self.on_event)(Event::Range(None)));
                    }

                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            IcedEvent::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let Some(point) = cursor.position_over(bounds) else {
                    return;
                };
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => f64::from(*y),
                    mouse::ScrollDelta::Pixels { y, .. } => f64::from(*y) / 60.0,
                };
                let (start, end) = playback.window;
                let span = end - start;

                let (start, end) = if state.shift {
                    let shift = -lines * span * 0.1;

                    (start + shift, end + shift)
                } else {
                    let pivot = self.value(bounds, point.x);
                    let factor = 0.8f64.powf(lines);

                    (
                        pivot - (pivot - start) * factor,
                        pivot + (end - pivot) * factor,
                    )
                };

                shell.publish((self.on_event)(Event::Window(start, end)));
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        match state.drag {
            Some(Drag::Scrub) => return mouse::Interaction::ResizingHorizontally,
            Some(Drag::Move { .. }) => return mouse::Interaction::Grabbing,
            Some(_) => return mouse::Interaction::ResizingHorizontally,
            None => {}
        }

        let Some(point) = cursor.position_over(bounds) else {
            return mouse::Interaction::None;
        };
        let rows = rows(bounds);

        if rows.range.contains(point)
            && let Some((start, end)) = self.playback.range
        {
            let near = |value| (self.x(bounds, value) - point.x).abs() <= EDGE;

            if near(start) || near(end) {
                return mouse::Interaction::ResizingHorizontally;
            }

            let value = self.value(bounds, point.x);

            if value > start && value < end {
                return mouse::Interaction::Grab;
            }
        }

        if self.marker_at(bounds, point).is_some() {
            return mouse::Interaction::Pointer;
        }

        mouse::Interaction::Crosshair
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let rows = rows(bounds);
        let playback = self.playback;
        let size = Pixels(typography::scaled(10.0).round());
        let font = typography::mono();

        renderer.with_layer(bounds, |renderer| {
            // Zemin: eksen başlık renginde, alt şeritler yüzeyde.
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        color: t.border,
                        width: 1.0,
                        radius: 3.0.into(),
                    },
                    ..Quad::default()
                },
                Background::Color(t.field),
            );
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(
                        Point::new(rows.axis.x + 1.0, rows.axis.y + 1.0),
                        Size::new(rows.axis.width - 2.0, rows.axis.height - 1.0),
                    ),
                    ..Quad::default()
                },
                Background::Color(t.header),
            );

            // Seçili aralık: bütün şeritlerde hafif, aralık şeridinde belirgin.
            if let Some((start, end)) = playback.range {
                let (left, right) = (self.x(bounds, start), self.x(bounds, end));
                let band = |renderer: &mut Renderer, row: Rectangle, alpha: f32| {
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(
                                Point::new(left, row.y),
                                Size::new((right - left).max(1.0), row.height),
                            ),
                            ..Quad::default()
                        },
                        Background::Color(t.accent.scale_alpha(alpha)),
                    );
                };

                band(renderer, rows.axis, 0.08);
                band(renderer, rows.markers, 0.08);
                band(
                    renderer,
                    Rectangle::new(
                        Point::new(rows.range.x, rows.range.y + 2.0),
                        Size::new(rows.range.width, rows.range.height - 4.0),
                    ),
                    0.3,
                );

                for x in [left, right] {
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(
                                Point::new(x - 1.5, rows.range.y + 1.0),
                                Size::new(3.0, rows.range.height - 2.0),
                            ),
                            border: Border {
                                radius: 1.5.into(),
                                ..Border::default()
                            },
                            ..Quad::default()
                        },
                        Background::Color(t.accent),
                    );
                }
            }

            // Çizgiler ve etiketler.
            let ticks = self.ticks(bounds);
            let clip = Rectangle::new(
                Point::new(bounds.x + 1.0, bounds.y),
                Size::new(bounds.width - 2.0, bounds.height),
            );
            let label_room = Size::new(LABEL_SPACING as f32, rows.axis.height);

            renderer.with_layer(clip, |renderer| {
                for value in &ticks.minor {
                    let x = self.x(bounds, *value).round();

                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(
                                Point::new(x, rows.axis.y + rows.axis.height - 5.0),
                                Size::new(1.0, 5.0),
                            ),
                            ..Quad::default()
                        },
                        Background::Color(t.muted.scale_alpha(0.6)),
                    );
                }

                for (value, text) in &ticks.major {
                    let x = self.x(bounds, *value).round();

                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(
                                Point::new(x, rows.axis.y + 1.0),
                                Size::new(1.0, rows.axis.height - 1.0),
                            ),
                            ..Quad::default()
                        },
                        Background::Color(t.muted.scale_alpha(0.8)),
                    );
                    // Alt şeritlerde soluk kılavuz.
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(
                                Point::new(x, rows.range.y),
                                Size::new(1.0, rows.range.height + rows.markers.height - 1.0),
                            ),
                            ..Quad::default()
                        },
                        Background::Color(t.layer(0.06)),
                    );
                    renderer.fill_text(
                        Text {
                            content: text.clone(),
                            bounds: label_room,
                            size,
                            line_height: LineHeight::Absolute(size),
                            font,
                            align_x: text::Alignment::Left,
                            align_y: Vertical::Top,
                            shaping: Shaping::Advanced,
                            wrapping: Wrapping::None,
                        },
                        Point::new(x + 4.0, rows.axis.y + 4.0),
                        t.muted,
                        clip,
                    );
                }

                // İmlecin yeri.
                if let (Some(x), None) = (state.cursor, state.drag) {
                    renderer.fill_quad(
                        Quad {
                            bounds: Rectangle::new(
                                Point::new(x.round(), bounds.y + 1.0),
                                Size::new(1.0, bounds.height - 2.0),
                            ),
                            ..Quad::default()
                        },
                        Background::Color(t.muted.scale_alpha(0.45)),
                    );
                }
            });

            // İşaretler ve oynatma başı.
            let mut frame = canvas::Frame::new(renderer, bounds.size());
            let origin = Vector::new(-bounds.x, -bounds.y);
            let middle = rows.markers.center_y();

            for (index, marker) in self.markers.iter().enumerate() {
                let x = self.x(bounds, marker.time);

                if x < bounds.x + 2.0 || x > bounds.x + bounds.width - 2.0 {
                    continue;
                }

                let radius = if state.hovered == Some(index) {
                    6.0
                } else {
                    4.5
                };
                let center = Point::new(x, middle) + origin;
                let diamond = Path::new(|builder| {
                    builder.move_to(center + Vector::new(0.0, -radius));
                    builder.line_to(center + Vector::new(radius, 0.0));
                    builder.line_to(center + Vector::new(0.0, radius));
                    builder.line_to(center + Vector::new(-radius, 0.0));
                    builder.close();
                });

                frame.fill(&diamond, marker.color.unwrap_or(t.accent));
                frame.stroke(
                    &diamond,
                    canvas::Stroke::default()
                        .with_color(t.field)
                        .with_width(1.0),
                );
            }

            let head = self.x(bounds, playback.current);

            if head >= bounds.x && head <= bounds.x + bounds.width {
                let top = Point::new(head, bounds.y + 1.0) + origin;
                let pointer = Path::new(|builder| {
                    builder.move_to(top + Vector::new(-5.0, 0.0));
                    builder.line_to(top + Vector::new(5.0, 0.0));
                    builder.line_to(top + Vector::new(5.0, 5.0));
                    builder.line_to(top + Vector::new(0.0, 10.0));
                    builder.line_to(top + Vector::new(-5.0, 5.0));
                    builder.close();
                });

                frame.fill(&pointer, t.accent);
            }

            renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
                renderer.draw_geometry(frame.into_geometry());
            });

            if head >= bounds.x && head <= bounds.x + bounds.width {
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(
                            Point::new(head - 1.0, bounds.y + 1.0),
                            Size::new(2.0, bounds.height - 2.0),
                        ),
                        ..Quad::default()
                    },
                    Background::Color(t.accent),
                );
            }

            // Üzerinde durulan işaretin adı, işaretin yanında.
            if let Some(marker) = state.hovered.and_then(|index| self.markers.get(index)) {
                let x = self.x(bounds, marker.time);
                let width = marker.label.chars().count() as f32 * size.0 * 0.62 + 12.0;
                let height = size.0 + 6.0;
                let left = if x + 10.0 + width <= bounds.x + bounds.width {
                    x + 10.0
                } else {
                    x - 10.0 - width
                };
                let tag = Rectangle::new(
                    Point::new(left, middle - height / 2.0),
                    Size::new(width, height),
                );

                renderer.with_layer(bounds, |renderer| {
                    renderer.fill_quad(
                        Quad {
                            bounds: tag,
                            border: Border {
                                color: t.border,
                                width: 1.0,
                                radius: 3.0.into(),
                            },
                            ..Quad::default()
                        },
                        Background::Color(t.popover),
                    );
                    renderer.fill_text(
                        Text {
                            content: marker.label.clone(),
                            bounds: tag.size(),
                            size,
                            line_height: LineHeight::Absolute(size),
                            font: typography::ui(),
                            align_x: text::Alignment::Center,
                            align_y: Vertical::Center,
                            shaping: Shaping::Advanced,
                            wrapping: Wrapping::None,
                        },
                        Point::new(tag.center_x(), tag.center_y()),
                        t.text,
                        tag,
                    );
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2025-03-12 14:30:00 UTC.
    const MOMENT: f64 = 1_741_789_800.0;

    #[test]
    fn playback_loops_in_the_range_and_stops_without_loop() {
        let mut playback = Playback::new((0.0, 100.0), 10.0);

        playback.update(Event::Play(true));
        playback.advance(Duration::from_secs(3));
        assert_eq!(playback.current, 30.0);

        // Aralıkta döner.
        playback.update(Event::Range(Some((60.0, 40.0))));
        assert_eq!(playback.range, Some((40.0, 60.0)));
        playback.update(Event::Seek(55.0));
        playback.advance(Duration::from_secs(1));
        assert_eq!(playback.current, 45.0);

        // Döngü kapalıyken sonda durur.
        playback.update(Event::Loop(false));
        playback.advance(Duration::from_secs(5));
        assert_eq!(playback.current, 60.0);
        assert!(!playback.playing);

        // Sondayken oynatmak baştan başlatır.
        playback.update(Event::Play(true));
        assert_eq!(playback.current, 40.0);

        playback.update(Event::Speed(2.0));
        playback.advance(Duration::from_millis(500));
        assert_eq!(playback.current, 50.0);
    }

    #[test]
    fn steps_and_jumps_stay_inside_the_extent() {
        let mut playback = Playback::new((0.0, 100.0), 1.0).with_step(30.0);

        playback.update(Event::Step(-1));
        assert_eq!(playback.current, 0.0);
        playback.update(Event::Step(4));
        assert_eq!(playback.current, 100.0);
        playback.update(Event::Start);
        assert_eq!(playback.current, 0.0);
        playback.update(Event::Range(Some((20.0, 80.0))));
        playback.update(Event::End);
        assert_eq!(playback.current, 80.0);
    }

    #[test]
    fn the_window_stays_inside_and_follows_the_playhead() {
        let mut playback = Playback::new((0.0, 1_000.0), 100.0);

        playback.update(Event::Window(-50.0, 150.0));
        assert_eq!(playback.window, (0.0, 200.0));

        playback.update(Event::Window(900.0, 1_300.0));
        assert_eq!(playback.window, (600.0, 1_000.0));

        // Oynatma başı pencereden çıkınca pencere onu izler.
        playback.update(Event::Window(0.0, 100.0));
        playback.update(Event::Seek(450.0));
        assert_eq!(playback.window, (440.0, 540.0));
    }

    #[test]
    fn number_ticks_label_every_nice_step() {
        // 240 karede 480 piksel: 2 piksel/kare, etiketler 50 karede bir.
        let ticks = number_ticks((0.0, 240.0), 2.0);
        let labels: Vec<&str> = ticks.major.iter().map(|(_, text)| text.as_str()).collect();

        assert_eq!(labels, ["0", "50", "100", "150", "200"]);
        assert!(ticks.minor.contains(&10.0));
    }

    #[test]
    fn calendar_ticks_pick_months_days_or_hours() {
        let day = 86_400.0;
        let year = 365.0 * day;

        // Bir yıl 900 piksel: ay adımı, ocakta yıl yazar.
        let ticks = calendar_ticks((MOMENT, MOMENT + year), 900.0 / year, 0);
        let labels: Vec<&str> = ticks.major.iter().map(|(_, text)| text.as_str()).collect();
        assert_eq!(labels.first(), Some(&"Nis"));
        assert!(labels.contains(&"2026"));

        // Bir hafta 900 piksel: gün adımı.
        let ticks = calendar_ticks((MOMENT, MOMENT + 7.0 * day), 900.0 / (7.0 * day), 0);
        assert_eq!(ticks.major[0].1, "13 Mar");

        // Bir gün 900 piksel: saat adımı; gece yarısı günü yazar. Türkiye
        // saatiyle 14:30 UTC 17:30'dur.
        let ticks = calendar_ticks((MOMENT, MOMENT + day), 900.0 / day, 180);
        assert!(ticks.major.iter().any(|(_, text)| text == "18:00"));
        assert!(ticks.major.iter().any(|(_, text)| text == "13 Mar"));
    }

    #[test]
    fn weeks_start_on_monday() {
        // 10 Mart 2025 pazartesidir.
        let instants = calendar_instants(MOMENT - 5.0 * 86_400.0, MOMENT, (Unit::Day, 7));
        let monday = Date::new(2025, 3, 10).map(|date| date.days_since_epoch() * 86_400);

        assert_eq!(instants.first().copied(), monday);
    }

    #[test]
    fn the_playhead_is_written_in_turkish() {
        assert_eq!(
            format(MOMENT, Scale::Calendar { offset: 180 }, 60.0),
            "12 Mart 2025 17:30"
        );
        assert_eq!(
            format(MOMENT, Scale::Calendar { offset: 0 }, 86_400.0),
            "12 Mart 2025"
        );
        assert_eq!(format(120.0, Scale::Number, 1.0), "120");
        assert_eq!(format(1.25, Scale::Number, 0.05), "1,25");
    }
}

/// Gerçek olaylarla: eksende sürükleme, aralık seçme ve işarete tıklama.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::{Element, Point, Size};

    use super::{AXIS, Event, MARKERS, Marker, Playback, RANGE, Scale, Track};
    use crate::snapshot::{Input, Snapshot};
    use crate::theme::typography;

    fn view(playback: &Playback) -> Element<'_, Event> {
        Element::new(Track {
            playback,
            scale: Scale::Number,
            markers: vec![Marker::new(75.0, "Anahtar kare")],
            on_event: Box::new(|event| event),
        })
    }

    #[test]
    fn the_track_seeks_selects_ranges_and_jumps_to_markers() {
        let mut snapshot = Snapshot::new(Size::new(420.0, 120.0)).expect("çizici kurulamadı");
        let mut playback = Playback::new((0.0, 100.0), 10.0);
        let mut update = |playback: &mut Playback, event| playback.update(event);
        let mut input =
            |playback: &mut Playback, input| snapshot.input(playback, view, &mut update, input);

        // Genişlik 420, iç boşluk 10: 4 piksel bir birimdir.
        let x = |value: f32| 10.0 + value * 4.0;
        let axis = typography::scaled(AXIS).round();
        let range = axis + typography::scaled(RANGE).round() / 2.0;
        let markers =
            axis + typography::scaled(RANGE).round() + typography::scaled(MARKERS).round() / 2.0;

        input(
            &mut playback,
            Input::Drag(Point::new(x(10.0), 5.0), Point::new(x(40.0), 5.0)),
        );
        assert_eq!(playback.current, 40.0);

        input(
            &mut playback,
            Input::Drag(Point::new(x(20.0), range), Point::new(x(60.0), range)),
        );
        assert_eq!(playback.range, Some((20.0, 60.0)));

        input(&mut playback, Input::Click(Point::new(x(75.0), markers)));
        assert_eq!(playback.current, 75.0);

        // Aralığın dışında boş yere tıklamak aralığı kaldırır.
        input(&mut playback, Input::Click(Point::new(x(90.0), range)));
        assert_eq!(playback.range, None);
    }
}

//! Zaman sürgüsü's bar under the drawing (docs/adr/0210 §5, §10; the web's
//! `ui/time/TimeBar.ts`): Başa, Geri, Oynat/Durdur, İleri, Sona; what the
//! position shows; the slider between its ends' dates; Adım (a number and a
//! unit), Anlık | Aralık, Hız, Döngü; Kapat. Narrow, the ends' dates go
//! first, then the words beside the fields, then Hız and Döngü, as far as the
//! slider needs to keep its room: what the bar holds is measured.

use iced::widget::{button, container, row, slider, text_input};
use iced::{Background, Border, Center, Element, Fill, Length, Theme};
use kentos_geometry_core::time::Unit;
use kentos_ui::icon::icon;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Tip, tip};
use kentos_ui::{label, style};

use super::{Event, SPEEDS, msg};
use crate::app::{App, Message};

/// The bar's height, logical pixels at the interface's type scale.
const HEIGHT: f32 = 36.0;

/// An icon button's width (a 16-pixel icon, 6 on each side).
const BUTTON: f32 = 28.0;

/// The room the slider keeps before something else gives way (the web's `.timebar__range`).
const TRACK_MIN: f32 = 80.0;

/// What the bar shows besides what it always does: the ends' dates, the words
/// beside the fields, Hız and Döngü.
struct Shown {
    ends: bool,
    words: bool,
    speed: bool,
    looping: bool,
}

/// What a bar `width` wide shows (the web's `TimeBar.fit`): the ends' dates
/// give way first, then the words, then Hız, then Döngü, each as soon as the
/// slider would have less than [`TRACK_MIN`]; the texts measured as drawn.
fn shown(width: f32, when: &str, ends: [&str; 2]) -> Shown {
    let measured = typography::measured_width;
    let (body, caption) = (typography::body(), typography::caption());
    let gap = 10.0;
    let kind = measured("Anlık", body, false) + measured("Aralık", body, false) + 44.0;
    // Always there: the padding, Başa … Sona, the position's text, Adım's number and unit,
    // Anlık | Aralık and Kapat, the slider's row, and the gaps between the seven.
    let fixed = 20.0
        + (5.0 * BUTTON + 8.0)
        + (measured(when, body, true) + 8.0)
        + typography::scaled(52.0)
        + typography::scaled(86.0)
        + kind
        + BUTTON
        + 6.0 * gap;
    let mut room = width - fixed;
    let mut take = |w: f32| {
        let fits = room - w >= TRACK_MIN;
        if fits {
            room -= w;
        }
        fits
    };
    let looping = take(BUTTON + gap);
    let speed = looping && take(typography::scaled(70.0) + gap);
    let words = speed
        && take(measured("Adım", caption, false) + measured("Hız", caption, false) + 2.0 * gap);
    let ends =
        words && take(measured(ends[0], caption, false) + measured(ends[1], caption, false) + 16.0);
    Shown {
        ends,
        words,
        speed,
        looping,
    }
}

/// Anlık or Aralık, as the segmented control writes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Instant,
    Range,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Kind::Instant => "Anlık",
            Kind::Range => "Aralık",
        })
    }
}

/// A speed as the bar writes it (“0,5×”).
fn speed_words(v: f64) -> String {
    format!("{}×", v.to_string().replace('.', ","))
}

impl App {
    /// The bar, while the slider is open; `width` the drawing area's (the bar's own), for what gives way.
    pub(crate) fn time_bar(&self, width: f32) -> Option<Element<'_, Message>> {
        let s = &self.time.slider;
        if !s.open {
            return None;
        }
        let action =
            |glyph: &'static str, words: &'static str, keys: &'static str, event: Event| {
                tip(
                    button(icon(crate::icons::from_web(Some(glyph))).size(16.0))
                        .on_press(msg(event))
                        .padding([5, 6])
                        .style(style::button::flat),
                    Tip::new(words).detail(keys),
                    iced::widget::tooltip::Position::Top,
                )
            };
        let playing = s.playing;
        let play = tip(
            button(
                icon(crate::icons::from_web(Some(if playing {
                    "pause"
                } else {
                    "play"
                })))
                .size(16.0),
            )
            .on_press(msg(Event::PlayStop))
            .padding([5, 6])
            .style(style::button::flat),
            Tip::new(if playing { "Durdur" } else { "Oynat" }).detail("Boşluk"),
            iced::widget::tooltip::Position::Top,
        );
        let nav = row![
            action("timeFirst", "Başa", "Home", Event::First),
            action("timePrev", "Geri", "←", Event::Prev),
            play,
            action("timeNext", "İleri", "→", Event::Next),
            action("timeLast", "Sona", "End", Event::Last),
        ]
        .spacing(2)
        .align_y(Center);
        let when = container(label::body(s.label()).font(typography::ui_strong()))
            .width(Length::Shrink)
            .padding([0, 4]);
        let track = slider(0.0..=s.last.max(0) as f64, s.position as f64, |v| {
            msg(Event::Go(v.round() as i64))
        })
        .step(1.0)
        .width(Fill);
        let [first, last] = s.ends();
        let show = shown(width, &s.label(), [&first, &last]);
        let mut middle = row![].spacing(8).align_y(Center).width(Fill);
        if show.ends {
            middle = middle.push(label::caption(first).style(kentos_ui::style::text::muted));
        }
        middle = middle.push(track);
        if show.ends {
            middle = middle.push(label::caption(last).style(kentos_ui::style::text::muted));
        }
        let count = text_input("", &s.count)
            .on_input(|t| msg(Event::Count(t)))
            .on_submit(msg(Event::CountDone))
            .size(typography::body())
            .padding([3, 6])
            .width(typography::scaled(52.0))
            .style(style::field::input);
        let units: Vec<Choice> = Unit::ALL.iter().map(|u| Choice::new(u.word())).collect();
        let unit = container(Select::new(
            units,
            Unit::ALL.iter().position(|u| *u == s.step.unit),
            |i| msg(Event::Unit(i)),
        ))
        .width(typography::scaled(86.0));
        let kind = Segmented::new(
            [Kind::Instant, Kind::Range],
            if s.ranged { Kind::Range } else { Kind::Instant },
            |k| msg(Event::Ranged(k == Kind::Range)),
        );
        let speeds: Vec<Choice> = SPEEDS
            .iter()
            .map(|v| Choice::new(speed_words(*v)))
            .collect();
        let speed = container(Select::new(speeds, Some(s.speed), |i| msg(Event::Speed(i))))
            .width(typography::scaled(70.0));
        let looping = s.looping;
        let loop_button = tip(
            button(icon(crate::icons::from_web(Some("loop"))).size(16.0))
                .on_press(msg(Event::Loop))
                .padding([5, 6])
                .style(style::button::tool(looping)),
            Tip::new("Döngü").body("Sonda başa döner."),
            iced::widget::tooltip::Position::Top,
        );
        let mut line = row![nav, when, middle].spacing(10).align_y(Center);
        if show.words {
            line = line.push(label::caption("Adım"));
        }
        line = line.push(count).push(unit).push(kind);
        if show.words {
            line = line.push(label::caption("Hız"));
        }
        if show.speed {
            line = line.push(speed);
        }
        if show.looping {
            line = line.push(loop_button);
        }
        line = line.push(action("close", "Zaman sürgüsünü kapat", "", Event::Close));
        Some(
            container(line)
                .height(typography::scaled(HEIGHT))
                .width(Fill)
                .padding([4, 10])
                .align_y(Center)
                .style(|theme: &Theme| {
                    let t = Tokens::of(theme);
                    container::Style {
                        background: Some(Background::Color(t.surface)),
                        border: Border {
                            color: t.border,
                            width: 0.0,
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }
                })
                .into(),
        )
    }
}

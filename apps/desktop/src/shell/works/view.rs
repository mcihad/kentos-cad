//! Çalışan işler on the screen (mod.rs): the status bar's cell while something runs, and the bottom panel's tab that
//! lists each work with who started it, since when, how far it is, its Göster and its Durdur.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Column, button, column, container, row, scrollable};
use iced::{Center, Element, Fill};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::status_bar::Readout;
use kentos_ui::widget::{Tip, horizontal_divider, progress};

use super::{Event, Work};
use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::cloud::local_time::{Zone, clock};

/// The tab's and the cell's icon (the web's).
pub(crate) const GLYPH: &str = "play";

/// The longest title the cell shows of a single work; the tip has it whole.
const CELL_TITLE: usize = 28;

/// The progress bar's width in a row: room for its step in a line.
const BAR_WIDTH: f32 = 220.0;

/// The buttons' room in a row, Göster and Durdur: the same in every row, so the bars stand in one column whether a
/// work has its Göster or not.
const BUTTONS_WIDTH: f32 = 150.0;

fn msg(event: Event) -> Message {
    Message::Works(event)
}

/// A moment as the time of day on this computer: “14:05:09”.
fn clock_of(since: Instant) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(0));
    let ago = u64::try_from(since.elapsed().as_millis()).unwrap_or(0);
    clock(now.saturating_sub(ago), Zone::system())
}

/// How far, as the list and the cell say it: “%42”.
fn per_cent(share: f64) -> String {
    format!("%{}", (share.clamp(0.0, 1.0) * 100.0).floor())
}

impl Work {
    /// Its kind, who started it and since when, in a line.
    fn about(&self) -> String {
        let mut words = format!("{} · Başlatan: {}", self.what, self.by.words());
        if let Some(since) = self.since {
            words.push_str(&format!(" · Başlangıç {}", clock_of(since)));
        }
        words
    }

    /// Its step or what it waits for, and how far: what the row's bar says under it.
    fn progress_words(&self) -> String {
        if self.stopping {
            return "Durduruluyor…".to_owned();
        }
        // A whole bar says it without a per cent; a step that counts says its own.
        match (&self.state, self.share) {
            (Some(state), Some(share)) if share < 1.0 && !state.contains('%') => {
                format!("{state} · {}", per_cent(share))
            }
            (Some(state), _) => state.clone(),
            (None, Some(share)) => per_cent(share),
            (None, None) => "Çalışıyor".to_owned(),
        }
    }
}

impl App {
    /// The cell's words: a single work's title and how far, else how many; the count alone once the bar is tight.
    pub(crate) fn works_words(&self, compact: bool) -> Option<String> {
        let works = self.works();
        let words = match works.as_slice() {
            [] => return None,
            [one] if !compact => {
                let mut title: String = one.title.chars().take(CELL_TITLE).collect();
                if one.title.chars().count() > CELL_TITLE {
                    title.push('…');
                }
                match one.share {
                    Some(share) if share < 1.0 => format!("{title} {}", per_cent(share)),
                    _ => title,
                }
            }
            all if compact => all.len().to_string(),
            all => format!("{} iş çalışıyor", all.len()),
        };
        Some(words)
    }

    /// The status bar's cell while something runs: clicked, the İşler tab.
    pub(crate) fn works_cell(&self, compact: bool) -> Option<Element<'_, Message>> {
        let words = self.works_words(compact)?;
        let lines: Vec<String> = self
            .works()
            .iter()
            .map(|w| format!("{} — {}", w.title, w.progress_words()))
            .collect();
        Some(
            Readout::new(label::body(words))
                .icon(crate::icons::from_web(Some(GLYPH)))
                .tip(Tip::new("Çalışan işler").body(format!(
                    "{}\nTıklayın: Çalışan işler sekmesinde kimin başlattığını ve ilerlemesini görün, durdurun.",
                    lines.join("\n")
                )))
                .on_press(Message::BottomTab(BottomTab::Works))
                .into(),
        )
    }

    /// The bottom panel's tab: every work in a row, or what shows here when something runs.
    pub(crate) fn works_tab(&self) -> Element<'_, Message> {
        let works = self.works();
        if works.is_empty() {
            return container(label::muted(
                "Şu anda çalışan iş yok. İşlemler araçları ve modelleri, eklenti komutları, denetimleri ve deneme \
                 koşuları, ajanların ve Asistan'ın işleri, konsoldaki kod ve Python kurulumları çalışırken burada, \
                 kimin başlattığıyla listelenir; her birini buradan durdurabilirsiniz.",
            ))
            .padding(12)
            .into();
        }
        let mut list = Column::new().spacing(0);
        for (i, work) in works.into_iter().enumerate() {
            if i > 0 {
                list = list.push(horizontal_divider());
            }
            list = list.push(work_row(work));
        }
        let note = label::caption(
            "Kendi penceresi açıkken çalışan işler (Topoloji, Dizi, CAD denetimi …) o pencerede durdurulur; \
             kataloğun sunucu işleri Katalog bölmesindedir.",
        );
        // The bar beside the rows, not over them: over them it hid the rows' Durdur (the real window, three works).
        let bar = Scrollbar::new().width(6).scroller_width(6).spacing(6);
        let list = scrollable(list)
            .direction(Direction::Vertical(bar))
            .height(Fill);
        container(column![list, note].spacing(8))
            .padding([8, 12])
            .into()
    }
}

/// A work's row: its title and what it is, its bar and step, its buttons.
fn work_row(work: Work) -> Element<'static, Message> {
    let about = column![
        label::strong(work.title.clone()),
        label::caption(work.about())
    ]
    .spacing(2)
    .width(Fill);
    let bar = column![
        progress::bar(work.share.map(|s| s.clamp(0.0, 1.0) as f32)).height(6.0),
        label::caption(work.progress_words()),
    ]
    .spacing(4)
    .width(BAR_WIDTH);
    let mut buttons = row![].spacing(8).align_y(Center);
    if work.can_show {
        buttons = buttons.push(
            button(label::body("Göster"))
                .style(style::button::ghost)
                .on_press(msg(Event::Show(work.key.clone()))),
        );
    }
    if work.can_stop {
        let stop = button(label::body("Durdur")).style(style::button::secondary);
        buttons = buttons.push(if work.stopping {
            stop
        } else {
            stop.on_press(msg(Event::Stop(work.key.clone())))
        });
    }
    let buttons = container(buttons)
        .width(BUTTONS_WIDTH)
        .align_x(iced::alignment::Horizontal::Right);
    row![about, bar, buttons]
        .spacing(16)
        .padding([8, 0])
        .align_y(Center)
        .into()
}

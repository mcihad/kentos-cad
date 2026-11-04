//! The road windows' one layout (plan/road.md Y26.0, DESIGN.md §7.19). The owner's words: “dialog form tasarımları çok
//! karmaşık oluyor, daha derli toplu hale getir”. Each road window had built its own: its settings in one long row,
//! its result as a column of key–value lines, three or four equal buttons and a paragraph of help over everything. The
//! road windows now share the layout the processing tools have (§7.11):
//!
//! ```text
//! ┌ Kırmızı kot önerisi ─────────────────────────────────────────────────────────────── × ┐
//! │ Girdi ─────────────────   ┌ picture ───────────────────────────────────────────────┐ │
//! │  Boykesit [Uzun kot ▾]    │                                                        │ │
//! │ Kurallar ──────────────   └────────────────────────────────────────────────────────┘ │
//! │  En büyük eğim [   ] %    legend                                                     │
//! │  ☐ Uçları kilitle         [fark 0,412 m] [kazı 120 m²] [dolgu 118 m²] [boy 640 m]   │
//! │  ☐ Dengeli                ⚠ notes                                                    │
//! │                           (PVI'lar | Eğimler)  table                                 │
//! ├───────────────────────────────────────────────────────────────────────────────────────┤
//! │ status                                                  [Kapat] [■ Yeni boykesit …] │
//! └───────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! - **Left, the form:** grouped (Girdi, Kurallar, Çıktı), a label column, help under a field only where it says
//!   something the label does not. No paragraph of help over the window.
//! - **Right, the result:** what the window reads, drawn; under it at most four figures, then notes, then the detail
//!   behind a segmented choice (one table at a time, not every list stacked). Before there is a result, one quiet
//!   sentence says what the window will show.
//! - **The result follows the form:** a window that only reads reads again when a value changes; there is no “Önizle”.
//! - **Footer:** the status on the left; on the right Kapat and the one primary action (DESIGN.md §7.9: one amber
//!   button per surface), disabled until there is something to write.

use iced::widget::{Column, Row, column, container, row, space};
use iced::{Element, Fill, Length};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Form;
use kentos_ui::widget::Banner;
use kentos_ui::widget::StatCard;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::{label, style};

use crate::app::Message;
use crate::exchange::words;

/// The form column's width, at the 12 px body text.
const FORM: f32 = 300.0;

/// A road window: its form, its result and its footer.
pub struct RoadDialog<'a> {
    title: &'a str,
    form: Form<'a, Message>,
    picture: Option<(Element<'a, Message>, f32)>,
    legend: Option<String>,
    empty: Option<String>,
    figures: Vec<StatCard<'a, Message>>,
    notes: Vec<Banner<'a, Message>>,
    detail: Option<Element<'a, Message>>,
    status: Option<String>,
    primary: Option<(&'a str, Option<Message>)>,
    others: Vec<(&'a str, Option<Message>)>,
    close: Message,
    width: f32,
}

// `status`, `secondary` and `width` are the other road windows' as they move onto this layout (docs/adr/0930 §4).
#[allow(dead_code)]
impl<'a> RoadDialog<'a> {
    pub fn new(title: &'a str, form: Form<'a, Message>, close: Message) -> Self {
        Self {
            title,
            form,
            picture: None,
            legend: None,
            empty: None,
            figures: Vec::new(),
            notes: Vec::new(),
            detail: None,
            status: None,
            primary: None,
            others: Vec::new(),
            close,
            width: 1120.0,
        }
    }

    /// What the window reads, drawn, `height` high at the 12 px body text.
    pub fn picture(mut self, picture: impl Into<Element<'a, Message>>, height: f32) -> Self {
        self.picture = Some((picture.into(), height));
        self
    }

    /// One line under the picture that says what its lines are.
    pub fn legend(mut self, legend: impl Into<String>) -> Self {
        self.legend = Some(legend.into());
        self
    }

    /// The sentence in the result's place before there is one.
    pub fn empty(mut self, words: impl Into<String>) -> Self {
        self.empty = Some(words.into());
        self
    }

    /// A figure under the picture: a label and a value; at most four read at a glance.
    pub fn figure(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.figures.push(StatCard::new(label, value));
        self
    }

    pub fn note(mut self, note: Banner<'a, Message>) -> Self {
        self.notes.push(note);
        self
    }

    /// The detail, one table at a time: [`detail_choice`] over it.
    pub fn detail(mut self, detail: impl Into<Element<'a, Message>>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// The window's one primary action, pressed with `on` (none: disabled).
    pub fn primary(mut self, caption: &'a str, on: Option<Message>) -> Self {
        self.primary = Some((caption, on));
        self
    }

    /// A secondary action, beside Kapat.
    pub fn secondary(mut self, caption: &'a str, on: Option<Message>) -> Self {
        self.others.push((caption, on));
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn view(self) -> Element<'a, Message> {
        let muted = |t: &iced::Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let mut result = Column::new().spacing(10).width(Fill);
        let has_result = !self.figures.is_empty() || self.detail.is_some();
        if let Some((picture, height)) = self.picture {
            result = result.push(
                container(picture).width(Fill).height(typography::scaled(height)).style(style::container::field_box),
            );
            if let Some(legend) = self.legend {
                result = result.push(label::caption(legend).style(muted));
            }
        }
        if !has_result && let Some(empty) = self.empty {
            result = result.push(label::caption(empty).style(muted));
        }
        if !self.figures.is_empty() {
            let mut figures = Row::new().spacing(8);
            for f in self.figures {
                figures = figures.push(f.width(Fill));
            }
            result = result.push(figures);
        }
        for n in self.notes {
            result = result.push(n);
        }
        if let Some(detail) = self.detail {
            result = result.push(detail);
        }
        let form = container(self.form).width(typography::scaled(FORM));
        let body = row![form, result].spacing(typography::scaled(20.0));

        let mut frame = Frame::new(self.title).scroll(body).width(self.width).on_close(self.close.clone());
        if let Some(status) = self.status {
            frame = frame.aside(label::caption(status).style(muted));
        }
        for (caption, on) in self.others {
            frame = frame.action(words::secondary(caption, on));
        }
        frame = frame.action(words::secondary("Kapat", Some(self.close.clone())));
        if let Some((caption, on)) = self.primary {
            frame = frame.action(words::primary(caption, on));
        }
        frame.modal(self.close)
    }
}

/// The detail's choice over its table: `Segmented` with the window's lists; a fixed width so the row does not jump.
pub fn detail_choice<'a, T>(
    options: impl IntoIterator<Item = T>,
    selected: T,
    on_select: impl Fn(T) -> Message,
    table: impl Into<Element<'a, Message>>,
) -> Element<'a, Message>
where
    T: Copy + PartialEq + std::fmt::Display,
{
    column![
        row![kentos_ui::widget::Segmented::new(options, selected, on_select), space::horizontal()],
        container(table.into()).width(Length::Fill),
    ]
    .spacing(6)
    .into()
}

/// A column of a result table: its title, its width at the 12 px body text (none: the rest) and whether it holds
/// numbers (right aligned, monospaced).
pub struct Col<'a> {
    pub title: &'a str,
    pub width: Option<f32>,
    pub number: bool,
}

pub fn col(title: &str, width: f32) -> Col<'_> {
    Col { title, width: Some(width), number: false }
}

pub fn num(title: &str, width: f32) -> Col<'_> {
    Col { title, width: Some(width), number: true }
}

/// A result table: the columns and the rows' words (DESIGN.md §7.13: numbers right, the head on its band).
pub fn table<'a>(cols: Vec<Col<'a>>, rows: Vec<Vec<String>>, empty: &'a str) -> Element<'a, Message> {
    use kentos_ui::widget::table::{self, Table};
    let numbers: Vec<bool> = cols.iter().map(|c| c.number).collect();
    let columns = cols.into_iter().map(|c| {
        let mut column = table::Column::new(c.title).width(c.width.map_or(Length::Fill, Length::Fixed));
        if c.number {
            column = column.align_right();
        }
        column
    });
    let body = Table::new(columns)
        .extend(rows.into_iter().map(|cells| {
            table::Row::new(cells.into_iter().zip(numbers.iter().copied()).map(|(t, n)| -> Element<'a, Message> {
                if n { table::number(t).into() } else { table::text(t).into() }
            }))
        }))
        .empty(empty);
    body.into()
}

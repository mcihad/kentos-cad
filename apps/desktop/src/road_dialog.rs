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
//! - **The picture** zooms, pans and is saved as a PNG from its button group (picture.rs, docs/adr/0937); resized or
//!   full screen, the window gives it the height the rest leaves, never less than its own height.

use iced::widget::{Column, Row, column, container, row, space};
use kentos_ui::widget::grow::Grows;
use iced::{Element, Fill, Length};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Form;
use kentos_ui::widget::Banner;
use kentos_ui::widget::StatCard;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::label;

use crate::app::Message;
use crate::exchange::words;

/// The form column's width, at the 12 px body text.
const FORM: f32 = 300.0;

/// A road window: its form, its result and its footer.
pub struct RoadDialog<'a> {
    title: &'a str,
    form: Form<'a, Message>,
    picture: Option<(Element<'a, Message>, f32)>,
    /// The picture's name, the saved file's: the window's title unless given.
    picture_name: Option<String>,
    legend: Option<String>,
    empty: Option<String>,
    figures: Vec<StatCard<'a, Message>>,
    notes: Vec<Banner<'a, Message>>,
    detail: Option<Element<'a, Message>>,
    status: Option<String>,
    primary: Option<(&'a str, Option<Message>)>,
    others: Vec<(&'a str, Option<Message>)>,
    /// Quiet actions at the footer's start (Sil, Onayla …), DESIGN.md §7.9.
    ghosts: Vec<(&'a str, Option<Message>)>,
    close: Message,
    close_caption: &'a str,
    width: f32,
    form_width: f32,
    /// Shown above the form (a list of what the window edits).
    head: Option<Element<'a, Message>>,
    /// Shown under the result (the window's output row).
    tail: Option<Element<'a, Message>>,
}

// `status`, `secondary` and `width` are the other road windows' as they move onto this layout (docs/adr/0930 §4).
#[allow(dead_code)]
impl<'a> RoadDialog<'a> {
    pub fn new(title: &'a str, form: Form<'a, Message>, close: Message) -> Self {
        Self {
            title,
            form,
            picture: None,
            picture_name: None,
            legend: None,
            empty: None,
            figures: Vec::new(),
            notes: Vec::new(),
            detail: None,
            status: None,
            primary: None,
            others: Vec::new(),
            ghosts: Vec::new(),
            close,
            close_caption: "Kapat",
            width: 1120.0,
            form_width: FORM,
            head: None,
            tail: None,
        }
    }

    /// What the window reads, drawn, at least `height` high at the 12 px body text; it zooms, pans, is saved and
    /// grows with the window.
    pub fn picture(mut self, picture: impl Into<Element<'a, Message>>, height: f32) -> Self {
        self.picture = Some((picture.into(), height));
        self
    }

    /// The picture's name, the saved file's (the chosen schedule's or section's): the window's title otherwise.
    pub fn picture_name(mut self, name: impl Into<String>) -> Self {
        self.picture_name = Some(name.into());
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

    /// The form column's width at the 12 px body text (a table of several columns needs more than 300).
    pub fn form_width(mut self, width: f32) -> Self {
        self.form_width = width;
        self
    }

    /// The close button's words: Vazgeç for a window that saves its own copy.
    pub fn close_caption(mut self, caption: &'a str) -> Self {
        self.close_caption = caption;
        self
    }

    /// A quiet action at the footer's start.
    pub fn ghost(mut self, caption: &'a str, on: Option<Message>) -> Self {
        self.ghosts.push((caption, on));
        self
    }

    /// Something over the form (the list of what the window edits).
    pub fn head(mut self, head: impl Into<Element<'a, Message>>) -> Self {
        self.head = Some(head.into());
        self
    }

    /// Something under the result (an output row: a format and a write button).
    pub fn tail(mut self, tail: impl Into<Element<'a, Message>>) -> Self {
        self.tail = Some(tail.into());
        self
    }

    pub fn view(self) -> Element<'a, Message> {
        let muted = |t: &iced::Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let mut result = Grows::new().spacing(10.0).width(Fill);
        let has_result = !self.figures.is_empty() || self.detail.is_some();
        if let Some((picture, height)) = self.picture {
            let name = self.picture_name.unwrap_or_else(|| self.title.to_owned());
            result = result.push_grow(crate::picture::picture(picture, name), typography::scaled(height));
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
            // The table takes what the picture leaves, up to its rows; its rows scroll inside it (docs/adr/0938 §3).
            result = result.push_fit(detail, typography::scaled(DETAIL_LEAST));
        }
        if let Some(tail) = self.tail {
            result = result.push(tail);
        }
        let mut left = Column::new().spacing(12).width(typography::scaled(self.form_width));
        if let Some(head) = self.head {
            left = left.push(head);
        }
        left = left.push(self.form);
        // A form taller than the window scrolls by itself, the picture and the table staying in view.
        let left = kentos_ui::widget::grow::Held::within_room(
            iced::widget::scrollable(container(left).padding(iced::Padding::ZERO.right(12)))
                .direction(kentos_ui::style::field::body_scrollbar())
                .height(Length::Shrink),
        );
        let body = row![left, result].spacing(typography::scaled(20.0));

        let mut frame = Frame::new(self.title).scroll(body).width(self.width).on_close(self.close.clone());
        for (caption, on) in self.ghosts {
            frame = frame.aside(words::ghost(caption, on));
        }
        if let Some(status) = self.status {
            frame = frame.aside(label::caption(status).style(muted));
        }
        for (caption, on) in self.others {
            frame = frame.action(words::secondary(caption, on));
        }
        frame = frame.action(words::secondary(self.close_caption, Some(self.close.clone())));
        if let Some((caption, on)) = self.primary {
            frame = frame.action(words::primary(caption, on));
        }
        frame.modal(self.close)
    }
}

/// The least height a window's table is given before the window scrolls: a head and three rows.
const DETAIL_LEAST: f32 = 120.0;

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

/// A result table: the columns and the rows' words (DESIGN.md §7.13: numbers right, the head on its band). Its rows
/// scroll inside it under its head, so the window keeps its place (a schedule's 24 critical stations made the whole
/// Dever window scroll, its form and picture with them; docs/adr/0938 §3).
pub fn table<'a>(cols: Vec<Col<'a>>, rows: Vec<Vec<String>>, empty: &'a str) -> Element<'a, Message> {
    use kentos_ui::widget::table;
    if rows.is_empty() {
        return sheet_in(cols, rows, empty, Length::Shrink);
    }
    // As high as its rows (at most MOST_SHOWN) where there is room, less where the window has less: a RoadDialog gives
    // its detail what the picture leaves (`Grows::push_fit`), the rows scroll inside.
    let shown = rows.len().min(MOST_SHOWN);
    let natural = table::row_height() * (shown + 1) as f32 + 1.0 + 6.0;
    kentos_ui::widget::grow::Held::natural(sheet_in(cols, rows, empty, Length::Fill), natural).into()
}

/// The most rows a result table asks room for; more scroll inside it.
pub const MOST_SHOWN: usize = 15;

/// A long result table (a sheet: Kübaj cetveli, an alignment's stakeout): the kit's data table whose head stays while
/// its rows scroll within `height` (12 px body text), the rows built only as they show.
pub fn sheet<'a>(cols: Vec<Col<'a>>, rows: Vec<Vec<String>>, empty: &'a str, height: f32) -> Element<'a, Message> {
    use kentos_ui::widget::table;
    if rows.is_empty() {
        return sheet_in(cols, rows, empty, Length::Shrink);
    }
    // At most `height`; a few rows take only their own (the head's band and line, the sideways bar's 6 px); less where
    // the window has less, its rows scrolling inside (docs/adr/0938 §3).
    let height = typography::scaled(height).min(table::row_height() * (rows.len() + 1) as f32 + 1.0 + 6.0);
    kentos_ui::widget::grow::Held::natural(sheet_in(cols, rows, empty, Length::Fill), height).into()
}

/// A sheet beside a picture that grows (İstinat duvarı's rows by km): it fills the height and is as wide as its
/// columns, the picture taking the rest of the row.
pub fn sheet_fill<'a>(cols: Vec<Col<'a>>, rows: Vec<Vec<String>>, empty: &'a str) -> Element<'a, Message> {
    // As the kit counts it: each column's text width grown with the text, and its cell's pads.
    let count = cols.len();
    let natural: f32 = cols
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let (left, right) = kentos_ui::widget::table::cell_pads(i, count);
            typography::scaled(c.width.unwrap_or(120.0)) + left + right
        })
        .sum();
    container(sheet_in(cols, rows, empty, Length::Fill)).width(natural.ceil() + 1.0).height(Fill).into()
}

/// A sheet that fills the room it is given both ways (Aplikasyon's points beside their plan, two thirds of the row).
pub fn sheet_room<'a>(cols: Vec<Col<'a>>, rows: Vec<Vec<String>>, empty: &'a str) -> Element<'a, Message> {
    sheet_in(cols, rows, empty, Length::Fill)
}

fn sheet_in<'a>(cols: Vec<Col<'a>>, rows: Vec<Vec<String>>, empty: &'a str, height: Length) -> Element<'a, Message> {
    use kentos_ui::widget::table::{self, Table};
    let columns = move |cols: &[Col<'a>]| {
        cols.iter()
            .map(|c| {
                let column = table::Column::new(c.title).width(c.width.map_or(Length::Fill, Length::Fixed));
                if c.number { column.align_right() } else { column }
            })
            .collect::<Vec<_>>()
    };
    if rows.is_empty() {
        return Table::new(columns(&cols)).empty(empty).into();
    }
    // Wider than its room it scrolls sideways (the Kübaj sheet's eleven columns in half a window); narrower, its rows
    // reach the room's edge. `responsive` gives the room's width, the table is built for it.
    let numbers: std::rc::Rc<Vec<bool>> = std::rc::Rc::new(cols.iter().map(|c| c.number).collect());
    let rows = std::rc::Rc::new(rows);
    iced::widget::responsive(move |room| -> Element<'a, Message> {
        let (rows, numbers) = (rows.clone(), numbers.clone());
        Table::new(columns(&cols))
            .horizontal()
            .min_width(room.width)
            .height(Length::Fill)
            .virtualized(rows.len(), move |i| {
                table::Row::new(rows[i].iter().zip(numbers.iter().copied()).map(|(t, n)| -> Element<'a, Message> {
                    if n { table::number(t.clone()).into() } else { table::text(t.clone()).into() }
                }))
            })
            .into()
    })
    .height(height)
    .into()
}

/// A value typed in its table cell (the kit's `EditCell`: Enter or leaving the cell commits).
pub fn edit<'a>(value: &str, numeric: bool, on_commit: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    kentos_ui::widget::EditCell::new(value.to_owned(), on_commit).numeric(numeric).into()
}

/// An edited table (DESIGN.md §7.6): the kit's data table as a grid of writable cells (`cell(row, column)`, mostly
/// [`edit`]), its first column the row's number; pressing a row chooses it (for Satırı sil), the chosen row marked.
/// Its rows have the grid's height so the cells meet line to line; past `MOST_EDITED` rows they scroll under the head.
pub fn edit_table<'a>(
    cols: Vec<Col<'a>>,
    count: usize,
    cell: impl Fn(usize, usize) -> Element<'a, Message> + 'a,
    chosen: Option<usize>,
    on_choose: impl Fn(usize) -> Message + 'a,
    empty: &'a str,
) -> Element<'a, Message> {
    edited(cols, count, cell, chosen, Some(Box::new(on_choose)), empty)
}

/// An edited table whose rows are not chosen (each has its own Sil, say): [`edit_table`] without the choice.
pub fn grid_table<'a>(
    cols: Vec<Col<'a>>,
    count: usize,
    cell: impl Fn(usize, usize) -> Element<'a, Message> + 'a,
    empty: &'a str,
) -> Element<'a, Message> {
    edited(cols, count, cell, None, None, empty)
}

type Choose<'a> = Box<dyn Fn(usize) -> Message + 'a>;

fn edited<'a>(
    cols: Vec<Col<'a>>,
    count: usize,
    cell: impl Fn(usize, usize) -> Element<'a, Message> + 'a,
    chosen: Option<usize>,
    on_choose: Option<Choose<'a>>,
    empty: &'a str,
) -> Element<'a, Message> {
    use kentos_ui::widget::table::{self, Table};
    let n = cols.len();
    let columns = std::iter::once(table::Column::new("#").width(Length::Fixed(32.0)).align_right())
        .chain(cols.into_iter().map(|c| table::Column::new(c.title).width(c.width.map_or(Length::Fill, Length::Fixed))));
    if count == 0 {
        return Table::new(columns).grid().empty(empty).into();
    }
    Table::new(columns)
        .grid()
        .height(Length::Fixed(table::row_height() * count.min(MOST_EDITED) as f32))
        .virtualized(count, move |i| {
            let row = table::Row::new(std::iter::once(table::number((i + 1).to_string()).into()).chain((0..n).map(|k| cell(i, k))))
                .selected(chosen == Some(i));
            match &on_choose {
                Some(choose) => row.on_press(choose(i)),
                None => row,
            }
        })
        .reveal(chosen)
        .into()
}

/// The rows an edited table shows before it scrolls.
const MOST_EDITED: usize = 8;

/// A choice typed in its table cell (the grid's drop-down cell): the chosen option's words, the options in its menu.
pub fn pick<'a, S: Into<String>>(
    options: impl IntoIterator<Item = S>,
    chosen: Option<usize>,
    on_pick: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    let options: Vec<String> = options.into_iter().map(Into::into).collect();
    let shown = chosen.and_then(|i| options.get(i).cloned()).unwrap_or_else(|| "—".to_owned());
    kentos_ui::widget::property_grid::choice(shown, None, move || {
        options.iter().enumerate().fold(kentos_ui::widget::Menu::new(), |menu, (i, o)| menu.item(o.clone(), on_pick(i)))
    })
}

/// The cell that removes its row (a row of a list each line of which goes on its own: Pozlar, Kurallar).
pub fn drop_cell<'a>(on_press: Message) -> Element<'a, Message> {
    let x = iced::widget::button(kentos_ui::label::caption("×")).style(kentos_ui::style::button::ghost).padding([0, 6]).on_press(on_press);
    kentos_ui::widget::Cell::padded(container(x).center_x(Fill)).into()
}

/// A number in Turkish for a table: thousands with points, the fraction after a comma (1.289.851,02).
pub fn grouped(v: f64, digits: usize) -> String {
    let text = format!("{:.digits$}", v.abs());
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let mut out = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    let sign = if v < 0.0 && text.chars().any(|c| c.is_ascii_digit() && c != '0') { "-" } else { "" };
    if fraction.is_empty() { format!("{sign}{out}") } else { format!("{sign}{out},{fraction}") }
}

/// A result table whose cells are not only words (a colour swatch before a range): [`table`] with built cells.
pub fn table_of<'a>(cols: Vec<Col<'a>>, rows: Vec<Vec<Element<'a, Message>>>, empty: &'a str) -> Element<'a, Message> {
    use kentos_ui::widget::table::{self, Table};
    let columns = cols.into_iter().map(|c| {
        let column = table::Column::new(c.title).width(c.width.map_or(Length::Fill, Length::Fixed));
        if c.number { column.align_right() } else { column }
    });
    let many = rows.len() > MOST_SHOWN;
    let t = Table::new(columns).extend(rows.into_iter().map(table::Row::new)).empty(empty);
    if many { t.height(table::row_height() * MOST_SHOWN as f32).into() } else { t.into() }
}

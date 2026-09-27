//! A Hesap window's measurements table (the web's `Grid`, `ui/calc/common.ts`,
//! docs/adr/0070, 0071): the row number, the columns' cells (some fixed, as
//! a known station's name), a delete button per row and “Satır ekle” under
//! it. Enter goes down a column and adds a row at the end; several lines or
//! cells pasted from a spreadsheet fill from the cell down and right, adding
//! rows as needed. It looks as the web's does: a sheet in a framed box, the
//! head fixed above its rows, which scroll past a height.

use iced::widget::tooltip::Position;
use iced::widget::{Column, Row, button, column, container, row, scrollable, text_input};
use iced::{Center, Element, Fill, Length, Right, Task};
use kentos_interaction::{is_js_space, js_trim};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::horizontal_divider;
use kentos_ui::widget::{Tip, tip};

use super::read::read_number;
use super::{Event, Window, event};
use crate::app::Message;

/// A column: its heading, the unit after it, whether its values are numbers.
#[derive(Clone, Copy, Debug)]
pub struct Col {
    pub label: &'static str,
    pub unit: Option<&'static str>,
    pub numeric: bool,
}

/// What a table's rows are and what may be done to them (the web's `GridModel`).
pub trait Table {
    fn columns(&self) -> usize;
    fn rows(&self) -> usize;
    fn get(&self, row: usize, col: usize) -> &str;
    fn set(&mut self, row: usize, col: usize, text: String);
    /// A cell that cannot be typed in (a known station's name, a leg after the last point).
    fn readonly(&self, _row: usize, _col: usize) -> bool {
        false
    }
    /// Whether a new row may follow this one.
    fn can_insert_after(&self, _row: usize) -> bool {
        true
    }
    /// A new row after the table's row `row`.
    fn insert_after(&mut self, row: usize);
    fn can_remove(&self, row: usize) -> bool;
    fn remove(&mut self, row: usize);
}

/// A cell's field, for moving the keyboard to it.
pub fn cell_id(window: Window, row: usize, col: usize) -> iced::widget::Id {
    iced::widget::Id::from(format!("calc-{window:?}-{row}-{col}"))
}

/// Enter in a cell: the next row with this column open, or a new row after
/// this one (the web's `key`).
pub fn submit(table: &mut dyn Table, window: Window, row: usize, col: usize) -> Task<Message> {
    if let Some(next) = (row + 1..table.rows()).find(|&next| !table.readonly(next, col)) {
        return iced::widget::operation::focus(cell_id(window, next, col));
    }
    if !table.can_insert_after(row) {
        return Task::none();
    }
    table.insert_after(row);
    iced::widget::operation::focus(cell_id(window, row + 1, col))
}

/// “Satır ekle”: a row after the last one that may take one (the web's
/// `add`), and the keyboard to its first open cell.
pub fn add(table: &mut dyn Table, window: Window) -> Task<Message> {
    let Some(after) = (0..table.rows()).rev().find(|&r| table.can_insert_after(r)) else {
        return Task::none();
    };
    table.insert_after(after);
    let row = after + 1;
    let col = (0..table.columns())
        .find(|&c| !table.readonly(row, c))
        .unwrap_or(0);
    iced::widget::operation::focus(cell_id(window, row, col))
}

/// A pasted line's cells: the web's `/\t|;|\s{2,}|\s(?=[-+\d.])/` split
/// (a tab, a semicolon, two white spaces or more, or one before a number),
/// each cell trimmed.
pub fn cells(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let spaces = chars[i..].iter().take_while(|&&c| is_js_space(c)).count();
        let split = if c == '\t' || c == ';' {
            1
        } else if spaces >= 2 {
            spaces
        } else if spaces == 1
            && chars
                .get(i + 1)
                .is_some_and(|n| n.is_ascii_digit() || matches!(n, '-' | '+' | '.'))
        {
            1
        } else {
            0
        };
        if split > 0 {
            out.push(js_trim(&cell).to_owned());
            cell.clear();
            i += split;
        } else {
            cell.push(c);
            i += 1;
        }
    }
    out.push(js_trim(&cell).to_owned());
    out
}

/// Several lines or cells pasted into the cell (`row`, `col`) fill from it
/// down and right, adding rows (the web's `paste`); the row the paste ended
/// in. None for a single value: it stays as the field pasted it.
pub fn paste(table: &mut dyn Table, row: usize, col: usize, raw: &str) -> Option<usize> {
    if !js_trim(raw).contains(['\t', ';', '\n']) {
        return None;
    }
    let text = raw.replace('\r', "");
    let lines = text.split('\n').filter(|l| !js_trim(l).is_empty());
    let mut at = row;
    for (i, line) in lines.enumerate() {
        if i > 0 {
            if at + 1 >= table.rows() || table.readonly(at + 1, col) {
                if !table.can_insert_after(at) {
                    break;
                }
                table.insert_after(at);
            }
            at += 1;
        }
        for (j, value) in cells(line).into_iter().enumerate() {
            let c = col + j;
            if c < table.columns() && !table.readonly(at, c) {
                table.set(at, c, value);
            }
        }
    }
    Some(at)
}

/// The row number's and the delete button's columns.
const NO: f32 = 34.0;
const ACT: f32 = 30.0;
/// Where the rows start to scroll (the web's 280 px box).
const HEIGHT: f32 = 280.0;

/// The table as the web draws it; `placeholder` gives a cell's hint.
pub fn view<'a>(
    window: Window,
    columns: &'a [Col],
    table: &'a dyn Table,
    placeholder: impl Fn(usize, usize) -> String,
) -> Element<'a, Message> {
    let number = |n: String| -> Element<'a, Message> {
        container(label::caption(n))
            .width(Length::Fixed(NO))
            .padding([0, 8])
            .align_x(Right)
            .into()
    };
    let mut head = Row::new().push(number("#".to_owned())).align_y(Center);
    for c in columns {
        let mut words = Row::new().push(label::caption(c.label).style(style::text::default));
        if let Some(unit) = c.unit {
            words = words.push(label::caption(format!(" ({unit})")));
        }
        // Left, over numbers too, as the web's heads are.
        head = head.push(container(words).width(Fill).padding([5, 8]));
    }
    head = head.push(container(label::caption("")).width(Length::Fixed(ACT)));
    let mut body = Column::new();
    for r in 0..table.rows() {
        if r > 0 {
            body = body.push(horizontal_divider());
        }
        let mut line = Row::new()
            .push(number(format!("{}", r + 1)))
            .height(Length::Fixed(typography::scaled(28.0)))
            .align_y(Center);
        for (c, col) in columns.iter().enumerate() {
            let value = table.get(r, c);
            line = line.push(if table.readonly(r, c) {
                fixed(value, col.numeric)
            } else {
                cell(window, r, c, col, value, placeholder(r, c))
            });
        }
        let remove: Element<'a, Message> = if table.can_remove(r) {
            tip(
                button(icon(Icon::Close).size(12.0))
                    .on_press(event(Event::RemoveRow(r)))
                    .padding(6)
                    .style(style::button::ghost),
                Tip::new("Satırı sil"),
                Position::Left,
            )
        } else {
            iced::widget::space().into()
        };
        body = body.push(line.push(container(remove).width(Length::Fixed(ACT)).align_x(Center)));
    }
    let sheet = container(
        column![
            container(head).width(Fill).style(style::container::header),
            horizontal_divider(),
            container(
                scrollable(body)
                    .direction(style::field::body_scrollbar())
                    .height(Length::Shrink)
            )
            .max_height(typography::scaled(HEIGHT)),
        ]
        .width(Fill),
    )
    .width(Fill)
    .padding(1)
    .style(style::container::field_box);
    let can_add = (0..table.rows()).any(|r| table.can_insert_after(r));
    let add = button(
        row![icon(Icon::Plus).size(14.0), label::body("Satır ekle")]
            .spacing(6)
            .align_y(Center),
    )
    .on_press_maybe(can_add.then(|| event(Event::AddRow)))
    .padding([5, 10])
    .style(style::button::secondary);
    column![sheet, add].spacing(6).into()
}

/// A cell that is typed in: borderless in the sheet, the accent edge while
/// typed in, a red one while its number cannot be read.
fn cell<'a>(
    window: Window,
    r: usize,
    c: usize,
    col: &Col,
    value: &'a str,
    placeholder: String,
) -> Element<'a, Message> {
    let bad = col.numeric && read_number(value).is_some_and(f64::is_nan);
    let input = text_input(&placeholder, value)
        .id(cell_id(window, r, c))
        .on_input(move |t| event(Event::Cell(r, c, t)))
        .on_paste(move |t| event(Event::Paste(r, c, t)))
        .on_submit(event(Event::Submit(r, c)))
        .padding([4, 8])
        .width(Fill)
        .size(typography::body())
        .style(style::field::cell(bad));
    // Numbers in the figures' face; a hint in the words' face, as the web's.
    if col.numeric && !value.is_empty() {
        input.font(typography::mono()).align_x(Right).into()
    } else if col.numeric {
        input.font(typography::ui()).align_x(Right).into()
    } else {
        input.font(typography::ui()).into()
    }
}

/// A fixed cell: a known station's name, or a dash where nothing is measured.
fn fixed<'a>(value: &str, numeric: bool) -> Element<'a, Message> {
    let shown = if value.is_empty() { "—" } else { value };
    let words = label::body(shown.to_owned())
        .font(typography::ui_strong())
        .style(style::text::muted);
    let boxed = container(words).width(Fill).padding([4, 9]);
    if numeric {
        boxed.align_x(Right).into()
    } else {
        boxed.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table of two columns, the first row's first cell fixed.
    struct Sheet(Vec<[String; 2]>);

    impl Table for Sheet {
        fn columns(&self) -> usize {
            2
        }
        fn rows(&self) -> usize {
            self.0.len()
        }
        fn get(&self, row: usize, col: usize) -> &str {
            &self.0[row][col]
        }
        fn set(&mut self, row: usize, col: usize, text: String) {
            self.0[row][col] = text;
        }
        fn readonly(&self, row: usize, col: usize) -> bool {
            row == 0 && col == 0
        }
        fn insert_after(&mut self, row: usize) {
            self.0.insert(row + 1, Default::default());
        }
        fn can_remove(&self, _row: usize) -> bool {
            self.0.len() > 1
        }
        fn remove(&mut self, row: usize) {
            self.0.remove(row);
        }
    }

    #[test]
    fn a_pasted_line_splits_as_the_webs_does() {
        assert_eq!(cells("P1\tP2"), ["P1", "P2"]);
        assert_eq!(cells("P1;12,3"), ["P1", "12,3"]);
        assert_eq!(cells("P1   Q"), ["P1", "Q"]);
        assert_eq!(cells("Nokta 12.5 -3"), ["Nokta", "12.5", "-3"]);
        assert_eq!(cells("Köşe taşı"), ["Köşe taşı"]);
        // Two white spaces, a tab among them, are one separator.
        assert_eq!(cells("a \t\tb"), ["a", "b"]);
        assert_eq!(cells("  P1\tP2"), ["", "P1", "P2"]);
    }

    #[test]
    fn a_pasted_block_fills_down_and_right_past_fixed_cells() {
        let mut sheet = Sheet(vec![Default::default(); 2]);
        // Into the second column of the first row: the fixed cell is kept.
        assert_eq!(paste(&mut sheet, 0, 1, "1\r\n2\n\n3\n"), Some(2));
        assert_eq!(
            sheet.0.iter().map(|r| r[1].as_str()).collect::<Vec<_>>(),
            ["1", "2", "3"]
        );
        // Two cells a line from the first column: the fixed cell is skipped.
        let mut sheet = Sheet(vec![Default::default(); 1]);
        paste(&mut sheet, 0, 0, "A\t1\nB\t2");
        assert_eq!(sheet.0[0], [String::new(), "1".to_owned()]);
        assert_eq!(sheet.0[1], ["B".to_owned(), "2".to_owned()]);
        // A single value stays as the field pasted it.
        assert_eq!(paste(&mut sheet, 1, 1, " 7 "), None);
        assert_eq!(sheet.0[1][1], "2");
    }
}

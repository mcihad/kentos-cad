//! Aplikasyon (the web's `ui/calc/StakeoutDialog.ts`, docs/adr/0070): the
//! values to set out known points from a station: the bearing (semt) and
//! horizontal distance to each (the second fundamental task) and, with a
//! back point, the angle to turn clockwise from it. Nothing is added to the
//! drawing; the report is copied for the field.

use iced::widget::{Column, button, column, container, row, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_domain::Document as Model;
use kentos_interaction::{Format, fixed};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::Dialog;

use super::read::{StakeoutRead, read_stakeout, resolve_point, unit_name};
use super::{Event, Field, angle_text, event, footer_button, known_field, result_table, summary};
use crate::app::Message;
use crate::exchange::words::Kind as Line;

pub const TITLE: &str = "Aplikasyon";

/// What is typed, kept while the app runs; three empty rows at first.
#[derive(Clone, Debug)]
pub struct Form {
    pub station: String,
    pub back: String,
    pub rows: Vec<String>,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            station: String::new(),
            back: String::new(),
            rows: vec![String::new(); 3],
        }
    }
}

/// A row's field, for moving the keyboard to it.
pub fn cell_id(row: usize) -> iced::widget::Id {
    iced::widget::Id::from(format!("calc-aplikasyon-{row}"))
}

/// A pasted line's cells (the web's split: a tab, a semicolon, two spaces
/// or more, or a space before a number).
fn cells(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let spaces = chars[i..]
            .iter()
            .take_while(|c| c.is_whitespace() && **c != '\t')
            .count();
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
            out.push(cell.trim().to_owned());
            cell.clear();
            i += split;
        } else {
            cell.push(c);
            i += 1;
        }
    }
    out.push(cell.trim().to_owned());
    out
}

impl Form {
    pub fn set(&mut self, row: usize, text: String) {
        if let Some(cell) = self.rows.get_mut(row) {
            *cell = text;
        }
    }

    /// Several lines pasted from a spreadsheet fill the column from the
    /// row down, adding rows at the end (the web's `paste`); a single
    /// value stays as the field pasted it.
    pub fn paste(&mut self, row: usize, raw: &str) {
        if !raw.trim().contains(['\t', ';', '\n']) {
            return;
        }
        let text = raw.replace('\r', "");
        let lines: Vec<&str> = text.split('\n').filter(|l| !l.trim().is_empty()).collect();
        let mut at = row;
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                if at + 1 >= self.rows.len() {
                    self.rows.insert(at + 1, String::new());
                }
                at += 1;
            }
            if let (Some(first), Some(cell)) =
                (cells(line).into_iter().next(), self.rows.get_mut(at))
            {
                *cell = first;
            }
        }
    }

    /// Enter in a row: the next row, or a new one at the end.
    pub fn submit(&mut self, row: usize) -> Task<Message> {
        if row + 1 >= self.rows.len() {
            self.rows.insert(row + 1, String::new());
        }
        iced::widget::operation::focus(cell_id(row + 1))
    }

    /// “Satır ekle”: a row at the end; its index.
    pub fn add_row(&mut self) -> usize {
        self.rows.push(String::new());
        self.rows.len() - 1
    }

    /// A row goes, but the last one stays.
    pub fn remove_row(&mut self, row: usize) {
        if self.rows.len() > 1 && row < self.rows.len() {
            self.rows.remove(row);
        }
    }

    /// The selected points after the rows already filled.
    pub fn append(&mut self, picked: Vec<String>) {
        self.rows.retain(|r| !r.trim().is_empty());
        self.rows.extend(picked);
    }

    pub fn compute(&self, model: &Model) -> StakeoutRead {
        read_stakeout(
            &self.station,
            &self.back,
            &self.rows,
            |t| resolve_point(model, t),
            unit_name(model.settings().angle_unit),
        )
    }

    pub fn report(&self, model: &Model, format: &Format) -> Option<Vec<Vec<String>>> {
        let read = self.compute(model);
        let stakes = read.stakes?;
        let mut lines = vec![
            vec![TITLE.to_owned()],
            vec![
                "Nokta".into(),
                "Semt".into(),
                "Yatay uzunluk".into(),
                "Açı".into(),
            ],
        ];
        for (s, name) in stakes.iter().zip(&read.names) {
            lines.push(vec![
                name.clone(),
                fixed(s.bearing, 4),
                format.length_bare(s.distance),
                s.angle.map(|a| fixed(a, 4)).unwrap_or_default(),
            ]);
        }
        Some(lines)
    }

    pub fn view<'a>(&'a self, model: &Model, format: &Format) -> Element<'a, Message> {
        let read = self.compute(model);
        let knowns = row![
            known_field(
                model,
                format,
                "Durulan nokta (istasyon)",
                Field::Station,
                &self.station,
                None
            ),
            known_field(
                model,
                format,
                "Bakılan nokta",
                Field::Back,
                &self.back,
                Some("Verilirse ondan dönülecek açı da hesaplanır"),
            ),
        ]
        .spacing(16);
        let from_selection = button(
            row![
                icon(Icon::Select).size(14.0),
                label::body("Seçili noktaları ekle")
            ]
            .spacing(6)
            .align_y(Center),
        )
        .on_press(event(Event::FromSelection))
        .padding([5, 10])
        .style(style::button::secondary);
        let head = row![
            label::caption(
                "Noktaları adlarıyla ya da Y,X olarak yazın; çizimde seçili noktaları da ekleyebilirsiniz."
            )
            .width(Fill),
            from_selection,
        ]
        .spacing(12)
        .align_y(Center);
        let mut dialog = Dialog::new(TITLE)
            .push(knowns)
            .push(column![head, self.grid()].spacing(8));
        let lines = match &read.stakes {
            None => read
                .errors
                .iter()
                .take(6)
                .map(|e| (Line::Warn, e.clone()))
                .collect(),
            Some(stakes) => {
                let mut lines = vec![(
                    Line::Ok,
                    format!("{} nokta için semt ve uzunluk hesaplandı.", stakes.len()),
                )];
                if !read.back {
                    lines.push((
                        Line::Info,
                        "Bakılan nokta verilmedi: dönülecek açılar yok, aleti semte göre yöneltin."
                            .to_owned(),
                    ));
                }
                lines
            }
        };
        if let Some(summary) = summary(lines) {
            dialog = dialog.push(summary);
        }
        if let Some(stakes) = &read.stakes {
            let rows = stakes
                .iter()
                .zip(&read.names)
                .map(|(s, name)| {
                    vec![
                        name.clone(),
                        angle_text(format, s.bearing),
                        format.length_bare(s.distance),
                        s.angle
                            .map_or_else(|| "—".to_owned(), |a| angle_text(format, a)),
                    ]
                })
                .collect();
            dialog = dialog.push(
                column![
                    label::strong("Aplikasyon değerleri"),
                    result_table(
                        &["Nokta", "Semt", "Yatay uzunluk (m)", "Bakılan noktadan açı"],
                        rows,
                        &[false, true, true, true],
                    ),
                ]
                .spacing(6),
            );
        }
        let done = read.stakes.is_some();
        dialog
            .action(footer_button(
                "Raporu kopyala",
                done.then(|| event(Event::CopyReport)),
                true,
            ))
            .action(footer_button("Kapat", Some(event(Event::Close)), false))
            .width(860.0)
            .into()
    }

    /// The points' table: the row number, the field, its delete button;
    /// “Satır ekle” under it (the web's `Grid` with one column).
    fn grid(&self) -> Element<'_, Message> {
        let head = row![
            container(label::caption("#")).width(Length::Fixed(28.0)),
            container(label::caption("Aplike edilecek nokta (ad ya da Y,X)")).width(Fill),
            container(label::caption("")).width(Length::Fixed(28.0)),
        ]
        .spacing(8)
        .align_y(Center);
        let many = self.rows.len() > 1;
        let mut table = Column::new().spacing(4).push(head);
        for (i, value) in self.rows.iter().enumerate() {
            let input = text_input("", value)
                .id(cell_id(i))
                .on_input(move |t| event(Event::Cell(i, t)))
                .on_paste(move |t| event(Event::Paste(i, t)))
                .on_submit(event(Event::Submit(i)))
                .padding([4, 8])
                .width(Fill)
                .font(kentos_ui::theme::typography::ui())
                .size(kentos_ui::theme::typography::body())
                .style(style::field::input);
            let remove: Element<'_, Message> = if many {
                button(icon(Icon::Close).size(12.0))
                    .on_press(event(Event::RemoveRow(i)))
                    .padding(6)
                    .style(style::button::ghost)
                    .into()
            } else {
                iced::widget::space().width(28).into()
            };
            table = table.push(
                row![
                    container(label::caption(format!("{}", i + 1))).width(Length::Fixed(28.0)),
                    input,
                    container(remove).width(Length::Fixed(28.0)),
                ]
                .spacing(8)
                .align_y(Center),
            );
        }
        let add = button(
            row![icon(Icon::Plus).size(14.0), label::body("Satır ekle")]
                .spacing(6)
                .align_y(Center),
        )
        .on_press(event(Event::AddRow))
        .padding([5, 10])
        .style(style::button::ghost);
        column![table, add].spacing(6).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pasted_line_splits_as_the_webs_does() {
        assert_eq!(cells("P1\tP2"), ["P1", "P2"]);
        assert_eq!(cells("P1;12,3"), ["P1", "12,3"]);
        assert_eq!(cells("P1   Q"), ["P1", "Q"]);
        assert_eq!(cells("Nokta 12.5 -3"), ["Nokta", "12.5", "-3"]);
        assert_eq!(cells("Köşe taşı"), ["Köşe taşı"]);
    }

    #[test]
    fn a_pasted_column_fills_down_and_adds_rows() {
        let mut form = Form::default();
        form.paste(1, "P1\r\nP2\n\nP3\n");
        assert_eq!(form.rows, ["", "P1", "P2", "P3"]);
        // A single value stays as the field pasted it.
        form.paste(0, "P9");
        assert_eq!(form.rows[0], "");
    }
}

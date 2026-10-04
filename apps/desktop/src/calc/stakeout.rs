//! Aplikasyon (the web's `ui/calc/StakeoutDialog.ts`, docs/adr/0070): the
//! values to set out known points from a station: the bearing (semt) and
//! horizontal distance to each (the second fundamental task) and, with a
//! back point, the angle to turn clockwise from it. Nothing is added to the
//! drawing; the report is copied for the field.

use iced::widget::{button, column, row};
use iced::{Center, Element, Fill};
use kentos_domain::Document as Model;
use kentos_interaction::{Format, fixed, js_trim};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::Dialog;

use super::grid::{self, Col, Table};
use super::read::{StakeoutRead, read_stakeout, resolve_point, unit_name};
use super::{
    Event, Field, Window, angle_text, event, footer_button, known_field, knowns, result_table,
    summary,
};
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

/// One column: the points to set out.
impl Table for Form {
    fn columns(&self) -> usize {
        1
    }

    fn rows(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, row: usize, _col: usize) -> &str {
        self.rows.get(row).map_or("", String::as_str)
    }

    fn set(&mut self, row: usize, _col: usize, text: String) {
        if let Some(cell) = self.rows.get_mut(row) {
            *cell = text;
        }
    }

    fn insert_after(&mut self, row: usize) {
        let at = (row + 1).min(self.rows.len());
        self.rows.insert(at, String::new());
    }

    /// A row goes, but the last one stays.
    fn can_remove(&self, _row: usize) -> bool {
        self.rows.len() > 1
    }

    fn remove(&mut self, row: usize) {
        if self.rows.len() > 1 && row < self.rows.len() {
            self.rows.remove(row);
        }
    }
}

impl Form {
    /// The selected points after the rows already filled (the web's
    /// `rows.filter(r => r.point?.trim())` and the picked ones).
    pub fn append(&mut self, picked: Vec<String>) {
        self.rows.retain(|r| !js_trim(r).is_empty());
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
        let fields = knowns(vec![
            known_field(
                model,
                format,
                "Durulan nokta (istasyon)",
                Field::Station,
                &self.station,
                None,
            ),
            known_field(
                model,
                format,
                "Bakılan nokta",
                Field::Back,
                &self.back,
                Some("Verilirse ondan dönülecek açı da hesaplanır"),
            ),
        ]);
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
        let table = grid::view(Window::Stakeout, &COLUMNS, self, |_, _| String::new());
        let mut body = column![fields, column![head, table].spacing(8)].spacing(16);
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
            body = body.push(summary);
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
            body = body.push(
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
        Dialog::new(TITLE)
            .scroll(body)
            .action(footer_button(
                "Cihaza gönder…",
                Some(event(Event::SendToDevice)),
                false,
            ))
            .action(footer_button(
                "Raporu kopyala",
                done.then(|| event(Event::CopyReport)),
                true,
            ))
            .action(footer_button("Kapat", Some(event(Event::Close)), false))
            .width(860.0)
            .max_height(super::MAX_HEIGHT)
            .into()
    }
}

const COLUMNS: [Col; 1] = [Col {
    label: "Aplike edilecek nokta (ad ya da Y,X)",
    unit: None,
    numeric: false,
}];

#[cfg(test)]
mod tests {
    use super::*;

    /// A column pasted from a spreadsheet fills down from the row, adding
    /// rows; the kept rows and the selected points follow the filled ones.
    #[test]
    fn a_pasted_column_fills_down_and_adds_rows() {
        let mut form = Form::default();
        assert_eq!(grid::paste(&mut form, 1, 0, "P1\r\nP2\n\nP3\n"), Some(3));
        assert_eq!(form.rows, ["", "P1", "P2", "P3"]);
        // A single value stays as the field pasted it.
        assert_eq!(grid::paste(&mut form, 0, 0, "P9"), None);
        assert_eq!(form.rows[0], "");
        form.append(vec!["Q1".to_owned()]);
        assert_eq!(form.rows, ["P1", "P2", "P3", "Q1"]);
    }
}

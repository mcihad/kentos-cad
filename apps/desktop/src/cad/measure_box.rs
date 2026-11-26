//! Ölçü kutusu (docs/adr/9328): the last measurement of Mesafe ölç, Alan hesapla, Açı ölç and Koordinat oku in a box
//! of its own at the drawing area's lower left, while the tool that measured it runs:
//!
//! - its values, a row each with its unit: every part of a path and the total, an area and its perimeter, an angle
//!   and its explement, a point's east (Y) and north (X) by their names;
//! - the units by name, how the values were reckoned and in which coordinate system;
//! - Kopyala writes all of it to the clipboard; × puts the box away until the next measurement.
//!
//! The values are the tools' own (`kentos_interaction::measured`); the log keeps saying them as before.

use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, button, column, container, scrollable};
use iced::{Element, Fill, Length, Padding};
use kentos_interaction::Level;
use kentos_interaction::measured::Measurement;
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::Pairs;
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::icons::from_web;

/// The box's width, as Bilgi's card's.
const WIDTH: f32 = 300.0;
/// How high its rows grow before they scroll.
const ROWS_HIGH: f32 = 220.0;
/// Its margin from the drawing area's lower left corner.
const MARGIN: f32 = 12.0;

#[derive(Debug, Clone)]
pub enum Event {
    /// Kopyala: the measurement as text to the clipboard.
    Copy,
    /// ×: the box put away until the next measurement.
    Close,
}

fn msg(e: Event) -> Message {
    Message::MeasureBox(e)
}

/// The box's heading for what was measured.
fn heading(title: &str) -> String {
    match title {
        "Koordinat" => "Okunan koordinat".to_owned(),
        other => format!("{other} ölçüsü"),
    }
}

/// A muted line under the rows: what, then how.
fn note<'a>(what: &'a str, how: String) -> Element<'a, Message> {
    label::caption(format!("{what}: {how}")).style(style::text::muted).wrapping(Wrapping::WordOrGlyph).into()
}

impl App {
    /// The measurement the box shows: the last one, while the tool that measured it runs.
    pub(crate) fn measure_shown(&self) -> Option<&Measurement> {
        self.measured.as_ref().filter(|m| self.session.tool_id() == m.tool)
    }

    /// The box at the drawing area's lower left; none without a measurement of the running tool.
    pub(crate) fn measure_box_view(&self) -> Option<Element<'_, Message>> {
        let m = self.measure_shown()?;
        let head = Row::new()
            .spacing(6)
            .align_y(iced::Center)
            .push(container(label::strong(heading(m.title))).width(Fill))
            .push(
                button(label::caption("Kopyala"))
                    .on_press(msg(Event::Copy))
                    .padding(kentos_ui::theme::metrics::action_small())
                    .style(style::button::secondary),
            )
            .push(
                button(icon(from_web(Some("close"))).size(14.0))
                    .on_press(msg(Event::Close))
                    .padding(kentos_ui::theme::metrics::icon_small(14.0))
                    .style(style::button::ghost),
            );
        let rows = m.rows.iter().fold(Pairs::new().spacing_x(10.0).spacing_y(2.0), |rows, (what, value)| {
            rows.push(
                label::caption(what.clone()).style(style::text::muted),
                label::body(value.clone()).wrapping(Wrapping::WordOrGlyph),
            )
        });
        let mut notes = Column::new().spacing(2).push(note("Birim", m.units.clone())).push(note("Hesap", m.method.clone()));
        if let Some(system) = &m.system {
            notes = notes.push(note("Sistem", system.clone()));
        }
        // The rows as high as they are, scrolling past ROWS_HIGH (a path of many parts).
        let rows = container(scrollable(rows).height(Length::Shrink)).max_height(typography::scaled(ROWS_HIGH));
        let body = column![head, rows, notes].spacing(8);
        let framed = container(body)
            .width(Length::Fixed(typography::from_default(WIDTH)))
            .padding(Padding::from([10, 12]))
            .style(style::container::popover);
        Some(
            container(iced::widget::mouse_area(framed).interaction(iced::mouse::Interaction::Idle))
                .width(Fill)
                .height(Fill)
                .padding(MARGIN)
                .align_left(Fill)
                .align_bottom(Fill)
                .into(),
        )
    }

    pub(crate) fn measure_box_event(&mut self, e: Event) -> iced::Task<Message> {
        match e {
            Event::Copy => {
                let Some(text) = self.measure_shown().map(Measurement::text) else {
                    return iced::Task::none();
                };
                self.say(Level::Success, "Ölçü panoya kopyalandı.");
                iced::clipboard::write(text)
            }
            Event::Close => {
                self.measured = None;
                iced::Task::none()
            }
        }
    }
}

//! Ölçü kutusu (docs/adr/9328, 9351): Mesafe ölç's, Alan hesapla's, Açı ölç's and Koordinat oku's measurement in a box
//! of its own at the drawing area's lower left:
//!
//! - **While measuring** (Mesafe ölç and Alan hesapla, a point given): the measurement as it stands, the cursor's part
//!   with it, on the snapped point; its parts listed with their running totals, the total large, said as going on.
//! - **Measured:** the last measurement, kept after its tool ends (Esc, Enter, another click on Seç) until another tool
//!   starts or × puts it away; the same tool again shows it as it was.
//! - Its values, a row each with its unit: every part of a path with the running total, an area and its perimeter, an
//!   angle and its explement, a point's east (Y) and north (X) by their names; the units by name, how the values were
//!   reckoned and in which coordinate system.
//! - Kopyala writes all of it to the clipboard; × puts the box away until the next measurement.
//!
//! The values are the tools' own (`kentos_interaction::measured`); the live one is reckoned from the tool's preview, so
//! the box and the drawing say the same. The log keeps saying the finished ones as before. While a measuring tool runs
//! the drawing tools' snap takes every shown data layer's rows too (data_view.rs `snap_all`).

use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, button, column, container, row, scrollable};
use iced::{Element, Fill, Length, Padding};
use kentos_interaction::Level;
use kentos_interaction::measured::Measurement;
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::icons::from_web;

/// The box's width.
const WIDTH: f32 = 300.0;
/// How high its rows grow before they scroll.
const ROWS_HIGH: f32 = 200.0;
/// Its margin from the drawing area's lower left corner.
const MARGIN: f32 = 12.0;
/// The tools that measure: their measurement shows, and they snap to every shown data layer's rows.
const MEASURING: [&str; 5] = [
    kentos_interaction::path::MEASURE_ID,
    kentos_interaction::path::AREA_ID,
    kentos_interaction::angle::ID,
    kentos_interaction::coordinate::ID,
    kentos_interaction::coordinate::ELEVATION_ID,
];

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
    label::caption(format!("{what}: {how}"))
        .style(style::text::muted)
        .wrapping(Wrapping::WordOrGlyph)
        .into()
}

/// A ring's area (shoelace) and perimeter, closed.
fn ring_measures(ring: &[kentos_interaction::Vec2]) -> (f64, f64) {
    let n = ring.len();
    let (mut twice, mut perimeter) = (0.0, 0.0);
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        twice += a.x * b.y - b.x * a.y;
        perimeter += (b.x - a.x).hypot(b.y - a.y);
    }
    ((twice / 2.0).abs(), perimeter)
}

impl App {
    /// Whether a measuring tool runs.
    pub(crate) fn measuring(&self) -> bool {
        MEASURING.contains(&self.session.tool_id())
    }

    /// The measurement as it stands while Mesafe ölç or Alan hesapla runs with a point given (docs/adr/9351): the
    /// tool's preview (its points and the cursor's, snapped) measured as the finished one is.
    pub(crate) fn measure_live(&self) -> Option<Measurement> {
        let id = self.session.tool_id();
        if id != kentos_interaction::path::MEASURE_ID && id != kentos_interaction::path::AREA_ID {
            return None;
        }
        let doc = self.document.as_ref()?;
        let f = self.format();
        let preview = self.session.preview(&f)?;
        let settings = doc.model.settings();
        if id == kentos_interaction::path::MEASURE_ID {
            return (preview.path.len() >= 2)
                .then(|| kentos_interaction::measured::path(settings, &f, &preview.path, None));
        }
        let ring = preview.ring.filter(|r| r.len() >= 3)?;
        let (area, perimeter) = ring_measures(&ring);
        Some(kentos_interaction::measured::area(
            settings, &f, area, perimeter,
        ))
    }

    /// The measurement the box shows: the one going on, else the last one while its tool runs or no other tool does.
    pub(crate) fn measure_shown(&self) -> Option<Measurement> {
        if let Some(live) = self.measure_live() {
            return Some(live);
        }
        let tool = self.session.tool_id();
        self.measured
            .as_ref()
            .filter(|m| tool == m.tool || tool == "select")
            .cloned()
    }

    /// The box at the drawing area's lower left; none without a measurement to show.
    pub(crate) fn measure_box_view(&self) -> Option<Element<'_, Message>> {
        let live = self.measure_live().is_some();
        let m = self.measure_shown()?;
        // The total large, as the figure the measurement is for (the path's total, the area).
        let (lead, rest): (Option<(String, String)>, Vec<(String, String)>) = match m.title {
            "Mesafe" => {
                let total = m.rows.iter().find(|(w, _)| w == "Toplam").cloned();
                (
                    total,
                    m.rows
                        .iter()
                        .filter(|(w, _)| w != "Toplam")
                        .cloned()
                        .collect(),
                )
            }
            "Alan" => (
                m.rows.first().cloned(),
                m.rows.iter().skip(1).cloned().collect(),
            ),
            _ => (None, m.rows.clone()),
        };
        let mut title = Row::new()
            .spacing(6)
            .align_y(iced::Center)
            .push(label::strong(heading(m.title)));
        if live {
            title = title.push(
                container(label::caption("sürüyor"))
                    .padding([0, 6])
                    .style(style::container::badge),
            );
        }
        let head = Row::new()
            .spacing(4)
            .align_y(iced::Center)
            .push(container(title).width(Fill))
            .push(
                button(icon(from_web(Some("copy"))).size(14.0))
                    .on_press(msg(Event::Copy))
                    .padding(kentos_ui::theme::metrics::icon_small(14.0))
                    .style(style::button::ghost),
            )
            .push(
                button(icon(from_web(Some("close"))).size(14.0))
                    .on_press(msg(Event::Close))
                    .padding(kentos_ui::theme::metrics::icon_small(14.0))
                    .style(style::button::ghost),
            );
        let mut body = column![head].spacing(8);
        if let Some((what, value)) = &lead {
            body = body.push(
                column![
                    label::caption(what.clone()).style(style::text::muted),
                    label::figure(value.clone()).style(style::text::accent),
                ]
                .spacing(1),
            );
        }
        // A path's parts with their running totals, as a short table; another measurement's rows by name.
        let parts: Element<'_, Message> = if m.title == "Mesafe"
            && !rest.is_empty()
            && rest.iter().all(|(w, _)| w.ends_with(". parça"))
        {
            let header = row![
                label::caption("#").width(typography::scaled(22.0)),
                label::caption("Parça")
                    .width(Fill)
                    .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(8);
            let lines = rest.iter().enumerate().map(|(i, (_, v))| {
                row![
                    label::mono_caption((i + 1).to_string()).width(typography::scaled(22.0)),
                    label::mono(v.clone())
                        .width(Fill)
                        .align_x(iced::alignment::Horizontal::Right),
                ]
                .spacing(8)
                .into()
            });
            column![
                header,
                kentos_ui::widget::horizontal_divider(),
                scrollable(Column::with_children(lines).spacing(3)).height(Length::Shrink)
            ]
            .spacing(4)
            .into()
        } else {
            rest.iter()
                .fold(
                    kentos_ui::widget::Pairs::new()
                        .spacing_x(10.0)
                        .spacing_y(2.0),
                    |rows, (what, value)| {
                        rows.push(
                            label::caption(what.clone()).style(style::text::muted),
                            label::body(value.clone()).wrapping(Wrapping::WordOrGlyph),
                        )
                    },
                )
                .into()
        };
        body = body.push(container(parts).max_height(typography::scaled(ROWS_HIGH)));
        let mut notes = Column::new()
            .spacing(2)
            .push(note("Birim", m.units.clone()))
            .push(note("Hesap", m.method.clone()));
        if let Some(system) = &m.system {
            notes = notes.push(note("Sistem", system.clone()));
        }
        if live {
            notes = notes.push(note(
                "Bitir",
                "Enter bitirir, Esc vazgeçer. Kenet açıkken nokta yakalanan yere konur.".to_owned(),
            ));
        }
        body = body.push(notes);
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
                let Some(text) = self.measure_shown().map(|m| m.text()) else {
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

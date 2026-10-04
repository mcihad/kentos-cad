//! Karne editörü's window (docs/adr/0169 §6): the file, a text book's
//! columns, the station, the observations and their reduction, the summary.

use iced::widget::{Column, Row, button, column, container, row};
use iced::{Center, Element, Fill, Length};
use kentos_contracts::{AngleUnit, FieldBookRead};
use kentos_domain::Document as Model;
use kentos_geometry_core::display::fixed;
use kentos_interaction::Format;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::Dialog;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};

use super::super::grid::{self, Col};
use super::super::{
    Event as CalcEvent, MAX_HEIGHT, Window, event, footer_button, number_field, summary,
};
use super::{COLUMNS, Event, Form, HZ, MAPPED, TARGET, TITLE, fb, marks, shown};
use crate::app::Message;
use crate::exchange::words::{self, Kind as Line};

/// The observations table's columns with the book's angle unit.
fn columns(unit: AngleUnit) -> &'static [Col; 9] {
    const fn with(mark: &'static str) -> [Col; 9] {
        let mut c = COLUMNS;
        c[3].unit = Some(mark);
        c[4].unit = Some(mark);
        c
    }
    const GRAD: [Col; 9] = with("g");
    const DEG: [Col; 9] = with("°");
    match unit {
        AngleUnit::Grad => &GRAD,
        AngleUnit::Deg => &DEG,
    }
}

/// A heading of the reduced table, over numbers right-aligned.
fn head<'a>(t: String, numeric: bool) -> Element<'a, Message> {
    let c = container(label::caption(t).font(typography::ui_strong()))
        .width(Fill)
        .padding([4, 8]);
    if numeric {
        c.align_x(iced::Right).into()
    } else {
        c.into()
    }
}

/// A table cell: numbers right-aligned in the mono face, one above its
/// tolerance in the danger colour.
fn cell<'a>(t: String, numeric: bool, over: bool) -> Element<'a, Message> {
    let words = if numeric {
        label::mono(t)
    } else {
        label::body(t)
    };
    let words = if over {
        words.style(style::text::danger)
    } else {
        words
    };
    let c = container(words).width(Fill).padding([4, 8]);
    if numeric {
        c.align_x(iced::Right).into()
    } else {
        c.into()
    }
}

impl Form {
    /// The window (the web's `FieldBookDialog`).
    pub fn view<'a>(&'a self, model: &Model, format: &Format) -> Element<'a, Message> {
        let settings = model.settings();
        let unit = self.unit(settings.angle_unit);
        let open = button(
            row![icon(Icon::Open).size(14.0), label::body("Dosya aç…")]
                .spacing(6)
                .align_y(Center),
        )
        .on_press(fb(Event::Open))
        .padding([5, 12])
        .style(style::button::secondary);
        let what = match (&self.file, &self.book) {
            (Some(file), Some(book)) => {
                let kind = if book.format == "gsi" {
                    "Leica GSI"
                } else {
                    "Metin karne"
                };
                let n: usize = book.stations.iter().map(|s| s.observations.len()).sum();
                let unit = match unit {
                    AngleUnit::Grad => "gon",
                    AngleUnit::Deg => "derece",
                };
                format!(
                    "{file} · {kind} · {unit} · {} istasyon, {n} gözlem · {}",
                    book.stations.len(),
                    book.encoding
                )
            }
            _ => {
                "Leica GSI dosyasını ya da sütunları eşlenecek bir CSV/TXT karneyi açın.".to_owned()
            }
        };
        let mut body = Column::new()
            .spacing(14)
            .push(row![open, label::caption(what)].spacing(12).align_y(Center));
        if let Some(e) = &self.error {
            body = body.push(kentos_ui::widget::Banner::warning(e.clone()));
        }
        if self.mapped() {
            body = body.push(self.mapping_view(unit));
        }
        if let Some(book) = self.book.as_ref().filter(|b| !b.stations.is_empty()) {
            body = body
                .push(self.station_bar(book, format))
                .push(
                    column![
                        label::strong("Gözlemler"),
                        grid::view(Window::FieldBook, columns(unit), self, |_, _| String::new()),
                    ]
                    .spacing(6),
                )
                .push(column![label::strong("İndirgenmiş"), self.reduced_view(unit)].spacing(6));
        }
        if let Some(s) = summary(self.summary_lines(settings, unit)) {
            body = body.push(s);
        }
        Dialog::new(TITLE)
            .scroll(body)
            .width(1040.0)
            .action(footer_button(
                "Raporu kopyala",
                self.reduction
                    .as_ref()
                    .filter(|r| !r.rows.is_empty())
                    .map(|_| event(CalcEvent::CopyReport)),
                false,
            ))
            .action(footer_button("Kapat", Some(event(CalcEvent::Close)), false))
            .max_height(MAX_HEIGHT)
            .into()
    }

    /// A text book's mapping: a column for each field, the header, the angle unit.
    fn mapping_view<'a>(&'a self, unit: AngleUnit) -> Element<'a, Message> {
        let first = self
            .book
            .as_ref()
            .map_or(&[][..], |b| b.first_line.as_slice());
        let mut choices = vec![Choice::new("—")];
        choices.extend(first.iter().enumerate().map(|(i, c)| {
            let c: String = c.chars().take(18).collect();
            Choice::new(format!("{} · {c}", i + 1))
        }));
        let field = |i: usize| -> Element<'a, Message> {
            let selected = match self.mapping.columns[i] {
                Some(c) if (c as usize) < first.len() => Some(c as usize + 1),
                _ => Some(0),
            };
            let caption = if i == TARGET || i == HZ {
                format!("{} *", MAPPED[i])
            } else {
                MAPPED[i].to_owned()
            };
            column![
                label::caption(caption),
                Select::new(choices.clone(), selected, move |k| {
                    fb(Event::Map(i, k.checked_sub(1).map(|k| k as u32)))
                })
                .searchable(false),
            ]
            .spacing(4)
            .width(Length::Fixed(176.0))
            .into()
        };
        let line = |from: usize| Row::with_children((from..from + 4).map(field)).spacing(12);
        let units = Segmented::new(
            [Angle(AngleUnit::Grad), Angle(AngleUnit::Deg)],
            Angle(unit),
            |a| fb(Event::Unit(a.0)),
        );
        let mut c = column![
            label::strong("Sütunlar"),
            line(0),
            line(4),
            row![
                words::check(
                    self.mapping.header,
                    "İlk satır başlık",
                    Some(fb(Event::Header(!self.mapping.header))),
                ),
                row![label::caption("Açı birimi"), units]
                    .spacing(8)
                    .align_y(Center),
            ]
            .spacing(24)
            .align_y(Center),
        ]
        .spacing(8);
        if self.mapping.options().is_none() {
            c = c.push(label::caption(
                "Nokta ve Yatay açı sütunlarını seçin; karne eşlenen sütunlarla okunur.",
            ));
        }
        container(c)
            .padding(12)
            .width(Fill)
            .style(style::container::bordered)
            .into()
    }

    /// The station shown, its instrument height and the coordinates the file gives.
    fn station_bar<'a>(&'a self, book: &'a FieldBookRead, format: &Format) -> Element<'a, Message> {
        let choices: Vec<Choice> = book
            .stations
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let name = if s.station.is_empty() {
                    format!("{}. istasyon (adsız)", i + 1)
                } else {
                    s.station.clone()
                };
                Choice::new(format!("{name} · {} gözlem", s.observations.len()))
            })
            .collect();
        let select = column![
            label::caption("İstasyon"),
            Select::new(choices, Some(self.station), |i| fb(Event::Station(i))).searchable(false),
        ]
        .spacing(4)
        .width(Length::Fixed(260.0));
        let height = self
            .edits
            .get(self.station)
            .map_or("", |e| e.height.as_str());
        let height = number_field("Alet yüksekliği (m)", height, "0", |t| {
            fb(Event::Height(t))
        });
        let mut bar = row![select, height].spacing(18).align_y(iced::Bottom);
        let st = &book.stations[self.station.min(book.stations.len() - 1)];
        if let (Some(e), Some(n)) = (st.east, st.north) {
            let mut place = format.point(kentos_interaction::Vec2::new(e, n));
            if let Some(h) = st.height {
                place.push_str(&format!("  Z {}", fixed(h, 3)));
            }
            bar = bar.push(
                column![label::caption("Dosyadaki koordinatlar"), label::mono(place)].spacing(4),
            );
        }
        bar.into()
    }

    /// The station's reduction: a row per target, the differences in cc or
    /// seconds and millimetres, one above its tolerance in the danger colour.
    fn reduced_view<'a>(&self, unit: AngleUnit) -> Element<'a, Message> {
        let (mark, fine, per) = marks(unit);
        let heads = [
            ("Nokta".to_owned(), false),
            ("Durum".to_owned(), false),
            (format!("Yatay açı ({mark})"), true),
            (format!("Fark ({fine})"), true),
            (format!("Başucu açısı ({mark})"), true),
            (format!("İndeks ({fine})"), true),
            ("Eğik uzunluk (m)".to_owned(), true),
            ("Fark (mm)".to_owned(), true),
            ("Yatay uzunluk (m)".to_owned(), true),
            ("Kot farkı (m)".to_owned(), true),
        ];
        let mut table = Column::new().push(
            container(Row::with_children(
                heads
                    .iter()
                    .map(|(h, n)| head(h.clone(), *n))
                    .collect::<Vec<_>>(),
            ))
            .width(Fill)
            .style(style::container::header),
        );
        let Some(r) = &self.reduction else {
            return container(table)
                .width(Fill)
                .style(style::container::bordered)
                .into();
        };
        for row_ in &r.rows {
            let single = row_
                .observations
                .first()
                .and_then(|k| r.faces.get(*k).copied().flatten());
            let face = match (row_.faces, single) {
                (2, _) => "I + II",
                (_, Some(1)) => "I",
                (_, Some(2)) => "II",
                _ => "Doğrultu",
            };
            let over = |key: &str| row_.over.contains(&key);
            let cells = vec![
                cell(row_.target.clone(), false, false),
                cell(face.to_owned(), false, false),
                cell(fixed(row_.hz, 5), true, false),
                cell(
                    shown(row_.hz_diff.map(|d| d * per), 1),
                    true,
                    over("faceHz"),
                ),
                cell(shown(row_.zenith, 5), true, false),
                cell(shown(row_.index.map(|d| d * per), 1), true, over("index")),
                cell(shown(row_.slope, 4), true, false),
                cell(
                    shown(row_.slope_diff.map(|d| d * 1000.0), 1),
                    true,
                    over("faceSlope"),
                ),
                cell(shown(row_.horizontal, 4), true, false),
                cell(shown(row_.dh, 4), true, false),
            ];
            table = table.push(Row::with_children(cells));
        }
        container(table)
            .width(Fill)
            .style(style::container::bordered)
            .into()
    }

    /// What is said under the tables: the lines not read, the observations
    /// that are no face, the tolerances and what is above them, k.
    fn summary_lines(
        &self,
        settings: &kentos_contracts::ProjectSettings,
        unit: AngleUnit,
    ) -> Vec<(Line, String)> {
        let mut lines = Vec::new();
        let Some(book) = &self.book else {
            return lines;
        };
        for p in book.problems.iter().take(6) {
            lines.push((Line::Warn, p.message.clone()));
        }
        if book.problems.len() > 6 {
            lines.push((
                Line::Warn,
                format!("… {} satır daha okunmadı.", book.problems.len() - 6),
            ));
        }
        let Some(r) = &self.reduction else {
            return lines;
        };
        if let Some(st) = book.stations.get(self.station) {
            for u in &r.problems {
                if let Some(&i) = self.reduced_from.get(u.observation)
                    && let Some(o) = st.observations.get(i)
                {
                    lines.push((
                        Line::Warn,
                        format!(
                            "Satır {}: {} noktasının başucu açısı ({}) bir durum değil; gözlem indirgenmedi.",
                            o.line,
                            o.target,
                            shown(o.zenith, 5)
                        ),
                    ));
                }
            }
        }
        let (_, fine, _) = marks(unit);
        // The tolerances as Ölçme writes them: cc or seconds, millimetres.
        let texts = kentos_project::survey_form::texts(settings.survey.as_ref(), unit);
        let given: Vec<String> = [
            (!texts[1].is_empty()).then(|| format!("yatay fark {} {fine}", texts[1])),
            (!texts[2].is_empty()).then(|| format!("indeks {} {fine}", texts[2])),
            (!texts[3].is_empty()).then(|| format!("uzunluk farkı {} mm", texts[3])),
        ]
        .into_iter()
        .flatten()
        .collect();
        if given.is_empty() {
            lines.push((
                Line::Info,
                "Tolerans verilmedi (Proje ayarları › Ölçme): farklar denetlenmedi.".to_owned(),
            ));
        } else {
            lines.push((Line::Info, format!("Toleranslar: {}.", given.join(", "))));
            let over = r.rows.iter().filter(|row_| !row_.over.is_empty()).count();
            if over > 0 {
                lines.push((Line::Warn, format!("{over} hedefte tolerans aşıldı.")));
            }
        }
        lines.push((
            Line::Info,
            format!(
                "Kot farkları yer eğriliği ve refraksiyonla, k = {}.",
                settings.refraction()
            ),
        ));
        lines
    }
}

/// Açı birimi's two choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Angle(AngleUnit);

impl std::fmt::Display for Angle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            AngleUnit::Grad => "Grad",
            AngleUnit::Deg => "Derece",
        })
    }
}

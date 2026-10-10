//! Koordinat listesi dışa aktar (the web's `ui/io/CoordExportDialog.ts`):
//! the point objects of the selection, of the visible layers or of the whole
//! drawing, one per line (name, Y, X, Z) as Netcad NCN, TXT or CSV. Values
//! are written as the shortest decimal that reads back to the same float64:
//! nothing is rounded (CLAUDE.md §23).
//!
//! Opened from Noktalar (docs/adr/0153 §5) it offers the table's rows first,
//! in the table's order: its selected rows, or every row it shows (“Seçili
//! satırlar”, “Tablodaki”), in place of the drawing's selection.

use std::fmt;

use iced::widget::{Column, row};
use iced::{Element, Task};
use kentos_contracts::{
    CoordDelimiter, CoordPoint, CoordWriteInput, Entity, ExportReport, TextEncoding, Vec2,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, overlay};

use super::coord_import::ORDERS;
use super::dxf_export::{Counted, Scope};
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message};

/// An output format of the export (the web's `COORD_FORMATS`).
struct Format {
    label: &'static str,
    delimiter: CoordDelimiter,
    extension: &'static str,
    header: bool,
}

const FORMATS: [Format; 4] = [
    Format {
        label: "NCN (Netcad; boşlukla ayrılmış)",
        delimiter: CoordDelimiter::Space,
        extension: "ncn",
        header: false,
    },
    Format {
        label: "TXT (sekmeyle ayrılmış)",
        delimiter: CoordDelimiter::Tab,
        extension: "txt",
        header: false,
    },
    Format {
        label: "CSV (noktalı virgülle)",
        delimiter: CoordDelimiter::Semicolon,
        extension: "csv",
        header: true,
    },
    Format {
        label: "CSV (virgülle)",
        delimiter: CoordDelimiter::Comma,
        extension: "csv",
        header: true,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Order(usize);

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(ORDERS.get(self.0).map_or("", |(name, _)| name))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Encoding(TextEncoding);

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            TextEncoding::Utf8 => "UTF-8",
            TextEncoding::Windows1254 => "Windows-1254 (Türkçe)",
        })
    }
}

/// Noktalar's target rows (docs/adr/0153 §5), in the table's order;
/// `selected`: they are its selected rows.
#[derive(Debug, Clone)]
pub struct TableRows {
    pub slots: Vec<Slot>,
    pub selected: bool,
}

/// What the window writes: the table's rows, or a scope of the drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Table,
    Of(Scope),
}

/// A choice with its count, as the segmented control shows it:
/// “Tablodaki (36)”, “Seçili satırlar (3)”, “Tümü (120)”.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shown {
    pick: Pick,
    count: usize,
    /// The table's rows are its selected ones.
    rows_selected: bool,
}

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.pick {
            Pick::Table if self.rows_selected => write!(f, "Seçili satırlar ({})", self.count),
            Pick::Table => write!(f, "Tablodaki ({})", self.count),
            Pick::Of(scope) => write!(f, "{}", Counted(scope, self.count)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    scope: Pick,
    /// Opened from Noktalar: its rows.
    table: Option<TableRows>,
    format: usize,
    order: usize,
    header: bool,
    encoding: TextEncoding,
    writing: bool,
    status: Option<(bool, String)>,
    written: Option<ExportReport>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Scope(Pick),
    Format(usize),
    Order(usize),
    Header(bool),
    Encoding(TextEncoding),
    Run,
    Bytes(Vec<u8>, ExportReport),
}

fn event(e: Event) -> Message {
    message(Exchange::CoordExport(e))
}

/// Point objects as coordinate list rows: the name is the label, else the
/// `Ad` attribute; the code the `Kod` attribute (the web's `coordPoints`).
pub fn coord_points<'a>(entities: impl IntoIterator<Item = &'a Entity>) -> Vec<CoordPoint> {
    entities
        .into_iter()
        .filter_map(|e| match e {
            Entity::Point(p) => Some(CoordPoint {
                name: p
                    .base
                    .label
                    .clone()
                    .or_else(|| p.base.attrs.get("Ad").cloned())
                    .unwrap_or_default(),
                p: Vec2 { x: p.p.x, y: p.p.y },
                z: p.z,
                code: p.base.attrs.get("Kod").filter(|c| !c.is_empty()).cloned(),
            }),
            _ => None,
        })
        .collect()
}

impl State {
    pub fn new(app: &App) -> Self {
        let selected = coord_points(app.scope_entities(Scope::Selection)).len();
        Self::with(
            if selected > 0 {
                Pick::Of(Scope::Selection)
            } else {
                Pick::Of(Scope::Visible)
            },
            None,
        )
    }

    /// Opened from Noktalar: its rows first.
    pub fn for_table(rows: TableRows) -> Self {
        Self::with(Pick::Table, Some(rows))
    }

    fn with(scope: Pick, table: Option<TableRows>) -> Self {
        Self {
            scope,
            table,
            format: 0,
            order: 0,
            header: FORMATS[0].header,
            encoding: TextEncoding::Utf8,
            writing: false,
            status: None,
            written: None,
        }
    }
}

impl App {
    fn coord_rows(&self, table: Option<&TableRows>, pick: Pick) -> Vec<CoordPoint> {
        match pick {
            Pick::Of(scope) => coord_points(self.scope_entities(scope)),
            Pick::Table => {
                let Some(doc) = &self.document else {
                    return Vec::new();
                };
                let slots = table.map_or(&[][..], |t| t.slots.as_slice());
                coord_points(slots.iter().filter_map(|s| doc.model.get(*s)))
            }
        }
    }

    pub(super) fn coord_export_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Run => return self.coord_export_run(),
            Event::Bytes(bytes, report) => {
                let Some(Window::CoordExport(s)) = &mut self.exchange else {
                    return Task::none();
                };
                s.written = Some(report);
                let extension = FORMATS[s.format.min(FORMATS.len() - 1)].extension;
                let name = self.export_name(&format!(".{extension}"));
                return self.save_export(bytes, name, ("Koordinat listesi", extension));
            }
            _ => {}
        }
        let Some(Window::CoordExport(s)) = &mut self.exchange else {
            return Task::none();
        };
        match e {
            Event::Scope(scope) => s.scope = scope,
            Event::Format(i) => {
                s.format = i.min(FORMATS.len() - 1);
                s.header = FORMATS[s.format].header;
            }
            Event::Order(i) => s.order = i.min(ORDERS.len() - 1),
            Event::Header(on) => s.header = on,
            Event::Encoding(e) => s.encoding = e,
            Event::Run | Event::Bytes(..) => {}
        }
        Task::none()
    }

    fn coord_export_run(&mut self) -> Task<Message> {
        let Some(Window::CoordExport(s)) = &self.exchange else {
            return Task::none();
        };
        let points = self.coord_rows(s.table.as_ref(), s.scope);
        if s.writing || points.is_empty() {
            return Task::none();
        }
        let input = CoordWriteInput {
            points,
            delimiter: FORMATS[s.format].delimiter,
            columns: ORDERS[s.order].1.to_vec(),
            header: s.header,
            encoding: s.encoding,
        };
        if let Some(Window::CoordExport(s)) = &mut self.exchange {
            s.writing = true;
            s.status = Some((false, "Yazılıyor…".to_owned()));
        }
        off_thread(
            move || kentos_formats::coords::write(&input),
            |(bytes, report)| Exchange::CoordExport(Event::Bytes(bytes, report)),
        )
    }

    pub(super) fn coord_export_written(
        &mut self,
        outcome: Option<Result<String, String>>,
    ) -> Task<Message> {
        let Some(Window::CoordExport(s)) = &mut self.exchange else {
            return Task::none();
        };
        s.writing = false;
        match outcome {
            None => s.status = None,
            Some(Err(e)) => s.status = Some((true, format!("Yazılamadı: {e}"))),
            Some(Ok(name)) => {
                let report = s.written.take().unwrap_or_default();
                let points = report.counts.get("point").copied().unwrap_or(0);
                self.say(Level::Success, format!("“{name}” yazıldı: {points} nokta."));
                for item in &report.notes {
                    self.output(words::report_text(item));
                }
                self.close_exchange();
            }
        }
        Task::none()
    }

    pub(super) fn coord_export_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let picks: Vec<Pick> = match &s.table {
            Some(_) => vec![Pick::Table, Pick::Of(Scope::Visible), Pick::Of(Scope::All)],
            None => Scope::ALL.iter().map(|&scope| Pick::Of(scope)).collect(),
        };
        let rows_selected = s.table.as_ref().is_some_and(|t| t.selected);
        let counts: Vec<Shown> = picks
            .iter()
            .map(|&pick| Shown {
                pick,
                count: self.coord_rows(s.table.as_ref(), pick).len(),
                rows_selected,
            })
            .collect();
        let current = counts
            .iter()
            .copied()
            .find(|c| c.pick == s.scope)
            .unwrap_or(Shown {
                pick: s.scope,
                count: 0,
                rows_selected,
            });
        let scope = Segmented::new_with(
            counts.clone(),
            current,
            |c| event(Event::Scope(c.pick)),
            |c| c.count > 0,
        );
        let format = Select::new(
            FORMATS.iter().map(|f| Choice::new(f.label)),
            Some(s.format),
            |i| event(Event::Format(i)),
        )
        .searchable(false);
        let order = Segmented::new((0..ORDERS.len()).map(Order), Order(s.order), |o| {
            event(Event::Order(o.0))
        });
        let header = words::check(
            s.header,
            "Başlık (Ad, Y, X, Z)",
            Some(event(Event::Header(!s.header))),
        );
        let encoding = Segmented::new(
            [
                Encoding(TextEncoding::Utf8),
                Encoding(TextEncoding::Windows1254),
            ],
            Encoding(s.encoding),
            |e| event(Event::Encoding(e.0)),
        );
        let points = self.coord_rows(s.table.as_ref(), s.scope);
        let no_z = points.iter().filter(|p| p.z.is_none()).count();
        let unnamed = points.iter().filter(|p| p.name.is_empty()).count();
        let mut lines = vec![if points.is_empty() {
            words::text_line(
                Line::Warn,
                "Bu kapsamda nokta yok. Başka bir kapsam seçin ya da noktaları seçip yeniden açın.",
            )
        } else {
            words::text_line(
                Line::Ok,
                format!(
                    "{} nokta yazılacak; değerler yuvarlanmadan, tam yazılır.",
                    points.len()
                ),
            )
        }];
        if no_z > 0 {
            lines.push(words::text_line(
                Line::Info,
                format!("{no_z} noktanın kotu yok; Z alanı boş kalır."),
            ));
        }
        if unnamed > 0 {
            lines.push(words::text_line(
                Line::Info,
                format!("{unnamed} noktanın adı yok; ad alanı boş kalır."),
            ));
        }
        let mut body = Column::new()
            .spacing(12)
            .push(words::field("Yazılacak noktalar", scope, None))
            .push(
                row![
                    words::field("Biçim", format, None),
                    words::field(
                        "Sütun sırası",
                        order,
                        Some("Y sağa (doğu), X yukarı (kuzey).".to_owned())
                    ),
                    words::field("İlk satır", header, None),
                ]
                .spacing(16),
            )
            .push(words::field(
                "Karakter kodlaması",
                encoding,
                (s.encoding == TextEncoding::Windows1254).then(|| {
                    "Eski Windows programları için; Türkçe dışındaki harfler “?” olur.".to_owned()
                }),
            ))
            .push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        overlay::blocking(
            Dialog::new("Koordinat listesi dışa aktar")
                // The body scrolls; the buttons stay in view whatever the window's height.
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Dışa aktar…",
                    (!points.is_empty() && !s.writing).then(|| event(Event::Run)),
                ))
                .width(720.0)
                .max_height(760.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use kentos_contracts::{EntityBase, PointEntity};

    use super::*;

    fn point(label: Option<&str>, ad: Option<&str>, kod: Option<&str>, z: Option<f64>) -> Entity {
        let mut attrs = BTreeMap::new();
        if let Some(ad) = ad {
            attrs.insert("Ad".to_owned(), ad.to_owned());
        }
        if let Some(kod) = kod {
            attrs.insert("Kod".to_owned(), kod.to_owned());
        }
        Entity::Point(PointEntity {
            base: EntityBase {
                id: 1,
                layer_id: "a".into(),
                color: None,
                attrs,
                label: label.map(str::to_owned),
                symbol: None,
                line_weight: None,
                label_pins: Vec::new(),
            },
            p: Vec2 { x: 1.5, y: 2.5 },
            z,
            parts: None,
        })
    }

    #[test]
    fn points_take_their_name_from_the_label_then_the_ad_attribute() {
        let points = coord_points(&[
            point(Some("P1"), Some("X"), Some("KT"), Some(10.0)),
            point(None, Some("P2"), Some(""), None),
            point(None, None, None, None),
        ]);
        assert_eq!(points[0].name, "P1");
        assert_eq!(points[0].code.as_deref(), Some("KT"));
        assert_eq!(points[0].z, Some(10.0));
        assert_eq!(points[1].name, "P2");
        assert_eq!(points[1].code, None);
        assert_eq!(points[2].name, "");
    }
}

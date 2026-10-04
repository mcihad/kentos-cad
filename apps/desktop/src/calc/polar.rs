//! Kutupsal alım (the web's `ui/calc/PolarDialog.ts`, docs/adr/0071): points
//! surveyed from a known station oriented on a known back point, by their
//! horizontal readings and lengths. A slope length has its zenith angle
//! (0 straight up, a quarter turn level); then the point's height comes too,
//! with the station's height and the instrument and target heights, and the
//! earth's curvature and refraction with the project's k (docs/adr/0169 §3).
//! The computation is the shared core's (`survey::polar::polar_survey`).

use iced::widget::{column, row};
use iced::{Element, Fill};
use kentos_domain::Document as Model;
use kentos_interaction::ground::{grid_note, survey_grid};
use kentos_interaction::{Format, fixed};
use kentos_ui::label;
use kentos_ui::widget::Dialog;

use super::grid::{self, Col, Table};
use super::read::{PolarRead, read_number, read_polar, resolve_point, unit_name};
use super::{
    Event, Field, NewPoint, Window, angle_text, event, footer, known_field, knowns, number_field,
    result_table, summary,
};
use crate::app::Message;
use crate::exchange::words::Kind as Line;

pub const TITLE: &str = "Kutupsal alım";

/// The table's columns: Nokta, Yatay açı okuması, Uzunluk, Başucu açısı,
/// Reflektör yüksekliği.
pub const NAME: usize = 0;
pub const READING: usize = 1;
pub const DISTANCE: usize = 2;
pub const ZENITH: usize = 3;
pub const TARGET: usize = 4;

/// What is typed, kept while the app runs; three empty rows at first.
#[derive(Clone, Debug)]
pub struct Form {
    pub station: String,
    pub back: String,
    pub back_reading: String,
    pub station_z: String,
    pub instrument_height: String,
    pub rows: Vec<[String; 5]>,
    pub layer: Option<String>,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            station: String::new(),
            back: String::new(),
            back_reading: "0".to_owned(),
            station_z: String::new(),
            instrument_height: String::new(),
            rows: vec![Default::default(); 3],
            layer: None,
        }
    }
}

impl Table for Form {
    fn columns(&self) -> usize {
        5
    }

    fn rows(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, row: usize, col: usize) -> &str {
        self.rows.get(row).map_or("", |r| r[col].as_str())
    }

    fn set(&mut self, row: usize, col: usize, text: String) {
        if let Some(r) = self.rows.get_mut(row) {
            r[col] = text;
        }
    }

    fn insert_after(&mut self, row: usize) {
        let at = (row + 1).min(self.rows.len());
        self.rows.insert(at, Default::default());
    }

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
    pub fn compute(&self, model: &Model) -> PolarRead {
        read_polar(
            self,
            |t| resolve_point(model, t),
            unit_name(model.settings().angle_unit),
            Some(model.settings().refraction()),
            survey_grid(model.settings()),
        )
    }

    /// The points to add: their names, places and heights.
    pub fn points(&self, model: &Model) -> Vec<NewPoint> {
        let read = self.compute(model);
        let Some(points) = read.points else {
            return Vec::new();
        };
        points
            .iter()
            .zip(&read.names)
            .map(|(p, name)| NewPoint {
                name: name.clone(),
                p: p.p,
                z: p.z,
            })
            .collect()
    }

    pub fn report(&self, model: &Model, format: &Format) -> Option<Vec<Vec<String>>> {
        let read = self.compute(model);
        let points = read.points?;
        // With the project's grid: the length on it and the line's factors (docs/adr/0171 §4).
        let reduced = points.iter().any(|p| p.grid.is_some());
        let mut head = vec!["Nokta", "Semt", "Yatay uzunluk"];
        if reduced {
            head.push("Düzlemde");
        }
        head.extend(["Y", "X", "Z"]);
        if reduced {
            head.extend(["Ölçek", "Yükseklik çarpanı"]);
        }
        let mut lines = vec![
            vec![TITLE.to_owned()],
            head.into_iter().map(str::to_owned).collect(),
        ];
        for (p, name) in points.iter().zip(&read.names) {
            let mut row = vec![
                name.clone(),
                fixed(p.bearing, 4),
                format.length_bare(p.horizontal),
            ];
            if reduced {
                row.push(p.grid.map(|g| format.length_bare(g)).unwrap_or_default());
            }
            row.extend([
                format.coord(p.p.x),
                format.coord(p.p.y),
                p.z.map(|z| format.length_bare(z)).unwrap_or_default(),
            ]);
            if reduced {
                row.push(p.scale.map(|k| fixed(k, 8)).unwrap_or_default());
                row.push(p.height_factor.map(|k| fixed(k, 8)).unwrap_or_default());
            }
            lines.push(row);
        }
        Some(lines)
    }

    pub fn view<'a>(&'a self, model: &Model, format: &Format) -> Element<'a, Message> {
        let read = self.compute(model);
        let unit = if format.angle_unit_label() == "°" {
            "°"
        } else {
            "g"
        };
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
                Some("Alet bu noktaya yöneltilir"),
            ),
        ]);
        let numbers = row![
            number_field(
                format!("Bakılan noktanın okuması ({unit})"),
                &self.back_reading,
                "",
                |t| event(Event::BackReading(t)),
            ),
            number_field(
                "İstasyon kotu (m)",
                &self.station_z,
                "kot yoksa boş",
                |t| { event(Event::StationZ(t)) }
            ),
            number_field("Alet yüksekliği (m)", &self.instrument_height, "0", |t| {
                event(Event::InstrumentHeight(t))
            }),
        ]
        .spacing(18);
        let columns: &'static [Col] = if unit == "°" {
            &COLUMNS_DEG
        } else {
            &COLUMNS_GRAD
        };
        let table = grid::view(Window::Polar, columns, self, |r, c| match c {
            NAME => format!("{}", r + 1),
            ZENITH => "yatay uzunluksa boş".to_owned(),
            _ => String::new(),
        });
        let mut body = column![
            column![fields, numbers].spacing(12),
            column![
                label::caption(format!(
                    "Okumalar saat yönündedir; bakılan noktanın okuması semtine eşlenir. Başucu açısı verilen uzunluk eğiktir (0 tam yukarı, çeyrek tur yatay); o zaman nokta kotu da hesaplanır, yer eğriliği ve refraksiyonla: (1 − k)·D²/2R, k = {} (Proje ayarları › Ölçme).",
                    model.settings().refraction()
                ))
                .width(Fill),
                table,
            ]
            .spacing(8),
        ]
        .spacing(16);
        let lines = match &read.points {
            None => read
                .errors
                .iter()
                .take(6)
                .map(|e| (Line::Warn, e.clone()))
                .collect(),
            Some(points) => {
                let with_z = points.iter().filter(|p| p.z.is_some()).count();
                let heights = if with_z > 0 {
                    format!(", {with_z} noktanın kotu ile")
                } else {
                    String::new()
                };
                let mut lines = vec![(
                    Line::Ok,
                    format!("{} nokta hesaplandı{heights}.", points.len()),
                )];
                if let Some(h) = model
                    .settings()
                    .ground_height()
                    .filter(|_| points.iter().any(|p| p.grid.is_some()))
                {
                    lines.push((Line::Info, grid_note(h)));
                }
                if points.iter().any(|p| p.dz.is_some()) && read_number(&self.station_z).is_none() {
                    lines.push((
                        Line::Info,
                        "İstasyon kotu verilmedi: yükseklik farkları hesaplandı, kotlar yazılmadı."
                            .to_owned(),
                    ));
                }
                lines
            }
        };
        if let Some(summary) = summary(lines) {
            body = body.push(summary);
        }
        if let Some(points) = &read.points {
            // With the project's grid the length on it and the line's factor come too (docs/adr/0171 §4).
            let reduced = points.iter().any(|p| p.grid.is_some());
            let rows = points
                .iter()
                .zip(&read.names)
                .map(|(p, name)| {
                    let mut row = vec![
                        name.clone(),
                        angle_text(format, p.bearing),
                        format.length_bare(p.horizontal),
                    ];
                    if reduced {
                        row.push(p.grid.map(|g| format.length_bare(g)).unwrap_or_default());
                        row.push(
                            p.scale
                                .zip(p.height_factor)
                                .map(|(k, h)| fixed(k * h, 8))
                                .unwrap_or_default(),
                        );
                    }
                    row.extend([
                        format.coord(p.p.x),
                        format.coord(p.p.y),
                        match (p.z, p.dz) {
                            (Some(z), _) => format.length_bare(z),
                            (None, Some(dz)) => format!("Δ {}", format.length_bare(dz)),
                            (None, None) => "—".to_owned(),
                        },
                    ]);
                    row
                })
                .collect();
            let table = if reduced {
                result_table(
                    &[
                        "Nokta",
                        "Semt",
                        "Zeminde (m)",
                        "Düzlemde (m)",
                        "Çarpan",
                        "Y (sağa)",
                        "X (yukarı)",
                        "Z (m)",
                    ],
                    rows,
                    &[false, true, true, true, true, true, true, true],
                )
            } else {
                result_table(
                    &[
                        "Nokta",
                        "Semt",
                        "Yatay uzunluk (m)",
                        "Y (sağa)",
                        "X (yukarı)",
                        "Z (m)",
                    ],
                    rows,
                    &[false, true, true, true, true, true],
                )
            };
            body = body.push(column![label::strong("Sonuç"), table].spacing(6));
        }
        let done = read.points.is_some();
        footer(
            Dialog::new(TITLE).scroll(body),
            model,
            self.layer.as_deref(),
            done,
            done,
        )
        .width(940.0)
        .into()
    }
}

const COLUMNS_GRAD: [Col; 5] = columns("g");
const COLUMNS_DEG: [Col; 5] = columns("°");

const fn columns(unit: &'static str) -> [Col; 5] {
    [
        Col {
            label: "Nokta",
            unit: None,
            numeric: false,
        },
        Col {
            label: "Yatay açı okuması",
            unit: Some(unit),
            numeric: true,
        },
        Col {
            label: "Uzunluk",
            unit: Some("m"),
            numeric: true,
        },
        Col {
            label: "Başucu açısı",
            unit: Some(unit),
            numeric: true,
        },
        Col {
            label: "Reflektör yüksekliği",
            unit: Some("m"),
            numeric: true,
        },
    ]
}

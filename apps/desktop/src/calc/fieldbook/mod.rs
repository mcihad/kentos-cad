//! Karne editörü (docs/adr/0169 §2–§3, §6; the web's
//! `ui/calc/FieldBookDialog.ts`): a field book opened from an instrument's
//! file (Leica GSI, told by its content) or a text book whose columns are
//! mapped here; its stations, their observations as the file has them,
//! which may be left out (Kullan) and renamed; the station shown reduced as
//! the shared core reduces it (`survey::fieldbook::reduce`) with the
//! project's k and tolerances: the faces paired and their differences, the
//! horizontal distances and the height differences, a difference above its
//! tolerance in the warning colour. What is opened stays while the app runs,
//! as the other Hesap windows' fields do; the file itself is not changed.

use std::path::PathBuf;
use std::sync::Arc;

use kentos_contracts::{AngleUnit, FieldBookRead, FieldCsvOptions, FieldStation};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::survey::Unit;
use kentos_geometry_core::survey::fieldbook::{
    Observation, PolarTransfer, Reduction, Station, Tolerances, polar_transfer, reduce,
};

use super::grid::{Col, Mark, Table};
use super::read::read_number;
use super::{Event as CalcEvent, event};
use crate::app::Message;

pub const TITLE: &str = "Karne editörü";

/// The largest field book read (an instrument's files are kilobytes).
const LIMIT: u64 = 64 << 20;

/// A text book's columns as the mapping names them, in the CSV options' order.
pub const MAPPED: [&str; 8] = [
    "İstasyon",
    "Alet yüksekliği",
    "Nokta",
    "Yatay açı",
    "Başucu açısı",
    "Eğik uzunluk",
    "Prizma yüksekliği",
    "Kod",
];
const TARGET: usize = 2;
const HZ: usize = 3;

/// The observations table's columns: Kullan, Nokta, then what the file says.
pub const USE: usize = 0;
pub const NAME: usize = 1;
const COLUMNS: [Col; 9] = [
    Col {
        label: "Kullan",
        unit: None,
        numeric: false,
    },
    Col {
        label: "Nokta",
        unit: None,
        numeric: false,
    },
    Col {
        label: "Durum",
        unit: None,
        numeric: false,
    },
    Col {
        label: "Yatay açı",
        unit: None,
        numeric: true,
    },
    Col {
        label: "Başucu açısı",
        unit: None,
        numeric: true,
    },
    Col {
        label: "Eğik uzunluk",
        unit: Some("m"),
        numeric: true,
    },
    Col {
        label: "Prizma",
        unit: Some("m"),
        numeric: true,
    },
    Col {
        label: "Kod",
        unit: None,
        numeric: false,
    },
    Col {
        label: "Satır",
        unit: None,
        numeric: true,
    },
];

/// What the window's own controls ask.
#[derive(Clone, Debug)]
pub enum Event {
    /// Dosya aç…: the file dialog, then the file picked (none: Vazgeç).
    Open,
    Picked(Option<PathBuf>),
    /// A text book's mapping: a column for a field (none: —), the header, the angle unit.
    Map(usize, Option<u32>),
    Header(bool),
    Unit(AngleUnit),
    /// The station shown, and its instrument height as typed.
    Station(usize),
    Height(String),
    /// The reduced row that is the back sight, and Kutupsal alım'a aktar.
    Back(usize),
    Transfer,
}

/// A text book's mapping, remembered while the app runs.
#[derive(Clone, Debug)]
pub struct Mapping {
    pub columns: [Option<u32>; 8],
    pub header: bool,
    pub unit: Option<AngleUnit>,
}

impl Default for Mapping {
    fn default() -> Self {
        Self {
            columns: [None; 8],
            header: true,
            unit: None,
        }
    }
}

impl Mapping {
    /// The reader's options: none until the point and the horizontal reading are mapped.
    fn options(&self) -> Option<FieldCsvOptions> {
        let c = &self.columns;
        Some(FieldCsvOptions {
            station: c[0],
            instrument_height: c[1],
            target: c[TARGET]?,
            hz: c[HZ]?,
            zenith: c[4],
            slope: c[5],
            target_height: c[6],
            code: c[7],
            header: self.header,
        })
    }
}

/// A station's edits: the observations used, their names, its instrument height as typed.
#[derive(Clone, Debug, Default)]
struct Edits {
    used: Vec<bool>,
    names: Vec<String>,
    height: String,
}

impl Edits {
    fn of(s: &FieldStation) -> Self {
        Self {
            used: vec![true; s.observations.len()],
            names: s.observations.iter().map(|o| o.target.clone()).collect(),
            height: s
                .instrument_height
                .map(|h| h.to_string())
                .unwrap_or_default(),
        }
    }
}

/// The window's state while the app runs.
#[derive(Debug, Default)]
pub struct Form {
    /// The file's name and bytes (a text book is read again when its mapping changes).
    pub file: Option<String>,
    bytes: Option<Arc<Vec<u8>>>,
    pub book: Option<FieldBookRead>,
    pub mapping: Mapping,
    pub station: usize,
    /// The station's reduced row Kutupsal alım is oriented on.
    pub back: usize,
    edits: Vec<Edits>,
    /// Why the file was not read.
    pub error: Option<String>,
    /// The station shown: its observations' cells, its reduction and the
    /// observations it was made of (the table's rows reduced, in order).
    rows: Vec<[String; 9]>,
    reduction: Option<Reduction>,
    reduced_from: Vec<usize>,
}

/// An angle's columns in gon or degrees, the differences in cc or seconds.
fn marks(unit: AngleUnit) -> (&'static str, &'static str, f64) {
    match unit {
        AngleUnit::Grad => ("g", "cc", 10_000.0),
        AngleUnit::Deg => ("°", "″", 3_600.0),
    }
}

/// A number of the tables, by the display rule; empty when there is none.
fn shown(v: Option<f64>, d: usize) -> String {
    v.map(|v| fixed(v, d)).unwrap_or_default()
}

impl Form {
    /// The book's angle unit: the file's, else the mapping's, else the project's.
    pub fn unit(&self, project: AngleUnit) -> AngleUnit {
        match self.book.as_ref().and_then(|b| b.unit.as_deref()) {
            Some("deg") => AngleUnit::Deg,
            Some("grad") => AngleUnit::Grad,
            _ => self.mapping.unit.unwrap_or(project),
        }
    }

    /// Whether the book is a text book, read with the mapping.
    pub fn mapped(&self) -> bool {
        self.book.as_ref().is_some_and(|b| b.format == "csv")
    }

    /// A file opened: an instrument's by its content, a text book with the
    /// mapping remembered (or only its first line, to map).
    pub fn open(&mut self, name: String, bytes: Vec<u8>) {
        self.file = Some(name);
        self.bytes = Some(Arc::new(bytes));
        self.error = None;
        self.read();
    }

    /// The bytes read again (a text book's mapping changed); the edits start over.
    fn read(&mut self) {
        let Some(bytes) = &self.bytes else {
            return;
        };
        let book = kentos_formats::field::read(bytes, self.mapping.options().as_ref());
        self.edits = book.stations.iter().map(Edits::of).collect();
        self.station = self.station.min(book.stations.len().saturating_sub(1));
        self.book = Some(book);
    }

    /// The station shown, in the core's terms: the observations used, as named here.
    fn core_station(&self) -> Option<(Station, Vec<usize>)> {
        let st = self.book.as_ref()?.stations.get(self.station)?;
        let edits = self.edits.get(self.station)?;
        let mut from = Vec::new();
        let observations = st
            .observations
            .iter()
            .enumerate()
            .filter(|(i, _)| edits.used.get(*i).copied().unwrap_or(true))
            .map(|(i, o)| {
                from.push(i);
                Observation {
                    target: edits.names.get(i).cloned().unwrap_or_default(),
                    hz: o.hz,
                    zenith: o.zenith,
                    slope: o.slope,
                    target_height: o.target_height,
                    code: o.code.clone(),
                    line: Some(o.line as usize),
                }
            })
            .collect();
        Some((
            Station {
                station: st.station.clone(),
                instrument_height: read_number(&edits.height).filter(|v| v.is_finite()),
                observations,
            },
            from,
        ))
    }

    /// The station shown again: its cells, and its reduction with the
    /// project's k and tolerances (docs/adr/0169 §3).
    pub fn sync(&mut self, settings: &kentos_contracts::ProjectSettings) {
        self.rows.clear();
        self.reduction = None;
        self.reduced_from.clear();
        let unit = self.unit(settings.angle_unit);
        let Some((station, from)) = self.core_station() else {
            return;
        };
        let survey = settings.survey.clone().unwrap_or_default();
        let tolerances = Tolerances {
            face_hz: survey.face_hz,
            index: survey.index,
            face_slope: survey.face_slope,
        };
        let core = match unit {
            AngleUnit::Grad => Unit::GRAD,
            AngleUnit::Deg => Unit::DEG,
        };
        let reduction = reduce(&station, core, settings.refraction(), &tolerances);
        let (Some(book), Some(edits)) = (&self.book, self.edits.get(self.station)) else {
            return;
        };
        let st = &book.stations[self.station];
        let mut faces: Vec<Option<Option<usize>>> = vec![None; st.observations.len()];
        for (k, &i) in from.iter().enumerate() {
            faces[i] = reduction.faces.get(k).copied();
        }
        self.rows = st
            .observations
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let face = match faces[i] {
                    None => "—",
                    Some(Some(1)) => "I",
                    Some(Some(2)) => "II",
                    Some(Some(_)) => "Doğrultu",
                    Some(None) => "Geçersiz",
                };
                [
                    if edits.used.get(i).copied().unwrap_or(true) {
                        "1"
                    } else {
                        "0"
                    }
                    .to_owned(),
                    edits.names.get(i).cloned().unwrap_or_default(),
                    face.to_owned(),
                    fixed(o.hz, 5),
                    shown(o.zenith, 5),
                    shown(o.slope, 4),
                    shown(o.target_height, 3),
                    o.code.clone().unwrap_or_default(),
                    o.line.to_string(),
                ]
            })
            .collect();
        self.reduction = Some(reduction);
        self.reduced_from = from;
    }
}

impl Form {
    /// Kutupsal alım's fields from the station shown, its back sight row
    /// the orientation, the angles in the project's unit (docs/adr/0169 §3).
    pub fn transfer(&self, project: AngleUnit) -> Option<PolarTransfer> {
        let core = |u: AngleUnit| match u {
            AngleUnit::Grad => Unit::GRAD,
            AngleUnit::Deg => Unit::DEG,
        };
        polar_transfer(
            self.reduction.as_ref()?,
            self.back,
            core(self.unit(project)),
            core(project),
        )
    }

    /// The station shown as the file has it, and its instrument height as typed.
    fn station_shown(&self) -> Option<(&FieldStation, &str)> {
        let st = self.book.as_ref()?.stations.get(self.station)?;
        let height = self
            .edits
            .get(self.station)
            .map_or("", |e| e.height.as_str());
        Some((st, height))
    }
}

/// A value Kutupsal alım is filled with: the display rule's `d` decimals
/// (8 for angles, 6 for lengths: far below what an instrument resolves),
/// trailing zeros and a bare point dropped.
fn exact(v: f64, d: usize) -> String {
    let s = fixed(v, d);
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        s.as_str()
    };
    if s.is_empty() || s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

impl Table for Form {
    fn columns(&self) -> usize {
        COLUMNS.len()
    }

    fn rows(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, row: usize, col: usize) -> &str {
        self.rows.get(row).map_or("", |r| r[col].as_str())
    }

    /// Kullan and Nokta are the station's edits; the rest is the file's.
    fn set(&mut self, row: usize, col: usize, text: String) {
        let Some(edits) = self.edits.get_mut(self.station) else {
            return;
        };
        match col {
            USE => {
                if let Some(u) = edits.used.get_mut(row) {
                    *u = text != "0";
                }
            }
            NAME => {
                if let Some(n) = edits.names.get_mut(row) {
                    n.clone_from(&text);
                }
            }
            _ => return,
        }
        if let Some(r) = self.rows.get_mut(row) {
            r[col] = text;
        }
    }

    fn readonly(&self, _row: usize, col: usize) -> bool {
        col > NAME
    }

    fn check(&self, col: usize) -> bool {
        col == USE
    }

    fn mark(&self, row: usize) -> Option<Mark> {
        (self.rows.get(row)?[USE] == "0").then_some(Mark::Off)
    }

    fn can_insert_after(&self, _row: usize) -> bool {
        false
    }

    fn insert_after(&mut self, _row: usize) {}

    fn can_remove(&self, _row: usize) -> bool {
        false
    }

    fn remove(&mut self, _row: usize) {}
}

fn fb(e: Event) -> Message {
    event(CalcEvent::FieldBook(e))
}

impl Form {
    /// The report: the file, the station, then the reduced rows (tab-separated lines).
    pub fn report(&self) -> Option<Vec<Vec<String>>> {
        let r = self.reduction.as_ref().filter(|r| !r.rows.is_empty())?;
        let st = self.book.as_ref()?.stations.get(self.station)?;
        let height = self
            .edits
            .get(self.station)
            .map(|e| e.height.clone())
            .unwrap_or_default();
        let mut lines = vec![
            vec![TITLE.to_owned(), self.file.clone().unwrap_or_default()],
            vec![
                "İstasyon".to_owned(),
                st.station.clone(),
                "Alet yüksekliği".to_owned(),
                height,
            ],
            [
                "Nokta",
                "Durum",
                "Yatay açı",
                "Fark",
                "Başucu açısı",
                "İndeks",
                "Eğik uzunluk",
                "Fark",
                "Yatay uzunluk",
                "Kot farkı",
            ]
            .map(str::to_owned)
            .to_vec(),
        ];
        for row_ in &r.rows {
            lines.push(vec![
                row_.target.clone(),
                row_.faces.to_string(),
                fixed(row_.hz, 5),
                shown(row_.hz_diff, 5),
                shown(row_.zenith, 5),
                shown(row_.index, 5),
                shown(row_.slope, 4),
                shown(row_.slope_diff, 4),
                shown(row_.horizontal, 4),
                shown(row_.dh, 4),
            ]);
        }
        Some(lines)
    }
}

mod apply;
mod view;

#[cfg(test)]
mod tests;

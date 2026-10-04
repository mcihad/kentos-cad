//! Topcon GTS-7 field books (docs/adr/0169 §1): a control word and its
//! comma-separated fields per line, read into stations and observations as
//! Topcon describes them (Topcon Link Reference Manual, P/N 7010-0522,
//! Appendix C "GTS-7 Raw Format", with its sample file). Every record not
//! read is named with the reason. The independent reference is
//! `scripts/fixtures/field_gts7_cases.py` (fixtures/field/v1/gts7.json).
//! A value is its exact decimal or sexagesimal value rounded once to the
//! nearest float64 (CLAUDE.md §23).

use std::cmp::Ordering;

use kentos_contracts::{FieldBookRead, FieldObservation, FieldStation, LineError};

use super::exact::{Dms, Exact};
use crate::text;

/// What is said of a record not read; `{line}`, `{data}`, `{target}` and
/// `{at}` are filled in.
pub const UNITS: &str = "Satır {line}: GTS-7 UNITS kaydı “{data}” okunmuyor; yalnız metre (M) ile derece (D) ya da gon (G); ölçüler okunmadı.";
pub const MIXED: &str = "Satır {line}: GTS-7 UNITS kaydının açı birimi karnenin birimiyle aynı değil; ölçüler okunmadı.";
pub const BEFORE: &str =
    "Satır {line}: GTS-7 UNITS kaydından önce; birim bilinmediğinden ölçüler okunmadı.";
pub const VALUE: &str = "Satır {line}: “{data}” sayı değil; satır okunmadı.";
pub const DMS: &str =
    "Satır {line}: açı “{data}” DDD.MMSS değil (dakika ya da saniye 60'tan büyük); satır okunmadı.";
pub const TURN: &str = "Satır {line}: açı “{data}” bir tam dönüşten büyük; satır okunmadı.";
pub const ZENITH: &str = "Satır {line}: başucu açısı “{data}” sıfırdan küçük; satır okunmadı.";
pub const NEGATIVE: &str = "Satır {line}: eğik uzunluk “{data}” sıfırdan küçük; satır okunmadı.";
pub const POINT: &str = "Satır {line}: ölçüden önce nokta kaydı (BS, FS, SS) yok; satır okunmadı.";
pub const HORIZONTAL: &str = "Satır {line}: {target} gözleminin yatay açısı yok; satır okunmadı.";
pub const REDUCED: &str = "Satır {line}: HD kaydı indirgenmiş ölçü (yatay uzunluk, kot farkı); ham gözlem değil, satır okunmadı.";
pub const OFFSET: &str =
    "Satır {line}: dışmerkez ölçü (OFFSET) okunmuyor; {target} gözlemi (satır {at}) okunmadı.";

/// GTS-7's control words; any other line is passed over.
const WORDS: [&str; 21] = [
    "GTS-700", "JOB", "DATE", "NAME", "INST", "UNITS", "SCALE", "ATMOS", "TEMP", "STN", "XYZ",
    "BKB", "BS", "FS", "SS", "CTL", "HV", "SD", "HD", "OFFSET", "NOTE",
];

/// An angle unit.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Unit {
    Deg,
    Grad,
}

impl Unit {
    fn full(self) -> i128 {
        match self {
            Self::Deg => 360,
            Self::Grad => 400,
        }
    }
}

/// The units in force: none yet, a UNITS not read, or read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Units {
    Unknown,
    Refused,
    Known(Unit),
}

/// The point the next measurements are of.
struct Point {
    name: String,
    target_height: Option<f64>,
    code: String,
}

/// Why a record is not read: its text and the field's text.
struct Unread(&'static str, String);

/// A number field: blank is none.
fn number(data: &str) -> Result<Option<Exact>, Unread> {
    if data.is_empty() {
        return Ok(None);
    }
    Exact::decimal(data)
        .map(Some)
        .ok_or_else(|| Unread(VALUE, data.to_owned()))
}

/// An angle field in the unit: DDD.MMSS for degrees, decimal gon; more than
/// a full turn is not read; below zero a horizontal reading is turned into
/// its turn, a zenith not read.
fn angle(data: &str, unit: Unit, horizontal: bool) -> Result<Option<Exact>, Unread> {
    if data.is_empty() {
        return Ok(None);
    }
    let v = match unit {
        Unit::Deg => Exact::dms(data).map_err(|e| match e {
            Dms::Number => Unread(VALUE, data.to_owned()),
            Dms::Sexagesimal => Unread(DMS, data.to_owned()),
        })?,
        Unit::Grad => Exact::decimal(data).ok_or_else(|| Unread(VALUE, data.to_owned()))?,
    };
    if v.size_cmp(unit.full()) == Ordering::Greater {
        return Err(Unread(TURN, data.to_owned()));
    }
    if v.negative() {
        if !horizontal {
            return Err(Unread(ZENITH, data.to_owned()));
        }
        return Ok(Some(v.plus(unit.full())));
    }
    Ok(Some(v))
}

/// A problem: `{line}`, then the placeholders in order; the file's own text
/// goes last, so that nothing in it is taken for a placeholder.
fn said(text: &str, line: u32, fill: &[(&str, &str)]) -> LineError {
    let mut message = text.replace("{line}", &line.to_string());
    for (k, v) in fill {
        message = message.replace(k, v);
    }
    LineError { line, message }
}

/// The book being read.
struct Book {
    read: FieldBookRead,
    units: Units,
    unit: Option<Unit>,
    before_said: bool,
    station: Option<usize>,
    point: Option<Point>,
    last_target_height: Option<f64>,
}

impl Book {
    /// The station observations go to: the current one, or the book's
    /// first, unnamed.
    fn station(&mut self) -> usize {
        match self.station {
            Some(at) => at,
            None => {
                self.read.stations.push(FieldStation {
                    station: String::new(),
                    instrument_height: None,
                    east: None,
                    north: None,
                    height: None,
                    observations: Vec::new(),
                });
                *self.station.insert(self.read.stations.len() - 1)
            }
        }
    }

    /// A record; `here` is the record before's word, `observation` the
    /// observation it read (station and index). Returns the observation
    /// this one read.
    fn record(
        &mut self,
        line: u32,
        word: &str,
        fields: &[&str],
        here: Option<&str>,
        observation: Option<(usize, usize)>,
    ) -> Result<Option<(usize, usize)>, Unread> {
        let get = |k: usize| fields.get(k).copied().unwrap_or("");
        match word {
            "UNITS" => {
                let first = |s: &str| s.chars().next().map(|c| c.to_ascii_uppercase());
                match (first(get(0)), first(get(1))) {
                    (Some('M'), Some(a @ ('D' | 'G'))) => {
                        let unit = if a == 'D' { Unit::Deg } else { Unit::Grad };
                        if *self.unit.get_or_insert(unit) == unit {
                            self.units = Units::Known(unit);
                        } else {
                            self.units = Units::Refused;
                            self.read.problems.push(said(MIXED, line, &[]));
                        }
                    }
                    _ => {
                        self.units = Units::Refused;
                        self.read.problems.push(said(
                            UNITS,
                            line,
                            &[("{data}", &fields.join(", "))],
                        ));
                    }
                }
            }
            "STN" => {
                let height = number(get(1))?;
                self.read.stations.push(FieldStation {
                    station: get(0).to_owned(),
                    instrument_height: height.map(Exact::value),
                    east: None,
                    north: None,
                    height: None,
                    observations: Vec::new(),
                });
                self.station = Some(self.read.stations.len() - 1);
                self.point = None;
                self.last_target_height = None;
            }
            "XYZ" => {
                if here == Some("STN")
                    && let Some(at) = self.station
                {
                    let (e, n, z) = (number(get(0))?, number(get(1))?, number(get(2))?);
                    let s = &mut self.read.stations[at];
                    s.east = e.map(Exact::value).or(s.east);
                    s.north = n.map(Exact::value).or(s.north);
                    s.height = z.map(Exact::value).or(s.height);
                }
            }
            "BS" | "FS" | "SS" => {
                if let Some(h) = number(get(1))? {
                    self.last_target_height = Some(h.value());
                }
                self.point = Some(Point {
                    name: get(0).to_owned(),
                    target_height: self.last_target_height,
                    code: if word == "BS" {
                        String::new()
                    } else {
                        get(2).to_owned()
                    },
                });
            }
            "HV" | "SD" | "HD" => {
                let unit = match self.units {
                    Units::Unknown => {
                        if !self.before_said {
                            self.read.problems.push(said(BEFORE, line, &[]));
                            self.before_said = true;
                        }
                        return Ok(None);
                    }
                    Units::Refused => return Ok(None),
                    Units::Known(unit) => unit,
                };
                if word == "HD" {
                    self.read.problems.push(said(REDUCED, line, &[]));
                    return Ok(None);
                }
                let hz = angle(get(0), unit, true)?;
                let zenith = angle(get(1), unit, false)?;
                let slope = if word == "SD" { number(get(2))? } else { None };
                if slope.is_some_and(Exact::negative) {
                    return Err(Unread(NEGATIVE, get(2).to_owned()));
                }
                let Some(point) = &self.point else {
                    self.read.problems.push(said(POINT, line, &[]));
                    return Ok(None);
                };
                let Some(hz) = hz else {
                    let target = point.name.clone();
                    self.read
                        .problems
                        .push(said(HORIZONTAL, line, &[("{target}", &target)]));
                    return Ok(None);
                };
                let observation = FieldObservation {
                    target: point.name.clone(),
                    hz: hz.value(),
                    zenith: zenith.map(Exact::value),
                    slope: slope.map(Exact::value),
                    target_height: point.target_height,
                    code: (!point.code.is_empty()).then(|| point.code.clone()),
                    line,
                };
                let at = self.station();
                let observations = &mut self.read.stations[at].observations;
                observations.push(observation);
                return Ok(Some((at, observations.len() - 1)));
            }
            "OFFSET" => {
                let offsets = [number(get(0))?, number(get(1))?, number(get(2))?];
                if let Some((at, k)) = observation
                    && offsets.iter().flatten().any(|v| v.nonzero())
                {
                    let o = self.read.stations[at].observations.remove(k);
                    self.read.problems.push(said(
                        OFFSET,
                        line,
                        &[("{at}", &o.line.to_string()), ("{target}", &o.target)],
                    ));
                }
            }
            _ => {}
        }
        Ok(None)
    }
}

/// Reads a Topcon GTS-7 field book.
pub fn read(bytes: &[u8]) -> FieldBookRead {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut book = Book {
        read: FieldBookRead {
            format: "gts7".to_owned(),
            encoding: enc.label().to_owned(),
            unit: None,
            first_line: Vec::new(),
            stations: Vec::new(),
            problems: Vec::new(),
        },
        units: Units::Unknown,
        unit: None,
        before_said: false,
        station: None,
        point: None,
        last_target_height: None,
    };
    // The record before's word (none for a line that is not a record) and
    // the observation it read.
    let mut previous: Option<&'static str> = None;
    let mut last: Option<(usize, usize)> = None;
    for (i, raw) in body.split('\n').enumerate() {
        let line = u32::try_from(i + 1).unwrap_or(u32::MAX);
        let s = raw.trim();
        if s.is_empty() {
            continue;
        }
        let (head, rest) = match s.split_once(char::is_whitespace) {
            Some((head, rest)) => (head, rest.trim_start()),
            None => (s, ""),
        };
        let upper = head.to_uppercase();
        let word = WORDS.iter().copied().find(|w| *w == upper);
        let here = previous;
        previous = word;
        let observation = last.take();
        let Some(word) = word else {
            continue;
        };
        let fields: Vec<&str> = if rest.is_empty() {
            Vec::new()
        } else {
            rest.split(',').map(str::trim).collect()
        };
        match book.record(line, word, &fields, here, observation) {
            Ok(read) => last = read,
            Err(Unread(text, data)) => {
                book.read
                    .problems
                    .push(said(text, line, &[("{data}", &data)]));
            }
        }
    }
    let mut read = book.read;
    read.unit = book.unit.map(|u| {
        match u {
            Unit::Grad => "grad",
            Unit::Deg => "deg",
        }
        .to_owned()
    });
    read
}

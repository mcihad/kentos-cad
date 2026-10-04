//! Nikon RAW field books (docs/adr/0169 §1): comma-separated records read
//! into stations and observations as Nikon describes them (Total Station
//! Nivo Series Instruction Manual, Nikon-Trimble: "Nikon raw record
//! formats" and "Data examples", Nikon RAW data format V2.00); the units
//! from the download's comment records. Every record not read is named with
//! the reason. The independent reference is
//! `scripts/fixtures/field_nikon_cases.py` (fixtures/field/v1/nikon.json).
//! A value is its exact decimal or sexagesimal value rounded once to the
//! nearest float64 (CLAUDE.md §23).

use std::cmp::Ordering;
use std::collections::HashMap;

use kentos_contracts::{FieldBookRead, FieldObservation, FieldStation, LineError};

use super::exact::{Dms, Exact};
use crate::text;

/// What is said of a record not read; `{line}`, `{data}` and `{target}` are
/// filled in.
pub const DISTANCE: &str =
    "Satır {line}: Nikon RAW uzunluk birimi “{data}” okunmuyor; yalnız metre; ölçüler okunmadı.";
pub const ANGLES: &str = "Satır {line}: Nikon RAW açı birimi “{data}” okunmuyor; yalnız DDDMMSS ve gon; ölçüler okunmadı.";
pub const VERTICAL: &str = "Satır {line}: Nikon RAW düşey açı başlangıcı “{data}” okunmuyor; yalnız Zenith ve Horizon; ölçüler okunmadı.";
pub const MIXED: &str =
    "Satır {line}: Nikon RAW açı birimi karnenin birimiyle aynı değil; ölçüler okunmadı.";
pub const BEFORE: &str = "Satır {line}: Nikon RAW birimleri (CO,Dist Units ve CO,Angle Units) bu satırdan önce yazılı değil; ölçüler okunmadı.";
pub const ZENITH: &str = "Satır {line}: Nikon RAW düşey açı başlangıcı (CO,Zero VA) yazılı değil; düşey açılar başucu açısı sayıldı.";
pub const VALUE: &str = "Satır {line}: “{data}” sayı değil; satır okunmadı.";
pub const DMS: &str =
    "Satır {line}: açı “{data}” DDD.MMSS değil (dakika ya da saniye 60'tan büyük); satır okunmadı.";
pub const TURN: &str = "Satır {line}: açı “{data}” bir tam dönüşten büyük; satır okunmadı.";
pub const BELOW: &str = "Satır {line}: başucu açısı “{data}” sıfırdan küçük; satır okunmadı.";
pub const NEGATIVE: &str = "Satır {line}: eğik uzunluk “{data}” sıfırdan küçük; satır okunmadı.";
pub const TARGET: &str = "Satır {line}: gözlemin hedef noktası yok; satır okunmadı.";
pub const HORIZONTAL: &str = "Satır {line}: {target} gözleminin yatay açısı yok; satır okunmadı.";

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

/// A unit from the comments: not given yet, not read, or read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Given<T> {
    No,
    Refused,
    Yes(T),
}

/// An observation record's fields: target (and the alternative), target
/// height, slope distance, horizontal, vertical, code.
struct Fields {
    target: usize,
    alternative: Option<usize>,
    target_height: usize,
    slope: usize,
    horizontal: usize,
    vertical: usize,
    code: Option<usize>,
}

/// F1 and F2 (face, pt, ht, sd, ha, va, time), SS (pt, ht, sd, ha, va,
/// time, code), CP (pt, pt id, ht, sd, ha, va, time, code), SO (pt,
/// original pt, ht, sd, ha, va, time).
fn fields(kind: &str) -> Option<Fields> {
    let (target, alternative, target_height, slope, horizontal, vertical, code) = match kind {
        "F1" | "F2" => (1, None, 2, 3, 4, 5, None),
        "SS" => (1, None, 2, 3, 4, 5, Some(7)),
        "CP" => (1, Some(2), 3, 4, 5, 6, Some(8)),
        "SO" => (1, None, 3, 4, 5, 6, None),
        _ => return None,
    };
    Some(Fields {
        target,
        alternative,
        target_height,
        slope,
        horizontal,
        vertical,
        code,
    })
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

/// An angle field in the unit (DDD.MMSS for degrees, decimal gon); more
/// than a full turn is not read.
fn angle(data: &str, unit: Unit) -> Result<Option<Exact>, Unread> {
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

/// A point's coordinates: east, north, height.
type Place = (Option<f64>, Option<f64>, Option<f64>);

/// The book being read.
struct Book {
    read: FieldBookRead,
    metres: Given<()>,
    unit: Given<Unit>,
    /// The vertical angles from the horizon (a zenith otherwise).
    horizon: Given<bool>,
    book: Option<Unit>,
    before_said: bool,
    zenith_said: bool,
    /// The last coordinates of each name: east, north, height.
    coordinates: HashMap<String, Place>,
    station: Option<usize>,
    back: String,
    last_target_height: Option<f64>,
}

impl Book {
    /// A comment that sets a unit.
    fn comment(&mut self, line: u32, text: &str) {
        let Some((key, value)) = text.split_once(':') else {
            return;
        };
        let (key, value) = (key.trim().to_lowercase(), value.trim());
        let lower = value.to_lowercase();
        match key.as_str() {
            "dist units" => {
                if lower.starts_with("met") {
                    self.metres = Given::Yes(());
                } else {
                    self.metres = Given::Refused;
                    self.read
                        .problems
                        .push(said(DISTANCE, line, &[("{data}", value)]));
                }
            }
            "angle units" => {
                let unit = match lower.as_str() {
                    "dddmmss" => Unit::Deg,
                    "gon" | "gons" | "grad" | "grads" => Unit::Grad,
                    _ => {
                        self.unit = Given::Refused;
                        self.read
                            .problems
                            .push(said(ANGLES, line, &[("{data}", value)]));
                        return;
                    }
                };
                if *self.book.get_or_insert(unit) == unit {
                    self.unit = Given::Yes(unit);
                } else {
                    self.unit = Given::Refused;
                    self.read.problems.push(said(MIXED, line, &[]));
                }
            }
            "zero va" => match lower.as_str() {
                "zenith" => self.horizon = Given::Yes(false),
                "horizon" => self.horizon = Given::Yes(true),
                _ => {
                    self.horizon = Given::Refused;
                    self.read
                        .problems
                        .push(said(VERTICAL, line, &[("{data}", value)]));
                }
            },
            _ => {}
        }
    }

    /// A record.
    fn record(&mut self, line: u32, kind: &str, f: &[&str]) -> Result<(), Unread> {
        let get = |k: usize| f.get(k).copied().unwrap_or("");
        match kind {
            "UP" | "MP" | "CC" | "RE" => {
                let (n, e, z) = (number(get(3))?, number(get(4))?, number(get(5))?);
                let at = (
                    e.map(Exact::value),
                    n.map(Exact::value),
                    z.map(Exact::value),
                );
                for name in [get(1), get(2)] {
                    if !name.is_empty() {
                        self.coordinates.insert(name.to_owned(), at);
                    }
                }
            }
            "ST" => {
                let name = if get(1).is_empty() { get(2) } else { get(1) };
                let height = number(get(5))?;
                let (east, north, z) = self
                    .coordinates
                    .get(name)
                    .copied()
                    .unwrap_or((None, None, None));
                self.read.stations.push(FieldStation {
                    station: name.to_owned(),
                    instrument_height: height.map(Exact::value),
                    east,
                    north,
                    height: z,
                    observations: Vec::new(),
                });
                self.station = Some(self.read.stations.len() - 1);
                self.back = if get(3).is_empty() { get(4) } else { get(3) }.to_owned();
                self.last_target_height = None;
            }
            _ => {
                let Some(at) = fields(kind) else {
                    return Ok(());
                };
                let (Given::Yes(()), Given::Yes(unit)) = (self.metres, self.unit) else {
                    if (matches!(self.metres, Given::No) || matches!(self.unit, Given::No))
                        && !self.before_said
                    {
                        self.read.problems.push(said(BEFORE, line, &[]));
                        self.before_said = true;
                    }
                    return Ok(());
                };
                if self.horizon == Given::Refused {
                    return Ok(());
                }
                let hz = angle(get(at.horizontal), unit)?;
                let va = angle(get(at.vertical), unit)?;
                let slope = number(get(at.slope))?;
                let target_height = number(get(at.target_height))?;
                if slope.is_some_and(Exact::negative) {
                    return Err(Unread(NEGATIVE, get(at.slope).to_owned()));
                }
                let hz = hz.map(|v| if v.negative() { v.plus(unit.full()) } else { v });
                let zenith = match (va, self.horizon) {
                    (Some(v), Given::Yes(true)) => {
                        Some(v.zenith_of_elevation(unit.full() / 4, unit.full()))
                    }
                    (v, _) => v,
                };
                if zenith.is_some_and(Exact::negative) {
                    return Err(Unread(BELOW, get(at.vertical).to_owned()));
                }
                let mut target = get(at.target).to_owned();
                if target.is_empty()
                    && let Some(k) = at.alternative
                {
                    target = get(k).to_owned();
                }
                if target.is_empty() && matches!(kind, "F1" | "F2") && self.station.is_some() {
                    target = self.back.clone();
                }
                if target.is_empty() {
                    self.read.problems.push(said(TARGET, line, &[]));
                    return Ok(());
                }
                let Some(hz) = hz else {
                    self.read
                        .problems
                        .push(said(HORIZONTAL, line, &[("{target}", &target)]));
                    return Ok(());
                };
                if self.horizon == Given::No && va.is_some() && !self.zenith_said {
                    self.read.problems.push(said(ZENITH, line, &[]));
                    self.zenith_said = true;
                }
                let station = match self.station {
                    Some(s) => s,
                    None => {
                        self.read.stations.push(FieldStation {
                            station: String::new(),
                            instrument_height: None,
                            east: None,
                            north: None,
                            height: None,
                            observations: Vec::new(),
                        });
                        self.back.clear();
                        self.last_target_height = None;
                        *self.station.insert(self.read.stations.len() - 1)
                    }
                };
                if let Some(h) = target_height {
                    self.last_target_height = Some(h.value());
                }
                let code = at
                    .code
                    .map(get)
                    .filter(|c| !c.is_empty())
                    .map(str::to_owned);
                self.read.stations[station]
                    .observations
                    .push(FieldObservation {
                        target,
                        hz: hz.value(),
                        zenith: zenith.map(Exact::value),
                        slope: slope.map(Exact::value),
                        target_height: self.last_target_height,
                        code,
                        line,
                    });
            }
        }
        Ok(())
    }
}

/// Reads a Nikon RAW field book.
pub fn read(bytes: &[u8]) -> FieldBookRead {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut book = Book {
        read: FieldBookRead {
            format: "nikon".to_owned(),
            encoding: enc.label().to_owned(),
            unit: None,
            first_line: Vec::new(),
            stations: Vec::new(),
            problems: Vec::new(),
        },
        metres: Given::No,
        unit: Given::No,
        horizon: Given::No,
        book: None,
        before_said: false,
        zenith_said: false,
        coordinates: HashMap::new(),
        station: None,
        back: String::new(),
        last_target_height: None,
    };
    for (i, raw) in body.split('\n').enumerate() {
        let line = u32::try_from(i + 1).unwrap_or(u32::MAX);
        if raw.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = raw.split(',').map(str::trim).collect();
        let kind = f[0].to_uppercase();
        if kind == "CO" {
            book.comment(line, &f[1..].join(","));
            continue;
        }
        if let Err(Unread(text, data)) = book.record(line, &kind, &f) {
            book.read
                .problems
                .push(said(text, line, &[("{data}", &data)]));
        }
    }
    let mut read = book.read;
    read.unit = book.book.map(|u| {
        match u {
            Unit::Grad => "grad",
            Unit::Deg => "deg",
        }
        .to_owned()
    });
    read
}

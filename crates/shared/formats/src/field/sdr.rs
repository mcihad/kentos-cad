//! Sokkia SDR field books (docs/adr/0169 §1): SDR2x (4-digit point
//! numbers, 10-character reals) and SDR33 (14-character point names,
//! 16-character fields), read into stations and observations as Sokkia's
//! description says ("Interfacing with the SOKKIA SDR Electronic Field
//! Book", software version 04-04.xx, 1999: chapter 3, the record formats of
//! 3.6.1 and 3.6.2). Every record not read is named with the reason. The
//! independent reference is `scripts/fixtures/field_sdr_cases.py`
//! (fixtures/field/v1/sdr.json). A value is its exact decimal rounded once
//! to the nearest float64 (CLAUDE.md §23).

use kentos_contracts::{FieldBookRead, FieldObservation, FieldStation, LineError};

use crate::text;

/// What is said of a record not read; `{line}`, `{unit}`, `{option}`,
/// `{data}`, `{target}` and `{count}` are filled in.
pub const BEFORE: &str =
    "Satır {line}: SDR başlık kaydından (00) önce; başlığa kadar kayıtlar okunmadı.";
pub const ANGLE_UNIT: &str =
    "Satır {line}: SDR işinin açı birimi ({unit}) okunmuyor; yalnız derece ve gon; iş okunmadı.";
pub const DISTANCE_UNIT: &str =
    "Satır {line}: SDR işinin uzunluk birimi ({unit}) okunmuyor; yalnız metre; iş okunmadı.";
pub const DIRECTION: &str = "Satır {line}: SDR işinin açı yönü seçeneği ({option}) okunmuyor; yalnız 1 (sağa); iş okunmadı.";
pub const MIXED: &str =
    "Satır {line}: SDR işinin açı birimi karnenin birimiyle aynı değil; iş okunmadı.";
pub const VALUE: &str = "Satır {line}: “{data}” sayı değil; satır okunmadı.";
pub const TURN: &str =
    "Satır {line}: açı “{data}” sıfırla bir tam dönüş arasında değil; satır okunmadı.";
pub const NEGATIVE: &str = "Satır {line}: eğik uzunluk “{data}” sıfırdan küçük; satır okunmadı.";
pub const TARGET: &str = "Satır {line}: gözlemin hedef noktası yok; satır okunmadı.";
pub const HORIZONTAL: &str = "Satır {line}: {target} gözleminin yatay açısı yok; satır okunmadı.";
pub const ZENITH: &str =
    "Satır {line}: işte alet kaydı (01) yok; düşey açılar başucu açısı sayıldı.";
pub const CORRECTED: &str = "Satır {line}: düzeltilmiş gözlemler (09MC, {count} kayıt) okunmadı; ham gözlemler (F1, F2) okunur.";
pub const COORDINATES: &str =
    "Satır {line}: koordinat kayıtları (08, {count} kayıt) karne gözlemi değil; okunmadı.";

/// A job's angle unit.
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

    fn quarter(self) -> i128 {
        self.full() / 4
    }
}

/// A record's fields' places (from, to; counted from 1, both included).
struct Layout {
    station: [(usize, usize); 5],
    target: (usize, usize),
    observation: [(usize, usize); 6],
}

/// SDR2x: station point, north, east, elevation, instrument height; the
/// target height; an observation's from, to, slope distance, vertical,
/// horizontal, description.
const SDR2X: Layout = Layout {
    station: [(5, 8), (9, 18), (19, 28), (29, 38), (39, 48)],
    target: (5, 14),
    observation: [(5, 8), (9, 12), (13, 22), (23, 32), (33, 42), (43, 58)],
};

/// SDR33, the same fields 16 characters wide.
const SDR33: Layout = Layout {
    station: [(5, 20), (21, 36), (37, 52), (53, 68), (69, 84)],
    target: (5, 20),
    observation: [(5, 20), (21, 36), (37, 52), (53, 68), (69, 84), (85, 100)],
};

/// The job being read.
struct Job {
    layout: &'static Layout,
    unit: Unit,
    /// An instrument record (01) has been read; its vertical option is 2.
    instrument: bool,
    from_horizon: bool,
    target: Option<f64>,
    said: bool,
}

/// A real's exact decimal: its digits as an integer, its decimals, its sign.
#[derive(Clone, Copy)]
struct Decimal {
    units: i128,
    places: u32,
}

impl Decimal {
    /// The decimal's nearest float64 (a negative zero is zero).
    fn value(self) -> f64 {
        let scale = 10i128.pow(self.places);
        let (whole, fraction) = (self.units.abs() / scale, self.units.abs() % scale);
        let sign = if self.units < 0 { "-" } else { "" };
        let width = self.places as usize;
        let text = if width == 0 {
            format!("{sign}{whole}")
        } else {
            format!("{sign}{whole}.{fraction:0width$}")
        };
        text.parse::<f64>().unwrap_or(f64::NAN) + 0.0
    }

    /// Below zero.
    fn negative(self) -> bool {
        self.units < 0
    }

    /// From zero up to a full turn of `unit` (not included).
    fn in_turn(self, unit: Unit) -> bool {
        self.units >= 0 && self.units < unit.full() * 10i128.pow(self.places)
    }

    /// A quarter turn less the decimal, within a turn: the zenith of a
    /// reading from the horizon, exactly.
    fn zenith_of_elevation(self, unit: Unit) -> Self {
        let scale = 10i128.pow(self.places);
        Self {
            units: (unit.quarter() * scale - self.units).rem_euclid(unit.full() * scale),
            places: self.places,
        }
    }
}

/// Why a record is not read: its key and the field's text.
struct Unread(&'static str, String);

/// A real field: blank is none; an optional minus, at least one digit, an
/// optional point and digits (the description's format); else not read.
fn real(data: &str) -> Result<Option<Decimal>, Unread> {
    if data.is_empty() {
        return Ok(None);
    }
    let bad = || Unread(VALUE, data.to_owned());
    let (minus, body) = match data.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, data),
    };
    let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    // At most 30 digits: well within an i128 and a quarter turn's scaling.
    if whole.is_empty() || !digits(whole) || !digits(fraction) || whole.len() + fraction.len() > 30
    {
        return Err(bad());
    }
    let units: i128 = format!("{whole}{fraction}").parse().map_err(|_| bad())?;
    Ok(Some(Decimal {
        units: if minus { -units } else { units },
        places: u32::try_from(fraction.len()).map_err(|_| bad())?,
    }))
}

/// An angle field: a real from zero up to a full turn.
fn angle(data: &str, unit: Unit) -> Result<Option<Decimal>, Unread> {
    let v = real(data)?;
    if v.is_some_and(|v| !v.in_turn(unit)) {
        return Err(Unread(TURN, data.to_owned()));
    }
    Ok(v)
}

/// A field's text: its places counted from 1, both included, trimmed;
/// blank past the line's end.
fn field(line: &[char], (from, to): (usize, usize)) -> String {
    line.iter()
        .skip(from - 1)
        .take(to + 1 - from)
        .collect::<String>()
        .trim()
        .to_owned()
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

/// The book being read: what has been read, the job in force, the station
/// observations go to, the corrected observations' and the coordinate
/// records' first line and count.
struct Book {
    read: FieldBookRead,
    unit: Option<Unit>,
    job: Option<Job>,
    station: Option<usize>,
    corrected: Option<(u32, usize)>,
    coordinates: Option<(u32, usize)>,
}

impl Book {
    /// A header (00): a job read, or said and passed over.
    fn header(&mut self, line: u32, chars: &[char]) {
        self.job = None;
        self.station = None;
        let option = |at: usize| {
            chars
                .get(at)
                .map_or_else(|| " ".to_owned(), char::to_string)
        };
        let (angle_unit, distance_unit, direction) = (option(40), option(41), option(45));
        let unit = match angle_unit.as_str() {
            "1" => Unit::Deg,
            "2" => Unit::Grad,
            _ => {
                self.read
                    .problems
                    .push(said(ANGLE_UNIT, line, &[("{unit}", &angle_unit)]));
                return;
            }
        };
        if distance_unit != "1" {
            self.read
                .problems
                .push(said(DISTANCE_UNIT, line, &[("{unit}", &distance_unit)]));
            return;
        }
        if direction != "1" {
            self.read
                .problems
                .push(said(DIRECTION, line, &[("{option}", &direction)]));
            return;
        }
        if *self.unit.get_or_insert(unit) != unit {
            self.read.problems.push(said(MIXED, line, &[]));
            return;
        }
        self.job = Some(Job {
            layout: if field(chars, (5, 20)).starts_with("SDR33") {
                &SDR33
            } else {
                &SDR2X
            },
            unit,
            instrument: false,
            from_horizon: false,
            target: None,
            said: false,
        });
    }

    /// A record of the job: an instrument, a station, a target height or an
    /// observation read; corrected observations and coordinates counted.
    fn record(&mut self, line: u32, chars: &[char]) -> Result<(), Unread> {
        let Some(job) = self.job.as_mut() else {
            return Ok(());
        };
        let kind: String = chars.iter().take(2).collect();
        let derivation: String = chars.iter().skip(2).take(2).collect();
        let layout = job.layout;
        match (kind.as_str(), derivation.as_str()) {
            ("01", _) => {
                job.instrument = true;
                job.from_horizon = chars.get(50) == Some(&'2');
            }
            ("02", _) => {
                let [point, north, east, height, instrument] =
                    layout.station.map(|at| field(chars, at));
                let north = real(&north)?;
                let east = real(&east)?;
                let height = real(&height)?;
                let instrument = real(&instrument)?;
                self.read.stations.push(FieldStation {
                    station: point,
                    instrument_height: instrument.map(Decimal::value),
                    east: east.map(Decimal::value),
                    north: north.map(Decimal::value),
                    height: height.map(Decimal::value),
                    observations: Vec::new(),
                });
                self.station = Some(self.read.stations.len() - 1);
            }
            ("03", _) => {
                job.target = real(&field(chars, layout.target))?.map(Decimal::value);
            }
            ("09", "F1" | "F2" | "MD") => {
                let [from, to, slope_at, vertical_at, horizontal_at, code] =
                    layout.observation.map(|at| field(chars, at));
                let hz = angle(&horizontal_at, job.unit)?;
                let vertical = angle(&vertical_at, job.unit)?;
                let slope = real(&slope_at)?;
                if slope.is_some_and(Decimal::negative) {
                    return Err(Unread(NEGATIVE, slope_at));
                }
                if to.is_empty() {
                    self.read.problems.push(said(TARGET, line, &[]));
                    return Ok(());
                }
                let Some(hz) = hz else {
                    self.read
                        .problems
                        .push(said(HORIZONTAL, line, &[("{target}", &to)]));
                    return Ok(());
                };
                if !job.instrument && !job.said {
                    self.read.problems.push(said(ZENITH, line, &[]));
                    job.said = true;
                }
                let zenith = vertical.map(|v| {
                    if job.from_horizon {
                        v.zenith_of_elevation(job.unit)
                    } else {
                        v
                    }
                });
                let target_height = job.target;
                let at = match self.station {
                    Some(at) if self.read.stations[at].station == from => at,
                    _ => {
                        self.read.stations.push(FieldStation {
                            station: from,
                            instrument_height: None,
                            east: None,
                            north: None,
                            height: None,
                            observations: Vec::new(),
                        });
                        *self.station.insert(self.read.stations.len() - 1)
                    }
                };
                self.read.stations[at].observations.push(FieldObservation {
                    target: to,
                    hz: hz.value(),
                    zenith: zenith.map(Decimal::value),
                    slope: slope.map(Decimal::value),
                    target_height,
                    code: (!code.is_empty()).then_some(code),
                    line,
                });
            }
            ("09", "MC") => self.corrected.get_or_insert((line, 0)).1 += 1,
            ("08", _) => self.coordinates.get_or_insert((line, 0)).1 += 1,
            _ => {}
        }
        Ok(())
    }
}

/// Reads a Sokkia SDR2x or SDR33 field book.
pub fn read(bytes: &[u8]) -> FieldBookRead {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut book = Book {
        read: FieldBookRead {
            format: "sdr".to_owned(),
            encoding: enc.label().to_owned(),
            unit: None,
            first_line: Vec::new(),
            stations: Vec::new(),
            problems: Vec::new(),
        },
        unit: None,
        job: None,
        station: None,
        corrected: None,
        coordinates: None,
    };
    // Before the first header its records are said once; after a header not
    // read they are passed over without a word.
    let mut header_seen = false;
    let mut before_said = false;
    for (i, raw) in body.split('\n').enumerate() {
        let line = u32::try_from(i + 1).unwrap_or(u32::MAX);
        if raw.trim().is_empty() || raw.starts_with(['\u{2}', '\u{3}']) {
            continue;
        }
        let chars: Vec<char> = raw.chars().collect();
        if chars.starts_with(&['0', '0']) {
            header_seen = true;
            book.header(line, &chars);
            continue;
        }
        if !header_seen {
            if !before_said {
                book.read.problems.push(said(BEFORE, line, &[]));
                before_said = true;
            }
            continue;
        }
        if let Err(Unread(text, data)) = book.record(line, &chars) {
            book.read
                .problems
                .push(said(text, line, &[("{data}", &data)]));
        }
    }
    for (text, kind) in [(CORRECTED, book.corrected), (COORDINATES, book.coordinates)] {
        if let Some((line, count)) = kind {
            book.read
                .problems
                .push(said(text, line, &[("{count}", &count.to_string())]));
        }
    }
    let mut read = book.read;
    read.problems.sort_by_key(|p| p.line);
    read.unit = book.unit.map(|u| {
        match u {
            Unit::Grad => "grad",
            Unit::Deg => "deg",
        }
        .to_owned()
    });
    read
}

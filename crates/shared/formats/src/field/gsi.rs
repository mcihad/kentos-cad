//! Leica GSI field books (docs/adr/0169 §1): GSI-8 and GSI-16, each line a
//! block of words, read into stations and observations as Leica's
//! description says ("GSI ONLINE for Leica TPS and DNA", 2003): a word's
//! first two characters are its index (WI), its sixth the units, its
//! seventh the sign, the rest its data. Every block that is not read is
//! named with the reason. The independent reference is
//! `scripts/fixtures/field_gsi_cases.py` (fixtures/field/v1/gsi.json).
//! A value is its exact decimal or sexagesimal value rounded once to the
//! nearest float64 (CLAUDE.md §23).

use kentos_contracts::{FieldBookRead, FieldObservation, FieldStation, LineError};

use crate::text;

/// What is said of a block not read; `{line}`, `{word}`, `{wi}`, `{unit}`,
/// `{data}` and `{target}` are filled in.
pub const WORD: &str = "Satır {line}: “{word}” GSI sözcüğü değil; satır okunmadı.";
pub const UNIT: &str = "Satır {line}: WI {wi} sözcüğünün birimi ({unit}) okunmuyor; yalnız metre, gon ve derece; satır okunmadı.";
pub const VALUE: &str =
    "Satır {line}: WI {wi} sözcüğünün değeri “{data}” okunamadı; satır okunmadı.";
pub const TURN: &str =
    "Satır {line}: WI {wi} sözcüğünün açısı “{data}” bir tam dönüşten büyük; satır okunmadı.";
pub const TARGET: &str = "Satır {line}: ölçünün nokta numarası (WI 11) yok; satır okunmadı.";
pub const HORIZONTAL: &str = "Satır {line}: {target} noktasının eğik uzunluğu yok (yalnız WI 32 yatay uzunluk); nokta doğrultu olarak okundu.";

/// The angle words (horizontal reading, zenith) and the length words
/// (slope and horizontal distance, the station's east, north and height,
/// the reflector's and the instrument's height), in the order they are read.
const ANGLES: [&str; 2] = ["21", "22"];
const LENGTHS: [&str; 7] = ["31", "32", "84", "85", "86", "87", "88"];

/// A word: its index, units digit, whether its sign is minus, its data.
struct Word<'a> {
    wi: &'a str,
    unit: char,
    minus: bool,
    data: &'a str,
}

/// A word's parts; none for one shorter than eight characters or without a
/// sign as its seventh.
fn word(w: &str) -> Option<Word<'_>> {
    let mut at = w.char_indices();
    let (wi_end, _) = at.nth(2)?;
    let (_, unit) = at.nth(2)?;
    let (_, sign) = at.next()?;
    let (data_at, _) = at.next()?;
    let minus = match sign {
        '+' => false,
        '-' => true,
        _ => return None,
    };
    Some(Word {
        wi: &w[..wi_end],
        unit,
        minus,
        data: &w[data_at..],
    })
}

/// A text word's value: its data without leading zeros (a zero alone stays).
fn text_of(data: &str) -> String {
    let t = data.trim_start_matches('0');
    if t.is_empty() { "0" } else { t }.to_owned()
}

/// ASCII digits as an integer, saturating; none for anything else.
fn digits(data: &str) -> Option<u64> {
    (!data.is_empty() && data.bytes().all(|b| b.is_ascii_digit())).then(|| {
        data.bytes().fold(0u64, |v, b| {
            v.saturating_mul(10).saturating_add(u64::from(b - b'0'))
        })
    })
}

/// Why a value is not read.
enum Unread {
    Unit,
    Value,
    Turn,
}

/// An angle's unit.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Grad,
    Deg,
}

/// An angle's exact value: a numerator over a denominator in its unit.
#[derive(Clone, Copy)]
struct Angle {
    num: i64,
    den: i64,
    kind: Kind,
}

impl Angle {
    /// The value in `unit`, rounded once: within a turn the numerators stay
    /// far below 2⁵³, so the division is the exact value's nearest float64.
    fn value_in(self, unit: Kind) -> f64 {
        let (num, den) = match (self.kind, unit) {
            (Kind::Grad, Kind::Deg) => (self.num * 9, self.den * 10),
            (Kind::Deg, Kind::Grad) => (self.num * 10, self.den * 9),
            _ => (self.num, self.den),
        };
        num as f64 / den as f64
    }
}

/// An angle word's value: units 2 (gon) and 3 (decimal degrees) carry five
/// decimals, 4 is DDD..MMSSs (sexagesimal, the last digit tenths of a
/// second); more than a full turn is not read.
fn angle(w: &Word<'_>) -> Result<Angle, Unread> {
    if !matches!(w.unit, '2' | '3' | '4') {
        return Err(Unread::Unit);
    }
    let n = digits(w.data).ok_or(Unread::Value)?;
    let (num, den, kind, full) = match w.unit {
        '4' => {
            let d = w.data;
            if d.len() < 6 {
                return Err(Unread::Value);
            }
            // The data is ASCII digits: its last five are MMSSs.
            let at = d.len() - 5;
            let part = |a: usize, b: usize| digits(&d[a..b]).unwrap_or(0);
            let degrees = digits(&d[..at]).unwrap_or(u64::MAX);
            let (minutes, seconds) = (part(at, at + 2), part(at + 2, at + 4));
            if minutes >= 60 || seconds >= 60 {
                return Err(Unread::Value);
            }
            let tenths = degrees
                .saturating_mul(36_000)
                .saturating_add(minutes * 600 + seconds * 10 + part(at + 4, at + 5));
            (tenths, 36_000, Kind::Deg, 360 * 36_000)
        }
        '2' => (n, 100_000, Kind::Grad, 400 * 100_000),
        _ => (n, 100_000, Kind::Deg, 360 * 100_000),
    };
    if num > full {
        return Err(Unread::Turn);
    }
    // Within a turn the numerator fits an i64.
    let num = num as i64;
    Ok(Angle {
        num: if w.minus { -num } else { num },
        den,
        kind,
    })
}

/// A length word's value in metres: units 0 (the last digit a millimetre),
/// 6 (a tenth of one) and 8 (a hundredth); the decimal's nearest float64.
fn length(w: &Word<'_>) -> Result<f64, Unread> {
    let places = match w.unit {
        '0' => 3,
        '6' => 4,
        '8' => 5,
        _ => return Err(Unread::Unit),
    };
    digits(w.data).ok_or(Unread::Value)?;
    let v: f64 = format!("{}e-{places}", w.data)
        .parse()
        .map_err(|_| Unread::Value)?;
    Ok(if w.minus && v != 0.0 { -v } else { v })
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

/// Reads a Leica GSI-8 or GSI-16 field book.
pub fn read(bytes: &[u8]) -> FieldBookRead {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut read = FieldBookRead {
        format: "gsi".to_owned(),
        encoding: enc.label().to_owned(),
        unit: None,
        first_line: Vec::new(),
        stations: Vec::new(),
        problems: Vec::new(),
    };
    // The book's angle unit (the first angle's), the station being read and its last target height.
    let mut unit: Option<Kind> = None;
    let mut current: Option<usize> = None;
    let mut last_target_height: Option<f64> = None;
    for (i, raw) in body.split('\n').enumerate() {
        let line = u32::try_from(i + 1).unwrap_or(u32::MAX);
        let block = raw.trim();
        if block.is_empty() {
            continue;
        }
        let block = block.strip_prefix('*').unwrap_or(block);
        // The block's words, the first of each index.
        let mut words: Vec<Word<'_>> = Vec::new();
        let mut bad = None;
        for w in block.split_whitespace() {
            let Some(w) = word(w) else {
                bad = Some(said(WORD, line, &[("{word}", w)]));
                break;
            };
            if !words.iter().any(|k| k.wi == w.wi) {
                words.push(w);
            }
        }
        if let Some(p) = bad {
            read.problems.push(p);
            continue;
        }
        let find = |wi: &str| words.iter().find(|w| w.wi == wi);
        let unread = |w: &Word<'_>, why: Unread| -> LineError {
            let digit = w.unit.to_string();
            match why {
                Unread::Unit => said(UNIT, line, &[("{wi}", w.wi), ("{unit}", &digit)]),
                Unread::Value => said(VALUE, line, &[("{wi}", w.wi), ("{data}", w.data)]),
                Unread::Turn => said(TURN, line, &[("{wi}", w.wi), ("{data}", w.data)]),
            }
        };
        // The block's numbers; the first that is not read leaves the block out.
        let mut angles = [None; 2];
        let mut lengths = [None; 7];
        let mut stop = None;
        for (k, wi) in ANGLES.iter().enumerate() {
            if let Some(w) = find(wi) {
                match angle(w) {
                    Ok(a) => angles[k] = Some(a),
                    Err(why) => {
                        stop = Some(unread(w, why));
                        break;
                    }
                }
            }
        }
        if stop.is_none() {
            for (k, wi) in LENGTHS.iter().enumerate() {
                if let Some(w) = find(wi) {
                    match length(w) {
                        Ok(v) => lengths[k] = Some(v),
                        Err(why) => {
                            stop = Some(unread(w, why));
                            break;
                        }
                    }
                }
            }
        }
        if let Some(p) = stop {
            read.problems.push(p);
            continue;
        }
        let text = |wi: &str| find(wi).map(|w| text_of(w.data));
        let [
            slope,
            horizontal,
            east,
            north,
            height,
            target_height,
            instrument_height,
        ] = lengths;
        if let [Some(hz), zenith] = angles {
            let Some(target) = text("11") else {
                read.problems.push(said(TARGET, line, &[]));
                continue;
            };
            let book = *unit.get_or_insert(hz.kind);
            // An observation before any station is the book's first station's, unnamed.
            let at = match current {
                Some(at) => at,
                None => {
                    read.stations.push(FieldStation {
                        station: String::new(),
                        instrument_height: None,
                        east: None,
                        north: None,
                        height: None,
                        observations: Vec::new(),
                    });
                    last_target_height = None;
                    *current.insert(read.stations.len() - 1)
                }
            };
            if slope.is_none() && horizontal.is_some() {
                read.problems
                    .push(said(HORIZONTAL, line, &[("{target}", &target)]));
            }
            // A target height left out is the station's last one (instruments write it when it changes).
            let target_height = target_height.or(last_target_height);
            last_target_height = target_height;
            read.stations[at].observations.push(FieldObservation {
                target,
                hz: hz.value_in(book),
                zenith: zenith.map(|z| z.value_in(book)),
                slope,
                target_height,
                code: text("71"),
                line,
            });
        } else if east.is_some()
            || north.is_some()
            || height.is_some()
            || instrument_height.is_some()
        {
            read.stations.push(FieldStation {
                station: text("16").or_else(|| text("11")).unwrap_or_default(),
                instrument_height,
                east,
                north,
                height,
                observations: Vec::new(),
            });
            current = Some(read.stations.len() - 1);
            last_target_height = None;
        }
    }
    read.unit = unit.map(|u| {
        match u {
            Kind::Grad => "grad",
            Kind::Deg => "deg",
        }
        .to_owned()
    });
    read
}

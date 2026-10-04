//! Field books (docs/adr/0169 §1–§2): a plain CSV or TXT field book whose
//! columns the user maps (istasyon, alet yüksekliği, nokta, yatay açı,
//! başucu açısı, eğik uzunluk, prizma yüksekliği, kod), read into stations
//! and observations as the instrument wrote them; every line that is not
//! read is named with the reason. The independent reference is
//! `scripts/fixtures/field_csv_cases.py` (fixtures/field/v1/csv.json).
//! Values are the float64 nearest to the text, never rounded (CLAUDE.md §23).

use kentos_contracts::{FieldBookRead, FieldCsvOptions, FieldObservation, FieldStation, LineError};

use crate::text;

/// What is said of a line not read; `{line}`, `{what}`, `{text}` and `{target}` are filled in.
pub const ROW: &str = "Satır {line}: {what} “{text}” sayı değil; satır okunmadı.";
pub const POINT: &str = "Satır {line}: {what} “{text}” sayı değil; nokta okunmadı.";
pub const NO_HZ: &str = "Satır {line}: {target} noktasının yatay açısı yok; nokta okunmadı.";

/// The values' names as the problems say them.
const INSTRUMENT_HEIGHT: &str = "alet yüksekliği";
const HZ: &str = "yatay açı";
const ZENITH: &str = "başucu açısı";
const SLOPE: &str = "eğik uzunluk";
const TARGET_HEIGHT: &str = "prizma yüksekliği";

/// The first line's separator: its most frequent of tab, semicolon and
/// comma (tab before semicolon before comma on a tie); none for one cell.
fn separator(line: &str) -> Option<char> {
    let mut best: Option<(usize, char)> = None;
    for c in ['\t', ';', ','] {
        let n = line.matches(c).count();
        if n > 0 && best.is_none_or(|(m, _)| n > m) {
            best = Some((n, c));
        }
    }
    best.map(|(_, c)| c)
}

/// A number as the Hesap windows read one: trimmed, its first comma a
/// point (unless commas separate the cells),
/// `^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$`.
fn number(text: &str, comma: bool) -> Option<f64> {
    let t = text.trim();
    let t = if comma {
        t.replacen(',', ".", 1)
    } else {
        t.to_owned()
    };
    let b = t.as_bytes();
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut i = usize::from(matches!(b.first(), Some(b'-' | b'+')));
    let whole = digits(i);
    i += whole;
    let mut fraction = 0;
    if b.get(i) == Some(&b'.') {
        fraction = digits(i + 1);
        i += 1 + fraction;
    }
    if whole == 0 && fraction == 0 {
        return None;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        i += usize::from(matches!(b.get(i), Some(b'-' | b'+')));
        let exponent = digits(i);
        if exponent == 0 {
            return None;
        }
        i += exponent;
    }
    (i == b.len()).then(|| t.parse().ok()).flatten()
}

/// Reads a plain-text field book with the user's column mapping.
pub fn read_csv(bytes: &[u8], opts: &FieldCsvOptions) -> FieldBookRead {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc);
    let mut lines = body.split('\n').enumerate().filter_map(|(i, l)| {
        let l = l.strip_suffix('\r').unwrap_or(l);
        (!l.trim().is_empty()).then(|| (u32::try_from(i + 1).unwrap_or(u32::MAX), l))
    });
    let mut read = FieldBookRead {
        encoding: enc.label().to_owned(),
        first_line: Vec::new(),
        stations: Vec::new(),
        problems: Vec::new(),
    };
    let Some(first) = lines.next() else {
        return read;
    };
    let sep = separator(first.1);
    let comma = sep != Some(',');
    let split = |l: &str| -> Vec<String> {
        match sep {
            Some(c) => l.split(c).map(|v| v.trim().to_owned()).collect(),
            None => vec![l.trim().to_owned()],
        }
    };
    read.first_line = split(first.1);
    let rows: Box<dyn Iterator<Item = (u32, &str)>> = if opts.header {
        Box::new(lines)
    } else {
        Box::new(std::iter::once(first).chain(lines))
    };
    // The station being read and its last target height.
    let mut current: Option<usize> = None;
    let mut last_target_height: Option<f64> = None;
    for (line, raw) in rows {
        let cells = split(raw);
        let cell = |col: Option<u32>| -> &str {
            col.and_then(|c| cells.get(c as usize))
                .map_or("", |v| v.as_str())
        };
        let problem = |said: &str, what: &str, t: &str| LineError {
            line,
            message: said
                .replace("{line}", &line.to_string())
                .replace("{what}", what)
                .replace("{text}", t),
        };
        // The station's part of the row: a bad instrument height leaves the whole row out.
        let ih_text = cell(opts.instrument_height);
        let ih = if ih_text.is_empty() {
            None
        } else {
            match number(ih_text, comma) {
                Some(v) => Some(v),
                None => {
                    read.problems.push(problem(ROW, INSTRUMENT_HEIGHT, ih_text));
                    continue;
                }
            }
        };
        let name = cell(opts.station);
        let starts = match current {
            None => true,
            Some(at) => {
                opts.station.is_some() && !name.is_empty() && read.stations[at].station != name
            }
        };
        if starts {
            read.stations.push(FieldStation {
                station: name.to_owned(),
                instrument_height: None,
                observations: Vec::new(),
            });
            current = Some(read.stations.len() - 1);
            last_target_height = None;
        }
        let at = current.unwrap_or(0);
        if ih.is_some() {
            read.stations[at].instrument_height = ih;
        }
        let target = cell(Some(opts.target));
        if target.is_empty() {
            continue;
        }
        // The observation's part: a bad value leaves only the observation out.
        let mut values = [None; 4];
        let mut bad = None;
        for (k, (col, what)) in [
            (Some(opts.hz), HZ),
            (opts.zenith, ZENITH),
            (opts.slope, SLOPE),
            (opts.target_height, TARGET_HEIGHT),
        ]
        .into_iter()
        .enumerate()
        {
            let t = cell(col);
            if t.is_empty() {
                continue;
            }
            match number(t, comma) {
                Some(v) => values[k] = Some(v),
                None => {
                    bad = Some(problem(POINT, what, t));
                    break;
                }
            }
        }
        if let Some(p) = bad {
            read.problems.push(p);
            continue;
        }
        let Some(hz) = values[0] else {
            read.problems.push(LineError {
                line,
                message: NO_HZ
                    .replace("{line}", &line.to_string())
                    .replace("{target}", target),
            });
            continue;
        };
        // A target height left empty is the station's last one (instruments write it when it changes).
        let target_height = values[3].or(last_target_height);
        last_target_height = target_height;
        let code = cell(opts.code);
        read.stations[at].observations.push(FieldObservation {
            target: target.to_owned(),
            hz,
            zenith: values[1],
            slope: values[2],
            target_height,
            code: (!code.is_empty()).then(|| code.to_owned()),
            line,
        });
    }
    read
}

//! Trimble JobXML field books (docs/adr/0169 §1): the FieldBook's records
//! read into stations and observations as Trimble's schema describes them
//! (JobXML schema 5.3 and its annotations: all angles decimal degrees, all
//! distances metres, the raw terrestrial readings in Circle elements).
//! Every record not read is named with the reason. The independent
//! reference is `scripts/fixtures/field_jobxml_cases.py`
//! (fixtures/field/v1/jobxml.json), read with Python's own XML parser. A
//! value is the float64 nearest to its text (CLAUDE.md §23).

use std::collections::HashMap;

use kentos_contracts::{FieldBookRead, FieldObservation, FieldStation, LineError};
use roxmltree::Node;

use crate::text;
use crate::xml::{DEPTH, Unparsed, document};

/// What is said of a record not read; `{line}`, `{depth}`, `{root}`,
/// `{data}`, `{name}`, `{method}` and `{id}` are filled in.
pub const XML: &str = "Satır 1: dosya XML olarak okunamadı; karne okunmadı.";
pub const DEEP: &str = "Satır 1: XML öğeleri {depth} düzeyden derin iç içe; karne okunmadı.";
pub const ROOT: &str = "Satır {line}: XML dosyası JobXML değil (kök öğe {root}); karne okunmadı.";
pub const FIELDBOOK: &str = "Satır {line}: JobXML dosyasında FieldBook yok; karne okunmadı.";
pub const VALUE: &str = "Satır {line}: “{data}” sayı değil; kayıt okunmadı.";
pub const TURN: &str =
    "Satır {line}: açı “{data}” sıfırla 360 derece arasında değil; kayıt okunmadı.";
pub const NEGATIVE: &str = "Satır {line}: eğik uzunluk “{data}” sıfırdan küçük; kayıt okunmadı.";
pub const METHOD: &str =
    "Satır {line}: {name} gözlemi {method} yöntemiyle; ham gözlem değil, okunmadı.";
pub const STATION: &str =
    "Satır {line}: {name} gözleminin istasyonu (StationID {id}) yok; okunmadı.";
pub const HORIZONTAL: &str = "Satır {line}: {name} gözleminin yatay açısı yok; okunmadı.";
pub const TARGET: &str =
    "Satır {line}: {name} gözleminin prizma kaydı (TargetID {id}) yok; prizma yüksekliği okunmadı.";

/// The methods whose Circle holds raw readings.
const RAW: [&str; 4] = [
    "DirectReading",
    "AverageMeasurements",
    "AngleOnly",
    "HorizontalAngleOnly",
];

/// Why a record is not read: its text and the value's text.
struct Unread(&'static str, String);

/// A child element's text, trimmed; empty when there is none.
fn value(node: Node<'_, '_>, tag: &str) -> String {
    child(node, tag)
        .map(|c| c.text().unwrap_or_default().trim().to_owned())
        .unwrap_or_default()
}

/// A node's first child element of a name.
fn child<'a, 'i>(node: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    node.children()
        .find(|c| c.is_element() && c.tag_name().name() == tag)
}

/// A number (the schema's double, INF and NaN aside): an optional sign,
/// digits with an optional point or a point and digits, an optional
/// exponent; blank is none.
fn number(data: &str) -> Result<Option<f64>, Unread> {
    if data.is_empty() {
        return Ok(None);
    }
    let b = data.as_bytes();
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut i = usize::from(matches!(b.first(), Some(b'-' | b'+')));
    let whole = digits(i);
    i += whole;
    let mut fraction = 0;
    if b.get(i) == Some(&b'.') {
        fraction = digits(i + 1);
        i += 1 + fraction;
    }
    let mut ok = whole + fraction > 0;
    if ok && matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        i += usize::from(matches!(b.get(i), Some(b'-' | b'+')));
        let exponent = digits(i);
        ok = exponent > 0;
        i += exponent;
    }
    if !ok || i != b.len() {
        return Err(Unread(VALUE, data.to_owned()));
    }
    data.parse::<f64>()
        .map(Some)
        .map_err(|_| Unread(VALUE, data.to_owned()))
}

/// An angle: from 0 up to 360 degrees.
fn angle(data: &str) -> Result<Option<f64>, Unread> {
    let v = number(data)?;
    if v.is_some_and(|v| !(0.0..360.0).contains(&v)) {
        return Err(Unread(TURN, data.to_owned()));
    }
    Ok(v)
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

/// Reads a Trimble JobXML field book.
pub fn read(bytes: &[u8]) -> FieldBookRead {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc);
    let mut read = FieldBookRead {
        format: "jobxml".to_owned(),
        encoding: enc.label().to_owned(),
        unit: None,
        first_line: Vec::new(),
        stations: Vec::new(),
        problems: Vec::new(),
    };
    let doc = match document(&body) {
        Ok(doc) => doc,
        Err(Unparsed::NotXml) => {
            read.problems.push(said(XML, 1, &[]));
            return read;
        }
        Err(Unparsed::TooDeep) => {
            read.problems
                .push(said(DEEP, 1, &[("{depth}", &DEPTH.to_string())]));
            return read;
        }
    };
    let line_of = |n: Node<'_, '_>| doc.text_pos_at(n.range().start).row;
    let root = doc.root_element();
    if root.tag_name().name() != "JOBFile" {
        let line = line_of(root);
        read.problems
            .push(said(ROOT, line, &[("{root}", root.tag_name().name())]));
        return read;
    }
    let Some(book) = child(root, "FieldBook") else {
        read.problems.push(said(FIELDBOOK, line_of(root), &[]));
        return read;
    };
    read.unit = Some("deg".to_owned());
    let mut places: HashMap<String, Place> = HashMap::new();
    let mut stations: HashMap<String, usize> = HashMap::new();
    let mut targets: HashMap<String, Option<f64>> = HashMap::new();
    for r in book.children().filter(Node::is_element) {
        let line = line_of(r);
        let id = r.attribute("ID").unwrap_or_default().to_owned();
        let outcome: Result<(), Unread> = match r.tag_name().name() {
            "TargetRecord" => number(&value(r, "TargetHeight")).map(|h| {
                targets.insert(id, h);
            }),
            "StationRecord" => number(&value(r, "TheodoliteHeight")).map(|height| {
                let name = value(r, "StationName");
                let (east, north, z) = places.get(&name).copied().unwrap_or((None, None, None));
                read.stations.push(FieldStation {
                    station: name,
                    instrument_height: height,
                    east,
                    north,
                    height: z,
                    observations: Vec::new(),
                });
                stations.insert(id, read.stations.len() - 1);
            }),
            "PointRecord" => point(r, line, &mut read, &mut places, &stations, &targets),
            _ => Ok(()),
        };
        if let Err(Unread(text, data)) = outcome {
            read.problems.push(said(text, line, &[("{data}", &data)]));
        }
    }
    read
}

/// A PointRecord: its coordinates kept, its raw readings an observation.
fn point(
    r: Node<'_, '_>,
    line: u32,
    read: &mut FieldBookRead,
    places: &mut HashMap<String, Place>,
    stations: &HashMap<String, usize>,
    targets: &HashMap<String, Option<f64>>,
) -> Result<(), Unread> {
    if value(r, "Deleted") == "true" {
        return Ok(());
    }
    let name = value(r, "Name");
    if let Some(grid) = child(r, "Grid") {
        let (n, e, z) = (
            number(&value(grid, "North"))?,
            number(&value(grid, "East"))?,
            number(&value(grid, "Elevation"))?,
        );
        places.insert(name.clone(), (e, n, z));
    }
    let Some(circle) = child(r, "Circle") else {
        return Ok(());
    };
    let method = value(r, "Method");
    if !RAW.contains(&method.as_str()) {
        read.problems.push(said(
            METHOD,
            line,
            &[("{method}", &method), ("{name}", &name)],
        ));
        return Ok(());
    }
    let hz = angle(&value(circle, "HorizontalCircle"))?;
    let zenith = angle(&value(circle, "VerticalCircle"))?;
    let edm = value(circle, "EDMDistance");
    let slope = number(&edm)?;
    if slope.is_some_and(|s| s < 0.0) {
        return Err(Unread(NEGATIVE, edm));
    }
    let station_id = value(r, "StationID");
    let Some(&at) = stations.get(&station_id) else {
        read.problems.push(said(
            STATION,
            line,
            &[("{id}", &station_id), ("{name}", &name)],
        ));
        return Ok(());
    };
    let Some(hz) = hz else {
        read.problems
            .push(said(HORIZONTAL, line, &[("{name}", &name)]));
        return Ok(());
    };
    let target_id = value(r, "TargetID");
    let mut target_height = None;
    if !target_id.is_empty() {
        match targets.get(&target_id) {
            Some(h) => target_height = *h,
            None => read.problems.push(said(
                TARGET,
                line,
                &[("{id}", &target_id), ("{name}", &name)],
            )),
        }
    }
    let code = value(r, "Code");
    read.stations[at].observations.push(FieldObservation {
        target: name,
        hz,
        zenith,
        slope,
        target_height,
        code: (!code.is_empty()).then_some(code),
        line,
    });
    Ok(())
}

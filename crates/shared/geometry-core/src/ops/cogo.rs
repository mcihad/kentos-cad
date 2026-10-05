//! Kayıtlı ölçüler (docs/adr/0180): a line's and an arc's recorded
//! measurements, kept as attributes beside the geometry (ArcGIS's COGO
//! fields), measured from the drawing, checked against it, and recorded from
//! a typed polar point. One rule for both platforms (the web calls `cogoMeasure`,
//! `cogoCheck` and `cogoRecord` through WASM); the independent reference is
//! scripts/fixtures/cogo_cases.py, whose cases (fixtures/cogo/v1/cases.json)
//! both run.
//!
//! - Measured: a line's semt (grads, from north clockwise, 0 ≤ semt < 400)
//!   from its start to its end and its plane length; an arc's chord semt and
//!   length (start to end, counter-clockwise from `a0` to `a1`), its radius
//!   and its length along the arc.
//! - Checked: each recorded value an object has against the measured one:
//!   the semt's difference in cc (0.0001 grad), brought between −200 and 200
//!   grads, the lengths' in metres; over the tolerance it differs. A value
//!   that is no number (a decimal comma is a point) is unreadable.
//! - Recorded: a polar point typed as `@d<a` or `d<a` gives its distance as
//!   Kayıtlı uzunluk in metres (a local project's millimetres or centimetres
//!   by moving the decimal point), and in a GIS project with grads its angle
//!   as Kayıtlı semt; the texts as typed, a decimal comma made a point, a
//!   leading plus dropped.

use crate::api::Op;
use crate::api::json::{Json, ToJson};
use crate::entity::Shape;
use crate::geometry::{bearing_grad, dist};
use crate::jsmath::{PI, cos, sin};
use crate::op;
use crate::ops::compare::Attrs;
use crate::tools::point_text::js_trim;
use crate::vec2::Vec2;

/// The four attributes' names.
pub const SEMT: &str = "Kayıtlı semt";
pub const LENGTH: &str = "Kayıtlı uzunluk";
pub const RADIUS: &str = "Kayıtlı yarıçap";
pub const ARC: &str = "Kayıtlı yay uzunluğu";

/// What the drawing gives: semt in grads, lengths in metres; radius and arc
/// length an arc's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measured {
    pub semt: f64,
    pub length: f64,
    pub radius: Option<f64>,
    pub arc: Option<f64>,
}

crate::json_struct!(out Measured { semt, length, radius, arc });

/// A line's or an arc's measurements; none for another kind.
pub fn measure(shape: &Shape) -> Option<Measured> {
    match shape {
        Shape::Line { a, b } => Some(Measured {
            semt: bearing_grad(*a, *b),
            length: dist(*a, *b),
            radius: None,
            arc: None,
        }),
        Shape::Arc { c, r, a0, a1 } => {
            let mut sweep = a1 - a0;
            while sweep <= 0.0 {
                sweep += 2.0 * PI;
            }
            while sweep > 2.0 * PI {
                sweep -= 2.0 * PI;
            }
            let start = Vec2::new(c.x + r * cos(*a0), c.y + r * sin(*a0));
            let end = Vec2::new(c.x + r * cos(*a1), c.y + r * sin(*a1));
            Some(Measured {
                semt: bearing_grad(start, end),
                length: dist(start, end),
                radius: Some(*r),
                arc: Some(r * sweep),
            })
        }
        _ => None,
    }
}

/// Which measurement an item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Semt,
    Length,
    Radius,
    Arc,
}

impl Field {
    /// The attribute's name.
    pub fn attribute(self) -> &'static str {
        match self {
            Field::Semt => SEMT,
            Field::Length => LENGTH,
            Field::Radius => RADIUS,
            Field::Arc => ARC,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Field::Semt => "semt",
            Field::Length => "length",
            Field::Radius => "radius",
            Field::Arc => "arc",
        }
    }
}

impl ToJson for Field {
    fn write_json(&self, out: &mut String) {
        self.key().write_json(out);
    }
}

/// A recorded value against the measured one: the difference in cc for the
/// semt, in metres for lengths; none when the text is no number.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub field: Field,
    pub recorded: String,
    pub measured: f64,
    pub difference: Option<f64>,
    pub over: bool,
}

crate::json_struct!(out Item { field, recorded, measured, difference, over });

/// An object's finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Differs,
    Unreadable,
}

impl ToJson for Status {
    fn write_json(&self, out: &mut String) {
        match self {
            Status::Ok => "ok",
            Status::Differs => "differs",
            Status::Unreadable => "unreadable",
        }
        .write_json(out);
    }
}

/// An object's recorded values checked.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub status: Status,
    pub items: Vec<Item>,
}

crate::json_struct!(out Finding { status, items });

/// A number as a recorded value is written: an optional sign, digits with
/// one decimal point or comma; none otherwise.
pub fn read_number(text: &str) -> Option<f64> {
    let t = js_trim(text);
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    let mut digits = 0;
    let mut seps = 0;
    for c in body.chars() {
        match c {
            '0'..='9' => digits += 1,
            '.' | ',' => seps += 1,
            _ => return None,
        }
    }
    if digits == 0 || seps > 1 {
        return None;
    }
    t.trim_start_matches('+')
        .replace(',', ".")
        .parse::<f64>()
        .ok()
}

/// Checks the recorded values an object has; none when it is no line or arc,
/// or has none of the values its kind takes.
pub fn check(shape: &Shape, attrs: &Attrs, tol_length: f64, tol_cc: f64) -> Option<Finding> {
    let m = measure(shape)?;
    let mut fields = vec![(Field::Semt, m.semt), (Field::Length, m.length)];
    if let (Some(r), Some(arc)) = (m.radius, m.arc) {
        fields.extend([(Field::Radius, r), (Field::Arc, arc)]);
    }
    let mut items = Vec::new();
    for (f, measured) in fields {
        let Some((_, text)) = attrs.0.iter().find(|(k, _)| k == f.attribute()) else {
            continue;
        };
        let Some(rec) = read_number(text) else {
            items.push(Item {
                field: f,
                recorded: text.clone(),
                measured,
                difference: None,
                over: false,
            });
            continue;
        };
        let (difference, over) = if f == Field::Semt {
            let d = (rec - measured + 200.0).rem_euclid(400.0) - 200.0;
            let cc = d * 10000.0;
            (cc, cc.abs() > tol_cc)
        } else {
            let d = rec - measured;
            (d, d.abs() > tol_length)
        };
        items.push(Item {
            field: f,
            recorded: text.clone(),
            measured,
            difference: Some(difference),
            over,
        });
    }
    if items.is_empty() {
        return None;
    }
    let status = if items.iter().any(|i| i.difference.is_none()) {
        Status::Unreadable
    } else if items.iter().any(|i| i.over) {
        Status::Differs
    } else {
        Status::Ok
    };
    Some(Finding { status, items })
}

/// A typed number's text as kept: a decimal comma made a point, a leading
/// plus dropped, a bare point given its zero, a trailing point dropped.
fn plain(text: &str) -> String {
    let mut t = text.replace(',', ".").trim_start_matches('+').to_owned();
    if t.starts_with('.') {
        t.insert(0, '0');
    }
    if let Some(rest) = t.strip_prefix("-.") {
        t = format!("-0.{rest}");
    }
    if t.ends_with('.') {
        t.pop();
    }
    t
}

/// A non-negative decimal text divided by 10^`places`, exactly, as text: the
/// point moved left.
fn shift(text: &str, places: usize) -> String {
    if places == 0 {
        return text.to_owned();
    }
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    let mut digits = format!("{whole}{frac}");
    let mut point = whole.len() as isize - places as isize;
    if point <= 0 {
        digits = "0".repeat((1 - point) as usize) + &digits;
        point = 1;
    }
    let point = point as usize;
    let head = digits[..point].trim_start_matches('0');
    let head = if head.is_empty() { "0" } else { head };
    let rest = &digits[point..];
    if rest.is_empty() {
        head.to_owned()
    } else {
        format!("{head}.{rest}")
    }
}

/// A number's text as the polar form takes it: `[sign]digits[.digits]` or
/// `[sign].digits`, a comma as the point; `signs` the signs it may start
/// with. The number and the rest of the text.
fn number_at<'a>(text: &'a str, signs: &[u8]) -> Option<(&'a str, &'a str)> {
    let b = text.as_bytes();
    let mut i = 0;
    if i < b.len() && signs.contains(&b[i]) {
        i += 1;
    }
    let w0 = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let whole = i - w0;
    if i < b.len() && (b[i] == b'.' || b[i] == b',') {
        i += 1;
        let f0 = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if whole == 0 && i == f0 {
            return None;
        }
    } else if whole == 0 {
        return None;
    }
    // Only ASCII bytes were passed: `i` is a character boundary.
    Some((&text[..i], &text[i..]))
}

/// The recorded values a typed polar point gives a line: Kayıtlı uzunluk in
/// metres, and in a GIS project with grads Kayıtlı semt; none when the text is
/// no polar point. `convention` is `gis` or `cad`, `angle_unit` `grad` or
/// `deg`, `length_unit` `m`, `cm` or `mm`.
pub fn record(text: &str, convention: &str, angle_unit: &str, length_unit: &str) -> Option<Attrs> {
    let t = text.trim_matches(char::is_whitespace);
    let t = t
        .strip_prefix('@')
        .unwrap_or(t)
        .trim_start_matches(char::is_whitespace);
    let (distance, rest) = number_at(t, b"+")?;
    let rest = rest
        .trim_start_matches(char::is_whitespace)
        .strip_prefix('<')?;
    let rest = rest.trim_start_matches(char::is_whitespace);
    let (angle, rest) = number_at(rest, b"+-")?;
    if !rest.trim_matches(char::is_whitespace).is_empty() {
        return None;
    }
    let places = match length_unit {
        "mm" => 3,
        "cm" => 2,
        _ => 0,
    };
    let mut out = vec![(LENGTH.to_owned(), shift(&plain(distance), places))];
    if convention == "gis" && angle_unit == "grad" {
        out.push((SEMT.to_owned(), plain(angle)));
    }
    Some(Attrs(out))
}

/// `cogoCheck`'s settings: the length tolerance in metres, the semt's in cc.
struct Tolerances {
    length: f64,
    cc: f64,
}

impl crate::api::json::FromJson for Tolerances {
    fn from_json(v: &Json) -> Result<Tolerances, String> {
        use crate::api::json::read_field;
        Ok(Tolerances {
            length: read_field(v, "length")?,
            cc: read_field(v, "cc")?,
        })
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("cogoMeasure", |shape: Shape| measure(&shape)),
    op!("cogoCheck", |shape: Shape,
                      attrs: Attrs,
                      tolerances: Tolerances| {
        check(&shape, &attrs, tolerances.length, tolerances.cc)
    }),
    op!("cogoRecord", |text: String,
                       convention: String,
                       angle_unit: String,
                       length_unit: String| {
        record(&text, &convention, &angle_unit, &length_unit)
    }),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::FromJson;

    fn file() -> Json {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/cogo/v1/cases.json"
        );
        Json::parse(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
    }

    fn text(v: &Json) -> String {
        match v {
            Json::Str(s) => s.clone(),
            _ => String::new(),
        }
    }

    fn num(v: &Json) -> Option<f64> {
        match v {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * (1.0 + b.abs())
    }

    /// The shared cases (fixtures/cogo/v1/cases.json), written by
    /// scripts/fixtures/cogo_cases.py from docs/adr/0180, not from this code;
    /// the web runs them through WASM (model/ops/cogo.test.ts).
    #[test]
    fn measures_checks_and_records_as_the_shared_cases_say() {
        let f = file();
        assert_eq!(f.get("format"), &Json::Str("kentos.cogo-cases".into()));
        let Json::Arr(measures) = f.get("measures") else {
            panic!("measures")
        };
        for c in measures {
            let name = text(c.get("name"));
            let shape = Shape::from_json(c.get("shape")).expect("a shape");
            let got = measure(&shape);
            match (got, c.get("measured")) {
                (None, Json::Null) => {}
                (Some(m), w) => {
                    assert!(
                        near(m.semt, num(w.get("semt")).expect("semt")),
                        "{name}: {}",
                        m.semt
                    );
                    assert!(
                        near(m.length, num(w.get("length")).expect("length")),
                        "{name}"
                    );
                    assert_eq!(m.radius.is_some(), num(w.get("radius")).is_some(), "{name}");
                    if let (Some(r), Some(e)) = (m.radius, num(w.get("radius"))) {
                        assert!(near(r, e), "{name}");
                    }
                    if let (Some(a), Some(e)) = (m.arc, num(w.get("arc"))) {
                        assert!(near(a, e), "{name}");
                    }
                }
                other => panic!("{name}: {other:?}"),
            }
        }
        let Json::Arr(checks) = f.get("checks") else {
            panic!("checks")
        };
        for c in checks {
            let name = text(c.get("name"));
            let shape = Shape::from_json(c.get("shape")).expect("a shape");
            let attrs = Attrs::from_json(c.get("attrs")).expect("attrs");
            let tl = num(c.get("toleranceLength")).expect("tolerance");
            let tc = num(c.get("toleranceCc")).expect("tolerance");
            let got = check(&shape, &attrs, tl, tc);
            match (got, c.get("result")) {
                (None, Json::Null) => {}
                (Some(g), w) => {
                    let mut status = String::new();
                    g.status.write_json(&mut status);
                    assert_eq!(
                        &Json::Str(status.trim_matches('"').to_owned()),
                        w.get("status"),
                        "{name}"
                    );
                    let Json::Arr(items) = w.get("items") else {
                        panic!("{name}: items")
                    };
                    assert_eq!(g.items.len(), items.len(), "{name}");
                    for (gi, wi) in g.items.iter().zip(items) {
                        assert_eq!(gi.field.key(), text(wi.get("field")), "{name}");
                        assert_eq!(gi.recorded, text(wi.get("recorded")), "{name}");
                        assert!(
                            near(gi.measured, num(wi.get("measured")).expect("measured")),
                            "{name}"
                        );
                        match (gi.difference, num(wi.get("difference"))) {
                            (Some(d), Some(e)) => assert!((d - e).abs() <= 1e-6, "{name}: {d} {e}"),
                            (None, None) => {}
                            other => panic!("{name}: {other:?}"),
                        }
                        assert_eq!(&Json::Bool(gi.over), wi.get("over"), "{name}");
                    }
                }
                other => panic!("{name}: {other:?}"),
            }
        }
        let Json::Arr(records) = f.get("records") else {
            panic!("records")
        };
        for c in records {
            let name = text(c.get("name"));
            let got = record(
                &text(c.get("text")),
                &text(c.get("convention")),
                &text(c.get("angleUnit")),
                &text(c.get("lengthUnit")),
            );
            let want = match c.get("recorded") {
                Json::Null => None,
                v => Some(Attrs::from_json(v).expect("attrs")),
            };
            assert_eq!(got, want, "{name}");
        }
    }
}

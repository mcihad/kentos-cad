//! KentOS's own extended data (application "KENTOS", group 1001) on the
//! objects and layers the DXF writer produces: what DXF cannot hold but a
//! KentOS import needs to get the same drawing back. Other programs keep
//! it with the object (the application is registered in the APPID table)
//! and draw the object as the plain DXF it also is.
//!
//! Every item is a list, so a reader skips what it does not know:
//!
//! ```text
//! 1001 KENTOS
//! 1002 {  1000 label    <string>                 1002 }   the object's label (parcel number, point name)
//! 1002 {  1000 attr     <string> <string>        1002 }   one GIS attribute: key, value
//! 1002 {  1000 symbol   <string>                 1002 }   the library symbol drawn for it
//! 1002 {  1000 color    <string>                 1002 }   the colour as the app names it ("fg", "#7fb2e5")
//! 1002 {  1000 hole     1005 <handle>            1002 }   a ring of the polygon with that handle
//! 1002 {  1000 curve    1070 <0|1>               1002 }   a SPLINE through its fit points is KentOS's curve (1: closed)
//! 1002 {  1000 arc      1040 <a0> 1040 <a1>      1002 }   the arc's angles in radians, exactly
//! 1002 {  1000 pattern  1040 <angle> 1040 <gap>  1002 }   the hatch's angle and spacing, exactly
//! 1002 {  1000 z                                 1002 }   the point has an elevation, even 0
//! ```
//!
//! A string is one 1000 group, or its pieces in a nested 1002 list when it
//! is longer than a group may be (255 bytes); control characters and the
//! caret are in DXF's caret notation ("^J", "^ "). Values that DXF itself
//! rounds (angles in degrees, pattern offsets) are used by the reader only
//! while they agree with the entity's own data: an edit in another program
//! wins over stale extended data.

use std::collections::BTreeMap;

use crate::num::{dxf_real, parse_int, parse_real};

/// The registered application name.
pub const APP: &str = "KENTOS";

/// Bytes of one string group at most (AutoCAD allows 255).
const PIECE: usize = 250;
/// Extended data AutoCAD keeps per object at most (16 KB), with room to spare.
pub const MAX_BYTES: usize = 16_000;
/// Lists nest at most this deep (a string's pieces are one level).
const MAX_DEPTH: usize = 8;

/// What an object's or a layer's KENTOS data says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Meta {
    pub label: Option<String>,
    pub attrs: BTreeMap<String, String>,
    pub symbol: Option<String>,
    pub color: Option<String>,
    /// Handle of the polygon this closed polyline is a hole of.
    pub hole_of: Option<u64>,
    /// The SPLINE is KentOS's curve through its fit points; true: closed.
    pub curve: Option<bool>,
    /// Arc angles in radians.
    pub arc: Option<(f64, f64)>,
    /// Hatch angle (degrees) and spacing.
    pub pattern: Option<(f64, f64)>,
    /// The point's elevation is data even when it is 0.
    pub z: bool,
}

impl Meta {
    pub fn is_empty(&self) -> bool {
        self == &Meta::default()
    }
}

// ── Writing ─────────────────────────────────────────────────────────────

/// A string in DXF's caret notation: control characters as "^" and the
/// character 64 above, the caret itself as "^ ".
pub fn caret_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        push_encoded(c, &mut out);
    }
    out
}

fn push_encoded(c: char, out: &mut String) {
    match c {
        '^' => out.push_str("^ "),
        '\u{0}'..='\u{1F}' => {
            out.push('^');
            out.push(char::from(c as u8 + 64));
        }
        _ => out.push(c),
    }
}

/// The string groups of one value: a single 1000 group, or its pieces in a
/// nested list (never splitting a character or a caret pair).
fn string(s: &str, out: &mut Vec<(i32, String)>) {
    let mut pieces: Vec<String> = vec![String::new()];
    let mut unit = String::new();
    for c in s.chars() {
        unit.clear();
        push_encoded(c, &mut unit);
        if let Some(last) = pieces.last_mut() {
            if last.len() + unit.len() > PIECE {
                pieces.push(unit.clone());
            } else {
                last.push_str(&unit);
            }
        }
    }
    if pieces.len() == 1 {
        out.push((1000, pieces.pop().unwrap_or_default()));
    } else {
        out.push((1002, "{".into()));
        out.extend(pieces.into_iter().map(|p| (1000, p)));
        out.push((1002, "}".into()));
    }
}

fn item(tag: &str, out: &mut Vec<(i32, String)>, values: impl FnOnce(&mut Vec<(i32, String)>)) {
    out.push((1002, "{".into()));
    out.push((1000, tag.into()));
    values(out);
    out.push((1002, "}".into()));
}

/// The groups of `meta`, "1001 KENTOS" first; empty when there is nothing to say.
pub fn groups(meta: &Meta) -> Vec<(i32, String)> {
    let mut out: Vec<(i32, String)> = Vec::new();
    if meta.is_empty() {
        return out;
    }
    out.push((1001, APP.into()));
    if let Some(l) = &meta.label {
        item("label", &mut out, |o| string(l, o));
    }
    for (k, v) in &meta.attrs {
        item("attr", &mut out, |o| {
            string(k, o);
            string(v, o);
        });
    }
    if let Some(s) = &meta.symbol {
        item("symbol", &mut out, |o| string(s, o));
    }
    if let Some(c) = &meta.color {
        item("color", &mut out, |o| string(c, o));
    }
    if let Some(h) = meta.hole_of {
        item("hole", &mut out, |o| o.push((1005, format!("{h:X}"))));
    }
    if let Some(closed) = meta.curve {
        item("curve", &mut out, |o| {
            o.push((1070, if closed { "1" } else { "0" }.into()))
        });
    }
    if let Some((a0, a1)) = meta.arc {
        item("arc", &mut out, |o| {
            o.push((1040, dxf_real(a0)));
            o.push((1040, dxf_real(a1)));
        });
    }
    if let Some((angle, spacing)) = meta.pattern {
        item("pattern", &mut out, |o| {
            o.push((1040, dxf_real(angle)));
            o.push((1040, dxf_real(spacing)));
        });
    }
    if meta.z {
        item("z", &mut out, |_| {});
    }
    out
}

/// About how many bytes AutoCAD counts for these groups (its limit is 16 KB per object).
pub fn size(groups: &[(i32, String)]) -> usize {
    groups.iter().map(|(_, v)| v.len() + 4).sum()
}

// ── Reading ─────────────────────────────────────────────────────────────

/// DXF's caret notation back to characters: "^" and a character from "@"
/// to "_" is the control character 64 below it, "^ " a caret. Anything else
/// after a caret stays as written.
pub fn caret_decode(s: &str) -> String {
    if !s.contains('^') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '^' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some(' ') => {
                chars.next();
                out.push('^');
            }
            Some(n @ '@'..='_') => {
                chars.next();
                out.push(char::from(n as u8 - 64));
            }
            _ => out.push('^'),
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Str(String),
    Real(f64),
    Int(i64),
    Handle(u64),
    List(Vec<Value>),
}

/// Passes over a list nested too deep to read, up to its closing "}".
fn skip_list(groups: &[(i32, String)], i: &mut usize) {
    let mut level = 1usize;
    while let Some((code, v)) = groups.get(*i) {
        *i += 1;
        if *code == 1002 {
            match v.trim() {
                "{" => level += 1,
                "}" => {
                    level -= 1;
                    if level == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
    }
}

/// The values of a list up to its closing "}" (or the end), from `groups[*i]`.
fn list(groups: &[(i32, String)], i: &mut usize, depth: usize) -> Vec<Value> {
    let mut out = Vec::new();
    while let Some((code, v)) = groups.get(*i) {
        *i += 1;
        match *code {
            1002 if v.trim() == "}" => break,
            1002 if v.trim() == "{" => {
                if depth >= MAX_DEPTH {
                    skip_list(groups, i);
                } else {
                    out.push(Value::List(list(groups, i, depth + 1)));
                }
            }
            1000 => out.push(Value::Str(caret_decode(v))),
            1040..=1042 => {
                if let Some(x) = parse_real(v) {
                    out.push(Value::Real(x));
                }
            }
            1070 | 1071 => {
                if let Some(x) = parse_int(v) {
                    out.push(Value::Int(x));
                }
            }
            1005 => {
                if let Ok(h) = u64::from_str_radix(v.trim(), 16) {
                    out.push(Value::Handle(h));
                }
            }
            _ => {}
        }
    }
    out
}

/// A string value: one string, or the pieces of a nested list.
fn text(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::Str(s) => Some(s.clone()),
        Value::List(parts) => parts
            .iter()
            .map(|p| match p {
                Value::Str(s) => Some(s.as_str()),
                _ => None,
            })
            .collect::<Option<Vec<&str>>>()
            .map(|p| p.concat()),
        _ => None,
    }
}

fn real(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Real(x) => Some(*x),
        _ => None,
    }
}

/// Reads the KENTOS data among an object's groups (code, decoded value):
/// the groups after "1001 KENTOS" up to the next application. None when
/// there is none; items and values it does not know are passed over.
pub fn read(groups: &[(i32, String)]) -> Option<Meta> {
    let start = groups
        .iter()
        .position(|(c, v)| *c == 1001 && v.trim().eq_ignore_ascii_case(APP))?
        + 1;
    let end = groups[start..]
        .iter()
        .position(|(c, _)| *c == 1001)
        .map_or(groups.len(), |k| start + k);
    let own = &groups[start..end];
    let mut m = Meta::default();
    let mut i = 0;
    while i < own.len() {
        let (code, v) = &own[i];
        i += 1;
        if !(*code == 1002 && v.trim() == "{") {
            continue;
        }
        let values = list(own, &mut i, 1);
        let Some(Value::Str(tag)) = values.first() else {
            continue;
        };
        let (a, b) = (values.get(1), values.get(2));
        match tag.as_str() {
            "label" => m.label = text(a).or(m.label),
            "attr" => {
                if let (Some(k), Some(v)) = (text(a), text(b)) {
                    m.attrs.insert(k, v);
                }
            }
            "symbol" => m.symbol = text(a).or(m.symbol),
            "color" => m.color = text(a).or(m.color),
            "hole" => {
                if let Some(Value::Handle(h)) = a {
                    m.hole_of = Some(*h);
                }
            }
            "curve" => {
                if let Some(Value::Int(n)) = a {
                    m.curve = Some(*n != 0);
                }
            }
            "arc" => {
                if let (Some(x), Some(y)) = (real(a), real(b)) {
                    m.arc = Some((x, y));
                }
            }
            "pattern" => {
                if let (Some(x), Some(y)) = (real(a), real(b)) {
                    m.pattern = Some((x, y));
                }
            }
            "z" => m.z = true,
            _ => {}
        }
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(list: &[(i32, &str)]) -> Vec<(i32, String)> {
        list.iter().map(|(c, s)| (*c, s.to_string())).collect()
    }

    #[test]
    fn everything_written_reads_back() {
        let mut attrs = BTreeMap::new();
        attrs.insert("Parsel".to_string(), "12".to_string());
        attrs.insert("Not".to_string(), "iki\nsatır ^ ve ^J".to_string());
        attrs.insert("Uzun".to_string(), "ğ".repeat(400));
        let meta = Meta {
            label: Some("attr".into()),
            attrs,
            symbol: Some("mpyy.konut".into()),
            color: Some("fg-dim".into()),
            hole_of: Some(0x2F),
            curve: Some(true),
            arc: Some((0.1 + 0.2, -1.0 / 3.0)),
            pattern: Some((45.0, 2.5e-3)),
            z: true,
        };
        let out = groups(&meta);
        assert_eq!(out[0], (1001, "KENTOS".to_string()));
        // No group holds more than a piece; a long value came in pieces.
        assert!(out.iter().all(|(_, v)| v.len() <= PIECE));
        assert!(
            out.iter()
                .any(|(c, v)| *c == 1000 && v == "iki^Jsatır ^  ve ^ J")
        );
        assert_eq!(read(&out), Some(meta));
        assert!(groups(&Meta::default()).is_empty());
    }

    #[test]
    fn unknown_items_other_applications_and_stray_groups_are_passed_over() {
        let list = g(&[
            (1001, "ACAD"),
            (1000, "label"),
            (1000, "başka program"),
            (1001, "KENTOS"),
            (1000, "label"),
            (1002, "{"),
            (1000, "yeni-bir-şey"),
            (1002, "{"),
            (1000, "label"),
            (1000, "iç içe"),
            (1002, "}"),
            (1040, "1.5"),
            (1002, "}"),
            (1002, "{"),
            (1000, "attr"),
            (1000, "Ada"),
            (1002, "{"),
            (1000, "bir "),
            (1000, "iki"),
            (1002, "}"),
            (1002, "}"),
            (1002, "{"),
            (1000, "hole"),
            (1005, "zz"),
            (1002, "}"),
            (1001, "OTHER"),
            (1002, "{"),
            (1000, "label"),
            (1000, "not ours"),
            (1002, "}"),
        ]);
        let m = read(&list).expect("KENTOS data");
        assert_eq!(m.label, None);
        assert_eq!(m.attrs.get("Ada").map(String::as_str), Some("bir iki"));
        assert_eq!(m.hole_of, None);
        assert_eq!(read(&g(&[(1001, "ACAD"), (1000, "x")])), None);
        // Unbalanced lists end with the data.
        let m = read(&g(&[
            (1001, "KENTOS"),
            (1002, "{"),
            (1000, "label"),
            (1000, "yarım"),
        ]))
        .expect("data");
        assert_eq!(m.label.as_deref(), Some("yarım"));
    }

    #[test]
    fn caret_notation() {
        assert_eq!(caret_encode("a^b\tc\n"), "a^ b^Ic^J");
        assert_eq!(caret_decode("a^ b^Ic^J"), "a^b\tc\n");
        // A caret before anything else stays (stacked fractions in MTEXT: "\S1^;").
        assert_eq!(caret_decode("x^2 ^; ^"), "x^2 ^; ^");
        for s in ["", "^", "^^", "a\u{1}b", "Çağ ^ \r\n"] {
            assert_eq!(caret_decode(&caret_encode(s)), s, "{s:?}");
        }
    }
}

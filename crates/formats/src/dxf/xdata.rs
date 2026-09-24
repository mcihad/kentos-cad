//! KentOS's own extended data (application "KENTOS", group 1001) on the
//! objects it writes: what DXF cannot hold but a KentOS import needs to get
//! the same objects back. Other programs keep it with the object and draw
//! the object as the plain DXF it also is.
//!
//! ```text
//! 1001 KENTOS
//! 1000 label        1000 <text>                the object's label (parcel number, point name)
//! 1000 attr         1000 <key>   1000 <value>  one per attribute
//! 1000 hole         1005 <handle>              an island of the polygon with that handle
//! 1000 catmull-rom                             a SPLINE drawn from KentOS's curve (its fit points are the curve's points)
//! 1000 closed                                  … and the curve is closed
//! ```
//!
//! A value longer than one group may hold (255 bytes) is written as 1000
//! pieces between 1002 "{" and 1002 "}".

use std::collections::BTreeMap;

pub const TAG_LABEL: &str = "label";
pub const TAG_ATTR: &str = "attr";
pub const TAG_HOLE: &str = "hole";
pub const TAG_SPLINE: &str = super::XDATA_SPLINE;
pub const TAG_CLOSED: &str = "closed";

/// What an object's KENTOS extended data says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Meta {
    pub label: Option<String>,
    pub attrs: BTreeMap<String, String>,
    /// Handle of the polygon this ring is an island of.
    pub hole_of: Option<u64>,
    pub spline: bool,
    pub closed: bool,
}

impl Meta {
    pub fn is_empty(&self) -> bool {
        self == &Meta::default()
    }
}

/// Reads the KENTOS groups (code, decoded value) of one object; anything it
/// does not know is passed over, so newer writers stay readable.
pub fn parse(groups: &[(i32, String)]) -> Meta {
    let mut m = Meta::default();
    let mut i = 0;
    while let Some((code, tag)) = groups.get(i) {
        i += 1;
        if *code != 1000 {
            continue;
        }
        match tag.as_str() {
            TAG_LABEL => m.label = value(groups, &mut i),
            TAG_ATTR => {
                if let Some(k) = value(groups, &mut i)
                    && let Some(v) = value(groups, &mut i)
                {
                    m.attrs.insert(k, v);
                }
            }
            TAG_HOLE => {
                if let Some((1005, h)) = groups.get(i) {
                    m.hole_of = u64::from_str_radix(h.trim(), 16).ok();
                    i += 1;
                }
            }
            TAG_SPLINE => m.spline = true,
            TAG_CLOSED => m.closed = true,
            _ => {}
        }
    }
    m
}

/// The string value at `i`: one 1000 group, or the pieces between 1002 "{" and "}".
fn value(groups: &[(i32, String)], i: &mut usize) -> Option<String> {
    match groups.get(*i) {
        Some((1000, s)) => {
            *i += 1;
            Some(s.clone())
        }
        Some((1002, s)) if s.trim() == "{" => {
            *i += 1;
            let mut out = String::new();
            while let Some((code, s)) = groups.get(*i) {
                *i += 1;
                match *code {
                    1000 => out.push_str(s),
                    1002 if s.trim() == "}" => break,
                    _ => {}
                }
            }
            Some(out)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(list: &[(i32, &str)]) -> Vec<(i32, String)> {
        list.iter().map(|(c, s)| (*c, s.to_string())).collect()
    }

    #[test]
    fn tags_and_values_are_read_by_position() {
        // A value may be any text, a tag name included.
        let m = parse(&g(&[(1000, "label"), (1000, "attr"), (1000, "attr"), (1000, "Parsel"), (1000, "12"), (1000, "hole"), (1005, "2F"), (1000, "closed")]));
        assert_eq!(m.label.as_deref(), Some("attr"));
        assert_eq!(m.attrs.get("Parsel").map(String::as_str), Some("12"));
        assert_eq!(m.hole_of, Some(0x2F));
        assert!(m.closed && !m.spline);
    }

    #[test]
    fn long_values_come_in_pieces_and_unknown_tags_are_passed_over() {
        let m = parse(&g(&[(1000, "yeni-bir-şey"), (1070, "3"), (1000, "attr"), (1000, "Not"), (1002, "{"), (1000, "bir "), (1000, "iki"), (1002, "}"), (1000, "catmull-rom")]));
        assert_eq!(m.attrs.get("Not").map(String::as_str), Some("bir iki"));
        assert!(m.spline);
        // A tag whose value is missing reads nothing.
        assert_eq!(parse(&g(&[(1000, "label")])), Meta::default());
        assert!(parse(&[]).is_empty());
    }
}

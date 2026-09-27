//! What the Hesap windows read from their fields, without the window (the
//! web's `ui/calc/read.ts`, docs/adr/0070): known points, numbers, and
//! Aplikasyon's checks before and around the core's computation. The
//! windows show the errors; the tests read them here.

use kentos_contracts::Entity;
use kentos_domain::Document as Model;
use kentos_interaction::survey::polar::{Stake, StakeoutInput, stakeout};
use kentos_interaction::{Vec2, js_trim, upper_tr};

/// A known point as typed: a point object's name, or “Y,X” (the web's `Known`).
#[derive(Clone, Debug, PartialEq)]
pub enum Known {
    /// Nothing typed.
    Empty,
    /// The point, and the name it was found by (none for typed coordinates).
    Point {
        p: Vec2,
        name: String,
    },
    Error(String),
}

impl Known {
    pub fn point(&self) -> Option<Vec2> {
        match self {
            Known::Point { p, .. } => Some(*p),
            _ => None,
        }
    }
}

/// “Y,X”, “Y;X” or “Y X” with point decimals, east first (the web's `COORDS`).
fn coords(text: &str) -> Option<Vec2> {
    let t = text.trim();
    // The first number, then a comma, a semicolon or white space, then the second.
    let (a, rest) = number_prefix(t)?;
    let rest = rest.trim_start();
    let rest = match rest.strip_prefix([',', ';']) {
        Some(after) => after.trim_start(),
        // White space alone separates them.
        None if rest.len() < t.len() - a.len() => rest,
        None => return None,
    };
    let (b, tail) = number_prefix(rest)?;
    if !tail.trim().is_empty() {
        return None;
    }
    Some(Vec2::new(a.parse().ok()?, b.parse().ok()?))
}

/// `-?\d+(?:\.\d+)?` at the start of `s`: the number and what follows it.
fn number_prefix(s: &str) -> Option<(&str, &str)> {
    let bytes = s.as_bytes();
    let mut end = usize::from(bytes.first() == Some(&b'-'));
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let whole = digits(end);
    if whole == 0 {
        return None;
    }
    end += whole;
    if bytes.get(end) == Some(&b'.') {
        let fraction = digits(end + 1);
        if fraction > 0 {
            end += 1 + fraction;
        }
    }
    Some((&s[..end], &s[end..]))
}

/// Reads a known point (the web's `resolvePointIn`): “Y,X”, or the first
/// point object whose label (else its Ad) matches, Turkish upper-casing.
pub fn resolve_point(doc: &Model, text: &str) -> Known {
    let t = js_trim(text);
    if t.is_empty() {
        return Known::Empty;
    }
    if let Some(p) = coords(t) {
        return Known::Point {
            p,
            name: String::new(),
        };
    }
    let key = upper_tr(t);
    let found = doc.entities().find_map(|e| match e {
        Entity::Point(pt) => {
            let name = pt
                .base
                .label
                .as_deref()
                .or_else(|| pt.base.attrs.get("Ad").map(String::as_str))
                .unwrap_or("");
            (upper_tr(name) == key).then(|| (pt.p, pt.base.label.clone()))
        }
        _ => None,
    });
    match found {
        Some((p, label)) => Known::Point {
            p: Vec2::new(p.x, p.y),
            name: label.unwrap_or_else(|| t.to_owned()),
        },
        None => Known::Error(format!(
            "“{t}” adlı nokta çizimde yok. Adını denetleyin ya da Y,X yazın."
        )),
    }
}

/// The name a point object at exactly `p` has, if any (the web's `nameAt`):
/// a picked point snapped to a named point is given by its name.
pub fn name_at(doc: &Model, p: Vec2) -> Option<String> {
    doc.entities().find_map(|e| match e {
        Entity::Point(pt) if pt.p.x == p.x && pt.p.y == p.y => {
            pt.base.label.clone().filter(|l| !l.is_empty())
        }
        _ => None,
    })
}

/// A number as typed (the web's `readNumber`): the first comma is a decimal
/// point; empty is none; anything else not a number is NaN.
pub fn read_number(text: &str) -> Option<f64> {
    let t = js_trim(text).replacen(',', ".", 1);
    if t.is_empty() {
        return None;
    }
    Some(if is_number(&t) {
        t.parse().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    })
}

/// `^[-+]?(\d+(\.\d*)?|\.\d+)(e[-+]?\d+)?$`, case-insensitive.
fn is_number(t: &str) -> bool {
    let b = t.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'-' | b'+')));
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let whole = digits(i);
    i += whole;
    let mut fraction = 0;
    if b.get(i) == Some(&b'.') {
        fraction = digits(i + 1);
        i += 1 + fraction;
    }
    if whole == 0 && fraction == 0 {
        return false;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(b.get(i + 1), Some(b'-' | b'+')));
        let exponent = digits(i + 1 + sign);
        if exponent == 0 {
            return false;
        }
        i += 1 + sign + exponent;
    }
    i == b.len()
}

/// The project's angle unit as the core takes it.
pub fn unit_name(unit: kentos_contracts::AngleUnit) -> &'static str {
    match unit {
        kentos_contracts::AngleUnit::Deg => "deg",
        kentos_contracts::AngleUnit::Grad => "grad",
    }
}

/// Aplikasyon's fields read and computed (the web's `readStakeout`).
#[derive(Clone, Debug, PartialEq)]
pub struct StakeoutRead {
    pub errors: Vec<String>,
    pub stakes: Option<Vec<Stake>>,
    pub names: Vec<String>,
    /// Whether a back point was given (the turning angles).
    pub back: bool,
}

/// Aplikasyon's fields read and computed: the values and the targets'
/// names, or why not. A target at the station has no bearing: its row says so.
pub fn read_stakeout(
    station: &str,
    back: &str,
    rows: &[String],
    resolve: impl Fn(&str) -> Known,
    unit: &str,
) -> StakeoutRead {
    let mut errors = Vec::new();
    let st = resolve(station);
    let bk = resolve(back);
    match &st {
        Known::Empty => errors.push("Durulan nokta verilmedi.".to_owned()),
        Known::Error(e) => errors.push(format!("Durulan nokta: {e}")),
        Known::Point { .. } => {}
    }
    if let Known::Error(e) = &bk {
        errors.push(format!("Bakılan nokta: {e}"));
    }
    let station = st.point();
    let mut targets = Vec::new();
    let mut names = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        match resolve(row) {
            Known::Empty => {}
            Known::Error(e) => errors.push(format!("{}. satır: {e}", i + 1)),
            Known::Point { p, .. } if station.is_some_and(|s| s.x == p.x && s.y == p.y) => {
                errors.push(format!(
                    "{}. satırdaki nokta durulan noktayla aynı yerde; semt tanımsız.",
                    i + 1
                ));
            }
            Known::Point { p, name } => {
                targets.push(p);
                names.push(if name.is_empty() {
                    js_trim(row).to_owned()
                } else {
                    name
                });
            }
        }
    }
    if targets.is_empty() && errors.is_empty() {
        errors.push("Tabloya aplike edilecek en az bir nokta yazın.".to_owned());
    }
    let back_point = bk.point();
    let out = |errors, stakes| StakeoutRead {
        errors,
        stakes,
        names: names.clone(),
        back: back_point.is_some(),
    };
    let Some(station) = station.filter(|_| errors.is_empty()) else {
        return out(errors, None);
    };
    match stakeout(&StakeoutInput {
        unit: unit.to_owned(),
        station,
        back: back_point,
        targets,
    }) {
        Ok(stakes) => out(errors, Some(stakes)),
        Err(e) => {
            errors.push(e);
            out(errors, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_are_read_east_first_as_the_web_reads_them() {
        assert_eq!(
            coords("487000.5,4420000"),
            Some(Vec2::new(487000.5, 4420000.0))
        );
        assert_eq!(coords(" -12.5 ; 3 "), Some(Vec2::new(-12.5, 3.0)));
        assert_eq!(coords("12 34"), Some(Vec2::new(12.0, 34.0)));
        // A decimal comma is not a coordinate: the comma separates.
        assert_eq!(coords("12,5"), Some(Vec2::new(12.0, 5.0)));
        for bad in ["12", "12.,3", "1e3,4", "P1", "12,3,4", "+12,3", "12 ,"] {
            assert_eq!(coords(bad), None, "{bad}");
        }
    }

    #[test]
    fn numbers_are_read_as_the_web_reads_them() {
        assert_eq!(read_number("  "), None);
        assert_eq!(read_number("12,5"), Some(12.5));
        assert_eq!(read_number("-.5"), Some(-0.5));
        assert_eq!(read_number("1e3"), Some(1000.0));
        assert_eq!(read_number("5."), Some(5.0));
        for bad in ["12a", "1,2,3", ".", "e3", "--1", "1e"] {
            assert!(read_number(bad).is_some_and(f64::is_nan), "{bad}");
        }
    }
}

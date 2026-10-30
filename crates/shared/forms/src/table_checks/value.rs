//! The values a table constraint is judged with (docs/adr/0755 §3): SQL's NULL, a value KentOS cannot know before the
//! table answers, and the typed values a record's fields hold as the database sees them. Numbers keep their exact
//! decimal beside the binary float the database may hold; an order is told only where both agree.

use std::cmp::Ordering;

use kentos_contracts::{ColumnMade, LayerField, LayerFieldKind};

use super::Dialect;
use super::parse::Aff;

/// The most digits an exact decimal keeps.
const MOST_SCALE: u32 = 38;

/// An exact decimal: `m / 10^s`, `s` the fewest that hold it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Dec {
    m: i128,
    s: u32,
}

fn pow10(n: u32) -> Option<i128> {
    10i128.checked_pow(n)
}

impl Dec {
    pub(super) fn int(i: i64) -> Dec {
        Dec {
            m: i128::from(i),
            s: 0,
        }
    }

    fn norm(mut self) -> Dec {
        while self.s > 0 && self.m % 10 == 0 {
            self.m /= 10;
            self.s -= 1;
        }
        self
    }

    /// A decimal written `[+-]digits[.digits][e[+-]digits]`, blanks around it allowed; none for another text or one
    /// past [`MOST_SCALE`] digits.
    pub(super) fn parse(t: &str) -> Option<Dec> {
        let t = t.trim_matches(|c: char| c.is_ascii_whitespace());
        let (neg, t) = match t.as_bytes().first()? {
            b'-' => (true, &t[1..]),
            b'+' => (false, &t[1..]),
            _ => (false, t),
        };
        let (mantissa, exp) = match t.find(['e', 'E']) {
            Some(k) => (&t[..k], t[k + 1..].parse::<i32>().ok()?),
            None => (t, 0),
        };
        let (whole, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        if (whole.is_empty() && frac.is_empty())
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !frac.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let digits = format!("{whole}{frac}");
        let digits = digits.trim_start_matches('0');
        if digits.len() > MOST_SCALE as usize {
            return None;
        }
        let mut m: i128 = if digits.is_empty() {
            0
        } else {
            digits.parse().ok()?
        };
        let mut s = i64::try_from(frac.len()).ok()? - i64::from(exp);
        if s < 0 {
            m = m.checked_mul(pow10(u32::try_from(-s).ok()?)?)?;
            s = 0;
        }
        let s = u32::try_from(s).ok()?;
        if s > 2 * MOST_SCALE {
            return None;
        }
        let d = Dec {
            m: if neg { -m } else { m },
            s,
        }
        .norm();
        (d.s <= MOST_SCALE).then_some(d)
    }

    fn aligned(a: Dec, b: Dec) -> Option<(i128, i128, u32)> {
        let s = a.s.max(b.s);
        Some((
            a.m.checked_mul(pow10(s - a.s)?)?,
            b.m.checked_mul(pow10(s - b.s)?)?,
            s,
        ))
    }

    pub(super) fn order(a: Dec, b: Dec) -> Option<Ordering> {
        let (x, y, _) = Dec::aligned(a, b)?;
        Some(x.cmp(&y))
    }

    pub(super) fn add(a: Dec, b: Dec) -> Option<Dec> {
        let (x, y, s) = Dec::aligned(a, b)?;
        Some(
            Dec {
                m: x.checked_add(y)?,
                s,
            }
            .norm(),
        )
    }

    pub(super) fn sub(a: Dec, b: Dec) -> Option<Dec> {
        Dec::add(a, b.neg()?)
    }

    pub(super) fn mul(a: Dec, b: Dec) -> Option<Dec> {
        let d = Dec {
            m: a.m.checked_mul(b.m)?,
            s: a.s + b.s,
        }
        .norm();
        (d.s <= MOST_SCALE).then_some(d)
    }

    pub(super) fn neg(self) -> Option<Dec> {
        Some(Dec {
            m: self.m.checked_neg()?,
            s: self.s,
        })
    }

    pub(super) fn abs(self) -> Option<Dec> {
        Some(Dec {
            m: self.m.checked_abs()?,
            s: self.s,
        })
    }

    pub(super) fn is_zero(self) -> bool {
        self.m == 0
    }

    /// Its whole part, toward zero, when it fits.
    pub(super) fn trunc(self) -> Option<i64> {
        i64::try_from(self.m / pow10(self.s)?).ok()
    }

    /// Whether its fraction is a half exactly.
    pub(super) fn is_half(self) -> bool {
        self.s == 1 && self.m.abs() % 10 == 5
    }

    /// The whole number it is, when it is one that fits.
    pub(super) fn whole(self) -> Option<i64> {
        (self.s == 0).then(|| i64::try_from(self.m).ok()).flatten()
    }

    /// Rounded to `digits` decimals, halves away from zero (PostgreSQL's `numeric`).
    pub(super) fn round(self, digits: u32) -> Option<Dec> {
        if self.s <= digits {
            return Some(self);
        }
        let cut = pow10(self.s - digits)?;
        let (q, r) = (self.m / cut, self.m % cut);
        let half = cut / 2;
        let q = if r.abs() >= half {
            q + self.m.signum()
        } else {
            q
        };
        Some(Dec { m: q, s: digits }.norm())
    }

    /// As its shortest decimal text (`1.5`, `-3`, `0.25`).
    pub(super) fn text(self) -> String {
        let neg = self.m < 0;
        let digits = self.m.unsigned_abs().to_string();
        let s = self.s as usize;
        let body = if s == 0 {
            digits
        } else if digits.len() > s {
            format!(
                "{}.{}",
                &digits[..digits.len() - s],
                &digits[digits.len() - s..]
            )
        } else {
            format!("0.{}{digits}", "0".repeat(s - digits.len()))
        };
        if neg { format!("-{body}") } else { body }
    }
}

/// A number that need not be whole: its exact decimal (none when an operation left it inexact) and the binary float a
/// database working in floats holds.
#[derive(Clone, Copy, Debug)]
pub(super) struct Real {
    pub exact: Option<Dec>,
    pub approx: f64,
}

impl Real {
    pub(super) fn parse(t: &str) -> Option<Real> {
        let approx: f64 = t
            .trim_matches(|c: char| c.is_ascii_whitespace())
            .parse()
            .ok()?;
        approx.is_finite().then(|| Real {
            exact: Dec::parse(t),
            approx,
        })
    }

    pub(super) fn of_int(i: i64) -> Real {
        Real {
            exact: Some(Dec::int(i)),
            approx: i as f64,
        }
    }
}

/// A day as `yyyymmdd`, sortable.
pub(super) fn day(t: &str) -> Option<i64> {
    let b = t.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        let s = &t[r];
        s.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| s.parse().ok())?
    };
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    (y >= 1 && (1..=days).contains(&d)).then_some(y * 10_000 + m * 100 + d)
}

/// A time of day in nanoseconds, `HH:MM:SS[.f]` with at most six decimals (PostgreSQL's microseconds); `24:00:00`
/// is the day's end.
pub(super) fn clock(t: &str) -> Option<u64> {
    let (hms, frac) = t.split_once('.').unwrap_or((t, ""));
    let b = hms.as_bytes();
    if b.len() != 8
        || b[2] != b':'
        || b[5] != b':'
        || frac.len() > 6
        || !frac.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<u64> {
        let s = &hms[r];
        s.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| s.parse().ok())?
    };
    let (h, m, s) = (num(0..2)?, num(3..5)?, num(6..8)?);
    let nanos = if frac.is_empty() {
        0
    } else {
        format!("{frac:0<9}").parse::<u64>().ok()?
    };
    let end = h == 24 && m == 0 && s == 0 && nanos == 0;
    (end || (h < 24 && m < 60 && s < 60)).then_some(((h * 60 + m) * 60 + s) * 1_000_000_000 + nanos)
}

/// A date and time, `YYYY-MM-DD` then `T` or a blank then a time of day.
pub(super) fn stamp(t: &str) -> Option<(i64, u64)> {
    if t.len() < 11 || !matches!(t.as_bytes()[10], b'T' | b' ') {
        return None;
    }
    Some((day(t.get(..10)?)?, clock(t.get(11..)?)?))
}

/// A value.
#[derive(Clone, Debug)]
pub(super) enum V {
    Null,
    /// A value KentOS cannot know before the table answers.
    Unknown,
    Bool(bool),
    Int(i64),
    Real(Real),
    Text(String),
    Date(i64),
    Time(u64),
    Stamp(i64, u64),
    Array(Vec<V>),
}

/// A value with SQLite's affinity and the collation an explicit `COLLATE` gave it.
#[derive(Clone, Debug)]
pub(super) struct Val {
    pub v: V,
    pub aff: Aff,
    pub coll: Option<String>,
}

impl Val {
    pub(super) fn of(v: V) -> Val {
        Val {
            v,
            aff: Aff::None,
            coll: None,
        }
    }

    pub(super) fn unknown() -> Val {
        Val::of(V::Unknown)
    }

    pub(super) fn null() -> Val {
        Val::of(V::Null)
    }
}

/// SQL's three truths and KentOS's “not known”.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum T3 {
    True,
    False,
    Null,
    Unknown,
}

impl T3 {
    pub(super) fn of(b: bool) -> T3 {
        if b { T3::True } else { T3::False }
    }

    pub(super) fn not(self) -> T3 {
        match self {
            T3::True => T3::False,
            T3::False => T3::True,
            other => other,
        }
    }

    pub(super) fn and(self, o: T3) -> T3 {
        match (self, o) {
            (T3::False, _) | (_, T3::False) => T3::False,
            (T3::Unknown, _) | (_, T3::Unknown) => T3::Unknown,
            (T3::Null, _) | (_, T3::Null) => T3::Null,
            _ => T3::True,
        }
    }

    pub(super) fn or(self, o: T3) -> T3 {
        match (self, o) {
            (T3::True, _) | (_, T3::True) => T3::True,
            (T3::Unknown, _) | (_, T3::Unknown) => T3::Unknown,
            (T3::Null, _) | (_, T3::Null) => T3::Null,
            _ => T3::False,
        }
    }
}

/// How two values compare: in order, different without a known order (PostgreSQL's text under its collation), NULL, or
/// not known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Order {
    Lt,
    Eq,
    Gt,
    Ne,
    Null,
    Unknown,
}

impl Order {
    pub(super) fn of(o: Ordering) -> Order {
        match o {
            Ordering::Less => Order::Lt,
            Ordering::Equal => Order::Eq,
            Ordering::Greater => Order::Gt,
        }
    }
}

/// The affinity SQLite gives a column of a field's kind as KentOS writes it (docs/adr/0755 §3): the reader marks a
/// constraint over a column of another one as the table's only.
pub(crate) fn natural(kind: LayerFieldKind) -> Aff {
    match kind {
        LayerFieldKind::Text | LayerFieldKind::Uuid | LayerFieldKind::Json => Aff::Text,
        LayerFieldKind::Integer | LayerFieldKind::BigInt => Aff::Integer,
        LayerFieldKind::Decimal => Aff::Real,
        LayerFieldKind::Binary => Aff::Blob,
        LayerFieldKind::Boolean
        | LayerFieldKind::Date
        | LayerFieldKind::DateTime
        | LayerFieldKind::Time
        | LayerFieldKind::Instant => Aff::Numeric,
    }
}

/// A field's value as the table holds it after KentOS wrote `attr` (docs/adr/0755 §3): none is NULL, an empty text
/// is `''`. A value the database works out (a generated column) and one KentOS writes in another form than its text
/// (SQLite's date and time, an instant, JSON in PostgreSQL, bytes) are not known.
pub(super) fn field_value(f: &LayerField, attr: Option<&str>, d: Dialect) -> Val {
    let aff = if d == Dialect::Sqlite {
        natural(f.kind)
    } else {
        Aff::None
    };
    let with = |v: V| Val { v, aff, coll: None };
    let generated = f
        .column
        .as_ref()
        .and_then(|c| c.making())
        .is_some_and(|m| matches!(m, ColumnMade::Generated(_)));
    if generated {
        return with(V::Unknown);
    }
    let Some(t) = attr else {
        return with(V::Null);
    };
    use LayerFieldKind as K;
    let v = match (d, f.kind) {
        (_, K::Text) => V::Text(t.to_owned()),
        (Dialect::Sqlite, K::Uuid | K::Json) => V::Text(t.to_owned()),
        (Dialect::Postgres, K::Uuid) => V::Text(t.to_ascii_lowercase()),
        (_, K::Integer | K::BigInt) => match t.parse::<i64>() {
            Ok(i) => V::Int(i),
            Err(_) if d == Dialect::Sqlite => V::Text(t.to_owned()),
            Err(_) => V::Unknown,
        },
        (_, K::Decimal) => match Real::parse(t) {
            Some(r) => V::Real(r),
            None if d == Dialect::Sqlite => V::Text(t.to_owned()),
            None => V::Unknown,
        },
        (Dialect::Sqlite, K::Boolean) => match t {
            "true" => V::Int(1),
            "false" => V::Int(0),
            _ => V::Text(t.to_owned()),
        },
        (Dialect::Postgres, K::Boolean) => match t {
            "true" => V::Bool(true),
            "false" => V::Bool(false),
            _ => V::Unknown,
        },
        (Dialect::Sqlite, K::Date | K::Time) => V::Text(t.to_owned()),
        (Dialect::Postgres, K::Date) => day(t).map_or(V::Unknown, V::Date),
        (Dialect::Postgres, K::Time) => clock(t).map_or(V::Unknown, V::Time),
        (Dialect::Postgres, K::DateTime) => stamp(t).map_or(V::Unknown, |(d, c)| V::Stamp(d, c)),
        (_, K::DateTime | K::Instant | K::Json | K::Binary) => V::Unknown,
    };
    with(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_are_exact() {
        let d = |t: &str| Dec::parse(t).unwrap_or_else(|| panic!("{t}"));
        assert_eq!(d("1.50").text(), "1.5");
        assert_eq!(d("-0.25").text(), "-0.25");
        assert_eq!(d("1e3").text(), "1000");
        assert_eq!(d("1.5e-3").text(), "0.0015");
        assert_eq!(d(" 007 ").text(), "7");
        assert_eq!(d(".5").text(), "0.5");
        assert_eq!(Dec::order(d("0.1"), d("0.10")), Some(Ordering::Equal));
        assert_eq!(
            Dec::order(d("0.30000000000000004"), d("0.3")),
            Some(Ordering::Greater)
        );
        assert_eq!(
            Dec::add(d("0.1"), d("0.2")).map(Dec::text).as_deref(),
            Some("0.3")
        );
        assert_eq!(
            Dec::mul(d("1.5"), d("-2")).map(Dec::text).as_deref(),
            Some("-3")
        );
        assert_eq!(d("2.675").round(2).map(Dec::text).as_deref(), Some("2.68"));
        assert_eq!(d("-2.5").round(0).map(Dec::text).as_deref(), Some("-3"));
        assert_eq!(d("2.4").round(0).map(Dec::text).as_deref(), Some("2"));
        for bad in ["", ".", "1.2.3", "abc", "1e", "--1", "0x10"] {
            assert_eq!(Dec::parse(bad), None, "{bad}");
        }
        assert_eq!(Dec::parse(&"9".repeat(60)), None);
    }

    #[test]
    fn days_times_and_stamps() {
        assert_eq!(day("2024-02-29"), Some(20240229));
        assert_eq!(day("2023-02-29"), None);
        assert_eq!(day("2024-13-01"), None);
        assert_eq!(day("2024-1-01"), None);
        assert_eq!(clock("10:30:00"), Some(37_800_000_000_000));
        assert_eq!(clock("10:30:00.5"), Some(37_800_500_000_000));
        assert_eq!(clock("24:00:00"), Some(86_400_000_000_000));
        assert_eq!(clock("24:00:01"), None);
        assert_eq!(clock("10:30:00.1234567"), None);
        assert_eq!(
            stamp("2024-01-15T10:00:00"),
            Some((20240115, 36_000_000_000_000))
        );
        assert_eq!(
            stamp("2024-01-15 10:00:00"),
            Some((20240115, 36_000_000_000_000))
        );
        assert_eq!(stamp("2024-01-15"), None);
    }

    #[test]
    fn truths() {
        use T3::*;
        assert_eq!(False.and(Unknown), False);
        assert_eq!(Null.and(Unknown), Unknown);
        assert_eq!(True.and(Null), Null);
        assert_eq!(True.or(Unknown), True);
        assert_eq!(False.or(Unknown), Unknown);
        assert_eq!(Null.or(False), Null);
        assert_eq!(Unknown.not(), Unknown);
    }
}

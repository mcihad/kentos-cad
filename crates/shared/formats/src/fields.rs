//! A layer's fields from a GIS file (docs/adr/0199 §6): a Shapefile's DBF
//! fields and the kinds of a GeoJSON's properties, and the values of a
//! number field in their canonical text. The rules are written out in the
//! ADR and followed by the independent reader in tools/formats/gis.py; both
//! read fixtures/formats/v1/gis alike.
//!
//! - A DBF C field is text (its width its length), N without decimals an
//!   integer (a decimal of no fraction when it is wider than 15), N with
//!   decimals a decimal of as many (at most 15), F a decimal, D a date, L a
//!   yes or no; a field of another type, without a name, or of a name taken
//!   (folded the Turkish way) is none.
//! - A GeoJSON key whose values in a layer are all JSON integers (within
//!   2⁵³ − 1) is an integer field, all numbers written without an exponent
//!   a decimal (of the most fraction digits, at most 15, else text), all
//!   booleans a yes or no, any other mix text; a key with only nulls none.
//! - A number field's value is written in its canonical text, an exponent
//!   written out in full, exactly (`1.2e3` → `1200`); one that does not keep
//!   the field's rules stays as read. Other fields' values stay as read.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{LayerField, LayerFieldKind, check_value, field_problem, fold, js_trim};

/// A GeoJSON property value's JSON kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonKind {
    Number,
    Boolean,
    /// A string, an object or an array (written as JSON text).
    Other,
}

/// The layer field of a DBF field, or none for a type the reading leaves out.
pub fn dbf_field(name: &str, kind: u8, width: usize, decimals: u8) -> Option<LayerField> {
    let field = |k| LayerField::new(name, k);
    Some(match kind {
        b'C' => LayerField {
            length: (1..=10_000).contains(&width).then_some(width as u32),
            ..field(LayerFieldKind::Text)
        },
        b'N' if decimals == 0 && width <= 15 => field(LayerFieldKind::Integer),
        b'N' if decimals == 0 => LayerField {
            scale: Some(0),
            ..field(LayerFieldKind::Decimal)
        },
        b'N' => LayerField {
            scale: (decimals <= 15).then_some(u32::from(decimals)),
            ..field(LayerFieldKind::Decimal)
        },
        b'F' => field(LayerFieldKind::Decimal),
        b'D' => field(LayerFieldKind::Date),
        b'L' => field(LayerFieldKind::Boolean),
        _ => return None,
    })
}

/// The fields of a DBF's descriptors (name, type, width, decimals), in order:
/// the known types, a usable name once (folded).
pub fn dbf_fields<'a>(
    descriptors: impl IntoIterator<Item = (&'a str, u8, usize, u8)>,
) -> Vec<LayerField> {
    let mut taken = HashSet::new();
    descriptors
        .into_iter()
        .filter_map(|(name, kind, width, decimals)| {
            let f = dbf_field(name, kind, width, decimals)?;
            (field_problem(&f).is_none() && taken.insert(fold(name))).then_some(f)
        })
        .collect()
}

/// The fields of a layer's GeoJSON properties: `seen` the key, JSON kind and
/// text of each value in reading order; the keys in their first order.
pub fn geojson_fields(seen: &[(String, JsonKind, String)]) -> Vec<LayerField> {
    let mut keys: Vec<&str> = Vec::new();
    let mut by_key: BTreeMap<&str, Vec<(JsonKind, &str)>> = BTreeMap::new();
    for (key, kind, text) in seen {
        let list = by_key.entry(key).or_default();
        if list.is_empty() {
            keys.push(key);
        }
        list.push((*kind, text));
    }
    keys.into_iter()
        .map(|key| {
            let values = &by_key[key];
            let all = |k: JsonKind| values.iter().all(|(kind, _)| *kind == k);
            let field = |k| LayerField::new(key, k);
            if all(JsonKind::Boolean) {
                return field(LayerFieldKind::Boolean);
            }
            if !all(JsonKind::Number) {
                return field(LayerFieldKind::Text);
            }
            let integer = values.iter().all(|(_, t)| {
                let digits = t.strip_prefix('-').unwrap_or(t);
                !digits.is_empty()
                    && digits.bytes().all(|b| b.is_ascii_digit())
                    && check_value(&field(LayerFieldKind::Integer), t).is_ok()
            });
            if integer {
                return field(LayerFieldKind::Integer);
            }
            let mut scale = 0;
            for (_, t) in values {
                let body = t.strip_prefix('-').unwrap_or(t);
                let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
                let plain = !whole.is_empty()
                    && whole.bytes().all(|b| b.is_ascii_digit())
                    && fraction.bytes().all(|b| b.is_ascii_digit())
                    && (!body.contains('.') || !fraction.is_empty())
                    && whole.len() + fraction.len() <= 30;
                if !plain {
                    return field(LayerFieldKind::Text);
                }
                scale = scale.max(fraction.len());
            }
            if scale > 15 {
                return field(LayerFieldKind::Text);
            }
            LayerField {
                scale: Some(scale as u32),
                ..field(LayerFieldKind::Decimal)
            }
        })
        .collect()
}

/// An exponent's number written out in full, exactly (`1.2e3` → `1200`,
/// `-4.5e-1` → `-0.45`); any other text as it is.
pub fn plain_number(text: &str) -> String {
    let (minus, rest) = match text.as_bytes().first() {
        Some(b'+') => ("", &text[1..]),
        Some(b'-') => ("-", &text[1..]),
        _ => ("", text),
    };
    let Some(at) = rest.find(['e', 'E']) else {
        return text.to_owned();
    };
    let (mantissa, exp) = (&rest[..at], &rest[at + 1..]);
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = |t: &str| t.bytes().all(|b| b.is_ascii_digit());
    let exp_digits = exp.strip_prefix(['+', '-']).unwrap_or(exp);
    if whole.is_empty()
        || !digits(whole)
        || !digits(fraction)
        || exp_digits.is_empty()
        || !digits(exp_digits)
    {
        return text.to_owned();
    }
    let Ok(exp) = exp.parse::<i64>() else {
        return text.to_owned();
    };
    if exp.abs() > 400 {
        return text.to_owned();
    }
    let all = format!("{whole}{fraction}");
    let point = whole.len() as i64 + exp;
    let body = if point <= 0 {
        format!("0.{}{all}", "0".repeat((-point) as usize))
    } else if point as usize >= all.len() {
        format!("{all}{}", "0".repeat(point as usize - all.len()))
    } else {
        format!("{}.{}", &all[..point as usize], &all[point as usize..])
    };
    format!("{minus}{body}")
}

/// The attributes with each number field's value in its canonical text (one
/// the field refuses stays as it is); other fields' values as they are.
pub fn canonical_attrs(fields: &[LayerField], attrs: &mut BTreeMap<String, String>) {
    for f in fields.iter().filter(|f| f.kind.is_number()) {
        if let Some(v) = attrs.get_mut(&f.name)
            && let Ok(canonical) = check_value(f, &plain_number(js_trim(v)))
            && !canonical.is_empty()
        {
            *v = canonical;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exponent_is_written_out_exactly() {
        assert_eq!(plain_number("1.23456780000e+003"), "1234.56780000");
        assert_eq!(plain_number("-4.50000000000e-001"), "-0.450000000000");
        assert_eq!(plain_number("0.00000000000e+000"), "0.00000000000");
        assert_eq!(plain_number("12E2"), "1200");
        assert_eq!(plain_number("12.5"), "12.5");
        assert_eq!(plain_number("1e99999"), "1e99999");
    }
}

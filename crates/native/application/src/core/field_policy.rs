//! A field's value when its object is split or merged (docs/adr/0419): the pieces keep it, empty it,
//! take the default or share it by their areas (lines: lengths); the merged object takes the first
//! one's, empties it, or the sum, least or greatest. Sharing and summing are exact in the field's
//! last digit: the value is whole units (an integer, or a decimal times ten to its fraction digits);
//! a share is each piece's floor of total × weight / weights, and what is left goes one unit at a time
//! to the pieces with the largest remainders, the earlier first on a tie (CLAUDE.md §23.2: a total
//! is kept, its leftover not given to the last piece by chance).

use kentos_commands::{LayerField, LayerFieldKind, MergePolicy, SplitPolicy};

/// A value in whole units of `field`'s last digit; none when it is not one of its numbers.
fn units(field: &LayerField, value: &str) -> Option<i128> {
    let scale = match field.kind {
        LayerFieldKind::Integer | LayerFieldKind::BigInt => 0,
        LayerFieldKind::Decimal => field.scale? as usize,
        _ => return None,
    };
    let (minus, rest) = match value.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, value),
    };
    let (whole, fraction) = rest.split_once('.').unwrap_or((rest, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > scale
    {
        return None;
    }
    let mut n: i128 = 0;
    for b in whole.bytes().chain(fraction.bytes()) {
        n = n.checked_mul(10)?.checked_add(i128::from(b - b'0'))?;
    }
    for _ in fraction.len()..scale {
        n = n.checked_mul(10)?;
    }
    Some(if minus { -n } else { n })
}

/// Whole units as `field` writes them: an integer, or a decimal with all its fraction digits.
fn text_of(field: &LayerField, n: i128) -> String {
    let scale = if field.kind == LayerFieldKind::Decimal {
        field.scale.unwrap_or(0) as usize
    } else {
        0
    };
    let digits = n.unsigned_abs().to_string();
    let sign = if n < 0 { "-" } else { "" };
    if scale == 0 {
        return format!("{sign}{digits}");
    }
    let padded = format!("{digits:0>width$}", width = scale + 1);
    let (whole, fraction) = padded.split_at(padded.len() - scale);
    format!("{sign}{whole}.{fraction}")
}

/// `total` shared by `weights` (docs/adr/0419): each its floor of total × weight / sum, the rest one
/// unit each to the largest remainders, the earlier first on a tie; equal weights when they sum to
/// nothing.
pub fn share_units(total: i128, weights: &[f64]) -> Vec<i128> {
    let n = weights.len();
    if n == 0 {
        return Vec::new();
    }
    let usable = weights.iter().all(|w| w.is_finite() && *w >= 0.0);
    let sum: f64 = weights.iter().sum();
    let weights: Vec<f64> = if usable && sum > 0.0 {
        weights.to_vec()
    } else {
        vec![1.0; n]
    };
    let sum: f64 = weights.iter().sum();
    let whole = total.unsigned_abs();
    let mut out = Vec::with_capacity(n);
    let mut rest = Vec::with_capacity(n);
    let mut given: u128 = 0;
    for (i, w) in weights.iter().enumerate() {
        let q = whole as f64 * w / sum;
        let floor = q.floor();
        let part = (floor as u128).min(whole);
        given += part;
        out.push(part);
        rest.push((q - floor, i));
    }
    // Largest remainders first; the earlier piece on a tie.
    rest.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut left = whole.saturating_sub(given);
    for (_, i) in rest.iter().cycle() {
        if left == 0 {
            break;
        }
        out[*i] += 1;
        left -= 1;
    }
    out.into_iter()
        .map(|u| {
            let v = u as i128;
            if total < 0 { -v } else { v }
        })
        .collect()
}

/// The pieces' values of `field` when the object holding `value` is split into pieces weighing
/// `weights` (their areas, else lengths); none to leave each piece's value as it is.
pub fn split(
    field: &LayerField,
    value: Option<&str>,
    weights: &[f64],
) -> Option<Vec<Option<String>>> {
    let n = weights.len();
    match field.split? {
        SplitPolicy::Copy => None,
        SplitPolicy::Clear => Some(vec![None; n]),
        SplitPolicy::Default => Some(vec![field.default.clone(); n]),
        SplitPolicy::Ratio => {
            let total = units(field, value?)?;
            Some(
                share_units(total, weights)
                    .into_iter()
                    .map(|u| Some(text_of(field, u)))
                    .collect(),
            )
        }
    }
}

/// The merged object's value of `field` from the objects' values in order (the first is the object
/// that stays); none to leave the first one's.
pub fn merge(field: &LayerField, values: &[Option<&str>]) -> Option<Option<String>> {
    let policy = field.merge?;
    let given: Vec<&str> = values
        .iter()
        .flatten()
        .copied()
        .filter(|v| !v.is_empty())
        .collect();
    match policy {
        MergePolicy::First => None,
        MergePolicy::Clear => Some(None),
        MergePolicy::Sum => {
            let mut sum: i128 = 0;
            for v in &given {
                sum = sum.checked_add(units(field, v)?)?;
            }
            Some((!given.is_empty()).then(|| text_of(field, sum)))
        }
        MergePolicy::Min | MergePolicy::Max => {
            let number = |v: &str| v.parse::<f64>().ok().filter(|x| x.is_finite());
            let mut best: Option<(&str, f64)> = None;
            for v in &given {
                let Some(x) = number(v) else { continue };
                let better = match best {
                    None => true,
                    Some((_, b)) if policy == MergePolicy::Min => x < b,
                    Some((_, b)) => x > b,
                };
                if better {
                    best = Some((v, x));
                }
            }
            Some(best.map(|(v, _)| v.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_share_keeps_its_total_and_gives_the_rest_to_the_largest_remainders() {
        assert_eq!(share_units(100, &[1.0, 1.0, 1.0]), [34, 33, 33]);
        assert_eq!(share_units(-10, &[3.0, 1.0]), [-8, -2]);
        assert_eq!(share_units(7, &[0.0, 0.0]), [4, 3]);
        let field = LayerField {
            scale: Some(2),
            ..LayerField::new("Pay", LayerFieldKind::Decimal)
        };
        assert_eq!(units(&field, "12.5"), Some(1250));
        assert_eq!(text_of(&field, 5), "0.05");
        assert_eq!(text_of(&field, -1250), "-12.50");
    }
}

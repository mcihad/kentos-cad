//! The attribute table's rows (docs/adr/0199 §4), one for both platforms
//! (the web through WASM): which objects of a layer the table shows and in
//! what order. A row arrives as its cells, each the text it shows and, when
//! its value is there and keeps its field's rules, its sort key (the value's
//! canonical text, `kentos_contracts::fields`; a value list's label), as each
//! platform reads them from its objects and the layer's fields; the drawing's
//! order is the rows' order. The independent reference is
//! `scripts/fixtures/feature_table_cases.py`.
//!
//! A row is shown by the Göster choice (every row, the selected ones, the
//! ones in the view) when the expression filter keeps it and, with a search,
//! one of its searched cells answers (`text::edit::search`). A sort by a
//! column orders numbers by their exact value, dates by the calendar, false
//! before true, texts in the natural order (`text::natural`); a row without
//! a key comes after every row with one whichever the way, equal ones keep
//! the drawing's order.

use std::cmp::Ordering;

use crate::api::Op;
use crate::op;
use crate::text::edit::search;
use crate::text::natural::natural_cmp;
use crate::tools::point_text::js_trim;

/// A column: how its keys are ordered (`text`, `number`, `date`, `boolean`;
/// any other word: as text) and whether the search looks in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub order: String,
    pub searched: bool,
}

crate::json_struct!(Column { order, searched });

/// A cell: the text it shows, and its sort key when its value is there and
/// keeps its field's rules.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    pub shown: String,
    pub key: Option<String>,
}

crate::json_struct!(Cell { shown, key });

/// An object of the layer as the table reads it: its cells (one per
/// column), whether it is selected, in the view and kept by the expression
/// filter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub selected: bool,
    pub in_view: bool,
    pub passes: bool,
}

crate::json_struct!(Row {
    cells,
    selected,
    in_view => "inView",
    passes
});

/// What the table shows: the search box's text, the Göster choice (`all`,
/// `selected`, `inView`; any other word every row), the column it is sorted
/// by (none, or one the table does not have: the drawing's order) and which
/// way.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Query {
    pub search: String,
    pub show: String,
    pub sort: Option<usize>,
    pub descending: bool,
}

crate::json_struct!(Query {
    search,
    show,
    sort,
    descending
});

/// A decimal key as its sign (−1, 0, 1), its whole digits without leading
/// zeros and its fraction digits without trailing ones; none when it is not
/// one (an optional minus, digits, an optional fraction).
fn decimal(s: &str) -> Option<(i8, &str, &str)> {
    let (minus, rest) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (whole, fraction) = rest.split_once('.').unwrap_or((rest, ""));
    let digits = |t: &str| t.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty() || !digits(whole) || !digits(fraction) {
        return None;
    }
    let whole = match whole.trim_start_matches('0') {
        "" => "0",
        w => w,
    };
    let fraction = fraction.trim_end_matches('0');
    let sign = if whole == "0" && fraction.is_empty() {
        0
    } else if minus {
        -1
    } else {
        1
    };
    Some((sign, whole, fraction))
}

/// Two numbers' order by their exact values; one that is not a number after
/// one that is.
fn numbers(a: &str, b: &str) -> Ordering {
    match (decimal(a), decimal(b)) {
        (Some((sa, wa, fa)), Some((sb, wb, fb))) => {
            let magnitude = wa
                .len()
                .cmp(&wb.len())
                .then_with(|| wa.cmp(wb))
                .then_with(|| fa.cmp(fb));
            sa.cmp(&sb).then(if sa < 0 {
                magnitude.reverse()
            } else {
                magnitude
            })
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Two keys of a column by its order.
fn keys(order: &str, a: &str, b: &str) -> Ordering {
    match order {
        "number" => numbers(a, b),
        // YYYY-AA-GG: the calendar's order is the text's.
        "date" => a.cmp(b),
        "boolean" => (a == "true").cmp(&(b == "true")),
        _ => natural_cmp(a, b),
    }
}

/// The rows the table shows, by their index in `rows` (the drawing's
/// order), in the order it shows them (docs/adr/0199 §4).
pub fn feature_table(columns: &[Column], rows: &[Row], query: &Query) -> Vec<u32> {
    let wanted = js_trim(&query.search);
    let searched: Vec<usize> = (0..columns.len())
        .filter(|&c| columns[c].searched)
        .collect();
    let mut out: Vec<u32> = (0..rows.len())
        .filter(|&i| {
            let r = &rows[i];
            r.passes
                && match query.show.as_str() {
                    "selected" => r.selected,
                    "inView" => r.in_view,
                    _ => true,
                }
                && (wanted.is_empty()
                    || searched
                        .iter()
                        .any(|&c| r.cells.get(c).is_some_and(|x| search(&x.shown, wanted))))
        })
        .map(|i| i as u32)
        .collect();
    let Some(column) = query.sort.filter(|&c| c < columns.len()) else {
        return out;
    };
    let order = columns[column].order.as_str();
    let key = |i: u32| {
        rows[i as usize]
            .cells
            .get(column)
            .and_then(|c| c.key.as_deref())
    };
    out.sort_by(|&a, &b| {
        let by = match (key(a), key(b)) {
            (Some(x), Some(y)) => {
                let o = keys(order, x, y);
                if query.descending { o.reverse() } else { o }
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        };
        by.then(a.cmp(&b))
    });
    out
}

pub(crate) static OPS: &[Op] = &[op!(
    "featureTable",
    |columns: Vec<Column>, rows: Vec<Row>, query: Query| feature_table(&columns, &rows, &query)
)];

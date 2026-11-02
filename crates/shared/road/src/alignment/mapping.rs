//! What an edit did to an alignment's stations (docs/adr/0901 §9, ROAD-ALIGN-10, 16): its elements before and after,
//! matched by id. An element the same and at the same station is kept; the same but at another station has moved;
//! one whose geometry differs has changed; others came or went. Each row says the station range before and after, so
//! what hangs on a station (a profile, a section line, a quantity) follows or is marked unresolved.

use super::chain::Alignment;
use super::element::Element;

/// What happened to an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Kept,
    Moved,
    Changed,
    Added,
    Removed,
}

impl Change {
    pub fn word(self) -> &'static str {
        match self {
            Change::Kept => "kept",
            Change::Moved => "moved",
            Change::Changed => "changed",
            Change::Added => "added",
            Change::Removed => "removed",
        }
    }
}

/// A row of the mapping: an element's id and station range before and after.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub element: String,
    pub change: Change,
    pub before: Option<[f64; 2]>,
    pub after: Option<[f64; 2]>,
}

/// Places and directions are the same within this.
const SAME: f64 = 1e-9;

fn same_shape(a: &Element, b: &Element) -> bool {
    a.kind == b.kind
        && (a.length - b.length).abs() <= SAME
        && (a.k0 - b.k0).abs() <= SAME
        && (a.k1 - b.k1).abs() <= SAME
        && (a.start[0] - b.start[0]).abs() <= SAME
        && (a.start[1] - b.start[1]).abs() <= SAME
        && super::chain::wrap(a.theta0 - b.theta0).abs() <= SAME
}

/// The mapping from `before` to `after`: rows in the new order, then the removed ones in the old order.
pub fn mapping(before: &Alignment, after: &Alignment) -> Vec<Row> {
    let range = |e: &Element| [e.station, e.end_station()];
    let mut rows: Vec<Row> = after
        .elements
        .iter()
        .map(|n| match before.elements.iter().find(|o| o.id == n.id) {
            Some(o) => Row {
                element: n.id.clone(),
                change: if !same_shape(o, n) {
                    Change::Changed
                } else if (o.station - n.station).abs() <= SAME {
                    Change::Kept
                } else {
                    Change::Moved
                },
                before: Some(range(o)),
                after: Some(range(n)),
            },
            None => Row {
                element: n.id.clone(),
                change: Change::Added,
                before: None,
                after: Some(range(n)),
            },
        })
        .collect();
    rows.extend(before.elements.iter().filter(|o| after.elements.iter().all(|n| n.id != o.id)).map(|o| Row {
        element: o.id.clone(),
        change: Change::Removed,
        before: Some(range(o)),
        after: None,
    }));
    rows
}

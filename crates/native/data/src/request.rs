//! What a layer asks a table for (docs/adr/9100 §4): the box (in the table's system), the condition, the columns, the
//! order, the page, the geometry or not and its simplification. A driver does with the source what the source can
//! do (the box by its spatial index, the condition and the order in its SQL, the simplification by its functions) and
//! the rest itself; the rows it hands over are the same either way, only the time differs.

use crate::condition::Condition;

/// An order by a field: rising or falling, NULLs after every value either way; rows with equal values by key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    pub field: String,
    pub descending: bool,
}

/// A request for rows.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    /// Rows whose geometry's box meets it (min x, min y, max x, max y, in the table's system).
    pub bbox: Option<[f64; 4]>,
    /// With `bbox`, rows whose geometry itself meets the box (not only its box): an identify or a pick.
    pub exact: bool,
    pub condition: Option<Condition>,
    /// The fields read, by name, in this order; none: all of them in the table's order.
    pub columns: Option<Vec<String>>,
    /// Whether the geometry is read.
    pub geometry: bool,
    /// Lines and areas simplified with this tolerance (in the table's units) for drawing: a vertex closer than it to
    /// the line through its neighbours may go, a ring never falls below four positions.
    pub simplify: Option<f64>,
    /// Rows by these fields, then by key; none: by key.
    pub order: Vec<Order>,
    /// At most this many rows.
    pub limit: Option<u64>,
    /// The first rows of the order left out.
    pub offset: u64,
    /// Rows handed over at a time.
    pub chunk: usize,
}

impl Default for Request {
    fn default() -> Self {
        Request {
            bbox: None,
            exact: false,
            condition: None,
            columns: None,
            geometry: true,
            simplify: None,
            order: Vec::new(),
            limit: None,
            offset: 0,
            chunk: 2048,
        }
    }
}

impl Request {
    /// Every row, geometry and all fields, by key.
    pub fn all() -> Request {
        Request::default()
    }

    /// The rows in a box, for drawing: geometry simplified with `tolerance`, the fields `columns` only.
    pub fn view(bbox: [f64; 4], columns: Vec<String>, tolerance: Option<f64>) -> Request {
        Request {
            bbox: Some(bbox),
            columns: Some(columns),
            simplify: tolerance.filter(|t| *t > 0.0),
            ..Request::default()
        }
    }

    /// The rows of these keys.
    pub fn fids(fids: Vec<i64>) -> Request {
        Request {
            condition: Some(Condition::Fids(fids)),
            ..Request::default()
        }
    }

    /// The same request without its page and order (what a count asks).
    pub fn counted(&self) -> Request {
        Request {
            order: Vec::new(),
            limit: None,
            offset: 0,
            columns: Some(Vec::new()),
            geometry: false,
            ..self.clone()
        }
    }
}

/// Whether two boxes meet (edges touching count).
pub fn meets(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

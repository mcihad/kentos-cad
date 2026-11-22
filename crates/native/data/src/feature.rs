//! A row as a driver hands it (docs/adr/9100 §3): its key, its geometry (read or not, NULL or not, unreadable or
//! not) and its values in the request's column order.

use crate::geom::Geometry;
use crate::value::Value;

/// A row.
#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    pub fid: i64,
    /// `None`: not read, or SQL NULL; `Some(Err)`: a geometry the driver could not read (the row is still handed).
    pub geometry: Option<Result<Geometry, String>>,
    /// The values of the request's columns, in its order.
    pub values: Vec<Value>,
}

impl Feature {
    /// Its geometry when it has a readable one.
    pub fn shape(&self) -> Option<&Geometry> {
        self.geometry.as_ref().and_then(|g| g.as_ref().ok())
    }
}

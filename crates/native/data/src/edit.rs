//! What a transaction writes (docs/adr/9100 §5): new rows, changed rows (their geometry, some of their values or
//! both) and new tables. A value is written to its field by name, so a change names only what it changes; a field
//! the source makes itself (generated, identity) is never written, and a new row's missing fields take the source's
//! defaults.

use crate::geom::Geometry;
use crate::schema::{Field, GeometryField};
use crate::value::Value;

/// A row to insert.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct NewFeature {
    /// A key of its own; none: the source's next.
    pub fid: Option<i64>,
    pub geometry: Option<Geometry>,
    /// Field values by name; a field not named takes its default.
    pub values: Vec<(String, Value)>,
}

/// A change of a row.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub fid: i64,
    /// `Some(g)`: the geometry becomes `g` (`Some(None)`: NULL); none: kept.
    pub geometry: Option<Option<Geometry>>,
    /// Field values by name; the others are kept.
    pub values: Vec<(String, Value)>,
}

/// A table to make (an export, a new layer).
#[derive(Clone, Debug, PartialEq)]
pub struct NewTable {
    /// `schema.table` in a database, the table's name in a file.
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    /// The key column's name (`fid`).
    pub key: String,
    pub fields: Vec<Field>,
    pub geometry: Option<GeometryField>,
    /// With a spatial index on its geometry.
    pub spatial_index: bool,
}

//! What a source holds, as its catalog says it without reading a row (docs/adr/9100 §3): each table's name, kind,
//! geometry column, estimated rows and extent, and whether the account may write it.

use crate::schema::GeometryField;

/// What a table is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableKind {
    /// Rows with a geometry.
    Features,
    /// Rows without one.
    Attributes,
    /// A view (rows with or without a geometry), read only unless the source says otherwise.
    View,
}

/// A table of a source.
#[derive(Clone, Debug, PartialEq)]
pub struct TableInfo {
    /// The name a request uses: `schema.table` in a database, the table's name in a file.
    pub name: String,
    /// Its title, when the source keeps one (a GeoPackage's identifier, a table's comment's first line).
    pub title: Option<String>,
    pub kind: TableKind,
    pub geometry: Option<GeometryField>,
    /// The rows the catalog estimates (statistics, not a count): may be stale after a bulk load.
    pub rows: Option<u64>,
    /// The box the catalog keeps, in the table's system.
    pub extent: Option<[f64; 4]>,
    /// Whether rows can be written: a key, and the right to change the table.
    pub writable: bool,
    pub comment: Option<String>,
}

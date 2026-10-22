//! Data sources a host provides (docs/adr/0505, plan/gis.md CBS-02, CBS-04): a database a bound layer works on.
//! The product commands stay pure (no runtime, no network): a host that can reach PostGIS (the desktop, the headless
//! host, Python, MCP) registers its provider once at start, and `cad.sources.*` ask it for a database's tables, a
//! table's rows and an edit in one transaction. A table travels as a GeoPackage table does (`kentos_geopackage`'s
//! layer, rows, values and edits), so a bound layer is compared and written the same way whatever its source.

use std::collections::HashMap;
use std::sync::OnceLock;

use kentos_geopackage as gpkg;

/// A table of a database as a provider describes it.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderTable {
    /// `schema.table`.
    pub name: String,
    /// `features` or `attributes`.
    pub data: String,
    pub rows: u64,
    /// The geometry column's type (`MULTIPOLYGON` …), none for an attribute table.
    pub geometry: Option<String>,
    pub srid: Option<u32>,
    pub columns: Vec<gpkg::Column>,
    /// Whether rows can be written: an integer primary key and the right to change the table.
    pub writable: bool,
    pub z: bool,
    pub m: bool,
}

/// A table (or a view, or a query) read whole or by an area: its definition as a GeoPackage layer (`name` is
/// `schema.table`, `fid` the key), its rows, the lasting ids KentOS wrote for some of them, its system, and the
/// revision of what was read (the SHA-256 of the rows in key order, with the area; docs/adr/0582 §1).
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderRead {
    pub layer: gpkg::Layer,
    pub rows: Vec<gpkg::Row>,
    pub uids: HashMap<i64, String>,
    pub srid: Option<u32>,
    pub writable: bool,
    pub revision: String,
}

/// Why a provider could not do what it was asked.
#[derive(Clone, Debug, PartialEq)]
pub enum ProviderError {
    /// The database cannot be reached or refuses the login.
    Unreachable(String),
    /// No such table (or not one the provider reads).
    NoTable(String),
    /// The table's content is not the revision the edit rests on: nothing was written.
    Changed,
    /// The edit failed and was rolled back.
    Refused(String),
    /// Rows the edit touches changed since the layer last matched them (their keys): nothing was written
    /// (docs/adr/0582 §2).
    RowsChanged(Vec<i64>),
    /// Rows the edit would update or delete that the account may see but not change (no privilege, or a row
    /// security policy): their keys, nothing was written (docs/adr/0585 §2).
    RowsDenied(Vec<i64>),
    /// A view or a query without an integer key column (docs/adr/0582 §3).
    NoKey(String),
    /// A view or a query whose key repeats: the key (docs/adr/0582 §3).
    KeyRepeats(String, i64),
    /// A query that cannot be read: not one statement, not a select, or refused by the database (docs/adr/0582 §3).
    Unreadable(String),
    /// A table of that name is already in the database (docs/adr/0590 §2).
    Exists(String),
}

/// How a database holds a many-to-many reference between two of its tables (docs/adr/0590 §1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DbRelation {
    /// No junction table: a text column of ids joined with `; `.
    Text,
    /// KentOS's junction table (`schema.table`).
    Mapped(String),
    /// A well-formed junction of another kind (why it is not KentOS's).
    Foreign(String),
    /// A table marked as KentOS's that does not hold to it (why).
    Broken(String),
}

/// A junction table's rows a transaction rewrites (docs/adr/0590 §3): the rows of the children `bases` are taken off,
/// `pairs` put in (in their order: a child's parents in the reference's order). A row of a child not among `bases` is
/// not touched: it belongs to a row outside a part's area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapWrite {
    pub mapping: String,
    pub bases: Vec<gpkg::relations::Key>,
    pub pairs: Vec<(gpkg::relations::Key, gpkg::relations::Key)>,
}

/// What an edit asks of the rows it touches, read again under the lock: `Err` with the keys of the rows that are not
/// as the layer last matched them (docs/adr/0582 §2).
pub type RowCheck<'a> = dyn FnMut(&gpkg::Layer, &[gpkg::Row]) -> Result<(), Vec<i64>> + 'a;

/// What an edit did: the inserted rows' keys, the area's revision before the edits (under the lock) and after.
#[derive(Clone, Debug, PartialEq)]
pub struct Edited {
    pub inserted: Vec<i64>,
    pub before: String,
    pub after: String,
}

/// One table's edits in a transaction of several (docs/adr/0584 §1): what [`TableProvider::edit`] takes for one.
pub struct TableEdit<'a, 'c> {
    pub table: &'a str,
    pub area: Option<[f64; 4]>,
    pub revision: &'a str,
    pub check: Option<&'a mut RowCheck<'c>>,
    pub edits: &'a gpkg::Edits,
}

/// A table's shape for a question (docs/adr/0581 §2): its definition (`indexed` when its geometry has a spatial
/// index), its system and the lasting ids KentOS keeps for some of its rows.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderShape {
    pub layer: gpkg::Layer,
    pub srid: Option<u32>,
    pub uids: HashMap<i64, String>,
}

/// A value a question's condition binds (docs/adr/0581 §3).
pub use kentos_expression::pushdown::Param;

/// A table read a part at a time (docs/adr/0581 §2): rows whose geometry's box meets `bbox` (in the table's system),
/// of which `filter`'s condition holds (its columns named `"name"`, its placeholders `$1` …), after the key `after`,
/// in key order; their values are `columns`' (indices into the shape's columns; none: all), their geometry read when
/// `geometry`. At most `chunk` rows a part.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProviderScan<'a> {
    pub bbox: Option<[f64; 4]>,
    pub columns: Option<&'a [usize]>,
    pub geometry: bool,
    pub filter: Option<(&'a str, &'a [Param])>,
    pub after: Option<i64>,
    pub chunk: usize,
}

/// A provider of database tables.
pub trait TableProvider: Send + Sync {
    fn tables(&self, address: &str) -> Result<Vec<ProviderTable>, ProviderError>;
    /// The rows, all of them or those whose box meets `area` (in the table's system).
    fn read(
        &self,
        address: &str,
        table: &str,
        area: Option<[f64; 4]>,
    ) -> Result<ProviderRead, ProviderError>;
    /// The edits in one transaction under a lock on the table. With `check`, the rows the edits update or delete are
    /// read again and checked (docs/adr/0582 §2), and the area must also still be at `revision` unless it is empty;
    /// without, the area must still be at `revision`. A row the account cannot change refuses the edit
    /// ([`ProviderError::RowsDenied`]).
    fn edit(
        &self,
        address: &str,
        table: &str,
        area: Option<[f64; 4]>,
        revision: &str,
        check: Option<&mut RowCheck<'_>>,
        edits: &gpkg::Edits,
    ) -> Result<Edited, ProviderError>;
    /// Several tables of one database in one transaction (docs/adr/0584 §1): the tables locked in their names' order,
    /// each checked and written as [`TableProvider::edit`] does; one refusal and nothing is written. The error names
    /// the table (its index in `items`) that refused. A provider without transactions over tables writes one alone.
    fn edit_many(
        &self,
        address: &str,
        items: Vec<TableEdit<'_, '_>>,
    ) -> Result<Vec<Edited>, (usize, ProviderError)> {
        let mut items = items;
        match items.len() {
            1 => {
                let Some(t) = items.pop() else {
                    return Ok(Vec::new());
                };
                self.edit(address, t.table, t.area, t.revision, t.check, t.edits)
                    .map(|e| vec![e])
                    .map_err(|e| (0, e))
            }
            0 => Ok(Vec::new()),
            _ => Err((
                1,
                ProviderError::Refused(
                    "bu sağlayıcı birden çok tabloyu bir işlemde yazamaz".into(),
                ),
            )),
        }
    }
    /// Several tables in one transaction and the junction tables of their many-to-many references with them (docs/adr/0590
    /// §3): a refusal of a junction table is said at `items.len()` and after. A provider without junction tables writes
    /// the tables alone, and refuses junction rows.
    fn edit_many_mapped(
        &self,
        address: &str,
        items: Vec<TableEdit<'_, '_>>,
        maps: &[MapWrite],
    ) -> Result<Vec<Edited>, (usize, ProviderError)> {
        if maps.is_empty() {
            return self.edit_many(address, items);
        }
        Err((
            items.len(),
            ProviderError::Refused("bu sağlayıcı bağlantı tablosu yazamaz".into()),
        ))
    }
    /// How the database holds a many-to-many reference from `child`'s rows to `parent`'s (docs/adr/0590 §1).
    fn relation(
        &self,
        _address: &str,
        _child: &str,
        _parent: &str,
    ) -> Result<DbRelation, ProviderError> {
        Ok(DbRelation::Text)
    }
    /// A junction table's rows, child key and parent key, by the child's key and the reference's order.
    fn mapping_rows(
        &self,
        _address: &str,
        mapping: &str,
    ) -> Result<Vec<(i64, i64)>, ProviderError> {
        Err(ProviderError::NoTable(mapping.to_owned()))
    }
    /// Makes KentOS's junction table `mapping` between `child` and `parent` with `pairs`, in one transaction.
    fn map(
        &self,
        _address: &str,
        _child: &str,
        _parent: &str,
        mapping: &str,
        _pairs: &[(i64, i64)],
    ) -> Result<(), ProviderError> {
        Err(ProviderError::Refused(format!(
            "{mapping}: bu sağlayıcı bağlantı tablosu kuramaz"
        )))
    }
    /// Takes KentOS's junction table `mapping` away.
    fn unmap(&self, _address: &str, mapping: &str) -> Result<(), ProviderError> {
        Err(ProviderError::Refused(format!(
            "{mapping}: bu sağlayıcı bağlantı tablosu kaldıramaz"
        )))
    }
    /// The table's shape for a question.
    fn describe(&self, _address: &str, table: &str) -> Result<ProviderShape, ProviderError> {
        Err(ProviderError::Refused(format!(
            "{table}: bu sağlayıcı sorgulanamaz"
        )))
    }
    /// The rows `s` asks for, handed to `f` a part at a time in key order; `f` returns false to stop. Returns how many
    /// rows the database handed over.
    fn scan(
        &self,
        _address: &str,
        table: &str,
        _s: &ProviderScan<'_>,
        _f: &mut dyn FnMut(Vec<gpkg::Row>) -> bool,
    ) -> Result<u64, ProviderError> {
        Err(ProviderError::Refused(format!(
            "{table}: bu sağlayıcı sorgulanamaz"
        )))
    }
}

static POSTGIS: OnceLock<Box<dyn TableProvider>> = OnceLock::new();

/// Registers the host's PostGIS provider; the first registration stays.
pub fn register_postgis(provider: Box<dyn TableProvider>) {
    let _ = POSTGIS.set(provider);
}

/// The host's PostGIS provider, none where the host registered none.
pub fn postgis() -> Option<&'static dyn TableProvider> {
    POSTGIS.get().map(|b| b.as_ref())
}

pub use kentos_commands::{address_problem, is_database};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_keep_no_password() {
        assert!(is_database("postgresql://ali@db/kent"));
        assert!(!is_database("/veri/kadastro.gpkg"));
        assert_eq!(
            address_problem("postgresql://ali@db:5432/kent?sslmode=require"),
            None
        );
        assert!(address_problem("postgresql://ali:gizli@db/kent").is_some());
        assert!(address_problem("postgresql://db/kent?password=gizli").is_some());
        assert!(address_problem("postgresql://ali@/kent").is_some());
        assert!(address_problem("postgresql://ali@db/").is_some());
    }
}

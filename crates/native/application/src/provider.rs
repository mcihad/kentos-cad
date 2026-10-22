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

/// A table read whole: its definition as a GeoPackage layer (`name` is `schema.table`, `fid` the primary key), its
/// rows, the lasting ids KentOS wrote for some of them, its system, and the revision of its content (the SHA-256 of
/// its rows in key order).
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
    fn read(&self, address: &str, table: &str) -> Result<ProviderRead, ProviderError>;
    /// The edits in one transaction, after checking under a lock that the table is still at `revision`: the
    /// inserted rows' keys and the table's revision after.
    fn edit(
        &self,
        address: &str,
        table: &str,
        revision: &str,
        edits: &gpkg::Edits,
    ) -> Result<(Vec<i64>, String), ProviderError>;
    /// The table's shape for a question.
    fn describe(&self, _address: &str, table: &str) -> Result<ProviderShape, ProviderError> {
        Err(ProviderError::Refused(format!("{table}: bu sağlayıcı sorgulanamaz")))
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
        Err(ProviderError::Refused(format!("{table}: bu sağlayıcı sorgulanamaz")))
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

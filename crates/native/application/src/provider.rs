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

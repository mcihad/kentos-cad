//! The traits a driver implements (docs/adr/9100 §2): [`Driver`] opens a [`Source`] from an address; a source answers
//! for its tables and hands their rows as a stream; a [`Transaction`] writes rows and tables together or not at all.
//! The methods are synchronous: a driver over an asynchronous client keeps its runtime inside, so the callers (the
//! drawing's threads, a table's page reader, a command) stay as they are. Every method may be called from any thread.

use std::sync::Arc;

use crate::cancel::Cancel;
use crate::catalog::TableInfo;
use crate::condition::Condition;
use crate::edit::{Change, NewFeature, NewTable};
use crate::error::DataError;
use crate::feature::Feature;
use crate::projects::ProjectStore;
use crate::request::Request;
use crate::schema::{Field, Schema};
use crate::value::Value;

/// What a table allows, as the driver and the account's rights decide it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub read: bool,
    pub insert: bool,
    pub update_values: bool,
    pub update_geometry: bool,
    pub delete: bool,
    /// Several tables of the source in one transaction.
    pub transactions: bool,
    /// The box answered by a spatial index (not by reading every row).
    pub spatial_index: bool,
    /// The geometry simplified by the source itself.
    pub simplify_at_source: bool,
    /// The condition and the order answered by the source's SQL.
    pub query_at_source: bool,
    /// Arcs and compound curves kept.
    pub curves: bool,
    pub create_table: bool,
    /// Nullable columns added to the table (docs/adr/9318): a GeoPackage file open for writing; a database table the
    /// account owns. Not a query's or a view's.
    pub add_columns: bool,
    /// The columns' rules written into the source (docs/adr/9324): as `add_columns`.
    pub set_rules: bool,
}

impl Capabilities {
    /// Whether anything can be written.
    pub fn writable(&self) -> bool {
        self.insert || self.update_values || self.update_geometry || self.delete
    }
}

/// What a read did: rows handed, and whether it stopped before the end (asked to, or the sink said so).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Read {
    pub rows: u64,
    pub stopped: bool,
}

/// A kind of source.
pub trait Driver: Send + Sync {
    /// Its id (`gpkg`, `postgis`).
    fn id(&self) -> &'static str;
    /// Its name in the interface (`GeoPackage`, `PostgreSQL/PostGIS`).
    fn title(&self) -> &'static str;
    /// Whether the address is one of its sources.
    fn accepts(&self, address: &str) -> bool;
    /// Opens the source (no row read; a database's first connection made).
    fn open(&self, address: &str) -> Result<Arc<dyn Source>, DataError>;
}

/// An opened file or database.
pub trait Source: Send + Sync {
    /// The driver's id.
    fn driver(&self) -> &'static str;
    /// The address it was opened from.
    fn address(&self) -> &str;
    /// Its tables, as its catalog says them (no row read).
    fn tables(&self) -> Result<Vec<TableInfo>, DataError>;
    /// A table's shape.
    fn schema(&self, table: &str) -> Result<Schema, DataError>;
    /// The table's columns' rules as the source keeps them (docs/adr/9324); none for a source that keeps none.
    fn rules(&self, table: &str) -> Result<Vec<crate::schema::RuledColumn>, DataError> {
        let _ = table;
        Ok(Vec::new())
    }
    /// What the table allows.
    fn capabilities(&self, table: &str) -> Result<Capabilities, DataError>;
    /// The table's box in its system: the catalog's when `estimate` (fast, may be stale), else measured.
    fn extent(&self, table: &str, estimate: bool) -> Result<Option<[f64; 4]>, DataError>;
    /// How many rows the request keeps (its box and condition; its page and order are not asked). `estimate` without
    /// a box and a condition answers from the catalog's statistics.
    fn count(&self, table: &str, request: &Request, estimate: bool) -> Result<u64, DataError>;
    /// A mark of the table's rows that moves when anyone writes them (docs/adr/9182 §2): a database's own counters of
    /// the rows added, changed and deleted. None for a source that keeps none: a file says its changes by its size and
    /// last write, which the caller reads. A mark may follow a write by the source's own delay.
    fn change_mark(&self, table: &str) -> Result<Option<String>, DataError> {
        let _ = table;
        Ok(None)
    }
    /// The rows the request asks for, in its order, handed `request.chunk` at a time to `sink` until the end, until
    /// `sink` returns false, or until `cancel` is flipped (checked between parts).
    fn features(
        &self,
        table: &str,
        request: &Request,
        cancel: &Cancel,
        sink: &mut dyn FnMut(Vec<Feature>) -> bool,
    ) -> Result<Read, DataError>;
    /// The distinct values of a field among the rows the condition keeps, ordered, at most `limit`.
    fn unique(
        &self,
        table: &str,
        field: &str,
        condition: Option<&Condition>,
        limit: usize,
    ) -> Result<Vec<Value>, DataError>;
    /// The rows per value of a field among the rows the condition keeps (`GROUP BY`, docs/adr/9108 §2): the empty value as
    /// [`Value::Null`], in the field's order (a text in the Turkish alphabet's, docs/adr/9102 §3) with the empty value
    /// last, at most `limit` groups. A driver without its own reads the field's values and counts them here.
    fn groups(
        &self,
        table: &str,
        field: &str,
        condition: Option<&Condition>,
        limit: usize,
    ) -> Result<Vec<(Value, u64)>, DataError> {
        let request = Request {
            condition: condition.cloned(),
            columns: Some(vec![field.to_owned()]),
            geometry: false,
            ..Request::all()
        };
        let mut counted: Vec<(Value, u64)> = Vec::new();
        self.features(table, &request, &Cancel::new(), &mut |part| {
            for f in part {
                let v = f.values.into_iter().next().unwrap_or(Value::Null);
                match counted.iter_mut().find(|(w, _)| *w == v) {
                    Some((_, n)) => *n += 1,
                    None => counted.push((v, 1)),
                }
            }
            true
        })?;
        counted.sort_by(|(a, _), (b, _)| group_order(a, b));
        counted.truncate(limit);
        Ok(counted)
    }
    /// The columns a query reads (docs/adr/9113 §1), each with whether it can be a layer's key (an integer) or its
    /// geometry; the query is not run past its first row. A driver that reads no queries refuses.
    fn query_columns(&self, sql: &str) -> Result<Vec<crate::query::QueryColumn>, DataError> {
        let _ = sql;
        Err(DataError::Unsupported(
            "Bu kaynak sorgu katmanı okumaz.".to_owned(),
        ))
    }
    /// Starts a transaction.
    fn begin(&self) -> Result<Box<dyn Transaction>, DataError>;
    /// Whether the source keeps write receipts in `scope` (a database's schema; a file has one place, ""): a table
    /// a write's receipt goes into with its rows, in the same transaction (docs/adr/9126). None is set up silently: the
    /// owner sets them up ([`Source::set_up_receipts`]).
    fn receipts(&self, scope: &str) -> Result<bool, DataError> {
        let _ = scope;
        Ok(false)
    }
    /// Makes the write receipts' table in `scope`, if it is not there; the source's other tables stay as they are.
    fn set_up_receipts(&self, scope: &str) -> Result<(), DataError> {
        let _ = scope;
        Err(DataError::Unsupported(
            "Bu kaynak işlem kaydı tutmaz.".to_owned(),
        ))
    }
    /// The receipt of the write `id` in `scope`, read outside any transaction; none when no such write committed.
    fn receipt(&self, scope: &str, id: &str) -> Result<Option<Receipt>, DataError> {
        let _ = (scope, id);
        Err(DataError::Unsupported(
            "Bu kaynak işlem kaydı tutmaz.".to_owned(),
        ))
    }
    /// Where it keeps projects with its data (docs/adr/9107); none for a source that cannot keep them.
    fn project_store(&self) -> Option<&dyn ProjectStore> {
        None
    }
}

/// The order of a field's groups: the empty value last, numbers by their size, texts in the Turkish alphabet's order.
pub fn group_order(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Null, _) => Ordering::Greater,
        (_, Value::Null) => Ordering::Less,
        _ => match (a.number(), b.number()) {
            (Some(x), Some(y)) => x.total_cmp(&y),
            _ => crate::turkish::compare(&a.to_string(), &b.to_string()),
        },
    }
}

/// Writes over a source's tables, all or nothing.
pub trait Transaction: Send {
    /// Inserts rows; their keys, in order.
    fn insert(&mut self, table: &str, rows: &[NewFeature]) -> Result<Vec<i64>, DataError>;
    /// Changes rows; a key not in the table is refused (nothing of the transaction is written).
    fn update(&mut self, table: &str, changes: &[Change]) -> Result<(), DataError>;
    /// Deletes rows; a key not in the table is refused.
    fn delete(&mut self, table: &str, fids: &[i64]) -> Result<(), DataError>;
    /// The rows of these keys as they are inside the transaction, every field (in the schema's order) and the geometry
    /// whole, by key; held against other writers until the transaction ends where the source can hold rows (a database
    /// locks them; a file's transaction holds the whole file). A key not in the table is left out. What a writer checks
    /// its rows against before it changes them (docs/adr/9104 §3).
    fn rows(&mut self, table: &str, fids: &[i64]) -> Result<Vec<Feature>, DataError>;
    /// Makes a table; one of its name is refused unless `replace`, which drops it first.
    fn create_table(&mut self, table: &NewTable, replace: bool) -> Result<(), DataError>;
    /// Drops a table.
    fn drop_table(&mut self, table: &str) -> Result<(), DataError>;
    /// Adds nullable columns to a table, in order (docs/adr/9318): each row is empty (NULL) in them. A source that cannot
    /// says so; nothing of the transaction is written then.
    fn add_columns(&mut self, table: &str, fields: &[Field]) -> Result<(), DataError> {
        let _ = (table, fields);
        Err(DataError::Unsupported(
            "Bu kaynağın tablosuna sütun eklenemez.".to_owned(),
        ))
    }
    /// Writes the columns' rules into the source (docs/adr/9324), each replacing the column's rule there; a column
    /// given none loses its rule. Rows already there are not checked against them. A source that cannot says so;
    /// nothing of the transaction is written then.
    fn set_rules(
        &mut self,
        table: &str,
        columns: &[crate::schema::RuledColumn],
    ) -> Result<(), DataError> {
        let _ = (table, columns);
        Err(DataError::Unsupported(
            "Bu kaynağın tablosuna kural yazılamaz.".to_owned(),
        ))
    }
    /// Writes a write's receipt into `scope`'s receipts table, inside the transaction: it commits with the rows or not
    /// at all (docs/adr/9126). A receipt of the same id already there is refused (its primary key).
    fn receipt(&mut self, scope: &str, receipt: &Receipt) -> Result<(), DataError> {
        let _ = (scope, receipt);
        Err(DataError::Unsupported(
            "Bu kaynak işlem kaydı tutmaz.".to_owned(),
        ))
    }
    /// Writes it all.
    fn commit(self: Box<Self>) -> Result<(), DataError>;
    /// Writes nothing.
    fn rollback(self: Box<Self>) -> Result<(), DataError>;
}

/// A write's receipt (docs/adr/9126): its id, who wrote it, and what it wrote as JSON (`{"table": {"added": [[uid,
/// key], …], "changed": n, "deleted": n}}`): the keys the source gave the new rows, by the objects' ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub id: String,
    pub by: Option<String>,
    pub written: String,
}

/// The name of the write receipts' table in a scope.
pub const RECEIPTS_TABLE: &str = "kentos_writes";

/// Every row of a request gathered (for tests and small tables; a layer streams instead).
pub fn collect(
    source: &dyn Source,
    table: &str,
    request: &Request,
) -> Result<Vec<Feature>, DataError> {
    let mut out = Vec::new();
    source.features(table, request, &Cancel::new(), &mut |part| {
        out.extend(part);
        true
    })?;
    Ok(out)
}

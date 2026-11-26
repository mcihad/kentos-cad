//! A source whose commit's answer is lost on demand (docs/adr/9126), for tests of uncertain writes: it wraps another
//! driver's source (its address [`address_of`] the wrapped one: `kayip:` before it, `#kayip` after it, so the wrapped
//! driver does not take it for its own) and reads and writes through it, but a commit asked to
//! lose its answer commits first ([`Loss::AfterCommit`]) or writes nothing ([`Loss::BeforeCommit`]) and then answers
//! [`DataError::Uncertain`], as a database whose connection ended around its COMMIT does. Each wrapped address has its
//! own loss, used once, so tests running together do not meet.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::{
    Cancel, Capabilities, Change, Condition, DataError, Driver, Feature, NewFeature, NewTable, Read, Receipt, Request,
    Schema, Source, TableInfo, Transaction, Value,
};

/// The address prefix of a lossy source.
pub const PREFIX: &str = "kayip:";
/// The address suffix of a lossy source.
pub const SUFFIX: &str = "#kayip";

/// The lossy source's address over the source at `wrapped`.
pub fn address_of(wrapped: &str) -> String {
    format!("{PREFIX}{wrapped}{SUFFIX}")
}

/// What the next commit of a wrapped source does with its answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loss {
    /// It commits, then its answer is lost: the rows are there.
    AfterCommit,
    /// The connection ends before the commit reaches the source: nothing is written.
    BeforeCommit,
}

fn losses() -> &'static Mutex<HashMap<String, Loss>> {
    static LOSSES: OnceLock<Mutex<HashMap<String, Loss>>> = OnceLock::new();
    LOSSES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The next commit of the source wrapped at `address` (without the prefix) loses its answer so.
pub fn lose(address: &str, loss: Loss) {
    if let Ok(mut m) = losses().lock() {
        m.insert(address.to_owned(), loss);
    }
}

fn take(address: &str) -> Option<Loss> {
    losses().lock().ok().and_then(|mut m| m.remove(address))
}

/// The driver: registered by the tests that use it.
pub struct Lossy;

/// Registers the driver in the process's registry.
pub fn register() {
    crate::registry().register(Arc::new(Lossy));
}

impl Driver for Lossy {
    fn id(&self) -> &'static str {
        "kayip"
    }

    fn title(&self) -> &'static str {
        "Yanıtı kaybolan sınama kaynağı"
    }

    fn accepts(&self, address: &str) -> bool {
        address.starts_with(PREFIX) && address.ends_with(SUFFIX)
    }

    fn open(&self, address: &str) -> Result<Arc<dyn Source>, DataError> {
        let wrapped = address.trim_start_matches(PREFIX).trim_end_matches(SUFFIX).to_owned();
        let inner = crate::registry().source(&wrapped)?;
        Ok(Arc::new(LossySource { address: address.to_owned(), wrapped, inner }))
    }
}

struct LossySource {
    address: String,
    wrapped: String,
    inner: Arc<dyn Source>,
}

impl Source for LossySource {
    /// The wrapped source's driver: a data layer over it is that driver's (only its commits' answers are lost).
    fn driver(&self) -> &'static str {
        self.inner.driver()
    }
    fn address(&self) -> &str {
        &self.address
    }
    fn tables(&self) -> Result<Vec<TableInfo>, DataError> {
        self.inner.tables()
    }
    fn schema(&self, table: &str) -> Result<Schema, DataError> {
        self.inner.schema(table)
    }

    fn rules(&self, table: &str) -> Result<Vec<crate::schema::RuledColumn>, DataError> {
        self.inner.rules(table)
    }
    fn capabilities(&self, table: &str) -> Result<Capabilities, DataError> {
        self.inner.capabilities(table)
    }
    fn extent(&self, table: &str, estimate: bool) -> Result<Option<[f64; 4]>, DataError> {
        self.inner.extent(table, estimate)
    }
    fn count(&self, table: &str, request: &Request, estimate: bool) -> Result<u64, DataError> {
        self.inner.count(table, request, estimate)
    }
    fn features(
        &self,
        table: &str,
        request: &Request,
        cancel: &Cancel,
        sink: &mut dyn FnMut(Vec<Feature>) -> bool,
    ) -> Result<Read, DataError> {
        self.inner.features(table, request, cancel, sink)
    }
    fn unique(&self, table: &str, field: &str, condition: Option<&Condition>, limit: usize) -> Result<Vec<Value>, DataError> {
        self.inner.unique(table, field, condition, limit)
    }
    fn groups(
        &self,
        table: &str,
        field: &str,
        condition: Option<&Condition>,
        limit: usize,
    ) -> Result<Vec<(Value, u64)>, DataError> {
        self.inner.groups(table, field, condition, limit)
    }
    fn begin(&self) -> Result<Box<dyn Transaction>, DataError> {
        Ok(Box::new(LossyTransaction { wrapped: self.wrapped.clone(), inner: self.inner.begin()? }))
    }
    fn receipts(&self, scope: &str) -> Result<bool, DataError> {
        self.inner.receipts(scope)
    }
    fn set_up_receipts(&self, scope: &str) -> Result<(), DataError> {
        self.inner.set_up_receipts(scope)
    }
    fn receipt(&self, scope: &str, id: &str) -> Result<Option<Receipt>, DataError> {
        self.inner.receipt(scope, id)
    }
}

struct LossyTransaction {
    wrapped: String,
    inner: Box<dyn Transaction>,
}

impl Transaction for LossyTransaction {
    fn insert(&mut self, table: &str, rows: &[NewFeature]) -> Result<Vec<i64>, DataError> {
        self.inner.insert(table, rows)
    }
    fn update(&mut self, table: &str, changes: &[Change]) -> Result<(), DataError> {
        self.inner.update(table, changes)
    }
    fn delete(&mut self, table: &str, fids: &[i64]) -> Result<(), DataError> {
        self.inner.delete(table, fids)
    }
    fn rows(&mut self, table: &str, fids: &[i64]) -> Result<Vec<Feature>, DataError> {
        self.inner.rows(table, fids)
    }
    fn create_table(&mut self, table: &NewTable, replace: bool) -> Result<(), DataError> {
        self.inner.create_table(table, replace)
    }
    fn drop_table(&mut self, table: &str) -> Result<(), DataError> {
        self.inner.drop_table(table)
    }
    fn add_columns(&mut self, table: &str, fields: &[crate::schema::Field]) -> Result<(), DataError> {
        self.inner.add_columns(table, fields)
    }

    fn set_rules(&mut self, table: &str, columns: &[crate::schema::RuledColumn]) -> Result<(), DataError> {
        self.inner.set_rules(table, columns)
    }
    fn receipt(&mut self, scope: &str, receipt: &Receipt) -> Result<(), DataError> {
        self.inner.receipt(scope, receipt)
    }
    fn commit(self: Box<Self>) -> Result<(), DataError> {
        let lost = "bağlantı COMMIT'in yanıtı gelmeden kapandı (sınama)";
        match take(&self.wrapped) {
            Some(Loss::AfterCommit) => {
                self.inner.commit()?;
                Err(DataError::Uncertain(lost.into()))
            }
            Some(Loss::BeforeCommit) => {
                self.inner.rollback()?;
                Err(DataError::Uncertain(lost.into()))
            }
            None => self.inner.commit(),
        }
    }
    fn rollback(self: Box<Self>) -> Result<(), DataError> {
        self.inner.rollback()
    }
}

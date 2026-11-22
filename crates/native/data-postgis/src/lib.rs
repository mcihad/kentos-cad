//! The PostGIS data driver (docs/adr/9100 §6, plan/veri.md VY-01): a PostgreSQL database with PostGIS as a
//! [`kentos_data::Source`]. Adding a layer reads the catalog only; a view's rows are read through a server-side cursor
//! (`DECLARE … NO SCROLL CURSOR`, `FETCH FORWARD n`) in a read-only transaction, so the first part is drawn while the
//! rest is still on its way and a request the view left behind stops at once (`CLOSE`). The box goes to the GiST
//! index (`&&`), the condition and the order to the SQL, the simplification to the server (`ST_Simplify` with
//! collapsed parts kept), so a zoomed-out million-row table sends its outlines, not its every vertex.
//!
//! One pool of at most [`connect::MOST`] connections per database (address), shared by every layer, table page and
//! window: no request opens its own connection. Passwords come from the host's [`kentos_data::Secrets`], then from
//! `PGPASSWORD` and `~/.pgpass`; an address never holds one.

mod catalog;
mod connect;
mod projects;
mod query;
mod read;
mod sql;
mod write;

use std::sync::{Arc, OnceLock};

use kentos_data::{DataError, Driver, Source};

pub use connect::PgSource;

/// The driver's runtime: a small one of its own, its workers named for it.
pub(crate) fn runtime() -> Result<&'static tokio::runtime::Runtime, DataError> {
    static RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("kentos-postgis")
                .enable_all()
                .build()
                .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| DataError::Failed(format!("PostGIS sürücüsü başlatılamadı: {e}")))
}

/// The driver.
pub struct PostGis;

impl Driver for PostGis {
    fn id(&self) -> &'static str {
        "postgis"
    }

    fn title(&self) -> &'static str {
        "PostgreSQL/PostGIS"
    }

    fn accepts(&self, address: &str) -> bool {
        address.starts_with("postgresql://") || address.starts_with("postgres://")
    }

    fn open(&self, address: &str) -> Result<Arc<dyn Source>, DataError> {
        Ok(Arc::new(PgSource::open(address)?))
    }
}

/// Registers the driver in the process's registry.
pub fn register() {
    kentos_data::registry().register(Arc::new(PostGis));
}

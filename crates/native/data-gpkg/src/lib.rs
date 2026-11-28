//! The GeoPackage data driver (docs/adr/9100 §5, plan/veri.md VY-01): a `.gpkg` file as a [`kentos_data::Source`].
//! A request is one SQL statement over the file: the box through the table's R*Tree index (`rtree_<table>_<column>`;
//! without one, through the geometry's own box), the condition in SQLite's words (Turkish letters folded by a
//! function of ours, `kentos_fold`), the order with NULLs last and the key after it, the page as `LIMIT`/`OFFSET`. The
//! rows come a part at a time from the open statement; nothing else is read. A geometry is simplified here, after
//! it is read: SQLite has no function for it.
//!
//! Reading connections are kept (a few, read only) and shared by every thread; a transaction opens its own writing
//! connection (`BEGIN IMMEDIATE`, so a second writer waits instead of failing half way) and keeps the spatial index by
//! the file's triggers and the table's box in `gpkg_contents` when it commits.

mod convert;
mod projects;
mod query;
mod receipts;
mod source;
mod sql;
mod statement;
mod write;

use std::path::Path;
use std::sync::Arc;

use kentos_data::{DataError, Driver, Reach, Source};

pub use source::GpkgSource;

/// The driver.
pub struct Gpkg;

/// The file an address names: `gpkg:` taken off, a path kept as it is.
pub fn path_of(address: &str) -> &str {
    address.strip_prefix("gpkg:").unwrap_or(address)
}

impl Driver for Gpkg {
    fn id(&self) -> &'static str {
        "gpkg"
    }

    fn title(&self) -> &'static str {
        "GeoPackage"
    }

    fn accepts(&self, address: &str) -> bool {
        address.starts_with("gpkg:") || path_of(address).to_ascii_lowercase().ends_with(".gpkg")
    }

    fn open(&self, address: &str) -> Result<Arc<dyn Source>, DataError> {
        let path = Path::new(path_of(address));
        if !path.exists() {
            return Err(DataError::Unreachable {
                reach: Reach::Offline,
                message: format!(
                    "“{}” bulunamadı: dosya taşınmış ya da silinmiş olabilir; yerini gösterin.",
                    path.display()
                ),
            });
        }
        Ok(Arc::new(GpkgSource::open(address, path)?))
    }
}

/// Registers the driver in the process's registry.
pub fn register() {
    kentos_data::registry().register(Arc::new(Gpkg));
}

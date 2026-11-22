//! The drivers a host registered and the sources opened through them (docs/adr/9100 §2): one source per address, so
//! a database's connection pool is shared by every layer, table and window reading it, and closed with the program.
//! A host registers its drivers once at start (`kentos_data_gpkg::register()`, `kentos_data_postgis::register()`);
//! a new database is one more registration.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use crate::driver::{Driver, Source};
use crate::error::DataError;

/// Drivers and opened sources.
#[derive(Default)]
pub struct Registry {
    drivers: RwLock<Vec<Arc<dyn Driver>>>,
    sources: RwLock<HashMap<String, Arc<dyn Source>>>,
}

impl Registry {
    /// Adds a driver; one of the same id is replaced.
    pub fn register(&self, driver: Arc<dyn Driver>) {
        if let Ok(mut list) = self.drivers.write() {
            list.retain(|d| d.id() != driver.id());
            list.push(driver);
        }
    }

    /// The registered drivers' ids and titles.
    pub fn drivers(&self) -> Vec<(&'static str, &'static str)> {
        self.drivers
            .read()
            .map(|l| l.iter().map(|d| (d.id(), d.title())).collect())
            .unwrap_or_default()
    }

    /// The driver that takes an address.
    pub fn driver_for(&self, address: &str) -> Option<Arc<dyn Driver>> {
        self.drivers
            .read()
            .ok()?
            .iter()
            .find(|d| d.accepts(address))
            .cloned()
    }

    /// The source at an address, opened once and kept.
    pub fn source(&self, address: &str) -> Result<Arc<dyn Source>, DataError> {
        if let Some(s) = self
            .sources
            .read()
            .ok()
            .and_then(|m| m.get(address).cloned())
        {
            return Ok(s);
        }
        let driver = self.driver_for(address).ok_or_else(|| {
            DataError::Unsupported(format!(
                "“{address}” adresini okuyabilen bir sürücü yok (GeoPackage dosyası ya da postgresql:// adresi bekleniyordu)"
            ))
        })?;
        let opened = driver.open(address)?;
        if let Ok(mut m) = self.sources.write() {
            // Another thread may have opened it meanwhile: the first one stays.
            return Ok(m.entry(address.to_owned()).or_insert(opened).clone());
        }
        Ok(opened)
    }

    /// Forgets the source at an address (a file moved, a connection's settings changed): the next ask opens it anew.
    pub fn forget(&self, address: &str) {
        if let Ok(mut m) = self.sources.write() {
            m.remove(address);
        }
    }
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// The process's registry.
pub fn registry() -> &'static Registry {
    REGISTRY.get_or_init(Registry::default)
}

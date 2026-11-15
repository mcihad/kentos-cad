//! The catalog's store in memory, the tests' (docs/adr/8059 §10). A test grants the roles, as the catalog's
//! administrator will on a running server.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard, PoisonError};

use axum::body::Bytes;
use kentos_extensions::release::{Release, Role};

use super::store::{Access, ReleaseStore, StoreError, StoreFuture};
use super::{CatalogId, ReleaseKey};

/// Which catalogs a role is granted in.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Where {
    /// Every catalog: the catalog's own services.
    Every,
    In(CatalogId),
}

#[derive(Default)]
pub struct MemoryStore {
    inner: Mutex<Memory>,
}

#[derive(Default)]
struct Memory {
    releases: BTreeMap<ReleaseKey, Release>,
    packages: BTreeMap<String, Bytes>,
    roles: BTreeMap<(Where, String), BTreeSet<Role>>,
    members: BTreeSet<(CatalogId, String)>,
    publishers: BTreeMap<String, BTreeSet<String>>,
    trusted: BTreeSet<(CatalogId, String)>,
}

impl MemoryStore {
    fn lock(&self) -> MutexGuard<'_, Memory> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Grants an account a role, in one catalog or in every one.
    pub fn grant(&self, place: Where, account: &str, role: Role) {
        self.lock()
            .roles
            .entry((place, account.to_owned()))
            .or_default()
            .insert(role);
    }

    /// Lets an account into an organisation's catalog without a role: it sees what is published there.
    pub fn admit(&self, catalog: CatalogId, account: &str) {
        self.lock().members.insert((catalog, account.to_owned()));
    }

    /// Makes an account a member of a publisher.
    pub fn join(&self, account: &str, publisher: &str) {
        self.lock()
            .publishers
            .entry(account.to_owned())
            .or_default()
            .insert(publisher.to_owned());
    }

    /// Lets a catalog trust a publisher.
    pub fn trust(&self, catalog: CatalogId, publisher: &str) {
        self.lock().trusted.insert((catalog, publisher.to_owned()));
    }

    /// The stored record, as a test reads it behind the API's back.
    pub fn stored(&self, key: &ReleaseKey) -> Option<Release> {
        self.lock().releases.get(key).cloned()
    }

    /// How many packages are kept.
    pub fn packages(&self) -> usize {
        self.lock().packages.len()
    }

    fn written(
        &self,
        key: &ReleaseKey,
        moves: Option<usize>,
        release: &Release,
    ) -> Result<(), StoreError> {
        let mut memory = self.lock();
        match (moves, memory.releases.get(key)) {
            (None, Some(_)) => return Err(StoreError::Exists),
            (None, None) => {}
            (Some(n), Some(old)) if old.audit().len() == n => {
                if !release.audit().starts_with(old.audit()) {
                    return Err(StoreError::Failed(format!(
                        "{} {}: yazılan kayıt eski hareketleri değiştiriyor",
                        key.extension, key.version
                    )));
                }
            }
            (Some(_), _) => return Err(StoreError::Changed),
        }
        memory.releases.insert(key.clone(), release.clone());
        Ok(())
    }
}

impl ReleaseStore for MemoryStore {
    fn releases<'a>(&'a self, catalog: &'a CatalogId) -> StoreFuture<'a, Vec<Release>> {
        let list: Vec<Release> = self
            .lock()
            .releases
            .iter()
            .filter(|(key, _)| key.catalog == *catalog)
            .map(|(_, release)| release.clone())
            .collect();
        Box::pin(async move { Ok(list) })
    }

    fn release<'a>(&'a self, key: &'a ReleaseKey) -> StoreFuture<'a, Option<Release>> {
        let found = self.lock().releases.get(key).cloned();
        Box::pin(async move { Ok(found) })
    }

    fn write<'a>(
        &'a self,
        key: &'a ReleaseKey,
        moves: Option<usize>,
        release: &'a Release,
    ) -> StoreFuture<'a, ()> {
        let result = self.written(key, moves, release);
        Box::pin(async move { result })
    }

    fn put_package<'a>(&'a self, digest: &'a str, bytes: Bytes) -> StoreFuture<'a, ()> {
        self.lock()
            .packages
            .entry(digest.to_owned())
            .or_insert(bytes);
        Box::pin(async move { Ok(()) })
    }

    fn package<'a>(&'a self, digest: &'a str) -> StoreFuture<'a, Option<Bytes>> {
        let found = self.lock().packages.get(digest).cloned();
        Box::pin(async move { Ok(found) })
    }

    fn access<'a>(&'a self, catalog: &'a CatalogId, account: &'a str) -> StoreFuture<'a, Access> {
        let memory = self.lock();
        let mut roles = BTreeSet::new();
        for place in [Where::Every, Where::In(catalog.clone())] {
            if let Some(granted) = memory.roles.get(&(place, account.to_owned())) {
                roles.extend(granted.iter().copied());
            }
        }
        let service = memory
            .roles
            .contains_key(&(Where::Every, account.to_owned()));
        let member = match catalog {
            CatalogId::Public => true,
            CatalogId::Tenant(_) => {
                service
                    || !roles.is_empty()
                    || memory
                        .members
                        .contains(&(catalog.clone(), account.to_owned()))
            }
            CatalogId::Personal(_) => service,
        };
        let access = Access {
            member,
            roles,
            publishers: memory.publishers.get(account).cloned().unwrap_or_default(),
        };
        Box::pin(async move { Ok(access) })
    }

    fn trusted<'a>(&'a self, catalog: &'a CatalogId, publisher: &'a str) -> StoreFuture<'a, bool> {
        let trusted = self
            .lock()
            .trusted
            .contains(&(catalog.clone(), publisher.to_owned()));
        Box::pin(async move { Ok(trusted) })
    }
}

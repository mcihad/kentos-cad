//! Where the extension catalog keeps its releases, their packages and its grants: a small trait, so the flow does not
//! depend on the store (docs/adr/8059 §10). The tests' store is in memory, the server's in PostgreSQL and the object
//! store (§13); both keep the promises below, and the catalog's tests run on both.

use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;

use axum::body::Bytes;
use kentos_extensions::release::{Release, Role};

use super::{CatalogId, ReleaseKey};

/// A store call's future, boxed so the store can be a trait object.
pub type StoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, StoreError>> + Send + 'a>>;

/// Why a store call did not go through.
#[derive(Debug)]
pub enum StoreError {
    /// The release moved since it was read: another request's move was written first.
    Changed,
    /// The version already has a record.
    Exists,
    /// The store failed; the detail is for the log.
    Failed(String),
}

/// Which catalogs a role is granted in.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Where {
    /// Every catalog: the catalog's own services.
    Every,
    In(CatalogId),
}

impl Where {
    /// `every`, `public` or `tenant:{slug}`. One's own catalog has no grants: its owner holds its roles.
    pub fn parse(text: &str) -> Result<Where, String> {
        match (text, CatalogId::parse(text)) {
            ("every", _) => Ok(Where::Every),
            (_, Some(CatalogId::Personal(_))) => Err(format!(
                "“{text}” bir hesabın kendi kataloğu: orada rol verilmez, sahibi yayımcı, inceleyici ve yöneticidir."
            )),
            (_, Some(catalog)) => Ok(Where::In(catalog)),
            (_, None) => Err(format!(
                "Katalog “{text}” olmaz: every, public ya da tenant:KISA-AD (kurumun kısa adı)."
            )),
        }
    }
}

/// What an account may do in one catalog.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Access {
    /// It may see the catalog at all: every account the public one, an organisation's members theirs, the catalog's
    /// own services every catalog.
    pub member: bool,
    /// The roles granted to it there.
    pub roles: BTreeSet<Role>,
    /// The publishers it is a member of.
    pub publishers: BTreeSet<String>,
}

/// The catalog's store.
pub trait ReleaseStore: Send + Sync {
    /// Every release of a catalog, whoever may see it: the caller filters.
    fn releases<'a>(&'a self, catalog: &'a CatalogId) -> StoreFuture<'a, Vec<Release>>;

    /// One release.
    fn release<'a>(&'a self, key: &'a ReleaseKey) -> StoreFuture<'a, Option<Release>>;

    /// Writes a release: a new one when `moves` is none (`Exists` when its version has a record), else over the stored
    /// one only when that has made exactly `moves` moves (`Changed` otherwise). A move once kept is never changed or
    /// taken away: the new record's moves begin with the stored record's.
    fn write<'a>(
        &'a self,
        key: &'a ReleaseKey,
        moves: Option<usize>,
        release: &'a Release,
    ) -> StoreFuture<'a, ()>;

    /// Keeps a package's bytes under its content digest, once: a digest is always the same content, and bytes once kept
    /// are never replaced.
    fn put_package<'a>(&'a self, digest: &'a str, bytes: Bytes) -> StoreFuture<'a, ()>;

    /// A package's bytes, by its content digest.
    fn package<'a>(&'a self, digest: &'a str) -> StoreFuture<'a, Option<Bytes>>;

    /// What an account may do in a catalog.
    fn access<'a>(&'a self, catalog: &'a CatalogId, account: &'a str) -> StoreFuture<'a, Access>;

    /// Whether the catalog trusts a publisher: its code takes the review's fast path when nothing new is asked (D12).
    fn trusted<'a>(&'a self, catalog: &'a CatalogId, publisher: &'a str) -> StoreFuture<'a, bool>;
}

//! The catalog's persistent store (docs/adr/8059 §11, §13): releases, their moves and the grants in PostgreSQL
//! (migration 0042), packages' bytes in the server's object store. The promises are [`ReleaseStore`]'s, the memory
//! store's too; the catalog's tests run on both.
//!
//! - **Scope:** every call is a transaction scoped to the catalog it is about: an organisation's (its id as the
//!   tenant), an account's own (its id as the user) or the public one (no scope). Row-level security shows no other
//!   catalog's releases; who may see which release is still the flow's (`Release::visible_to`).
//! - **Reading:** a record is read through the flow's checks ([`Release::read`]) and its moves must be the move
//!   table's, row for row; a record that is not is the store's failure, never a release.
//! - **Writing:** a new release is an insert that loses to the same version's record (`Exists`); a move is an update
//!   of the record it was made on only (`where moves = <as read>`, else `Changed`), its new moves added in the same
//!   transaction after the kept ones are checked. The database refuses a frozen content, a changed identity and a
//!   move taken away by itself (a trigger), and the server's role cannot change or remove a kept move.
//! - **Packages:** written once under their content digest, under a lock on the digest: the bytes go to a key of their
//!   own, are flushed and renamed into place, then the row is written. A package already kept is not written again.

use std::collections::BTreeMap;

use axum::body::Bytes;
use kentos_application::blobs::Blobs;
use kentos_extensions::release::Release;
use kentos_postgres::{Db, Scope, rescope};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::store::{Access, ReleaseStore, StoreError, StoreFuture};
use super::{CatalogId, ReleaseKey};

#[path = "postgres/access.rs"]
mod access;
#[path = "postgres/moves.rs"]
mod moves;
#[path = "postgres/packages.rs"]
mod packages;

pub(crate) use access::{role_id, role_of};
use moves::{MoveRow, add_moves, moves_of, read, rows_of};

/// The persistent store: the database as the server's role, the server's object store.
pub struct PgStore {
    db: Db,
    blobs: Blobs,
}

/// A catalog as the tables name it: its kind and its reference (none, an organisation's id, an account's id).
struct Place {
    kind: &'static str,
    reference: String,
}

fn failed(e: impl std::fmt::Display) -> StoreError {
    StoreError::Failed(e.to_string())
}

/// An account's id as the server writes it: a UUID in its own lowercase form; anything else names no account here.
fn account_id(text: &str) -> Option<Uuid> {
    Uuid::parse_str(text)
        .ok()
        .filter(|id| id.to_string() == text)
}

/// An organisation's id by its short name; none when there is none.
async fn tenant_id(
    tx: &mut Transaction<'static, Postgres>,
    slug: &str,
) -> Result<Option<Uuid>, StoreError> {
    sqlx::query_scalar("select kentos.extension_catalog_tenant($1)")
        .bind(slug)
        .fetch_one(&mut **tx)
        .await
        .map_err(failed)
}

impl PgStore {
    pub fn new(db: Db, blobs: Blobs) -> PgStore {
        PgStore { db, blobs }
    }

    /// A transaction scoped to `catalog`, and how the tables name it; none for a catalog that does not exist (an
    /// organisation that is not, an account id that is not one).
    async fn begin(
        &self,
        catalog: &CatalogId,
    ) -> Result<Option<(Transaction<'static, Postgres>, Place)>, StoreError> {
        let mut tx = self.db.pool.begin().await.map_err(failed)?;
        let (place, scope) = match catalog {
            CatalogId::Public => (
                Place {
                    kind: "public",
                    reference: String::new(),
                },
                Scope::default(),
            ),
            CatalogId::Tenant(slug) => {
                let Some(id) = tenant_id(&mut tx, slug).await? else {
                    return Ok(None);
                };
                let place = Place {
                    kind: "tenant",
                    reference: id.to_string(),
                };
                (
                    place,
                    Scope {
                        tenant: Some(id),
                        ..Scope::default()
                    },
                )
            }
            CatalogId::Personal(account) => {
                let Some(id) = account_id(account) else {
                    return Ok(None);
                };
                let place = Place {
                    kind: "personal",
                    reference: id.to_string(),
                };
                (
                    place,
                    Scope {
                        user: Some(id),
                        ..Scope::default()
                    },
                )
            }
        };
        rescope(&mut tx, scope).await.map_err(failed)?;
        Ok(Some((tx, place)))
    }

    async fn list(&self, catalog: &CatalogId) -> Result<Vec<Release>, StoreError> {
        let Some((mut tx, place)) = self.begin(catalog).await? else {
            return Ok(Vec::new());
        };
        let records: Vec<(Uuid, String)> = sqlx::query_as(
            "select id, record::text from kentos.extension_release
              where catalog_kind = $1 and catalog_ref = $2 order by extension_id, version",
        )
        .bind(place.kind)
        .bind(&place.reference)
        .fetch_all(&mut *tx)
        .await
        .map_err(failed)?;
        let moves: Vec<MoveRow> = sqlx::query_as(
            "select m.release_id, m.seq, m.at, m.account, m.via, m.action, m.from_state, m.to_state, m.content,
                    m.gate, m.reason
               from kentos.extension_release_move m join kentos.extension_release r on r.id = m.release_id
              where r.catalog_kind = $1 and r.catalog_ref = $2 order by m.release_id, m.seq",
        )
        .bind(place.kind)
        .bind(&place.reference)
        .fetch_all(&mut *tx)
        .await
        .map_err(failed)?;
        tx.commit().await.map_err(failed)?;
        let mut by_release: BTreeMap<Uuid, Vec<MoveRow>> = BTreeMap::new();
        for row in moves {
            by_release.entry(row.release_id).or_default().push(row);
        }
        records
            .iter()
            .map(|(id, text)| {
                read(
                    *id,
                    text,
                    by_release.get(id).map(Vec::as_slice).unwrap_or_default(),
                )
            })
            .collect()
    }

    async fn one(&self, key: &ReleaseKey) -> Result<Option<Release>, StoreError> {
        let Some((mut tx, place)) = self.begin(&key.catalog).await? else {
            return Ok(None);
        };
        let record: Option<(Uuid, String)> = sqlx::query_as(
            "select id, record::text from kentos.extension_release
              where catalog_kind = $1 and catalog_ref = $2 and extension_id = $3 and version = $4",
        )
        .bind(place.kind)
        .bind(&place.reference)
        .bind(&key.extension)
        .bind(&key.version)
        .fetch_optional(&mut *tx)
        .await
        .map_err(failed)?;
        let Some((id, text)) = record else {
            return Ok(None);
        };
        let kept = moves_of(&mut tx, id).await?;
        tx.commit().await.map_err(failed)?;
        read(id, &text, &kept).map(Some)
    }

    async fn written(
        &self,
        key: &ReleaseKey,
        moves: Option<usize>,
        release: &Release,
    ) -> Result<(), StoreError> {
        let Some((mut tx, place)) = self.begin(&key.catalog).await? else {
            return Err(StoreError::Failed(format!(
                "{}: böyle bir katalog yok",
                key.catalog
            )));
        };
        let record = serde_json::to_string(release).map_err(failed)?;
        let content = release.content();
        let capabilities = serde_json::to_string(&content.capabilities).map_err(failed)?;
        let count = i32::try_from(release.audit().len()).map_err(failed)?;
        let (id, from) = match moves {
            None => {
                let inserted: Option<Uuid> = sqlx::query_scalar(
                    "insert into kentos.extension_release (id, catalog_kind, catalog_ref, extension_id, version,
                        publisher, author, state, package_digest, lock_digest, sbom_digest, capabilities, record, moves)
                     values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12::jsonb, $13::jsonb, $14)
                     on conflict (catalog_kind, catalog_ref, extension_id, version) do nothing
                     returning id",
                )
                .bind(Uuid::now_v7())
                .bind(place.kind)
                .bind(&place.reference)
                .bind(&key.extension)
                .bind(&key.version)
                .bind(release.publisher())
                .bind(release.author())
                .bind(release.state().id())
                .bind(&content.package)
                .bind(&content.lock)
                .bind(&content.sbom)
                .bind(&capabilities)
                .bind(&record)
                .bind(count)
                .fetch_optional(&mut *tx)
                .await
                .map_err(failed)?;
                (inserted.ok_or(StoreError::Exists)?, 0)
            }
            Some(n) => {
                let expected = i32::try_from(n).map_err(failed)?;
                let updated: Option<Uuid> = sqlx::query_scalar(
                    "update kentos.extension_release
                        set state = $5, package_digest = $6, lock_digest = $7, sbom_digest = $8,
                            capabilities = $9::jsonb, record = $10::jsonb, moves = $11
                      where catalog_kind = $1 and catalog_ref = $2 and extension_id = $3 and version = $4
                        and moves = $12
                     returning id",
                )
                .bind(place.kind)
                .bind(&place.reference)
                .bind(&key.extension)
                .bind(&key.version)
                .bind(release.state().id())
                .bind(&content.package)
                .bind(&content.lock)
                .bind(&content.sbom)
                .bind(&capabilities)
                .bind(&record)
                .bind(count)
                .bind(expected)
                .fetch_optional(&mut *tx)
                .await
                .map_err(failed)?;
                let id = updated.ok_or(StoreError::Changed)?;
                (id, n)
            }
        };
        let rows = rows_of(id, release)?;
        if rows.len() < from || (from > 0 && moves_of(&mut tx, id).await? != rows[..from]) {
            return Err(StoreError::Failed(format!(
                "{} {}: yazılan kayıt eski hareketleri değiştiriyor",
                key.extension, key.version
            )));
        }
        add_moves(&mut tx, id, &rows[from..]).await?;
        tx.commit().await.map_err(failed)
    }
}

impl ReleaseStore for PgStore {
    fn releases<'a>(&'a self, catalog: &'a CatalogId) -> StoreFuture<'a, Vec<Release>> {
        Box::pin(self.list(catalog))
    }

    fn release<'a>(&'a self, key: &'a ReleaseKey) -> StoreFuture<'a, Option<Release>> {
        Box::pin(self.one(key))
    }

    fn write<'a>(
        &'a self,
        key: &'a ReleaseKey,
        moves: Option<usize>,
        release: &'a Release,
    ) -> StoreFuture<'a, ()> {
        Box::pin(self.written(key, moves, release))
    }

    fn put_package<'a>(&'a self, digest: &'a str, bytes: Bytes) -> StoreFuture<'a, ()> {
        Box::pin(self.keep_package(digest, bytes))
    }

    fn package<'a>(&'a self, digest: &'a str) -> StoreFuture<'a, Option<Bytes>> {
        Box::pin(self.kept_package(digest))
    }

    fn access<'a>(&'a self, catalog: &'a CatalogId, account: &'a str) -> StoreFuture<'a, Access> {
        Box::pin(self.access_of(catalog, account))
    }

    fn trusted<'a>(&'a self, catalog: &'a CatalogId, publisher: &'a str) -> StoreFuture<'a, bool> {
        Box::pin(self.trusts(catalog, publisher))
    }
}

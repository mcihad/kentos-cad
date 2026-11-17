//! The store a catalog case runs on (docs/adr/8059 §13): the memory store, or the persistent one on a test database,
//! where accounts and organisations are rows and the administrator's grants are written as `kentosd extension-catalog`
//! writes them.

use std::sync::Arc;

use kentos_application::blobs::Blobs;
use kentos_contracts::TenantRole;
use kentos_extensions::release::{Release, Role};
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

use super::id;
use crate::extension_catalog::memory::MemoryStore;
use crate::extension_catalog::postgres::PgStore;
use crate::extension_catalog::store::{ReleaseStore, Where};
use crate::extension_catalog::{CatalogId, ReleaseKey, grants as granted};

/// The store a case runs on, and what the case does to it behind the API's back: the catalog's grants, the stored
/// record, how many packages are kept.
#[derive(Clone)]
pub(super) enum Bench {
    Memory(Arc<MemoryStore>),
    Database(Arc<Database>),
}

/// The persistent store on a test database, and the owner's pool the administrator grants with.
pub(super) struct Database {
    owner: sqlx::PgPool,
    store: Arc<PgStore>,
}

impl Database {
    /// The account named so, a row the first time.
    pub(super) async fn account(&self, name: &str) {
        let account = Uuid::parse_str(&id(name)).unwrap();
        sqlx::query(
            "insert into kentos.app_user (id, issuer, subject, display_name) values ($1, 'kentos:local', $2, $3)
             on conflict do nothing",
        )
        .bind(account)
        .bind(account.to_string())
        .bind(name)
        .execute(&self.owner)
        .await
        .unwrap();
    }

    /// The organisation of this short name, opened the first time.
    pub(super) async fn tenant(&self, slug: &str) {
        sqlx::query(
            "insert into kentos.tenant (id, slug, name, seat_limit) values ($1, $2, $2, 10)
             on conflict (slug) do nothing",
        )
        .bind(Uuid::now_v7())
        .bind(slug)
        .execute(&self.owner)
        .await
        .unwrap();
    }

    pub(super) async fn place(&self, place: &Where) {
        if let Where::In(CatalogId::Tenant(slug)) = place {
            self.tenant(slug).await;
        }
    }
}

impl Bench {
    pub(super) fn memory() -> Bench {
        Bench::Memory(Arc::new(MemoryStore::default()))
    }

    /// The persistent store on `db`, its objects in a folder of their own.
    pub(super) fn database(db: &TestDb) -> Bench {
        let blobs =
            Blobs::new(std::env::temp_dir().join(format!("kentos-katalog-{}", Uuid::now_v7())));
        Bench::Database(Arc::new(Database {
            owner: db.owner.clone(),
            store: Arc::new(PgStore::new(db.app.clone(), blobs)),
        }))
    }

    pub(super) fn store(&self) -> Arc<dyn ReleaseStore> {
        match self {
            Bench::Memory(m) => m.clone(),
            Bench::Database(d) => d.store.clone(),
        }
    }

    pub(super) async fn grant(&self, place: Where, name: &str, role: Role) {
        match self {
            Bench::Memory(m) => m.grant(place, &id(name), role),
            Bench::Database(d) => {
                d.account(name).await;
                d.place(&place).await;
                granted::grant(&d.owner, &place, &id(name), role)
                    .await
                    .unwrap();
            }
        }
    }

    /// Lets an account into an organisation's catalog without a role: in the database, a membership.
    pub(super) async fn admit(&self, catalog: CatalogId, name: &str) {
        match self {
            Bench::Memory(m) => m.admit(catalog, &id(name)),
            Bench::Database(d) => {
                let CatalogId::Tenant(slug) = &catalog else {
                    panic!("only an organisation's catalog has members: {catalog}");
                };
                d.account(name).await;
                d.tenant(slug).await;
                kentos_application::admin::set_membership(
                    &d.owner,
                    slug,
                    &id(name),
                    TenantRole::Viewer,
                    false,
                )
                .await
                .unwrap();
            }
        }
    }

    pub(super) async fn join(&self, name: &str, publisher: &str) {
        match self {
            Bench::Memory(m) => m.join(&id(name), publisher),
            Bench::Database(d) => {
                d.account(name).await;
                granted::publisher(&d.owner, publisher, Some(publisher))
                    .await
                    .unwrap();
                granted::member(&d.owner, publisher, &id(name), true)
                    .await
                    .unwrap();
            }
        }
    }

    pub(super) async fn trust(&self, catalog: CatalogId, publisher: &str) {
        match self {
            Bench::Memory(m) => m.trust(catalog, publisher),
            Bench::Database(d) => {
                let place = Where::In(catalog);
                d.place(&place).await;
                granted::publisher(&d.owner, publisher, Some(publisher))
                    .await
                    .unwrap();
                granted::trust(&d.owner, &place, publisher, true)
                    .await
                    .unwrap();
            }
        }
    }

    /// The stored record, read behind the API's back.
    pub(super) async fn stored(&self, key: &ReleaseKey) -> Option<Release> {
        self.store().release(key).await.unwrap()
    }

    /// How many packages are kept.
    pub(super) async fn packages(&self) -> usize {
        match self {
            Bench::Memory(m) => m.packages(),
            Bench::Database(d) => {
                let n: i64 = sqlx::query_scalar("select count(*) from kentos.extension_package")
                    .fetch_one(&d.owner)
                    .await
                    .unwrap();
                usize::try_from(n).unwrap()
            }
        }
    }
}

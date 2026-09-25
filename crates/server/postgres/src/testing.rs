//! Throwaway databases for tests on the local server. Each test gets its own
//! `kentos_cad_test_<time>_<random>` database with the migrations applied,
//! and pools for the owner and the server's role. Needs the roles from
//! `kentosd db-setup` (their passwords come from `.env.local`) and an admin
//! URL (`KENTOS_TEST_ADMIN_URL`, default the local development server). When
//! the server cannot be reached the test is skipped, loudly, unless
//! `KENTOS_TEST_DB=required`.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool};

use crate::setup::{DATABASE_URL, OWNER_URL, prepare_database};
use crate::{Db, MIGRATOR, OWNER_ROLE, env, migrate};

const DEFAULT_ADMIN: &str = "postgres://postgres:postgres@127.0.0.1:5432/postgres";
/// Test databases older than this are left over from a crashed run and dropped.
const STALE_SECS: u64 = 3600;

pub struct TestDb {
    pub name: String,
    pub app: Db,
    pub owner: PgPool,
    admin: PgConnectOptions,
}

fn repo_env() -> std::collections::BTreeMap<String, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.env.local");
    env::read_file(&path).unwrap_or_default()
}

fn skip_or_fail(why: String) -> Option<TestDb> {
    if std::env::var("KENTOS_TEST_DB").as_deref() == Ok("required") {
        panic!("test veritabanı gerekli ama kurulamadı: {why}");
    }
    eprintln!("⚠ veritabanı testi atlandı: {why}");
    None
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

async fn drop_stale(admin: &PgPool) {
    let names: Vec<String> = sqlx::query_scalar(
        "select datname::text from pg_database where datname like 'kentos\\_cad\\_test\\_%'",
    )
    .fetch_all(admin)
    .await
    .unwrap_or_default();
    for name in names {
        let born = name
            .split('_')
            .nth(3)
            .and_then(|t| t.parse::<u64>().ok())
            .unwrap_or(u64::MAX);
        if now_secs().saturating_sub(born) > STALE_SECS && born != u64::MAX {
            let _ = sqlx::query(AssertSqlSafe(format!(
                "drop database if exists {name} with (force)"
            )))
            .execute(admin)
            .await;
        }
    }
}

impl TestDb {
    /// A fresh migrated database, or `None` (skipped) when no local server is available.
    pub async fn create() -> Option<TestDb> {
        Self::create_up_to(None).await
    }

    /// A fresh database with the migrations up to `version` only, for testing
    /// how a later migration converts existing rows: fill it, then [`Self::migrate`].
    pub async fn create_before(version: i64) -> Option<TestDb> {
        Self::create_up_to(Some(version - 1)).await
    }

    /// Applies the migrations not applied yet (after [`Self::create_before`]).
    pub async fn migrate(&self) {
        migrate(&self.owner).await.expect("migration uygulanamadı");
    }

    async fn create_up_to(last: Option<i64>) -> Option<TestDb> {
        let vars = repo_env();
        let admin_url =
            std::env::var("KENTOS_TEST_ADMIN_URL").unwrap_or_else(|_| DEFAULT_ADMIN.into());
        let (Some(app_url), Some(owner_url)) = (
            env::lookup(&vars, DATABASE_URL),
            env::lookup(&vars, OWNER_URL),
        ) else {
            return skip_or_fail("`.env.local` yok: önce `kentosd db-setup` çalıştırın".into());
        };
        let admin: PgConnectOptions = match admin_url.parse() {
            Ok(o) => o,
            Err(e) => return skip_or_fail(format!("yönetici adresi: {e}")),
        };
        let pool = |o: PgConnectOptions, n: u32| {
            PgPoolOptions::new()
                .max_connections(n)
                .acquire_timeout(Duration::from_secs(10))
                .connect_with(o)
        };
        let root = match pool(admin.clone(), 1).await {
            Ok(p) => p,
            Err(e) => return skip_or_fail(format!("PostgreSQL'e bağlanılamadı: {e}")),
        };
        drop_stale(&root).await;
        let name = format!(
            "kentos_cad_test_{}_{}",
            now_secs(),
            &uuid::Uuid::new_v4().simple().to_string()[..10]
        );
        sqlx::query(AssertSqlSafe(format!(
            "create database {name} owner {OWNER_ROLE}"
        )))
        .execute(&root)
        .await
        .expect("test veritabanı açılamadı");
        root.close().await;
        let on_db = pool(admin.clone().database(&name), 1)
            .await
            .expect("test veritabanına bağlanılamadı");
        prepare_database(&on_db, &name)
            .await
            .expect("eklentiler kurulamadı");
        on_db.close().await;

        let with_db = |url: &str| -> PgConnectOptions {
            url.parse::<PgConnectOptions>()
                .expect("adres")
                .database(&name)
        };
        let owner = pool(with_db(&owner_url), 4)
            .await
            .expect("sahip rolüyle bağlanılamadı");
        match last {
            None => migrate(&owner).await,
            Some(version) => MIGRATOR.run_to(version, &owner).await,
        }
        .expect("migration uygulanamadı");
        let app = Db {
            pool: pool(with_db(&app_url), 8)
                .await
                .expect("sunucu rolüyle bağlanılamadı"),
        };
        Some(TestDb {
            name,
            app,
            owner,
            admin,
        })
    }

    /// Drops the database (call at the end of the test; a crashed run is cleaned up after an hour).
    pub async fn close(self) {
        self.app.pool.close().await;
        self.owner.close().await;
        if let Ok(root) = PgPoolOptions::new()
            .max_connections(1)
            .connect_with(self.admin)
            .await
        {
            let _ = sqlx::query(AssertSqlSafe(format!(
                "drop database if exists {} with (force)",
                self.name
            )))
            .execute(&root)
            .await;
            root.close().await;
        }
    }
}

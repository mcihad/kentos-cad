//! PostgreSQL + PostGIS access for KentOS (CLAUDE.md §13–16, docs/adr/0006,
//! 0015).
//!
//! Two roles: the owner (`kentos_cad_owner`) runs migrations and the admin
//! commands; the server connects as `kentos_cad_app`, which is not the owner
//! and has no BYPASSRLS, so row-level security scopes every tenant-bound
//! row. A tenant-bound query runs only inside [`Db::scoped`], which sets the
//! tenant, user and project for that transaction alone (`set_config(…,
//! true)`), so a pooled connection never carries one request's scope into
//! the next. A project's own rows (objects, command log, audit, events) are
//! visible only with that project in scope, and only while the user has a
//! role in it (migration 0004).

pub mod env;
pub mod setup;
/// Throwaway test databases (used by the tests of this crate and the crates above).
pub mod testing;

use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub use sqlx;

/// The migrations of this crate, applied by `kentosd migrate` (owner role only).
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// The role the server connects as; the migrations grant it its rights by this name.
pub const APP_ROLE: &str = "kentos_cad_app";
pub const OWNER_ROLE: &str = "kentos_cad_owner";

/// Who a transaction acts for, and in which project. Any part may be unknown
/// (signing in knows no tenant yet; a tenant's project list is no project's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    pub tenant: Option<Uuid>,
    pub user: Option<Uuid>,
    pub project: Option<Uuid>,
}

#[derive(Clone, Debug)]
pub struct Db {
    pub pool: PgPool,
}

impl Db {
    /// A pool of at most `max` connections; each waits at most `acquire` for a free one.
    pub async fn connect(url: &str, max: u32, acquire: Duration) -> Result<Self, sqlx::Error> {
        let options: PgConnectOptions = url.parse()?;
        let pool = PgPoolOptions::new()
            .max_connections(max)
            .acquire_timeout(acquire)
            .connect_with(options.application_name("kentosd"))
            .await?;
        Ok(Self { pool })
    }

    /// A transaction acting for `scope`: row-level security sees this tenant, user and project until it ends.
    pub async fn scoped(
        &self,
        scope: Scope,
    ) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        rescope(&mut tx, scope).await?;
        Ok(tx)
    }

    /// A read-only transaction acting for `scope` that sees one moment of the
    /// database in all its statements (repeatable read): a project's rows and
    /// its revision are read together without locking it (TODOS.md SYNC-05,
    /// docs/adr/0033). The moment is taken by the first statement, which is
    /// setting the scope.
    pub async fn snapshot(
        &self,
        scope: Scope,
    ) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read, read only")
            .execute(&mut *tx)
            .await?;
        rescope(&mut tx, scope).await?;
        Ok(tx)
    }

    /// Whether every migration of this build is applied (the server refuses to start otherwise).
    pub async fn schema_ready(&self) -> Result<bool, sqlx::Error> {
        let applied: Vec<i64> = match sqlx::query_scalar(
            "select version from _sqlx_migrations where success order by version",
        )
        .fetch_all(&self.pool)
        .await
        {
            Ok(v) => v,
            // No migrations table: nothing applied yet.
            Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("42P01") => {
                return Ok(false);
            }
            Err(e) => return Err(e),
        };
        Ok(MIGRATOR.iter().all(|m| applied.contains(&m.version)))
    }
}

/// Moves an open transaction to another scope, for the rest of that
/// transaction only (as [`Db::scoped`] sets it): one that touches two
/// projects (duplicating one into another) writes each one's rows in its
/// own scope, so row-level security still checks every row.
pub async fn rescope(
    tx: &mut Transaction<'static, Postgres>,
    scope: Scope,
) -> Result<(), sqlx::Error> {
    let text = |id: Option<Uuid>| id.map(|v| v.to_string()).unwrap_or_default();
    sqlx::query(
        "select set_config('app.tenant_id', $1, true), set_config('app.user_id', $2, true), set_config('app.project_id', $3, true)",
    )
    .bind(text(scope.tenant))
    .bind(text(scope.user))
    .bind(text(scope.project))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Applies the pending migrations; must run as the owner role.
pub async fn migrate(owner: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(owner).await
}

#[cfg(test)]
mod tests {
    use super::MIGRATOR;

    /// The SHA-384 that sqlx records for each migration once a database has
    /// applied it. A released migration never changes, not even a comment:
    /// sqlx refuses to migrate a database whose recorded checksum differs
    /// (the layout move of 595ecf1 edited a path in 0001's comment and
    /// stopped the development database). Change the schema with a new
    /// migration, and pin it here once a kept database has applied it.
    const RELEASED: [(i64, &str); 7] = [
        (
            1,
            "63c0207c18858174260b3de415ec2beb450cde6d8d80e8d8b01c54dbf67fc845621290e7855212f41dc82a8a1b888d02",
        ),
        (
            2,
            "0033569198c64566d8c0aaa288e98ce5e47b698ba93545ed7fc946d63638fd85e2c7151f3d622c3dbc211bd28e6399d9",
        ),
        (
            3,
            "082ebabcc1c80dda911fbc296ea061222f01f726e617434174b7c89ad464fb676d5d7dd6128d457f8aa106df31691084",
        ),
        (
            4,
            "feb6ce3e969c6ef0dbff47a7e23043e04835da9ebebaa1e736543bad8160478b59b1a10f0e16413899d756965e62335a",
        ),
        (
            5,
            "022aab0faf958fc6ea4fbce94961ff802969863458b706e5068e9cad72c847af72c263086a88886f2c1d74c2d3a098b6",
        ),
        (
            6,
            "e924cf7281cef923655989e02f52ade07eb5649a161d1c2b2ca92ffd2e5181f8f3426551e048f57f86630a15bbc3f31f",
        ),
        (
            7,
            "57184bf82f8bec5bdf8e82355de6baf0eba3987100d48177a46c7ffa5953611c16e51c4ada6bb1686581b532c741f130",
        ),
    ];

    #[test]
    fn released_migrations_are_unchanged() {
        for (version, pinned) in RELEASED {
            let migration = MIGRATOR
                .iter()
                .find(|m| m.version == version)
                .unwrap_or_else(|| panic!("migration {version} is gone"));
            let checksum: String = migration
                .checksum
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert_eq!(
                checksum, pinned,
                "migration {version} ({}) changed after databases applied it; restore it and add a new migration",
                migration.description
            );
        }
    }
}

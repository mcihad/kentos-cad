//! PostgreSQL + PostGIS access for KentOS (CLAUDE.md §13–16, docs/adr/0006).
//!
//! Two roles: the owner (`kentos_cad_owner`) runs migrations and the admin
//! commands; the server connects as `kentos_cad_app`, which is not the owner
//! and has no BYPASSRLS, so row-level security scopes every tenant-bound
//! row. A tenant-bound query runs only inside [`Db::scoped`], which sets the
//! tenant and user for that transaction alone (`set_config(…, true)`), so a
//! pooled connection never carries one request's scope into the next.

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

/// Who a transaction acts for. Either part may be unknown (signing in knows no tenant yet).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    pub tenant: Option<Uuid>,
    pub user: Option<Uuid>,
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

    /// A transaction acting for `scope`: row-level security sees this tenant and user until it ends.
    pub async fn scoped(
        &self,
        scope: Scope,
    ) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "select set_config('app.tenant_id', $1, true), set_config('app.user_id', $2, true)",
        )
        .bind(scope.tenant.map(|t| t.to_string()).unwrap_or_default())
        .bind(scope.user.map(|u| u.to_string()).unwrap_or_default())
        .execute(&mut *tx)
        .await?;
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

/// Applies the pending migrations; must run as the owner role.
pub async fn migrate(owner: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(owner).await
}

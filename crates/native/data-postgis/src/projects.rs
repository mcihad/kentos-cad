//! A PostgreSQL database keeps projects in a schema's `kentos_projects` table (docs/adr/9107): `name` (the key),
//! `metadata` (jsonb: when and by whom it was last written, its format) and `content` (bytea: the `.kcad` bytes). The
//! first write in a schema makes the table (`CREATE TABLE IF NOT EXISTS`), so the account needs the right to make one
//! there; reading needs only the right to read it. A write is one transaction: the table made and the row put
//! (an upsert), or nothing. The time written is the server's, in UTC.

use kentos_data::DataError;
use kentos_data::projects::{PROJECTS_TABLE, ProjectStore, StoredProject, checked_name, ordered};
use sqlx::Row;

use crate::catalog::quote;
use crate::connect::{PgSource, error_of};
use crate::runtime;

/// The schema projects are kept in (`public` when none is said), quoted, with the table.
fn table_in(schema: Option<&str>) -> Result<String, DataError> {
    let schema = schema
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("public");
    Ok(format!("{}.{}", quote(schema), quote(PROJECTS_TABLE)))
}

impl PgSource {
    /// Whether the table is there yet (`to_regclass`: no error for a table that is not).
    async fn has_projects(
        &self,
        conn: &mut sqlx::PgConnection,
        table: &str,
    ) -> Result<bool, DataError> {
        let row = sqlx::query("SELECT to_regclass($1) IS NOT NULL")
            .bind(table)
            .fetch_one(conn)
            .await
            .map_err(error_of)?;
        row.try_get::<bool, _>(0).map_err(error_of)
    }
}

impl ProjectStore for PgSource {
    fn projects(&self, schema: Option<&str>) -> Result<Vec<StoredProject>, DataError> {
        let table = table_in(schema)?;
        runtime()?.block_on(async {
            let mut conn = self.pool.acquire().await.map_err(error_of)?;
            if !self.has_projects(&mut conn, &table).await? {
                return Ok(Vec::new());
            }
            let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
                "SELECT name, metadata->>'modified', metadata->>'user', octet_length(content)::bigint FROM {table}"
            )))
            .fetch_all(&mut *conn)
            .await
            .map_err(error_of)?;
            let list = rows
                .iter()
                .map(|r| {
                    Ok(StoredProject {
                        name: r.try_get(0).map_err(error_of)?,
                        modified: r.try_get(1).map_err(error_of)?,
                        user: r.try_get(2).map_err(error_of)?,
                        size: r.try_get::<i64, _>(3).map_err(error_of)?.max(0) as u64,
                    })
                })
                .collect::<Result<Vec<_>, DataError>>()?;
            Ok(ordered(list))
        })
    }

    fn read_project(&self, schema: Option<&str>, name: &str) -> Result<Vec<u8>, DataError> {
        let name = checked_name(name)?;
        let table = table_in(schema)?;
        runtime()?.block_on(async {
            let mut conn = self.pool.acquire().await.map_err(error_of)?;
            if !self.has_projects(&mut conn, &table).await? {
                return Err(DataError::NoProject(name.to_owned()));
            }
            let row = sqlx::query(sqlx::AssertSqlSafe(format!(
                "SELECT content FROM {table} WHERE name = $1"
            )))
            .bind(name)
            .fetch_optional(&mut *conn)
            .await
            .map_err(error_of)?;
            match row {
                Some(r) => r.try_get::<Vec<u8>, _>(0).map_err(error_of),
                None => Err(DataError::NoProject(name.to_owned())),
            }
        })
    }

    fn write_project(
        &self,
        schema: Option<&str>,
        name: &str,
        content: &[u8],
        user: Option<&str>,
    ) -> Result<(), DataError> {
        let name = checked_name(name)?;
        let table = table_in(schema)?;
        runtime()?.block_on(async {
            let mut tx = self.pool.begin().await.map_err(error_of)?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE TABLE IF NOT EXISTS {table} (name text PRIMARY KEY, metadata jsonb NOT NULL, content bytea NOT NULL)"
            )))
            .execute(&mut *tx)
            .await
            .map_err(error_of)?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "INSERT INTO {table} (name, metadata, content) VALUES ($1,
                   jsonb_build_object('modified', to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                                      'user', $2::text, 'format', 'kcad'), $3)
                 ON CONFLICT (name) DO UPDATE SET metadata = EXCLUDED.metadata, content = EXCLUDED.content"
            )))
            .bind(name)
            .bind(user)
            .bind(content)
            .execute(&mut *tx)
            .await
            .map_err(error_of)?;
            tx.commit().await.map_err(error_of)
        })
    }

    fn delete_project(&self, schema: Option<&str>, name: &str) -> Result<(), DataError> {
        let name = checked_name(name)?;
        let table = table_in(schema)?;
        runtime()?.block_on(async {
            let mut conn = self.pool.acquire().await.map_err(error_of)?;
            if !self.has_projects(&mut conn, &table).await? {
                return Err(DataError::NoProject(name.to_owned()));
            }
            let done = sqlx::query(sqlx::AssertSqlSafe(format!(
                "DELETE FROM {table} WHERE name = $1"
            )))
            .bind(name)
            .execute(&mut *conn)
            .await
            .map_err(error_of)?;
            if done.rows_affected() == 0 {
                return Err(DataError::NoProject(name.to_owned()));
            }
            Ok(())
        })
    }
}

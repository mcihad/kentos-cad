//! Write receipts in a schema (docs/adr/9126): `kentos_writes` (`id` text primary key, `at` when, `by` who, `written`
//! jsonb). A write puts its receipt in with its rows, in the same transaction, so a commit whose answer was lost is
//! asked here afterwards: the receipt there, the write committed and its new rows' keys are in it; none, it did not.
//! The table is made only when the owner sets the receipts up; the schema's other tables stay as they are.

use kentos_data::{DataError, RECEIPTS_TABLE, Receipt};
use sqlx::Row;

use crate::catalog::quote;
use crate::connect::{PgSource, error_of};
use crate::runtime;

/// The receipts' table of a schema, quoted.
pub(crate) fn table_in(scope: &str) -> String {
    let schema = scope.trim();
    let schema = if schema.is_empty() { "public" } else { schema };
    format!("{}.{}", quote(schema), quote(RECEIPTS_TABLE))
}

/// The statement that writes a receipt; its values `$1` id, `$2` who, `$3` what it wrote.
pub(crate) fn insert_sql(scope: &str) -> String {
    format!("INSERT INTO {} (id, by, written) VALUES ($1, $2, $3::jsonb)", table_in(scope))
}

impl PgSource {
    pub(crate) fn receipts_in(&self, scope: &str) -> Result<bool, DataError> {
        let table = table_in(scope);
        runtime()?.block_on(async {
            let mut conn = self.pool.acquire().await.map_err(error_of)?;
            let row = sqlx::query("SELECT to_regclass($1) IS NOT NULL")
                .bind(&table)
                .fetch_one(&mut *conn)
                .await
                .map_err(error_of)?;
            row.try_get::<bool, _>(0).map_err(error_of)
        })
    }

    pub(crate) fn set_up_receipts_in(&self, scope: &str) -> Result<(), DataError> {
        let table = table_in(scope);
        runtime()?.block_on(async {
            let mut conn = self.pool.acquire().await.map_err(error_of)?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE TABLE IF NOT EXISTS {table} (id text PRIMARY KEY, at timestamptz NOT NULL DEFAULT now(), \
                 by text, written jsonb NOT NULL)"
            )))
            .execute(&mut *conn)
            .await
            .map_err(error_of)?;
            Ok(())
        })
    }

    pub(crate) fn receipt_in(&self, scope: &str, id: &str) -> Result<Option<Receipt>, DataError> {
        let table = table_in(scope);
        runtime()?.block_on(async {
            let mut conn = self.pool.acquire().await.map_err(error_of)?;
            let row = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT by, written::text FROM {table} WHERE id = $1")))
                .bind(id)
                .fetch_optional(&mut *conn)
                .await
                .map_err(error_of)?;
            row.map(|r| {
                Ok(Receipt {
                    id: id.to_owned(),
                    by: r.try_get(0).map_err(error_of)?,
                    written: r.try_get(1).map_err(error_of)?,
                })
            })
            .transpose()
        })
    }
}

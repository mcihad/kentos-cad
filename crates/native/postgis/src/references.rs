//! A bound table's foreign keys in a PostGIS database (docs/adr/0753 §2): its single-column ones as `pg_constraint` holds
//! them, each with the referenced table's rows as a value list's codes and labels. A key of several columns is left out
//! (a field holds one value); a view or a query has none.

use kentos_native_application::provider::{ProviderError, Reference, label_column};
use sqlx::postgres::PgConnection;

use crate::{is_query, qualified, quote, unreachable};

/// `table`'s single-column foreign keys in their column's order, each with at most `most` rows of the table it refers
/// to (more: none).
pub(crate) async fn read(
    conn: &mut PgConnection,
    table: &str,
    most: usize,
) -> Result<Vec<Reference>, ProviderError> {
    if is_query(table) {
        return Ok(Vec::new());
    }
    let (sql_name, ..) = qualified(table);
    let exists: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
        .bind(&sql_name)
        .fetch_one(&mut *conn)
        .await
        .map_err(unreachable)?;
    if exists.is_none() {
        return Err(ProviderError::NoTable(table.to_owned()));
    }
    // The column, the referenced table (as `schema.table` and as SQL) and its column.
    let keys: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT a.attname::text, pn.nspname || '.' || pc.relname, c.confrelid::regclass::text, pa.attname::text
           FROM pg_constraint c
           JOIN pg_class pc ON pc.oid = c.confrelid
           JOIN pg_namespace pn ON pn.oid = pc.relnamespace
           JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = c.conkey[1]
           JOIN pg_attribute pa ON pa.attrelid = c.confrelid AND pa.attnum = c.confkey[1]
          WHERE c.contype = 'f' AND c.conrelid = $1::regclass AND cardinality(c.conkey) = 1
          ORDER BY a.attnum, c.conname",
    )
    .bind(&sql_name)
    .fetch_all(&mut *conn)
    .await
    .map_err(unreachable)?;
    let mut out = Vec::with_capacity(keys.len());
    for (column, parent, parent_sql, to) in keys {
        // The referenced table's text columns (the string category: text, varchar, char, name) in their order.
        let texts: Vec<String> = sqlx::query_scalar(
            "SELECT a.attname::text FROM pg_attribute a JOIN pg_type t ON t.oid = a.atttypid
              WHERE a.attrelid = $1::regclass AND a.attnum > 0 AND NOT a.attisdropped AND t.typcategory = 'S'
                AND a.attname <> $2
              ORDER BY a.attnum",
        )
        .bind(&parent_sql)
        .bind(&to)
        .fetch_all(&mut *conn)
        .await
        .map_err(unreachable)?;
        let label = label_column(&texts);
        let k = quote(&to);
        let l = label
            .as_deref()
            .map_or_else(|| "NULL".to_owned(), |l| format!("{}::text", quote(l)));
        let limit = i64::try_from(most).unwrap_or(i64::MAX - 1) + 1;
        let rows: Vec<(String, Option<String>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT {k}::text, {l} FROM {parent_sql} WHERE {k} IS NOT NULL LIMIT $1"
        )))
        .bind(limit)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| ProviderError::Unreadable(format!("{parent}: {e}")))?;
        let rows = (rows.len() <= most).then_some(rows);
        out.push(Reference {
            column,
            table: parent,
            to,
            label,
            rows,
        });
    }
    Ok(out)
}

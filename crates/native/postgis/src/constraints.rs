//! A bound table's CHECK and unique constraints in a PostgreSQL database (docs/adr/0755 §2): `pg_constraint`'s CHECKs
//! (with their columns and `pg_get_constraintdef`'s text inside `CHECK (…)`, a `NOT VALID` one too) and UNIQUEs (with
//! `NULLS NOT DISTINCT` from version 15), and `pg_index`'s unique indexes no constraint owns, neither partial nor over an
//! expression. A CHECK reading a column whose comparisons KentOS cannot follow (`character(n)`, a non-deterministic
//! collation) is the table's only. A write's refusal by one of them is read back by its SQLSTATE.

use kentos_contracts::{MAX_CHECK_SQL, MAX_TABLE_CONSTRAINTS, TableCheck, TableUnique};
use kentos_native_application::provider::{ConstraintKind, ProviderError, TableConstraints};
use sqlx::postgres::PgConnection;

use crate::{is_query, qualified, unreachable};

/// A refusal by one of the table's constraints (SQLSTATE class 23): its kind, the constraint's and the column's names
/// the database gave and its words.
pub(crate) fn violation(e: &sqlx::Error) -> Option<ProviderError> {
    let d = e.as_database_error()?;
    let kind = match d.code()?.as_ref() {
        "23514" | "23P01" => ConstraintKind::Check,
        "23505" => ConstraintKind::Unique,
        "23503" => ConstraintKind::Foreign,
        "23502" => ConstraintKind::NotNull,
        _ => return None,
    };
    let pg = d.try_downcast_ref::<sqlx::postgres::PgDatabaseError>();
    Some(ProviderError::Constraint {
        kind,
        name: d.constraint().map(str::to_owned),
        column: pg.and_then(|p| p.column()).map(str::to_owned),
        detail: pg
            .and_then(|p| p.detail())
            .map_or_else(|| d.message().to_owned(), str::to_owned),
    })
}

/// The text inside `CHECK (…)` of `pg_get_constraintdef`, without `NOT VALID` and `NO INHERIT` and the blanks at its ends.
fn condition(definition: &str) -> Option<String> {
    let mut d = definition.trim();
    loop {
        let before = d;
        for suffix in [" NOT VALID", " NO INHERIT"] {
            d = d.strip_suffix(suffix).unwrap_or(d).trim_end();
        }
        if d == before {
            break;
        }
    }
    let inner = d.strip_prefix("CHECK (")?.strip_suffix(')')?.trim();
    (!inner.is_empty()).then(|| inner.to_owned())
}

/// `table`'s constraints; a view or a query has none.
pub(crate) async fn read(
    conn: &mut PgConnection,
    table: &str,
) -> Result<TableConstraints, ProviderError> {
    if is_query(table) {
        return Ok(TableConstraints::default());
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
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(&mut *conn)
        .await
        .map_err(unreachable)?;
    let nulls = if version >= 150_000 {
        "i.indnullsnotdistinct"
    } else {
        "false"
    };
    // A column KentOS cannot follow: blank-padded, or under a collation that is not deterministic.
    let unfollowed = "EXISTS (SELECT 1 FROM unnest(c.conkey) k JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k
                        LEFT JOIN pg_collation co ON co.oid = a.attcollation
                       WHERE a.atttypid = 'bpchar'::regtype OR (co.oid IS NOT NULL AND NOT co.collisdeterministic))";
    let checks: Vec<(String, String, Vec<String>, bool)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT c.conname::text, pg_get_constraintdef(c.oid),
                ARRAY(SELECT a.attname::text FROM unnest(c.conkey) k
                        JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k ORDER BY a.attnum),
                {unfollowed}
           FROM pg_constraint c
          WHERE c.conrelid = $1::regclass AND c.contype = 'c'
          ORDER BY c.conname"
    )))
    .bind(&sql_name)
    .fetch_all(&mut *conn)
    .await
    .map_err(unreachable)?;
    let mut out = TableConstraints::default();
    for (name, definition, columns, table_only) in checks {
        let Some(sql) = condition(&definition) else {
            continue;
        };
        if sql.chars().count() > MAX_CHECK_SQL || out.checks.len() >= MAX_TABLE_CONSTRAINTS {
            continue;
        }
        out.checks.push(TableCheck {
            name: Some(name),
            sql,
            columns,
            table_only,
        });
    }
    let uniques: Vec<(String, Vec<String>, bool)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT name, columns, nulls FROM (
            SELECT c.conname::text AS name,
                   ARRAY(SELECT a.attname::text FROM unnest(c.conkey) WITH ORDINALITY k(n, o)
                           JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k.n ORDER BY k.o) AS columns,
                   COALESCE({nulls}, false) AS nulls, 0 AS kind
              FROM pg_constraint c LEFT JOIN pg_index i ON i.indexrelid = c.conindid
             WHERE c.conrelid = $1::regclass AND c.contype = 'u'
            UNION ALL
            SELECT ic.relname::text,
                   ARRAY(SELECT a.attname::text FROM unnest(i.indkey::int2[]) WITH ORDINALITY k(n, o)
                           JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = k.n ORDER BY k.o),
                   COALESCE({nulls}, false), 1
              FROM pg_index i JOIN pg_class ic ON ic.oid = i.indexrelid
             WHERE i.indrelid = $1::regclass AND i.indisunique AND NOT i.indisprimary AND i.indpred IS NULL
               AND i.indexprs IS NULL AND NOT (0 = ANY (i.indkey::int2[]))
               AND NOT EXISTS (SELECT 1 FROM pg_constraint c WHERE c.conindid = i.indexrelid)
          ) u ORDER BY kind, name"
    )))
    .bind(&sql_name)
    .fetch_all(&mut *conn)
    .await
    .map_err(unreachable)?;
    for (name, columns, nulls_equal) in uniques {
        if columns.is_empty()
            || out.uniques.len() >= MAX_TABLE_CONSTRAINTS
            || out.uniques.iter().any(|u| u.columns == columns)
        {
            continue;
        }
        out.uniques.push(TableUnique {
            name: Some(name),
            columns,
            nulls_equal,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_condition_of_a_definition() {
        assert_eq!(
            condition("CHECK ((oncelik >= 1))").as_deref(),
            Some("(oncelik >= 1)")
        );
        assert_eq!(
            condition("CHECK ((a > 0)) NOT VALID").as_deref(),
            Some("(a > 0)")
        );
        assert_eq!(
            condition("CHECK ((a > 0)) NO INHERIT NOT VALID").as_deref(),
            Some("(a > 0)")
        );
        assert_eq!(
            condition("CHECK (\nCASE\n    WHEN a THEN true\n    ELSE false\nEND)").as_deref(),
            Some("CASE\n    WHEN a THEN true\n    ELSE false\nEND")
        );
        assert_eq!(condition("UNIQUE (a)"), None);
    }
}

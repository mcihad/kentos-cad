//! A query read as a table (docs/adr/9113 §2): its shape found as ADR 0582 found a bound query's, without running it.
//!
//! - The query's columns are those of a temporary view made of it in a transaction that is taken back: PostgreSQL does
//!   not take a query that writes as a view, and nothing of the view stays.
//! - The key is the column the user chose; it must be an integer (int2, int4, int8). The geometry is the column the user
//!   chose, `geometry` or `geography`, or none; its system, type and dimensions are its first row's, read with
//!   `LIMIT 1` in the same transaction, made read-only before: the query is never run where it could write.
//! - It is read as `(<query>)`: a subquery under the request's box, condition and order, in the read-only transaction
//!   every read runs in. It is never writable.

use kentos_data::{DataError, Field, QueryTable, Schema};
use sqlx::{Connection, PgConnection, Row};

use crate::catalog::{Shape, geometry_field, kind_of, quote};
use crate::connect::error_of;

/// The temporary view's name: per session, gone with the transaction.
const VIEW: &str = "kentos_sorgu";

fn unreadable(e: sqlx::Error) -> DataError {
    match error_of(e) {
        DataError::Failed(why) => DataError::Invalid(format!("Sorgu okunamadı: {why}")),
        other => other,
    }
}

/// The query's columns, each with whether it is an integer or a geometry (the temporary view's, taken back).
pub(crate) async fn columns(
    conn: &mut PgConnection,
    sql: &str,
) -> Result<Vec<kentos_data::QueryColumn>, DataError> {
    kentos_data::query::check(sql).map_err(DataError::Invalid)?;
    let mut tx = conn.begin().await.map_err(error_of)?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE TEMPORARY VIEW {VIEW} AS {sql}"
    )))
    .execute(&mut *tx)
    .await
    .map_err(unreadable)?;
    let cols = sqlx::query(
        "SELECT a.attname::text AS name, t.typname::text AS typ
         FROM pg_attribute a JOIN pg_type t ON t.oid = a.atttypid
         WHERE a.attrelid = 'pg_temp.kentos_sorgu'::regclass AND a.attnum > 0 AND NOT a.attisdropped
         ORDER BY a.attnum",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(error_of)?;
    tx.rollback().await.map_err(error_of)?;
    cols.iter()
        .map(|c| {
            let name: String = c.try_get("name").map_err(error_of)?;
            let typ: String = c.try_get("typ").map_err(error_of)?;
            Ok(kentos_data::QueryColumn {
                integer: matches!(typ.as_str(), "int2" | "int4" | "int8"),
                geometry: matches!(typ.as_str(), "geometry" | "geography"),
                name,
            })
        })
        .collect()
}

/// The query's shape.
pub(crate) async fn shape(conn: &mut PgConnection, q: &QueryTable) -> Result<Shape, DataError> {
    kentos_data::query::check(&q.sql).map_err(DataError::Invalid)?;
    let mut tx = conn.begin().await.map_err(error_of)?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE TEMPORARY VIEW {VIEW} AS {}",
        q.sql
    )))
    .execute(&mut *tx)
    .await
    .map_err(unreadable)?;
    let cols = sqlx::query(
        "SELECT a.attname::text AS name, format_type(a.atttypid, a.atttypmod) AS formatted, t.typname::text AS typ
         FROM pg_attribute a JOIN pg_type t ON t.oid = a.atttypid
         WHERE a.attrelid = 'pg_temp.kentos_sorgu'::regclass AND a.attnum > 0 AND NOT a.attisdropped
         ORDER BY a.attnum",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(error_of)?;
    let mut fields = Vec::new();
    let mut key_kind = None;
    let mut geometry_type = None;
    for c in &cols {
        let name: String = c.try_get("name").map_err(error_of)?;
        let formatted: String = c.try_get("formatted").map_err(error_of)?;
        let typ: String = c.try_get("typ").map_err(error_of)?;
        if name == q.key {
            key_kind = Some(typ.clone());
            continue;
        }
        if typ == "geometry" || typ == "geography" {
            if Some(&name) == q.geometry.as_ref() {
                geometry_type = Some(typ);
            }
            continue;
        }
        fields.push(Field {
            kind: kind_of(&typ, &formatted),
            native: formatted,
            nullable: true,
            default: None,
            generated: false,
            comment: None,
            name,
        });
    }
    match key_kind.as_deref() {
        Some("int2" | "int4" | "int8") => {}
        Some(other) => {
            return Err(DataError::Invalid(format!(
                "“{}” sütunu tam sayı değil ({other}); anahtar olarak sorgunun tam sayı bir sütununu seçin.",
                q.key
            )));
        }
        None => {
            return Err(DataError::Invalid(format!(
                "Sorguda “{}” sütunu yok; anahtar olarak sorgunun bir sütununu seçin.",
                q.key
            )));
        }
    }
    let geography = geometry_type.as_deref() == Some("geography");
    let geometry = match (&q.geometry, &geometry_type) {
        (None, _) => None,
        (Some(g), None) => {
            return Err(DataError::Invalid(format!(
                "Sorguda “{g}” adlı bir geometri sütunu yok; geometri olarak bir geometry sütunu seçin."
            )));
        }
        (Some(g), Some(_)) => {
            // The query is run here for the first time: the transaction is made read-only first (a transaction may
            // become read-only after its first statement), so a function the query calls cannot write either.
            sqlx::query("SET TRANSACTION READ ONLY")
                .execute(&mut *tx)
                .await
                .map_err(error_of)?;
            let col = if geography {
                format!("{}::geometry", quote(g))
            } else {
                quote(g)
            };
            let first = sqlx::query(sqlx::AssertSqlSafe(format!(
                "SELECT ST_SRID({col})::int4 AS srid, GeometryType({col})::text AS t, ST_NDims({col})::int4 AS dims
                 FROM ({}) q WHERE {} IS NOT NULL LIMIT 1",
                q.sql,
                quote(g)
            )))
            .fetch_optional(&mut *tx)
            .await
            .map_err(unreadable)?;
            let (t, srid, dims) = first
                .map(|r| {
                    (
                        r.try_get::<String, _>("t")
                            .unwrap_or_else(|_| "GEOMETRY".into()),
                        r.try_get::<i32, _>("srid").unwrap_or(0),
                        r.try_get::<i32, _>("dims").unwrap_or(2),
                    )
                })
                .unwrap_or(("GEOMETRY".into(), 0, 2));
            Some(geometry_field(g.clone(), &t, srid, dims))
        }
    };
    tx.rollback().await.map_err(error_of)?;
    let turkish: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_collation WHERE collname = 'tr-x-icu')",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap_or(false);
    Ok(Shape {
        schema: Schema {
            key: q.key.clone(),
            key_made: false,
            fields,
            geometry,
        },
        sql: format!("({})", q.sql),
        namespace: String::new(),
        relation: String::new(),
        geography,
        writable: false,
        rights: crate::catalog::Rights::default(),
        turkish,
        query: true,
    })
}

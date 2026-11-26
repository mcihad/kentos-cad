//! A database's tables read (docs/adr/9100 §6): the catalog, a table's shape (cached until one of our transactions
//! commits), counts, boxes, distinct values, and rows through a server-side cursor in a read-only transaction: the
//! cursor is declared once, its rows fetched a part at a time, closed when the reader is done or told to stop. A query
//! layer's counts, values and groups are read in a read-only transaction too (docs/adr/9113 §2).

use kentos_data::driver::{Capabilities, Read};
use kentos_data::geom::read_wkb;
use kentos_data::{
    Cancel, Condition, DataError, Family, Feature, Request, Schema, Source, TableInfo, Transaction,
    Value,
};
use sqlx::{Postgres, Row};

use crate::catalog::{Shape, quote, shape, tables};
use crate::connect::{PgSource, error_of};
use crate::runtime;
use crate::sql::{
    Arg, Sql, columns_of, geometry_expr, order_and_page, read_expr, value_of, where_of,
};
use crate::write::PgTransaction;

/// A statement with its values bound.
pub(crate) fn bound<'q>(
    sql: &'q str,
    args: &'q [Arg],
) -> sqlx::query::Query<'q, Postgres, sqlx::postgres::PgArguments> {
    let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
    for a in args {
        q = match a {
            Arg::Int(i) => q.bind(*i),
            Arg::Real(r) => q.bind(*r),
            Arg::Bool(b) => q.bind(*b),
            Arg::Text(t) => q.bind(t.as_str()),
            Arg::Null => q.bind(Option::<String>::None),
            Arg::Bytes(b) => q.bind(b.as_slice()),
            Arg::Ints(v) => q.bind(v.as_slice()),
        };
    }
    q
}

/// A statement's rows on a connection of the pool. A query layer's (docs/adr/9113 §2) are read in a read-only
/// transaction, as `features` reads every table's: a function the query calls cannot write while it is counted or its
/// values and groups are read.
async fn all_rows(
    pool: &sqlx::PgPool,
    sql: &str,
    args: &[Arg],
    read_only: bool,
) -> Result<Vec<sqlx::postgres::PgRow>, DataError> {
    if !read_only {
        let mut conn = pool.acquire().await.map_err(error_of)?;
        return bound(sql, args).fetch_all(&mut *conn).await.map_err(error_of);
    }
    let mut tx = pool.begin().await.map_err(error_of)?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(error_of)?;
    let rows = bound(sql, args).fetch_all(&mut *tx).await.map_err(error_of)?;
    tx.commit().await.map_err(error_of)?;
    Ok(rows)
}

impl PgSource {
    /// A table's shape, read once.
    pub(crate) fn shape(&self, table: &str) -> Result<Shape, DataError> {
        if let Some(s) = self.shapes.read().ok().and_then(|m| m.get(table).cloned()) {
            return Ok(s);
        }
        let s = runtime()?.block_on(async {
            let mut conn = self.pool.acquire().await.map_err(error_of)?;
            shape(&mut conn, table).await
        })?;
        if let Ok(mut m) = self.shapes.write() {
            m.insert(table.to_owned(), s.clone());
        }
        Ok(s)
    }
}

/// The geometry read, simplified by the server when asked (lines and areas; arcs kept).
fn geometry_select(shape: &Shape, request: &Request, s: &mut Sql) -> String {
    let Some(g) = geometry_expr(shape).filter(|_| request.geometry) else {
        return "NULL::bytea".into();
    };
    let family = shape.schema.geometry.as_ref().map(|g| g.family());
    match request.simplify.filter(|t| *t > 0.0) {
        Some(t) if family != Some(Family::Point) => {
            let p = s.bind(Arg::Real(t));
            format!(
                "ST_AsBinary(CASE WHEN ST_HasArc({g}) OR GeometryType({g}) LIKE '%POINT%' THEN {g} ELSE ST_Simplify({g}, {p}, true) END)"
            )
        }
        _ => format!("ST_AsBinary({g})"),
    }
}

impl Source for PgSource {
    fn driver(&self) -> &'static str {
        "postgis"
    }

    fn address(&self) -> &str {
        &self.address
    }

    fn tables(&self) -> Result<Vec<TableInfo>, DataError> {
        runtime()?.block_on(async {
            let mut conn = self.pool.acquire().await.map_err(error_of)?;
            tables(&mut conn).await
        })
    }

    fn schema(&self, table: &str) -> Result<Schema, DataError> {
        Ok(self.shape(table)?.schema)
    }

    fn capabilities(&self, table: &str) -> Result<Capabilities, DataError> {
        let s = self.shape(table)?;
        Ok(capabilities_of(s.writable, s.rights, s.schema.geometry.is_some()))
    }

    fn extent(&self, table: &str, estimate: bool) -> Result<Option<[f64; 4]>, DataError> {
        let s = self.shape(table)?;
        let Some(g) = s.schema.geometry.clone() else {
            return Ok(None);
        };
        let expr = geometry_expr(&s).unwrap_or_default();
        runtime()?.block_on(async {
            let mut conn = self.pool.acquire().await.map_err(error_of)?;
            if estimate && !s.geography && !s.query {
                let est = sqlx::query(
                    "SELECT ST_XMin(e)::float8 AS a, ST_YMin(e)::float8 AS b, ST_XMax(e)::float8 AS c, ST_YMax(e)::float8 AS d
                     FROM (SELECT ST_EstimatedExtent($1, $2, $3) AS e) s",
                )
                .bind(&s.namespace)
                .bind(&s.relation)
                .bind(&g.name)
                .fetch_one(&mut *conn)
                .await;
                if let Ok(r) = est {
                    let b: [Option<f64>; 4] = [
                        r.try_get("a").ok().flatten(),
                        r.try_get("b").ok().flatten(),
                        r.try_get("c").ok().flatten(),
                        r.try_get("d").ok().flatten(),
                    ];
                    if let [Some(a), Some(b), Some(c), Some(d)] = b {
                        return Ok(Some([a, b, c, d]));
                    }
                }
            }
            let sql = format!(
                "SELECT ST_XMin(e)::float8 AS a, ST_YMin(e)::float8 AS b, ST_XMax(e)::float8 AS c, ST_YMax(e)::float8 AS d
                 FROM (SELECT ST_Extent({expr}) AS e FROM {} t) s",
                s.sql
            );
            let r = sqlx::query(sqlx::AssertSqlSafe(sql))
                .fetch_one(&mut *conn)
                .await
                .map_err(error_of)?;
            let b: [Option<f64>; 4] = [
                r.try_get("a").map_err(error_of)?,
                r.try_get("b").map_err(error_of)?,
                r.try_get("c").map_err(error_of)?,
                r.try_get("d").map_err(error_of)?,
            ];
            Ok(match b {
                [Some(a), Some(b), Some(c), Some(d)] => Some([a, b, c, d]),
                _ => None,
            })
        })
    }

    fn count(&self, table: &str, request: &Request, estimate: bool) -> Result<u64, DataError> {
        let s = self.shape(table)?;
        let mut q = Sql::default();
        let w = where_of(request, &s, table, &mut q)?;
        runtime()?.block_on(async {
            if estimate && w.is_empty() && !s.query {
                let mut conn = self.pool.acquire().await.map_err(error_of)?;
                let n: Option<f64> = sqlx::query_scalar(
                    "SELECT c.reltuples::float8 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = $1 AND c.relname = $2",
                )
                .bind(&s.namespace)
                .bind(&s.relation)
                .fetch_optional(&mut *conn)
                .await
                .map_err(error_of)?;
                if let Some(n) = n.filter(|n| *n >= 0.0) {
                    return Ok(n as u64);
                }
            }
            let sql = format!("SELECT count(*)::int8 FROM {} t{w}", s.sql);
            let rows = all_rows(&self.pool, &sql, &q.args, s.query).await?;
            let n: i64 = rows
                .first()
                .ok_or(sqlx::Error::RowNotFound)
                .and_then(|r| r.try_get(0))
                .map_err(error_of)?;
            Ok(n.max(0) as u64)
        })
    }

    /// The table's counters of the rows added, changed and deleted (`pg_stat_all_tables`). Any session's write moves
    /// them once that session's statistics are flushed: at its transaction's end when a second has passed since its
    /// last flush, else within about ten seconds of idling. A view or a query keeps none.
    fn change_mark(&self, table: &str) -> Result<Option<String>, DataError> {
        let s = self.shape(table)?;
        if s.query {
            return Ok(None);
        }
        runtime()?.block_on(async {
            let mut conn = self.pool.acquire().await.map_err(error_of)?;
            let row: Option<(i64, i64, i64)> = sqlx::query_as(
                "SELECT n_tup_ins, n_tup_upd, n_tup_del FROM pg_stat_all_tables WHERE schemaname = $1 AND relname = $2",
            )
            .bind(&s.namespace)
            .bind(&s.relation)
            .fetch_optional(&mut *conn)
            .await
            .map_err(error_of)?;
            Ok(row.map(|(added, changed, deleted)| format!("{added}:{changed}:{deleted}")))
        })
    }

    fn features(
        &self,
        table: &str,
        request: &Request,
        cancel: &Cancel,
        sink: &mut dyn FnMut(Vec<Feature>) -> bool,
    ) -> Result<Read, DataError> {
        let s = self.shape(table)?;
        let fields = columns_of(request, &s.schema, table)?;
        let kinds: Vec<_> = fields.iter().map(|f| f.kind.clone()).collect();
        let mut q = Sql::default();
        let mut select = vec![
            format!("t.{}::int8", quote(&s.schema.key)),
            geometry_select(&s, request, &mut q),
        ];
        select.extend(
            fields
                .iter()
                .map(|f| read_expr(&format!("t.{}", quote(&f.name)), &f.kind)),
        );
        let w = where_of(request, &s, table, &mut q)?;
        let query = format!(
            "SELECT {} FROM {} t{w}{}",
            select.join(", "),
            s.sql,
            order_and_page(request, &s, table)?
        );
        let declare = format!("DECLARE kentos_rows NO SCROLL CURSOR FOR {query}");
        let fetch = format!("FETCH FORWARD {} FROM kentos_rows", request.chunk.max(1));
        runtime()?.block_on(async {
            let mut tx = self.pool.begin().await.map_err(error_of)?;
            sqlx::query("SET TRANSACTION READ ONLY")
                .execute(&mut *tx)
                .await
                .map_err(error_of)?;
            bound(&declare, &q.args)
                .execute(&mut *tx)
                .await
                .map_err(error_of)?;
            let mut read = Read::default();
            loop {
                if cancel.is_cancelled() {
                    read.stopped = true;
                    break;
                }
                let rows = sqlx::query(sqlx::AssertSqlSafe(fetch.as_str()))
                    .persistent(false)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(error_of)?;
                let done = rows.len() < request.chunk.max(1);
                let mut part = Vec::with_capacity(rows.len());
                for r in &rows {
                    let fid: i64 = r.try_get(0).map_err(error_of)?;
                    let wkb: Option<Vec<u8>> = r.try_get(1).map_err(error_of)?;
                    let mut values = Vec::with_capacity(kinds.len());
                    for (i, k) in kinds.iter().enumerate() {
                        values.push(value_of(r.try_get(i + 2).map_err(error_of)?, k));
                    }
                    part.push(Feature {
                        fid,
                        geometry: wkb.map(|b| read_wkb(&b).map_err(|e| e.to_string())),
                        values,
                    });
                }
                read.rows += part.len() as u64;
                let go = part.is_empty() || sink(part);
                if !go || cancel.is_cancelled() {
                    read.stopped = true;
                    break;
                }
                if done {
                    break;
                }
            }
            let _ = sqlx::query("CLOSE kentos_rows").execute(&mut *tx).await;
            tx.rollback().await.map_err(error_of)?;
            Ok(read)
        })
    }

    fn unique(
        &self,
        table: &str,
        field: &str,
        condition: Option<&Condition>,
        limit: usize,
    ) -> Result<Vec<Value>, DataError> {
        let s = self.shape(table)?;
        let kind = s
            .schema
            .field(field)
            .map(|f| f.kind.clone())
            // The key too: a query's is checked to tell its rows apart (docs/adr/9113 §3).
            .or_else(|| (field == s.schema.key).then_some(kentos_data::FieldKind::INT))
            .ok_or_else(|| DataError::NoField {
                table: table.to_owned(),
                field: field.to_owned(),
            })?;
        let request = Request {
            condition: Some(match condition {
                Some(c) => Condition::And(vec![c.clone(), Condition::NotNull(field.to_owned())]),
                None => Condition::NotNull(field.to_owned()),
            }),
            ..Request::default()
        };
        let mut q = Sql::default();
        let w = where_of(&request, &s, table, &mut q)?;
        let col = format!("t.{}", quote(field));
        // A text in the Turkish alphabet's order, as a group's (docs/adr/9108 §2).
        let by = match (kind.is_text(), s.turkish) {
            (true, true) => "d.v COLLATE \"tr-x-icu\"".to_owned(),
            (true, false) => {
                let (from, to) = kentos_data::turkish::sort_key_translation();
                format!("translate(d.v::text, '{from}', '{to}') COLLATE \"C\"")
            }
            _ => "d.v".to_owned(),
        };
        let sql = format!(
            "SELECT {} FROM (SELECT DISTINCT {col} AS v FROM {} t{w}) d ORDER BY {by} LIMIT {limit}",
            read_expr("d.v", &kind),
            s.sql
        );
        runtime()?.block_on(async {
            let rows = all_rows(&self.pool, &sql, &q.args, s.query).await?;
            rows.iter()
                .map(|r| Ok(value_of(r.try_get(0).map_err(error_of)?, &kind)))
                .collect()
        })
    }

    /// `GROUP BY` on the server: a text in the Turkish alphabet's order (`tr-x-icu`, or the translation where the server
    /// has no ICU), the empty value last.
    fn groups(
        &self,
        table: &str,
        field: &str,
        condition: Option<&Condition>,
        limit: usize,
    ) -> Result<Vec<(Value, u64)>, DataError> {
        let s = self.shape(table)?;
        let kind = s
            .schema
            .field(field)
            .map(|f| f.kind.clone())
            // The key too: a query's is checked to tell its rows apart (docs/adr/9113 §3).
            .or_else(|| (field == s.schema.key).then_some(kentos_data::FieldKind::INT))
            .ok_or_else(|| DataError::NoField {
                table: table.to_owned(),
                field: field.to_owned(),
            })?;
        let request = Request {
            condition: condition.cloned(),
            ..Request::default()
        };
        let mut q = Sql::default();
        let w = where_of(&request, &s, table, &mut q)?;
        let col = format!("t.{}", quote(field));
        let by = match (kind.is_text(), s.turkish) {
            (true, true) => "g.v COLLATE \"tr-x-icu\"".to_owned(),
            (true, false) => {
                let (from, to) = kentos_data::turkish::sort_key_translation();
                format!("translate(g.v::text, '{from}', '{to}') COLLATE \"C\"")
            }
            _ => "g.v".to_owned(),
        };
        let sql = format!(
            "SELECT {}, g.n FROM (SELECT {col} AS v, count(*) AS n FROM {} t{w} GROUP BY {col}) g \
             ORDER BY {by} NULLS LAST LIMIT {limit}",
            read_expr("g.v", &kind),
            s.sql
        );
        runtime()?.block_on(async {
            let rows = all_rows(&self.pool, &sql, &q.args, s.query).await?;
            rows.iter()
                .map(|r| {
                    let n: i64 = r.try_get(1).map_err(error_of)?;
                    Ok((value_of(r.try_get(0).map_err(error_of)?, &kind), u64::try_from(n).unwrap_or(0)))
                })
                .collect()
        })
    }

    fn query_columns(&self, sql: &str) -> Result<Vec<kentos_data::QueryColumn>, DataError> {
        runtime()?.block_on(async {
            let mut conn = self.pool.acquire().await.map_err(error_of)?;
            crate::query::columns(&mut conn, sql).await
        })
    }

    fn begin(&self) -> Result<Box<dyn Transaction>, DataError> {
        Ok(Box::new(PgTransaction::begin(self)?))
    }

    fn receipts(&self, scope: &str) -> Result<bool, DataError> {
        self.receipts_in(scope)
    }

    fn set_up_receipts(&self, scope: &str) -> Result<(), DataError> {
        self.set_up_receipts_in(scope)
    }

    fn receipt(&self, scope: &str, id: &str) -> Result<Option<kentos_data::Receipt>, DataError> {
        self.receipt_in(scope, id)
    }

    fn project_store(&self) -> Option<&dyn kentos_data::ProjectStore> {
        Some(self)
    }
}

/// A table's capabilities from what its shape says: each right on its own (docs/adr/9162), an account may update rows
/// and not add any; none where rows cannot be written at all (a view, a table without a single key).
pub(crate) fn capabilities_of(writable: bool, rights: crate::catalog::Rights, geometry: bool) -> Capabilities {
    Capabilities {
        read: true,
        insert: writable && rights.insert,
        update_values: writable && rights.update,
        update_geometry: writable && rights.update && geometry,
        delete: writable && rights.delete,
        transactions: true,
        spatial_index: true,
        simplify_at_source: true,
        query_at_source: true,
        curves: true,
        create_table: true,
        add_columns: writable && rights.alter,
    }
}

#[cfg(test)]
mod tests {
    use super::capabilities_of;
    use crate::catalog::Rights;

    /// docs/adr/9162: an account with SELECT and UPDATE only may change rows and add none; a table that cannot be
    /// written at all gives no right, whatever was granted.
    #[test]
    fn each_right_on_rows_is_its_own() {
        let update_only = capabilities_of(true, Rights { insert: false, update: true, delete: false, alter: false }, true);
        assert!(!update_only.insert && update_only.update_values && update_only.update_geometry && !update_only.delete);
        assert!(update_only.writable());
        let all = capabilities_of(true, Rights { insert: true, update: true, delete: true, alter: true }, false);
        assert!(all.insert && all.update_values && !all.update_geometry && all.delete, "no geometry to update");
        let view = capabilities_of(false, Rights { insert: true, update: true, delete: true, alter: true }, true);
        assert!(!view.writable(), "nothing where rows cannot be written");
        // docs/adr/9318: columns are added by the table's owner alone, never to a view.
        assert!(all.add_columns && !update_only.add_columns && !view.add_columns);
    }
}

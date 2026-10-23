//! New tables in a database for a run's result (docs/adr/0592 §2, plan/gis.md CBS-23): each a GeoPackage table's shape
//! made a PostgreSQL table in one transaction: `fid bigint` its key, the geometry column typed by its kind, Z and M
//! (`geometry(MultiPolygonZ, 5256)`; a column whose rows mix dimensions is an untyped `geometry` with the system's
//! check), the columns by [`crate::pg_type`], a spatial index; the rows a batch at a time, their lasting ids in the
//! schema's `kentos_uids`, the table's comment the run's lineage. A table of the same name is refused or taken out and
//! made again; a schema not there is made. Nothing is there until the transaction commits; a refusal, a failure or a
//! stop rolls it all back.

use kentos_geopackage as gpkg;
use kentos_native_application::provider::{Created, NewTable, ProviderError, TableClash};
use sqlx::Executor;
use sqlx::postgres::PgConnection;

use crate::{Bound, bound, pg_type, qualified, quote, refused};

/// The geometry column's PostgreSQL type: the kind with its dimensions and the system, or untyped when the rows may
/// or may not have a dimension.
fn geometry_type(g: &gpkg::GeometryColumn, srid: Option<u32>) -> String {
    if g.z == 2 || g.m == 2 {
        return "geometry".into();
    }
    let kind = match g.type_name.to_ascii_uppercase().as_str() {
        "GEOMETRY" => "Geometry".to_owned(),
        k => k.to_owned(),
    };
    let dims = match (g.z, g.m) {
        (1, 1) => "ZM",
        (1, _) => "Z",
        (_, 1) => "M",
        _ => "",
    };
    format!("geometry({kind}{dims}, {})", srid.unwrap_or(0))
}

/// A comment's text as a SQL literal.
fn literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// The tables made in `tx`; for each, whether one of its name was replaced; how many batches went. `go` stops it
/// (`Ok(None)`): the caller rolls back.
pub(crate) async fn create(
    tx: &mut PgConnection,
    tables: &[NewTable],
    clash: TableClash,
    batch: usize,
    go: &mut dyn FnMut(u64, u64) -> bool,
) -> Created {
    let total: u64 = tables.iter().map(|t| t.rows.len() as u64).sum();
    let mut done = 0u64;
    let mut batches = 0u32;
    let mut replaced = Vec::with_capacity(tables.len());
    let batch = batch.max(1);
    for (k, t) in tables.iter().enumerate() {
        let fail = |e: ProviderError| (k, e);
        let (sql_name, schema, name) = qualified(&t.name);
        // A schema not there yet is made, in the same transaction.
        let has: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = $1)")
                .bind(&schema)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| fail(refused(e)))?;
        if !has {
            tx.execute(sqlx::AssertSqlSafe(format!(
                "CREATE SCHEMA {}",
                quote(&schema)
            )))
            .await
            .map_err(|e| fail(refused(format!("{schema} şeması açılamadı: {e}"))))?;
        }
        // PostgreSQL cuts a longer name to 63 bytes without a word; the table would not be the one asked for.
        if let Some(long) = std::iter::once(name.as_str())
            .chain(t.layer.columns.iter().map(|c| c.name.as_str()))
            .find(|n| n.len() > 63)
        {
            return Err(fail(refused(format!(
                "“{long}” adı PostgreSQL'in 63 baytını aşıyor; katmanın ya da alanın adını kısaltın"
            ))));
        }
        let there: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
            .bind(&sql_name)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| fail(refused(e)))?;
        if there {
            match clash {
                TableClash::Refuse => return Err(fail(ProviderError::Exists(t.name.clone()))),
                TableClash::Replace => {
                    tx.execute(sqlx::AssertSqlSafe(format!("DROP TABLE {sql_name}")))
                        .await
                        .map_err(|e| fail(refused(format!("{} kaldırılamadı: {e}", t.name))))?;
                    let side = format!("{}.kentos_uids", quote(&schema));
                    let _ = sqlx::query(sqlx::AssertSqlSafe(format!(
                        "DELETE FROM {side} WHERE table_name = $1"
                    )))
                    .bind(format!("{schema}.{name}"))
                    .execute(&mut *tx)
                    .await;
                }
            }
        }
        replaced.push(there);
        // The table: its key, its geometry, its columns.
        let mut decls = vec![format!("{} bigint PRIMARY KEY", quote(&t.layer.fid))];
        if let Some(g) = &t.layer.geometry {
            decls.push(format!("{} {}", quote(&g.name), geometry_type(g, t.srid)));
        }
        for c in &t.layer.columns {
            decls.push(format!("{} {}", quote(&c.name), pg_type(c)));
        }
        tx.execute(sqlx::AssertSqlSafe(format!(
            "CREATE TABLE {sql_name} ({})",
            decls.join(", ")
        )))
        .await
        .map_err(|e| fail(refused(format!("{} oluşturulamadı: {e}", t.name))))?;
        // A geometry whose system is not the column's is refused by the type; an untyped column keeps its own.
        let srid = t.srid.unwrap_or(0);
        let mut targets = vec![quote(&t.layer.fid)];
        let mut slots = vec!["$1".to_owned()];
        let mut n = 1usize;
        if let Some(g) = &t.layer.geometry {
            n += 1;
            targets.push(quote(&g.name));
            slots.push(format!("ST_SetSRID(ST_GeomFromWKB(${n}), {srid})"));
        }
        for c in &t.layer.columns {
            n += 1;
            targets.push(quote(&c.name));
            slots.push(if matches!(c.kind, gpkg::ColumnKind::Blob(_)) {
                format!("${n}")
            } else {
                format!("CAST(${n} AS {})", pg_type(c))
            });
        }
        let insert = format!(
            "INSERT INTO {sql_name} ({}) VALUES ({})",
            targets.join(", "),
            slots.join(", ")
        );
        let width = t.layer.columns.len();
        for part in t.rows.chunks(batch) {
            if !go(done, total) {
                return Ok(None);
            }
            batches += 1;
            for (i, row) in part.iter().enumerate() {
                if row.values.len() != width {
                    return Err(fail(refused(format!(
                        "{}: satırda {} değer var, {width} sütun bekleniyor",
                        t.name,
                        row.values.len()
                    ))));
                }
                let fid = row.fid.unwrap_or((done + i as u64 + 1) as i64);
                let mut q = sqlx::query(sqlx::AssertSqlSafe(insert.as_str())).bind(fid);
                if t.layer.geometry.is_some() {
                    q = q.bind(row.geometry.as_ref().map(gpkg::write_wkb));
                }
                for v in &row.values {
                    q = match bound(v) {
                        Bound::Text(x) => q.bind(x),
                        Bound::Bytes(x) => q.bind(x),
                    };
                }
                q.execute(&mut *tx).await.map_err(|e| {
                    fail(refused(format!(
                        "{}: {fid} anahtarlı satır yazılamadı: {e}",
                        t.name
                    )))
                })?;
            }
            done += part.len() as u64;
        }
        if let Some(g) = &t.layer.geometry {
            tx.execute(sqlx::AssertSqlSafe(format!(
                "CREATE INDEX ON {sql_name} USING GIST ({})",
                quote(&g.name)
            )))
            .await
            .map_err(|e| fail(refused(e)))?;
        }
        // The rows' lasting ids, as a bound layer's are kept (docs/adr/0505).
        if !t.uids.is_empty() {
            let side = format!("{}.kentos_uids", quote(&schema));
            tx.execute(sqlx::AssertSqlSafe(format!(
                "CREATE TABLE IF NOT EXISTS {side} (table_name text NOT NULL, pk bigint NOT NULL, uid uuid NOT NULL,
                 PRIMARY KEY (table_name, pk))"
            )))
            .await
            .map_err(|e| fail(refused(format!("kalıcı kimliklerin tablosu ({side}) açılamadı: {e}"))))?;
            let put = format!(
                "INSERT INTO {side} VALUES ($1, $2, CAST($3 AS uuid)) ON CONFLICT (table_name, pk) DO UPDATE SET uid = EXCLUDED.uid"
            );
            let full = format!("{schema}.{name}");
            for (pk, uid) in &t.uids {
                sqlx::query(sqlx::AssertSqlSafe(put.as_str()))
                    .bind(&full)
                    .bind(*pk)
                    .bind(uid.to_string())
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| fail(refused(e)))?;
            }
        }
        if !t.comment.is_empty() {
            tx.execute(sqlx::AssertSqlSafe(format!(
                "COMMENT ON TABLE {sql_name} IS {}",
                literal(&t.comment)
            )))
            .await
            .map_err(|e| fail(refused(e)))?;
        }
    }
    if !go(done, total) {
        return Ok(None);
    }
    Ok(Some((replaced, batches)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(type_name: &str, z: u8, m: u8) -> gpkg::GeometryColumn {
        gpkg::GeometryColumn {
            name: "geom".into(),
            type_name: type_name.into(),
            z,
            m,
        }
    }

    #[test]
    fn a_geometry_column_takes_its_kind_and_dimensions() {
        assert_eq!(
            geometry_type(&column("MULTIPOLYGON", 0, 0), Some(5256)),
            "geometry(MULTIPOLYGON, 5256)"
        );
        assert_eq!(
            geometry_type(&column("POINT", 1, 0), Some(5256)),
            "geometry(POINTZ, 5256)"
        );
        assert_eq!(
            geometry_type(&column("LINESTRING", 1, 1), None),
            "geometry(LINESTRINGZM, 0)"
        );
        assert_eq!(
            geometry_type(&column("GEOMETRY", 0, 0), Some(4326)),
            "geometry(Geometry, 4326)"
        );
        assert_eq!(
            geometry_type(&column("POLYGON", 2, 0), Some(5256)),
            "geometry"
        );
        assert_eq!(literal("O'Neil"), "'O''Neil'");
    }
}

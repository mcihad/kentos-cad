//! PostGIS as a bound layer's source (docs/adr/0505, plan/gis.md CBS-04): the product commands' `TableProvider` over
//! sqlx, TLS with rustls and the system's root certificates. A table is read whole, as a GeoPackage table is
//! (`kentos_geopackage`'s layer, rows and values; geometries as ISO WKB), its revision the SHA-256 of its rows in key
//! order. An edit runs in one transaction: the table locked against writers, its revision checked again, the columns
//! added, the rows updated, inserted and deleted, the lasting ids of inserted rows kept in the schema's `kentos_uids`.
//! Passwords never come from the project: the address carries none; the environment's `PGPASSWORD` or `~/.pgpass`.
//!
//! Tables with a single integer primary key are listed; a feature table is one in `geometry_columns`.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Duration;

use kentos_contracts::encoding::sha256_hex;
use kentos_geopackage::{self as gpkg, ColumnKind, DataType, Value};
use kentos_native_application::provider::{
    DbRelation, Edited, MapWrite, Param, ProviderError, ProviderRead, ProviderScan, ProviderShape, ProviderTable,
    RowCheck, TableEdit, TableProvider, register_postgis,
};
use sqlx::postgres::{PgConnectOptions, PgConnection, PgRow};
use sqlx::{Connection, Executor, Row};

mod junctions;

/// How long a connection may take before the database counts as unreachable.
const CONNECT: Duration = Duration::from_secs(15);

/// The provider: a runtime of its own on the calling thread.
pub struct PostGis {
    runtime: tokio::runtime::Runtime,
}

impl PostGis {
    pub fn new() -> Result<Self, String> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map(|runtime| PostGis { runtime })
            .map_err(|e| e.to_string())
    }
}

/// Registers the PostGIS provider for this process's product commands.
pub fn register() {
    if let Ok(p) = PostGis::new() {
        register_postgis(Box::new(p));
    }
}

fn unreachable(e: impl std::fmt::Display) -> ProviderError {
    ProviderError::Unreachable(e.to_string())
}

fn refused(e: impl std::fmt::Display) -> ProviderError {
    ProviderError::Refused(e.to_string())
}

/// An identifier quoted for SQL.
fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// `schema.table` quoted, and its two parts (a name without a schema is `public`'s).
fn qualified(table: &str) -> (String, String, String) {
    let (schema, name) = table.split_once('.').unwrap_or(("public", table));
    (
        format!("{}.{}", quote(schema), quote(name)),
        schema.to_owned(),
        name.to_owned(),
    )
}

async fn connect(address: &str) -> Result<PgConnection, ProviderError> {
    let mut options = PgConnectOptions::from_str(address).map_err(unreachable)?;
    if let Ok(pw) = std::env::var("PGPASSWORD") {
        options = options.password(&pw);
    }
    tokio::time::timeout(CONNECT, PgConnection::connect_with(&options))
        .await
        .map_err(|_| {
            ProviderError::Unreachable(format!("{} saniyede bağlanılamadı", CONNECT.as_secs()))
        })?
        .map_err(unreachable)
}

/// A column's kind from its PostgreSQL type (`format_type`): exact numbers (`numeric`) and times as text, so nothing
/// passes through a double that should not.
fn kind_of(decl: &str) -> ColumnKind {
    let d = decl.to_ascii_lowercase();
    let len = d
        .split_once('(')
        .and_then(|(_, r)| r.trim_end_matches(')').parse::<u32>().ok());
    match d.split('(').next().unwrap_or_default().trim() {
        "smallint" | "integer" | "bigint" => ColumnKind::Integer,
        "real" | "double precision" => ColumnKind::Real,
        "boolean" => ColumnKind::Boolean,
        "date" => ColumnKind::Date,
        "character varying" | "character" => ColumnKind::Text(len),
        "text" => ColumnKind::Text(None),
        "bytea" => ColumnKind::Blob(None),
        _ => ColumnKind::Other(decl.to_owned()),
    }
}

/// A new column's PostgreSQL type from its kind.
fn pg_type(c: &gpkg::Column) -> String {
    match &c.kind {
        ColumnKind::Integer => "bigint".into(),
        ColumnKind::Real => "double precision".into(),
        ColumnKind::Boolean => "boolean".into(),
        ColumnKind::Date => "date".into(),
        ColumnKind::DateTime => "timestamp".into(),
        ColumnKind::Text(Some(n)) => format!("varchar({n})"),
        ColumnKind::Text(None) | ColumnKind::Other(_) => "text".into(),
        ColumnKind::Blob(_) => "bytea".into(),
    }
}

/// What a select reads a column as.
fn read_as(c: &gpkg::Column) -> String {
    let q = quote(&c.name);
    match c.kind {
        ColumnKind::Integer => format!("{q}::bigint"),
        ColumnKind::Real => format!("{q}::float8"),
        ColumnKind::Boolean => format!("{q}::boolean"),
        ColumnKind::Blob(_) => format!("{q}::bytea"),
        _ => format!("{q}::text"),
    }
}

/// A table's, a view's or a query's shape: its definition, system, whether rows can be written, and what a select
/// reads from (`schema.table` quoted, or the query as a subquery; docs/adr/0582 §3).
struct Shape {
    layer: gpkg::Layer,
    srid: Option<u32>,
    writable: bool,
    from: String,
    /// The schema whose `kentos_uids` keeps lasting ids; none for a query.
    schema: Option<String>,
}

/// The integer columns a view or a query is keyed by, in the order looked for (QGIS's habits).
const KEYS: [&str; 4] = ["fid", "id", "gid", "ogc_fid"];

/// What `table` names: a query when it starts with `SELECT` or `WITH`, else a table or a view.
fn is_query(table: &str) -> bool {
    let word: String = table
        .trim_start()
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect::<String>()
        .to_ascii_lowercase();
    word == "select" || word == "with"
}

/// A query's own checks and its name (`sorgu_` and the first 16 digits of its SHA-256: the same query, the same
/// lasting ids).
fn query_of(table: &str) -> Result<(String, String), ProviderError> {
    let sql = table.trim();
    if sql.contains(';') {
        return Err(ProviderError::Unreadable(
            "sorgu katmanı tek bir SELECT deyimidir; noktalı virgül olmaz".into(),
        ));
    }
    Ok((sql.to_owned(), format!("sorgu_{}", &sha256_hex(sql.as_bytes())[..16])))
}

/// A view's or a query's key: the first of `KEYS` among its integer columns.
fn key_among(name: &str, attributes: &[(String, String)]) -> Result<String, ProviderError> {
    KEYS.iter()
        .find_map(|k| {
            attributes
                .iter()
                .find(|(n, d)| n == k && kind_of(d) == ColumnKind::Integer)
                .map(|(n, _)| n.clone())
        })
        .ok_or_else(|| ProviderError::NoKey(name.to_owned()))
}

/// A geometry column's type and system from its declared type (`geometry(MultiPolygon,5256)`, `geometry`).
fn geometry_decl(decl: &str) -> Option<(String, Option<u32>)> {
    let d = decl.trim();
    if !d.to_ascii_lowercase().starts_with("geometry") {
        return None;
    }
    let inner = d
        .split_once('(')
        .map(|(_, r)| r.trim_end_matches(')').to_owned());
    let Some(inner) = inner else {
        return Some(("GEOMETRY".into(), None));
    };
    let mut parts = inner.split(',');
    let kind = parts.next().unwrap_or("Geometry").trim().to_ascii_uppercase();
    let srid = parts
        .next()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&s| s > 0);
    Some((kind, srid))
}

/// The geometry column of a type and dimension as `geometry_columns` says it.
fn geometry_column(name: String, kind: &str, dims: i32) -> gpkg::GeometryColumn {
    let up = kind.to_ascii_uppercase();
    let (base, z, m) = if let Some(b) = up.strip_suffix("ZM") {
        (b.to_owned(), 1, 1)
    } else if let Some(b) = up.strip_suffix('Z') {
        (b.to_owned(), 1, 0)
    } else if let Some(b) = up.strip_suffix('M') {
        (b.to_owned(), 0, 1)
    } else {
        (up, u8::from(dims >= 3), u8::from(dims >= 4))
    };
    gpkg::GeometryColumn {
        name,
        type_name: base,
        z,
        m,
    }
}

fn layer_of(
    name: String,
    geometry: Option<gpkg::GeometryColumn>,
    srid: Option<u32>,
    fid: String,
    columns: Vec<gpkg::Column>,
) -> gpkg::Layer {
    gpkg::Layer {
        name,
        data_type: if geometry.is_some() {
            DataType::Features
        } else {
            DataType::Attributes
        },
        identifier: None,
        description: None,
        bounds: None,
        srs_id: srid.and_then(|s| i32::try_from(s).ok()),
        geometry,
        fid,
        columns,
        indexed: false,
    }
}

fn columns_of(
    attributes: Vec<(String, String, bool, Option<String>)>,
    fid: &str,
    geometry: Option<&String>,
) -> Vec<gpkg::Column> {
    attributes
        .into_iter()
        .filter(|(n, ..)| n != fid && Some(n) != geometry)
        .map(|(name, decl, not_null, default)| gpkg::Column {
            kind: kind_of(&decl),
            name,
            decl,
            not_null,
            default,
        })
        .collect()
}

async fn shape(conn: &mut PgConnection, table: &str) -> Result<Shape, ProviderError> {
    if is_query(table) {
        return query_shape(conn, table).await;
    }
    let (sql_name, schema, name) = qualified(table);
    let exists: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
        .bind(&sql_name)
        .fetch_one(&mut *conn)
        .await
        .map_err(unreachable)?;
    if exists.is_none() {
        return Err(ProviderError::NoTable(table.to_owned()));
    }
    let relkind: String = sqlx::query_scalar("SELECT relkind::text FROM pg_class WHERE oid = $1::regclass")
        .bind(&sql_name)
        .fetch_one(&mut *conn)
        .await
        .map_err(unreachable)?;
    let view = matches!(relkind.as_str(), "v" | "m");
    let attributes: Vec<(String, String, bool, Option<String>)> = sqlx::query_as(
        "SELECT a.attname::text, format_type(a.atttypid, a.atttypmod), a.attnotnull, pg_get_expr(d.adbin, d.adrelid)
         FROM pg_attribute a LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
         WHERE a.attrelid = $1::regclass AND a.attnum > 0 AND NOT a.attisdropped ORDER BY a.attnum",
    )
    .bind(&sql_name)
    .fetch_all(&mut *conn)
    .await
    .map_err(unreachable)?;
    let full = format!("{schema}.{name}");
    let fid = if view {
        let named: Vec<(String, String)> = attributes
            .iter()
            .map(|(n, d, ..)| (n.clone(), d.clone()))
            .collect();
        key_among(&full, &named)?
    } else {
        let keys: Vec<(String, String)> = sqlx::query_as(
            "SELECT a.attname::text, format_type(a.atttypid, a.atttypmod) FROM pg_index i
             JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
             WHERE i.indrelid = $1::regclass AND i.indisprimary",
        )
        .bind(&sql_name)
        .fetch_all(&mut *conn)
        .await
        .map_err(unreachable)?;
        match keys.as_slice() {
            [(k, t)] if matches!(t.as_str(), "smallint" | "integer" | "bigint") => k.clone(),
            _ => {
                return Err(ProviderError::NoTable(format!(
                    "{table} (tam sayı birincil anahtarı yok)"
                )));
            }
        }
    };
    let geometry: Option<(String, String, i32, i32)> = sqlx::query_as(
        "SELECT f_geometry_column::text, type::text, srid, coord_dimension FROM geometry_columns
         WHERE f_table_schema = $1 AND f_table_name = $2 LIMIT 1",
    )
    .bind(&schema)
    .bind(&name)
    .fetch_optional(&mut *conn)
    .await
    .map_err(unreachable)?;
    let gname = geometry.as_ref().map(|g| g.0.clone());
    // An untyped `geometry` column takes any dimension: Z and M optional (docs/adr/0584 §2).
    let untyped = gname
        .as_ref()
        .is_some_and(|g| attributes.iter().any(|(n, d, ..)| n == g && !d.contains('(')));
    let columns = columns_of(attributes, &fid, gname.as_ref());
    let writable: bool = !view
        && sqlx::query_scalar("SELECT has_table_privilege($1::regclass, 'INSERT,UPDATE,DELETE')")
            .bind(&sql_name)
            .fetch_one(&mut *conn)
            .await
            .map_err(unreachable)?;
    let srid = geometry
        .as_ref()
        .and_then(|g| u32::try_from(g.2).ok())
        .filter(|&s| s > 0);
    let geometry = geometry.map(|(column, kind, _, dims)| {
        let mut g = geometry_column(column, &kind, dims);
        if untyped {
            (g.z, g.m) = (2, 2);
        }
        g
    });
    Ok(Shape {
        layer: layer_of(full, geometry, srid, fid, columns),
        srid,
        writable,
        from: sql_name,
        schema: Some(schema),
    })
}

/// A query's shape (docs/adr/0582 §3): its columns as a temporary view says them, in a transaction rolled back (a view
/// is only made, the query does not run; PostgreSQL refuses a view that would write), its geometry the first geometry
/// column, its system the declared one or the first geometry's.
async fn query_shape(conn: &mut PgConnection, table: &str) -> Result<Shape, ProviderError> {
    let (sql, name) = query_of(table)?;
    let attributes: Vec<(String, String, bool, Option<String>)> = {
        let mut tx = conn.begin().await.map_err(unreachable)?;
        tx.execute(sqlx::AssertSqlSafe(format!(
            "CREATE TEMP VIEW kentos_sorgu AS {sql}"
        )))
        .await
        .map_err(|e| ProviderError::Unreadable(e.to_string()))?;
        let found = sqlx::query_as(
            "SELECT a.attname::text, format_type(a.atttypid, a.atttypmod), false, NULL::text FROM pg_attribute a
             WHERE a.attrelid = 'pg_temp.kentos_sorgu'::regclass AND a.attnum > 0 AND NOT a.attisdropped
             ORDER BY a.attnum",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(unreachable)?;
        tx.rollback().await.map_err(unreachable)?;
        found
    };
    let named: Vec<(String, String)> = attributes
        .iter()
        .map(|(n, d, ..)| (n.clone(), d.clone()))
        .collect();
    let fid = key_among(&name, &named)?;
    let from = format!("({sql}) AS q");
    let geometry = attributes
        .iter()
        .find_map(|(n, d, ..)| geometry_decl(d).map(|(kind, srid)| (n.clone(), kind, srid)));
    let srid = match &geometry {
        None => None,
        Some((_, _, Some(s))) => Some(*s),
        Some((g, ..)) => {
            let first: Option<i32> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT ST_SRID({q}) FROM {from} WHERE {q} IS NOT NULL LIMIT 1",
                q = quote(g)
            )))
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| ProviderError::Unreadable(e.to_string()))?;
            first.and_then(|s| u32::try_from(s).ok()).filter(|&s| s > 0)
        }
    };
    let gname = geometry.as_ref().map(|g| g.0.clone());
    let columns = columns_of(attributes, &fid, gname.as_ref());
    let geometry = geometry.map(|(column, kind, _)| geometry_column(column, &kind, 2));
    Ok(Shape {
        layer: layer_of(name, geometry, srid, fid, columns),
        srid,
        writable: false,
        from,
        schema: None,
    })
}

/// What a select reads of a row: its key, its geometry as WKB (or none) and `columns`' values.
fn select_of(s: &Shape, columns: &[gpkg::Column], geometry: bool) -> String {
    let mut select = vec![format!("{}::bigint", quote(&s.layer.fid))];
    select.push(match (&s.layer.geometry, geometry) {
        (Some(g), true) => format!("ST_AsBinary({})", quote(&g.name)),
        _ => "NULL::bytea".into(),
    });
    select.extend(columns.iter().map(read_as));
    select.join(", ")
}

/// A row read by `select_of`.
fn row_of(r: &PgRow, columns: &[gpkg::Column]) -> Result<(gpkg::Row, Option<Vec<u8>>), ProviderError> {
    let fid: i64 = r.try_get(0).map_err(unreachable)?;
    let wkb: Option<Vec<u8>> = r.try_get(1).map_err(unreachable)?;
    Ok((
        gpkg::Row {
            fid,
            geometry: wkb.as_deref().map(gpkg::read_wkb),
            values: values_of(r, columns, 2)?,
        },
        wkb,
    ))
}

/// The box condition of `area` on the shape's geometry (its placeholders from `$first`), none without one.
fn area_where(s: &Shape, area: Option<[f64; 4]>, first: usize) -> Option<String> {
    let (g, _) = (s.layer.geometry.as_ref()?, area?);
    Some(format!(
        "{} && ST_MakeEnvelope(${}, ${}, ${}, ${}, {})",
        quote(&g.name),
        first,
        first + 1,
        first + 2,
        first + 3,
        s.srid.unwrap_or(0)
    ))
}

/// The rows in key order, all of them or those whose box meets `area`, and their revision (the SHA-256 of what was
/// read, the area first; docs/adr/0582 §1). A key seen twice is refused (a view's or a query's).
async fn rows(
    conn: &mut PgConnection,
    s: &Shape,
    area: Option<[f64; 4]>,
) -> Result<(Vec<gpkg::Row>, String), ProviderError> {
    let boxed = area_where(s, area, 1);
    let sql = format!(
        "SELECT {} FROM {} {} ORDER BY {}",
        select_of(s, &s.layer.columns, true),
        s.from,
        boxed.as_ref().map_or(String::new(), |w| format!("WHERE {w}")),
        quote(&s.layer.fid)
    );
    let mut q = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));
    if boxed.is_some()
        && let Some(a) = area
    {
        for v in a {
            q = q.bind(v);
        }
    }
    let found: Vec<PgRow> = q.fetch_all(&mut *conn).await.map_err(unreachable)?;
    let mut digest = Vec::new();
    if let (Some(a), Some(_)) = (area, &boxed) {
        digest.extend_from_slice(b"area");
        for v in a {
            digest.extend_from_slice(&v.to_le_bytes());
        }
    }
    let mut out: Vec<gpkg::Row> = Vec::with_capacity(found.len());
    for r in &found {
        let (row, wkb) = row_of(r, &s.layer.columns)?;
        if out.last().is_some_and(|p| p.fid == row.fid) {
            return Err(ProviderError::KeyRepeats(s.layer.name.clone(), row.fid));
        }
        digest.extend_from_slice(&row.fid.to_le_bytes());
        match &wkb {
            Some(b) => {
                digest.extend_from_slice(&(b.len() as u64).to_le_bytes());
                digest.extend_from_slice(b);
            }
            None => digest.extend_from_slice(&u64::MAX.to_le_bytes()),
        }
        for v in &row.values {
            digest.extend_from_slice(format!("{v:?}").as_bytes());
            digest.push(0);
        }
        out.push(row);
    }
    Ok((out, sha256_hex(&digest)))
}

/// The rows of `keys`, as they now are, in key order (docs/adr/0582 §2).
async fn rows_by_keys(
    conn: &mut PgConnection,
    s: &Shape,
    keys: &[i64],
) -> Result<Vec<gpkg::Row>, ProviderError> {
    let sql = format!(
        "SELECT {} FROM {} WHERE {} = ANY($1) ORDER BY {}",
        select_of(s, &s.layer.columns, true),
        s.from,
        quote(&s.layer.fid),
        quote(&s.layer.fid)
    );
    let found: Vec<PgRow> = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(keys)
        .fetch_all(&mut *conn)
        .await
        .map_err(unreachable)?;
    found
        .iter()
        .map(|r| row_of(r, &s.layer.columns).map(|(row, _)| row))
        .collect()
}

/// Of `keys`, those whose rows the account sees but may not change: seen by a plain select, not by one `FOR UPDATE`
/// (which takes the UPDATE privilege and the row security's UPDATE policies). A key no select sees is not here: the
/// update finds no row and says so. Without the privilege at all, every key.
async fn denied_keys(conn: &mut PgConnection, s: &Shape, keys: &[i64]) -> Result<Vec<i64>, ProviderError> {
    let seen_sql = format!("SELECT {}::bigint FROM {} WHERE {} = ANY($1)", quote(&s.layer.fid), s.from, quote(&s.layer.fid));
    let seen: Vec<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(seen_sql.as_str()))
        .bind(keys)
        .fetch_all(&mut *conn)
        .await
        .map_err(refused)?;
    let mine_sql = format!("{seen_sql} FOR UPDATE");
    let mine: HashSet<i64> = match sqlx::query_scalar(sqlx::AssertSqlSafe(mine_sql.as_str()))
        .bind(keys)
        .fetch_all(&mut *conn)
        .await
    {
        Ok(found) => found.into_iter().collect(),
        Err(e) if is_denied(&e) => HashSet::new(),
        Err(e) => return Err(refused(e)),
    };
    let mut out: Vec<i64> = seen.into_iter().filter(|k| !mine.contains(k)).collect();
    out.sort_unstable();
    Ok(out)
}

/// Whether the database said the account lacks a privilege (SQLSTATE 42501).
fn is_denied(e: &sqlx::Error) -> bool {
    e.as_database_error().and_then(|d| d.code()).as_deref() == Some("42501")
}

/// A write's error: the row `key` the account may not change, or a refusal.
fn denied_or_refused(e: sqlx::Error, key: i64) -> ProviderError {
    if is_denied(&e) { ProviderError::RowsDenied(vec![key]) } else { refused(e) }
}

/// A row's values for `columns`, read from `first` on.
fn values_of(r: &PgRow, columns: &[gpkg::Column], first: usize) -> Result<Vec<Value>, ProviderError> {
    let mut values = Vec::with_capacity(columns.len());
    for (i, c) in columns.iter().enumerate() {
        let at = i + first;
        let v = match c.kind {
            ColumnKind::Integer => r
                .try_get::<Option<i64>, _>(at)
                .map_err(unreachable)?
                .map(Value::Integer),
            ColumnKind::Real => r
                .try_get::<Option<f64>, _>(at)
                .map_err(unreachable)?
                .map(Value::Real),
            ColumnKind::Boolean => r
                .try_get::<Option<bool>, _>(at)
                .map_err(unreachable)?
                .map(|b| Value::Integer(i64::from(b))),
            ColumnKind::Blob(_) => r
                .try_get::<Option<Vec<u8>>, _>(at)
                .map_err(unreachable)?
                .map(Value::Blob),
            _ => r
                .try_get::<Option<String>, _>(at)
                .map_err(unreachable)?
                .map(Value::Text),
        }
        .unwrap_or(Value::Null);
        values.push(v);
    }
    Ok(values)
}

/// Whether the table's geometry column has a spatial index (a box is then found through it, docs/adr/0581 §2).
async fn indexed(conn: &mut PgConnection, s: &Shape) -> Result<bool, ProviderError> {
    let Some(g) = &s.layer.geometry else {
        return Ok(false);
    };
    // A query's rows have no index of their own.
    if s.schema.is_none() {
        return Ok(false);
    }
    let sql_name = s.from.clone();
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_index i JOIN pg_class c ON c.oid = i.indexrelid
         JOIN pg_am am ON am.oid = c.relam
         JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
         WHERE i.indrelid = $1::regclass AND am.amname IN ('gist', 'spgist', 'brin') AND a.attname = $2)",
    )
    .bind(&sql_name)
    .bind(&g.name)
    .fetch_one(&mut *conn)
    .await
    .map_err(unreachable)
}

/// The rows a question asks for, a part at a time in key order (docs/adr/0581 §2): the box through `&&` (the index
/// when there is one), the condition as the application sent it, the key after the last part's.
async fn scan(
    conn: &mut PgConnection,
    s: &Shape,
    q: &ProviderScan<'_>,
    f: &mut dyn FnMut(Vec<gpkg::Row>) -> bool,
) -> Result<u64, ProviderError> {
    let sql_name = s.from.clone();
    let columns: Vec<gpkg::Column> = match q.columns {
        None => s.layer.columns.clone(),
        Some(list) => list
            .iter()
            .map(|&i| {
                s.layer
                    .columns
                    .get(i)
                    .cloned()
                    .ok_or_else(|| refused(format!("{}: {i}. sütun yok", s.layer.name)))
            })
            .collect::<Result<_, _>>()?,
    };
    let key = quote(&s.layer.fid);
    let mut select = vec![format!("{key}::bigint")];
    select.push(match (&s.layer.geometry, q.geometry) {
        (Some(g), true) => format!("ST_AsBinary({})", quote(&g.name)),
        _ => "NULL::bytea".into(),
    });
    select.extend(columns.iter().map(read_as));
    let params = q.filter.map_or(&[][..], |(_, p)| p);
    let mut wheres = Vec::new();
    if let Some((sql, _)) = q.filter {
        wheres.push(format!("({sql})"));
    }
    let mut next = params.len();
    let boxed = match (&s.layer.geometry, q.bbox) {
        (Some(g), Some(_)) => {
            let srid = s.srid.unwrap_or(0);
            wheres.push(format!(
                "{} && ST_MakeEnvelope(${}, ${}, ${}, ${}, {srid})",
                quote(&g.name),
                next + 1,
                next + 2,
                next + 3,
                next + 4
            ));
            next += 4;
            q.bbox
        }
        _ => None,
    };
    // The order's column among those read (docs/adr/0591 §1): each part starts after the last row's value and key.
    let ordered = match &q.order {
        None => None,
        Some(o) => {
            let at = q
                .columns
                .map_or(Some(o.column), |list| list.iter().position(|&i| i == o.column))
                .ok_or_else(|| refused(format!("{}. sütun sıralanıyor ama okunmuyor", o.column)))?;
            let c = s
                .layer
                .columns
                .get(o.column)
                .ok_or_else(|| refused(format!("{}: {}. sütun yok", s.layer.name, o.column)))?;
            Some((at, quote(&c.name), c.decl.clone(), o.descending))
        }
    };
    let mut place: Option<(gpkg::Value, i64)> = q.order.as_ref().and_then(|o| o.after.clone());
    let mut after = q.after.unwrap_or(i64::MIN);
    let mut handed = 0u64;
    loop {
        let mut conditions = wheres.clone();
        let mut extra: Vec<Option<String>> = Vec::new();
        match &ordered {
            None => conditions.push(format!("{key} > ${}", next + 1)),
            Some((_, c, decl, d)) => match &place {
                None => {}
                Some((gpkg::Value::Null, _)) => {
                    conditions.push(format!("({c} IS NULL AND {key} > ${})", next + 1));
                }
                Some((value, _)) => {
                    let past = if *d { "<" } else { ">" };
                    let v = format!("CAST(${} AS {decl})", next + 2);
                    conditions.push(format!(
                        "({c} {past} {v} OR ({c} = {v} AND {key} > ${}) OR {c} IS NULL)",
                        next + 1
                    ));
                    extra.push(Some(match value {
                        gpkg::Value::Integer(i) => i.to_string(),
                        gpkg::Value::Real(x) => x.to_string(),
                        gpkg::Value::Text(t) => t.clone(),
                        _ => String::new(),
                    }));
                }
            },
        }
        let order_by = match &ordered {
            None => key.clone(),
            Some((_, c, _, d)) => format!("{c} {} NULLS LAST, {key}", if *d { "DESC" } else { "ASC" }),
        };
        let sql = format!(
            "SELECT {} FROM {sql_name}{}{} ORDER BY {order_by} LIMIT {}",
            select.join(", "),
            if conditions.is_empty() { "" } else { " WHERE " },
            conditions.join(" AND "),
            q.chunk.max(1)
        );
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));
        for p in params {
            query = match p {
                Param::Number(x) => query.bind(*x),
                Param::Text(t) => query.bind(t.clone()),
            };
        }
        if let Some(b) = boxed {
            for v in b {
                query = query.bind(v);
            }
        }
        let last_key = match (&ordered, &place) {
            (None, _) => Some(after),
            (Some(_), Some((_, k))) => Some(*k),
            (Some(_), None) => None,
        };
        if let Some(k) = last_key {
            query = query.bind(k);
        }
        for v in extra.into_iter().flatten() {
            query = query.bind(v);
        }
        let found: Vec<PgRow> = query.fetch_all(&mut *conn).await.map_err(unreachable)?;
        let done = found.len() < q.chunk.max(1);
        let mut part = Vec::with_capacity(found.len());
        for r in &found {
            let fid: i64 = r.try_get(0).map_err(unreachable)?;
            let wkb: Option<Vec<u8>> = r.try_get(1).map_err(unreachable)?;
            part.push(gpkg::Row {
                fid,
                geometry: wkb.as_deref().map(gpkg::read_wkb),
                values: values_of(r, &columns, 2)?,
            });
            after = fid;
        }
        if let (Some((at, ..)), Some(r)) = (&ordered, part.last()) {
            place = Some((r.values[*at].clone(), r.fid));
        }
        handed += part.len() as u64;
        if part.is_empty() || !f(part) || done {
            return Ok(handed);
        }
    }
}

async fn uids(
    conn: &mut PgConnection,
    schema: &str,
    table: &str,
) -> Result<HashMap<i64, String>, ProviderError> {
    let side = format!("{}.kentos_uids", quote(schema));
    let exists: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
        .bind(&side)
        .fetch_one(&mut *conn)
        .await
        .map_err(unreachable)?;
    if exists.is_none() {
        return Ok(HashMap::new());
    }
    let found: Vec<(i64, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT pk, uid::text FROM {side} WHERE table_name = $1"
    )))
    .bind(table)
    .fetch_all(&mut *conn)
    .await
    .map_err(unreachable)?;
    Ok(found.into_iter().collect())
}

/// A value bound for a column: text cast to the column's type, or bytes.
enum Bound {
    Text(Option<String>),
    Bytes(Option<Vec<u8>>),
}

fn bound(v: &Value) -> Bound {
    match v {
        Value::Null => Bound::Text(None),
        Value::Integer(i) => Bound::Text(Some(i.to_string())),
        Value::Real(r) => Bound::Text(Some(format!("{r:?}"))),
        Value::Text(t) => Bound::Text(Some(t.clone())),
        Value::Blob(b) => Bound::Bytes(Some(b.clone())),
    }
}

/// A transaction that cannot change the database: a view's and a query's rows are read in one (docs/adr/0582 §3).
async fn read_only(
    conn: &mut PgConnection,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, ProviderError> {
    let mut tx = conn.begin().await.map_err(unreachable)?;
    tx.execute("SET TRANSACTION READ ONLY")
        .await
        .map_err(unreachable)?;
    Ok(tx)
}

/// The lasting ids KentOS keeps for the rows of `s` (none for a query).
async fn uids_of(conn: &mut PgConnection, s: &Shape) -> Result<HashMap<i64, String>, ProviderError> {
    match &s.schema {
        Some(schema) => uids(conn, schema, &s.layer.name).await,
        None => Ok(HashMap::new()),
    }
}

/// One table's edits inside an open transaction (docs/adr/0582 §2, 0584 §1): the table locked, the rows the edits touch
/// checked (or the area's revision), the edits written, the area's revision before and after.
async fn edit_in(
    tx: &mut PgConnection,
    table: &str,
    area: Option<[f64; 4]>,
    revision: &str,
    check: Option<&mut RowCheck<'_>>,
    edits: &gpkg::Edits,
) -> Result<Edited, ProviderError> {
    let s = shape(&mut *tx, table).await?;
    if !s.writable {
        return Err(refused(format!("{} salt okunur", s.layer.name)));
    }
    let sql_name = s.from.clone();
    let full = s.layer.name.clone();
    let schema = s.schema.clone().unwrap_or_default();
    tx.execute(sqlx::AssertSqlSafe(format!("LOCK TABLE {sql_name} IN SHARE ROW EXCLUSIVE MODE")))
        .await
        .map_err(refused)?;
    // The rows the edits touch, as they now are, against the layer's base (docs/adr/0582 §2).
    let checked = check.is_some();
    if let Some(check) = check {
        let mut touched: Vec<i64> = edits.update.iter().map(|(fid, _)| *fid).collect();
        touched.extend(edits.delete.iter().copied());
        touched.sort_unstable();
        touched.dedup();
        if !touched.is_empty() {
            let now = rows_by_keys(&mut *tx, &s, &touched).await?;
            if let Err(changed) = check(&s.layer, &now) {
                return Err(ProviderError::RowsChanged(changed));
            }
        }
    }
    let (_, before) = rows(&mut *tx, &s, area).await?;
    if (!checked || !revision.is_empty()) && before != revision {
        return Err(ProviderError::Changed);
    }
    // The rows the edits change that the account sees but may not change (docs/adr/0585 §2): refused before a row is
    // written, so a strict write is all or nothing over privileges and row security too.
    let mut keys: Vec<i64> = edits.update.iter().map(|(fid, _)| *fid).collect();
    keys.extend(edits.delete.iter().copied());
    keys.sort_unstable();
    keys.dedup();
    if !keys.is_empty() {
        let denied = denied_keys(&mut *tx, &s, &keys).await?;
        if !denied.is_empty() {
            return Err(ProviderError::RowsDenied(denied));
        }
    }
    for c in &edits.add_columns {
        tx.execute(sqlx::AssertSqlSafe(format!("ALTER TABLE {sql_name} ADD COLUMN {} {}", quote(&c.name), pg_type(c))))
            .await
            .map_err(refused)?;
    }
    // The columns written, in the rows' order: the table's, then those added.
    let mut columns: Vec<(String, String, bool)> = s
        .layer
        .columns
        .iter()
        .map(|c| (quote(&c.name), c.decl.clone(), matches!(c.kind, ColumnKind::Blob(_))))
        .collect();
    columns.extend(
        edits
            .add_columns
            .iter()
            .map(|c| (quote(&c.name), pg_type(c), matches!(c.kind, ColumnKind::Blob(_)))),
    );
    let srid = s.srid.unwrap_or(0);
    let geom = s.layer.geometry.as_ref().map(|g| quote(&g.name));
    let mut targets: Vec<String> = Vec::new();
    let mut slots: Vec<String> = Vec::new();
    let mut n = 0usize;
    if let Some(g) = &geom {
        n += 1;
        targets.push(g.clone());
        slots.push(format!("ST_SetSRID(ST_GeomFromWKB(${n}), {srid})"));
    }
    for (q, decl, bytes) in &columns {
        n += 1;
        targets.push(q.clone());
        slots.push(if *bytes { format!("${n}") } else { format!("CAST(${n} AS {decl})") });
    }
    let width = columns.len();
    let bind_row = |row: &gpkg::NewRow| -> Result<(Option<Vec<u8>>, Vec<Bound>), ProviderError> {
        if row.values.len() != width {
            return Err(refused(format!("satırda {} değer var, {width} sütun bekleniyor", row.values.len())));
        }
        Ok((
            row.geometry.as_ref().map(gpkg::write_wkb),
            row.values.iter().map(bound).collect(),
        ))
    };
    macro_rules! with_values {
        ($q:expr, $wkb:expr, $vals:expr) => {{
            let mut q = $q;
            if geom.is_some() {
                q = q.bind($wkb);
            }
            for v in $vals {
                q = match v {
                    Bound::Text(t) => q.bind(t),
                    Bound::Bytes(b) => q.bind(b),
                };
            }
            q
        }};
    }
    if !edits.update.is_empty() && !targets.is_empty() {
        let sets: Vec<String> = targets.iter().zip(&slots).map(|(t, s)| format!("{t} = {s}")).collect();
        let sql = format!(
            "UPDATE {sql_name} SET {} WHERE {} = ${}",
            sets.join(", "),
            quote(&s.layer.fid),
            n + 1
        );
        for (fid, row) in &edits.update {
            let (wkb, vals) = bind_row(row)?;
            let q = with_values!(sqlx::query(sqlx::AssertSqlSafe(sql.as_str())), wkb, vals).bind(*fid);
            let done = q.execute(&mut *tx).await.map_err(|e| denied_or_refused(e, *fid))?;
            if done.rows_affected() != 1 {
                return Err(refused(format!("{full} tablosunda {fid} anahtarlı satır yok")));
            }
        }
    }
    let mut inserted = Vec::with_capacity(edits.insert.len());
    if !edits.insert.is_empty() {
        let sql = if targets.is_empty() {
            format!("INSERT INTO {sql_name} DEFAULT VALUES RETURNING {}::bigint", quote(&s.layer.fid))
        } else {
            format!(
                "INSERT INTO {sql_name} ({}) VALUES ({}) RETURNING {}::bigint",
                targets.join(", "),
                slots.join(", "),
                quote(&s.layer.fid)
            )
        };
        for (_, row) in &edits.insert {
            let (wkb, vals) = bind_row(row)?;
            let q = with_values!(sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(sql.as_str())), wkb, vals);
            inserted.push(q.fetch_one(&mut *tx).await.map_err(refused)?);
        }
    }
    if !edits.delete.is_empty() {
        let sql = format!("DELETE FROM {sql_name} WHERE {} = $1", quote(&s.layer.fid));
        for fid in &edits.delete {
            sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
                .bind(*fid)
                .execute(&mut *tx)
                .await
                .map_err(|e| denied_or_refused(e, *fid))?;
        }
    }
    if !edits.insert.is_empty() || !edits.delete.is_empty() {
        let side = format!("{}.kentos_uids", quote(&schema));
        tx.execute(sqlx::AssertSqlSafe(format!(
            "CREATE TABLE IF NOT EXISTS {side} (table_name text NOT NULL, pk bigint NOT NULL, uid uuid NOT NULL,
             PRIMARY KEY (table_name, pk))"
        )))
        .await
        .map_err(|e| refused(format!("kalıcı kimliklerin tablosu ({side}) açılamadı: {e}")))?;
        let put = format!(
            "INSERT INTO {side} VALUES ($1, $2, CAST($3 AS uuid)) ON CONFLICT (table_name, pk) DO UPDATE SET uid = EXCLUDED.uid"
        );
        for (pk, (uid, _)) in inserted.iter().zip(&edits.insert) {
            sqlx::query(sqlx::AssertSqlSafe(put.as_str()))
                .bind(&full)
                .bind(*pk)
                .bind(uid)
                .execute(&mut *tx)
                .await
                .map_err(refused)?;
        }
        let gone = format!("DELETE FROM {side} WHERE table_name = $1 AND pk = $2");
        for fid in &edits.delete {
            sqlx::query(sqlx::AssertSqlSafe(gone.as_str())).bind(&full).bind(*fid).execute(&mut *tx).await.map_err(refused)?;
        }
    }
    let s = shape(&mut *tx, table).await?;
    let (_, after) = rows(&mut *tx, &s, area).await?;
    Ok(Edited {
        inserted,
        before,
        after,
    })
}

impl TableProvider for PostGis {
    fn tables(&self, address: &str) -> Result<Vec<ProviderTable>, ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let names: Vec<(String, String)> = sqlx::query_as(
                "SELECT n.nspname::text, c.relname::text FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE c.relkind IN ('r', 'p', 'v', 'm') AND n.nspname NOT IN ('pg_catalog', 'information_schema', 'topology', 'tiger')
                   AND n.nspname NOT LIKE 'pg_toast%'
                   AND c.relname NOT IN ('spatial_ref_sys', 'kentos_uids', 'geometry_columns', 'geography_columns')
                 ORDER BY n.nspname, c.relname",
            )
            .fetch_all(&mut conn)
            .await
            .map_err(unreachable)?;
            let mut out = Vec::new();
            for (schema, name) in names {
                let table = format!("{schema}.{name}");
                // Tables without a single integer key and views without one of KEYS are not read, so not listed.
                let Ok(s) = shape(&mut conn, &table).await else { continue };
                let rows: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {}", s.from)))
                    .fetch_one(&mut conn)
                    .await
                    .map_err(unreachable)?;
                let g = s.layer.geometry.as_ref();
                out.push(ProviderTable {
                    data: if g.is_some() { "features" } else { "attributes" }.into(),
                    rows: rows.max(0) as u64,
                    geometry: g.map(|g| g.type_name.clone()),
                    srid: s.srid,
                    z: g.is_some_and(|g| g.z != 0),
                    m: g.is_some_and(|g| g.m != 0),
                    columns: s.layer.columns.clone(),
                    writable: s.writable,
                    name: table,
                });
            }
            let _ = conn.close().await;
            Ok(out)
        })
    }

    fn read(
        &self,
        address: &str,
        table: &str,
        area: Option<[f64; 4]>,
    ) -> Result<ProviderRead, ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let s = shape(&mut conn, table).await?;
            let uids = uids_of(&mut conn, &s).await?;
            let (rows, revision) = {
                let mut tx = read_only(&mut conn).await?;
                let read = rows(&mut tx, &s, area).await?;
                tx.rollback().await.map_err(unreachable)?;
                read
            };
            let _ = conn.close().await;
            Ok(ProviderRead {
                layer: s.layer,
                rows,
                uids,
                srid: s.srid,
                writable: s.writable,
                revision,
            })
        })
    }

    fn describe(&self, address: &str, table: &str) -> Result<ProviderShape, ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let mut s = shape(&mut conn, table).await?;
            s.layer.indexed = indexed(&mut conn, &s).await?;
            let uids = uids_of(&mut conn, &s).await?;
            let _ = conn.close().await;
            Ok(ProviderShape {
                layer: s.layer,
                srid: s.srid,
                uids,
            })
        })
    }

    fn scan(
        &self,
        address: &str,
        table: &str,
        q: &ProviderScan<'_>,
        f: &mut dyn FnMut(Vec<gpkg::Row>) -> bool,
    ) -> Result<u64, ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let s = shape(&mut conn, table).await?;
            let handed = {
                let mut tx = read_only(&mut conn).await?;
                let handed = scan(&mut tx, &s, q, f).await?;
                tx.rollback().await.map_err(unreachable)?;
                handed
            };
            let _ = conn.close().await;
            Ok(handed)
        })
    }

    fn edit(
        &self,
        address: &str,
        table: &str,
        area: Option<[f64; 4]>,
        revision: &str,
        check: Option<&mut RowCheck<'_>>,
        edits: &gpkg::Edits,
    ) -> Result<Edited, ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let mut tx = conn.begin().await.map_err(unreachable)?;
            let edited = edit_in(&mut tx, table, area, revision, check, edits).await?;
            tx.commit().await.map_err(refused)?;
            let _ = conn.close().await;
            Ok(edited)
        })
    }

    fn edit_many(
        &self,
        address: &str,
        items: Vec<TableEdit<'_, '_>>,
    ) -> Result<Vec<Edited>, (usize, ProviderError)> {
        self.edit_many_mapped(address, items, &[])
    }

    fn edit_many_mapped(
        &self,
        address: &str,
        items: Vec<TableEdit<'_, '_>>,
        maps: &[MapWrite],
    ) -> Result<Vec<Edited>, (usize, ProviderError)> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await.map_err(|e| (0, e))?;
            let mut tx = conn.begin().await.map_err(|e| (0, unreachable(e)))?;
            // Every table locked first, in its name's order, so two writers of the same tables cannot wait on each
            // other (docs/adr/0584 §1); the junction tables of their references with them (docs/adr/0590 §3).
            let mut order: Vec<(String, usize)> = Vec::with_capacity(items.len() + maps.len());
            for (i, t) in items.iter().enumerate() {
                let s = shape(&mut tx, t.table).await.map_err(|e| (i, e))?;
                if !s.writable {
                    return Err((i, refused(format!("{} salt okunur", s.layer.name))));
                }
                order.push((s.from.clone(), i));
            }
            for (m, w) in maps.iter().enumerate() {
                order.push((qualified(&w.mapping).0, items.len() + m));
            }
            order.sort();
            order.dedup_by(|a, b| a.0 == b.0);
            for (from, i) in &order {
                tx.execute(sqlx::AssertSqlSafe(format!("LOCK TABLE {from} IN SHARE ROW EXCLUSIVE MODE")))
                    .await
                    .map_err(|e| (*i, refused(e)))?;
            }
            let n = items.len();
            let mut out = Vec::with_capacity(n);
            for (i, t) in items.into_iter().enumerate() {
                out.push(edit_in(&mut tx, t.table, t.area, t.revision, t.check, t.edits).await.map_err(|e| (i, e))?);
            }
            for (m, w) in maps.iter().enumerate() {
                junctions::write(&mut tx, w, &out).await.map_err(|e| (n + m, e))?;
            }
            tx.commit().await.map_err(|e| (0, refused(e)))?;
            let _ = conn.close().await;
            Ok(out)
        })
    }

    fn relation(&self, address: &str, child: &str, parent: &str) -> Result<DbRelation, ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let found = junctions::relation(&mut conn, child, parent).await;
            let _ = conn.close().await;
            found
        })
    }

    fn mapping_rows(&self, address: &str, mapping: &str) -> Result<Vec<(i64, i64)>, ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let rows = junctions::rows(&mut conn, mapping).await;
            let _ = conn.close().await;
            rows
        })
    }

    fn map(
        &self,
        address: &str,
        child: &str,
        parent: &str,
        mapping: &str,
        pairs: &[(i64, i64)],
    ) -> Result<(), ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let mut tx = conn.begin().await.map_err(unreachable)?;
            junctions::map(&mut tx, child, parent, mapping, pairs).await?;
            tx.commit().await.map_err(refused)?;
            let _ = conn.close().await;
            Ok(())
        })
    }

    fn unmap(&self, address: &str, mapping: &str) -> Result<(), ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let mut tx = conn.begin().await.map_err(unreachable)?;
            junctions::unmap(&mut tx, mapping).await?;
            tx.commit().await.map_err(refused)?;
            let _ = conn.close().await;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_read_as_kinds_and_new_columns_get_types() {
        assert_eq!(kind_of("integer"), ColumnKind::Integer);
        assert_eq!(kind_of("character varying(40)"), ColumnKind::Text(Some(40)));
        assert_eq!(
            kind_of("numeric(12,2)"),
            ColumnKind::Other("numeric(12,2)".into())
        );
        assert_eq!(kind_of("double precision"), ColumnKind::Real);
        let c = gpkg::Column {
            name: "durum".into(),
            decl: String::new(),
            kind: ColumnKind::Text(None),
            not_null: false,
            default: None,
        };
        assert_eq!(pg_type(&c), "text");
        assert_eq!(qualified("kadastro.parsel").0, "\"kadastro\".\"parsel\"");
        assert_eq!(qualified("parsel").1, "public");
    }
}

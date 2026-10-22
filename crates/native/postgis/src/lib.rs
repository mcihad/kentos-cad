//! PostGIS as a bound layer's source (docs/adr/0505, plan/gis.md CBS-04): the product commands' `TableProvider` over
//! sqlx, TLS with rustls and the system's root certificates. A table is read whole, as a GeoPackage table is
//! (`kentos_geopackage`'s layer, rows and values; geometries as ISO WKB), its revision the SHA-256 of its rows in key
//! order. An edit runs in one transaction: the table locked against writers, its revision checked again, the columns
//! added, the rows updated, inserted and deleted, the lasting ids of inserted rows kept in the schema's `kentos_uids`.
//! Passwords never come from the project: the address carries none; the environment's `PGPASSWORD` or `~/.pgpass`.
//!
//! Tables with a single integer primary key are listed; a feature table is one in `geometry_columns`.

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use kentos_contracts::encoding::sha256_hex;
use kentos_geopackage::{self as gpkg, ColumnKind, DataType, Value};
use kentos_native_application::provider::{
    Param, ProviderError, ProviderRead, ProviderScan, ProviderShape, ProviderTable, TableProvider,
    register_postgis,
};
use sqlx::postgres::{PgConnectOptions, PgConnection, PgRow};
use sqlx::{Connection, Executor, Row};

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

/// A table's shape as the catalogue says it.
struct Shape {
    layer: gpkg::Layer,
    srid: Option<u32>,
    writable: bool,
}

async fn shape(conn: &mut PgConnection, table: &str) -> Result<Shape, ProviderError> {
    let (sql_name, schema, name) = qualified(table);
    let exists: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
        .bind(&sql_name)
        .fetch_one(&mut *conn)
        .await
        .map_err(unreachable)?;
    if exists.is_none() {
        return Err(ProviderError::NoTable(table.to_owned()));
    }
    let keys: Vec<(String, String)> = sqlx::query_as(
        "SELECT a.attname::text, format_type(a.atttypid, a.atttypmod) FROM pg_index i
         JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
         WHERE i.indrelid = $1::regclass AND i.indisprimary",
    )
    .bind(&sql_name)
    .fetch_all(&mut *conn)
    .await
    .map_err(unreachable)?;
    let fid = match keys.as_slice() {
        [(k, t)] if matches!(t.as_str(), "smallint" | "integer" | "bigint") => k.clone(),
        _ => {
            return Err(ProviderError::NoTable(format!(
                "{table} (tam sayı birincil anahtarı yok)"
            )));
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
    let attributes: Vec<(String, String, bool, Option<String>)> = sqlx::query_as(
        "SELECT a.attname::text, format_type(a.atttypid, a.atttypmod), a.attnotnull, pg_get_expr(d.adbin, d.adrelid)
         FROM pg_attribute a LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
         WHERE a.attrelid = $1::regclass AND a.attnum > 0 AND NOT a.attisdropped ORDER BY a.attnum",
    )
    .bind(&sql_name)
    .fetch_all(&mut *conn)
    .await
    .map_err(unreachable)?;
    let gname = geometry.as_ref().map(|g| g.0.clone());
    let columns = attributes
        .into_iter()
        .filter(|(n, ..)| *n != fid && Some(n) != gname.as_ref())
        .map(|(name, decl, not_null, default)| gpkg::Column {
            kind: kind_of(&decl),
            name,
            decl,
            not_null,
            default,
        })
        .collect();
    let writable: bool =
        sqlx::query_scalar("SELECT has_table_privilege($1::regclass, 'INSERT,UPDATE,DELETE')")
            .bind(&sql_name)
            .fetch_one(&mut *conn)
            .await
            .map_err(unreachable)?;
    let srid = geometry
        .as_ref()
        .and_then(|g| u32::try_from(g.2).ok())
        .filter(|&s| s > 0);
    let geometry = geometry.map(|(column, kind, _, dims)| {
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
            name: column,
            type_name: base,
            z,
            m,
        }
    });
    Ok(Shape {
        layer: gpkg::Layer {
            name: format!("{schema}.{name}"),
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
        },
        srid,
        writable,
    })
}

/// Every row in key order, and the table's revision (the SHA-256 of what was read).
async fn rows(
    conn: &mut PgConnection,
    s: &Shape,
) -> Result<(Vec<gpkg::Row>, String), ProviderError> {
    let (sql_name, ..) = qualified(&s.layer.name);
    let mut select = vec![format!("{}::bigint", quote(&s.layer.fid))];
    select.push(match &s.layer.geometry {
        Some(g) => format!("ST_AsBinary({})", quote(&g.name)),
        None => "NULL::bytea".into(),
    });
    select.extend(s.layer.columns.iter().map(read_as));
    let sql = format!(
        "SELECT {} FROM {sql_name} ORDER BY {}",
        select.join(", "),
        quote(&s.layer.fid)
    );
    let found: Vec<PgRow> = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .fetch_all(&mut *conn)
        .await
        .map_err(unreachable)?;
    let mut digest = Vec::new();
    let mut out = Vec::with_capacity(found.len());
    for r in &found {
        let fid: i64 = r.try_get(0).map_err(unreachable)?;
        digest.extend_from_slice(&fid.to_le_bytes());
        let wkb: Option<Vec<u8>> = r.try_get(1).map_err(unreachable)?;
        let geometry = match &wkb {
            Some(b) => {
                digest.extend_from_slice(&(b.len() as u64).to_le_bytes());
                digest.extend_from_slice(b);
                Some(gpkg::read_wkb(b))
            }
            None => {
                digest.extend_from_slice(&u64::MAX.to_le_bytes());
                None
            }
        };
        let values = values_of(r, &s.layer.columns, 2)?;
        for v in &values {
            digest.extend_from_slice(format!("{v:?}").as_bytes());
            digest.push(0);
        }
        out.push(gpkg::Row {
            fid,
            geometry,
            values,
        });
    }
    Ok((out, sha256_hex(&digest)))
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
    let (sql_name, ..) = qualified(&s.layer.name);
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
    let (sql_name, ..) = qualified(&s.layer.name);
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
    wheres.push(format!("{key} > ${}", next + 1));
    let sql = format!(
        "SELECT {} FROM {sql_name} WHERE {} ORDER BY {key} LIMIT {}",
        select.join(", "),
        wheres.join(" AND "),
        q.chunk.max(1)
    );
    let mut after = q.after.unwrap_or(i64::MIN);
    let mut handed = 0u64;
    loop {
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
        let found: Vec<PgRow> = query
            .bind(after)
            .fetch_all(&mut *conn)
            .await
            .map_err(unreachable)?;
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

impl TableProvider for PostGis {
    fn tables(&self, address: &str) -> Result<Vec<ProviderTable>, ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let names: Vec<(String, String)> = sqlx::query_as(
                "SELECT n.nspname::text, c.relname::text FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE c.relkind IN ('r', 'p') AND n.nspname NOT IN ('pg_catalog', 'information_schema', 'topology', 'tiger')
                   AND n.nspname NOT LIKE 'pg_toast%' AND c.relname NOT IN ('spatial_ref_sys', 'kentos_uids')
                 ORDER BY n.nspname, c.relname",
            )
            .fetch_all(&mut conn)
            .await
            .map_err(unreachable)?;
            let mut out = Vec::new();
            for (schema, name) in names {
                let table = format!("{schema}.{name}");
                // Tables without a single integer key are not read, so not listed.
                let Ok(s) = shape(&mut conn, &table).await else { continue };
                let (sql_name, ..) = qualified(&table);
                let rows: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {sql_name}")))
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

    fn read(&self, address: &str, table: &str) -> Result<ProviderRead, ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let s = shape(&mut conn, table).await?;
            let (rows, revision) = rows(&mut conn, &s).await?;
            let (_, schema, name) = qualified(table);
            let uids = uids(&mut conn, &schema, &format!("{schema}.{name}")).await?;
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
            let (_, schema, name) = qualified(table);
            let uids = uids(&mut conn, &schema, &format!("{schema}.{name}")).await?;
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
            let handed = scan(&mut conn, &s, q, f).await?;
            let _ = conn.close().await;
            Ok(handed)
        })
    }

    fn edit(
        &self,
        address: &str,
        table: &str,
        revision: &str,
        edits: &gpkg::Edits,
    ) -> Result<(Vec<i64>, String), ProviderError> {
        self.runtime.block_on(async {
            let mut conn = connect(address).await?;
            let mut tx = conn.begin().await.map_err(unreachable)?;
            let (sql_name, schema, name) = qualified(table);
            let full = format!("{schema}.{name}");
            tx.execute(sqlx::AssertSqlSafe(format!("LOCK TABLE {sql_name} IN SHARE ROW EXCLUSIVE MODE")))
                .await
                .map_err(refused)?;
            let s = shape(&mut tx, table).await?;
            let (_, now) = rows(&mut tx, &s).await?;
            if now != revision {
                return Err(ProviderError::Changed);
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
                    let done = q.execute(&mut *tx).await.map_err(refused)?;
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
                    sqlx::query(sqlx::AssertSqlSafe(sql.as_str())).bind(*fid).execute(&mut *tx).await.map_err(refused)?;
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
            let s = shape(&mut tx, table).await?;
            let (_, after) = rows(&mut tx, &s).await?;
            tx.commit().await.map_err(refused)?;
            let _ = conn.close().await;
            Ok((inserted, after))
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

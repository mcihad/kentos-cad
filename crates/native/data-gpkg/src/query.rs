//! A query read as a table of a GeoPackage (docs/adr/9113 §2): its shape found from the statement, without reading
//! more than its first row.
//!
//! - The columns are the statement's, each of the kind its declared type says (a column of a table keeps the table's
//!   type); a computed column has none, and its first value's type tells (an integer, a real, a text, a blob).
//! - The key is the column the user chose; it must be an integer. The geometry is the column the user chose, or none;
//!   its system, type and dimensions are those of its first value (a GeoPackage geometry's header says its system).
//! - It is read as `(<query>)` on the source's read connections, which are opened read-only: the query cannot change the
//!   file. It has no spatial index: the box is asked of each geometry's own (`ST_MinX` …). It is never writable.

use kentos_data::{DataError, QueryTable};
use kentos_geopackage::{
    Column, ColumnKind, DataType, GeoPackage, GeometryColumn, Layer, quote, read_blob,
};
use rusqlite::types::ValueRef;

fn unreadable(e: rusqlite::Error) -> DataError {
    DataError::Invalid(format!("Sorgu okunamadı: {e}"))
}

/// A value's type as a declared type: what a computed column is read as.
fn declared_by(v: ValueRef<'_>) -> Option<&'static str> {
    match v {
        ValueRef::Integer(_) => Some("INTEGER"),
        ValueRef::Real(_) => Some("REAL"),
        ValueRef::Text(_) => Some("TEXT"),
        ValueRef::Blob(_) => Some("BLOB"),
        ValueRef::Null => None,
    }
}

/// The query's columns with their declared types (empty: a computed column's), as a temporary view of it says them:
/// the view is made in the connection's own temporary schema, which a read-only connection may write, and dropped.
fn declared(c: &rusqlite::Connection, sql: &str) -> Result<Vec<(String, String)>, DataError> {
    c.execute_batch(&format!(
        "DROP VIEW IF EXISTS temp.kentos_sorgu; CREATE TEMP VIEW kentos_sorgu AS {sql}"
    ))
    .map_err(unreadable)?;
    let read = c
        .prepare("SELECT name, type FROM pragma_table_info('kentos_sorgu', 'temp') ORDER BY cid")
        .and_then(|mut st| {
            st.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                ))
            })?
            .collect::<Result<Vec<_>, _>>()
        });
    let _ = c.execute_batch("DROP VIEW IF EXISTS temp.kentos_sorgu");
    read.map_err(unreadable)
}

/// A declared type that names a geometry (`gpkg_geometry_columns`' type names).
fn names_geometry(decl: &str) -> bool {
    let up = decl.trim().to_ascii_uppercase();
    matches!(
        up.as_str(),
        "GEOMETRY"
            | "POINT"
            | "LINESTRING"
            | "POLYGON"
            | "MULTIPOINT"
            | "MULTILINESTRING"
            | "MULTIPOLYGON"
            | "GEOMETRYCOLLECTION"
            | "CIRCULARSTRING"
            | "COMPOUNDCURVE"
            | "CURVEPOLYGON"
            | "MULTICURVE"
            | "MULTISURFACE"
    )
}

/// The query's columns, each with whether it is an integer or a geometry: its declared type, else its first value.
pub(crate) fn columns(
    gp: &GeoPackage,
    sql: &str,
) -> Result<Vec<kentos_data::QueryColumn>, DataError> {
    kentos_data::query::check(sql).map_err(DataError::Invalid)?;
    let c = gp.connection();
    let declared = declared(c, sql)?;
    let mut stmt = c
        .prepare(&format!("SELECT * FROM ({sql}) LIMIT 1"))
        .map_err(unreadable)?;
    let mut rows = stmt.query([]).map_err(unreadable)?;
    let first = rows.next().map_err(unreadable)?;
    Ok(declared
        .iter()
        .enumerate()
        .map(|(i, (name, decl))| {
            let value = first.and_then(|r| r.get_ref(i).ok());
            kentos_data::QueryColumn {
                name: name.clone(),
                integer: decl.to_ascii_uppercase().contains("INT")
                    || matches!(value, Some(ValueRef::Integer(_))),
                geometry: names_geometry(decl)
                    || matches!(value, Some(ValueRef::Blob(b)) if read_blob(b).is_ok()),
            }
        })
        .collect())
}

/// The query's layer, as the catalog would say a table's (`table` its text).
pub(crate) fn layer(gp: &GeoPackage, table: &str, q: &QueryTable) -> Result<Layer, DataError> {
    kentos_data::query::check(&q.sql).map_err(DataError::Invalid)?;
    let c = gp.connection();
    let declared = declared(c, &q.sql)?;
    let mut stmt = c
        .prepare(&format!("SELECT * FROM ({}) LIMIT 1", q.sql))
        .map_err(unreadable)?;
    let mut rows = stmt.query([]).map_err(unreadable)?;
    let first = rows.next().map_err(unreadable)?;
    let mut columns = Vec::new();
    let mut key_seen = false;
    let mut geometry_seen = false;
    for (i, (name, decl)) in declared.iter().enumerate() {
        let value = first.and_then(|r| r.get_ref(i).ok());
        let decl = if decl.is_empty() {
            value.and_then(declared_by).unwrap_or("TEXT").to_owned()
        } else {
            decl.clone()
        };
        if *name == q.key {
            let integer = decl.to_ascii_uppercase().contains("INT")
                || matches!(value, Some(ValueRef::Integer(_)));
            if !integer {
                return Err(DataError::Invalid(format!(
                    "“{name}” sütunu tam sayı değil ({decl}); anahtar olarak sorgunun tam sayı bir sütununu seçin."
                )));
            }
            key_seen = true;
            continue;
        }
        if Some(name) == q.geometry.as_ref() {
            geometry_seen = true;
            continue;
        }
        columns.push(Column {
            name: name.clone(),
            kind: ColumnKind::read(&decl),
            decl,
            not_null: false,
            default: None,
        });
    }
    drop(rows);
    if !key_seen {
        return Err(DataError::Invalid(format!(
            "Sorguda “{}” sütunu yok; anahtar olarak sorgunun bir sütununu seçin.",
            q.key
        )));
    }
    let mut srs_id = None;
    let geometry = match &q.geometry {
        None => None,
        Some(g) if !geometry_seen => {
            return Err(DataError::Invalid(format!(
                "Sorguda “{g}” sütunu yok; geometri olarak sorgunun bir sütununu seçin."
            )));
        }
        Some(g) => {
            let col = quote(g);
            let blob: Option<Vec<u8>> = c
                .query_row(
                    &format!(
                        "SELECT {col} FROM ({}) WHERE {col} IS NOT NULL LIMIT 1",
                        q.sql
                    ),
                    [],
                    |r| r.get(0),
                )
                .map(Some)
                .or_else(|e| {
                    if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                        Ok(None)
                    } else {
                        Err(unreadable(e))
                    }
                })?;
            let read = blob.as_deref().map(read_blob).transpose().map_err(|e| {
                DataError::Invalid(format!("“{g}” sütunu GeoPackage geometrisi tutmuyor ({e}); geometri olarak bir geometri sütunu seçin."))
            })?;
            let (type_name, (z, m)) = read.as_ref().map_or(("GEOMETRY", (false, false)), |b| {
                (b.geometry.type_name(), b.geometry.dims())
            });
            srs_id = read.as_ref().map(|b| b.srs_id).filter(|s| *s > 0);
            Some(GeometryColumn {
                name: g.clone(),
                type_name: type_name.to_owned(),
                z: u8::from(z),
                m: u8::from(m),
            })
        }
    };
    Ok(Layer {
        name: table.to_owned(),
        data_type: if geometry.is_some() {
            DataType::Features
        } else {
            DataType::Attributes
        },
        identifier: None,
        description: None,
        bounds: None,
        srs_id,
        geometry,
        fid: q.key.clone(),
        columns,
        indexed: false,
    })
}

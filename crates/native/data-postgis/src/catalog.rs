//! What a database holds and a table's shape, from its catalog (docs/adr/9100 §6): tables, views and materialized
//! views the account may read, each with its first geometry column (`geometry_columns`, `geography_columns`), its
//! estimated rows (`reltuples`), its comment and whether it can be written (a table, a single integer primary key, the
//! right to insert, update or delete). A view's key is its first integer column named `fid`, `id`, `gid` or
//! `ogc_fid`; without one it cannot be read as a layer.

use kentos_data::{DataError, Field, FieldKind, GeometryField, Schema, TableInfo, TableKind};
use sqlx::{PgConnection, Row};

use crate::connect::error_of;

/// A table's shape and what its statements need.
#[derive(Clone, Debug)]
pub struct Shape {
    pub schema: Schema,
    /// `"schema"."table"`.
    pub sql: String,
    /// Its parts, unquoted.
    pub namespace: String,
    pub relation: String,
    /// The geometry column is `geography`: cast to `geometry` to be read.
    pub geography: bool,
    /// Rows can be written.
    pub writable: bool,
    /// The database has ICU's Turkish collation (`tr-x-icu`): texts are ordered by it (docs/adr/9102 §3).
    pub turkish: bool,
}

/// An identifier quoted.
pub fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// `schema.table` split (a name without a schema is `public`'s) and quoted.
pub fn qualified(table: &str) -> (String, String, String) {
    let (schema, name) = table.split_once('.').unwrap_or(("public", table));
    (
        format!("{}.{}", quote(schema), quote(name)),
        schema.to_owned(),
        name.to_owned(),
    )
}

/// A column's kind from its type.
pub fn kind_of(typname: &str, formatted: &str) -> FieldKind {
    let f = formatted.to_ascii_lowercase();
    match typname {
        "bool" => FieldKind::Bool,
        "int2" => FieldKind::Int { bits: 16 },
        "int4" => FieldKind::INT32,
        "int8" | "oid" => FieldKind::INT,
        "float4" | "float8" => FieldKind::Real,
        "numeric" => {
            // numeric(12,2)
            let inner = f
                .split_once('(')
                .and_then(|(_, r)| r.strip_suffix(')'))
                .map(|r| {
                    let mut it = r.split(',').map(|x| x.trim().parse::<u32>().ok());
                    (it.next().flatten(), it.next().flatten())
                });
            let (precision, scale) = inner.unwrap_or((None, None));
            FieldKind::Decimal { precision, scale }
        }
        "text" | "citext" | "name" | "bpchar" | "varchar" => FieldKind::Text {
            max: f
                .split_once('(')
                .and_then(|(_, r)| r.strip_suffix(')'))
                .and_then(|r| r.trim().parse().ok()),
        },
        "date" => FieldKind::Date,
        "timestamp" | "timestamptz" => FieldKind::DateTime,
        "time" | "timetz" => FieldKind::Time,
        "bytea" => FieldKind::Blob,
        "json" | "jsonb" => FieldKind::Json,
        "uuid" => FieldKind::Uuid,
        _ => FieldKind::Other,
    }
}

/// The tables the account may read; a schema's projects table (docs/adr/9107) is the program's own, not a layer.
pub async fn tables(conn: &mut PgConnection) -> Result<Vec<TableInfo>, DataError> {
    let rows = sqlx::query(
        "SELECT c.oid::int8 AS oid, n.nspname::text AS ns, c.relname::text AS rel, c.relkind::text AS kind,
                c.reltuples::float8 AS tuples, obj_description(c.oid, 'pg_class') AS comment,
                (has_table_privilege(c.oid, 'INSERT') OR has_table_privilege(c.oid, 'UPDATE')
                   OR has_table_privilege(c.oid, 'DELETE')) AS writes
         FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE c.relkind IN ('r', 'v', 'm', 'p', 'f')
           AND n.nspname NOT IN ('pg_catalog', 'information_schema', 'topology', 'tiger', 'tiger_data')
           AND n.nspname NOT LIKE 'pg\\_toast%' AND n.nspname NOT LIKE 'pg\\_temp%'
           AND has_table_privilege(c.oid, 'SELECT')
           AND NOT (c.relname IN ('spatial_ref_sys', 'geometry_columns', 'geography_columns', 'raster_columns',
                                  'raster_overviews') AND n.nspname = 'public')
           AND c.relname <> 'kentos_projects'
         ORDER BY n.nspname, c.relname",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(error_of)?;
    let geoms = sqlx::query(
        "SELECT f_table_schema::text AS ns, f_table_name::text AS rel, f_geometry_column::text AS col, type::text AS t,
                srid::int4 AS srid, coord_dimension::int4 AS dims FROM geometry_columns
         UNION ALL
         SELECT f_table_schema::text, f_table_name::text, f_geography_column::text, type::text, srid::int4,
                coord_dimension::int4 FROM geography_columns",
    )
    .fetch_all(&mut *conn)
    .await
    .unwrap_or_default();
    let keys = sqlx::query(
        "SELECT i.indrelid::int8 AS oid, t.typname::text AS typ
         FROM pg_index i
         JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = i.indkey[0]
         JOIN pg_type t ON t.oid = a.atttypid
         WHERE i.indisprimary AND i.indnatts = 1",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(error_of)?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let oid: i64 = r.try_get("oid").map_err(error_of)?;
        let ns: String = r.try_get("ns").map_err(error_of)?;
        let rel: String = r.try_get("rel").map_err(error_of)?;
        let kind: String = r.try_get("kind").map_err(error_of)?;
        let tuples: f64 = r.try_get("tuples").map_err(error_of)?;
        let comment: Option<String> = r.try_get("comment").map_err(error_of)?;
        let writes: bool = r.try_get("writes").map_err(error_of)?;
        let geometry = geoms.iter().find_map(|g| {
            let gns: String = g.try_get("ns").ok()?;
            let grel: String = g.try_get("rel").ok()?;
            if gns != ns || grel != rel {
                return None;
            }
            let t: String = g.try_get("t").ok()?;
            let srid: i32 = g.try_get("srid").ok()?;
            let dims: i32 = g.try_get("dims").ok()?;
            Some(geometry_field(g.try_get("col").ok()?, &t, srid, dims))
        });
        let int_key = keys.iter().any(|k| {
            k.try_get::<i64, _>("oid").ok() == Some(oid)
                && matches!(
                    k.try_get::<String, _>("typ").ok().as_deref(),
                    Some("int2" | "int4" | "int8")
                )
        });
        let table_kind = match (kind.as_str(), &geometry) {
            ("v" | "m", _) => TableKind::View,
            (_, Some(_)) => TableKind::Features,
            (_, None) => TableKind::Attributes,
        };
        out.push(TableInfo {
            name: format!("{ns}.{rel}"),
            title: comment
                .as_deref()
                .and_then(|c| c.lines().next())
                .map(str::trim)
                .filter(|c| !c.is_empty() && !c.starts_with('{'))
                .map(str::to_owned),
            kind: table_kind,
            geometry,
            rows: (tuples >= 0.0).then_some(tuples as u64),
            extent: None,
            writable: writes && int_key && matches!(kind.as_str(), "r" | "p"),
            comment,
        });
    }
    Ok(out)
}

/// A geometry column from `geometry_columns`' type (`MULTIPOLYGON`, `POINTM` …), system and dimensions.
fn geometry_field(name: String, type_name: &str, srid: i32, dims: i32) -> GeometryField {
    let t = type_name.to_ascii_uppercase();
    let m_only = t.ends_with('M') && !t.ends_with("ZM");
    let base = t
        .trim_end_matches("ZM")
        .trim_end_matches('M')
        .trim_end_matches('Z')
        .to_owned();
    // `dims` counts the positions' numbers: 3 is XYZ unless the type ends in M.
    let (z, m) = match dims {
        4 => (true, true),
        3 if m_only => (false, true),
        3 => (true, false),
        _ => (false, false),
    };
    GeometryField {
        name,
        type_name: if base.is_empty() {
            "GEOMETRY".into()
        } else {
            base
        },
        z,
        m,
        srid: (srid > 0).then_some(srid),
    }
}

/// A table's shape, read on `conn` (a transaction's own connection sees the tables it made).
pub async fn shape(conn: &mut PgConnection, table: &str) -> Result<Shape, DataError> {
    let (sql, ns, rel) = qualified(table);
    let found = sqlx::query(
        "SELECT c.oid::int8 AS oid, c.relkind::text AS kind,
                (has_table_privilege(c.oid, 'INSERT') OR has_table_privilege(c.oid, 'UPDATE')
                   OR has_table_privilege(c.oid, 'DELETE')) AS writes
         FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = $1 AND c.relname = $2",
    )
    .bind(&ns)
    .bind(&rel)
    .fetch_optional(&mut *conn)
    .await
    .map_err(error_of)?
    .ok_or_else(|| DataError::NoTable(table.to_owned()))?;
    let oid: i64 = found.try_get("oid").map_err(error_of)?;
    let kind: String = found.try_get("kind").map_err(error_of)?;
    let writes: bool = found.try_get("writes").map_err(error_of)?;
    let cols = sqlx::query(
        "SELECT a.attname::text AS name, format_type(a.atttypid, a.atttypmod) AS formatted, t.typname::text AS typ,
                a.attnotnull AS notnull, pg_get_expr(d.adbin, d.adrelid) AS def, a.attgenerated::text AS generated,
                a.attidentity::text AS identity, col_description(a.attrelid, a.attnum) AS comment
         FROM pg_attribute a
         JOIN pg_type t ON t.oid = a.atttypid
         LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
         WHERE a.attrelid = $1::oid AND a.attnum > 0 AND NOT a.attisdropped
         ORDER BY a.attnum",
    )
    .bind(oid)
    .fetch_all(&mut *conn)
    .await
    .map_err(error_of)?;
    let pk: Option<String> = sqlx::query(
        "SELECT a.attname::text AS name FROM pg_index i
         JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = i.indkey[0]
         JOIN pg_type t ON t.oid = a.atttypid
         WHERE i.indrelid = $1::oid AND i.indisprimary AND i.indnatts = 1 AND t.typname IN ('int2', 'int4', 'int8')",
    )
    .bind(oid)
    .fetch_optional(&mut *conn)
    .await
    .map_err(error_of)?
    .and_then(|r| r.try_get("name").ok());
    let geom_info = sqlx::query(
        "SELECT f_geometry_column::text AS col, type::text AS t, srid::int4 AS srid, coord_dimension::int4 AS dims,
                false AS geography FROM geometry_columns WHERE f_table_schema = $1 AND f_table_name = $2
         UNION ALL
         SELECT f_geography_column::text, type::text, srid::int4, coord_dimension::int4, true
           FROM geography_columns WHERE f_table_schema = $1 AND f_table_name = $2",
    )
    .bind(&ns)
    .bind(&rel)
    .fetch_all(&mut *conn)
    .await
    .unwrap_or_default();
    let mut fields = Vec::new();
    let mut geometry: Option<GeometryField> = None;
    let mut geography = false;
    let mut ints: Vec<String> = Vec::new();
    // Columns the database fills itself: a serial's default, an identity of either kind.
    let mut made: Vec<String> = Vec::new();
    for c in &cols {
        let name: String = c.try_get("name").map_err(error_of)?;
        let formatted: String = c.try_get("formatted").map_err(error_of)?;
        let typ: String = c.try_get("typ").map_err(error_of)?;
        if typ == "geometry" || typ == "geography" {
            if geometry.is_none() {
                let info = geom_info
                    .iter()
                    .find(|g| g.try_get::<String, _>("col").ok().as_deref() == Some(name.as_str()));
                let (t, srid, dims) = info
                    .map(|g| {
                        (
                            g.try_get::<String, _>("t")
                                .unwrap_or_else(|_| "GEOMETRY".into()),
                            g.try_get::<i32, _>("srid").unwrap_or(0),
                            g.try_get::<i32, _>("dims").unwrap_or(2),
                        )
                    })
                    .unwrap_or(("GEOMETRY".into(), 0, 2));
                geometry = Some(geometry_field(name.clone(), &t, srid, dims));
                geography = typ == "geography";
            }
            continue;
        }
        if matches!(typ.as_str(), "int2" | "int4" | "int8") {
            ints.push(name.clone());
        }
        let generated: String = c.try_get("generated").unwrap_or_default();
        let identity: String = c.try_get("identity").unwrap_or_default();
        if !identity.is_empty() || c.try_get::<Option<String>, _>("def").ok().flatten().is_some() {
            made.push(name.clone());
        }
        fields.push(Field {
            kind: kind_of(&typ, &formatted),
            native: formatted,
            nullable: !c.try_get::<bool, _>("notnull").unwrap_or(false),
            default: c.try_get::<Option<String>, _>("def").ok().flatten(),
            generated: !generated.is_empty() || identity == "a",
            comment: c.try_get::<Option<String>, _>("comment").ok().flatten(),
            name,
        });
    }
    let key = pk.clone().or_else(|| {
        ["fid", "id", "gid", "ogc_fid"]
            .iter()
            .find(|k| ints.iter().any(|i| i == *k))
            .map(|k| (*k).to_owned())
    });
    let Some(key) = key else {
        return Err(DataError::Unsupported(format!(
            "“{table}” tablosunun tam sayı birincil anahtarı yok (bir görünümde fid, id, gid ya da ogc_fid adlı tam sayı sütunu); katman olarak okunamaz"
        )));
    };
    let key_made = made.contains(&key);
    fields.retain(|f| f.name != key);
    let turkish: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_collation WHERE collname = 'tr-x-icu')")
        .fetch_one(&mut *conn)
        .await
        .unwrap_or(false);
    Ok(Shape {
        schema: Schema {
            key,
            key_made,
            fields,
            geometry,
        },
        sql,
        namespace: ns,
        relation: rel,
        geography,
        writable: writes && pk.is_some() && matches!(kind.as_str(), "r" | "p"),
        turkish,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_read_as_kinds() {
        assert_eq!(
            kind_of("numeric", "numeric(12,2)"),
            FieldKind::Decimal {
                precision: Some(12),
                scale: Some(2)
            }
        );
        assert_eq!(
            kind_of("varchar", "character varying(40)"),
            FieldKind::Text { max: Some(40) }
        );
        assert_eq!(
            kind_of("timestamptz", "timestamp with time zone"),
            FieldKind::DateTime
        );
        let g = geometry_field("geom".into(), "MULTIPOLYGONZ", 5256, 3);
        assert_eq!(
            (g.type_name.as_str(), g.z, g.m),
            ("MULTIPOLYGON", true, false)
        );
        let g = geometry_field("geom".into(), "POINTM", 4326, 3);
        assert_eq!((g.type_name.as_str(), g.z, g.m), ("POINT", false, true));
        assert_eq!(qualified("kbs.parseller").0, "\"kbs\".\"parseller\"");
    }
}

//! A GeoPackage column's type and value in kentos-data's words, and back (docs/adr/9100 §5): the column's declared
//! type decides the kind (`BOOLEAN` 0/1 a yes or no, `DATE` and `DATETIME` text an ISO date); a cell SQLite holds in
//! another storage class than its column's stays what it is.

use kentos_data::{Field, FieldKind, GeometryField, Value};
use kentos_geopackage::{Column, ColumnKind, Layer};
use rusqlite::types::{ToSqlOutput, Value as Sql, ValueRef};

/// An integer column's width by its declared type: TINYINT 8, SMALLINT 16, MEDIUMINT 32, INT and INTEGER 64 bits
/// (GeoPackage 1.3, Table 1; GDAL reads them so).
fn int_bits(decl: &str) -> u8 {
    match decl.split('(').next().unwrap_or_default().trim().to_ascii_uppercase().as_str() {
        "TINYINT" => 8,
        "SMALLINT" => 16,
        "MEDIUMINT" => 32,
        _ => 64,
    }
}

/// An integer of `bits` bits as a GeoPackage declares it (the narrowest type that holds it).
fn int_decl(bits: u8) -> &'static str {
    match bits {
        0..=8 => "TINYINT",
        9..=16 => "SMALLINT",
        17..=32 => "MEDIUMINT",
        _ => "INTEGER",
    }
}

/// A column's kind, by its kind and its declared type (empty: the kind's).
pub fn kind_of(c: &ColumnKind, decl: &str) -> FieldKind {
    match c {
        ColumnKind::Boolean => FieldKind::Bool,
        ColumnKind::Integer => FieldKind::Int { bits: int_bits(decl) },
        ColumnKind::Real => FieldKind::Real,
        ColumnKind::Text(max) => FieldKind::Text { max: *max },
        ColumnKind::Blob(_) => FieldKind::Blob,
        ColumnKind::Date => FieldKind::Date,
        ColumnKind::DateTime => FieldKind::DateTime,
        ColumnKind::Other(decl) => {
            let d = decl.to_ascii_uppercase();
            if d.contains("CHAR") || d.contains("CLOB") || d.contains("TEXT") {
                FieldKind::Text { max: None }
            } else if d.contains("INT") {
                FieldKind::INT
            } else if d.contains("REAL")
                || d.contains("FLOA")
                || d.contains("DOUB")
                || d.contains("NUMERIC")
            {
                FieldKind::Real
            } else {
                FieldKind::Other
            }
        }
    }
}

/// The column a field is written as.
pub fn column_of(f: &Field) -> Column {
    let kind = match &f.kind {
        FieldKind::Bool => ColumnKind::Boolean,
        FieldKind::Int { .. } => ColumnKind::Integer,
        // GeoPackage has no exact decimal: a double, as other writers do.
        FieldKind::Real | FieldKind::Decimal { .. } => ColumnKind::Real,
        FieldKind::Text { max } => ColumnKind::Text(*max),
        FieldKind::Date => ColumnKind::Date,
        FieldKind::DateTime => ColumnKind::DateTime,
        FieldKind::Blob => ColumnKind::Blob(None),
        FieldKind::Uuid => ColumnKind::Text(Some(36)),
        FieldKind::Time | FieldKind::Json | FieldKind::Other => ColumnKind::Text(None),
    };
    Column {
        name: f.name.clone(),
        // A 32-bit integer is MEDIUMINT, not INTEGER: another program reads INTEGER as a 64-bit one (docs/adr/9108 §1).
        decl: match &f.kind {
            FieldKind::Int { bits } => int_decl(*bits).to_owned(),
            _ => String::new(),
        },
        kind,
        not_null: !f.nullable,
        default: f.default.clone(),
    }
}

/// A layer's fields.
pub fn fields_of(layer: &Layer) -> Vec<Field> {
    layer
        .columns
        .iter()
        .map(|c| Field {
            name: c.name.clone(),
            kind: kind_of(&c.kind, &c.decl),
            native: if c.decl.is_empty() {
                c.kind.decl()
            } else {
                c.decl.clone()
            },
            nullable: !c.not_null,
            default: c.default.clone().filter(|d| !d.contains("GENERATED")),
            generated: c
                .default
                .as_deref()
                .is_some_and(|d| d.contains("GENERATED")),
            comment: None,
        })
        .collect()
}

/// A layer's geometry column.
pub fn geometry_of(layer: &Layer) -> Option<GeometryField> {
    layer.geometry.as_ref().map(|g| GeometryField {
        name: g.name.clone(),
        type_name: g.type_name.to_ascii_uppercase(),
        z: g.z == 1,
        m: g.m == 1,
        srid: layer.srs_id.filter(|s| *s > 0),
    })
}

/// A cell read as its column's kind.
pub fn value_of(v: ValueRef<'_>, kind: &FieldKind) -> Value {
    match (v, kind) {
        (ValueRef::Null, _) => Value::Null,
        (ValueRef::Integer(i), FieldKind::Bool) => Value::Bool(i != 0),
        (ValueRef::Integer(i), FieldKind::Real) => Value::Real(i as f64),
        (ValueRef::Integer(i), _) => Value::Int(i),
        (ValueRef::Real(r), _) => Value::Real(r),
        (ValueRef::Text(t), k) => {
            let s = String::from_utf8_lossy(t).into_owned();
            match k {
                FieldKind::Date => Value::Date(s),
                FieldKind::DateTime => Value::DateTime(s),
                FieldKind::Bool => match s.to_ascii_lowercase().as_str() {
                    "true" | "1" => Value::Bool(true),
                    "false" | "0" => Value::Bool(false),
                    _ => Value::Text(s),
                },
                _ => Value::Text(s),
            }
        }
        (ValueRef::Blob(b), _) => Value::Blob(b.to_vec()),
    }
}

/// A value as SQLite binds it.
pub fn sql_of(v: &Value) -> Sql {
    match v {
        Value::Null => Sql::Null,
        Value::Bool(b) => Sql::Integer(i64::from(*b)),
        Value::Int(i) => Sql::Integer(*i),
        Value::Real(r) => Sql::Real(*r),
        Value::Decimal(d) => d
            .trim()
            .parse::<f64>()
            .map_or_else(|_| Sql::Text(d.clone()), Sql::Real),
        Value::Text(t) | Value::Date(t) | Value::DateTime(t) | Value::Time(t) | Value::Json(t) => {
            Sql::Text(t.clone())
        }
        Value::Blob(b) => Sql::Blob(b.clone()),
    }
}

/// Binds owned SQLite values.
pub struct Bind<'a>(pub &'a Sql);

impl rusqlite::ToSql for Bind<'_> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(match self.0 {
            Sql::Null => ValueRef::Null,
            Sql::Integer(i) => ValueRef::Integer(*i),
            Sql::Real(r) => ValueRef::Real(*r),
            Sql::Text(t) => ValueRef::Text(t.as_bytes()),
            Sql::Blob(b) => ValueRef::Blob(b),
        }))
    }
}

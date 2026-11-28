//! A table's shape as every driver describes it (docs/adr/9100 §3): its key, its fields with their kinds and rules,
//! and its geometry column with its type, Z, M and system. A source's own type names stay with each field
//! (`native`): a driver writes a table back with the types it read.

use crate::value::Value;

/// What a field holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Bool,
    /// An integer of `bits` bits (8, 16, 32 or 64) as the source declares it (`SMALLINT`, `int4`, `MEDIUMINT`, `bigint`):
    /// a writer makes the column that holds it and a value past its range is refused (docs/adr/9108 §1).
    Int {
        bits: u8,
    },
    /// A double.
    Real,
    /// An exact decimal: digits in all and after the point, when the source says them.
    Decimal {
        precision: Option<u32>,
        scale: Option<u32>,
    },
    /// Text, with its largest length when it has one.
    Text {
        max: Option<u32>,
    },
    Date,
    DateTime,
    Time,
    Blob,
    Json,
    Uuid,
    /// Anything else (an array, a range, a type of the database's own): read as its text, never written.
    Other,
}

impl FieldKind {
    /// A 64-bit integer: what a source declares without a width.
    pub const INT: FieldKind = FieldKind::Int { bits: 64 };
    /// A 32-bit integer (`MEDIUMINT`, `integer`).
    pub const INT32: FieldKind = FieldKind::Int { bits: 32 };

    /// The smallest and the largest value an integer of `bits` bits holds.
    pub fn int_range(bits: u8) -> (i64, i64) {
        match bits {
            8 => (i64::from(i8::MIN), i64::from(i8::MAX)),
            16 => (i64::from(i16::MIN), i64::from(i16::MAX)),
            32 => (i64::from(i32::MIN), i64::from(i32::MAX)),
            _ => (i64::MIN, i64::MAX),
        }
    }

    /// Its name in the interface.
    pub fn words(&self) -> &'static str {
        match self {
            FieldKind::Bool => "Evet/Hayır",
            FieldKind::Int { bits } if *bits > 32 => "Büyük tam sayı",
            FieldKind::Int { .. } => "Tam sayı",
            FieldKind::Real => "Ondalık sayı",
            FieldKind::Decimal { .. } => "Kesin ondalık",
            FieldKind::Text { .. } => "Metin",
            FieldKind::Date => "Tarih",
            FieldKind::DateTime => "Tarih ve saat",
            FieldKind::Time => "Saat",
            FieldKind::Blob => "İkili veri",
            FieldKind::Json => "JSON",
            FieldKind::Uuid => "UUID",
            FieldKind::Other => "Diğer",
        }
    }

    /// Whether it holds numbers (a box's order, a class's range).
    pub fn is_number(&self) -> bool {
        matches!(
            self,
            FieldKind::Int { .. } | FieldKind::Real | FieldKind::Decimal { .. }
        )
    }

    /// Whether a text search looks in it.
    pub fn is_text(&self) -> bool {
        matches!(
            self,
            FieldKind::Text { .. } | FieldKind::Uuid | FieldKind::Json
        )
    }
}

/// A field of a table, its key and geometry aside.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub kind: FieldKind,
    /// The type as the source declares it (`varchar(40)`, `MEDIUMINT`, `numeric(12,2)`).
    pub native: String,
    pub nullable: bool,
    /// Its default as the source's SQL text.
    pub default: Option<String>,
    /// The source makes its value itself (generated, identity): never written.
    pub generated: bool,
    pub comment: Option<String>,
}

impl Field {
    /// A plain nullable field of `kind`, its native type written by the driver.
    pub fn new(name: &str, kind: FieldKind) -> Field {
        Field {
            name: name.to_owned(),
            kind,
            native: String::new(),
            nullable: true,
            default: None,
            generated: false,
            comment: None,
        }
    }
}

/// Whether a geometry type admits points, lines or areas: what a layer of it draws and what its tools add.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    Point,
    Line,
    Area,
    /// A column of any type, or a collection.
    Any,
}

/// The geometry column.
#[derive(Clone, Debug, PartialEq)]
pub struct GeometryField {
    pub name: String,
    /// The type's name as OGC writes it (`POINT`, `MULTIPOLYGON`, `GEOMETRY` …), upper case.
    pub type_name: String,
    pub z: bool,
    pub m: bool,
    /// The system's EPSG code (or the source's own number); none: undefined.
    pub srid: Option<i32>,
}

impl GeometryField {
    /// What the type draws.
    pub fn family(&self) -> Family {
        family_of(&self.type_name)
    }

    /// Whether the type is a multi type (`MULTIPOINT` …): a single part written to it is made one.
    pub fn is_multi(&self) -> bool {
        let t = self.type_name.as_str();
        t.starts_with("MULTI") || t == "GEOMETRYCOLLECTION"
    }
}

/// What a type of geometry draws, by its OGC name.
pub fn family_of(type_name: &str) -> Family {
    match type_name.trim().to_ascii_uppercase().as_str() {
        "POINT" | "MULTIPOINT" => Family::Point,
        "LINESTRING" | "MULTILINESTRING" | "CIRCULARSTRING" | "COMPOUNDCURVE" | "MULTICURVE"
        | "CURVE" => Family::Line,
        "POLYGON" | "MULTIPOLYGON" | "CURVEPOLYGON" | "MULTISURFACE" | "SURFACE"
        | "POLYHEDRALSURFACE" | "TIN" | "TRIANGLE" => Family::Area,
        _ => Family::Any,
    }
}

/// A table's shape.
#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    /// The integer key column (a GeoPackage's fid, a database table's primary key).
    pub key: String,
    /// The source gives a new row its key itself (a GeoPackage's integer key, a database's serial or identity column);
    /// when not, a writer gives it one (docs/adr/9103 §4).
    pub key_made: bool,
    pub fields: Vec<Field>,
    pub geometry: Option<GeometryField>,
}

impl Schema {
    /// The index of the field named `name`.
    pub fn index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }

    /// The field named `name`.
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// `values` fitted to the fields: one a field, `Null` where none was given.
    pub fn row(&self, values: &[(String, Value)]) -> Vec<Value> {
        self.fields
            .iter()
            .map(|f| {
                values
                    .iter()
                    .find(|(n, _)| *n == f.name)
                    .map(|(_, v)| v.clone())
                    .unwrap_or(Value::Null)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_say_what_they_draw() {
        assert_eq!(family_of("MultiPolygon"), Family::Area);
        assert_eq!(family_of("COMPOUNDCURVE"), Family::Line);
        assert_eq!(family_of("POINT"), Family::Point);
        assert_eq!(family_of("GEOMETRY"), Family::Any);
        let g = GeometryField {
            name: "geom".into(),
            type_name: "MULTILINESTRING".into(),
            z: false,
            m: false,
            srid: Some(5256),
        };
        assert!(g.is_multi());
        assert_eq!(g.family(), Family::Line);
    }
}

/// A column's rule as its source keeps it (docs/adr/9324): the values the column takes, each with its label, or a
/// number's range (its ends each in or out, either open). A GeoPackage keeps it in its schema extension, a database as
/// a check constraint with its words beside it.
#[derive(Clone, Debug, PartialEq)]
pub enum ColumnRule {
    Values(Vec<(String, Option<String>)>),
    Range {
        min: Option<f64>,
        min_included: bool,
        max: Option<f64>,
        max_included: bool,
    },
}

/// A column and its rule; none: the column's rule taken away.
#[derive(Clone, Debug, PartialEq)]
pub struct RuledColumn {
    pub column: String,
    pub rule: Option<ColumnRule>,
}

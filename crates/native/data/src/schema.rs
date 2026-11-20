//! A table's shape as every driver describes it (docs/adr/9100 §3): its key, its fields with their kinds and rules,
//! and its geometry column with its type, Z, M and system. A source's own type names stay with each field
//! (`native`): a driver writes a table back with the types it read.

use crate::value::Value;

/// What a field holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Bool,
    /// A 64-bit integer at most.
    Int,
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
    /// Its name in the interface.
    pub fn words(&self) -> &'static str {
        match self {
            FieldKind::Bool => "Evet/Hayır",
            FieldKind::Int => "Tam sayı",
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
            FieldKind::Int | FieldKind::Real | FieldKind::Decimal { .. }
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

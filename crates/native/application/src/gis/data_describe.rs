//! A data source's table as a data layer holds it (docs/adr/9101 §4): its catalogue and shape read through its driver
//! (no row), its columns made the layer's fields. The hosts' Katman olarak ekle and the opening of a GeoPackage as a
//! project (docs/adr/9116) describe their tables here, then add them with `cad.layers.data`.

use kentos_commands::{DataGeometry, DataLayer, LayerField, LayerFieldKind, fields_problem};
use kentos_data::{FieldKind, Schema};

/// A table as a data layer would hold it: where it is, its fields, its catalogue's box and a name for the layer.
#[derive(Clone, Debug)]
pub struct Described {
    pub data: DataLayer,
    pub fields: Vec<LayerField>,
    /// Columns not taken as fields, by name, with why.
    pub left: Vec<String>,
    pub extent: Option<[f64; 4]>,
    pub name: String,
}

/// A geometry type's name as a data layer keeps it: OGC's upper case name, a type it does not draw as `GEOMETRY`.
pub fn type_name(t: &str) -> String {
    let t = t.trim().to_ascii_uppercase();
    if kentos_commands::DATA_GEOMETRY_TYPES.contains(&t.as_str()) {
        t
    } else {
        "GEOMETRY".into()
    }
}

/// A column's kind as a layer's field: a 64-bit integer column as a big integer, a date with its offset as an instant.
pub fn field_kind(kind: &FieldKind, native: &str) -> Option<LayerFieldKind> {
    let n = native.to_ascii_lowercase();
    Some(match kind {
        FieldKind::Bool => LayerFieldKind::Boolean,
        // A 64-bit column (`bigint`, a GeoPackage `INTEGER`) is a big integer, a narrower one an integer (docs/adr/9108 §1).
        FieldKind::Int { bits } if *bits > 32 => LayerFieldKind::BigInt,
        FieldKind::Int { .. } => LayerFieldKind::Integer,
        FieldKind::Real | FieldKind::Decimal { .. } => LayerFieldKind::Decimal,
        FieldKind::Text { .. } | FieldKind::Other => LayerFieldKind::Text,
        FieldKind::Date => LayerFieldKind::Date,
        FieldKind::DateTime if n.contains("with time zone") || n == "timestamptz" => LayerFieldKind::Instant,
        FieldKind::DateTime => LayerFieldKind::DateTime,
        FieldKind::Time => LayerFieldKind::Time,
        FieldKind::Json => LayerFieldKind::Json,
        FieldKind::Uuid => LayerFieldKind::Uuid,
        FieldKind::Blob => LayerFieldKind::Binary,
    })
}

/// The layer's fields from the table's columns: each one a field when it keeps the fields' rules; a name met twice
/// (folded the Turkish way) or one the rules refuse is left out and said.
pub fn fields_of(schema: &Schema) -> (Vec<LayerField>, Vec<String>) {
    let mut fields: Vec<LayerField> = Vec::new();
    let mut left = Vec::new();
    for f in &schema.fields {
        let Some(kind) = field_kind(&f.kind, &f.native) else {
            continue;
        };
        let mut field = LayerField::new(f.name.clone(), kind);
        field.length = match f.kind {
            FieldKind::Text { max } => max,
            _ => None,
        };
        field.scale = match f.kind {
            FieldKind::Decimal { scale, .. } => scale,
            _ => None,
        };
        field.required = !f.nullable && f.default.is_none() && !f.generated;
        let mut with = fields.clone();
        with.push(field.clone());
        match fields_problem(&with) {
            None => fields.push(field),
            Some(why) => left.push(format!("“{}”: {why}", f.name)),
        }
    }
    (fields, left)
}

/// Reads a table's shape through its driver (the catalogue only, no row) for a data layer.
pub fn describe(address: &str, table: &str) -> Result<Described, String> {
    let source = kentos_data::registry().source(address).map_err(|e| e.to_string())?;
    describe_in(source.as_ref(), address, table)
}

/// [`describe`] with the source already open.
pub fn describe_in(source: &dyn kentos_data::Source, address: &str, table: &str) -> Result<Described, String> {
    let schema = source.schema(table).map_err(|e| e.to_string())?;
    let (fields, left) = fields_of(&schema);
    let data = DataLayer {
        driver: source.driver().to_owned(),
        address: address.to_owned(),
        relative: None,
        table: table.to_owned(),
        key: schema.key.clone(),
        geometry: schema.geometry.as_ref().map(|g| DataGeometry {
            column: g.name.clone(),
            type_name: type_name(&g.type_name),
            srid: g.srid.filter(|s| *s > 0).map(|s| s as u32),
            z: g.z,
            m: g.m,
        }),
        extent: None,
        read_only: !source.capabilities(table).is_ok_and(|c| c.writable()),
        text: None,
    };
    let extent = source.extent(table, true).ok().flatten();
    let data = DataLayer { extent: extent.filter(|_| data.geometry.is_some()), ..data };
    if let Some(problem) = data.problem() {
        return Err(problem);
    }
    let name = table.rsplit('.').next().unwrap_or(table).to_owned();
    Ok(Described { data, fields, left, extent, name })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_become_fields_and_types_their_names() {
        assert_eq!(type_name("MultiPolygon"), "MULTIPOLYGON");
        assert_eq!(type_name("TIN"), "GEOMETRY");
        assert_eq!(field_kind(&FieldKind::INT, "bigint"), Some(LayerFieldKind::BigInt));
        assert_eq!(field_kind(&FieldKind::INT32, "MEDIUMINT"), Some(LayerFieldKind::Integer));
        assert_eq!(field_kind(&FieldKind::DateTime, "timestamp with time zone"), Some(LayerFieldKind::Instant));
        let schema = Schema {
            key: "fid".into(),
            key_made: true,
            fields: vec![
                kentos_data::Field::new("ad", FieldKind::Text { max: Some(40) }),
                kentos_data::Field::new("AD", FieldKind::Text { max: None }),
                kentos_data::Field::new("alan", FieldKind::Real),
            ],
            geometry: None,
        };
        let (fields, left) = fields_of(&schema);
        assert_eq!(fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["ad", "alan"]);
        assert_eq!(fields[0].length, Some(40));
        assert_eq!(left.len(), 1, "{left:?}");
    }
}

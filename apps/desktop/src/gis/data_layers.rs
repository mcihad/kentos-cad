//! Veri katmanı on the desktop (docs/adr/9101, plan/veri.md VY-02): the data drivers registered at start with the
//! device's passwords, and a table added as a data layer: its catalogue and shape read (no row), its columns made the
//! layer's fields, `cad.layers.data` run as one undo step. The rows stay in the source; the view reads them as it draws
//! (VY-03).

use kentos_commands::{
    CommandResult, DataGeometry, DataLayer, LayerDataOperation, LayerField, LayerFieldKind,
    LayersData, fields_problem,
};
use kentos_data::{FieldKind, Schema};
use kentos_native_application::{ExecutionContext, layers_data};

use iced::Task;

use crate::app::{App, Message};

/// What the data layers' work says back.
#[derive(Debug, Clone)]
pub enum Event {
    /// Tables described for data layers (each one, or why not), then added to the drawing.
    Described(Vec<Result<Described, String>>),
}

fn message(event: Event) -> Message {
    Message::Gis(crate::gis::Event::DataLayers(event))
}

/// The device's passwords for the PostGIS driver: the saved connections' and the session's (docs/adr/0623).
struct Device;

impl kentos_data::Secrets for Device {
    fn password(&self, host: &str, port: u16, database: &str, user: &str) -> Option<String> {
        kentos_postgis::connections::password_for(&kentos_postgis::connections::Key {
            host: host.to_ascii_lowercase(),
            port,
            database: database.to_owned(),
            user: user.to_owned(),
        })
    }
}

/// Registers the data drivers and the device's password finder (once, at start).
pub fn register() {
    kentos_data_gpkg::register();
    kentos_data_postgis::register();
    kentos_data::set_secrets(Box::new(Device));
}

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
fn type_name(t: &str) -> String {
    let t = t.trim().to_ascii_uppercase();
    if kentos_commands::DATA_GEOMETRY_TYPES.contains(&t.as_str()) {
        t
    } else {
        "GEOMETRY".into()
    }
}

/// A column's kind as a layer's field: a 64-bit integer column as a big integer, a date with its offset as an instant.
fn field_kind(kind: &FieldKind, native: &str) -> Option<LayerFieldKind> {
    let n = native.to_ascii_lowercase();
    Some(match kind {
        FieldKind::Bool => LayerFieldKind::Boolean,
        // A 64-bit column (`bigint`, a GeoPackage `INTEGER`) is a big integer, a narrower one an integer (docs/adr/9108 §1).
        FieldKind::Int { bits } if *bits > 32 => LayerFieldKind::BigInt,
        FieldKind::Int { .. } => LayerFieldKind::Integer,
        FieldKind::Real | FieldKind::Decimal { .. } => LayerFieldKind::Decimal,
        FieldKind::Text { .. } | FieldKind::Other => LayerFieldKind::Text,
        FieldKind::Date => LayerFieldKind::Date,
        FieldKind::DateTime if n.contains("with time zone") || n == "timestamptz" => {
            LayerFieldKind::Instant
        }
        FieldKind::DateTime => LayerFieldKind::DateTime,
        FieldKind::Time => LayerFieldKind::Time,
        FieldKind::Json => LayerFieldKind::Json,
        FieldKind::Uuid => LayerFieldKind::Uuid,
        FieldKind::Blob => LayerFieldKind::Binary,
    })
}

/// The layer's fields from the table's columns: each one a field when it keeps the fields' rules; a name met twice
/// (folded the Turkish way) or one the rules refuse is left out and said.
fn fields_of(schema: &Schema) -> (Vec<LayerField>, Vec<String>) {
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
    let source = kentos_data::registry()
        .source(address)
        .map_err(|e| e.to_string())?;
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
    };
    let extent = source.extent(table, true).ok().flatten();
    let data = DataLayer {
        extent: extent.filter(|_| data.geometry.is_some()),
        ..data
    };
    if let Some(problem) = data.problem() {
        return Err(problem);
    }
    let name = table.rsplit('.').next().unwrap_or(table).to_owned();
    Ok(Described {
        data,
        fields,
        left,
        extent,
        name,
    })
}

/// The most rows a query's key is checked to be unique over when it is added (more are taken as they are).
pub const KEY_CHECKED: u64 = 1_000_000;

/// A query read as a data layer (docs/adr/9113 §3): its shape as a table's, read-only, named `name` (none: after its
/// first table). Its key is checked to tell its rows apart (counted at the source, up to [`KEY_CHECKED`] rows): a key
/// two rows share would pick, label and show the wrong row.
pub fn describe_query(address: &str, q: &kentos_data::QueryTable, name: Option<&str>) -> Result<Described, String> {
    let text = q.text();
    let source = kentos_data::registry().source(address).map_err(|e| e.to_string())?;
    let mut described = describe(address, &text)?;
    let rows = source.count(&text, &kentos_data::Request::all(), false).map_err(|e| e.to_string())?;
    if rows <= KEY_CHECKED {
        let distinct = source.unique(&text, &q.key, None, rows as usize + 1).map_err(|e| e.to_string())?;
        if (distinct.len() as u64) < rows {
            return Err(format!(
                "“{}” sütunu benzersiz değil: sorgunun {rows} satırında {} farklı değeri var. Anahtar olarak her satırda başka olan bir tam sayı sütunu seçin.",
                q.key,
                distinct.len()
            ));
        }
    }
    described.name = name.map(str::trim).filter(|n| !n.is_empty()).map_or_else(|| q.shown(), str::to_owned);
    described.data.read_only = true;
    Ok(described)
}

/// A data layer's box in the project's system: its table's box as last read, moved when the table is in another
/// system; none without one.
pub fn extent_in_project(
    node: &kentos_commands::LayerNode,
    settings: &kentos_commands::ProjectSettings,
) -> Option<[f64; 4]> {
    let data = node.data.as_ref()?;
    let b = data.extent?;
    let srid = data.geometry.as_ref().and_then(|g| g.srid);
    kentos_native_application::sources::extent_in_project(settings, srid, b).ok()
}

impl App {
    /// Adds tables of a source as data layers (Katman olarak ekle, Bağla): their shapes read off the interface's thread
    /// (a database's catalogue may be slow to answer), then each added as one undo step.
    pub(crate) fn add_tables(&mut self, address: String, tables: Vec<String>) -> Task<Message> {
        if tables.is_empty() {
            return Task::none();
        }
        self.say(
            kentos_interaction::Level::Info,
            format!("{} tablo okunuyor…", tables.len()),
        );
        crate::db_connections::off_thread(
            move || {
                tables
                    .iter()
                    .map(|t| describe(&address, t))
                    .collect::<Vec<_>>()
            },
            |r| message(Event::Described(r)),
        )
    }

    /// A query added as a data layer (docs/adr/9113 §3), read and checked off the interface's thread.
    pub(crate) fn add_query(&mut self, address: String, q: kentos_data::QueryTable, name: String) -> Task<Message> {
        self.say(kentos_interaction::Level::Info, "Sorgu okunuyor…");
        crate::db_connections::off_thread(
            move || vec![describe_query(&address, &q, Some(&name))],
            |r| message(Event::Described(r)),
        )
    }

    pub(crate) fn data_layers_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Described(read) => {
                let mut ok = Vec::new();
                for r in read {
                    match r {
                        Ok(d) => ok.push(d),
                        Err(why) => self.warn(why),
                    }
                }
                let left: Vec<String> = ok.iter().flat_map(|d| d.left.clone()).collect();
                let names: Vec<String> = ok.iter().map(|d| d.name.clone()).collect();
                let first = ok.first().cloned();
                match self.add_data_layers(ok) {
                    Ok(added) if !added.is_empty() => {
                        self.say(
                            kentos_interaction::Level::Success,
                            format!(
                                "{} veri katmanı eklendi: {}. Satırlar kaynağında kalır; görünüm gerektikçe okunur.",
                                added.len(),
                                names.join(", ")
                            ),
                        );
                        if let Some(b) = first.and_then(|d| d.extent) {
                            self.zoom_to_data_extent(b);
                        }
                    }
                    Ok(_) => {}
                    Err(why) => self.warn(why),
                }
                for l in left {
                    self.warn(format!("Alan olarak alınmadı: {l}"));
                }
            }
        }
        Task::none()
    }

    /// Frames a table's catalogue box (in the table's system; the project's when they are the same).
    fn zoom_to_data_extent(&mut self, b: [f64; 4]) {
        if !(b.iter().all(|v| v.is_finite()) && b[2] >= b[0] && b[3] >= b[1]) {
            return;
        }
        self.navigating(|app| {
            app.zoom_to_bounds(&kentos_commands::Bounds {
                min_x: b[0],
                min_y: b[1],
                max_x: b[2],
                max_y: b[3],
            })
        });
    }

    /// Adds described tables as data layers, each one undo step; their names in a sentence, or why one could not be.
    pub(crate) fn add_data_layers(&mut self, tables: Vec<Described>) -> Result<Vec<String>, String> {
        let Some(doc) = &mut self.document else {
            return Err("Açık çizim yok. Önce bir çizim açın ya da yeni bir proje başlatın.".into());
        };
        let mut added = Vec::new();
        for t in tables {
            let input = LayersData {
                operation: LayerDataOperation::Add,
                layer: None,
                name: Some(t.name.clone()),
                parent: None,
                index: Some(0),
                data: Some(t.data.clone()),
                fields: (!t.fields.is_empty()).then(|| t.fields.clone()),
                expected_revision: None,
            };
            match layers_data::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                CommandResult::Completed { output, .. } => added.push(output.layer),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    return Err(format!("“{}” eklenemedi: {}", t.name, error.message));
                }
                _ => return Err(format!("“{}” eklenemedi.", t.name)),
            }
        }
        Ok(added)
    }
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
        assert_eq!(
            field_kind(&FieldKind::DateTime, "timestamp with time zone"),
            Some(LayerFieldKind::Instant)
        );
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
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].length, Some(40));
        assert_eq!(left.len(), 1);
    }
}

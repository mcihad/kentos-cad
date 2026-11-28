//! The texts of a layer moved into a source (docs/adr/9114 §2). A table cannot hold a text as a geometry, so the layer's
//! texts are written into a table of their own beside its table: `<table>_yazilar` (a layer of texts only: its table),
//! a point and the columns `DataText::written` names, one row a text, in the project's system, in one transaction. A
//! data layer reading it draws its rows as texts.
//!
//! What a table has no column for (a text's style, mask, curve, scales, link, paragraph formats) is counted and said;
//! the words, the point, the height, the angle, the alignment and the width factor are written.

use kentos_commands::{
    DataGeometry, DataLayer, DataText, LayerField, LayerFieldKind, ProjectSettings, TextEntity,
};
use kentos_data::{Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable};
use kentos_domain::Document;

use crate::sources::TableSystem;

/// The longest table name a database takes (PostgreSQL's identifiers: 63 bytes).
const MOST: usize = 63;
/// What the texts' table's name has after the layer's table's.
const AFTER: &str = "_yazilar";

/// The layer's texts: every one, those its layer filter hides too (the layer goes, none may stay behind).
pub(crate) fn of<'d>(doc: &'d Document, layer: &str) -> Vec<&'d TextEntity> {
    doc.by_layer_with_uids(layer)
        .filter_map(|(_, e)| match e {
            kentos_commands::Entity::Text(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// Whether the layer holds anything but texts.
pub(crate) fn others(doc: &Document, layer: &str) -> bool {
    doc.by_layer_with_uids(layer)
        .any(|(_, e)| !matches!(e, kentos_commands::Entity::Text(_)))
}

/// The texts' table's name beside `table` (`schema.table` in a database): `_yazilar` after its name, which is shortened
/// to keep within 63 letters.
pub(crate) fn beside(table: &str) -> String {
    let (schema, name) = match table.rsplit_once('.') {
        Some((s, n)) => (Some(s), n),
        None => (None, table),
    };
    let mut head: String = name.to_owned();
    while head.len() + AFTER.len() > MOST {
        head.pop();
    }
    let name = format!("{head}{AFTER}");
    match schema {
        Some(s) => format!("{s}.{name}"),
        None => name,
    }
}

/// How many of the texts have what their table keeps no column for.
pub(crate) fn details_left(texts: &[&TextEntity]) -> u64 {
    texts
        .iter()
        .filter(|t| crate::data_text::details_left(t))
        .count() as u64
}

/// The texts' table: its key `fid`, its point and the columns of a text.
fn new_table(table: &str, srid: Option<u32>) -> NewTable {
    let text = DataText::written();
    let real =
        |name: &Option<String>| Field::new(name.as_deref().unwrap_or_default(), FieldKind::Real);
    NewTable {
        name: table.to_owned(),
        title: Some(table.to_owned()),
        description: Some("Yazılar: metin, yükseklik (m), açı (derece, doğudan saat yönünün tersine), hiza, genişlik çarpanı".into()),
        key: "fid".into(),
        fields: vec![
            Field::new(&text.text, FieldKind::Text { max: None }),
            real(&text.height),
            real(&text.angle),
            Field::new(text.align.as_deref().unwrap_or_default(), FieldKind::Text { max: Some(16) }),
            real(&text.width),
        ],
        geometry: Some(GeometryField {
            name: "geom".into(),
            type_name: "POINT".into(),
            z: false,
            m: false,
            srid: srid.and_then(|s| i32::try_from(s).ok()),
        }),
        spatial_index: true,
        system: crate::layer_export_sinks::system_text(srid.and_then(|s| i32::try_from(s).ok())),
    }
}

/// The texts as rows: each at its point in the table's system, its parts in their columns.
fn rows(texts: &[&TextEntity], system: Option<&TableSystem>) -> Result<Vec<NewFeature>, String> {
    let text = DataText::written();
    texts
        .iter()
        .map(|t| {
            let (x, y) = match system {
                Some(s) => s.to_table_point(t.p.x, t.p.y).ok_or_else(|| {
                    format!(
                        "“{}” yazısının noktası tablonun sistemine çevrilemedi.",
                        t.text
                    )
                })?,
                None => (t.p.x, t.p.y),
            };
            let values = text
                .cells(&crate::data_text::parts_of(t))
                .iter()
                .map(|(c, v)| (c.clone(), crate::data_text::value_of(v)))
                .collect();
            Ok(NewFeature {
                fid: None,
                geometry: Some(Geometry::Point(Some(kentos_data::Coord::xy(x, y)))),
                values,
            })
        })
        .collect()
}

/// Writes the texts' table into the source at `address` (a GeoPackage made when it is not there): made and filled in
/// one transaction; one there already made again (`replaceLayer`) or added to (`append`, docs/adr/9339). Its rows
/// written, or why not; a GeoPackage it made is removed when it fails.
pub(crate) fn write(
    address: &str,
    gpkg: bool,
    table: &str,
    srid: Option<u32>,
    settings: &ProjectSettings,
    texts: &[&TextEntity],
    existing: Option<kentos_commands::ExportExisting>,
) -> Result<u64, String> {
    let path = std::path::Path::new(address);
    let made = gpkg && !path.exists();
    if made {
        if let Some(folder) = path.parent().filter(|f| !f.as_os_str().is_empty()) {
            std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        }
        kentos_data::registry().forget(address);
        kentos_geopackage::GeoPackage::create(path).map_err(|e| e.0)?;
    }
    let written = (|| {
        let system = TableSystem::of(settings, srid)?;
        let rows = rows(texts, system.as_ref())?;
        let source = kentos_data::registry()
            .source(address)
            .map_err(|e| e.to_string())?;
        let there = !made
            && source
                .tables()
                .map_err(|e| e.to_string())?
                .iter()
                .any(|t| t.name == table);
        let append = there && existing == Some(kentos_commands::ExportExisting::Append);
        let replace = there && existing == Some(kentos_commands::ExportExisting::ReplaceLayer);
        let mut tx = source.begin().map_err(|e| e.to_string())?;
        let made_table = if append {
            Ok(())
        } else {
            tx.create_table(&new_table(table, srid), replace)
        };
        let done = made_table.and_then(|()| tx.insert(table, &rows).map(|_| ()));
        match done {
            Ok(()) => tx
                .commit()
                .map_err(|e| e.to_string())
                .map(|()| rows.len() as u64),
            Err(e) => {
                let _ = tx.rollback();
                Err(e.to_string())
            }
        }
    })();
    if written.is_err() && made {
        kentos_data::registry().forget(address);
        let _ = std::fs::remove_file(path);
    }
    written
}

/// Takes back a table written before the texts' table failed: it is dropped (a GeoPackage made for it removed whole).
pub(crate) fn take_back(address: &str, table: &str, made: bool) {
    if made {
        kentos_data::registry().forget(address);
        let _ = std::fs::remove_file(address);
        return;
    }
    if let Ok(source) = kentos_data::registry().source(address)
        && let Ok(mut tx) = source.begin()
    {
        match tx.drop_table(table) {
            Ok(()) => {
                let _ = tx.commit();
            }
            Err(_) => {
                let _ = tx.rollback();
            }
        }
    }
}

/// The data layer reading the texts' table, its rows drawn as texts, and its fields.
pub(crate) fn data_layer(
    gpkg: bool,
    address: &str,
    table: &str,
    srid: Option<u32>,
) -> (DataLayer, Vec<LayerField>) {
    let text = DataText::written();
    let field = |name: &str, kind: LayerFieldKind| LayerField::new(name, kind);
    let fields = vec![
        field(&text.text, LayerFieldKind::Text),
        field(
            text.height.as_deref().unwrap_or_default(),
            LayerFieldKind::Decimal,
        ),
        field(
            text.angle.as_deref().unwrap_or_default(),
            LayerFieldKind::Decimal,
        ),
        field(
            text.align.as_deref().unwrap_or_default(),
            LayerFieldKind::Text,
        ),
        field(
            text.width.as_deref().unwrap_or_default(),
            LayerFieldKind::Decimal,
        ),
    ];
    let data = DataLayer {
        driver: if gpkg { "gpkg" } else { "postgis" }.to_owned(),
        address: address.to_owned(),
        relative: None,
        table: table.to_owned(),
        key: "fid".into(),
        geometry: Some(DataGeometry {
            column: "geom".into(),
            type_name: "POINT".into(),
            srid,
            z: false,
            m: false,
        }),
        extent: None,
        read_only: false,
        text: Some(text),
    };
    (data, fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_texts_table_is_named_after_the_layers_within_63_letters() {
        assert_eq!(beside("eski_direkler"), "eski_direkler_yazilar");
        assert_eq!(beside("public.yollar"), "public.yollar_yazilar");
        let long = "a".repeat(63);
        let named = beside(&long);
        assert_eq!(named.len(), 63);
        assert!(named.ends_with("_yazilar"));
    }
}

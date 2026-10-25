//! A GeoPackage's relations and styles beside its tables (docs/adr/0627 §6, §7). `file.export.gpkg` writes the
//! project's relations between the layers it writes as OGC 18-000 related tables (`kentos_references`: the child's
//! table the base, the parent's the related one, a mapping by fid) with a KentOS document of each (its id, name, field
//! and deletion rule; `gpkg_metadata`, standard [`RELATION_STANDARD`]: a KentOS extension, documented, other readers
//! pass it over), and each feature table's look as QGIS's default style (`layer_styles`, QGIS's own table: the QML and
//! the SLD). `file.import.gpkg` takes them back: a KentOS relation as the project's; another writer's many-to-many
//! relation (QGIS's, GDAL's) as a multi-valued reference field of the base table's layer, named after its mapping
//! table, filled from the mapping's rows, and a relation; a table's default QGIS style as the look of the layer the
//! import makes for it. What is not taken is said, never dropped quietly; a form is not written (its package is
//! `cad.forms.export`'s).

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use kentos_commands::{LayerField, LayerFieldKind, LayerNodeType, ProjectRelation, RelationDelete};
use kentos_domain::{Document, Uuid};
use kentos_geopackage::relations::{KENTOS_RELATION, Relation};
use kentos_geopackage::styles::NewStyle;
use kentos_geopackage::{GeoPackage, Layer};
use serde::{Deserialize, Serialize};

use crate::geopackage::{TableRead, TableWritten};

/// The standard URI of KentOS's document of a relation (`gpkg_metadata.md_standard_uri`).
pub const RELATION_STANDARD: &str = "https://kentos.app/relation/1";
/// A table's description as a dataset (docs/adr/0633 §5): the layer's `dataset` as its JSON.
pub const DATASET_STANDARD: &str = "https://kentos.app/dataset/1";
/// What the layer the table was written from was derived from (docs/adr/0633 §5): its `derivation` as its JSON, kept
/// with the table; an import does not take it (its inputs are another drawing's layers). Not ADR 0530's
/// `https://kentos.app/derivation/1`, which makes a table a CAD source's GIS derivative.
pub const DERIVATION_STANDARD: &str = "https://kentos.app/layer-derivation/1";

/// KentOS's document of a relation in a GeoPackage, on the child table's field column: what OGC 18-000 does not say.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationDocument {
    /// The relation's mapping table (its row of `gpkgext_relations`).
    pub mapping: String,
    pub id: String,
    pub name: String,
    /// The child field that holds the parent's lasting id (several, `; ` apart, in a multi-valued field).
    pub field: String,
    /// The field holds several parents' ids: a GeoPackage column says only that it is text.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub multiple: bool,
    #[serde(default)]
    pub on_delete: RelationDelete,
}

/// What was written beside the tables.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProfileWritten {
    /// Each relation written: its mapping table, base (child) and related (parent) table, and its rows.
    pub relations: Vec<(String, String, String, u32)>,
    /// The feature tables given a QGIS style.
    pub styles: u32,
    /// What was not written, and why.
    pub left: Vec<String>,
}

/// A name SQLite and GeoPackage take for a table: letters, digits and `_`, not starting with a digit.
fn table_word(text: &str) -> String {
    let mut name: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert(0, '_');
    }
    name
}

/// The parent ids a child's field holds: one, or several `; ` apart.
fn references(value: Option<&String>) -> Vec<&str> {
    value
        .map(|v| {
            v.split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The file's rows by lasting id, for a table KentOS just wrote (its `kentos_uids`).
fn fids_by_uid(gp: &GeoPackage, layer: &Layer) -> Result<HashMap<String, i64>, String> {
    let mut out = HashMap::new();
    gp.each_uid(layer, |fid, uid| {
        if let Some(uid) = uid {
            out.insert(uid.to_owned(), fid);
        }
    })
    .map_err(|e| e.0)?;
    Ok(out)
}

/// The project's relations and the layers' looks written into the GeoPackage at `gp` beside `tables` (just written);
/// `made` the time the metadata documents carry.
pub fn write_profile(
    doc: &Document,
    tables: &[TableWritten],
    gp: &mut GeoPackage,
    made: &str,
) -> Result<ProfileWritten, String> {
    let mut out = ProfileWritten::default();
    let table_of: BTreeMap<&str, &TableWritten> = tables
        .iter()
        .filter_map(|t| t.layer.as_deref().map(|l| (l, t)))
        .collect();
    let layers = gp.layers().map_err(|e| e.0)?;
    let file_layer = |name: &str| layers.iter().find(|l| l.name == name).cloned();
    let layer_name = |id: &str| {
        doc.layers()
            .get(id)
            .map_or_else(|| id.to_owned(), |n| n.name.clone())
    };
    // The kinds a GeoPackage's column types do not say (docs/adr/0629 §4), on each table written from a layer.
    for t in tables {
        let Some(layer) = t.layer.as_deref() else {
            continue;
        };
        let Some(node) = doc.layers().get(layer) else {
            continue;
        };
        // The layer as a dataset and what it was derived from (docs/adr/0633 §5), each its own document.
        if let Some(d) = &node.dataset {
            let text = serde_json::to_string(d).map_err(|e| e.to_string())?;
            gp.add_metadata(&t.table, DATASET_STANDARD, "application/json", &text, made)
                .map_err(|e| e.0)?;
        }
        if let Some(d) = &node.derivation {
            let text = serde_json::to_string(d).map_err(|e| e.to_string())?;
            gp.add_metadata(&t.table, DERIVATION_STANDARD, "application/json", &text, made)
                .map_err(|e| e.0)?;
        }
        if let Some(text) = crate::geopackage::field_document(&node.fields, &t.columns) {
            gp.add_metadata(
                &t.table,
                crate::geopackage::FIELDS_STANDARD,
                "application/json",
                &text,
                made,
            )
            .map_err(|e| e.0)?;
        }
    }
    for r in &doc.settings().relations {
        let (Some(child), Some(parent)) = (
            table_of.get(r.child.as_str()),
            table_of.get(r.parent.as_str()),
        ) else {
            if table_of.contains_key(r.child.as_str()) || table_of.contains_key(r.parent.as_str()) {
                out.left.push(format!(
                    "“{}” ilişkisi yazılmadı: {} katmanı dışa aktarılmadı",
                    r.name,
                    if table_of.contains_key(r.child.as_str()) {
                        layer_name(&r.parent)
                    } else {
                        layer_name(&r.child)
                    }
                ));
            }
            continue;
        };
        let (Some(base), Some(related)) = (file_layer(&child.table), file_layer(&parent.table))
        else {
            continue;
        };
        let child_fids = fids_by_uid(gp, &base)?;
        let parent_fids = fids_by_uid(gp, &related)?;
        let mut pairs = Vec::new();
        let mut dangling = 0u32;
        for (uid, e) in doc.by_layer_with_uids(&r.child) {
            let Some(&fid) = child_fids.get(&uid.to_string()) else {
                continue;
            };
            for parent in references(e.base().attrs.get(&r.field)) {
                match parent_fids.get(parent) {
                    Some(&p) if !pairs.contains(&(fid, p)) => pairs.push((fid, p)),
                    Some(_) => {}
                    None => dangling += 1,
                }
            }
        }
        let stem = table_word(&format!("{}_{}", child.table, parent.table));
        let mut mapping = stem.clone();
        let mut n = 2;
        while gp.table_exists(&mapping).map_err(|e| e.0)? {
            mapping = format!("{stem}_{n}");
            n += 1;
        }
        gp.add_relation(&base, &related, &mapping, &pairs)
            .map_err(|e| e.0)?;
        let multiple = doc
            .layers()
            .get(&r.child)
            .is_some_and(|n| n.fields.iter().any(|f| f.name == r.field && f.multiple));
        let document = RelationDocument {
            mapping: mapping.clone(),
            id: r.id.clone(),
            name: r.name.clone(),
            field: r.field.clone(),
            multiple,
            on_delete: r.on_delete,
        };
        let text = serde_json::to_string(&document).map_err(|e| e.to_string())?;
        // On the child's column that holds the parents' ids; on its table when no row had the field.
        match child.columns.iter().find(|(a, _)| *a == r.field) {
            Some((_, column)) => gp.add_column_metadata(
                &child.table,
                column,
                RELATION_STANDARD,
                "application/json",
                &text,
                made,
            ),
            None => gp.add_metadata(
                &child.table,
                RELATION_STANDARD,
                "application/json",
                &text,
                made,
            ),
        }
        .map_err(|e| e.0)?;
        if dangling > 0 {
            out.left.push(format!(
                "“{}” ilişkisinde {dangling} başvuru dosyaya yazılmayan bir üst kayda gidiyor; eşlemede yok",
                r.name
            ));
        }
        out.relations.push((
            mapping,
            child.table.clone(),
            parent.table.clone(),
            pairs.len() as u32,
        ));
    }
    // Each feature table's look as QGIS's default style (an attribute table has none). A layer that keeps the
    // drawing's default look (no renderer, the theme's colour) has no look of its own to give: QGIS draws it with its
    // own default.
    let mut rows = Vec::new();
    for t in tables.iter().filter(|t| t.geometry.is_some()) {
        let Some(layer) = t.layer.as_deref() else {
            continue;
        };
        if doc
            .layers()
            .get(layer)
            .is_some_and(|n| n.style.renderer.is_none() && !n.style.color.starts_with('#'))
        {
            continue;
        }
        match crate::qml::export(doc, layer) {
            Ok((name, qml)) => rows.push(NewStyle {
                table: t.table.clone(),
                geometry: "geom".to_owned(),
                name,
                qml: qml.xml,
                sld: crate::sld::export(doc, layer).ok().map(|(_, s)| s.xml),
            }),
            Err(e) => out.left.push(format!(
                "“{}” katmanının görünüşü QGIS stili olarak yazılmadı: {}",
                layer_name(layer),
                e.message
            )),
        }
    }
    if !rows.is_empty() {
        gp.add_default_styles(&rows).map_err(|e| e.0)?;
        out.styles = rows.len() as u32;
    }
    Ok(out)
}

/// A relation the file declares, as the import takes it.
#[derive(Clone, Debug, PartialEq)]
pub struct FileRelation {
    /// Its row in `gpkgext_relations`.
    pub relation: Relation,
    /// KentOS's document of it; none for another writer's.
    pub document: Option<RelationDocument>,
}

/// A table's default style in the file: its renderer, the SVG drawings it carries and what it lost.
#[derive(Clone, Debug)]
pub struct FileStyle {
    pub renderer: serde_json::Value,
    pub assets: Vec<kentos_formats::sld::StyleAsset>,
    pub lost: Vec<String>,
}

/// What the file holds beside its tables, for the import.
#[derive(Clone, Debug, Default)]
pub struct Profile {
    pub relations: Vec<FileRelation>,
    /// A table's description as a dataset (docs/adr/0633 §5), by the table's name.
    pub datasets: BTreeMap<String, kentos_commands::DatasetInfo>,
    /// A table's default style, by the table's name.
    pub styles: BTreeMap<String, FileStyle>,
    /// What cannot be read, and why.
    pub left: Vec<String>,
}

/// The relations and default styles of the GeoPackage at `path`.
pub fn read_profile(path: &Path) -> Result<Profile, String> {
    let gp = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let mut out = Profile::default();
    let layers = gp.layers().map_err(|e| e.0)?;
    for relation in gp.relations().map_err(|e| e.0)? {
        if let Some(why) = gp.relation_problem(&relation, &layers).map_err(|e| e.0)? {
            out.left.push(format!(
                "“{}” ilişkisi (OGC 18-000) okunmadı: {why}",
                relation.mapping
            ));
            continue;
        }
        let document = gp
            .metadata(&relation.base_table)
            .map_err(|e| e.0)?
            .into_iter()
            .filter(|(standard, _)| standard == RELATION_STANDARD)
            .filter_map(|(_, text)| serde_json::from_str::<RelationDocument>(&text).ok())
            .find(|d| d.mapping == relation.mapping)
            .filter(|_| relation.name == KENTOS_RELATION);
        out.relations.push(FileRelation { relation, document });
    }
    for l in &layers {
        let found = gp
            .metadata(&l.name)
            .map_err(|e| e.0)?
            .into_iter()
            .find(|(standard, _)| standard == DATASET_STANDARD);
        if let Some((_, text)) = found {
            match serde_json::from_str::<kentos_commands::DatasetInfo>(&text) {
                Ok(d) if kentos_commands::dataset_problem(&d).is_none() => {
                    out.datasets.insert(l.name.clone(), d);
                }
                Ok(d) => out.left.push(format!(
                    "“{}” tablosunun veri kümesi bilgisi alınmadı: {}",
                    l.name,
                    kentos_commands::dataset_problem(&d).unwrap_or_default()
                )),
                Err(e) => out.left.push(format!(
                    "“{}” tablosunun veri kümesi bilgisi okunamadı: {e}",
                    l.name
                )),
            }
        }
    }
    for (table, text) in gp.default_styles().map_err(|e| e.0)? {
        match kentos_qml::read(&text) {
            Ok(read) => {
                if let Some(why) = crate::layers_renderer::renderer_problem(&read.renderer) {
                    out.left
                        .push(format!("“{table}” tablosunun QGIS stili alınmadı: {why}"));
                    continue;
                }
                out.styles.insert(
                    table,
                    FileStyle {
                        renderer: read.renderer,
                        assets: read.assets,
                        lost: read.lost,
                    },
                );
            }
            Err(why) => out
                .left
                .push(format!("“{table}” tablosunun QGIS stili okunmadı: {why}")),
        }
    }
    Ok(out)
}

/// A field key a layer takes from another writer's relation: its mapping table's name, `_2` … when the table has
/// such a field.
fn reference_field(table: &TableRead, mapping: &str) -> String {
    let stem = mapping.to_owned();
    let mut key = stem.clone();
    let mut n = 2;
    while table.fields.iter().any(|f| f.name == key) {
        key = format!("{stem}_{n}");
        n += 1;
    }
    key
}

/// Another writer's many-to-many relations whose two tables are both taken, as multi-valued reference fields of the
/// base tables: each base row's field holds its related rows' lasting ids, `; ` apart, in the mapping's order. The
/// field each relation got, by its mapping table; what was not taken is said in `left`.
pub fn take_foreign(
    path: &Path,
    profile: &Profile,
    tables: &mut [TableRead],
    left: &mut Vec<String>,
) -> Result<BTreeMap<String, String>, String> {
    let gp = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let mut fields = BTreeMap::new();
    // KentOS's relations: a multi-valued field is one again (its column said only text).
    for d in profile
        .relations
        .iter()
        .filter_map(|r| r.document.as_ref().map(|d| (r, d)))
    {
        let (r, d) = d;
        if !d.multiple {
            continue;
        }
        if let Some(t) = tables.iter_mut().find(|t| t.name == r.relation.base_table)
            && let Some(f) = t.fields.iter_mut().find(|f| f.name == d.field)
        {
            f.multiple = true;
        }
    }
    for r in profile.relations.iter().filter(|r| r.document.is_none()) {
        let rel = &r.relation;
        let (Some(b), Some(rt)) = (
            tables.iter().position(|t| t.name == rel.base_table),
            tables.iter().position(|t| t.name == rel.related_table),
        ) else {
            left.push(format!(
                "“{}” ilişkisi alınmadı: “{}” ve “{}” tablolarının ikisi birden alınmıyor",
                rel.mapping, rel.base_table, rel.related_table
            ));
            continue;
        };
        if b == rt {
            left.push(format!(
                "“{}” ilişkisi alınmadı: tablo kendisiyle ilişkili",
                rel.mapping
            ));
            continue;
        }
        let related: HashMap<i64, Uuid> = tables[rt]
            .fids
            .iter()
            .copied()
            .zip(tables[rt].objects.iter().map(|(u, _)| *u))
            .collect();
        let mut held: HashMap<i64, Vec<String>> = HashMap::new();
        let mut dangling = 0u32;
        for (base, rel_fid) in gp.mapping_rows(&rel.mapping).map_err(|e| e.0)? {
            match related.get(&rel_fid) {
                Some(u) => {
                    let ids = held.entry(base).or_default();
                    let id = u.to_string();
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
                None => dangling += 1,
            }
        }
        let key = reference_field(&tables[b], &rel.mapping);
        let t = &mut tables[b];
        let mut field = LayerField::new(key.clone(), LayerFieldKind::Text);
        field.multiple = true;
        t.fields.push(field);
        for (fid, (_, e)) in t.fids.iter().zip(t.objects.iter_mut()) {
            if let Some(ids) = held.get(fid) {
                e.base_mut().attrs.insert(key.clone(), ids.join("; "));
            }
        }
        if dangling > 0 {
            left.push(format!(
                "“{}” ilişkisinin {dangling} satırı alınmayan bir kayda gidiyor",
                rel.mapping
            ));
        }
        fields.insert(rel.mapping.clone(), key);
    }
    Ok(fields)
}

/// A relation id the project does not have: the given one, else with `-2`, `-3` ….
fn free_id(have: &[ProjectRelation], id: &str) -> String {
    let mut out = id.to_owned();
    let mut n = 2;
    while have.iter().any(|r| r.id == out) {
        out = format!("{id}-{n}");
        n += 1;
    }
    out
}

/// The file's relations whose two tables were taken, as the project's (`layers`: each table's layer; `foreign`: the
/// field another writer's relation got): the relations added (their ids); what was not taken in `left`.
pub fn add_relations(
    doc: &mut Document,
    profile: &Profile,
    layers: &BTreeMap<String, String>,
    foreign: &BTreeMap<String, String>,
    left: &mut Vec<String>,
) -> Vec<String> {
    let mut settings = doc.settings().clone();
    let mut added = Vec::new();
    for r in &profile.relations {
        let rel = &r.relation;
        let (Some(child), Some(parent)) =
            (layers.get(&rel.base_table), layers.get(&rel.related_table))
        else {
            if r.document.is_some() {
                left.push(format!(
                    "“{}” ilişkisi alınmadı: “{}” ve “{}” tablolarının ikisi birden alınmıyor",
                    rel.mapping, rel.base_table, rel.related_table
                ));
            }
            continue;
        };
        let is_layer = |id: &str| {
            doc.layers()
                .get(id)
                .is_some_and(|n| n.kind == LayerNodeType::Layer)
        };
        if !is_layer(child) || !is_layer(parent) {
            continue;
        }
        let made = match &r.document {
            Some(d) => ProjectRelation {
                id: free_id(&settings.relations, &d.id),
                name: d.name.clone(),
                parent: parent.clone(),
                child: child.clone(),
                field: d.field.clone(),
                on_delete: d.on_delete,
            },
            None => {
                let Some(field) = foreign.get(&rel.mapping) else {
                    continue;
                };
                let base = table_word(&rel.mapping).replace('_', "-");
                ProjectRelation {
                    id: free_id(&settings.relations, &base),
                    name: rel.mapping.clone(),
                    parent: parent.clone(),
                    child: child.clone(),
                    field: field.clone(),
                    on_delete: RelationDelete::Unlink,
                }
            }
        };
        // The same relation already in the project (taken before, or the drawing it came from): not twice.
        if settings
            .relations
            .iter()
            .any(|o| o.parent == made.parent && o.child == made.child && o.field == made.field)
        {
            continue;
        }
        if let Some(why) = kentos_commands::relation_problem(&made) {
            left.push(format!("“{}” ilişkisi alınmadı: {why}", rel.mapping));
            continue;
        }
        if settings.relations.len() >= kentos_commands::MOST_RELATIONS {
            left.push(format!(
                "“{}” ilişkisi alınmadı: projede en çok {} ilişki olur",
                rel.mapping,
                kentos_commands::MOST_RELATIONS
            ));
            continue;
        }
        let mut next = settings.relations.clone();
        next.push(made.clone());
        if let Some(why) = crate::relations_edit::cascade_problem(&next) {
            left.push(format!("“{}” ilişkisi alınmadı: {why}", rel.mapping));
            continue;
        }
        settings.relations = next;
        added.push(made.id);
    }
    if !added.is_empty() {
        doc.set_settings(settings);
    }
    added
}

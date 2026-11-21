//! `cad.layers.export` v1 (docs/adr/9106): a layer's objects or rows written to a GeoPackage or PostGIS table or to a
//! GeoJSON, Shapefile, CSV or KML file.
//!
//! What is written is read here: a drawing layer's objects (those its filter shows; `uids` only), a data layer's rows
//! read from their source a part at a time (`keys` only; the box asked of the source), both narrowed by `bbox` and
//! `filter`. Each object, in the project's system, is made a row of the target as an edit session's save makes one
//! (`data_write::Rows`): its fields renamed and given their listed values' labels, its values made the field's kind,
//! its geometry made the kind asked (an area's outline for a line, a closed line for an area), arcs chorded, moved into
//! the system written, Z and M as asked, multi-part when asked. An object that cannot be made one is left out and
//! counted (`export_skipped`). The rows go to the target as they come (layer_export_sinks.rs); `plan` makes them and
//! writes nothing.

use std::collections::HashMap;

use kentos_commands::{
    CommandResult, CommandWarning, Entity, ExportExisting, ExportFormat, ExportGeometry, LayerFieldKind, LayerFilter,
    LayersExport, LayersExported,
};
use kentos_data::{Cancel, Condition, Feature, Field, FieldKind, GeometryField, NewFeature, Request, Schema};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::data_write::Rows;
use crate::geopackage::{attribute_text, geometry_of, object_of};
use crate::layer_export_sinks::{Sink, sink_for};
use crate::layer_filter::{CompiledFilter, compile_filter};
use crate::sources::TableSystem;

/// Rows asked of a data layer's source, and handed to the target, at a time.
pub(crate) const CHUNK: usize = 2048;

pub(crate) fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

pub(crate) fn unreachable(e: impl std::fmt::Display) -> Stop {
    Stop::failed(error(codes::SOURCE_UNREACHABLE, format!("Okunamadı ya da yazılamadı: {e}"), None))
}

/// A field written: the layer's field, the name written, its kind, its listed values' labels.
pub(crate) struct Out {
    /// The layer's field as it is.
    pub field: kentos_commands::LayerField,
    pub from: String,
    pub to: String,
    pub kind: LayerFieldKind,
    pub length: Option<u32>,
    pub labels: Option<HashMap<String, String>>,
    pub multiple: bool,
}

/// The export, checked against the document.
pub(crate) struct Plan<'d> {
    pub input: &'d LayersExport,
    pub node: &'d kentos_commands::LayerNode,
    pub settings: &'d kentos_commands::ProjectSettings,
    pub fields: Vec<Out>,
    /// The table (or the virtual one of a file) the rows are made for.
    pub schema: Schema,
    /// The system written: an EPSG code; none for a local project.
    pub srid: Option<u32>,
    pub filter: Option<CompiledFilter>,
}

/// The table name a layer's name makes: its letters and digits, others `_`, Turkish letters kept as their bare ones.
pub fn table_name_of(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        let c = match c {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'İ' | 'I' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            c => c.to_ascii_lowercase(),
        };
        out.push(if c.is_ascii_alphanumeric() { c } else { '_' });
    }
    let out = out.trim_matches('_').to_owned();
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) { format!("k_{out}") } else { out }
}

/// The input's own checks: the layer named, where it goes, the format's own choices.
fn checked(input: &LayersExport) -> Result<(), Stop> {
    if input.layer.trim().is_empty() {
        return Err(fail(codes::NO_LAYER, "Dışa aktarılacak katmanı (layer) verin.".into(), "layer"));
    }
    match input.format {
        ExportFormat::Postgis if input.address.as_deref().is_none_or(|a| a.trim().is_empty()) => {
            return Err(fail(
                codes::NO_TARGET,
                "PostGIS'e yazmak için veritabanının adresini verin (address): postgresql://kullanıcı@sunucu:port/veritabanı.".into(),
                "address",
            ));
        }
        ExportFormat::Postgis => {}
        _ if input.path.as_deref().is_none_or(|p| p.trim().is_empty()) => {
            return Err(fail(codes::NO_TARGET, "Yazılacak dosyanın yolunu verin (path).".into(), "path"));
        }
        _ => {}
    }
    let option = |what: &str, path: &str| -> Result<(), Stop> {
        Err(fail(
            codes::INVALID_OPTION,
            format!("{what} bu biçimde yoktur; {} için verilmez.", format_words(input.format)),
            path,
        ))
    };
    let table = matches!(input.format, ExportFormat::Gpkg | ExportFormat::Postgis);
    if let Some(o) = &input.options {
        if !table {
            if o.description.is_some() {
                option("Açıklama", "options.description")?;
            }
            if o.key.is_some() {
                option("Anahtar sütunu", "options.key")?;
            }
            if o.geometry_column.is_some() {
                option("Geometri sütunu", "options.geometryColumn")?;
            }
            if o.no_spatial_index.is_some() {
                option("Mekânsal dizin", "options.noSpatialIndex")?;
            }
        }
        if input.format != ExportFormat::Csv {
            if o.separator.is_some() {
                option("Ayırıcı", "options.separator")?;
            }
            if o.xy.is_some() {
                option("X ve Y sütunları", "options.xy")?;
            }
        }
        if input.format != ExportFormat::Geojson && o.project_coordinates.is_some() {
            option("Proje koordinatları", "options.projectCoordinates")?;
        }
        if let Some(s) = &o.separator
            && s.chars().count() != 1
        {
            return Err(fail(codes::INVALID_OPTION, "Ayırıcı tek bir karakter olmalı.".into(), "options.separator"));
        }
    }
    if input.encoding.is_some() && !matches!(input.format, ExportFormat::Shapefile | ExportFormat::Csv) {
        option("Kodlama", "encoding")?;
    }
    if input.schema.is_some() && input.format != ExportFormat::Postgis {
        option("Şema", "schema")?;
    }
    if input.metadata == Some(true) && input.format != ExportFormat::Gpkg {
        option("Üst veri", "metadata")?;
    }
    if input.add_to_map == Some(true) && !table {
        option("Haritaya eklemek", "addToMap")?;
    }
    if matches!(input.existing, Some(ExportExisting::ReplaceLayer | ExportExisting::Append)) && !table {
        option("Katmanı değiştirmek ya da eklemek", "existing")?;
    }
    if input.format == ExportFormat::Postgis && input.existing == Some(ExportExisting::OverwriteFile) {
        option("Dosyanın üstüne yazmak", "existing")?;
    }
    if input.format == ExportFormat::Kml && input.srid.is_some_and(|s| s != 4326) {
        option("WGS 84'ten başka bir sistem", "srid")?;
    }
    if let Some(name) = &input.name
        && (name.trim().is_empty() || name.chars().count() > 63)
    {
        return Err(fail(codes::INVALID_NAME, "Tablonun adı boş ya da 63 karakterden uzun olamaz.".into(), "name"));
    }
    for (i, f) in input.fields.iter().flatten().enumerate() {
        if let Some(to) = &f.to {
            let most = if input.format == ExportFormat::Shapefile { 10 } else { 63 };
            if to.trim().is_empty() || to.chars().count() > most {
                return Err(fail(
                    codes::INVALID_NAME,
                    format!("Alanın yazılacak adı boş ya da {most} karakterden uzun olamaz."),
                    &format!("fields[{i}].to"),
                ));
            }
        }
    }
    Ok(())
}

/// The format's name in words.
pub(crate) fn format_words(f: ExportFormat) -> &'static str {
    match f {
        ExportFormat::Gpkg => "GeoPackage",
        ExportFormat::Postgis => "PostGIS",
        ExportFormat::Geojson => "GeoJSON",
        ExportFormat::Shapefile => "Shapefile",
        ExportFormat::Csv => "CSV",
        ExportFormat::Kml => "KML",
    }
}

/// A layer field's kind as a table's.
pub(crate) fn field_kind(kind: LayerFieldKind, length: Option<u32>) -> FieldKind {
    match kind {
        LayerFieldKind::Integer | LayerFieldKind::BigInt => FieldKind::Int,
        LayerFieldKind::Decimal => FieldKind::Real,
        LayerFieldKind::Boolean => FieldKind::Bool,
        LayerFieldKind::Date => FieldKind::Date,
        LayerFieldKind::DateTime | LayerFieldKind::Instant => FieldKind::DateTime,
        _ => FieldKind::Text { max: length },
    }
}

/// The geometry type the rows take: the kind asked (or the layer's), multi-part when asked; none: no geometry.
fn geometry_type(input: &LayersExport, node: &kentos_commands::LayerNode) -> Option<&'static str> {
    use kentos_commands::LayerGeometry as G;
    let kind = match input.geometry.unwrap_or_default() {
        ExportGeometry::None => return None,
        ExportGeometry::Point => Some(G::Point),
        ExportGeometry::Line => Some(G::Line),
        ExportGeometry::Area => Some(G::Area),
        ExportGeometry::Auto => node.geometry.or_else(|| {
            node.data.as_ref().and_then(|d| d.geometry.as_ref()).and_then(|g| {
                match kentos_data::schema::family_of(&g.type_name) {
                    kentos_data::Family::Point => Some(G::Point),
                    kentos_data::Family::Line => Some(G::Line),
                    kentos_data::Family::Area => Some(G::Area),
                    kentos_data::Family::Any => None,
                }
            })
        }),
    };
    let multi = input.multi == Some(true);
    Some(match (kind, multi) {
        (Some(G::Point), false) => "POINT",
        (Some(G::Point), true) => "MULTIPOINT",
        (Some(G::Line), false) => "LINESTRING",
        (Some(G::Line), true) => "MULTILINESTRING",
        (Some(G::Area), false) => "POLYGON",
        (Some(G::Area), true) => "MULTIPOLYGON",
        _ => "GEOMETRY",
    })
}

/// The export checked against the document: the layer, its fields, the condition, the system.
pub(crate) fn planned<'d>(doc: &'d Document, input: &'d LayersExport) -> Result<Plan<'d>, Stop> {
    checked(input)?;
    let id = input.layer.trim();
    let node = doc
        .layers()
        .get(id)
        .ok_or_else(|| fail(codes::LAYER_NOT_FOUND, format!("“{id}” kimlikli katman çizimde yok."), "layer"))?;
    if node.kind == kentos_commands::LayerNodeType::Group {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!("“{}” bir katman grubu; dışa aktarılan bir katmandır. Grubun bir katmanını verin.", node.name),
            "layer",
        ));
    }
    if node.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!("“{}” bir harita servisinden çizilir; nesnesi ya da satırı yoktur.", node.name),
            "layer",
        ));
    }
    let settings = doc.settings();
    let mut fields = Vec::new();
    let wanted: Vec<kentos_commands::ExportField> = match &input.fields {
        Some(f) => f.clone(),
        None => node
            .fields
            .iter()
            .map(|f| kentos_commands::ExportField {
                name: f.name.clone(),
                ..kentos_commands::ExportField::default()
            })
            .collect(),
    };
    for (i, w) in wanted.iter().enumerate() {
        let Some(f) = node.fields.iter().find(|f| f.name == w.name) else {
            return Err(fail(
                codes::FIELD_NOT_FOUND,
                format!("“{}” alanı “{}” katmanında yok.", w.name, node.name),
                &format!("fields[{i}].name"),
            ));
        };
        let labels = (w.labels == Some(true)).then(|| {
            f.values
                .iter()
                .flatten()
                .map(|c| (c.code.clone(), c.label.clone()))
                .collect::<HashMap<_, _>>()
        });
        // A label is text whatever the code's kind.
        let kind = if labels.is_some() { LayerFieldKind::Text } else { f.kind };
        fields.push(Out {
            field: f.clone(),
            from: f.name.clone(),
            to: w.to.clone().unwrap_or_else(|| f.name.clone()),
            kind,
            length: if labels.is_some() { None } else { f.length },
            labels,
            multiple: f.multiple,
        });
    }
    let filter = match input.filter.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(text) => {
            let unit = crate::expression_unit::reading_unit(settings);
            let f = compile_filter(
                &LayerFilter {
                    expression: Some(text.to_owned()),
                    ..LayerFilter::default()
                },
                unit,
            )
            .map_err(|why| fail(codes::INVALID_EXPRESSION, format!("Süzgeç: {why}"), "filter"))?;
            if f.expr().is_some_and(|e| !e.world.is_empty()) {
                return Err(fail(
                    codes::INVALID_EXPRESSION,
                    "Süzgeç başka nesnelere bakamaz: her nesne yalnız kendi alanlarıyla ve geometrisiyle sorulur.".into(),
                    "filter",
                ));
            }
            Some(f)
        }
    };
    // The system written: GeoJSON (unless the project's coordinates are asked) and KML are moved into WGS 84 by their
    // writers from the project's own; the others from the given one, a data table's own or the project's.
    let wgs84_writer = match input.format {
        ExportFormat::Kml => true,
        ExportFormat::Geojson => input.options.as_ref().and_then(|o| o.project_coordinates) != Some(true),
        _ => false,
    };
    let own = node.data.as_ref().and_then(|d| d.geometry.as_ref()).and_then(|g| g.srid);
    let project = (settings.srid > 0).then_some(settings.srid);
    let srid = if wgs84_writer { project } else { input.srid.or(own).or(project) };
    if let Some(s) = srid
        && let Err(why) = TableSystem::of(settings, Some(s))
    {
        return Err(fail(codes::UNKNOWN_SRID, format!("EPSG:{s} bu projede yazılamaz: {why}"), "srid"));
    }
    let key = input
        .options
        .as_ref()
        .and_then(|o| o.key.clone())
        .unwrap_or_else(|| "fid".to_owned());
    let geometry = geometry_type(input, node).map(|type_name| {
        let has = node.data.as_ref().and_then(|d| d.geometry.as_ref());
        GeometryField {
            name: input
                .options
                .as_ref()
                .and_then(|o| o.geometry_column.clone())
                .unwrap_or_else(|| "geom".to_owned()),
            type_name: type_name.to_owned(),
            z: input.z.unwrap_or_else(|| has.is_some_and(|g| g.z)),
            m: input.m.unwrap_or_else(|| has.is_some_and(|g| g.m)),
            srid: srid.and_then(|s| i32::try_from(s).ok()),
        }
    });
    let schema = Schema {
        key: key.clone(),
        key_made: true,
        fields: fields.iter().map(|o| Field::new(&o.to, field_kind(o.kind, o.length))).collect(),
        geometry,
    };
    Ok(Plan {
        input,
        node,
        settings,
        fields,
        schema,
        srid,
        filter,
    })
}

impl Out {
    /// The field as the written layer has it: its name, its kind (a label is text), no list when labels are written.
    pub(crate) fn written(&self) -> kentos_commands::LayerField {
        let mut f = self.field.clone();
        f.id = None;
        f.name = self.to.clone();
        f.kind = self.kind;
        f.length = self.length;
        if self.labels.is_some() {
            f.values = None;
            f.domain = None;
        }
        f
    }
}

/// The object with its attributes as written: the fields renamed, their listed values given as their labels.
fn mapped(plan: &Plan<'_>, e: &Entity) -> Entity {
    let mut out = e.clone();
    let attrs = &e.base().attrs;
    let mut written = std::collections::BTreeMap::new();
    for f in &plan.fields {
        let Some(v) = attrs.get(&f.from).filter(|v| !v.is_empty()) else {
            continue;
        };
        let v = match &f.labels {
            Some(labels) if f.multiple => v
                .split("; ")
                .map(|code| labels.get(code).cloned().unwrap_or_else(|| code.to_owned()))
                .collect::<Vec<_>>()
                .join("; "),
            Some(labels) => labels.get(v).cloned().unwrap_or_else(|| v.clone()),
            None => v.clone(),
        };
        written.insert(f.to.clone(), v);
    }
    out.base_mut().attrs = written;
    out
}

/// The run: the objects read, made rows, handed to the sink; what was written and what was left out.
/// How far an export is (the desktop's bar): the rows or objects read of those it reads, and the rows written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExportProgress {
    pub read: u64,
    /// What is read in all: the rows the source keeps for the box and the keys (counted before), a drawing layer's
    /// objects; none when the source cannot count them.
    pub total: Option<u64>,
    pub written: u64,
}

/// What a part of the reading says along with its objects: how many are read so far, of how many.
pub(crate) struct Reading {
    read: u64,
    total: Option<u64>,
}

pub(crate) struct Run {
    pub written: u64,
    pub skipped: u64,
    pub first_skip: Option<String>,
    /// The fields an `append` left out (the table lacks them).
    pub left_out: Vec<String>,
}

/// Makes the rows of `objects` (in the project's system) and hands them to the sink.
fn feed(plan: &Plan<'_>, rows: &Rows<'_>, objects: &[Entity], sink: &mut dyn Sink, run: &mut Run) -> Result<(), Stop> {
    let mut made: Vec<NewFeature> = Vec::with_capacity(objects.len());
    for e in objects {
        // A text or a dimension has no geometry a table holds: left out of a table with geometry.
        if plan.schema.geometry.is_some() && matches!(geometry_of(e), Ok(None) | Err(_)) {
            run.skipped += 1;
            run.first_skip.get_or_insert_with(|| "yazı ya da ölçü bir geometri sütununa yazılmaz".to_owned());
            continue;
        }
        match rows.new_row(&mapped(plan, e)) {
            Ok(row) => made.push(row),
            Err(why) => {
                run.skipped += 1;
                run.first_skip.get_or_insert(why);
            }
        }
    }
    run.written += made.len() as u64;
    sink.write(&made).map_err(unreachable)
}

/// The objects of a drawing layer the export keeps.
fn drawing_objects(doc: &Document, plan: &Plan<'_>) -> Result<Vec<Entity>, Stop> {
    let input = plan.input;
    let hidden = plan.node.filter.as_ref().map(|_| crate::layer_filter::left_out(doc));
    let wanted: Option<std::collections::HashSet<&str>> =
        input.uids.as_ref().map(|u| u.iter().map(String::as_str).collect());
    let mut list: Vec<Entity> = Vec::new();
    for (uid, e) in doc.by_layer_with_uids(&plan.node.id) {
        if hidden.as_ref().is_some_and(|h| h.contains(&kentos_domain::Slot(e.base().id))) {
            continue;
        }
        if wanted.as_ref().is_some_and(|w| !w.contains(uid.to_string().as_str())) {
            continue;
        }
        if let Some(b) = input.bbox {
            let meets = geometry_of(e)
                .ok()
                .flatten()
                .and_then(|(g, _)| g.bounds())
                .is_some_and(|g| g[0] <= b[2] && g[2] >= b[0] && g[1] <= b[3] && g[3] >= b[1]);
            if !meets {
                continue;
            }
        }
        list.push(e.clone());
    }
    if let Some(f) = &plan.filter {
        let refs: Vec<&Entity> = list.iter().collect();
        let shapes: Vec<_> = if f.reads_geometry() {
            refs.iter().map(|e| crate::geometry::shape(e)).collect()
        } else {
            Vec::new()
        };
        let names = |id: &str| doc.layers().get(id).map(|n| n.name.clone()).unwrap_or_default();
        let held = f.condition(&refs, &names, |i| shapes.get(i));
        list = list.into_iter().zip(held).filter(|(_, h)| *h).map(|(e, _)| e).collect();
    }
    Ok(list)
}

/// A data layer's rows the export keeps, read from the source a part at a time, each part handed on.
fn data_rows(
    plan: &Plan<'_>,
    stopped: &dyn Fn() -> bool,
    each: &mut dyn FnMut(Vec<Entity>, Reading) -> Result<(), Stop>,
) -> Result<(), Stop> {
    let Some(data) = &plan.node.data else {
        return Ok(());
    };
    let input = plan.input;
    let source = kentos_data::registry().source(&data.address).map_err(unreachable)?;
    let srid = data.geometry.as_ref().and_then(|g| g.srid);
    let system = TableSystem::of(plan.settings, srid).map_err(unreachable)?;
    let bbox = match (input.bbox, &system, &data.geometry) {
        (None, ..) | (_, _, None) => None,
        (Some(b), None, Some(_)) => Some(b),
        (Some(b), Some(s), Some(_)) => Some(
            s.to_table(b)
                .ok_or_else(|| fail(codes::INVALID_EXTENT, "Kutu tablonun sistemine çevrilemedi.".into(), "bbox"))?,
        ),
    };
    let known: Vec<String> = plan.node.fields.iter().map(|f| f.name.clone()).collect();
    let mut columns: Vec<String> = plan.fields.iter().map(|f| f.from.clone()).collect();
    if let Some(e) = plan.filter.as_ref().and_then(|f| f.expr()) {
        for f in &e.fields {
            if known.contains(f) && !columns.contains(f) {
                columns.push(f.clone());
            }
        }
    }
    let request = Request {
        bbox,
        condition: input.keys.as_ref().map(|k| Condition::Fids(k.clone())),
        columns: Some(columns.clone()),
        geometry: data.geometry.is_some(),
        chunk: CHUNK,
        ..Request::default()
    };
    // The rows read in all, counted first for the bar (a count is a fraction of the reading's time).
    let total = source.count(&data.table, &request.counted(), false).ok();
    let cancel = Cancel::new();
    let mut failure: Option<Stop> = None;
    let mut next_id = 1u32;
    let mut read = 0u64;
    source
        .features(&data.table, &request, &cancel, &mut |part: Vec<Feature>| {
            read += part.len() as u64;
            let mut objects: Vec<Entity> = Vec::with_capacity(part.len());
            for f in &part {
                let attrs: serde_json::Map<String, serde_json::Value> = columns
                    .iter()
                    .zip(&f.values)
                    .filter_map(|(n, v)| attribute_text(v).map(|t| (n.clone(), serde_json::Value::String(t))))
                    .collect();
                let g = match (f.shape(), &system) {
                    (Some(g), Some(s)) => s.moved(g),
                    (Some(g), None) => Some(g.clone()),
                    // A row without geometry is written as a record (a table without geometry) or left out.
                    (None, _) => None,
                };
                let shape = g.unwrap_or(kentos_data::Geometry::Point(None));
                if let Some(e) = object_of(&shape, &plan.node.id, next_id, attrs) {
                    next_id = next_id.wrapping_add(1);
                    objects.push(e);
                }
            }
            if let Some(f) = &plan.filter {
                let refs: Vec<&Entity> = objects.iter().collect();
                let shapes: Vec<_> = if f.reads_geometry() {
                    refs.iter().map(|e| crate::geometry::shape(e)).collect()
                } else {
                    Vec::new()
                };
                let name = plan.node.name.clone();
                let held = f.condition(&refs, &|_| name.clone(), |i| shapes.get(i));
                objects = objects.into_iter().zip(held).filter(|(_, h)| *h).map(|(e, _)| e).collect();
            }
            if let Err(stop) = each(objects, Reading { read, total }) {
                failure = Some(stop);
                return false;
            }
            !stopped()
        })
        .map_err(unreachable)?;
    if let Some(stop) = failure {
        return Err(stop);
    }
    if stopped() {
        return Err(Stop::failed(error(codes::CANCELLED, "İş durduruldu; hiçbir şey yazılmadı.".into(), None)));
    }
    Ok(())
}

/// Reads what the export keeps and writes it (`write`: false, a plan: nothing is written), saying how far it is after
/// each part. A stop or a failure takes back what it wrote: the transaction is rolled back, a file it made removed.
pub(crate) fn run(
    doc: &Document,
    plan: &Plan<'_>,
    write: bool,
    stopped: &dyn Fn() -> bool,
    progress: &dyn Fn(ExportProgress),
) -> Result<(Run, Vec<String>), Stop> {
    let system = TableSystem::of(plan.settings, plan.srid).map_err(unreachable)?;
    let curves = false;
    let rows = Rows::new(&plan.schema, curves, system.as_ref());
    let mut sink: Box<dyn Sink> = sink_for(doc, plan, write)?;
    let mut run = Run {
        written: 0,
        skipped: 0,
        first_skip: None,
        left_out: sink.left_out(),
    };
    let fed = if plan.node.data.is_some() {
        data_rows(plan, stopped, &mut |objects, at| {
            feed(plan, &rows, &objects, sink.as_mut(), &mut run)?;
            progress(ExportProgress {
                read: at.read,
                total: at.total,
                written: run.written,
            });
            Ok(())
        })
    } else {
        drawing_objects(doc, plan).and_then(|objects| {
            let total = objects.len() as u64;
            let mut read = 0u64;
            for part in objects.chunks(CHUNK) {
                if stopped() {
                    return Err(Stop::failed(error(codes::CANCELLED, "İş durduruldu; hiçbir şey yazılmadı.".into(), None)));
                }
                feed(plan, &rows, part, sink.as_mut(), &mut run)?;
                read += part.len() as u64;
                progress(ExportProgress {
                    read,
                    total: Some(total),
                    written: run.written,
                });
            }
            Ok(())
        })
    };
    if let Err(stop) = fed {
        sink.abandon();
        return Err(stop);
    }
    if run.written == 0 {
        sink.abandon();
        return Err(fail(
            codes::NOTHING_TO_EXPORT,
            match &run.first_skip {
                Some(why) => format!("Yazılacak nesne kalmadı: {} nesne yazılamadı ({why}).", run.skipped),
                None => "Yazılacak nesne kalmadı: seçim, kutu ve süzgeç hiçbir nesne bırakmadı.".to_owned(),
            },
            "layer",
        ));
    }
    let files = sink.finish().map_err(unreachable)?;
    Ok((run, files))
}

/// The output for what was written.
fn exported(plan: &Plan<'_>, run: &Run, files: Vec<String>, layer: Option<String>, revision: String) -> LayersExported {
    let geometry = plan.schema.geometry.as_ref().map(|g| {
        let dims = match (g.z, g.m) {
            (true, true) => " ZM",
            (true, false) => " Z",
            (false, true) => " M",
            (false, false) => "",
        };
        format!("{}{dims}", g.type_name)
    });
    let srid = match plan.input.format {
        ExportFormat::Kml => Some(4326),
        ExportFormat::Geojson if plan.input.options.as_ref().and_then(|o| o.project_coordinates) != Some(true) => {
            Some(4326)
        }
        ExportFormat::Csv => None,
        _ => plan.srid,
    };
    LayersExported {
        format: plan.input.format,
        files,
        table: crate::layer_export_sinks::table_of(plan),
        written: run.written,
        fields: plan.fields.iter().map(|f| f.to.clone()).collect(),
        geometry,
        srid,
        layer,
        revision,
    }
}

fn warnings_of(run: &Run) -> Vec<CommandWarning> {
    let mut out: Vec<CommandWarning> = run
        .left_out
        .iter()
        .map(|f| CommandWarning {
            code: codes::FIELD_LEFT_OUT.into(),
            message: format!("Eklenen tabloda “{f}” alanı yok; değerleri yazılmadı."),
            path: Some("fields".to_owned()),
        })
        .collect();
    if let Some(why) = &run.first_skip {
        out.push(CommandWarning {
            code: codes::EXPORT_SKIPPED.into(),
            message: format!("{} nesne yazılamadı; ilki: {why}.", run.skipped),
            path: None,
        });
    }
    out
}

/// Why a stop stopped, in words.
fn message(stop: Stop) -> String {
    match CommandResult::<()>::from(stop) {
        CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
            error.message
        }
        _ => "Dışa aktarılamadı.".to_owned(),
    }
}

/// The export written off the interface's thread (the desktop's Farklı kaydet…): what was written and said, and with
/// `addToMap` the data layer the window adds once it is back (its own undo step); the message of why not.
pub fn written(
    doc: &Document,
    input: &LayersExport,
    stopped: &dyn Fn() -> bool,
    progress: &dyn Fn(ExportProgress),
) -> Result<(LayersExported, Vec<CommandWarning>, Option<kentos_commands::LayersData>), String> {
    let plan = planned(doc, input).map_err(message)?;
    let (run, files) = run(doc, &plan, true, stopped, progress).map_err(message)?;
    let add = (input.add_to_map == Some(true))
        .then(|| crate::layer_export_sinks::data_layer_of(&plan))
        .flatten()
        .map(|(data, mut fields, name)| {
            fields.retain(|f| !run.left_out.contains(&f.name));
            kentos_commands::LayersData {
                operation: kentos_commands::LayerDataOperation::Add,
                layer: None,
                name: Some(name),
                parent: None,
                index: None,
                data: Some(data),
                fields: Some(fields),
                expected_revision: None,
            }
        });
    Ok((exported(&plan, &run, files, None, doc.revision().to_string()), warnings_of(&run), add))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &LayersExport) -> CommandResult<()> {
    match planned(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write: every row made, nothing written.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersExport) -> CommandResult<LayersExported> {
    let plan = match planned(cx.doc, input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    match run(cx.doc, &plan, false, &|| false, &|_| {}) {
        Ok((run, files)) => CommandResult::Completed {
            output: exported(&plan, &run, files, None, cx.doc.revision().to_string()),
            warnings: warnings_of(&run),
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the export; with `addToMap` the written table is added as a data layer, the export's undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersExport) -> CommandResult<LayersExported> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_EXPORT,
        kentos_commands::CAD_LAYERS_EXPORT_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` journals it as one step of the document.
fn write_execute(cx: &mut ExecutionContext<'_>, input: LayersExport) -> CommandResult<LayersExported> {
    let (run, files, add) = {
        let plan = match planned(cx.doc, &input) {
            Ok(p) => p,
            Err(stop) => return stop.into(),
        };
        // The protocol's plan runs this on a reading copy: nothing written then (docs/adr/0532 §4).
        let write = !cx.dry();
        let (run, files) = match run(cx.doc, &plan, write, &|| false, &|_| {}) {
            Ok(r) => r,
            Err(stop) => return stop.into(),
        };
        let add = (input.add_to_map == Some(true)).then(|| crate::layer_export_sinks::data_layer_of(&plan));
        let out = exported(&plan, &run, files.clone(), None, String::new());
        (run, out, add)
    };
    let mut out = files;
    let mut warnings = warnings_of(&run);
    if let Some(Some((data, mut fields, name))) = add {
        // An append's table keeps its own fields: the ones it lacks are not the layer's either.
        fields.retain(|f| !run.left_out.contains(&f.name));
        let added = crate::layers_data::write_execute(
            cx,
            kentos_commands::LayersData {
                operation: kentos_commands::LayerDataOperation::Add,
                layer: None,
                name: Some(name),
                parent: None,
                index: None,
                data: Some(data),
                fields: Some(fields),
                expected_revision: None,
            },
        );
        match added {
            CommandResult::Completed { output, warnings: w } => {
                out.layer = Some(output.layer);
                warnings.extend(w);
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                warnings.push(CommandWarning {
                    code: error.code,
                    message: format!("Yazıldı ama haritaya eklenemedi: {}", error.message),
                    path: Some("addToMap".to_owned()),
                });
            }
            _ => {}
        }
    }
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings }
}

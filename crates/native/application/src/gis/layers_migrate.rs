//! `cad.layers.migrate` v1 (docs/adr/9107 §8): a drawing layer of a GIS project moved into a data source. Its objects
//! are written as `cad.layers.export` writes them (every one, those its layer filter hides too), into a new table; the
//! objects leave the drawing and the layer itself reads the table, under its own id, name and look (docs/adr/9321), so
//! every reference to it stays: a network's edges, a relation, a form, a join, a view. All in one undo step: undone,
//! the drawing layer and its objects are back and the tables stay in their source.
//!
//! Its texts go into a table of their own beside it, read by a data layer that draws its rows as texts, above the
//! layer's (docs/adr/9114 §2, layers_migrate_texts.rs); a layer of texts only becomes that table, in its place. A layer
//! holding what no table holds (a dimension, a block, a hatch …) is refused before anything is written: removing the
//! layer would lose them. Counted first by a dry run of the export (nothing written).
//!
//! A table there already (docs/adr/9339) is refused unless `existing` says to make it again (`replaceLayer`) or to add
//! the rows to it (`append`); the texts' table beside it goes the same way.

use kentos_commands::{
    CommandResult, CommandWarning, CreateTarget, DataLayer, Entity, ExportExisting, ExportFormat,
    LayerDataOperation, LayerField, LayerNodeType, LayersData, LayersExport, LayersMigrate,
    LayersMigrated, MigratedTexts, TextEntity,
};
use kentos_domain::{Document, ServedBy, Slot};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::layers_migrate_texts as texts;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The export that writes the layer's objects into the table.
fn export_of(input: &LayersMigrate) -> LayersExport {
    let gpkg = input.target == CreateTarget::Gpkg;
    LayersExport {
        layer: input.layer.trim().to_owned(),
        format: if gpkg {
            ExportFormat::Gpkg
        } else {
            ExportFormat::Postgis
        },
        path: gpkg.then(|| input.path.clone().unwrap_or_default()),
        address: (!gpkg).then(|| input.address.clone().unwrap_or_default()),
        schema: (!gpkg).then(|| input.schema.clone().unwrap_or_else(|| "public".to_owned())),
        name: input
            .table
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_owned),
        existing: input.existing.filter(|e| *e != ExportExisting::Refuse),
        ..LayersExport::default()
    }
}

/// The source the tables go into: the GeoPackage's path, the database's address.
fn address_of(export: &LayersExport) -> String {
    export
        .path
        .clone()
        .or(export.address.clone())
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// The migration's own checks, in the contract's order, before the export's.
fn checked(doc: &Document, input: &LayersMigrate) -> Result<(), Stop> {
    let id = input.layer.trim();
    if id.is_empty() {
        return Err(fail(
            codes::NO_LAYER,
            "Taşınacak katman verilmedi (layer).".into(),
            "layer",
        ));
    }
    match input.target {
        CreateTarget::Gpkg if input.path.as_deref().is_none_or(|p| p.trim().is_empty()) => {
            return Err(fail(
                codes::NO_TARGET,
                "Tablonun yapılacağı GeoPackage dosyası verilmedi (path).".into(),
                "path",
            ));
        }
        CreateTarget::Postgis if input.address.as_deref().is_none_or(|a| a.trim().is_empty()) => {
            return Err(fail(
                codes::NO_TARGET,
                "Tablonun yapılacağı veritabanı verilmedi (address).".into(),
                "address",
            ));
        }
        _ => {}
    }
    if input.existing == Some(ExportExisting::OverwriteFile) {
        return Err(fail(
            codes::INVALID_OPTION,
            "Taşımada dosyanın üstüne yazılmaz (existing: overwriteFile): kaynağın öbür tabloları silinirdi. Var olan tablo için replaceLayer ya da append verin.".into(),
            "existing",
        ));
    }
    let node = doc.layers().get(id).ok_or_else(|| {
        fail(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok."),
            "layer",
        )
    })?;
    if node.kind == LayerNodeType::Group {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!(
                "“{}” bir katman grubu; kaynağa taşınan bir katmandır. Grubun bir katmanını verin.",
                node.name
            ),
            "layer",
        ));
    }
    if node.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!(
                "“{}” bir harita servisinden çizilir; taşınacak nesnesi yoktur.",
                node.name
            ),
            "layer",
        ));
    }
    if let Some(d) = &node.data {
        return Err(fail(
            codes::DATA_LAYER,
            format!(
                "“{}” zaten “{}” tablosunu okuyor; nesneleri kaynağında.",
                node.name, d.table
            ),
            "layer",
        ));
    }
    if doc.layers().is_locked(id) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{}” kilitli; nesneleri taşınmaz. Kilidi açıp yeniden deneyin.",
                node.name
            ),
            "layer",
        ));
    }
    Ok(())
}

/// An export's refusal in the migration's words: the table's name is its `table`, not the export's `name`.
fn as_migration(stop: Stop) -> Stop {
    match stop {
        Stop::Failed(mut e) if e.path.as_deref() == Some("name") => {
            e.path = Some("table".to_owned());
            Stop::Failed(e)
        }
        other => other,
    }
}

/// The export's plan of the layer's objects a table holds: every one, its texts left to their own table.
fn plan_of<'d>(
    doc: &'d Document,
    export: &'d LayersExport,
) -> Result<crate::layer_export::Plan<'d>, Stop> {
    let mut plan = crate::layer_export::planned(doc, export).map_err(as_migration)?;
    plan.all_objects = true;
    plan.texts_apart = true;
    Ok(plan)
}

/// What the migration writes.
struct Counted<'d> {
    /// The layer's table: a GeoPackage's table name, PostGIS's `schema.table`.
    table: String,
    /// The dry run of the export of the objects a table holds; none for a layer of texts only.
    run: Option<crate::layer_export::Run>,
    /// The layer's texts, and their table (the layer's own for a layer of texts only).
    texts: Vec<&'d TextEntity>,
    texts_table: Option<String>,
}

/// The layer's objects counted by a dry export: written, and what no table holds (refused then).
fn counted<'d>(
    doc: &'d Document,
    input: &LayersMigrate,
    export: &'d LayersExport,
) -> Result<Counted<'d>, Stop> {
    checked(doc, input)?;
    let id = input.layer.trim();
    let plan = plan_of(doc, export)?;
    let table = crate::layer_export_sinks::table_of(&plan).unwrap_or_default();
    let list = texts::of(doc, id);
    let only_texts = !list.is_empty() && !texts::others(doc, id);
    let texts_table = (!list.is_empty()).then(|| {
        if only_texts {
            table.clone()
        } else {
            texts::beside(&table)
        }
    });
    // The tables there already: said for the migration (the export's words name its `existing`, which this has not).
    let mut tables: Vec<&String> = Vec::new();
    if !only_texts {
        tables.push(&table);
    }
    tables.extend(texts_table.iter());
    let refused = matches!(input.existing, None | Some(ExportExisting::Refuse));
    for name in tables.into_iter().filter(|_| refused) {
        if table_there(export, name)? {
            return Err(fail(
                codes::TARGET_EXISTS,
                format!("“{name}” tablosu kaynakta var; başka bir tablo adı verin (table)."),
                "table",
            ));
        }
    }
    if only_texts {
        return Ok(Counted {
            table,
            run: None,
            texts: list,
            texts_table,
        });
    }
    let (run, _) = crate::layer_export::run(doc, &plan, false, &|| false, &|_| {})?;
    if run.skipped > 0 {
        let first = first_left(doc, id)
            .or_else(|| run.first_skip.clone())
            .unwrap_or_default();
        return Err(fail(
            codes::UNWRITABLE_OBJECTS,
            format!(
                "Katmanın {} nesnesi bir tabloya yazılamaz ({first}); katman taşınırsa kaybolurdu, hiçbir şey yazılmadı. Onları başka bir katmana alıp yeniden deneyin.",
                run.skipped
            ),
            "layer",
        ));
    }
    Ok(Counted {
        table,
        run: Some(run),
        texts: list,
        texts_table,
    })
}

/// The first object of the layer a table has no geometry for, its texts aside, in words: “ölçü bir geometri sütununa
/// yazılmaz”.
fn first_left(doc: &Document, layer: &str) -> Option<String> {
    doc.by_layer_with_uids(layer).find_map(|(_, e)| match e {
        Entity::Text(_) => None,
        e => crate::geopackage::geometry_of(e)
            .err()
            .map(|word| format!("{word} bir geometri sütununa yazılmaz")),
    })
}

/// Whether the table is there: a GeoPackage file that is not there has none.
fn table_there(export: &LayersExport, table: &str) -> Result<bool, Stop> {
    let address = address_of(export);
    if export.format == ExportFormat::Gpkg && !std::path::Path::new(&address).exists() {
        return Ok(false);
    }
    let unreachable = |e: kentos_data::DataError| {
        Stop::failed(error(
            codes::SOURCE_UNREACHABLE,
            format!("Okunamadı ya da yazılamadı: {e}"),
            None,
        ))
    };
    let source = kentos_data::registry()
        .source(&address)
        .map_err(unreachable)?;
    Ok(source
        .tables()
        .map_err(unreachable)?
        .iter()
        .any(|t| t.name == table))
}

/// The texts' details their table has no column for, said (docs/adr/9114 §2).
fn warnings_of(list: &[&TextEntity]) -> Vec<CommandWarning> {
    let left = texts::details_left(list);
    if left == 0 {
        return Vec::new();
    }
    vec![CommandWarning {
        code: codes::TEXT_DETAILS_LEFT.into(),
        message: format!(
            "{left} yazının stili, zemini, eğrisi, ölçekleri, nesne bağı ya da paragraf biçimi yazı tablosuna yazılmadı; metni, noktası, yüksekliği, açısı, hizası ve genişlik çarpanı yazıldı."
        ),
        path: Some("layer".into()),
    }]
}

pub fn validate(cx: &ExecutionContext<'_>, input: &LayersMigrate) -> CommandResult<()> {
    let export = export_of(input);
    match counted(cx.doc, input, &export) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do: the rows it would write; nothing written.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersMigrate) -> CommandResult<LayersMigrated> {
    let export = export_of(input);
    let c = match counted(cx.doc, input, &export) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let geometry = match &c.run {
        Some(_) => match plan_of(cx.doc, &export) {
            Ok(p) => p.schema.geometry.as_ref().map(|g| g.type_name.clone()),
            Err(stop) => return stop.into(),
        },
        None => Some("POINT".to_owned()),
    };
    let n = c.texts.len() as u64;
    CommandResult::Completed {
        output: LayersMigrated {
            layer: String::new(),
            table: c.table.clone(),
            address: address_of(&export),
            written: c.run.as_ref().map_or(n, |r| r.written),
            geometry,
            texts: c
                .run
                .as_ref()
                .and(c.texts_table.clone())
                .map(|table| MigratedTexts {
                    layer: String::new(),
                    table,
                    written: n,
                }),
            revision: cx.doc.revision().to_string(),
        },
        warnings: warnings_of(&c.texts),
    }
}

/// Moves the layer into the source, the call's one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersMigrate,
) -> CommandResult<LayersMigrated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_MIGRATE,
        kentos_commands::CAD_LAYERS_MIGRATE_VERSION,
        input,
        write_execute,
    )
}

/// A data layer added in the layer's place (its parent, its index): its id.
fn add(
    cx: &mut ExecutionContext<'_>,
    name: String,
    place: (Option<String>, usize),
    data: DataLayer,
    fields: Vec<LayerField>,
    activate: bool,
) -> Result<String, Box<CommandResult<LayersMigrated>>> {
    let added = crate::layers_data::write_activated(
        cx,
        LayersData {
            operation: LayerDataOperation::Add,
            layer: None,
            name: Some(name),
            parent: place.0,
            index: u32::try_from(place.1).ok(),
            data: Some(data),
            fields: Some(fields),
            expected_revision: None,
        },
        activate,
    );
    match added {
        CommandResult::Completed { output, .. } => Ok(output.layer),
        CommandResult::Failed { error } => Err(Box::new(CommandResult::Failed { error })),
        CommandResult::Conflict { error } => Err(Box::new(CommandResult::Conflict { error })),
        CommandResult::NeedsInput { error } => Err(Box::new(CommandResult::NeedsInput { error })),
        CommandResult::Queued { job_id } => Err(Box::new(CommandResult::Queued { job_id })),
        CommandResult::Cancelled => Err(Box::new(CommandResult::Cancelled)),
    }
}

/// The layer's table as written: its rows, its geometry type, the data layer reading it and its fields.
type Written = (u64, Option<String>, DataLayer, Vec<LayerField>);

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersMigrate,
) -> CommandResult<LayersMigrated> {
    let export = export_of(&input);
    let id = input.layer.trim().to_owned();
    let gpkg = input.target == CreateTarget::Gpkg;
    let address = address_of(&export);
    let file_there = gpkg && std::path::Path::new(&address).exists();
    let settings = cx.doc.settings().clone();
    let srid = (settings.srid > 0).then_some(settings.srid);
    // What is written, counted on the drawing as it is: the layer's table and its texts.
    let (table, main, texts_table, list, warnings) = {
        let c = match counted(cx.doc, &input, &export) {
            Ok(c) => c,
            Err(stop) => return stop.into(),
        };
        let main: Option<Written> = match &c.run {
            None => None,
            Some(_) => {
                // The table written (none on the protocol's reading copy: docs/adr/0532 §4) and the data layer it makes.
                let plan = match plan_of(cx.doc, &export) {
                    Ok(p) => p,
                    Err(stop) => return stop.into(),
                };
                let (run, _) =
                    match crate::layer_export::run(cx.doc, &plan, !cx.dry(), &|| false, &|_| {}) {
                        Ok(r) => r,
                        Err(stop) => return stop.into(),
                    };
                let Some((data, fields, _)) = crate::layer_export_sinks::data_layer_of(&plan)
                else {
                    return fail(
                        codes::SOURCE_UNREACHABLE,
                        "Yazılan tablonun yeri okunamadı.".into(),
                        "target",
                    )
                    .into();
                };
                Some((
                    run.written,
                    plan.schema.geometry.as_ref().map(|g| g.type_name.clone()),
                    data,
                    fields,
                ))
            }
        };
        let list: Vec<TextEntity> = c.texts.iter().map(|t| (*t).clone()).collect();
        (
            c.table.clone(),
            main,
            c.texts_table.clone(),
            list,
            warnings_of(&c.texts),
        )
    };
    // The texts' table; the layer's table taken back when it cannot be written, so nothing is moved by halves.
    let texts_written = match &texts_table {
        Some(t) if !cx.dry() => {
            let refs: Vec<&TextEntity> = list.iter().collect();
            match texts::write(&address, gpkg, t, srid, &settings, &refs, input.existing) {
                Ok(n) => n,
                Err(why) => {
                    // A table made for the move is taken back; one there before (replaced, added to) is not dropped.
                    let made = matches!(input.existing, None | Some(ExportExisting::Refuse));
                    if main.is_some() && made {
                        texts::take_back(&address, &table, gpkg && !file_there);
                    }
                    let kept = if made {
                        "Katmanın tablosu geri alındı; hiçbir şey taşınmadı"
                    } else {
                        "Katmanın tablosuna yazılanlar kaynakta kaldı; katman çizimde kaldı"
                    };
                    return fail(
                        codes::SOURCE_UNREACHABLE,
                        format!("Yazılar “{t}” tablosuna yazılamadı: {why}. {kept}."),
                        "layer",
                    )
                    .into();
                }
            }
        }
        Some(_) => list.len() as u64,
        None => 0,
    };
    let Some(old) = cx.doc.layers().get(&id).cloned() else {
        return fail(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok."),
            "layer",
        )
        .into();
    };
    let (parent, index) = cx.doc.layers().place_of(&id).unwrap_or((None, 0));
    let mut output = LayersMigrated {
        layer: String::new(),
        table: table.clone(),
        address: address.clone(),
        written: texts_written,
        geometry: Some("POINT".to_owned()),
        texts: None,
        revision: String::new(),
    };
    // The layer's objects are its table's rows now (docs/adr/9321): they leave the drawing, and the layer itself reads
    // the table under its own id, name and look (a layer of texts only: its texts' table). Every reference to it stays:
    // a network's edges and junctions, a relation, a form, a join, a view, its filter, its time.
    let beside = main.is_some();
    let own = match &main {
        Some((_, _, data, fields)) => (data.clone(), fields.clone()),
        None => match &texts_table {
            Some(t) => texts::data_layer(gpkg, &address, t, srid),
            None => {
                return fail(
                    codes::NOTHING_TO_EXPORT,
                    "Taşınacak nesne yok.".into(),
                    "layer",
                )
                .into();
            }
        },
    };
    if let Err(why) = read_in_place(cx.doc, &id, own.0, own.1) {
        return fail(codes::LAYER_REFUSED, why, "layer").into();
    }
    output.layer = id.clone();
    if let Some((written, geometry, _, _)) = main {
        output.written = written;
        output.geometry = geometry;
    }
    // A layer's texts beside other objects: a data layer of their own above it, in the layer's colour.
    if let Some(t) = texts_table.as_ref().filter(|_| beside) {
        let (data, fields) = texts::data_layer(gpkg, &address, t, srid);
        let layer = match add(
            cx,
            format!("{} (yazılar)", old.name),
            (parent, index),
            data,
            fields,
            false,
        ) {
            Ok(l) => l,
            Err(r) => return *r,
        };
        if let Some(mut style) = cx.doc.layers().get(&layer).map(|n| n.style.clone()) {
            style.color = old.style.color.clone();
            cx.doc.set_layer_style(&layer, style, LABEL);
        }
        output.texts = Some(MigratedTexts {
            layer,
            table: t.clone(),
            written: texts_written,
        });
    }
    output.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output, warnings }
}

/// The layer `id`'s objects taken out of the drawing and the layer made to read `data` with its table's `fields` and
/// the table's geometry kind, as the step [`LABEL`]; why the drawing refused, in its words (nothing changed then).
fn read_in_place(
    doc: &mut Document,
    id: &str,
    data: DataLayer,
    fields: Vec<LayerField>,
) -> Result<(), String> {
    let objects: Vec<Slot> = doc.by_layer(id).map(|e| Slot(e.base().id)).collect();
    let family = data.family();
    doc.transact(LABEL, |doc| {
        doc.remove(&objects);
        doc.set_layer_service(
            id,
            ServedBy {
                data: Some(data),
                ..ServedBy::default()
            },
            None,
            LABEL,
        )?;
        doc.set_layer_fields(id, fields, &[])?;
        doc.set_layer_geometry(id, family, LABEL)?;
        Ok(())
    })
    .map_err(|r: kentos_domain::Refusal| r.0)
}

/// The step's name.
pub const LABEL: &str = "Katmanı kaynağa taşı";

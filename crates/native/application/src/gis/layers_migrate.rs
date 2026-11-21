//! `cad.layers.migrate` v1 (docs/adr/9107 §8): a drawing layer of a GIS project moved into a data source. Its objects
//! are written as `cad.layers.export` writes them (every one, those its layer filter hides too), into a new table; a
//! data layer reading that table takes the layer's place with its name and style (its renderer and labels), active if
//! it was; the drawing layer goes with its objects. All in one undo step: undone, the drawing layer and its objects
//! are back and the table stays in its source.
//!
//! A layer holding what a table cannot (a text, a dimension) is refused before anything is written: removing the
//! layer would lose them. Counted first by a dry run of the export (nothing written).

use kentos_commands::{
    CommandResult, CreateTarget, ExportFormat, LayerDataOperation, LayerNodeType, LayersData, LayersExport,
    LayersMigrate, LayersMigrated,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The export that writes the layer's objects into the table.
fn export_of(input: &LayersMigrate) -> LayersExport {
    let gpkg = input.target == CreateTarget::Gpkg;
    LayersExport {
        layer: input.layer.trim().to_owned(),
        format: if gpkg { ExportFormat::Gpkg } else { ExportFormat::Postgis },
        path: gpkg.then(|| input.path.clone().unwrap_or_default()),
        address: (!gpkg).then(|| input.address.clone().unwrap_or_default()),
        schema: (!gpkg).then(|| input.schema.clone().unwrap_or_else(|| "public".to_owned())),
        name: input.table.as_deref().map(str::trim).filter(|t| !t.is_empty()).map(str::to_owned),
        ..LayersExport::default()
    }
}

/// The migration's own checks, in the contract's order, before the export's.
fn checked(doc: &Document, input: &LayersMigrate) -> Result<(), Stop> {
    let id = input.layer.trim();
    if id.is_empty() {
        return Err(fail(codes::NO_LAYER, "Taşınacak katman verilmedi (layer).".into(), "layer"));
    }
    match input.target {
        CreateTarget::Gpkg if input.path.as_deref().is_none_or(|p| p.trim().is_empty()) => {
            return Err(fail(codes::NO_TARGET, "Tablonun yapılacağı GeoPackage dosyası verilmedi (path).".into(), "path"));
        }
        CreateTarget::Postgis if input.address.as_deref().is_none_or(|a| a.trim().is_empty()) => {
            return Err(fail(codes::NO_TARGET, "Tablonun yapılacağı veritabanı verilmedi (address).".into(), "address"));
        }
        _ => {}
    }
    let node = doc
        .layers()
        .get(id)
        .ok_or_else(|| fail(codes::LAYER_NOT_FOUND, format!("“{id}” kimlikli katman çizimde yok."), "layer"))?;
    if node.kind == LayerNodeType::Group {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!("“{}” bir katman grubu; kaynağa taşınan bir katmandır. Grubun bir katmanını verin.", node.name),
            "layer",
        ));
    }
    if node.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!("“{}” bir harita servisinden çizilir; taşınacak nesnesi yoktur.", node.name),
            "layer",
        ));
    }
    if let Some(d) = &node.data {
        return Err(fail(
            codes::DATA_LAYER,
            format!("“{}” zaten “{}” tablosunu okuyor; nesneleri kaynağında.", node.name, d.table),
            "layer",
        ));
    }
    if doc.layers().is_locked(id) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!("“{}” kilitli; nesneleri taşınmaz. Kilidi açıp yeniden deneyin.", node.name),
            "layer",
        ));
    }
    Ok(())
}

/// The layer's objects counted by a dry export: written, and what a table cannot hold (refused then).
fn counted(doc: &Document, input: &LayersMigrate, export: &LayersExport) -> Result<crate::layer_export::Run, Stop> {
    checked(doc, input)?;
    let mut plan = crate::layer_export::planned(doc, export)?;
    plan.all_objects = true;
    // The table there already: said for the migration (the export's words name its `existing`, which this has not).
    let table = crate::layer_export_sinks::table_of(&plan).unwrap_or_default();
    if table_there(export, &table)? {
        return Err(fail(
            codes::TARGET_EXISTS,
            format!("“{table}” tablosu kaynakta var; başka bir tablo adı verin (table)."),
            "table",
        ));
    }
    let (run, _) = crate::layer_export::run(doc, &plan, false, &|| false, &|_| {})?;
    if run.skipped > 0 {
        let first = run.first_skip.clone().unwrap_or_default();
        return Err(fail(
            codes::UNWRITABLE_OBJECTS,
            format!(
                "Katmanın {} nesnesi bir tabloya yazılamaz ({first}); katman taşınırsa kaybolurdu, hiçbir şey yazılmadı. Onları başka bir katmana alıp yeniden deneyin.",
                run.skipped
            ),
            "layer",
        ));
    }
    Ok(run)
}

/// Whether the table is there: a GeoPackage file that is not there has none.
fn table_there(export: &LayersExport, table: &str) -> Result<bool, Stop> {
    let address = export.path.clone().or(export.address.clone()).unwrap_or_default();
    let address = address.trim();
    if export.format == ExportFormat::Gpkg && !std::path::Path::new(address).exists() {
        return Ok(false);
    }
    let unreachable = |e: kentos_data::DataError| {
        Stop::failed(error(codes::SOURCE_UNREACHABLE, format!("Okunamadı ya da yazılamadı: {e}"), None))
    };
    let source = kentos_data::registry().source(address).map_err(unreachable)?;
    Ok(source.tables().map_err(unreachable)?.iter().any(|t| t.name == table))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &LayersMigrate) -> CommandResult<()> {
    let export = export_of(input);
    match counted(cx.doc, input, &export) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would do: the rows it would write; nothing written.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersMigrate) -> CommandResult<LayersMigrated> {
    let export = export_of(input);
    let run = match counted(cx.doc, input, &export) {
        Ok(r) => r,
        Err(stop) => return stop.into(),
    };
    let plan = match crate::layer_export::planned(cx.doc, &export) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    CommandResult::Completed {
        output: LayersMigrated {
            layer: String::new(),
            table: crate::layer_export_sinks::table_of(&plan).unwrap_or_default(),
            address: export.path.clone().or(export.address.clone()).unwrap_or_default().trim().to_owned(),
            written: run.written,
            geometry: plan.schema.geometry.as_ref().map(|g| g.type_name.clone()),
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// Moves the layer into the source, the call's one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersMigrate) -> CommandResult<LayersMigrated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_MIGRATE,
        kentos_commands::CAD_LAYERS_MIGRATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: LayersMigrate) -> CommandResult<LayersMigrated> {
    let export = export_of(&input);
    if let Err(stop) = counted(cx.doc, &input, &export) {
        return stop.into();
    }
    let id = input.layer.trim().to_owned();
    // The table written (none on the protocol's reading copy: docs/adr/0532 §4) and the data layer it makes.
    let (written, table, geometry, data, fields) = {
        let mut plan = match crate::layer_export::planned(cx.doc, &export) {
            Ok(p) => p,
            Err(stop) => return stop.into(),
        };
        plan.all_objects = true;
        let (run, _) = match crate::layer_export::run(cx.doc, &plan, !cx.dry(), &|| false, &|_| {}) {
            Ok(r) => r,
            Err(stop) => return stop.into(),
        };
        let Some((data, fields, _)) = crate::layer_export_sinks::data_layer_of(&plan) else {
            return fail(codes::SOURCE_UNREACHABLE, "Yazılan tablonun yeri okunamadı.".into(), "target").into();
        };
        let table = crate::layer_export_sinks::table_of(&plan).unwrap_or_default();
        (run.written, table, plan.schema.geometry.as_ref().map(|g| g.type_name.clone()), data, fields)
    };
    let Some(old) = cx.doc.layers().get(&id).cloned() else {
        return fail(codes::LAYER_NOT_FOUND, format!("“{id}” kimlikli katman çizimde yok."), "layer").into();
    };
    let (parent, index) = cx.doc.layers().place_of(&id).unwrap_or((None, 0));
    let was_active = cx.doc.layers().active() == id;
    let address = data.address.clone();
    // The data layer in the layer's place, with its name.
    let added = crate::layers_data::write_activated(
        cx,
        LayersData {
            operation: LayerDataOperation::Add,
            layer: None,
            name: Some(old.name.clone()),
            parent,
            index: u32::try_from(index).ok(),
            data: Some(data),
            fields: Some(fields),
            expected_revision: None,
        },
        was_active,
    );
    let new_id = match added {
        CommandResult::Completed { output, .. } => output.layer,
        CommandResult::Failed { error } => return CommandResult::Failed { error },
        CommandResult::Conflict { error } => return CommandResult::Conflict { error },
        CommandResult::NeedsInput { error } => return CommandResult::NeedsInput { error },
        CommandResult::Queued { job_id } => return CommandResult::Queued { job_id },
        CommandResult::Cancelled => return CommandResult::Cancelled,
    };
    // Its look: the renderer, the labels, the colour.
    cx.doc.set_layer_style(&new_id, old.style.clone(), crate::layers_migrate::LABEL);
    // The drawing layer goes with its objects, in the same step.
    if let Err(refusal) = cx.doc.remove_layer(&id) {
        return fail(codes::LAYER_LOCKED, refusal.0, "layer").into();
    }
    CommandResult::Completed {
        output: LayersMigrated { layer: new_id, table, address, written, geometry, revision: cx.doc.revision().to_string() },
        warnings: Vec::new(),
    }
}

/// The step's name.
pub const LABEL: &str = "Katmanı kaynağa taşı";

//! `cad.layers.create` v1 (docs/adr/9107 §7): a new layer of a GIS project made as a new table of a data source and
//! added as a data layer. The table goes through its driver (`Transaction::create_table`) in a transaction of its own,
//! with its system's name and WKT where the source keeps them (a GeoPackage, `NewTable.system`); the layer goes in as
//! `cad.layers.data` adds one, the call's one undo step. A GeoPackage file that is not there is made.

use std::path::Path;

use kentos_commands::{
    CommandResult, CreateGeometry, CreateTarget, DataGeometry, DataLayer, LayerDataOperation, LayerField, LayersCreate,
    LayersCreated, LayersData,
};
use kentos_data::{Field, GeometryField, NewTable};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::sources::TableSystem;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn unreachable(e: impl std::fmt::Display) -> Stop {
    Stop::failed(error(codes::SOURCE_UNREACHABLE, format!("Okunamadı ya da yazılamadı: {e}"), None))
}

/// The longest name a source takes, in bytes (PostgreSQL's identifiers).
const NAME_MOST: usize = 63;

/// What the input makes: where, the table as its driver names it, its columns, its geometry, the layer's fields.
struct Made {
    driver: &'static str,
    address: String,
    /// A GeoPackage's table name, PostGIS's `schema.table`.
    table: String,
    key: String,
    fields: Vec<Field>,
    geometry: Option<GeometryField>,
    layer_fields: Vec<LayerField>,
}

/// Why a table or column name cannot be taken: empty, longer than a source takes, a control character, a name a
/// GeoPackage keeps for itself.
fn name_problem(name: &str) -> Option<String> {
    if name.trim().is_empty() {
        return Some("ad boş olamaz".into());
    }
    if name.len() > NAME_MOST {
        return Some(format!("“{name}” en çok {NAME_MOST} bayt olabilir"));
    }
    if name.chars().any(char::is_control) {
        return Some(format!("“{name}” denetim karakteri taşıyor"));
    }
    let lower = name.to_ascii_lowercase();
    if ["gpkg_", "rtree_", "sqlite_"].iter().any(|p| lower.starts_with(p)) || lower == "kentos_projects" {
        return Some(format!("“{name}” kaynağın kendi tablolarına ayrılmış bir ad"));
    }
    None
}

/// OGC's name of the geometry type asked.
fn type_name(geometry: CreateGeometry, multi: bool) -> Option<&'static str> {
    Some(match (geometry, multi) {
        (CreateGeometry::Point, false) => "POINT",
        (CreateGeometry::Point, true) => "MULTIPOINT",
        (CreateGeometry::Line, false) => "LINESTRING",
        (CreateGeometry::Line, true) => "MULTILINESTRING",
        (CreateGeometry::Area, false) => "POLYGON",
        (CreateGeometry::Area, true) => "MULTIPOLYGON",
        (CreateGeometry::None, _) => return None,
    })
}

fn checked(doc: &Document, input: &LayersCreate) -> Result<Made, Stop> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(fail(codes::EMPTY_NAME, "Katmanın adı boş olamaz.".into(), "name"));
    }
    let (driver, address) = match input.target {
        CreateTarget::Gpkg => {
            let path = input.path.as_deref().map(str::trim).filter(|p| !p.is_empty());
            let Some(path) = path else {
                return Err(fail(codes::NO_TARGET, "Tablonun yapılacağı GeoPackage dosyası verilmedi (path).".into(), "path"));
            };
            ("gpkg", path.to_owned())
        }
        CreateTarget::Postgis => {
            let address = input.address.as_deref().map(str::trim).filter(|a| !a.is_empty());
            let Some(address) = address else {
                return Err(fail(codes::NO_TARGET, "Tablonun yapılacağı veritabanı verilmedi (address).".into(), "address"));
            };
            ("postgis", address.to_owned())
        }
    };
    let bare = input
        .table
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| crate::layer_export::table_name_of(name));
    if let Some(why) = name_problem(&bare) {
        return Err(fail(codes::INVALID_NAME, format!("Tablonun adı alınamaz: {why}."), "table"));
    }
    let key = input.key.as_deref().map(str::trim).filter(|k| !k.is_empty()).unwrap_or("fid").to_owned();
    let column = input.geometry_column.as_deref().map(str::trim).filter(|c| !c.is_empty()).unwrap_or("geom").to_owned();
    for (what, n) in [("key", &key), ("geometryColumn", &column)] {
        if let Some(why) = name_problem(n) {
            return Err(fail(codes::INVALID_NAME, format!("Sütunun adı alınamaz: {why}."), what));
        }
    }
    let layer_fields = input.fields.clone().unwrap_or_default();
    for (i, f) in layer_fields.iter().enumerate() {
        if let Some(why) = name_problem(&f.name) {
            return Err(fail(codes::INVALID_NAME, format!("Alanın adı alınamaz: {why}."), &format!("fields.{i}.name")));
        }
        if f.name.eq_ignore_ascii_case(&key) || (input.geometry != CreateGeometry::None && f.name.eq_ignore_ascii_case(&column)) {
            return Err(fail(
                codes::INVALID_NAME,
                format!("“{}” anahtar ya da geometri sütununun adı; alana başka bir ad verin.", f.name),
                &format!("fields.{i}.name"),
            ));
        }
    }
    if let Some(why) = kentos_commands::fields_problem(&layer_fields) {
        return Err(fail(codes::INVALID_FIELDS, why, "fields"));
    }
    let settings = doc.settings();
    let project = (settings.srid > 0).then_some(settings.srid);
    let srid = input.srid.or(project);
    let geometry = match type_name(input.geometry, input.multi == Some(true)) {
        Some(t) => {
            if let Some(s) = srid
                && let Err(why) = TableSystem::of(settings, Some(s))
            {
                return Err(fail(codes::UNKNOWN_SRID, format!("EPSG:{s} bu projede yazılamaz: {why}"), "srid"));
            }
            Some(GeometryField {
                name: column,
                type_name: t.to_owned(),
                z: input.z == Some(true),
                m: false,
                srid: srid.and_then(|s| i32::try_from(s).ok()),
            })
        }
        None => None,
    };
    let table = match input.target {
        CreateTarget::Gpkg => bare,
        CreateTarget::Postgis => {
            let schema = input.schema.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or("public");
            if let Some(why) = name_problem(schema) {
                return Err(fail(codes::INVALID_NAME, format!("Şemanın adı alınamaz: {why}."), "schema"));
            }
            format!("{schema}.{bare}")
        }
    };
    let fields = layer_fields
        .iter()
        .map(|f| Field::new(&f.name, crate::layer_export::field_kind(f.kind, f.length)))
        .collect();
    Ok(Made { driver, address, table, key, fields, geometry, layer_fields })
}

/// Whether the table is there: a GeoPackage file that is not there has none.
fn there(made: &Made) -> Result<bool, Stop> {
    if made.driver == "gpkg" && !Path::new(&made.address).exists() {
        return Ok(false);
    }
    let source = kentos_data::registry().source(&made.address).map_err(unreachable)?;
    let tables = source.tables().map_err(unreachable)?;
    Ok(tables.iter().any(|t| t.name == made.table))
}

fn checked_free(doc: &Document, input: &LayersCreate) -> Result<Made, Stop> {
    let made = checked(doc, input)?;
    if there(&made)? {
        return Err(fail(
            codes::TARGET_EXISTS,
            format!("“{}” tablosu kaynakta var; başka bir ad verin ya da onu Kaynaklar'dan veri katmanı olarak ekleyin.", made.table),
            "table",
        ));
    }
    Ok(made)
}

/// The output for what is (or would be) made.
fn created(made: &Made, layer: String, revision: String) -> LayersCreated {
    LayersCreated {
        layer,
        table: made.table.clone(),
        address: made.address.clone(),
        geometry: made.geometry.as_ref().map(|g| if g.z { format!("{} Z", g.type_name) } else { g.type_name.clone() }),
        srid: made.geometry.as_ref().and_then(|g| g.srid).and_then(|s| u32::try_from(s).ok()),
        revision,
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &LayersCreate) -> CommandResult<()> {
    match checked_free(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would make; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersCreate) -> CommandResult<LayersCreated> {
    match checked_free(cx.doc, input) {
        Ok(made) => CommandResult::Completed {
            output: created(&made, String::new(), cx.doc.revision().to_string()),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Makes the table and adds it as a data layer, the call's one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersCreate) -> CommandResult<LayersCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LAYERS_CREATE,
        kentos_commands::CAD_LAYERS_CREATE_VERSION,
        input,
        write_execute,
    )
}

/// The table made in its own transaction (a GeoPackage file made first when it is not there).
fn make_table(made: &Made) -> Result<(), Stop> {
    if made.driver == "gpkg" {
        let path = Path::new(&made.address);
        if !path.exists() {
            if let Some(folder) = path.parent().filter(|f| !f.as_os_str().is_empty()) {
                std::fs::create_dir_all(folder).map_err(unreachable)?;
            }
            kentos_geopackage::GeoPackage::create(path).map_err(|e| unreachable(e.0))?;
            kentos_data::registry().forget(&made.address);
        }
    }
    let source = kentos_data::registry().source(&made.address).map_err(unreachable)?;
    let mut tx = source.begin().map_err(unreachable)?;
    let table = NewTable {
        name: made.table.clone(),
        title: Some(made.table.rsplit('.').next().unwrap_or(&made.table).to_owned()),
        description: None,
        key: made.key.clone(),
        fields: made.fields.clone(),
        geometry: made.geometry.clone(),
        spatial_index: made.geometry.is_some(),
        system: crate::layer_export_sinks::system_text(made.geometry.as_ref().and_then(|g| g.srid)),
    };
    if let Err(e) = tx.create_table(&table, false) {
        let _ = tx.rollback();
        return Err(unreachable(e));
    }
    tx.commit().map_err(unreachable)
}

/// The data layer that reads a table made.
fn data_of(made: &Made) -> DataLayer {
    DataLayer {
        driver: made.driver.to_owned(),
        address: made.address.clone(),
        relative: None,
        table: made.table.clone(),
        key: made.key.clone(),
        geometry: made.geometry.as_ref().map(|g| DataGeometry {
            column: g.name.clone(),
            type_name: g.type_name.clone(),
            srid: g.srid.and_then(|s| u32::try_from(s).ok()),
            z: g.z,
            m: g.m,
        }),
        extent: None,
        read_only: false,
        text: None,
    }
}

/// A new CBS project's table (docs/adr/9196 §2, plan/ayrim.md AY-05): `input`'s table made in its source, the way
/// `cad.layers.create` makes one, and the data layer that reads it with its columns as the layer's fields, for the
/// wizard to put on the layer's node before the project opens. Nothing is added to `doc`, which gives the system and
/// the names taken; no undo step, since the project starts with its tables. Why not, in the command's words.
pub fn new_project_table(doc: &Document, input: &LayersCreate) -> Result<(DataLayer, Vec<LayerField>), String> {
    let made = checked_free(doc, input).map_err(crate::checks::words)?;
    make_table(&made).map_err(crate::checks::words)?;
    Ok((data_of(&made), made.layer_fields))
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: LayersCreate) -> CommandResult<LayersCreated> {
    let made = match checked_free(cx.doc, &input) {
        Ok(m) => m,
        Err(stop) => return stop.into(),
    };
    // The protocol's plan runs this on a reading copy: no table made then (docs/adr/0532 §4).
    if !cx.dry()
        && let Err(stop) = make_table(&made)
    {
        return stop.into();
    }
    let data = data_of(&made);
    let added = crate::layers_data::write_execute(
        cx,
        LayersData {
            operation: LayerDataOperation::Add,
            layer: None,
            name: Some(input.name.trim().to_owned()),
            parent: input.parent.clone(),
            index: input.index,
            data: Some(data),
            fields: Some(made.layer_fields.clone()),
            expected_revision: None,
        },
    );
    match added {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: created(&made, output.layer, cx.doc.revision().to_string()),
            warnings,
        },
        CommandResult::Failed { error } => CommandResult::Failed { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::Cancelled => CommandResult::Cancelled,
    }
}

//! Paketle's data layers (docs/adr/9338). A data layer's rows are its source's, never the drawing's objects
//! (docs/adr/9101, 9196), so the package takes the source with it, or names it, and its open files take the rows from
//! it.
//!
//! - **GeoPackage:** its data layers follow `geopackages`. Copied (and with `embed` too: a data layer is never written
//!   into the project file), the file goes byte for byte under `veri/`, once however many layers read it; each layer
//!   names the copy (its address the copy in the package's folder, its path from the project `veri/<name>`, which the
//!   project follows when the folder moves). Kept, the layer names the file where it is.
//! - **Database:** its data layers follow `databases`. Copied, the layer's rows are written into a GeoPackage under
//!   `veri/` by `cad.layers.export`, and the layer reads that table. Kept (and with `embed`), the layer names the
//!   database without its password.
//! - **The open files:** every data layer's rows read from its source go into them; in their copy of the project the
//!   layers are the copy's own, as an open file holds no link. A source that cannot be read refuses the package with
//!   its layer's name: no file is written.

use std::path::{Path, PathBuf};

use kentos_commands::{CallOp, DataLayer, LeftWhy, PackageLeft, PackageWay, PackagedKind, codes};
use kentos_domain::Document;
use kentos_native_application::links::find_in;
use serde_json::{Value, json};

use crate::file_commands::Refused;
use crate::package::Copies;
use crate::package_sources::Scratch;

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Refused {
    Box::new(kentos_commands::CommandError {
        code: code.to_owned(),
        message: message.into(),
        path: Some(path.to_owned()),
        ..Default::default()
    })
}

/// How the package takes the data layers' sources.
pub(crate) struct Ways<'a> {
    pub geopackages: PackageWay,
    pub databases: PackageWay,
    /// The project file's folder, where a source's relative path starts.
    pub from: Option<&'a Path>,
    /// The package's folder.
    pub target: &'a Path,
    /// Whether databases are read for their copies (a validate does not).
    pub read: bool,
}

/// What [`plan_layers`] leaves for the plan.
pub(crate) struct Planned<'a> {
    pub copies: &'a mut Copies,
    pub left: &'a mut Vec<PackageLeft>,
    pub scratch: &'a mut Option<Scratch>,
}

/// A GeoPackage address's file, the driver's prefix aside.
fn file_of(address: &str) -> &str {
    address.strip_prefix("gpkg:").unwrap_or(address)
}

/// Each data layer of the snapshot's tree (`layers`, groups' `children` inside) planned: its source copied and named in
/// the package, or kept and said.
pub(crate) fn plan_layers(
    doc: &Document,
    layers: &mut [Value],
    ways: &Ways<'_>,
    out: &mut Planned<'_>,
) -> Result<(), Refused> {
    for node in layers.iter_mut() {
        if let Some(Value::Array(children)) = node.get_mut("children") {
            plan_layers(doc, children, ways, out)?;
        }
        let Some(data) = node.get("data").filter(|d| d.is_object()).cloned() else {
            continue;
        };
        let Ok(mut data) = serde_json::from_value::<DataLayer>(data) else {
            continue;
        };
        let id = node.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
        let name = node.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
        let database = data.driver != "gpkg";
        let way = if database { ways.databases } else { ways.geopackages };
        let kept = |why: LeftWhy| PackageLeft {
            kind: PackagedKind::Source,
            path: data.address.clone(),
            why,
            at: format!("veri:{id}"),
            objects: None,
        };
        match (database, way) {
            (false, PackageWay::Keep) => out.left.push(kept(LeftWhy::Kept)),
            (false, _) => {
                let found = find_in(file_of(&data.address), data.relative.as_deref(), ways.from, &[]).at;
                let Some(at) = found else {
                    out.left.push(kept(LeftWhy::Missing));
                    continue;
                };
                let rel = out.copies.take(&at, PackagedKind::Source);
                let abs = rel.split('/').fold(ways.target.to_path_buf(), |p, part| p.join(part));
                let prefix = if data.address.starts_with("gpkg:") { "gpkg:" } else { "" };
                data.address = format!("{prefix}{}", abs.display());
                data.relative = Some(rel);
                node["data"] = serde_json::to_value(&data).unwrap_or(Value::Null);
            }
            (true, PackageWay::Copy) => {
                if !ways.read {
                    continue;
                }
                if out.scratch.is_none() {
                    *out.scratch = Some(Scratch::new()?);
                }
                let dir = out.scratch.as_ref().map(|d| d.0.clone()).unwrap_or_default();
                let (file, copied, rows) = copy_table(doc, &id, &name, &data, &dir)?;
                let rel = out.copies.take(&file, PackagedKind::Source);
                out.copies.databases.insert(rel.clone(), (data.address.clone(), data.table.clone(), rows));
                let abs = rel.split('/').fold(ways.target.to_path_buf(), |p, part| p.join(part));
                let mut copied = copied;
                copied.address = abs.display().to_string();
                copied.relative = Some(rel);
                copied.read_only = data.read_only;
                copied.text = data.text.clone();
                node["data"] = serde_json::to_value(&copied).unwrap_or(Value::Null);
            }
            (true, _) => out.left.push(kept(LeftWhy::Database)),
        }
    }
    Ok(())
}

/// A database data layer's rows written into a GeoPackage in `dir` by `cad.layers.export` (the table added to a copy of
/// the drawing as a data layer, so its catalog is the export's): the file, the layer's data reading it, the rows.
fn copy_table(
    doc: &Document,
    id: &str,
    name: &str,
    data: &DataLayer,
    dir: &Path,
) -> Result<(PathBuf, DataLayer, u64), Refused> {
    let short = data.table.rsplit('.').next().unwrap_or(&data.table).to_owned();
    let stem: String = short
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '\0') || c.is_control() { '-' } else { c })
        .collect();
    let file = dir.join(format!("{stem}.gpkg"));
    let mut copy = doc.clone();
    let answer = {
        let mut cx = kentos_native_application::ExecutionContext::new(&mut copy);
        kentos_native_application::registry::run(
            &mut cx,
            "cad.layers.export",
            CallOp::Execute,
            json!({ "layer": id, "format": "gpkg", "path": file.display().to_string(), "name": short, "addToMap": true }),
        )
    };
    let fail = |why: String| {
        refuse(
            codes::SOURCE_UNREACHABLE,
            format!("“{name}” veri katmanının tablosu ({}) GeoPackage'a kopyalanamadı: {why} Paket yapılmadı, hiçbir dosya yazılmadı; veritabanına ulaşın ya da Veritabanı kaynakları'nı Bağlı bırak yapın.", data.table),
            "databases",
        )
    };
    let answer = answer.map_err(|e| fail(format!("{e:?}")))?;
    if answer["status"] != "completed" {
        return Err(fail(answer["error"]["message"].as_str().unwrap_or("").to_owned()));
    }
    let added = answer["output"]["layer"].as_str().unwrap_or_default();
    let read = copy
        .layers()
        .get(added)
        .and_then(|l| l.data.clone())
        .ok_or_else(|| fail("kopyanın katmanı kurulamadı.".into()))?;
    let rows = answer["output"]["written"].as_u64().unwrap_or(0);
    Ok((file, read, rows))
}

/// The file exports that write the project's layers (docs/adr/9344): a data layer's rows go too, read from its source,
/// for every channel alike (the window, Python, MCP, an agent).
const WRITES_LAYERS: [&str; 7] = [
    kentos_commands::FILE_EXPORT_GPKG,
    kentos_commands::FILE_EXPORT_SHP,
    kentos_commands::FILE_EXPORT_CSV,
    kentos_commands::FILE_EXPORT_FGB,
    kentos_commands::FILE_EXPORT_PARQUET,
    kentos_commands::FILE_EXPORT_GML,
    kentos_commands::FILE_EXPORT_XYZ,
];

/// A file command run on the drawing with its data layers' rows when it writes the project's layers; as it is
/// otherwise. A source that cannot be read refuses the export with its layer's name: nothing is written.
pub(crate) fn typed_with_rows(
    doc: &Document,
    command: &str,
    op: CallOp,
    input: Value,
    progress: Option<&kentos_native_application::jobs::Progress>,
) -> Result<(kentos_commands::CommandResult<Value>, Option<kentos_commands::FileEffect>), String> {
    if WRITES_LAYERS.contains(&command) && doc.layers().leaves().iter().any(|l| l.data.is_some()) {
        match rows_said(doc, "layers", "Dışa aktarma yapılmadı, hiçbir dosya yazılmadı. Kaynağa ulaşın ya da katmanı dışarıda bırakın.") {
            Ok(Some(open)) => return crate::file_commands::run_typed_with_progress(&open, command, op, input, progress),
            Ok(None) => {}
            Err(refused) => return Ok((kentos_commands::CommandResult::Failed { error: *refused }, None)),
        }
    }
    crate::file_commands::run_typed_with_progress(doc, command, op, input, progress)
}

/// The drawing as its open files hold it: every data layer's rows read from its source, the layers the copy's own
/// (their links gone); none without a data layer. A source that cannot be read refuses, its layer named.
pub(crate) fn with_rows(doc: &Document) -> Result<Option<Document>, Refused> {
    rows_said(
        doc,
        "exchange",
        "Devir dosyaları bu katmanın satırlarını taşıyamaz; paket yapılmadı, hiçbir dosya yazılmadı. Kaynağa ulaşın ya da devir dosyalarını kaldırın.",
    )
}

/// [`with_rows`], a refusal at `path` saying `then` after why the source could not be read.
fn rows_said(doc: &Document, path: &str, then: &str) -> Result<Option<Document>, Refused> {
    let nodes: Vec<kentos_commands::LayerNode> =
        doc.layers().leaves().into_iter().filter(|l| l.data.is_some()).cloned().collect();
    if nodes.is_empty() {
        return Ok(None);
    }
    let mut rows = Vec::new();
    for node in &nodes {
        let held = doc.data_hidden(&node.id);
        let read = kentos_native_application::data_write::all_rows(node, doc.settings(), &held, &|| false).map_err(|why| {
            refuse(codes::SOURCE_UNREACHABLE, format!("“{}” veri katmanının kaynağı okunamadı: {why}. {then}", node.name), path)
        })?;
        rows.extend(read.rows.into_iter().map(|(_, e)| e));
    }
    let mut snapshot = doc.to_snapshot_v2();
    unlink(&mut snapshot.layers);
    let fail = |why: String| refuse(codes::FILE_UNWRITABLE, format!("satırlarıyla çizim kurulamadı: {why}"), path);
    let mut open = Document::from_snapshot_v2(snapshot).map_err(fail)?;
    open.add_many(rows, "Paketle").map_err(|_| fail("çizimde yer kalmadı".into()))?;
    Ok(Some(open))
}

/// The layers' links to their sources taken off, groups' children too.
fn unlink(layers: &mut [kentos_commands::LayerNode]) {
    for l in layers {
        l.data = None;
        unlink(&mut l.children);
    }
}

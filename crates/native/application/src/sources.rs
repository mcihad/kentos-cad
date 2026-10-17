//! Bound layers (docs/adr/0503, plan/gis.md CBS-01, CBS-02, CBS-11, CBS-36): a layer of the drawing kept as the working
//! copy of a table of a GeoPackage. The binding (`SourceLink`, the project's `sources`) keeps the file's SHA-256 and
//! the layer's digest from when the two last matched, so each side's changes are told apart without keeping a copy:
//! a write is refused when the file changed since (`source_changed`), a reload when the layer did (`pending_changes`,
//! unless `discard`). The two are compared object by object by lasting id, as rows of the table: an object as the row
//! it would be written as (`geopackage::row_for`) against the row the file holds, coordinates within the kernel's
//! tolerance. A write edits the table in place in one transaction (columns the layer's fields add, rows updated,
//! inserted with their ids, deleted; `GeoPackage::edit`); a reload is one undo step, keeping each object's look. The
//! shared cases are `fixtures/commands/v1/cad.sources.*.json` (`scripts/fixtures/sources_cases.py`).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use kentos_contracts::canonical::digest_of;
use kentos_contracts::encoding::sha256_hex;
use kentos_contracts::{
    CommandResult, CommandWarning, ImportGpkg, ImportedLayer, LayerNodeType, MAX_SOURCES,
    SourceCapabilities, SourceColumn, SourceKind, SourceLink, SourceState, SourceTable,
    SourcesDescribe, SourcesDescribed, SourcesEdit, SourcesEditOp, SourcesEdited, SourcesSync,
    SourcesSynced, SyncDirection,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geopackage::{self as gpkg, DataType, Edits, GeoPackage, Geometry, Query, Value};
use serde_json::{Value as J, json};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geopackage as gp;

/// Coordinates of an object and its row agree within this (m): the geometry kernel's tolerance.
const SAME: f64 = 1e-6;

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn conflict(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::conflict(error(code, message.into(), Some(path.into())))
}

fn completed<T>(r: Result<(T, Vec<CommandWarning>), Stop>) -> CommandResult<T> {
    match r {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The layer's digest: the SHA-256 of its objects' kinds, geometries and attributes by lasting id (not their look).
pub fn layer_digest(doc: &Document, layer: &str) -> String {
    let mut items: Vec<(Uuid, J)> = doc
        .by_layer_with_uids(layer)
        .map(|(uid, e)| (uid, gp::essence(e)))
        .collect();
    items.sort_by_key(|(u, _)| *u);
    let list: Vec<J> = items
        .into_iter()
        .map(|(u, e)| json!([u.to_string(), e]))
        .collect();
    digest_of(&J::Array(list), 64)
}

fn file_sha(path: &Path) -> Result<String, String> {
    std::fs::read(path)
        .map(|b| sha256_hex(&b))
        .map_err(|e| format!("{} okunamadı: {e}", path.display()))
}

fn checked<'a>(path: &'a str, at: &str) -> Result<&'a Path, Stop> {
    let p = Path::new(path);
    if path.trim().is_empty() || !p.is_absolute() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{path}” mutlak bir dosya yolu değil; GeoPackage dosyasının tam yolunu verin (/klasör/ad.gpkg)."
            ),
            at,
        ));
    }
    if p.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("{path} bir klasör; GeoPackage dosyasının adını da verin."),
            at,
        ));
    }
    if !p.exists() {
        return Err(refuse(
            codes::FILE_NOT_FOUND,
            format!("{path} yok; dosyanın yolunu denetleyin."),
            at,
        ));
    }
    Ok(p)
}

fn readonly(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.permissions().readonly())
}

/// A table as the file holds it: its definition, its system, and the rows the import reads, by lasting id.
struct Stored {
    layer: gpkg::Layer,
    srid: Option<u32>,
    rows: HashMap<Uuid, (i64, Option<Geometry>, Vec<Value>)>,
    writable: bool,
    sha256: String,
}

fn load(path: &Path, table: &str) -> Result<Stored, String> {
    let read = gp::read(path, Some(&[table.to_owned()]))?;
    let Some(t) = read.tables.into_iter().next() else {
        return Err(format!("“{table}” tablosu dosyada yok"));
    };
    let file = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let Some(layer) = file
        .layers()
        .map_err(|e| e.0)?
        .into_iter()
        .find(|l| l.name == table)
    else {
        return Err(format!("“{table}” tablosu dosyada yok"));
    };
    let mut raw: HashMap<i64, gpkg::Row> = file
        .rows(&layer, &Query::default())
        .map_err(|e| e.0)?
        .into_iter()
        .map(|r| (r.fid, r))
        .collect();
    let mut rows = HashMap::with_capacity(t.fids.len());
    for ((uid, _), fid) in t.objects.iter().zip(&t.fids) {
        let Some(r) = raw.remove(fid) else { continue };
        let g = match r.geometry {
            // An empty geometry is no geometry: the import makes a record of it (docs/adr/0502).
            Some(Ok(g)) if g.is_empty() => None,
            Some(Ok(g)) => Some(g),
            Some(Err(_)) => continue,
            None => None,
        };
        rows.insert(*uid, (*fid, g, r.values));
    }
    let srid = layer
        .srs_id
        .and_then(|id| read.systems.get(&id).cloned().flatten())
        .filter(|(org, _)| org == "EPSG")
        .and_then(|(_, id)| u32::try_from(id).ok());
    let writable =
        matches!(layer.data_type, DataType::Features | DataType::Attributes) && !readonly(path);
    Ok(Stored {
        layer,
        srid,
        rows,
        writable,
        sha256: read.sha256,
    })
}

fn load_or_refuse(path: &Path, table: &str, at: &str) -> Result<Stored, Stop> {
    load(path, table).map_err(|why| {
        if why == format!("“{table}” tablosu dosyada yok") {
            refuse(
                codes::TABLE_NOT_FOUND,
                format!("{why}; dosyanın tablolarından birinin adını verin."),
                at,
            )
        } else {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!("{} GeoPackage olarak okunamadı: {why}", path.display()),
                at,
            )
        }
    })
}

/// How a layer differs from its table, as the edits that would make the table the layer.
#[derive(Default)]
struct Diff {
    added_columns: Vec<gpkg::Column>,
    insert: Vec<(String, gpkg::NewRow)>,
    update: Vec<(i64, gpkg::NewRow)>,
    delete: Vec<i64>,
    /// Objects a table has no geometry for (texts, dimensions …): they stay in the drawing.
    left: u32,
    mismatch: Option<(Uuid, String)>,
    /// Attributes neither a column nor a field of the layer.
    lost: BTreeSet<String>,
}

impl Diff {
    fn is_empty(&self) -> bool {
        self.insert.is_empty() && self.update.is_empty() && self.delete.is_empty()
    }
}

fn walk(g: &Geometry, sig: &mut Vec<u8>, xs: &mut Vec<f64>) {
    fn coord(c: &gpkg::Coord, sig: &mut Vec<u8>, xs: &mut Vec<f64>) {
        sig.push(10 + u8::from(c.z.is_some()) + 2 * u8::from(c.m.is_some()));
        xs.extend([c.x, c.y]);
        xs.extend(c.z);
        xs.extend(c.m);
    }
    fn curve(c: &gpkg::Curve, sig: &mut Vec<u8>, xs: &mut Vec<f64>) {
        match c {
            gpkg::Curve::Line(p) | gpkg::Curve::Arc(p) => {
                sig.push(if matches!(c, gpkg::Curve::Line(_)) {
                    1
                } else {
                    2
                });
                for c in p {
                    coord(c, sig, xs);
                }
            }
            gpkg::Curve::Compound(parts) => {
                sig.push(3);
                for p in parts {
                    curve(p, sig, xs);
                }
            }
        }
        sig.push(0);
    }
    match g {
        Geometry::Point(p) => {
            sig.push(4);
            if let Some(c) = p {
                coord(c, sig, xs);
            }
        }
        Geometry::Curve(c) => curve(c, sig, xs),
        Geometry::Polygon(r) => {
            sig.push(5);
            r.iter().for_each(|c| curve(c, sig, xs));
        }
        Geometry::MultiPoint(p) => {
            sig.push(6);
            p.iter().for_each(|c| coord(c, sig, xs));
        }
        Geometry::MultiCurve(c) => {
            sig.push(7);
            c.iter().for_each(|c| curve(c, sig, xs));
        }
        Geometry::MultiPolygon(p) => {
            sig.push(8);
            for r in p {
                sig.push(5);
                r.iter().for_each(|c| curve(c, sig, xs));
                sig.push(0);
            }
        }
        Geometry::Collection(g) => {
            sig.push(9);
            g.iter().for_each(|g| walk(g, sig, xs));
        }
    }
    sig.push(0);
}

fn same_geometry(a: Option<&Geometry>, b: Option<&Geometry>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            let (mut sa, mut xa, mut sb, mut xb) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            walk(a, &mut sa, &mut xa);
            walk(b, &mut sb, &mut xb);
            sa == sb && xa.iter().zip(&xb).all(|(p, q)| (p - q).abs() <= SAME)
        }
        _ => false,
    }
}

fn same_value(a: &Value, b: &Value) -> bool {
    let num = |v: &Value| match v {
        Value::Integer(i) => Some(*i as f64),
        Value::Real(r) => Some(*r),
        _ => None,
    };
    match (num(a), num(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

fn diff(doc: &Document, layer: &str, s: &Stored) -> Diff {
    let mut d = Diff::default();
    let fields = doc
        .layers()
        .get(layer)
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    let taken = |name: &str, cols: &[gpkg::Column]| {
        name.eq_ignore_ascii_case(&s.layer.fid)
            || s.layer
                .geometry
                .as_ref()
                .is_some_and(|g| g.name.eq_ignore_ascii_case(name))
            || cols.iter().any(|c| c.name.eq_ignore_ascii_case(name))
    };
    let mut columns = s.layer.columns.clone();
    for f in &fields {
        if !taken(&f.name, &columns) {
            let c = gp::column_of(f);
            columns.push(c.clone());
            d.added_columns.push(c);
        }
    }
    let width = s.layer.columns.len();
    let geometry = s.layer.geometry.as_ref().map(|g| g.type_name.as_str());
    let mut seen = HashSet::new();
    for (uid, e) in doc.by_layer_with_uids(layer) {
        for k in e.base().attrs.keys() {
            if !columns.iter().any(|c| c.name.eq_ignore_ascii_case(k)) {
                d.lost.insert(k.clone());
            }
        }
        let row = match gp::row_for(e, &columns, geometry) {
            Ok(Some(r)) => r,
            Ok(None) => {
                d.left += 1;
                continue;
            }
            Err(why) => {
                if d.mismatch.is_none() {
                    d.mismatch = Some((uid, why));
                }
                continue;
            }
        };
        match s.rows.get(&uid) {
            None => d.insert.push((uid.to_string(), row)),
            Some((fid, g, values)) => {
                seen.insert(uid);
                let same = same_geometry(row.geometry.as_ref(), g.as_ref())
                    && row.values.iter().enumerate().all(|(i, v)| {
                        same_value(v, if i < width { &values[i] } else { &Value::Null })
                    });
                if !same {
                    d.update.push((*fid, row));
                }
            }
        }
    }
    d.delete = s
        .rows
        .iter()
        .filter(|(u, _)| !seen.contains(u))
        .map(|(_, (fid, ..))| *fid)
        .collect();
    d.delete.sort_unstable();
    d.update.sort_by_key(|(fid, _)| *fid);
    d
}

fn link_of<'a>(doc: &'a Document, id: &str) -> Option<&'a SourceLink> {
    doc.settings().sources.iter().find(|s| s.id == id)
}

fn set_link(doc: &mut Document, link: Option<SourceLink>, remove: Option<&str>) {
    let mut settings = doc.settings().clone();
    if let Some(id) = remove {
        settings.sources.retain(|s| s.id != id);
    }
    if let Some(link) = link {
        match settings.sources.iter_mut().find(|s| s.id == link.id) {
            Some(slot) => *slot = link,
            None => settings.sources.push(link),
        }
    }
    doc.set_settings(settings);
}

// ── cad.sources.describe ───────────────────────────────────────────────────

fn table_of(
    file: &GeoPackage,
    l: gpkg::Layer,
    srid: Option<u32>,
    writable: bool,
) -> Result<SourceTable, String> {
    let rows = file.count(&l).map_err(|e| e.0)?;
    let g = l.geometry.as_ref();
    let can = writable && matches!(l.data_type, DataType::Features | DataType::Attributes);
    Ok(SourceTable {
        data: match l.data_type {
            DataType::Features => "features",
            DataType::Attributes => "attributes",
            _ => "other",
        }
        .into(),
        rows,
        geometry: g.map(|g| g.type_name.clone()),
        srid,
        columns: l
            .columns
            .iter()
            .map(|c| SourceColumn {
                name: c.name.clone(),
                declared: c.decl.clone(),
                kind: gp::column_kind(c),
            })
            .collect(),
        capabilities: SourceCapabilities {
            read: true,
            insert: can,
            update: can,
            delete: can,
            transaction: true,
            z: g.is_some_and(|g| g.z != 0),
            m: g.is_some_and(|g| g.m != 0),
            curves: true,
        },
        name: l.name,
    })
}

fn describe_file(path: &Path) -> Result<Vec<SourceTable>, String> {
    let file = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let writable = !readonly(path);
    let mut out = Vec::new();
    for l in file.layers().map_err(|e| e.0)? {
        if !matches!(l.data_type, DataType::Features | DataType::Attributes) {
            continue;
        }
        let srid = match l.srs_id.filter(|&id| id > 0) {
            Some(id) => file
                .srs(id)
                .map_err(|e| e.0)?
                .filter(|s| {
                    s.organization.eq_ignore_ascii_case("EPSG") && s.definition != "undefined"
                })
                .and_then(|s| u32::try_from(s.organization_id).ok()),
            None => None,
        };
        out.push(table_of(&file, l, srid, writable)?);
    }
    Ok(out)
}

fn state_of(doc: &Document, link: &SourceLink) -> SourceState {
    let path = Path::new(&link.path);
    let edited = layer_digest(doc, &link.layer) != link.digest;
    let mut state = SourceState {
        link: link.clone(),
        found: path.is_file(),
        changed: false,
        edited,
        added: 0,
        modified: 0,
        removed: 0,
    };
    if let Ok(s) = load(path, &link.table) {
        state.changed = s.sha256 != link.revision;
        let d = diff(doc, &link.layer, &s);
        state.added = d.insert.len() as u32;
        state.modified = d.update.len() as u32;
        state.removed = d.delete.len() as u32;
    }
    state
}

fn describe_run(
    doc: &Document,
    input: &SourcesDescribe,
) -> Result<(SourcesDescribed, Vec<CommandWarning>), Stop> {
    let revision = doc.revision().to_string();
    match &input.path {
        Some(path) => {
            let p = checked(path, "path")?;
            let tables = describe_file(p).map_err(|why| {
                refuse(
                    codes::IMPORT_UNREADABLE,
                    format!("{path} GeoPackage olarak okunamadı: {why}"),
                    "path",
                )
            })?;
            Ok((
                SourcesDescribed {
                    tables,
                    links: Vec::new(),
                    revision,
                },
                Vec::new(),
            ))
        }
        None => Ok((
            SourcesDescribed {
                tables: Vec::new(),
                links: doc
                    .settings()
                    .sources
                    .iter()
                    .map(|l| state_of(doc, l))
                    .collect(),
                revision,
            },
            Vec::new(),
        )),
    }
}

pub fn describe_validate(cx: &ExecutionContext<'_>, input: &SourcesDescribe) -> CommandResult<()> {
    match describe_run(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn describe_plan(
    cx: &ExecutionContext<'_>,
    input: &SourcesDescribe,
) -> CommandResult<SourcesDescribed> {
    completed(describe_run(cx.doc, input))
}

pub fn describe_execute(
    cx: &mut ExecutionContext<'_>,
    input: SourcesDescribe,
) -> CommandResult<SourcesDescribed> {
    completed(describe_run(cx.doc, &input))
}

// ── cad.sources.edit ───────────────────────────────────────────────────────

fn next_id(doc: &Document) -> String {
    let taken: HashSet<&str> = doc
        .settings()
        .sources
        .iter()
        .map(|s| s.id.as_str())
        .collect();
    (1..)
        .map(|n| format!("kaynak-{n}"))
        .find(|id| !taken.contains(id.as_str()))
        .unwrap_or_default()
}

fn existing_layer(doc: &Document, id: &str, at: &str) -> Result<(), Stop> {
    match doc.layers().get(id) {
        None => Err(refuse(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok; var olan bir katmanın kimliğini verin."),
            at,
        )),
        Some(node) if node.kind != LayerNodeType::Layer => Err(refuse(
            codes::NOT_A_LAYER,
            format!("“{}” bir grup; yalnız katman bağlanır.", node.name),
            at,
        )),
        Some(node) if doc.layers().is_locked(id) => Err(refuse(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; kilidi Katmanlar panelinden açın.",
                node.name
            ),
            at,
        )),
        Some(_) => Ok(()),
    }
}

fn link_table(
    doc: &mut Document,
    path: &str,
    table: &str,
    layer: Option<&str>,
) -> Result<(SourcesEdited, Vec<CommandWarning>), Stop> {
    let p = checked(path, "edit.path")?;
    if doc.settings().sources.len() >= MAX_SOURCES {
        return Err(refuse(
            codes::TOO_MANY_SOURCES,
            format!("Projede {MAX_SOURCES} bağlı katman var; birini çözmeden yenisi bağlanamaz."),
            "edit",
        ));
    }
    if let Some(s) = doc
        .settings()
        .sources
        .iter()
        .find(|s| s.path == path && s.table == table)
    {
        return Err(refuse(
            codes::SOURCE_BOUND,
            format!(
                "“{table}” tablosu zaten “{}” katmanına bağlı ({}).",
                s.layer, s.id
            ),
            "edit.table",
        ));
    }
    let stored = load_or_refuse(p, table, "edit.table")?;
    let project = doc.settings().srid;
    if let Some(s) = stored.srid.filter(|&s| s != project) {
        return Err(refuse(
            codes::CRS_MISMATCH,
            format!(
                "“{table}” tablosu EPSG:{s} sisteminde, proje EPSG:{project}. Bağlı katman dosyayla aynı sistemde olmalı: projeyi EPSG:{s}'de açın ya da tabloyu çevirip içe aktarın."
            ),
            "edit.path",
        ));
    }
    let mut warnings = Vec::new();
    let (layer, objects, layers) = match layer {
        Some(id) => {
            existing_layer(doc, id, "edit.layer")?;
            if let Some(s) = doc.settings().sources.iter().find(|s| s.layer == id) {
                return Err(refuse(
                    codes::SOURCE_BOUND,
                    format!(
                        "“{id}” katmanı zaten “{}” tablosuna bağlı ({}).",
                        s.table, s.id
                    ),
                    "edit.layer",
                ));
            }
            let d = diff(doc, id, &stored);
            if !d.is_empty() || d.mismatch.is_some() {
                return Err(refuse(
                    codes::SOURCE_DIFFERS,
                    format!(
                        "“{id}” katmanı “{table}” tablosunun nesnelerini taşımıyor: tabloda olmayan {}, farklı {}, katmanda olmayan {} nesne. Tabloyu yeni bir katmana bağlayın (layer vermeden).",
                        d.insert.len(),
                        d.update.len(),
                        d.delete.len()
                    ),
                    "edit.layer",
                ));
            }
            (id.to_owned(), 0, Vec::new())
        }
        None => {
            let input = ImportGpkg {
                path: path.to_owned(),
                tables: vec![table.to_owned()],
                layers: Default::default(),
                reproject: false,
            };
            let (out, w) = crate::file_import_gpkg::run_with(doc, &input, true)?;
            // A table without geometry has no system to declare.
            let geometry = stored.layer.geometry.is_some();
            warnings.extend(
                w.into_iter()
                    .filter(|w| geometry || w.code != codes::CRS_UNDECLARED),
            );
            let layer = out
                .tables
                .first()
                .map(|t| t.layer.clone())
                .unwrap_or_default();
            let layers: Vec<ImportedLayer> = out.layers;
            (layer, out.objects, layers)
        }
    };
    let link = SourceLink {
        id: next_id(doc),
        kind: SourceKind::Geopackage,
        path: path.to_owned(),
        table: table.to_owned(),
        digest: layer_digest(doc, &layer),
        layer,
        revision: stored.sha256,
        srid: stored.srid,
        writable: stored.writable,
    };
    set_link(doc, Some(link.clone()), None);
    Ok((
        SourcesEdited {
            link: Some(link),
            objects,
            layers,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

fn edit_run(
    doc: &mut Document,
    input: &SourcesEdit,
) -> Result<(SourcesEdited, Vec<CommandWarning>), Stop> {
    match &input.edit {
        SourcesEditOp::Link { path, table, layer } => {
            link_table(doc, path, table, layer.as_deref())
        }
        SourcesEditOp::Unlink { id } => {
            if link_of(doc, id).is_none() {
                return Err(refuse(
                    codes::SOURCE_NOT_FOUND,
                    format!("“{id}” kimlikli bir bağ yok; cad.sources.describe bağları listeler."),
                    "edit.id",
                ));
            }
            set_link(doc, None, Some(id));
            Ok((
                SourcesEdited {
                    link: None,
                    objects: 0,
                    layers: Vec::new(),
                    revision: doc.revision().to_string(),
                },
                Vec::new(),
            ))
        }
        SourcesEditOp::Relink { id, path } => {
            let Some(mut link) = link_of(doc, id).cloned() else {
                return Err(refuse(
                    codes::SOURCE_NOT_FOUND,
                    format!("“{id}” kimlikli bir bağ yok; cad.sources.describe bağları listeler."),
                    "edit.id",
                ));
            };
            let p = checked(path, "edit.path")?;
            let sha =
                file_sha(p).map_err(|why| refuse(codes::IMPORT_UNREADABLE, why, "edit.path"))?;
            if sha != link.revision {
                return Err(conflict(
                    codes::SOURCE_CHANGED,
                    format!(
                        "{path} katmanın son eşleştiği dosya değil (içeriği farklı). Aynı dosyanın yeni yerini verin; başka bir dosyayı bağlamak için bağı çözüp yeniden bağlayın."
                    ),
                    "edit.path",
                ));
            }
            load_or_refuse(p, &link.table, "edit.path")?;
            link.path = path.clone();
            set_link(doc, Some(link.clone()), None);
            Ok((
                SourcesEdited {
                    link: Some(link),
                    objects: 0,
                    layers: Vec::new(),
                    revision: doc.revision().to_string(),
                },
                Vec::new(),
            ))
        }
    }
}

pub fn edit_validate(cx: &ExecutionContext<'_>, input: &SourcesEdit) -> CommandResult<()> {
    let mut copy = cx.doc.clone();
    match edit_run(&mut copy, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn edit_plan(cx: &ExecutionContext<'_>, input: &SourcesEdit) -> CommandResult<SourcesEdited> {
    let mut copy = cx.doc.clone();
    completed(edit_run(&mut copy, input).map(|(mut out, w)| {
        out.revision = cx.doc.revision().to_string();
        (out, w)
    }))
}

pub fn edit_execute(
    cx: &mut ExecutionContext<'_>,
    input: SourcesEdit,
) -> CommandResult<SourcesEdited> {
    completed(edit_run(cx.doc, &input))
}

// ── cad.sources.sync ───────────────────────────────────────────────────────

/// The undo step of a reload: “Kaynaktan oku: tablo”.
pub fn read_step(table: &str) -> String {
    format!("Kaynaktan oku: {table}")
}

fn sync_run(
    doc: &mut Document,
    input: &SourcesSync,
    write: bool,
) -> Result<(SourcesSynced, Vec<CommandWarning>), Stop> {
    let Some(mut link) = doc
        .settings()
        .sources
        .iter()
        .find(|s| s.layer == input.layer)
        .cloned()
    else {
        return Err(refuse(
            codes::SOURCE_NOT_FOUND,
            format!(
                "“{}” katmanı bir kaynağa bağlı değil; önce cad.sources.edit ile bağlayın.",
                input.layer
            ),
            "layer",
        ));
    };
    if doc.layers().get(&link.layer).is_none() {
        return Err(refuse(
            codes::LAYER_NOT_FOUND,
            format!(
                "Bağlı “{}” katmanı çizimde yok; bağı çözün (cad.sources.edit unlink).",
                link.layer
            ),
            "layer",
        ));
    }
    let path = Path::new(&link.path);
    if !path.is_file() {
        return Err(refuse(
            codes::SOURCE_GONE,
            format!(
                "{} yerinde yok; dosyanın yeni yerini cad.sources.edit relink ile verin.",
                link.path
            ),
            "layer",
        ));
    }
    let stored = load_or_refuse(path, &link.table, "layer")?;
    match input.direction {
        SyncDirection::Write => {
            if !link.writable || !stored.writable {
                return Err(refuse(
                    codes::SOURCE_READONLY,
                    format!(
                        "{} dosyasının “{}” tablosuna yazılamaz (salt okunur).",
                        link.path, link.table
                    ),
                    "layer",
                ));
            }
            if stored.sha256 != link.revision {
                return Err(conflict(
                    codes::SOURCE_CHANGED,
                    format!(
                        "{} katman onunla son eşleştiğinden beri değişti; üzerine yazılmadı. Önce tablodan okuyun (direction read) ya da dosyanın kopyasını bağlayın.",
                        link.path
                    ),
                    "layer",
                ));
            }
            let d = diff(doc, &link.layer, &stored);
            if let Some((uid, why)) = &d.mismatch {
                return Err(refuse(
                    codes::GEOMETRY_MISMATCH,
                    format!("{uid} kimlikli nesne tabloya yazılamaz: {why}."),
                    "layer",
                ));
            }
            let mut warnings = Vec::new();
            let mut losses = Vec::new();
            if d.left > 0 {
                losses.push(format!(
                    "tablonun geometri tutmadığı {} nesne (yazı, ölçü …) yalnız çizimde kaldı",
                    d.left
                ));
            }
            if !d.lost.is_empty() {
                losses.push(format!(
                    "katmanın alanı olmayan öznitelikler yazılmadı ({}); Alanlar penceresinden alan yapın",
                    d.lost.iter().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
            if !losses.is_empty() {
                warnings.push(CommandWarning {
                    code: codes::EXPORT_LOSSES.into(),
                    message: format!("{}: {}.", link.table, losses.join("; ")),
                    path: Some("layer".into()),
                });
            }
            let mut out = SourcesSynced {
                added: d.insert.len() as u32,
                modified: d.update.len() as u32,
                removed: d.delete.len() as u32,
                columns: d.added_columns.iter().map(|c| c.name.clone()).collect(),
                source: stored.sha256.clone(),
                revision: doc.revision().to_string(),
            };
            if !write {
                return Ok((out, warnings));
            }
            if !d.is_empty() || !d.added_columns.is_empty() {
                let edits = Edits {
                    add_columns: d.added_columns,
                    update: d.update,
                    insert: d.insert,
                    delete: d.delete,
                };
                let mut file = GeoPackage::open(path, true).map_err(|e| {
                    refuse(
                        codes::FILE_UNWRITABLE,
                        format!("{} yazmak için açılamadı: {}", link.path, e.0),
                        "layer",
                    )
                })?;
                file.edit(&stored.layer, &edits).map_err(|e| {
                    refuse(
                        codes::FILE_UNWRITABLE,
                        format!("{} yazılamadı, dosya değişmedi: {}", link.path, e.0),
                        "layer",
                    )
                })?;
                file.close().map_err(|e| {
                    refuse(
                        codes::FILE_UNWRITABLE,
                        format!("{} kapatılamadı: {}", link.path, e.0),
                        "layer",
                    )
                })?;
            }
            link.revision =
                file_sha(path).map_err(|why| refuse(codes::FILE_UNWRITABLE, why, "layer"))?;
            link.digest = layer_digest(doc, &link.layer);
            out.source = link.revision.clone();
            set_link(doc, Some(link), None);
            out.revision = doc.revision().to_string();
            Ok((out, warnings))
        }
        SyncDirection::Read => {
            if doc.layers().is_locked(&link.layer) {
                return Err(refuse(
                    codes::LAYER_LOCKED,
                    format!(
                        "“{}” katmanı kilitli; kilidi Katmanlar panelinden açın.",
                        link.layer
                    ),
                    "layer",
                ));
            }
            if !input.discard && layer_digest(doc, &link.layer) != link.digest {
                return Err(conflict(
                    codes::PENDING_CHANGES,
                    format!(
                        "“{}” katmanı dosyayla son eşleştiğinden beri değişti; okumak bu değişiklikleri götürür. Önce yazın (direction write) ya da discard ile tablonun hâlini alın.",
                        link.layer
                    ),
                    "layer",
                ));
            }
            let read = gp::read(path, Some(std::slice::from_ref(&link.table)))
                .map_err(|why| refuse(codes::IMPORT_UNREADABLE, why, "layer"))?;
            let objects = read
                .tables
                .into_iter()
                .next()
                .map(|t| t.objects)
                .unwrap_or_default();
            let wanted: HashSet<Uuid> = objects.iter().map(|(u, _)| *u).collect();
            let mut updates: Vec<(Slot, kentos_contracts::Entity)> = Vec::new();
            let mut adds = Vec::new();
            for (uid, mut e) in objects {
                match doc
                    .slot_of(uid)
                    .and_then(|s| doc.get(s).map(|old| (s, old)))
                {
                    Some((slot, old)) => {
                        if (gp::essence(old) != gp::essence(&e)
                            || old.base().layer_id != link.layer)
                            && let Some(mut next) = gp::with_essence(old, &e)
                        {
                            next.base_mut().layer_id = link.layer.clone();
                            updates.push((slot, next));
                        }
                    }
                    None => {
                        e.base_mut().layer_id = link.layer.clone();
                        adds.push((uid, e));
                    }
                }
            }
            let gone: Vec<Slot> = doc
                .by_layer_with_uids(&link.layer)
                .filter(|(u, _)| !wanted.contains(u))
                .filter_map(|(u, _)| doc.slot_of(u))
                .collect();
            let out = SourcesSynced {
                added: adds.len() as u32,
                modified: updates.len() as u32,
                removed: gone.len() as u32,
                columns: Vec::new(),
                source: stored.sha256.clone(),
                revision: doc.revision().to_string(),
            };
            if !write {
                return Ok((out, Vec::new()));
            }
            let label = read_step(&link.table);
            doc.transact(&label, |doc| {
                if !updates.is_empty() {
                    doc.update_many(updates, &label);
                }
                if !adds.is_empty() {
                    doc.add_many_with_uids(adds, &label).map_err(|e| e.0)?;
                }
                if !gone.is_empty() {
                    doc.remove(&gone);
                }
                Ok::<(), String>(())
            })
            .map_err(|why| refuse(codes::IMPORT_UNREADABLE, why, "layer"))?;
            link.revision = stored.sha256;
            link.digest = layer_digest(doc, &link.layer);
            set_link(doc, Some(link), None);
            Ok((
                SourcesSynced {
                    revision: doc.revision().to_string(),
                    ..out
                },
                Vec::new(),
            ))
        }
    }
}

pub fn sync_validate(cx: &ExecutionContext<'_>, input: &SourcesSync) -> CommandResult<()> {
    let mut copy = cx.doc.clone();
    match sync_run(&mut copy, input, false) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn sync_plan(cx: &ExecutionContext<'_>, input: &SourcesSync) -> CommandResult<SourcesSynced> {
    let mut copy = cx.doc.clone();
    completed(sync_run(&mut copy, input, false))
}

pub fn sync_execute(
    cx: &mut ExecutionContext<'_>,
    input: SourcesSync,
) -> CommandResult<SourcesSynced> {
    completed(sync_run(cx.doc, &input, true))
}

/// `cad.sources.describe` as the registry calls it.
pub mod describe {
    pub use super::{
        describe_execute as execute, describe_plan as plan, describe_validate as validate,
    };
}

/// `cad.sources.edit` as the registry calls it.
pub mod edit {
    pub use super::{edit_execute as execute, edit_plan as plan, edit_validate as validate};
}

/// `cad.sources.sync` as the registry calls it.
pub mod sync {
    pub use super::{sync_execute as execute, sync_plan as plan, sync_validate as validate};
}

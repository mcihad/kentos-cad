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

use kentos_contracts::{
    CommandResult, CommandWarning, ImportGpkg, ImportedLayer, LabelPin, LayerNodeType, LostPin,
    MAX_SOURCES, PinLoss, PinMove, SourceCapabilities, SourceColumn, SourceKind, SourceLink,
    SourceState, SourceTable, SourcesDescribe, SourcesDescribed, SourcesEdit, SourcesEditOp,
    SourcesEdited, SourcesSync, SourcesSynced, SyncDirection, SyncTopology, SyncedPins,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geopackage::{self as gpkg, DataType, Edits, GeoPackage, Geometry, Query, Value};
use serde_json::json;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geopackage as gp;
use crate::provider;

/// Coordinates of an object and its row agree within this (m): the geometry kernel's tolerance.
const SAME: f64 = 1e-6;
/// The same in degrees, for a table in a geographic system (about a millimetre).
const SAME_DEGREES: f64 = 1e-11;

/// How a bound layer's objects move between the project's system and its table's (docs/adr/0504 §1).
struct Mover {
    project: kentos_geometry_core::crs::System,
    source: kentos_geometry_core::crs::System,
    choices: Vec<kentos_geometry_core::crs::Choice>,
}

/// The move between the project and a table of `srid`: none in the same system or for a table that declares none;
/// why not, when one of the systems has no transform.
fn mover_for(doc: &Document, srid: Option<u32>) -> Result<Option<Mover>, String> {
    let settings = doc.settings();
    let Some(srid) = srid.filter(|&s| s != settings.srid) else {
        return Ok(None);
    };
    let Some(project) = kentos_project::systems::own(settings).and_then(|n| n.system) else {
        return Err(format!(
            "tablo EPSG:{srid} sisteminde; projenin koordinat sistemi yok (yerel proje), çevrilemez"
        ));
    };
    let Some(source) = kentos_project::crs::system(srid).and_then(|s| s.transform_system()) else {
        return Err(format!(
            "tablonun sistemi EPSG:{srid} KentOS'un kayıtlı sistemlerinde yok"
        ));
    };
    Ok(Some(Mover {
        project,
        source,
        choices: kentos_project::systems::choices(settings),
    }))
}

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

/// The layer's digest: the SHA-256 of its objects' kinds, geometries and attributes by lasting id (not their look):
/// the canonical JSON of `[[uid, essence], …]` in the ids' order, hashed one object at a time (plan/gis.md CBS-42: a
/// million objects' JSON is not built at once; the bytes, and so the digest, are those of `digest_of` on the list).
pub fn layer_digest(doc: &Document, layer: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut items: Vec<(Uuid, &kentos_contracts::Entity)> = doc.by_layer_with_uids(layer).collect();
    items.sort_by_key(|(u, _)| *u);
    let mut hasher = Sha256::new();
    hasher.update(b"[");
    for (i, (u, e)) in items.into_iter().enumerate() {
        if i > 0 {
            hasher.update(b",");
        }
        let item = json!([u.to_string(), gp::essence(e)]);
        hasher.update(kentos_contracts::canonical::canonical_json(&item).as_bytes());
    }
    hasher.update(b"]");
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The file's SHA-256, read a piece at a time.
fn file_sha(path: &Path) -> Result<String, String> {
    std::fs::File::open(path)
        .and_then(|f| kentos_contracts::encoding::sha256_reader_hex(std::io::BufReader::new(f)))
        .map(|(_, sha)| sha)
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
    /// The rows as the import reads them, in the table's own system.
    objects: Vec<(Uuid, kentos_contracts::Entity)>,
    /// A part's working copy (docs/adr/0510): the table's rows outside the area, by lasting id. An object of the layer
    /// moved out of the area is written over its row, not as a new one.
    outside: HashMap<Uuid, i64>,
    /// The columns as the layer's fields, and what does not come over (linking onto a new layer).
    fields: Vec<kentos_contracts::LayerField>,
    losses: BTreeSet<String>,
    writable: bool,
    sha256: String,
}

impl Stored {
    /// Leaves out the rows the move into the project's system does not reach (an arc, a vertex outside it): the
    /// layer never holds them, so a write must not delete them.
    fn reachable(&mut self, mover: Option<&Mover>) {
        let Some(m) = mover else { return };
        let rows = &mut self.rows;
        self.objects.retain(|(uid, e)| {
            let mut e = e.clone();
            let ok =
                kentos_formats::reproject::move_entity(&mut e, &m.source, &m.project, &m.choices)
                    .is_ok();
            if !ok {
                rows.remove(uid);
            }
            ok
        });
    }

    /// The table's objects in the project's system (those that reach it).
    fn in_project(&self, mover: Option<&Mover>) -> Vec<(Uuid, kentos_contracts::Entity)> {
        let mut out = self.objects.clone();
        if let Some(m) = mover {
            out.retain_mut(|(_, e)| {
                kentos_formats::reproject::move_entity(e, &m.source, &m.project, &m.choices).is_ok()
            });
        }
        out
    }
}

/// A file's table with its rows; with `area` (in the table's system, docs/adr/0510) only the rows whose box meets it,
/// the others' lasting ids kept apart.
fn load(path: &Path, table: &str, area: Option<[f64; 4]>) -> Result<Stored, String> {
    let read = gp::read_in(path, Some(&[table.to_owned()]), area)?;
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
        .rows(
            &layer,
            &Query {
                bbox: area,
                ..Query::default()
            },
        )
        .map_err(|e| e.0)?
        .into_iter()
        .map(|r| (r.fid, r))
        .collect();
    let outside: HashMap<Uuid, i64> = match area {
        None => HashMap::new(),
        Some(_) => {
            let ids = file.uids(table).map_err(|e| e.0)?;
            file.fids(&layer)
                .map_err(|e| e.0)?
                .into_iter()
                .filter(|fid| !raw.contains_key(fid))
                .map(|fid| {
                    let uid = ids
                        .get(&fid)
                        .and_then(|u| Uuid::parse_str(u).ok())
                        .unwrap_or_else(|| gp::row_uid(table, fid));
                    (uid, fid)
                })
                .collect()
        }
    };
    Ok(Stored {
        outside,
        srid: layer
            .srs_id
            .and_then(|id| read.systems.get(&id).cloned().flatten())
            .filter(|(org, _)| org == "EPSG")
            .and_then(|(_, id)| u32::try_from(id).ok()),
        writable: matches!(layer.data_type, DataType::Features | DataType::Attributes)
            && !readonly(path),
        sha256: read.sha256,
        ..stored_of(layer, &mut raw, t)
    })
}

/// A table's rows by lasting id, as read (`t`) and as stored (`raw`, taken by fid).
fn stored_of(layer: gpkg::Layer, raw: &mut HashMap<i64, gpkg::Row>, t: gp::TableRead) -> Stored {
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
    Stored {
        layer,
        srid: None,
        rows,
        objects: t.objects,
        outside: HashMap::new(),
        fields: t.fields,
        losses: t.losses,
        writable: false,
        sha256: String::new(),
    }
}

// ── Databases (docs/adr/0505) ──────────────────────────────────────────────

/// Where a source is: a GeoPackage file or a database.
#[derive(Clone, Copy)]
enum Origin<'a> {
    File(&'a Path),
    Database(&'a str),
}

impl Origin<'_> {
    fn kind(self) -> SourceKind {
        match self {
            Origin::File(_) => SourceKind::Geopackage,
            Origin::Database(_) => SourceKind::Postgis,
        }
    }
}

/// The source at `address`: a database's address (no password in it) or a GeoPackage's path.
fn origin<'a>(address: &'a str, at: &str) -> Result<Origin<'a>, Stop> {
    if provider::is_database(address) {
        if let Some(why) = provider::address_problem(address) {
            return Err(refuse(
                codes::INVALID_ADDRESS,
                format!("“{address}”: {why}."),
                at,
            ));
        }
        return Ok(Origin::Database(address));
    }
    checked(address, at).map(Origin::File)
}

/// A bound layer's source, without checking that it is there.
fn origin_of(link: &SourceLink) -> Origin<'_> {
    origin_from(link.kind, &link.path)
}

fn origin_from(kind: SourceKind, path: &str) -> Origin<'_> {
    match kind {
        SourceKind::Geopackage => Origin::File(Path::new(path)),
        SourceKind::Postgis => Origin::Database(path),
    }
}

fn database_provider(at: &str) -> Result<&'static dyn provider::TableProvider, Stop> {
    provider::postgis().ok_or_else(|| {
        refuse(
            codes::SOURCE_UNREACHABLE,
            "Bu ortamda PostGIS sağlayıcısı yok; masaüstü, Python ve MCP onu kaydeder.",
            at,
        )
    })
}

/// Where a refusal points: the address's field and the table's (the same field when one names both).
#[derive(Clone, Copy)]
struct At<'a> {
    address: &'a str,
    table: &'a str,
}

const fn at_one(field: &str) -> At<'_> {
    At {
        address: field,
        table: field,
    }
}

fn provider_stop(e: provider::ProviderError, address: &str, at: At<'_>) -> Stop {
    match e {
        provider::ProviderError::Unreachable(why) => refuse(
            codes::SOURCE_UNREACHABLE,
            format!("{address} veritabanına ulaşılamadı: {why}"),
            at.address,
        ),
        provider::ProviderError::NoTable(t) => refuse(
            codes::TABLE_NOT_FOUND,
            format!(
                "“{t}” tablosu veritabanında yok; şema.tablo adıyla verin (cad.sources.describe listeler)."
            ),
            at.table,
        ),
        provider::ProviderError::Changed => conflict(
            codes::SOURCE_CHANGED,
            format!(
                "{address} tablosu katman onunla son eşleştiğinden beri değişti; hiçbir şey yazılmadı. Önce tablodan okuyun (direction read)."
            ),
            at.table,
        ),
        provider::ProviderError::Refused(why) => refuse(
            codes::FILE_UNWRITABLE,
            format!("{address} veritabanına yazılamadı, hiçbir şey değişmedi: {why}"),
            at.table,
        ),
    }
}

/// A database's name in its address, for the lasting ids of its rows.
fn database_name(address: &str) -> &str {
    let rest = address
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or_default();
    let tail = rest.split_once('/').map(|(_, t)| t).unwrap_or_default();
    tail.split_once('?').map_or(tail, |(d, _)| d)
}

fn load_database(address: &str, table: &str, at: At<'_>) -> Result<Stored, Stop> {
    let p = database_provider(at.address)?;
    let r = p
        .read(address, table)
        .map_err(|e| provider_stop(e, address, at))?;
    let database = database_name(address).to_owned();
    let name = r.layer.name.clone();
    let t = gp::table_of_rows(&r.layer, &r.rows, &r.uids, &|pk| {
        gp::database_row_uid(&database, &name, pk)
    })
    .map_err(|why| refuse(codes::IMPORT_UNREADABLE, why, at.table))?;
    let mut raw: HashMap<i64, gpkg::Row> = r.rows.into_iter().map(|row| (row.fid, row)).collect();
    Ok(Stored {
        srid: r.srid,
        writable: r.writable,
        sha256: r.revision,
        ..stored_of(r.layer, &mut raw, t)
    })
}

/// A file's table without its rows (plan/gis.md CBS-42): linking it onto a new layer reads the rows once, through
/// the import; the table, its system, whether it is writable and the file's SHA-256 are all the link needs.
fn load_meta(path: &Path, table: &str) -> Result<Stored, String> {
    let file = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let Some(layer) = file
        .layers()
        .map_err(|e| e.0)?
        .into_iter()
        .find(|l| l.name == table)
    else {
        return Err(format!("“{table}” tablosu dosyada yok"));
    };
    let srid = match layer.srs_id {
        Some(id) if id > 0 => file
            .srs(id)
            .map_err(|e| e.0)?
            .filter(|s| s.definition != "undefined" && s.organization.eq_ignore_ascii_case("EPSG"))
            .and_then(|s| u32::try_from(s.organization_id).ok()),
        _ => None,
    };
    let writable =
        matches!(layer.data_type, DataType::Features | DataType::Attributes) && !readonly(path);
    drop(file);
    Ok(Stored {
        layer,
        srid,
        rows: HashMap::new(),
        objects: Vec::new(),
        outside: HashMap::new(),
        fields: Vec::new(),
        losses: BTreeSet::new(),
        writable,
        sha256: file_sha(path)?,
    })
}

fn load_at(
    origin: Origin<'_>,
    table: &str,
    at: At<'_>,
    area: Option<[f64; 4]>,
) -> Result<Stored, Stop> {
    match origin {
        Origin::File(path) => load_or_refuse(path, table, at.table, area),
        Origin::Database(address) => load_database(address, table, at),
    }
}

fn load_or_refuse(
    path: &Path,
    table: &str,
    at: &str,
    area: Option<[f64; 4]>,
) -> Result<Stored, Stop> {
    refused_load(path, table, at, load(path, table, area))
}

/// A bound layer's area in its table's system (docs/adr/0510): the project's box moved across, its sides densified;
/// none for the whole table, or when the box does not reach the table's system.
fn table_area(area: Option<[f64; 4]>, mover: Option<&Mover>) -> Result<Option<[f64; 4]>, ()> {
    let Some(a) = area else { return Ok(None) };
    match mover {
        None => Ok(Some(a)),
        Some(m) => {
            kentos_geometry_core::crs::transform_bounds(&m.project, &m.source, a, 16, &m.choices)
                .map(Some)
                .ok_or(())
        }
    }
}

fn refused_load(
    path: &Path,
    table: &str,
    at: &str,
    loaded: Result<Stored, String>,
) -> Result<Stored, Stop> {
    loaded.map_err(|why| {
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

fn same_geometry(a: Option<&Geometry>, b: Option<&Geometry>, tol: f64) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            let (mut sa, mut xa, mut sb, mut xb) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            walk(a, &mut sa, &mut xa);
            walk(b, &mut sb, &mut xb);
            sa == sb && xa.iter().zip(&xb).all(|(p, q)| (p - q).abs() <= tol)
        }
        _ => false,
    }
}

/// Whether a cell the layer would write is the table's: by the column's reading rule (docs/adr/0512) when the column is
/// known, else exactly.
fn same_value(a: &Value, b: &Value, kind: Option<&gpkg::ColumnKind>) -> bool {
    match kind {
        Some(k) => gp::same_cell(a, b, k),
        None => a == b,
    }
}

fn diff(doc: &Document, layer: &str, s: &Stored, mover: Option<&Mover>) -> Diff {
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
    let tol = if mover.is_some_and(|m| m.source.is_geographic()) {
        SAME_DEGREES
    } else {
        SAME
    };
    let mut seen = HashSet::new();
    for (uid, e) in doc.by_layer_with_uids(layer) {
        for k in e.base().attrs.keys() {
            if !columns.iter().any(|c| c.name.eq_ignore_ascii_case(k)) {
                d.lost.insert(k.clone());
            }
        }
        // In the table's own system (docs/adr/0504 §1); an object the move does not take stays in the drawing.
        let moved;
        let e = match mover {
            Some(m) => {
                let mut c = e.clone();
                if kentos_formats::reproject::move_entity(&mut c, &m.project, &m.source, &m.choices)
                    .is_err()
                {
                    d.left += 1;
                    continue;
                }
                moved = c;
                &moved
            }
            None => e,
        };
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
            // Moved out of a part's area (docs/adr/0510): its row is written, not a new one.
            None if s.outside.contains_key(&uid) => d.update.push((s.outside[&uid], row)),
            None => d.insert.push((uid.to_string(), row)),
            Some((fid, g, values)) => {
                seen.insert(uid);
                let same = same_geometry(row.geometry.as_ref(), g.as_ref(), tol)
                    && row.values.iter().enumerate().all(|(i, v)| {
                        same_value(
                            v,
                            if i < width { &values[i] } else { &Value::Null },
                            columns.get(i).map(|c| &c.kind),
                        )
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

/// Why `key` cannot name the rows `objects` are (docs/adr/0533 §1): each holds the attribute with a value, no two
/// alike. `path` is where the refusal points.
fn key_check<'a>(
    key: &str,
    objects: impl Iterator<Item = (Uuid, &'a kentos_contracts::Entity)>,
    path: &str,
) -> Result<(), Stop> {
    let mut seen: HashMap<&str, Uuid> = HashMap::new();
    let (mut missing, mut first) = (0, None);
    for (uid, e) in objects {
        match e.base().attrs.get(key).map(String::as_str) {
            None | Some("") => {
                missing += 1;
                first.get_or_insert(uid);
            }
            Some(v) => {
                if let Some(other) = seen.insert(v, uid) {
                    return Err(refuse(
                        codes::INVALID_KEY,
                        format!(
                            "“{key}” alanının “{v}” değeri iki nesnede ({other} ve {uid}); anahtar iki nesnede aynı olamaz."
                        ),
                        path,
                    ));
                }
            }
        }
    }
    if let Some(uid) = first {
        return Err(refuse(
            codes::INVALID_KEY,
            format!(
                "“{key}” alanı {missing} nesnede yok ya da boş (örneğin {uid}); anahtar her nesnede bir değer taşımalı."
            ),
            path,
        ));
    }
    Ok(())
}

/// What a read does to the label pins of a bound layer's objects (docs/adr/0533 §1): the objects holding pins as
/// they are in the drawing, against the table's rows as read (`objects`). An object whose row is there stays as it
/// is. With a key, a row id that is another record now (the key's value differs) or whose row is gone leaves its
/// pins to the row of the old value when the table has exactly one; otherwise they are lost, and said (deleted,
/// outside the area, another record, or the row of the key has pins of its own).
struct PinPlan {
    /// The pins an object holds after the read, where they are not its own: set on a row they followed to, emptied
    /// on one that is another record now.
    set: HashMap<Uuid, Vec<LabelPin>>,
    report: Option<SyncedPins>,
    warnings: Vec<CommandWarning>,
}

fn follow_pins(
    doc: &Document,
    link: &SourceLink,
    objects: &[(Uuid, kentos_contracts::Entity)],
    outside: &HashMap<Uuid, i64>,
) -> PinPlan {
    let bearers: Vec<(Uuid, &kentos_contracts::Entity)> = doc
        .by_layer_with_uids(&link.layer)
        .filter(|(_, e)| !e.base().label_pins.is_empty())
        .collect();
    let mut plan = PinPlan {
        set: HashMap::new(),
        report: None,
        warnings: Vec::new(),
    };
    if bearers.is_empty() {
        return plan;
    }
    let key = link.key.as_deref();
    let new: HashMap<Uuid, &kentos_contracts::Entity> =
        objects.iter().map(|(u, e)| (*u, e)).collect();
    let value_of = |e: &kentos_contracts::Entity| -> Option<String> {
        key.and_then(|k| e.base().attrs.get(k))
            .filter(|v| !v.is_empty())
            .cloned()
    };
    let mut by_key: HashMap<String, Vec<Uuid>> = HashMap::new();
    if key.is_some() {
        for (u, e) in objects {
            if let Some(v) = value_of(e) {
                by_key.entry(v).or_default().push(*u);
            }
        }
    }
    let repeated = by_key.values().any(|l| l.len() > 1);
    let mut kept = 0u32;
    let mut moves: Vec<(Uuid, Uuid, String)> = Vec::new();
    let mut lost: Vec<(Uuid, PinLoss)> = Vec::new();
    for (uid, old) in &bearers {
        let old_value = value_of(old);
        let here = new.get(uid);
        let stays = match (here, key.is_some(), &old_value) {
            (Some(n), true, Some(v)) => value_of(n).as_ref() == Some(v),
            (Some(_), _, _) => true,
            (None, _, _) => false,
        };
        if stays {
            kept += 1;
            continue;
        }
        let target = old_value
            .as_ref()
            .and_then(|v| by_key.get(v).map(|l| (v, l)))
            .filter(|(_, l)| l.len() == 1)
            .map(|(v, l)| (l[0], v.clone()))
            .filter(|(t, _)| t != uid);
        match target {
            Some((to, v)) => moves.push((*uid, to, v)),
            None => lost.push((
                *uid,
                if here.is_some() {
                    PinLoss::Rebound
                } else if outside.contains_key(uid) {
                    PinLoss::Outside
                } else {
                    PinLoss::Deleted
                },
            )),
        }
    }
    let bearer_pins: HashMap<Uuid, &Vec<LabelPin>> = bearers
        .iter()
        .map(|(u, e)| (*u, &e.base().label_pins))
        .collect();
    let leaving: HashSet<Uuid> = moves.iter().map(|(f, ..)| *f).collect();
    let mut moved = Vec::new();
    for (from, to, v) in moves {
        // The row of the key keeps its own pins when it is not itself leaving.
        let taken = bearer_pins.contains_key(&to) && !leaving.contains(&to);
        if taken {
            lost.push((from, PinLoss::Taken));
        } else {
            plan.set.insert(to, bearer_pins[&from].clone());
            moved.push(PinMove {
                from: from.to_string(),
                to: to.to_string(),
                key: v,
            });
        }
    }
    // What an object that stays in the layer as another record holds is emptied, where nothing moved onto it.
    for uid in leaving.iter().chain(lost.iter().map(|(u, _)| u)) {
        if new.contains_key(uid) {
            plan.set.entry(*uid).or_default();
        }
    }
    moved.sort_by(|a, b| a.from.cmp(&b.from));
    let mut lost: Vec<LostPin> = lost
        .into_iter()
        .map(|(u, reason)| LostPin {
            uid: u.to_string(),
            reason,
        })
        .collect();
    lost.sort_by(|a, b| a.uid.cmp(&b.uid));
    if !lost.is_empty() {
        let count = |r: PinLoss| lost.iter().filter(|l| l.reason == r).count();
        let mut parts = Vec::new();
        for (r, what) in [
            (PinLoss::Deleted, "satırı tabloda yok"),
            (PinLoss::Outside, "satırı katmanın alanının dışında"),
            (
                PinLoss::Rebound,
                "satır kimliği artık başka bir kayıt (anahtar değeri değişmiş)",
            ),
            (PinLoss::Taken, "anahtarının satırında zaten iğneler var"),
        ] {
            if count(r) > 0 {
                parts.push(format!("{} nesnenin {what}", count(r)));
            }
        }
        plan.warnings.push(CommandWarning {
            code: codes::LABEL_PINS_LOST.into(),
            message: format!(
                "{}: etiket iğneleri korunamadı ({}); okumayı geri alırsanız dönerler.",
                link.table,
                parts.join("; ")
            ),
            path: Some("layer".into()),
        });
    }
    if !moved.is_empty() {
        plan.warnings.push(CommandWarning {
            code: codes::LABEL_PINS_MOVED.into(),
            message: format!(
                "{}: {} nesnenin etiket iğneleri “{}” anahtarının yeni satırına taşındı.",
                link.table,
                moved.len(),
                key.unwrap_or_default()
            ),
            path: Some("layer".into()),
        });
    }
    if repeated {
        plan.warnings.push(CommandWarning {
            code: codes::KEY_NOT_UNIQUE.into(),
            message: format!(
                "{}: “{}” alanı tabloda tekrarlanıyor; o değerlerin satırlarına iğneler anahtarla taşınmaz.",
                link.table,
                key.unwrap_or_default()
            ),
            path: Some("layer".into()),
        });
    }
    plan.report = Some(SyncedPins { kept, moved, lost });
    plan
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

/// A database's tables as `cad.sources.describe` says them.
fn describe_database(address: &str) -> Result<Vec<SourceTable>, Stop> {
    let p = database_provider("path")?;
    let tables = p
        .tables(address)
        .map_err(|e| provider_stop(e, address, at_one("path")))?;
    Ok(tables
        .into_iter()
        .map(|t| SourceTable {
            columns: t
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
                insert: t.writable,
                update: t.writable,
                delete: t.writable,
                transaction: true,
                z: t.z,
                m: t.m,
                curves: true,
            },
            name: t.name,
            data: t.data,
            rows: t.rows,
            geometry: t.geometry,
            srid: t.srid,
        })
        .collect())
}

fn state_of(doc: &Document, link: &SourceLink) -> SourceState {
    let edited = layer_digest(doc, &link.layer) != link.digest;
    let mut state = SourceState {
        link: link.clone(),
        found: false,
        changed: false,
        edited,
        added: 0,
        modified: 0,
        removed: 0,
    };
    let area = mover_for(doc, link.srid)
        .ok()
        .and_then(|m| table_area(link.extent, m.as_ref()).ok())
        .flatten();
    let loaded = match origin_of(link) {
        Origin::File(path) => {
            state.found = path.is_file();
            load(path, &link.table, area).ok()
        }
        // A database is there when it answers.
        Origin::Database(address) => {
            let s = load_database(address, &link.table, at_one("")).ok();
            state.found = s.is_some();
            s
        }
    };
    if let (Some(mut s), Ok(mover)) = (loaded, mover_for(doc, link.srid)) {
        state.changed = s.sha256 != link.revision;
        s.reachable(mover.as_ref());
        let d = diff(doc, &link.layer, &s, mover.as_ref());
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
            let tables = match origin(path, "path")? {
                Origin::File(p) => describe_file(p).map_err(|why| {
                    refuse(
                        codes::IMPORT_UNREADABLE,
                        format!("{path} GeoPackage olarak okunamadı: {why}"),
                        "path",
                    )
                })?,
                Origin::Database(address) => describe_database(address)?,
            };
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
    reproject: bool,
    extent: Option<[f64; 4]>,
    key: Option<&str>,
) -> Result<(SourcesEdited, Vec<CommandWarning>), Stop> {
    let at = origin(path, "edit.path")?;
    if let Some(k) = key
        && let Some(why) = kentos_contracts::key_problem(k)
    {
        return Err(refuse(
            codes::INVALID_KEY,
            format!("Anahtar alanı {why}."),
            "edit.key",
        ));
    }
    if let Some(e) = &extent {
        let why = kentos_contracts::extent_problem(e).map(|w| format!("Alan sınırının {w}."));
        let why = why.or_else(|| {
            layer.map(|_| {
                "Var olan katmana bağlamada alan sınırı olmaz: katman tablonun bütün nesnelerini taşır. Yeni katmana bağlayın (layer vermeden).".to_owned()
            })
        });
        let why = why.or_else(|| {
            matches!(at, Origin::Database(_)).then(|| {
                "Veritabanı tablosunda alan sınırı henüz yok; GeoPackage tablosu için verin."
                    .to_owned()
            })
        });
        if let Some(why) = why {
            return Err(refuse(codes::INVALID_EXTENT, why, "edit.extent"));
        }
    }
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
    // Onto a new layer from a file the import reads the rows; the table's own rows are read only to compare a layer.
    let mut stored = match at {
        Origin::File(p) if layer.is_none() => {
            refused_load(p, table, "edit.table", load_meta(p, table))?
        }
        _ => load_at(
            at,
            table,
            At {
                address: "edit.path",
                table: "edit.table",
            },
            None,
        )?,
    };
    let project = doc.settings().srid;
    if let Some(s) = stored.srid.filter(|&s| s != project && !reproject) {
        return Err(refuse(
            codes::CRS_MISMATCH,
            format!(
                "“{table}” tablosu EPSG:{s} sisteminde, proje EPSG:{project}. Katmanın nesneleri projenin sisteminde, tablonunkiler kendi sisteminde tutulsun diye reproject verin ya da projeyi EPSG:{s}'de açın."
            ),
            "edit.reproject",
        ));
    }
    let mover = mover_for(doc, stored.srid).map_err(|why| {
        refuse(
            codes::CRS_MISMATCH,
            format!("“{table}” bağlanamaz: {why}."),
            "edit.reproject",
        )
    })?;
    if extent.is_some() && stored.layer.geometry.is_none() {
        return Err(refuse(
            codes::INVALID_EXTENT,
            format!("“{table}” tablosunun geometrisi yok; alan sınırı geometrili tabloda olur."),
            "edit.extent",
        ));
    }
    let area = table_area(extent, mover.as_ref()).map_err(|()| {
        refuse(
            codes::INVALID_EXTENT,
            format!("Alan sınırı “{table}” tablosunun sistemine çevrilemiyor."),
            "edit.extent",
        )
    })?;
    stored.reachable(mover.as_ref());
    // A key names the rows the layer will hold: the table's own, or the layer given's objects.
    if let Some(k) = key
        && layer.is_none()
    {
        // The import reads a file's rows later; a key is checked against them now, before anything is written.
        let rows = match at {
            Origin::File(p) => {
                let mut all = refused_load(p, table, "edit.table", load(p, table, area))?;
                all.reachable(mover.as_ref());
                all.in_project(mover.as_ref())
            }
            Origin::Database(_) => stored.in_project(mover.as_ref()),
        };
        key_check(k, rows.iter().map(|(u, e)| (*u, e)), "edit.key")?;
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
            let d = diff(doc, id, &stored, mover.as_ref());
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
            if let Some(k) = key {
                key_check(k, doc.by_layer_with_uids(id), "edit.key")?;
            }
            (id.to_owned(), 0, Vec::new())
        }
        None if matches!(at, Origin::Database(_)) => {
            let (layer, objects, layers, w) =
                link_database_layer(doc, path, &stored, mover.as_ref())?;
            warnings.extend(w);
            (layer, objects, layers)
        }
        None => {
            let input = ImportGpkg {
                path: path.to_owned(),
                tables: vec![table.to_owned()],
                layers: Default::default(),
                reproject: mover.is_some(),
            };
            let (out, w) = crate::file_import_gpkg::run_with(doc, &input, true, area)?;
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
    // The table's rows are in the drawing now: let them go before the layer is digested.
    let Stored {
        sha256: revision,
        srid,
        writable,
        ..
    } = stored;
    let link = SourceLink {
        id: next_id(doc),
        kind: at.kind(),
        path: path.to_owned(),
        relative: None,
        table: table.to_owned(),
        digest: layer_digest(doc, &layer),
        layer,
        revision,
        srid,
        extent,
        key: key.map(str::to_owned),
        writable,
    };
    set_link(doc, Some(link.clone()), None);
    Ok((
        SourcesEdited {
            link: Some(link),
            objects,
            layers,
            pins: None,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

/// A database table's rows onto a new layer with the table's columns as fields, its lasting ids kept, one undo step
/// (“PostGIS: şema.tablo”); the rows moved into the project's system when the table is in another.
fn link_database_layer(
    doc: &mut Document,
    address: &str,
    stored: &Stored,
    mover: Option<&Mover>,
) -> Result<(String, u32, Vec<ImportedLayer>, Vec<CommandWarning>), Stop> {
    let source = stored.layer.name.clone();
    let name = source
        .rsplit_once('.')
        .map_or(source.as_str(), |(_, t)| t)
        .to_owned();
    let objects = stored.in_project(mover);
    let count = objects.len() as u32;
    let plan = crate::import_apply::ImportPlan {
        label: format!("PostGIS: {source}"),
        layers: vec![(
            source.clone(),
            crate::import_apply::LayerTarget::New {
                name,
                style: Box::new(kentos_domain::default_style()),
                visible: true,
                locked: false,
                fields: stored.fields.clone(),
                plot: None,
                frozen: None,
            },
        )],
        group: None,
    };
    if let Some((uid, _)) = objects.iter().find(|(u, _)| doc.slot_of(*u).is_some()) {
        return Err(refuse(
            codes::ENTITY_EXISTS,
            format!(
                "Tablodaki bir satırın kalıcı kimliği ({uid}) çizimde zaten bir nesnenin: {address} › {source} daha önce alınmış."
            ),
            "edit.table",
        ));
    }
    let applied = crate::import_apply::apply_import_with_uids(doc, objects, &plan)
        .map_err(|why| refuse(codes::IMPORT_UNREADABLE, why, "edit.table"))?;
    let layer = applied
        .targets
        .iter()
        .find(|(s, _)| *s == source)
        .map(|(_, t)| t.clone())
        .unwrap_or_default();
    let mut warnings = Vec::new();
    if !stored.losses.is_empty() {
        warnings.push(CommandWarning {
            code: codes::IMPORT_LOSSES.into(),
            message: format!(
                "{source} tablosunun bazı değerleri olduğu gibi alınamadı: {}.",
                stored.losses.iter().cloned().collect::<Vec<_>>().join("; ")
            ),
            path: Some("edit.table".into()),
        });
    }
    let layers = applied
        .made
        .iter()
        .map(|(id, name)| ImportedLayer {
            id: id.clone(),
            name: name.clone(),
        })
        .collect();
    Ok((layer, count, layers, warnings))
}

fn edit_run(
    doc: &mut Document,
    input: &SourcesEdit,
) -> Result<(SourcesEdited, Vec<CommandWarning>), Stop> {
    match &input.edit {
        SourcesEditOp::Link {
            path,
            table,
            layer,
            reproject,
            extent,
            key,
        } => link_table(
            doc,
            path,
            table,
            layer.as_deref(),
            *reproject,
            *extent,
            key.as_deref(),
        ),
        SourcesEditOp::Key { id, key } => {
            let Some(mut link) = link_of(doc, id).cloned() else {
                return Err(refuse(
                    codes::SOURCE_NOT_FOUND,
                    format!("“{id}” kimlikli bir bağ yok; cad.sources.describe bağları listeler."),
                    "edit.id",
                ));
            };
            if let Some(k) = key {
                if let Some(why) = kentos_contracts::key_problem(k) {
                    return Err(refuse(
                        codes::INVALID_KEY,
                        format!("Anahtar alanı {why}."),
                        "edit.key",
                    ));
                }
                key_check(k, doc.by_layer_with_uids(&link.layer), "edit.key")?;
            }
            link.key = key.clone();
            set_link(doc, Some(link.clone()), None);
            Ok((
                SourcesEdited {
                    link: Some(link),
                    objects: 0,
                    layers: Vec::new(),
                    pins: None,
                    revision: doc.revision().to_string(),
                },
                Vec::new(),
            ))
        }
        SourcesEditOp::Extent { id, extent } => change_extent(doc, id, *extent),
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
                    pins: None,
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
            let at = origin(path, "edit.path")?;
            if at.kind() != link.kind {
                return Err(refuse(
                    codes::INVALID_ADDRESS,
                    format!(
                        "{path} bağın kaynağıyla aynı türden değil (dosya ya da veritabanı); başka türden bir kaynak için bağı çözüp yeniden bağlayın."
                    ),
                    "edit.path",
                ));
            }
            let sha = match at {
                Origin::File(p) => {
                    file_sha(p).map_err(|why| refuse(codes::IMPORT_UNREADABLE, why, "edit.path"))?
                }
                Origin::Database(_) => load_at(at, &link.table, at_one("edit.path"), None)?.sha256,
            };
            if sha != link.revision {
                return Err(conflict(
                    codes::SOURCE_CHANGED,
                    format!(
                        "{path} katmanın son eşleştiği dosya değil (içeriği farklı). Aynı dosyanın yeni yerini verin; başka bir dosyayı bağlamak için bağı çözüp yeniden bağlayın."
                    ),
                    "edit.path",
                ));
            }
            // Only the table is asked for: its rows are read again by the next sync.
            match at {
                Origin::File(p) => {
                    refused_load(p, &link.table, "edit.path", load_meta(p, &link.table))?;
                }
                Origin::Database(_) => {
                    load_at(at, &link.table, at_one("edit.path"), None)?;
                }
            }
            link.path = path.clone();
            // The next save writes the new place's relative path (docs/adr/0506).
            link.relative = None;
            set_link(doc, Some(link.clone()), None);
            Ok((
                SourcesEdited {
                    link: Some(link),
                    objects: 0,
                    layers: Vec::new(),
                    pins: None,
                    revision: doc.revision().to_string(),
                },
                Vec::new(),
            ))
        }
    }
}

/// The area of a part's working copy changed (docs/adr/0533 §2): the table read again in the new area as one undo
/// step. The layer must match its table, so nothing the layer holds is lost with the rows that leave it; the label
/// pins of an object that leaves are said.
fn change_extent(
    doc: &mut Document,
    id: &str,
    extent: Option<[f64; 4]>,
) -> Result<(SourcesEdited, Vec<CommandWarning>), Stop> {
    let Some(link) = link_of(doc, id).cloned() else {
        return Err(refuse(
            codes::SOURCE_NOT_FOUND,
            format!("“{id}” kimlikli bir bağ yok; cad.sources.describe bağları listeler."),
            "edit.id",
        ));
    };
    if let Some(e) = &extent {
        let why = kentos_contracts::extent_problem(e).map(|w| format!("Alan sınırının {w}."));
        let why = why.or_else(|| {
            (link.kind != SourceKind::Geopackage).then(|| {
                "Veritabanı tablosunda alan sınırı henüz yok; GeoPackage tablosu için verin."
                    .to_owned()
            })
        });
        if let Some(why) = why {
            return Err(refuse(codes::INVALID_EXTENT, why, "edit.extent"));
        }
        if let Origin::File(p) = origin_of(&link)
            && p.is_file()
            && refused_load(p, &link.table, "edit.id", load_meta(p, &link.table))?
                .layer
                .geometry
                .is_none()
        {
            return Err(refuse(
                codes::INVALID_EXTENT,
                format!(
                    "“{}” tablosunun geometrisi yok; alan sınırı geometrili tabloda olur.",
                    link.table
                ),
                "edit.extent",
            ));
        }
    }
    if extent == link.extent {
        return Ok((
            SourcesEdited {
                link: Some(link),
                objects: 0,
                layers: Vec::new(),
                pins: None,
                revision: doc.revision().to_string(),
            },
            Vec::new(),
        ));
    }
    if layer_digest(doc, &link.layer) != link.digest {
        return Err(conflict(
            codes::PENDING_CHANGES,
            format!(
                "“{}” katmanı dosyayla son eşleştiğinden beri değişti; alanı değiştirmek bu değişiklikleri götürür. Önce yazın (cad.sources.sync write).",
                link.layer
            ),
            "edit.id",
        ));
    }
    let mut next = link.clone();
    next.extent = extent;
    let input = SourcesSync {
        layer: link.layer.clone(),
        direction: SyncDirection::Read,
        discard: false,
        topology: None,
    };
    let label = format!("Alan sınırı: {}", link.table);
    let (synced, warnings) = doc.transact(&label, |doc| {
        set_link(doc, Some(next), None);
        sync_run(doc, &input, true)
    })?;
    Ok((
        SourcesEdited {
            link: link_of(doc, id).cloned(),
            objects: synced.added,
            layers: Vec::new(),
            pins: synced.pins,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
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

/// The rows outside a part's area that follow a write (docs/adr/0531): their edits, and how many kept their shape.
#[derive(Default)]
struct Followed {
    rows: Vec<(i64, gpkg::NewRow)>,
    left: u32,
}

/// The table's rows outside the part's area (none in the drawing) sharing the corners and edges the layer's written
/// objects changed: read again by the changes' box from the same file state, put right by the topology core
/// (`topology_edit::changes`, `apply`: the same rule as the drawing's Topoloji), back in the table's system as rows.
/// A strict write refuses when one would be left invalid; a loose one keeps that one as it is.
#[allow(clippy::too_many_arguments)]
fn follow_outside(
    doc: &Document,
    link: &SourceLink,
    source: Origin<'_>,
    area: Option<[f64; 4]>,
    stored: &Stored,
    columns: &[gpkg::Column],
    mover: Option<&Mover>,
    profile: SyncTopology,
) -> Result<Followed, Stop> {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::ops::topology_edit::{self, Neighbour, SAME};
    // A whole table is all in the drawing: nothing lies outside it.
    let (Origin::File(path), Some(_)) = (source, area) else {
        return Ok(Followed::default());
    };
    if stored.outside.is_empty() {
        return Ok(Followed::default());
    }
    let shares = |s: &Shape| {
        matches!(
            s,
            Shape::Line { .. } | Shape::Polyline { .. } | Shape::Polygon { .. }
        )
    };
    let before: HashMap<Uuid, kentos_contracts::Entity> =
        stored.in_project(mover).into_iter().collect();
    let mut changes = Vec::new();
    for (uid, e) in doc.by_layer_with_uids(&link.layer) {
        let Some(old) = before.get(&uid) else {
            continue;
        };
        let (was, now) = (crate::geometry::shape(old), crate::geometry::shape(e));
        if shares(&was) && shares(&now) {
            changes.extend(topology_edit::changes(&was, &now));
        }
    }
    if changes.is_empty() {
        return Ok(Followed::default());
    }
    let anchors: Vec<_> = changes.iter().map(crate::topology_follow::anchor).collect();
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in &anchors {
        b = [
            b[0].min(p.x - SAME),
            b[1].min(p.y - SAME),
            b[2].max(p.x + SAME),
            b[3].max(p.y + SAME),
        ];
    }
    let near_box = table_area(Some(b), mover).ok().flatten().ok_or_else(|| {
        refuse(
            codes::INVALID_EXTENT,
            format!(
                "“{}” katmanının değişen köşeleri tablonun sistemine çevrilemiyor.",
                link.layer
            ),
            "layer",
        )
    })?;
    let near = load_or_refuse(path, &link.table, "layer", Some(near_box))?;
    if near.sha256 != stored.sha256 {
        return Err(conflict(
            codes::SOURCE_CHANGED,
            format!(
                "{} okunurken değişti; üzerine yazılmadı. Yeniden deneyin.",
                link.path
            ),
            "layer",
        ));
    }
    let in_drawing: HashSet<Uuid> = doc
        .by_layer_with_uids(&link.layer)
        .map(|(u, _)| u)
        .collect();
    let candidates: Vec<(Uuid, kentos_contracts::Entity)> = near
        .in_project(mover)
        .into_iter()
        .filter(|(u, e)| {
            stored.outside.contains_key(u)
                && !in_drawing.contains(u)
                && shares(&crate::geometry::shape(e))
        })
        .collect();
    let list: Vec<Neighbour> = candidates
        .iter()
        .map(|(_, e)| Neighbour {
            shape: crate::geometry::shape(e),
            locked: false,
        })
        .collect();
    let answer = topology_edit::apply(&list, &changes, false);
    if answer.invalid > 0 && profile == SyncTopology::Strict {
        return Err(refuse(
            codes::OUTSIDE_NEIGHBOUR_INVALID,
            format!(
                "Alanın dışındaki {} komşu kayıt ortak köşeyi izleyince geçersiz kalırdı; hiçbir şey yazılmadı. Alanı genişletip komşuları da çalışma kopyasına alın ya da gevşek kipte (loose) yazın.",
                answer.invalid
            ),
            "topology",
        ));
    }
    let geometry = stored.layer.geometry.as_ref().map(|g| g.type_name.as_str());
    let mut rows = Vec::new();
    for (i, shape) in answer.edited {
        let Some((uid, e)) = candidates.get(i) else {
            continue;
        };
        let Some(mut next) =
            crate::geometry::with_shape(e, crate::topology_follow::straight(shape))
        else {
            continue;
        };
        if let Some(m) = mover
            && kentos_formats::reproject::move_entity(&mut next, &m.project, &m.source, &m.choices)
                .is_err()
        {
            continue;
        }
        if let Ok(Some(row)) = gp::row_for(&next, columns, geometry) {
            rows.push((stored.outside[uid], row));
        }
    }
    rows.sort_by_key(|(fid, _)| *fid);
    Ok(Followed {
        rows,
        left: answer.invalid as u32,
    })
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
    let address = link.path.clone();
    let source = origin_from(link.kind, &address);
    if let Origin::File(path) = source
        && !path.is_file()
    {
        return Err(refuse(
            codes::SOURCE_GONE,
            format!(
                "{} yerinde yok; dosyanın yeni yerini cad.sources.edit relink ile verin.",
                link.path
            ),
            "layer",
        ));
    }
    let mover = mover_for(doc, link.srid).map_err(|why| {
        refuse(
            codes::CRS_MISMATCH,
            format!("“{}” eşitlenemez: {why}.", link.layer),
            "layer",
        )
    })?;
    let area = table_area(link.extent, mover.as_ref()).map_err(|()| {
        refuse(
            codes::INVALID_EXTENT,
            format!(
                "“{}” katmanının alan sınırı tablonun sistemine çevrilemiyor.",
                link.layer
            ),
            "layer",
        )
    })?;
    let mut stored = load_at(source, &link.table, at_one("layer"), area)?;
    stored.reachable(mover.as_ref());
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
            let d = diff(doc, &link.layer, &stored, mover.as_ref());
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
            let followed = match input.topology {
                None => Followed::default(),
                Some(profile) => {
                    let mut columns = stored.layer.columns.clone();
                    columns.extend(d.added_columns.iter().cloned());
                    follow_outside(
                        doc,
                        &link,
                        source,
                        area,
                        &stored,
                        &columns,
                        mover.as_ref(),
                        profile,
                    )?
                }
            };
            if followed.left > 0 {
                warnings.push(CommandWarning {
                    code: codes::NEIGHBOURS_LEFT.into(),
                    message: format!(
                        "Alanın dışındaki {} komşu kayıt ortak köşeyi izleyince geçersiz kalacağı için olduğu gibi kaldı.",
                        followed.left
                    ),
                    path: Some("topology".into()),
                });
            }
            let mut out = SourcesSynced {
                added: d.insert.len() as u32,
                modified: d.update.len() as u32,
                removed: d.delete.len() as u32,
                columns: d.added_columns.iter().map(|c| c.name.clone()).collect(),
                source: stored.sha256.clone(),
                pins: None,
                revision: doc.revision().to_string(),
                followed: followed.rows.len() as u32,
                left: followed.left,
            };
            if !write {
                return Ok((out, warnings));
            }
            if !d.is_empty() || !d.added_columns.is_empty() || !followed.rows.is_empty() {
                let mut update = d.update;
                update.extend(followed.rows);
                update.sort_by_key(|(fid, _)| *fid);
                let edits = Edits {
                    add_columns: d.added_columns,
                    update,
                    insert: d.insert,
                    delete: d.delete,
                };
                match source {
                    Origin::File(path) => {
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
                    // In one transaction, after checking under a lock that the table is still at the revision.
                    Origin::Database(address) => {
                        let p = database_provider("layer")?;
                        let (_, revision) = p
                            .edit(address, &link.table, &link.revision, &edits)
                            .map_err(|e| provider_stop(e, address, at_one("layer")))?;
                        link.revision = revision;
                    }
                }
            }
            if let Origin::File(path) = source {
                link.revision =
                    file_sha(path).map_err(|why| refuse(codes::FILE_UNWRITABLE, why, "layer"))?;
            }
            link.digest = layer_digest(doc, &link.layer);
            out.source = link.revision.clone();
            set_link(doc, Some(link), None);
            out.revision = doc.revision().to_string();
            Ok((out, warnings))
        }
        SyncDirection::Read => {
            if input.topology.is_some() {
                return Err(refuse(
                    codes::TOPOLOGY_ON_READ,
                    "Komşuların izlenmesi (topology) yalnız yazarken olur; okumada tablonun satırları olduğu gibi gelir."
                        .to_owned(),
                    "topology",
                ));
            }
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
            let objects = stored.in_project(mover.as_ref());
            let wanted: HashSet<Uuid> = objects.iter().map(|(u, _)| *u).collect();
            // The label pins of the layer's objects, by the key when the layer has one (docs/adr/0533 §1).
            let pins = follow_pins(doc, &link, &objects, &stored.outside);
            let mut updates: Vec<(Slot, kentos_contracts::Entity)> = Vec::new();
            let mut adds = Vec::new();
            for (uid, mut e) in objects {
                match doc
                    .slot_of(uid)
                    .and_then(|s| doc.get(s).map(|old| (s, old)))
                {
                    Some((slot, old)) => {
                        let changed = gp::essence(old) != gp::essence(&e)
                            || old.base().layer_id != link.layer;
                        let pinned = pins.set.get(&uid);
                        if (changed || pinned.is_some())
                            && let Some(mut next) =
                                if changed { gp::with_essence(old, &e) } else { Some(old.clone()) }
                        {
                            next.base_mut().layer_id = link.layer.clone();
                            if let Some(p) = pinned {
                                next.base_mut().label_pins = p.clone();
                            }
                            updates.push((slot, next));
                        }
                    }
                    None => {
                        e.base_mut().layer_id = link.layer.clone();
                        if let Some(p) = pins.set.get(&uid) {
                            e.base_mut().label_pins = p.clone();
                        }
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
                followed: 0,
                left: 0,
                added: adds.len() as u32,
                modified: updates.len() as u32,
                removed: gone.len() as u32,
                columns: Vec::new(),
                source: stored.sha256.clone(),
                pins: pins.report.clone(),
                revision: doc.revision().to_string(),
            };
            if !write {
                return Ok((out, pins.warnings));
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
                pins.warnings,
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

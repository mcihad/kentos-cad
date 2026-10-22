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

use kentos_commands::{
    CommandResult, CommandWarning, ImportGpkg, ImportedLayer, LabelPin, LayerNodeType, LostPin,
    MAX_PARKED, MAX_SOURCES, ParkedPin, PinLoss, PinMatch, PinMove, SourceCapabilities,
    SourceColumn, SourceKind, SourceLink, SourceNext, SourceState, SourceTable, SourcesDescribe,
    SourcesDescribed, SourcesEdit, SourcesEditOp, SourcesEdited, SourcesSync, SourcesSynced,
    SyncDirection, SyncTopology, SyncedPins,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{entity_bounds, entity_vertices};
use kentos_geometry_core::geometry::is_empty_bounds;
use kentos_geometry_core::ops::pin_match::{self, Found};
use kentos_geopackage::{self as gpkg, DataType, GeoPackage, Geometry, Query, Value};
use serde_json::json;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geopackage as gp;
use crate::provider;

mod base;
mod database;
mod merge;
pub mod reconcile;
pub mod relate;
mod relations;
mod together;

/// Coordinates of an object and its row agree within this (m): the geometry kernel's tolerance.
const SAME: f64 = 1e-6;
/// The same in degrees, for a table in a geographic system (about a millimetre).
const SAME_DEGREES: f64 = 1e-11;

/// How a bound layer's objects move between the project's system and its table's (docs/adr/0504 §1).
pub(crate) struct Mover {
    pub(crate) project: kentos_geometry_core::crs::System,
    pub(crate) source: kentos_geometry_core::crs::System,
    pub(crate) choices: Vec<kentos_geometry_core::crs::Choice>,
}

/// The move between the project and a table of `srid`: none in the same system or for a table that declares none;
/// why not, when one of the systems has no transform.
fn mover_for(doc: &Document, srid: Option<u32>) -> Result<Option<Mover>, String> {
    mover_in(doc.settings(), srid)
}

/// [`mover_for`] from the project's settings alone.
pub(crate) fn mover_in(
    settings: &kentos_commands::ProjectSettings,
    srid: Option<u32>,
) -> Result<Option<Mover>, String> {
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

/// Why a derivative's table is not bound, read into a layer or written back (docs/adr/0530 §4).
fn derived_message(table: &str, rec: &kentos_commands::DerivationRecord) -> String {
    format!(
        "“{table}” tablosu çizimden türetilmiş bir GIS verisi ({}, tolerans {} m; kaynak “{}”, {} sürümü): geometrisi kaynak yayların kirişleridir ve kaynağın yerine geçmez. Bağlanmaz, katmana okunmaz, geri yazılmaz; GIS'te yapılan değişiklikleri cad.gis.derivation gösterir, kaynağı çizimde düzenleyin.",
        rec.algorithm, rec.tolerance, rec.source.name, rec.source.revision
    )
}

pub(crate) fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn conflict(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::conflict(error(code, message.into(), Some(path.into())))
}

/// A conflict about some objects: their lasting ids with it (docs/adr/0582 §2).
fn conflict_on(code: &str, message: impl Into<String>, path: &str, uids: &[Uuid]) -> Stop {
    let mut e = error(code, message.into(), Some(path.into()));
    e.uids = uids.iter().map(Uuid::to_string).collect();
    Stop::conflict(e)
}

fn completed<T>(r: Result<(T, Vec<CommandWarning>), Stop>) -> CommandResult<T> {
    match r {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// How many objects are hashed per round, and the fewest that are worth another thread (docs/adr/0551 §5).
const DIGEST_ROUND: usize = 1 << 15;
const DIGEST_SPLIT: usize = 1 << 11;

/// The bytes of `items` in the digest's list, comma first from the second object of the layer: `first` is the first
/// item's place in the layer.
fn digest_bytes(items: &[(Uuid, &kentos_commands::Entity)], first: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(items.len() * 256);
    for (i, (u, e)) in items.iter().enumerate() {
        if first + i > 0 {
            out.push(b',');
        }
        let item = json!([u.to_string(), gp::essence(e)]);
        out.extend_from_slice(kentos_commands::canonical::canonical_json(&item).as_bytes());
    }
    out
}

/// The layer's digest: the SHA-256 of its objects' kinds, geometries and attributes by lasting id (not their look):
/// the canonical JSON of `[[uid, essence], …]` in the ids' order (plan/gis.md CBS-42: a million objects' JSON is not
/// built at once; the bytes, and so the digest, are those of `digest_of` on the list). The objects' bytes are made
/// 32 768 at a time, a round across the machine's threads (the bytes in the ids' order, so the digest is the same
/// as one thread's), and hashed as they come: a batch's row that writes a layer takes the digest of the new state
/// before the next row (docs/adr/0551 §5).
pub fn layer_digest(doc: &Document, layer: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut items: Vec<(Uuid, &kentos_commands::Entity)> = doc.by_layer_with_uids(layer).collect();
    items.sort_by_key(|(u, _)| *u);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get().min(8));
    let mut hasher = Sha256::new();
    hasher.update(b"[");
    for (round, window) in items.chunks(DIGEST_ROUND).enumerate() {
        let first = round * DIGEST_ROUND;
        if threads < 2 || window.len() < DIGEST_SPLIT {
            hasher.update(digest_bytes(window, first));
            continue;
        }
        let piece = window.len().div_ceil(threads);
        let parts: Vec<Vec<u8>> = std::thread::scope(|scope| {
            let handles: Vec<_> = window
                .chunks(piece)
                .enumerate()
                .map(|(k, part)| scope.spawn(move || digest_bytes(part, first + k * piece)))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap_or_default())
                .collect()
        });
        for part in parts {
            hasher.update(part);
        }
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
        .and_then(|f| kentos_commands::encoding::sha256_reader_hex(std::io::BufReader::new(f)))
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
    objects: Vec<(Uuid, kentos_commands::Entity)>,
    /// A part's working copy (docs/adr/0510): the table's rows outside the area, by lasting id. An object of the layer
    /// moved out of the area is written over its row, not as a new one.
    outside: HashMap<Uuid, i64>,
    /// The columns as the layer's fields, and what does not come over (linking onto a new layer).
    fields: Vec<kentos_commands::LayerField>,
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
    fn in_project(&self, mover: Option<&Mover>) -> Vec<(Uuid, kentos_commands::Entity)> {
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
pub(crate) enum Origin<'a> {
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
pub(crate) fn origin<'a>(address: &'a str, at: &str) -> Result<Origin<'a>, Stop> {
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

pub(crate) fn database_provider(at: &str) -> Result<&'static dyn provider::TableProvider, Stop> {
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
pub(crate) struct At<'a> {
    pub(crate) address: &'a str,
    pub(crate) table: &'a str,
}

pub(crate) const fn at_one(field: &str) -> At<'_> {
    At {
        address: field,
        table: field,
    }
}

pub(crate) fn provider_stop(e: provider::ProviderError, address: &str, at: At<'_>) -> Stop {
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
        provider::ProviderError::RowsChanged(keys) => conflict(
            codes::SOURCE_CHANGED,
            format!(
                "{address} tablosunun {} satırını katman onlarla son eşleştiğinden beri başka biri değiştirdi; hiçbir şey yazılmadı.",
                keys.len()
            ),
            at.table,
        ),
        provider::ProviderError::NoKey(name) => refuse(
            codes::NO_ROW_KEY,
            format!(
                "“{name}” view ya da sorgusunda tam sayı bir anahtar sütunu (fid, id, gid ya da ogc_fid) yok; sorguya ekleyin."
            ),
            at.table,
        ),
        provider::ProviderError::Unreadable(why) => refuse(
            codes::IMPORT_UNREADABLE,
            format!("{address} veritabanının sorgusu okunamadı: {why}"),
            at.table,
        ),
        provider::ProviderError::KeyRepeats(name, key) => refuse(
            codes::ROW_KEY_NOT_UNIQUE,
            format!(
                "“{name}” view ya da sorgusunda {key} anahtarı birden çok satırda; anahtar her satırda farklı olmalı."
            ),
            at.table,
        ),
    }
}

/// A database's name in its address, for the lasting ids of its rows.
pub(crate) fn database_name(address: &str) -> &str {
    let rest = address
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or_default();
    let tail = rest.split_once('/').map(|(_, t)| t).unwrap_or_default();
    tail.split_once('?').map_or(tail, |(d, _)| d)
}

fn load_database(
    address: &str,
    table: &str,
    at: At<'_>,
    area: Option<[f64; 4]>,
) -> Result<Stored, Stop> {
    let p = database_provider(at.address)?;
    let r = p
        .read(address, table, area)
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

/// A database table's area in its own system (docs/adr/0582 §1), its shape asked first: a table without geometry has
/// no area, a system the project cannot reach none either.
fn database_area(
    doc: &Document,
    address: &str,
    table: &str,
    extent: Option<[f64; 4]>,
) -> Result<Option<[f64; 4]>, Stop> {
    let at = At {
        address: "edit.path",
        table: "edit.table",
    };
    let p = database_provider(at.address)?;
    let shape = p
        .describe(address, table)
        .map_err(|e| provider_stop(e, address, at))?;
    if shape.layer.geometry.is_none() {
        return Err(refuse(
            codes::INVALID_EXTENT,
            format!("“{table}” tablosunun geometrisi yok; alan sınırı geometrili tabloda olur."),
            "edit.extent",
        ));
    }
    let mover = mover_for(doc, shape.srid).map_err(|why| {
        refuse(
            codes::CRS_MISMATCH,
            format!("“{table}” bağlanamaz: {why}."),
            "edit.reproject",
        )
    })?;
    table_area(extent, mover.as_ref()).map_err(|()| {
        refuse(
            codes::INVALID_EXTENT,
            format!("Alan sınırı “{table}” tablosunun sistemine çevrilemiyor."),
            "edit.extent",
        )
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
        Origin::Database(address) => load_database(address, table, at, area),
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
pub(crate) fn table_area(
    area: Option<[f64; 4]>,
    mover: Option<&Mover>,
) -> Result<Option<[f64; 4]>, ()> {
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

pub(crate) fn same_geometry(a: Option<&Geometry>, b: Option<&Geometry>, tol: f64) -> bool {
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

/// Keeps of `d` only what the layer changed since its base (docs/adr/0582 §2): a row someone else changed and the
/// layer did not is left as the table holds it, a row the layer did not change is not written back. Whether the table
/// holds changes not the layer's; a refusal when an object the layer changed is a row someone else deleted.
fn mine_only(
    doc: &Document,
    layer: &str,
    stored: &Stored,
    base: &HashMap<Uuid, base::BaseObj>,
    d: &mut Diff,
) -> Result<bool, Stop> {
    let fid_uid: HashMap<i64, Uuid> = stored.rows.iter().map(|(u, (f, ..))| (*f, *u)).collect();
    let now: HashMap<Uuid, &kentos_commands::Entity> = doc.by_layer_with_uids(layer).collect();
    let mine = |u: &Uuid| match (now.get(u), base.get(u)) {
        (Some(e), Some(b)) => base::hash_of(e) != b.layer,
        (None, Some(_)) | (Some(_), None) => true,
        (None, None) => false,
    };
    let mut others = false;
    d.update.retain(|(fid, _)| {
        let keep = fid_uid.get(fid).is_some_and(&mine);
        others |= !keep;
        keep
    });
    d.delete.retain(|fid| {
        let keep = fid_uid
            .get(fid)
            .is_some_and(|u| base.contains_key(u) && mine(u));
        others |= !keep;
        keep
    });
    let mut gone = Vec::new();
    d.insert.retain(|(uid, _)| {
        let Ok(u) = Uuid::parse_str(uid) else {
            return true;
        };
        match base.get(&u) {
            None => true,
            Some(_) if mine(&u) => {
                gone.push(u);
                false
            }
            Some(_) => {
                others = true;
                false
            }
        }
    });
    if !gone.is_empty() {
        return Err(conflict_on(
            codes::SOURCE_CHANGED,
            format!(
                "Katmanda değiştirilen {} nesnenin satırını başka biri tablodan sildi; hiçbir şey yazılmadı. Tablodan okuyun (direction read) ya da o nesnelerdeki değişikliği bırakın.",
                gone.len()
            ),
            "layer",
            &gone,
        ));
    }
    Ok(others)
}

/// The keys of `keys` whose rows are not as the base's table side holds them: changed, or gone (docs/adr/0582 §2).
fn rows_changed(
    base: Option<&HashMap<Uuid, base::BaseObj>>,
    layer: &gpkg::Layer,
    rows: &[gpkg::Row],
    keys: &[i64],
    fid_uid: &HashMap<i64, Uuid>,
    mover: Option<&Mover>,
) -> Vec<i64> {
    let Some(base) = base else {
        return Vec::new();
    };
    let found: HashMap<i64, &gpkg::Row> = rows.iter().map(|r| (r.fid, r)).collect();
    let mut out = Vec::new();
    for key in keys {
        let Some(uid) = fid_uid.get(key) else {
            continue;
        };
        let same = found.get(key).is_some_and(|row| {
            let Ok(t) =
                gp::table_of_rows(layer, std::slice::from_ref(*row), &HashMap::new(), &|_| {
                    *uid
                })
            else {
                return false;
            };
            t.objects.into_iter().next().is_some_and(|(_, mut e)| {
                if let Some(m) = mover
                    && kentos_formats::reproject::move_entity(
                        &mut e, &m.source, &m.project, &m.choices,
                    )
                    .is_err()
                {
                    return false;
                }
                base.get(uid).is_some_and(|b| base::hash_of(&e) == b.table)
            })
        });
        if !same {
            out.push(*key);
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn diff(doc: &Document, layer: &str, s: &Stored, mover: Option<&Mover>) -> Diff {
    diff_skipping(doc, layer, s, mover, &[])
}

/// How the layer differs from its table, leaving the fields in `skip` out: the many-to-many references a mapping table
/// holds (docs/adr/0532 §2), which are neither columns to add nor cells to compare nor attributes lost. A column of the
/// table by the name keeps what it holds.
fn diff_skipping(
    doc: &Document,
    layer: &str,
    s: &Stored,
    mover: Option<&Mover>,
    skip: &[String],
) -> Diff {
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
    let skipped = |name: &str| skip.iter().any(|k| k.eq_ignore_ascii_case(name));
    let mut columns = s.layer.columns.clone();
    for f in &fields {
        if !taken(&f.name, &columns) && !skipped(&f.name) {
            let c = gp::column_of(f);
            columns.push(c.clone());
            d.added_columns.push(c);
        }
    }
    let kept_cells: Vec<usize> = columns
        .iter()
        .enumerate()
        .filter(|(_, c)| skipped(&c.name))
        .map(|(i, _)| i)
        .collect();
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
            if !columns.iter().any(|c| c.name.eq_ignore_ascii_case(k)) && !skipped(k) {
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
        let mut row = match gp::row_for(e, &columns, geometry) {
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
        // Z and M as the column takes them (docs/adr/0584 §2): the object is still a change the layer has, the write
        // refuses it before anything is written.
        if d.mismatch.is_none()
            && let (Some(g), Some(col)) = (row.geometry.as_ref(), s.layer.geometry.as_ref())
            && let Some(why) = gp::dims_problem(g, col.z, col.m)
        {
            d.mismatch = Some((uid, why));
        }
        // A mapped reference's column keeps what the table holds (nothing, in a new row).
        for &i in &kept_cells {
            if let Some(cell) = row.values.get_mut(i) {
                *cell = match s.rows.get(&uid) {
                    Some((_, _, values)) if i < width => values[i].clone(),
                    _ => Value::Null,
                };
            }
        }
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
    objects: impl Iterator<Item = (Uuid, &'a kentos_commands::Entity)>,
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

/// What a read does to the label pins of a bound layer's objects (docs/adr/0533 §1, 0578): the objects holding pins
/// as they are in the drawing, against the table's rows as read (`objects`). An object whose row is there stays as it
/// is. With a key, a row id that is another record now (the key's value differs) or whose row is gone leaves its
/// pins to the row of the old value when the table has exactly one; without a key, a row gone is looked for by its
/// geometry and then the nearest feature of the same kind (`pin_match`, §2). An object whose row is in the table but
/// outside the part's area is parked, not lost: its pins stay in the link (§3), and go back to the object when the
/// row comes into the area again. What cannot be kept is lost, and said.
struct PinPlan {
    /// The pins an object holds after the read, where they are not its own: set on a row they followed to or were
    /// given back to, emptied on one that is another record now.
    set: HashMap<Uuid, Vec<LabelPin>>,
    /// The link's parked pins after the read.
    parked: Vec<ParkedPin>,
    report: Option<SyncedPins>,
    warnings: Vec<CommandWarning>,
}

/// A feature as the pin matching sees it; none for an object with no geometry (a record), which no pin follows.
fn match_feature(e: &kentos_commands::Entity) -> Option<pin_match::Feature> {
    let (_, _, _, shape) = crate::geometry_store::record(e);
    let b = entity_bounds(&shape);
    if is_empty_bounds(&b) {
        return None;
    }
    let mut bits: Vec<u64> = entity_vertices(&shape)
        .iter()
        .flat_map(|v| [v.x.to_bits(), v.y.to_bits()])
        .collect();
    bits.extend([b.min_x, b.min_y, b.max_x, b.max_y].map(f64::to_bits));
    Some(pin_match::Feature {
        kind: e.kind().to_owned(),
        bits,
        middle: [(b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0],
    })
}

/// One pin move, as `follow_pins` collects them.
struct Move {
    from: Uuid,
    to: Uuid,
    key: Option<String>,
    by: PinMatch,
    distance: Option<f64>,
}

fn follow_pins(
    doc: &Document,
    link: &SourceLink,
    objects: &[(Uuid, kentos_commands::Entity)],
    outside: &HashMap<Uuid, i64>,
) -> PinPlan {
    let bearers: Vec<(Uuid, &kentos_commands::Entity)> = doc
        .by_layer_with_uids(&link.layer)
        .filter(|(_, e)| !e.base().label_pins.is_empty())
        .collect();
    let mut plan = PinPlan {
        set: HashMap::new(),
        parked: link.parked.clone(),
        report: None,
        warnings: Vec::new(),
    };
    if bearers.is_empty() && link.parked.is_empty() {
        return plan;
    }
    let key = link.key.as_deref();
    let new: HashMap<Uuid, &kentos_commands::Entity> =
        objects.iter().map(|(u, e)| (*u, e)).collect();
    let value_of = |e: &kentos_commands::Entity| -> Option<String> {
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
    let mut moves: Vec<Move> = Vec::new();
    let mut lost: Vec<(Uuid, PinLoss)> = Vec::new();
    // The objects whose row is outside the area now, and (without a key) whose row is gone.
    let mut parks: Vec<(Uuid, Option<String>)> = Vec::new();
    let mut orphans: Vec<Uuid> = Vec::new();
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
            Some((to, v)) => moves.push(Move {
                from: *uid,
                to,
                key: Some(v),
                by: PinMatch::Key,
                distance: None,
            }),
            None if here.is_some() => lost.push((*uid, PinLoss::Rebound)),
            None if outside.contains_key(uid) => parks.push((*uid, old_value)),
            None if key.is_none() => orphans.push(*uid),
            None => lost.push((*uid, PinLoss::Deleted)),
        }
    }
    // Without a key, the pins of an object whose row is gone follow its geometry, or the nearest feature of its kind
    // (docs/adr/0578 §2), to a row the layer has no object for.
    if !orphans.is_empty() {
        let present: HashSet<Uuid> = doc
            .by_layer_with_uids(&link.layer)
            .map(|(u, _)| u)
            .collect();
        let fresh: Vec<(Uuid, pin_match::Feature)> = objects
            .iter()
            .filter(|(u, _)| !present.contains(u))
            .filter_map(|(u, e)| match_feature(e).map(|f| (*u, f)))
            .collect();
        let olds: Vec<(Uuid, pin_match::Feature)> = orphans
            .iter()
            .filter_map(|u| {
                bearers
                    .iter()
                    .find(|(b, _)| b == u)
                    .and_then(|(_, e)| match_feature(e))
                    .map(|f| (*u, f))
            })
            .collect();
        for u in &orphans {
            if !olds.iter().any(|(o, _)| o == u) {
                lost.push((*u, PinLoss::Deleted));
            }
        }
        let found = pin_match::match_pins(
            &olds.iter().map(|(_, f)| f.clone()).collect::<Vec<_>>(),
            &fresh.iter().map(|(_, f)| f.clone()).collect::<Vec<_>>(),
            pin_match::TOLERANCE,
        );
        for ((from, _), f) in olds.iter().zip(found) {
            match f {
                Found::Geometry(j) => moves.push(Move {
                    from: *from,
                    to: fresh[j].0,
                    key: None,
                    by: PinMatch::Geometry,
                    distance: None,
                }),
                Found::Nearest(j, d) => moves.push(Move {
                    from: *from,
                    to: fresh[j].0,
                    key: None,
                    by: PinMatch::Nearest,
                    distance: Some(d),
                }),
                Found::Ambiguous => lost.push((*from, PinLoss::Ambiguous)),
                Found::None => lost.push((*from, PinLoss::Deleted)),
            }
        }
    }
    let bearer_pins: HashMap<Uuid, &Vec<LabelPin>> = bearers
        .iter()
        .map(|(u, e)| (*u, &e.base().label_pins))
        .collect();
    let leaving: HashSet<Uuid> = moves
        .iter()
        .map(|m| m.from)
        .chain(parks.iter().map(|(u, _)| *u))
        .collect();
    let mut moved: Vec<PinMove> = Vec::new();
    for m in moves {
        // The row of the key keeps its own pins when it is not itself leaving.
        let taken = bearer_pins.contains_key(&m.to) && !leaving.contains(&m.to);
        if taken {
            lost.push((m.from, PinLoss::Taken));
        } else {
            plan.set.insert(m.to, bearer_pins[&m.from].clone());
            moved.push(PinMove {
                from: m.from.to_string(),
                to: m.to.to_string(),
                key: m.key,
                by: m.by,
                distance: m.distance,
            });
        }
    }
    // The pins parked before: back to a row that came into the area (by its id, else by its key's value), kept while
    // the row is still outside, lost with a row that is gone.
    let reparked: HashSet<Uuid> = parks.iter().map(|(u, _)| *u).collect();
    let mut carried: Vec<ParkedPin> = Vec::new();
    let mut restored: Vec<Uuid> = Vec::new();
    for p in &link.parked {
        let Ok(puid) = Uuid::parse_str(&p.uid) else {
            continue;
        };
        // The object came back (an undo) and left again: the new parking replaces the old.
        if reparked.contains(&puid) {
            continue;
        }
        let to = if new.contains_key(&puid) {
            Some(puid)
        } else {
            p.key
                .as_ref()
                .filter(|_| key.is_some())
                .and_then(|k| by_key.get(k))
                .filter(|l| l.len() == 1)
                .map(|l| l[0])
        };
        match to {
            Some(t) => {
                let holds = bearer_pins.get(&t).filter(|_| !leaving.contains(&t));
                if let Some(have) = holds {
                    // The object holds these very pins already (an undo brought it back): nothing to say.
                    if **have != p.pins {
                        lost.push((puid, PinLoss::Taken));
                    }
                } else if plan.set.get(&t).is_some_and(|v| !v.is_empty()) {
                    lost.push((puid, PinLoss::Taken));
                } else {
                    plan.set.insert(t, p.pins.clone());
                    restored.push(t);
                }
            }
            None if outside.contains_key(&puid) => carried.push(p.clone()),
            None => lost.push((puid, PinLoss::Deleted)),
        }
    }
    // A layer parks only so many: the rest are lost as outside.
    let room = MAX_PARKED.saturating_sub(carried.len());
    let mut parked_now: Vec<Uuid> = Vec::new();
    for (uid, value) in parks {
        if parked_now.len() >= room {
            lost.push((uid, PinLoss::Outside));
            continue;
        }
        carried.push(ParkedPin {
            uid: uid.to_string(),
            key: value,
            pins: bearer_pins[&uid].clone(),
        });
        parked_now.push(uid);
    }
    carried.sort_by(|a, b| a.uid.cmp(&b.uid));
    plan.parked = carried;
    // What an object that stays in the layer as another record holds is emptied, where nothing moved onto it.
    for uid in leaving.iter().chain(lost.iter().map(|(u, _)| u)) {
        if new.contains_key(uid) && !restored.contains(uid) {
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
    let mut parked_uids: Vec<String> = parked_now.iter().map(Uuid::to_string).collect();
    parked_uids.sort();
    let mut restored_uids: Vec<String> = restored.iter().map(Uuid::to_string).collect();
    restored_uids.sort();
    if !lost.is_empty() {
        let count = |r: PinLoss| lost.iter().filter(|l| l.reason == r).count();
        let mut parts = Vec::new();
        for (r, what) in [
            (PinLoss::Deleted, "satırı tabloda yok"),
            (
                PinLoss::Outside,
                "satırı katmanın alanının dışında (park yeri dolu)",
            ),
            (
                PinLoss::Rebound,
                "satır kimliği artık başka bir kayıt (anahtar değeri değişmiş)",
            ),
            (PinLoss::Taken, "anahtarının satırında zaten iğneler var"),
            (
                PinLoss::Ambiguous,
                "satırı belirsiz (aynı geometride ya da aynı uzaklıkta birden çok satır)",
            ),
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
    let by_key_moves = moved.iter().filter(|m| m.by == PinMatch::Key).count();
    if by_key_moves > 0 {
        plan.warnings.push(CommandWarning {
            code: codes::LABEL_PINS_MOVED.into(),
            message: format!(
                "{}: {} nesnenin etiket iğneleri “{}” anahtarının yeni satırına taşındı.",
                link.table,
                by_key_moves,
                key.unwrap_or_default()
            ),
            path: Some("layer".into()),
        });
    }
    let (geometry, nearest) = (
        moved.iter().filter(|m| m.by == PinMatch::Geometry).count(),
        moved.iter().filter(|m| m.by == PinMatch::Nearest).count(),
    );
    if geometry + nearest > 0 {
        plan.warnings.push(CommandWarning {
            code: codes::LABEL_PINS_MOVED.into(),
            message: format!(
                "{}: {} nesnenin etiket iğneleri anahtarsız eşlemeyle yeni satırlarına taşındı ({geometry} aynı geometri, {nearest} en yakın satır, en çok 0,05 m).",
                link.table,
                geometry + nearest
            ),
            path: Some("layer".into()),
        });
    }
    if !parked_uids.is_empty() {
        plan.warnings.push(CommandWarning {
            code: codes::LABEL_PINS_PARKED.into(),
            message: format!(
                "{}: {} nesnenin etiket iğneleri satırı alanın dışında kaldığı için park edildi; satır alana girince geri verilir.",
                link.table,
                parked_uids.len()
            ),
            path: Some("layer".into()),
        });
    }
    if !restored_uids.is_empty() {
        plan.warnings.push(CommandWarning {
            code: codes::LABEL_PINS_RESTORED.into(),
            message: format!(
                "{}: {} nesnenin park edilmiş etiket iğneleri geri verildi.",
                link.table,
                restored_uids.len()
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
    plan.report = Some(SyncedPins {
        kept,
        moved,
        lost,
        parked: parked_uids,
        restored: restored_uids,
    });
    plan
}

/// How wide the band round a part is, as a share of the part's shorter side (docs/adr/0578 §1).
pub const BAND: f64 = 0.15;

/// The band round a part: the area grown on every side by [`BAND`] of its shorter side (a share, with no clamp: the
/// rule has no unit).
pub fn band_of(extent: &[f64; 4]) -> [f64; 4] {
    let width = BAND * (extent[2] - extent[0]).min(extent[3] - extent[1]);
    [
        extent[0] - width,
        extent[1] - width,
        extent[2] + width,
        extent[3] + width,
    ]
}

/// What reading the band round a bound layer's part needs, taken from the drawing so the read can run on a thread of its
/// own: the binding, the project's settings (the move into its system) and the lasting ids the layer holds.
pub struct BandJob {
    link: SourceLink,
    settings: kentos_commands::ProjectSettings,
    held: HashSet<Uuid>,
}

/// The band job of a bound layer with a part in a file; none for any other layer.
pub fn band_job(doc: &Document, link: &SourceLink) -> Option<BandJob> {
    link.extent?;
    if link.kind != SourceKind::Geopackage {
        return None;
    }
    Some(BandJob {
        link: link.clone(),
        settings: doc.settings().clone(),
        held: doc
            .by_layer_with_uids(&link.layer)
            .map(|(u, _)| u)
            .collect(),
    })
}

impl BandJob {
    /// The band: the part grown by [`BAND`] of its shorter side.
    pub fn area(&self) -> [f64; 4] {
        band_of(&self.link.extent.unwrap_or_default())
    }

    /// The rows of the band that the layer holds no object for, as the same version of the table the layer matches
    /// reads them (docs/adr/0578 §1): the label engine takes them as obstacles only. Why they could not be read when
    /// the file is gone or is not the version the layer matches.
    pub fn read(&self) -> BandRows {
        let link = &self.link;
        let Origin::File(path) = origin_of(link) else {
            return Err("kaynak bir dosya değil".into());
        };
        if !path.is_file() {
            return Err(format!("“{}” dosyası yok", path.display()));
        }
        let mover = mover_in(&self.settings, link.srid)?;
        let table_area = table_area(Some(self.area()), mover.as_ref())
            .map_err(|()| "bant tablonun sistemine çevrilemedi".to_owned())?;
        let stored = load(path, &link.table, table_area)?;
        if stored.sha256 != link.revision {
            return Err("dosya katmanın son eşleştiği sürümden farklı".into());
        }
        Ok(stored
            .in_project(mover.as_ref())
            .into_iter()
            .filter(|(u, _)| !self.held.contains(u))
            .map(|(_, e)| e)
            .collect())
    }
}

/// The rows a band read gave, or why it could not.
pub type BandRows = Result<Vec<kentos_commands::Entity>, String>;

/// [`band_job`] and its read at once: the band, and the rows or why none could be read; none for a layer without a
/// part.
pub fn band_objects(doc: &Document, link: &SourceLink) -> Option<([f64; 4], BandRows)> {
    let job = band_job(doc, link)?;
    Some((job.area(), job.read()))
}

fn set_link(doc: &mut Document, link: Option<SourceLink>, remove: Option<&str>) {
    let mut settings = doc.settings().clone();
    if let Some(id) = remove {
        settings.sources.retain(|s| s.id != id);
        settings.source_bases.retain(|b| b.source != id);
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
        relations: Vec::new(),
        name: l.name,
    })
}

fn describe_file(path: &Path) -> Result<Vec<SourceTable>, String> {
    let file = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let writable = !readonly(path);
    let mut out = Vec::new();
    let all = file.layers().map_err(|e| e.0)?;
    for l in all.iter().cloned() {
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
        let mut table = table_of(&file, l, srid, writable)?;
        table.relations = relations::listed(&file, &all, &table.name);
        out.push(table);
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
            relations: Vec::new(),
        })
        .collect())
}

fn state_of(doc: &Document, link: &SourceLink, ties: &[relations::Tie]) -> SourceState {
    let edited = layer_digest(doc, &link.layer) != link.digest;
    let mut state = SourceState {
        link: link.clone(),
        found: false,
        changed: false,
        edited,
        added: 0,
        modified: 0,
        removed: 0,
        objects: count(doc.by_layer_with_uids(&link.layer).count()),
        rows: None,
        total: None,
        next: SourceNext::Relink,
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
            let s = load_database(address, &link.table, at_one(""), area).ok();
            state.found = s.is_some();
            s
        }
    };
    if let (Some(mut s), Ok(mover)) = (loaded, mover_for(doc, link.srid)) {
        state.changed = s.sha256 != link.revision;
        state.total = Some(count(s.rows.len() + s.outside.len()));
        s.reachable(mover.as_ref());
        state.rows = Some(count(s.rows.len()));
        let skip: Vec<String> = ties
            .iter()
            .filter(|t| matches!(t.kept, relations::Kept::Mapped(_)) && t.child_layer == link.layer)
            .map(|t| t.field.clone())
            .collect();
        let d = diff_skipping(doc, &link.layer, &s, mover.as_ref(), &skip);
        state.added = d.insert.len() as u32;
        state.modified = d.update.len() as u32;
        state.removed = d.delete.len() as u32;
    }
    state.next = SourceNext::of(
        state.found,
        state.rows.is_some(),
        state.changed,
        state.edited,
        state.added + state.modified + state.removed,
    );
    state
}

/// The state of one bound layer, as `cad.sources.describe` gives it (the same function, so the rule that says what
/// waits is not written twice): the Öznitelik tablosu's source line reads it (docs/adr/0546 §3). It reads the table,
/// so a caller keeps the answer and asks again only when something it depends on changed. None: the layer is not
/// bound.
pub fn layer_state(doc: &Document, layer: &str) -> Option<SourceState> {
    let link = doc.settings().sources.iter().find(|l| l.layer == layer)?;
    Some(state_of(doc, link, &relations::ties(doc)))
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
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
        None => {
            let ties = relations::ties(doc);
            Ok((
                SourcesDescribed {
                    tables: Vec::new(),
                    links: doc
                        .settings()
                        .sources
                        .iter()
                        .map(|l| state_of(doc, l, &ties))
                        .collect(),
                    revision,
                },
                Vec::new(),
            ))
        }
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
        && let Some(why) = kentos_commands::key_problem(k)
    {
        return Err(refuse(
            codes::INVALID_KEY,
            format!("Anahtar alanı {why}."),
            "edit.key",
        ));
    }
    if let Some(e) = &extent {
        let why = kentos_commands::extent_problem(e).map(|w| format!("Alan sınırının {w}."));
        let why = why.or_else(|| {
            layer.map(|_| {
                "Var olan katmana bağlamada alan sınırı olmaz: katman tablonun bütün nesnelerini taşır. Yeni katmana bağlayın (layer vermeden).".to_owned()
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
    // A derivative of a drawing is never bound back (docs/adr/0530 §4): its chords would stand for the drawing's arcs.
    if let Origin::File(p) = at
        && let Some(rec) = crate::gis_derive::derived_table(p, table)
    {
        return Err(refuse(
            codes::DERIVED_DATASET,
            derived_message(table, &rec),
            "edit.table",
        ));
    }
    // A database table bound by an area reads only the area's rows (docs/adr/0582 §1): its shape first, for the
    // area in its system.
    let db_area = match at {
        Origin::Database(address) if extent.is_some() => {
            database_area(doc, address, table, extent)?
        }
        _ => None,
    };
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
            db_area,
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
        parked: Vec::new(),
        writable,
    };
    set_link(doc, Some(link.clone()), None);
    // The layer holds the table's rows as a reload reads them (docs/adr/0538 §1).
    base::take(doc, &link, None);
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
        source_srid: stored.srid,
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
                if let Some(why) = kentos_commands::key_problem(k) {
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
        let why = kentos_commands::extent_problem(e).map(|w| format!("Alan sınırının {w}."));
        if let Some(why) = why {
            return Err(refuse(codes::INVALID_EXTENT, why, "edit.extent"));
        }
        if let Origin::Database(address) = origin_of(&link) {
            database_area(doc, address, &link.table, extent)?;
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
        with: Vec::new(),
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
    // One step of the drawing and its own row in the journal (docs/adr/0253 §3, 0548 §4).
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SOURCES_EDIT,
        kentos_commands::CAD_SOURCES_EDIT_VERSION,
        input,
        written_edit_execute,
    )
}

/// [`edit_execute`]'s write; `call::recorded` runs it as one step and journals it.
fn written_edit_execute(
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
    let before: HashMap<Uuid, kentos_commands::Entity> =
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
    let candidates: Vec<(Uuid, kentos_commands::Entity)> = near
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
    if !input.with.is_empty() && input.direction == SyncDirection::Read {
        return Err(refuse(
            codes::INVALID_INPUT,
            "with yalnız yazma (direction write) içindir: okuma her katmanı ayrı alır.",
            "with",
        ));
    }
    let primary = doc
        .settings()
        .sources
        .iter()
        .find(|s| s.layer == input.layer)
        .map(|s| s.kind);
    // A write is one transaction, one layer or several: a GeoPackage's through `together` (docs/adr/0532 §1), a
    // PostGIS database's through `database` (docs/adr/0584 §1).
    if input.direction == SyncDirection::Write {
        let mut ids = vec![input.layer.clone()];
        ids.extend(input.with.iter().cloned());
        return if primary == Some(SourceKind::Postgis) {
            database::write(doc, &ids, write, input.topology)
        } else {
            together::write(doc, &ids, write, input.topology)
        };
    }
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
    if let Origin::File(path) = source
        && let Some(rec) = crate::gis_derive::derived_table(path, &link.table)
    {
        return Err(refuse(
            codes::DERIVED_DATASET,
            derived_message(&link.table, &rec),
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
        // Written above.
        SyncDirection::Write => database::write(doc, &[link.layer], write, input.topology),
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
            // A mapped many-to-many reference is read from its mapping table, not from a column (docs/adr/0532 §2).
            together::patch_read(doc, &link, &mut stored)?;
            let objects = stored.in_project(mover.as_ref());
            let wanted: HashSet<Uuid> = objects.iter().map(|(u, _)| *u).collect();
            // The label pins of the layer's objects, by the key when the layer has one (docs/adr/0533 §1).
            let pins = follow_pins(doc, &link, &objects, &stored.outside);
            let mut updates: Vec<(Slot, kentos_commands::Entity)> = Vec::new();
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
                            && let Some(mut next) = if changed {
                                gp::with_essence(old, &e)
                            } else {
                                Some(old.clone())
                            }
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
                layers: Vec::new(),
                relations: Vec::new(),
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
            // The pins of rows outside the part wait in the binding (docs/adr/0578 §3).
            link.parked = pins.parked.clone();
            set_link(doc, Some(link.clone()), None);
            base::take(doc, &link, None);
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

/// Writes the bound layers among `layers` to their sources in one transaction (docs/adr/0532 §1), as
/// `cad.sources.sync` with `with` does, `write` false saying only what it would do. `source_not_found` when none of
/// them is bound; layers that are not bound are left out.
pub(crate) fn commit(
    doc: &mut Document,
    layers: &[String],
    write: bool,
) -> Result<(SourcesSynced, Vec<CommandWarning>), Stop> {
    let bound: Vec<String> = layers
        .iter()
        .filter(|l| doc.settings().sources.iter().any(|s| s.layer == **l))
        .cloned()
        .collect();
    let Some(first) = bound.first() else {
        return Err(refuse(
            codes::SOURCE_NOT_FOUND,
            "Kaynağa yazma istendi ama kaydın ve alt kayıtlarının katmanlarından hiçbiri bir kaynağa bağlı değil; önce cad.sources.edit ile bağlayın.",
            "commit",
        ));
    };
    let input = SourcesSync {
        layer: first.clone(),
        direction: SyncDirection::Write,
        discard: false,
        with: bound[1..].to_vec(),
        topology: None,
    };
    sync_run(doc, &input, write)
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
    // One step of the drawing and its own row in the journal (docs/adr/0253 §3, 0548 §4).
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SOURCES_SYNC,
        kentos_commands::CAD_SOURCES_SYNC_VERSION,
        input,
        written_sync_execute,
    )
}

/// [`sync_execute`]'s write; `call::recorded` runs it as one step and journals it.
fn written_sync_execute(
    cx: &mut ExecutionContext<'_>,
    input: SourcesSync,
) -> CommandResult<SourcesSynced> {
    // A plan through the protocol runs this on a reading copy: it writes no file then (docs/adr/0532 §4).
    let write = !(cx.dry() && input.direction == SyncDirection::Write);
    completed(sync_run(cx.doc, &input, write))
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

//! `file.import.shp`, `file.import.geojson` and `file.import.kml` v1 (docs/adr/0563 §1): a Shapefile, a GeoJSON file or
//! a KML/KMZ taken into the drawing as the desktop's windows take them (docs/adr/0046, 0053, 0222), as one undo step
//! named after the format and the file (“Shapefile: parseller.shp”), recorded in the journal under the command's own
//! name. The shared reader (`kentos_formats::shp`, `geojson`, `kml`) reads the file once; the three commands differ
//! only in how a file is found and read, the rest is here:
//!
//! - the file's coordinate system is a question with an answer in the input: what `srid` says, else what the file
//!   declares (a `.prj`, a `crs` member, RFC 7946's or KML's WGS 84), else the project's own (said, `crs_undeclared`);
//!   an unreadable statement is `crs_unknown`, a system other than the project's `crs_mismatch` unless `reproject`
//!   asks for the move; a project without a system (a local one) takes any file's coordinates as they are;
//! - the move is the one Koordinat dönüştür makes (`kentos_formats::reproject`: the projections, the datum shift, the
//!   project's datum choices), its way and its accuracy at the middle of the file's coordinates are the answer's
//!   `moved` (docs/adr/0222 §2); what it cannot reach is left out and said in the report;
//! - each source layer goes onto the layer `layers` names, else `layer`, else the drawing's layer of the same name,
//!   else a new layer with the file's fields, the same routing the DXF import uses ([`crate::file_import`]).
//!
//! The desktop's windows read the file off the UI thread and show it first; they write with `execute_read`, the
//! command with the file already read (and, when asked, already moved: [`Premoved`]), so a window and a script write
//! alike. The shared cases are `fixtures/commands/v1/file.import.{shp,geojson,kml}.json`
//! (`scripts/fixtures/gis_import_cases.py`).

use std::collections::BTreeMap;
use std::path::Path;

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    Bounds, CommandResult, CommandWarning, CrsMove, DeclaredCrs, GeoJsonReadOptions, GisImported,
    ImportGeojson, ImportKml, ImportResult, ImportShp, ImportedLayer, ReportItem,
    ShapefileReadOptions, SourceFact,
};
use kentos_domain::Document;
use kentos_geometry_core::Vec2;
use kentos_geometry_core::crs as core;
use serde::Serialize;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::file_import::{self, Routing, group_of, nothing_to_import, route_layers};
use crate::import_apply::{self, ImportPlan};

/// What a file is, to its command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Shp,
    Geojson,
    Kml,
}

impl Format {
    /// The undo step's word: “Shapefile”.
    pub fn word(self) -> &'static str {
        match self {
            Format::Shp => "Shapefile",
            Format::Geojson => "GeoJSON",
            Format::Kml => "KML",
        }
    }

    fn command(self) -> (&'static str, u32) {
        match self {
            Format::Shp => (
                kentos_commands::FILE_IMPORT_SHP,
                kentos_commands::FILE_IMPORT_SHP_VERSION,
            ),
            Format::Geojson => (
                kentos_commands::FILE_IMPORT_GEOJSON,
                kentos_commands::FILE_IMPORT_GEOJSON_VERSION,
            ),
            Format::Kml => (
                kentos_commands::FILE_IMPORT_KML,
                kentos_commands::FILE_IMPORT_KML_VERSION,
            ),
        }
    }

    fn example(self) -> &'static str {
        match self {
            Format::Shp => "/klasör/ad.shp",
            Format::Geojson => "/klasör/ad.geojson",
            Format::Kml => "/klasör/ad.kml",
        }
    }
}

/// The undo step's name: “Shapefile: ad.shp”.
pub fn step_name(format: Format, file: &str) -> String {
    format!("{}: {file}", format.word())
}

/// What the three inputs share, the input's fields without the format's own.
#[derive(Clone, Debug, Default)]
pub struct Opts {
    pub path: String,
    pub entry: Option<String>,
    pub srid: Option<u32>,
    pub reproject: bool,
    pub skip_layers: Vec<String>,
    pub layers: BTreeMap<String, String>,
    pub layer: Option<String>,
    pub group: Option<String>,
    pub expected_revision: Option<String>,
    pub merge: Option<kentos_commands::ImportMerge>,
    pub rejects: Option<String>,
}

impl From<ImportShp> for Opts {
    fn from(i: ImportShp) -> Self {
        Self {
            path: i.path,
            entry: i.entry,
            srid: i.srid,
            reproject: i.reproject,
            skip_layers: i.skip_layers,
            layers: i.layers,
            layer: i.layer,
            group: i.group,
            expected_revision: i.expected_revision,
            merge: i.merge,
            rejects: i.rejects,
        }
    }
}

impl From<ImportGeojson> for Opts {
    fn from(i: ImportGeojson) -> Self {
        Self {
            path: i.path,
            entry: None,
            srid: i.srid,
            reproject: i.reproject,
            skip_layers: i.skip_layers,
            layers: i.layers,
            layer: i.layer,
            group: i.group,
            expected_revision: i.expected_revision,
            merge: i.merge,
            rejects: i.rejects,
        }
    }
}

impl From<ImportKml> for Opts {
    fn from(i: ImportKml) -> Self {
        Self {
            path: i.path,
            entry: None,
            srid: i.srid,
            reproject: i.reproject,
            skip_layers: i.skip_layers,
            layers: i.layers,
            layer: i.layer,
            group: i.group,
            expected_revision: i.expected_revision,
            merge: i.merge,
            rejects: i.rejects,
        }
    }
}

/// A file read for an import, and what the file was. The result of a file the window moved already carries the
/// move in `premoved`.
#[derive(Clone, Debug)]
pub struct ReadGis {
    pub result: ImportResult,
    /// The file's name, its size and its SHA-256 (a Shapefile's: its `.shp`'s).
    pub file: String,
    pub bytes: u64,
    pub sha256: String,
    pub premoved: Option<Premoved>,
}

/// The move a host made off its UI thread: `result` is already in the project's system, moved from `from`; the box
/// the coordinates had before it and the objects it left out.
#[derive(Clone, Debug)]
pub struct Premoved {
    pub from: u32,
    pub bounds: Option<Bounds>,
    pub left: u32,
}

impl ReadGis {
    /// What reading `bytes` (the file `name`) gave.
    pub fn of(name: &str, bytes: &[u8], result: ImportResult) -> Self {
        Self {
            result,
            file: name.to_owned(),
            bytes: bytes.len() as u64,
            sha256: sha256_hex(bytes),
            premoved: None,
        }
    }
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn name_of(path: &Path, fallback: &str) -> String {
    path.file_name()
        .map_or_else(|| fallback.to_owned(), |n| n.to_string_lossy().into_owned())
}

fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// A file's name without its last extension: the layer a GeoJSON file's objects go onto when it names none.
fn base_name(name: &str) -> String {
    match name.rfind('.') {
        Some(dot) if dot > 0 => name[..dot].to_owned(),
        _ => name.to_owned(),
    }
}

/// The path: absolute, not a folder, and for a Shapefile a `.shp` or a `.zip`.
fn check_path(format: Format, opts: &Opts) -> Result<(), Stop> {
    let path = Path::new(&opts.path);
    if opts.path.trim().is_empty() || !path.is_absolute() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” mutlak bir dosya yolu değil; okunacak dosyanın tam yolunu verin ({}).",
                opts.path,
                format.example()
            ),
            "path",
        ));
    }
    if path.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "{} bir klasör; okunacak {} dosyasının adını da verin.",
                opts.path,
                format.word()
            ),
            "path",
        ));
    }
    if format == Format::Shp && !matches!(extension(path).as_str(), "shp" | "zip") {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "“{}” bir Shapefile'ın .shp'si ya da .zip arşivi değil; katmanın .shp dosyasının ya da arşivin yolunu verin.",
                opts.path
            ),
            "path",
        ));
    }
    Ok(())
}

/// The files beside a `.shp` that are its own, found without case: `shx`, `dbf`, `prj`, `cpg`.
fn sidecars(shp: &Path) -> Result<Vec<(&'static str, Vec<u8>)>, String> {
    const PARTS: [&str; 4] = ["shx", "dbf", "prj", "cpg"];
    let stem = shp
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let Some(folder) = shp.parent() else {
        return Ok(Vec::new());
    };
    let mut found: Vec<(&'static str, Vec<u8>)> = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(folder)
        .map_err(|e| format!("{} listelenemedi: {e}", folder.display()))?
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for e in entries {
        let name = e.file_name().to_string_lossy().to_lowercase();
        let Some((s, ext)) = name.rsplit_once('.') else {
            continue;
        };
        if s != stem {
            continue;
        }
        let Some(part) = PARTS.iter().find(|p| **p == ext) else {
            continue;
        };
        if found.iter().any(|(p, _)| p == part) {
            continue;
        }
        let bytes = std::fs::read(e.path())
            .map_err(|err| format!("{} okunamadı: {err}", e.path().display()))?;
        found.push((part, bytes));
    }
    Ok(found)
}

/// The results of a zip archive's layers as one: objects, layers and the report added up, the system the first
/// layer's, the box the union.
fn merge(mut results: Vec<ImportResult>) -> ImportResult {
    let mut one = results.remove(0);
    let first = one.declared_crs.as_ref().and_then(|d| d.srid);
    let mut names: Vec<String> = one.layers.iter().map(|l| l.name.clone()).collect();
    for r in results {
        names.extend(r.layers.iter().map(|l| l.name.clone()));
        one.entities.extend(r.entities);
        one.layers.extend(r.layers);
        for (kind, n) in r.report.counts {
            *one.report.counts.entry(kind).or_default() += n;
        }
        one.report.skipped.extend(r.report.skipped);
        one.report.notes.extend(r.report.notes);
        let union = |a: Option<Bounds>, b: Option<Bounds>| match (a, b) {
            (Some(a), Some(b)) => Some(Bounds {
                min_x: a.min_x.min(b.min_x),
                min_y: a.min_y.min(b.min_y),
                max_x: a.max_x.max(b.max_x),
                max_y: a.max_y.max(b.max_y),
            }),
            (a, b) => a.or(b),
        };
        one.bounds = union(one.bounds, r.bounds);
        one.view = union(one.view, r.view);
        if let Some(d) = &r.declared_crs
            && d.srid != first
        {
            one.report.notes.push(ReportItem {
                severity: None,
                details: Vec::new(),
                what: "Katmanların sistemi".into(),
                count: 1,
                reason: format!(
                    "bir katman başka bir sistem bildiriyor (“{}”); ilk katmanınki esas alındı",
                    d.text
                ),
                lines: Vec::new(),
            });
        }
    }
    one.report.source.push(SourceFact {
        label: "Katmanlar".into(),
        value: names.join(", "),
    });
    one
}

/// The layers of a zip archive the input reads: the one `entry` names (its path in the archive, with or without
/// `.shp`, or its name), else every one.
fn zip_selection(layers: &[String], entry: Option<&str>) -> Result<Vec<String>, Stop> {
    let Some(entry) = entry else {
        return Ok(layers.to_vec());
    };
    let wanted = entry
        .strip_suffix(".shp")
        .or_else(|| entry.strip_suffix(".SHP"))
        .unwrap_or(entry)
        .to_lowercase();
    let by_path = layers.iter().find(|l| l.to_lowercase() == wanted);
    let by_name: Vec<&String> = layers
        .iter()
        .filter(|l| kentos_formats::shp::zip_layer_name(l).to_lowercase() == wanted)
        .collect();
    match (by_path, by_name.as_slice()) {
        (Some(l), _) => Ok(vec![l.clone()]),
        (None, [l]) => Ok(vec![(*l).clone()]),
        (None, []) => Err(refuse(
            codes::ENTRY_NOT_FOUND,
            format!(
                "“{entry}” katmanı arşivde yok; arşivdeki katmanlar: {}.",
                layers.join(", ")
            ),
            "entry",
        )),
        (None, many) => Err(refuse(
            codes::ENTRY_NOT_FOUND,
            format!(
                "“{entry}” adı arşivde birden çok katmanın adı ({}); katmanın arşivdeki yolunu verin.",
                many.iter()
                    .map(|l| l.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            "entry",
        )),
    }
}

fn read_shp(path: &Path, name: &str, opts: &Opts) -> Result<ReadGis, Stop> {
    let unreadable = |why: String| {
        refuse(
            codes::IMPORT_UNREADABLE,
            format!("{name} Shapefile olarak okunamadı: {why}"),
            "path",
        )
    };
    let bytes = std::fs::read(path).map_err(|e| {
        refuse(
            codes::IMPORT_UNREADABLE,
            format!(
                "{} okunamadı: {e}. Dosyanın izinlerini denetleyin.",
                opts.path
            ),
            "path",
        )
    })?;
    if extension(path) == "zip" {
        let all = kentos_formats::shp::zip_layers(&bytes).map_err(unreadable)?;
        let chosen = zip_selection(&all, opts.entry.as_deref())?;
        let mut read = Vec::new();
        for layer in &chosen {
            let o = ShapefileReadOptions {
                layer: kentos_formats::shp::zip_layer_name(layer).to_owned(),
                max_entities: 0,
            };
            read.push(kentos_formats::shp::read_zip(&bytes, layer, &o).map_err(unreadable)?);
        }
        let mut names: Vec<&str> = chosen
            .iter()
            .map(|l| kentos_formats::shp::zip_layer_name(l))
            .collect();
        names.sort_unstable();
        if names.windows(2).any(|w| w[0].eq_ignore_ascii_case(w[1])) {
            return Err(refuse(
                codes::ENTRY_NOT_FOUND,
                format!(
                    "Arşivde aynı adlı birden çok katman var ({}); entry ile birinin arşivdeki yolunu verin.",
                    names.join(", ")
                ),
                "entry",
            ));
        }
        let result = if read.len() == 1 {
            read.remove(0)
        } else {
            merge(read)
        };
        return Ok(ReadGis::of(name, &bytes, result));
    }
    let parts = sidecars(path).map_err(unreadable)?;
    let part = |p: &str| {
        parts
            .iter()
            .find(|(n, _)| *n == p)
            .map(|(_, b)| b.as_slice())
    };
    let files = kentos_formats::shp::Files {
        shp: &bytes,
        shx: part("shx"),
        dbf: part("dbf"),
        prj: part("prj"),
        cpg: part("cpg"),
    };
    let o = ShapefileReadOptions {
        layer: base_name(name),
        max_entities: 0,
    };
    let mut result = kentos_formats::shp::read(&files, &o).map_err(unreadable)?;
    let mut listed = vec![".shp".to_owned()];
    listed.extend(
        ["shx", "dbf", "prj", "cpg"]
            .into_iter()
            .filter(|p| part(p).is_some())
            .map(|p| format!(".{p}")),
    );
    result.report.source.insert(
        0,
        SourceFact {
            label: "Dosyalar".into(),
            value: listed.join(", "),
        },
    );
    Ok(ReadGis::of(name, &bytes, result))
}

/// Reads the file with the shared reader; the refusal when it is not there or not of its format.
fn read_file(format: Format, opts: &Opts) -> Result<ReadGis, Stop> {
    let path = Path::new(&opts.path);
    if !path.exists() {
        return Err(refuse(
            codes::FILE_NOT_FOUND,
            format!("{} yok; dosyanın yolunu denetleyin.", opts.path),
            "path",
        ));
    }
    let name = name_of(path, &opts.path);
    let read = match format {
        Format::Shp => read_shp(path, &name, opts)?,
        Format::Geojson | Format::Kml => {
            let bytes = std::fs::read(path).map_err(|e| {
                refuse(
                    codes::IMPORT_UNREADABLE,
                    format!(
                        "{} okunamadı: {e}. Dosyanın izinlerini denetleyin.",
                        opts.path
                    ),
                    "path",
                )
            })?;
            let result = if format == Format::Geojson {
                let o = GeoJsonReadOptions {
                    layer: base_name(&name),
                    max_entities: 0,
                };
                kentos_formats::geojson::read(&bytes, &o)
            } else {
                kentos_formats::kml::read(&bytes, 0)
            }
            .map_err(|e| {
                refuse(
                    codes::IMPORT_UNREADABLE,
                    format!("{name} {} olarak okunamadı: {e}", format.word()),
                    "path",
                )
            })?;
            ReadGis::of(&name, &bytes, result)
        }
    };
    if let Some((place, kind)) = import_apply::unusable(&read.result.entities) {
        return Err(refuse(
            codes::IMPORT_UNREADABLE,
            import_apply::unusable_text(place, kind),
            "path",
        ));
    }
    Ok(read)
}

/// How sure a point moved between two systems is, as Koordinat dönüştür says it: “±2.1 m, EPSG:1783 + EPSG:5260;
/// resmî dönüşüm değil”, “doğruluğu bilinmiyor, Bölge 7”, “kesin, yalnız projeksiyon” within one datum.
fn accuracy_text(t: &core::Transformed) -> String {
    if t.via.is_empty() {
        return "kesin, yalnız projeksiyon".to_owned();
    }
    let mut text = match t.accuracy {
        Some(a) => format!("±{a} m, {}", t.via),
        None => format!("doğruluğu bilinmiyor, {}", t.via),
    };
    if t.unofficial {
        text.push_str("; resmî dönüşüm değil");
    }
    text
}

/// What the file's system is taken to be, and what was said of that.
struct Taken {
    /// None: the file declares nothing and none was given, so the project's own.
    srid: Option<u32>,
    warnings: Vec<CommandWarning>,
}

fn warning(code: &str, message: String, path: &str) -> CommandWarning {
    CommandWarning {
        code: code.into(),
        message,
        path: Some(path.into()),
    }
}

/// The system the coordinates are in: `srid`, else the file's statement, else the project's own.
fn taken(opts: &Opts, file: &str, declared: Option<&DeclaredCrs>) -> Result<Taken, Stop> {
    let mut warnings = Vec::new();
    let srid = match (opts.srid, declared) {
        (Some(s), d) => {
            if let Some(said) = d.and_then(|d| d.srid)
                && said != s
            {
                warnings.push(warning(
                    codes::CRS_OVERRIDDEN,
                    format!(
                        "{file} EPSG:{said} diyor; srid ile EPSG:{s} verildi: koordinatlar EPSG:{s} sayıldı, dönüştürülmeden (reproject verilirse yalnız EPSG:{s}'den projenin sistemine çevrilir)."
                    ),
                    "srid",
                ));
            }
            Some(s)
        }
        (None, Some(d)) => match d.srid {
            Some(s) if kentos_project::crs::system(s).is_some() => Some(s),
            Some(s) => {
                return Err(refuse(
                    codes::CRS_UNKNOWN,
                    format!(
                        "{file} koordinatların EPSG:{s} sisteminde olduğunu söylüyor; bu sistem KentOS'ta tanımlı değil. Koordinatlar projenin sistemindeyse srid olarak onu verin, değilse dosyayı projenin sistemine çevirip verin."
                    ),
                    "srid",
                ));
            }
            None => {
                return Err(refuse(
                    codes::CRS_UNKNOWN,
                    format!(
                        "{file}'nin koordinat sistemi okunamadı (“{}”). Koordinatların hangi sistemde olduğunu srid ile verin.",
                        d.text
                    ),
                    "srid",
                ));
            }
        },
        (None, None) => {
            warnings.push(warning(
                codes::CRS_UNDECLARED,
                format!(
                    "{file} koordinat sistemi bildirmiyor; koordinatlar projenin sisteminde sayılıp olduğu gibi alındı."
                ),
                "path",
            ));
            None
        }
    };
    Ok(Taken { srid, warnings })
}

/// The way the coordinates move into the project's system.
struct Way {
    from: core::System,
    to: core::System,
    choices: Vec<core::Choice>,
    /// The move of the box's middle: how sure, by what.
    mid: Option<core::Transformed>,
    from_srid: u32,
    to_srid: u32,
}

/// Whether the file's coordinates need moving into the project's system, and the way when they do: none when the
/// system is the project's, or the project is local (it takes any file's coordinates as they are, docs/adr/0165 §2),
/// or the file's is not known; the refusal when the file is in another system and the move was not asked or cannot
/// be made.
fn way(
    doc: &Document,
    opts: &Opts,
    file: &str,
    srid: Option<u32>,
    bounds: Option<Bounds>,
) -> Result<Option<Way>, Stop> {
    let settings = doc.settings();
    let Some(from_srid) = srid else {
        return Ok(None);
    };
    let project = settings.srid;
    if from_srid == project && settings.has_system() {
        return Ok(None);
    }
    let local = !settings.has_system();
    if local {
        if opts.reproject {
            return Err(refuse(
                codes::CRS_MISMATCH,
                format!(
                    "{file} EPSG:{from_srid} sisteminde; projenin koordinat sistemi yok (yerel proje), çevrilecek bir sistem yok. Projeyi bir koordinat sisteminde açın ya da reproject vermeyin: koordinatlar olduğu gibi alınır."
                ),
                "reproject",
            ));
        }
        return Ok(None);
    }
    let own = if project == 0 {
        "projenin kendi tanımlı sistemi".to_owned()
    } else {
        format!("proje EPSG:{project}")
    };
    if !opts.reproject {
        return Err(refuse(
            codes::CRS_MISMATCH,
            format!(
                "{file} EPSG:{from_srid} sisteminde, {own}. Koordinatların projenin sistemine çevrilmesi için reproject verin ya da projeyi EPSG:{from_srid}'de açın."
            ),
            "reproject",
        ));
    }
    let Some(from) = kentos_project::crs::system(from_srid).and_then(|s| s.transform_system())
    else {
        return Err(refuse(
            codes::CRS_MISMATCH,
            format!(
                "{file}'nin sistemi EPSG:{from_srid} KentOS'un çevirebildiği sistemlerde yok; dosyayı projenin sistemine çevirip verin."
            ),
            "reproject",
        ));
    };
    let Some(to) = kentos_project::systems::own(settings).and_then(|n| n.system) else {
        return Err(refuse(
            codes::CRS_MISMATCH,
            format!(
                "Projenin sistemi ({own}) koordinat çevirmeye elvermiyor; {file} dosyasını projenin sistemine çevirip verin."
            ),
            "reproject",
        ));
    };
    let choices = kentos_project::systems::choices(settings);
    let mid = match bounds {
        Some(b) => {
            let at = Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
            match core::transform_in(&from, &to, at, &choices) {
                Ok(t) => Some(t),
                Err(why) => {
                    return Err(refuse(
                        codes::CRS_MISMATCH,
                        format!(
                            "{file}'nin koordinatları projenin sistemine ({own}) çevrilemez: ortaları {}. Dosya aslında projenin sistemindeyse srid ile onu verin.",
                            kentos_formats::reproject::why(why)
                        ),
                        "reproject",
                    ));
                }
            }
        }
        None => None,
    };
    Ok(Some(Way {
        from,
        to,
        choices,
        mid,
        from_srid,
        to_srid: project,
    }))
}

fn crs_move(w: &Way, moved: u32, left: u32) -> CrsMove {
    let (accuracy, via, unofficial, text) = match &w.mid {
        Some(t) => (t.accuracy, t.via.clone(), t.unofficial, accuracy_text(t)),
        None => (
            None,
            String::new(),
            false,
            "kesin, yalnız projeksiyon".into(),
        ),
    };
    CrsMove {
        from: w.from_srid,
        to: w.to_srid,
        via,
        accuracy,
        unofficial,
        text,
        moved,
        left,
    }
}

/// The routing of the input's layers.
fn routing<'a>(opts: &'a Opts) -> Routing<'a> {
    Routing {
        skip_layers: &opts.skip_layers,
        layers: &opts.layers,
        all_onto: opts.layer.as_ref(),
        group: opts.group.as_ref(),
    }
}

/// Puts `read` into `doc` by `opts`: one step (the new layers, then the objects); `files` writes the rejects file.
fn write(
    doc: &mut Document,
    format: Format,
    opts: &Opts,
    read: ReadGis,
    files: bool,
) -> CommandResult<GisImported> {
    match write_checked(doc, format, opts, read, files) {
        Ok(done) => done,
        Err(stop) => stop.into(),
    }
}

fn write_checked(
    doc: &mut Document,
    format: Format,
    opts: &Opts,
    read: ReadGis,
    files: bool,
) -> Result<CommandResult<GisImported>, Stop> {
    crate::import_merge::check_rejects_path(&opts.rejects)?;
    let ReadGis {
        mut result,
        file,
        bytes,
        sha256,
        premoved,
    } = read;
    let declared = result.declared_crs.clone();
    let said = taken(opts, &file, declared.as_ref())?;
    let routing = routing(opts);
    // The refusals of a layer come before the move is tried.
    route_layers(doc, &routing, &result.layers)?;
    let bounds = premoved.as_ref().map_or(result.bounds, |p| p.bounds);
    let mover = way(doc, opts, &file, said.srid, bounds)?;
    let mut moved: Option<CrsMove> = None;
    if let Some(w) = &mover {
        match &premoved {
            Some(p) if p.from == w.from_srid => {
                moved = Some(crs_move(w, result.entities.len() as u32, p.left));
            }
            Some(p) => {
                return Err(refuse(
                    codes::IMPORT_UNREADABLE,
                    format!(
                        "Okunan dosya EPSG:{} sisteminden çevrilmiş, ama verilen sistem EPSG:{}; dosyayı yeniden okuyun.",
                        p.from, w.from_srid
                    ),
                    "srid",
                ));
            }
            None => {
                let before = result.entities.len() as u32;
                kentos_formats::reproject::reproject(&mut result, &w.from, &w.to, &w.choices);
                let kept = result.entities.len() as u32;
                moved = Some(crs_move(w, kept, before - kept));
            }
        }
    }
    let (layers, objects) = route_layers(doc, &routing, &result.layers)?;
    nothing_to_import(
        &file,
        result.layers.is_empty(),
        layers.is_empty(),
        objects,
        &opts.skip_layers,
    )?;
    let plan = ImportPlan {
        source_srid: said.srid,
        label: step_name(format, &file),
        layers,
        group: group_of(routing.group, &file),
    };
    let mut warnings = said.warnings;
    if moved.as_ref().is_some_and(|m| m.unofficial) {
        warnings.push(warning(
            codes::CRS_UNOFFICIAL,
            format!(
                "{file} koordinatları resmî olmayan bir dönüşümle projenin sistemine çevrildi ({}); resmî dönüşümü olan bir dosya gerekiyorsa dosyayı o dönüşümle çevirip verin.",
                moved.as_ref().map_or("", |m| m.text.as_str())
            ),
            "reproject",
        ));
    }
    warnings.extend(file_import::warnings_of_read(&result));
    let source_layers: Vec<String> = result.layers.iter().map(|l| l.name.clone()).collect();
    let report = serde_json::to_value(&result.report).unwrap_or(serde_json::Value::Null);
    // A layer going onto one of the drawing's meets its objects (docs/adr/0628 §5); the rows have no lasting ids.
    let mut updates = Vec::new();
    let mut merged = Vec::new();
    let mut rejected = Vec::new();
    let mut reject_columns: Vec<String> = Vec::new();
    for (source, target) in &plan.layers {
        // A layer's columns: its fields, else its objects' attribute names (a GeoJSON file's properties).
        let mut columns: Vec<String> = result
            .layers
            .iter()
            .find(|l| &l.name == source)
            .map(|l| l.fields.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default();
        if columns.is_empty() {
            let keys: std::collections::BTreeSet<&String> = result
                .entities
                .iter()
                .filter(|e| &e.base().layer_id == source)
                .flat_map(|e| e.base().attrs.keys())
                .collect();
            columns = keys.into_iter().cloned().collect();
        }
        for c in &columns {
            if !reject_columns.iter().any(|x| x.eq_ignore_ascii_case(c)) {
                reject_columns.push(c.clone());
            }
        }
        if let (Some(m), import_apply::LayerTarget::Existing(id)) = (&opts.merge, target) {
            let (mine, rest): (Vec<kentos_commands::Entity>, Vec<kentos_commands::Entity>) = std::mem::take(&mut result.entities)
                .into_iter()
                .partition(|e| &e.base().layer_id == source);
            result.entities = rest;
            let rows: Vec<(kentos_domain::Uuid, kentos_commands::Entity)> = mine
                .into_iter()
                .enumerate()
                .map(|(i, e)| {
                    let name = format!("{file}:{source}:{i}");
                    (kentos_domain::Uuid::from_bytes(kentos_commands::identity::uuid_v5(&[0u8; 16], name.as_bytes())), e)
                })
                .collect();
            let numbers: Vec<i64> = (1..=rows.len() as i64).collect();
            let mut out = crate::import_merge::merge(doc, id, source, &rows, &numbers, &columns, m, false)?;
            result.entities.extend(out.add.drain(..).map(|(_, e)| e));
            updates.append(&mut out.update);
            merged.extend(out.report);
            rejected.append(&mut out.rejected);
        }
    }
    let rejects_bytes = opts
        .rejects
        .as_ref()
        .filter(|_| files)
        .map(|_| crate::import_merge::rejects_csv(&rejected, &reject_columns));
    let mut rejects_written = None;
    let entities = std::mem::take(&mut result.entities);
    let applied = doc.transact(&plan.label, |doc| {
        if !updates.is_empty() {
            doc.update_many(std::mem::take(&mut updates), &plan.label);
        }
        let applied = import_apply::apply_import(doc, entities, Vec::new(), &plan)
            .map_err(|message| refuse(codes::IMPORT_UNREADABLE, message, "path"))?;
        if let (Some(path), Some(bytes)) = (&opts.rejects, &rejects_bytes) {
            rejects_written = Some(crate::import_merge::write_rejects(path, bytes, rejected.len() as u32)?);
        }
        Ok::<_, Stop>(applied)
    })?;
    if let Some(w) = crate::import_merge::warning(&merged, &file) {
        warnings.push(w);
    }
    Ok(CommandResult::Completed {
        output: GisImported {
            file,
            bytes,
            sha256,
            declared,
            srid: said.srid,
            moved,
            objects: u32::try_from(applied.slots.len()).unwrap_or(u32::MAX),
            source_layers,
            targets: applied.targets.into_iter().collect::<BTreeMap<_, _>>(),
            layers: applied
                .made
                .into_iter()
                .map(|(id, name)| ImportedLayer { id, name })
                .collect(),
            group: applied.group,
            report,
            merged,
            rejects: rejects_written,
            revision: doc.revision().to_string(),
        },
        warnings,
    })
}

/// Every check, the file read: what plan and execute stop with.
fn check(doc: &Document, format: Format, opts: &Opts) -> Result<ReadGis, Stop> {
    check_path(format, opts)?;
    checks::revision(doc, opts.expected_revision.as_deref())?;
    read_file(format, opts)
}

/// Checks `input` against the document and the file, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, format: Format, opts: &Opts) -> CommandResult<()> {
    let read = match check(cx.doc, format, opts) {
        Ok(read) => read,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    match write_checked(&mut copy, format, opts, read, false) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would take in, on a reading copy; the drawing is left as it is.
pub fn plan(cx: &ExecutionContext<'_>, format: Format, opts: &Opts) -> CommandResult<GisImported> {
    let read = match check(cx.doc, format, opts) {
        Ok(read) => read,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut copy, format, opts, read, false);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Reads the file and takes it in as one undo step, recorded under the command's name.
pub fn execute<I>(
    cx: &mut ExecutionContext<'_>,
    format: Format,
    input: I,
) -> CommandResult<GisImported>
where
    I: Serialize + Into<Opts>,
{
    let (name, version) = format.command();
    crate::call::recorded(cx, name, version, input, move |cx, input| {
        let opts: Opts = input.into();
        let read = match check(cx.doc, format, &opts) {
            Ok(read) => read,
            Err(stop) => return stop.into(),
        };
        write(cx.doc, format, &opts, read, true)
    })
}

/// [`execute`] with the file its host already read (the desktop's window reads it on a thread of its own and shows
/// it first): `read` must be what the shared reader gives for `input`. The journal names the command and its input.
pub fn execute_read<I>(
    cx: &mut ExecutionContext<'_>,
    format: Format,
    input: I,
    read: ReadGis,
) -> CommandResult<GisImported>
where
    I: Serialize + Into<Opts>,
{
    let (name, version) = format.command();
    crate::call::recorded(cx, name, version, input, move |cx, input| {
        let opts: Opts = input.into();
        if let Err(stop) = checks::revision(cx.doc, opts.expected_revision.as_deref()) {
            return stop.into();
        }
        write(cx.doc, format, &opts, read, true)
    })
}

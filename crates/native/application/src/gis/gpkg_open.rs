//! A GeoPackage opened as a project (docs/adr/9116). A file that keeps one KentOS project opens it; one that keeps
//! more says their names and opens none until one is named. A file that keeps none opens as a new project of its
//! tables, made in memory under the file's name (or the one asked for) and written nowhere (docs/adr/9122): looking at
//! a file changes no byte of it, and a project goes into the file only when the owner saves it there ([`keep`]):
//!
//! - its system is the one most of its geometry tables are in, of the systems a project can have, the first in the
//!   file's order on a tie; none of them, no system ("Yerel"), said;
//! - its layers are the tables as data layers (their rows stay in the file), points first, then lines, areas, other
//!   geometry, then tables without geometry, each group in the file's order, under the Taslak drawing layer a new
//!   project starts on; another program's style table is not a layer;
//! - a table's default style (`layer_styles`: the one marked default, else the latest written) is its layer's look,
//!   and a table's KentOS dataset description its layer's;
//! - its tile tables (pictures, gridded coverages) are rasters linked to the table, each on a drawing layer of its own
//!   under the tables' layers (docs/adr/9117 §2); they vote on the system as the geometry tables do;
//! - its first view frames the tables in its system.
//!
//! Opening reads no row and writes nothing: the file's connections are read-only, so a file without write permission
//! opens as well. The desktop's Aç and a GeoPackage dropped on an empty window, the headless host's open (Python,
//! MCP) come here.

use std::path::Path;

use kentos_commands::{
    Bounds, CommandResult, DocumentSnapshotV2, DrawingFont, GpkgTileSet, LayerDataOperation, LayersData, RastersGpkgRead,
    Workspace,
};
use kentos_data::StoredProject;
use kentos_domain::{Document, NewLayer};

use crate::ExecutionContext;
use crate::data_describe::{self, Described};

/// The drawing layer a new project starts on, kept over the tables' layers.
pub const DRAFT_LAYER: &str = kentos_project::new_project::STANDARD_ACTIVE_LAYER;
/// Another program's table of its layers' styles: read for the looks, never a layer.
pub const STYLE_TABLE: &str = kentos_geopackage::styles::STYLES_TABLE;

/// What opening a GeoPackage gives.
pub enum Opened {
    /// The project it keeps (its only one, or the one asked for): its name and `.kcad` bytes.
    Kept { name: String, bytes: Vec<u8> },
    /// More than one project and none asked for: each one, in the Turkish alphabet's order.
    Several(Vec<StoredProject>),
    /// A project of its tables, made here in memory: the file is as it was.
    Built(Box<Built>),
}

/// A project made of a GeoPackage's tables, kept nowhere yet.
pub struct Built {
    /// The project, its undo history empty and nothing in it changed: closed as it is, nothing is lost.
    pub document: Document,
    /// The name it would be kept under in the file (the file's, or the one asked for).
    pub name: String,
    /// What the owner should know: the system chosen and why, what was not taken.
    pub notes: Vec<Note>,
}

/// A thing said about the opening, with a code the tests and the hosts tell apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub code: &'static str,
    pub text: String,
}

fn note(code: &'static str, text: impl Into<String>) -> Note {
    Note { code, text: text.into() }
}

/// The first 16 bytes of an SQLite database file.
const SQLITE: &[u8; 16] = b"SQLite format 3\0";

/// Whether the file at `path` is an SQLite database (a GeoPackage is one), read from its first bytes.
pub fn is_sqlite(path: &Path) -> bool {
    use std::io::Read;
    let mut head = [0u8; 16];
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)).is_ok() && &head == SQLITE
}

/// The address a GeoPackage's tables and projects are read at: its absolute path.
pub fn address_of(path: &Path) -> String {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()).display().to_string()
}

/// The name a project made of `path` takes: the file's name without `.gpkg`.
pub fn project_name(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().trim().to_owned()).filter(|s| !s.is_empty()).unwrap_or_else(|| "Proje".into())
}

/// Opens the GeoPackage at `path`: the project named `project` (made of its tables when the file keeps none), or
/// without a name its only project, the names of its projects, or a new project of its tables.
pub fn open(path: &Path, project: Option<&str>) -> Result<Opened, String> {
    let file = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
    if !path.is_file() {
        return Err(format!("“{file}” bulunamadı ({}). Dosya taşınmış ya da silinmiş olabilir.", path.display()));
    }
    if !is_sqlite(path) {
        return Err(format!(
            "“{file}” bir GeoPackage değil: dosyanın başında SQLite veritabanı imzası yok. Dosya bozuk ya da başka türden olabilir."
        ));
    }
    let address = address_of(path);
    let kept = crate::project_store::list(&address, None)?;
    let asked = project.map(str::trim).filter(|p| !p.is_empty());
    match (asked, kept.len()) {
        (Some(name), _) if kept.iter().any(|p| p.name == name) => {
            let bytes = crate::project_store::read(&address, None, name)?;
            Ok(Opened::Kept { name: name.to_owned(), bytes })
        }
        (Some(name), 0) => build(path, &address, name).map(|b| Opened::Built(Box::new(b))),
        (Some(name), _) => Err(format!(
            "“{file}” dosyasında “{name}” adında proje yok. Dosyadaki projeler: {}.",
            kept.iter().map(|p| format!("“{}”", p.name)).collect::<Vec<_>>().join(", ")
        )),
        (None, 0) => build(path, &address, &project_name(path)).map(|b| Opened::Built(Box::new(b))),
        (None, 1) => {
            let name = kept[0].name.clone();
            let bytes = crate::project_store::read(&address, None, &name)?;
            Ok(Opened::Kept { name, bytes })
        }
        (None, _) => Ok(Opened::Several(kept)),
    }
}

/// Where a geometry type's layer stands: points over lines over areas over other geometry over tables without.
fn group(d: &Described) -> u8 {
    let Some(g) = &d.data.geometry else {
        return 4;
    };
    match g.type_name.as_str() {
        "POINT" | "MULTIPOINT" => 0,
        "LINESTRING" | "MULTILINESTRING" | "CIRCULARSTRING" | "COMPOUNDCURVE" | "MULTICURVE" | "CURVE" => 1,
        "POLYGON" | "MULTIPOLYGON" | "CURVEPOLYGON" | "MULTISURFACE" | "SURFACE" => 2,
        _ => 3,
    }
}

/// Whether a table is the file's own bookkeeping rather than data: the style table, KentOS's tables.
fn bookkeeping(table: &str) -> bool {
    table == STYLE_TABLE || table.starts_with("kentos_")
}

/// The project's system of its tables' systems (in the file's order): the one most of them are in, of those a project
/// can have, the first on a tie; none of them: no system. The notes say a mix and a system left out.
fn system_of(tables: &[Described], tiles: &[GpkgTileSet]) -> (u32, Vec<Note>) {
    if tables.is_empty() && tiles.is_empty() {
        return (
            0,
            vec![note(
                "no_tables",
                "Dosyada tablo yok; proje yalnız Taslak katmanıyla açıldı. Yeni katman, tablosunu bu dosyada açar.",
            )],
        );
    }
    let systems: Vec<u32> = tables
        .iter()
        .filter_map(|t| t.data.geometry.as_ref().and_then(|g| g.srid))
        .chain(tiles.iter().filter_map(|t| t.srid.filter(|s| *s > 0)))
        .collect();
    let mut notes = Vec::new();
    let mut distinct: Vec<u32> = Vec::new();
    for s in &systems {
        if !distinct.contains(s) {
            distinct.push(*s);
        }
    }
    let listed: Vec<u32> = systems.iter().copied().filter(|s| kentos_project::crs::system(*s).is_some()).collect();
    let chosen = {
        let count = |s: u32| listed.iter().filter(|x| **x == s).count();
        let most = listed.iter().map(|s| count(*s)).max().unwrap_or(0);
        listed.iter().copied().find(|s| count(*s) == most)
    };
    let words = |list: &[u32]| list.iter().map(|s| format!("EPSG:{s}")).collect::<Vec<_>>().join(", ");
    if distinct.len() > 1 {
        let to = chosen.map_or_else(|| "sistemsiz (Yerel)".to_owned(), |s| format!("EPSG:{s}"));
        notes.push(note(
            "mixed_systems",
            format!("Tablolar birden çok koordinat sisteminde ({}); proje {to} oldu, öbür tablolar projenin sisteminde çizilir.", words(&distinct)),
        ));
    }
    match chosen {
        Some(s) => (s, notes),
        None if !distinct.is_empty() => {
            notes.push(note(
                "system_not_listed",
                format!(
                    "Tabloların sistemi ({}) KentOS'ta proje sistemi olarak yok; proje sistemsiz (Yerel) açıldı, tablolar kendi sisteminde kalır. Proje ayarları › Koordinat sistemi'nden bir sistem seçebilirsiniz.",
                    words(&distinct)
                ),
            ));
            (0, notes)
        }
        None => {
            notes.push(note(
                "no_system",
                "Dosyanın tablolarında koordinat sistemi yok; proje sistemsiz (Yerel) açıldı. Proje ayarları › Koordinat sistemi'nden bir sistem seçebilirsiniz.",
            ));
            (0, notes)
        }
    }
}

/// The new project's first view: the tables' boxes in its system, framed together; none without one.
fn home_of(tables: &[Described], tiles: &[GpkgTileSet], settings: &kentos_commands::ProjectSettings) -> Option<Bounds> {
    let mut out: Option<[f64; 4]> = None;
    let boxes = tables
        .iter()
        .filter_map(|t| Some((t.extent?, t.data.geometry.as_ref()?.srid)))
        .chain(tiles.iter().map(|t| (t.data_bounds.unwrap_or(t.bounds), t.srid.filter(|s| *s > 0))));
    for (b, srid) in boxes {
        let Ok(b) = crate::sources::extent_in_project(settings, srid, b) else {
            continue;
        };
        if !(b.iter().all(|v| v.is_finite()) && b[2] >= b[0] && b[3] >= b[1]) {
            continue;
        }
        out = Some(match out {
            None => b,
            Some(o) => [o[0].min(b[0]), o[1].min(b[1]), o[2].max(b[2]), o[3].max(b[3])],
        });
    }
    out.map(|b| Bounds { min_x: b[0], min_y: b[1], max_x: b[2], max_y: b[3] })
}

/// A project of the file's tables, named `name`, in memory.
fn build(path: &Path, address: &str, name: &str) -> Result<Built, String> {
    let source = kentos_data::registry().source(address).map_err(|e| e.to_string())?;
    let mut notes = Vec::new();
    let mut tables: Vec<Described> = Vec::new();
    for t in source.tables().map_err(|e| e.to_string())? {
        if bookkeeping(&t.name) {
            continue;
        }
        match data_describe::describe_in(source.as_ref(), address, &t.name) {
            Ok(d) => {
                if !d.left.is_empty() {
                    notes.push(note(
                        "fields_left",
                        format!("“{}” tablosunun alan olarak alınmayan sütunları: {}", t.name, d.left.join("; ")),
                    ));
                }
                tables.push(d);
            }
            Err(why) => notes.push(note("table_not_taken", format!("“{}” tablosu katman olarak alınmadı: {why}", t.name))),
        }
    }
    let tiles = tile_tables(path, &mut notes);
    // The system of the file's order; then the layers' order, stable inside each group.
    let (srid, said) = system_of(&tables, &tiles);
    notes.splice(0..0, said);
    tables.sort_by_key(group);

    let mut snapshot = kentos_project::new_project::new_project(&kentos_project::new_project::NewProject {
        name: name.to_owned(),
        srid,
        plot_scale: 1000.0,
        workspace: Workspace::Gis,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
        annotation: None,
        cad_work: None,
        heights: None,
    })?;
    // The tables are its layers: of a new project's tree only the layer it starts drawing on stays.
    snapshot.layers.retain(|n| n.id == DRAFT_LAYER);
    snapshot.active_layer = DRAFT_LAYER.to_owned();
    if let Some(home) = home_of(&tables, &tiles, &snapshot.settings) {
        snapshot.home_view = Some(home);
    }
    let mut doc = Document::from_snapshot(snapshot)?;

    let mut layers: Vec<(String, String)> = Vec::new();
    for t in &tables {
        let input = LayersData {
            operation: LayerDataOperation::Add,
            layer: None,
            name: Some(t.name.clone()),
            parent: None,
            index: None,
            data: Some(t.data.clone()),
            fields: (!t.fields.is_empty()).then(|| t.fields.clone()),
            expected_revision: None,
        };
        match crate::layers_data::execute(&mut ExecutionContext::new(&mut doc), input) {
            CommandResult::Completed { output, .. } => layers.push((t.data.table.clone(), output.layer)),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                notes.push(note("table_not_taken", format!("“{}” tablosu katman olarak alınmadı: {}", t.name, error.message)));
            }
            _ => notes.push(note("table_not_taken", format!("“{}” tablosu katman olarak alınmadı.", t.name))),
        }
    }
    looks(&mut doc, path, &layers, &mut notes);
    rasters(&mut doc, path, &tiles, &mut notes);

    // Built through commands, opened as read: no undo step stays, and nothing the owner did waits to be saved.
    let snapshot: DocumentSnapshotV2 = doc.to_snapshot_v2();
    let doc = Document::from_snapshot_v2(snapshot)?;
    Ok(Built { document: doc, name: name.to_owned(), notes })
}

/// Keeps `document` in the GeoPackage at `path` under `name` (docs/adr/9122): the owner's own save, never opening's.
/// Its data layers' and rasters' paths are written relative to the file, the bytes read back before they count.
pub fn keep(document: &Document, path: &Path, name: &str) -> Result<(), String> {
    let address = address_of(path);
    let mut snapshot = document.to_snapshot_v2();
    crate::project_store::with_relative(&mut snapshot, &address);
    let bytes = kentos_kcad::encode_verified(&snapshot).map_err(|e| format!("proje KCAD v2'ye yazılamadı: {e}"))?;
    crate::project_store::write(&address, None, name, &bytes)
}

/// The file's tile tables (none when it keeps no tile matrix set); why they could not be read, said.
fn tile_tables(path: &Path, notes: &mut Vec<Note>) -> Vec<GpkgTileSet> {
    let has = kentos_geopackage::GeoPackage::open(path, false)
        .map_err(|e| e.0)
        .and_then(|gp| {
            gp.connection()
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'gpkg_tile_matrix_set'",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(|e| e.to_string())
        });
    match has {
        Ok(0) => Vec::new(),
        Ok(_) => crate::gpkg_tiles::tile_tables(path).unwrap_or_else(|why| {
            notes.push(note("tiles_unread", format!("Dosyanın döşeme tabloları okunamadı: {why}")));
            Vec::new()
        }),
        Err(why) => {
            notes.push(note("tiles_unread", format!("Dosyanın döşeme tabloları okunamadı: {why}")));
            Vec::new()
        }
    }
}

/// Each tile table as a raster linked to it (`cad.rasters.gpkgread`, `link`: its tiles read from the file as the view
/// needs them, at the finest zoom), on a drawing layer of its own named after the table, under the other layers.
fn rasters(doc: &mut Document, path: &Path, tiles: &[GpkgTileSet], notes: &mut Vec<Note>) {
    let file = address_of(path);
    for t in tiles {
        let layer = match doc.add_layer_at(NewLayer::layer(t.table.clone()), None, None, None, false) {
            Ok(id) => id,
            Err(why) => {
                notes.push(note("tiles_not_taken", format!("“{}” döşeme tablosu raster olarak alınmadı: {}", t.table, why.0)));
                continue;
            }
        };
        let input = RastersGpkgRead {
            path: file.clone(),
            table: Some(t.table.clone()),
            zoom: None,
            area: None,
            output: None,
            layer: Some(layer.clone()),
            overwrite: None,
            link: Some(true),
            drawing: None,
            expected_revision: None,
        };
        let why = match crate::gpkg_tiles::read::execute(&mut ExecutionContext::new(doc), input) {
            CommandResult::Completed { .. } => continue,
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                // The reader's words are a command caller's (which input to change); opening says what the file is.
                match error.code.as_str() {
                    crate::codes::CRS_MISMATCH => format!(
                        "karoları {}, proje {}; raster bir sistemden öbürüne çevrilmeden çizilmez.",
                        t.srid.map_or_else(|| "tanımsız bir sistemde".to_owned(), |s| format!("EPSG:{s}'de")),
                        match doc.settings().srid {
                            0 => "sistemsiz".to_owned(),
                            s => format!("EPSG:{s}'de"),
                        }
                    ),
                    crate::codes::TILE_TABLE_EMPTY => {
                        "tablonun verisinin kutusu en ayrıntılı düzeyde hiçbir karoyu kapsamıyor (kutu bir nokta ya da ızgaranın dışında)."
                            .to_owned()
                    }
                    _ => error.message,
                }
            }
            _ => String::new(),
        };
        let _ = doc.remove_layer(&layer);
        notes.push(note("tiles_not_taken", format!("“{}” döşeme tablosu raster olarak alınmadı: {why}", t.table)));
    }
}

/// The file's looks on its tables' layers: each table's default style (another program's `layer_styles`) as its
/// layer's renderer, its KentOS dataset description as its layer's; what cannot be taken is said.
fn looks(doc: &mut Document, path: &Path, layers: &[(String, String)], notes: &mut Vec<Note>) {
    if layers.is_empty() {
        return;
    }
    let profile = match crate::gpkg_profile::read_profile(path) {
        Ok(p) => p,
        Err(why) => {
            notes.push(note("styles_unread", format!("Dosyanın katman stilleri okunamadı: {why}")));
            return;
        }
    };
    for (table, layer) in layers {
        if let Some(style) = profile.styles.get(table) {
            crate::sld::add_assets(doc, &style.assets);
            let input = kentos_commands::LayersRenderer {
                layer: layer.clone(),
                renderer: Some(style.renderer.clone()),
                expected_revision: None,
            };
            match crate::layers_renderer::style_of(doc, &input) {
                Ok(look) => {
                    doc.set_layer_style(layer, look, crate::layers_renderer::LABEL);
                    if !style.lost.is_empty() {
                        notes.push(note(
                            "style_lost",
                            format!("“{table}” tablosunun stilinden alınamayanlar: {}", style.lost.join(", ")),
                        ));
                    }
                }
                Err(why) => notes.push(note("style_not_taken", format!("“{table}” tablosunun stili alınmadı: {why}"))),
            }
        }
        if let Some(dataset) = profile.datasets.get(table)
            && let Some(mut info) = doc.layers().info_of(layer)
        {
            info.dataset = Some(dataset.clone());
            if let Err(why) = doc.set_layer_info(layer, info, crate::datasets_edit::LABEL) {
                notes.push(note("dataset_not_taken", format!("“{table}” tablosunun veri kümesi bilgisi alınmadı: {why}")));
            }
        }
    }
    for left in profile.left {
        notes.push(note("profile_left", left));
    }
    if !profile.relations.is_empty() {
        notes.push(note(
            "relations_not_taken",
            format!(
                "Dosyadaki {} ilişki bu sürümde projeye alınmadı; tablolar katman olarak açıldı.",
                profile.relations.len()
            ),
        ));
    }
}

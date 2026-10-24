//! Kaynaklar's folder listing (docs/adr/0199 §7; the web's
//! `model/sources.ts`), both platforms held to the shared cases
//! fixtures/sources/v1/cases.json written by
//! scripts/fixtures/source_list_cases.py: a folder shows its folders and the
//! files the panel adds as layers, but those whose name begins with a dot, in
//! the natural order; a file's kind by its extension, case aside; a Shapefile
//! as its .shp with the files of the same name and the extensions .shx, .dbf,
//! .prj and .cpg as its parts, which are not shown on their own. The panel is
//! the desktop's `sources`. The source browser (docs/adr/0599) adds GeoPackage,
//! FlatGeobuf, GML, raster and point cloud files, the panel's search and its
//! recent and favourite sources, held to the same cases.

use kentos_geometry_core::ops::point_editor::natural_order;

/// What a file of a folder is added as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceKind {
    GeoJson,
    Shapefile,
    Dxf,
    Ncz,
    Gnss,
    Coords,
    /// KML or KMZ (docs/adr/0222).
    Kml,
    /// A GeoPackage, its tables shown under it (docs/adr/0599 §2).
    GeoPackage,
    FlatGeobuf,
    Gml,
    /// A GeoParquet (docs/adr/0628).
    GeoParquet,
    /// GeoTIFF, TIFF or a VRT mosaic (docs/adr/0204, 0517).
    Raster,
    /// LAS or LAZ, COPC too (docs/adr/0207).
    PointCloud,
}

impl SourceKind {
    /// What the panel calls it.
    pub fn label(self) -> &'static str {
        match self {
            SourceKind::GeoJson => "GeoJSON",
            SourceKind::Shapefile => "Shapefile",
            SourceKind::Dxf => "DXF",
            SourceKind::Ncz => "Netcad NCZ",
            SourceKind::Gnss => "GNSS (GPX, NMEA)",
            SourceKind::Coords => "Koordinat listesi",
            SourceKind::Kml => "KML/KMZ",
            SourceKind::GeoPackage => "GeoPackage",
            SourceKind::FlatGeobuf => "FlatGeobuf",
            SourceKind::Gml => "GML",
            SourceKind::GeoParquet => "GeoParquet",
            SourceKind::Raster => "Raster (GeoTIFF, VRT)",
            SourceKind::PointCloud => "Nokta bulutu (LAS, LAZ)",
        }
    }

    /// Its key in the shared cases.
    pub fn key(self) -> &'static str {
        match self {
            SourceKind::GeoJson => "geojson",
            SourceKind::Shapefile => "shapefile",
            SourceKind::Dxf => "dxf",
            SourceKind::Ncz => "ncz",
            SourceKind::Gnss => "gnss",
            SourceKind::Coords => "coords",
            SourceKind::Kml => "kml",
            SourceKind::GeoPackage => "geopackage",
            SourceKind::FlatGeobuf => "flatgeobuf",
            SourceKind::Gml => "gml",
            SourceKind::GeoParquet => "geoparquet",
            SourceKind::Raster => "raster",
            SourceKind::PointCloud => "pointcloud",
        }
    }

    /// Its icon: its import command's (the web's names).
    pub fn icon(self) -> &'static str {
        match self {
            SourceKind::GeoJson => "importGeojson",
            SourceKind::Shapefile => "importShp",
            SourceKind::Dxf => "importDxf",
            SourceKind::Ncz => "importNcz",
            SourceKind::Gnss => "importGnss",
            SourceKind::Coords => "importNcn",
            SourceKind::Kml => "importKml",
            SourceKind::GeoPackage => "importGpkg",
            SourceKind::FlatGeobuf => "importFgb",
            SourceKind::Gml => "importGml",
            SourceKind::GeoParquet => "importParquet",
            SourceKind::Raster => "rasterAdd",
            SourceKind::PointCloud => "pointCloudAdd",
        }
    }

    pub const ALL: [SourceKind; 13] = [
        SourceKind::GeoJson,
        SourceKind::Shapefile,
        SourceKind::Dxf,
        SourceKind::Ncz,
        SourceKind::Gnss,
        SourceKind::Coords,
        SourceKind::Kml,
        SourceKind::GeoPackage,
        SourceKind::FlatGeobuf,
        SourceKind::Gml,
        SourceKind::GeoParquet,
        SourceKind::Raster,
        SourceKind::PointCloud,
    ];
}

/// One entry of a folder: its name, and whether it is a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEntry {
    pub name: String,
    pub dir: bool,
}

/// A file the panel shows: its name, its kind, the files read with it (a
/// Shapefile's parts; the file itself first).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    pub name: String,
    pub kind: SourceKind,
    pub parts: Vec<String>,
}

/// What a folder shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listing {
    pub folders: Vec<String>,
    pub files: Vec<SourceFile>,
}

const PARTS: [&str; 4] = ["shx", "dbf", "prj", "cpg"];

/// The stem and the extension (lower case); no extension without a dot
/// past the first letter.
fn split(name: &str) -> (&str, String) {
    match name.rfind('.') {
        Some(at) if at > 0 => (&name[..at], name[at + 1..].to_lowercase()),
        _ => (name, String::new()),
    }
}

/// A file's kind by its extension; none for one the panel does not add.
pub fn source_kind(name: &str) -> Option<SourceKind> {
    Some(match split(name).1.as_str() {
        "geojson" | "json" => SourceKind::GeoJson,
        "shp" => SourceKind::Shapefile,
        "dxf" => SourceKind::Dxf,
        "ncz" => SourceKind::Ncz,
        "gpx" | "nmea" | "nma" => SourceKind::Gnss,
        "ncn" | "txt" | "csv" | "xyz" | "dat" | "asc" => SourceKind::Coords,
        "kml" | "kmz" => SourceKind::Kml,
        "gpkg" => SourceKind::GeoPackage,
        "fgb" => SourceKind::FlatGeobuf,
        "gml" => SourceKind::Gml,
        "parquet" => SourceKind::GeoParquet,
        "tif" | "tiff" | "vrt" => SourceKind::Raster,
        "las" | "laz" => SourceKind::PointCloud,
        _ => return None,
    })
}

fn natural(names: Vec<String>) -> Vec<String> {
    natural_order(&names)
        .into_iter()
        .map(|i| names[i as usize].clone())
        .collect()
}

/// What a folder with these entries shows.
pub fn listing(entries: &[SourceEntry]) -> Listing {
    let shown: Vec<&SourceEntry> = entries
        .iter()
        .filter(|e| !e.name.starts_with('.'))
        .collect();
    let folders = natural(
        shown
            .iter()
            .filter(|e| e.dir)
            .map(|e| e.name.clone())
            .collect(),
    );
    let names: Vec<String> = shown
        .iter()
        .filter(|e| !e.dir)
        .map(|e| e.name.clone())
        .collect();
    let mut files = Vec::new();
    for name in natural(names.clone()) {
        let Some(kind) = source_kind(&name) else {
            continue;
        };
        let mut parts = vec![name.clone()];
        if kind == SourceKind::Shapefile {
            let stem = split(&name).0.to_owned();
            for p in PARTS {
                if let Some(part) = names.iter().find(|f| {
                    let (s, x) = split(f);
                    s == stem && x == p
                }) {
                    parts.push(part.clone());
                }
            }
        }
        files.push(SourceFile { name, kind, parts });
    }
    Listing { folders, files }
}

// ── Search (docs/adr/0599 §5) ─────────────────────────────────────────

/// A letter as the panel's search takes it: Turkish case (I is ı's, İ is i's), then dotless ı as i, as the panel's
/// words are English ones too (MULTIPOLYGON, EPSG).
fn search_fold(c: char) -> char {
    match kentos_geometry_core::text::edit::fold(c) {
        'ı' => 'i',
        f => f,
    }
}

fn folded(text: &str) -> String {
    text.chars().map(search_fold).collect()
}

/// A query as its folded words; none for an empty one.
pub fn search_words(query: &str) -> Vec<String> {
    query.split_whitespace().map(folded).collect()
}

/// Whether a row with these fields answers the query's words: each word is somewhere in one of them.
pub fn search_matches(words: &[String], fields: &[&str]) -> bool {
    if words.is_empty() {
        return true;
    }
    let fields: Vec<String> = fields.iter().map(|f| folded(f)).collect();
    words
        .iter()
        .all(|w| fields.iter().any(|f| f.contains(w.as_str())))
}

/// Which rows of a tree (in their order, with their depths) show for the query: those that match, and the rows above
/// them.
pub fn search_shown(depths: &[usize], matched: &[bool]) -> Vec<bool> {
    let mut keep = matched.to_vec();
    for (i, &m) in matched.iter().enumerate() {
        if !m {
            continue;
        }
        let mut depth = depths[i];
        for j in (0..i).rev() {
            if depth == 0 {
                break;
            }
            if depths[j] < depth {
                keep[j] = true;
                depth = depths[j];
            }
        }
    }
    keep
}

// ── Recent and favourite sources (docs/adr/0599 §6) ───────────────────

/// The recent sources kept at most.
pub const RECENT_MOST: usize = 12;
/// The favourite sources kept at most.
pub const FAVOURITES_MOST: usize = 50;

/// The recent list after `used`: it goes first, once, at most [`RECENT_MOST`].
pub fn recent_with<T: Clone + PartialEq>(recent: &[T], used: &T) -> Vec<T> {
    let mut out = vec![used.clone()];
    out.extend(recent.iter().filter(|r| *r != used).cloned());
    out.truncate(RECENT_MOST);
    out
}

/// The favourites after starring `source` (it leaves when it is there, else joins the end), and whether it was
/// refused (the list is full).
pub fn starred<T: Clone + PartialEq>(favourites: &[T], source: &T) -> (Vec<T>, bool) {
    if favourites.contains(source) {
        return (
            favourites
                .iter()
                .filter(|f| *f != source)
                .cloned()
                .collect(),
            false,
        );
    }
    if favourites.len() >= FAVOURITES_MOST {
        return (favourites.to_vec(), true);
    }
    let mut out = favourites.to_vec();
    out.push(source.clone());
    (out, false)
}

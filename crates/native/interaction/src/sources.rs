//! Kaynaklar's folder listing (docs/adr/0199 §7; the web's
//! `model/sources.ts`), both platforms held to the shared cases
//! fixtures/sources/v1/cases.json written by
//! scripts/fixtures/source_list_cases.py: a folder shows its folders and the
//! files the panel adds as layers, but those whose name begins with a dot, in
//! the natural order; a file's kind by its extension, case aside; a Shapefile
//! as its .shp with the files of the same name and the extensions .shx, .dbf,
//! .prj and .cpg as its parts, which are not shown on their own. The panel is
//! the desktop's `sources`.

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
        }
    }

    pub const ALL: [SourceKind; 6] = [
        SourceKind::GeoJson,
        SourceKind::Shapefile,
        SourceKind::Dxf,
        SourceKind::Ncz,
        SourceKind::Gnss,
        SourceKind::Coords,
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

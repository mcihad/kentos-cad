//! The geoid models and velocity grids `cad.crs.transform` reads (docs/adr/0631 §3): a GTX file or a GeoTIFF of
//! latitude and longitude read into the geometry core's node grid (`crs::geoid`) and kept there under the file's
//! SHA-256. A GeoTIFF of one band is a geoid model (N, metres); of three bands a velocity grid (east, north and up,
//! millimetres a year, as PROJ's deformation grids are). Its nodes are the pixels' centres, whether the file says
//! PixelIsArea or PixelIsPoint (the formats core gives the area's corners), rows turned to run from the south. The
//! device keeps them beside its NTv2 grids (`$XDG_DATA_HOME/kentos-cad/izgara/`, `<sha>.gtx` or `<sha>.tif` and a
//! `<sha>.json` note saying the kind); the desktop's Izgaralar adds them, every host reads them from there.

use std::path::PathBuf;

use kentos_formats::raster::source::open_bytes;
use kentos_geometry_core::crs::geoid::{self, GeoidError, MAX_BYTES, NodeGrid};
use sha2::{Digest, Sha256};

/// What a grid file is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelKind {
    Geoid,
    Velocity,
}

impl ModelKind {
    pub fn name(self) -> &'static str {
        match self {
            ModelKind::Geoid => "geoid",
            ModelKind::Velocity => "velocity",
        }
    }
}

/// A grid file read: its kind, the extension it is kept with and its nodes.
#[derive(Debug)]
pub struct Model {
    pub kind: ModelKind,
    pub ext: &'static str,
    pub grid: NodeGrid,
}

/// SHA-256 of `bytes`, 64 lower-case hex digits.
pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn tiff(bytes: &[u8]) -> bool {
    bytes.starts_with(b"II*\0")
        || bytes.starts_with(b"MM\0*")
        || bytes.starts_with(b"II+\0")
        || bytes.starts_with(b"MM\0+")
}

/// Reads a GTX file or a GeoTIFF of latitude and longitude, or says why not in the user's words.
pub fn read(bytes: &[u8]) -> Result<Model, String> {
    if bytes.len() > MAX_BYTES {
        return Err("1 GiB'tan büyük".into());
    }
    if !tiff(bytes) {
        return geoid::read_gtx(bytes)
            .map(|grid| Model {
                kind: ModelKind::Geoid,
                ext: "gtx",
                grid,
            })
            .map_err(|e| match e {
                GeoidError::NotGtx => "GTX ya da GeoTIFF dosyası değil".into(),
                GeoidError::Truncated => "dosya düğümleri bitmeden kesiliyor".into(),
                _ => "ızgaranın kapsamı ya da aralığı geçersiz".into(),
            });
    }
    let mut r =
        open_bytes(bytes, None, MAX_BYTES).map_err(|e| format!("GeoTIFF okunamadı: {}", e.0))?;
    if !r.info.geographic {
        return Err("GeoTIFF enlem ve boylamda değil (geoit ve hız ızgarası coğrafi olur)".into());
    }
    let Some([x0, a, b, y0, c, d]) = r.info.affine else {
        return Err("GeoTIFF'in yeri yok".into());
    };
    if b != 0.0 || c != 0.0 || !(a > 0.0 && d < 0.0) {
        return Err("GeoTIFF dönük ya da ters: düğümleri enlem ve boylam ızgarası değil".into());
    }
    let bands = r.info.bands as usize;
    let kind = match bands {
        1 => ModelKind::Geoid,
        3 => ModelKind::Velocity,
        _ => {
            return Err(format!(
                "{bands} bantlı GeoTIFF: geoit 1, hız ızgarası 3 bant (doğu, kuzey, yukarı) olur"
            ));
        }
    };
    let (w, h) = (r.levels[0].width, r.levels[0].height);
    for need in r.needs(0, 0, 0, w, h) {
        let (from, to) = (need.offset as usize, (need.offset + need.len) as usize);
        let Some(block) = bytes.get(from..to) else {
            return Err("GeoTIFF bloklarından biri dosyanın dışında".into());
        };
        r.put_block(&need, block)
            .map_err(|e| format!("GeoTIFF bloğu okunamadı: {}", e.0))?;
    }
    let region = r
        .region(0, 0, 0, w, h)
        .ok_or_else(|| "GeoTIFF'in pikselleri okunamadı".to_owned())?;
    let (cols, rows) = (w as usize, h as usize);
    let mut values = Vec::with_capacity(cols * rows * bands);
    for row in 0..rows {
        // Rows from the south: the file's last row first.
        let j = (rows - 1 - row) as u32;
        for i in 0..w {
            for band in 0..bands as u32 {
                values.push(region.at(i, j, band) as f32);
            }
        }
    }
    let west = x0 + a / 2.0;
    let north = y0 + d / 2.0;
    let south = north + d * (rows as f64 - 1.0);
    let grid = NodeGrid::new(
        west,
        south,
        a,
        -d,
        cols,
        rows,
        bands,
        values,
        r.nodata.map(|v| v as f32),
    )
    .map_err(|e| match e {
        GeoidError::Truncated => "GeoTIFF'in pikselleri eksik".to_owned(),
        _ => "GeoTIFF'in kapsamı enlem ve boylam ızgarası değil".to_owned(),
    })?;
    Ok(Model {
        kind,
        ext: "tif",
        grid,
    })
}

/// The device's grid folder: `KENTOS_GRID_DIR` when it is set (a server's, the tests'), else
/// `$XDG_DATA_HOME/kentos-cad/izgara`, else `~/.local/share/kentos-cad/izgara`.
pub fn device_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("KENTOS_GRID_DIR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(base.join("kentos-cad").join("izgara"))
}

/// Why a named grid is not at hand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Missing {
    /// This device does not keep it.
    NotHere,
    /// The kept file is no longer the one its name says.
    Changed,
    /// It is kept but is not the kind asked for, or is not read.
    Unread(String),
}

/// The grid `id` of `kind`: in the core already, or read from `dir` (the device's folder when none) into it.
pub fn load(id: &str, kind: ModelKind, dir: Option<&std::path::Path>) -> Result<(), Missing> {
    let wrong = || {
        Missing::Unread(match kind {
            ModelKind::Geoid => "bir hız ızgarası, geoit modeli değil".into(),
            ModelKind::Velocity => "bir geoit modeli, hız ızgarası değil".into(),
        })
    };
    if let Some(g) = geoid::get(id) {
        return if (g.bands == 1) == (kind == ModelKind::Geoid) {
            Ok(())
        } else {
            Err(wrong())
        };
    }
    let dir = match dir {
        Some(d) => d.to_path_buf(),
        None => device_dir().ok_or(Missing::NotHere)?,
    };
    let bytes = ["gtx", "tif"]
        .iter()
        .find_map(|ext| std::fs::read(dir.join(format!("{id}.{ext}"))).ok())
        .ok_or(Missing::NotHere)?;
    if sha256(&bytes) != id {
        return Err(Missing::Changed);
    }
    let model = read(&bytes).map_err(Missing::Unread)?;
    if model.kind != kind {
        return Err(wrong());
    }
    geoid::register(id, model.grid);
    Ok(())
}

/// Whether `s` is a SHA-256 as 64 lower-case hex digits.
pub fn is_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The import's word on the heights' system a file names (docs/adr/0631 §4): the heights are taken as they are,
/// said against the project's own (`vertical_crs`).
pub fn vertical_said(
    file: &str,
    srid: Option<u32>,
    name: &str,
    settings: &kentos_commands::ProjectSettings,
) -> kentos_commands::CommandWarning {
    let named = match srid {
        Some(s) => format!("{name} (EPSG:{s})"),
        None => name.to_owned(),
    };
    let project = settings.heights.as_ref();
    let same = srid.is_some() && project.and_then(|h| h.vertical_srid) == srid;
    let message = if same {
        format!(
            "{file} kotların {named} olduğunu söylüyor; projenin kotları da öyle: kotlar olduğu gibi alındı."
        )
    } else {
        let theirs = project.map_or_else(|| "söylenmemiş".to_owned(), |h| h.title());
        format!(
            "{file} kotların {named} olduğunu söylüyor; projenin kotları {theirs}: kotlar olduğu gibi alındı, çevrilmedi. Çevirmek için Koordinat dönüştür'ü (cad.crs.transform) geoit modeliyle kullanın."
        )
    };
    kentos_commands::CommandWarning {
        code: crate::codes::VERTICAL_CRS.into(),
        message,
        path: Some("path".into()),
    }
}

//! A raster's file read the same way by the desktop and the headless server (docs/adr/0573): a run of a file's bytes, a
//! JPEG's size and pixels (zune-jpeg; a JPEG-compressed TIFF's tiles too), a picture (PNG or JPEG) read whole, and a
//! virtual mosaic (GDAL VRT, docs/adr/0517 §2) whose files are opened one by one and whose blocks are read where they are.

use std::fs::File;
use std::path::{Path, PathBuf};

use kentos_formats::raster::source::{BlockNeed, Reader};
use kentos_formats::raster::{Samples, png, vrt};

/// `len` bytes of `file` from `offset`; why not.
pub fn read_at(file: &File, offset: u64, len: u64) -> Result<Vec<u8>, String> {
    let mut out = vec![0u8; usize::try_from(len).map_err(|_| "Blok çok büyük.".to_owned())?];
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.read_exact_at(&mut out, offset)
            .map_err(|e| format!("Raster dosyası okunamadı: {e}."))?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut done = 0;
        while done < out.len() {
            let n = file
                .seek_read(&mut out[done..], offset + done as u64)
                .map_err(|e| format!("Raster dosyası okunamadı: {e}."))?;
            if n == 0 {
                return Err("Raster dosyası beklenenden kısa.".into());
            }
            done += n;
        }
    }
    Ok(out)
}

pub use kentos_native_application::raster_files::{is_jpeg, jpeg_pixels, jpeg_size};

/// A raster linked to a GeoPackage tile table opened for an analysis (docs/adr/0627 §3) from the table's file at
/// `path`: its reader, its blocks the table's tiles framed with their scale and offset (the reader decodes them).
pub fn table_open<'f>(
    path: &Path,
    raster: &kentos_commands::RasterFields,
) -> Result<crate::files::RasterOpen<'f>, String> {
    let (reader, tiles) = kentos_native_application::gpkg_tiles::open_linked(
        path,
        raster,
        crate::files::RASTER_READER_BUDGET,
    )?;
    let blocks = reader
        .tile_blocks()
        .ok_or("Raster bir karo tablosu değil.")?;
    Ok(crate::files::RasterOpen {
        reader,
        block: Box::new(move |need| {
            let at = blocks
                .at(need)
                .ok_or_else(|| "Rasterin bu karosu okunamadı.".to_owned())?;
            tiles.framed_at(at.zoom, at.column, at.row)
        }),
        jpeg: Box::new(jpeg_pixels),
    })
}

/// A picture read whole (PNG or JPEG) as a raster: its pixels in memory, no blocks read later; why not.
pub fn picture_reader(data: &[u8], budget: usize) -> Result<Reader, String> {
    if png::sniff(data) {
        let p = png::decode(data).map_err(|e| e.0)?;
        return Reader::image(
            p.width, p.height, p.bands, p.samples, p.nodata, None, budget,
        )
        .map_err(|e| e.0);
    }
    if is_jpeg(data) {
        let (w, h) = jpeg_size(data)?;
        let (pixels, comps) = jpeg_pixels(data)?;
        return Reader::image(w, h, comps, Samples::U8(pixels), None, None, budget)
            .map_err(|e| e.0);
    }
    Err("Dosya PNG ya da JPEG değil.".into())
}

/// A virtual mosaic's reader from its VRT text at `path` (docs/adr/0517 §2) and its files in order: `open` opens each
/// file as a raster of its own with the budget given; a mosaic among them is refused.
pub fn mosaic_reader(
    path: &Path,
    text: &str,
    budget: usize,
    mut open: impl FnMut(&Path, usize) -> Result<Reader, String>,
) -> Result<(Reader, Vec<PathBuf>), String> {
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    vrt::open_mosaic(text, &folder, budget, |p, each| {
        let head = File::open(p).ok().and_then(|f| {
            let n = f.metadata().ok()?.len().min(64);
            read_at(&f, 0, n).ok()
        });
        if head.as_deref().is_some_and(vrt::sniff) {
            return Err("mozaiğin içinde mozaik açılmaz".into());
        }
        open(p, each)
    })
    .map_err(|e| e.0)
}

/// A mosaic's VRT text read from its file; why not (more than `vrt::MAX_TEXT` bytes, not UTF-8).
pub fn mosaic_text(bytes: Vec<u8>) -> Result<String, String> {
    if bytes.len() > vrt::MAX_TEXT {
        return Err("VRT dosyası 16 MB'tan büyük.".into());
    }
    String::from_utf8(bytes).map_err(|_| "VRT dosyası UTF-8 değil.".to_owned())
}

/// A block of a mosaic's file (`need.member`, counted from 1); why not.
pub fn read_member(members: &[PathBuf], need: &BlockNeed) -> Result<Vec<u8>, String> {
    let path = need
        .member
        .checked_sub(1)
        .and_then(|m| members.get(m as usize))
        .ok_or_else(|| "Mozaiğin böyle bir dosyası yok.".to_owned())?;
    if need.file != 0 {
        return Err("Mozaiğin dosyalarının önizleme piramidi okunmaz.".into());
    }
    let file = File::open(path).map_err(|e| format!("“{}” açılamadı: {e}.", path.display()))?;
    read_at(&file, need.offset, need.len)
}

//! `cad.rasters.mosaic` v1 (docs/adr/0517; plan/gis.md `CBS-28`): GeoTIFF files made one virtual mosaic, written as
//! GDAL's VRT (`kentos_formats::raster::vrt`). Each file's header is read (its directories, never its pixels); the
//! mosaic's extent is the files' union and its pixel the finest, coarsest or mean of theirs, by gdalbuildvrt's rule
//! (`(int)(0.5 + extent / pixel)` pixels, each file's DstRect its offset and size in mosaic pixels); the files are
//! written bottom first, so that the one on top (by the order given, the finer pixel or the newer DateTime) is last,
//! as a VRT lays a later source over the earlier. The file is written beside its place, read back and checked, then
//! moved there. With `layer` the mosaic is added as a raster linked to the file, as one undo step “Sanal mozaik”.
//! The shared cases are `fixtures/commands/v1/cad.rasters.mosaic.json` (`scripts/fixtures/raster_mosaic_cases.py`,
//! from GDAL's own gdalbuildvrt).

use std::fs::File;
use std::path::{Component, Path, PathBuf};

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, MOSAIC_MOST_SOURCES, MosaicOrder, MosaicResolution,
    MosaicSource, RasterEntity, RasterFields, RasterSample, RastersMosaic, RastersMosaicked, codes,
};
use kentos_domain::Document;
use kentos_formats::raster::source::Reader;
use kentos_formats::raster::tiff::{self, Tiff};
use kentos_formats::raster::vrt::{self, Vrt, VrtBand, VrtSource};
use kentos_formats::raster::{ByteStore, Step};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::links::find;

/// The most bytes a file's header may take (its directories and offsets).
const HEADER_MOST: u64 = 64 * 1024 * 1024;
/// The largest side a mosaic may have, in pixels.
const SIDE_MOST: f64 = 2_000_000_000.0;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

fn file_name(path: &str) -> String {
    Path::new(path).file_name().map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// A file of the mosaic as its header says.
struct Source {
    /// Its place in `sources`.
    given: usize,
    read: PathBuf,
    width: u32,
    height: u32,
    bands: u32,
    sample: RasterSample,
    epsg: Option<u32>,
    nodata: Option<f64>,
    affine: [f64; 6],
    date: Option<String>,
    rgb: bool,
}

/// `len` bytes of `file` from `offset`.
fn read_at(file: &File, offset: u64, len: u64) -> std::io::Result<Vec<u8>> {
    let mut out = vec![0u8; usize::try_from(len).map_err(|_| std::io::Error::other("çok büyük"))?];
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.read_exact_at(&mut out, offset)?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut done = 0;
        while done < out.len() {
            let n = file.seek_read(&mut out[done..], offset + done as u64)?;
            if n == 0 {
                return Err(std::io::Error::other("dosya kısa"));
            }
            done += n;
        }
    }
    Ok(out)
}

/// A TIFF's directories read from its file; why not.
fn header(path: &Path) -> Result<Tiff, String> {
    let file = File::open(path).map_err(|e| format!("açılamadı: {e}"))?;
    let size = file.metadata().map_err(|e| format!("okunamadı: {e}"))?.len();
    let head = read_at(&file, 0, size.min(16)).map_err(|e| format!("okunamadı: {e}"))?;
    if !tiff::sniff(&head) {
        return Err("GeoTIFF değil".into());
    }
    let mut store = ByteStore::new();
    let mut taken = 0u64;
    loop {
        match tiff::parse(&store, size).map_err(|e| e.0)? {
            Step::Done(t) => return Ok(t),
            Step::Need(n) => {
                taken += n.len;
                if taken > HEADER_MOST {
                    return Err("TIFF'in başlığı çok büyük".into());
                }
                store.put(n.offset, read_at(&file, n.offset, n.len).map_err(|e| format!("okunamadı: {e}"))?);
            }
        }
    }
}

/// A TIFF DateTime (`YYYY:MM:DD HH:MM:SS`) as `YYYY-MM-DDTHH:MM:SS`; none when it is not one.
pub fn iso_date(text: &str) -> Option<String> {
    let t = text.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    let b = t.as_bytes();
    let digits = |r: std::ops::Range<usize>| b.get(r).is_some_and(|s| s.iter().all(u8::is_ascii_digit));
    let ok = b.len() == 19
        && digits(0..4)
        && b[4] == b':'
        && digits(5..7)
        && b[7] == b':'
        && digits(8..10)
        && b[10] == b' '
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16)
        && b[16] == b':'
        && digits(17..19);
    ok.then(|| format!("{}-{}-{}T{}", &t[0..4], &t[5..7], &t[8..10], &t[11..19]))
}

/// The path without its `.` and with each `..` taking the part before it (no file system asked).
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// How the VRT names a file: from its folder when the file is under it, else absolute.
fn vrt_name(file: &Path, folder: &Path) -> (String, bool) {
    let (f, d) = (lexical(file), lexical(folder));
    match f.strip_prefix(&d) {
        Ok(rest) if !rest.as_os_str().is_empty() => (
            rest.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"),
            true,
        ),
        _ => (file.display().to_string(), false),
    }
}

/// A source's header read and checked on its own; why not.
fn source(doc_drawing: Option<&str>, text: &str, given: usize) -> Result<Source, Stop> {
    let at = format!("sources[{given}]");
    let found = find(text.trim(), None, doc_drawing, &[]);
    let Some(read) = found.at else {
        return Err(refuse(
            codes::FILE_NOT_FOUND,
            format!("{} bulunamadı. Bakılan yerler: {}.", file_name(text), found.tried.join(", ")),
            &at,
        ));
    };
    let name = file_name(text);
    let t = header(&read).map_err(|why| {
        refuse(codes::IMPORT_UNREADABLE, format!("{name} okunamadı ({why}); mozaiğe yalnız GeoTIFF konur."), &at)
    })?;
    let reader = Reader::tiff(&t, None, 0)
        .map_err(|e| refuse(codes::IMPORT_UNREADABLE, format!("{name} okunamadı: {}", e.0), &at))?;
    let info = &reader.info;
    let affine = match (info.affine, info.placed_by.as_str()) {
        (Some(a), "geotiff") => a,
        _ => {
            return Err(refuse(
                codes::MOSAIC_UNPLACED,
                format!("{name} dosyasının yerini GeoTIFF etiketleri söylemiyor; önce Raster oturt ile yerine koyup GeoTIFF olarak yazın."),
                &at,
            ));
        }
    };
    let [_, a, b, _, c, d] = affine;
    let north_up = b == 0.0 && c == 0.0 && a > 0.0 && d < 0.0;
    if !north_up {
        return Err(refuse(
            codes::MOSAIC_ROTATED,
            format!("{name} dönük ya da kuzeyi yukarıda değil; mozaik yalnız kuzeyi yukarıda dosyaları yan yana koyar."),
            &at,
        ));
    }
    if reader.palette.is_some() {
        return Err(refuse(
            codes::MOSAIC_MISMATCH,
            format!("{name} paletli; paletli raster mozaiğe konmaz (önce RGB'ye çevirin)."),
            &at,
        ));
    }
    let main = t.ifds.iter().find(|d| d.subfile & 0b101 == 0);
    Ok(Source {
        given,
        read,
        width: info.width,
        height: info.height,
        bands: info.bands,
        sample: info.sample,
        epsg: info.epsg,
        nodata: info.nodata,
        affine,
        date: main.and_then(|d| d.date_time.as_deref()).and_then(iso_date),
        rgb: info.color == "rgb" || info.color == "ycbcr",
    })
}

/// What the command writes: the VRT, where, its output and the raster it adds.
struct Planned {
    vrt: Vrt,
    path: PathBuf,
    out: RastersMosaicked,
    raster: Option<Entity>,
    warnings: Vec<CommandWarning>,
}

fn sample_name(s: RasterSample) -> &'static str {
    vrt::type_name(s)
}

fn check(doc: &Document, input: &RastersMosaic) -> Result<Planned, Stop> {
    let drawing = input.drawing.as_deref();
    if input.sources.is_empty() {
        return Err(refuse(codes::MOSAIC_EMPTY, "Mozaiğe konacak GeoTIFF dosyası verilmedi.".into(), "sources"));
    }
    if input.sources.len() > MOSAIC_MOST_SOURCES {
        return Err(refuse(
            codes::MOSAIC_TOO_LARGE,
            format!("{} dosya verildi; mozaik en çok {MOSAIC_MOST_SOURCES} dosya alır.", input.sources.len()),
            "sources",
        ));
    }
    // The mosaic's path: a .vrt in a folder that is there.
    let text = input.path.trim();
    if text.is_empty() {
        return Err(refuse(codes::INVALID_FILE_PATH, "Mozaiğin dosya yolu boş; bir .vrt dosyasının yolunu verin.".into(), "path"));
    }
    if !text.to_ascii_lowercase().ends_with(".vrt") {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("“{}” bir .vrt dosyası değil; mozaik GDAL'ın VRT dosyası olarak yazılır.", file_name(text)),
            "path",
        ));
    }
    let path = if Path::new(text).is_absolute() {
        PathBuf::from(text)
    } else if let Some(folder) = drawing.and_then(|d| Path::new(d).parent()) {
        folder.join(text)
    } else {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("“{text}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin."),
            "path",
        ));
    };
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    if !folder.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("“{}” klasörü yok; mozaik var olan bir klasöre yazılır.", folder.display()),
            "path",
        ));
    }
    for (i, s) in input.sources.iter().enumerate() {
        let t = s.trim();
        if t.is_empty() {
            return Err(refuse(codes::INVALID_FILE_PATH, format!("{}. dosyanın yolu boş.", i + 1), &format!("sources[{i}]")));
        }
        if Path::new(t).is_relative() && drawing.is_none() {
            return Err(refuse(
                codes::INVALID_FILE_PATH,
                format!("“{t}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin."),
                &format!("sources[{i}]"),
            ));
        }
    }
    if let Some(p) = input.pixel_size
        && !(p.is_finite() && p > 0.0)
    {
        return Err(refuse(codes::INVALID_PIXEL_SIZE, "Piksel boyu sıfırdan büyük, sonlu bir sayı olmalı.".into(), "pixelSize"));
    }
    // Each file's header, in the order given; every file like the first.
    let mut files: Vec<Source> = Vec::with_capacity(input.sources.len());
    for (i, s) in input.sources.iter().enumerate() {
        let f = source(drawing, s, i)?;
        if let Some(first) = files.first() {
            let name = file_name(s);
            let first_name = file_name(&input.sources[0]);
            let why = if f.epsg != first.epsg {
                let sys = |e: Option<u32>| e.map_or_else(|| "sistemsiz".to_owned(), |e| format!("EPSG:{e}"));
                Some(format!("{name} {}, {first_name} {}", sys(f.epsg), sys(first.epsg)))
            } else if f.bands != first.bands {
                Some(format!("{name} {} bantlı, {first_name} {} bantlı", f.bands, first.bands))
            } else if f.sample != first.sample {
                Some(format!("{name} {}, {first_name} {}", sample_name(f.sample), sample_name(first.sample)))
            } else {
                None
            };
            if let Some(why) = why {
                return Err(refuse(
                    codes::MOSAIC_MISMATCH,
                    format!("Dosyalar mozaiğe birlikte konmaz: {why}. Hepsi aynı sistemde, bant sayısında ve veri türünde olmalı."),
                    &format!("sources[{i}]"),
                ));
            }
        }
        files.push(f);
    }
    // The extent (the union) and the pixel, by gdalbuildvrt's rule; sums in the order given.
    let (mut min_x, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut max_x, mut min_y) = (f64::NEG_INFINITY, f64::INFINITY);
    for f in &files {
        let [x0, a, _, y0, _, d] = f.affine;
        min_x = min_x.min(x0);
        max_y = max_y.max(y0);
        max_x = max_x.max(x0 + f64::from(f.width) * a);
        min_y = min_y.min(y0 + f64::from(f.height) * d);
    }
    let (we, ns) = match (input.pixel_size, input.resolution) {
        (Some(p), _) => (p, p),
        (None, MosaicResolution::Highest) => files
            .iter()
            .fold((f64::INFINITY, f64::INFINITY), |(w, n), f| (w.min(f.affine[1]), n.min(-f.affine[5]))),
        (None, MosaicResolution::Lowest) => files.iter().fold((0.0_f64, 0.0_f64), |(w, n), f| (w.max(f.affine[1]), n.max(-f.affine[5]))),
        (None, MosaicResolution::Average) => {
            let (mut w, mut n) = (0.0, 0.0);
            for f in &files {
                w += f.affine[1];
                n += -f.affine[5];
            }
            let k = files.len() as f64;
            (w / k, n / k)
        }
    };
    let (wx, wy) = (0.5 + (max_x - min_x) / we, 0.5 + (max_y - min_y) / ns);
    if !(wx < SIDE_MOST && wy < SIDE_MOST) {
        return Err(refuse(
            codes::MOSAIC_TOO_LARGE,
            format!("Mozaik {wx:.0} × {wy:.0} piksel olurdu; bir kenarı en çok 2 milyar piksel olabilir. Daha büyük bir piksel boyu verin."),
            if input.pixel_size.is_some() { "pixelSize" } else { "resolution" },
        ));
    }
    let (width, height) = ((wx as u32).max(1), (wy as u32).max(1));
    if path.exists() && input.overwrite != Some(true) {
        return Err(refuse(
            codes::FILE_EXISTS,
            format!("{} var; üzerine yazmak için overwrite verin.", path.display()),
            "path",
        ));
    }
    // The order: rank 1 on top.
    let mut ranked: Vec<&Source> = files.iter().collect();
    match input.order {
        MosaicOrder::Given => {}
        MosaicOrder::Finest => ranked.sort_by(|p, q| {
            let area = |f: &Source| f.affine[1] * -f.affine[5];
            area(p).total_cmp(&area(q)).then(p.given.cmp(&q.given))
        }),
        MosaicOrder::Newest => ranked.sort_by(|p, q| match (&p.date, &q.date) {
            (Some(a), Some(b)) => b.cmp(a).then(p.given.cmp(&q.given)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => p.given.cmp(&q.given),
        }),
    }
    let bands = files[0].bands;
    let sample = files[0].sample;
    let srid = files[0].epsg.unwrap_or(0);
    let nodata = input.nodata.or_else(|| files.iter().find_map(|f| f.nodata));
    let mut sources = Vec::with_capacity(ranked.len());
    let mut vrt_sources = Vec::with_capacity(ranked.len());
    for f in &ranked {
        // gdalbuildvrt's windows: the file's pixels scaled to the mosaic's, cut where the mosaic's rounded size
        // ends; a file left with nothing on the mosaic is left out.
        let [x0, a, _, y0, _, d] = f.affine;
        let (rx, ry) = (a / we, -d / ns);
        let (mut sw, mut sh) = (f64::from(f.width), f64::from(f.height));
        let (dx, dy) = ((x0 - min_x) / we, (max_y - y0) / ns);
        let (mut dw, mut dh) = (sw * rx, sh * ry);
        if dx + dw > f64::from(width) {
            dw = f64::from(width) - dx;
            sw = dw / rx;
        }
        if dy + dh > f64::from(height) {
            dh = f64::from(height) - dy;
            sh = dh / ry;
        }
        if !(sw > 0.0 && sh > 0.0 && dw > 0.0 && dh > 0.0) {
            continue;
        }
        let (src, dst) = ([0.0, 0.0, sw, sh], [dx, dy, dw, dh]);
        let (file, relative) = vrt_name(&f.read, &folder);
        sources.push(MosaicSource {
            file: file.clone(),
            read: f.read.display().to_string(),
            rank: sources.len() as u32 + 1,
            width: f.width,
            height: f.height,
            pixel: [a, -d],
            date: f.date.clone(),
            src,
            dst,
            nodata: f.nodata,
        });
        vrt_sources.push(VrtSource { file, relative, band: 1, src, dst, nodata: f.nodata });
    }
    // The VRT bottom first: the source on top is the last.
    vrt_sources.reverse();
    let rgb = files[0].rgb && (bands == 3 || bands == 4);
    let vrt_bands = (0..bands)
        .map(|b| VrtBand {
            sample,
            nodata,
            color: Some(
                match (rgb, b) {
                    (true, 0) => "Red",
                    (true, 1) => "Green",
                    (true, 2) => "Blue",
                    (true, 3) => "Alpha",
                    (false, 0) => "Gray",
                    _ => "Undefined",
                }
                .to_owned(),
            ),
            sources: vrt_sources.iter().map(|s| VrtSource { band: b + 1, ..s.clone() }).collect(),
        })
        .collect();
    let affine = [min_x, we, 0.0, max_y, 0.0, -ns];
    let v = Vrt {
        width,
        height,
        affine: Some(affine),
        srs: (srid != 0).then(|| format!("EPSG:{srid}")),
        epsg: (srid != 0).then_some(srid),
        bands: vrt_bands,
    };
    let bytes = vrt::write(&v).len() as u64;
    // Warnings: files read at another pixel, files without a date in the date's order.
    let mut warnings = Vec::new();
    let resampled: Vec<String> = files
        .iter()
        .filter(|f| f.affine[1] != we || -f.affine[5] != ns)
        .map(|f| file_name(&input.sources[f.given]))
        .collect();
    if !resampled.is_empty() {
        warnings.push(CommandWarning {
            code: codes::MOSAIC_RESAMPLED.into(),
            message: format!(
                "{} {} pikseli mozaiğinkinden ({} × {}) başka; en yakın pikselden okunur.",
                resampled.join(", "),
                if resampled.len() == 1 { "dosyasının" } else { "dosyalarının" },
                kentos_formats::num::plain(we),
                kentos_formats::num::plain(ns)
            ),
            path: Some("sources".into()),
        });
    }
    if input.order == MosaicOrder::Newest {
        let undated: Vec<String> = files.iter().filter(|f| f.date.is_none()).map(|f| file_name(&input.sources[f.given])).collect();
        if !undated.is_empty() {
            warnings.push(CommandWarning {
                code: codes::MOSAIC_UNDATED.into(),
                message: format!(
                    "{} {} tarihi (TIFF DateTime) yok; en alta {}.",
                    undated.join(", "),
                    if undated.len() == 1 { "dosyasının" } else { "dosyalarının" },
                    if undated.len() == 1 { "kondu" } else { "kondular" }
                ),
                path: Some("order".into()),
            });
        }
    }
    // The raster added, on a layer that takes it, in the project's system.
    let raster = match &input.layer {
        None => None,
        Some(layer) => {
            warnings.extend(checks::layer_at(doc, layer, "layer")?);
            let project = doc.settings().srid;
            if srid != 0 && srid != project {
                return Err(refuse(
                    codes::CRS_MISMATCH,
                    format!(
                        "Mozaiğin sistemi EPSG:{srid}, projeninki {}; raster çevrilmeden eklenmez. Dosyayı yazmak için layer'ı vermeyin.",
                        if project == 0 { "yerel".to_owned() } else { format!("EPSG:{project}") }
                    ),
                    "layer",
                ));
            }
            Some(Entity::Raster(RasterEntity {
                base: EntityBase {
                    look: Default::default(),
                    id: 0,
                    layer_id: layer.clone(),
                    color: None,
                    attrs: Default::default(),
                    label: None,
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                },
                raster: RasterFields {
                    affine,
                    width,
                    height,
                    bands,
                    sample,
                    asset: None,
                    file: Some(text.to_owned()),
                    url: None,
                    srid,
                    style: kentos_formats::raster::style::default_style(bands, sample, false),
                    opacity: None,
                    dataset: None,
                    georef: None,
                },
            }))
        }
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Planned {
        out: RastersMosaicked {
            path: path.display().to_string(),
            width,
            height,
            bands,
            sample,
            affine,
            srid,
            nodata,
            sources,
            bytes,
            uid: None,
            revision: doc.revision().to_string(),
        },
        vrt: v,
        path,
        raster,
        warnings,
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &RastersMosaic) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: (), warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &RastersMosaic) -> CommandResult<RastersMosaicked> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: p.out, warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again, writes the VRT (beside its place, read back, then moved there) and, with `layer`, adds the
/// raster as one undo step “Sanal mozaik”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RastersMosaic) -> CommandResult<RastersMosaicked> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_RASTERS_MOSAIC,
        kentos_commands::CAD_RASTERS_MOSAIC_VERSION,
        input,
        write_execute,
    )
}

/// The VRT's text written at `path` whole or not at all: written beside it, read back and parsed as the reader will,
/// then moved over it.
fn write_file(path: &Path, v: &Vrt) -> Result<(), String> {
    let text = vrt::write(v);
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".kentos-yaziliyor");
    let tmp = PathBuf::from(tmp);
    let done = (|| -> Result<(), String> {
        std::fs::write(&tmp, text.as_bytes()).map_err(|e| format!("{} yazılamadı: {e}", tmp.display()))?;
        let back = std::fs::read_to_string(&tmp).map_err(|e| format!("{} geri okunamadı: {e}", tmp.display()))?;
        let read = vrt::parse(&back).map_err(|e| e.0)?;
        vrt::members(&read).map_err(|e| e.0)?;
        if read != *v {
            return Err("yazılan VRT geri okununca aynı çıkmadı".into());
        }
        std::fs::rename(&tmp, path).map_err(|e| format!("{} yerine konamadı: {e}", path.display()))
    })();
    if done.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    done
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: RastersMosaic) -> CommandResult<RastersMosaicked> {
    let Planned { vrt: v, path, mut out, raster, warnings } = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    if let Err(why) = write_file(&path, &v) {
        return Stop::failed(checks::error(
            codes::INVALID_FILE_PATH,
            format!("Mozaik yazılamadı: {why}. Hiçbir şey değişmedi."),
            Some("path".into()),
        ))
        .into();
    }
    if let Some(entity) = raster {
        let added = cx.doc.transact("Sanal mozaik", |doc| checks::add(doc, entity));
        match added {
            Ok((_, uid, revision)) => {
                out.uid = Some(uid);
                out.revision = revision;
            }
            Err(stop) => return stop.into(),
        }
    }
    CommandResult::Completed { output: out, warnings }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiff_dates_are_read_as_iso() {
        assert_eq!(iso_date("2023:06:01 10:00:00").as_deref(), Some("2023-06-01T10:00:00"));
        assert_eq!(iso_date("2023:06:01 10:00:00\0").as_deref(), Some("2023-06-01T10:00:00"));
        assert_eq!(iso_date("2023-06-01"), None);
        assert_eq!(iso_date(""), None);
    }

    #[test]
    fn files_under_the_vrts_folder_are_named_from_it() {
        assert_eq!(vrt_name(Path::new("/a/b/c/x.tif"), Path::new("/a/b")), ("c/x.tif".into(), true));
        assert_eq!(vrt_name(Path::new("/a/b/../d/x.tif"), Path::new("/a/d")), ("x.tif".into(), true));
        assert_eq!(vrt_name(Path::new("/e/x.tif"), Path::new("/a/b")), ("/e/x.tif".into(), false));
    }
}

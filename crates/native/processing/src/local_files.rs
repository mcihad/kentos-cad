//! The headless server's files (docs/adr/0515): what a raster tool reads and writes where there is no window — a
//! raster's linked file or embedded bytes, read where they are, and a result written beside its place, then renamed
//! (a COG laid out again). Python's `cad.processing.run` and MCP run the raster tools through these, as the desktop
//! runs them through its own; the point clouds' files (LAS, LAZ, COPC, text; a COPC index built from a result) and a
//! NetCDF file's cube for the multidimensional tools too (docs/adr/0572, `cloud_files`), a virtual mosaic (VRT) and a PNG
//! or JPEG picture or JPEG-compressed TIFF (docs/adr/0573, `raster_files`). An address (HTTP ranges) stays the
//! desktop's: such a run is refused, saying why.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use kentos_commands::{CloudFormat, CloudSource, RasterFields};
use kentos_domain::Document;
use kentos_formats::raster::source::Reader;
use kentos_formats::raster::{ByteStore, Step, tiff, vrt};

use crate::cloud_files::{ReadAt, cloud_read, copc_file, netcdf_reader, netcdf_slice, text_plan};
use crate::files::{
    Beside, CloudRead, CubeOpen, Files, RASTER_READER_BUDGET, RasterOpen, Sink, cog_file,
    with_extension,
};
use crate::raster_files::{
    is_jpeg, jpeg_pixels, mosaic_reader, mosaic_text, picture_reader, read_member,
};
use crate::types::Feedback;

/// At most this many bytes of a TIFF's directories are read to open it.
const HEADER_MOST: u64 = 256 << 20;
/// At most this big a PNG or JPEG is read whole.
const IMAGE_MOST: u64 = 512 << 20;

/// A raster's bytes: a file read by ranges, or an embedded file's bytes.
enum Bytes {
    File(Mutex<File>, u64),
    Memory(Arc<Vec<u8>>),
}

impl ReadAt for Bytes {
    fn size(&self) -> u64 {
        Bytes::size(self)
    }

    fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        Bytes::read(self, offset, len)
    }
}

/// What a file is of, in the refusals' words: a raster's or a point cloud's.
#[derive(Clone, Copy)]
enum Of {
    Raster,
    Cloud,
}

impl Of {
    fn noun(self) -> &'static str {
        match self {
            Of::Raster => "raster",
            Of::Cloud => "nokta bulutu",
        }
    }

    fn whose(self) -> &'static str {
        match self {
            Of::Raster => "Rasterin",
            Of::Cloud => "Bulutun",
        }
    }
}

impl Bytes {
    fn size(&self) -> u64 {
        match self {
            Bytes::File(_, n) => *n,
            Bytes::Memory(b) => b.len() as u64,
        }
    }

    fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        let size = self.size();
        let len = len.min(size.saturating_sub(offset));
        match self {
            Bytes::Memory(b) => Ok(b[offset as usize..(offset + len) as usize].to_vec()),
            Bytes::File(f, _) => {
                let mut f = f.lock().unwrap_or_else(PoisonError::into_inner);
                let mut out = vec![0u8; len as usize];
                f.seek(SeekFrom::Start(offset))
                    .and_then(|_| f.read_exact(&mut out))
                    .map_err(|e| format!("Dosya okunamadı: {e}."))?;
                Ok(out)
            }
        }
    }
}

/// The files of a drawing with no window: its folder (relative paths are taken from it) and its library's
/// embedded rasters.
pub struct LocalFiles {
    folder: Option<PathBuf>,
    /// The project's search paths (docs/adr/0349 §1): a raster not at its path is looked for there.
    search: Vec<String>,
    assets: HashMap<String, String>,
    /// The text clouds' plans, each file scanned once (its key `file:<path>` or `asset:<id>`).
    plans: Mutex<HashMap<String, kentos_pointcloud::text::Plan>>,
}

impl LocalFiles {
    /// The files of `doc`, whose file is in `folder` (none: relative paths from the working directory).
    pub fn of(doc: &Document, folder: Option<PathBuf>) -> LocalFiles {
        let mut assets = HashMap::new();
        for it in &doc.styles().items {
            let id = it.get("id").and_then(serde_json::Value::as_str);
            let asset = it.get("kind").and_then(serde_json::Value::as_str) == Some("asset");
            if let (true, Some(id), Some(data)) = (
                asset,
                id,
                it.get("data").and_then(serde_json::Value::as_str),
            ) {
                assets.insert(id.to_owned(), data.to_owned());
            }
        }
        LocalFiles {
            folder,
            search: doc.settings().xref_paths.clone(),
            assets,
            plans: Mutex::new(HashMap::new()),
        }
    }

    /// A file at `path` opened for reading by ranges (`whose`: “Rasterin”, “Bulutun”).
    fn file_bytes(path: &Path, whose: &str) -> Result<Bytes, String> {
        let file = File::open(path).map_err(|e| {
            format!(
                "{whose} dosyası “{}” açılamadı: {e}. Dosyanın yerinde olduğunu denetleyin.",
                path.display()
            )
        })?;
        let size = file
            .metadata()
            .map_err(|e| format!("{whose} dosyası okunamadı: {e}."))?
            .len();
        Ok(Bytes::File(Mutex::new(file), size))
    }

    fn resolve(&self, file: &str) -> PathBuf {
        let path = Path::new(file);
        match &self.folder {
            Some(dir) if path.is_relative() => dir.join(path),
            _ => path.to_path_buf(),
        }
    }

    fn bytes_of(&self, raster: &RasterFields) -> Result<Bytes, String> {
        self.bytes_from(
            raster.asset.as_deref(),
            raster.file.as_deref(),
            raster.url.is_some(),
            Of::Raster,
        )
        .map(|(b, _)| b)
    }

    /// A file's bytes and its key (`asset:<id>`, `file:<path>`): an embedded one's from the library, a linked one
    /// found as the desktop finds it; an address refused.
    fn bytes_from(
        &self,
        asset: Option<&str>,
        file: Option<&str>,
        url: bool,
        of: Of,
    ) -> Result<(Bytes, String), String> {
        let (noun, whose) = (of.noun(), of.whose());
        match (asset, file, url) {
            (Some(id), _, _) => {
                let url = self
                    .assets
                    .get(id)
                    .ok_or_else(|| format!("“{id}” kimlikli {noun} projenin kitaplığında yok."))?;
                let data = url
                    .split_once(";base64,")
                    .and_then(|(_, d)| kentos_commands::encoding::base64_decode(d))
                    .ok_or_else(|| format!("“{id}” kimlikli gömülü {noun} okunamadı."))?;
                Ok((Bytes::Memory(Arc::new(data)), format!("asset:{id}")))
            }
            (None, Some(f), _) => {
                // As the desktop finds a linked file: its path, its relative path from the drawing's folder, the
                // search paths (docs/adr/0548 §3); a drawing with no file reads a relative path as it is given.
                let found = kentos_native_application::links::find_in(
                    f,
                    None,
                    self.folder.as_deref(),
                    &self.search,
                );
                let path = match found.at {
                    Some(at) => at,
                    None if self.folder.is_none() && Path::new(f).is_file() => PathBuf::from(f),
                    None if !found.tried.is_empty() => {
                        return Err(format!(
                            "{whose} dosyası “{f}” bulunamadı; bakılan yerler: {}. Dosyayı yerine koyun ya da \
                             projenin arama yollarına klasörünü ekleyin.",
                            found.tried.join(", ")
                        ));
                    }
                    None => self.resolve(f),
                };
                let bytes = Self::file_bytes(&path, whose)?;
                Ok((bytes, format!("file:{}", path.display())))
            }
            (None, None, true) => Err(format!(
                "Adresteki {noun} yalnız masaüstünde okunur; dosyayı indirip ekleyin."
            )),
            (None, None, false) => Err(format!("{whose} dosyası yok.")),
        }
    }
}

impl Files for LocalFiles {
    fn open_cloud(&self, source: &CloudSource) -> Result<Box<dyn CloudRead + '_>, String> {
        let (bytes, key) = self.bytes_from(
            source.asset.as_deref(),
            source.file.as_deref(),
            source.url.is_some(),
            Of::Cloud,
        )?;
        // A text cloud is scanned once a run; the run's next passes take its plan.
        let plan = if source.format == CloudFormat::Xyz {
            let kept = self
                .plans
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(&key)
                .cloned();
            let plan = match kept {
                Some(p) => p,
                None => text_plan(&bytes)?,
            };
            self.plans
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(key, plan.clone());
            Some(plan)
        } else {
            None
        };
        cloud_read(Box::new(bytes), source.format, plan)
    }

    fn output_path(
        &self,
        asked: &str,
        beside: Option<Beside<'_>>,
        suffix: &str,
        ext: &str,
    ) -> Result<String, String> {
        let asked = asked.trim();
        if !asked.is_empty() {
            return Ok(with_extension(&self.resolve(asked).to_string_lossy(), ext));
        }
        let beside = beside.ok_or("Çıktı dosyasını yazın.")?;
        let (dir, stem) = match (beside.file, beside.name) {
            (Some(f), _) => {
                let p = self.resolve(f);
                let stem = p.file_stem().map_or_else(
                    || beside.fallback.to_owned(),
                    |s| s.to_string_lossy().into_owned(),
                );
                (p.parent().map(Path::to_path_buf), stem)
            }
            (None, Some(n)) => (self.folder.clone(), n.to_owned()),
            (None, None) => (self.folder.clone(), beside.fallback.to_owned()),
        };
        let dir = dir.ok_or(
            "Kaynağın yanında yazılacak klasör yok (gömülü raster); çıktı dosyasını yazın.",
        )?;
        Ok(dir
            .join(format!("{stem}{suffix}{ext}"))
            .to_string_lossy()
            .into_owned())
    }

    fn create(&self, path: &str) -> Result<Box<dyn Sink + '_>, String> {
        let path = PathBuf::from(path);
        let name = path
            .file_name()
            .map_or_else(|| "cikti".to_owned(), |n| n.to_string_lossy().into_owned());
        let part = path.with_file_name(format!("{name}.yaziliyor"));
        let file = File::create(&part).map_err(|e| {
            format!(
                "“{}” yazılamadı: {e}. Klasörün yazılabilir olduğunu denetleyin.",
                path.display()
            )
        })?;
        Ok(Box::new(FileSink {
            path,
            part,
            file: Some(BufWriter::with_capacity(1 << 20, file)),
        }))
    }

    fn copc(
        &self,
        from: &str,
        to: &str,
        feedback: &mut dyn Feedback,
        start: f64,
        end: f64,
    ) -> Result<(), String> {
        let bytes = Self::file_bytes(Path::new(from), "Bulutun")?;
        let mut tick = |done: f64| {
            feedback.progress(start + (end - start) * done, "COPC dizini kuruluyor");
            feedback.canceled()
        };
        copc_file(Box::new(bytes), Path::new(to), &mut tick)
    }

    fn remove(&self, path: &str) {
        let _ = std::fs::remove_file(path);
    }

    fn open_raster(&self, raster: &RasterFields) -> Result<RasterOpen<'_>, String> {
        let (source, key) = self.bytes_from(
            raster.asset.as_deref(),
            raster.file.as_deref(),
            raster.url.is_some(),
            Of::Raster,
        )?;
        if let Some(part) = kentos_formats::multidim::cube::Part::of(raster) {
            let reader = netcdf_reader(&source, &part, RASTER_READER_BUDGET)?;
            return Ok(RasterOpen {
                reader,
                block: Box::new(move |need| source.read(need.offset, need.len)),
                jpeg: Box::new(jpeg_pixels),
            });
        }
        let head = source.read(0, source.size().min(64))?;
        if vrt::sniff(&head) {
            // A mosaic's files are beside its VRT (docs/adr/0517 §2): an embedded VRT has no folder to find them in.
            let Some(path) = key.strip_prefix("file:").map(PathBuf::from) else {
                return Err(
                    "Gömülü mozaik (VRT) açılmaz: dosyaları yanında olmalı; mozaiği bağlı dosya olarak ekleyin."
                        .into(),
                );
            };
            let text = mosaic_text(source.read(0, source.size().min(vrt::MAX_TEXT as u64 + 1))?)?;
            let (reader, members) =
                mosaic_reader(&path, &text, RASTER_READER_BUDGET, |p, each| {
                    let bytes = Self::file_bytes(p, "Rasterin")?;
                    reader_of(&bytes, each).map(|(r, _)| r)
                })?;
            return Ok(RasterOpen {
                reader,
                block: Box::new(move |need| {
                    if need.member > 0 {
                        read_member(&members, need)
                    } else {
                        Err("Mozaiğin bu bloğu okunamadı.".to_owned())
                    }
                }),
                jpeg: Box::new(jpeg_pixels),
            });
        }
        let (reader, tiled) = reader_of(&source, RASTER_READER_BUDGET)?;
        Ok(RasterOpen {
            reader,
            block: Box::new(move |need| {
                if tiled && need.file == 0 {
                    source.read(need.offset, need.len)
                } else {
                    Err("Rasterin bu bloğu okunamadı.".to_owned())
                }
            }),
            jpeg: Box::new(jpeg_pixels),
        })
    }

    fn open_cube(&self, raster: &RasterFields) -> Result<CubeOpen<'_>, String> {
        let Some(part) = kentos_formats::multidim::cube::Part::of(raster) else {
            return Err("Bu raster NetCDF veri seti göstermiyor.".to_owned());
        };
        let source = self.bytes_of(raster)?;
        let cube = netcdf_slice(&source, &part)?;
        Ok(CubeOpen {
            cube,
            part,
            read: Box::new(move |offset, len| source.read(offset, len)),
        })
    }
}

/// A raster's reader from its bytes and whether its blocks are read later from them (a TIFF read in pieces) or it is in
/// memory (a PNG or JPEG decoded whole); why not.
fn reader_of(source: &Bytes, budget: usize) -> Result<(Reader, bool), String> {
    let size = source.size();
    let head = source.read(0, size.min(16))?;
    if tiff::sniff(&head) {
        let mut store = ByteStore::new();
        let mut taken = 0u64;
        let t = loop {
            match tiff::parse(&store, size).map_err(|e| e.0)? {
                Step::Done(t) => break t,
                Step::Need(n) => {
                    taken += n.len;
                    if taken > HEADER_MOST {
                        return Err("TIFF'in başlığı çok büyük.".into());
                    }
                    store.put(n.offset, source.read(n.offset, n.len)?);
                }
            }
        };
        return Ok((Reader::tiff(&t, None, budget).map_err(|e| e.0)?, true));
    }
    if kentos_formats::raster::png::sniff(&head) || is_jpeg(&head) {
        if size > IMAGE_MOST {
            return Err(format!(
                "Resim {} MB; en çok {} MB okunur.",
                size >> 20,
                IMAGE_MOST >> 20
            ));
        }
        return Ok((picture_reader(&source.read(0, size)?, budget)?, false));
    }
    Err("Rasterin dosyası GeoTIFF, TIFF, PNG ya da JPEG değil.".into())
}

/// A file written beside its place, then renamed.
struct FileSink {
    path: PathBuf,
    part: PathBuf,
    file: Option<BufWriter<File>>,
}

impl FileSink {
    fn fail(&self, e: std::io::Error) -> String {
        format!("“{}” yazılamadı: {e}.", self.path.display())
    }

    fn closed(&mut self) -> Result<(), String> {
        let f = self.file.take().ok_or("Dosya kapalı.")?;
        let f = f.into_inner().map_err(|e| self.fail(e.into_error()))?;
        f.sync_all().map_err(|e| self.fail(e))
    }
}

impl Sink for FileSink {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        let path = self.path.display().to_string();
        let f = self.file.as_mut().ok_or("Dosya kapalı.")?;
        f.write_all(bytes)
            .map_err(|e| format!("“{path}” yazılamadı: {e}."))
    }

    fn patch(&mut self, at: u64, bytes: &[u8]) -> Result<(), String> {
        let path = self.path.display().to_string();
        let fail = |e: std::io::Error| format!("“{path}” yazılamadı: {e}.");
        let f = self.file.as_mut().ok_or("Dosya kapalı.")?;
        let end = f.stream_position().map_err(fail)?;
        f.seek(SeekFrom::Start(at)).map_err(fail)?;
        f.write_all(bytes).map_err(fail)?;
        f.seek(SeekFrom::Start(end)).map_err(fail)?;
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<(), String> {
        self.closed()?;
        std::fs::rename(&self.part, &self.path).map_err(|e| self.fail(e))
    }

    fn finish_cog(mut self: Box<Self>) -> Result<(), String> {
        self.closed()?;
        let done = cog_file(&self.part, &self.path);
        let _ = std::fs::remove_file(&self.part);
        done
    }
}

impl Drop for FileSink {
    fn drop(&mut self) {
        if self.file.take().is_some() {
            let _ = std::fs::remove_file(&self.part);
        }
    }
}

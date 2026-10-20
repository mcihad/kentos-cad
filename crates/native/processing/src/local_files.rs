//! The headless server's files (docs/adr/0515): what a raster tool reads and writes where there is no window — a
//! raster's linked file or embedded bytes, read where they are, and a result written beside its place, then renamed
//! (a COG laid out again). Python's `cad.processing.run` and MCP run the raster tools through these, as the desktop
//! runs them through its own. An address (HTTP ranges), a NetCDF slice, a JPEG and the point clouds' files stay the
//! desktop's: such a run is refused, saying why.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use kentos_commands::{CloudSource, RasterFields};
use kentos_domain::Document;
use kentos_formats::raster::source::Reader;
use kentos_formats::raster::{ByteStore, Step, tiff};

use crate::files::{
    Beside, CloudRead, Files, RASTER_READER_BUDGET, RasterOpen, Sink, cog_file, with_extension,
};
use crate::types::Feedback;

/// At most this many bytes of a TIFF's directories are read to open it.
const HEADER_MOST: u64 = 256 << 20;
/// At most this big a PNG is read whole.
const IMAGE_MOST: u64 = 512 << 20;

/// A raster's bytes: a file read by ranges, or an embedded file's bytes.
enum Bytes {
    File(Mutex<File>, u64),
    Memory(Arc<Vec<u8>>),
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
                    .map_err(|e| format!("Rasterin dosyası okunamadı: {e}."))?;
                Ok(out)
            }
        }
    }
}

/// The files of a drawing with no window: its folder (relative paths are taken from it) and its library's
/// embedded rasters.
pub struct LocalFiles {
    folder: Option<PathBuf>,
    assets: HashMap<String, String>,
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
        LocalFiles { folder, assets }
    }

    fn resolve(&self, file: &str) -> PathBuf {
        let path = Path::new(file);
        match &self.folder {
            Some(dir) if path.is_relative() => dir.join(path),
            _ => path.to_path_buf(),
        }
    }

    fn bytes_of(&self, raster: &RasterFields) -> Result<Bytes, String> {
        match (&raster.asset, &raster.file, &raster.url) {
            (Some(id), _, _) => {
                let url = self
                    .assets
                    .get(id)
                    .ok_or_else(|| format!("“{id}” kimlikli raster projenin kitaplığında yok."))?;
                let data = url
                    .split_once(";base64,")
                    .and_then(|(_, d)| kentos_commands::encoding::base64_decode(d))
                    .ok_or_else(|| format!("“{id}” kimlikli gömülü raster okunamadı."))?;
                Ok(Bytes::Memory(Arc::new(data)))
            }
            (None, Some(f), _) => {
                let path = self.resolve(f);
                let file = File::open(&path).map_err(|e| {
                    format!(
                        "Rasterin dosyası “{}” açılamadı: {e}. Dosyanın yerinde olduğunu denetleyin.",
                        path.display()
                    )
                })?;
                let size = file
                    .metadata()
                    .map_err(|e| format!("Rasterin dosyası okunamadı: {e}."))?
                    .len();
                Ok(Bytes::File(Mutex::new(file), size))
            }
            (None, None, Some(_)) => {
                Err("Adresteki raster yalnız masaüstünde okunur; dosyayı indirip ekleyin.".into())
            }
            (None, None, None) => Err("Rasterin dosyası yok.".into()),
        }
    }
}

impl Files for LocalFiles {
    fn open_cloud(&self, _: &CloudSource) -> Result<Box<dyn CloudRead + '_>, String> {
        Err("Nokta bulutu araçları yalnız masaüstünde çalışır.".into())
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

    fn copc(&self, _: &str, _: &str, _: &mut dyn Feedback, _: f64, _: f64) -> Result<(), String> {
        Err("COPC dizini yalnız masaüstünde kurulur.".into())
    }

    fn remove(&self, path: &str) {
        let _ = std::fs::remove_file(path);
    }

    fn open_raster(&self, raster: &RasterFields) -> Result<RasterOpen<'_>, String> {
        if raster.dataset.is_some() {
            return Err("NetCDF veri setinin rasteri yalnız masaüstünde okunur.".into());
        }
        let source = self.bytes_of(raster)?;
        let size = source.size();
        let head = source.read(0, size.min(16))?;
        let reader = if tiff::sniff(&head) {
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
            Reader::tiff(&t, None, RASTER_READER_BUDGET).map_err(|e| e.0)?
        } else if kentos_formats::raster::png::sniff(&head) {
            if size > IMAGE_MOST {
                return Err(format!(
                    "PNG {} MB; en çok {} MB okunur.",
                    size >> 20,
                    IMAGE_MOST >> 20
                ));
            }
            let p = kentos_formats::raster::png::decode(&source.read(0, size)?).map_err(|e| e.0)?;
            Reader::image(
                p.width,
                p.height,
                p.bands,
                p.samples,
                p.nodata,
                None,
                RASTER_READER_BUDGET,
            )
            .map_err(|e| e.0)?
        } else if head.starts_with(&[0xFF, 0xD8]) {
            return Err(
                "JPEG raster yalnız masaüstünde okunur; GeoTIFF ya da PNG'ye çevirin.".into(),
            );
        } else {
            return Err("Rasterin dosyası GeoTIFF, TIFF ya da PNG değil.".into());
        };
        let tiled = tiff::sniff(&head);
        Ok(RasterOpen {
            reader,
            block: Box::new(move |need| {
                if tiled && need.file == 0 {
                    source.read(need.offset, need.len)
                } else {
                    Err("Rasterin bu bloğu okunamadı.".to_owned())
                }
            }),
            jpeg: Box::new(|_| Err("JPEG sıkıştırmalı TIFF yalnız masaüstünde okunur.".to_owned())),
        })
    }
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

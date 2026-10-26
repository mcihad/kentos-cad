//! The desktop's files for İşlemler's point cloud tools (docs/adr/0207 §7;
//! `kentos_processing::files`): a cloud's files read whole where they are (a
//! linked file beside the drawing, an address by HTTP ranges, an embedded
//! file's bytes from the project's library), several pieces decompressed at
//! once on worker threads and handed out in the file's order; a text cloud
//! scanned once (its plan kept for the run's next passes); results written
//! beside their place (`.yaziliyor`) and renamed when whole, a COPC made
//! from its LAZ by the index's builder. The analyses' rasters (docs/adr/0231
//! §2) are opened the same ways, each with its own reader.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use kentos_commands::{CloudFormat, CloudSource, RasterFields};
use kentos_pointcloud::text;
use kentos_processing::Feedback;
use kentos_processing::cloud_files;
use kentos_processing::files::{
    Beside, CloudRead, CubeOpen, Files, RASTER_READER_BUDGET, RasterOpen, Sink, with_extension,
};

use super::bytes::{Bytes, Remote};
use crate::rasters::tiles::{Origin, jpeg_pixels};

/// The run's files: the drawing's folder and the embedded clouds' bytes (as the library holds them, decoded when read).
pub struct DesktopFiles {
    folder: Option<PathBuf>,
    assets: HashMap<String, Arc<String>>,
    decoded: Mutex<HashMap<String, Arc<Vec<u8>>>>,
    plans: Mutex<HashMap<String, text::Plan>>,
}

impl DesktopFiles {
    /// For a drawing in `folder` whose library's embedded clouds are `assets` (id → data URL).
    pub fn new(folder: Option<PathBuf>, assets: HashMap<String, Arc<String>>) -> DesktopFiles {
        DesktopFiles {
            folder,
            assets,
            decoded: Mutex::new(HashMap::new()),
            plans: Mutex::new(HashMap::new()),
        }
    }

    /// An embedded file's bytes from the library (`what`: “nokta bulutu”, “raster”), decoded once a run.
    fn asset_bytes(&self, id: &str, what: &str) -> Result<Arc<Vec<u8>>, String> {
        if let Some(b) = self
            .decoded
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(id)
        {
            return Ok(b.clone());
        }
        let url = self
            .assets
            .get(id)
            .ok_or_else(|| format!("“{id}” kimlikli {what} projenin kitaplığında yok."))?;
        let data = url
            .split_once(";base64,")
            .and_then(|(_, d)| kentos_sheet::template::base64_decode(d))
            .ok_or_else(|| format!("“{id}” kimlikli gömülü {what} okunamadı."))?;
        let data = Arc::new(data);
        self.decoded
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id.to_owned(), data.clone());
        Ok(data)
    }

    fn bytes_of(&self, s: &CloudSource) -> Result<(Bytes, String), String> {
        match (&s.asset, &s.file, &s.url) {
            (Some(id), _, _) => Ok((
                Bytes::Memory(self.asset_bytes(id, "nokta bulutu")?),
                format!("asset:{id}"),
            )),
            (None, Some(f), _) => {
                let path = crate::pictures::resolve(f, self.folder.as_deref());
                let b = Bytes::file(&path)?;
                Ok((b, format!("file:{}", path.display())))
            }
            (None, None, Some(u)) => Ok((Bytes::Remote(Remote::open(u)?), format!("url:{u}"))),
            (None, None, None) => Err("Bulutun dosyası yok.".to_owned()),
        }
    }

    /// A text cloud's plan: scanned once a run.
    fn text_plan(&self, key: &str, bytes: &Bytes) -> Result<text::Plan, String> {
        if let Some(p) = self
            .plans
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
        {
            return Ok(p.clone());
        }
        let plan = cloud_files::text_plan(bytes)?;
        self.plans
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key.to_owned(), plan.clone());
        Ok(plan)
    }
}

/// A file written beside its place and renamed when whole; left behind, the piece is removed.
struct FileSink {
    path: PathBuf,
    part: PathBuf,
    file: Option<BufWriter<File>>,
}

impl Sink for FileSink {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        let f = self.file.as_mut().ok_or("Dosya kapalı.")?;
        f.write_all(bytes)
            .map_err(|e| format!("“{}” yazılamadı: {e}.", self.path.display()))
    }

    fn patch(&mut self, at: u64, bytes: &[u8]) -> Result<(), String> {
        let f = self.file.as_mut().ok_or("Dosya kapalı.")?;
        let fail = |e: std::io::Error| format!("“{}” yazılamadı: {e}.", self.path.display());
        let end = f.stream_position().map_err(fail)?;
        f.seek(SeekFrom::Start(at)).map_err(fail)?;
        f.write_all(bytes).map_err(fail)?;
        f.seek(SeekFrom::Start(end)).map_err(fail)?;
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<(), String> {
        let Some(f) = self.file.take() else {
            return Err("Dosya kapalı.".into());
        };
        let fail = |e: std::io::Error| format!("“{}” yazılamadı: {e}.", self.path.display());
        let f = f.into_inner().map_err(|e| fail(e.into_error()))?;
        f.sync_all().map_err(fail)?;
        drop(f);
        std::fs::rename(&self.part, &self.path).map_err(fail)
    }

    fn finish_cog(mut self: Box<Self>) -> Result<(), String> {
        let Some(f) = self.file.take() else {
            return Err("Dosya kapalı.".into());
        };
        let fail = |e: std::io::Error| format!("“{}” yazılamadı: {e}.", self.path.display());
        let f = f.into_inner().map_err(|e| fail(e.into_error()))?;
        drop(f);
        let done = kentos_processing::files::cog_file(&self.part, &self.path);
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

impl Files for DesktopFiles {
    fn open_cloud(&self, source: &CloudSource) -> Result<Box<dyn CloudRead + '_>, String> {
        let (bytes, key) = self.bytes_of(source)?;
        let plan = if source.format == CloudFormat::Xyz {
            Some(self.text_plan(&key, &bytes)?)
        } else {
            None
        };
        cloud_files::cloud_read(Box::new(bytes), source.format, plan)
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
            return Ok(with_extension(asked, ext));
        }
        let source = beside.ok_or("Çıktının yeri yok: bir çıktı dosyası seçin.")?;
        let stem = source.stem();
        let folder = match source.file {
            Some(f) => crate::pictures::resolve(f, self.folder.as_deref())
                .parent()
                .map(Path::to_path_buf),
            None => self.folder.clone(),
        }
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .ok_or("Çıktının klasörü bulunamadı: bir çıktı dosyası seçin.")?;
        Ok(folder
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
        let mut tick = |done: f64| {
            feedback.progress(start + (end - start) * done, "COPC dizini kuruluyor");
            feedback.canceled()
        };
        super::index::copc_file(Path::new(from), Path::new(to), &mut tick)
    }

    fn remove(&self, path: &str) {
        let _ = std::fs::remove_file(path);
    }

    fn stage_raster(&self, raster: &RasterFields) -> Result<Option<String>, String> {
        let Some(url) = &raster.url else {
            return Ok(None);
        };
        let dir = crate::rasters::pyramid::staging_folder()
            .ok_or("Rasterin geçici kopyası için klasör bulunamadı (HOME yok).")?;
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Geçici klasör açılamadı: {}: {e}", dir.display()))?;
        // One copy per run: the address's digest and the moment, so two runs never share a file.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let digest = kentos_sheet::template::sha256_hex(url.as_bytes());
        let to = dir.join(format!("{}-{nanos}.tif", &digest[..16]));
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        kentos_cloud::range::download(url, &to, kentos_cloud::range::DOWNLOAD_MOST, stop)?;
        Ok(Some(to.to_string_lossy().into_owned()))
    }

    fn open_raster(&self, raster: &RasterFields) -> Result<RasterOpen<'_>, String> {
        // A GeoPackage tile table read from its file (docs/adr/0627 §3).
        if let (Some(_), Some(f)) = (&raster.tiles, &raster.file) {
            let path = crate::pictures::resolve(f, self.folder.as_deref());
            return kentos_processing::raster_files::table_open(&path, raster);
        }
        let origin = match (&raster.asset, &raster.file, &raster.url) {
            (Some(id), _, _) => Origin::Bytes(self.asset_bytes(id, "raster")?),
            (None, Some(f), _) => Origin::File(crate::pictures::resolve(f, self.folder.as_deref())),
            (None, None, Some(u)) => Origin::Url(u.clone()),
            (None, None, None) => return Err("Rasterin dosyası yok.".to_owned()),
        };
        let part = crate::rasters::part_of(raster);
        let open =
            crate::rasters::tiles::open_reader_part(&origin, part.as_ref(), RASTER_READER_BUDGET)?;
        // A mosaic's blocks are its files' (docs/adr/0517 §2): the block source reads either.
        let (reader, blocks) = open.split();
        Ok(RasterOpen {
            reader,
            block: Box::new(move |need| blocks.read(need)),
            jpeg: Box::new(jpeg_pixels),
        })
    }

    fn open_cube(&self, raster: &RasterFields) -> Result<CubeOpen<'_>, String> {
        let origin = match (&raster.asset, &raster.file, &raster.url) {
            (Some(id), _, _) => Origin::Bytes(self.asset_bytes(id, "raster")?),
            (None, Some(f), _) => Origin::File(crate::pictures::resolve(f, self.folder.as_deref())),
            (None, None, Some(u)) => Origin::Url(u.clone()),
            (None, None, None) => return Err("Rasterin dosyası yok.".to_owned()),
        };
        let Some(part) = crate::rasters::part_of(raster) else {
            return Err("Bu raster NetCDF veri seti göstermiyor.".to_owned());
        };
        let (cube, source) = crate::rasters::tiles::fresh_cube(&origin, &part)?;
        Ok(CubeOpen {
            cube,
            part,
            read: Box::new(move |offset, len| source.read(offset, len)),
        })
    }
}

/// The run's files for the open drawing: its folder, its library's embedded clouds.
pub fn for_drawing(model: &kentos_domain::Document, path: Option<&Path>) -> Arc<dyn Files> {
    let mut assets = HashMap::new();
    for it in &model.styles().items {
        let id = it.get("id").and_then(serde_json::Value::as_str);
        // The clouds' files, and the rasters' the analyses read (docs/adr/0231 §2; a raster's id is `raster-…`).
        let wanted = it.get("kind").and_then(serde_json::Value::as_str) == Some("asset")
            && (matches!(
                it.get("format").and_then(serde_json::Value::as_str),
                Some("las" | "laz" | "copc" | "xyz")
            ) || id.is_some_and(|id| id.starts_with("raster-")));
        if let (true, Some(id), Some(data)) = (
            wanted,
            id,
            it.get("data").and_then(serde_json::Value::as_str),
        ) {
            assets.insert(id.to_owned(), Arc::new(data.to_owned()));
        }
    }
    Arc::new(DesktopFiles::new(
        path.and_then(Path::parent).map(Path::to_path_buf),
        assets,
    ))
}

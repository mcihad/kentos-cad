//! The desktop's files for İşlemler's point cloud tools (docs/adr/0207 §7;
//! `kentos_processing::files`): a cloud's files read whole where they are (a
//! linked file beside the drawing, an address by HTTP ranges, an embedded
//! file's bytes from the project's library), several pieces decompressed at
//! once on worker threads and handed out in the file's order; a text cloud
//! scanned once (its plan kept for the run's next passes); results written
//! beside their place (`.yaziliyor`) and renamed when whole, a COPC made
//! from its LAZ by the index's builder. The analyses' rasters (docs/adr/0231
//! §2) are opened the same ways, each with its own reader.

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use kentos_contracts::{CloudFormat, CloudSource, RasterFields};
use kentos_pointcloud::ops::convert::Input;
use kentos_pointcloud::source::{Cloud, Opening, Run};
use kentos_pointcloud::{Step, text};
use kentos_processing::Feedback;
use kentos_processing::files::{
    Beside, CloudRead, Files, RASTER_READER_BUDGET, RasterOpen, Sink, with_extension,
};

use super::bytes::{Bytes, Remote};
use crate::rasters::tiles::{Origin, jpeg_pixels, open_reader};

/// A text cloud is read in pieces of this size.
const PIECE: u64 = 4 * 1024 * 1024;

/// Threads decompressing a file's pieces at once.
fn threads() -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get().saturating_sub(1))
        .clamp(1, 4)
}

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
        let mut scan = text::Scan::new();
        let size = bytes.size();
        let mut at = 0;
        while at < size {
            let n = PIECE.min(size - at);
            scan.feed(&bytes.read(at, n)?).map_err(|e| e.0)?;
            at += n;
        }
        let plan = scan.finish().map_err(|e| e.0)?;
        self.plans
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key.to_owned(), plan.clone());
        Ok(plan)
    }
}

/// A LAS, LAZ or COPC read whole, its runs decompressed several at once.
struct LasRead {
    input: Input,
    cloud: Arc<Cloud>,
    bytes: Arc<Bytes>,
    runs: Vec<Run>,
    next: usize,
    ready: VecDeque<Vec<u8>>,
    total: u64,
    seen: u64,
}

impl CloudRead for LasRead {
    fn input(&self) -> &Input {
        &self.input
    }

    fn next(&mut self, out: &mut Vec<u8>) -> Result<bool, String> {
        if self.ready.is_empty() {
            if self.next >= self.runs.len() {
                return Ok(false);
            }
            let n = (threads() * 2).min(self.runs.len() - self.next);
            let batch = &self.runs[self.next..self.next + n];
            self.next += n;
            let (cloud, bytes) = (&self.cloud, &self.bytes);
            let decoded: Vec<Result<Vec<u8>, String>> = std::thread::scope(|s| {
                let handles: Vec<_> = batch
                    .iter()
                    .map(|run| {
                        s.spawn(move || -> Result<Vec<u8>, String> {
                            let raw = bytes.read(run.need.offset, run.need.len)?;
                            let mut recs = Vec::new();
                            cloud.records(run, &raw, &mut recs).map_err(|e| e.0)?;
                            Ok(recs)
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|h| h.join().unwrap_or_else(|_| Err("Parça çözülemedi.".into())))
                    .collect()
            });
            for d in decoded {
                self.ready.push_back(d?);
            }
        }
        let Some(recs) = self.ready.pop_front() else {
            return Ok(false);
        };
        self.seen += (recs.len() / self.input.layout.len.max(1)) as u64;
        out.extend_from_slice(&recs);
        Ok(true)
    }

    fn done(&self) -> f64 {
        self.seen as f64 / self.total.max(1) as f64
    }
}

/// A text cloud parsed piece by piece.
struct TextRead {
    input: Input,
    parse: text::Parse,
    bytes: Bytes,
    at: u64,
    finished: bool,
}

impl CloudRead for TextRead {
    fn input(&self) -> &Input {
        &self.input
    }

    fn next(&mut self, out: &mut Vec<u8>) -> Result<bool, String> {
        let size = self.bytes.size();
        if self.at < size {
            let n = PIECE.min(size - self.at);
            let piece = self.bytes.read(self.at, n)?;
            self.at += n;
            self.parse.feed(&piece, out).map_err(|e| e.0)?;
            return Ok(true);
        }
        if self.finished {
            return Ok(false);
        }
        self.finished = true;
        self.parse.finish(out).map_err(|e| e.0)?;
        Ok(true)
    }

    fn done(&self) -> f64 {
        self.at as f64 / self.bytes.size().max(1) as f64
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
        if source.format == CloudFormat::Xyz {
            let plan = self.text_plan(&key, &bytes)?;
            let input = Input::text(&plan);
            return Ok(Box::new(TextRead {
                input,
                parse: text::Parse::new(plan),
                bytes,
                at: 0,
                finished: false,
            }));
        }
        let mut o = Opening::new(bytes.size());
        let cloud = loop {
            match o.step().map_err(|e| e.0)? {
                Step::Done(c) => break c,
                Step::Need(n) => o.put(n.offset, bytes.read(n.offset, n.len)?),
            }
        };
        let input = Input {
            head: cloud.head.clone(),
            layout: cloud.layout,
            vlrs: cloud.vlrs.clone(),
            evlrs: cloud.evlrs.clone(),
        };
        let runs = cloud.runs();
        let total = cloud.head.count;
        Ok(Box::new(LasRead {
            input,
            cloud: Arc::new(cloud),
            bytes: Arc::new(bytes),
            runs,
            next: 0,
            ready: VecDeque::new(),
            total,
            seen: 0,
        }))
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

    fn open_raster(&self, raster: &RasterFields) -> Result<RasterOpen<'_>, String> {
        let origin = match (&raster.asset, &raster.file, &raster.url) {
            (Some(id), _, _) => Origin::Bytes(self.asset_bytes(id, "raster")?),
            (None, Some(f), _) => Origin::File(crate::pictures::resolve(f, self.folder.as_deref())),
            (None, None, Some(u)) => Origin::Url(u.clone()),
            (None, None, None) => return Err("Rasterin dosyası yok.".to_owned()),
        };
        let open = open_reader(&origin, RASTER_READER_BUDGET)?;
        let source = open.source;
        Ok(RasterOpen {
            reader: open.reader,
            block: Box::new(move |need| match &source {
                Some(b) if need.file == 0 => b.read(need.offset, need.len),
                _ => Err("Rasterin bu bloğu okunamadı.".to_owned()),
            }),
            jpeg: Box::new(jpeg_pixels),
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

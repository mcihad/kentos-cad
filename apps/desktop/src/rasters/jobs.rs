//! The rasters' long work and its panel (docs/adr/0204 §3, §6): a large
//! raster's pyramid pass (`pyramid`) and Raster oturt's resampling run on
//! threads of their own; the panel at the drawing's lower right shows each
//! with its share and Durdur, as a large import's (exchange/drawing_import.rs).
//! A resampling reads the source tile by tile through its own reader and
//! writes a tiled, Deflate GeoTIFF beside a linked source
//! (`<ad>-oturtulmus.tif`, a file beside it first, renamed when whole) or,
//! for an embedded one, in memory to embed.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use iced::widget::container;
use iced::{Bottom, Element, Fill, Right};
use kentos_formats::raster::warp::Job;
use kentos_ui::widget::progress::{Task as Job_, TaskList};

use super::Event as Rasters;
use super::tiles::{Origin, open};
use crate::app::{App, Message};

/// A resampling under way: its raster's name, its share, its stop.
#[derive(Clone, Debug)]
struct Warping {
    name: String,
    done: f64,
    stop: Arc<AtomicBool>,
}

fn warping() -> &'static Mutex<Option<Warping>> {
    static WARPING: OnceLock<Mutex<Option<Warping>>> = OnceLock::new();
    WARPING.get_or_init(|| Mutex::new(None))
}

/// Durdur on the resampling.
pub fn stop_warp() {
    if let Some(w) = warping()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
    {
        w.stop.store(true, Ordering::Relaxed);
    }
}

/// Where a resampling's output goes.
#[derive(Clone, Debug)]
pub enum Output {
    /// A file (beside a linked source).
    File(PathBuf),
    /// Memory, to embed (an embedded source).
    Memory,
}

/// A resampling's result: the new raster's grid and kind, and its file or bytes.
#[derive(Clone, Debug)]
pub struct Warped {
    pub affine: [f64; 6],
    pub width: u32,
    pub height: u32,
    pub bands: u32,
    pub sample: kentos_contracts::RasterSample,
    pub alpha: bool,
    pub path: Option<PathBuf>,
    pub bytes: Option<Arc<Vec<u8>>>,
}

/// Runs a resampling of `origin` by `make` (the job and its header), to `output`; its share in the panel.
pub fn warp(
    name: String,
    origin: Origin,
    output: Output,
    make: impl FnOnce(&kentos_formats::raster::source::Reader) -> Result<(Job, Vec<u8>), String>,
) -> Result<Warped, String> {
    let stop = Arc::new(AtomicBool::new(false));
    *warping().lock().unwrap_or_else(PoisonError::into_inner) = Some(Warping {
        name,
        done: 0.0,
        stop: stop.clone(),
    });
    let result = run(origin, output, make, &stop);
    *warping().lock().unwrap_or_else(PoisonError::into_inner) = None;
    super::tiles::service().wake();
    result
}

fn run(
    origin: Origin,
    output: Output,
    make: impl FnOnce(&kentos_formats::raster::source::Reader) -> Result<(Job, Vec<u8>), String>,
    stop: &AtomicBool,
) -> Result<Warped, String> {
    use std::io::{Seek, SeekFrom, Write};
    let opened = open(&origin, None)?;
    let (mut job, header) = {
        let r = opened.reader.lock().unwrap_or_else(PoisonError::into_inner);
        make(&r)?
    };
    let part = match &output {
        Output::File(path) => Some(path.with_extension("tif.part")),
        Output::Memory => None,
    };
    let mut file = match &part {
        Some(p) => Some(
            std::fs::File::create(p).map_err(|e| format!("“{}” yazılamadı: {e}.", p.display()))?,
        ),
        None => None,
    };
    let mut memory: Vec<u8> = Vec::new();
    let mut put = |bytes: &[u8], file: &mut Option<std::fs::File>| -> Result<(), String> {
        match file {
            Some(f) => f.write_all(bytes).map_err(|e| e.to_string()),
            None => {
                memory.extend_from_slice(bytes);
                if memory.len() > super::MOST_EMBEDDED {
                    return Err(format!(
                        "Oturtulan raster {} MB'ı geçiyor; gömülemez. Rasteri bağlı yapıp yeniden deneyin.",
                        super::MOST_EMBEDDED >> 20
                    ));
                }
                Ok(())
            }
        }
    };
    let cleanup = |part: &Option<PathBuf>| {
        if let Some(p) = part {
            let _ = std::fs::remove_file(p);
        }
    };
    put(&header, &mut file).inspect_err(|_| cleanup(&part))?;
    while let Some(region) = job.region() {
        if stop.load(Ordering::Relaxed) {
            drop(file);
            cleanup(&part);
            return Err("durduruldu".to_owned());
        }
        let samples = match region {
            Some((x, y, w, h)) => {
                opened.fill(0, x, y, w, h).inspect_err(|_| cleanup(&part))?;
                opened
                    .reader
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .region(0, x, y, w, h)
            }
            None => None,
        };
        let bytes = job
            .tile(samples.as_ref())
            .map_err(|e| e.0)
            .inspect_err(|_| cleanup(&part))?;
        put(&bytes, &mut file).inspect_err(|_| cleanup(&part))?;
        if let Some(w) = warping()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
        {
            w.done = job.share();
        }
        super::tiles::service().wake();
    }
    let grid = job.grid;
    let (bands, sample, alpha) = (job.bands, job.sample, job.alpha);
    let (dirs, head) = job
        .finish()
        .map_err(|e| e.0)
        .inspect_err(|_| cleanup(&part))?;
    put(&dirs, &mut file).inspect_err(|_| cleanup(&part))?;
    let (path, bytes) = match (&output, file) {
        (Output::File(target), Some(mut f)) => {
            f.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
            f.write_all(&head).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
            drop(f);
            if let Some(p) = &part {
                std::fs::rename(p, target)
                    .map_err(|e| format!("“{}” yazılamadı: {e}.", target.display()))?;
            }
            (Some(target.clone()), None)
        }
        _ => {
            memory[..head.len()].copy_from_slice(&head);
            (None, Some(Arc::new(memory)))
        }
    };
    Ok(Warped {
        affine: grid.affine,
        width: grid.width,
        height: grid.height,
        bands,
        sample,
        alpha,
        path,
        bytes,
    })
}

impl App {
    /// The panel of the rasters' long work: each pyramid pass and the resampling, with Durdur.
    pub(crate) fn rasters_jobs_view(&self) -> Option<Element<'_, Message>> {
        let passes = super::pyramid::progress();
        let warp = warping()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        // The point clouds' indexes, made once (pointclouds/index.rs, docs/adr/0207 §4).
        let indexes = crate::pointclouds::index::progress();
        if passes.is_empty() && warp.is_none() && indexes.is_empty() {
            return None;
        }
        let mut list = TaskList::new();
        for p in indexes {
            let key = p.key.clone();
            list = list.push(
                Job_::new(format!("Nokta bulutu dizini: {}", p.name))
                    .detail(format!(
                        "%{} hazır; bitince bulut kat kat çizilir",
                        (p.done * 100.0).round()
                    ))
                    .running(Some(p.done as f32))
                    .on_cancel(Message::PointClouds(crate::pointclouds::Event::StopIndex(
                        key,
                    ))),
            );
        }
        for p in passes {
            let key = p.key.clone();
            list = list.push(
                Job_::new(format!("Önizleme piramidi: {}", p.name))
                    .detail(format!(
                        "%{} hazır; bitince raster her ölçekte hızla çizilir",
                        (p.done * 100.0).round()
                    ))
                    .running(Some(p.done as f32))
                    .on_cancel(Message::Rasters(Rasters::StopPyramid(key))),
            );
        }
        if let Some(w) = warp {
            list = list.push(
                Job_::new(format!("Raster oturtuluyor: {}", w.name))
                    .detail(format!("%{} yeniden örneklendi", (w.done * 100.0).round()))
                    .running(Some(w.done as f32))
                    .on_cancel(Message::Rasters(Rasters::StopWarp)),
            );
        }
        let panel = container(list)
            .width(380)
            .padding(10)
            .style(kentos_ui::style::container::popover);
        Some(
            container(panel)
                .width(Fill)
                .height(Fill)
                .padding([72, 16])
                .align_x(Right)
                .align_y(Bottom)
                .into(),
        )
    }
}

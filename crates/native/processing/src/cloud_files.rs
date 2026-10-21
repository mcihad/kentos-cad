//! The point clouds' and NetCDF files as İşlemler reads them on every host (docs/adr/0572): a cloud's LAS, LAZ or
//! COPC read whole, several pieces decompressed at once on worker threads and handed out in the file's order; a text
//! cloud scanned once and parsed piece by piece; a COPC index built from a LAS or LAZ (its bins beside the result);
//! a NetCDF file's cube with what a slice needs read. The bytes come from the host (`ReadAt`): the desktop reads a
//! linked file, an embedded one or an address; the headless server a linked or an embedded file.

use std::collections::{BTreeMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Arc;

use kentos_commands::CloudFormat;
use kentos_formats::multidim::cube::{Cube as NetCube, Part, Want as CubeWant};
use kentos_formats::raster::ByteStore;
use kentos_pointcloud::copc::Key;
use kentos_pointcloud::index::{BinOut, Builder, Cube, Distribute, Plan};
use kentos_pointcloud::ops::convert::Input;
use kentos_pointcloud::record::{Layout, standard_len, wide_format, widen};
use kentos_pointcloud::source::{Cloud, Opening, Run};
use kentos_pointcloud::{Step, text};

use crate::files::CloudRead;

/// A file's bytes as a host reads them: its size and any range of it.
pub trait ReadAt: Send + Sync {
    fn size(&self) -> u64;
    fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String>;
}

/// A text cloud is read in pieces of this size.
pub const PIECE: u64 = 4 * 1024 * 1024;

/// At most this many bytes of a NetCDF file's header are read.
const NETCDF_HEADER_MOST: u64 = 128 * 1024 * 1024;

/// Threads decompressing a file's pieces at once.
fn threads() -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get().saturating_sub(1))
        .clamp(1, 4)
}

/// A text cloud's plan: the whole file scanned once.
pub fn text_plan(bytes: &dyn ReadAt) -> Result<text::Plan, String> {
    let mut scan = text::Scan::new();
    let size = bytes.size();
    let mut at = 0;
    while at < size {
        let n = PIECE.min(size - at);
        scan.feed(&bytes.read(at, n)?).map_err(|e| e.0)?;
        at += n;
    }
    scan.finish().map_err(|e| e.0)
}

/// A LAS, LAZ or COPC file's header, layout and pieces.
pub fn las_cloud(bytes: &dyn ReadAt) -> Result<Cloud, String> {
    let mut o = Opening::new(bytes.size());
    loop {
        match o.step().map_err(|e| e.0)? {
            Step::Done(c) => return Ok(c),
            Step::Need(n) => o.put(n.offset, bytes.read(n.offset, n.len)?),
        }
    }
}

/// A cloud's file opened for a full read: a text cloud with its plan (`plan`: one scanned before, else scanned now).
pub fn cloud_read<'a>(
    bytes: Box<dyn ReadAt + 'a>,
    format: CloudFormat,
    plan: Option<text::Plan>,
) -> Result<Box<dyn CloudRead + 'a>, String> {
    if format == CloudFormat::Xyz {
        let plan = match plan {
            Some(p) => p,
            None => text_plan(bytes.as_ref())?,
        };
        let input = Input::text(&plan);
        return Ok(Box::new(TextRead {
            input,
            parse: text::Parse::new(plan),
            bytes,
            at: 0,
            finished: false,
        }));
    }
    let cloud = las_cloud(bytes.as_ref())?;
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
        bytes,
        runs,
        next: 0,
        ready: VecDeque::new(),
        total,
        seen: 0,
    }))
}

/// A LAS, LAZ or COPC read whole, its runs decompressed several at once.
struct LasRead<'a> {
    input: Input,
    cloud: Arc<Cloud>,
    bytes: Box<dyn ReadAt + 'a>,
    runs: Vec<Run>,
    next: usize,
    ready: VecDeque<Vec<u8>>,
    total: u64,
    seen: u64,
}

impl CloudRead for LasRead<'_> {
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
            let (cloud, bytes) = (&self.cloud, self.bytes.as_ref());
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
struct TextRead<'a> {
    input: Input,
    parse: text::Parse,
    bytes: Box<dyn ReadAt + 'a>,
    at: u64,
    finished: bool,
}

impl CloudRead for TextRead<'_> {
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

/// The records of a source for an index, handed over in its order (a LAS file's header boxed: it is large).
pub enum Reading<'a> {
    Las(Box<Cloud>, Box<dyn ReadAt + 'a>),
    Text(text::Plan, Box<dyn ReadAt + 'a>),
}

/// A source opened for an index: a text cloud scanned, a LAS, LAZ or COPC's header read.
pub fn reading<'a>(
    bytes: Box<dyn ReadAt + 'a>,
    format: CloudFormat,
) -> Result<Reading<'a>, String> {
    if format == CloudFormat::Xyz {
        let plan = text_plan(bytes.as_ref())?;
        return Ok(Reading::Text(plan, bytes));
    }
    let cloud = las_cloud(bytes.as_ref())?;
    Ok(Reading::Las(Box::new(cloud), bytes))
}

/// The system's WKT for the index: the file's, else the registry's for its EPSG code.
fn wkt_of(cloud: &Cloud) -> Option<String> {
    if let Some(w) = &cloud.crs.wkt {
        return Some(w.clone());
    }
    let epsg = cloud.crs.epsg?;
    let system = kentos_project::crs::system(epsg)?;
    kentos_geometry_core::crs::text::write_wkt(&system.name, &system.transform_system()?, None)
}

fn bin_path(dir: &Path, k: Key) -> std::path::PathBuf {
    dir.join(format!("{}-{}-{}-{}.bin", k.d, k.x, k.y, k.z))
}

fn append(dir: &Path, k: Key, bytes: &[u8]) -> Result<(), String> {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(bin_path(dir, k))
        .map_err(|e| format!("Dizinin kutusu yazılamadı: {e}."))?;
    f.write_all(bytes)
        .map_err(|e| format!("Dizinin kutusu yazılamadı: {e}."))
}

/// The plan of a source.
fn plan_of(reading: &Reading<'_>) -> (Plan, Option<(Layout, Layout)>) {
    match reading {
        Reading::Las(c, _) => {
            let from = c.layout;
            let fmt = wide_format(from.format);
            let to = Layout::new(fmt, standard_len(fmt) + from.extra());
            let mut plan = Plan::new(
                c.bounds(),
                c.head.count,
                c.head.scale,
                c.head.offset,
                fmt,
                from.extra() as u16,
            );
            plan.wkt = wkt_of(c);
            plan.extra_vlr = c
                .vlrs
                .iter()
                .chain(&c.evlrs)
                .find(|v| v.is("LASF_Spec", 4))
                .cloned();
            plan.gps_standard = c.head.gps_standard();
            plan.source_id = c.head.source_id;
            (plan, Some((from, to)))
        }
        Reading::Text(p, _) => (
            Plan::new(p.bounds, p.count, p.scale, p.offset, p.columns.format(), 0),
            None,
        ),
    }
}

/// The index of `reading` written at `part`, its bins in `bins`; `tick` takes the share done (0–1) and says whether
/// to stop (an empty error then).
pub fn build_index(
    reading: Reading<'_>,
    tick: &mut dyn FnMut(f64) -> bool,
    bins: &Path,
    part: &Path,
) -> Result<(), String> {
    let (mut plan, layouts) = plan_of(&reading);
    let threads = threads();
    let mut attempt = 0;
    let summary = loop {
        attempt += 1;
        let _ = std::fs::remove_dir_all(bins);
        std::fs::create_dir_all(bins).map_err(|e| format!("Dizinin kutuları açılamadı: {e}."))?;
        let mut dist = Distribute::new(plan.clone());
        match &reading {
            Reading::Las(cloud, bytes) => {
                let runs = cloud.runs();
                let total = runs.len().max(1);
                let (from, to) = layouts.ok_or("Kaydın düzeni yok.")?;
                let bytes = bytes.as_ref();
                for (b, batch) in runs.chunks(threads * 2).enumerate() {
                    if tick(0.5 * (b * threads * 2).min(total) as f64 / total as f64) {
                        return Err(String::new());
                    }
                    let read: Vec<Result<Vec<u8>, String>> = std::thread::scope(|s| {
                        let handles: Vec<_> = batch
                            .iter()
                            .map(|run| {
                                s.spawn(move || -> Result<Vec<u8>, String> {
                                    let raw = bytes.read(run.need.offset, run.need.len)?;
                                    let mut recs = Vec::new();
                                    cloud.records(run, &raw, &mut recs).map_err(|e| e.0)?;
                                    let mut wide = vec![0u8; to.len * run.count as usize];
                                    for (i, r) in recs.chunks_exact(from.len).enumerate() {
                                        widen(
                                            &from,
                                            r,
                                            &to,
                                            &mut wide[i * to.len..(i + 1) * to.len],
                                        );
                                    }
                                    Ok(wide)
                                })
                            })
                            .collect();
                        handles
                            .into_iter()
                            .map(|h| h.join().unwrap_or_else(|_| Err("Parça çözülemedi.".into())))
                            .collect()
                    });
                    for (run, wide) in batch.iter().zip(read) {
                        for (k, chunk) in dist.add(&wide?, run.first) {
                            append(bins, k, &chunk)?;
                        }
                    }
                }
            }
            Reading::Text(p, bytes) => {
                let mut parse = text::Parse::new(p.clone());
                let size = bytes.size().max(1);
                let mut at = 0;
                let mut first = 0u64;
                let mut out = Vec::new();
                while at < bytes.size() {
                    if tick(0.5 * at as f64 / size as f64) {
                        return Err(String::new());
                    }
                    let n = PIECE.min(bytes.size() - at);
                    out.clear();
                    parse.feed(&bytes.read(at, n)?, &mut out).map_err(|e| e.0)?;
                    at += n;
                    for (k, chunk) in dist.add(&out, first) {
                        append(bins, k, &chunk)?;
                    }
                    first += (out.len() / parse.record_len()) as u64;
                }
                out.clear();
                parse.finish(&mut out).map_err(|e| e.0)?;
                for (k, chunk) in dist.add(&out, first) {
                    append(bins, k, &chunk)?;
                }
            }
        }
        let (rest, summary) = dist.finish();
        for (k, chunk) in rest {
            append(bins, k, &chunk)?;
        }
        if summary.outside && attempt == 1 {
            // The header's bounds were wrong: once more round the points' own (not again).
            plan.cube = Cube::of(summary.bounds);
            continue;
        }
        break summary;
    };
    let mut builder = Builder::new(plan.clone(), summary.clone()).map_err(|e| e.0)?;
    let laz = builder.laz().clone();
    let mut out =
        File::create(part).map_err(|e| format!("“{}” yazılamadı: {e}.", part.display()))?;
    out.write_all(&builder.head().map_err(|e| e.0)?)
        .map_err(|e| e.to_string())?;
    let mut queue: Vec<Key> = summary.bins.keys().copied().collect();
    let total = queue.len().max(1);
    let mut built = 0usize;
    while !queue.is_empty() {
        if tick(0.5 + 0.5 * built as f64 / total as f64) {
            return Err(String::new());
        }
        let take: Vec<Key> = queue.drain(..threads.min(queue.len())).collect();
        let results: Vec<Result<BinOut, String>> = std::thread::scope(|s| {
            let handles: Vec<_> = take
                .iter()
                .map(|&k| {
                    let (plan, laz) = (&plan, &laz);
                    s.spawn(move || -> Result<BinOut, String> {
                        let bytes = std::fs::read(bin_path(bins, k))
                            .map_err(|e| format!("Dizinin kutusu okunamadı: {e}."))?;
                        kentos_pointcloud::index::build_bin(plan, laz, k, &bytes).map_err(|e| e.0)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap_or_else(|_| Err("Kutu kurulamadı.".into())))
                .collect()
        });
        let mut front: Vec<Key> = Vec::new();
        for (k, r) in take.iter().zip(results) {
            match r? {
                BinOut::Built { chunks, up } => {
                    for c in &chunks {
                        out.write_all(&c.bytes).map_err(|e| e.to_string())?;
                    }
                    builder.accept(*k, &chunks, up);
                    let _ = std::fs::remove_file(bin_path(bins, *k));
                    built += 1;
                }
                BinOut::Split(parts) => {
                    let _ = std::fs::remove_file(bin_path(bins, *k));
                    let mut sub: BTreeMap<Key, Vec<u8>> = BTreeMap::new();
                    for (sk, b) in parts {
                        sub.entry(sk).or_default().extend(b);
                    }
                    for (sk, b) in sub {
                        append(bins, sk, &b)?;
                        front.push(sk);
                    }
                }
            }
        }
        // A split bin's parts come next, in their order.
        for (i, k) in front.into_iter().enumerate() {
            queue.insert(i, k);
        }
    }
    let fin = builder.finish().map_err(|e| e.0)?;
    for c in &fin.chunks {
        out.write_all(&c.bytes).map_err(|e| e.to_string())?;
    }
    out.write_all(&fin.tail).map_err(|e| e.to_string())?;
    for (at, b) in &fin.patches {
        out.seek(SeekFrom::Start(*at)).map_err(|e| e.to_string())?;
        out.write_all(b).map_err(|e| e.to_string())?;
    }
    out.sync_all().map_err(|e| e.to_string())?;
    Ok(())
}

/// A COPC of the LAS or LAZ in `from` written at `to` (İşlemler's COPC results, docs/adr/0207 §7): the index, its
/// bins beside `to`; `tick` takes the share and says whether to stop (an empty error then).
pub fn copc_file(
    from: Box<dyn ReadAt + '_>,
    to: &Path,
    tick: &mut dyn FnMut(f64) -> bool,
) -> Result<(), String> {
    let cloud = las_cloud(from.as_ref())?;
    let name = to
        .file_name()
        .map_or_else(|| "bulut".to_owned(), |n| n.to_string_lossy().into_owned());
    let bins = to.with_file_name(format!("{name}.kutular"));
    let part = to.with_file_name(format!("{name}.yaziliyor"));
    let result = build_index(Reading::Las(Box::new(cloud), from), tick, &bins, &part);
    let _ = std::fs::remove_dir_all(&bins);
    match result {
        Ok(()) => {
            std::fs::rename(&part, to).map_err(|e| format!("“{}” yazılamadı: {e}.", to.display()))
        }
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

/// A NetCDF file's cube of its own (a tool steps it alone) with what `part` needs read.
pub fn netcdf_slice(source: &dyn ReadAt, part: &Part) -> Result<NetCube, String> {
    let size = source.size();
    let mut store = ByteStore::new();
    let mut taken = 0u64;
    let mut cube = loop {
        match NetCube::parse(&store, size).map_err(|e| e.0)? {
            kentos_formats::raster::Step::Done(c) => break c,
            kentos_formats::raster::Step::Need(n) => {
                taken += n.len;
                if taken > NETCDF_HEADER_MOST {
                    return Err("NetCDF'in başlığı çok büyük.".into());
                }
                store.put(n.offset, source.read(n.offset, n.len)?);
            }
        }
    };
    for n in cube.needs(CubeWant::Part(part)) {
        if n.len > 1 << 30 {
            return Err("NetCDF'in bir değişkeni 1 GB'tan büyük; okunmuyor.".into());
        }
        let bytes = source.read(n.offset, n.len)?;
        cube.put(n.offset, bytes);
    }
    Ok(cube)
}

/// A NetCDF file's slice `part` opened as a raster (a tool reads a dataset's raster: Kesit, Zaman serisi; docs/adr/0243 §5):
/// the cube with what the slice needs, a mesh's second round of needs too, then its reader.
pub fn netcdf_reader(
    source: &dyn ReadAt,
    part: &Part,
    budget: usize,
) -> Result<kentos_formats::raster::source::Reader, String> {
    let mut cube = netcdf_slice(source, part)?;
    for n in cube.needs(CubeWant::Part(part)) {
        if n.len > 1 << 30 {
            return Err("NetCDF'in bir değişkeni 1 GB'tan büyük; okunmuyor.".into());
        }
        let bytes = source.read(n.offset, n.len)?;
        cube.put(n.offset, bytes);
    }
    cube.open(part, budget).map_err(|e| e.0)
}

//! Where a large drawing's memory goes on the desktop (plan/gis.md CBS-42, docs/adr/0654): the parcels of
//! `perf::drawing`, stage by stage: the synthetic snapshot, the document made of it, then the app taking it (the
//! geometry store, the styled layers' first build and upload, the labels). Each stage's peak is its own (the peak is
//! started over before it) and what it keeps is read after the allocator gave back what was freed (glibc's
//! `malloc_trim`), so a stage's transient memory is not counted as kept. Not a correctness test:
//!
//! ```text
//! KENTOS_PERF_SIZES=250000 cargo test --release -p kentos-desktop --lib perf::memory -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `file_parts` opens a real drawing (`KENTOS_PERF_FILE`, plan/render.md RM-19) as the app does and then lets go of
//! what holds its memory one by one: the drawing area's built scene and styled layers, the geometry store, the
//! document, the GPU. What each step gave back is what that part held:
//!
//! ```text
//! KENTOS_PERF_FILE=.run/rm19/uip.ncz cargo test -p kentos-desktop --lib perf::memory::file_parts -- --ignored --nocapture
//! ```

use super::frame::Harness;
use super::{drawing, memory, reset_peak};
use crate::app::{App, Message};
use crate::document::Document;

fn give_back() {
    crate::give_back::now();
}

/// The Rust heap's live bytes, counted by the test binary's allocator when built with `heap-count`: what the
/// process's own data holds, apart from a GPU driver's allocations and the allocator's free pages.
#[cfg(feature = "heap-count")]
mod heap {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

    pub static LIVE: AtomicUsize = AtomicUsize::new(0);

    pub struct Counting;

    // SAFETY: every call is the system allocator's; only the live count is kept beside it.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let p = unsafe { System.alloc(layout) };
            if !p.is_null() {
                LIVE.fetch_add(layout.size(), Relaxed);
            }
            p
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            let p = unsafe { System.alloc_zeroed(layout) };
            if !p.is_null() {
                LIVE.fetch_add(layout.size(), Relaxed);
            }
            p
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) };
            LIVE.fetch_sub(layout.size(), Relaxed);
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            let p = unsafe { System.realloc(ptr, layout, size) };
            if !p.is_null() {
                LIVE.fetch_add(size, Relaxed);
                LIVE.fetch_sub(layout.size(), Relaxed);
            }
            p
        }
    }

    #[global_allocator]
    static COUNTING: Counting = Counting;
}

/// The Rust heap's live MB, when counted (`heap-count`).
fn rust_heap() -> Option<f64> {
    #[cfg(feature = "heap-count")]
    return Some(heap::LIVE.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1_048_576.0);
    #[cfg(not(feature = "heap-count"))]
    None
}

/// Memory now, after the allocator gave back what was freed, and the peak since the last start, in MB.
fn kept() -> (f64, f64) {
    give_back();
    memory()
}

fn sizes() -> Vec<usize> {
    std::env::var("KENTOS_PERF_SIZES")
        .ok()
        .map(|s| s.split(',').filter_map(|v| v.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![250_000])
}

#[test]
#[ignore = "a measurement; run by hand (see the module's comment)"]
fn memory_stages() {
    use kentos_commands as k;
    use std::mem::size_of;
    println!(
        "Entity {} bayt; türler: Point {}, Line {}, Path {}, Circle {}, Arc {}, Ellipse {}, Spline {}, Nurbs {}, Xline {}, Text {}, Dimension {}, Hatch {}, Insert {}, Leader {}, Table {}, Image {}, Raster {}, PointCloud {}, Wall {}, Opening {}, Space {}, Grid {}, Column {}, Stair {}, Ramp {}, Tag {}, ViewMark {}, Record {}; EntityBase {}",
        size_of::<k::Entity>(),
        size_of::<k::PointEntity>(),
        size_of::<k::LineEntity>(),
        size_of::<k::PathEntity>(),
        size_of::<k::CircleEntity>(),
        size_of::<k::ArcEntity>(),
        size_of::<k::EllipseEntity>(),
        size_of::<k::SplineEntity>(),
        size_of::<k::NurbsEntity>(),
        size_of::<k::ConstructionEntity>(),
        size_of::<k::TextEntity>(),
        size_of::<k::DimensionEntity>(),
        size_of::<k::HatchEntity>(),
        size_of::<k::InsertEntity>(),
        size_of::<k::LeaderEntity>(),
        size_of::<k::TableEntity>(),
        size_of::<k::ImageEntity>(),
        size_of::<k::RasterEntity>(),
        size_of::<k::PointCloudEntity>(),
        size_of::<k::WallEntity>(),
        size_of::<k::OpeningEntity>(),
        size_of::<k::SpaceEntity>(),
        size_of::<k::GridEntity>(),
        size_of::<k::ColumnEntity>(),
        size_of::<k::StairEntity>(),
        size_of::<k::RampEntity>(),
        size_of::<k::TagEntity>(),
        size_of::<k::ViewMarkEntity>(),
        size_of::<k::RecordEntity>(),
        size_of::<k::EntityBase>(),
    );
    for n in sizes() {
        let (start, _) = kept();
        reset_peak();
        let snapshot = drawing(n);
        let (with_snapshot, snapshot_peak) = kept();
        reset_peak();
        let doc = Document::from_v2(snapshot, None).expect("the drawing opens");
        let (with_doc, doc_peak) = kept();
        reset_peak();
        let (mut app, _) = App::boot(None);
        let mut harness = Harness::new();
        harness.settle(&mut app);
        let (booted, _) = kept();
        reset_peak();
        let _ = harness.frame_with(&mut app, &[], move |app| {
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        });
        harness.settle(&mut app);
        let (opened, open_peak) = kept();
        println!(
            "{n} parsel: anlık görüntü {:.0} MB (tepe {:.0}); belge {:.0} MB (tepe {:.0}); uygulama {:.0} MB; açılınca {:.0} MB daha (tepe {:.0}); toplam {:.0} MB",
            with_snapshot - start,
            snapshot_peak - start,
            with_doc - start,
            doc_peak - start,
            booted - with_doc,
            opened - booted,
            open_peak - booted,
            opened
        );
        drop(app);
        give_back();
    }
}

/// The process's memory after the allocator gave back what was freed, in MB: resident, of it the heap (anonymous),
/// the mapped files and the shared memory (a GPU driver's buffers), and the peak since the last start.
fn split() -> [f64; 5] {
    give_back();
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(name))
            .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<f64>().ok())
            .map_or(0.0, |kb| kb / 1024.0)
    };
    [
        field("VmRSS:"),
        field("RssAnon:"),
        field("RssFile:"),
        field("RssShmem:"),
        field("VmHWM:"),
    ]
}

fn said(step: &str, now: [f64; 5], before: Option<[f64; 5]>) {
    if let Some(live) = rust_heap() {
        println!("  Rust yığını: {live:.0} MB");
    }
    let [rss, anon, file, shmem, peak] = now;
    match before {
        Some(b) => println!(
            "{step}: yerleşik {rss:.0} MB (yığın {anon:.0}, dosya {file:.0}, paylaşılan {shmem:.0}; tepe {peak:.0}); \
             verilen {:.0} MB (yığın {:.0}, paylaşılan {:.0})",
            b[0] - rss,
            b[1] - anon,
            b[3] - shmem
        ),
        None => println!(
            "{step}: yerleşik {rss:.0} MB (yığın {anon:.0}, dosya {file:.0}, paylaşılan {shmem:.0}; tepe {peak:.0})"
        ),
    }
}

#[test]
#[ignore = "a measurement; run by hand (see the module's comment)"]
fn file_parts() {
    let Ok(file) = std::env::var("KENTOS_PERF_FILE") else {
        return;
    };
    let (mut app, _) = App::boot(None);
    let mut harness = Harness::new();
    harness.settle(&mut app);
    let boot = split();
    said("açılış öncesi", boot, None);
    // The open as the app makes it, its peak the whole open's; with `KENTOS_PERF_STAGES=1`, stage by stage, each
    // with its own peak and what it kept (the allocator's free pages given back between them): the file read into a
    // document on the open's thread, the app taking it, its first frame (the styled layers built and uploaded),
    // settled.
    reset_peak();
    if std::env::var("KENTOS_PERF_STAGES").as_deref() == Ok("1") {
        let rx = super::large::drive(
            app.start_opening(std::path::PathBuf::from(&file), crate::opening::Purpose::File),
        );
        let done = loop {
            match rx.recv_timeout(std::time::Duration::from_millis(16)) {
                Ok((_, m)) if super::large::is_done(&m) => break m,
                Ok((_, m)) => {
                    harness.frame_with(&mut app, &[], |app| {
                        let _ = app.update(m);
                    });
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    harness.frame(&mut app, &[]);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    panic!("the open ended without an answer")
                }
            }
        };
        let read = split();
        said("dosya belgeye okundu", read, Some(boot));
        reset_peak();
        let _ = app.update(done);
        if app.opening.is_some() {
            let rx = super::large::drive(app.opening_event(crate::opening::Event::ConfirmUnit));
            while let Ok((_, m)) = rx.recv_timeout(std::time::Duration::from_secs(600)) {
                let _ = app.update(m);
            }
        }
        let taken = split();
        said("uygulama belgeyi aldı", taken, Some(read));
        reset_peak();
        harness.frame(&mut app, &[]);
        let first = split();
        said("ilk kare", first, Some(taken));
        reset_peak();
        app.dialog = None;
        harness.settle(&mut app);
    } else {
        let opened = super::large::open(&mut app, &mut harness, std::path::Path::new(&file));
        let (resident, peak) = super::memory();
        println!("  açılış: {opened}");
        println!("açıldığı gibi: yerleşik {resident:.0} MB, tepe {peak:.0} MB");
    }
    harness.frame(&mut app, &[]);
    let open = split();
    said("çizim açık", open, None);
    println!(
        "  nesne {}; depo: Item {} bayt, Shape {} bayt, {} nesne",
        app.document.as_ref().map_or(0, |d| d.model.entities().count()),
        std::mem::size_of::<kentos_geometry_core::store::Item>(),
        std::mem::size_of::<kentos_geometry_core::entity::Shape>(),
        app.spatial.store().len(),
    );
    let mb = |b: usize| b as f64 / 1_048_576.0;
    if let Some(doc) = app.document.as_ref() {
        let mut kinds: std::collections::BTreeMap<&str, (usize, usize)> = Default::default();
        for e in doc.model.entities() {
            let k = kinds.entry(e.kind()).or_default();
            k.0 += 1;
            k.1 += std::mem::size_of_val(e);
        }
        let mut kinds: Vec<_> = kinds.into_iter().collect();
        kinds.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
        println!(
            "  nesne türleri ({} bayt her biri): {}",
            std::mem::size_of::<kentos_commands::Entity>(),
            kinds
                .iter()
                .map(|(k, (n, _))| format!("{k} {n}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let [data, low, batches, fills, segments, markers] = app.viewport.built_bytes();
    println!(
        "  stilli katmanlar: veri {:.0} MB, artık {:.0} MB, toplu {:.0} MB; düz sahne: dolgu {:.0} MB, kesim {:.0} MB, işaret {:.0} MB",
        mb(data),
        mb(low),
        mb(batches),
        mb(fills),
        mb(segments),
        mb(markers)
    );
    app.viewport.forget_built();
    let scene = split();
    said("sahne ve stilli katmanlar bırakıldı", scene, Some(open));
    app.label_spots = Default::default();
    app.gpu_layers = Default::default();
    let labels = split();
    said("yazı ve katman resimleri bırakıldı", labels, Some(scene));
    app.spatial = Default::default();
    let store = split();
    said("geometri deposu bırakıldı", store, Some(labels));
    app.document = None;
    let document = split();
    said("belge bırakıldı", document, Some(store));
    drop(harness);
    let gpu = split();
    said("GPU bırakıldı", gpu, Some(document));
    drop(app);
    let end = split();
    said("uygulama bırakıldı", end, Some(gpu));
}

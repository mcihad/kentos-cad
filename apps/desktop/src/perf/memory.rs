//! Where a large drawing's memory goes on the desktop (plan/gis.md CBS-42, docs/adr/0654): the parcels of
//! `perf::drawing`, stage by stage: the synthetic snapshot, the document made of it, then the app taking it (the
//! geometry store, the styled layers' first build and upload, the labels). Each stage's peak is its own (the peak is
//! started over before it) and what it keeps is read after the allocator gave back what was freed (glibc's
//! `malloc_trim`), so a stage's transient memory is not counted as kept. Not a correctness test:
//!
//! ```text
//! KENTOS_PERF_SIZES=250000 cargo test --release -p kentos-desktop --lib perf::memory -- --ignored --nocapture --test-threads=1
//! ```

use super::frame::Harness;
use super::{drawing, memory, reset_peak};
use crate::app::{App, Message};
use crate::document::Document;

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn give_back() {
    unsafe extern "C" {
        fn malloc_trim(pad: usize) -> i32;
    }
    // SAFETY: glibc's malloc_trim takes a pad and only returns free memory to the system.
    unsafe {
        malloc_trim(0);
    }
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn give_back() {}

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

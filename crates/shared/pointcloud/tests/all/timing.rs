//! What decoding a picture's nodes costs (docs/adr/0207 §6, §12): every node
//! of the measured synthetic COPC (`perf::clouds` of the desktop writes it to
//! `.run/pointcloud-perf/`) decompressed with each choice of LAS 1.4's
//! layers, then decoded and coloured. Not a correctness test and not run by
//! default:
//!
//! ```text
//! cargo test --release -p kentos-pointcloud --test all timing -- --ignored --nocapture
//! ```

use std::path::Path;
use std::time::Instant;

use kentos_contracts::{CloudRender, PointCloudStyle, PointShape, PointSizeUnit};
use kentos_pointcloud::chunks::{DecompressionSelection, decompress_selected, view_selection};
use kentos_pointcloud::{look, nodes};

use crate::host::open;

fn style(render: CloudRender) -> PointCloudStyle {
    PointCloudStyle {
        render,
        ramp: None,
        invert: false,
        min: Some(835.0),
        max: Some(885.0),
        hidden: Vec::new(),
        rgb8: false,
        size: 2.0,
        size_unit: PointSizeUnit::Px,
        shape: PointShape::Round,
    }
}

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn timing() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../.run/pointcloud-perf/sentetik-4000000.copc.laz");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("{} yok: önce perf::clouds'u çalıştırın.", path.display());
        return;
    };
    let copc = open(&bytes).expect("a COPC");
    let laz = copc.laz.clone().expect("compressed");
    let hierarchy = copc.hierarchy.as_ref().expect("a hierarchy");
    let runs: Vec<_> = hierarchy
        .sorted()
        .into_iter()
        .filter_map(|n| copc.node_run(n.key))
        .collect();
    let total: u64 = runs.iter().map(|r| r.count).sum();
    let per = |s: f64| s / total as f64 * 50_000.0 * 1000.0;
    let base = DecompressionSelection::base().decompress_z();
    let mut recs = Vec::new();
    for (what, sel) in [
        ("bütün alanlar", DecompressionSelection::all()),
        (
            "görünüşün alanları (konum, sınıf, yoğunluk, renk)",
            view_selection(),
        ),
        ("konum", base),
        ("konum ve renk", base.decompress_rgb()),
        ("konum ve sınıf", base.decompress_classification()),
        ("konum ve yoğunluk", base.decompress_intensity()),
    ] {
        let mut best = f64::INFINITY;
        for _ in 0..3 {
            let t = Instant::now();
            for run in &runs {
                let a = run.need.offset as usize;
                let b = a + run.need.len as usize;
                recs.clear();
                decompress_selected(&laz, &bytes[a..b], run.count as usize, &mut recs, sel)
                    .expect("decodes");
            }
            best = best.min(t.elapsed().as_secs_f64());
        }
        println!(
            "Çözme, {what}: {:.2} milyon nokta/s; 50 000 nokta {:.1} ms",
            total as f64 / best / 1e6,
            per(best)
        );
    }
    // Writing every point again as a LAZ: one chunk after the other, then whole chunks on threads
    // (as İşlemler's results go: kentos-processing's compress_parallel does the same).
    let mut all = Vec::new();
    for run in &runs {
        let a = run.need.offset as usize;
        let b = a + run.need.len as usize;
        copc.records(run, &bytes[a..b], &mut all).expect("decodes");
    }
    let spec = kentos_pointcloud::write::Spec::like(&copc.head, &copc.vlrs, &copc.evlrs, true);
    let len = copc.layout.len;
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get().saturating_sub(1))
        .clamp(1, 4);
    let mut on_threads = |items: &[kentos_pointcloud::chunks::LazItem], chunks: &[&[u8]]| {
        std::thread::scope(|sc| {
            let handles: Vec<_> = chunks
                .iter()
                .map(|c| sc.spawn(move || kentos_pointcloud::chunks::compress(items, c)))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("compresses"))
                .collect::<kentos_pointcloud::Result<Vec<_>>>()
        })
    };
    let t = Instant::now();
    let one = kentos_pointcloud::write::write_all(spec.clone(), &all).expect("writes");
    let single = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let (mut w, mut many) = kentos_pointcloud::write::Writer::new(spec).expect("a writer");
    for piece in all.chunks(len * 50_000) {
        many.extend(
            w.put_with(piece, threads, &mut on_threads)
                .expect("compresses"),
        );
    }
    let (tail, patches) = w.finish_with(&mut on_threads).expect("finishes");
    many.extend(tail);
    for (at, b) in patches {
        many[at as usize..at as usize + b.len()].copy_from_slice(&b);
    }
    let parallel = t.elapsed().as_secs_f64();
    assert!(many == one, "the same bytes");
    println!(
        "LAZ yazma: tek iş parçacığı {:.2} milyon nokta/s; {threads} iş parçacığıyla {:.2} milyon nokta/s",
        total as f64 / single / 1e6,
        total as f64 / parallel / 1e6
    );
    drop(all);
    // Each look's node: its layers decompressed, decoded into the picture's points and coloured.
    let mut rgba = Vec::new();
    for render in [
        CloudRender::Rgb,
        CloudRender::Classification,
        CloudRender::Elevation,
        CloudRender::Intensity,
        CloudRender::Returns,
    ] {
        let st = style(render);
        let needs = nodes::Needs::of(&st);
        let mut best = f64::INFINITY;
        for _ in 0..3 {
            let t = Instant::now();
            for run in &runs {
                let a = run.need.offset as usize;
                let b = a + run.need.len as usize;
                recs.clear();
                copc.view_records(run, &bytes[a..b], &mut recs, needs)
                    .expect("decodes");
                let p = nodes::decode(
                    &copc.layout,
                    &recs,
                    copc.head.scale,
                    copc.head.offset,
                    [0.0; 3],
                    needs,
                );
                rgba.clear();
                look::colours(&st, &p, [128; 3], &mut rgba);
            }
            best = best.min(t.elapsed().as_secs_f64());
        }
        println!(
            "Düğüm (çözme, noktalar, renkler), {render:?}: 50 000 nokta {:.1} ms",
            per(best)
        );
    }
}

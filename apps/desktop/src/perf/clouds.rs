//! Point clouds on the desktop, measured (docs/adr/0207 §12): a synthetic
//! airborne LiDAR of `KENTOS_PERF_POINTS` points (4 000 000 by default:
//! 1 km² at 4 points a square metre, LAS 1.2 point format 3 with colours,
//! ground with hills, buildings, trees with several returns and some low
//! noise), written as a LAZ once into the worktree's `.run/pointcloud-perf/`
//! (never committed, never the user's folders). Then the index made (the
//! cache's builder), the first picture's work (the COPC opened, its root
//! decoded and coloured), the nodes decoded and coloured, the whole file read
//! as İşlemler reads it, the operations' cores over every point, the LAZ
//! written, and the frames of a drawing that shows the cloud (fitted: until
//! every node the view asks for is drawn, a pan with the middle button and
//! the wheel's zoom), as `perf::frame` measures them. Not a correctness test
//! and not run by default:
//!
//! ```text
//! cargo test --release -p kentos-desktop perf::clouds -- --ignored --nocapture --test-threads=1
//! ```

use std::collections::{BTreeMap, HashMap};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use iced::advanced::renderer::Headless as _;
use iced::{Event, Point, mouse};
use kentos_contracts::{
    CloudFormat, CloudRender, CloudSource, DocumentSnapshotV2, Entity, EntityBase, EntityId,
    PointCloudEntity, PointCloudFields, PointCloudStyle, PointShape, PointSizeUnit, Vec2,
};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::pointcloud::visible;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_pointcloud::copc::Key;
use kentos_pointcloud::ops::ground::{self, Ground};
use kentos_pointcloud::ops::height::{self, Height};
use kentos_pointcloud::ops::raster::{Frame, Occupancy, Rasterize, Value as RasterValue};
use kentos_pointcloud::ops::region::Region;
use kentos_pointcloud::ops::thin::Cells;
use kentos_pointcloud::record::Layout;
use kentos_pointcloud::write::{Spec, Writer};
use kentos_processing::files::Files;
use serde_json::Value;

use super::frame::{Harness, Parts, moved, pan, summary};
use super::{environment, median, ms};
use crate::app::{App, Message};
use crate::document::Document;
use crate::pointclouds::bytes::Bytes;
use crate::pointclouds::files::DesktopFiles;
use crate::pointclouds::service::{self, open_copc};

/// The ground's side, metres.
const SIDE: f64 = 1000.0;
/// Its lower left corner.
const X0: f64 = 487_100.0;
const Y0: f64 = 4_420_100.0;
/// Point format 3's record: XYZ, intensity, returns, class, angle, user, source, GPS time, RGB.
const FORMAT: u8 = 3;
const RECORD: usize = 34;

/// A small deterministic generator (xorshift64*).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// 0..1.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn mix(a: u64, b: u64) -> u64 {
    let mut h = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 31;
    h.wrapping_mul(0x1656_67B1_9E37_79F9)
}

/// The ground's height at (x, y) from the lower left: hills and a little roughness.
fn ground_z(x: f64, y: f64) -> f64 {
    850.0 + 15.0 * (x / 180.0).sin() * (y / 140.0).cos() + 3.0 * (x / 23.0 + y / 31.0).sin()
}

/// What stands at (x, y): a building's height, or a tree's crown height, or nothing.
enum Over {
    Building(f64),
    Tree(f64),
    Open,
}

fn over(x: f64, y: f64) -> Over {
    // 40 m lots; a building of 18 × 14 m on four lots in ten, trees on the others.
    let (i, j) = ((x / 40.0).floor() as u64, (y / 40.0).floor() as u64);
    let (u, v) = (x - i as f64 * 40.0, y - j as f64 * 40.0);
    let h = mix(i, j);
    if h % 10 < 4 {
        if (11.0..29.0).contains(&u) && (13.0..27.0).contains(&v) {
            return Over::Building(6.0 + (h >> 8) as f64 % 9.0);
        }
        return Over::Open;
    }
    // Up to three trees a lot, crowns of 2–5 m.
    for t in 0..(h % 4) {
        let k = mix(h, t);
        let cx = 4.0 + (k % 32) as f64;
        let cy = 4.0 + ((k >> 8) % 32) as f64;
        let r = 2.0 + ((k >> 16) % 30) as f64 / 10.0;
        let d2 = (u - cx).powi(2) + (v - cy).powi(2);
        if d2 < r * r {
            let top = 6.0 + ((k >> 24) % 120) as f64 / 10.0;
            return Over::Tree(top * (1.0 - 0.4 * d2 / (r * r)));
        }
    }
    Over::Open
}

/// The `i`th point's record (`rng` its own generator's state).
fn record(i: u64, rng: &mut Rng, out: &mut [u8]) {
    let (x, y) = (rng.unit() * SIDE, rng.unit() * SIDE);
    let g = ground_z(x, y);
    let (mut z, class, ret, rets, intensity, rgb) = match over(x, y) {
        Over::Building(h) => (
            g + h + 0.3 * (x % 18.0) / 18.0,
            6u8,
            1u8,
            1u8,
            160u16,
            [182u16, 74, 58],
        ),
        Over::Tree(h) => {
            let rets = 1 + (rng.next() % 3) as u8;
            let ret = 1 + (rng.next() % u64::from(rets)) as u8;
            if ret == rets && rets > 1 && rng.unit() < 0.6 {
                (g, 2, ret, rets, 60, [112, 96, 70])
            } else {
                (
                    g + h * (0.55 + 0.45 * rng.unit()),
                    5,
                    ret,
                    rets,
                    90,
                    [46, 112, 52],
                )
            }
        }
        Over::Open => (g, 2, 1, 1, 110, [140, 150, 96]),
    };
    let class = if rng.next().is_multiple_of(1000) {
        z = g - 4.0 - 6.0 * rng.unit();
        7
    } else {
        class
    };
    z += 0.03 * (rng.unit() - 0.5);
    let put_i32 = |out: &mut [u8], at: usize, v: f64| {
        out[at..at + 4].copy_from_slice(&((v / 0.01).round() as i32).to_le_bytes());
    };
    put_i32(out, 0, X0 + x - 487_000.0);
    put_i32(out, 4, Y0 + y - 4_420_000.0);
    put_i32(out, 8, z);
    out[12..14].copy_from_slice(&(intensity + (rng.next() % 40) as u16).to_le_bytes());
    out[14] = (ret & 7) | ((rets & 7) << 3);
    out[15] = class;
    out[16] = ((x / SIDE - 0.5) * 30.0) as i8 as u8;
    out[17] = 0;
    out[18..20].copy_from_slice(&(1 + (y / 250.0) as u16).to_le_bytes());
    out[20..28].copy_from_slice(&(3.0e8 + i as f64 * 1e-5).to_le_bytes());
    for (k, c) in rgb.iter().enumerate() {
        let c = (u32::from(*c) * 257)
            .saturating_add((rng.next() % 2000) as u32)
            .min(65_535) as u16;
        out[28 + 2 * k..30 + 2 * k].copy_from_slice(&c.to_le_bytes());
    }
}

/// The synthetic LAZ of `n` points at `path`; the seconds its writing took (once).
fn synthetic(n: u64, path: &Path) -> Option<f64> {
    if path.is_file() {
        return None;
    }
    let spec = Spec {
        minor: 2,
        format: FORMAT,
        record_len: RECORD as u16,
        scale: [0.01; 3],
        offset: [487_000.0, 4_420_000.0, 0.0],
        vlrs: Vec::new(),
        evlrs: Vec::new(),
        compressed: true,
        source_id: 0,
        global_encoding: 1,
        software: "KentOS CAD ölçüm".into(),
    };
    let part = path.with_extension("yaziliyor");
    let mut file = std::io::BufWriter::new(std::fs::File::create(&part).expect("creates the file"));
    let t = Instant::now();
    let (mut w, first) = Writer::new(spec).expect("a writer");
    file.write_all(&first).expect("writes");
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut batch = vec![0u8; 1_000_000 * RECORD];
    let mut i = 0;
    while i < n {
        let k = (n - i).min(1_000_000) as usize;
        for (j, r) in batch[..k * RECORD].chunks_exact_mut(RECORD).enumerate() {
            record(i + j as u64, &mut rng, r);
        }
        file.write_all(&w.put(&batch[..k * RECORD]).expect("compresses"))
            .expect("writes");
        i += k as u64;
    }
    let (tail, patches) = w.finish().expect("finishes");
    file.write_all(&tail).expect("writes");
    let mut file = file.into_inner().expect("flushes");
    for (at, bytes) in patches {
        file.seek(SeekFrom::Start(at)).expect("seeks");
        file.write_all(&bytes).expect("writes");
    }
    file.sync_all().expect("syncs");
    drop(file);
    std::fs::rename(&part, path).expect("renames");
    Some(t.elapsed().as_secs_f64())
}

fn rate(points: u64, seconds: f64) -> String {
    format!("{:.2} milyon nokta/s", points as f64 / seconds / 1e6)
}

/// The cloud's source as a drawing holds it.
fn source(path: &Path, format: CloudFormat, count: u64, bounds: [f64; 6]) -> CloudSource {
    CloudSource {
        asset: None,
        file: Some(path.display().to_string()),
        url: None,
        format,
        count,
        bounds,
    }
}

fn style(render: CloudRender) -> PointCloudStyle {
    PointCloudStyle {
        render,
        ramp: None,
        invert: false,
        min: (render == CloudRender::Elevation).then_some(835.0),
        max: (render == CloudRender::Elevation).then_some(885.0),
        hidden: Vec::new(),
        rgb8: false,
        size: 2.0,
        size_unit: PointSizeUnit::Px,
        shape: PointShape::Round,
    }
}

/// perf.rs's drawing without parcels, the cloud on its layer.
fn drawing(cloud: PointCloudFields) -> DocumentSnapshotV2 {
    let mut doc = super::drawing(0);
    doc.name = "bulut-olcum".into();
    doc.origin = Vec2 { x: X0, y: Y0 };
    doc.entities.push(Entity::PointCloud(PointCloudEntity {
        base: EntityBase {
            id: 1,
            layer_id: doc.active_layer.clone(),
            color: None,
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
        },
        cloud,
    }));
    doc.uids.push(EntityId([
        0x01, 0x92, 0xf5, 0xa0, 0x7c, 0x3e, 0x70, 0x00, 0x80, 0, 0, 0, 0, 0, 0, 1,
    ]));
    doc
}

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn clouds() {
    let n: u64 = std::env::var("KENTOS_PERF_POINTS")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(4_000_000);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join(".run/pointcloud-perf");
    std::fs::create_dir_all(&dir).expect("the folder");
    let laz = dir.join(format!("sentetik-{n}.laz"));
    let copc = dir.join(format!("sentetik-{n}.copc.laz"));
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut row = |what: &str, value: String| {
        println!("{what}: {value}");
        rows.push((what.to_owned(), value));
    };
    if let Some(s) = synthetic(n, &laz) {
        row(
            "LAZ yazma, üretimle (tek iş parçacığı)",
            format!("{s:.2} s, {}", rate(n, s)),
        );
    }
    let size = std::fs::metadata(&laz).map_or(0, |m| m.len());
    row(
        "Sentetik bulut",
        format!(
            "{n} nokta, LAS 1.2 biçim 3, LAZ {:.1} MB",
            size as f64 / 1e6
        ),
    );

    // The index, as the cache makes it once (copc_file is its builder).
    let _ = std::fs::remove_file(&copc);
    let t = Instant::now();
    crate::pointclouds::index::copc_file(&laz, &copc, &mut |_| false).expect("the index");
    let s = t.elapsed().as_secs_f64();
    row(
        "Dizin (COPC) hazırlığı",
        format!("{s:.2} s, {}", rate(n, s)),
    );

    // The first picture's work: the COPC opened (header, hierarchy, octree), its root decoded and coloured.
    let mut firsts = Vec::new();
    let mut root_points = 0;
    for _ in 0..7 {
        let t = Instant::now();
        let opened = open_copc(Bytes::file(&copc).expect("opens")).expect("a COPC");
        let run = opened.cloud.node_run(Key::ROOT).expect("the root");
        let raw = opened
            .bytes
            .read(run.need.offset, run.need.len)
            .expect("reads");
        let mut recs = Vec::new();
        let needs = kentos_pointcloud::nodes::Needs::of(&style(CloudRender::Rgb));
        opened
            .cloud
            .view_records(&run, &raw, &mut recs, needs)
            .expect("decodes");
        let center = opened.tree.node_center([0, 0, 0, 0]);
        let points = kentos_pointcloud::nodes::decode(
            &opened.cloud.layout,
            &recs,
            opened.cloud.head.scale,
            opened.cloud.head.offset,
            center,
            needs,
        );
        let mut rgba = Vec::new();
        kentos_pointcloud::look::colours(&style(CloudRender::Rgb), &points, [128; 3], &mut rgba);
        root_points = points.len();
        firsts.push(ms(t));
    }
    row(
        "İlk görüntü: COPC'yi açma, kök düğümü çözme ve renkleme",
        format!(
            "{:.1} ms (ortanca, kökte {root_points} nokta)",
            median(firsts)
        ),
    );

    // Every node decoded and coloured, as the workers do: the rate, and a 50 000-point node's time.
    let opened = open_copc(Bytes::file(&copc).expect("opens")).expect("a COPC");
    let tree = opened.tree.clone();
    let mut decode_s = 0.0;
    let mut whole_s = 0.0;
    let mut colour_s = 0.0;
    let mut points_total = 0u64;
    let mut largest = 0usize;
    let mut recs = Vec::new();
    let mut rgba = Vec::new();
    let elevation_needs = kentos_pointcloud::nodes::Needs::of(&style(CloudRender::Elevation));
    for (i, k) in tree.keys.iter().enumerate() {
        if tree.counts[i] == 0 {
            continue;
        }
        let key = Key {
            d: k[0],
            x: k[1],
            y: k[2],
            z: k[3],
        };
        let Some(run) = opened.cloud.node_run(key) else {
            continue;
        };
        let raw = opened
            .bytes
            .read(run.need.offset, run.need.len)
            .expect("reads");
        // Every field, as İşlemler and XYZ sor read a node.
        let t = Instant::now();
        recs.clear();
        opened
            .cloud
            .records(&run, &raw, &mut recs)
            .expect("decodes");
        whole_s += t.elapsed().as_secs_f64();
        // What the looks read, as the picture's workers decode it.
        let t = Instant::now();
        recs.clear();
        opened
            .cloud
            .view_records(&run, &raw, &mut recs, elevation_needs)
            .expect("decodes");
        let points = kentos_pointcloud::nodes::decode(
            &opened.cloud.layout,
            &recs,
            opened.cloud.head.scale,
            opened.cloud.head.offset,
            tree.node_center(*k),
            elevation_needs,
        );
        decode_s += t.elapsed().as_secs_f64();
        let t = Instant::now();
        rgba.clear();
        kentos_pointcloud::look::colours(
            &style(CloudRender::Elevation),
            &points,
            [128; 3],
            &mut rgba,
        );
        colour_s += t.elapsed().as_secs_f64();
        points_total += points.len() as u64;
        largest = largest.max(points.len());
    }
    let per = |s: f64| s / points_total as f64 * 50_000.0 * 1000.0;
    // Each layer's cost by itself: kentos-pointcloud's own timing (tests/all/timing.rs).
    row(
        "Düğüm çözme ve renkleme, yükseklik görünüşü (tek iş parçacığı)",
        format!(
            "{} düğüm, en büyüğü {largest} nokta; konumları çözme {}, renkleme {}; 50 000 noktalı düğüm {:.1} ms ({:.1} + {:.1}); bütün alanları çözme {} (50 000 nokta {:.1} ms)",
            tree.keys.len(),
            rate(points_total, decode_s),
            rate(points_total, colour_s),
            per(decode_s + colour_s),
            per(decode_s),
            per(colour_s),
            rate(points_total, whole_s),
            per(whole_s),
        ),
    );

    // The frame's choice of nodes: every node of the fitted view at 1440 × 900 device pixels.
    let mut picked = Vec::new();
    let px_per_m = 900.0 / SIDE;
    let view = [X0, Y0, X0 + SIDE * 1.6, Y0 + SIDE];
    let t = Instant::now();
    for _ in 0..100 {
        visible(&tree, view, px_per_m, 2.0, 4_000_000, &mut picked);
    }
    let shown: u64 = picked.iter().map(|&i| tree.counts[i as usize]).sum();
    row(
        "Karenin düğüm seçimi (sığdırılmış görünüm)",
        format!(
            "{:.3} ms, {} düğüm, {shown} nokta",
            ms(t) / 100.0,
            picked.len()
        ),
    );

    // The whole file read as İşlemler reads it: the LAZ's chunks on several threads, and the COPC.
    let files = DesktopFiles::new(Some(dir.clone()), HashMap::new());
    let bounds = {
        let opened = open_copc(Bytes::file(&copc).expect("opens")).expect("a COPC");
        opened.cloud.bounds()
    };
    let mut all = Vec::with_capacity(n as usize * RECORD);
    let mut read = |path: &Path, format: CloudFormat, keep: bool| -> f64 {
        let src = source(path, format, n, bounds);
        let t = Instant::now();
        let mut r = files.open_cloud(&src).expect("opens");
        let mut out = Vec::new();
        let mut count = 0u64;
        let len = r.input().layout.len;
        while r.next(&mut out).expect("reads") {
            count += (out.len() / len) as u64;
            if keep {
                all.extend_from_slice(&out);
            }
            out.clear();
        }
        let s = t.elapsed().as_secs_f64();
        assert_eq!(count, n, "every point read");
        s
    };
    let s = read(&laz, CloudFormat::Laz, true);
    row("Tam okuma: LAZ", format!("{s:.2} s, {}", rate(n, s)));
    let s = read(&copc, CloudFormat::Copc, false);
    row("Tam okuma: COPC", format!("{s:.2} s, {}", rate(n, s)));

    // The operations' cores over every point (the records in memory; reading and writing apart).
    let layout = Layout::new(FORMAT, RECORD);
    let (scale, offset) = ([0.01; 3], [487_000.0, 4_420_000.0, 0.0]);
    let plan = [bounds[0], bounds[1], bounds[3], bounds[4]];
    let mut timed = |what: &str, f: &mut dyn FnMut() -> String| {
        let t = Instant::now();
        let said = f();
        let s = t.elapsed().as_secs_f64();
        row(what, format!("{s:.2} s, {}; {said}", rate(n, s)));
    };
    let mut classed = all.clone();
    timed(
        "Zemin süzgeci (SMRF, varsayılanlar): iki geçiş",
        &mut || {
            let mut g = Ground::new(plan, ground::Params::default()).expect("a ground");
            g.feed(&layout, &classed, scale, offset);
            assert!(g.surface());
            let k = g.classify(&layout, &mut classed, scale, offset);
            format!("{k} zemin noktası")
        },
    );
    timed("Yüksekliğe göre sınıfla: iki geçiş", &mut || {
        let mut h = Height::new(plan, height::Params::default()).expect("a surface");
        h.feed(&layout, &classed, scale, offset);
        assert!(h.surface());
        let k = h.classify(&layout, &mut classed, scale, offset);
        format!("{} sınıflanan", k.iter().sum::<u64>())
    });
    timed("Seyrelt (0,5 m'lik hücreler)", &mut || {
        let mut c = Cells::new(0.5).expect("cells");
        c.feed(&layout, &all, scale, offset, 0);
        format!("{} nokta kaldı", c.kept().len())
    });
    timed("Rasterleştir (1 m, ortalama)", &mut || {
        let frame = Frame::over(plan, 1.0).expect("a frame");
        let mut r = Rasterize::new(frame, RasterValue::Mean, None);
        r.feed(&layout, &all, scale, offset);
        let v = r.values();
        format!("{} × {} hücre", frame.cols, frame.rows.min(v.len()))
    });
    timed("Sınır çıkar (2 m)", &mut || {
        let frame = Frame::over(plan, 2.0).expect("a frame");
        let mut o = Occupancy::new(frame);
        o.feed(&layout, &all, scale, offset);
        let parts = kentos_pointcloud::ops::boundary::parts(&frame, &o.filled(1));
        format!("{} parça", parts.len())
    });
    let circle = Shape::Polygon {
        pts: (0..64)
            .map(|k| {
                let a = f64::from(k) / 64.0 * std::f64::consts::TAU;
                kentos_geometry_core::vec2::Vec2::new(
                    X0 + 500.0 + 300.0 * a.cos(),
                    Y0 + 500.0 + 300.0 * a.sin(),
                )
            })
            .collect(),
        bulges: None,
        holes: None,
        parts: None,
    };
    let region = Region::new(&areas_of_entity(&circle)).expect("a region");
    timed("Bulutu kırp (64 köşeli alanın içi)", &mut || {
        let index = region.index();
        let mut inside = 0u64;
        for r in all.chunks_exact(RECORD) {
            let p = kentos_geometry_core::vec2::Vec2::new(
                f64::from(layout.x(r)) * scale[0] + offset[0],
                f64::from(layout.y(r)) * scale[1] + offset[1],
            );
            inside += u64::from(region.contains(&index, p));
        }
        format!("{inside} nokta içeride")
    });
    timed("LAZ yazma (tek iş parçacığı)", &mut || {
        let spec = Spec {
            minor: 2,
            format: FORMAT,
            record_len: RECORD as u16,
            scale,
            offset,
            vlrs: Vec::new(),
            evlrs: Vec::new(),
            compressed: true,
            source_id: 0,
            global_encoding: 1,
            software: "KentOS CAD ölçüm".into(),
        };
        let (mut w, mut bytes) = Writer::new(spec).expect("a writer");
        bytes.extend(w.put(&classed).expect("compresses"));
        let (tail, _) = w.finish().expect("finishes");
        format!("{:.1} MB", (bytes.len() + tail.len()) as f64 / 1e6)
    });
    timed(
        "LAZ yazma (İşlemler gibi, iş parçacıklarıyla)",
        &mut || {
            let spec = Spec {
                minor: 2,
                format: FORMAT,
                record_len: RECORD as u16,
                scale,
                offset,
                vlrs: Vec::new(),
                evlrs: Vec::new(),
                compressed: true,
                source_id: 0,
                global_encoding: 1,
                software: "KentOS CAD ölçüm".into(),
            };
            let (mut w, mut bytes) = Writer::new(spec).expect("a writer");
            use kentos_processing::builtin::pointcloud::{compress_parallel, compress_threads};
            // As İşlemler's results go: in a file's pieces, whole chunks compressed at once.
            for piece in classed.chunks(RECORD * 50_000) {
                bytes.extend(
                    w.put_with(piece, compress_threads(), &mut compress_parallel)
                        .expect("compresses"),
                );
            }
            let (tail, _) = w.finish_with(&mut compress_parallel).expect("finishes");
            format!("{:.1} MB", (bytes.len() + tail.len()) as f64 / 1e6)
        },
    );
    drop(all);
    drop(classed);

    // The frames: the drawing with the cloud (the COPC linked), fitted.
    let cloud = PointCloudFields {
        sources: vec![source(&copc, CloudFormat::Copc, n, bounds)],
        bounds,
        count: n,
        srid: 5256,
        style: style(CloudRender::Rgb),
        opacity: None,
    };
    let (mut app, _) = App::boot(None);
    let doc = Document::from_v2(drawing(cloud), None).expect("the drawing opens");
    let mut h = Harness::new();
    h.settle(&mut app);
    let first = h.frame_with(&mut app, &[], move |app| {
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    });
    h.settle(&mut app);
    let area = app.viewport.bounds;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: bounds[0],
            min_y: bounds[1],
            max_x: bounds[3],
            max_y: bounds[4],
        },
        24.0,
    );
    let mut table: Vec<(&str, Vec<Parts>)> = vec![("Çizimi açma (ilk kare)", vec![first])];
    // Until every node the view asks for is drawn: frames while the workers make nodes.
    let t = Instant::now();
    let mut loading = Vec::new();
    let mut quiet = 0;
    for _ in 0..600 {
        loading.push(h.frame(&mut app, &[]));
        service::service().settle(Duration::from_secs(30));
        if service::service().idle() {
            quiet += 1;
            if quiet >= 8 {
                break;
            }
        } else {
            quiet = 0;
        }
    }
    let full = t.elapsed().as_secs_f64();
    row(
        "Sığdırılmış görünümün bütün düğümleri çizilene dek",
        format!("{:.2} s, {} kare", full, loading.len()),
    );
    table.push(("Düğümler gelirken", loading));
    let center = area.center();
    table.push((
        "Boş kare (yeniden çizim)",
        (0..30).map(|_| h.frame(&mut app, &[])).collect(),
    ));
    h.frame(&mut app, &[moved(center)]);
    table.push(("Orta tuşla kaydırma", pan(&mut app, &mut h, center)));
    let wheel = |lines: f32| {
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: lines },
        })
    };
    let mut frames = Vec::new();
    for lines in [1.0, -1.0] {
        for _ in 0..8 {
            frames.push(h.frame(&mut app, &[wheel(lines)]));
        }
    }
    table.push(("Tekerlekle yakınlaştırma", frames));
    // Closer: 1:500 at the middle, the nodes there loaded, then panned.
    app.viewport.camera.scale = 1.0 / (500.0 * 0.000_264_58);
    for _ in 0..200 {
        h.frame(&mut app, &[]);
        service::service().settle(Duration::from_secs(30));
        if service::service().idle() {
            break;
        }
    }
    h.frame(&mut app, &[moved(Point::new(center.x, center.y))]);
    table.push(("Orta tuşla kaydırma, 1:500", pan(&mut app, &mut h, center)));

    let env = environment(&root);
    let renderer = h.renderer.name();
    println!();
    println!(
        "# Nokta bulutu ölçümü ({}, {}{}), {renderer}",
        env["date"].as_str().unwrap_or_default(),
        env["commit"].as_str().unwrap_or_default(),
        if env["dirtyTree"].as_bool().unwrap_or(false) {
            ", kaydedilmemiş değişiklikle"
        } else {
            ""
        }
    );
    println!("{}", env["machineText"].as_str().unwrap_or_default());
    println!();
    println!("| Ölçü | Sonuç |\n|---|---|");
    for (what, value) in &rows {
        println!("| {what} | {value} |");
    }
    println!();
    println!(
        "| Durum | Kare | MİB p50 | MİB p95 | MİB en çok | Görünüm p50 | Çizim p50 | GPU p50 |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    let f = |v: &Value| v.as_f64().map_or("–".to_owned(), |x| format!("{x:.2}"));
    for (what, frames) in &table {
        let s = summary(frames);
        println!(
            "| {what} | {} | {} | {} | {} | {} | {} | {} |",
            s["frames"],
            f(&s["cpuP50"]),
            f(&s["cpuP95"]),
            f(&s["cpuMax"]),
            f(&s["viewP50"]),
            f(&s["drawP50"]),
            f(&s["gpuP50"]),
        );
    }
}

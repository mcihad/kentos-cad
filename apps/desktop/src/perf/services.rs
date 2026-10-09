//! Map services on the desktop, measured (docs/adr/0208 §16): what a frame
//! asks of a service (the tiles a view shows, in TUREF / TM33 over Web
//! Mercator), a tile's mesh, a picture tile decoded and cut into the atlas's
//! slots, a city's vector tile read and drawn by its style (and drawn again
//! a quarter zoom on), and the frames of a drawing over two basemaps
//! (OpenStreetMap's pictures under OpenFreeMap's vector tiles) while it is
//! panned. The real tiles at Kızılay are fetched once into the worktree's
//! `.run/perf/` (never committed) and read from there afterwards. Not a
//! correctness test and not run by default:
//!
//! ```text
//! cargo test --release -p kentos-desktop perf::services -- --ignored --nocapture --test-threads=1
//! ```

use std::path::PathBuf;
use std::time::{Duration, Instant};

use iced::Point;
use iced::advanced::renderer::Headless as _;
use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::tiles::{Grid, TileRef, cells_of, mesh, shown};
use kentos_render_wgpu::styled::ServiceView;
use kentos_services::request::Request;
use serde_json::json;

use super::frame::{Harness, moved, pan, summary};
use crate::app::App;
use crate::services::decode::{VectorLook, picture, vector_layers, vector_tile};
use crate::services::systems::{ProjectSystem, pair};

/// Kızılay in TUREF / TM33.
const X0: f64 = 487_526.0;
const Y0: f64 = 4_420_745.0;

/// The sorted samples' value at fraction `p`.
fn at(xs: &mut [f64], p: f64) -> f64 {
    xs.sort_by(f64::total_cmp);
    xs[((xs.len() - 1) as f64 * p).round() as usize]
}

/// `run` timed `n` times: its median and 99th percentile, microseconds.
fn micros<T>(n: usize, mut run: impl FnMut() -> T) -> (f64, f64) {
    let mut xs: Vec<f64> = (0..n)
        .map(|_| {
            let t = Instant::now();
            std::hint::black_box(run());
            t.elapsed().as_secs_f64() * 1e6
        })
        .collect();
    (at(&mut xs, 0.5), at(&mut xs, 0.99))
}

/// `url`'s answer, from `.run/perf/<name>` when fetched before.
fn cached(name: &str, url: &str) -> Vec<u8> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/perf");
    let path = dir.join(name);
    if let Ok(bytes) = std::fs::read(&path) {
        return bytes;
    }
    let res = crate::services::net::send(Request::get(url.to_owned()), None, url, 2)
        .unwrap_or_else(|e| panic!("{url}: {e}"));
    std::fs::create_dir_all(&dir).expect("the folder");
    std::fs::write(&path, &res.body).expect("kept");
    res.body
}

/// Web Mercator's tile at `level` that Kızılay is in.
fn kizilay(level: u32) -> TileRef {
    let (lon, lat) = (32.8541_f64, 39.9208_f64);
    let n = f64::from(1u32 << level);
    let col = ((lon + 180.0) / 360.0 * n).floor() as u64;
    let lat = lat.to_radians();
    let row = ((1.0 - (lat.tan() + 1.0 / lat.cos()).ln() / std::f64::consts::PI) / 2.0 * n).floor()
        as u64;
    TileRef { level, col, row }
}

#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn services() {
    let mut rows = Vec::new();
    let settings = kentos_project::new_project::default_settings(5255);
    let project = ProjectSystem::of(&settings);
    let mercator = pair(&project, 3857).expect("TM33 and Web Mercator");

    // The tiles a 1440 × 900 view shows, at 1:1000 and 1:50 000 (a screen pixel 0.264 mm).
    let grid = Grid::web_mercator(256, 19);
    for (scale, name) in [(1000.0, "1:1000"), (50_000.0, "1:50 000")] {
        let px = 1.0 / (0.000_264_583 * scale);
        let (w, h) = (1440.0 / px / 2.0, 900.0 / px / 2.0);
        let view = [X0 - w, Y0 - h, X0 + w, Y0 + h];
        let tiles = shown(&grid, 0, 19, view, px, |x, y| (mercator.to_grid)(x, y))
            .map_or(0, |s| s.tiles.len());
        let (p50, p99) = micros(5000, || {
            shown(&grid, 0, 19, view, px, |x, y| (mercator.to_grid)(x, y))
        });
        println!("görünen karolar {name} ({tiles} karo): p50 {p50:.1} µs, p99 {p99:.1} µs");
        rows.push(json!({ "what": format!("Görünen karolar, {name}"), "tiles": tiles, "p50us": p50, "p99us": p99 }));
    }

    // A tile's mesh, divided as the apps divide it.
    for level in [18u32, 8] {
        let t = kizilay(level);
        let n = cells_of(&grid, level, false, mercator.metres_per_unit, 1);
        let mut out = Vec::new();
        let (p50, p99) = micros(5000, || {
            mesh(&grid, t, n, |x, y| (mercator.to_project)(x, y), &mut out)
        });
        println!("karo ağı z{level} ({n} × {n}): p50 {p50:.1} µs, p99 {p99:.1} µs");
        rows.push(json!({ "what": format!("Karonun ağı, kat {level}, {n} × {n}"), "p50us": p50, "p99us": p99 }));
    }

    // OpenStreetMap's picture tile (z17): decoded and cut into the atlas's slots.
    let t = kizilay(17);
    let png = cached(
        "osm-17.png",
        &format!(
            "https://tile.openstreetmap.org/{}/{}/{}.png",
            t.level, t.col, t.row
        ),
    );
    let (p50, p99) = micros(300, || picture(&png, Some((256, 256))).expect("a tile"));
    println!(
        "PNG karo ({} bayt) çözme ve kesme: p50 {:.2} ms, p99 {:.2} ms",
        png.len(),
        p50 / 1000.0,
        p99 / 1000.0
    );
    rows.push(json!({ "what": "256'lık PNG karonun çözülmesi ve kesilmesi", "bytes": png.len(), "p50ms": p50 / 1000.0, "p99ms": p99 / 1000.0 }));

    // OpenFreeMap's city tile (z14): read, drawn by Liberty's style, drawn again a quarter zoom on.
    let style_url = "https://tiles.openfreemap.org/styles/liberty";
    let style_text = String::from_utf8(cached("ofm-liberty.json", style_url)).expect("text");
    let style = kentos_services::style::parse(&style_text, style_url).expect("the style");
    let source = style.main_vector().expect("a vector source").clone();
    let template = match source.tiles.first() {
        Some(t) => t.clone(),
        None => {
            let url = source.url.clone().expect("a TileJSON");
            let text = String::from_utf8(cached("ofm-tilejson.json", &url)).expect("text");
            kentos_services::caps::tilejson::read(&text, &url)
                .expect("TileJSON")
                .tiles[0]
                .clone()
        }
    };
    let t = kizilay(14);
    let url = template
        .replace("{z}", &t.level.to_string())
        .replace("{x}", &t.col.to_string())
        .replace("{y}", &t.row.to_string());
    let pbf = cached("ofm-14.pbf", &url);
    let (p50, p99) = micros(100, || vector_layers(&pbf).expect("a tile"));
    println!(
        "MVT okuma ({} bayt): p50 {:.2} ms, p99 {:.2} ms",
        pbf.len(),
        p50 / 1000.0,
        p99 / 1000.0
    );
    rows.push(json!({ "what": "Şehir karosu (z14) MVT okuma", "bytes": pbf.len(), "p50ms": p50 / 1000.0, "p99ms": p99 / 1000.0 }));
    let layers = vector_layers(&pbf).expect("a tile");
    let view = ServiceView {
        id: 1,
        grid: Grid::web_mercator(512, 14),
        min_level: 0,
        max_level: 14,
        vector: true,
        same_system: false,
        to_grid: mercator.to_grid.clone(),
        to_project: mercator.to_project.clone(),
        metres_per_unit: mercator.metres_per_unit,
        across: 2,
        down: 2,
    };
    let look = VectorLook {
        origin: Vec2 { x: X0, y: Y0 },
        palette: kentos_native_style::color::StylePalette::graphite(),
        view: kentos_native_style::color::ViewColors::default(),
    };
    for zoom in [14.0, 14.25] {
        let (p50, p99) = micros(30, || {
            vector_tile(&layers, &style, &source.id, zoom, &view, t, &look)
        });
        println!(
            "MVT stil ve partiler z{zoom}: p50 {:.2} ms, p99 {:.2} ms",
            p50 / 1000.0,
            p99 / 1000.0
        );
        rows.push(json!({ "what": format!("Şehir karosunun stili ve partileri, kat {zoom}"), "p50ms": p50 / 1000.0, "p99ms": p99 / 1000.0 }));
    }

    // A drawing over two basemaps, panned: the frames' CPU while tiles come and go.
    let (mut app, _) = App::boot(None);
    let mut h = Harness::new();
    h.settle(&mut app);
    let (osm, none) = crate::service_scenes::preset("osm-standard");
    crate::service_scenes::opened(&mut app, osm, none);
    // OpenFreeMap's vector tiles over OpenStreetMap's pictures.
    if let Some(doc) = &mut app.document {
        let (ofm, _) = crate::service_scenes::preset("ofm-liberty");
        let service: kentos_contracts::ServiceLayer =
            serde_json::from_value(ofm).expect("a service");
        let input = kentos_contracts::LayersService {
            operation: kentos_contracts::LayerServiceOperation::Add,
            layer: None,
            name: Some("OpenFreeMap".into()),
            parent: None,
            index: Some(2),
            service: Some(service),
            feed: None,
            fields: None,
            connections: None,
            expected_revision: None,
        };
        let _ = kentos_native_application::layers_service::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            input,
        );
    }
    h.settle(&mut app);
    // Every tile the view asks for, from the network the first time (unmeasured).
    // The headless GPU's read back of each frame is the harness's own, not the app's: left out.
    let full = |app: &mut App, h: &mut Harness| {
        let start = Instant::now();
        let mut readback = 0.0;
        loop {
            readback += h.frame(app, &[]).gpu_ms();
            if crate::services::hub().settle(Duration::from_millis(100))
                || start.elapsed() > Duration::from_secs(60)
            {
                break;
            }
        }
        start.elapsed().as_secs_f64() * 1000.0 - readback
    };
    full(&mut app, &mut h);
    // The services read again with their tiles let go: the whole view from the device's cache.
    let keys: Vec<String> = app
        .document
        .as_ref()
        .map(|d| {
            let settings = d.model.settings();
            d.model
                .layers()
                .leaves()
                .iter()
                .filter_map(|n| n.service.as_ref())
                .map(|s| {
                    let c = s
                        .connection
                        .as_ref()
                        .and_then(|id| settings.connections.iter().find(|k| &k.id == id));
                    crate::services::key_of(s, c)
                })
                .collect()
        })
        .unwrap_or_default();
    for k in &keys {
        crate::services::hub().reload(k);
    }
    let first = full(&mut app, &mut h);
    println!("tam görünüm cihazın önbelleğinden: {first:.0} ms");
    rows.push(json!({ "what": "Tam görünüm cihazın önbelleğinden (iki altlık)", "ms": first }));
    let center = app.viewport.bounds.center();
    h.frame(&mut app, &[moved(Point::new(center.x, center.y))]);
    let still: Vec<_> = (0..40).map(|_| h.frame(&mut app, &[])).collect();
    let s = summary(&still);
    println!("iki altlıklı boş kare: {s}");
    rows.push(json!({ "what": "İki altlıklı boş kare", "summary": s }));
    let frames = pan(&mut app, &mut h, center);
    let s = summary(&frames);
    println!("iki altlıklı kaydırma: {s}");
    rows.push(json!({ "what": "İki altlıklı orta tuşla kaydırma", "summary": s, "renderer": h.renderer.name() }));
    println!(
        "{}",
        serde_json::to_string_pretty(&rows).unwrap_or_default()
    );
}

/// Where a city tile's drawing time goes: the style with every vertex moved
/// by the core's transformation, and with a plain linear map instead.
#[test]
#[ignore = "a measurement, run by hand in release mode"]
fn services_style_split() {
    use kentos_services::style::build::{TileInput, build};
    let settings = kentos_project::new_project::default_settings(5255);
    let mercator = pair(&ProjectSystem::of(&settings), 3857).expect("TM33 and Web Mercator");
    let style_url = "https://tiles.openfreemap.org/styles/liberty";
    let style = kentos_services::style::parse(
        &String::from_utf8(cached("ofm-liberty.json", style_url)).expect("text"),
        style_url,
    )
    .expect("the style");
    let source = style.main_vector().expect("a vector source").clone();
    let pbf = cached("ofm-14.pbf", "");
    let layers = vector_layers(&pbf).expect("a tile");
    let t = kizilay(14);
    let grid = Grid::web_mercator(512, 14);
    let [x1, y1, x2, y2] = grid.matrices[14].bounds(t.col, t.row);
    let calls = std::cell::Cell::new(0usize);
    let exact = |u: f64, v: f64| {
        calls.set(calls.get() + 1);
        (mercator.to_project)(x1 + (x2 - x1) * u, y2 - (y2 - y1) * v).map(|(x, y)| Vec2 { x, y })
    };
    let a = (mercator.to_project)(x1, y2).expect("a corner");
    let b = (mercator.to_project)(x2, y2).expect("a corner");
    let c = (mercator.to_project)(x1, y1).expect("a corner");
    let linear = |u: f64, v: f64| {
        Some(Vec2 {
            x: a.0 + (b.0 - a.0) * u + (c.0 - a.0) * v,
            y: a.1 + (b.1 - a.1) * u + (c.1 - a.1) * v,
        })
    };
    let run = |f: &dyn Fn(f64, f64) -> Option<Vec2>| {
        build(&TileInput {
            layers: &layers,
            style: &style,
            source: &source.id,
            zoom: 14.0,
            to_project: f,
            origin: Vec2 { x: X0, y: Y0 },
            tile: 1,
        })
    };
    let seen = std::cell::RefCell::new(Vec::new());
    let record = |u: f64, v: f64| {
        seen.borrow_mut().push((u, v));
        exact(u, v)
    };
    let _ = run(&record);
    let per = calls.get();
    // The same points through the tile's mesh, bilinear in a cell: how far from the exact place.
    for n in [4u32, 8, 16] {
        let mut nodes = Vec::new();
        mesh(&grid, t, n, |x, y| (mercator.to_project)(x, y), &mut nodes);
        let node = |i: u32, j: u32| {
            let k = ((j * (n + 1) + i) * 2) as usize;
            (nodes[k], nodes[k + 1])
        };
        let mut worst = 0.0f64;
        for &(u, v) in seen.borrow().iter() {
            let (fu, fv) = (
                (u.clamp(0.0, 1.0)) * f64::from(n),
                (v.clamp(0.0, 1.0)) * f64::from(n),
            );
            let (i, j) = (
                (fu.floor() as u32).min(n - 1),
                (fv.floor() as u32).min(n - 1),
            );
            let (s, r) = (fu - f64::from(i), fv - f64::from(j));
            let (p00, p10, p01, p11) = (
                node(i, j),
                node(i + 1, j),
                node(i, j + 1),
                node(i + 1, j + 1),
            );
            let x = p00.0 * (1.0 - s) * (1.0 - r)
                + p10.0 * s * (1.0 - r)
                + p01.0 * (1.0 - s) * r
                + p11.0 * s * r;
            let y = p00.1 * (1.0 - s) * (1.0 - r)
                + p10.1 * s * (1.0 - r)
                + p01.1 * (1.0 - s) * r
                + p11.1 * s * r;
            if let Some(e) = exact(u, v) {
                worst = worst.max((x - e.x).hypot(y - e.y));
            }
        }
        println!("ağ {n} × {n}: en büyük sapma {:.4} mm", worst * 1000.0);
    }
    let (e50, _) = micros(30, || run(&exact));
    let (l50, _) = micros(30, || run(&linear));
    let (t50, _) = micros(20_000, || (mercator.to_project)(x1 + 10.0, y2 - 10.0));
    println!(
        "dönüşümle {:.2} ms, doğrusal {:.2} ms; karoda {per} nokta, nokta başına dönüşüm {t50:.3} µs",
        e50 / 1000.0,
        l50 / 1000.0
    );
}

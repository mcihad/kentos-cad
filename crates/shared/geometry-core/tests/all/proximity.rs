//! Yakınlık analizi (docs/adr/0215): how long the store's searches take on a
//! parcel layer, against the ADR's budgets (§6). The tools' answers are the
//! shared cases' (`fixtures/processing/v1/proximity.json`, written by
//! `scripts/fixtures/proximity_cases.py` without KentOS code), checked on both
//! platforms by the processing tests; the core's own rules by
//! `store::proximity`'s and `ops::proximity`'s unit tests.

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::jsmath::js_min;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::proximity::{Measure, NEAREST_STRIDE, NEIGHBOR_STRIDE};
use kentos_geometry_core::vec2::Vec2;

/// The scene's objects: their ids by kind.
pub struct Scene {
    pub store: Store,
    pub parcels: Vec<f64>,
    /// 5 m squares inside every 97th parcel, each overlapping it alone.
    pub overlapping: Vec<f64>,
    pub stops: Vec<f64>,
    pub hubs: Vec<f64>,
    pub roads: Vec<f64>,
}

/// The R2 sequence's fractions (Roberts 2018): even, and the same in JavaScript.
fn r2(k: f64) -> (f64, f64) {
    (
        (k * 0.754_877_666_246_693) % 1.0,
        (k * 0.569_840_290_998_053_2) % 1.0,
    )
}

/// The scene the web's `scripts/perf/proximity.test.ts` builds too: 10 000
/// parcels on a 100 × 100 grid of 20 m whose corners are moved up to 3 m
/// (shared by the neighbours, so boxes overlap as real parcels' do), a 5 m
/// square inside every 97th, 2 000 stops and 2 000 road pieces spread by
/// the R2 sequence, 100 hubs at the 200 m blocks' middles. Ids: parcels
/// from 1, stops from 10 001, hubs from 12 001, roads from 12 101, the
/// squares from 14 101.
pub fn scene() -> Scene {
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let corner = |i: u32, j: u32| {
        let (u, v) = r2(f64::from(i * 101 + j) + 0.5);
        Vec2::new(
            e0 + f64::from(i) * 20.0 + 6.0 * (u - 0.5),
            n0 + f64::from(j) * 20.0 + 6.0 * (v - 0.5),
        )
    };
    // Put at once, as a run's store is (`put_many` builds the index).
    let mut items: Vec<(f64, &str, bool, Shape)> = Vec::new();
    let polygon = |pts: Vec<Vec2>| Shape::Polygon {
        pts,
        bulges: None,
        holes: None,
        parts: None,
    };
    let point = |p: Vec2| Shape::Point {
        p,
        z: None,
        parts: None,
    };
    let (mut parcels, mut overlapping) = (Vec::new(), Vec::new());
    for i in 0..100u32 {
        for j in 0..100u32 {
            let id = f64::from(i * 100 + j + 1);
            let pts = vec![
                corner(i, j),
                corner(i + 1, j),
                corner(i + 1, j + 1),
                corner(i, j + 1),
            ];
            if (i * 100 + j) % 97 == 0 {
                let x = (pts[0].x + pts[1].x + pts[2].x + pts[3].x) / 4.0;
                let y = (pts[0].y + pts[1].y + pts[2].y + pts[3].y) / 4.0;
                let square = vec![
                    Vec2::new(x - 2.5, y - 2.5),
                    Vec2::new(x + 2.5, y - 2.5),
                    Vec2::new(x + 2.5, y + 2.5),
                    Vec2::new(x - 2.5, y + 2.5),
                ];
                let small = 14_101.0 + overlapping.len() as f64;
                items.push((small, "parsel", false, polygon(square)));
                overlapping.push(small);
            }
            items.push((id, "parsel", false, polygon(pts)));
            parcels.push(id);
        }
    }
    let mut stops = Vec::new();
    for k in 0..2000u32 {
        let (u, v) = r2(f64::from(k) + 0.5);
        let id = f64::from(10_001 + k);
        items.push((
            id,
            "durak",
            false,
            point(Vec2::new(e0 + 2000.0 * u, n0 + 2000.0 * v)),
        ));
        stops.push(id);
    }
    let mut hubs = Vec::new();
    for a in 0..10u32 {
        for b in 0..10u32 {
            let id = f64::from(12_001 + a * 10 + b);
            let p = Vec2::new(
                e0 + 100.0 + f64::from(a) * 200.0,
                n0 + 100.0 + f64::from(b) * 200.0,
            );
            items.push((id, "merkez", false, point(p)));
            hubs.push(id);
        }
    }
    let mut roads = Vec::new();
    for k in 0..2000u32 {
        let (u, v) = r2(f64::from(k) + 0.25);
        let (x, y) = (e0 + 2000.0 * u, n0 + 2000.0 * v);
        let id = f64::from(12_101 + k);
        items.push((
            id,
            "yol",
            false,
            Shape::Line {
                a: Vec2::new(x, y),
                b: Vec2::new(x + 15.0 * (2.0 * v - 1.0), y + 10.0),
            },
        ));
        roads.push(id);
    }
    let mut store = Store::new();
    store.put_many(items);
    Scene {
        store,
        parcels,
        overlapping,
        stops,
        hubs,
        roads,
    }
}

/// The least of `runs` runs after a warm-up, in milliseconds, and the work's
/// record count.
fn least(runs: usize, mut work: impl FnMut() -> usize) -> (f64, usize) {
    let mut count = work();
    let mut best = f64::INFINITY;
    for _ in 0..runs {
        let t = std::time::Instant::now();
        count = work();
        best = js_min(best, t.elapsed().as_secs_f64() * 1000.0);
    }
    (best, count)
}

/// How long the five tools' searches take (docs/adr/0215 §6), release build:
/// `cargo test --release -p kentos-geometry-core --test all proximity::timing -- --ignored --nocapture`.
#[test]
#[ignore = "süre ölçümü: cargo test --release -p kentos-geometry-core --test all proximity::timing -- --ignored --nocapture"]
fn timing() {
    let Scene {
        store,
        parcels,
        overlapping,
        stops,
        hubs,
        roads,
    } = scene();
    let areas: Vec<f64> = parcels.iter().chain(&overlapping).copied().collect();
    let inf = f64::INFINITY;
    let mut over = Vec::new();
    let mut say = |name: &str, (ms, n): (f64, usize), budget: f64| {
        let ok = ms <= budget;
        println!(
            "{name:<46} {ms:>8.2} ms  {n:>6} kayıt  bütçe {budget:>5.0} ms: {}",
            if ok {
                "bütçede"
            } else {
                "BÜTÇEYİ AŞIYOR"
            }
        );
        if !ok {
            over.push(name.to_owned());
        }
    };
    let rows = |r: Vec<f64>, stride: usize| r.len() / stride;
    say(
        "En yakını bul (10 000 → 2 000 durak)",
        least(5, || {
            rows(
                store.nearest(&parcels, &stops, 1, inf, Measure::Edges),
                NEAREST_STRIDE,
            )
        }),
        40.0,
    );
    say(
        "Uzaklık matrisi (10 000 → 2 000, k 5)",
        least(5, || {
            rows(
                store.nearest(&parcels, &stops, 5, inf, Measure::Edges),
                NEAREST_STRIDE,
            )
        }),
        60.0,
    );
    say(
        "En yakın merkeze bağla (10 000 → 100 merkez)",
        least(5, || {
            rows(
                store.nearest(&parcels, &hubs, 1, inf, Measure::Centers),
                NEAREST_STRIDE,
            )
        }),
        20.0,
    );
    say(
        "Komşu alanlar (10 104, köşeler, örtüşmeler)",
        least(5, || {
            rows(store.neighbors(&areas, 0.001, true, true), NEIGHBOR_STRIDE)
        }),
        100.0,
    );
    say(
        "En kısa çizgi (10 000 → 2 000 yol, k 1)",
        least(5, || {
            rows(
                store.nearest(&parcels, &roads, 1, inf, Measure::Edges),
                NEAREST_STRIDE,
            )
        }),
        40.0,
    );
    // What the scene must give: every parcel finds its five targets; each
    // parcel neighbours the eight around it but those past the block's rim
    // (99 · 100 east–west, 100 · 99 north–south and 2 · 99 · 99 diagonal
    // pairs, both ways round), and each square overlaps its parcel alone.
    assert_eq!(
        rows(
            store.nearest(&parcels, &stops, 5, inf, Measure::Edges),
            NEAREST_STRIDE
        ),
        5 * parcels.len()
    );
    let found = store.neighbors(&areas, 0.001, true, true);
    let kinds = |kind: f64| {
        found
            .chunks_exact(NEIGHBOR_STRIDE)
            .filter(|r| r[2] == kind)
            .count()
    };
    assert_eq!(kinds(0.0), 2 * (2 * 99 * 100), "kenar komşuları");
    assert_eq!(kinds(1.0), 2 * (2 * 99 * 99), "köşe komşuları");
    assert_eq!(kinds(2.0), 2 * overlapping.len(), "örtüşmeler");
    assert!(over.is_empty(), "bütçeyi aşanlar: {over:?}");
}

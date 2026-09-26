//! Timings of the geometry store beside the native document on a large
//! drawing (docs/adr/0029): reading it all, following edits, and the queries
//! a pointer move makes (snap, pick) and a box makes. Not a pass/fail test:
//! run it by hand, in release, and write the numbers down with the machine:
//!
//! `cargo test --release -p kentos-interaction --test perf -- --ignored --nocapture`

use std::time::{Duration, Instant};

use kentos_contracts::{
    DocumentSnapshotV1, Entity, EntityBase, LineEntity, PathEntity, Vec2 as Wire,
};
use kentos_domain::{Document, Slot};
use kentos_interaction::{Spatial, Vec2, snap_kinds};

const E: f64 = 487000.0;
const N: f64 = 4420000.0;

fn base(layer: &str) -> EntityBase {
    EntityBase {
        id: 0,
        layer_id: layer.into(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
    }
}

/// A parcel map: `side` × `side` blocks of five-cornered parcels 20 m apart,
/// and a road line along every row: `side² + side` objects.
fn drawing(side: usize) -> Document {
    let text = include_str!("../../../../fixtures/interaction/v1/objects.kcad");
    let mut snapshot = DocumentSnapshotV1::from_json(text).expect("reads");
    snapshot.entities.clear();
    let mut id = 1;
    for row in 0..side {
        for col in 0..side {
            let (x, y) = (E + col as f64 * 20.0, N + row as f64 * 20.0);
            // A little skew per parcel, so no two are alike.
            let k = ((row * 31 + col * 17) % 7) as f64 * 0.137;
            let pts = [
                (x, y),
                (x + 18.0, y + k),
                (x + 18.5, y + 9.0),
                (x + 17.0, y + 18.0 - k),
                (x + k, y + 17.5),
            ]
            .map(|(x, y)| Wire { x, y })
            .to_vec();
            let mut b = base("parsel");
            b.id = id;
            id += 1;
            snapshot.entities.push(Entity::Polygon(PathEntity {
                base: b,
                pts,
                bulges: None,
                holes: None,
            }));
        }
        let mut b = base("cizim");
        b.id = id;
        id += 1;
        let y = N + row as f64 * 20.0 + 19.0;
        snapshot.entities.push(Entity::Line(LineEntity {
            base: b,
            a: Wire { x: E, y },
            b: Wire {
                x: E + side as f64 * 20.0,
                y,
            },
        }));
    }
    Document::from_snapshot(snapshot).expect("opens")
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

/// Runs `f` `n` times; the mean and the worst, in microseconds.
fn each(n: usize, mut f: impl FnMut(usize)) -> (f64, f64) {
    let mut worst = Duration::ZERO;
    let start = Instant::now();
    for i in 0..n {
        let t = Instant::now();
        f(i);
        worst = worst.max(t.elapsed());
    }
    (
        start.elapsed().as_secs_f64() * 1e6 / n as f64,
        worst.as_secs_f64() * 1e6,
    )
}

#[test]
#[ignore = "a measurement: run by hand in release"]
fn store_sync_and_queries_on_a_large_drawing() {
    for side in [100usize, 316] {
        let t = Instant::now();
        let mut doc = drawing(side);
        let opened = t.elapsed();
        let objects = doc.len();
        let t = Instant::now();
        let mut spatial = Spatial::of(&doc);
        let loaded = t.elapsed();
        println!(
            "\n{objects} nesne ({side}×{side} parsel ve {side} yol): belge {:.1} ms, depoya ilk okuma {:.1} ms",
            ms(opened),
            ms(loaded)
        );

        // Following edits: one object added, changed, removed; a thousand removed and undone.
        let line = |x: f64| {
            Entity::Line(LineEntity {
                base: base("cizim"),
                a: Wire { x, y: N - 5.0 },
                b: Wire {
                    x: x + 10.0,
                    y: N - 5.0,
                },
            })
        };
        let (add, _) = each(200, |i| {
            doc.add(line(E + i as f64)).expect("a slot");
            spatial.sync(&doc);
        });
        let (undo, _) = each(200, |_| {
            doc.undo();
            spatial.sync(&doc);
        });
        let slots: Vec<Slot> = doc
            .entities()
            .take(1000)
            .map(|e| Slot(e.base().id))
            .collect();
        let t = Instant::now();
        doc.remove(&slots);
        spatial.sync(&doc);
        let remove_many = t.elapsed();
        let t = Instant::now();
        doc.undo();
        spatial.sync(&doc);
        let undo_many = t.elapsed();
        let t = Instant::now();
        spatial.sync(&doc);
        let idle = t.elapsed();
        doc.toggle_layer_visible("cizim");
        let t = Instant::now();
        spatial.sync(&doc);
        let layers = t.elapsed();
        doc.toggle_layer_visible("cizim");
        spatial.sync(&doc);
        assert_eq!(spatial.reloads(), 1, "followed, never read again");
        println!(
            "  izleme: nesne ekle + eşitle {add:.1} µs, geri al + eşitle {undo:.1} µs; \
             1000 nesne sil + eşitle {:.2} ms, geri al + eşitle {:.2} ms; \
             değişiklik yokken {:.3} µs; katman görünürlüğü {:.3} ms",
            ms(remove_many),
            ms(undo_many),
            idle.as_secs_f64() * 1e6,
            ms(layers)
        );

        // Queries as the pointer makes them: snap and pick where the pointer is.
        let width = side as f64 * 20.0;
        let kinds = snap_kinds(|key| key != "snap.nearest");
        let point = |i: usize| {
            let f = (i as f64 * 0.618_033_988_75).fract();
            let g = (i as f64 * 0.414_213_562_37).fract();
            Vec2::new(E + f * width, N + g * width)
        };
        // Close in: 8 px per metre (the traces' 0.125 m per pixel), an 11 px aperture.
        let (snap_near, snap_near_worst) = each(2000, |i| {
            std::hint::black_box(spatial.snap(point(i), 11.0 / 8.0, kinds, None));
        });
        let (snap_from, _) = each(2000, |i| {
            std::hint::black_box(spatial.snap(point(i), 11.0 / 8.0, kinds, Some(point(i + 7))));
        });
        let (pick_near, _) = each(2000, |i| {
            std::hint::black_box(spatial.pick(point(i), 5.0 / 8.0));
        });
        // The whole drawing on a 1100 px wide area.
        let scale = 1100.0 / width;
        let (snap_far, snap_far_worst) = each(200, |i| {
            std::hint::black_box(spatial.snap(point(i), 11.0 / scale, kinds, None));
        });
        let (pick_far, _) = each(200, |i| {
            std::hint::black_box(spatial.pick(point(i), 5.0 / scale));
        });
        let t = Instant::now();
        let window = spatial.in_rect(
            Vec2::new(E, N),
            Vec2::new(E + width * 0.3, N + width * 0.3),
            false,
        );
        let window_time = t.elapsed();
        let t = Instant::now();
        let crossing = spatial.in_rect(
            Vec2::new(E + width * 0.3, N + width * 0.3),
            Vec2::new(E, N),
            true,
        );
        let crossing_time = t.elapsed();
        println!(
            "  kenet (yakın, 1,375 m): ortalama {snap_near:.1} µs, en kötü {snap_near_worst:.0} µs; \
             son noktadan dik ve teğetle {snap_from:.1} µs; seçme {pick_near:.1} µs"
        );
        println!(
            "  kenet (bütün çizim ekranda): ortalama {snap_far:.0} µs, en kötü {snap_far_worst:.0} µs; seçme {pick_far:.0} µs"
        );
        println!(
            "  pencere %9 alan: {} nesne {:.2} ms; kesişim: {} nesne {:.2} ms",
            window.len(),
            ms(window_time),
            crossing.len(),
            ms(crossing_time)
        );
    }
}

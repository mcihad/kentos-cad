//! Timings of the geometry store beside the native document on a large
//! drawing (docs/adr/0029): reading it all, following edits, and the queries
//! a pointer move makes (snap, pick) and a box makes. Not a pass/fail test:
//! run it by hand, in release, and write the numbers down with the machine:
//!
//! `cargo test --release -p kentos-interaction --test perf -- --ignored --nocapture`

use std::time::{Duration, Instant};

use kentos_contracts::{
    DocumentSnapshotV1, Entity, EntityBase, EntityId, LayerFilter, LineEntity, PathEntity,
    Vec2 as Wire,
};
use kentos_domain::{Document, Slot};
use kentos_interaction::{Spatial, Vec2, default_snap_kinds};

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
        line_weight: None,
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
                zs: None,
                parts: None,
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
            za: None,
            zb: None,
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
                za: None,
                zb: None,
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
        let kinds = default_snap_kinds();
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

/// The kinds of land a parcel of the filter's drawing is, one after another.
const KINDS: [&str; 3] = ["Arsa", "Tarla", "Bağ"];

/// Katman süzgeci on a large layer (docs/adr/0211 §6): the parcels of [`drawing`] with a
/// `Nitelik` each, the filter set on their layer and the store following it whole
/// (compiled, asked of every object, marked, counted), then one parcel's attribute
/// changed (only it asked again). Beside it the command's count, the core alone.
///
/// `cargo test --release -p kentos-interaction --test perf layer_filter -- --ignored --nocapture`
#[test]
#[ignore = "a measurement: run by hand in release"]
fn layer_filter_on_a_large_layer() {
    let mut doc = drawing(316);
    let slots: Vec<Slot> = doc
        .entities()
        .filter(|e| e.base().layer_id == "parsel")
        .map(|e| Slot(e.base().id))
        .collect();
    for (i, slot) in slots.iter().enumerate() {
        let mut e = doc.get(*slot).expect("a parcel").clone();
        e.base_mut()
            .attrs
            .insert("Nitelik".into(), KINDS[i % KINDS.len()].into());
        doc.update(*slot, e);
    }
    let parcels = slots.len();
    let mut spatial = Spatial::of(&doc);
    let uids: Vec<EntityId> = slots
        .iter()
        .step_by(2)
        .map(|s| EntityId(*doc.uid(*s).expect("a persistent id").as_bytes()))
        .collect();
    let filters = [
        (
            "öznitelik ifadesi (Nitelik = 'Arsa')",
            LayerFilter {
                expression: Some("Nitelik = 'Arsa'".into()),
                objects: Vec::new(),
            },
            50.0,
        ),
        (
            "geometri ifadesi ($alan > 310)",
            LayerFilter {
                expression: Some("$alan > 310".into()),
                objects: Vec::new(),
            },
            150.0,
        ),
        (
            "nesne listesi (her ikinci parsel)",
            LayerFilter {
                expression: None,
                objects: uids,
            },
            10.0,
        ),
    ];
    println!("\nKatman süzgeci, {parcels} parsel:");
    for (name, filter, budget) in filters {
        let mut store = Vec::new();
        let mut core = Vec::new();
        let mut passed = (0, 0);
        for _ in 0..7 {
            doc.set_layer_filter("parsel", None, "Katman süzgeci")
                .expect("taken away");
            spatial.sync(&doc);
            doc.set_layer_filter("parsel", Some(filter.clone()), "Katman süzgeci")
                .expect("written");
            let t = Instant::now();
            spatial.sync(&doc);
            store.push(ms(t.elapsed()));
            passed = spatial.filter_counts("parsel").expect("counted");
            let compiled =
                kentos_native_application::layer_filter::compile_filter(&filter).expect("compiles");
            let t = Instant::now();
            let counted =
                kentos_native_application::layers_filter::count(&doc, "parsel", Some(&compiled));
            core.push(ms(t.elapsed()));
            assert_eq!((counted.0 as usize, counted.1 as usize), passed);
        }
        store.sort_by(f64::total_cmp);
        core.sort_by(f64::total_cmp);
        println!(
            "  {name:<38} geçen {}/{}: depo {:.1} ms (en yavaşı {:.1}), çekirdek {:.1} ms; bütçe {budget} ms",
            passed.0, passed.1, store[3], store[6], core[3]
        );
    }
    // One parcel's attribute changed under the attribute filter: only it is asked again.
    doc.set_layer_filter(
        "parsel",
        Some(LayerFilter {
            expression: Some("Nitelik = 'Arsa'".into()),
            objects: Vec::new(),
        }),
        "Katman süzgeci",
    )
    .expect("written");
    spatial.sync(&doc);
    let (edit, worst) = each(200, |i| {
        let slot = slots[i * 37 % slots.len()];
        let mut e = doc.get(slot).expect("a parcel").clone();
        e.base_mut()
            .attrs
            .insert("Nitelik".into(), KINDS[(i + 1) % KINDS.len()].into());
        doc.update(slot, e);
        spatial.sync(&doc);
    });
    println!("  bir parselin özniteliği değişince: ortalama {edit:.0} µs, en yavaşı {worst:.0} µs");
}

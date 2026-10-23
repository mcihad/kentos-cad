//! Timings of the geometry store beside the native document on a large
//! drawing (docs/adr/0029): reading it all, following edits, and the queries
//! a pointer move makes (snap, pick) and a box makes. Not a pass/fail test:
//! run it by hand, in release, and write the numbers down with the machine:
//!
//! `cargo test --release -p kentos-interaction --test perf -- --ignored --nocapture`

use std::time::{Duration, Instant};

use kentos_commands::{
    DocumentSnapshotV1, Entity, EntityBase, EntityId, LayerFilter, LineEntity, PathEntity,
    Vec2 as Wire,
};
use kentos_domain::{Document, Slot};
use kentos_interaction::{Spatial, Vec2, default_snap_kinds};

const E: f64 = 487000.0;
const N: f64 = 4420000.0;

fn base(layer: &str) -> EntityBase {
    EntityBase {
        look: Default::default(),
        id: 0,
        layer_id: layer.into(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
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
                ms: None,
                base: b,
                pts,
                bulges: None,
                holes: None,
                zs: None,
                parts: None,
                closed: false,
                widths: None,
            }));
        }
        let mut b = base("cizim");
        b.id = id;
        id += 1;
        let y = N + row as f64 * 20.0 + 19.0;
        snapshot.entities.push(Entity::Line(LineEntity {
            ma: None,
            mb: None,
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
                ma: None,
                mb: None,
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
            let compiled = kentos_native_application::layer_filter::compile_filter(
                &filter,
                kentos_native_application::expression_unit::Unit::METRE,
            )
            .expect("compiles");
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

/// ADR 0212 §6: the texts of 100 000 parcels' labels worked out when their layer's labelling
/// changes (Kurallı: the number by an expression, the owner's name by a condition and an
/// expression), as the store follows the drawing (`Spatial::sync`). Budget 80 ms.
#[test]
#[ignore = "a measurement: run by hand in release"]
fn label_texts_on_a_large_layer() {
    use kentos_commands::{LabelClass, LabelPlacement, LabelStyle, LabelsMode, LayerLabels};
    let mut doc = drawing(316);
    let slots: Vec<Slot> = doc
        .entities()
        .filter(|e| e.base().layer_id == "parsel")
        .map(|e| Slot(e.base().id))
        .collect();
    for (i, slot) in slots.iter().enumerate() {
        let mut e = doc.get(*slot).expect("a parcel").clone();
        let attrs = &mut e.base_mut().attrs;
        attrs.insert("Ada".into(), format!("{}", 100 + i / 300));
        attrs.insert("Parsel".into(), format!("{}", 1 + i % 300));
        attrs.insert(
            "Malik".into(),
            if i % 3 == 0 {
                "Ayşe Demir".into()
            } else {
                String::new()
            },
        );
        doc.update(*slot, e);
    }
    let style = |text: &str| LabelStyle {
        placement: LabelPlacement::Center,
        size: 10.0,
        text: Some(text.into()),
        ..LabelStyle::default()
    };
    let labelled = LayerLabels {
        mode: LabelsMode::Rules,
        classes: vec![
            LabelClass {
                name: "No".into(),
                when: None,
                style: style("Ada || '/' || Parsel"),
            },
            LabelClass {
                name: "Malik".into(),
                when: Some("Malik <> ''".into()),
                style: style("upper(Malik)"),
            },
        ],
        obstacle: None,
    };
    let base = doc.layers().get("parsel").expect("the layer").style.clone();
    let mut spatial = Spatial::of(&doc);
    let mut runs = Vec::new();
    for _ in 0..5 {
        let mut plain = base.clone();
        plain.labels = None;
        doc.set_layer_style("parsel", plain, "Etiketler");
        spatial.sync(&doc);
        let mut ruled = base.clone();
        ruled.labels = Some(labelled.clone());
        doc.set_layer_style("parsel", ruled, "Etiketler");
        let t = Instant::now();
        spatial.sync(&doc);
        runs.push(ms(t.elapsed()));
    }
    runs.sort_by(f64::total_cmp);
    println!(
        "\nEtiket metinleri, {} parsel, iki ifadeli sınıf: en iyi {:.1} ms, p50 {:.1} ms   bütçe 80 ms  {}",
        slots.len(),
        runs[0],
        runs[runs.len() / 2],
        if runs[runs.len() / 2] <= 80.0 {
            "✓"
        } else {
            "✗"
        }
    );
}

/// A layer of `n` points for the charts (docs/adr/0227 §11): Grup of 30 values,
/// Değer with two fraction digits, and with `xy` a Yaş too; deterministic.
fn chart_entities(n: usize, xy: bool) -> Vec<Entity> {
    use kentos_commands::PointEntity;
    (0..n)
        .map(|i| {
            let mut b = base("nokta");
            b.id = i as u32 + 1;
            // The R2 sequence's fractions (Roberts 2018): spread, not random.
            let u = (i as f64 * 0.754_877_666_246_693).fract();
            let v = (i as f64 * 0.569_840_290_998_053_2).fract();
            b.attrs.insert("Grup".into(), format!("G{:02}", 1 + i % 30));
            b.attrs
                .insert("Değer".into(), format!("{:.2}", 1000.0 + 9000.0 * u));
            if xy {
                b.attrs
                    .insert("Yaş".into(), format!("{}", (v * 80.0).floor() as i64));
            }
            Entity::Point(PointEntity {
                base: b,
                p: Wire {
                    x: E + u * 1000.0,
                    y: N + v * 1000.0,
                },
                z: None,
                parts: None,
            })
        })
        .collect()
}

/// Grafikler ve alan istatistikleri (docs/adr/0227 §11) on 100 000 objects:
/// their data read (the fields' cells checked, the core's columns read), a
/// field's summary, a field's statistics and values, Çubuk (Toplam, 30
/// categories) with its layout, Histogram and Kutu (30 groups), Saçılım of
/// 100 000 points with its layout, the selection's marks and a hover's hit.
/// The median of seven runs against the ADR's desktop budget.
#[test]
#[ignore = "a measurement: run by hand in release"]
fn charts_on_a_large_layer() {
    use kentos_commands::{LayerField, LayerFieldKind};
    use kentos_geometry_core::ops::charts::Work;
    use kentos_geometry_core::ops::charts::data::DataSet;
    use kentos_geometry_core::ops::charts::layout::Frame;
    use kentos_geometry_core::ops::charts::model::{Aggregate, ChartKind, Spec};
    use kentos_interaction::chart_data::layer_columns;

    const N_OBJECTS: usize = 100_000;
    let median = |mut v: Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    };
    let say = |name: &str, ms: f64, budget: f64| {
        println!(
            "  {name:<52} {ms:>7.2} ms; bütçe {budget:>4} ms: {}",
            if ms <= budget {
                "bütçede"
            } else {
                "BÜTÇEYİ AŞIYOR"
            }
        );
    };
    let mut decimal = LayerField::new("Değer", LayerFieldKind::Decimal);
    decimal.scale = Some(2);
    let two = vec![
        LayerField::new("Grup", LayerFieldKind::Text),
        decimal.clone(),
    ];
    let three = vec![
        LayerField::new("Grup", LayerFieldKind::Text),
        decimal,
        LayerField::new("Yaş", LayerFieldKind::Integer),
    ];
    let frame = Frame {
        width: 900.0,
        height: 260.0,
        font: 12.0,
        family: "Plus Jakarta Sans".into(),
    };
    println!("\nGrafikler, {N_OBJECTS} nesne:");
    // The data: the fields' cells read from the objects, the core's columns read.
    let entities = chart_entities(N_OBJECTS, false);
    let refs: Vec<&Entity> = entities.iter().collect();
    let mut setup = Vec::new();
    for _ in 0..7 {
        let t = Instant::now();
        let columns = layer_columns(&two, &refs, None, 1.0);
        let mut work = Work::new(DataSet {
            rows: N_OBJECTS,
            columns,
        });
        work.read(0);
        work.read(1);
        setup.push(ms(t.elapsed()));
    }
    say("verinin kurulması (2 sütun)", median(setup), 40.0);

    let entities = chart_entities(N_OBJECTS, true);
    let refs: Vec<&Entity> = entities.iter().collect();
    let mut work = Work::new(DataSet {
        rows: N_OBJECTS,
        columns: layer_columns(&three, &refs, None, 1.0),
    });
    let rows: Vec<u32> = (0..N_OBJECTS as u32).collect();
    work.read(0);
    work.read(1);
    work.read(2);
    let runs = |f: &mut dyn FnMut()| {
        let mut v = Vec::new();
        for _ in 0..7 {
            let t = Instant::now();
            f();
            v.push(ms(t.elapsed()));
        }
        median(v)
    };
    let s = runs(&mut || {
        work.summary(1, &rows);
    });
    say("özet tablonun bir alanı (Değer)", s, 12.0);
    let s = runs(&mut || {
        work.detail(1, &rows);
    });
    say(
        "seçilen alanın istatistikleri ve değerleri (Değer)",
        s,
        25.0,
    );
    let bars = Spec {
        category: Some("Grup".into()),
        aggregate: Aggregate::Sum,
        value: Some("Değer".into()),
        ..Spec::new(ChartKind::Bar)
    };
    let s = runs(&mut || {
        work.chart(&bars, &rows).expect("bars");
        work.lay_out(&frame);
    });
    say("çubuk (Toplam, 30 kategori), model ve yerleşim", s, 15.0);
    let histogram = Spec {
        value: Some("Değer".into()),
        ..Spec::new(ChartKind::Histogram)
    };
    let s = runs(&mut || {
        work.chart(&histogram, &rows).expect("histogram");
        work.lay_out(&frame);
    });
    say("histogram, model ve yerleşim", s, 25.0);
    let boxes = Spec {
        value: Some("Değer".into()),
        group: Some("Grup".into()),
        ..Spec::new(ChartKind::Box)
    };
    let s = runs(&mut || {
        work.chart(&boxes, &rows).expect("boxes");
        work.lay_out(&frame);
    });
    say("kutu (30 grup), model ve yerleşim", s, 25.0);
    let scatter = Spec {
        x: Some("Değer".into()),
        y: vec!["Yaş".into()],
        ..Spec::new(ChartKind::Scatter)
    };
    let s = runs(&mut || {
        work.chart(&scatter, &rows).expect("scatter");
        work.lay_out(&frame);
    });
    say("saçılım (100 000 nokta), model ve yerleşim", s, 25.0);
    // Half the objects selected, then the other half: the marks found again each time.
    let halves = [
        rows.iter()
            .copied()
            .filter(|r| r % 2 == 0)
            .collect::<Vec<u32>>(),
        rows.iter()
            .copied()
            .filter(|r| r % 2 == 1)
            .collect::<Vec<u32>>(),
    ];
    let mut k = 0;
    let s = runs(&mut || {
        k += 1;
        work.select(&halves[k % 2]);
    });
    say("seçimin vurgusu (50 000 seçili nesne)", s, 4.0);
    work.lay_out(&frame);
    let (mean, worst) = each(1000, |i| {
        let x = 20.0 + (i as f32 * 0.754_877_7).fract() * 860.0;
        let y = 20.0 + (i as f32 * 0.569_840_3).fract() * 220.0;
        let _ = work.hit(x, y);
    });
    say(
        &format!(
            "üzerine gelme (tıklama sınaması; en yavaşı {:.0} µs)",
            worst
        ),
        mean / 1000.0,
        1.0,
    );
}

/// The resident and the highest memory of this process, in MB (Linux's `/proc/self/status`; none elsewhere).
fn memory() -> (f64, f64) {
    let text = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        text.lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<f64>().ok())
            .map_or(0.0, |kb| kb / 1024.0)
    };
    (field("VmRSS:"), field("VmHWM:"))
}

/// `n` timings of `f`, in milliseconds: the median, the 95th percentile and the worst.
fn spread(n: usize, mut f: impl FnMut(usize)) -> (f64, f64, f64) {
    let mut all: Vec<f64> = (0..n)
        .map(|i| {
            let t = Instant::now();
            f(i);
            ms(t.elapsed())
        })
        .collect();
    all.sort_by(f64::total_cmp);
    let at = |q: f64| all[((all.len() - 1) as f64 * q).round() as usize];
    (at(0.5), at(0.95), at(1.0))
}

/// CBS-42's budgets on a million parcels (docs/adr/0597): the store read whole, and the queries a pointer makes in a
/// warm view at three scales (a parcel 120 px wide, the town on the screen, the whole city on it), each 2000 times
/// at points spread over the view; the median, the 95th percentile and the worst against the candidate budget of
/// 30 ms for pick and snap. `KENTOS_PERF_SIDE` sets the side (1000: a million parcels and a thousand roads).
///
/// `cargo test --release -p kentos-interaction --test perf budgets_on_a_million_parcels -- --ignored --nocapture`
#[test]
#[ignore = "a measurement: run by hand in release"]
fn budgets_on_a_million_parcels() {
    let side: usize = std::env::var("KENTOS_PERF_SIDE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    let t = Instant::now();
    let doc = drawing(side);
    let opened = t.elapsed();
    let (rss, _) = memory();
    let t = Instant::now();
    let spatial = Spatial::of(&doc);
    let loaded = t.elapsed();
    let (rss2, top) = memory();
    println!(
        "\n{} nesne ({side}×{side} parsel ve {side} yol): belge {:.0} ms ({rss:.0} MB), depo {:.0} ms ({:.0} MB daha; tepe {top:.0} MB)",
        doc.len(),
        ms(opened),
        ms(loaded),
        rss2 - rss
    );
    let width = side as f64 * 20.0;
    let kinds = default_snap_kinds();
    // A view of `span` metres wide (a 1100 px wide area) centred in the city; points spread over it.
    for (name, span) in [
        ("1:1000 (parsel 120 px)", 1100.0 * 0.264_58),
        ("ilçe ekranda (4 km)", 4000.0),
        ("bütün şehir ekranda", width),
    ] {
        let per_px = span / 1100.0;
        let (cx, cy) = (E + width / 2.0, N + width / 2.0);
        let point = |i: usize| {
            let f = (i as f64 * 0.618_033_988_75).fract() - 0.5;
            let g = (i as f64 * 0.414_213_562_37).fract() - 0.5;
            Vec2::new(cx + f * span, cy + g * span * 0.6)
        };
        let snap = spread(2000, |i| {
            std::hint::black_box(spatial.snap(point(i), 11.0 * per_px, kinds, None));
        });
        let snap_from = spread(2000, |i| {
            std::hint::black_box(spatial.snap(point(i), 11.0 * per_px, kinds, Some(point(i + 7))));
        });
        let pick = spread(2000, |i| {
            std::hint::black_box(spatial.pick(point(i), 5.0 * per_px));
        });
        let say = |(p50, p95, max): (f64, f64, f64)| {
            format!(
                "p50 {p50:.3}, p95 {p95:.3}, en kötü {max:.2} ms{}",
                if p95 <= 30.0 {
                    ""
                } else {
                    " — BÜTÇEYİ AŞIYOR"
                }
            )
        };
        println!(
            "  {name}: kenet {}; son noktadan kenet {}; seçme {}",
            say(snap),
            say(snap_from),
            say(pick)
        );
    }
    let t = Instant::now();
    let window = spatial.in_rect(
        Vec2::new(E + width * 0.45, N + width * 0.45),
        Vec2::new(E + width * 0.55, N + width * 0.55),
        false,
    );
    println!(
        "  pencereyle seçim (%1 alan): {} nesne {:.1} ms",
        window.len(),
        ms(t.elapsed())
    );
    let (rss3, top3) = memory();
    println!("  bellek sonunda {rss3:.0} MB, tepe {top3:.0} MB");
}

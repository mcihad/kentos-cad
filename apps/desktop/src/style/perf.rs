//! The styled drawing's cost, measured (docs/adr/0090; performance is a red
//! line: nothing is traded for speed): building every layer through the
//! style core against the plain scene of before, one changed object's
//! rebuild, and a frame's CPU work over the batches (what shows, at which
//! scale). Not a correctness test and not run by default:
//!
//! ```text
//! cargo test --release -p kentos-desktop style::perf -- --ignored --nocapture --test-threads=1
//! ```

use std::collections::BTreeMap;
use std::time::Instant;

use kentos_contracts::{
    DocumentSnapshotV1, Entity, EntityBase, LayerNode, LayerNodeType, LayerStyle, LineType,
    PathEntity, PointEntity, PointStyle, PointSymbol, ProjectSettings, ProjectStyles, Vec2,
};
use kentos_domain::Slot;
use kentos_interaction::Spatial;
use kentos_native_style::StylePalette;
use kentos_render_wgpu::Bounds;
use kentos_render_wgpu::scene::{self, lod};
use kentos_render_wgpu::styled::StyledScene;

use super::scene::{Look, StyledCache};
use crate::document::Document;

fn layer(id: &str, style: LayerStyle) -> LayerNode {
    LayerNode {
        id: id.into(),
        name: id.into(),
        kind: LayerNodeType::Layer,
        visible: true,
        locked: false,
        expanded: true,
        style,
        children: Vec::new(),
        snap: None,
        fields: Vec::new(),
        service: None,
        feed: None,
        time: None,
        scenario: None,
        replaces: None,
        filter: None,
    }
}

fn style(color: &str, line_type: LineType, weight: f64) -> LayerStyle {
    LayerStyle {
        color: color.into(),
        line_type,
        line_weight: weight,
        fill: None,
        point: None,
        label: None,
        pick_interior: None,
        renderer: None,
        labels: None,
    }
}

/// `n` parcels of 20 vertices, `n` points and `n / 2` two-bend paths on three simple-look layers.
fn drawing(n: usize) -> Document {
    let mut entities = Vec::with_capacity(3 * n);
    let mut id = 0u32;
    let mut base = |layer: &str| {
        id += 1;
        EntityBase {
            id,
            layer_id: layer.into(),
            color: None,
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
        }
    };
    for i in 0..n {
        let (x0, y0) = (
            486_000.0 + (i % 300) as f64 * 31.7,
            4_420_000.0 + (i / 300) as f64 * 27.3,
        );
        let pts = (0..20)
            .map(|k| {
                let a = k as f64 / 20.0 * std::f64::consts::TAU;
                Vec2 {
                    x: x0 + 12.5 + 11.0 * a.cos(),
                    y: y0 + 12.5 + 9.0 * a.sin(),
                }
            })
            .collect();
        entities.push(Entity::Polygon(PathEntity {
            base: base("parsel"),
            pts,
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
        }));
        entities.push(Entity::Point(PointEntity {
            base: base("nokta"),
            p: Vec2 {
                x: x0 + 3.0,
                y: y0 + 3.0,
            },
            z: None,
            parts: None,
        }));
        if i % 2 == 0 {
            entities.push(Entity::Polyline(PathEntity {
                base: base("yol"),
                pts: vec![
                    Vec2 { x: x0, y: y0 },
                    Vec2 {
                        x: x0 + 20.0,
                        y: y0 + 4.0,
                    },
                    Vec2 {
                        x: x0 + 30.0,
                        y: y0 + 20.0,
                    },
                ],
                bulges: None,
                holes: None,
                zs: None,
                parts: None,
            }));
        }
    }
    let mut points = style("#2E7D32", LineType::Continuous, 0.25);
    points.point = Some(PointStyle {
        symbol: PointSymbol::Cross,
        size: 9.0,
    });
    document(
        format!("olcum-{n}"),
        vec![
            layer("parsel", style("#E06C75", LineType::Continuous, 0.35)),
            layer("nokta", points),
            layer("yol", style("#4E79A7", LineType::Dashdot, 0.5)),
        ],
        entities,
    )
}

/// A GIS drawing of these layers and objects.
fn document(name: String, layers: Vec<LayerNode>, entities: Vec<Entity>) -> Document {
    let snapshot = DocumentSnapshotV1 {
        format: "kentos.document".into(),
        version: 1,
        name,
        settings: ProjectSettings {
            srid: 5256,
            length_decimals: 3,
            area_decimals: 2,
            area_unit: kentos_contracts::AreaUnit::M2,
            angle_unit: kentos_contracts::AngleUnit::Grad,
            plot_scale: 1000.0,
            workspace: None,
            drawing_font: None,
            drawing_unit: None,
            second_srid: None,
            custom_crs: None,
            second_custom_crs: None,
            datum_transforms: Vec::new(),
            layer_states: Vec::new(),
            survey: None,
            dimension_styles: Vec::new(),
            topology: None,
            annotation: None,
            text_styles: Vec::new(),
            connections: Vec::new(),
            networks: Vec::new(),
            variables: Vec::new(),
        },
        origin: Vec2 {
            x: 486_000.0,
            y: 4_420_000.0,
        },
        home_view: None,
        layers,
        active_layer: "parsel".into(),
        entities,
        styles: ProjectStyles::default(),
        blocks: Vec::new(),
    };
    Document::new(snapshot, None).expect("opens")
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn look(doc: &Document, screen_scale: f64) -> Look {
    Look {
        palette: StylePalette::graphite(),
        symbol_scale: screen_scale,
        screen: false,
        hairlines: false,
        origin: scene::scene_origin(doc),
        view_build: Default::default(),
        view_colors: Default::default(),
    }
}

/// A frame's CPU work over the batches: scale range, view box, legibility (the renderer's `prepare`).
fn frame_cpu(scene: &StyledScene, view: [f64; 4], px_per_m: f64) -> usize {
    let mut shown = 0;
    for part in &scene.layers {
        for b in &part.layer.batches {
            if b.in_scale(1000.0) && b.in_view(view, px_per_m, 1.0) && b.legible(px_per_m, 1.0) {
                shown += 1;
            }
        }
    }
    shown
}

#[test]
#[ignore = "a measurement, run by hand in release"]
fn perf() {
    let library = kentos_native_style::system::library();
    for n in [10_000usize, 50_000, 100_000] {
        let doc = drawing(n);
        let objects = doc.model.len();
        let spatial = Spatial::of(&doc.model);
        let palette = crate::viewport::palette(kentos_ui::theme::Mode::Dark);
        let origin = scene::scene_origin(&doc);

        let t = Instant::now();
        let fixed = scene::build_fixed(&doc, &palette, origin);
        let curves = scene::build_curves(&doc, &palette, origin, lod::tolerance(0), 1_000_000);
        let plain = ms(t);
        let plain_bytes = fixed.byte_size() + curves.byte_size();

        let view = Bounds {
            min_x: 486_000.0,
            min_y: 4_420_000.0,
            max_x: 486_400.0,
            max_y: 4_420_250.0,
        };
        let mut cache = StyledCache::default();
        let t = Instant::now();
        let styled = cache.scene(
            1,
            &doc.model,
            spatial.store(),
            &library,
            &look(&doc, 1000.0),
            &view,
            1.0,
            true,
            1,
        );
        let full = ms(t);
        let batches: usize = styled.layers.iter().map(|l| l.layer.batches.len()).sum();
        let floats: usize = styled.layers.iter().map(|l| l.layer.data.len()).sum();

        // One object moved: its layer alone is built again.
        let mut doc2 = doc.clone();
        let slot = Slot(2);
        if let Some(Entity::Point(p)) = doc2.model.get(slot).cloned() {
            let mut moved = p.clone();
            moved.p.x += 1.0;
            let _ = doc2.model.update(slot, Entity::Point(moved));
        }
        let mut spatial2 = spatial;
        spatial2.sync(&doc2.model);
        let _ = cache.scene(
            1,
            &doc.model,
            spatial2.store(),
            &library,
            &look(&doc, 1000.0),
            &view,
            1.0,
            true,
            1,
        );
        let t = Instant::now();
        let _ = cache.scene(
            1,
            &doc2.model,
            spatial2.store(),
            &library,
            &look(&doc2, 1000.0),
            &view,
            1.0,
            true,
            1,
        );
        let one = ms(t);

        let t = Instant::now();
        let again = cache.scene(
            1,
            &doc2.model,
            spatial2.store(),
            &library,
            &look(&doc2, 1000.0),
            &view,
            1.0,
            true,
            1,
        );
        let unchanged = ms(t);

        let rel = [
            view.min_x - origin.x,
            view.min_y - origin.y,
            view.max_x - origin.x,
            view.max_y - origin.y,
        ];
        let t = Instant::now();
        let mut shown = 0;
        for _ in 0..100 {
            shown = frame_cpu(&again, rel, 3.78);
        }
        let frame = ms(t) / 100.0;

        println!(
            "{objects} nesne: düz sahne {plain:.1} ms ({:.1} MB) · stilli kurulum {full:.1} ms ({batches} topluluk, {:.1} MB) · bir nesne değişince {one:.1} ms · değişmeyen {unchanged:.3} ms · karede CPU {frame:.3} ms ({shown} topluluk görünür)",
            plain_bytes as f64 / 1e6,
            floats as f64 * 4.0 / 1e6,
        );
    }
    // The showcase: every system symbol once, on 124 layers.
    if let Some(demo) = super::screens::demo() {
        let spatial = Spatial::of(&demo.model);
        let mut cache = StyledCache::default();
        let view = Bounds {
            min_x: 486_540.0,
            min_y: 4_418_000.0,
            max_x: 490_000.0,
            max_y: 4_420_300.0,
        };
        let t = Instant::now();
        let s = cache.scene(
            1,
            &demo.model,
            spatial.store(),
            &library,
            &look(&demo, 1000.0),
            &view,
            1.0,
            true,
            1,
        );
        println!(
            "gösterim kataloğu: {} nesne, {} katman, stilli kurulum {:.1} ms ({} topluluk)",
            demo.model.len(),
            s.layers.len(),
            ms(t),
            s.layers
                .iter()
                .map(|l| l.layer.batches.len())
                .sum::<usize>()
        );
    }
}

/// `n` parcels of 20 vertices on a temporal layer (docs/adr/0210): each
/// starts in one of a hundred years (1925–2024), none ends. `in_order`: the
/// drawing holds them in time order, as a layer grows over the years (a
/// year's parcels together); else the years take turns (every part of the
/// layer holds every year, the worst case of ADR 0121's parts).
fn temporal_drawing(n: usize, in_order: bool) -> Document {
    let entities = (0..n)
        .map(|i| {
            let (x0, y0) = (
                486_000.0 + (i % 300) as f64 * 31.7,
                4_420_000.0 + (i / 300) as f64 * 27.3,
            );
            let year = 1925 + if in_order { i * 100 / n } else { i % 100 };
            let pts = (0..20)
                .map(|k| {
                    let a = k as f64 / 20.0 * std::f64::consts::TAU;
                    Vec2 {
                        x: x0 + 12.5 + 11.0 * a.cos(),
                        y: y0 + 12.5 + 9.0 * a.sin(),
                    }
                })
                .collect();
            Entity::Polygon(PathEntity {
                base: EntityBase {
                    id: i as u32 + 1,
                    layer_id: "parsel".into(),
                    color: None,
                    attrs: BTreeMap::from([("baslangic".to_owned(), format!("{year}-01-01"))]),
                    label: None,
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                },
                pts,
                bulges: None,
                holes: None,
                zs: None,
                parts: None,
            })
        })
        .collect();
    let mut parcels = layer("parsel", style("#E06C75", LineType::Continuous, 0.35));
    parcels.time = Some(kentos_contracts::LayerTime {
        start: "baslangic".into(),
        end: Some("bitis".into()),
        key: None,
        cumulative: false,
    });
    document(format!("zaman-{n}"), vec![parcels], entities)
}

/// ADR 0210 §12: the time slider's step on a temporal layer of 100 000
/// parcels (each step shows 1 % more of them: a year's), its first build at a
/// moment, and Senaryo oluştur copying the layer. Release, by hand:
///
/// ```text
/// cargo test --release -p kentos-desktop style::perf::temporal -- --ignored --nocapture --test-threads=1
/// ```
#[test]
#[ignore = "a measurement, run by hand in release"]
fn temporal() {
    use kentos_geometry_core::time::{Window, days_from_civil};

    let library = kentos_native_style::system::library();
    let at = |y: i64| (days_from_civil(y, 1, 1) * 86_400_000) as f64;
    let view = Bounds {
        min_x: 486_000.0,
        min_y: 4_420_000.0,
        max_x: 486_400.0,
        max_y: 4_420_250.0,
    };
    let row = |what: &str, ms: f64, budget: Option<f64>| match budget {
        Some(b) => println!(
            "{what:<58} {ms:>9.1} ms   bütçe {b:>5} ms  {}",
            if ms <= b { "✓" } else { "✗" }
        ),
        None => println!("{what:<58} {ms:>9.1} ms"),
    };
    for in_order in [true, false] {
        let doc = temporal_drawing(100_000, in_order);
        let how = if in_order {
            "zaman sırasıyla"
        } else {
            "yıllar karışık"
        };
        println!("100 000 parsel, {how}:");
        let t = Instant::now();
        let mut spatial = Spatial::of(&doc.model);
        row("  depo (zamanların okunması dahil)", ms(t), None);
        let mut cache = StyledCache::default();
        spatial.set_time_window(Some(Window::Instant(at(1990))));
        let t = Instant::now();
        let _ = cache.scene(
            1,
            &doc.model,
            spatial.store(),
            &library,
            &look(&doc, 1000.0),
            &view,
            1.0,
            true,
            1,
        );
        row("  ilk kurulum, 1990'da", ms(t), None);
        // Ten steps, a year each: the slowest.
        let mut worst: f64 = 0.0;
        for y in 1991..=2000 {
            spatial.set_time_window(Some(Window::Instant(at(y))));
            let t = Instant::now();
            let _ = cache.scene(
                1,
                &doc.model,
                spatial.store(),
                &library,
                &look(&doc, 1000.0),
                &view,
                1.0,
                true,
                1,
            );
            worst = worst.max(ms(t));
        }
        row(
            "  sürgünün bir adımı (%1 değişir), en yavaşı",
            worst,
            Some(50.0),
        );
        spatial.set_time_window(None);
        let t = Instant::now();
        let _ = cache.scene(
            1,
            &doc.model,
            spatial.store(),
            &library,
            &look(&doc, 1000.0),
            &view,
            1.0,
            true,
            1,
        );
        row("  sürgü kapanınca (hepsi görünür)", ms(t), None);
    }
    let mut doc = temporal_drawing(100_000, true);
    let t = Instant::now();
    let result = kentos_native_application::scenarios_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        kentos_contracts::ScenariosEdit {
            operation: kentos_contracts::ScenarioOperation::Create,
            name: Some("Öneri".into()),
            layers: Some(vec!["parsel".into()]),
            copy_objects: None,
            note: None,
            scenario: None,
            expected_revision: None,
        },
    );
    let copied = ms(t);
    assert!(matches!(
        result,
        kentos_contracts::CommandResult::Completed { .. }
    ));
    assert_eq!(doc.model.len(), 200_000);
    row(
        "Senaryo oluştur: 100 000 nesnenin kopyası",
        copied,
        Some(1000.0),
    );
}

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
    let snapshot = DocumentSnapshotV1 {
        format: "kentos.document".into(),
        version: 1,
        name: format!("olcum-{n}"),
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
            text_styles: Vec::new(),
        },
        origin: Vec2 {
            x: 486_000.0,
            y: 4_420_000.0,
        },
        home_view: None,
        layers: vec![
            layer("parsel", style("#E06C75", LineType::Continuous, 0.35)),
            layer("nokta", points),
            layer("yol", style("#4E79A7", LineType::Dashdot, 0.5)),
        ],
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

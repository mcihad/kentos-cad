//! The selection and the hovered object on the drawing area (docs/adr/0029):
//! the web's `uploadHighlight`, as two wgpu scene parts drawn after every
//! layer of the drawing. The selection is in the accent with a 13 % fill of
//! closed areas and 15 px rings on points; the hovered object, unless it is
//! selected, is in the accent at 85 % without fill (a fill would flicker
//! across large areas as the pointer moves).
//!
//! A part is built when what it shows changes: the selection, the hovered
//! object, the drawing, the accent or the curves' band. A pan, or a zoom
//! within the band, draws both from the GPU as they are. Measured against
//! Iced's canvas, which tessellates its paths on the CPU in every frame the
//! view moves (ADR 0029): with 100 172 objects selected the part is built
//! once in 42 ms and costs no CPU per frame; the canvas took 109 ms a frame.

use std::sync::Arc;

use kentos_domain::Slot;
use kentos_interaction::Selection;
use kentos_render_wgpu::scene::{self, Highlight};
use kentos_render_wgpu::{Rgba8, ScenePart};

use super::{Viewport, revision};
use crate::document::Document;

/// The highlight parts as last built, and what they were built from.
#[derive(Default)]
pub(super) struct Highlighted {
    selected: Option<(HighlightKey, Arc<ScenePart>)>,
    hovered: Option<(HighlightKey, Arc<ScenePart>)>,
}

/// What a highlight part depends on. The selection's part does not depend
/// on the hovered object: a pointer moving over a large selection's objects
/// never builds it again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HighlightKey {
    generation: u64,
    revision: u64,
    selection: u64,
    /// The hovered object's changes; 0 in the selection's key.
    hover: u64,
    accent: Rgba8,
    /// The scene's layer count: the highlights draw after them.
    below: usize,
    /// The curves' tolerance (its bits): curves are highlighted as finely as drawn.
    tolerance: u64,
}

/// The selection's fill of closed areas, and the hovered object's alpha (the web's).
const SELECTION_FILL: f64 = 0.13;
const HOVER_ALPHA: f64 = 0.85;
/// Point marks of a highlight, logical pixels (the web's `pointStyle: { size: 15, shape: 'ring' }`).
const HIGHLIGHT_MARK: f32 = 15.0;

impl Viewport {
    /// The selection's part and the hovered object's, from the cache when
    /// nothing they depend on changed.
    pub(super) fn highlights(
        &self,
        doc: &Document,
        selection: &Selection,
        accent: Rgba8,
        fixed: &ScenePart,
        curves: &ScenePart,
    ) -> (Arc<ScenePart>, Arc<ScenePart>) {
        let key = HighlightKey {
            generation: self.generation,
            revision: revision(doc),
            selection: selection.version(),
            hover: 0,
            accent,
            below: fixed.layers.len(),
            tolerance: curves.tolerance.to_bits(),
        };
        let ring = kentos_render_wgpu::layout::marker_shape::RING;
        let build = |ids: &mut dyn Iterator<Item = Slot>, style: Highlight| {
            let objects = ids.filter_map(|slot| doc.model.get(slot));
            Arc::new(scene::build_highlight(
                objects,
                &style,
                fixed.origin,
                curves.tolerance,
                key.below,
            ))
        };
        let mut cache = self.highlight.borrow_mut();
        let selected = match &cache.selected {
            Some((known, part)) if *known == key => part.clone(),
            _ => {
                let part = build(
                    &mut selection.ids().iter().copied(),
                    Highlight {
                        color: accent,
                        fill: Some(accent.with_alpha(SELECTION_FILL)),
                        mark_size: HIGHLIGHT_MARK,
                        mark_shape: ring,
                    },
                );
                cache.selected = Some((key, part.clone()));
                part
            }
        };
        let hover_key = HighlightKey {
            hover: selection.hover_version(),
            ..key
        };
        let hovered = match &cache.hovered {
            Some((known, part)) if *known == hover_key => part.clone(),
            _ => {
                // A selected object is not hovered over its own highlight.
                let part = build(
                    &mut selection
                        .hover()
                        .filter(|h| !selection.contains(*h))
                        .into_iter(),
                    Highlight {
                        color: accent.with_alpha(HOVER_ALPHA),
                        fill: None,
                        mark_size: HIGHLIGHT_MARK,
                        mark_shape: ring,
                    },
                );
                cache.hovered = Some((hover_key, part.clone()));
                part
            }
        };
        (selected, hovered)
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Rectangle};
    use kentos_contracts::Entity;
    use kentos_render_wgpu::RenderSettings;
    use kentos_render_wgpu::Vec2;
    use kentos_ui::theme::Mode;

    use super::super::{Event, palette};
    use super::*;

    fn sample() -> Document {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/document/v1/sample.json");
        Document::read(&path).expect("the sample reads")
    }

    /// The highlights are scene parts built when what they show changes; a
    /// pointer moving over objects builds the hovered one's part only, never
    /// the selection's again (docs/adr/0029).
    #[test]
    fn a_hover_builds_only_the_hover_highlight() {
        let doc = sample();
        let viewport = Viewport::new();
        let palette = palette(Mode::Dark);
        let settings = RenderSettings::new(palette.background);
        let (_, fixed, curves) = viewport.scene(&doc, Mode::Dark, &palette, &settings);
        let accent = Rgba8::rgb(0x4c, 0x9b, 0xe8);
        let mut selection = Selection::new();
        let ids: Vec<Slot> = doc.model.entities().map(|e| Slot(e.base().id)).collect();
        selection.set(ids.iter().copied().take(ids.len() - 1));
        let (sel, hover) = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
        assert!(!sel.segments.is_empty() && hover.segments.is_empty());
        selection.set_hover(ids.last().copied());
        let (sel2, hover2) = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
        assert_eq!(sel2.id, sel.id, "the selection's part is kept");
        assert_ne!(hover2.id, hover.id);
        let (sel3, hover3) = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
        assert_eq!((sel3.id, hover3.id), (sel.id, hover2.id), "nothing changed");
        // A selected object is not hovered over its own highlight.
        selection.set_hover(ids.first().copied());
        let (_, hover4) = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
        assert!(hover4.segments.is_empty() && hover4.fills.is_empty());
        // Its colour and layer: after every layer of the scene.
        assert!(sel.segments.iter().all(|s| s.color == accent.0));
        assert_eq!(sel.layers.len(), fixed.layers.len() + 1);
    }

    /// Where the selection's highlight is drawn, measured (docs/adr/0029): as
    /// a wgpu scene part, built once per selection and drawn from the GPU
    /// every frame, against Iced's canvas, whose paths (dashed, as the web
    /// draws the selection) are tessellated on the CPU in every frame the view
    /// moves. A parcel map of `side²` parcels and `side` roads, all selected,
    /// the whole map on a 1400 × 800 area. Run by hand, in release:
    /// `cargo test --release -p kentos-desktop highlight_costs -- --ignored --nocapture`.
    #[test]
    #[ignore = "a measurement: run by hand in release, with a GPU"]
    fn highlight_costs_on_a_large_drawing() {
        use iced::advanced::renderer::Headless;
        use iced::widget::canvas::{self, LineDash, Path, Stroke};
        use kentos_contracts::{EntityBase, LineEntity, PathEntity, Vec2 as Wire};
        use std::time::Instant;

        let Some(renderer) = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            kentos_ui::theme::typography::ui(),
            iced::Pixels(16.0),
            Some("wgpu"),
        )) else {
            println!("wgpu çizicisi kurulamadı: ölçüm yapılmadı");
            return;
        };
        let base = |id: u32, layer: &str| EntityBase {
            id,
            layer_id: layer.into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
        };
        for side in [32usize, 100, 316] {
            let sample = sample();
            let layer = sample
                .model
                .layers()
                .leaves()
                .first()
                .map(|l| l.id.clone())
                .expect("a layer");
            let mut snapshot = sample.model.to_snapshot();
            snapshot.entities.clear();
            let (e, n) = (487000.0, 4420000.0);
            let mut id = 1;
            for row in 0..side {
                for col in 0..side {
                    let (x, y) = (e + col as f64 * 20.0, n + row as f64 * 20.0);
                    let pts = [
                        (x, y),
                        (x + 18.0, y),
                        (x + 18.5, y + 9.0),
                        (x + 17.0, y + 18.0),
                        (x, y + 17.5),
                    ]
                    .map(|(x, y)| Wire { x, y })
                    .to_vec();
                    snapshot.entities.push(Entity::Polygon(PathEntity {
                        base: base(id, &layer),
                        pts,
                        bulges: None,
                        holes: None,
                    }));
                    id += 1;
                }
                let y = n + row as f64 * 20.0 + 19.0;
                snapshot.entities.push(Entity::Line(LineEntity {
                    base: base(id, &layer),
                    a: Wire { x: e, y },
                    b: Wire {
                        x: e + side as f64 * 20.0,
                        y,
                    },
                }));
                id += 1;
            }
            let doc = Document::new(snapshot, None).expect("opens");
            let objects = doc.entity_count();
            let mut viewport = Viewport::new();
            viewport.update(
                Event::Resized(Rectangle::new(
                    Point::ORIGIN,
                    iced::Size::new(1400.0, 800.0),
                )),
                Some(&doc),
            );
            viewport.opened(&doc);
            viewport.update(Event::Extents, Some(&doc));
            let palette = palette(Mode::Dark);
            let settings = RenderSettings::new(palette.background);
            let t = Instant::now();
            let (_, fixed, curves) = viewport.scene(&doc, Mode::Dark, &palette, &settings);
            let scene_time = t.elapsed();
            let mut selection = Selection::new();
            selection.set(doc.model.entities().map(|e| Slot(e.base().id)));
            let accent = Rgba8::rgb(0x4c, 0x9b, 0xe8);
            let t = Instant::now();
            let (part, _) = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
            let part_time = t.elapsed();
            let t = Instant::now();
            let _ = viewport.highlights(&doc, &selection, accent, &fixed, &curves);
            let cached = t.elapsed();

            // The same highlight on Iced's canvas: every frame the view moves.
            let camera = viewport.camera;
            let dashed = Stroke {
                line_dash: LineDash {
                    segments: &[6.0, 3.0],
                    offset: 0,
                },
                ..Stroke::default()
                    .with_color(iced::Color::from_rgb8(0x4c, 0x9b, 0xe8))
                    .with_width(1.0)
            };
            let fill = iced::Color::from_rgba8(0x4c, 0x9b, 0xe8, 0.13);
            let screen = |p: &Wire| {
                let [x, y] = camera.world_to_screen(Vec2::new(p.x, p.y));
                Point::new(x as f32, y as f32)
            };
            let frames = 3;
            let t = Instant::now();
            for _ in 0..frames {
                let mut frame = canvas::Frame::new(&renderer, iced::Size::new(1400.0, 800.0));
                for entity in doc.model.entities() {
                    match entity {
                        Entity::Polygon(p) => {
                            let path = Path::new(|b| {
                                b.move_to(screen(&p.pts[0]));
                                for q in &p.pts[1..] {
                                    b.line_to(screen(q));
                                }
                                b.close();
                            });
                            frame.fill(&path, fill);
                            frame.stroke(&path, dashed);
                        }
                        Entity::Line(l) => {
                            frame.stroke(&Path::line(screen(&l.a), screen(&l.b)), dashed);
                        }
                        _ => {}
                    }
                }
                std::hint::black_box(frame.into_geometry());
            }
            let canvas_frame = t.elapsed() / frames;
            println!(
                "{objects} seçili nesne: sahne {:.1} ms; wgpu vurgu parçası bir kez {:.1} ms ({} KB, {} çizgi parçası, {} dolgu üçgeni), \
                 önbellekten {:.3} ms, kare başına CPU 0; Iced canvas'ı kare başına {:.1} ms",
                scene_time.as_secs_f64() * 1e3,
                part_time.as_secs_f64() * 1e3,
                part.byte_size() / 1024,
                part.segments.len(),
                part.fills.len() / 3,
                cached.as_secs_f64() * 1e3,
                canvas_frame.as_secs_f64() * 1e3,
            );
        }
    }
}

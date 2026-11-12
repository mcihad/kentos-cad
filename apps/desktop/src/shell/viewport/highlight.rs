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
//!
//! A block's insert is highlighted by its pieces as the geometry store placed
//! them (docs/adr/0144).

use std::sync::Arc;

use kentos_commands::Entity;
use kentos_domain::Slot;
use kentos_interaction::{Selection, Spatial};
use kentos_render_wgpu::scene::{self, Highlight};
use kentos_render_wgpu::{Bounds, Rgba8, ScenePart};

use super::{Viewport, changes};
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
    changes: u64,
    selection: u64,
    /// The hovered object's changes; 0 in the selection's key.
    hover: u64,
    accent: Rgba8,
    /// The scene's layer count: the highlights draw after them.
    below: usize,
    /// The curves' tolerance (its bits): curves are highlighted as finely as drawn.
    tolerance: u64,
    /// The construction lines' box (its bits): they are highlighted as clipped.
    clip: [u64; 4],
    /// The view's theme (its id's hash): an object on a layer it hides is not highlighted in it (docs/adr/0637 §3).
    theme: u64,
}

/// The selection's fill of closed areas, and the hovered object's alpha (the web's).
const SELECTION_FILL: f64 = 0.13;
const HOVER_ALPHA: f64 = 0.85;
/// Point marks of a highlight, logical pixels (the web's `pointStyle: { size: 15, shape: 'ring' }`).
const HIGHLIGHT_MARK: f32 = 15.0;
/// A selection this large is highlighted on several cores (docs/adr/0123).
const PARALLEL_FROM: usize = 2048;
/// Past this many selected objects the selection is highlighted as the box round it, not object by object
/// (AutoCAD's SELECTIONPREVIEWLIMIT default, docs/adr/1027): building a million objects' highlight held the screen
/// for over half a second.
pub(crate) const HIGHLIGHT_LIMIT: usize = 20_000;

impl Viewport {
    /// The selection's part and the hovered object's, from the cache when
    /// nothing they depend on changed; `spatial` places the blocks' inserts.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn highlights(
        &self,
        doc: &Document,
        selection: &Selection,
        accent: Rgba8,
        fixed: &ScenePart,
        curves: &ScenePart,
        clip: &Bounds,
        spatial: Option<&Spatial>,
    ) -> (Arc<ScenePart>, Arc<ScenePart>) {
        let key = HighlightKey {
            generation: self.generation,
            changes: changes(doc),
            selection: selection.version(),
            hover: 0,
            accent,
            below: fixed.layers.len(),
            tolerance: curves.tolerance.to_bits(),
            clip: [clip.min_x, clip.min_y, clip.max_x, clip.max_y].map(f64::to_bits),
            theme: self.theme.as_ref().map_or(0, |t| {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                t.id.hash(&mut h);
                h.finish() | 1
            }),
        };
        // The view's theme hides some layers: their objects are not highlighted in it (docs/adr/0637 §3).
        let tree = self.themed_tree(doc);
        let shown = |slot: &Slot| {
            tree.as_ref().is_none_or(|t| {
                doc.model
                    .get(*slot)
                    .is_none_or(|e| t.is_visible(&e.base().layer_id))
            })
        };
        let ring = kentos_render_wgpu::layout::marker_shape::RING;
        let build = |ids: &mut dyn Iterator<Item = Slot>, style: Highlight| {
            let mut pieces = Vec::new();
            let objects = objects_of(doc, spatial, ids, &mut pieces);
            let moved = on_map(&objects, self.camera.map);
            let objects = moved.as_ref().map_or(objects, |m| m.iter().collect());
            Arc::new(scene::build_highlight(
                objects,
                &style,
                fixed.origin,
                curves.tolerance,
                key.below,
                clip,
            ))
        };
        let mut cache = self.highlight.borrow_mut();
        let selected = match &cache.selected {
            Some((known, part)) if *known == key => part.clone(),
            _ => {
                let style = Highlight {
                    color: accent,
                    fill: Some(accent.with_alpha(SELECTION_FILL)),
                    mark_size: HIGHLIGHT_MARK,
                    mark_shape: ring,
                };
                // A large selection (select all, a window over a map) is built on
                // the machine's cores; the part is the one built in turn. Past the limit, the box round it alone.
                let boxed = (selection.len() > HIGHLIGHT_LIMIT)
                    .then(|| spatial.and_then(|sp| selection_box(sp, selection.ids())))
                    .flatten();
                let part = if let Some(frame) = boxed {
                    let style = Highlight { fill: None, ..style };
                    Arc::new(scene::build_highlight(
                        vec![&frame],
                        &style,
                        fixed.origin,
                        curves.tolerance,
                        key.below,
                        clip,
                    ))
                } else if selection.len() >= PARALLEL_FROM {
                    let threads = std::thread::available_parallelism()
                        .map_or(1, |n| n.get())
                        .clamp(1, 8);
                    let mut pieces: Vec<Vec<Entity>> = Vec::new();
                    let objects = objects_of_parallel(
                        doc,
                        spatial,
                        &mut selection.ids().iter().copied().filter(|s| shown(s)),
                        &mut pieces,
                        threads,
                    );
                    let moved = on_map(&objects, self.camera.map);
                    let objects = moved.as_ref().map_or(objects, |m| m.iter().collect());
                    Arc::new(scene::build_highlight_parallel(
                        &objects,
                        &style,
                        fixed.origin,
                        curves.tolerance,
                        key.below,
                        clip,
                        threads,
                    ))
                } else {
                    build(
                        &mut selection.ids().iter().copied().filter(|s| shown(s)),
                        style,
                    )
                };
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
                        .filter(|h| !selection.contains(*h) && shown(h))
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

/// The box round the selected objects as the geometry store keeps their bounds (docs/adr/1027): one closed area.
/// None when none of them is in the store.
fn selection_box(spatial: &Spatial, ids: &[Slot]) -> Option<Entity> {
    let store = spatial.store();
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for slot in ids {
        if let Some(it) = store.get(f64::from(slot.0)) {
            let b = &it.bounds;
            lo = [lo[0].min(b.min_x), lo[1].min(b.min_y)];
            hi = [hi[0].max(b.max_x), hi[1].max(b.max_y)];
        }
    }
    if !(lo[0] <= hi[0] && lo[1] <= hi[1]) {
        return None;
    }
    serde_json::from_value(serde_json::json!({"kind": "polygon", "id": 0, "layerId": "", "attrs": {},
        "pts": [{"x": lo[0], "y": lo[1]}, {"x": hi[0], "y": lo[1]}, {"x": hi[0], "y": hi[1]}, {"x": lo[0], "y": hi[1]}]}))
    .ok()
}

/// The objects moved into the map's system (docs/adr/0583 §3), their vertices; none without a map. An object whose
/// curve does not move vertex by vertex, or that the map does not reach, is not highlighted there.
fn on_map(
    objects: &[&Entity],
    map: Option<&'static kentos_geometry_core::crs::map::MapProjection>,
) -> Option<Vec<Entity>> {
    let m = map?;
    Some(
        objects
            .iter()
            .filter_map(|e| {
                let mut e = (*e).clone();
                kentos_formats::reproject::move_entity(&mut e, &m.project, &m.map, &m.choices)
                    .ok()
                    .map(|()| e)
            })
            .collect(),
    )
}

/// The objects to highlight, a block's insert by its pieces (docs/adr/0144),
/// which `pieces` holds.
pub(crate) fn objects_of<'a>(
    doc: &'a Document,
    spatial: Option<&Spatial>,
    ids: &mut dyn Iterator<Item = Slot>,
    pieces: &'a mut Vec<Entity>,
) -> Vec<&'a Entity> {
    let mut objects = Vec::new();
    for slot in ids {
        let Some(e) = doc.model.get(slot) else {
            continue;
        };
        match spatial.map(|s|s.represented(e)).unwrap_or_else(||Some(std::borrow::Cow::Borrowed(e))) {
            None=>{},
            Some(std::borrow::Cow::Owned(e))=>pieces.push(e),
            Some(std::borrow::Cow::Borrowed(e))=>match spatial.and_then(|s|s.pieces(e)) {Some(placed)=>pieces.extend(placed),None=>objects.push(e)},
        }
    }
    let pieces: &'a Vec<Entity> = pieces;
    objects.extend(pieces.iter());
    objects
}

/// [`objects_of`] for a large selection (docs/adr/1066 §3): the inserts' pieces made into objects on `threads` cores,
/// the store read on this thread; the same objects in the same order (each core's kept where it made them).
pub(crate) fn objects_of_parallel<'a>(
    doc: &'a Document,
    spatial: Option<&Spatial>,
    ids: &mut dyn Iterator<Item = Slot>,
    pieces: &'a mut Vec<Vec<Entity>>,
    threads: usize,
) -> Vec<&'a Entity> {
    // A sheet's view represents the objects otherwise: as one by one.
    let Some(sp) = spatial.filter(|s| !s.has_sheet_view()) else {
        pieces.push(Vec::new());
        let [own] = pieces.as_mut_slice() else {
            unreachable!("one list was pushed");
        };
        return objects_of(doc, spatial, ids, own);
    };
    let mut objects = Vec::new();
    let mut sources = Vec::new();
    for slot in ids {
        let Some(e) = doc.model.get(slot) else {
            continue;
        };
        match sp.pieces_source(e) {
            Some((x, shown)) => sources.push((e, x, shown)),
            None => objects.push(e),
        }
    }
    let chunk = sources.len().div_ceil(threads.max(1)).max(1);
    let made: Vec<Vec<Entity>> = std::thread::scope(|scope| {
        let handles: Vec<_> = sources
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    let mut out = Vec::new();
                    for (e, x, shown) in part {
                        out.extend(kentos_native_application::blocks::piece_entities_shown(e, x, |i| shown[i]));
                    }
                    out
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });
    *pieces = made;
    let pieces: &'a Vec<Vec<Entity>> = pieces;
    objects.extend(pieces.iter().flatten());
    objects
}

#[cfg(test)]
mod tests {
    /// docs/adr/1027: past the limit the selection is highlighted as the box round it: the box is the store's bounds of
    /// the selected objects (30 001 lines in a 300 × 100 m grid), a single closed area.
    #[test]
    fn a_selection_past_the_limit_is_highlighted_as_its_box() {
        let mut app = crate::files_testing::app_with_drawing();
        let mut o = crate::tools_scenes::Objects::new();
        for k in 0..30_001u32 {
            let (x, y) = (f64::from(k % 300), f64::from(k / 300));
            o.line("cizim", [x, y], [x + 0.5, y + 0.25]);
        }
        crate::tools_scenes::open(&mut app, o);
        let ids: Vec<Slot> = (1..=30_001).map(Slot).collect();
        assert!(ids.len() > super::HIGHLIGHT_LIMIT);
        app.spatial.sync(&app.document.as_ref().expect("open").model);
        let frame = super::selection_box(&app.spatial, &ids).expect("a box");
        let kentos_commands::Entity::Polygon(p) = frame else { panic!("a closed area") };
        let (e, n) = (crate::tools_scenes::E, crate::tools_scenes::N);
        let xs: Vec<f64> = p.pts.iter().map(|q| q.x - e).collect();
        let ys: Vec<f64> = p.pts.iter().map(|q| q.y - n).collect();
        let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
        assert!(near(xs.iter().cloned().fold(f64::INFINITY, f64::min), 0.0) && near(xs.iter().cloned().fold(f64::MIN, f64::max), 299.5));
        assert!(near(ys.iter().cloned().fold(f64::INFINITY, f64::min), 0.0) && near(ys.iter().cloned().fold(f64::MIN, f64::max), 100.25));
    }

    use iced::{Point, Rectangle};
    use kentos_commands::Entity;
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
        let (_, fixed, curves, _, clip) = viewport.scene(&doc, Mode::Dark, &palette, &settings);
        let accent = Rgba8::rgb(0x4c, 0x9b, 0xe8);
        let mut selection = Selection::new();
        let ids: Vec<Slot> = doc.model.entities().map(|e| Slot(e.base().id)).collect();
        selection.set(ids.iter().copied().take(ids.len() - 1));
        let (sel, hover) =
            viewport.highlights(&doc, &selection, accent, &fixed, &curves, &clip, None);
        assert!(!sel.segments.is_empty() && hover.segments.is_empty());
        selection.set_hover(ids.last().copied());
        let (sel2, hover2) =
            viewport.highlights(&doc, &selection, accent, &fixed, &curves, &clip, None);
        assert_eq!(sel2.id, sel.id, "the selection's part is kept");
        assert_ne!(hover2.id, hover.id);
        let (sel3, hover3) =
            viewport.highlights(&doc, &selection, accent, &fixed, &curves, &clip, None);
        assert_eq!((sel3.id, hover3.id), (sel.id, hover2.id), "nothing changed");
        // A selected object is not hovered over its own highlight.
        selection.set_hover(ids.first().copied());
        let (_, hover4) =
            viewport.highlights(&doc, &selection, accent, &fixed, &curves, &clip, None);
        assert!(hover4.segments.is_empty() && hover4.fills.is_empty());
        // Its colour and layer: after every layer of the scene.
        assert!(sel.segments.iter().all(|s| s.color == accent.0));
        assert_eq!(sel.layers.len(), fixed.layers.len() + 1);
    }

    /// A view whose theme hides a layer highlights none of its objects (docs/adr/0637 §3): neither selected nor
    /// hovered; the same selection in a view without a theme is.
    #[test]
    fn a_theme_hides_the_highlight_of_what_it_hides() {
        let doc = sample();
        let palette = palette(Mode::Dark);
        let settings = RenderSettings::new(palette.background);
        let accent = Rgba8::rgb(0x4c, 0x9b, 0xe8);
        let first = doc.model.entities().next().expect("an object");
        let (slot, layer) = (Slot(first.base().id), first.base().layer_id.clone());
        let mut selection = Selection::new();
        selection.set([slot]);
        let plain = Viewport::new();
        let (_, fixed, curves, _, clip) = plain.scene(&doc, Mode::Dark, &palette, &settings);
        let (sel, _) = plain.highlights(&doc, &selection, accent, &fixed, &curves, &clip, None);
        assert!(!sel.segments.is_empty() || !sel.fills.is_empty() || !sel.markers.is_empty());
        // A theme that hides the object's layer.
        let mut themed = Viewport::new();
        themed.theme = Some(kentos_commands::LayerState {
            id: "gizli".into(),
            name: "Gizli".into(),
            nodes: vec![kentos_commands::LayerStateNode {
                node: layer.clone(),
                visible: false,
                locked: None,
                style: None,
                frozen: None,
                plot: None,
                path: None,
            }],
        });
        let (sel, hover) = themed.highlights(&doc, &selection, accent, &fixed, &curves, &clip, None);
        assert!(sel.segments.is_empty() && sel.fills.is_empty() && sel.markers.is_empty());
        let mut hovering = Selection::new();
        hovering.set_hover(Some(slot));
        let (_, hover2) = themed.highlights(&doc, &hovering, accent, &fixed, &curves, &clip, None);
        assert!(hover.segments.is_empty() && hover2.segments.is_empty() && hover2.markers.is_empty());
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
        use kentos_commands::{EntityBase, LineEntity, PathEntity, Vec2 as Wire};
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
            look: Default::default(),
            id,
            layer_id: layer.into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
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
                        space_separator: None,
                        ms: None,
                        base: base(id, &layer),
                        pts,
                        bulges: None,
                        holes: None,
                        zs: None,
                        parts: None,
                        closed: false,
                        widths: None,
                    }));
                    id += 1;
                }
                let y = n + row as f64 * 20.0 + 19.0;
                snapshot.entities.push(Entity::Line(LineEntity {
                    ma: None,
                    mb: None,
                    base: base(id, &layer),
                    a: Wire { x: e, y },
                    b: Wire {
                        x: e + side as f64 * 20.0,
                        y,
                    },
                    za: None,
                    zb: None,
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
            viewport.opened(&doc, None);
            viewport.update(Event::Extents, Some(&doc));
            let palette = palette(Mode::Dark);
            let settings = RenderSettings::new(palette.background);
            let t = Instant::now();
            let (_, fixed, curves, _, clip) = viewport.scene(&doc, Mode::Dark, &palette, &settings);
            let scene_time = t.elapsed();
            let mut selection = Selection::new();
            selection.set(doc.model.entities().map(|e| Slot(e.base().id)));
            let accent = Rgba8::rgb(0x4c, 0x9b, 0xe8);
            let t = Instant::now();
            let (part, _) =
                viewport.highlights(&doc, &selection, accent, &fixed, &curves, &clip, None);
            let part_time = t.elapsed();
            let t = Instant::now();
            let _ = viewport.highlights(&doc, &selection, accent, &fixed, &curves, &clip, None);
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

    /// A large selection's inserts made into objects on several cores are the objects one core makes, in the same
    /// order (docs/adr/1066 §3).
    #[test]
    fn a_large_selection_is_made_into_the_same_objects_on_several_cores() {
        let (snapshot, _) = crate::perf::mixed::drawing(4_000, crate::perf::mixed::SEED);
        let doc = Document::from_v2(snapshot, None).expect("the drawing opens");
        let mut spatial = Spatial::new();
        spatial.reload(&doc.model);
        let ids: Vec<Slot> = doc.model.entities().map(|e| Slot(e.base().id)).collect();
        let mut one = Vec::new();
        let a = objects_of(&doc, Some(&spatial), &mut ids.iter().copied(), &mut one);
        let mut many = Vec::new();
        let b = objects_of_parallel(&doc, Some(&spatial), &mut ids.iter().copied(), &mut many, 8);
        assert!(a.len() > ids.len(), "the inserts' pieces: {} objects", a.len());
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).all(|(x, y)| x == y), "the same objects in the same order");
    }
}

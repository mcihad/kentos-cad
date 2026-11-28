//! Building the scene from a `.kcad` v1 document (TODOS.md REN-01, REN-07):
//! what is drawn, in which order and colour, how finely curves are cut, and
//! that the document is only read.

use kentos_contracts::{DocumentSnapshotV1, Entity, LayerNode};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::text::Font;
use kentos_render_wgpu::precision::{MAX_SEGMENT_LENGTH, join};
use kentos_render_wgpu::scene::{self, lod};
use kentos_render_wgpu::{Palette, Rgba8, ScenePart, Vec2};

const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");

fn sample() -> DocumentSnapshotV1 {
    DocumentSnapshotV1::from_json(SAMPLE).expect("the sample reads")
}

fn palette() -> Palette {
    Palette {
        background: Rgba8::rgb(0x14, 0x1a, 0x21),
        fg: Rgba8::rgb(0xe4, 0xea, 0xf0),
        fg_dim: Rgba8::rgb(0xa3, 0xaf, 0xbc),
        ink: Rgba8::rgb(0xff, 0xff, 0xff),
    }
}

fn find<'a>(nodes: &'a mut [LayerNode], id: &str) -> Option<&'a mut LayerNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find(&mut node.children, id) {
            return Some(found);
        }
    }
    None
}

fn origin(doc: &DocumentSnapshotV1) -> Vec2 {
    scene::scene_origin(doc)
}

/// Segment ends of a part in world coordinates.
fn segments(part: &ScenePart) -> Vec<(Vec2, Vec2)> {
    let o = part.origin;
    part.segments
        .iter()
        .map(|s| {
            let a = join(s.a_hi, s.a_lo);
            let b = join(s.b_hi, s.b_lo);
            (
                Vec2::new(o.x + a[0], o.y + a[1]),
                Vec2::new(o.x + b[0], o.y + b[1]),
            )
        })
        .collect()
}

#[test]
fn the_sample_draws_its_visible_layers_bottom_first_in_their_colours() {
    let doc = sample();
    let fixed = scene::build_fixed(&doc, &palette(), origin(&doc));
    let curves = scene::build_curves(&doc, &palette(), origin(&doc), 0.001, 1_000_000);
    // Kadastro/{Parsel, Bina} are visible, Çizim is hidden: Bina (lower in the list) first.
    assert_eq!(fixed.layers.len(), 2);
    assert_eq!(curves.layers.len(), 2);
    // Bina: the hatch's lines (fixed), the circle and the arc (curves), all in `ink`.
    let bina_fixed = &fixed.layers[0];
    let bina_curves = &curves.layers[0];
    assert!(!bina_fixed.segments.is_empty(), "the hatch is drawn");
    assert!(
        !bina_curves.segments.is_empty(),
        "the circle and the arc are drawn"
    );
    let ink = palette().ink.0;
    for s in &fixed.segments[bina_fixed.segments.start as usize..bina_fixed.segments.end as usize] {
        assert_eq!(s.color, ink);
    }
    // Parsel: the bulged polygon with its hole, in #E06C75; the layer has no fill.
    let parsel = &curves.layers[1];
    assert!(!parsel.segments.is_empty());
    assert!(parsel.fills.is_empty());
    for s in &curves.segments[parsel.segments.start as usize..parsel.segments.end as usize] {
        assert_eq!(s.color, [0xe0, 0x6c, 0x75, 0xff]);
    }
    // Nothing on the hidden layer: no point mark, and nothing counted as undrawn.
    assert!(fixed.markers.is_empty());
    assert!(fixed.not_drawn.is_empty(), "{:?}", fixed.not_drawn);
    // Every value the GPU gets is finite.
    for part in [&fixed, &curves] {
        for s in &part.segments {
            assert!(
                [s.a_hi, s.a_lo, s.b_hi, s.b_lo]
                    .iter()
                    .flatten()
                    .all(|v| v.is_finite())
            );
        }
    }
}

#[test]
fn showing_the_hidden_layer_draws_its_objects_its_dimension_and_its_construction_lines() {
    let mut doc = sample();
    find(&mut doc.layers, "cizim").expect("the layer").visible = true;
    let fixed = scene::build_fixed(&doc, &palette(), origin(&doc));
    let curves = scene::build_curves(&doc, &palette(), origin(&doc), 0.001, 1_000_000);
    assert_eq!(fixed.layers.len(), 3);
    // Çizim is last in the list: drawn first, at the bottom (the list's top is drawn on top).
    let cizim = &fixed.layers[0];
    assert_eq!(cizim.markers.len(), 1, "the point");
    let mark = fixed.markers[0];
    assert_eq!(mark.size, 8.0, "the layer's point size");
    assert_eq!(mark.shape, kentos_render_wgpu::layout::marker_shape::CROSS);
    // The line's own colour overrides the layer's.
    assert!(
        fixed.segments[cizim.segments.start as usize..cizim.segments.end as usize]
            .iter()
            .any(|s| s.color == [0xff, 0, 0, 0xff])
    );
    // Bulged polyline, ellipse and spline are curves.
    assert!(curves.layers[0].segments.len() > 3);
    // The dimension is its layout's lines; text is the host's, over the scene.
    let without: Vec<kentos_contracts::Entity> = doc
        .entities
        .iter()
        .filter(|e| e.kind() != "dimension")
        .cloned()
        .collect();
    let no_dimension = kentos_contracts::DocumentSnapshotV1 {
        entities: without,
        ..doc.clone()
    };
    let plain = scene::build_fixed(&no_dimension, &palette(), origin(&doc));
    assert!(
        fixed.layers[0].segments.len() >= plain.layers[0].segments.len() + 3,
        "extension lines, the dimension line, its arrows"
    );
    assert!(fixed.not_drawn.is_empty(), "{:?}", fixed.not_drawn);
    // The infinite line and the ray, clipped to a box around the drawing.
    let b = scene::extents(&doc).expect("objects");
    let around = kentos_render_wgpu::Bounds {
        min_x: b.min_x - 100.0,
        min_y: b.min_y - 100.0,
        max_x: b.max_x + 100.0,
        max_y: b.max_y + 100.0,
    };
    let construction = scene::build_construction(&doc, &palette(), origin(&doc), &around);
    assert_eq!(
        construction.layers.len(),
        3,
        "the same layers as the other parts"
    );
    let lines = &construction.layers[0].segments;
    assert!(lines.end > lines.start, "both drawn inside the box");
    // A box they do not cross draws nothing.
    let far = kentos_render_wgpu::Bounds {
        min_x: b.max_x + 1.0e6,
        min_y: b.max_y + 1.0e6,
        max_x: b.max_x + 1.0e6 + 1.0,
        max_y: b.max_y + 1.0e6 + 1.0,
    };
    let nothing = scene::build_construction(&doc, &palette(), origin(&doc), &far);
    assert!(nothing.segments.len() <= construction.segments.len());
}

#[test]
fn a_hidden_group_hides_its_layers() {
    let mut doc = sample();
    find(&mut doc.layers, "layer-g").expect("the group").visible = false;
    let fixed = scene::build_fixed(&doc, &palette(), origin(&doc));
    assert!(
        fixed.layers.is_empty(),
        "Parsel and Bina are in the hidden group; Çizim is hidden"
    );
    assert!(fixed.segments.is_empty());
}

#[test]
fn curves_keep_within_the_chord_tolerance() {
    let doc = sample();
    // The circle: centre (486524.34, 4420199.52), r = 3.25, on Bina.
    let c = Vec2::new(486_524.34, 4_420_199.52);
    for tolerance in [0.1, 0.01, 0.001, 0.0001] {
        let part = scene::build_curves(&doc, &palette(), origin(&doc), tolerance, 1_000_000);
        assert_eq!(part.tolerance, tolerance, "within the budget: as asked");
        let on_circle: Vec<_> = segments(&part)
            .into_iter()
            .filter(|(a, b)| {
                let ra = (a.x - c.x).hypot(a.y - c.y);
                let rb = (b.x - c.x).hypot(b.y - c.y);
                (ra - 3.25).abs() < 1e-6 && (rb - 3.25).abs() < 1e-6
            })
            .collect();
        assert!(
            on_circle.len() >= 8,
            "{tolerance}: {} chords",
            on_circle.len()
        );
        for (a, b) in on_circle {
            let mid = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            let sagitta = 3.25 - (mid.x - c.x).hypot(mid.y - c.y);
            assert!(
                sagitta <= tolerance * 1.000_001,
                "{tolerance}: sagitta {sagitta}"
            );
        }
    }
}

#[test]
fn the_zoom_band_keeps_chords_on_screen_within_tolerance() {
    let tolerance_px = 0.25;
    let mut scale = 1e-3;
    while scale < 5e3 {
        let built = lod::build_band(scale, tolerance_px);
        // Until the view needs a rebuild, chords stay within the tolerance on screen.
        let mut s = scale;
        while !lod::stale(built, lod::band(s, tolerance_px)) {
            assert!(
                lod::tolerance(built) * s <= tolerance_px * 1.000_001,
                "{scale} → {s}"
            );
            s *= 1.1;
        }
        // Zooming in from the build rebuilds after at most 4×.
        assert!(s / scale <= 4.0 * 1.1 + 1e-9, "{scale}: rebuilt at {s}");
        // Zooming out rebuilds past 8× (chords needlessly fine), not long after 16×.
        let mut s = scale;
        while !lod::stale(built, lod::band(s, tolerance_px)) {
            s /= 1.1;
        }
        assert!(
            scale / s > 8.0 && scale / s < 64.0,
            "{scale}: coarsened at {s}"
        );
        scale *= 3.3;
    }
}

#[test]
fn a_budget_coarsens_the_tolerance_instead_of_exhausting_memory() {
    let mut doc = sample();
    let circle = doc
        .entities
        .iter()
        .find(|e| matches!(e, Entity::Circle(_)))
        .cloned()
        .expect("a circle");
    for _ in 0..500 {
        doc.entities.push(circle.clone());
    }
    let generous = scene::build_curves(&doc, &palette(), origin(&doc), 1e-5, usize::MAX);
    let tight = scene::build_curves(&doc, &palette(), origin(&doc), 1e-5, 100_000);
    assert!(generous.segments.len() > 100_000);
    assert!(tight.segments.len() <= 100_000, "{}", tight.segments.len());
    assert!(tight.tolerance > 1e-5);
}

#[test]
fn long_segments_are_cut_into_collinear_pieces_that_keep_the_exact_ends() {
    let mut doc = sample();
    let Some(Entity::Polygon(polygon)) = doc
        .entities
        .iter()
        .find(|e| matches!(e, Entity::Polygon(_)))
        .cloned()
    else {
        panic!("a polygon");
    };
    // A straight 12.5 km square on Parsel.
    let mut square = polygon;
    let (x, y) = (486_000.0, 4_415_000.0);
    square.pts = vec![
        kentos_contracts::Vec2 { x, y },
        kentos_contracts::Vec2 { x: x + 12_500.0, y },
        kentos_contracts::Vec2 {
            x: x + 12_500.0,
            y: y + 12_500.0,
        },
        kentos_contracts::Vec2 { x, y: y + 12_500.0 },
    ];
    square.bulges = None;
    square.holes = None;
    doc.entities = vec![Entity::Polygon(square)];
    let fixed = scene::build_fixed(&doc, &palette(), origin(&doc));
    let pieces = segments(&fixed);
    let per_side = (12_500.0 / MAX_SEGMENT_LENGTH).ceil() as usize;
    assert_eq!(pieces.len(), 4 * per_side);
    for (a, b) in &pieces {
        assert!((b.x - a.x).hypot(b.y - a.y) <= MAX_SEGMENT_LENGTH + 1e-6);
        // Collinear with the square's sides: every piece is horizontal or vertical.
        assert!((a.x - b.x).abs() < 1e-6 || (a.y - b.y).abs() < 1e-6);
    }
    // The corners are where the document has them (to the split's nanometres).
    let first = pieces[0].0;
    assert!((first.x - x).abs() < 1e-8 && (first.y - y).abs() < 1e-8);
    let (_, end) = pieces[per_side - 1];
    assert!((end.x - (x + 12_500.0)).abs() < 1e-8 && (end.y - y).abs() < 1e-8);
}

#[test]
fn a_filled_polygon_covers_its_area_minus_its_holes() {
    let mut doc = sample();
    // Give Parsel a translucent fill; its polygon has bulges (arcs) and a 5 × 5 hole.
    find(&mut doc.layers, "parsel")
        .expect("the layer")
        .style
        .fill = Some("#E06C7533".into());
    let tolerance = 0.0005;
    let curves = scene::build_curves(&doc, &palette(), origin(&doc), tolerance, 1_000_000);
    let parsel = &curves.layers[1];
    assert!(!parsel.fills.is_empty());
    let mut area = 0.0;
    for tri in curves.fills[parsel.fills.start as usize..parsel.fills.end as usize].chunks(3) {
        let p: Vec<[f64; 2]> = tri.iter().map(|v| join(v.hi, v.lo)).collect();
        area += ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1])
            - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]))
            / 2.0;
        assert_eq!(tri[0].color, [0xe0, 0x6c, 0x75, 0x33]);
    }
    // The geometry core's area of the bulged ring (exact arcs), minus the hole.
    let Some(Entity::Polygon(p)) = doc
        .entities
        .iter()
        .find(|e| matches!(e, Entity::Polygon(_)))
    else {
        panic!("the polygon");
    };
    let pts: Vec<Vec2> = p.pts.iter().map(|q| Vec2::new(q.x, q.y)).collect();
    let exact =
        kentos_geometry_core::geom::bulge::bulge_ring_area(&pts, p.bulges.as_deref()).abs() - 25.0;
    // Chords cut off at most the tolerance along the arcs' length.
    assert!(
        (area.abs() - exact).abs() < tolerance * 60.0,
        "{area} vs {exact}"
    );
}

#[test]
fn building_never_changes_the_document_and_rejects_no_value_silently() {
    let doc = sample();
    let before = doc.clone();
    let _ = scene::build_fixed(&doc, &palette(), origin(&doc));
    let _ = scene::build_curves(&doc, &palette(), origin(&doc), 0.01, 1_000_000);
    let _ = scene::extents(&doc);
    assert_eq!(doc, before);

    // Non-finite or degenerate geometry is left out, never uploaded.
    let mut bad = sample();
    for entity in &mut bad.entities {
        match entity {
            Entity::Line(l) => l.b.x = f64::NAN,
            Entity::Circle(c) => c.r = f64::INFINITY,
            Entity::Arc(a) => a.r = -4.0,
            Entity::Polygon(p) => p.pts[1].y = f64::NEG_INFINITY,
            _ => {}
        }
    }
    find(&mut bad.layers, "cizim").expect("the layer").visible = true;
    let fixed = scene::build_fixed(&bad, &palette(), origin(&bad));
    let curves = scene::build_curves(&bad, &palette(), origin(&bad), 0.01, 1_000_000);
    for part in [&fixed, &curves] {
        for s in &part.segments {
            assert!(
                [s.a_hi, s.a_lo, s.b_hi, s.b_lo]
                    .iter()
                    .flatten()
                    .all(|v| v.is_finite())
            );
        }
        for v in &part.fills {
            assert!([v.hi, v.lo].iter().flatten().all(|c| c.is_finite()));
        }
    }
}

#[test]
fn extents_are_the_web_stores_extents() {
    let doc = sample();
    let ours = scene::extents(&doc).expect("the sample has extents");
    // The geometry store reads the objects from the web's JSON itself.
    let mut store = Store::new();
    store.set_font(Font::from_id("barlow"));
    store
        .put_json(&serde_json::to_string(&doc.entities).expect("entities write"))
        .expect("the store reads them");
    let theirs = store.extent(None).expect("the store has extents");
    assert_eq!(ours, theirs);
    // Everything drawn lies inside.
    let fixed = scene::build_fixed(&doc, &palette(), origin(&doc));
    for (a, b) in segments(&fixed) {
        for p in [a, b] {
            assert!(p.x >= ours.min_x - 1e-6 && p.x <= ours.max_x + 1e-6);
            assert!(p.y >= ours.min_y - 1e-6 && p.y <= ours.max_y + 1e-6);
        }
    }
}

/// A host's own document, read in place: the scene needs no snapshot copy.
struct Live {
    layers: Vec<LayerNode>,
    objects: Vec<Entity>,
    anchor: kentos_contracts::Vec2,
}

impl scene::Drawing for Live {
    fn layer_tree(&self) -> &[LayerNode] {
        &self.layers
    }

    fn objects(&self) -> impl Iterator<Item = &Entity> {
        self.objects.iter()
    }

    fn anchor(&self) -> kentos_contracts::Vec2 {
        self.anchor
    }

    fn drawing_font(&self) -> Option<kentos_contracts::DrawingFont> {
        None
    }
}

#[test]
fn any_drawing_builds_the_same_scene_as_its_snapshot() {
    let doc = sample();
    let live = Live {
        layers: doc.layers.clone(),
        objects: doc.entities.clone(),
        anchor: doc.origin,
    };
    let from_snapshot = scene::build_fixed(&doc, &palette(), origin(&doc));
    let from_live = scene::build_fixed(&live, &palette(), scene::scene_origin(&live));
    assert_eq!(from_live.segments, from_snapshot.segments);
    assert_eq!(from_live.layers, from_snapshot.layers);
    assert_eq!(scene::extents(&live), scene::extents(&doc));
}

#[test]
fn every_build_is_a_new_part() {
    let doc = sample();
    let a = scene::build_fixed(&doc, &palette(), origin(&doc));
    let b = scene::build_fixed(&doc, &palette(), origin(&doc));
    assert_ne!(a.id, b.id);
    assert_eq!(
        a.segments, b.segments,
        "the same document gives the same data"
    );
}

/// A highlight (the selection, the hovered object; docs/adr/0029) draws its
/// objects in one colour after every layer of the scene: its ranges sit
/// past the scene's layers, closed areas take the highlight's fill, points
/// its marks, and curves come tessellated in the same part.
#[test]
fn a_highlight_draws_after_the_scene_in_one_colour() {
    let doc = sample();
    let o = origin(&doc);
    let fixed = scene::build_fixed(&doc, &palette(), o);
    let below = fixed.layers.len();
    let accent = Rgba8::rgb(0x4c, 0x9b, 0xe8);
    let style = scene::Highlight {
        color: accent,
        fill: Some(accent.with_alpha(0.13)),
        mark_size: 15.0,
        mark_shape: kentos_render_wgpu::layout::marker_shape::RING,
    };
    let all: Vec<&Entity> = doc.entities.iter().collect();
    let clip = kentos_render_wgpu::Bounds {
        min_x: -1e6,
        min_y: -1e6,
        max_x: 1e6,
        max_y: 1e6,
    };
    let part = scene::build_highlight(all.iter().copied(), &style, o, 0.05, below, &clip, None);
    assert_eq!(part.layers.len(), below + 1, "one layer past the scene's");
    assert!(
        part.layers[..below]
            .iter()
            .all(|l| l.segments.is_empty() && l.fills.is_empty() && l.markers.is_empty())
    );
    assert!(!part.segments.is_empty());
    assert!(part.segments.iter().all(|s| s.color == accent.0));
    let polygons = doc
        .entities
        .iter()
        .filter(|e| matches!(e, Entity::Polygon(_) | Entity::Hatch(_)))
        .count();
    assert!(
        polygons > 0 && !part.fills.is_empty(),
        "closed areas are filled"
    );
    assert!(
        part.fills
            .iter()
            .all(|f| f.color == accent.with_alpha(0.13).0)
    );
    let hover = scene::build_highlight(
        all.iter().copied(),
        &scene::Highlight {
            fill: None,
            ..style
        },
        o,
        0.05,
        below,
        &clip,
        None,
    );
    assert!(
        hover.fills.is_empty(),
        "the hovered object is outlined only"
    );
    assert!(
        scene::build_highlight(std::iter::empty(), &style, o, 0.05, below, &clip, None)
            .segments
            .is_empty()
    );
}

/// The grid's spacing as the web's `gridSpacing`: the smallest 1-2-5 step
/// keeping minor lines 14 px apart, a major every five (four after a 5);
/// its lines on round world values, minor and major one layer each.
#[test]
fn the_grid_spaces_its_lines_as_the_webs() {
    assert_eq!(scene::grid_spacing(1.0, 14.0), (20.0, 100.0));
    assert_eq!(scene::grid_spacing(0.5, 14.0), (50.0, 200.0));
    let (minor, major) = scene::grid_spacing(100.0, 14.0);
    assert!((minor - 0.2).abs() < 1e-12 && (major - 1.0).abs() < 1e-12);
    let area = kentos_render_wgpu::Bounds {
        min_x: 486_503.0,
        min_y: 4_420_101.0,
        max_x: 486_603.0,
        max_y: 4_420_151.0,
    };
    let colors = [Rgba8([255, 255, 255, 0x0a]), Rgba8([255, 255, 255, 0x17])];
    let part = scene::build_grid(
        &area,
        (20.0, 100.0),
        Vec2::new(486_500.0, 4_420_100.0),
        colors,
    );
    assert_eq!(part.layers.len(), 2, "minor, then major");
    let segments = |i: usize| part.layers[i].segments.end - part.layers[i].segments.start;
    // From the round value at or below the area (the web's `x0`): x 486500…486600
    // every 20 (486500 and 486600 major), y 4420100…4420140 (4420100 major).
    assert_eq!((segments(0), segments(1)), (6, 3));
    assert!(part.segments[..6].iter().all(|s| s.color == colors[0].0));
}

/// A drawing of numbers near the largest float, shown whole (fixtures/kcad/v2/drawing.json; plan/render.md RM-16): the
/// view's box is not finite or far wider than its spacing can count. The grid is empty and comes back at once; its
/// line count once ran past `u32` (an overflow in a debug build, a loop without end in a release one).
#[test]
fn a_grid_over_a_box_it_cannot_count_is_empty() {
    let colors = [Rgba8([255, 255, 255, 0x0a]), Rgba8([255, 255, 255, 0x17])];
    let origin = Vec2::new(0.0, 0.0);
    let boxes = [
        kentos_render_wgpu::Bounds {
            min_x: -1.0,
            min_y: -1.0,
            max_x: f64::INFINITY,
            max_y: 1.0,
        },
        kentos_render_wgpu::Bounds {
            min_x: f64::NAN,
            min_y: -1.0,
            max_x: 1.0,
            max_y: 1.0,
        },
        kentos_render_wgpu::Bounds {
            min_x: -1e300,
            min_y: -1.0,
            max_x: 1e300,
            max_y: 1.0,
        },
        // A spacing below the numbers' precision: x0 + i·minor never moves.
        kentos_render_wgpu::Bounds {
            min_x: 4.5e15,
            min_y: 0.0,
            max_x: 4.5e15 + 64.0,
            max_y: 1.0,
        },
    ];
    for (k, area) in boxes.iter().enumerate() {
        let spacing = if k == 3 { (0.1, 0.5) } else { (1.0, 5.0) };
        let part = scene::build_grid(area, spacing, origin, colors);
        assert!(part.segments.is_empty(), "box {k}: no grid");
        let turned =
            scene::build_grid_framed(area, spacing, origin, colors, Vec2::new(1.0, 1.0), 0.3);
        assert!(turned.segments.is_empty(), "box {k}: no turned grid");
    }
    // Twenty thousand lines are still drawn; more are not.
    let wide = kentos_render_wgpu::Bounds {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 19_990.0,
        max_y: 1.0,
    };
    assert!(
        !scene::build_grid(&wide, (1.0, 5.0), origin, colors)
            .segments
            .is_empty()
    );
    let wider = kentos_render_wgpu::Bounds {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 20_010.0,
        max_y: 1.0,
    };
    assert!(
        scene::build_grid(&wider, (1.0, 5.0), origin, colors)
            .segments
            .is_empty()
    );
}

/// A large selection's highlight (docs/adr/0123) built on several cores is
/// the part one core builds: every kind of the sample, many times over, in
/// runs of any length, joined in their order.
#[test]
fn a_highlight_built_side_by_side_is_the_one_built_in_turn() {
    use kentos_render_wgpu::scene::{Highlight, build_highlight, build_highlight_parallel};

    let doc = sample();
    let objects: Vec<&Entity> = std::iter::repeat_n(&doc.entities, 300).flatten().collect();
    let style = Highlight {
        color: Rgba8::rgb(0x3b, 0x82, 0xf6),
        fill: Some(Rgba8([0x3b, 0x82, 0xf6, 0x21])),
        mark_size: 15.0,
        mark_shape: 1,
    };
    let origin = origin(&doc);
    let clip = kentos_render_wgpu::Bounds {
        min_x: origin.x - 5_000.0,
        min_y: origin.y - 5_000.0,
        max_x: origin.x + 5_000.0,
        max_y: origin.y + 5_000.0,
    };
    // And in a deep view round one of the sample's corners (plan/render.md RM-24).
    let deep = kentos_geometry_core::geom::deep::Deep {
        bounds: kentos_render_wgpu::Bounds {
            min_x: origin.x - 2.0,
            min_y: origin.y - 2.0,
            max_x: origin.x + 3.0,
            max_y: origin.y + 3.0,
        },
        tolerance: 1e-4,
    };
    for deep in [None, Some(&deep)] {
        let whole = build_highlight(
            objects.iter().copied(),
            &style,
            origin,
            0.05,
            3,
            &clip,
            deep,
        );
        assert!(!whole.segments.is_empty() && !whole.fills.is_empty() && !whole.markers.is_empty());
        for threads in [1, 2, 3, 7, 8, objects.len() + 5] {
            let side =
                build_highlight_parallel(&objects, &style, origin, 0.05, 3, &clip, deep, threads);
            assert_eq!(side.layers, whole.layers, "{threads} runs: the layers");
            assert_eq!(side.segments, whole.segments, "{threads} runs: the strokes");
            assert_eq!(side.fills, whole.fills, "{threads} runs: the fills");
            assert_eq!(side.markers, whole.markers, "{threads} runs: the marks");
        }
    }
}

/// A deep view's highlight (plan/render.md RM-24, docs/adr/9220): a long line
/// split at the view's box, a circle through the box round to the view's
/// tolerance in it, an area's fill ring split as its outline is. Without the
/// box, as before.
#[test]
fn a_deep_view_s_highlight_is_split_at_its_box_and_round_in_it() {
    use kentos_geometry_core::geom::deep::Deep;

    let objects: Vec<Entity> = serde_json::from_str(
        r#"[{"kind":"line","layerId":"0","a":{"x":-5000,"y":0.25},"b":{"x":5000,"y":0.25},"attrs":{},"id":1},
            {"kind":"circle","layerId":"0","c":{"x":-49.5,"y":0.3},"r":50,"attrs":{},"id":2},
            {"kind":"polygon","layerId":"0","pts":[{"x":-400,"y":-400},{"x":400,"y":-400},{"x":400,"y":0.5},{"x":-400,"y":0.5}],"attrs":{},"id":3}]"#,
    )
    .expect("objects");
    let style = scene::Highlight {
        color: Rgba8::rgb(0x4c, 0x9b, 0xe8),
        fill: Some(Rgba8([0x4c, 0x9b, 0xe8, 0x21])),
        mark_size: 15.0,
        mark_shape: 1,
    };
    let o = Vec2::new(0.0, 0.0);
    let clip = kentos_render_wgpu::Bounds {
        min_x: -1e6,
        min_y: -1e6,
        max_x: 1e6,
        max_y: 1e6,
    };
    let deep = Deep {
        bounds: kentos_render_wgpu::Bounds {
            min_x: -1.0,
            min_y: -1.0,
            max_x: 1.0,
            max_y: 1.0,
        },
        tolerance: 1e-4,
    };
    let plain = scene::build_highlight(objects.iter(), &style, o, 0.05, 0, &clip, None);
    let part = scene::build_highlight(objects.iter(), &style, o, 0.05, 0, &clip, Some(&deep));
    let inside = |p: Vec2| p.x.abs() <= 1.0 + 1e-9 && p.y.abs() <= 1.0 + 1e-9;
    let pieces = segments(&part);
    // Every stroke reaching into the box has both its ends in it.
    for (a, c) in &pieces {
        if inside(Vec2::new((a.x + c.x) / 2.0, (a.y + c.y) / 2.0)) {
            assert!(
                inside(*a) && inside(*c),
                "{a:?} → {c:?} reaches into the box from outside it"
            );
        }
    }
    // The line: one piece from edge to edge of the box.
    assert!(pieces.iter().any(|(a, c)| (a.y - 0.25).abs() < 1e-9
        && (a.x + 1.0).abs() < 1e-9
        && (c.x - 1.0).abs() < 1e-9));
    // The circle's chords in the box stray at most the tolerance.
    let centre = Vec2::new(-49.5, 0.3);
    let on_circle = |p: &Vec2| ((p.x - centre.x).hypot(p.y - centre.y) - 50.0).abs() < 1e-6;
    let chords: Vec<f64> = pieces
        .iter()
        .filter(|(a, c)| on_circle(a) && on_circle(c))
        .filter(|(a, c)| inside(Vec2::new((a.x + c.x) / 2.0, (a.y + c.y) / 2.0)))
        .map(|(a, c)| 50.0 - ((a.x + c.x) / 2.0 - centre.x).hypot((a.y + c.y) / 2.0 - centre.y))
        .collect();
    assert!(chords.len() > 5, "{} chords in the box", chords.len());
    assert!(chords.iter().all(|s| *s <= 1e-4 * 1.000_001), "{chords:?}");
    // The area's fill ring has the points where its top edge crosses the box.
    let fill_at = |part: &ScenePart, x: f64| {
        part.fills.iter().any(|f| {
            let p = join(f.hi, f.lo);
            (o.x + p[0] - x).abs() < 1e-9 && (o.y + p[1] - 0.5).abs() < 1e-9
        })
    };
    assert!(
        fill_at(&part, 1.0) && fill_at(&part, -1.0),
        "the fill's ring split at the box"
    );
    assert!(!fill_at(&plain, 1.0), "without the box as before");
    assert!(part.segments.len() > plain.segments.len());
}

/// docs/adr/9242: a ring's long sides cut into pieces meet straight on (1024), its corners at their turn (a left turn
/// a quarter round, 1536), the closing corner too: each draws its side of the joint, so a seen-through line is drawn
/// once there.
#[test]
fn a_rings_pieces_and_corners_carry_their_joins() {
    use kentos_render_wgpu::layout::join_code;
    let mut doc = sample();
    let Some(Entity::Polygon(mut square)) = doc
        .entities
        .iter()
        .find(|e| matches!(e, Entity::Polygon(_)))
        .cloned()
    else {
        panic!("a polygon");
    };
    let (x, y) = (486_000.0, 4_415_000.0);
    square.pts = [
        (x, y),
        (x + 1_000.0, y),
        (x + 1_000.0, y + 1_000.0),
        (x, y + 1_000.0),
    ]
    .map(|(x, y)| kentos_contracts::Vec2 { x, y })
    .to_vec();
    square.bulges = None;
    square.holes = None;
    doc.entities = vec![Entity::Polygon(square)];
    let fixed = scene::build_fixed(&doc, &palette(), origin(&doc));
    let per_side = (1_000.0 / MAX_SEGMENT_LENGTH).ceil() as usize;
    assert_eq!(fixed.segments.len(), 4 * per_side);
    let left = join_code([1.0, 0.0], [0.0, 1.0]);
    assert_eq!(
        (
            left,
            join_code([1.0, 0.0], [1.0, 0.0]),
            join_code([0.0, 1.0], [1.0, 0.0])
        ),
        (1536, 1024, 512)
    );
    assert_eq!(
        join_code([1.0, 0.0], [-1.0, 0.0]),
        0,
        "a turn back on itself has no join"
    );
    for (k, s) in fixed.segments.iter().enumerate() {
        let at = k % per_side;
        let want = (
            if at == 0 { left } else { 1024 },
            if at + 1 == per_side { left } else { 1024 },
        );
        assert_eq!((s.joins & 2047, (s.joins >> 11) & 2047), want, "piece {k}");
    }
}

/// docs/adr/9242: plain lines (an underlay's plan) that follow one another end to start meet at their corner, a run
/// returning to its start there too, past a line of no length; a line that meets nothing keeps its round ends (0).
#[test]
fn plain_lines_that_follow_one_another_meet() {
    let p = |x: f64, y: f64| Vec2::new(x, y);
    let codes = |lines: &[[Vec2; 2]]| -> Vec<(u32, u32)> {
        let part = scene::build_lines(p(0.0, 0.0), &[(lines, Rgba8::rgb(1, 2, 3))]);
        part.segments
            .iter()
            .map(|s| (s.joins & 2047, (s.joins >> 11) & 2047))
            .collect()
    };
    // Up a corner, then a line apart.
    let apart = [
        [p(0., 0.), p(10., 0.)],
        [p(10., 0.), p(10., 10.)],
        [p(20., 0.), p(30., 0.)],
    ];
    assert_eq!(codes(&apart), vec![(0, 1536), (1536, 0), (0, 0)]);
    // A square's four lines back to their start, one of no length in between: four corners.
    let square = [
        [p(0., 0.), p(10., 0.)],
        [p(10., 0.), p(10., 0.)],
        [p(10., 0.), p(10., 10.)],
        [p(10., 10.), p(0., 10.)],
        [p(0., 10.), p(0., 0.)],
    ];
    assert_eq!(codes(&square), vec![(1536, 1536); 4]);
    // Broken by a point that is no number: nothing meets across it.
    let broken = [
        [p(0., 0.), p(10., 0.)],
        [p(10., 0.), p(f64::NAN, 0.)],
        [p(f64::NAN, 0.), p(10., 10.)],
        [p(10., 10.), p(0., 10.)],
    ];
    assert_eq!(codes(&broken), vec![(0, 0), (0, 0)]);
}

/// A hatch pattern's dots (dashes of no length) are drawn as the shortest of lines: a round dot as wide as a line. The
/// plain scene dropped them, as `segment` drops a segment of no length (found with docs/adr/9242).
#[test]
fn a_hatch_patterns_dots_are_drawn() {
    let mut doc = sample();
    let Some(Entity::Hatch(mut hatch)) = doc
        .entities
        .iter()
        .find(|e| matches!(e, Entity::Hatch(_)))
        .cloned()
    else {
        panic!("a hatch");
    };
    hatch.pattern.kind = kentos_contracts::HatchPatternType::Pattern;
    hatch.pattern.scale = Some(1.0);
    hatch.pattern.lines = Some(vec![kentos_contracts::PatternLine {
        angle: 0.0,
        origin: [0.0, 0.0],
        offset: [0.0, 1.0],
        dashes: vec![0.0, -1.0],
    }]);
    doc.entities = vec![Entity::Hatch(hatch)];
    let fixed = scene::build_fixed(&doc, &palette(), origin(&doc));
    let dots = segments(&fixed).iter().filter(|(a, b)| a == b).count();
    // A dot a metre over 10 × 10 m less the 2 × 2 m hole: some hundred.
    assert!((80..=121).contains(&dots), "{dots} dots");
}

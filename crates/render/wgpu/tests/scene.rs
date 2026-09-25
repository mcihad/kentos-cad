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
fn showing_the_hidden_layer_draws_its_objects_and_counts_what_is_not_drawn_yet() {
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
    let undrawn: Vec<(&str, usize)> = fixed.not_drawn.iter().map(|(k, v)| (*k, *v)).collect();
    assert_eq!(
        undrawn,
        [("dimension", 1), ("ray", 1), ("text", 1), ("xline", 1)]
    );
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

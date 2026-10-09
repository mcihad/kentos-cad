//! A large layer's styled parts (docs/adr/0121): built in runs of places,
//! one edit builds its own part again and leaves the others as they are, a
//! part left empty goes, a layer that reads `$sıra` stays one run, and the
//! parts drawn in the scene's order draw what the layer built whole draws.

use std::collections::BTreeMap;
use std::sync::Arc;

use kentos_contracts::{
    AngleUnit, AreaUnit, DocumentSnapshotV1, Entity, EntityBase, LayerNode, LayerNodeType,
    LayerStyle, LineType, PathEntity, ProjectSettings, ProjectStyles, Vec2,
};
use kentos_domain::Slot;
use kentos_interaction::Spatial;
use kentos_native_style::StylePalette;
use kentos_native_style::batches::{DecodeOptions, decode};
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::program::{BuildOptions, build_layer};
use kentos_render_wgpu::Bounds;
use kentos_render_wgpu::scene;
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use serde_json::{Value, json};

use super::scene::{Look, PART_PLACES, PARTS_FROM, StyledCache, construction_clip};
use crate::document::Document;

fn layer(id: &str, color: &str, renderer: Option<Value>) -> LayerNode {
    LayerNode {
        id: id.into(),
        name: id.into(),
        kind: LayerNodeType::Layer,
        visible: true,
        locked: false,
        expanded: true,
        style: LayerStyle {
            color: color.into(),
            line_type: LineType::Continuous,
            line_weight: 0.35,
            fill: Some("#E06C7533".into()),
            point: None,
            label: None,
            pick_interior: None,
            renderer,
        },
        children: Vec::new(),
        snap: None,
        fields: Vec::new(),
        service: None,
        feed: None,
    }
}

/// `n` square parcels on `parsel`, then ten two-bend roads on `yol`, in that document order.
fn drawing(n: usize, renderer: Option<Value>) -> Document {
    let mut entities = Vec::with_capacity(n + 10);
    let base = |id: u32, layer: &str| EntityBase {
        id,
        layer_id: layer.into(),
        color: None,
        attrs: BTreeMap::from([("Nitelik".to_owned(), "Arsa".to_owned())]),
        label: None,
        symbol: None,
        line_weight: None,
    };
    for i in 0..n {
        let (x, y) = (
            486_000.0 + (i % 200) as f64 * 30.0,
            4_420_000.0 + (i / 200) as f64 * 30.0,
        );
        let pts = [(0.0, 0.0), (25.0, 0.0), (25.0, 25.0), (0.0, 25.0)]
            .iter()
            .map(|(dx, dy)| Vec2 {
                x: x + dx,
                y: y + dy,
            })
            .collect();
        entities.push(Entity::Polygon(PathEntity {
            base: base(i as u32 + 1, "parsel"),
            pts,
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
        }));
    }
    for i in 0..10 {
        let y = 4_420_000.0 + f64::from(i) * 100.0;
        entities.push(Entity::Polyline(PathEntity {
            base: base(n as u32 + i + 1, "yol"),
            pts: vec![
                Vec2 { x: 486_000.0, y },
                Vec2 {
                    x: 487_000.0,
                    y: y + 40.0,
                },
                Vec2 { x: 488_000.0, y },
            ],
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
        }));
    }
    let snapshot = DocumentSnapshotV1 {
        format: "kentos.document".into(),
        version: 1,
        name: "parcalar".into(),
        settings: ProjectSettings {
            srid: 5256,
            length_decimals: 3,
            area_decimals: 2,
            area_unit: AreaUnit::M2,
            angle_unit: AngleUnit::Grad,
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
        },
        origin: Vec2 {
            x: 486_000.0,
            y: 4_420_000.0,
        },
        home_view: None,
        // The top of the tree draws last: `parsel` over `yol`, which is the scene's first part.
        layers: vec![
            layer("parsel", "#E06C75", renderer),
            layer("yol", "#4E79A7", None),
        ],
        active_layer: "parsel".into(),
        entities,
        styles: ProjectStyles::default(),
        blocks: Vec::new(),
    };
    Document::new(snapshot, None).expect("opens")
}

const VIEW: Bounds = Bounds {
    min_x: 486_000.0,
    min_y: 4_420_000.0,
    max_x: 492_000.0,
    max_y: 4_422_000.0,
};

fn look(doc: &Document) -> Look {
    Look {
        palette: StylePalette::graphite(),
        symbol_scale: 1000.0,
        screen: false,
        hairlines: false,
        origin: scene::scene_origin(doc),
        view_build: Default::default(),
        view_colors: Default::default(),
    }
}

fn scene_of(
    cache: &mut StyledCache,
    doc: &Document,
    spatial: &mut Spatial,
    library: &StyleLibrary,
) -> StyledScene {
    spatial.sync(&doc.model);
    cache.scene(
        1,
        &doc.model,
        spatial.store(),
        library,
        &look(doc),
        &VIEW,
        1,
    )
}

/// The scene's parts of `parsel`: every part after the first (`yol`, one run, drawn beneath).
fn parcel_parts(scene: &StyledScene) -> &[Arc<StyledLayerPart>] {
    &scene.layers[1..]
}

#[test]
fn a_large_layer_is_built_in_runs_of_places_and_a_small_one_whole() {
    let library = kentos_native_style::system::library();
    let n = PARTS_FROM + 2000;
    let doc = drawing(n, None);
    let mut spatial = Spatial::of(&doc.model);
    let mut cache = StyledCache::default();
    let scene = scene_of(&mut cache, &doc, &mut spatial, &library);
    let runs = (n as u64).div_ceil(PART_PLACES) as usize;
    assert_eq!(scene.layers.len(), 1 + runs, "yol whole, parsel in runs");
    assert!(scene.order.is_some(), "the parts have their order");
    // A layer below the threshold draws its batches in turn: the first entries are yol's.
    let order = scene.order.as_ref().expect("order");
    let yol = scene.layers[0].layer.batches.len();
    assert!(
        order[..yol]
            .iter()
            .enumerate()
            .all(|(i, &(l, b))| l == 0 && b as usize == i)
    );
    assert_eq!(
        order.len(),
        scene
            .layers
            .iter()
            .map(|l| l.layer.batches.len())
            .sum::<usize>(),
        "every batch once"
    );
}

#[test]
fn one_edit_builds_its_part_again_and_the_others_stay() {
    let library = kentos_native_style::system::library();
    let n = PARTS_FROM + 2000;
    let mut doc = drawing(n, None);
    let mut spatial = Spatial::of(&doc.model);
    let mut cache = StyledCache::default();
    let before = scene_of(&mut cache, &doc, &mut spatial, &library);
    // A parcel of the second run moves by a metre.
    let slot = Slot(PART_PLACES as u32 + 100);
    let Some(Entity::Polygon(mut p)) = doc.model.get(slot).cloned() else {
        panic!("a parcel");
    };
    for v in &mut p.pts {
        v.x += 1.0;
    }
    assert!(doc.model.update(slot, Entity::Polygon(p)));
    let after = scene_of(&mut cache, &doc, &mut spatial, &library);
    assert_eq!(after.layers.len(), before.layers.len());
    assert!(
        Arc::ptr_eq(&after.layers[0], &before.layers[0]),
        "yol stays"
    );
    let (was, now) = (parcel_parts(&before), parcel_parts(&after));
    for (i, (a, b)) in was.iter().zip(now).enumerate() {
        assert_eq!(
            Arc::ptr_eq(a, b),
            i != 1,
            "part {i}: only the moved parcel's is built again"
        );
    }
    assert_eq!(
        cache.last_build.map(|(_, parts)| parts),
        Some(1),
        "one part built"
    );
}

#[test]
fn a_part_left_empty_goes() {
    let library = kentos_native_style::system::library();
    let n = 2 * PART_PLACES as usize + 100;
    let mut doc = drawing(n, None);
    let mut spatial = Spatial::of(&doc.model);
    let mut cache = StyledCache::default();
    let before = scene_of(&mut cache, &doc, &mut spatial, &library);
    assert_eq!(parcel_parts(&before).len(), 3);
    // The last run's hundred parcels are removed.
    let last: Vec<Slot> = (2 * PART_PLACES as u32 + 1..=n as u32).map(Slot).collect();
    assert_eq!(doc.model.remove(&last), 100);
    let after = scene_of(&mut cache, &doc, &mut spatial, &library);
    assert_eq!(parcel_parts(&after).len(), 2, "the empty part is gone");
    assert!(Arc::ptr_eq(
        &parcel_parts(&after)[0],
        &parcel_parts(&before)[0]
    ));
    assert!(Arc::ptr_eq(
        &parcel_parts(&after)[1],
        &parcel_parts(&before)[1]
    ));
}

#[test]
fn a_layer_that_reads_its_objects_places_stays_one_run() {
    let library = kentos_native_style::system::library();
    let renderer = json!({
        "type": "categorized",
        "expr": "$sıra % 2",
        "categories": [{
            "value": "1",
            "label": "Tek",
            "symbols": { "line": { "type": "line", "layers": [
                { "id": "t", "type": "simpleLine", "color": "fg", "width": 0.35 }
            ] } }
        }],
        "other": { "line": { "type": "line", "layers": [
            { "id": "o", "type": "simpleLine", "color": "fgDim", "width": 0.18 }
        ] } }
    });
    let doc = drawing(PARTS_FROM + 2000, Some(renderer));
    let mut spatial = Spatial::of(&doc.model);
    let mut cache = StyledCache::default();
    let scene = scene_of(&mut cache, &doc, &mut spatial, &library);
    assert_eq!(
        scene.layers.len(),
        2,
        "parsel is one run: $sıra counts the whole layer"
    );
    assert!(scene.order.is_none(), "no layer in parts");
}

#[test]
fn the_parts_draw_what_the_layer_built_whole_draws() {
    let library = kentos_native_style::system::library();
    let n = PARTS_FROM + 2000;
    let doc = drawing(n, None);
    let mut spatial = Spatial::of(&doc.model);
    let mut cache = StyledCache::default();
    let scene = scene_of(&mut cache, &doc, &mut spatial, &library);
    // The layer built whole, as the cache builds a part.
    let look = look(&doc);
    let names = |id: &str| id.to_owned();
    let opts = BuildOptions {
        origin: look.origin,
        plot_scale: look.symbol_scale,
        screen: look.screen,
        hairlines: look.hairlines,
        clip: Some(construction_clip(&VIEW)),
        library: &library,
        layer_name: &names,
        view: Default::default(),
    };
    let entities: Vec<&Entity> = doc.model.by_layer("parsel").collect();
    let node = doc.model.layers().get("parsel").expect("parsel").clone();
    let (_, batches) = build_layer(spatial.store(), &node.style, &entities, &opts).expect("built");
    let whole = decode(
        batches,
        &DecodeOptions {
            palette: &look.palette,
            plot_scale: look.symbol_scale,
            library: &library,
            view: Default::default(),
        },
    )
    .expect("decoded");
    // What the scene draws of parsel, a batch's parts one after another.
    let mut drawn: Vec<(u64, Vec<f32>)> = Vec::new();
    for &(l, b) in scene.order.as_ref().expect("order").iter() {
        if l == 0 {
            continue;
        }
        let part = &scene.layers[l as usize].layer;
        let batch = &part.batches[b as usize];
        let data = &part.data[batch.range.clone()];
        match drawn.last_mut() {
            Some((key, numbers)) if *key == batch.key => numbers.extend_from_slice(data),
            _ => drawn.push((batch.key, data.to_vec())),
        }
    }
    assert_eq!(
        drawn.len(),
        whole.batches.len(),
        "the whole layer's batches"
    );
    for (i, ((key, numbers), batch)) in drawn.iter().zip(&whole.batches).enumerate() {
        assert_eq!(*key, batch.key, "batch {i} in its place");
        let want = &whole.data[batch.range.clone()];
        assert!(
            numbers.len() == want.len()
                && numbers
                    .iter()
                    .zip(want)
                    .all(|(a, b)| a.to_bits() == b.to_bits()),
            "batch {i}: the same numbers in the same order"
        );
    }
}

#[test]
fn a_changed_definition_draws_its_inserts_again() {
    let library = kentos_native_style::system::library();
    let mut doc = drawing(10, None);
    let block: kentos_contracts::BlockDefinition = serde_json::from_value(json!({
        "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001", "name": "Rögar", "base": { "x": 0, "y": 0 },
        "entities": [{ "kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": { "x": -2, "y": 0 }, "b": { "x": 2, "y": 0 } }]
    }))
    .expect("a definition");
    doc.model.add_block(block.clone()).expect("defined");
    let insert: Entity = serde_json::from_value(json!({
        "kind": "insert", "id": 0, "layerId": "yol", "attrs": {}, "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001",
        "p": { "x": 486_500.0, "y": 4_420_500.0 }, "scale": 1.0, "rotation": 0.0
    }))
    .expect("an insert");
    doc.model.add(insert).expect("placed");
    let mut spatial = Spatial::of(&doc.model);
    let mut cache = StyledCache::default();
    let before = scene_of(&mut cache, &doc, &mut spatial, &library);
    // A new base point: the insert draws its line elsewhere.
    let moved = kentos_contracts::BlockDefinition {
        base: Vec2 { x: 5.0, y: 5.0 },
        ..block
    };
    assert!(doc.model.update_block(moved).expect("changed"));
    let after = scene_of(&mut cache, &doc, &mut spatial, &library);
    assert!(
        !Arc::ptr_eq(&after.layers[0], &before.layers[0]),
        "yol, which holds the insert, is built again"
    );
    assert!(
        Arc::ptr_eq(&after.layers[1], &before.layers[1]),
        "parsel, which does not, stays"
    );
}

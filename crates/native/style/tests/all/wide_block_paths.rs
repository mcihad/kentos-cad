//! A geometric width is solid ink both at the root and inside transformed/nested blocks.
use std::sync::Arc;

use kentos_contracts::{BlockDefinition, Entity, LayerStyle};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::store::Store;
use kentos_native_application::blocks::core_blocks;
use kentos_native_application::geometry::{edit_geometry, entity_of, shape};
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::program::{BuildOptions, ReadingUnit, build_layer};
use serde_json::json;

fn build(
    e: &Entity,
    blocks: &[Arc<BlockDefinition>],
    fills: bool,
    layer_fill: bool,
    plot_precision: bool,
) -> (String, Vec<f32>) {
    let mut style = json!({"color":"#336699","lineType":"continuous","lineWeight":0.25});
    if layer_fill {
        style["fill"] = json!("#00ff00");
    }
    let style: LayerStyle = serde_json::from_value(style).unwrap();
    let mut store = Store::new();
    store.set_blocks(core_blocks(blocks));
    store.put_many([(1.0, "a", false, shape(e))]);
    let library = StyleLibrary::default();
    let opts = BuildOptions {
        origin: Vec2::new(0.0, 0.0),
        plot_scale: 100.0,
        screen: false,
        hairlines: false,
        clip: None,
        library: &library,
        layer_name: &|_| "A".into(),
        view: kentos_native_style::View {
            fills,
            plot_precision,
            ..Default::default()
        },
        frame: None,
        reading_unit: ReadingUnit::METRE,
        layer_styles: None,
        line_types: None,
    };
    let (_, batches) = build_layer(&store, &style, &[e], &opts).unwrap();
    (batches.json, batches.data)
}

#[test]
fn curved_tapered_paths_keep_their_solid_ink_inside_nested_mirrored_scaled_blocks() {
    let path: Entity = serde_json::from_value(json!({
        "kind":"polyline","id":1,"layerId":"a","attrs":{},"color":"#884422",
        "transparency":0.35,"pts":[{"x":0,"y":0},{"x":2,"y":0},{"x":1.5,"y":1}],
        "bulges":[0.25,0,-0.1],"widths":[[0.1,0.2],[0.2,0.3],[0.3,0.1]],"closed":true
    }))
    .unwrap();
    let child: BlockDefinition = serde_json::from_value(json!({
        "id":"22222222-3333-4444-8555-666666666666","name":"Geniş çizgi",
        "base":{"x":0,"y":0},"entities":[path]
    }))
    .unwrap();
    for nested in [false, true] {
        for mirror in [false, true] {
            let mut blocks = vec![Arc::new(child.clone())];
            let mut block = child.id;
            if nested {
                let parent: BlockDefinition = serde_json::from_value(json!({
                    "id":"33333333-4444-4555-8666-777777777777","name":"İç içe",
                    "base":{"x":0,"y":0},"entities":[
                        {"kind":"insert","id":1,"layerId":"a","attrs":{},"block":block,
                         "p":{"x":1,"y":3},"rotation":0.4,"scale":1.5}
                    ]
                }))
                .unwrap();
                block = parent.id;
                blocks.push(Arc::new(parent));
            }
            let insert: Entity = serde_json::from_value(json!({
                "kind":"insert","id":1,"layerId":"a","attrs":{},"block":block,
                "p":{"x":10,"y":20},"rotation":0.7,"scale":2,"mirror":mirror
            }))
            .unwrap();
            let placed = core_blocks(&blocks).expand(&shape(&insert));
            assert_eq!(placed.len(), 1);
            let root = entity_of(
                &edit_geometry(placed[0].shape.clone()).unwrap(),
                path.base().clone(),
            );
            for (fills, layer_fill) in [(true, true), (true, false), (false, true), (false, false)]
            {
                for plot_precision in [false, true] {
                    let expected = build(&root, &[], fills, layer_fill, plot_precision);
                    let actual = build(&insert, &blocks, fills, layer_fill, plot_precision);
                    assert_eq!(
                        actual, expected,
                        "nested={nested}, mirror={mirror}, fills={fills}, layer_fill={layer_fill}, plot={plot_precision}"
                    );
                    // Preserve the root path's existing intrinsic ink policy even when area fills are hidden.
                    assert!(actual.0.contains("\"kind\":\"fill\""));
                }
            }
        }
    }
}

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
    build_pattern(e,blocks,fills,layer_fill,plot_precision,None,100.)
}
fn build_pattern(
    e: &Entity, blocks: &[Arc<BlockDefinition>], fills:bool, layer_fill:bool, plot_precision:bool,
    pattern:Option<&kentos_native_style::line_types::LineTypes>,scale:f64,
) -> (String,Vec<f32>) {
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
        plot_scale: scale,
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
        line_types: pattern,
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

fn patterns(scale:f64) -> kentos_native_style::line_types::LineTypes {
    let settings:kentos_contracts::ProjectSettings=serde_json::from_value(json!({
        "srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":scale,
        "lineTypes":[{"name":"P","unit":"paper","elements":[{"length":30.},{"length":-10.}]}]
    })).unwrap();
    kentos_native_style::line_types::LineTypes::of(&settings)
}
fn fill_area(data:&[f32])->f64 {
    data.chunks_exact(6).map(|t|{
        let v:Vec<f64>=t.iter().map(|x|*x as f64).collect();
        ((v[2]-v[0])*(v[5]-v[1])-(v[3]-v[1])*(v[4]-v[0])).abs()/2.
    }).sum()
}
#[test]
fn named_dashes_cut_the_actual_tapered_band_at_each_paper_scale() {
    let raw=json!({"kind":"polyline","id":1,"layerId":"a","attrs":{},"linePattern":"P",
        "pts":[{"x":0.,"y":0.},{"x":5.3,"y":0.}],"widths":[[1.,6.3],[0.,0.]]});
    let entity:Entity=serde_json::from_value(raw).unwrap();
    for scale in [1.,20.,100.] {
        let pattern=patterns(scale);
        for plot in [false,true] { for fills in [false,true] {
            let (json,data)=build_pattern(&entity,&[],fills,true,plot,Some(&pattern),scale);
            assert!(!json.contains("lineTypeLoss"),"{json}");
            assert!(!json.contains("\"kind\":\"stroke\""));
            // Independent integral of w(x)=1+x over each [n*period,n*period+on].
            let mut expected=0.; let mut x=0.; let period=40.*scale/1000.; let on=30.*scale/1000.;
            while x<5.3 {let end=(x+on).min(5.3);expected+=(end-x)+(end*end-x*x)/2.;x+=period;}
            assert!((fill_area(&data)-expected).abs()<1e-5,"scale={scale} plot={plot} fills={fills}: {} != {expected}",fill_area(&data));
        }}
    }
}
#[test]
fn named_patterns_match_root_geometry_in_transformed_nested_blocks() {
    let path:Entity=serde_json::from_value(json!({"kind":"polyline","id":1,"layerId":"a","attrs":{},"linePattern":"P",
      "pts":[{"x":0.,"y":0.},{"x":2.,"y":0.},{"x":2.,"y":3.}],"bulges":[0.2,0.,0.],"widths":[[0.1,0.2],[0.2,0.4],[0.,0.]]})).unwrap();
    let child:BlockDefinition=serde_json::from_value(json!({"id":"22222222-3333-4444-8555-666666666666","name":"Desen","base":{"x":0.,"y":0.},"entities":[path]})).unwrap();
    let parent:BlockDefinition=serde_json::from_value(json!({"id":"33333333-4444-4555-8666-777777777777","name":"İç içe","base":{"x":0.,"y":0.},"entities":[
      {"kind":"insert","id":1,"layerId":"a","attrs":{},"block":child.id,"p":{"x":1.,"y":3.},"rotation":0.4,"scale":1.5}]})).unwrap();
    let blocks=vec![Arc::new(child),Arc::new(parent.clone())];
    let pattern=patterns(100.);
    for mirror in [false,true] {let insert:Entity=serde_json::from_value(json!({"kind":"insert","id":1,"layerId":"a","attrs":{},"block":parent.id,
      "p":{"x":10.,"y":20.},"rotation":0.7,"scale":2.,"mirror":mirror})).unwrap();
      let placed=core_blocks(&blocks).expand(&shape(&insert));
      let root=entity_of(&edit_geometry(placed[0].shape.clone()).unwrap(),path.base().clone());
      for plot in [false,true] {assert_eq!(build_pattern(&insert,&blocks,true,true,plot,Some(&pattern),100.),build_pattern(&root,&[],true,true,plot,Some(&pattern),100.),"mirror={mirror} plot={plot}");}
    }
}

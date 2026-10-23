//! Physical block sizes and world coordinates from the independent Fraction fixtures (ADR 0589).
use kentos_contracts::{BlockDefinition, DrawingUnit, DxfReadOptions, Entity, Vec2};
use kentos_formats::dxf;
use serde_json::Value;

fn near(actual: Vec2, want: &Value, at: &str) {
    assert!(
        (actual.x - want[0].as_f64().unwrap()).abs() < 1e-9,
        "{at}: x {actual:?} != {want}"
    );
    assert!(
        (actual.y - want[1].as_f64().unwrap()).abs() < 1e-9,
        "{at}: y {actual:?} != {want}"
    );
}

fn placed(e: &Entity, blocks: &[BlockDefinition]) -> Vec<Vec2> {
    match e {
        Entity::Line(l) => vec![l.a, l.b],
        Entity::Insert(i) => {
            let b = blocks.iter().find(|b| b.id == i.block).unwrap();
            assert_eq!(i.rotation, 0.0);
            b.entities
                .iter()
                .flat_map(|e| placed(e, blocks))
                .map(|p| Vec2 {
                    x: i.p.x + (p.x - b.base.x) * i.scale,
                    y: i.p.y + (p.y - b.base.y) * i.y_scale.unwrap_or(i.scale),
                })
                .collect()
        }
        _ => panic!("line or insert"),
    }
}

#[test]
fn mixed_block_units_keep_physical_definitions_and_existing_nested_world_geometry() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/formats/v1/cad-sheet");
    let all: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).unwrap()).unwrap();
    let cases = all["units"].as_array().unwrap();
    assert!(cases.len() >= 20);
    for case in cases {
        let at = case["file"].as_str().unwrap();
        let bytes = std::fs::read(dir.join(at)).unwrap();
        let options = DxfReadOptions {
            unit: Some(DrawingUnit::M),
            file_unit: serde_json::from_value(case["fileUnit"].clone()).unwrap(),
            ..Default::default()
        };
        let read = dxf::read_checked(&bytes, &options).unwrap();
        let child = read.blocks.iter().find(|b| b.name == "CHILD").unwrap();
        near(child.base, &case["childBase"], at);
        let Entity::Line(line) = &child.entities[0] else {
            panic!("child line");
        };
        near(line.a, &case["childBase"], at);
        near(line.b, &case["childEnd"], at);
        for (entity, key) in read.entities.iter().zip(["direct", "nested"]) {
            let points = placed(entity, &read.blocks);
            assert_eq!(points.len(), 2, "{at}");
            near(points[0], &case[key][0], at);
            near(points[1], &case[key][1], at);
        }
        // The separate flattening path must agree with the same analytic coordinates.
        let flat = dxf::read_checked(
            &bytes,
            &DxfReadOptions {
                explode_blocks: true,
                ..options
            },
        )
        .unwrap();
        assert_eq!(flat.entities.len(), 2, "{at}");
        for (entity, key) in flat.entities.iter().zip(["direct", "nested"]) {
            let points = placed(entity, &flat.blocks);
            near(points[0], &case[key][0], at);
            near(points[1], &case[key][1], at);
        }
    }
}

#[test]
fn calibrated_block_units_keep_physical_definitions_and_existing_nested_world_geometry() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/formats/v1/cad-sheet");
    let all: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).unwrap()).unwrap();
    let cases = all["units"].as_array().unwrap();
    assert!(cases.len() >= 20);
    for case in cases {
        let at = case["file"].as_str().unwrap();
        let bytes = std::fs::read(dir.join(at)).unwrap();
        let options = DxfReadOptions {
            unit: Some(DrawingUnit::M),
            scale_reference:Some(kentos_contracts::CadImportScale{source_length:1.,real_length:match case["fileUnit"].as_str().or_else(||case["drawing"].as_str()).unwrap(){"mm"=>0.001,"cm"=>0.01,"m"=>1.,"in"=>0.0254,"ft"=>0.3048,_=>panic!("unit")}}),
            ..Default::default()
        };
        let read = dxf::read_checked(&bytes, &options).unwrap();
        let child = read.blocks.iter().find(|b| b.name == "CHILD").unwrap();
        near(child.base, &case["childBase"], at);
        let Entity::Line(line) = &child.entities[0] else {
            panic!("child line");
        };
        near(line.a, &case["childBase"], at);
        near(line.b, &case["childEnd"], at);
        for (entity, key) in read.entities.iter().zip(["direct", "nested"]) {
            let points = placed(entity, &read.blocks);
            assert_eq!(points.len(), 2, "{at}");
            near(points[0], &case[key][0], at);
            near(points[1], &case[key][1], at);
        }
        // The separate flattening path must agree with the same analytic coordinates.
        let flat = dxf::read_checked(
            &bytes,
            &DxfReadOptions {
                explode_blocks: true,
                ..options
            },
        )
        .unwrap();
        assert_eq!(flat.entities.len(), 2, "{at}");
        for (entity, key) in flat.entities.iter().zip(["direct", "nested"]) {
            let points = placed(entity, &flat.blocks);
            near(points[0], &case[key][0], at);
            near(points[1], &case[key][1], at);
        }
    }
}

#[test]
fn import_scale_bounds_use_the_arc_span_and_exclude_unused_definitions_and_paper(){
    let source=b"0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n6\n0\nENDSEC\n0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n2\nUNUSED\n10\n0\n20\n0\n0\nLINE\n8\n0\n10\n-1000000\n20\n-1000000\n11\n1000000\n21\n1000000\n0\nENDBLK\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nARC\n8\n0\n10\n0\n20\n0\n40\n2\n50\n0\n51\n90\n0\nLINE\n8\n0\n67\n1\n410\nLayout1\n10\n100000\n20\n100000\n11\n100001\n21\n100001\n0\nENDSEC\n0\nEOF\n";
    let read=dxf::read_checked(source,&DxfReadOptions{unit:Some(DrawingUnit::M),..Default::default()}).unwrap();
    let bounds=read.bounds.unwrap();assert!(bounds.min_x.abs()<1e-12 && bounds.min_y.abs()<1e-12);assert_eq!((bounds.max_x,bounds.max_y),(2.,2.));assert_eq!(read.view,read.bounds);
    let calibrated=dxf::read_checked(source,&DxfReadOptions{unit:Some(DrawingUnit::M),scale_reference:Some(kentos_contracts::CadImportScale{source_length:1.,real_length:0.01}),..Default::default()}).unwrap();
    let b=calibrated.bounds.unwrap();assert!(b.min_x.abs()<1e-12 && b.min_y.abs()<1e-12);assert_eq!((b.max_x,b.max_y),(0.02,0.02));
    assert_eq!(read.layouts.iter().map(|p|(p.width_mm,p.height_mm)).collect::<Vec<_>>(),calibrated.layouts.iter().map(|p|(p.width_mm,p.height_mm)).collect::<Vec<_>>());
}

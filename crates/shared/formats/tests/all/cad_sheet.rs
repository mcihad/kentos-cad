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

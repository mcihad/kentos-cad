//! Architecture data 2 (docs/adr/8070 §5): a block card's building element (its class, type and body) is written with
//! `modules.arch = 2` and read back as it was; a type its class does not have is refused when written and when read.

use kentos_contracts::{BlockDefinition, BlockElement, BlockElementClass};

fn sample() -> kentos_contracts::DocumentSnapshotV2 {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/arch/v1/section-building.kcad");
    kentos_kcad::decode(&std::fs::read(path).expect("the building")).expect("decodes")
}

/// A 1.20 × 0.80 m table block, its card saying it is furniture.
fn table(element: Option<BlockElement>) -> BlockDefinition {
    serde_json::from_value(serde_json::json!({
        "id": "0192f5b0-0000-7000-8000-0000000000aa",
        "name": "Masa",
        "base": {"x": 0.0, "y": 0.0},
        "entities": [{"kind": "polygon", "id": 1, "layerId": "", "attrs": {},
                      "pts": [{"x": 0.0, "y": 0.0}, {"x": 1.2, "y": 0.0}, {"x": 1.2, "y": 0.8}, {"x": 0.0, "y": 0.8}]}],
        "units": "m",
        "card": {"category": "Mobilya/Masalar", "version": 1, "element": element}
    }))
    .expect("a block")
}

#[test]
fn a_block_s_building_element_round_trips_under_arch_data_2() {
    let plain = sample();
    assert_eq!(kentos_kcad::arch_version(&plain), 0);
    let mut doc = plain.clone();
    doc.blocks.push(table(Some(BlockElement {
        class: BlockElementClass::Furniture,
        predefined: Some("TABLE".into()),
        height: Some(0.75),
        elevation: None,
    })));
    assert!(kentos_kcad::uses_elements(&doc));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_ELEMENTS);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the element read back");
    // A wall unit's underside and a user-defined type.
    let mut wall_unit = doc.clone();
    wall_unit.blocks[0].card.as_mut().unwrap().element = Some(BlockElement {
        class: BlockElementClass::Furniture,
        predefined: Some("USERDEFINED".into()),
        height: Some(0.7),
        elevation: Some(1.45),
    });
    let bytes = kentos_kcad::encode(&wall_unit).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), wall_unit);
    // A block without an element keeps the file as it was.
    let mut none = plain.clone();
    none.blocks.push(table(None));
    assert_eq!(kentos_kcad::arch_version(&none), 0);
}

#[test]
fn a_type_its_class_does_not_have_is_refused() {
    let mut doc = sample();
    doc.blocks.push(table(Some(BlockElement {
        class: BlockElementClass::Furniture,
        predefined: Some("WASHHANDBASIN".into()),
        height: Some(0.75),
        elevation: None,
    })));
    let refused = kentos_kcad::encode(&doc).expect_err("a basin is no furniture type");
    assert!(refused.message.contains("WASHHANDBASIN"), "{}", refused.message);
    let mut low = sample();
    low.blocks.push(table(Some(BlockElement {
        class: BlockElementClass::SanitaryTerminal,
        predefined: Some("WASHHANDBASIN".into()),
        height: Some(0.0),
        elevation: None,
    })));
    let refused = kentos_kcad::encode(&low).expect_err("no height");
    assert!(refused.message.contains("yüksekliği 0"), "{}", refused.message);
}

/// Architecture data 3 (docs/adr/8070 §7): a curtain wall's grid is written with `modules.arch = 3` and read back as it
/// was; a grid whose frame leaves no panel is refused when written.
#[test]
fn a_curtain_wall_round_trips_under_arch_data_3() {
    let mut doc = sample();
    let wall = doc
        .entities
        .iter_mut()
        .find_map(|e| match e {
            kentos_contracts::Entity::Wall(w) => Some(w),
            _ => None,
        })
        .expect("a wall");
    wall.wall.components.clear();
    wall.wall.style = None;
    wall.wall.curtain = Some(kentos_contracts::CurtainGrid {
        spacing: 1.5,
        rows: 1.2,
        frame: None,
        panel: Some("Cam".into()),
        frames: Some("Alüminyum".into()),
    });
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_CURTAIN_WALLS);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the grid read back");
    let mut bad = doc.clone();
    if let Some(kentos_contracts::Entity::Wall(w)) =
        bad.entities.iter_mut().find(|e| matches!(e, kentos_contracts::Entity::Wall(w) if w.wall.curtain.is_some()))
    {
        w.wall.curtain.as_mut().unwrap().frame = Some(0.8);
    }
    let refused = kentos_kcad::encode(&bad).expect_err("a frame wider than half a bay");
    assert!(refused.message.contains("çerçevesi"), "{}", refused.message);
}

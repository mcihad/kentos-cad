//! Architecture data 2–4 (docs/adr/8070 §5, §7, §8): a block card's building element (its class, type and body), a
//! curtain wall's grid, a post and a rail are written with their version (`modules.arch`) and read back as they were.
//! Architecture data 2: a block card's building element is written with
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
fn electrical_cards_require_their_new_readers() {
    let mut doc = sample();
    doc.blocks.push(table(Some(BlockElement {
        class: BlockElementClass::ElectricAppliance,
        predefined: Some("DISHWASHER".into()),
        height: None,
        elevation: None,
    })));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_ELECTRIC_APPLIANCES);
    assert_eq!(kentos_contracts::block_library_version(&doc.blocks), kentos_contracts::BLOCKS_APPLIANCE_VERSION);
    let bytes = kentos_kcad::encode(&doc).expect("writes the appliance");
    assert_eq!(kentos_kcad::decode(&bytes).expect("reads the appliance"), doc);
}

#[test]
fn kitchen_placement_requires_arch_39_and_preserves_legacy_files() {
    let bytes = include_bytes!("../../../../../fixtures/kcad/v2/casework.kcad");
    let original = kentos_kcad::decode(bytes).expect("legacy kitchen");
    let original_version = kentos_kcad::arch_version(&original);
    assert!(original_version < kentos_kcad::ARCH_WITH_CASEWORK_PLACEMENT);
    for with_floor in [true, false] {
        let mut doc = original.clone();
        let recipe = &mut doc.casework[0].recipe;
        if with_floor {
            recipe.level = Some("zemin".into());
        } else {
            recipe.filler_ends = vec![Some(kentos_contracts::CaseworkEnd::Start)];
        }
        assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_CASEWORK_PLACEMENT);
        let bytes = kentos_kcad::encode(&doc).expect("writes the new placement");
        assert_eq!(kentos_kcad::decode(&bytes).expect("reads the placement"), doc);
    }
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

/// Architecture data 4 (docs/adr/8070 §8): a post (a column of type POST, of its own height) and a rail (a beam,
/// USERDEFINED "Kayıt") are written with `modules.arch = 4` and read back as they were; a type IFC 4.3 has not, and a
/// user-defined member without its name, are refused when written.
#[test]
fn a_post_and_a_rail_round_trip_under_arch_data_4() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_members(&doc));
    let column = doc
        .entities
        .iter_mut()
        .find_map(|e| match e {
            kentos_contracts::Entity::Column(c) => Some(c),
            _ => None,
        })
        .expect("a column");
    column.column.member = Some(kentos_contracts::Member::post());
    column.column.height = Some(2.4);
    column.column.material = Some("Çelik".into());
    let beam = doc
        .entities
        .iter_mut()
        .find_map(|e| match e {
            kentos_contracts::Entity::Beam(b) => Some(b),
            _ => None,
        })
        .expect("a beam");
    beam.beam.member = Some(kentos_contracts::Member::rail());
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_MEMBERS);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the post and the rail read back");
    // A column of its own height alone needs version 4 too.
    let mut tall = sample();
    if let Some(kentos_contracts::Entity::Column(c)) =
        tall.entities.iter_mut().find(|e| matches!(e, kentos_contracts::Entity::Column(_)))
    {
        c.column.height = Some(4.5);
    }
    assert_eq!(kentos_kcad::arch_version(&tall), kentos_kcad::ARCH_WITH_MEMBERS);
    let mut bad = doc.clone();
    if let Some(kentos_contracts::Entity::Column(c)) =
        bad.entities.iter_mut().find(|e| matches!(e, kentos_contracts::Entity::Column(c) if c.column.member.is_some()))
    {
        c.column.member = Some(kentos_contracts::Member { predefined: "NOTDEFINED".into(), name: None });
    }
    let refused = kentos_kcad::encode(&bad).expect_err("NOTDEFINED is no type");
    assert!(refused.message.contains("NOTDEFINED"), "{}", refused.message);
    let mut nameless = doc.clone();
    if let Some(kentos_contracts::Entity::Beam(b)) =
        nameless.entities.iter_mut().find(|e| matches!(e, kentos_contracts::Entity::Beam(b) if b.beam.member.is_some()))
    {
        b.beam.member = Some(kentos_contracts::Member { predefined: "USERDEFINED".into(), name: None });
    }
    let refused = kentos_kcad::encode(&nameless).expect_err("a user-defined member says what it is");
    assert!(refused.message.contains("USERDEFINED"), "{}", refused.message);
}

/// Architecture data 22 (docs/adr/8130): a sloped member (a beam's `rise`) is written with `modules.arch = 22` and read
/// back as it was; a frame beam with a rise is refused when written.
#[test]
fn a_sloped_member_round_trips_under_arch_data_22() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_sloped_members(&doc));
    let beam = doc
        .entities
        .iter_mut()
        .find_map(|e| match e {
            kentos_contracts::Entity::Beam(b) => Some(b),
            _ => None,
        })
        .expect("a beam");
    beam.beam.pts.truncate(2);
    beam.beam.member = Some(kentos_contracts::Member::rail());
    beam.beam.rise = Some(1.2);
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_SLOPED_MEMBERS);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the sloped member reads back");
    let mut frame = doc.clone();
    if let Some(kentos_contracts::Entity::Beam(b)) =
        frame.entities.iter_mut().find(|e| matches!(e, kentos_contracts::Entity::Beam(b) if b.beam.rise.is_some()))
    {
        b.beam.member = None;
    }
    let refused = kentos_kcad::encode(&frame).expect_err("a frame beam is level");
    assert!(refused.message.contains("eğimli"), "{}", refused.message);
}

/// Architecture data 5 (docs/adr/8070 §10): a covering (a slab's `covering`) is written with `modules.arch = 5` and read
/// back as it was; one with layers is refused when written.
#[test]
fn a_covering_round_trips_under_arch_data_5() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_coverings(&doc));
    let slab = doc
        .entities
        .iter_mut()
        .find_map(|e| match e {
            kentos_contracts::Entity::Slab(s) => Some(s),
            _ => None,
        })
        .expect("a slab");
    slab.slab.layers = Default::default();
    slab.slab.edge = None;
    slab.slab.role = kentos_contracts::SlabRole::Floor;
    slab.slab.covering = Some(kentos_contracts::Covering::ceiling());
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_COVERINGS);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the covering read back");
    let mut layered = doc.clone();
    if let Some(kentos_contracts::Entity::Slab(s)) =
        layered.entities.iter_mut().find(|e| matches!(e, kentos_contracts::Entity::Slab(s) if s.slab.covering.is_some()))
    {
        s.slab.layers.over.push(kentos_contracts::SlabLayer {
            kind: kentos_contracts::SlabLayerKind::Finish,
            thickness: 0.02,
            material: None,
        });
    }
    let refused = kentos_kcad::encode(&layered).expect_err("a covering has no layers");
    assert!(refused.message.contains("Kaplamanın katmanları"), "{}", refused.message);
}

/// Architecture data 7 (docs/adr/8071 §6): a sloped slab (a slab's `slope`) is written with `modules.arch = 7` and read
/// back as it was; one too steep is refused when written, with its words.
#[test]
fn a_sloped_slab_round_trips_under_arch_data_7() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_slab_slopes(&doc));
    let slab = doc
        .entities
        .iter_mut()
        .find_map(|e| match e {
            kentos_contracts::Entity::Slab(s) => Some(s),
            _ => None,
        })
        .expect("a slab");
    for part in &mut slab.slab.parts {
        part.holes.clear();
    }
    let from = slab.slab.parts[0].outer.pts[0];
    slab.slab.slope = Some(kentos_contracts::SlabSlope {
        from,
        to: kentos_contracts::Vec2 { x: from.x, y: from.y + 4.0 },
        rise: 0.2,
    });
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_SLAB_SLOPES);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the slope read back");
    let mut steep = doc.clone();
    if let Some(kentos_contracts::Entity::Slab(s)) =
        steep.entities.iter_mut().find(|e| matches!(e, kentos_contracts::Entity::Slab(s) if s.slab.slope.is_some()))
        && let Some(sl) = s.slab.slope.as_mut()
    {
        sl.rise = 5.0;
    }
    let refused = kentos_kcad::encode(&steep).expect_err("steeper than 100 %");
    assert!(refused.message.contains("en çok %100"), "{}", refused.message);
}

/// Architecture data 8 (docs/adr/8071 §8): a roof's layers are written with `modules.arch = 8` and read back as they
/// were; layers out of their order, or a thickness given beside them, are refused when written, with their words.
#[test]
fn a_layered_roof_round_trips_under_arch_data_8() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_roof_layers(&doc));
    let id = doc.entities.iter().map(|e| e.base().id).max().unwrap_or(0) + 1;
    let roof: kentos_contracts::Entity = serde_json::from_value(serde_json::json!({
        "kind": "roof", "id": id, "layerId": "0", "attrs": {},
        "pts": [{"x": -0.625, "y": -0.625}, {"x": 12.625, "y": -0.625}, {"x": 12.625, "y": 9.625},
                {"x": -0.625, "y": 9.625}],
        "slopes": [33.0, 33.0, 33.0, 33.0], "arch": {"level": "k1"},
        "layers": [
            {"kind": "covering", "thickness": 0.03, "material": "Kiremit"},
            {"kind": "battens", "thickness": 0.03},
            {"kind": "rafters", "thickness": 0.125, "material": "Çam"}
        ]
    }))
    .expect("a roof");
    doc.entities.push(roof);
    let mut uid = [0x22u8; 16];
    uid[15] = 0x81;
    doc.uids.push(kentos_contracts::EntityId(uid));
    assert!(kentos_kcad::uses_roof_layers(&doc));
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_ROOF_LAYERS);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the layers read back");
    let edit = |doc: &kentos_contracts::DocumentSnapshotV2, change: &dyn Fn(&mut kentos_contracts::RoofFields)| {
        let mut doc = doc.clone();
        if let Some(kentos_contracts::Entity::Roof(r)) = doc.entities.last_mut() {
            change(&mut r.roof);
        }
        doc
    };
    let swapped = edit(&doc, &|r| r.layers.swap(0, 2));
    let refused = kentos_kcad::encode(&swapped).expect_err("out of their order");
    assert!(refused.message.contains("sırasıyla"), "{}", refused.message);
    let both = edit(&doc, &|r| r.thickness = Some(0.25));
    let refused = kentos_kcad::encode(&both).expect_err("a thickness beside its layers");
    assert!(refused.message.contains("katmanlarının toplamıdır"), "{}", refused.message);
}

/// Architecture data 9 (docs/adr/8072 §1): two buildings' codes, uses and ±0.00 and their shared basement's building
/// are written with `modules.arch = 9` and read back as they were; a code in a wrong form is refused when written.
#[test]
fn building_places_round_trip_under_arch_data_9() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_building_places(&doc));
    let arch = doc.settings.arch.as_mut().expect("its levels");
    let levels: Vec<String> = arch.levels.iter().map(|l| l.id.clone()).collect();
    arch.program.buildings = vec![
        kentos_contracts::ProgramBuilding {
            id: "a".into(),
            name: "A Blok".into(),
            levels: levels.clone(),
            code: Some("A1".into()),
            usage: Some("Konut".into()),
            zero: Some(0.0),
            ..Default::default()
        },
        kentos_contracts::ProgramBuilding {
            id: "b".into(),
            name: "B Blok".into(),
            levels: vec![levels[0].clone()],
            code: Some("B1".into()),
            zero: Some(1.2),
            ..Default::default()
        },
    ];
    arch.program.shared = vec![kentos_contracts::SharedLevel { level: levels[0].clone(), building: "a".into() }];
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_BUILDING_PLACES);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the buildings read back");
    let mut wrong = doc.clone();
    if let Some(a) = wrong.settings.arch.as_mut() {
        a.program.buildings[1].code = Some("b-1".into());
    }
    let refused = kentos_kcad::encode(&wrong).expect_err("a code in a wrong form");
    assert!(refused.message.contains("bina kodu"), "{}", refused.message);
}

/// Architecture data 40 (docs/adr/8172): a footing (a beam of a pad or strip footing role, at its base under the
/// storey's floor) is written with `modules.arch = 40` and read back as it was; one without its base is refused when
/// written.
#[test]
fn a_footing_round_trips_under_arch_data_40() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_footings(&doc));
    let beam = doc
        .entities
        .iter_mut()
        .find_map(|e| match e {
            kentos_contracts::Entity::Beam(b) => Some(b),
            _ => None,
        })
        .expect("a beam");
    beam.beam.role = kentos_contracts::BeamRole::StripFooting;
    beam.beam.width = 0.6;
    beam.beam.base_offset = Some(-0.8);
    beam.beam.offset = 0.0;
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_FOOTINGS);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the strip footing reads back");
    let mut pad = doc.clone();
    if let Some(kentos_contracts::Entity::Beam(b)) = pad.entities.iter_mut().find(|e| matches!(e, kentos_contracts::Entity::Beam(_))) {
        b.beam.role = kentos_contracts::BeamRole::PadFooting;
        b.beam.pts.truncate(2);
    }
    let bytes = kentos_kcad::encode(&pad).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), pad, "the pad footing reads back");
    let mut baseless = doc.clone();
    if let Some(kentos_contracts::Entity::Beam(b)) =
        baseless.entities.iter_mut().find(|e| matches!(e, kentos_contracts::Entity::Beam(_)))
    {
        b.beam.base_offset = None;
    }
    let refused = kentos_kcad::encode(&baseless).expect_err("a footing stands at its base");
    assert!(refused.message.contains("Temelin"), "{}", refused.message);
}

#[test]
fn a_raft_round_trips_under_arch_data_41() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_rafts(&doc));
    let slab = doc
        .entities
        .iter_mut()
        .find_map(|e| match e {
            kentos_contracts::Entity::Slab(s) if s.slab.covering.is_none() => Some(s),
            _ => None,
        })
        .expect("a slab");
    slab.slab.role = kentos_contracts::SlabRole::Raft;
    slab.slab.thickness = 0.6;
    slab.slab.offset = -0.3;
    slab.slab.edge = None;
    slab.slab.slope = None;
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_RAFTS);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the raft reads back");
    // A raft with a parapet is refused while written (docs/adr/8172 §6).
    let mut edged = doc.clone();
    if let Some(kentos_contracts::Entity::Slab(s)) =
        edged.entities.iter_mut().find(|e| matches!(e, kentos_contracts::Entity::Slab(s) if s.slab.role == kentos_contracts::SlabRole::Raft))
    {
        s.slab.edge = Some(kentos_contracts::SlabEdge {
            kind: kentos_contracts::SlabEdgeKind::Parapet,
            width: 0.2,
            runs: Vec::new(),
        });
    }
    let refused = kentos_kcad::encode(&edged).expect_err("a raft has no edge");
    assert!(refused.message.contains("Radyenin"), "{}", refused.message);
}

#[test]
fn a_profile_round_trips_under_arch_data_42() {
    let mut doc = sample();
    assert!(!kentos_kcad::uses_profiles(&doc));
    let beam = doc
        .entities
        .iter_mut()
        .find_map(|e| match e {
            kentos_contracts::Entity::Beam(b) => Some(b),
            _ => None,
        })
        .expect("a beam");
    // A coping (docs/adr/8173): a band on a wall's top, written as an IfcCovering.
    beam.beam.role = kentos_contracts::BeamRole::Beam;
    beam.beam.member = None;
    beam.beam.rise = None;
    beam.beam.offset = 0.0;
    beam.beam.base_offset = Some(1.0);
    beam.beam.width = 0.35;
    beam.beam.height = 0.05;
    beam.beam.covering = Some(kentos_contracts::Covering::coping());
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_PROFILES);
    let bytes = kentos_kcad::encode(&doc).expect("encodes");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the coping reads back");
    // A profile stands at its base: without one it is refused while written.
    let mut baseless = doc.clone();
    if let Some(kentos_contracts::Entity::Beam(b)) =
        baseless.entities.iter_mut().find(|e| matches!(e, kentos_contracts::Entity::Beam(_)))
    {
        b.beam.base_offset = None;
    }
    let refused = kentos_kcad::encode(&baseless).expect_err("a profile stands at its base");
    assert!(refused.message.contains("Profilin"), "{}", refused.message);
}

#[test]
fn a_site_s_ground_round_trips_under_arch_data_43() {
    use kentos_contracts::{CadWork, SiteGround};
    let mut doc = sample();
    assert!(!kentos_kcad::uses_ground(&doc));
    // An architecture project's (docs/adr/8174 §3.1): its road settings would be refused.
    assert_eq!(doc.settings.cad_work, Some(CadWork::Architecture));
    assert!(doc.settings.road.is_none());
    // The site's natural and graded ground: each its survey's layers, the site's own.
    let arch = doc.settings.arch.get_or_insert_with(Default::default);
    let site = arch.site.get_or_insert_with(Default::default);
    site.datum = Some(1285.4);
    site.natural_ground = Some(SiteGround::of_layer("arazi"));
    site.graded_ground = Some(SiteGround { layers: vec!["tesviye".into(), "bahce".into()] });
    assert_eq!(kentos_kcad::arch_version(&doc), kentos_kcad::ARCH_WITH_GROUND);
    let bytes = kentos_kcad::encode(&doc).expect("an architecture project writes its grounds");
    assert_eq!(kentos_kcad::decode(&bytes).expect("decodes"), doc, "the grounds read back");
    let refused = |change: &dyn Fn(&mut kentos_contracts::SitePlan)| {
        let mut wrong = doc.clone();
        change(wrong.settings.arch.as_mut().and_then(|a| a.site.as_mut()).unwrap());
        kentos_kcad::encode(&wrong).expect_err("refused").message
    };
    // The same layers cannot be both: the earthwork is their difference.
    let same = refused(&|s| s.graded_ground = Some(SiteGround { layers: vec!["arazi".into()] }));
    assert!(same.contains("aynı katmanlardan"), "{same}");
    assert!(refused(&|s| s.natural_ground = Some(SiteGround::default())).contains("en az bir"));
    assert!(refused(&|s| s.natural_ground = Some(SiteGround { layers: vec![" ".into()] })).contains("boş olamaz"));
    let twice = refused(&|s| s.natural_ground = Some(SiteGround { layers: vec!["arazi".into(), "arazi".into()] }));
    assert!(twice.contains("birer kez"), "{twice}");
}

//! Blocks through the session (docs/adr/0144): Patlat opens an insert into
//! its definition's objects, one level, each on its own layer when the
//! drawing has it (else the insert's), a nested insert staying an insert
//! with its similarity composed. The drawing is the traces' `blocks.kcad`
//! (fixtures/interaction/README.md). Expected values are worked out by hand.

use crate::common;

use common::Bench;
use kentos_commands::Entity;
use kentos_domain::Slot;

const BLOCKS: &str = include_str!("../../../../../fixtures/interaction/v1/blocks.kcad");
const LAMBA: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1003";

fn bench(selected: &[u32]) -> Bench {
    let mut b = Bench::on(BLOCKS);
    b.draft.snap = false;
    b.selection.set(selected.iter().map(|s| Slot(*s)));
    b
}

#[test]
fn explode_opens_an_insert_one_level() {
    // The pole at (487014, 4420014.8), turned −90°: its foundation, its arm and its lamp (a nested insert).
    let mut b = bench(&[7]);
    b.start("explode");
    assert_eq!(b.last_text(), Some("1 nesne patlatıldı: 3 parça."));
    assert!(b.doc.get(Slot(7)).is_none());
    assert_eq!(b.selected(), [13, 14, 15]);
    // The definition's objects are on layer 0, which the drawing lacks: the insert's layer.
    for slot in [13, 14, 15] {
        let e = b.doc.get(Slot(slot)).expect("a piece");
        assert_eq!(e.base().layer_id, "aydinlatma", "piece {slot}");
        assert_eq!(
            e.base().color,
            None,
            "neither the object nor the insert has a colour"
        );
    }
    let Some(Entity::Polygon(foundation)) = b.doc.get(Slot(13)) else {
        panic!("the foundation: {:?}", b.doc.get(Slot(13)));
    };
    // A quarter turn is exact: (−0.3, −0.3) goes to (−0.3, 0.3) from the insertion point.
    assert_eq!(
        (foundation.pts[0].x, foundation.pts[0].y),
        (487013.7, 4420015.1)
    );
    let Some(Entity::Line(arm)) = b.doc.get(Slot(14)) else {
        panic!("the arm");
    };
    assert_eq!((arm.a.x, arm.a.y), (487014.0, 4420014.5));
    assert_eq!((arm.b.x, arm.b.y), (487014.0, 4420012.35));
    // The lamp stays an insert of Lamba, placed at the arm's end and turned with the pole.
    let Some(Entity::Insert(lamp)) = b.doc.get(Slot(15)) else {
        panic!("the lamp: {:?}", b.doc.get(Slot(15)));
    };
    assert_eq!(lamp.block.to_text(), LAMBA);
    assert_eq!((lamp.p.x, lamp.p.y), (487014.0, 4420012.0));
    assert_eq!(lamp.scale, 1.0);
    assert!((lamp.rotation - 3.0 * std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert_eq!(b.doc.undo().as_deref(), Some("Patlat"));
    assert_eq!(b.doc.len(), 12);
}

#[test]
fn a_nested_insert_explodes_in_turn() {
    let mut b = bench(&[7]);
    b.start("explode");
    b.selection.set([Slot(15)]);
    b.start("explode");
    // Lamba: a circle and two crossing lines.
    assert_eq!(b.last_text(), Some("1 nesne patlatıldı: 3 parça."));
    let Some(Entity::Circle(c)) = b.doc.get(Slot(16)) else {
        panic!("the lamp's circle");
    };
    assert_eq!((c.c.x, c.c.y, c.r), (487014.0, 4420012.0, 0.35));
    assert_eq!(b.doc.undo().as_deref(), Some("Patlat"));
    assert!(matches!(b.doc.get(Slot(15)), Some(Entity::Insert(_))));
}

/// Label records index the varied pieces, whereas hiding a piece changes the attribute's position in that
/// list. The same tag must remain readable before and after a visibility edit and its undo.
#[test]
fn visibility_changes_keep_attribute_labels_and_their_moved_positions() {
    use kentos_geometry_core::store::placing::PlaceOptions;
    use kentos_interaction::Vec2;
    use kentos_interaction::spatial::{LabelSize, LabelSpot, shown_text};
    use serde_json::{Value, json};

    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/dynamic-blocks.json"
    ))
    .expect("the sample");
    raw["version"] = json!(1);
    let block = &mut raw["blocks"][0];
    let visibility = block["dynamic"]["parameters"]
        .as_array_mut()
        .expect("parameters")
        .iter_mut()
        .find(|p| p["kind"] == "visibility")
        .expect("visibility");
    visibility["states"]
        .as_array_mut()
        .expect("states")
        .reverse();
    block["attributes"] = json!([{
        "tag":"NO", "value":"DEFAULT", "p":{"x":0.05,"y":0.15},
        "height":0.08, "rotation":0.0, "movable":true
    }]);
    raw["entities"][0]["attrs"] = json!({"NO":"K-1"});
    raw["entities"][0]["attributePlaces"] = json!({"NO":{"x":0.25,"y":0.35}});
    raw["entities"][1]["attrs"] = json!({"NO":"K-2"});
    raw["entities"][2]["attrs"] = json!({"NO":"K-3"});
    raw["entities"][2]["parameters"] = json!({"Görünüş":"Açılım"});
    let mut b = Bench::on(&raw.to_string());
    let labels = |b: &mut Bench| {
        b.spatial.sync(&b.doc);
        let mut found: Vec<_> = b
            .spatial
            .labels_shown(
                Vec2::new(-10.0, -10.0),
                Vec2::new(20.0, 20.0),
                100.0,
                LabelSize::Print,
                PlaceOptions {
                    unplaced: true,
                    hidden: true,
                },
                false,
            )
            .into_iter()
            .filter_map(|(label, _)| {
                let LabelSpot::PieceText {
                    slot,
                    at,
                    text,
                    attribute,
                    ..
                } = label
                else {
                    return None;
                };
                let entity = b.doc.get(slot).expect("insert");
                Some((
                    slot.0,
                    shown_text(&text, attribute.as_deref(), &entity.base().attrs).to_owned(),
                    at,
                ))
            })
            .collect();
        found.sort_by_key(|v| v.0);
        found
    };
    let before = labels(&mut b);
    assert_eq!(
        before.iter().map(|v| v.1.as_str()).collect::<Vec<_>>(),
        ["K-1", "K-2", "K-3"]
    );
    assert!((before[0].2.x - 0.25).abs() < 1e-12);
    assert!((before[0].2.y - 0.35).abs() < 1e-12);
    let mut entity = b.doc.get(Slot(1)).expect("first").clone();
    let Entity::Insert(insert) = &mut entity else {
        panic!("insert");
    };
    insert.parameters.insert(
        "Görünüş".into(),
        kentos_commands::ParamValue::Text("Açılım".into()),
    );
    assert!(b.doc.update(Slot(1), entity));
    assert_eq!(labels(&mut b), before);
    assert!(b.doc.undo().is_some());
    assert_eq!(labels(&mut b), before);
}

#[test]
fn block_highlight_does_not_reveal_hidden_layer_pieces_and_updates_when_the_layer_is_shown() {
    use serde_json::{Value, json};
    let fixture: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw = fixture["setup"].clone();
    let mut hidden = raw["layers"][0].clone();
    hidden["id"] = json!("hidden"); hidden["name"] = json!("Hidden"); hidden["visible"] = json!(false);
    raw["layers"].as_array_mut().unwrap().push(hidden);
    let block = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4001";
    raw["blocks"] = json!([{"id":block,"name":"Two layers","base":{"x":0.,"y":0.},"entities":[
       {"kind":"line","id":1,"layerId":"","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":2.,"y":0.}},
       {"kind":"line","id":2,"layerId":"hidden","attrs":{},"a":{"x":-1000.,"y":0.},"b":{"x":1000.,"y":0.}}]}]);
    raw["entities"] = json!([{"kind":"insert","id":1,"layerId":"a","attrs":{},"block":block,"p":{"x":0.,"y":0.},"scale":1.,"rotation":0.}]);
    let mut b = Bench::on(&raw.to_string());
    let pieces = b.spatial.pieces(b.doc.get(Slot(1)).unwrap()).unwrap();
    assert_eq!(pieces.len(),1);
    let Entity::Line(line) = &pieces[0] else { panic!("line") };
    assert_eq!(line.b.x,2.);
    b.doc.set_layer_visible("hidden",true);
    b.spatial.sync(&b.doc);
    assert_eq!(b.spatial.pieces(b.doc.get(Slot(1)).unwrap()).unwrap().len(),2);
    b.doc.set_layer_visible("hidden",false); b.spatial.sync(&b.doc);
    assert_eq!(b.spatial.pieces(b.doc.get(Slot(1)).unwrap()).unwrap().len(),1);
}

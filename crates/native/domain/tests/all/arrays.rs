//! The drawing's associative arrays (docs/adr/0300): what follows their items
//! at a step's end, and another editor's arrays taken as the drawing's blocks
//! and objects hold them (§5).

use kentos_domain::contracts::{
    ArrayId, ArrayItem, ArrayLayout, AssocArray, BlockDefinition, BlockId, DocumentSnapshotV1,
    Entity, EntityBase, EntityId, InsertEntity, PointEntity, Vec2,
};
use kentos_domain::{Document, External, ExternalMeta, Slot, Uuid};

fn base(layer: &str) -> EntityBase {
    EntityBase {
        look: Default::default(),
        id: 0,
        layer_id: layer.into(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    }
}

fn empty() -> Document {
    let text = r#"{"format":"kentos.document","version":1,"name":"Deneme","settings":{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},"origin":{"x":0,"y":0},
        "layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}}"#;
    Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads")).expect("opens")
}

const BLOCK: BlockId = BlockId([7; 16]);

fn insert(x: f64) -> Entity {
    Entity::Insert(InsertEntity {
        arch: Default::default(),
        clipping: None,
        parameters: Default::default(),
        attribute_places: Default::default(),
        base: base("a"),
        block: BLOCK,
        p: Vec2 { x, y: 0.0 },
        scale: 1.0,
        y_scale: None,
        rotation: 0.0,
        mirror: false,
    })
}

/// A drawing with a block and its three inserts at x = 0, 2, 4 made one grid array, in one step.
fn arrayed() -> (Document, Vec<Slot>, AssocArray) {
    let mut doc = empty();
    let block = BlockDefinition {
        dynamic: None,
        xref: None,
        card: None,
        id: BLOCK,
        name: "Dizi 1".into(),
        base: Vec2 { x: 0.0, y: 0.0 },
        entities: vec![Entity::Point(PointEntity {
            base: base("a"),
            p: Vec2 { x: 0.0, y: 0.0 },
            z: None,
            parts: None,
        })],
        attributes: Vec::new(),
        description: None,
        units: None,
    };
    let (slots, array) = doc
        .transact("İlişkili dizi", |doc| {
            doc.add_block(block).map_err(|r| r.to_string())?;
            let slots = doc
                .add_many(vec![insert(0.0), insert(2.0), insert(4.0)], "ekle")
                .map_err(|e| e.to_string())?;
            let array = AssocArray {
                id: ArrayId([9; 16]),
                block: BLOCK,
                layer_id: "a".into(),
                p: Vec2 { x: 0.0, y: 0.0 },
                scale: 1.0,
                rotation: 0.0,
                mirror: false,
                layout: ArrayLayout::Grid {
                    rows: 1,
                    cols: 3,
                    dx: 2.0,
                    dy: 0.0,
                },
                items: slots
                    .iter()
                    .map(|s| ArrayItem {
                        uid: Some(EntityId(doc.uid(*s).unwrap().into_bytes())),
                        kept: false,
                    })
                    .collect(),
            };
            doc.add_array(array.clone()).map_err(|r| r.to_string())?;
            Ok::<_, String>((slots, array))
        })
        .unwrap();
    (doc, slots, array)
}

#[test]
fn an_items_insert_removed_or_changed_by_hand_is_followed_in_the_same_step() {
    let (mut doc, slots, array) = arrayed();
    assert_eq!(doc.arrays().len(), 1);
    assert_eq!(
        doc.array_of_item(doc.uid(slots[1]).unwrap())
            .map(|(_, k)| k),
        Some(1)
    );

    doc.remove(&[slots[1]]);
    assert_eq!(doc.array(array.id).unwrap().items[1], ArrayItem::default());
    doc.undo();
    assert_eq!(
        doc.array(array.id).unwrap().items[1],
        array.items[1],
        "one step with the object"
    );

    assert!(doc.update(slots[2], insert(5.0)));
    let item = &doc.array(array.id).unwrap().items[2];
    assert!(item.kept && item.uid == array.items[2].uid);
    doc.undo();
    assert!(!doc.array(array.id).unwrap().items[2].kept);
}

#[test]
fn another_editors_arrays_are_taken_as_the_drawing_holds_them() {
    let (mut doc, _, array) = arrayed();
    let mut theirs = array.clone();
    theirs.items[2].uid = Some(EntityId(*Uuid::now_v7().as_bytes()));
    theirs.items[0].kept = true;
    let mut no_block = array.clone();
    no_block.id = ArrayId([8; 16]);
    no_block.block = BlockId([6; 16]);
    doc.apply_external(External {
        meta: Some(ExternalMeta {
            arrays: Some(vec![theirs.clone(), no_block]),
            ..ExternalMeta::default()
        }),
        ..External::default()
    })
    .unwrap();
    let now = doc.arrays();
    assert_eq!(
        now.len(),
        1,
        "the array whose block the drawing lacks is left out"
    );
    assert!(now[0].items[0].kept);
    assert_eq!(
        now[0].items[2],
        ArrayItem::default(),
        "an item whose insert is not here is removed"
    );
    // The step that made the array would put back one they replaced: it is dropped.
    assert!(!doc.can_undo());

    // None at all: the drawing has no arrays any more, its inserts stay.
    let len = doc.len();
    doc.apply_external(External {
        meta: Some(ExternalMeta {
            arrays: Some(Vec::new()),
            ..ExternalMeta::default()
        }),
        ..External::default()
    })
    .unwrap();
    assert!(doc.arrays().is_empty());
    assert_eq!(doc.len(), len);
}

fn path_entity(points: &[(f64, f64)], closed: bool, bulges: Option<Vec<f64>>) -> Entity {
    serde_json::from_value(serde_json::json!({
        "kind":"polyline", "id":0, "layerId":"a", "attrs":{},
        "pts":points.iter().map(|(x,y)| serde_json::json!({"x":x,"y":y})).collect::<Vec<_>>(),
        "closed":closed, "bulges":bulges
    }))
    .expect("path")
}

#[test]
fn a_closed_square_places_eight_items_on_all_four_sides_without_repeating_the_start() {
    let (_, _, mut array) = arrayed();
    array.layout = ArrayLayout::Path {
        path: Uuid::now_v7().to_string(),
        count: 8,
        spacing: None,
        align: false,
    };
    let path = path_entity(&[(0., 0.), (4., 0.), (4., 4.), (0., 4.)], true, None);
    let places = kentos_domain::arrays::placements(&array, Some(&path)).expect("closed path");
    let actual: Vec<_> = places.iter().map(|p| (p.p.x, p.p.y)).collect();
    assert_eq!(
        actual,
        [
            (0., 0.),
            (2., 0.),
            (4., 0.),
            (4., 2.),
            (4., 4.),
            (2., 4.),
            (0., 4.),
            (0., 2.)
        ]
    );
}

#[test]
fn closing_semicircle_station_and_tangent_are_analytic_in_both_windings() {
    let (_, _, mut array) = arrayed();
    array.layout = ArrayLayout::Path {
        path: Uuid::now_v7().to_string(),
        count: 3,
        spacing: Some(4.),
        align: true,
    };
    for winding in [1., -1.] {
        let path = path_entity(&[(0., 0.), (4., 0.)], true, Some(vec![0., winding]));
        let places = kentos_domain::arrays::placements(&array, Some(&path))
            .expect("4 + 2π metre closed path");
        let last = places[2];
        assert!((last.p.x - (2. + 2. * 2.0_f64.cos())).abs() < 1e-12);
        assert!((last.p.y - winding * 2. * 2.0_f64.sin()).abs() < 1e-12);
        let angle = winding * (2. + std::f64::consts::FRAC_PI_2);
        assert!((last.rotation.cos() - angle.cos()).abs() < 1e-12);
        assert!((last.rotation.sin() - angle.sin()).abs() < 1e-12);
    }
}

#[test]
fn closing_a_path_again_repositions_live_items_in_the_same_step_with_stable_ids_and_undo() {
    let (mut doc, slots, mut array) = arrayed();
    let square = |closed| path_entity(&[(0., 0.), (4., 0.), (4., 4.), (0., 4.)], closed, None);
    let path_slot = doc.add_many(vec![square(true)], "Yol").unwrap()[0];
    array.layout = ArrayLayout::Path {
        path: doc.uid(path_slot).unwrap().to_string(),
        count: 3,
        spacing: None,
        align: false,
    };
    let closed = [(0., 0.), (4., 4. / 3.), (4. / 3., 4.)];
    doc.transact("Yola bağla", |doc| {
        for (slot, (x, y)) in slots.iter().zip(closed) {
            let Entity::Insert(mut e) = insert(x) else {
                unreachable!()
            };
            e.p.y = y;
            doc.update(*slot, Entity::Insert(e));
        }
        doc.update_array(array.clone()).unwrap();
        Ok::<_, String>(())
    })
    .unwrap();
    let ids: Vec<_> = slots.iter().map(|s| doc.uid(*s)).collect();
    let before = doc.to_snapshot_v2();
    doc.update(path_slot, square(false));
    let opened = doc.to_snapshot_v2();
    for (slot, (x, y)) in slots.iter().zip([(0., 0.), (4., 2.), (0., 4.)]) {
        let Some(Entity::Insert(e)) = doc.get(*slot) else {
            panic!("insert")
        };
        assert!((e.p.x - x).abs() < 1e-12 && (e.p.y - y).abs() < 1e-12);
    }
    doc.update(path_slot, square(true));
    for (slot, (x, y)) in slots.iter().zip(closed) {
        let Some(Entity::Insert(e)) = doc.get(*slot) else {
            panic!("insert")
        };
        assert!((e.p.x - x).abs() < 1e-12 && (e.p.y - y).abs() < 1e-12);
    }
    assert_eq!(slots.iter().map(|s| doc.uid(*s)).collect::<Vec<_>>(), ids);
    assert!(doc.array(array.id).unwrap().items.iter().all(|i| !i.kept));
    let after = doc.to_snapshot_v2();
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), opened);
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
    doc.redo();
    doc.redo();
    assert_eq!(doc.to_snapshot_v2(), after);
}

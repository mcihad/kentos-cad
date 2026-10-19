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

use crate::common::Bench;
use kentos_commands::Entity;
use serde_json::json;

fn setup() -> Bench {
    let mut b = Bench::new("spaceDiscover");
    let layer = b.doc.layers().active().to_owned();
    let walls=[
        json!({"kind":"wall","id":0,"layerId":layer,"attrs":{},"width":0.2,"closed":true,"pts":[{"x":0.,"y":0.},{"x":10.,"y":0.},{"x":10.,"y":8.},{"x":0.,"y":8.}]}),
        json!({"kind":"wall","id":0,"layerId":layer,"attrs":{},"width":0.2,"pts":[{"x":5.,"y":0.},{"x":5.,"y":8.}]})
    ].map(|v|{let mut e:Entity=serde_json::from_value(v).unwrap();if let Entity::Wall(w)=&mut e {for p in &mut w.wall.pts{p.x+=crate::common::E;p.y+=crate::common::N;}}e});
    b.doc.add_many(walls.to_vec(), "Walls").unwrap();
    b.start("spaceDiscover");
    b
}

#[test]
fn bulk_preview_is_read_only_and_one_confirm_creates_two_without_repeat() {
    let mut b = setup();
    let before = b.doc.to_snapshot_v2();
    assert!(
        b.session
            .settings(&Default::default())
            .iter()
            .any(|r| r.key == "S" && r.value == "2" && !r.editable)
    );
    assert!(
        !b.session
            .preview(&Default::default())
            .unwrap()
            .strokes
            .is_empty()
    );
    b.move_to(3., 2.);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.run(|s, cx| s.confirm(cx));
    assert_eq!(
        b.doc
            .entities()
            .filter(|e| matches!(e, Entity::Space(_)))
            .count(),
        2
    );
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
    b.doc.redo().unwrap();
    let made = b.doc.to_snapshot_v2();
    b.start("spaceDiscover");
    assert!(
        b.session
            .settings(&Default::default())
            .iter()
            .any(|r| r.key == "V" && r.value == "2" && !r.editable)
    );
    b.run(|s, cx| s.confirm(cx));
    assert_eq!(b.doc.to_snapshot_v2().entities, made.entities);
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}

#[test]
fn region_requires_two_picks_then_enter_and_escape_leaves_sources_intact() {
    let mut b = setup();
    let before = b.doc.to_snapshot_v2();
    assert!(b.run(|s, cx| s.configure("K", "region", cx)));
    let before_region = b.doc.to_snapshot_v2();
    b.click(5.5, -1.);
    b.move_to(11., 9.);
    assert_eq!(b.doc.to_snapshot_v2(), before_region);
    b.run(|s, cx| s.confirm(cx));
    assert_eq!(b.doc.to_snapshot_v2(), before_region);
    b.click(11., 9.);
    assert!(
        b.session
            .settings(&Default::default())
            .iter()
            .any(|r| r.key == "S" && r.value == "1" && !r.editable)
    );
    assert_eq!(b.doc.to_snapshot_v2(), before_region);
    b.session.exit();
    assert_eq!(b.doc.to_snapshot_v2(), before_region);
    assert_eq!(before.entities.len(), 2);
}

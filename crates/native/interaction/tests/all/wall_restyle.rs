use crate::common::{Bench, E, N};
use kentos_commands::Entity;
use serde_json::json;

#[test]
fn restyle_reviews_scope_before_writing_and_rejects_stale_confirmation() {
    let mut b = Bench::new("wallRestyle");
    let walls = [0., 5.].map(|dy| {
        serde_json::from_value(json!({
            "kind": "wall", "id": 0, "layerId": b.doc.layers().active(), "attrs": {},
            "width": 0.30, "justify": "center", "style": "kentos-dis-duvar-25",
            "pts": [{"x": E, "y": N+dy}, {"x": E+5., "y": N+dy}]
        }))
        .unwrap()
    });
    let slots = b.doc.add_many(Vec::from(walls), "Duvarlar").unwrap();
    let before = b.doc.to_snapshot_v2();
    b.click(2., 0.);
    assert!(b.type_text("İç duvar 15"));
    assert!(b.session.prompt().step.contains("1 duvar"));
    assert!(b.type_text("T"));
    assert!(b.session.prompt().step.contains("2 duvar"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    // An edit by another caller between the preview and Enter must invalidate the scope.
    let mut settings = b.doc.settings().clone();
    settings.length_decimals = 4;
    b.doc.set_settings(settings);
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
    assert!(b.type_text("T"));
    assert!(b.type_text("İç duvar 15"));
    b.confirm();
    let Entity::Wall(first) = b.doc.get(slots[0]).unwrap() else {
        panic!("first wall")
    };
    assert_eq!(first.wall.style.as_deref(), Some("kentos-ic-duvar-15"));
    assert_eq!(first.wall.width, 0.30);
    assert_eq!(b.doc.get(slots[1]), Some(&before.entities[1]));
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}

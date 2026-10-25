use crate::common::{Bench, E, N};
use kentos_commands::Entity;
use serde_json::json;

fn setup() -> Bench {
    let mut b = Bench::new("space");
    let mut settings = b.doc.settings().clone();
    settings.arch = Some(
        serde_json::from_value(json!({"levels":[
        {"id":"ground","name":"Zemin","elevation":0.,"height":3.},
        {"id":"upper","name":"Üst kat","elevation":0.,"height":3.}]}))
        .unwrap(),
    );
    b.doc.set_settings(settings);
    let layer = b.doc.layers().active().to_owned();
    let walls = [("ground", 6., 4.), ("upper", 10., 8.)].map(|(level, w, h)| {
        serde_json::from_value(
            json!({"kind":"wall","id":0,"layerId":layer,"attrs":{},"width":0.2,"closed":true,
            "arch":{"level":level},"pts":[{"x":E,"y":N},{"x":E+w,"y":N},
            {"x":E+w,"y":N+h},{"x":E,"y":N+h}]}),
        )
        .unwrap()
    });
    b.doc.add_many(walls.to_vec(), "Walls").unwrap();
    b.start("space");
    b
}
#[test]
fn visible_room_level_changes_preview_only_then_writes_one_level_bound_space() {
    let mut b = setup();
    assert!(b.session.settings(&Default::default()).iter().any(|r| r.key=="L" && r.value=="Tüm katlar"));
    let before = b.doc.to_snapshot_v2();
    assert!(b.run(|s, cx| s.configure("L", "id:upper", cx)));
    b.move_to(2., 2.);
    let rows = b.session.settings(&Default::default());
    assert!(rows.iter().any(|r| r.key == "L" && r.value == "Üst kat"));
    let preview = b.session.preview(&Default::default()).unwrap();
    assert!(!preview.areas.is_empty());
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(2., 2.);
    let Entity::Space(s) = b.newest() else {
        panic!()
    };
    assert_eq!(s.space.arch.level.as_deref(), Some("upper"));
    assert!((kentos_native_application::spaces::area_of(&s.space.parts) - 9.8 * 7.8).abs() < 1e-7);
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}
#[test]
fn removed_room_level_is_retained_and_refused_instead_of_falling_back_to_other_walls() {
    let mut b = setup();
    assert!(b.run(|s, cx| s.configure("L", "id:upper", cx)));
    let mut settings = b.doc.settings().clone();
    settings
        .arch
        .as_mut()
        .unwrap()
        .levels
        .retain(|l| l.id != "upper");
    b.doc.set_settings(settings);
    b.move_to(2., 2.);
    assert!(
        b.session
            .settings(&Default::default())
            .iter()
            .any(|r| r.key == "L" && r.value.contains("Bulunamadı"))
    );
    let before = b.doc.to_snapshot_v2();
    b.click(2., 2.);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(b.log.iter().any(|l| l.text.contains("kat")));
    assert!(!b.run(|s, cx| s.configure("L", "id:missing", cx)));
}

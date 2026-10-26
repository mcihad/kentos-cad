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

#[test]
fn unassigned_is_an_explicit_visible_scope_in_both_tools_and_survives_one_undo() {
    for tool in ["space","spaceDiscover"] {
        let mut b=setup();
        let mut wall=b.doc.entities().find(|e|matches!(e,Entity::Wall(_))).unwrap().clone();
        wall.base_mut().id=0;
        let Entity::Wall(w)=&mut wall else{panic!()};
        w.wall.arch.level=None;
        w.wall.pts=vec![kentos_commands::Vec2{x:E,y:N},kentos_commands::Vec2{x:E+12.,y:N},
            kentos_commands::Vec2{x:E+12.,y:N+10.},kentos_commands::Vec2{x:E,y:N+10.}];
        b.doc.add(wall).unwrap();
        b.start(tool);
        let before=b.doc.to_snapshot_v2();
        assert!(b.run(|s,cx|s.configure("L","unassigned",cx)));
        assert!(b.session.settings(&Default::default()).iter().any(|r|r.key=="L" && r.value=="Atanmamış"
            && r.choices.iter().any(|c|c.checked && c.typed=="unassigned" && c.label=="Yalnız atanmamış")));
        b.move_to(2.,2.);
        assert_eq!(b.doc.to_snapshot_v2(),before,"scope preview wrote the document");
        if tool=="space" {b.click(2.,2.);}else{b.run(|s,cx|s.confirm(cx));}
        let Entity::Space(s)=b.newest() else{panic!()};
        assert_eq!(s.space.scope,kentos_commands::SpaceScope::Unassigned);
        assert!(s.space.arch.level.is_none());
        assert!((kentos_native_application::spaces::area_of(&s.space.parts)-11.8*9.8).abs()<1e-6);
        b.doc.undo().unwrap(); assert_eq!(b.doc.to_snapshot_v2().entities,before.entities);
    }
}

#[test]
fn keyboard_can_reach_unassigned_without_any_defined_levels() {
    for tool in ["space","spaceDiscover"] {
        let mut b=Bench::new(tool);
        let before=b.doc.to_snapshot_v2();
        assert!(b.run(|s,cx|s.input("L",cx)));
        assert!(b.session.settings(&Default::default()).iter().any(|r|r.key=="L" && r.value=="Atanmamış"));
        assert!(b.run(|s,cx|s.input("L",cx)));
        assert!(b.session.settings(&Default::default()).iter().any(|r|r.key=="L" && r.value=="Tüm katlar"));
        assert_eq!(b.doc.to_snapshot_v2(),before);
    }
}

#[test]
fn pick_new_room_seed_previews_rejects_wall_keeps_identity_and_confirms_once() {
    let mut b=setup();
    assert!(b.run(|s,cx|s.configure("L","id:upper",cx)));
    assert!(b.run(|s,cx|s.configure("N","M-01",cx)));
    b.click(2.,2.);
    let slot=kentos_domain::Slot(b.newest().base().id);
    let uid=b.doc.uid(slot).unwrap().to_string();
    let before=b.doc.to_snapshot_v2();
    b.selection.set([slot]); b.start("spaceRebound");
    b.move_to(0.,2.); b.click(0.,2.);
    assert_eq!(b.doc.to_snapshot_v2(),before,"physical wall was accepted as a room point");
    b.move_to(3.,3.);
    assert!(!b.session.preview(&Default::default()).unwrap().areas.is_empty());
    assert_eq!(b.doc.to_snapshot_v2(),before,"preview wrote a room point");
    b.run(|s,cx|s.confirm(cx));
    let Entity::Space(room)=b.doc.get(slot).unwrap() else{panic!()};
    assert_eq!(room.space.seed,Some(kentos_commands::Vec2{x:E+3.,y:N+3.}));
    assert_eq!(room.space.number.as_deref(),Some("M-01"));
    assert_eq!(room.space.arch.level.as_deref(),Some("upper"));
    assert_eq!(b.doc.uid(slot).unwrap().to_string(),uid);
    let after=b.doc.to_snapshot_v2();
    b.doc.undo().unwrap(); assert_eq!(b.doc.to_snapshot_v2(),before);
    b.doc.redo().unwrap(); assert_eq!(b.doc.to_snapshot_v2(),after);
    b.selection.set([slot]);b.start("spaceRebound");b.move_to(4.,4.);
    b.run(|s,cx|s.cancel(cx));
    assert!(!b.session.is_running());
    assert_eq!(b.doc.to_snapshot_v2(),after,"cancel changed the saved room");
}

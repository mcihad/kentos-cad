use crate::common::{Bench,E,N};
use kentos_commands::Entity;
use kentos_native_application::spaces;
use serde_json::json;
#[test]
fn room_source_editor_is_advertised_to_the_real_desktop_dispatcher() {
    assert!(kentos_interaction::Session::tools().contains(&"spaceSources"));
}
fn setup(tool:&str)->Bench {
    let mut b=Bench::new(tool);let layer=b.doc.layers().active().to_owned();
    let p=|x:f64,y:f64|json!({"x":E+x,"y":N+y});
    let es=json!([
        {"kind":"wall","id":0,"layerId":layer,"attrs":{},"width":0.2,"closed":true,"pts":[p(0.,0.),p(10.,0.),p(10.,8.),p(0.,8.)]},
        {"kind":"wall","id":0,"layerId":layer,"attrs":{},"width":0.2,"pts":[p(5.,0.),p(5.,8.)]},
        {"kind":"column","id":0,"layerId":layer,"attrs":{},"width":2.,"depth":2.,"at":p(3.,4.)}
    ]);
    let added=b.doc.add_many(serde_json::from_value(es).unwrap(),"Sources").unwrap();
    b.selection.set([added[0],added[2]]);b.start(tool);b
}
#[test]
fn both_tools_capture_explicit_selection_preview_without_write_and_persist_the_same_ids() {
    for tool in ["space","spaceDiscover"] {
        let mut b=setup(tool);let before=b.doc.to_snapshot_v2();
        let mut sources=b.selection.ids().iter().map(|&s|b.doc.uid(s).unwrap().to_string()).collect::<Vec<_>>();sources.sort();
        assert!(b.run(|s,cx|s.configure("B","selected",cx)));
        assert!(b.session.settings(&Default::default()).iter().any(|r|r.key=="B"&&r.value.contains("2")));
        b.move_to(2.,6.);assert_eq!(b.doc.to_snapshot_v2(),before);
        if tool=="space" {b.click(2.,6.);}else{b.run(|s,cx|s.confirm(cx));}
        let Entity::Space(room)=b.newest() else{panic!()};
        assert_eq!(room.space.sources.as_ref(),Some(&sources));assert!((spaces::area_of(&room.space.parts)-72.44).abs()<1e-6);
        b.doc.undo().unwrap();assert_eq!(b.doc.to_snapshot_v2(),before);
    }
}
#[test]
fn existing_room_source_tool_reviews_toggles_and_confirms_one_step_or_cancels() {
    let mut b=setup("space");assert!(b.run(|s,cx|s.configure("B","selected",cx)));b.click(2.,6.);
    let room=kentos_domain::Slot(b.newest().base().id);let uid=b.doc.uid(room).unwrap();let before=b.doc.to_snapshot_v2();
    b.selection.set([room]);b.start("spaceSources");b.click(3.,3.);
    assert_eq!(b.doc.to_snapshot_v2(),before,"source click wrote before confirmation");
    assert!(!b.session.preview(&Default::default()).unwrap().areas.is_empty());
    b.run(|s,cx|s.confirm(cx));let Entity::Space(s)=b.doc.get(room).unwrap()else{panic!()};
    assert_eq!(s.space.sources.as_ref().unwrap().len(),1);assert!((spaces::area_of(&s.space.parts)-76.44).abs()<1e-6);
    assert_eq!(b.doc.uid(room),Some(uid));b.doc.undo().unwrap();assert_eq!(b.doc.to_snapshot_v2(),before);
    b.selection.set([room]);b.start("spaceSources");b.click(0.,4.);b.run(|s,cx|s.confirm(cx));
    assert_eq!(b.doc.to_snapshot_v2(),before,"unenclosed source subset was accepted");
    b.run(|s,cx|s.cancel(cx));assert_eq!(b.doc.to_snapshot_v2(),before);
}

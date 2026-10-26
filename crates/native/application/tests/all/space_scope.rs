//! Explicit unassigned sources survive edit, follower, undo and the real codec.
use super::wall_contours::{completed, drawing};
use kentos_commands::*;
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, spaces, spaces_create, spaces_discover, spaces_edit};
use serde_json::json;

fn setup(unit: &str, far: bool) -> Document {
    let (x,y)=if far {(500000.,4400000.)}else{(0.,0.)};
    let p=|a:f64,b:f64|json!({"x":x+a,"y":y+b});
    drawing(json!([
        {"kind":"wall","id":1,"layerId":"cizim","attrs":{},"width":0.2,"closed":true,"pts":[p(0.,0.),p(10.,0.),p(10.,8.),p(0.,8.)]},
        {"kind":"wall","id":2,"layerId":"cizim","attrs":{},"width":0.2,"arch":{"level":"zemin"},"pts":[p(5.,0.),p(5.,8.)]},
        {"kind":"column","id":3,"layerId":"cizim","attrs":{},"width":2.,"depth":2.,"at":p(3.,4.)},
        {"kind":"column","id":4,"layerId":"cizim","attrs":{},"width":2.,"depth":2.,"at":p(8.,4.),"arch":{"level":"kat-1"}}
    ]),unit)
}
fn request(far:bool,method:&str,scope:&str)->SpacesCreate {
    serde_json::from_value(json!({"layerId":"cizim","spaces":[{"seed":{"x":if far{500002.}else{2.},"y":if far{4400006.}else{6.}},"method":method,"scope":scope,"columns":"all"}]})).unwrap()
}
fn fields(e:&Entity)->&SpaceFields { let Entity::Space(s)=e else{panic!()}; &s.space }
fn area(doc:&Document,slot:Slot)->f64 { spaces::area_of(&fields(doc.get(slot).unwrap()).parts) }
fn close(a:f64,b:f64){assert!((a-b).abs()<1e-6,"{a} != {b}");}
fn create(doc:&mut Document,input:SpacesCreate)->Slot {
    Slot(completed(spaces_create::execute(&mut ExecutionContext::new(doc),input)).ids[0])
}
fn level(doc:&mut Document,slot:Slot,target:Option<&str>) {
    let mut e=doc.get(slot).unwrap().clone();
    match &mut e {
        Entity::Wall(w)=>w.wall.arch.level=target.map(str::to_owned),
        Entity::Column(c)=>c.column.arch.level=target.map(str::to_owned),
        _=>panic!(),
    }
    assert!(doc.update(slot,e));
}

#[test]
fn three_methods_five_units_two_origins_and_real_codec_keep_the_source_scope() {
    for unit in ["mm","cm","m","in","ft"] {for far in [false,true] {
        for (method,delta) in [("net",-0.2),("centerline",0.),("gross",0.2)] {
            let mut doc=setup(unit,far);
            let before=doc.to_snapshot_v2();
            let input=request(far,method,"unassigned");
            let plan=completed(spaces_create::plan_in(&doc,&input));
            close(spaces::area_of(&fields(&plan.spaces[0]).parts),(10.+delta)*(8.+delta)-4.);
            assert_eq!(doc.to_snapshot_v2(),before,"plan wrote sources");
            let room=create(&mut doc,input);
            let saved=doc.to_snapshot_v2();
            let bytes=kentos_kcad::encode(&saved).unwrap();
            assert_eq!(kentos_kcad::decode(&bytes).unwrap(),saved);
            assert_eq!(fields(doc.get(room).unwrap()).scope,SpaceScope::Unassigned);
            doc.undo().unwrap(); assert_eq!(doc.to_snapshot_v2().entities,before.entities);
            doc.redo().unwrap(); assert_eq!(doc.to_snapshot_v2(),saved);
            let mut legacy=setup(unit,far);
            let r=create(&mut legacy,request(far,method,"project"));
            close(area(&legacy,r),(5.+delta)*(8.+delta)-4.);
            assert!(serde_json::to_value(fields(legacy.get(r).unwrap())).unwrap().get("scope").is_none());
        }
    }}
}

#[test]
fn assigning_and_unassigning_wall_and_column_sources_recomputes_in_the_same_undo_step() {
    let mut doc=setup("mm",true);
    let room=create(&mut doc,request(true,"net","unassigned"));
    let uid=doc.uid(room).unwrap().to_string();
    let before=doc.to_snapshot_v2();
    close(area(&doc,room),72.44);
    level(&mut doc,Slot(2),None); close(area(&doc,room),33.44);
    let after=doc.to_snapshot_v2();
    doc.undo().unwrap(); assert_eq!(doc.to_snapshot_v2(),before);
    doc.redo().unwrap(); assert_eq!(doc.to_snapshot_v2(),after);
    doc.undo().unwrap();
    level(&mut doc,Slot(4),None); close(area(&doc,room),68.44);
    doc.undo().unwrap(); assert_eq!(doc.to_snapshot_v2(),before);
    level(&mut doc,Slot(1),Some("zemin"));
    assert_eq!(fields(doc.get(room).unwrap()).broken,Some(SpaceBreak::LostEnclosure));
    close(area(&doc,room),72.44);
    level(&mut doc,Slot(1),None);
    assert!(fields(doc.get(room).unwrap()).broken.is_none());
    assert_eq!(doc.uid(room).unwrap().to_string(),uid);
    close(area(&doc,room),72.44);
}

#[test]
fn bulk_and_single_room_share_scope_while_changing_it_is_one_undo() {
    let mut doc=setup("m",false);
    let input:SpacesDiscover=serde_json::from_value(json!({"layerId":"cizim","scope":"unassigned","columns":"all"})).unwrap();
    let before=doc.to_snapshot_v2();
    let plan=completed(spaces_discover::plan_in(&doc,&input));
    assert_eq!(plan.proposals.len(),1);
    close(spaces::area_of(&plan.proposals[0].space.parts),72.44);
    assert_eq!(plan.proposals[0].space.scope,SpaceScope::Unassigned);
    assert_eq!(doc.to_snapshot_v2(),before);
    let done=completed(spaces_discover::execute(&mut ExecutionContext::new(&mut doc),input.clone()));
    assert_eq!(done.ids.len(),1);
    let room=Slot(done.ids[0]);
    let repeat=completed(spaces_discover::plan_in(&doc,&input));
    assert_eq!(repeat.proposals[0].existing,done.uids);
    let saved=doc.to_snapshot_v2();
    level(&mut doc,Slot(2),None);
    assert_eq!(fields(doc.get(room).unwrap()).broken,Some(SpaceBreak::LostEnclosure));
    close(area(&doc,room),72.44);
    doc.undo().unwrap(); assert_eq!(doc.to_snapshot_v2(),saved);
    let edit=SpacesEdit {uids:done.uids,operation:SpacesOperation::Set(SpaceChange {scope:Some(SpaceScope::Project),..Default::default()}),expected_revision:None};
    let CommandResult::Failed{error}=spaces_edit::execute(&mut ExecutionContext::new(&mut doc),edit.clone()) else{panic!()};
    assert_eq!(error.path.as_deref(),Some("operation.scope"));
    assert_eq!(doc.to_snapshot_v2(),saved,"unsafe scope change wrote a wall-material room");
    let rebound=SpacesEdit {uids:edit.uids.clone(),operation:SpacesOperation::Rebound {seed:Some(Vec2{x:2.,y:6.})},expected_revision:None};
    completed(spaces_edit::execute(&mut ExecutionContext::new(&mut doc),rebound));
    let reseeded=doc.to_snapshot_v2();
    completed(spaces_edit::execute(&mut ExecutionContext::new(&mut doc),edit));
    close(area(&doc,room),33.44);
    doc.undo().unwrap(); assert_eq!(doc.to_snapshot_v2(),reseeded);
}

#[test]
fn contradictory_level_and_drawn_source_scope_are_atomic_explicit_errors() {
    let mut doc=setup("m",false);
    for spec in [
        json!({"seed":{"x":2.,"y":6.},"scope":"unassigned","arch":{"level":"zemin"}}),
        json!({"scope":"unassigned","parts":[{"outer":{"pts":[{"x":0.,"y":0.},{"x":2.,"y":0.},{"x":2.,"y":2.},{"x":0.,"y":2.}]}}]}),
    ] {
        let before=doc.to_snapshot_v2();
        let input:SpacesCreate=serde_json::from_value(json!({"layerId":"cizim","spaces":[spec]})).unwrap();
        let CommandResult::Failed{error}=spaces_create::execute(&mut ExecutionContext::new(&mut doc),input) else{panic!()};
        assert_eq!(error.code,"invalid_space"); assert_eq!(error.path.as_deref(),Some("spaces[0].scope"));
        assert_eq!(doc.to_snapshot_v2(),before);
    }
}

#[test]
fn split_and_merge_detach_scope_preserve_physical_area_and_undo_restores_the_link() {
    let mut doc = setup("m", false);
    let room = create(&mut doc, request(false, "net", "unassigned"));
    let uid = doc.uid(room).unwrap().to_string();
    let original = doc.to_snapshot_v2();
    let split = SpacesEdit {
        uids: vec![uid.clone()],
        operation: SpacesOperation::Split {
            line: vec![Vec2 { x: 5., y: -1. }, Vec2 { x: 5., y: 9. }],
        },
        expected_revision: None,
    };
    let done = completed(spaces_edit::execute(&mut ExecutionContext::new(&mut doc), split));
    assert_eq!(done.added.len(), 1);
    assert_eq!(doc.uid(room).unwrap().to_string(), uid);
    let added = Slot(done.added[0].base().id);
    let added_uid = doc.uid(added).unwrap().to_string();
    // Independent rectangle arithmetic: 4.9 × 7.8 on each side, one 2 × 2 hole.
    close(area(&doc, room), 38.22);
    close(area(&doc, added), 34.22);
    for slot in [room, added] {
        let f = fields(doc.get(slot).unwrap());
        assert_eq!(f.scope, SpaceScope::Project);
        assert_eq!(f.columns, SpaceColumns::Ignore);
        assert!(f.seed.is_none() && f.broken.is_none());
    }
    let divided = doc.to_snapshot_v2();
    let merge = SpacesEdit {
        uids: vec![uid.clone(), added_uid],
        operation: SpacesOperation::Merge,
        expected_revision: None,
    };
    completed(spaces_edit::execute(&mut ExecutionContext::new(&mut doc), merge));
    close(area(&doc, room), 72.44);
    assert!(doc.get(added).is_none());
    assert_eq!(doc.uid(room).unwrap().to_string(), uid);
    let f = fields(doc.get(room).unwrap());
    assert_eq!(f.scope, SpaceScope::Project);
    assert_eq!(f.columns, SpaceColumns::Ignore);
    assert!(f.seed.is_none() && f.broken.is_none());
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), divided);
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), original);
}

//! Persistent source subsets: physical rectangles worked out independently.
use super::wall_contours::{completed,drawing};
use kentos_commands::*;
use kentos_domain::{Document,Slot};
use kentos_native_application::{ExecutionContext,spaces,spaces_create,spaces_discover,spaces_edit};
use serde_json::json;
fn setup(unit:&str,far:bool)->Document {
    let (x,y)=if far{(500000.,4400000.)}else{(0.,0.)};
    let p=|a:f64,b:f64|json!({"x":x+a,"y":y+b});
    drawing(json!([
        {"kind":"wall","id":1,"layerId":"cizim","attrs":{},"width":0.2,"closed":true,"pts":[p(0.,0.),p(10.,0.),p(10.,8.),p(0.,8.)]},
        {"kind":"wall","id":2,"layerId":"cizim","attrs":{},"width":0.2,"pts":[p(5.,0.),p(5.,8.)]},
        {"kind":"column","id":3,"layerId":"cizim","attrs":{},"width":2.,"depth":2.,"at":p(3.,4.)},
        {"kind":"column","id":4,"layerId":"cizim","attrs":{},"width":2.,"depth":2.,"at":p(8.,4.)},
        {"kind":"line","id":5,"layerId":"cizim","attrs":{},"a":p(0.,0.),"b":p(10.,8.)}
    ]),unit)
}
fn ids(doc:&Document,slots:&[u32])->Vec<String>{slots.iter().map(|&s|doc.uid(Slot(s)).unwrap().to_string()).collect()}
fn request(doc:&Document,slots:&[u32],far:bool,method:&str)->SpacesCreate {
    serde_json::from_value(json!({"layerId":"cizim","spaces":[{"seed":{"x":if far{500002.}else{2.},"y":if far{4400006.}else{6.}},"method":method,"columns":"all","sources":ids(doc,slots)}]})).unwrap()
}
fn fields(e:&Entity)->&SpaceFields{let Entity::Space(s)=e else{panic!()};&s.space}
fn close(a:f64,b:f64){assert!((a-b).abs()<1e-6,"{a} != {b}");}
fn area(doc:&Document,slot:Slot)->f64{spaces::area_of(&fields(doc.get(slot).unwrap()).parts)}
fn create(doc:&mut Document,input:SpacesCreate)->Slot{Slot(completed(spaces_create::execute(&mut ExecutionContext::new(doc),input)).ids[0])}
fn edit(doc:&Document,room:Slot,source:Option<Vec<String>>)->SpacesEdit {
    SpacesEdit{uids:ids(doc,&[room.0]),operation:SpacesOperation::Set(SpaceChange{sources:Some(source),..Default::default()}),expected_revision:None}
}
#[test]
fn methods_units_origins_and_codecs_use_only_the_explicit_sources() {
    for unit in ["mm","cm","m","in","ft"]{for far in [false,true]{for (method,d) in [("net",-0.2),("centerline",0.),("gross",0.2)]{
        let mut doc=setup(unit,far);let before=doc.to_snapshot_v2();
        let input=request(&doc,&[3,1],far,method);
        let plan=completed(spaces_create::plan_in(&doc,&input));
        close(spaces::area_of(&fields(&plan.spaces[0]).parts),(10.+d)*(8.+d)-4.);
        assert_eq!(doc.to_snapshot_v2(),before);
        let room=create(&mut doc,input);
        let f=fields(doc.get(room).unwrap());let mut want=ids(&doc,&[1,3]);want.sort();
        assert_eq!(f.sources.as_ref(),Some(&want));
        let snapshot=doc.to_snapshot_v2();let bytes=kentos_kcad::encode(&snapshot).unwrap();
        assert_eq!(kentos_kcad::decode(&bytes).unwrap(),snapshot);
        doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
        doc.redo().unwrap();assert_eq!(doc.to_snapshot_v2(),snapshot);
    }}}
}
#[test]
fn excluded_sources_do_not_rebind_and_selected_sources_follow_in_one_step() {
    let mut doc=setup("mm",true);let input=request(&doc,&[1,3],true,"net");let room=create(&mut doc,input);
    let before=doc.to_snapshot_v2();close(area(&doc,room),72.44);
    let mut other=doc.get(Slot(2)).unwrap().clone();let Entity::Wall(w)=&mut other else{panic!()};w.wall.width=1.;
    doc.update(Slot(2),other);close(area(&doc,room),72.44);doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
    let mut col=doc.get(Slot(3)).unwrap().clone();let Entity::Column(c)=&mut col else{panic!()};c.column.width=1.;
    doc.update(Slot(3),col);close(area(&doc,room),74.44);let after=doc.to_snapshot_v2();
    doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);doc.redo().unwrap();assert_eq!(doc.to_snapshot_v2(),after);doc.undo().unwrap();
    doc.remove(&[Slot(3)]);assert_eq!(fields(doc.get(room).unwrap()).broken,Some(SpaceBreak::SourceUnavailable));close(area(&doc,room),72.44);
    doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
    // A listed remote source has a persistent identity even outside the room's boxes.
    let req=edit(&doc,room,Some(ids(&doc,&[1,3,4])));completed(spaces_edit::execute(&mut ExecutionContext::new(&mut doc),req));close(area(&doc,room),68.44);
    let mut remote=doc.get(Slot(4)).unwrap().clone();let Entity::Column(c)=&mut remote else{panic!()};c.column.at.x+=1000.;doc.update(Slot(4),remote);
    close(area(&doc,room),72.44);doc.remove(&[Slot(4)]);
    assert_eq!(fields(doc.get(room).unwrap()).broken,Some(SpaceBreak::SourceUnavailable));
    doc.undo().unwrap();assert!(fields(doc.get(room).unwrap()).broken.is_none());
}
#[test]
fn bulk_persists_list_while_explicit_replacement_clear_and_reject_are_atomic() {
    let mut doc=setup("m",false);
    let input:SpacesDiscover=serde_json::from_value(json!({"layerId":"cizim","sources":ids(&doc,&[1,3]),"columns":"all"})).unwrap();
    let plan=completed(spaces_discover::plan_in(&doc,&input));assert_eq!(plan.proposals.len(),1);close(spaces::area_of(&plan.proposals[0].space.parts),72.44);
    let done=completed(spaces_discover::execute(&mut ExecutionContext::new(&mut doc),input.clone()));let room=Slot(done.ids[0]);
    assert_eq!(completed(spaces_discover::plan_in(&doc,&input)).proposals[0].existing,done.uids);
    // The whole-room label may lie on the excluded partition; source changes
    // must refuse that seed, not silently choose a neighbouring free face.
    let old=doc.to_snapshot_v2();let bad=edit(&doc,room,Some(ids(&doc,&[1,2,3])));
    if fields(doc.get(room).unwrap()).seed.unwrap().x == 5. {
        assert!(!matches!(spaces_edit::execute(&mut ExecutionContext::new(&mut doc),bad),CommandResult::Completed{..}));
        assert_eq!(doc.to_snapshot_v2(),old);
    }
    let rebound=SpacesEdit{uids:done.uids.clone(),operation:SpacesOperation::Rebound{seed:Some(Vec2{x:2.,y:6.})},expected_revision:None};
    completed(spaces_edit::execute(&mut ExecutionContext::new(&mut doc),rebound));
    let before=doc.to_snapshot_v2();let req=edit(&doc,room,Some(ids(&doc,&[1,2,3])));
    let plan=completed(spaces_edit::plan_in(&doc,&req));close(spaces::area_of(&fields(&plan.spaces[0]).parts),33.44);assert_eq!(doc.to_snapshot_v2(),before);
    completed(spaces_edit::execute(&mut ExecutionContext::new(&mut doc),req));close(area(&doc,room),33.44);doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
    let req=edit(&doc,room,None);completed(spaces_edit::execute(&mut ExecutionContext::new(&mut doc),req));assert!(fields(doc.get(room).unwrap()).sources.is_none());close(area(&doc,room),33.44);
    doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
    for source in [vec![],ids(&doc,&[1,1]),ids(&doc,&[1,5]),vec!["00000000-0000-0000-0000-000000000001".into()]] {
        let req=edit(&doc,room,Some(source));assert!(!matches!(spaces_edit::execute(&mut ExecutionContext::new(&mut doc),req),CommandResult::Completed{..}));assert_eq!(doc.to_snapshot_v2(),before);
    }
    let mut other=doc.get(Slot(1)).unwrap().clone();let Entity::Wall(w)=&mut other else{panic!()};w.wall.arch.level=Some("zemin".into());
    // project scope without level accepts every level; explicit unassigned does not.
    doc.update(Slot(1),other);assert!(fields(doc.get(room).unwrap()).broken.is_none());
    let req=SpacesEdit{uids:done.uids,operation:SpacesOperation::Set(SpaceChange{scope:Some(SpaceScope::Unassigned),..Default::default()}),expected_revision:None};
    assert!(!matches!(spaces_edit::execute(&mut ExecutionContext::new(&mut doc),req),CommandResult::Completed{..}));
}
#[test]
fn copy_with_all_sources_relinks_before_following_but_room_alone_is_independent() {
    use kentos_native_application::transform;
    let mut doc=setup("mm",true);let input=request(&doc,&[1,3],true,"net");let room=create(&mut doc,input);
    let before=doc.to_snapshot_v2();
    let command=|doc:&Document,slots:&[u32]|serde_json::from_value::<EntitiesTransform>(json!({
        "uids":ids(doc,slots),"copy":true,"transform":{"kind":"move","dx":20.,"dy":0.}
    })).unwrap();
    let input=command(&doc,&[1,3,room.0]);
    let done=completed(transform::execute(&mut ExecutionContext::new(&mut doc),input));
    let added:Vec<_>=done.created.iter().map(|id|doc.slot_of(kentos_domain::Uuid::parse_str(id).unwrap()).unwrap()).collect();
    assert_eq!(added.len(),3);
    let copied=fields(doc.get(added[2]).unwrap());let mut expected=ids(&doc,&[added[0].0,added[1].0]);expected.sort();
    assert_eq!(copied.sources.as_ref(),Some(&expected));assert!(copied.broken.is_none());close(area(&doc,added[2]),72.44);
    assert_eq!(copied.seed.unwrap().x,500022.);
    doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
    let input=command(&doc,&[room.0]);
    let preview=transform::plan(&ExecutionContext::new(&mut doc),&input);
    let CommandResult::Completed{output,warnings}=preview else{panic!("{preview:?}")};
    assert!(warnings.iter().any(|w|w.code=="room_sources_not_copied"));
    assert!(fields(&output.entities[0]).sources.is_none());
    assert!(fields(&output.entities[0]).seed.is_none());
    assert_eq!(doc.to_snapshot_v2(),before);
    let result=transform::execute(&mut ExecutionContext::new(&mut doc),input);
    let CommandResult::Completed{output,warnings}=result else{panic!("{result:?}")};
    assert!(warnings.iter().any(|w|w.code=="room_sources_not_copied"));
    let added=doc.slot_of(kentos_domain::Uuid::parse_str(&output.created[0]).unwrap()).unwrap();
    let copied=fields(doc.get(added).unwrap());assert!(copied.seed.is_none()&&copied.sources.is_none());close(area(&doc,added),72.44);
    doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
}

#[test]
fn array_and_clipboard_relink_each_group_and_preserve_original_room() {
    use kentos_native_application::{array, entities_copy, entities_paste};
    let mut doc=setup("mm",false);
    let input=request(&doc,&[1,3],false,"net");
    let room=create(&mut doc,input);
    let before=doc.to_snapshot_v2();
    let input:EntitiesArray=serde_json::from_value(json!({"uids":ids(&doc,&[1,3,room.0]),
        "layout":{"kind":"grid","rows":1,"cols":3,"dx":20.,"dy":0.}})).unwrap();
    let plan=completed(array::plan(&ExecutionContext::new(&mut doc),&input));
    assert_eq!(plan.entities.len(),6);assert_eq!(doc.to_snapshot_v2(),before);
    let done=completed(array::execute(&mut ExecutionContext::new(&mut doc),input));
    for group in done.created.chunks(3) {
        let slot=doc.slot_of(kentos_domain::Uuid::parse_str(&group[2]).unwrap()).unwrap();
        let mut sources=group[..2].to_vec();sources.sort();
        assert_eq!(fields(doc.get(slot).unwrap()).sources.as_ref(),Some(&sources));
        assert!(fields(doc.get(slot).unwrap()).broken.is_none());close(area(&doc,slot),72.44);
    }
    doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
    for complete in [true,false] {
        let slots=if complete{vec![1,3,room.0]}else{vec![room.0]};
        let clip=completed(entities_copy::answer(&doc,&EntitiesCopy{uids:ids(&doc,&slots),base:Some(Vec2{x:0.,y:0.})}));
        let input=EntitiesPaste{clip,at:Some(Vec2{x:20.,y:0.}),as_block:None,conflicts:Default::default(),expected_revision:None};
        let plan=completed(entities_paste::plan_in(&doc,None,&input));
        assert_eq!(plan.created.len(),slots.len());assert_eq!(doc.to_snapshot_v2(),before);
        let done=completed(entities_paste::execute(&mut ExecutionContext::new(&mut doc),input));
        let slot=doc.slot_of(kentos_domain::Uuid::parse_str(done.created.last().unwrap()).unwrap()).unwrap();
        let f=fields(doc.get(slot).unwrap());
        if complete {
            let mut sources=done.created[..2].to_vec();sources.sort();
            assert_eq!(f.sources.as_ref(),Some(&sources));assert!(f.broken.is_none());
        } else {assert!(f.sources.is_none()&&f.seed.is_none());}
        close(area(&doc,slot),72.44);
        assert_eq!(doc.get(room),Some(&before.entities[room.0 as usize-1]));
        doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
    }
}

#[test]
fn dxf_import_relinks_present_sources_and_preserves_missing_source_warning() {
    use kentos_native_application::import_apply::{self, ImportPlan, LayerTarget};
    let mut original = setup("mm", true);
    let input = request(&original, &[1, 3], true, "net");
    let room = create(&mut original, input);
    for complete in [true, false] {
        let slots = if complete { vec![1, 3, room.0] } else { vec![1, room.0] };
        let entities: Vec<_> = slots.iter().map(|&id| original.get(Slot(id)).unwrap().clone()).collect();
        let mut input: DxfWriteInput = serde_json::from_value(json!({
            "entities": entities, "layers": [{"id":"cizim", "name":"Çizim", "path":[],
                "color":"#FFFFFF", "visible":true, "locked":false, "lineType":"continuous", "lineWeight":0.25}],
            "scale":100., "lengthDecimals":3, "grads":false, "unit":"mm"
        })).unwrap();
        input.uids = slots.iter().map(|&id| (id, EntityId(original.uid(Slot(id)).unwrap().into_bytes()))).collect();
        let (bytes, _) = kentos_formats::dxf::write(&input);
        // Use the production import's physical-unit options. Low-level None
        // intentionally leaves file coordinates raw (200 mm is then 200).
        let options = kentos_native_application::file_import::read_options(&original, None, false, None);
        let read = kentos_formats::dxf::read(&bytes, &options).unwrap();
        assert_eq!(read.entities.len(), slots.len());
        for entity in &read.entities {
            match entity {
                Entity::Wall(w) => assert!(w.wall.problem().is_none(), "DXF wall: {:?}, {:?}", w.wall.problem(), w),
                Entity::Column(c) => assert!(c.column.problem().is_none(), "DXF column: {:?}, {:?}", c.column.problem(), c),
                _ => {},
            }
        }
        let imported = fields(read.entities.last().unwrap());
        let mut expected = vec![import_placeholder(0).to_text(),
            import_placeholder(if complete { 1 } else { u32::MAX }).to_text()];
        expected.sort();
        assert_eq!(imported.sources.as_ref(), Some(&expected));
        let mut doc = setup("mm", true);
        let before = doc.to_snapshot_v2();
        let plan = ImportPlan { label: "DXF kaynak kabulü".into(),
            layers: read.layers.iter().map(|l| (l.name.clone(), LayerTarget::Existing("cizim".into()))).collect(),
            group: None, source_srid: None };
        let applied = import_apply::apply_import(&mut doc, read.entities, read.blocks, &plan).unwrap();
        let added = *applied.slots.last().unwrap();
        let f = fields(doc.get(added).unwrap());
        let mut expected = vec![doc.uid(applied.slots[0]).unwrap().to_string(),
            if complete { doc.uid(applied.slots[1]).unwrap().to_string() }
            else { import_placeholder(u32::MAX).to_text() }];
        expected.sort();
        assert_eq!(f.sources.as_ref(), Some(&expected));
        assert_eq!(f.broken, if complete { None } else { Some(SpaceBreak::SourceUnavailable) });
        close(area(&doc, added), 72.44);
        doc.undo().unwrap(); assert_eq!(doc.to_snapshot_v2(), before);
    }
}

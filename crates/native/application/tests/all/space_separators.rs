use super::wall_contours::{completed,drawing};
use kentos_commands::*;
use kentos_domain::{Document,Slot};
use kentos_native_application::{ExecutionContext,spaces,spaces_create,spaces_edit,spaces_separators,transform,edit};
use serde_json::json;

/// The answer without what docs/adr/0722 §5 added (a room's baseboard, its rule's sentence, the totals' baseboard
/// length, the floor changes): the frozen answers predate it, and everything else must stay as it was.
fn before_0722(mut v: serde_json::Value) -> serde_json::Value {
    const SENTENCE: &str = "; süpürgelik: çevreden kenarları boyunca duran geçişlerin (kapı, döşemeye inen boşluk) genişliği düşülür";
    fn walk(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(map) => {
                if let Some(b) = map.remove("baseboard") {
                    let perimeter = map.get("perimeter").and_then(|p| p.as_f64()).unwrap_or(f64::INFINITY);
                    assert!(b.as_f64().is_some_and(|b| b >= 0.0 && b <= perimeter + 1e-9), "{b} {perimeter}");
                }
                map.remove("baseboardLength");
                map.remove("transitions");
                if let Some(serde_json::Value::String(rule)) = map.get_mut("rule") {
                    *rule = rule.replace(SENTENCE, "");
                }
                map.values_mut().for_each(walk);
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(walk),
            _ => {}
        }
    }
    walk(&mut v);
    v
}

#[test]
fn quantity_query_reuses_sources_without_changing_frozen_answers_or_document_order() {
    use kentos_native_application::spaces_quantities;
    let base=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/baseline/2026-10-25-architecture-quantity-cache");
    // Frozen from the preceding implementation, not a new geometry oracle.
    // Independent physical expectations are also exercised by the shared cases.
    let cases:serde_json::Value=serde_json::from_slice(&std::fs::read(base.join("answers-before.json")).unwrap()).unwrap();
    assert_eq!(cases.as_array().unwrap().len(),20);
    for case in cases.as_array().unwrap() {
        let name=case["file"].as_str().unwrap();
        let snapshot:DocumentSnapshotV2=serde_json::from_slice(&std::fs::read(base.join(name)).unwrap()).unwrap();
        let mut doc=Document::from_snapshot_v2(snapshot).unwrap();
        let before=doc.to_snapshot_v2();
        for answer in case["answers"].as_array().unwrap() {
            let input:SpacesQuantities=serde_json::from_value(answer["query"].clone()).unwrap();
            let plan=completed(spaces_quantities::plan_in(&doc,&input));
            assert_eq!(before_0722(serde_json::to_value(&plan).unwrap()),answer["output"],"{name}: {:?}",input);
            let executed=completed(spaces_quantities::execute(&mut ExecutionContext::new(&mut doc),input));
            assert_eq!(executed,plan);
            assert_eq!(doc.to_snapshot_v2(),before);
            assert!(!doc.can_undo() && !doc.is_dirty());
        }
        // Reversed/repeated methods and source keys must not reuse another
        // room's level, scope, explicit identities or physical source order.
        let fields:Vec<_>=doc.entities().filter_map(|e|if let Entity::Space(s)=e {Some(s.space.clone())}else{None}).collect();
        let mut search=kentos_domain::space_follow::SpaceSearch::new(&doc);
        for room in fields.iter().rev().chain(&fields) {
            for method in SpaceMethod::ALL.iter().rev() {
                let mut room=room.clone();room.method = *method;
                assert_eq!(search.found_preserving_legacy(&room),doc.space_found(&room),"{name}: {method:?}");
            }
        }
    }
}

fn setup()->Document {
    drawing(json!([
        {"kind":"wall","id":1,"layerId":"cizim","attrs":{},"width":0.2,"closed":true,"pts":[{"x":0.,"y":0.},{"x":10.,"y":0.},{"x":10.,"y":8.},{"x":0.,"y":8.}]},
        {"kind":"column","id":2,"layerId":"cizim","attrs":{},"width":2.,"depth":2.,"at":{"x":3.,"y":4.}}
    ]),"m")
}
fn create_room(doc:&mut Document,sources:Option<Vec<String>>)->Slot {
    Slot(completed(spaces_create::execute(&mut ExecutionContext::new(doc),serde_json::from_value(json!({"layerId":"cizim","spaces":[{"seed":{"x":2.,"y":6.},"columns":"all","sources":sources}]})).unwrap())).ids[0])
}
fn create_separator(doc:&mut Document)->Slot {
    Slot(completed(spaces_separators::execute(&mut ExecutionContext::new(doc),serde_json::from_value(json!({"operation":{"kind":"create","layerId":"cizim","paths":[{"pts":[{"x":5.,"y":0.},{"x":5.,"y":8.}]}]}})).unwrap())).ids[0])
}
fn fields(doc:&Document,slot:Slot)->&SpaceFields {let Entity::Space(s)=doc.get(slot).unwrap()else{panic!()};&s.space}
fn area(doc:&Document,slot:Slot,expected:f64){let actual=spaces::area_of(&fields(doc,slot).parts);assert!((actual-expected).abs()<1e-7,"{actual} != {expected}");}
fn uids(doc:&Document,slots:&[Slot])->Vec<String>{slots.iter().map(|&s|doc.uid(s).unwrap().to_string()).collect()}
fn clear(doc:&mut Document,slot:Slot) {
    let input=SpacesSeparators {operation:SpaceSeparatorOperation::Clear {uids:uids(doc,&[slot])},expected_revision:None};
    completed(spaces_separators::execute(&mut ExecutionContext::new(doc),input));
}
#[test]
fn ordinary_linework_is_not_a_boundary_and_role_changes_follow_one_step() {
    let mut doc=setup();let room=create_room(&mut doc,None);area(&doc,room,72.44);
    let before=doc.to_snapshot_v2();let separator=create_separator(&mut doc);area(&doc,room,34.22);
    let with=doc.to_snapshot_v2();doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
    doc.redo().unwrap();assert_eq!(doc.to_snapshot_v2(),with);
    clear(&mut doc,separator);area(&doc,room,72.44);
    let Entity::Polyline(p)=doc.get(separator).unwrap()else{panic!()};assert!(p.space_separator.is_none());
    doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),with);
    let bytes=kentos_kcad::encode_verified(&with).unwrap();assert_eq!(kentos_kcad::decode(&bytes).unwrap(),with);
}
#[test]
fn explicit_subset_excludes_unlisted_separators_and_keeps_missing_source_boundary() {
    let mut doc=setup();let selected=uids(&doc,&[Slot(1),Slot(2)]);let room=create_room(&mut doc,Some(selected));
    let separator=create_separator(&mut doc);area(&doc,room,72.44);
    let input=SpacesEdit {uids:uids(&doc,&[room]),operation:SpacesOperation::Set(SpaceChange {sources:Some(Some(uids(&doc,&[Slot(1),Slot(2),separator]))),..Default::default()}),expected_revision:None};
    completed(spaces_edit::execute(&mut ExecutionContext::new(&mut doc),input));area(&doc,room,34.22);
    let before=doc.to_snapshot_v2();clear(&mut doc,separator);area(&doc,room,34.22);
    assert_eq!(fields(&doc,room).broken,Some(SpaceBreak::SourceUnavailable));
    doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
    doc.remove(&[separator]);area(&doc,room,34.22);assert_eq!(fields(&doc,room).broken,Some(SpaceBreak::SourceUnavailable));
    doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
}
#[test]
fn complete_copy_relinks_virtual_sources_and_original_move_does_not_change_copy() {
    let mut doc=setup();let room=create_room(&mut doc,None);let separator=create_separator(&mut doc);
    let input=SpacesEdit {uids:uids(&doc,&[room]),operation:SpacesOperation::Set(SpaceChange {sources:Some(Some(uids(&doc,&[Slot(1),Slot(2),separator]))),..Default::default()}),expected_revision:None};
    completed(spaces_edit::execute(&mut ExecutionContext::new(&mut doc),input));
    let input:EntitiesTransform=serde_json::from_value(json!({"uids":uids(&doc,&[Slot(1),Slot(2),room,separator]),"copy":true,"transform":{"kind":"move","dx":20.,"dy":0.}})).unwrap();
    completed(transform::execute(&mut ExecutionContext::new(&mut doc),input));
    let copy_room=Slot(7);let copy_separator=Slot(8);area(&doc,room,34.22);area(&doc,copy_room,34.22);
    assert!(kentos_domain::space_follow::separator_path(doc.get(copy_separator).unwrap()).is_some());
    let mut expected=uids(&doc,&[Slot(5),Slot(6),copy_separator]);expected.sort();assert_eq!(fields(&doc,copy_room).sources.as_ref(),Some(&expected));
    let input:EntitiesTransform=serde_json::from_value(json!({"uids":uids(&doc,&[separator]),"transform":{"kind":"move","dx":1.,"dy":0.}})).unwrap();
    completed(transform::execute(&mut ExecutionContext::new(&mut doc),input));area(&doc,room,42.02);area(&doc,copy_room,34.22);
    let before=doc.to_snapshot_v2();doc.remove(&[copy_separator]);area(&doc,room,42.02);area(&doc,copy_room,34.22);
    assert_eq!(fields(&doc,copy_room).broken,Some(SpaceBreak::SourceUnavailable));doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
}
#[test]
fn generic_edit_retains_line_and_arc_roles_and_refuses_loss_or_width_atomically() {
    let mut doc=setup();let separator=create_separator(&mut doc);let uid=doc.uid(separator).unwrap().to_string();
    for geometry in [json!({"kind":"line","a":{"x":5.,"y":0.},"b":{"x":5.,"y":8.}}),
        json!({"kind":"arc","c":{"x":5.,"y":4.},"r":4.,"a0":0.,"a1":std::f64::consts::PI})] {
        let input:EntitiesEdit=serde_json::from_value(json!({"operation":"properties","changes":[{"kind":"update","uid":uid,"geometry":geometry}]})).unwrap();
        completed(edit::execute(&mut ExecutionContext::new(&mut doc),input));assert!(kentos_domain::space_follow::separator_path(doc.get(separator).unwrap()).is_some());
        assert_eq!(doc.uid(separator).unwrap().to_string(),uid);
    }
    for geometry in [json!({"kind":"ellipse","c":{"x":5.,"y":4.},"major":{"x":4.,"y":0.},"ratio":0.5,"t0":0.,"t1":std::f64::consts::TAU}),
        json!({"kind":"polyline","pts":[{"x":5.,"y":0.},{"x":5.,"y":8.}],"widths":[[1.,1.],[0.,0.]]})] {
        let before=doc.to_snapshot_v2();let input:EntitiesEdit=serde_json::from_value(json!({"operation":"properties","changes":[{"kind":"update","uid":uid,"geometry":geometry}]})).unwrap();
        let result=edit::execute(&mut ExecutionContext::new(&mut doc),input);
        assert!(matches!(result,CommandResult::Failed {error} if error.code=="invalid_space_separator"));assert_eq!(doc.to_snapshot_v2(),before);
    }
}

#[test]
fn dxf_preserves_separator_roles_and_never_guesses_missing_or_ambiguous_room_sources() {
    use kentos_native_application::import_apply::{self,ImportPlan,LayerTarget};
    for case in ["complete","missing","ambiguous"] {
        let mut original=setup();let room=create_room(&mut original,None);let separator=create_separator(&mut original);
        let input=SpacesEdit {uids:uids(&original,&[room]),operation:SpacesOperation::Set(SpaceChange {sources:Some(Some(uids(&original,&[Slot(1),Slot(2),separator]))),..Default::default()}),expected_revision:None};
        completed(spaces_edit::execute(&mut ExecutionContext::new(&mut original),input));
        let mut slots=vec![Slot(1),Slot(2)];
        if case!="missing" {slots.push(separator);}
        if case=="ambiguous" {
            let mut copy=original.get(separator).unwrap().clone();
            if let Entity::Polyline(p)=&mut copy {for p in &mut p.pts {p.x+=2.;}}
            slots.push(original.add(copy).unwrap());
        }
        slots.push(room);
        let mut input:DxfWriteInput=serde_json::from_value(json!({"entities":slots.iter().map(|s|original.get(*s).unwrap().clone()).collect::<Vec<_>>(),
            "layers":[{"id":"cizim","name":"Çizim","path":[],"color":"#FFFFFF","visible":true,"locked":false,"lineType":"continuous","lineWeight":0.25}],
            "scale":100.,"lengthDecimals":3,"grads":false,"unit":"mm"})).unwrap();
        input.uids=slots.iter().map(|&s|(s.0,EntityId(original.uid(s).unwrap().into_bytes()))).collect();
        if case=="ambiguous" {input.uids.insert(slots[3].0,EntityId(original.uid(separator).unwrap().into_bytes()));}
        let (bytes,_)=kentos_formats::dxf::write(&input);
        let options=kentos_native_application::file_import::read_options(&original,None,false,None);
        let read=kentos_formats::dxf::read(&bytes,&options).unwrap();
        assert_eq!(read.entities.len(),slots.len(),"{case}");
        assert_eq!(read.entities.iter().filter(|e|kentos_domain::space_follow::separator_path(e).is_some()).count(),if case=="complete"{1}else if case=="ambiguous"{2}else{0});
        let mut doc=setup();let before=doc.to_snapshot_v2();
        let plan=ImportPlan {label:"DXF ayırıcı kabulü".into(),layers:read.layers.iter().map(|l|(l.name.clone(),LayerTarget::Existing("cizim".into()))).collect(),group:None,source_srid:None};
        let applied=import_apply::apply_import(&mut doc,read.entities,read.blocks,&plan).unwrap();
        let added=*applied.slots.last().unwrap();area(&doc,added,34.22);
        let mut expected=uids(&doc,&applied.slots[..2]);
        expected.push(if case=="complete"{doc.uid(applied.slots[2]).unwrap().to_string()}else{import_placeholder(u32::MAX).to_text()});expected.sort();
        assert_eq!(fields(&doc,added).sources.as_ref(),Some(&expected),"{case}");
        assert_eq!(fields(&doc,added).broken,if case=="complete"{None}else{Some(SpaceBreak::SourceUnavailable)},"{case}");
        doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);
        if case=="complete" {
            input.annotations.metadata=false;
            let (bytes,report)=kentos_formats::dxf::write(&input);
            assert!(serde_json::to_string(&report).unwrap().contains("mahal ayırıcı rolü"));
            let plain=kentos_formats::dxf::read(&bytes,&options).unwrap();
            assert!(plain.entities.iter().all(|e|kentos_domain::space_follow::separator_path(e).is_none()));
        }
    }
}

#[test]
fn block_conversions_refuse_separator_role_loss_without_touching_the_drawing() {
    use kentos_native_application::{blocks_define,blocks_edit,entities_copy,entities_paste,entities_explode};
    let mut doc=setup();let room=create_room(&mut doc,None);let separator=create_separator(&mut doc);
    let before=doc.to_snapshot_v2();
    let input:BlocksDefine=serde_json::from_value(json!({"name":"Sınır","uids":uids(&doc,&[separator]),"base":{"x":0.,"y":0.},"replace":true,"layerId":"cizim"})).unwrap();
    assert!(matches!(blocks_define::execute(&mut ExecutionContext::new(&mut doc),input),CommandResult::Failed {error} if error.code=="block_refused"));
    assert_eq!(doc.to_snapshot_v2(),before);area(&doc,room,34.22);
    let clip=completed(entities_copy::answer(&doc,&EntitiesCopy {uids:uids(&doc,&[separator]),base:None}));
    let input:EntitiesPaste=serde_json::from_value(json!({"clip":clip,"asBlock":"Sınır"})).unwrap();
    assert!(matches!(entities_paste::execute(&mut ExecutionContext::new(&mut doc),input),CommandResult::Failed {error} if error.code=="invalid_clip"));
    assert_eq!(doc.to_snapshot_v2(),before);
    // KCAD may contain such a definition: preserve it on read/save and refuse an unsupported
    // flatten, even through nested inserts, instead of silently discarding the child's role.
    let id=BlockId([7;16]);let nested=BlockId([8;16]);
    let definition=|id,name,entities|serde_json::from_value::<BlockDefinition>(json!({"id":id,"name":name,"base":{"x":0.,"y":0.},"entities":entities})).unwrap();
    let mut child=doc.get(separator).unwrap().clone();
    if let Entity::Polyline(p)=&mut child {p.base.id=1;}
    doc.add_block(definition(id,"İç sınır",vec![child])).unwrap();
    let insert=|id,block|serde_json::from_value::<Entity>(json!({"kind":"insert","id":id,"block":block,"layerId":"cizim","attrs":{},"p":{"x":20.,"y":0.},"scale":1.,"rotation":0.})).unwrap();
    doc.add_block(definition(nested,"Dış sınır",vec![insert(1,id)])).unwrap();
    let slot=doc.add(insert(0,nested)).unwrap();let before=doc.to_snapshot_v2();
    let input:BlocksEdit=serde_json::from_value(json!({"block":id,"operation":"update","entities":[doc.get(separator).unwrap()]})).unwrap();
    assert!(matches!(blocks_edit::execute(&mut ExecutionContext::new(&mut doc),input),CommandResult::Failed {error} if error.code=="block_refused"));
    assert_eq!(doc.to_snapshot_v2(),before);
    let input:EntitiesExplode=serde_json::from_value(json!({"uids":uids(&doc,&[slot])})).unwrap();
    assert!(matches!(entities_explode::execute(&mut ExecutionContext::new(&mut doc),input),CommandResult::Failed {error} if error.code=="not_explodable"));
    assert_eq!(doc.to_snapshot_v2(),before);
    let encoded=kentos_kcad::encode_verified(&before).unwrap();assert_eq!(kentos_kcad::decode(&encoded).unwrap(),before);
}

#[test]
fn stale_quantities_warn_without_a_column_scope_or_explicit_source_option() {
    use kentos_native_application::spaces_quantities;
    for virtual_boundary in [false,true] {
        let mut source=json!({"kind":if virtual_boundary{"polyline"}else{"wall"},"id":1,"layerId":"cizim","attrs":{},"closed":true,
            "pts":[{"x":0.,"y":0.},{"x":4.,"y":0.},{"x":4.,"y":3.},{"x":0.,"y":3.}]});
        if virtual_boundary {source["spaceSeparator"]=json!({});}else{source["width"]=json!(0.2);}
        let mut doc=drawing(json!([source]),"m");
        completed(spaces_create::execute(&mut ExecutionContext::new(&mut doc),serde_json::from_value(json!({"layerId":"cizim","spaces":[{"seed":{"x":1.,"y":1.}}]})).unwrap()));
        let before=doc.to_snapshot_v2();
        let query=|doc:&Document|completed(spaces_quantities::plan_in(doc,&Default::default()));
        let valid=query(&doc);assert_eq!(valid.spaces[0].boundary_break,None);
        assert!(!serde_json::to_value(&valid.spaces[0]).unwrap().as_object().unwrap().contains_key("boundaryBreak"));
        doc.remove(&[Slot(1)]);let lost=doc.to_snapshot_v2();let q=query(&doc);
        assert_eq!(q.spaces[0].boundary_break,Some(SpaceBreak::LostEnclosure));
        assert!(q.spaces[0].rule.contains("son geçerli sınır"));
        assert!((q.spaces[0].area-if virtual_boundary{12.}else{10.64}).abs()<1e-8);
        assert_eq!(q.spaces[0].by_method,Some(Default::default()));
        assert_eq!(q.totals.space_area,q.spaces[0].area);
        assert_eq!(doc.to_snapshot_v2(),lost,"query wrote the document");
        doc.undo().unwrap();assert_eq!(doc.to_snapshot_v2(),before);assert_eq!(query(&doc),valid);
    }
}

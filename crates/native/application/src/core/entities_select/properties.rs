//! Property refinement of the existing selection query. No second geometry or expression engine.
use kentos_commands::{Entity, LayerFilter, SelectProperties};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::store::Store;
use crate::checks::{Stop, error};
use crate::{codes, layer_filter};

fn fail(message: &str, field: &str) -> Stop {
    Stop::failed(error(codes::INVALID_INPUT, message.into(), Some(format!("region.query.{field}"))))
}

fn expression(doc: &Document, q: &SelectProperties) -> Result<layer_filter::CompiledFilter, Stop> {
    layer_filter::compile_filter(&LayerFilter { expression: q.expression.clone(), objects: Vec::new() },
        crate::expression_unit::reading_unit(doc.settings()))
        .map_err(|why| fail(&why, "expression"))
}

pub(super) fn check(doc: &Document, q: &SelectProperties) -> Result<(), Stop> {
    if q.block_path.len() > kentos_commands::SUBREF_PATH_DEPTH {
        return Err(fail("Blok parçası yolu en çok 16 düzey olabilir.", "blockPath"));
    }
    if !q.block_path.is_empty() && (q.text_height_min.is_some() || q.text_height_max.is_some() || q.expression.is_some()) {
        return Err(fail("Blok parçası yolu yalnız renk sorgularında kullanılır; sonuç tanım değil yerleştirme kimliğidir.", "blockPath"));
    }
    for (name, value) in [("textHeightMin", q.text_height_min), ("textHeightMax", q.text_height_max)] {
        if value.is_some_and(|v| !v.is_finite() || v <= 0.0) {
            return Err(fail("Yazı yüksekliği sonlu ve sıfırdan büyük metre değeri olmalı.", name));
        }
    }
    if q.text_height_min.zip(q.text_height_max).is_some_and(|(min,max)| min > max) {
        return Err(fail("En küçük yükseklik en büyük yükseklikten büyük olamaz.", "textHeightMax"));
    }
    for (name, value) in [("assignedColor", &q.assigned_color), ("displayedColor", &q.displayed_color)] {
        if value.as_ref().is_some_and(|v| v.trim().is_empty() || v.len() > 128) {
            return Err(fail("Renk boş olamaz; CAD renk değeri veya atanan renk için byLayer kullanın.", name));
        }
    }
    if q.expression.as_ref().is_some_and(|s| s.len() > 32768) {
        return Err(fail("Seçim ifadesi en çok 32768 bayt olabilir.", "expression"));
    }
    // A thematic symbol may paint different parts in different colours. Never call the layer's fallback its displayed colour.
    if q.displayed_color.is_some() && doc.layers().leaves().iter().any(|l| l.style.renderer.is_some()) {
        return Err(fail("Tematik katman görünüşü bulunan çizimde tek CAD görünür rengi sorgulanamaz. Atanan renk veya tematik ifade kullanın.", "displayedColor"));
    }
    expression(doc, q)?;
    Ok(())
}

pub(super) fn filter(doc: &Document, store: &Store, ids: Vec<Slot>, q: &SelectProperties) -> Vec<Slot> {
    let layers = doc.layers();
    let blocks = (!q.block_path.is_empty()).then(|| crate::blocks::drawing_blocks(doc));
    let ids: Vec<_> = ids.into_iter().filter(|slot| {
        let Some(e) = doc.get(*slot) else { return false; };
        if q.text_height_min.is_some() || q.text_height_max.is_some() {
            let Entity::Text(t) = e else { return false; };
            if q.text_height_min.is_some_and(|min| t.height < min) || q.text_height_max.is_some_and(|max| t.height > max) {
                return false;
            }
        }
        let base = e.base();
        let piece = if let Some(blocks) = &blocks {
            let Some(colors) = piece_colors(doc, blocks, e, &q.block_path) else { return false; };
            Some(colors)
        } else { None };
        if let Some(wanted) = &q.assigned_color {
            let assigned = piece.as_ref().map_or_else(|| base.color.as_deref().unwrap_or("byLayer"), |p| p.0.as_str());
            if !assigned.eq_ignore_ascii_case(wanted.trim()) { return false; }
        }
        if let Some(wanted) = &q.displayed_color {
            let displayed = piece.as_ref().map(|p| p.1.as_str()).or_else(|| base.color.as_deref().or_else(|| layers.get(&base.layer_id).map(|l| l.style.color.as_str())));
            if displayed.is_none_or(|c| !c.eq_ignore_ascii_case(wanted.trim())) { return false; }
        }
        true
    }).collect();
    let Ok(compiled) = expression(doc, q) else { return Vec::new(); };
    if !compiled.has_condition() { return ids; }
    let entities: Vec<_> = ids.iter().filter_map(|s| doc.get(*s)).collect();
    let passing = compiled.condition(&entities, &|id| layers.get(id).map_or_else(|| id.into(), |l| l.name.clone()),
        |i| store.get(f64::from(ids[i].0)).map(|item| &item.shape));
    ids.into_iter().zip(passing).filter_map(|(id, pass)| pass.then_some(id)).collect()
}

/// Resolve every placement in order; flattened colours cannot reveal whether a leaf was ByBlock or explicit.
pub(super) fn piece_colors(doc: &Document, blocks: &kentos_geometry_core::block::Blocks, e: &Entity, path: &[u32]) -> Option<(String, String)> {
    use kentos_commands::{OwnLook, LookPart, resolve_look};
    use kentos_geometry_core::api::json::{Json, to_string};
    let root = e.base();
    let layers = doc.layers();
    let mut layer = root.layer_id.clone();
    let style = &layers.get(&layer)?.style;
    let mut resolved = resolve_look(OwnLook {color:root.color.as_deref(),line_weight:root.line_weight,look:&root.look}, style, None);
    let chain = blocks.piece_chain(&crate::blocks::core_entity(e).shape, path)?;
    let mut assigned = String::new();
    for part in chain {
        let base: kentos_commands::EntityBase = serde_json::from_str(&to_string(&Json::Obj(part.rest))).ok()?;
        if !base.layer_id.is_empty() { layer = base.layer_id.clone(); }
        let style = &layers.get(&layer)?.style;
        assigned = base.color.clone().unwrap_or_else(|| if base.look.by_layer.contains(&LookPart::Color) {"byLayer"} else {"byBlock"}.into());
        resolved = resolve_look(OwnLook {color:base.color.as_deref(),line_weight:base.line_weight,look:&base.look}, style, Some(&resolved));
    }
    Some((assigned, resolved.color))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{CallOp, DocumentSnapshotV1};
    use crate::{ExecutionContext, registry};
    use serde_json::{Value, json};

    fn document() -> (Document, Vec<String>) {
        let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(include_str!("../../../../../../fixtures/interaction/v1/objects.kcad")).unwrap()).unwrap();
        let mut settings = doc.settings().clone();
        settings.workspace = Some(kentos_commands::Workspace::Cad);
        settings.srid = 0;
        settings.drawing_unit = Some(kentos_commands::DrawingUnit::M);
        doc.set_settings(settings);
        let mut uids = Vec::new();
        for (height, color) in [(0.2,None), (0.3,Some("#E5484D")), (0.3,None)] {
            let e = serde_json::from_value(json!({"kind":"text","id":0,"layerId":"parsel","attrs":{"Cat":"target"},"p":{"x":487000.,"y":4420000.},"text":"Denetim","height":height,"rotation":0,"color":color})).unwrap();
            let slot = doc.add(e).unwrap();
            uids.push(doc.uid(slot).unwrap().to_string());
        }
        (doc,uids)
    }
    fn ask(doc: &mut Document, uids: &[String], query: Value) -> Value {
        registry::run(&mut ExecutionContext::new(doc), "cad.entities.select", CallOp::Execute,
            json!({"region":{"kind":"properties","source":{"kind":"objects","uids":uids},"query":query}})).unwrap()
    }

    #[test]
    fn selection_distinguishes_inherited_assignment_from_identical_displayed_paint_and_height() {
        let (mut doc,uids) = document();
        let before = doc.to_snapshot_v2();
        let answer = ask(&mut doc,&uids,json!({"assignedColor":"byLayer"}));
        assert_eq!(answer["output"]["uids"],json!([uids[0],uids[2]]),"{answer}");
        let answer = ask(&mut doc,&uids,json!({"displayedColor":"#e5484d"}));
        assert_eq!(answer["output"]["uids"],json!(uids),"{answer}");
        let answer = ask(&mut doc,&uids,json!({"textHeightMin":0.3,"textHeightMax":0.3,"expression":"Cat = 'target'","assignedColor":"byLayer"}));
        assert_eq!(answer["output"]["uids"],json!([uids[2]]),"{answer}");
        for query in [json!({"textHeightMin":0.5,"textHeightMax":0.3}),json!({"textHeightMax":0}),json!({"expression":"("})] {
            let answer = ask(&mut doc,&uids,query);
            assert_eq!(answer["status"],"failed","{answer}");
        }
        assert_eq!(doc.to_snapshot_v2(),before,"queries/refusals write nothing");
    }

    #[test]
    fn semantic_text_height_plan_is_read_only_execute_is_one_undo_and_lock_refusal_is_atomic() {
        let (mut doc,uids) = document();
        let before = doc.to_snapshot_v2();
        let input = json!({"edit":{"kind":"height","uids":uids,"height":0.25}});
        for op in [CallOp::Validate,CallOp::Plan] {
            let answer=registry::run(&mut ExecutionContext::new(&mut doc),"cad.texts.edit",op,input.clone()).unwrap();
            assert_eq!(answer["status"],"completed","{answer}");
            assert_eq!(doc.to_snapshot_v2(),before);
        }
        let answer=registry::run(&mut ExecutionContext::new(&mut doc),"cad.texts.edit",CallOp::Execute,input).unwrap();
        assert_eq!(answer["status"],"completed","{answer}");
        for uid in &uids {
            let Entity::Text(t)=doc.get(doc.slot_of(uid.parse().unwrap()).unwrap()).unwrap() else{panic!("text")};
            assert_eq!(t.height,0.25);
            assert_eq!(t.text,"Denetim");
        }
        assert_eq!(doc.undo().as_deref(),Some("Yazı yüksekliği"));
        assert_eq!(doc.to_snapshot_v2(),before);
        doc.toggle_layer_locked("parsel");
        let before = doc.to_snapshot_v2();
        let answer=registry::run(&mut ExecutionContext::new(&mut doc),"cad.texts.edit",CallOp::Execute,json!({"edit":{"kind":"height","uids":uids,"height":0.4}})).unwrap();
        assert_eq!(answer["status"],"failed","{answer}");
        assert_eq!(doc.to_snapshot_v2(),before);
    }

    #[test]
    fn nested_piece_query_distinguishes_by_layer_by_block_and_explicit_color_without_editing_definitions() {
        let (mut doc, _) = document();
        let mut owners = Vec::new();
        for (i, look) in [json!({"byLayer":["color"]}), json!({"color":"#E5484D"}), json!({})].into_iter().enumerate() {
            let inner = format!("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d41{:02}", i * 2);
            let outer = format!("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d41{:02}", i * 2 + 1);
            let mut leaf = json!({"kind":"line","id":1,"layerId":"parsel","attrs":{},"a":{"x":0,"y":0},"b":{"x":2,"y":0}});
            for (key,value) in look.as_object().unwrap() { leaf[key] = value.clone(); }
            for value in [json!({"id":inner,"name":format!("İç {i}"),"base":{"x":0,"y":0},"entities":[leaf]}),
                json!({"id":outer,"name":format!("Dış {i}"),"base":{"x":0,"y":0},"entities":[{"kind":"insert","id":1,"layerId":"","attrs":{},"block":inner,"p":{"x":0,"y":0},"scale":1,"rotation":0}]})] {
                doc.add_block(serde_json::from_value(value).unwrap()).unwrap();
            }
            let slot=doc.add(serde_json::from_value(json!({"kind":"insert","id":0,"layerId":"cizim","attrs":{},"color":"#0099FF","block":outer,"p":{"x":i as f64*5.,"y":0},"scale":1,"rotation":0})).unwrap()).unwrap();
            owners.push(doc.uid(slot).unwrap().to_string());
        }
        let before=doc.to_snapshot_v2();
        for (color, wanted) in [("byLayer",0),("#E5484D",1),("byBlock",2)] {
            let answer=ask(&mut doc,&owners,json!({"blockPath":[0,0],"assignedColor":color}));
            assert_eq!(answer["output"]["uids"],json!([owners[wanted]]),"{answer}");
        }
        let answer=ask(&mut doc,&owners,json!({"blockPath":[0,0],"displayedColor":"#E5484D"}));
        assert_eq!(answer["output"]["uids"],json!([owners[0],owners[1]]),"{answer}");
        let answer=ask(&mut doc,&owners,json!({"blockPath":[0,0],"displayedColor":"#0099FF"}));
        assert_eq!(answer["output"]["uids"],json!([owners[2]]),"{answer}");
        assert_eq!(ask(&mut doc,&owners,json!({"blockPath":[0,0],"textHeightMin":0.3}))["status"],"failed");
        assert_eq!(ask(&mut doc,&owners,json!({"blockPath":[0,99],"assignedColor":"byBlock"}))["output"]["uids"],json!([]));
        assert_eq!(doc.to_snapshot_v2(),before,"queries return owner UIDs without altering definitions");
    }
}

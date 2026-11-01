//! Temporary object isolation is a view command: no file, layer or geometry history changes.
use crate::{
    ExecutionContext,
    checks::{self, Stop, error},
    codes,
};
use kentos_commands::{
    CommandResult, EntitiesIsolate, EntitiesIsolated, IsolationAction, MAX_SELECT_ITEMS,
};
use kentos_domain::{Document, Slot};

fn check(doc: &Document, input: &EntitiesIsolate) -> Result<(), Stop> {
    if let IsolationAction::Isolate { uids } = &input.action {
        if uids.len() > MAX_SELECT_ITEMS {
            return Err(Stop::failed(error(
                codes::INVALID_INPUT,
                format!("Bir işlemde en çok {MAX_SELECT_ITEMS} nesne yalıtılabilir."),
                Some("uids".into()),
            )));
        }
        checks::uids(uids, "Yalıtılacak nesne yok.")?;
        checks::objects(doc, uids)?;
    }
    Ok(())
}

fn state(doc: &Document) -> EntitiesIsolated {
    let retained = doc
        .entities()
        .filter(|e| doc.isolation_shown(Slot(e.base().id)))
        .count();
    EntitiesIsolated {
        active: doc.objects_isolated(),
        hidden: doc.len() - retained,
        retained,
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesIsolate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesIsolate) -> CommandResult<EntitiesIsolated> {
    let mut copy = cx.doc.reading_copy();
    execute(&mut ExecutionContext::new(&mut copy), input.clone())
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesIsolate,
) -> CommandResult<EntitiesIsolated> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    if matches!(input.action, IsolationAction::Query) {
        return CommandResult::Completed {
            output: state(cx.doc),
            warnings: Vec::new(),
        };
    }
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ENTITIES_ISOLATE,
        kentos_commands::CAD_ENTITIES_ISOLATE_VERSION,
        input,
        apply,
    )
}

fn apply(cx: &mut ExecutionContext<'_>, input: EntitiesIsolate) -> CommandResult<EntitiesIsolated> {
    match input.action {
        IsolationAction::Isolate { uids } => {
            let keep = uids
                .iter()
                .filter_map(|u| u.parse().ok())
                .collect::<Vec<_>>();
            cx.doc.isolate_objects(&keep);
        }
        IsolationAction::End => cx.doc.end_object_isolation(),
        IsolationAction::Query => {}
    }
    CommandResult::Completed {
        output: state(cx.doc),
        warnings: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry;
    use kentos_commands::{CallOp, DocumentSnapshotV1};
    use serde_json::{Value, json};

    fn drawing() -> Document {
        Document::from_snapshot(
            DocumentSnapshotV1::from_json(include_str!(
                "../../../../../fixtures/interaction/v1/objects.kcad"
            ))
            .unwrap(),
        )
        .unwrap()
    }
    fn call(doc: &mut Document, op: CallOp, input: Value) -> Value {
        registry::run(
            &mut ExecutionContext::new(doc),
            "cad.entities.isolate",
            op,
            json!({"action":input}),
        )
        .unwrap()
    }
    fn hits(doc: &mut Document) -> Value {
        registry::run(
            &mut ExecutionContext::new(doc),
            "cad.entities.select",
            CallOp::Execute,
            json!({"region":{"kind":"all"}}),
        )
        .unwrap()["output"]["uids"]
            .clone()
    }

    #[test]
    fn isolation_limit_is_atomic() {
        let mut doc=drawing();let before=doc.to_snapshot_v2();let generation=doc.generation();
        let uid=doc.uid(Slot(1)).unwrap().to_string();
        let result=call(&mut doc,CallOp::Execute,json!({"kind":"isolate","uids":vec![uid;MAX_SELECT_ITEMS+1]}));
        assert_eq!(result["error"]["code"],"invalid_input");
        assert_eq!(doc.to_snapshot_v2(),before);assert_eq!(doc.generation(),generation);
        assert!(!doc.objects_isolated());
    }

    #[test]
    fn temporary_isolation_is_readonly_in_plan_unsaved_and_separate_from_geometry_undo() {
        let mut doc = drawing();
        let slot = doc.add(serde_json::from_value(json!({"kind":"line","id":0,"layerId":"cizim","attrs":{},"a":{"x":0,"y":0},"b":{"x":2,"y":0}})).unwrap()).unwrap();
        let uid = doc.uid(slot).unwrap().to_string();
        let before = doc.to_snapshot_v2();
        let revision = doc.revision();
        let generation = doc.generation();
        let history = doc.undo_labels();
        let input = json!({"kind":"isolate","uids":[uid]});
        let planned = call(&mut doc, CallOp::Plan, input.clone());
        assert_eq!(planned["output"]["retained"], 1);
        assert!(!doc.objects_isolated());
        assert_eq!(doc.generation(), generation);
        assert_eq!(
            call(&mut doc, CallOp::Execute, input)["output"],
            planned["output"]
        );
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.revision(), revision);
        assert_eq!(doc.undo_labels(), history);
        assert_eq!(hits(&mut doc), json!([uid]));
        assert!(
            doc.undo().is_some(),
            "the next Undo removes the actual line, not the view mask"
        );
        assert!(doc.get(slot).is_none());
        assert!(doc.can_redo());
        call(&mut doc, CallOp::Execute, json!({"kind":"end"}));
        assert!(
            doc.can_redo(),
            "ending isolation does not erase geometry Redo"
        );
        assert!(doc.redo().is_some());
        assert_eq!(doc.to_snapshot_v2(), before);
        let mut reopened = Document::from_snapshot_v2(before).unwrap();
        assert!(!reopened.objects_isolated());
        assert!(hits(&mut reopened).as_array().unwrap().len() > 1);
    }

    #[test]
    fn invalid_targets_are_atomic_and_new_objects_remain_visible_while_hidden_undo_keeps_uid() {
        let mut doc = drawing();
        let uid = doc.uid(Slot(1)).unwrap().to_string();
        let before = doc.to_snapshot_v2();
        let generation = doc.generation();
        for uids in [
            json!([]),
            json!([uid, "not-a-uid"]),
            json!([uid, "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6dffff"]),
        ] {
            assert_ne!(
                call(
                    &mut doc,
                    CallOp::Execute,
                    json!({"kind":"isolate","uids":uids})
                )["status"],
                "completed"
            );
            assert!(!doc.objects_isolated());
            assert_eq!(doc.generation(), generation);
            assert_eq!(doc.to_snapshot_v2(), before);
        }
        call(
            &mut doc,
            CallOp::Execute,
            json!({"kind":"isolate","uids":[uid,uid]}),
        );
        let other = doc
            .entities()
            .map(|e| Slot(e.base().id))
            .find(|s| *s != Slot(1))
            .unwrap();
        assert!(!doc.isolation_shown(other));
        assert_eq!(doc.remove(&[other]), 1);
        doc.undo().unwrap();
        assert!(
            !doc.isolation_shown(other),
            "restored hidden object keeps its persistent-identity mask"
        );
        let new = doc.add(serde_json::from_value(json!({"kind":"line","id":0,"layerId":"cizim","attrs":{},"a":{"x":0,"y":0},"b":{"x":2,"y":0}})).unwrap()).unwrap();
        assert!(doc.isolation_shown(new));
        let copy = doc.reading_copy();
        assert!(!copy.isolation_shown(other));
        assert!(copy.isolation_shown(new));
        let state = call(&mut doc, CallOp::Execute, json!({"kind":"query"}));
        assert_eq!(state["output"]["retained"], 2);
    }
}

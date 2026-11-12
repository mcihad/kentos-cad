//! `cad.drawing.take` v1 (Farkları al, docs/adr/1000 §5): chosen objects of another state of the drawing taken into
//! the open drawing by their persistent ids. An object only the other state has comes back with its id, one only
//! the open drawing has goes, one that differs is replaced by its other state (its slot and id kept). “The same” is
//! the stamp's data being equal: taking carries the data as it is, so there is no tolerance. All or nothing (a
//! locked layer refuses all, ADR 0047), one undo step “Farkları al”; the linked dimensions, hatches and texts follow
//! once at its end (`call::recorded`). With a common base, an object both sides changed apart is a conflict unless
//! `overwrite` says so. Block definitions and layer looks are not objects: they stay, with a warning when they differ.
//! The shared cases are `fixtures/commands/v1/cad.drawing.take.json` (`scripts/fixtures/review_cases.py`).

use std::collections::HashSet;

use kentos_commands::{
    CommandResult, CommandWarning, DRAWING_TAKE_STEP, DrawingTake, DrawingTaken, Entity, MAX_TAKE_UIDS,
};
use kentos_domain::etag::entity_value;
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::drawing_compare::other_state;
use crate::review_notes::block_chain_stamp;

fn fail(code: &str, message: String, path: String) -> Stop {
    Stop::failed(error(code, message, Some(path)))
}

/// What happens to one id.
enum Step {
    Add(Uuid, Entity),
    Replace(Slot, Entity),
    Remove(Slot),
    Same,
}

/// The checked plan: each id's step, in the input's order, and the warnings.
struct Checked {
    steps: Vec<(String, Step)>,
    warnings: Vec<CommandWarning>,
}

fn object<'a>(doc: &'a Document, uid: Uuid) -> Option<(Slot, &'a Entity)> {
    let slot = doc.slot_of(uid)?;
    Some((slot, doc.get(slot)?))
}

fn same(a: Option<&Entity>, b: Option<&Entity>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => entity_value(a) == entity_value(b),
        _ => false,
    }
}

fn layer_name(doc: &Document, id: &str) -> String {
    doc.layers().get(id).map_or_else(|| id.to_owned(), |n| n.name.clone())
}

/// The states the input names: the other state and the base. Read before the write opens its step, since the
/// history's “steps back” counts from the drawing as the command found it.
type States = Result<(Document, Option<Document>), Stop>;

fn states(doc: &Document, input: &DrawingTake) -> States {
    let (other, _) = other_state(doc, &input.against, "against")?;
    let base = match &input.base {
        Some(b) => Some(other_state(doc, b, "base")?.0),
        None => None,
    };
    Ok((other, base))
}

fn check(doc: &Document, input: &DrawingTake, read: impl FnOnce() -> States) -> Result<Checked, Stop> {
    if input.uids.is_empty() || input.uids.len() > MAX_TAKE_UIDS {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Alınacak nesne sayısı 1 ile {MAX_TAKE_UIDS} arasında olmalı; {} verildi.", input.uids.len()),
            "uids".into(),
        ));
    }
    let mut seen = HashSet::new();
    for (i, uid) in input.uids.iter().enumerate() {
        if !checks::is_uid_text(uid) {
            return Err(fail(
                codes::INVALID_UID,
                format!("“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir. Kimliği karşılaştırmanın çıktısından alın."),
                format!("uids[{i}]"),
            ));
        }
        if !seen.insert(uid.as_str()) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{uid}” iki kez verildi; her nesneyi bir kez verin."),
                format!("uids[{i}]"),
            ));
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let (other, base) = read()?;
    let layers = doc.layers();
    let locked = |id: &str, at: usize| -> Result<(), Stop> {
        if layers.is_locked(id) {
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{}” katmanı kilitli; üzerindeki nesne değiştirilemez, hiçbir fark alınmadı. Kilidi Katmanlar panelinden açın.",
                    layer_name(doc, id)
                ),
                format!("uids[{at}]"),
            ));
        }
        Ok(())
    };
    let mut steps = Vec::with_capacity(input.uids.len());
    let mut warnings = Vec::new();
    let mut definition_differs = 0usize;
    let mut style_differs = 0usize;
    for (i, text) in input.uids.iter().enumerate() {
        let path = format!("uids[{i}]");
        let Ok(uid) = Uuid::parse_str(text) else {
            unreachable!("checked as uid text above");
        };
        let then = object(&other, uid);
        let now = object(doc, uid);
        if then.is_none() && now.is_none() {
            return Err(fail(
                codes::ENTITY_NOT_FOUND,
                format!("“{text}” kimlikli nesne ne açık çizimde ne de karşılaştırılan sürümde var; karşılaştırmayı yenileyin."),
                path,
            ));
        }
        if same(then.map(|t| t.1), now.map(|n| n.1)) {
            steps.push((text.clone(), Step::Same));
            continue;
        }
        if let Some((_, e)) = then {
            let layer = &e.base().layer_id;
            if layers.get(layer).is_none() {
                return Err(fail(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "“{}” katmanı açık çizimde yok; nesne alınamaz. Önce katmanı ekleyin ya da sürümden alın.",
                        layer_name(&other, layer)
                    ),
                    path,
                ));
            }
            if let Entity::Insert(ins) = e {
                if doc.block(ins.block).is_none() {
                    let name = other.block(ins.block).map_or_else(|| "?".to_owned(), |b| b.name.clone());
                    return Err(fail(
                        codes::UNKNOWN_BLOCK,
                        format!("“{name}” bloğunun tanımı açık çizimde yok; yerleştirme alınamaz. Önce tanımı blok kitaplığından ekleyin."),
                        path,
                    ));
                }
                if block_chain_stamp(doc, ins.block) != block_chain_stamp(&other, ins.block) {
                    definition_differs += 1;
                }
            }
            if let (Some(a), Some(b)) = (other.layers().get(layer), layers.get(layer))
                && a.style != b.style
            {
                style_differs += 1;
            }
            locked(layer, i)?;
        }
        if let Some((_, e)) = now {
            locked(&e.base().layer_id, i)?;
        }
        if let Some(base) = &base
            && !input.overwrite
        {
            let was = object(base, uid).map(|b| b.1);
            let moved_here = !same(was, now.map(|n| n.1));
            let moved_there = !same(was, then.map(|t| t.1));
            if moved_here && moved_there {
                return Err(fail(
                    codes::TAKE_CONFLICT,
                    format!("“{text}” nesnesi ortak tabandan sonra iki tarafta da farklı değişti; hiçbir fark alınmadı. Üzerine yazmayı açıkça seçin ya da nesneyi karşılaştırmadan çıkarın."),
                    path,
                ));
            }
        }
        let step = match (then, now) {
            (Some((_, e)), None) => Step::Add(uid, e.clone()),
            (Some((_, e)), Some((slot, _))) => Step::Replace(slot, e.clone()),
            (None, Some((slot, _))) => Step::Remove(slot),
            (None, None) => unreachable!("refused above"),
        };
        steps.push((text.clone(), step));
    }
    if definition_differs > 0 {
        warnings.push(CommandWarning {
            code: codes::BLOCK_DEFINITION_DIFFERS.into(),
            message: format!(
                "{definition_differs} blok yerleştirmesinin tanımı iki sürümde farklı; tanımlar açık çizimdeki olarak kaldı, görüntü sürümdekinden farklı olabilir."
            ),
            path: Some("uids".into()),
        });
    }
    if style_differs > 0 {
        warnings.push(CommandWarning {
            code: codes::LAYER_STYLE_DIFFERS.into(),
            message: format!(
                "{style_differs} nesnenin katman görünüşü iki sürümde farklı; katman görünüşü alınmadı."
            ),
            path: Some("uids".into()),
        });
    }
    Ok(Checked { steps, warnings })
}

fn listed(c: &Checked, revision: String) -> DrawingTaken {
    let mut out = DrawingTaken { revision, ..DrawingTaken::default() };
    for (uid, step) in &c.steps {
        match step {
            Step::Add(..) => out.added.push(uid.clone()),
            Step::Replace(..) => out.replaced.push(uid.clone()),
            Step::Remove(_) => out.removed.push(uid.clone()),
            Step::Same => out.unchanged.push(uid.clone()),
        }
    }
    out
}

/// Checks the take, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &DrawingTake) -> CommandResult<()> {
    match check(cx.doc, input, || states(cx.doc, input)) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would take now and the revision to expect for exactly that.
pub fn plan(cx: &ExecutionContext<'_>, input: &DrawingTake) -> CommandResult<DrawingTaken> {
    match check(cx.doc, input, || states(cx.doc, input)) {
        Ok(c) => {
            let output = listed(&c, cx.doc.revision().to_string());
            CommandResult::Completed { output, warnings: c.warnings }
        }
        Err(stop) => stop.into(),
    }
}

/// Takes the objects as one undo step “Farkları al”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: DrawingTake) -> CommandResult<DrawingTaken> {
    let read = states(cx.doc, &input);
    crate::call::recorded(
        cx,
        kentos_commands::CAD_DRAWING_TAKE,
        kentos_commands::CAD_DRAWING_TAKE_VERSION,
        input,
        move |cx, input| write(cx, input, read),
    )
}

fn write(cx: &mut ExecutionContext<'_>, input: DrawingTake, read: States) -> CommandResult<DrawingTaken> {
    let c = match check(cx.doc, &input, move || read) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let done = listed(&c, String::new());
    if done.added.is_empty() && done.replaced.is_empty() && done.removed.is_empty() {
        let revision = cx.doc.revision().to_string();
        return CommandResult::Completed { output: DrawingTaken { revision, ..done }, warnings: c.warnings };
    }
    cx.doc.name_step(DRAWING_TAKE_STEP);
    let mut adds = Vec::new();
    let mut updates = Vec::new();
    let mut removes = Vec::new();
    for (_, step) in c.steps {
        match step {
            Step::Add(uid, e) => adds.push((uid, e)),
            Step::Replace(slot, e) => updates.push((slot, e)),
            Step::Remove(slot) => removes.push(slot),
            Step::Same => {}
        }
    }
    if !removes.is_empty() {
        cx.doc.remove(&removes);
    }
    if !updates.is_empty() {
        cx.doc.update_many(updates, DRAWING_TAKE_STEP);
    }
    if !adds.is_empty()
        && let Err(refused) = cx.doc.add_many_with_uids(adds, DRAWING_TAKE_STEP)
    {
        return Stop::failed(error(codes::SLOTS_EXHAUSTED, refused.to_string(), None)).into();
    }
    CommandResult::Completed {
        output: DrawingTaken { revision: cx.doc.revision().to_string(), ..done },
        warnings: c.warnings,
    }
}

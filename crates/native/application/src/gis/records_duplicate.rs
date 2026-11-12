//! `cad.records.duplicate` v1 (docs/adr/0415): objects copied where they are with the records their
//! composite relations hold, as one undo step (“Kaydı çoğalt”). The shared cases are
//! `fixtures/commands/v1/cad.records.duplicate.json`.
//!
//! The checks, in order: the ids, their count, the expected revision, the objects, the locked layers
//! (every object on one: refused; some: left out with a warning). A child's field takes its parent's
//! copy's id; a field a form of the layer keeps unique, the layer keeps unique (docs/adr/0438 §3) or a unique index of
//! the layer holds (its key, docs/adr/0758 §3) is left empty in the copy; copies whose emptied key another record
//! holds (an index taking empty values as the same) are refused.

use std::collections::HashSet;

use kentos_commands::{
    CommandResult, CommandWarning, Entity, MAX_DUPLICATES, RecordCopy, RecordsDuplicate,
    RecordsDuplicatePlan, RecordsDuplicated, RelationDelete,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "Kaydı çoğalt";

/// One copy to make, in order: the object, its parent's place in the list and the field that names
/// it (a child), or none (an object given).
struct Copy {
    slot: Slot,
    parent: Option<(usize, String)>,
}

/// What the checks found: the copies in order, how many are children and the warnings.
struct Planned {
    copies: Vec<Copy>,
    children: usize,
    warnings: Vec<CommandWarning>,
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The children `slot` holds through its layer's composite relations, in the relations' and the
/// drawing's order: the child, the name of its field and whether its layer is locked.
fn kids(doc: &Document, slot: Slot) -> Vec<(Slot, String, bool)> {
    let (Some(e), Some(uid)) = (doc.get(slot), doc.uid(slot)) else {
        return Vec::new();
    };
    let uid = uid.to_string();
    let layer = &e.base().layer_id;
    let layers = doc.layers();
    let mut out = Vec::new();
    for r in &doc.settings().relations {
        if r.parent != *layer || r.on_delete != RelationDelete::Cascade {
            continue;
        }
        let name = layers
            .get(&r.child)
            .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
            .map_or_else(|| r.field.clone(), |f| f.name.clone());
        let locked = layers.is_locked(&r.child);
        for c in doc.by_layer(&r.child) {
            if c.base().attrs.get(&name) == Some(&uid) {
                out.push((Slot(c.base().id), name.clone(), locked));
            }
        }
    }
    out
}

fn below(doc: &Document, slot: Slot, seen: &mut HashSet<Slot>) {
    for (c, _, _) in kids(doc, slot) {
        if seen.insert(c) {
            below(doc, c, seen);
        }
    }
}

fn check(doc: &Document, input: &RecordsDuplicate) -> Result<Planned, Stop> {
    checks::uids(&input.uids, "Çoğaltılacak nesne yok.")?;
    if input.uids.len() > MAX_DUPLICATES {
        return Err(fail(
            codes::INVALID_COUNT,
            format!(
                "En çok {MAX_DUPLICATES} nesne birlikte çoğaltılır; {} verildi.",
                input.uids.len()
            ),
            "uids",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let found = checks::named(doc, &input.uids)?;
    let (open, locked) = checks::skip_locked(doc, found)?;
    let mut warnings: Vec<CommandWarning> = locked.into_iter().collect();
    let tops: Vec<Slot> = open.iter().map(|(_, slot, _, _)| *slot).collect();
    // An object that is a composite child of another one given is copied with it only.
    let mut under = HashSet::new();
    if input.children {
        for &s in &tops {
            below(doc, s, &mut under);
        }
    }
    let mut copies: Vec<Copy> = Vec::new();
    let mut done: HashSet<Slot> = HashSet::new();
    let (mut children, mut skipped) = (0, 0);
    fn follow(
        doc: &Document,
        slot: Slot,
        at: usize,
        copies: &mut Vec<Copy>,
        done: &mut HashSet<Slot>,
        children: &mut usize,
        skipped: &mut usize,
    ) {
        for (c, name, locked) in kids(doc, slot) {
            if done.contains(&c) {
                continue;
            }
            if locked {
                *skipped += 1;
                continue;
            }
            done.insert(c);
            copies.push(Copy {
                slot: c,
                parent: Some((at, name)),
            });
            *children += 1;
            let here = copies.len() - 1;
            follow(doc, c, here, copies, done, children, skipped);
        }
    }
    for s in tops {
        if done.contains(&s) || under.contains(&s) {
            continue;
        }
        done.insert(s);
        copies.push(Copy {
            slot: s,
            parent: None,
        });
        if input.children {
            let at = copies.len() - 1;
            follow(
                doc,
                s,
                at,
                &mut copies,
                &mut done,
                &mut children,
                &mut skipped,
            );
        }
    }
    if skipped > 0 {
        warnings.push(CommandWarning {
            code: codes::LAYER_LOCKED.into(),
            message: format!(
                "Kilitli katmandaki {skipped} alt kayıt çoğaltılmadı; katmanın kilidini Katmanlar panelinden açın."
            ),
            path: Some("uids".into()),
        });
    }
    // The unique fields the copies leave empty, each form's and field's once, in their names' order.
    let mut cleared: Vec<(String, String)> = Vec::new();
    for c in &copies {
        let Some(e) = doc.get(c.slot) else { continue };
        for (form, field) in unique_fields(doc, &e.base().layer_id) {
            if e.base().attrs.contains_key(&field)
                && !cleared.contains(&(form.clone(), field.clone()))
            {
                cleared.push((form, field));
            }
        }
    }
    cleared.sort();
    // The layers' own unique fields and keys the copies leave empty, once each, after the forms' (docs/adr/0758 §3).
    let mut keys: Vec<String> = Vec::new();
    for c in &copies {
        let Some(e) = doc.get(c.slot) else { continue };
        for (said, field) in key_fields(doc, &e.base().layer_id) {
            if e.base().attrs.contains_key(&field)
                && !cleared.iter().any(|(_, f)| *f == field)
                && !keys.contains(&said)
            {
                keys.push(said);
            }
        }
    }
    for (form, field) in cleared {
        warnings.push(CommandWarning {
            code: codes::UNIQUE_CLEARED.into(),
            message: format!(
                "“{form}” formunda tekil olan “{field}” alanı kopyalarda boş bırakıldı."
            ),
            path: Some("uids".into()),
        });
    }
    for said in keys {
        warnings.push(CommandWarning {
            code: codes::UNIQUE_CLEARED.into(),
            message: format!("{said}; kopyalarda boş bırakıldı."),
            path: Some("uids".into()),
        });
    }
    // The copies as they are written, against the layers' keys: an index taking empty values as the same refuses a
    // second empty one.
    let made: Vec<(String, std::collections::BTreeMap<String, String>)> = copies
        .iter()
        .filter_map(|c| doc.get(c.slot))
        .map(|e| (e.base().layer_id.clone(), copied_attrs(doc, e)))
        .collect();
    let writes: Vec<checks::UniqueWrite<'_>> = made
        .iter()
        .map(|(layer, attrs)| checks::UniqueWrite {
            slot: None,
            layer,
            before: None,
            after: attrs,
            at: "uids".to_owned(),
        })
        .collect();
    checks::unique_fit(doc, &writes)?;
    Ok(Planned {
        copies,
        children,
        warnings,
    })
}

/// The fields of `layer` a form keeps unique (its draft or its last release): the form's name and the
/// field's name.
fn unique_fields(doc: &Document, layer: &str) -> Vec<(String, String)> {
    let node = doc.layers().get(layer);
    let mut out = Vec::new();
    for form in kentos_commands::forms_of_layer(&doc.settings().forms, layer) {
        let bodies = std::iter::once(&form.draft).chain(form.releases.last().map(|r| &r.body));
        for body in bodies {
            for f in body.fields().into_iter().filter(|f| f.unique) {
                let name = node
                    .and_then(|n| n.fields.iter().find(|x| x.key() == f.field))
                    .map_or_else(|| f.field.clone(), |x| x.name.clone());
                if !out.contains(&(form.name.clone(), name.clone())) {
                    out.push((form.name.clone(), name));
                }
            }
        }
    }
    out
}

/// The fields of `layer` the layer itself keeps unique (docs/adr/0438 §3) and those its unique indexes hold (docs/adr/0758
/// §3): what keeps each, said, and the field's name.
fn key_fields(doc: &Document, layer: &str) -> Vec<(String, String)> {
    let Some(node) = doc.layers().get(layer) else {
        return Vec::new();
    };
    let mut out: Vec<(String, String)> = Vec::new();
    for f in node.fields.iter().filter(|f| f.unique) {
        out.push((format!("Katmanda tekil olan “{}” alanı", f.label()), f.name.clone()));
    }
    for x in node.indexes.iter().filter(|x| x.unique) {
        for k in &x.fields {
            if let Some(f) = node.fields.iter().find(|f| f.key() == k)
                && !out.iter().any(|(_, n)| *n == f.name)
            {
                out.push((
                    format!("Katmanın “{}” anahtarındaki “{}” alanı", x.name, f.label()),
                    f.name.clone(),
                ));
            }
        }
    }
    out
}

/// A copy's values: the object's, without what a form or the layer keeps unique.
fn copied_attrs(doc: &Document, e: &Entity) -> std::collections::BTreeMap<String, String> {
    let layer = &e.base().layer_id;
    let mut attrs = e.base().attrs.clone();
    for (_, field) in unique_fields(doc, layer) {
        attrs.remove(&field);
    }
    for (_, field) in key_fields(doc, layer) {
        attrs.remove(&field);
    }
    attrs
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RecordsDuplicate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: (),
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// How many copies execute would make now; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &RecordsDuplicate,
) -> CommandResult<RecordsDuplicatePlan> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: RecordsDuplicatePlan {
                count: u32::try_from(p.copies.len()).unwrap_or(u32::MAX),
                children: u32::try_from(p.children).unwrap_or(u32::MAX),
                revision: cx.doc.revision().to_string(),
            },
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the copies as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: RecordsDuplicate,
) -> CommandResult<RecordsDuplicated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_RECORDS_DUPLICATE,
        kentos_commands::CAD_RECORDS_DUPLICATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: RecordsDuplicate,
) -> CommandResult<RecordsDuplicated> {
    let planned = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    cx.doc.name_step(STEP);
    let mut out = RecordsDuplicated {
        copies: Vec::with_capacity(planned.copies.len()),
        revision: String::new(),
    };
    for c in &planned.copies {
        let (Some(original), Some(from)) = (cx.doc.get(c.slot), cx.doc.uid(c.slot)) else {
            continue;
        };
        let mut entity: Entity = original.clone();
        let attrs = copied_attrs(cx.doc, original);
        let base = entity.base_mut();
        base.id = 0;
        base.attrs = attrs;
        if let Some((at, field)) = &c.parent {
            let parent: &RecordCopy = &out.copies[*at];
            base.attrs.insert(field.clone(), parent.uid.clone());
        }
        match checks::add(cx.doc, entity) {
            Ok((id, uid, revision)) => {
                out.copies.push(RecordCopy {
                    from: from.to_string(),
                    uid,
                    id,
                });
                out.revision = revision;
            }
            Err(stop) => return stop.into(),
        }
    }
    CommandResult::Completed {
        output: out,
        warnings: planned.warnings,
    }
}

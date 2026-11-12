//! `cad.forms.submit` v1 (docs/adr/0401 §9): a record written through a form, one undo step
//! (“Form: <ad>”). The values go by the layer fields' keys; the form's rules and its fields' decide
//! whether they go in, the whole record or nothing. A new record (`defaults`) takes the form's defaults
//! after the values given; the calculations follow; a soft check's warning does not stop the write
//! (docs/adr/0402 §3, §6). The record's children go in the same step (docs/adr/0440 §2): each through
//! its relation, linked or unlinked, changed or made with the id the caller gave, through the child
//! layer's form when it has one; the record and its children together or nothing. The answer says the
//! records written, the snapshot a reviewer approves and the undo step (docs/adr/0440 §4, §5). The
//! shared cases are `fixtures/commands/v1/cad.forms.submit.json` and `cad.forms.submit.children.json`.

use std::collections::BTreeMap;

use kentos_commands::{
    ChildSubmitted, CommandResult, Entity, EntityBase, FormChild, FormItem, FormsSubmit,
    FormsSubmitted, RecordChange, RecordEntity,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_forms::Attrs;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::forms::{self, fail};

/// The object, its slot, the record afterwards, the release used, the undo step's name and the
/// form's warnings.
struct Checked {
    slot: Slot,
    entity: Entity,
    version: Option<u32>,
    label: String,
    warnings: Vec<kentos_commands::CommandWarning>,
    children: Vec<Child>,
    /// The fields the snapshot leaves out (the workflow's state and log).
    skip: Vec<String>,
    /// The record's attributes before.
    before_attrs: Attrs,
}

/// A child as the submission would write it.
struct Child {
    relation: String,
    uid: String,
    /// Its slot; none for a new record.
    slot: Option<Slot>,
    /// It before (empty for a new record) and after.
    before: Attrs,
    entity: Entity,
}

fn check(doc: &Document, input: &FormsSubmit) -> Result<Checked, Stop> {
    forms::uid_text(&input.uid, "uid")?;
    let variables = forms::variables_given(doc, input.moment.as_deref(), input.user.as_deref())?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let form = forms::form(doc.settings(), &input.form, "form")?;
    forms::writable(form)?;
    let (body, version) = forms::body(form, input.draft, "draft")?;
    // The screen's release against the form's now (docs/adr/0438 §6): a release published since is
    // not written past.
    if let Some(opened) = input.release
        && !input.draft
        && version != Some(opened)
    {
        return Err(Stop::conflict(checks::error(
            codes::FORM_RELEASE_CHANGED,
            format!(
                "“{}” formu {opened}. yayınla açıldı; şimdi {}. yayında. Formu yeniden açıp değerleri yeniden girin.",
                form.name,
                version.map_or_else(|| "yayınsız".to_owned(), |v| v.to_string())
            ),
            Some("release".to_owned()),
        )));
    }
    let prepared = forms::prepared(doc, form, body, &variables, "form")?;
    let (slot, e) = forms::record(doc, form, &input.uid, "uid")?;
    // The record against what the screen opened (docs/adr/0438 §7): changed since, it is not
    // written over.
    if let Some(opened) = &input.opened
        && *opened != e.base().attrs
    {
        let changed: Vec<String> = opened
            .keys()
            .chain(e.base().attrs.keys())
            .filter(|k| opened.get(*k) != e.base().attrs.get(*k))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        return Err(Stop::conflict(checks::error(
            codes::RECORD_CHANGED,
            format!(
                "Kayıt form açıldıktan sonra değişti ({}). Formu yeniden yükleyin ya da değerlerinizi yeniden kaydedin.",
                changed.join(", ")
            ),
            Some("opened".to_owned()),
        )));
    }
    if doc.layers().is_locked(&form.layer) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; kayıt değiştirilemez. Kilidi Katmanlar panelinden açın.",
                doc.layers()
                    .get(&form.layer)
                    .map_or(form.layer.as_str(), |n| n.name.as_str())
            ),
            "uid",
        ));
    }
    let mut object = forms::object(doc, slot, e);
    // The children this submission links or unlinks count with those already there (docs/adr/0444 §1).
    let parent_uid = doc.uid(slot).map(|u| u.to_string()).unwrap_or_default();
    for c in &input.children {
        let Some(r) = doc.settings().relations.iter().find(|r| r.id == c.relation) else {
            continue;
        };
        let linked = linked_to(doc, r, &c.uid, &parent_uid);
        let delta = match (c.unlink, linked) {
            (true, true) => -1,
            (false, false) => 1,
            _ => 0,
        };
        *object
            .added
            .entry(kentos_forms::added_key(&r.child, &r.field))
            .or_default() += delta;
    }
    let values: Vec<(String, String)> = input
        .values
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let uid = doc.uid(slot).unwrap_or_default();
    let (after, warnings) = prepared
        .apply(&e.base().attrs, &values, &object, input.defaults)
        .and_then(|(after, warnings)| {
            let taken =
                prepared.unique_issues(&after, &object, forms::others(doc, &form.layer, uid));
            if taken.is_empty() {
                Ok((after, warnings))
            } else {
                Err(taken)
            }
        })
        .map_err(|issues| {
            let issues: Vec<_> = issues
                .into_iter()
                .filter(|i| i.severity == kentos_commands::FormSeverity::Error)
                .collect();
            let first = &issues[0];
            let path = first
                .field
                .as_ref()
                .filter(|k| input.values.contains_key(*k))
                .map_or_else(|| "values".to_owned(), |k| format!("values.{k}"));
            let more = match issues.len() - 1 {
                0 => String::new(),
                n => format!(" ({n} sorun daha var.)"),
            };
            fail(
                codes::FORM_INVALID,
                format!("{}{more} Değeri düzeltip yeniden kaydedin.", first.message),
                &path,
            )
        })?;
    // A value the bound table always makes itself stays the table's (docs/adr/0754 §3).
    checks::made_fit(doc, &form.layer, Some(&e.base().attrs), &after, "values")?;
    let mut entity = e.clone();
    entity.base_mut().attrs = after;
    let uid_text = uid.to_string();
    let mut children = Vec::with_capacity(input.children.len());
    for (i, c) in input.children.iter().enumerate() {
        children.push(child(doc, &form.layer, &uid_text, i, c, input, &children)?);
        offered(doc, body, &uid_text, i, c, &children[i])?;
    }
    let skip = forms::snapshot_skip(doc, &form.layer, body)
        .into_iter()
        .map(str::to_owned)
        .collect();
    Ok(Checked {
        slot,
        entity,
        version,
        label: format!("Form: {}", form.name),
        warnings: forms::warnings_of(&warnings),
        children,
        skip,
        before_attrs: e.base().attrs.clone(),
    })
}

/// A child of the record `parent` (on `layer`) as the submission would write it (docs/adr/0440 §2).
fn child(
    doc: &Document,
    layer: &str,
    parent: &str,
    i: usize,
    c: &FormChild,
    input: &FormsSubmit,
    earlier: &[Child],
) -> Result<Child, Stop> {
    let at = |rest: &str| format!("children[{i}].{rest}");
    forms::uid_text(&c.uid, &at("uid"))?;
    if earlier.iter().any(|e| e.uid == c.uid) || c.uid == input.uid {
        return Err(fail(
            codes::INVALID_UID,
            format!(
                "“{}” kimliği gönderimde iki kez var; her alt kaydı bir kez verin.",
                c.uid
            ),
            &at("uid"),
        ));
    }
    let Some(r) = doc.settings().relations.iter().find(|r| r.id == c.relation) else {
        return Err(fail(
            codes::UNKNOWN_RELATION,
            format!(
                "“{}” kimlikli bir ilişki yok. Var olan bir ilişkinin kimliğini verin.",
                c.relation
            ),
            &at("relation"),
        ));
    };
    if r.parent != layer {
        return Err(fail(
            codes::RELATION_MISMATCH,
            format!(
                "“{}” ilişkisinin üst katmanı formun katmanı değil; alt kayıt bu ilişkiyle bu kayda bağlanamaz.",
                r.name
            ),
            &at("relation"),
        ));
    }
    let name_of = |id: &str| {
        doc.layers()
            .get(id)
            .map_or(id.to_owned(), |n| n.name.clone())
    };
    let Some(link) = doc
        .layers()
        .get(&r.child)
        .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
        .cloned()
    else {
        return Err(fail(
            codes::RELATION_FIELD_UNKNOWN,
            format!(
                "“{}” katmanında “{}” alanı yok.",
                name_of(&r.child),
                r.field
            ),
            &at("relation"),
        ));
    };
    let existing = Uuid::parse_str(&c.uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|slot| doc.get(slot).map(|e| (slot, e)));
    let (slot, base) = match existing {
        Some((slot, e)) => {
            if e.base().layer_id != r.child {
                return Err(fail(
                    codes::FORM_LAYER_MISMATCH,
                    format!(
                        "“{}” nesnesi “{}” katmanında değil; ilişkinin alt kaydı olamaz.",
                        c.uid,
                        name_of(&r.child)
                    ),
                    &at("uid"),
                ));
            }
            (Some(slot), e.clone())
        }
        None if c.unlink => {
            return Err(fail(
                codes::ENTITY_NOT_FOUND,
                format!("Bağı koparılacak “{}” nesnesi çizimde yok.", c.uid),
                &at("uid"),
            ));
        }
        None => (
            None,
            Entity::Record(RecordEntity {
                base: EntityBase {
                    look: Default::default(),
                    id: 0,
                    layer_id: r.child.clone(),
                    color: None,
                    attrs: BTreeMap::new(),
                    label: None,
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                },
            }),
        ),
    };
    if doc.layers().is_locked(&r.child) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; alt kayıt değiştirilemez. Kilidi Katmanlar panelinden açın.",
                name_of(&r.child)
            ),
            &at("uid"),
        ));
    }
    let before = base.base().attrs.clone();
    // The link: the record's id in the child's field, or taken off it.
    let mut attrs = before.clone();
    let held: Vec<String> = attrs
        .get(&link.name)
        .map(|v| {
            v.split(';')
                .map(|h| h.trim().to_owned())
                .filter(|h| !h.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let next: Vec<String> = if c.unlink {
        held.into_iter().filter(|h| h != parent).collect()
    } else if link.multiple {
        let mut all = held;
        if !all.iter().any(|h| h == parent) {
            all.push(parent.to_owned());
        }
        all
    } else {
        vec![parent.to_owned()]
    };
    if next.is_empty() {
        attrs.remove(&link.name);
    } else {
        attrs.insert(
            link.name.clone(),
            next.join(kentos_commands::MULTIPLE_JOIN),
        );
    }
    // The values: through the child layer's form when it has one, else by its fields' rules.
    let values: Vec<(String, String)> = c
        .values
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let invalid = |message: String, key: Option<&String>| {
        let path = key.map_or_else(|| at("values"), |k| at(&format!("values.{k}")));
        fail(
            codes::FORM_INVALID,
            format!("{message} Alt kaydın değerini düzeltip yeniden kaydedin."),
            &path,
        )
    };
    let form = doc
        .settings()
        .forms
        .iter()
        .find(|f| f.layer == r.child && f.release().is_some());
    let attrs = match form {
        Some(f) => {
            let variables =
                forms::variables_given(doc, input.moment.as_deref(), input.user.as_deref())?;
            let body = &f.release().map(|r| &r.body).unwrap_or(&f.draft);
            let prepared = forms::prepared(doc, f, body, &variables, &at("relation"))?;
            let object = match slot {
                Some(s) => forms::object(doc, s, &base),
                None => kentos_forms::Object::default(),
            };
            let (after, _) = prepared
                .apply(&attrs, &values, &object, slot.is_none())
                .map_err(|issues| {
                    let first = issues
                        .iter()
                        .find(|i| i.severity == kentos_commands::FormSeverity::Error)
                        .or(issues.first());
                    let message = first.map_or_else(String::new, |i| i.message.clone());
                    let key = first
                        .and_then(|i| i.field.as_ref())
                        .filter(|k| c.values.contains_key(*k));
                    invalid(message, key)
                })?;
            after
        }
        None => {
            let fields = forms::fields(doc, &r.child);
            let mut out = attrs;
            for (k, v) in &c.values {
                let Some(f) = fields.iter().find(|f| f.key() == k) else {
                    return Err(invalid(
                        format!("“{}” katmanında “{k}” alanı yok.", name_of(&r.child)),
                        Some(k),
                    ));
                };
                match kentos_commands::check_value(f, v) {
                    Ok(x) if x.is_empty() => {
                        out.remove(&f.name);
                    }
                    Ok(x) => {
                        out.insert(f.name.clone(), x);
                    }
                    Err(why) => return Err(invalid(why.message, Some(k))),
                }
            }
            out
        }
    };
    // A kept child's value its bound table always makes itself stays the table's (docs/adr/0754 §3).
    checks::made_fit(doc, &r.child, slot.map(|_| &before), &attrs, &at("values"))?;
    let mut entity = base;
    entity.base_mut().attrs = attrs;
    Ok(Child {
        relation: r.id.clone(),
        uid: c.uid.clone(),
        slot,
        before,
        entity,
    })
}

/// The child's actions against what the form's related items showing its relation offer
/// (docs/adr/0762 §3): one not offered refuses the submission; a relation no item shows is the
/// form's silence.
fn offered(
    doc: &Document,
    body: &kentos_commands::FormBody,
    parent: &str,
    i: usize,
    c: &FormChild,
    written: &Child,
) -> Result<(), Stop> {
    let Some(r) = doc.settings().relations.iter().find(|r| r.id == c.relation) else {
        return Ok(());
    };
    let Some(offer) = kentos_forms::relation_offer(body, &r.child, &r.field) else {
        return Ok(());
    };
    // A value it is given that differs from the one it holds, by the child layer's fields.
    let fields = forms::fields(doc, &r.child);
    let changes = written.slot.is_some()
        && c.values.iter().any(|(k, v)| {
            let name = fields
                .iter()
                .find(|f| f.key() == k)
                .map_or(k.as_str(), |f| f.name.as_str());
            written.before.get(name).map_or("", String::as_str) != v.as_str()
        });
    let asked = kentos_forms::child_actions(
        written.slot.is_some(),
        linked_to(doc, r, &c.uid, parent),
        c.unlink,
        changes,
    );
    let Some(missing) = asked.into_iter().find(|a| !offer.contains(a)) else {
        return Ok(());
    };
    let label = related_label(doc, body, &r.child, &r.field);
    let mut refusal = checks::error(
        codes::FORM_RELATED_ACTION,
        format!(
            "“{label}” alt kayıtlarında “{}” sunmuyor; formun tasarımında öğenin eylemlerine ekleyin.",
            missing.label()
        ),
        Some(format!("children[{i}]")),
    );
    if written.slot.is_some() {
        refusal.uids = vec![c.uid.clone()];
    }
    Err(Stop::failed(refusal))
}

/// The first related item's words for the child layer's field: its label, else the layer's name.
fn related_label(
    doc: &Document,
    body: &kentos_commands::FormBody,
    layer: &str,
    field: &str,
) -> String {
    fn find<'a>(items: &'a [FormItem], layer: &str, field: &str) -> Option<&'a FormItem> {
        items.iter().find_map(|i| match i {
            FormItem::Related {
                layer: l, field: f, ..
            } if l == layer && f == field => Some(i),
            FormItem::Tab { items, .. } | FormItem::Group { items, .. } => {
                find(items, layer, field)
            }
            _ => None,
        })
    }
    match find(&body.items, layer, field) {
        Some(FormItem::Related { label: Some(l), .. }) => l.clone(),
        _ => doc
            .layers()
            .get(layer)
            .map_or_else(|| layer.to_owned(), |n| n.name.clone()),
    }
}

/// The fields whose values differ between `a` and `b`, by name in order.
fn changed_fields(a: &Attrs, b: &Attrs) -> Vec<String> {
    a.keys()
        .chain(b.keys())
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The answer for `c`: in a plan the record and children as they would be, after a write as the
/// document holds them (its forms completed them).
fn answer(doc: &Document, c: &Checked, written: bool, uid: &str) -> FormsSubmitted {
    let read = |slot: Option<Slot>, uid: &str, planned: &Entity| -> Entity {
        if !written {
            return planned.clone();
        }
        slot.or_else(|| Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)))
            .and_then(|s| doc.get(s).cloned())
            .unwrap_or_else(|| planned.clone())
    };
    let record = read(Some(c.slot), uid, &c.entity);
    let before = if written {
        None
    } else {
        doc.get(c.slot).map(|e| e.base().attrs.clone())
    };
    let layer = record.base().layer_id.clone();
    let mut changes = Vec::new();
    let own_before = before.unwrap_or_else(|| c.before_attrs.clone());
    let own = changed_fields(&own_before, &record.base().attrs);
    let changed = !own.is_empty();
    if changed {
        changes.push(RecordChange {
            uid: uid.to_owned(),
            layer: layer.clone(),
            created: false,
            fields: own,
        });
    }
    let mut children = Vec::with_capacity(c.children.len());
    let mut kids: Vec<(String, String, Attrs)> = Vec::new();
    for ch in &c.children {
        let now = read(ch.slot, &ch.uid, &ch.entity);
        let fields = changed_fields(&ch.before, &now.base().attrs);
        let created = ch.slot.is_none();
        let moved = created || !fields.is_empty();
        if moved {
            changes.push(RecordChange {
                uid: ch.uid.clone(),
                layer: now.base().layer_id.clone(),
                created,
                fields,
            });
        }
        kids.push((
            ch.uid.clone(),
            now.base().layer_id.clone(),
            now.base().attrs.clone(),
        ));
        children.push(ChildSubmitted {
            uid: ch.uid.clone(),
            relation: ch.relation.clone(),
            created,
            attrs: now.base().attrs.clone(),
            changed: moved,
        });
    }
    let pending: Vec<forms::Pending<'_>> = kids
        .iter()
        .map(|(u, l, a)| (u.as_str(), l.as_str(), a))
        .collect();
    let skip: Vec<&str> = c.skip.iter().map(String::as_str).collect();
    let snapshot = forms::snapshot(
        doc,
        &layer,
        uid,
        &record.base().attrs,
        &record.base().attachments,
        &skip,
        &pending,
    );
    FormsSubmitted {
        uid: uid.to_owned(),
        version: c.version,
        attrs: record.base().attrs.clone(),
        changed,
        children,
        undo: (written && !changes.is_empty()).then(|| c.label.clone()),
        changes,
        snapshot,
        committed: None,
        revision: doc.revision().to_string(),
    }
}

/// The layers a submission writes: the record's and its children's, the record's first, each once.
fn written_layers(c: &Checked) -> Vec<String> {
    let mut out = vec![c.entity.base().layer_id.clone()];
    for ch in &c.children {
        let l = ch.entity.base().layer_id.clone();
        if !out.contains(&l) {
            out.push(l);
        }
    }
    out
}

/// The record and its children written into `doc`: one undo step, all or nothing.
fn apply(doc: &mut Document, c: &Checked) -> Result<bool, Stop> {
    let mut updates: Vec<(Slot, Entity)> = Vec::new();
    if doc.get(c.slot) != Some(&c.entity) {
        updates.push((c.slot, c.entity.clone()));
    }
    let mut made: Vec<(Uuid, Entity)> = Vec::new();
    for ch in &c.children {
        match ch.slot {
            Some(slot) if doc.get(slot) != Some(&ch.entity) => {
                updates.push((slot, ch.entity.clone()));
            }
            Some(_) => {}
            None => {
                if let Ok(u) = Uuid::parse_str(&ch.uid) {
                    made.push((u, ch.entity.clone()));
                }
            }
        }
    }
    if updates.is_empty() && made.is_empty() {
        return Ok(false);
    }
    let label = c.label.clone();
    doc.transact(&label, |doc| {
        doc.update_many(updates.clone(), &label);
        if !made.is_empty() {
            doc.add_many_with_uids(made.clone(), &label)?;
        }
        Ok::<(), kentos_domain::Refusal>(())
    })
    .map(|()| true)
    .map_err(|why| fail(codes::FORM_INVALID, why.0, "children"))
}

/// With `commit`: what writing the submission to the sources would do, found on a copy of the drawing the
/// submission is already written in (docs/adr/0532 §1). Any refusal of it is the submission's.
fn commit_plan(
    doc: &Document,
    input: &FormsSubmit,
    c: &Checked,
) -> Result<Option<(kentos_commands::SourcesSynced, Vec<kentos_commands::CommandWarning>)>, Stop> {
    if !input.commit {
        return Ok(None);
    }
    let mut copy = doc.clone();
    apply(&mut copy, c)?;
    crate::sources::commit(&mut copy, &written_layers(c), false).map(Some)
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsSubmit) -> CommandResult<()> {
    match check(cx.doc, input).and_then(|c| commit_plan(cx.doc, input, &c).map(|_| c)) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsSubmit) -> CommandResult<FormsSubmitted> {
    match check(cx.doc, input).and_then(|c| commit_plan(cx.doc, input, &c).map(|p| (c, p))) {
        Ok((c, planned)) => {
            let mut output = answer(cx.doc, &c, false, &input.uid);
            let mut warnings = c.warnings.clone();
            if let Some((mut synced, more)) = planned {
                // The plan was made on a copy the submission is written in: it says the revision it is made against.
                synced.revision = output.revision.clone();
                output.committed = Some(synced);
                warnings.extend(more);
            }
            CommandResult::Completed { output, warnings }
        }
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the record.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FormsSubmit) -> CommandResult<FormsSubmitted> {
    // A plan through the protocol runs this on a reading copy: it writes no file then (docs/adr/0532 §4).
    let dry = cx.dry();
    crate::call::recorded(
        cx,
        kentos_commands::CAD_FORMS_SUBMIT,
        kentos_commands::CAD_FORMS_SUBMIT_VERSION,
        input,
        move |cx, input| write_execute(cx, input, dry),
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: FormsSubmit,
    dry: bool,
) -> CommandResult<FormsSubmitted> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    // The sources first on a copy, so a refusal of theirs is found before anything is written (docs/adr/0532 §1).
    if let Err(stop) = commit_plan(cx.doc, &input, &c) {
        return stop.into();
    }
    let wrote = match apply(cx.doc, &c) {
        Ok(wrote) => wrote,
        Err(stop) => return stop.into(),
    };
    let mut warnings = c.warnings.clone();
    let mut committed = None;
    if input.commit {
        match crate::sources::commit(cx.doc, &written_layers(&c), !dry) {
            Ok((synced, more)) => {
                committed = Some(synced);
                warnings.extend(more);
            }
            Err(stop) => {
                // The sources changed between the copy and now: the drawing goes back to before the step.
                if wrote {
                    cx.doc.undo();
                }
                return stop.into();
            }
        }
    }
    let mut output = answer(cx.doc, &c, true, &input.uid);
    output.committed = committed;
    CommandResult::Completed { output, warnings }
}

/// Whether the object `child` already refers to `parent` through `r`'s field.
fn linked_to(
    doc: &Document,
    r: &kentos_commands::ProjectRelation,
    child: &str,
    parent: &str,
) -> bool {
    let Some(slot) = kentos_domain::Uuid::parse_str(child)
        .ok()
        .and_then(|u| doc.slot_of(u))
    else {
        return false;
    };
    let Some(e) = doc.get(slot).filter(|e| e.base().layer_id == r.child) else {
        return false;
    };
    let name = doc
        .layers()
        .get(&r.child)
        .and_then(|n| {
            n.fields
                .iter()
                .find(|f| f.key() == r.field)
                .map(|f| f.name.clone())
        })
        .unwrap_or_else(|| r.field.clone());
    e.base()
        .attrs
        .get(&name)
        .is_some_and(|v| v.split(';').any(|h| h.trim() == parent))
}

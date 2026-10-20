//! `cad.fields.edit` v1 (docs/adr/0325 §1–§3): a field put into a text in place of some of its letters, taken out
//! (its letters stay as text), formatted, or the fields written again (Alanları yenile), each as one undo step. The
//! value is the document's rule (`kentos_domain::text_fields`); the document keeps the fields with their sources at
//! every later step. The web is paused (docs/adr/0228): the desktop's handler alone, passing
//! `fixtures/commands/v1/cad.fields.edit.json`.

use std::collections::HashSet;

use kentos_commands::{
    CommandResult, CommandWarning, Entity, FieldEdit, FieldSource, FieldsEdit, FieldsEdited,
    TextEntity, TextField, TextsPlan, text_fields_problem,
};
use kentos_domain::text_fields::replace_letters;
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
pub use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The undo step's name: the edit's.
fn label(e: &FieldEdit) -> &'static str {
    match e {
        FieldEdit::Insert { .. } => "Alan ekle",
        FieldEdit::Remove { .. } => "Alanı kaldır",
        FieldEdit::Format { .. } => "Alanın biçimi",
        FieldEdit::Update { .. } => "Alanları yenile",
    }
}

/// The text `uid` names, unlocked: its slot, id and itself.
pub(crate) fn text_at(
    doc: &Document,
    uid: &str,
    path: &str,
    unlocked: bool,
) -> Result<(Slot, Uuid, TextEntity), Stop> {
    if !is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
            ),
            path,
        ));
    }
    let found = Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u).map(|s| (s, u)));
    let Some((slot, u)) = found else {
        return Err(fail(
            codes::ENTITY_NOT_FOUND,
            format!(
                "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
            ),
            path,
        ));
    };
    let Some(Entity::Text(t)) = doc.get(slot) else {
        return Err(fail(
            codes::NOT_A_TEXT,
            "Verilen nesne bir yazı değil; bu komut yalnız yazıları değiştirir.".into(),
            path,
        ));
    };
    if unlocked && doc.layers().is_locked(&t.base.layer_id) {
        let layers = doc.layers();
        let name = layers
            .get(&t.base.layer_id)
            .map_or(t.base.layer_id.as_str(), |n| n.name.as_str());
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli; üzerindeki yazı değişmez. Kilidi Katmanlar panelinden açın."
            ),
            path,
        ));
    }
    Ok((slot, u, t.clone()))
}

/// A `field_broken` warning for each broken field of `t`.
pub(crate) fn broken_warnings(t: &TextEntity, out: &mut Vec<CommandWarning>) {
    for (i, f) in t.paragraph.fields.iter().enumerate() {
        if let Some(b) = f.broken {
            out.push(CommandWarning {
                code: codes::FIELD_BROKEN.into(),
                message: format!(
                    "“{}” yazısının {}. alanı okunamadı: {}; harfleri ####.",
                    t.text,
                    i + 1,
                    b.words()
                ),
                path: Some("edit".into()),
            });
        }
    }
}

/// What the checks give: each text changed, as it would be, and the warnings.
struct Checked {
    changed: Vec<(Slot, Uuid, TextEntity)>,
    warnings: Vec<CommandWarning>,
}

/// `t` (persistent id `uid`, its fields as they would be) with its fields written again in `doc`: a field reading `t`
/// itself, or a text that reads `t` back, is broken (`cycle`).
pub(crate) fn written(doc: &Document, uid: Uuid, t: &TextEntity) -> TextEntity {
    let mut ring: HashSet<Uuid> = HashSet::from([uid]);
    for f in &t.paragraph.fields {
        if let FieldSource::Object {
            uid: of,
            value: kentos_commands::ObjectValue::Text,
            ..
        } = &f.source
        {
            let of = Uuid::from_bytes(of.0);
            if doc.field_reaches(of, uid) {
                ring.insert(of);
            }
        }
    }
    let letters = |u: Uuid| match doc.slot_of(u).and_then(|s| doc.get(s)) {
        Some(Entity::Text(other)) => Some(other.text.clone()),
        _ => None,
    };
    doc.fields_written(t, &ring, &letters)
}

fn check(doc: &Document, input: &FieldsEdit) -> Result<Checked, Stop> {
    let mut changed = Vec::new();
    match &input.edit {
        FieldEdit::Insert {
            uid,
            start,
            end,
            source,
            format,
        } => {
            let (slot, u, t) = text_at(doc, uid, "edit.uid", true)?;
            let letters = t.text.chars().count() as u32;
            if start > end || *end > letters {
                return Err(fail(
                    codes::INVALID_TEXT_FIELD,
                    format!(
                        "Alanın yeri {start}–{end}: başı sonundan sonra olamaz, sonu yazının harf sayısını ({letters}) aşamaz."
                    ),
                    "edit.start",
                ));
            }
            if let FieldSource::Object { uid: of, .. } = source {
                let of = Uuid::from_bytes(of.0);
                if doc.slot_of(of).is_none() {
                    return Err(fail(
                        codes::ENTITY_NOT_FOUND,
                        format!(
                            "Alanın kaynağı “{of}” çizimde yok. Var olan bir nesnenin kimliğini verin."
                        ),
                        "edit.source.uid",
                    ));
                }
            }
            let touches = |f: &TextField| {
                if start == end {
                    f.start < *start && *start < f.end
                } else {
                    f.start < *end && *start < f.end
                }
            };
            if let Some(i) = t.paragraph.fields.iter().position(touches) {
                return Err(fail(
                    codes::INVALID_TEXT_FIELD,
                    format!(
                        "Alanın yeri {start}–{end} yazının {}. alanına değiyor; alanlar üst üste gelmez.",
                        i + 1
                    ),
                    "edit.start",
                ));
            }
            // The field over its letters (a placeholder when it is put in between two letters), then its value.
            let mut next = if start == end {
                replace_letters(&t, *start, *end, "#", None)
            } else {
                t.clone()
            };
            let at = next
                .paragraph
                .fields
                .iter()
                .position(|f| f.start >= *start)
                .unwrap_or(next.paragraph.fields.len());
            next.paragraph.fields.insert(
                at,
                TextField {
                    start: *start,
                    end: if start == end { start + 1 } else { *end },
                    source: source.clone(),
                    format: format.clone(),
                    broken: None,
                },
            );
            if let Some(words) = text_fields_problem(&next.paragraph.fields, &next.text)
                .filter(|_| next.paragraph.fields.iter().all(|f| f.broken.is_none()))
            {
                return Err(fail(codes::INVALID_TEXT_FIELD, words, "edit"));
            }
            changed.push((slot, u, written(doc, u, &next)));
        }
        FieldEdit::Remove { uid, index } | FieldEdit::Format { uid, index, .. } => {
            let (slot, u, t) = text_at(doc, uid, "edit.uid", true)?;
            if *index as usize >= t.paragraph.fields.len() {
                return Err(fail(
                    codes::FIELD_NOT_FOUND,
                    format!(
                        "Yazının {} alanı var; {index}. sıradaki yok (sıra 0'dan başlar).",
                        t.paragraph.fields.len()
                    ),
                    "edit.index",
                ));
            }
            let mut next = t.clone();
            match &input.edit {
                FieldEdit::Remove { .. } => {
                    next.paragraph.fields.remove(*index as usize);
                }
                FieldEdit::Format { format, .. } => {
                    if let Some(words) = format
                        .as_ref()
                        .and_then(kentos_commands::FieldFormat::problem)
                    {
                        return Err(fail(codes::INVALID_TEXT_FIELD, words, "edit.format"));
                    }
                    next.paragraph.fields[*index as usize].format = format.clone();
                    if let Some(words) = text_fields_problem(&next.paragraph.fields, &next.text)
                        .filter(|_| next.paragraph.fields.iter().all(|f| f.broken.is_none()))
                    {
                        return Err(fail(codes::INVALID_TEXT_FIELD, words, "edit.format"));
                    }
                    next = written(doc, u, &next);
                }
                _ => {}
            }
            changed.push((slot, u, next));
        }
        FieldEdit::Update { uids } => {
            let mut seen = HashSet::new();
            let texts: Vec<(Slot, Uuid, TextEntity)> = if uids.is_empty() {
                doc.entities()
                    .filter_map(|e| match e {
                        Entity::Text(t) if !t.paragraph.fields.is_empty() => {
                            let slot = Slot(t.base.id);
                            Some((slot, doc.uid(slot)?, t.clone()))
                        }
                        _ => None,
                    })
                    .filter(|(_, _, t)| !doc.layers().is_locked(&t.base.layer_id))
                    .collect()
            } else {
                let mut out = Vec::new();
                for (i, u) in uids.iter().enumerate() {
                    if seen.insert(u.as_str()) {
                        out.push(text_at(doc, u, &format!("edit.uids[{i}]"), true)?);
                    }
                }
                out
            };
            for (slot, u, t) in texts {
                changed.push((slot, u, written(doc, u, &t)));
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut warnings = Vec::new();
    for (_, _, t) in &changed {
        broken_warnings(t, &mut warnings);
    }
    // A text that would not change is not counted.
    changed.retain(|(slot, _, t)| doc.get(*slot) != Some(&Entity::Text(t.clone())));
    Ok(Checked { changed, warnings })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &FieldsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &FieldsEdit) -> CommandResult<TextsPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document (a window's preview).
pub fn plan_in(doc: &Document, input: &FieldsEdit) -> CommandResult<TextsPlan> {
    match check(doc, input) {
        Ok(c) => CommandResult::Completed {
            output: TextsPlan {
                texts: c
                    .changed
                    .into_iter()
                    .map(|(_, _, t)| Entity::Text(t))
                    .collect(),
                removed: Vec::new(),
                revision: doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: FieldsEdit) -> CommandResult<FieldsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_FIELDS_EDIT,
        kentos_commands::CAD_FIELDS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: FieldsEdit) -> CommandResult<FieldsEdited> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let changed = c.changed.iter().map(|(_, u, _)| u.to_string()).collect();
    if !c.changed.is_empty() {
        doc.update_many(
            c.changed
                .into_iter()
                .map(|(s, _, t)| (s, Entity::Text(t)))
                .collect(),
            label(&input.edit),
        );
    }
    CommandResult::Completed {
        output: FieldsEdited {
            changed,
            revision: doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}

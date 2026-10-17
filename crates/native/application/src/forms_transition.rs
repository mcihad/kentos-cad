//! `cad.forms.transition` v1 (İş akışı adımı, docs/adr/0428): a record taken a step of its form's
//! workflow, one undo step (“Form: <adım>”). The step must be one the workflow takes from the record's
//! state; the values given go in by the form's rules (on a locked record only the step's reason); the
//! state field takes the step's state; the record is completed again; a step that asks for a valid
//! record refuses one with the form's errors, and one naming a reason refuses it empty. The shared
//! cases are `fixtures/commands/v1/cad.forms.transition.json`.

use kentos_contracts::{
    CommandResult, Entity, FormSeverity, FormsTransition, FormsTransitioned, check_value,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::forms::{self, fail};

struct Checked {
    snapshot: String,
    slot: Slot,
    entity: Entity,
    from: String,
    to: String,
    version: Option<u32>,
    label: String,
}

/// `form_invalid` with the first of `issues`' errors, at the value that gave it.
fn invalid(input: &FormsTransition, issues: &[kentos_contracts::FormIssue]) -> Stop {
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == FormSeverity::Error)
        .collect();
    let first = errors[0];
    let path = first
        .field
        .as_ref()
        .filter(|k| input.values.contains_key(*k))
        .map_or_else(|| "step".to_owned(), |k| format!("values.{k}"));
    let more = match errors.len() - 1 {
        0 => String::new(),
        n => format!(" ({n} sorun daha var.)"),
    };
    fail(
        codes::FORM_INVALID,
        format!(
            "“{}” adımı atılamadı: {}{more} Kaydı düzeltip yeniden deneyin.",
            input.step, first.message
        ),
        &path,
    )
}

fn check(doc: &Document, input: &FormsTransition) -> Result<Checked, Stop> {
    forms::uid_text(&input.uid, "uid")?;
    let variables = forms::variables_given(doc, input.moment.as_deref(), input.user.as_deref())?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let form = forms::form(doc.settings(), &input.form, "form")?;
    forms::writable(form)?;
    let (body, version) = forms::body(form, input.draft, "draft")?;
    let prepared = forms::prepared(doc, form, body, &variables, "form")?;
    let Some(workflow) = prepared.workflow().cloned() else {
        return Err(fail(
            codes::NO_WORKFLOW,
            format!(
                "“{}” formunun iş akışı yok; tasarımcının İş akışı penceresinden tanımlayın.",
                form.name
            ),
            "step",
        ));
    };
    let (slot, e) = forms::record(doc, form, &input.uid, "uid")?;
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
    let Some(step) = workflow.steps.iter().find(|s| s.label == input.step) else {
        return Err(fail(
            codes::UNKNOWN_STEP,
            format!(
                "“{}” formunun iş akışında “{}” adımı yok.",
                form.name, input.step
            ),
            "step",
        ));
    };
    let fields = forms::fields(doc, &form.layer);
    // The preflight checked the state field; a drawing changed under it is said so.
    let Some(state_field) = fields.iter().find(|f| f.key() == workflow.field) else {
        return Err(fail(
            codes::FORM_PREFLIGHT,
            format!(
                "“{}” formu kullanılamıyor: iş akışının durum alanı katmanda yok. Formu tasarımcıda düzeltin.",
                form.name
            ),
            "form",
        ));
    };
    let words_state = |code: &str| {
        state_field
            .values
            .iter()
            .flatten()
            .find(|c| c.code == code)
            .map_or_else(|| code.to_owned(), |c| c.label.clone())
    };
    let before = &e.base().attrs;
    let from = prepared.state_of(before).unwrap_or_default();
    if !(step.from.is_empty() || step.from.contains(&from)) {
        return Err(fail(
            codes::STEP_NOT_ALLOWED,
            format!(
                "“{}” adımı “{}” durumundaki kayıttan atılmaz.",
                step.label,
                words_state(&from)
            ),
            "step",
        ));
    }
    if input.values.contains_key(state_field.key()) {
        return Err(fail(
            codes::FORM_INVALID,
            format!(
                "“{}” alanı iş akışının adımlarıyla değişir; değerlerden çıkarın.",
                state_field.label()
            ),
            &format!("values.{}", state_field.key()),
        ));
    }
    let object = forms::object(doc, slot, e);
    let reason = step
        .reason
        .as_ref()
        .and_then(|k| fields.iter().find(|f| f.key() == k));
    let log = workflow
        .log
        .as_ref()
        .and_then(|k| fields.iter().find(|f| f.key() == k));
    if let Some(l) = log
        && input.values.contains_key(l.key())
    {
        return Err(fail(
            codes::FORM_INVALID,
            format!(
                "“{}” alanını iş akışının adımları yazar; değerlerden çıkarın.",
                l.label()
            ),
            &format!("values.{}", l.key()),
        ));
    }
    // The step's notes on the record's fields (docs/adr/0442 §4): the form's own fields, each a short text.
    let notes_field = workflow
        .notes
        .as_ref()
        .and_then(|k| fields.iter().find(|f| f.key() == k));
    if let Some(n) = notes_field
        && input.values.contains_key(n.key())
    {
        return Err(fail(
            codes::FORM_INVALID,
            format!(
                "“{}” alanını iş akışının adımları yazar; notları `notes` ile verin.",
                n.label()
            ),
            &format!("values.{}", n.key()),
        ));
    }
    if !input.notes.is_empty() && notes_field.is_none() {
        return Err(fail(
            codes::FORM_INVALID,
            format!(
                "“{}” formunun iş akışında not alanı yok; alanlara not yazılamaz.",
                form.name
            ),
            "notes",
        ));
    }
    let bound = forms::bound_keys(body);
    for (k, note) in &input.notes {
        if !bound.iter().any(|b| b == k) {
            return Err(fail(
                codes::FORM_INVALID,
                format!("“{k}” formun bir alanı değil; not yalnız formdaki alanlara yazılır."),
                &format!("notes.{k}"),
            ));
        }
        if note.trim().is_empty()
            || note.chars().count() > kentos_contracts::MOST_FORM_TEXT
            || note.chars().any(|c| c.is_control() && c != '\n')
        {
            return Err(fail(
                codes::FORM_INVALID,
                format!(
                    "Not boş olamaz, en çok {} karakterdir ve denetim karakteri içermez.",
                    kentos_contracts::MOST_FORM_TEXT
                ),
                &format!("notes.{k}"),
            ));
        }
    }
    let mut after = if prepared.locked_by_state(before) {
        // A locked record takes only the step's reason.
        if let Some(k) = input
            .values
            .keys()
            .find(|k| reason.is_none_or(|r| r.key() != k.as_str()))
        {
            return Err(fail(
                codes::RECORD_LOCKED,
                format!(
                    "“{}” durumundaki kayıt değiştirilemez; bu adımla yalnız gerekçe yazılır.",
                    words_state(&from)
                ),
                &format!("values.{k}"),
            ));
        }
        let mut a = before.clone();
        if let Some(r) = reason
            && let Some(text) = input.values.get(r.key())
        {
            let v = check_value(r, text).map_err(|why| {
                fail(
                    codes::FORM_INVALID,
                    why.message,
                    &format!("values.{}", r.key()),
                )
            })?;
            if v.is_empty() {
                a.remove(&r.name);
            } else {
                a.insert(r.name.clone(), v);
            }
        }
        a
    } else {
        let values: Vec<(String, String)> = input
            .values
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let (a, issues) = prepared.assess(before, &values, &object, false);
        // The values given must be right; the record's other problems stop only a step that asks for
        // a valid record (below).
        let wrong: Vec<_> = issues
            .into_iter()
            .filter(|i| {
                i.severity == FormSeverity::Error
                    && i.field
                        .as_ref()
                        .is_some_and(|k| input.values.contains_key(k))
            })
            .collect();
        if !wrong.is_empty() {
            return Err(invalid(input, &wrong));
        }
        a
    };
    after.insert(state_field.name.clone(), step.to.clone());
    let (after, worked) = prepared.complete(&after, &object, false);
    if step.valid {
        let mut issues = worked;
        // A step that asks the record valid asks the checks kept for it too (docs/adr/0438 §2).
        issues.extend(prepared.validate_at(&after, &object, kentos_forms::Moment::Submit));
        if issues.iter().any(|i| i.severity == FormSeverity::Error) {
            return Err(invalid(input, &issues));
        }
    }
    if let Some(r) = reason
        && after.get(&r.name).is_none_or(|v| v.trim().is_empty())
    {
        return Err(fail(
            codes::REASON_REQUIRED,
            format!(
                "“{}” adımının gerekçesi (“{}”) yazılmalı.",
                step.label,
                r.label()
            ),
            &format!("values.{}", r.key()),
        ));
    }
    // The snapshot the step was taken on (docs/adr/0440 §4): the record with its children and
    // attachments, its state and log aside; what a reviewer approves.
    let skip = forms::snapshot_skip(doc, &form.layer, body);
    let snapshot = forms::snapshot(
        doc,
        &form.layer,
        &input.uid,
        &after,
        &e.base().attachments,
        &skip,
        &[],
    );
    // The step's line in the log (docs/adr/0429): the oldest lines go when the field's length is full.
    let mut after = after;
    if let Some(l) = log {
        let mut words: Vec<String> = Vec::new();
        words.extend(input.moment.clone());
        words.extend(input.user.clone());
        words.push(format!(
            "{} ({} → {})",
            step.label,
            words_state(&from),
            words_state(&step.to)
        ));
        let mut line = words.join(" · ");
        if let Some(text) = reason
            .and_then(|r| after.get(&r.name))
            .filter(|t| !t.trim().is_empty())
        {
            line.push_str(&format!(": {}", text.trim()));
        }
        if !input.notes.is_empty() {
            line.push_str(&format!(" · {} alan notu", input.notes.len()));
        }
        line.push_str(&format!(" · #{snapshot}"));
        let mut lines: Vec<String> = after
            .get(&l.name)
            .map(|t| t.lines().map(str::to_owned).collect())
            .unwrap_or_default();
        lines.push(line);
        if let Some(most) = l.length.map(|n| n as usize) {
            while lines.len() > 1 && lines.join("\n").chars().count() > most {
                lines.remove(0);
            }
            if let Some(last) = lines.last_mut()
                && last.chars().count() > most
            {
                *last = last.chars().take(most).collect();
            }
        }
        after.insert(l.name.clone(), lines.join("\n"));
    }
    // The latest step's notes, canonical JSON; a step without empties them.
    if let Some(n) = notes_field {
        if input.notes.is_empty() {
            after.remove(&n.name);
        } else {
            let trimmed: std::collections::BTreeMap<&str, &str> = input
                .notes
                .iter()
                .map(|(k, v)| (k.as_str(), v.trim()))
                .collect();
            let text = serde_json::to_string(&trimmed).unwrap_or_default();
            if n.length
                .is_some_and(|most| text.chars().count() > most as usize)
            {
                return Err(fail(
                    codes::FORM_INVALID,
                    format!(
                        "Notlar “{}” alanına sığmıyor ({} karakter); notları kısaltın ya da alanın uzunluğunu artırın.",
                        n.label(),
                        text.chars().count()
                    ),
                    "notes",
                ));
            }
            after.insert(n.name.clone(), text);
        }
    }
    let mut entity = e.clone();
    entity.base_mut().attrs = after;
    Ok(Checked {
        slot,
        entity,
        from,
        to: step.to.clone(),
        version,
        label: format!("Form: {}", step.label),
        snapshot,
    })
}

fn answer(doc: &Document, c: &Checked, uid: &str) -> FormsTransitioned {
    FormsTransitioned {
        uid: uid.to_owned(),
        from: c.from.clone(),
        to: c.to.clone(),
        attrs: c.entity.base().attrs.clone(),
        version: c.version,
        snapshot: c.snapshot.clone(),
        revision: doc.revision().to_string(),
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsTransition) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; nothing is written.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &FormsTransition,
) -> CommandResult<FormsTransitioned> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: answer(cx.doc, &c, &input.uid),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and takes the step as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: FormsTransition,
) -> CommandResult<FormsTransitioned> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_FORMS_TRANSITION,
        kentos_contracts::CAD_FORMS_TRANSITION_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: FormsTransition,
) -> CommandResult<FormsTransitioned> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    if cx.doc.get(c.slot) != Some(&c.entity) {
        cx.doc
            .update_many(vec![(c.slot, c.entity.clone())], &c.label);
    }
    CommandResult::Completed {
        output: answer(cx.doc, &c, &input.uid),
        warnings: Vec::new(),
    }
}

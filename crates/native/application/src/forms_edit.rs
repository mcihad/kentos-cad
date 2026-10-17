//! `cad.forms.edit` v1 (docs/adr/0401 §9): the project's forms. A project setting
//! (`Document::set_settings`: an edit, no undo step), as the named views are; the designer keeps its
//! own undo. The shared cases are `fixtures/commands/v1/cad.forms.edit.json`.

use kentos_contracts::{
    CommandResult, CommandWarning, FormBody, FormIssue, FormRelease, FormsEdit, FormsEdited,
    FormsOp, LayerNodeType, MOST_FORM_RELEASES, MOST_FORM_TEXT, MOST_FORMS, ProjectForm,
    ProjectSettings, body_digest, body_problem, form_id_ok, form_name_ok,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::forms::{self, fail};

/// The input's own checks: what needs no document.
fn check_input(input: &FormsEdit) -> Result<(), Stop> {
    let id = match &input.edit {
        FormsOp::Create { id, .. } => id.as_deref(),
        FormsOp::Update { id, .. }
        | FormsOp::Rename { id, .. }
        | FormsOp::Remove { id }
        | FormsOp::Publish { id, .. }
        | FormsOp::Enforce { id, .. }
        | FormsOp::Withdraw { id, .. }
        | FormsOp::Comment { id, .. }
        | FormsOp::Resolve { id, .. } => Some(id.as_str()),
    };
    if let Some(id) = id
        && !form_id_ok(id)
    {
        return Err(fail(
            codes::INVALID_FORM_ID,
            format!("“{id}” bir form kimliği olamaz: 1–64 harf, rakam, -, _ ya da . olmalı."),
            "edit.id",
        ));
    }
    if let FormsOp::Create { name, .. } | FormsOp::Rename { name, .. } = &input.edit
        && !form_name_ok(name.trim())
    {
        return Err(fail(
            codes::INVALID_FORM_NAME,
            "Form adı boş olamaz, en çok 64 karakterdir ve denetim karakteri içermez. Başka bir ad yazın.".into(),
            "edit.name",
        ));
    }
    // A comment's words (docs/adr/0441 §5).
    if let FormsOp::Comment {
        text, author, at, ..
    } = &input.edit
    {
        if text.trim().is_empty()
            || text.chars().count() > MOST_FORM_TEXT
            || text.chars().any(|c| c.is_control() && c != '\n')
        {
            return Err(fail(
                codes::INVALID_FORM,
                format!(
                    "Yorum boş olamaz, en çok {MOST_FORM_TEXT} karakterdir ve denetim karakteri içermez."
                ),
                "edit.text",
            ));
        }
        for (v, path) in [(author, "edit.author"), (at, "edit.at")] {
            if v.as_deref().is_some_and(|v| !form_name_ok(v)) {
                return Err(fail(
                    codes::INVALID_FORM,
                    "Yorumun yazarı ve anı boş olamaz, en çok 64 karakterdir.".into(),
                    path,
                ));
            }
        }
    }
    let body = match &input.edit {
        FormsOp::Create { body, .. } => body.as_ref(),
        FormsOp::Update { body, .. } => Some(body),
        _ => None,
    };
    if let Some(problem) = body.and_then(body_problem) {
        return Err(fail(
            codes::INVALID_FORM,
            format!("Formun yapısı kurallarına uymuyor: {problem}"),
            "edit.body",
        ));
    }
    if let FormsOp::Publish {
        note: Some(note), ..
    } = &input.edit
        && (note.chars().count() > MOST_FORM_TEXT
            || note.chars().any(|c| c.is_control() && c != '\n'))
    {
        return Err(fail(
            codes::INVALID_FORM,
            "Yayın notu en çok 1000 karakterdir ve denetim karakteri içermez.".into(),
            "edit.note",
        ));
    }
    Ok(())
}

fn unknown(id: &str) -> Stop {
    fail(
        codes::UNKNOWN_FORM,
        format!("“{id}” kimlikli bir form yok. Formlar penceresinden var olan bir formu seçin."),
        "edit.id",
    )
}

/// The settings afterwards, the form kept, the warnings and the records a publish breaks.
type Next = (
    ProjectSettings,
    Option<ProjectForm>,
    Vec<CommandWarning>,
    Option<kentos_contracts::FormBreaking>,
);

fn next(doc: &Document, input: &FormsEdit) -> Result<Next, Stop> {
    let mut s = doc.settings().clone();
    let mut warnings = Vec::new();
    let mut broken = None;
    let at = |s: &ProjectSettings, id: &str| s.forms.iter().position(|f| f.id == id);
    let at_of = at;
    let kept = match &input.edit {
        FormsOp::Create {
            id,
            name,
            layer,
            body,
        } => {
            let taken: Vec<String> = s.forms.iter().map(|f| f.id.clone()).collect();
            let id = match id {
                Some(id) => {
                    if taken.contains(id) {
                        return Err(fail(
                            codes::FORM_EXISTS,
                            format!("“{id}” kimlikli başka bir form var. Başka bir kimlik verin."),
                            "edit.id",
                        ));
                    }
                    id.clone()
                }
                None => {
                    let stem = if name.chars().any(char::is_alphanumeric) {
                        name.as_str()
                    } else {
                        "form"
                    };
                    kentos_forms::item_id_of(stem, &taken)
                }
            };
            if s.forms.len() >= MOST_FORMS {
                return Err(fail(
                    codes::TOO_MANY_FORMS,
                    "Projede en çok 100 form olur. Kullanmadığınız birini silin.".into(),
                    "edit",
                ));
            }
            let Some(node) = doc.layers().get(layer) else {
                return Err(fail(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "“{layer}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    "edit.layer",
                ));
            };
            if node.kind != LayerNodeType::Layer {
                return Err(fail(
                    codes::NOT_A_LAYER,
                    format!(
                        "“{}” bir katman grubu; form yalnız katmanın olur. Grubun içinden bir katman seçin.",
                        node.name
                    ),
                    "edit.layer",
                ));
            }
            let mut draft = match body {
                Some(b) => b.clone(),
                None => kentos_forms::generate(&node.fields),
            };
            draft.revision = 1;
            let form = ProjectForm {
                withdrawn: false,
                id,
                name: name.trim().to_owned(),
                layer: layer.clone(),
                draft,
                releases: Vec::new(),
                enforce: false,
                comments: Vec::new(),
            };
            s.forms.push(form.clone());
            Some(form)
        }
        FormsOp::Update { id, body } => {
            let i = at(&s, id).ok_or_else(|| unknown(id))?;
            let form = &mut s.forms[i];
            if body.revision != form.draft.revision {
                return Err(Stop::conflict(checks::error(
                    codes::FORM_DRAFT_CHANGED,
                    format!(
                        "“{}” formunun taslağı {}. sürümde; girdi {}. sürüme göre hazırlanmış. Taslağı yeniden okuyup değişikliği onun üstüne yapın.",
                        form.name, form.draft.revision, body.revision
                    ),
                    Some("edit.body.revision".into()),
                )));
            }
            // The whole draft: its items, scenarios, workflow, steps and translations (docs/adr/0443 §5).
            let next = FormBody {
                revision: form.draft.revision,
                ..body.clone()
            };
            if next != form.draft {
                form.draft = FormBody {
                    revision: form.draft.revision + 1,
                    ..next
                };
            }
            Some(form.clone())
        }
        FormsOp::Rename { id, name } => {
            let i = at(&s, id).ok_or_else(|| unknown(id))?;
            s.forms[i].name = name.trim().to_owned();
            Some(s.forms[i].clone())
        }
        FormsOp::Remove { id } => {
            let i = at(&s, id).ok_or_else(|| unknown(id))?;
            s.forms.remove(i);
            None
        }
        FormsOp::Publish { id, note } => {
            let i = at(&s, id).ok_or_else(|| unknown(id))?;
            let issues = forms::preflight_of(
                doc,
                &s.forms[i].draft,
                &s.forms[i].layer,
                &forms::variables(doc),
            );
            if kentos_forms::has_errors(&issues) {
                return Err(fail(
                    codes::FORM_PREFLIGHT,
                    format!(
                        "“{}” formu yayınlanamıyor: {} Formu tasarımcıda düzeltin.",
                        s.forms[i].name,
                        forms::first_error(&issues)
                    ),
                    "edit.id",
                ));
            }
            // The draft's scenarios must pass (docs/adr/0442 §1); a draft as last published is not run.
            if s.forms[i]
                .releases
                .last()
                .is_none_or(|r| r.body != s.forms[i].draft)
                && !s.forms[i].draft.tests.is_empty()
            {
                let form = &s.forms[i];
                let prepared = crate::forms_test::prepared_for(doc, form, &form.draft)
                    .map_err(|why| fail(codes::FORM_PREFLIGHT, why, "edit.id"))?;
                for test in &form.draft.tests {
                    let differences = kentos_forms::scenario::run(&prepared, test);
                    if let Some(first) = differences.first() {
                        return Err(fail(
                            codes::FORM_TESTS_FAILED,
                            format!(
                                "“{}” formu yayınlanamıyor: “{}” senaryosu tutmuyor ({first}). Senaryoyu ya da formu düzeltin.",
                                form.name, test.name
                            ),
                            "edit.id",
                        ));
                    }
                }
            }
            // The records the earlier release held and this one does not (docs/adr/0442 §2).
            if let Some(last) = s.forms[i].releases.last()
                && last.body != s.forms[i].draft
            {
                broken = forms::breaking(doc, &s.forms[i], &last.body, &s.forms[i].draft);
                if let Some(b) = &broken {
                    warnings.push(CommandWarning {
                        code: codes::FORM_BREAKING.into(),
                        path: Some("edit.id".into()),
                        message: format!(
                            "“{}” formunun yeni yayınına {} kayıt uymuyor (önceki yayına uyuyordu): {}. Yayından önce kayıtları düzeltin ya da formu gözden geçirin.",
                            s.forms[i].name,
                            b.records,
                            b.fields
                                .iter()
                                .map(|f| format!("{} {}", f.field, f.records))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    });
                }
            }
            let bound = kentos_contracts::bound_fields_digest(
                &s.forms[i].draft,
                forms::fields(doc, &s.forms[i].layer),
            );
            let form = &mut s.forms[i];
            // The draft as it was last published, over the same fields: nothing to publish. The layer's
            // fields changed since: published again over them (docs/adr/0444 §2).
            if form.releases.last().is_none_or(|r| {
                r.body != form.draft || r.fields.as_ref().is_some_and(|d| *d != bound)
            }) {
                let version = form.releases.last().map_or(1, |r| r.version + 1);
                form.releases.push(FormRelease {
                    version,
                    digest: body_digest(&form.draft),
                    body: form.draft.clone(),
                    note: note.clone().filter(|n| !n.trim().is_empty()),
                    fields: Some(bound),
                });
                if form.releases.len() > MOST_FORM_RELEASES {
                    let gone = form.releases.remove(0);
                    warnings.push(CommandWarning {
                        code: codes::FORM_RELEASE_DROPPED.into(),
                        message: format!(
                            "“{}” formu en çok {MOST_FORM_RELEASES} yayın tutar; en eski yayını ({}. sürüm) kaldırıldı.",
                            form.name, gone.version
                        ),
                        path: Some("edit.id".into()),
                    });
                }
            }
            Some(form.clone())
        }
        FormsOp::Enforce { id, on } => {
            let i = at(&s, id).ok_or_else(|| unknown(id))?;
            let form = &mut s.forms[i];
            if *on && form.releases.is_empty() {
                return Err(fail(
                    codes::FORM_UNPUBLISHED,
                    format!(
                        "“{}” formu hiç yayınlanmadı; kuralları uygulanmadan önce yayınlayın.",
                        form.name
                    ),
                    "edit.on",
                ));
            }
            // A withdrawn form's rules are not enforced (docs/adr/0445 §1).
            if *on && form.withdrawn {
                return Err(fail(
                    codes::FORM_WITHDRAWN,
                    format!(
                        "“{}” formu yayından kaldırıldı; kuralları uygulanmadan önce yeniden yayına alın.",
                        form.name
                    ),
                    "edit.on",
                ));
            }
            form.enforce = *on;
            Some(form.clone())
        }
        FormsOp::Withdraw { id, on } => {
            let i = at(&s, id).ok_or_else(|| unknown(id))?;
            let form = &mut s.forms[i];
            if *on && form.releases.is_empty() {
                return Err(fail(
                    codes::FORM_UNPUBLISHED,
                    format!(
                        "“{}” formu hiç yayınlanmadı; yayından kaldırılacak bir yayını yok.",
                        form.name
                    ),
                    "edit.on",
                ));
            }
            if *on && form.enforce {
                return Err(fail(
                    codes::FORM_ENFORCED,
                    format!(
                        "“{}” formunun kuralları katmanın yazma kuralı; önce Kuralları uygula'yı kapatın.",
                        form.name
                    ),
                    "edit.on",
                ));
            }
            form.withdrawn = *on;
            Some(form.clone())
        }
        FormsOp::Comment {
            id,
            item,
            text,
            author,
            at,
        } => {
            let i = at_of(&s, id).ok_or_else(|| unknown(id))?;
            let form = &mut s.forms[i];
            if let Some(item) = item
                && !form.draft.walk().iter().any(|(_, x)| x.id() == item)
            {
                return Err(fail(
                    codes::FORM_ITEM_UNKNOWN,
                    format!("“{}” formunun taslağında “{item}” öğesi yok.", form.name),
                    "edit.item",
                ));
            }
            if form.comments.len() >= kentos_contracts::MOST_FORM_COMMENTS {
                return Err(fail(
                    codes::INVALID_FORM,
                    format!(
                        "Bir formda en çok {} yorum olur; çözülenlerden birini silin.",
                        kentos_contracts::MOST_FORM_COMMENTS
                    ),
                    "edit",
                ));
            }
            let next_id = form.comments.iter().map(|c| c.id).max().unwrap_or(0) + 1;
            form.comments.push(kentos_contracts::FormComment {
                id: next_id,
                item: item.clone(),
                author: author.clone(),
                at: at.clone(),
                text: text.trim().to_owned(),
                resolved: false,
            });
            Some(form.clone())
        }
        FormsOp::Resolve {
            id,
            comment,
            resolved,
        } => {
            let i = at_of(&s, id).ok_or_else(|| unknown(id))?;
            let form = &mut s.forms[i];
            let Some(c) = form.comments.iter_mut().find(|c| c.id == *comment) else {
                return Err(fail(
                    codes::FORM_COMMENT_UNKNOWN,
                    format!("“{}” formunda {comment} numaralı yorum yok.", form.name),
                    "edit.comment",
                ));
            };
            c.resolved = *resolved;
            Some(form.clone())
        }
    };
    Ok((s, kept, warnings, broken))
}

fn check(doc: &Document, input: &FormsEdit) -> Result<Next, Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    next(doc, input)
}

fn answer(
    doc: &Document,
    s: &ProjectSettings,
    form: Option<ProjectForm>,
    changed: bool,
    revision: String,
    breaking: Option<kentos_contracts::FormBreaking>,
) -> FormsEdited {
    let issues: Vec<FormIssue> = form.as_ref().map_or_else(Vec::new, |f| {
        forms::preflight_of(doc, &f.draft, &f.layer, &forms::variables(doc))
    });
    FormsEdited {
        forms: forms::summaries(s),
        form,
        issues,
        changed,
        revision,
        breaking,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsEdit) -> CommandResult<FormsEdited> {
    match check(cx.doc, input) {
        Ok((next, form, warnings, breaking)) => {
            let changed = next != *cx.doc.settings();
            CommandResult::Completed {
                output: answer(
                    cx.doc,
                    &next,
                    form,
                    changed,
                    cx.doc.revision().to_string(),
                    breaking,
                ),
                warnings,
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the settings.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FormsEdit) -> CommandResult<FormsEdited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_FORMS_EDIT,
        kentos_contracts::CAD_FORMS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: FormsEdit) -> CommandResult<FormsEdited> {
    let (next, form, warnings, breaking) = match check(cx.doc, &input) {
        Ok(next) => next,
        Err(stop) => return stop.into(),
    };
    let changed = next != *cx.doc.settings();
    if changed {
        cx.doc.set_settings(next);
    }
    let settings = cx.doc.settings().clone();
    CommandResult::Completed {
        output: answer(
            cx.doc,
            &settings,
            form,
            changed,
            cx.doc.revision().to_string(),
            breaking,
        ),
        warnings,
    }
}

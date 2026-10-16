//! `cad.forms.edit` v1 (docs/adr/0401 §9): the project's forms. A project setting
//! (`Document::set_settings`: an edit, no undo step), as the named views are; the designer keeps its
//! own undo. The shared cases are `fixtures/commands/v1/cad.forms.edit.json`.

use kentos_contracts::{
    CommandResult, CommandWarning, FormIssue, FormRelease, FormsEdit, FormsEdited, FormsOp,
    LayerNodeType, MOST_FORM_RELEASES, MOST_FORM_TEXT, MOST_FORMS, ProjectForm, ProjectSettings,
    body_digest, body_problem, form_id_ok, form_name_ok,
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
        | FormsOp::Enforce { id, .. } => Some(id.as_str()),
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

/// The settings afterwards, the form kept and the warnings.
type Next = (ProjectSettings, Option<ProjectForm>, Vec<CommandWarning>);

fn next(doc: &Document, input: &FormsEdit) -> Result<Next, Stop> {
    let mut s = doc.settings().clone();
    let mut warnings = Vec::new();
    let at = |s: &ProjectSettings, id: &str| s.forms.iter().position(|f| f.id == id);
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
                id,
                name: name.trim().to_owned(),
                layer: layer.clone(),
                draft,
                releases: Vec::new(),
                enforce: false,
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
            if body.items != form.draft.items || body.clear_hidden != form.draft.clear_hidden {
                form.draft.items = body.items.clone();
                form.draft.clear_hidden = body.clear_hidden;
                form.draft.revision += 1;
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
            let issues = kentos_forms::preflight(
                &s.forms[i].draft,
                forms::fields(doc, &s.forms[i].layer),
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
            let form = &mut s.forms[i];
            // The draft as it was last published: nothing to publish.
            if form.releases.last().is_none_or(|r| r.body != form.draft) {
                let version = form.releases.last().map_or(1, |r| r.version + 1);
                form.releases.push(FormRelease {
                    version,
                    digest: body_digest(&form.draft),
                    body: form.draft.clone(),
                    note: note.clone().filter(|n| !n.trim().is_empty()),
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
            form.enforce = *on;
            Some(form.clone())
        }
    };
    Ok((s, kept, warnings))
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
) -> FormsEdited {
    let issues: Vec<FormIssue> = form.as_ref().map_or_else(Vec::new, |f| {
        kentos_forms::preflight(
            &f.draft,
            forms::fields(doc, &f.layer),
            &forms::variables(doc),
        )
    });
    FormsEdited {
        forms: forms::summaries(s),
        form,
        issues,
        changed,
        revision,
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
        Ok((next, form, warnings)) => {
            let changed = next != *cx.doc.settings();
            CommandResult::Completed {
                output: answer(cx.doc, &next, form, changed, cx.doc.revision().to_string()),
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
    let (next, form, warnings) = match check(cx.doc, &input) {
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
        ),
        warnings,
    }
}

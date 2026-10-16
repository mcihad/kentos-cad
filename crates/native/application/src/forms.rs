//! What the forms' commands share (docs/adr/0401 §9): the form an input names, the body it asks
//! (the latest release or the draft), the form prepared over its layer's fields, the record an input
//! names and the write profile a layer's enforcing forms make (`cad.entities.set`, §8).

use kentos_contracts::{
    Entity, FormBody, FormIssue, FormSeverity, FormSummary, LayerField, ProjectForm,
    ProjectSettings,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_expression::Variable;
use kentos_forms::{Attrs, Object, Prepared};

use crate::checks::{Stop, error, is_uid_text};
use crate::codes;

pub(crate) fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The form of `id`, or `unknown_form` at `path`.
pub(crate) fn form<'a>(
    settings: &'a ProjectSettings,
    id: &str,
    path: &str,
) -> Result<&'a ProjectForm, Stop> {
    settings.forms.iter().find(|f| f.id == id).ok_or_else(|| {
        fail(
            codes::UNKNOWN_FORM,
            format!(
                "“{id}” kimlikli bir form yok. Formlar penceresinden var olan bir formu seçin."
            ),
            path,
        )
    })
}

/// The fields of the form's layer (none when the layer is gone).
pub(crate) fn fields<'a>(doc: &'a Document, layer: &str) -> &'a [LayerField] {
    doc.layers().get(layer).map_or(&[], |n| n.fields.as_slice())
}

/// The `@` values a form's rules read: the project's own and the built-in ones that do not change
/// with the moment or the user (a rule's answer is the same wherever it is asked).
pub(crate) fn variables(doc: &Document) -> Vec<Variable> {
    let mut all =
        kentos_project::variables::expression_variables(doc.name(), doc.settings(), 0.0, "");
    all.retain(|v| !matches!(v.name.as_str(), "tarih" | "simdi" | "kullanici"));
    all
}

/// The body `draft` asks: the draft, else the latest release (`form_unpublished` at `path`
/// without one), with the release's version.
pub(crate) fn body<'a>(
    form: &'a ProjectForm,
    draft: bool,
    path: &str,
) -> Result<(&'a FormBody, Option<u32>), Stop> {
    if draft {
        return Ok((&form.draft, None));
    }
    form.releases
        .last()
        .map(|r| (&r.body, Some(r.version)))
        .ok_or_else(|| {
            fail(
                codes::FORM_UNPUBLISHED,
                format!(
                    "“{}” formu hiç yayınlanmadı. Önce yayınlayın ya da taslağıyla çalışın.",
                    form.name
                ),
                path,
            )
        })
}

/// The body prepared over the layer's fields, or `form_preflight` at `path` with its first error.
pub(crate) fn prepared(
    doc: &Document,
    form: &ProjectForm,
    body: &FormBody,
    path: &str,
) -> Result<Prepared, Stop> {
    Prepared::new(body, fields(doc, &form.layer), &variables(doc)).map_err(|issues| {
        let first = first_error(&issues);
        fail(
            codes::FORM_PREFLIGHT,
            format!(
                "“{}” formu kullanılamıyor: {first} Formu tasarımcıda düzeltin.",
                form.name
            ),
            path,
        )
    })
}

/// The first error's sentence.
pub(crate) fn first_error(issues: &[FormIssue]) -> String {
    issues
        .iter()
        .find(|i| i.severity == FormSeverity::Error)
        .map_or_else(String::new, |i| i.message.clone())
}

/// The object `uid` names on the form's layer: `invalid_uid`, `entity_not_found`,
/// `form_layer_mismatch` at `path`.
pub(crate) fn record<'a>(
    doc: &'a Document,
    form: &ProjectForm,
    uid: &str,
    path: &str,
) -> Result<(Slot, &'a Entity), Stop> {
    let found = Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|slot| Some((slot, doc.get(slot)?)));
    let Some((slot, e)) = found else {
        return Err(fail(
            codes::ENTITY_NOT_FOUND,
            format!(
                "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
            ),
            path,
        ));
    };
    if e.base().layer_id != form.layer {
        return Err(fail(
            codes::FORM_LAYER_MISMATCH,
            format!(
                "Nesne “{}” formunun katmanında değil. Formun katmanındaki bir nesneyi seçin.",
                form.name
            ),
            path,
        ));
    }
    Ok((slot, e))
}

/// A uid's shape: `invalid_uid` at `path`.
pub(crate) fn uid_text(uid: &str, path: &str) -> Result<(), Stop> {
    if is_uid_text(uid) {
        return Ok(());
    }
    Err(fail(
        codes::INVALID_UID,
        format!(
            "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
        ),
        path,
    ))
}

/// What a form's rules see of an object beside its attributes.
pub(crate) fn object(doc: &Document, slot: Slot, e: &Entity) -> Object {
    Object {
        kind: crate::layer_filter::kind_label(e).to_owned(),
        layer: doc
            .layers()
            .get(&e.base().layer_id)
            .map_or_else(String::new, |n| n.name.clone()),
        id: f64::from(slot.0),
        label: e.base().label.clone(),
        measured: Default::default(),
    }
}

/// The forms as the lists show them.
pub(crate) fn summaries(settings: &ProjectSettings) -> Vec<FormSummary> {
    settings
        .forms
        .iter()
        .map(|f| FormSummary {
            id: f.id.clone(),
            name: f.name.clone(),
            layer: f.layer.clone(),
            revision: f.draft.revision,
            version: f.releases.last().map(|r| r.version),
            enforce: f.enforce,
        })
        .collect()
}

/// Whether an object's attributes going from `before` to `after` keep the rules of the forms its
/// layer enforces (docs/adr/0401 §8): `form_rules` at `path` with the first broken rule. A release
/// that no longer fits the layer's fields refuses every write until the form is published again.
pub(crate) fn enforced(
    doc: &Document,
    slot: Slot,
    e: &Entity,
    layer: &str,
    after: &Attrs,
    path: &str,
) -> Result<(), Stop> {
    // A write that changes none of the layer's fields is not the form's.
    let before = &e.base().attrs;
    if fields(doc, layer)
        .iter()
        .all(|f| before.get(&f.name) == after.get(&f.name))
    {
        return Ok(());
    }
    for form in doc
        .settings()
        .forms
        .iter()
        .filter(|f| f.enforce && f.layer == layer)
    {
        let Some(release) = form.releases.last() else {
            continue;
        };
        let refuse = |message: String| {
            fail(
                codes::FORM_RULES,
                format!(
                    "“{}” formunun kuralları: {message} Değeri formla düzenleyin ya da formun Kuralları uygula seçeneğini kapatın.",
                    form.name
                ),
                path,
            )
        };
        let prepared = Prepared::new(&release.body, fields(doc, layer), &variables(doc))
            .map_err(|issues| refuse(first_error(&issues)))?;
        let issues = prepared.enforce(before, after, &object(doc, slot, e));
        if !issues.is_empty() {
            return Err(refuse(first_error(&issues)));
        }
    }
    Ok(())
}

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
    all.retain(|v| !kentos_forms::MOMENT.contains(&v.name.as_str()));
    all
}

/// Every `@` value a rule may read, the moment's and the user's among them (read empty): what a
/// layer's record rules compile against (docs/adr/0438 §1).
pub(crate) fn variables_all(doc: &Document) -> Vec<Variable> {
    kentos_project::variables::expression_variables(doc.name(), doc.settings(), 0.0, "")
}

/// [`variables`] with the moment's (`@tarih`, `@simdi`) and the user's (`@kullanici`) when the input
/// gives them (docs/adr/0402 §5): `invalid_moment` at `moment`, `invalid_user` at `user`.
pub(crate) fn variables_given(
    doc: &Document,
    moment: Option<&str>,
    user: Option<&str>,
) -> Result<Vec<Variable>, Stop> {
    let now = match moment {
        None => None,
        Some(text) => match kentos_geometry_core::time::read(text) {
            kentos_geometry_core::time::Read::Moment(t) => Some(t),
            _ => {
                return Err(fail(
                    codes::INVALID_MOMENT,
                    format!(
                        "“{text}” bir an değil. Bir tarih (2026-10-16) ya da tarih ve yerel saat (2026-10-16T14:30:00) yazın."
                    ),
                    "moment",
                ));
            }
        },
    };
    if let Some(name) = user
        && (name.trim().is_empty()
            || name.trim() != name
            || name.chars().count() > 64
            || name.chars().any(char::is_control))
    {
        return Err(fail(
            codes::INVALID_USER,
            "Kullanıcının adı boş olamaz, başında ve sonunda boşluk olmaz, en çok 64 karakterdir."
                .into(),
            "user",
        ));
    }
    let mut all = kentos_project::variables::expression_variables(
        doc.name(),
        doc.settings(),
        now.unwrap_or(0.0),
        user.unwrap_or(""),
    );
    all.retain(|v| match v.name.as_str() {
        "tarih" | "simdi" => now.is_some(),
        "kullanici" => user.is_some(),
        _ => true,
    });
    Ok(all)
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

/// The layer's record rules (docs/adr/0438 §1), which a form over it shows.
pub(crate) fn rules<'d>(doc: &'d Document, layer: &str) -> &'d [kentos_contracts::LayerRule] {
    doc.layers()
        .get(layer)
        .map_or(&[][..], |n| n.rules.as_slice())
}

/// The layer's value groups (docs/adr/0423), which a form over it reads.
pub(crate) fn contingents<'d>(
    doc: &'d Document,
    layer: &str,
) -> &'d [kentos_contracts::FieldGroup] {
    doc.layers()
        .get(layer)
        .map_or(&[][..], |n| n.contingents.as_slice())
}

/// The layer's subtypes (docs/adr/0422), which a form over it reads.
pub(crate) fn subtypes<'d>(
    doc: &'d Document,
    layer: &str,
) -> Option<&'d kentos_contracts::LayerSubtypes> {
    doc.layers().get(layer).and_then(|n| n.subtypes.as_ref())
}

/// The body prepared over the layer's fields with `variables`, its geometry values in the drawing's
/// unit, or `form_preflight` at `path` with its first error.
pub(crate) fn prepared(
    doc: &Document,
    form: &ProjectForm,
    body: &FormBody,
    variables: &[Variable],
    path: &str,
) -> Result<Prepared, Stop> {
    Prepared::new_with(
        body,
        fields(doc, &form.layer),
        variables,
        crate::expression_unit::reading_unit(doc.settings()),
        &doc.form_world(body),
    )
    .map(|p| {
        p.with_subtypes(subtypes(doc, &form.layer))
            .with_contingents(contingents(doc, &form.layer))
            .with_rules(rules(doc, &form.layer))
    })
    .map_err(|issues| {
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

/// The preflight of `body` over the layer's fields, the other layers it reads known (docs/adr/0404).
pub(crate) fn preflight_of(
    doc: &Document,
    body: &FormBody,
    layer: &str,
    variables: &[Variable],
) -> Vec<FormIssue> {
    kentos_forms::preflight_in(body, fields(doc, layer), variables, &doc.form_world(body))
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

/// What a form's rules see of an object beside its attributes (its measures from its shape).
pub(crate) fn object(doc: &Document, slot: Slot, e: &Entity) -> Object {
    let layer = doc
        .layers()
        .get(&e.base().layer_id)
        .map_or("", |n| n.name.as_str());
    let mut object = kentos_forms::object_of(e, layer, f64::from(slot.0));
    object.uid = doc.uid(slot).map(|u| u.to_string());
    // A command judges the record's attachments (the document's following does not: docs/adr/0407 §1).
    object.attachments = Some(kentos_forms::Attached::of(&e.base().attachments));
    object
}

/// The latest release of the form `id` prepared as a record's editor would (the drawing's `@` values,
/// its subtypes and value groups); the reason it cannot be, else.
pub fn prepared_in(doc: &Document, id: &str) -> Result<Prepared, String> {
    let variables = variables_given(doc, None, None).map_err(|s| s.into_error().message)?;
    let form = self::form(doc.settings(), id, "form").map_err(|s| s.into_error().message)?;
    let (body, _) = self::body(form, false, "draft").map_err(|s| s.into_error().message)?;
    prepared(doc, form, body, &variables, "form").map_err(|s| s.into_error().message)
}

/// The suggesting field items' values in the layer (docs/adr/0407 §3): each item's id and its
/// suggestions, the most used first.
pub fn suggestions(doc: &Document, layer: &str, prepared: &Prepared) -> Vec<(String, Vec<String>)> {
    prepared
        .suggesting()
        .into_iter()
        .map(|(id, name)| {
            let values = doc
                .by_layer_with_uids(layer)
                .filter_map(|(_, e)| e.base().attrs.get(name).map(String::as_str));
            (id.to_owned(), kentos_forms::suggestions_of(values))
        })
        .collect()
}

/// The attributes of the other objects of the layer: what a unique field is asked against.
pub(crate) fn others<'a>(
    doc: &'a Document,
    layer: &'a str,
    uid: Uuid,
) -> impl Iterator<Item = &'a Attrs> {
    doc.by_layer_with_uids(layer)
        .filter(move |(u, _)| *u != uid)
        .map(|(_, e)| &e.base().attrs)
}

/// A form ready for one record's screen (docs/adr/0402 §7): Formla düzenle asks it once with the
/// rules `cad.forms.validate` and `cad.forms.submit` follow, then answers each keystroke with the
/// forms core as they would.
pub struct RecordScreen {
    pub prepared: Prepared,
    /// What the rules see of the object (its measures in metres).
    pub object: Object,
    /// The record's attributes now.
    pub attrs: Attrs,
    /// The release's version; none for the draft.
    pub version: Option<u32>,
    /// The form's layer.
    pub layer: String,
    pub uid: Uuid,
}

/// The screen of the object `uid` through the form `form` (its draft with `draft`), the moment and
/// the user given as the commands take them; the commands' refusal when it cannot be.
pub fn record_screen(
    doc: &Document,
    form: &str,
    uid: &str,
    draft: bool,
    moment: Option<&str>,
    user: Option<&str>,
) -> Result<RecordScreen, Box<kentos_contracts::CommandError>> {
    let made = (|| {
        uid_text(uid, "uid")?;
        let variables = variables_given(doc, moment, user)?;
        let form = self::form(doc.settings(), form, "form")?;
        let (body, version) = body(form, draft, "draft")?;
        let (slot, e) = record(doc, form, uid, "uid")?;
        let prepared = prepared(doc, form, body, &variables, "form")?;
        Ok(RecordScreen {
            prepared,
            object: object(doc, slot, e),
            attrs: e.base().attrs.clone(),
            version,
            layer: form.layer.clone(),
            uid: doc.uid(slot).unwrap_or_default(),
        })
    })();
    made.map_err(|stop: Stop| Box::new(stop.into_error()))
}

/// The screen of a body not yet kept (the designer's copy: docs/adr/0403 §3) for the object `uid`,
/// or for a new, empty record without a shape; the commands' refusal when it cannot be.
pub fn preview_screen(
    doc: &Document,
    form: &str,
    body: &FormBody,
    uid: Option<&str>,
    moment: Option<&str>,
    user: Option<&str>,
) -> Result<RecordScreen, Box<kentos_contracts::CommandError>> {
    let made = (|| {
        let variables = variables_given(doc, moment, user)?;
        let form = self::form(doc.settings(), form, "form")?;
        let prepared = prepared(doc, form, body, &variables, "form")?;
        let layer_name = doc
            .layers()
            .get(&form.layer)
            .map_or("", |n| n.name.as_str());
        let (object, attrs, uid) = match uid {
            Some(uid) => {
                uid_text(uid, "uid")?;
                let (slot, e) = record(doc, form, uid, "uid")?;
                (
                    object(doc, slot, e),
                    e.base().attrs.clone(),
                    doc.uid(slot).unwrap_or_default(),
                )
            }
            None => (
                Object {
                    layer: layer_name.to_owned(),
                    ..Object::default()
                },
                Attrs::new(),
                Uuid::nil(),
            ),
        };
        Ok(RecordScreen {
            prepared,
            object,
            attrs,
            version: None,
            layer: form.layer.clone(),
            uid,
        })
    })();
    made.map_err(|stop: Stop| Box::new(stop.into_error()))
}

/// The issues of the screen's unique fields for `attrs` against the layer's other objects.
pub fn unique_on_layer(doc: &Document, screen: &RecordScreen, attrs: &Attrs) -> Vec<FormIssue> {
    if screen.prepared.unique_keys().is_empty() {
        return Vec::new();
    }
    screen.prepared.unique_issues(
        attrs,
        &screen.object,
        others(doc, &screen.layer, screen.uid),
    )
}

/// The warnings among `issues` as a command's warnings (the field's value their path).
pub(crate) fn warnings_of(issues: &[FormIssue]) -> Vec<kentos_contracts::CommandWarning> {
    issues
        .iter()
        .filter(|i| i.severity == FormSeverity::Warning)
        .map(|i| kentos_contracts::CommandWarning {
            code: i.code.clone(),
            message: i.message.clone(),
            path: i.field.as_ref().map(|k| format!("values.{k}")),
        })
        .collect()
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
        let prepared = Prepared::new_with(
            &release.body,
            fields(doc, layer),
            &variables(doc),
            crate::expression_unit::reading_unit(doc.settings()),
            &doc.form_world(&release.body),
        )
        .map(|p| {
            p.with_subtypes(subtypes(doc, layer))
                .with_contingents(contingents(doc, layer))
                .with_rules(rules(doc, layer))
        })
        .map_err(|issues| refuse(first_error(&issues)))?;
        let mut probe = e.clone();
        probe.base_mut().attrs = after.clone();
        let issues = prepared.enforce(before, after, &object(doc, slot, &probe));
        if kentos_forms::has_errors(&issues) {
            return Err(refuse(first_error(&issues)));
        }
    }
    Ok(())
}

/// A child as a write would leave it: its id, layer and attributes (docs/adr/0440 §4).
pub(crate) type Pending<'a> = (&'a str, &'a str, &'a Attrs);

/// A record's snapshot (docs/adr/0440 §4): the first 16 hexadecimal digits of the SHA-256 of the
/// canonical JSON of its attributes (but `skip`'s: a workflow's state and log), its attachments and its
/// children (each relation whose parent is `layer`, in the project's order, its children holding `uid`
/// sorted by their ids, with their attributes), `pending` standing for the document where a write would
/// change it. What a reviewer approves: the same record with the same children reads the same.
pub(crate) fn snapshot(
    doc: &Document,
    layer: &str,
    uid: &str,
    attrs: &Attrs,
    attachments: &[kentos_contracts::Attachment],
    skip: &[&str],
    pending: &[Pending<'_>],
) -> String {
    let own: serde_json::Map<String, serde_json::Value> = attrs
        .iter()
        .filter(|(k, _)| !skip.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();
    let mut children = Vec::new();
    for r in doc
        .settings()
        .relations
        .iter()
        .filter(|r| r.parent == layer)
    {
        let Some(name) = doc
            .layers()
            .get(&r.child)
            .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
            .map(|f| f.name.clone())
        else {
            continue;
        };
        let mut rows: Vec<(String, Attrs)> = doc
            .by_layer_with_uids(&r.child)
            .map(|(u, e)| (u.to_string(), e.base().attrs.clone()))
            .filter(|(u, _)| !pending.iter().any(|(p, ..)| p == u))
            .collect();
        rows.extend(
            pending
                .iter()
                .filter(|(_, l, _)| *l == r.child)
                .map(|(u, _, a)| ((*u).to_owned(), (*a).clone())),
        );
        rows.retain(|(_, a)| {
            a.get(&name)
                .is_some_and(|v| v.split(';').any(|h| h.trim() == uid))
        });
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        for (u, a) in rows {
            children.push(serde_json::json!({ "relation": r.id, "uid": u, "attrs": a }));
        }
    }
    let value = serde_json::json!({
        "attrs": own,
        "attachments": serde_json::to_value(attachments).unwrap_or_default(),
        "children": children,
    });
    kentos_contracts::canonical::digest_of(&value, 16)
}

/// The fields a form's workflow keeps out of the snapshot: its state's and its log's names.
pub(crate) fn snapshot_skip<'a>(doc: &'a Document, layer: &str, body: &FormBody) -> Vec<&'a str> {
    let Some(w) = &body.workflow else {
        return Vec::new();
    };
    let fields = fields(doc, layer);
    [Some(w.field.as_str()), w.log.as_deref(), w.notes.as_deref()]
        .into_iter()
        .flatten()
        .filter_map(|k| {
            fields
                .iter()
                .find(|f| f.key() == k)
                .map(|f| f.name.as_str())
        })
        .collect()
}

/// The snapshot of the record `uid` holding `attrs` as `form` sees it (docs/adr/0440 §4): what a workflow
/// step would write at the end of its log line; none for a form or a record the drawing has not.
pub fn record_snapshot(doc: &Document, form: &str, uid: &str, attrs: &Attrs) -> Option<String> {
    let f = doc.settings().forms.iter().find(|f| f.id == form)?;
    let body = f.release().map_or(&f.draft, |r| &r.body);
    let slot = doc.slot_of(Uuid::parse_str(uid).ok()?)?;
    let e = doc.get(slot)?;
    let skip = snapshot_skip(doc, &f.layer, body);
    Some(snapshot(
        doc,
        &f.layer,
        uid,
        attrs,
        &e.base().attachments,
        &skip,
        &[],
    ))
}

/// The snapshot a record's log names last (docs/adr/0440 §4): the `#…` that ends its last line.
pub fn logged_snapshot(log: &str) -> Option<&str> {
    let line = log.lines().rev().find(|l| !l.trim().is_empty())?;
    let (_, tail) = line.rsplit_once(" · #")?;
    (tail.len() == 16 && tail.chars().all(|c| c.is_ascii_hexdigit())).then_some(tail)
}

/// The layer field keys the body's field items bind (mirrors too), once each, in the form's order.
pub(crate) fn bound_keys(body: &FormBody) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, item) in body.walk() {
        if let kentos_contracts::FormItem::Field(f) = item
            && !out.contains(&f.field)
        {
            out.push(f.field.clone());
        }
    }
    out
}

/// The errors each of the layer's records has by `body` (completed as kept, then validated, as
/// Kayıtları denetle judges them; unique values aside), by the records' ids; none when the body cannot
/// be prepared.
pub(crate) fn record_errors(
    doc: &Document,
    form: &ProjectForm,
    body: &FormBody,
) -> Option<Vec<(kentos_domain::Uuid, Vec<FormIssue>)>> {
    let variables = variables_given(doc, None, None).ok()?;
    let prepared = prepared(doc, form, body, &variables, "form").ok()?;
    let mut out = Vec::new();
    for (uid, e) in doc.by_layer_with_uids(&form.layer) {
        let Some(slot) = doc.slot_of(uid) else {
            continue;
        };
        let object = self::object(doc, slot, e);
        let (after, mut issues) = prepared.complete(&e.base().attrs, &object, false);
        let said: Vec<Option<String>> = issues.iter().map(|i| i.field.clone()).collect();
        issues.extend(
            prepared
                .validate(&after, &object)
                .into_iter()
                .filter(|i| !said.contains(&i.field)),
        );
        issues.retain(|i| i.severity == kentos_contracts::FormSeverity::Error);
        out.push((uid, issues));
    }
    Some(out)
}

/// The records `new` breaks that `old` held (docs/adr/0442 §2); none when none.
pub(crate) fn breaking(
    doc: &Document,
    form: &ProjectForm,
    old: &FormBody,
    new: &FormBody,
) -> Option<kentos_contracts::FormBreaking> {
    let before = record_errors(doc, form, old)?;
    let after = record_errors(doc, form, new)?;
    let mut records = 0u32;
    let mut by_field: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for ((_, was), (_, now)) in before.iter().zip(&after) {
        if was.is_empty() && !now.is_empty() {
            records += 1;
            let mut fields: Vec<&str> = now.iter().filter_map(|i| i.field.as_deref()).collect();
            fields.sort_unstable();
            fields.dedup();
            for f in fields {
                *by_field.entry(f.to_owned()).or_default() += 1;
            }
        }
    }
    (records > 0).then(|| kentos_contracts::FormBreaking {
        records,
        fields: by_field
            .into_iter()
            .map(|(field, records)| kentos_contracts::FieldCount { field, records })
            .collect(),
    })
}

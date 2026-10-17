//! `cad.forms.import` v1 (Formu içe aktar, docs/adr/0435): a form package made a new form of a layer. Its
//! fields bind to the layer's (`map`, else the same key, else the same name); the missing ones are added
//! with their value lists when asked, else the import is refused. The fields and the lists are written as
//! one undo step “Formu içe aktar”; the form itself is a project setting (as `cad.forms.edit`'s). The
//! shared cases are `fixtures/commands/v1/cad.forms.import.json`.

use kentos_contracts::{
    CommandResult, CommandWarning, FORM_PACKAGE_FORMAT, FORM_PACKAGE_VERSION, FieldBinding,
    FieldDomain, FormItem, FormSeverity, FormsImport, FormsImported, LayerField, LayerNodeType,
    MOST_FORMS, ProjectForm, body_problem, domains_problem, fields_problem, form_id_ok,
    form_name_ok, link_problem,
};
use kentos_domain::Document;
use kentos_forms::field_refs::Rebind;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::forms::{self, fail};

/// The undo step's name.
pub const LABEL: &str = "Formu içe aktar";

/// What the import writes.
struct Planned {
    form: ProjectForm,
    bound: Vec<FieldBinding>,
    /// The layer's fields afterwards, when fields are added.
    fields: Option<Vec<LayerField>>,
    added: Vec<String>,
    /// The project's lists afterwards, when lists are added.
    domains: Option<Vec<FieldDomain>>,
    added_domains: Vec<String>,
    errors: u32,
    warnings: Vec<CommandWarning>,
}

fn kind_words(f: &LayerField) -> String {
    f.kind.label().to_lowercase()
}

/// The keys the body's items bind.
fn bound_keys(items: &[FormItem], out: &mut Vec<String>) {
    for i in items {
        if let FormItem::Field(f) = i {
            out.push(f.field.clone());
        }
        bound_keys(i.items(), out);
    }
}

fn check(doc: &Document, input: &FormsImport) -> Result<Planned, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let p = &input.package;
    if p.format != FORM_PACKAGE_FORMAT || p.version != FORM_PACKAGE_VERSION {
        return Err(fail(
            codes::INVALID_PACKAGE,
            format!(
                "Paketin biçimi “{}” sürüm {}; KentOS form paketi ({FORM_PACKAGE_FORMAT}, sürüm {FORM_PACKAGE_VERSION}) değil.",
                p.format, p.version
            ),
            "package",
        ));
    }
    if let Some(problem) = body_problem(&p.body) {
        return Err(fail(
            codes::INVALID_PACKAGE,
            format!("Paketin formu kurallarına uymuyor: {problem}"),
            "package.body",
        ));
    }
    if !p.fields.is_empty()
        && let Some(problem) = fields_problem(&p.fields)
    {
        return Err(fail(
            codes::INVALID_PACKAGE,
            format!("Paketin alanları kurallarına uymuyor: {problem}"),
            "package.fields",
        ));
    }
    if let Some((_, problem)) = domains_problem(&p.domains) {
        return Err(fail(
            codes::INVALID_PACKAGE,
            format!("Paketin değer listeleri kurallarına uymuyor: {problem}"),
            "package.domains",
        ));
    }
    let mut keys = Vec::new();
    bound_keys(&p.body.items, &mut keys);
    if let Some(k) = keys
        .iter()
        .find(|k| !p.fields.iter().any(|f| f.key() == k.as_str()))
    {
        return Err(fail(
            codes::INVALID_PACKAGE,
            format!("Paketin formu “{k}” alanını anıyor; alan paketin alanlarında yok."),
            "package.body",
        ));
    }
    let Some(node) = doc.layers().get(&input.layer) else {
        return Err(fail(
            codes::LAYER_NOT_FOUND,
            format!(
                "“{}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                input.layer
            ),
            "layer",
        ));
    };
    if node.kind != LayerNodeType::Layer || node.service.is_some() {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!(
                "“{}” nesne tutan bir katman değil; form yalnız katmanın olur. Bir katman seçin.",
                node.name
            ),
            "layer",
        ));
    }
    let name = input.name.clone().unwrap_or_else(|| p.name.clone());
    if !form_name_ok(name.trim()) {
        return Err(fail(
            codes::INVALID_FORM_NAME,
            "Form adı boş olamaz, en çok 64 karakterdir ve denetim karakteri içermez. Başka bir ad yazın.".into(),
            "name",
        ));
    }
    let taken: Vec<String> = doc.settings().forms.iter().map(|f| f.id.clone()).collect();
    let id = match &input.id {
        Some(id) if !form_id_ok(id) => {
            return Err(fail(
                codes::INVALID_FORM_ID,
                format!("“{id}” bir form kimliği olamaz: 1–64 harf, rakam, -, _ ya da . olmalı."),
                "id",
            ));
        }
        Some(id) if taken.contains(id) => {
            return Err(fail(
                codes::FORM_EXISTS,
                format!("“{id}” kimlikli başka bir form var. Başka bir kimlik verin."),
                "id",
            ));
        }
        Some(id) => id.clone(),
        None => kentos_forms::item_id_of(
            if name.chars().any(char::is_alphanumeric) {
                &name
            } else {
                "form"
            },
            &taken,
        ),
    };
    if taken.len() >= MOST_FORMS {
        return Err(fail(
            codes::TOO_MANY_FORMS,
            "Projede en çok 100 form olur. Kullanmadığınız birini silin.".into(),
            "id",
        ));
    }
    for (from, to) in &input.map {
        if !p.fields.iter().any(|f| f.key() == from) {
            return Err(fail(
                codes::INVALID_MAP,
                format!("Eşlemedeki “{from}” paketin bir alanı değil."),
                &format!("map/{from}"),
            ));
        }
        if !node.fields.iter().any(|f| f.key() == to) {
            return Err(fail(
                codes::FIELD_NOT_FOUND,
                format!("“{}” katmanında “{to}” alanı yok.", node.name),
                &format!("map/{from}"),
            ));
        }
    }
    let mut bound = Vec::new();
    let mut pairs = Vec::new();
    let mut missing: Vec<&LayerField> = Vec::new();
    for pf in &p.fields {
        let target = match input.map.get(pf.key()) {
            Some(to) => node.fields.iter().find(|f| f.key() == to),
            None => node
                .fields
                .iter()
                .find(|f| f.key() == pf.key())
                .or_else(|| node.fields.iter().find(|f| f.name == pf.name)),
        };
        match target {
            Some(tf) => {
                if tf.kind != pf.kind {
                    return Err(fail(
                        codes::FIELD_KIND_MISMATCH,
                        format!(
                            "Paketin “{}” alanı {}; katmanın “{}” alanı {}. Aynı türde bir alana bağlayın.",
                            pf.label(),
                            kind_words(pf),
                            tf.label(),
                            kind_words(tf)
                        ),
                        &format!("map/{}", pf.key()),
                    ));
                }
                bound.push(FieldBinding {
                    from: pf.key().to_owned(),
                    to: tf.key().to_owned(),
                });
                pairs.push(Rebind {
                    from_key: pf.key().to_owned(),
                    from_name: pf.name.clone(),
                    to_key: tf.key().to_owned(),
                    to_name: tf.name.clone(),
                });
            }
            None => missing.push(pf),
        }
    }
    if !missing.is_empty() && !input.add_fields {
        let said: Vec<&str> = missing.iter().map(|f| f.label()).collect();
        return Err(fail(
            codes::FORM_FIELDS_MISSING,
            format!(
                "Paketin şu alanları “{}” katmanında yok: {}. map ile bağlayın ya da addFields ile ekleyin.",
                node.name,
                said.join(", ")
            ),
            "package.fields",
        ));
    }
    // The added fields and their value lists.
    let mut warnings = Vec::new();
    let mut domains = doc.settings().domains.clone();
    let mut added_domains = Vec::new();
    let mut added_fields: Vec<LayerField> = Vec::new();
    for pf in &missing {
        let mut f = (*pf).clone();
        if let Some(did) = f.domain.clone() {
            match domains.iter().find(|d| d.id == did) {
                Some(d) => {
                    let same = p.domains.iter().find(|x| x.id == did) == Some(d);
                    if !same || link_problem(d, &f).is_some() {
                        warnings.push(CommandWarning {
                            code: codes::DOMAIN_KEPT.into(),
                            message: format!(
                                "Projede “{}” değer listesi vardı; “{}” alanı projeninkine bağlandı.",
                                d.name,
                                f.label()
                            ),
                            path: Some("package.domains".into()),
                        });
                    }
                    if link_problem(d, &f).is_some() {
                        f.domain = None;
                    }
                }
                None => match p.domains.iter().find(|x| x.id == did) {
                    Some(d) => {
                        domains.push(d.clone());
                        added_domains.push(did);
                    }
                    None => f.domain = None,
                },
            }
        }
        bound.push(FieldBinding {
            from: pf.key().to_owned(),
            to: f.key().to_owned(),
        });
        added_fields.push(f);
    }
    let added: Vec<String> = added_fields.iter().map(|f| f.key().to_owned()).collect();
    let fields_after: Vec<LayerField> = node.fields.iter().cloned().chain(added_fields).collect();
    if !added.is_empty()
        && let Some(problem) = fields_problem(&fields_after)
    {
        return Err(fail(
            codes::INVALID_PACKAGE,
            format!("Paketin alanları katmana eklenemiyor: {problem}"),
            "package.fields",
        ));
    }
    let mut body = p.body.clone();
    kentos_forms::field_refs::rebind_body(&mut body, &pairs);
    body.revision = 1;
    let form = ProjectForm {
        withdrawn: false,
        id,
        name: name.trim().to_owned(),
        layer: input.layer.clone(),
        draft: body,
        releases: Vec::new(),
        enforce: false,
        comments: Vec::new(),
    };
    let (fields_after, _) = kentos_contracts::follow_domains(&fields_after, &domains);
    let errors = kentos_forms::preflight_in(
        &form.draft,
        &fields_after,
        &forms::variables(doc),
        &doc.form_world(&form.draft),
    )
    .iter()
    .filter(|i| i.severity == FormSeverity::Error)
    .count() as u32;
    Ok(Planned {
        form,
        bound,
        fields: (!added.is_empty()).then_some(fields_after),
        added,
        domains: (!added_domains.is_empty()).then_some(domains),
        added_domains,
        errors,
        warnings,
    })
}

fn answer(p: &Planned, revision: String) -> FormsImported {
    FormsImported {
        id: p.form.id.clone(),
        bound: p.bound.clone(),
        added: p.added.clone(),
        domains: p.added_domains.clone(),
        errors: p.errors,
        revision,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsImport) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: (),
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsImport) -> CommandResult<FormsImported> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: answer(&p, cx.doc.revision().to_string()),
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the lists, the fields (one undo step) and the new form.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FormsImport) -> CommandResult<FormsImported> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_FORMS_IMPORT,
        kentos_contracts::CAD_FORMS_IMPORT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: FormsImport,
) -> CommandResult<FormsImported> {
    let p = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let written = doc.transact(LABEL, |doc| {
        if let Some(domains) = &p.domains {
            doc.set_domains(domains.clone(), LABEL)?;
        }
        if let Some(fields) = &p.fields {
            doc.set_layer_fields(&input.layer, fields.clone(), &[])?;
        }
        Ok::<(), kentos_domain::Refusal>(())
    });
    if let Err(refusal) = written {
        return fail(codes::INVALID_PACKAGE, refusal.0, "package").into();
    }
    let mut settings = doc.settings().clone();
    settings.forms.push(p.form.clone());
    doc.set_settings(settings);
    CommandResult::Completed {
        output: answer(&p, doc.revision().to_string()),
        warnings: p.warnings,
    }
}

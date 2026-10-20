//! `cad.fields.migrate` v1 (Değerleri biçime çevir, docs/adr/0439 §8): a layer's values written before its
//! fields were typed (an import's `1,5`, `17.10.2026`, `evet`, a listed value's label) turned into their
//! fields' canonical texts, as one undo step. Nothing is lost: a value that does not fit stays as written
//! and is counted, an empty one stays empty, an attribute no field names stays as it is. The shared cases
//! are `fixtures/commands/v1/cad.fields.migrate.json`.

use std::collections::BTreeSet;

use kentos_commands::{
    CommandResult, CommandWarning, Entity, FieldMigration, FieldsMigrate, FieldsMigrated,
    LayerField, LayerFieldKind, LayerNodeType, check_value, js_trim,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Değerleri biçime çevir";

/// The most values a field's report shows that do not fit.
const SAMPLES: usize = 5;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// What the migration writes and says.
struct Planned {
    fields: Vec<FieldMigration>,
    writes: Vec<(Slot, Entity)>,
    unknown: Vec<String>,
    unfit: Vec<String>,
}

/// Whether a field's values have a canonical text other than what may be written: every kind but text
/// without a list.
fn typed(f: &LayerField) -> bool {
    f.kind != LayerFieldKind::Text || f.values.as_ref().is_some_and(|v| !v.is_empty())
}

fn check(doc: &Document, input: &FieldsMigrate) -> Result<Planned, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
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
    if node.kind != LayerNodeType::Layer {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!(
                "“{}” bir katman grubu; alanlar yalnız katmanın olur. Grubun bir katmanını verin.",
                node.name
            ),
            "layer",
        ));
    }
    let chosen: Vec<&LayerField> = if input.fields.is_empty() {
        node.fields.iter().filter(|f| typed(f)).collect()
    } else {
        let mut out = Vec::new();
        for (i, key) in input.fields.iter().enumerate() {
            let Some(f) = node.fields.iter().find(|f| f.key() == key) else {
                return Err(fail(
                    codes::FIELD_NOT_FOUND,
                    format!("“{}” katmanında “{key}” alanı yok.", node.name),
                    &format!("fields/{i}"),
                ));
            };
            if !out.iter().any(|x: &&LayerField| x.key() == f.key()) {
                out.push(f);
            }
        }
        // The layer's order.
        node.fields
            .iter()
            .filter(|f| out.iter().any(|x| x.key() == f.key()))
            .collect()
    };
    let loose: Vec<LayerField> = chosen
        .iter()
        .map(|f| LayerField {
            required: false,
            ..(*f).clone()
        })
        .collect();
    let mut fields: Vec<FieldMigration> = chosen
        .iter()
        .map(|f| FieldMigration {
            field: f.key().to_owned(),
            changed: 0,
            kept: 0,
            unfit: 0,
            blank: 0,
            absent: 0,
            samples: Vec::new(),
        })
        .collect();
    let names: BTreeSet<&str> = node.fields.iter().map(|f| f.name.as_str()).collect();
    let mut unknown = BTreeSet::new();
    let mut writes = Vec::new();
    let mut unfit = Vec::new();
    for e in doc.by_layer(&input.layer) {
        let attrs = &e.base().attrs;
        unknown.extend(
            attrs
                .keys()
                .filter(|k| !names.contains(k.as_str()))
                .cloned(),
        );
        let mut next: Option<Entity> = None;
        for (f, m) in loose.iter().zip(fields.iter_mut()) {
            let Some(v) = attrs.get(&f.name) else {
                m.absent += 1;
                continue;
            };
            if js_trim(v).is_empty() {
                m.blank += 1;
                continue;
            }
            match check_value(f, v) {
                Ok(c) if c == *v => m.kept += 1,
                Ok(c) => {
                    m.changed += 1;
                    next.get_or_insert_with(|| e.clone())
                        .base_mut()
                        .attrs
                        .insert(f.name.clone(), c);
                }
                Err(_) => {
                    m.unfit += 1;
                    if m.samples.len() < SAMPLES {
                        m.samples.push(v.clone());
                    }
                    if unfit.len() < SAMPLES {
                        unfit.push(v.clone());
                    }
                }
            }
        }
        if let Some(n) = next {
            writes.push((Slot(e.base().id), n));
        }
    }
    if !writes.is_empty() && doc.layers().is_locked(&input.layer) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; değerleri biçime çevrilemez. Kilidi Katmanlar panelinden açın.",
                node.name
            ),
            "layer",
        ));
    }
    Ok(Planned {
        fields,
        writes,
        unknown: unknown.into_iter().collect(),
        unfit,
    })
}

fn warnings(p: &Planned) -> Vec<CommandWarning> {
    let n: u32 = p.fields.iter().map(|m| m.unfit).sum();
    if n == 0 {
        return Vec::new();
    }
    let said: Vec<String> = p.unfit.iter().map(|v| format!("“{v}”")).collect();
    vec![CommandWarning {
        code: codes::VALUES_UNFIT.into(),
        message: format!(
            "{n} değer alanına uymuyor; yazıldığı gibi bırakıldı ({}). Formla düzenle ya da Öznitelik tablosuyla düzeltin.",
            said.join(", ")
        ),
        path: Some("fields".into()),
    }]
}

fn answer(input: &FieldsMigrate, p: &Planned, revision: String) -> FieldsMigrated {
    FieldsMigrated {
        layer: input.layer.clone(),
        fields: p.fields.clone(),
        objects: p.writes.len() as u32,
        unknown: p.unknown.clone(),
        changed: !p.writes.is_empty(),
        revision,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FieldsMigrate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: (),
            warnings: warnings(&p),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &FieldsMigrate) -> CommandResult<FieldsMigrated> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: answer(input, &p, cx.doc.revision().to_string()),
            warnings: warnings(&p),
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the values in their canonical texts as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: FieldsMigrate,
) -> CommandResult<FieldsMigrated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_FIELDS_MIGRATE,
        kentos_commands::CAD_FIELDS_MIGRATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: FieldsMigrate,
) -> CommandResult<FieldsMigrated> {
    let p = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    if !p.writes.is_empty() {
        cx.doc
            .transact(LABEL, |doc| {
                doc.update_many(p.writes.clone(), LABEL);
                Ok::<(), kentos_domain::Refusal>(())
            })
            .ok();
    }
    CommandResult::Completed {
        output: answer(&input, &p, cx.doc.revision().to_string()),
        warnings: warnings(&p),
    }
}

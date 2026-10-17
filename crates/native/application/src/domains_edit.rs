//! `cad.domains.edit` v1 (Değer listeleri, docs/adr/0430): the project's value lists made, changed and
//! linked from layer fields, as one undo step “Değer listeleri”. A linked field's values are its list's
//! copy; the document writes the copies anew in the step (`Document::set_domains`). A code records hold
//! may not just leave a list: `recode` moves them to another code in the same step. The shared cases
//! are `fixtures/commands/v1/cad.domains.edit.json`.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{
    CommandResult, CommandWarning, DomainField, DomainsEdit, DomainsEdited, DomainsOp, Entity,
    FieldDomain, LayerField, LayerNodeType, MULTIPLE_JOIN, compare_decimals, domain_id_of,
    domain_uses, domains_problem, follow_domains, link_problem,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Değer listeleri";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// What the edit writes.
struct Planned {
    domains: Vec<FieldDomain>,
    /// Fields given a link or unlinked: (layer, the field as given).
    links: Vec<(String, LayerField)>,
    /// Whether the links go before the lists (an unlink before its list is taken away).
    links_first: bool,
    recodes: Vec<(Slot, Entity)>,
    domain: Option<String>,
    warnings: Vec<CommandWarning>,
}

fn unknown_domain(id: &str, path: &str) -> Stop {
    fail(
        codes::UNKNOWN_DOMAIN,
        format!(
            "“{id}” kimlikli bir değer listesi yok. Değer listeleri penceresinden var olan bir listeyi seçin."
        ),
        path,
    )
}

/// The layer `id` and its field `key`, checked.
fn field_of<'a>(doc: &'a Document, layer: &str, key: &str) -> Result<&'a LayerField, Stop> {
    let Some(node) = doc.layers().get(layer) else {
        return Err(fail(
            codes::LAYER_NOT_FOUND,
            format!(
                "“{layer}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
            ),
            "edit/layer",
        ));
    };
    if node.kind != LayerNodeType::Layer {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!(
                "“{}” bir katman grubu; alanlar yalnız katmanın olur. Grubun bir katmanını verin.",
                node.name
            ),
            "edit/layer",
        ));
    }
    node.fields.iter().find(|f| f.key() == key).ok_or_else(|| {
        fail(
            codes::FIELD_NOT_FOUND,
            format!("“{}” katmanında “{key}” alanı yok.", node.name),
            "edit/field",
        )
    })
}

fn domains_fit(domains: &[FieldDomain]) -> Result<(), Stop> {
    match domains_problem(domains) {
        Some((_, problem)) => Err(fail(codes::INVALID_DOMAIN, problem, "edit")),
        None => Ok(()),
    }
}

/// A record's codes in `value` as `field` holds them.
fn codes_in<'a>(field: &LayerField, value: &'a str) -> Vec<&'a str> {
    if field.holds_many() {
        field.codes_of(value)
    } else {
        vec![value]
    }
}

/// The records of the fields `uses` holding a code `list` lacks (after `recode`): their count and the
/// codes, sorted.
fn misfits(
    doc: &Document,
    uses: &[(String, String)],
    list: &[kentos_contracts::FieldChoice],
    recode: &BTreeMap<String, String>,
) -> (usize, BTreeSet<String>) {
    let mut n = 0;
    let mut codes = BTreeSet::new();
    for (layer, key) in uses {
        let Some(f) = doc
            .layers()
            .get(layer)
            .and_then(|l| l.fields.iter().find(|f| f.key() == key))
        else {
            continue;
        };
        for (_, value) in doc.values_of(layer, &f.name) {
            let lost: Vec<&str> = codes_in(f, &value)
                .into_iter()
                .map(|c| recode.get(c).map_or(c, String::as_str))
                .filter(|c| !list.iter().any(|x| x.code == *c))
                .collect();
            if !lost.is_empty() {
                n += 1;
                codes.extend(lost.into_iter().map(str::to_owned));
            }
        }
    }
    (n, codes)
}

/// The records of the fields `uses` whose number is outside `min`…`max`.
fn outside(doc: &Document, uses: &[(String, String)], d: &FieldDomain) -> usize {
    let mut n = 0;
    for (layer, key) in uses {
        let Some(f) = doc
            .layers()
            .get(layer)
            .and_then(|l| l.fields.iter().find(|f| f.key() == key))
        else {
            continue;
        };
        for (_, value) in doc.values_of(layer, &f.name) {
            let below = d
                .min
                .as_deref()
                .is_some_and(|m| compare_decimals(&value, m) == Some(Ordering::Less));
            let above = d
                .max
                .as_deref()
                .is_some_and(|m| compare_decimals(&value, m) == Some(Ordering::Greater));
            n += usize::from(below || above);
        }
    }
    n
}

fn outside_warning(n: usize) -> Vec<CommandWarning> {
    if n == 0 {
        return Vec::new();
    }
    vec![CommandWarning {
        code: codes::DOMAIN_VALUES_OUTSIDE.into(),
        message: format!("{n} kaydın değeri listenin aralığı dışında kaldı."),
        path: Some("edit".into()),
    }]
}

fn in_use_refusal(n: usize, codes: &BTreeSet<String>, path: &str) -> Stop {
    let said: Vec<&str> = codes.iter().map(String::as_str).collect();
    fail(
        codes::DOMAIN_CODE_IN_USE,
        format!(
            "{n} kayıt listeden çıkan kodları tutuyor ({}); recode ile başka bir koda taşıyın ya da kodu kullanımdan kaldırın.",
            said.join(", ")
        ),
        path,
    )
}

fn check(doc: &Document, input: &DomainsEdit) -> Result<Planned, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let now = &doc.settings().domains;
    let find = |id: &str| now.iter().find(|d| d.id == id);
    let taken = |id: &Option<String>| -> Result<(), Stop> {
        match id {
            Some(id) if find(id).is_some() => Err(fail(
                codes::DOMAIN_EXISTS,
                format!("“{id}” kimlikli bir değer listesi zaten var; başka bir kimlik verin."),
                "edit/id",
            )),
            _ => Ok(()),
        }
    };
    let fresh = |id: &Option<String>, name: &str| {
        id.clone().unwrap_or_else(|| {
            let ids: Vec<&str> = now.iter().map(|d| d.id.as_str()).collect();
            domain_id_of(name, &ids)
        })
    };
    let plain = |domain: Option<String>, domains: Vec<FieldDomain>| Planned {
        domains,
        links: Vec::new(),
        links_first: false,
        recodes: Vec::new(),
        domain,
        warnings: Vec::new(),
    };
    match &input.edit {
        DomainsOp::Create {
            id,
            name,
            description,
            domain_kind,
            values,
            min,
            max,
            min_exclusive,
            max_exclusive,
            unit,
        } => {
            taken(id)?;
            let d = FieldDomain {
                id: fresh(id, name),
                name: name.clone(),
                description: description.clone(),
                kind: *domain_kind,
                values: values.clone(),
                min: min.clone(),
                max: max.clone(),
                min_exclusive: *min_exclusive,
                max_exclusive: *max_exclusive,
                unit: unit.clone(),
            };
            let mut next = now.clone();
            let id = d.id.clone();
            next.push(d);
            domains_fit(&next)?;
            Ok(plain(Some(id), next))
        }
        DomainsOp::FromField {
            layer,
            field,
            name,
            id,
        } => {
            let f = field_of(doc, layer, field)?;
            taken(id)?;
            if f.values.is_none() && f.min.is_none() && f.max.is_none() {
                return Err(fail(
                    codes::INVALID_DOMAIN,
                    format!(
                        "“{}” alanının ne değer listesi ne aralığı var; listesi olan bir alan seçin.",
                        f.label()
                    ),
                    "edit/field",
                ));
            }
            let d = FieldDomain {
                id: fresh(id, name),
                name: name.clone(),
                description: None,
                kind: f.kind,
                values: f.values.clone(),
                min: if f.values.is_some() {
                    None
                } else {
                    f.min.clone()
                },
                max: if f.values.is_some() {
                    None
                } else {
                    f.max.clone()
                },
                min_exclusive: f.values.is_none() && f.min_exclusive,
                max_exclusive: f.values.is_none() && f.max_exclusive,
                unit: f.unit.clone(),
            };
            let mut next = now.clone();
            next.push(d.clone());
            domains_fit(&next)?;
            if let Some(problem) = link_problem(&d, f) {
                return Err(fail(codes::DOMAIN_MISMATCH, problem, "edit/field"));
            }
            let mut planned = plain(Some(d.id.clone()), next);
            planned.links.push((
                layer.clone(),
                LayerField {
                    domain: Some(d.id),
                    ..f.clone()
                },
            ));
            Ok(planned)
        }
        DomainsOp::Update {
            id,
            name,
            description,
            values,
            min,
            max,
            min_exclusive,
            max_exclusive,
            unit,
            recode,
        } => {
            let Some(old) = find(id) else {
                return Err(unknown_domain(id, "edit/id"));
            };
            let d = FieldDomain {
                id: id.clone(),
                name: name.clone(),
                description: description.clone(),
                kind: old.kind,
                values: values.clone(),
                min: min.clone(),
                max: max.clone(),
                min_exclusive: *min_exclusive,
                max_exclusive: *max_exclusive,
                unit: unit.clone(),
            };
            let next: Vec<FieldDomain> = now
                .iter()
                .map(|o| if o.id == *id { d.clone() } else { o.clone() })
                .collect();
            domains_fit(&next)?;
            let uses = domain_uses(doc.layers().nodes(), id);
            for (layer, key) in &uses {
                if let Ok(f) = field_of(doc, layer, key)
                    && let Some(problem) = link_problem(&d, f)
                {
                    return Err(fail(codes::DOMAIN_MISMATCH, problem, "edit"));
                }
            }
            let old_codes = old.values.as_deref().unwrap_or_default();
            let new_codes = d.values.as_deref().unwrap_or_default();
            for (from, to) in recode {
                if !old_codes.iter().any(|c| c.code == *from) {
                    return Err(fail(
                        codes::INVALID_RECODE,
                        format!("“{from}” kodu “{}” listesinde yok.", old.name),
                        &format!("edit/recode/{from}"),
                    ));
                }
                if !new_codes.iter().any(|c| c.code == *to && !c.retired) {
                    return Err(fail(
                        codes::INVALID_RECODE,
                        format!("“{to}” kodu yeni listede yok ya da kullanımdan kalkmış."),
                        &format!("edit/recode/{from}"),
                    ));
                }
            }
            let mut planned = plain(Some(id.clone()), next);
            if d.coded() {
                let (n, lost) = misfits(doc, &uses, new_codes, recode);
                if n > 0 {
                    return Err(in_use_refusal(n, &lost, "edit/values"));
                }
            } else {
                planned.warnings = outside_warning(outside(doc, &uses, &d));
            }
            for (layer, key) in &uses {
                let Ok(f) = field_of(doc, layer, key) else {
                    continue;
                };
                for (slot, value) in doc.values_of(layer, &f.name) {
                    let codes = codes_in(f, &value);
                    if !codes.iter().any(|c| recode.contains_key(*c)) {
                        continue;
                    }
                    if doc.layers().is_locked(layer) {
                        return Err(fail(
                            codes::LAYER_LOCKED,
                            format!(
                                "“{}” katmanı kilitli; kayıtlarının kodu değiştirilemez. Kilidi Katmanlar panelinden açın.",
                                doc.layers()
                                    .get(layer)
                                    .map_or(layer.as_str(), |n| n.name.as_str())
                            ),
                            "edit/recode",
                        ));
                    }
                    let mut moved: Vec<&str> = Vec::new();
                    for c in codes {
                        let c = recode.get(c).map_or(c, String::as_str);
                        if !moved.contains(&c) {
                            moved.push(c);
                        }
                    }
                    let Some(mut entity) = doc.get(slot).cloned() else {
                        continue;
                    };
                    entity
                        .base_mut()
                        .attrs
                        .insert(f.name.clone(), moved.join(MULTIPLE_JOIN));
                    planned.recodes.push((slot, entity));
                }
            }
            Ok(planned)
        }
        DomainsOp::Remove { id, detach } => {
            let Some(old) = find(id) else {
                return Err(unknown_domain(id, "edit/id"));
            };
            let uses = domain_uses(doc.layers().nodes(), id);
            if !uses.is_empty() && !detach {
                let said: Vec<String> = uses
                    .iter()
                    .map(|(layer, key)| {
                        let node = doc.layers().get(layer);
                        let name = node.map_or(layer.as_str(), |n| n.name.as_str());
                        let label = node
                            .and_then(|n| n.fields.iter().find(|f| f.key() == key))
                            .map_or(key.as_str(), |f| f.label());
                        format!("{name} › {label}")
                    })
                    .collect();
                return Err(fail(
                    codes::DOMAIN_IN_USE,
                    format!(
                        "“{}” listesine {} alan bağlı ({}); bağları çözün ya da detach ile silin.",
                        old.name,
                        uses.len(),
                        said.join(", ")
                    ),
                    "edit/id",
                ));
            }
            let next: Vec<FieldDomain> = now.iter().filter(|d| d.id != *id).cloned().collect();
            let mut planned = plain(Some(id.clone()), next);
            planned.links_first = true;
            for (layer, key) in uses {
                if let Ok(f) = field_of(doc, &layer, &key) {
                    let f = LayerField {
                        domain: None,
                        ..f.clone()
                    };
                    planned.links.push((layer, f));
                }
            }
            Ok(planned)
        }
        DomainsOp::Link {
            layer,
            field,
            domain,
        } => {
            let d = match domain {
                Some(id) => Some(find(id).ok_or_else(|| unknown_domain(id, "edit/domain"))?),
                None => None,
            };
            let f = field_of(doc, layer, field)?;
            let mut planned = plain(domain.clone(), now.clone());
            if let Some(d) = d {
                if let Some(problem) = link_problem(d, f) {
                    return Err(fail(codes::DOMAIN_MISMATCH, problem, "edit/field"));
                }
                let uses = vec![(layer.clone(), field.clone())];
                match &d.values {
                    Some(list) => {
                        let (n, lost) = misfits(doc, &uses, list, &BTreeMap::new());
                        if n > 0 {
                            return Err(in_use_refusal(n, &lost, "edit/domain"));
                        }
                    }
                    None => planned.warnings = outside_warning(outside(doc, &uses, d)),
                }
            }
            if f.domain != *domain {
                planned.links.push((
                    layer.clone(),
                    LayerField {
                        domain: domain.clone(),
                        ..f.clone()
                    },
                ));
            }
            Ok(planned)
        }
    }
}

/// The fields whose definition `p` changes, in the layer tree's order.
fn changed_fields(doc: &Document, p: &Planned) -> Vec<DomainField> {
    let mut out = Vec::new();
    for node in doc.layers().leaves() {
        let given: Vec<LayerField> = node
            .fields
            .iter()
            .map(|f| {
                p.links
                    .iter()
                    .find(|(l, g)| *l == node.id && g.key() == f.key())
                    .map_or_else(|| f.clone(), |(_, g)| g.clone())
            })
            .collect();
        let (after, _) = follow_domains(&given, &p.domains);
        for (a, b) in node.fields.iter().zip(&after) {
            if a != b {
                out.push(DomainField {
                    layer: node.id.clone(),
                    field: a.key().to_owned(),
                });
            }
        }
    }
    out
}

fn answer(doc: &Document, p: &Planned, revision: String) -> DomainsEdited {
    let fields = changed_fields(doc, p);
    let changed =
        p.domains != doc.settings().domains || !fields.is_empty() || !p.recodes.is_empty();
    DomainsEdited {
        domain: p.domain.clone(),
        fields,
        recoded: p.recodes.len() as u32,
        changed,
        revision,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &DomainsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: (),
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &DomainsEdit) -> CommandResult<DomainsEdited> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: answer(cx.doc, &p, cx.doc.revision().to_string()),
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the lists, the linked fields' copies and the records moved as one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: DomainsEdit) -> CommandResult<DomainsEdited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_DOMAINS_EDIT,
        kentos_contracts::CAD_DOMAINS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: DomainsEdit,
) -> CommandResult<DomainsEdited> {
    let p = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let before = answer(cx.doc, &p, String::new());
    if !before.changed {
        return CommandResult::Completed {
            output: DomainsEdited {
                revision: cx.doc.revision().to_string(),
                ..before
            },
            warnings: p.warnings,
        };
    }
    let doc = &mut *cx.doc;
    let links = |doc: &mut Document| -> Result<(), kentos_domain::Refusal> {
        for (layer, field) in &p.links {
            let Some(node) = doc.layers().get(layer) else {
                continue;
            };
            let fields: Vec<LayerField> = node
                .fields
                .iter()
                .map(|f| {
                    if f.key() == field.key() {
                        field.clone()
                    } else {
                        f.clone()
                    }
                })
                .collect();
            doc.set_layer_fields(layer, fields, &[])?;
        }
        Ok(())
    };
    let written = doc.transact(LABEL, |doc| {
        if p.links_first {
            links(doc)?;
        }
        doc.set_domains(p.domains.clone(), LABEL)?;
        if !p.links_first {
            links(doc)?;
        }
        doc.update_many(p.recodes.clone(), LABEL);
        Ok::<(), kentos_domain::Refusal>(())
    });
    match written {
        Ok(()) => CommandResult::Completed {
            output: DomainsEdited {
                revision: doc.revision().to_string(),
                ..before
            },
            warnings: p.warnings,
        },
        Err(refusal) => fail(codes::INVALID_DOMAIN, refusal.0, "edit").into(),
    }
}

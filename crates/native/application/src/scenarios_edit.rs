//! `cad.scenarios.edit` v1 (docs/adr/0210 §9, §11): Senaryo oluştur makes a
//! scenario group on top of the tree, hidden (the view stays Mevcut durum
//! until the interface shows it), with a copy of each base layer given
//! standing for its source (`replaces`) and, when asked, copies of its
//! objects under new ids; Senaryoyu uygula moves each scenario layer's
//! objects onto its base layer in place of that layer's own, removes the
//! emptied scenario layers and makes the group an ordinary one keeping its
//! other layers (or removes it when it keeps none); a kept layer standing for
//! a base layer the tree no longer has loses that link. Each is one undo step.
//! The desktop's handler over the native document; the web's is
//! `apps/web/src/product/scenariosEdit.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.scenarios.edit.json`.

use std::collections::HashSet;

use kentos_contracts::{
    CommandResult, Entity, LayerNode, LayerNodeType, SCENARIO_NAME_MAX, ScenarioInfo,
    ScenarioOperation, ScenarioPair, ScenariosEdit, ScenariosEditPlan, ScenariosEdited,
    scenario_pairs,
};
use kentos_domain::{Document, NewLayer, Refusal, Slot, Temporal};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_blank};
/// The stable codes of the answers (`CommandError.code`).
pub use crate::codes;

/// The undo step's name.
pub fn label(op: ScenarioOperation) -> &'static str {
    match op {
        ScenarioOperation::Create => "Senaryo oluştur",
        ScenarioOperation::Apply => "Senaryoyu uygula",
    }
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

/// Whether the node or a group above it is a scenario.
fn in_scenario(doc: &Document, id: &str) -> bool {
    let mut at = doc.layers().get(id);
    while let Some(n) = at {
        if n.scenario.is_some() {
            return true;
        }
        at = doc.layers().parent(&n.id);
    }
    false
}

/// What a checked input names.
enum Checked {
    /// The base layers to copy, in order.
    Create(Vec<LayerNode>),
    /// The scenario group and its (base, scenario layer) pairs.
    Apply(Box<LayerNode>, Vec<(String, String)>),
}

fn check(doc: &Document, input: &ScenariosEdit) -> Result<Checked, Stop> {
    match input.operation {
        ScenarioOperation::Create => {
            let name = input.name.as_deref().unwrap_or("");
            if is_blank(name) {
                return Err(fail(
                    codes::EMPTY_NAME,
                    "Senaryonun adı boş olamaz; bir ad verin.".into(),
                    "name",
                ));
            }
            if name.trim().chars().count() > SCENARIO_NAME_MAX {
                return Err(fail(
                    codes::INVALID_NAME,
                    format!("Senaryonun adı {SCENARIO_NAME_MAX} karakterden uzun; kısaltın."),
                    "name",
                ));
            }
            let info = ScenarioInfo {
                note: input.note.clone(),
            };
            if let Some(problem) = info.problem() {
                return Err(fail(
                    codes::INVALID_NOTE,
                    format!("Senaryonun notu: {problem}."),
                    "note",
                ));
            }
            let layers = input.layers.as_deref().unwrap_or_default();
            let mut seen = HashSet::new();
            for (i, id) in layers.iter().enumerate() {
                if !seen.insert(id.as_str()) {
                    return Err(fail(
                        codes::DUPLICATE_LAYER,
                        format!("“{id}” katmanı iki kez verildi; her katman bir kez kopyalanır."),
                        &format!("layers/{i}"),
                    ));
                }
            }
            checks::revision(doc, input.expected_revision.as_deref())?;
            let copy = input.copy_objects.unwrap_or(true);
            let mut sources = Vec::with_capacity(layers.len());
            for (i, id) in layers.iter().enumerate() {
                let at = format!("layers/{i}");
                let Some(node) = doc.layers().get(id) else {
                    return Err(fail(
                        codes::LAYER_NOT_FOUND,
                        format!(
                            "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                        ),
                        &at,
                    ));
                };
                let name = &node.name;
                if node.kind != LayerNodeType::Layer
                    || in_scenario(doc, id)
                    || node.service.is_some()
                {
                    return Err(fail(
                        codes::NOT_A_BASE_LAYER,
                        format!(
                            "“{name}” bir ana katman değil; senaryo yalnız senaryo dışındaki, nesne tutan katmanları kopyalar."
                        ),
                        &at,
                    ));
                }
                if copy && doc.layers().is_locked(id) {
                    return Err(fail(
                        codes::LAYER_LOCKED,
                        format!(
                            "“{name}” katmanı kilitli; nesneleri kopyalanmaz. Kilidi Katmanlar panelinden açın."
                        ),
                        &at,
                    ));
                }
                sources.push(node.clone());
            }
            Ok(Checked::Create(sources))
        }
        ScenarioOperation::Apply => {
            let Some(id) = input.scenario.as_deref().filter(|s| !s.is_empty()) else {
                return Err(fail(
                    codes::NO_SCENARIO,
                    "Uygulanacak senaryonun kimliğini (scenario) verin.".into(),
                    "scenario",
                ));
            };
            checks::revision(doc, input.expected_revision.as_deref())?;
            let Some(group) = doc
                .layers()
                .get(id)
                .filter(|n| n.kind == LayerNodeType::Group && n.scenario.is_some())
            else {
                return Err(fail(
                    codes::SCENARIO_NOT_FOUND,
                    format!(
                        "“{id}” kimlikli senaryo çizimde yok. Bir senaryo grubunun kimliğini verin."
                    ),
                    "scenario",
                ));
            };
            let pairs: Vec<(String, String)> = scenario_pairs(doc.layers().nodes(), group)
                .into_iter()
                .map(|(b, l)| (b.to_owned(), l.to_owned()))
                .collect();
            let locked = doc
                .layers()
                .leaves_of(id)
                .into_iter()
                .map(|l| l.id.clone())
                .chain(pairs.iter().map(|(b, _)| b.clone()))
                .find(|l| doc.layers().is_locked(l));
            if let Some(l) = locked {
                let name = doc.layers().get(&l).map_or(l.as_str(), |n| n.name.as_str());
                return Err(fail(
                    codes::LAYER_LOCKED,
                    format!(
                        "“{name}” katmanı kilitli; senaryo uygulanmadı. Kilidi Katmanlar panelinden açın."
                    ),
                    "scenario",
                ));
            }
            Ok(Checked::Apply(Box::new(group.clone()), pairs))
        }
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ScenariosEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The scenario group's layers that stand for no base layer of the tree, in order.
fn kept(doc: &Document, group: &str, pairs: &[(String, String)]) -> Vec<String> {
    doc.layers()
        .leaves_of(group)
        .into_iter()
        .filter(|l| !pairs.iter().any(|(_, s)| *s == l.id))
        .map(|l| l.id.clone())
        .collect()
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ScenariosEdit) -> CommandResult<ScenariosEditPlan> {
    let doc = &*cx.doc;
    let checked = match check(doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let output = match checked {
        Checked::Create(sources) => {
            let ids = doc.layers().next_ids(1 + sources.len());
            let copy = input.copy_objects.unwrap_or(true);
            let objects = if copy {
                sources.iter().map(|s| doc.by_layer(&s.id).count()).sum()
            } else {
                0
            };
            ScenariosEditPlan {
                scenario: ids[0].clone(),
                layers: sources
                    .iter()
                    .zip(&ids[1..])
                    .map(|(s, id)| ScenarioPair {
                        base: s.id.clone(),
                        layer: id.clone(),
                    })
                    .collect(),
                kept: Vec::new(),
                objects: count(objects),
                removed: 0,
                revision: doc.revision().to_string(),
            }
        }
        Checked::Apply(group, pairs) => ScenariosEditPlan {
            scenario: group.id.clone(),
            kept: kept(doc, &group.id, &pairs),
            objects: count(pairs.iter().map(|(_, l)| doc.by_layer(l).count()).sum()),
            removed: count(pairs.iter().map(|(b, _)| doc.by_layer(b).count()).sum()),
            layers: pairs
                .into_iter()
                .map(|(base, layer)| ScenarioPair { base, layer })
                .collect(),
            revision: doc.revision().to_string(),
        },
    };
    CommandResult::Completed {
        output,
        warnings: Vec::new(),
    }
}

/// The objects of `layer` given to `to` (copies for a new layer keep nothing of their slots).
fn onto(doc: &Document, layer: &str, to: &str) -> Vec<(Slot, Entity)> {
    doc.by_layer(layer)
        .map(|e| {
            let mut c = e.clone();
            to.clone_into(&mut c.base_mut().layer_id);
            (Slot(e.base().id), c)
        })
        .collect()
}

fn create(
    doc: &mut Document,
    input: &ScenariosEdit,
    sources: &[LayerNode],
) -> Result<ScenariosEdited, Refusal> {
    let step = label(ScenarioOperation::Create);
    let name = input.name.as_deref().unwrap_or("").trim().to_owned();
    let group = doc.add_layer_at(
        NewLayer {
            visible: false,
            scenario: Some(ScenarioInfo {
                note: input.note.clone(),
            }),
            ..NewLayer::group(name)
        },
        None,
        Some(0),
        Some(step),
        false,
    )?;
    let copy = input.copy_objects.unwrap_or(true);
    let mut pairs = Vec::with_capacity(sources.len());
    let mut objects = 0;
    for src in sources {
        let layer = doc.add_layer_at(
            NewLayer {
                style: src.style.clone(),
                snap: src.snap.clone(),
                fields: src.fields.clone(),
                time: src.time.clone(),
                replaces: Some(src.id.clone()),
                ..NewLayer::layer(src.name.clone())
            },
            Some(&group),
            None,
            Some(step),
            false,
        )?;
        if copy {
            let copies: Vec<Entity> = onto(doc, &src.id, &layer)
                .into_iter()
                .map(|(_, e)| e)
                .collect();
            objects += copies.len();
            doc.add_many(copies, step).map_err(|_| {
                Refusal("Çizimin bütün nesne yuvaları dolu; nesneler kopyalanamadı.".into())
            })?;
        }
        pairs.push(ScenarioPair {
            base: src.id.clone(),
            layer,
        });
    }
    Ok(ScenariosEdited {
        scenario: group,
        layers: pairs,
        kept: Vec::new(),
        objects: count(objects),
        removed: 0,
        revision: String::new(),
    })
}

fn apply(
    doc: &mut Document,
    group: &LayerNode,
    pairs: &[(String, String)],
) -> Result<ScenariosEdited, Refusal> {
    let step = label(ScenarioOperation::Apply);
    let (mut moved, mut removed) = (0, 0);
    for (base, layer) in pairs {
        let old: Vec<Slot> = doc.by_layer(base).map(|e| Slot(e.base().id)).collect();
        removed += doc.remove(&old);
        moved += doc.update_many(onto(doc, layer, base), step);
        if doc.layers().active() == layer {
            doc.set_active_layer(base);
        }
        doc.remove_layer(layer)?;
    }
    let kept = kept(doc, &group.id, pairs);
    // A layer standing for a base layer the tree no longer has stays as a base layer itself, its link dropped.
    for id in &kept {
        if let Some(now) = doc
            .layers()
            .temporal_of(id)
            .filter(|t| t.replaces.is_some())
        {
            doc.set_layer_temporal(
                id,
                Temporal {
                    replaces: None,
                    ..now
                },
                step,
            )?;
        }
    }
    let still = doc
        .layers()
        .get(&group.id)
        .is_some_and(|g| !g.children.is_empty());
    if still {
        let now = doc.layers().temporal_of(&group.id).unwrap_or_default();
        doc.set_layer_temporal(
            &group.id,
            Temporal {
                scenario: None,
                ..now
            },
            step,
        )?;
    } else {
        doc.remove_layer(&group.id)?;
    }
    Ok(ScenariosEdited {
        scenario: group.id.clone(),
        layers: pairs
            .iter()
            .map(|(base, layer)| ScenarioPair {
                base: base.clone(),
                layer: layer.clone(),
            })
            .collect(),
        kept,
        objects: count(moved),
        removed: count(removed),
        revision: String::new(),
    })
}

/// Writes the scenario's change as one undo step named after the operation;
/// a refusal halfway takes back what was written.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ScenariosEdit,
) -> CommandResult<ScenariosEdited> {
    let checked = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let group = doc.begin_group(label(input.operation));
    let written = match &checked {
        Checked::Create(sources) => create(doc, &input, sources),
        Checked::Apply(node, pairs) => apply(doc, node, pairs),
    };
    match written {
        Ok(mut output) => {
            doc.end_group(group);
            output.revision = doc.revision().to_string();
            CommandResult::Completed {
                output,
                warnings: Vec::new(),
            }
        }
        Err(refusal) => {
            doc.cancel_group(group);
            fail(codes::LAYER_REFUSED, refusal.0, "layers").into()
        }
    }
}

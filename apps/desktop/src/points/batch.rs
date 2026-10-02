//! Nokta editörü's batch operations (docs/adr/0153 §5; the web's
//! `ui/bottom/pointBatch.ts`): Yeniden adlandır, Sıralı numara ver and Katmana
//! taşı over the table's target rows, each one undo step named after it,
//! written through `cad.entities.set`. fixtures/point-editor/v1/batch.json
//! holds both platforms to the same drawing, messages and steps.

use std::collections::HashMap;

use kentos_contracts::{
    CommandResult, EntitiesSetProperties, Entity, PointEntity, PropertiesOperation,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::text::edit::increment;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, set};

use super::edit::{PREFIX, in_step, refusal};

/// An operation's kind: its window and its undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rename,
    Number,
    Layer,
}

impl Kind {
    /// The undo step, and the window's title.
    pub fn step(self) -> &'static str {
        match self {
            Kind::Rename => "Yeniden adlandır",
            Kind::Number => "Sıralı numara ver",
            Kind::Layer => "Katmana taşı",
        }
    }
}

/// An operation as its window gives it.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// Önek ekle (`add`) or Önek kaldır.
    Rename {
        add: bool,
        prefix: String,
    },
    Number {
        start: String,
    },
    Layer {
        layer: String,
    },
}

impl Op {
    pub fn kind(&self) -> Kind {
        match self {
            Op::Rename { .. } => Kind::Rename,
            Op::Number { .. } => Kind::Number,
            Op::Layer { .. } => Kind::Layer,
        }
    }
}

/// What came of an operation: what to say, and the undo step written (none: nothing).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    pub said: Vec<String>,
    pub step: Option<&'static str>,
}

/// The rows an operation takes (docs/adr/0153 §5): the table's selected rows
/// in its order, or every row when none of them is selected; and the header
/// naming them. A selected point the table does not show takes no part.
pub fn targets(shown: &[Slot], selected: impl Fn(Slot) -> bool) -> (Vec<Slot>, String) {
    let picked: Vec<Slot> = shown.iter().copied().filter(|&s| selected(s)).collect();
    if picked.is_empty() {
        (shown.to_vec(), format!("Tablodaki {} nokta", shown.len()))
    } else {
        let header = format!("{} seçili nokta", picked.len());
        (picked, header)
    }
}

/// The names the points would take, in order (none: the point keeps its
/// own), or why none can be given (the web's `plannedNames`).
pub fn planned_names(points: &[&PointEntity], op: &Op) -> Result<Vec<Option<String>>, String> {
    match op {
        Op::Rename { add, prefix } => {
            let prefix = js_trim(prefix);
            if prefix.is_empty() {
                return Err(format!("{PREFIX}Önek yazılmalı."));
            }
            Ok(points
                .iter()
                .map(|p| {
                    let name = p.base.label.as_deref().map(js_trim).unwrap_or_default();
                    if *add {
                        (!name.is_empty()).then(|| format!("{prefix}{name}"))
                    } else {
                        name.strip_prefix(prefix)
                            .map(js_trim)
                            .filter(|rest| !rest.is_empty())
                            .map(str::to_owned)
                    }
                })
                .collect())
        }
        Op::Number { start } => {
            let start = js_trim(start);
            if !start.as_bytes().last().is_some_and(u8::is_ascii_digit) {
                return Err(format!("{PREFIX}Başlangıç adı sayıyla bitmeli."));
            }
            let mut name = start.to_owned();
            Ok(points
                .iter()
                .map(|_| {
                    let this = name.clone();
                    name = increment(&name).unwrap_or_else(|| name.clone());
                    Some(this)
                })
                .collect())
        }
        Op::Layer { .. } => Ok(points.iter().map(|_| None).collect()),
    }
}

/// What an operation would change, for its window: why it may not, or the
/// points that change (with their new name; a layer move keeps the name).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub error: Option<String>,
    pub changes: Vec<(Slot, Option<String>)>,
}

fn points_of(doc: &Document, slots: &[Slot]) -> Vec<(Slot, PointEntity)> {
    slots
        .iter()
        .filter_map(|&s| match doc.get(s) {
            Some(Entity::Point(p)) => Some((s, p.clone())),
            _ => None,
        })
        .collect()
}

pub fn plan(doc: &Document, slots: &[Slot], op: &Op) -> Plan {
    let points = points_of(doc, slots);
    if let Op::Layer { layer } = op {
        return Plan {
            error: None,
            changes: points
                .iter()
                .filter(|(_, p)| &p.base.layer_id != layer)
                .map(|(s, _)| (*s, None))
                .collect(),
        };
    }
    let refs: Vec<&PointEntity> = points.iter().map(|(_, p)| p).collect();
    match planned_names(&refs, op) {
        Err(error) => Plan {
            error: Some(error),
            changes: Vec::new(),
        },
        Ok(names) => Plan {
            error: None,
            changes: points
                .iter()
                .zip(names)
                .filter_map(|((s, p), name)| {
                    name.filter(|n| p.base.label.as_deref() != Some(n.as_str()))
                        .map(|n| (*s, Some(n)))
                })
                .collect(),
        },
    }
}

fn input(uids: Vec<String>, label: Option<String>, layer: Option<String>) -> EntitiesSetProperties {
    EntitiesSetProperties {
        operation: if layer.is_some() {
            PropertiesOperation::Layer
        } else {
            PropertiesOperation::Label
        },
        uids,
        layer_id: layer,
        color: None,
        line_weight: None,
        symbol: None,
        attrs: None,
        label: label.map(Some),
        expected_revision: None,
    }
}

/// The operation written as one undo step named after it (docs/adr/0153 §5;
/// the web's `runBatch`). Only the points that change are written, names one
/// by one in the targets' order, a layer move in one call; a refusal of the
/// command is said as it is and nothing is written. Then it is said what
/// changed: for names, also how many of the new names another point has too;
/// for a move, the command's warning (a hidden layer).
pub fn run(doc: &mut Document, slots: &[Slot], op: &Op) -> Outcome {
    let plan = plan(doc, slots, op);
    if let Some(error) = plan.error {
        return Outcome {
            said: vec![error],
            step: None,
        };
    }
    let step = op.kind().step();
    let uid = |doc: &Document, s: Slot| doc.uid(s).map(|u| u.to_string()).unwrap_or_default();
    if let Op::Layer { layer } = op {
        if plan.changes.is_empty() {
            return Outcome {
                said: vec![format!("{PREFIX}Taşınacak nokta yok.")],
                step: None,
            };
        }
        let uids: Vec<String> = plan.changes.iter().map(|(s, _)| uid(doc, *s)).collect();
        let mut warnings = Vec::new();
        let refused = in_step(doc, step, |doc| {
            match set::execute(
                &mut ExecutionContext::new(doc),
                input(uids, None, Some(layer.clone())),
            ) {
                CommandResult::Completed { warnings: w, .. } => {
                    warnings = w.into_iter().map(|w| w.message).collect();
                    None
                }
                other => Some(
                    refusal(other)
                        .map(|e| e.message)
                        .unwrap_or_else(|| format!("{PREFIX}noktalar taşınamadı.")),
                ),
            }
        });
        if let Some(refused) = refused {
            return Outcome {
                said: vec![refused],
                step: None,
            };
        }
        let name = doc
            .layers()
            .get(layer)
            .map_or_else(|| layer.clone(), |n| n.name.clone());
        let mut said = vec![format!(
            "{PREFIX}{} nokta “{name}” katmanına taşındı.",
            plan.changes.len()
        )];
        said.extend(warnings);
        return Outcome {
            said,
            step: Some(step),
        };
    }
    if plan.changes.is_empty() {
        return Outcome {
            said: vec![format!("{PREFIX}Adı değişen nokta yok.")],
            step: None,
        };
    }
    let writes: Vec<(String, String)> = plan
        .changes
        .iter()
        .map(|(s, name)| (uid(doc, *s), name.clone().unwrap_or_default()))
        .collect();
    let refused = in_step(doc, step, |doc| {
        for (uid, name) in writes {
            let result = set::execute(
                &mut ExecutionContext::new(doc),
                input(vec![uid], Some(name), None),
            );
            if !matches!(result, CommandResult::Completed { .. }) {
                return Some(
                    refusal(result)
                        .map(|e| e.message)
                        .unwrap_or_else(|| format!("{PREFIX}ad yazılamadı.")),
                );
            }
        }
        None
    });
    if let Some(refused) = refused {
        return Outcome {
            said: vec![refused],
            step: None,
        };
    }
    // The new names are trimmed: one counted twice is another point's too.
    let mut count: HashMap<&str, usize> = HashMap::new();
    for e in doc.entities() {
        if let Entity::Point(p) = e
            && let Some(name) = p.base.label.as_deref().map(js_trim)
            && !name.is_empty()
        {
            *count.entry(name).or_default() += 1;
        }
    }
    let shared = plan
        .changes
        .iter()
        .filter(|(_, name)| {
            count
                .get(name.as_deref().unwrap_or_default())
                .copied()
                .unwrap_or(0)
                > 1
        })
        .count();
    let n = plan.changes.len();
    let said = if shared > 0 {
        format!("{PREFIX}{n} noktanın adı değişti; {shared} ad başka noktalarda da var.")
    } else {
        format!("{PREFIX}{n} noktanın adı değişti.")
    };
    Outcome {
        said: vec![said],
        step: Some(step),
    }
}

//! Nokta editörü's batch operations (docs/adr/0153 §5; the web's
//! `ui/bottom/pointBatch.ts`): Yeniden adlandır, Sıralı numara ver and Katmana
//! taşı over the table's target rows, written through `cad.entities.set`, and
//! Çift noktaları ayıkla, through `cad.entities.edit` and
//! `cad.entities.delete`; each one undo step named after it.
//! fixtures/point-editor/v1/batch.json and dedupe.json hold both platforms to
//! the same drawing, messages and steps.

use std::collections::HashMap;

use kentos_contracts::{
    CommandResult, EditOperation, EntitiesDelete, EntitiesEdit, EntitiesSetProperties, Entity,
    EntityEdit, EntityGeometry, PointEntity, PropertiesOperation,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::ops::point_editor::{
    DupBy, DupPoint, Keep, duplicate_points, follow_point,
};
use kentos_geometry_core::text::edit::increment;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::elevation::paths as elevated_paths;
use kentos_native_application::{ExecutionContext, delete, edit, set};

use super::edit::{FOLLOW_LOCKED, PREFIX, in_step, refusal, with_paths};

/// An operation's kind: its window and its undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rename,
    Number,
    Layer,
    Dedupe,
}

impl Kind {
    /// The undo step, and the window's title.
    pub fn step(self) -> &'static str {
        match self {
            Kind::Rename => "Yeniden adlandır",
            Kind::Number => "Sıralı numara ver",
            Kind::Layer => "Katmana taşı",
            Kind::Dedupe => "Çift noktaları ayıkla",
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
    /// Aynı ad (`by_name`) or Aynı yer within `tolerance` (the window's text, metres).
    Dedupe {
        by_name: bool,
        tolerance: String,
        keep: Keep,
    },
}

impl Op {
    pub fn kind(&self) -> Kind {
        match self {
            Op::Rename { .. } => Kind::Rename,
            Op::Number { .. } => Kind::Number,
            Op::Layer { .. } => Kind::Layer,
            Op::Dedupe { .. } => Kind::Dedupe,
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

/// After İçe aktar from the table (docs/adr/0153 §5): the imported points'
/// names (trimmed; one without a name is left out) that two points or more of
/// the drawing carry, and the points carrying them in the drawing's order,
/// for Çift noktaları ayıkla by Aynı ad (İlki keeps the drawing's, Sonuncusu
/// the file's), with the window's header; none when there are none: no
/// window (the web's `importTargets`).
pub fn import_targets(doc: &Document, imported: &[Slot]) -> Option<(Vec<Slot>, String)> {
    fn name_of(e: Option<&Entity>) -> &str {
        match e {
            Some(Entity::Point(p)) => p.base.label.as_deref().map_or("", js_trim),
            _ => "",
        }
    }
    let names: std::collections::HashSet<&str> = imported
        .iter()
        .map(|&s| name_of(doc.get(s)))
        .filter(|n| !n.is_empty())
        .collect();
    if names.is_empty() {
        return None;
    }
    // The table's points: a multi-point object is none of them (docs/adr/0174).
    let points: Vec<&Entity> = doc
        .entities()
        .filter(|e| matches!(e, Entity::Point(p) if p.parts.is_none()))
        .collect();
    let mut carried: HashMap<&str, usize> = HashMap::new();
    for &e in &points {
        let name = name_of(Some(e));
        if names.contains(name) {
            *carried.entry(name).or_default() += 1;
        }
    }
    let slots: Vec<Slot> = points
        .iter()
        .filter(|&&e| carried.get(name_of(Some(e))).is_some_and(|&n| n > 1))
        .map(|e| Slot(e.base().id))
        .collect();
    if slots.is_empty() {
        return None;
    }
    let header = format!("Aynı adlı {} nokta", slots.len());
    Some((slots, header))
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
        Op::Layer { .. } | Op::Dedupe { .. } => Ok(points.iter().map(|_| None).collect()),
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
            Some(Entity::Point(p)) if p.parts.is_none() => Some((s, p.clone())),
            _ => None,
        })
        .collect()
}

pub fn plan(doc: &Document, slots: &[Slot], op: &Op) -> Plan {
    let points = points_of(doc, slots);
    if let Op::Dedupe { .. } = op {
        let d = plan_dedupe(doc, slots, op);
        return Plan {
            error: d.error,
            changes: d.removed.iter().map(|&s| (s, None)).collect(),
        };
    }
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
        unlink: false,
    }
}

/// The operation written as one undo step named after it (docs/adr/0153 §5;
/// the web's `runBatch`). Only the points that change are written, names one
/// by one in the targets' order, a layer move in one call; a refusal of the
/// command is said as it is and nothing is written. Then it is said what
/// changed: for names, also how many of the new names another point has too;
/// for a move, the command's warning (a hidden layer). Çift noktaları ayıkla
/// takes Bağlı çizgiler izler (`follow`).
pub fn run(doc: &mut Document, slots: &[Slot], op: &Op, follow: bool) -> Outcome {
    if let Op::Dedupe { .. } = op {
        return run_dedupe(doc, slots, op, follow);
    }
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

/// A kept point whose place or elevation changes: where to, and whether its
/// elevation does.
#[derive(Clone, Debug, PartialEq)]
pub struct Move {
    pub slot: Slot,
    pub from: Vec2,
    pub to: Vec2,
    pub z: Option<f64>,
    pub z_changed: bool,
}

/// Çift noktaları ayıkla's plan (the web's `DedupePlan`): the groups, the
/// points removed, the kept ones that move, and the window's summary.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DedupePlan {
    /// Why nothing may be done (a tolerance that is no number).
    pub error: Option<String>,
    /// The groups' members, in the drawing's order.
    pub groups: Vec<Vec<Slot>>,
    /// “6 grupta 15 nokta; 9 nokta silinecek.”, or “Çift nokta yok.”; none with an error.
    pub summary: Option<String>,
    pub removed: Vec<Slot>,
    pub moves: Vec<Move>,
}

/// The drawing's order of its objects.
fn drawing_order(doc: &Document) -> HashMap<Slot, usize> {
    doc.entities()
        .enumerate()
        .map(|(k, e)| (Slot(e.base().id), k))
        .collect()
}

/// The groups of duplicates among the target points (docs/adr/0153 §5), by
/// the shared core (`duplicate_points`): by name, or by place within the
/// tolerance (zero or more); the points taken in the drawing's order.
pub fn plan_dedupe(doc: &Document, slots: &[Slot], op: &Op) -> DedupePlan {
    let Op::Dedupe {
        by_name,
        tolerance,
        keep,
    } = op
    else {
        return DedupePlan::default();
    };
    let by = if *by_name {
        DupBy::Name
    } else {
        match parse_number(tolerance) {
            Some(t) if t >= 0.0 => DupBy::Place(t),
            _ => {
                return DedupePlan {
                    error: Some(format!(
                        "{PREFIX}Tolerans sıfır ya da daha büyük bir sayı olmalı."
                    )),
                    ..DedupePlan::default()
                };
            }
        }
    };
    let order = drawing_order(doc);
    let mut points = points_of(doc, slots);
    points.sort_by_key(|(s, _)| order.get(s).copied().unwrap_or(usize::MAX));
    let input: Vec<DupPoint> = points
        .iter()
        .map(|(_, p)| DupPoint {
            p: Vec2::new(p.p.x, p.p.y),
            z: p.z,
            name: p.base.label.clone(),
        })
        .collect();
    let found = duplicate_points(&input, by, *keep);
    let groups: Vec<Vec<Slot>> = found
        .groups
        .iter()
        .map(|g| g.members.iter().map(|&i| points[i as usize].0).collect())
        .collect();
    let removed: Vec<Slot> = found
        .removed
        .iter()
        .map(|&i| points[i as usize].0)
        .collect();
    let moves: Vec<Move> = found
        .groups
        .iter()
        .filter_map(|g| {
            let (slot, p) = &points[g.kept as usize];
            let from = Vec2::new(p.p.x, p.p.y);
            let z_changed = g.z != p.z;
            (g.p != from || z_changed).then_some(Move {
                slot: *slot,
                from,
                to: g.p,
                z: g.z,
                z_changed,
            })
        })
        .collect();
    let total: usize = groups.iter().map(Vec::len).sum();
    let summary = if groups.is_empty() {
        "Çift nokta yok.".to_owned()
    } else if moves.is_empty() {
        format!(
            "{} grupta {total} nokta; {} nokta silinecek.",
            groups.len(),
            removed.len()
        )
    } else {
        format!(
            "{} grupta {total} nokta; {} nokta silinecek, {} nokta ortalamaya taşınacak.",
            groups.len(),
            removed.len(),
            moves.len()
        )
    };
    DedupePlan {
        error: None,
        groups,
        summary: Some(summary),
        removed,
        moves,
    }
}

/// Çift noktaları ayıkla written as one undo step (docs/adr/0153 §5; the
/// web's `runDedupe`): the kept points moved group by group (with `follow`,
/// the line work at a kept point's place moves with it and takes its new
/// elevation when that changes), then the others removed. The first point
/// that would change, in the drawing's order, on a locked layer stops it all
/// with the edit command's words; so does a line work that would follow on
/// one.
fn run_dedupe(doc: &mut Document, slots: &[Slot], op: &Op, follow: bool) -> Outcome {
    let plan = plan_dedupe(doc, slots, op);
    if let Some(error) = plan.error {
        return Outcome {
            said: vec![error],
            step: None,
        };
    }
    if plan.groups.is_empty() {
        return Outcome {
            said: vec![format!("{PREFIX}Çift nokta yok.")],
            step: None,
        };
    }
    let order = drawing_order(doc);
    let mut changing: Vec<Slot> = plan
        .moves
        .iter()
        .map(|m| m.slot)
        .chain(plan.removed.iter().copied())
        .collect();
    changing.sort_by_key(|s| order.get(s).copied().unwrap_or(usize::MAX));
    let layers = doc.layers();
    if let Some(layer) = changing
        .iter()
        .filter_map(|&s| doc.get(s))
        .map(|e| e.base().layer_id.clone())
        .find(|l| layers.is_locked(l))
    {
        let name = layers
            .get(&layer)
            .map_or_else(|| layer.clone(), |n| n.name.clone());
        return Outcome {
            said: vec![format!(
                "“{name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."
            )],
            step: None,
        };
    }
    let step = Kind::Dedupe.step();
    let uid = |doc: &Document, s: Slot| doc.uid(s).map(|u| u.to_string()).unwrap_or_default();
    let removed: Vec<String> = plan.removed.iter().map(|&s| uid(doc, s)).collect();
    let refused = in_step(doc, step, |doc| {
        for m in &plan.moves {
            let mut changes = vec![EntityEdit::Update {
                uid: uid(doc, m.slot),
                geometry: EntityGeometry::Point {
                    p: kentos_contracts::Vec2 {
                        x: m.to.x,
                        y: m.to.y,
                    },
                    z: m.z,
                    parts: None,
                },
            }];
            if follow {
                for other in doc.entities() {
                    if !matches!(
                        other,
                        Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
                    ) {
                        continue;
                    }
                    let Some(moved) = follow_point(
                        &elevated_paths(other),
                        m.from,
                        m.to,
                        m.z_changed,
                        if m.z_changed { m.z } else { None },
                    ) else {
                        continue;
                    };
                    let Some(geometry) = with_paths(other, &moved) else {
                        continue;
                    };
                    if doc.layers().is_locked(&other.base().layer_id) {
                        return Some(FOLLOW_LOCKED.to_owned());
                    }
                    changes.push(EntityEdit::Update {
                        uid: uid(doc, Slot(other.base().id)),
                        geometry,
                    });
                }
            }
            let input = EntitiesEdit {
                operation: EditOperation::Properties,
                changes,
                expected_revision: None,
            };
            if let Some(e) = refusal(edit::execute(&mut ExecutionContext::new(doc), input)) {
                return Some(e.message);
            }
        }
        let input = EntitiesDelete {
            uids: removed,
            expected_revision: None,
        };
        refusal(delete::execute(&mut ExecutionContext::new(doc), input)).map(|e| e.message)
    });
    if let Some(refused) = refused {
        return Outcome {
            said: vec![refused],
            step: None,
        };
    }
    let n = plan.groups.len();
    let gone = plan.removed.len();
    let said = if plan.moves.is_empty() {
        format!("{PREFIX}{n} grupta {gone} nokta silindi.")
    } else {
        format!(
            "{PREFIX}{n} grupta {gone} nokta silindi, {} nokta ortalamaya taşındı.",
            plan.moves.len()
        )
    };
    Outcome {
        said: vec![said],
        step: Some(step),
    }
}

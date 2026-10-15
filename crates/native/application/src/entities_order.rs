//! `cad.entities.order` v1 (docs/adr/0260 §5, 0310 §3): Öne getir, Arkaya
//! gönder, Nesnenin üstüne and Nesnenin altına, as one undo step. The shared cases are
//! `fixtures/commands/v1/cad.entities.order.json`.
//!
//! A layer draws its objects by (order, place in the document). The objects
//! moved take, in their own drawing order, the orders after the largest of
//! their layer's other objects' (to the front) or before the smallest (to the
//! back); a layer whose objects already are the last (first) it draws, or
//! that has no other object, does not change.
//!
//! Above and under put the objects right after (before) the reference in its
//! layer's drawing order; the reference's far side moves by one step when it
//! has no room, so every other pair keeps its order (docs/adr/0310 §3).
//!
//! The checks, in order: the ids (`checks::uids`), the reference's rule, the
//! revision, the objects and the reference (`entity_not_found`), their layers
//! (`layer_locked`, `not_same_layer`), the bound (`order_limit`).

use std::collections::HashMap;

use kentos_contracts::{
    CommandResult, EntitiesOrder, EntitiesOrderPlan, EntitiesOrdered, Entity, OrderTo,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
/// The stable codes of the answers.
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesOrder) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would change, and the revision to expect for exactly that.
pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesOrder) -> CommandResult<EntitiesOrderPlan> {
    match check(cx.doc, input) {
        Ok((changes, warnings)) => CommandResult::Completed {
            output: EntitiesOrderPlan {
                changed: changes.into_iter().map(|c| c.entity).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the new orders as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesOrder,
) -> CommandResult<EntitiesOrdered> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_ENTITIES_ORDER,
        kentos_contracts::CAD_ENTITIES_ORDER_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesOrder,
) -> CommandResult<EntitiesOrdered> {
    let (changes, warnings) = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let (uids, slots): (Vec<String>, Vec<Slot>) =
        changes.iter().map(|c| (c.uid.clone(), c.slot)).unzip();
    if !changes.is_empty() {
        let writes = changes.into_iter().map(|c| (c.slot, c.entity)).collect();
        cx.doc.update_many(writes, label(input.to));
    }
    CommandResult::Completed {
        output: EntitiesOrdered {
            changed: uids,
            ids: slots.iter().map(|s| s.0).collect(),
            revision: cx.doc.revision().to_string(),
        },
        warnings,
    }
}

/// The undo step's name.
pub fn label(to: OrderTo) -> &'static str {
    match to {
        OrderTo::Front => "Öne getir",
        OrderTo::Back => "Arkaya gönder",
        OrderTo::Above => "Nesnenin üstüne",
        OrderTo::Under => "Nesnenin altına",
    }
}

/// An object whose order changes: its slot, id and itself as it will be.
struct Change {
    slot: Slot,
    uid: String,
    entity: Entity,
}

/// Where an object is drawn in its layer: its order (absent 0), then its place in the document.
fn place(at: usize, e: &Entity) -> (i64, usize) {
    (i64::from(e.base().look.order.unwrap_or(0)), at)
}

fn refused(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(checks::error(code, message.into(), Some(path.into())))
}

/// The reference's rule: given exactly for above and under, not one of the objects moved.
fn reference_rule(input: &EntitiesOrder) -> Result<(), Stop> {
    let wants = matches!(input.to, OrderTo::Above | OrderTo::Under);
    match (&input.reference, wants) {
        (None, true) => Err(refused(
            codes::INVALID_REFERENCE,
            "Nesnenin üstüne ve altına bir başvuru nesnesi ister; üstüne ya da altına konacak nesnenin kimliğini verin.",
            "reference",
        )),
        (Some(_), false) => Err(refused(
            codes::INVALID_REFERENCE,
            "Öne getir ve Arkaya gönder başvuru nesnesi almaz; başvuruyu kaldırın ya da Nesnenin üstüne ya da altına'yı seçin.",
            "reference",
        )),
        (Some(r), true) if !checks::is_uid_text(r) => Err(refused(
            codes::INVALID_UID,
            format!("Başvuru “{r}” kalıcı kimlik değil: küçük harfli, tireli UUID metni verin."),
            "reference",
        )),
        (Some(r), true) if input.uids.contains(r) => Err(refused(
            codes::INVALID_REFERENCE,
            "Başvuru nesnesi taşınan nesnelerden biri olamaz; başka bir nesnenin üstüne ya da altına koyun.",
            "reference",
        )),
        _ => Ok(()),
    }
}

fn check(
    doc: &Document,
    input: &EntitiesOrder,
) -> Result<(Vec<Change>, Vec<kentos_contracts::CommandWarning>), Stop> {
    checks::uids(&input.uids, "Sırası değişecek nesne verilmedi.")?;
    reference_rule(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let named = checks::named(doc, &input.uids)?;
    let reference = match &input.reference {
        Some(r) => {
            let found = kentos_domain::Uuid::parse_str(r)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|slot| Some((slot, doc.get(slot)?)));
            let Some(found) = found else {
                return Err(refused(
                    codes::ENTITY_NOT_FOUND,
                    format!(
                        "Başvuru “{r}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                    ),
                    "reference",
                ));
            };
            Some(found)
        }
        None => None,
    };
    // What is on a locked layer is left out, said by its count (docs/adr/0278).
    let (named, locked) = checks::skip_locked(doc, named)?;
    let layers = doc.layers();
    if let Some((_, r)) = reference {
        let layer = &r.base().layer_id;
        if let Some((i, _, e, _)) = named
            .iter()
            .find(|(_, _, e, _)| &e.base().layer_id != layer)
        {
            let name = |id: &str| layers.get(id).map_or(id.to_owned(), |l| l.name.clone());
            return Err(refused(
                codes::NOT_SAME_LAYER,
                format!(
                    "Nesne “{}” katmanında, başvuru “{}” katmanında; çizim sırası katmanın içindedir. Nesneyi başvurunun katmanına taşıyın ya da katmanların sırasını Katmanlar panelinde değiştirin.",
                    name(&e.base().layer_id),
                    name(layer)
                ),
                &format!("uids[{i}]"),
            ));
        }
    }
    // Every object's place in the document, once (the drawing may hold 100 000 of them).
    let mut layer_ids: Vec<&str> = Vec::new();
    for (_, _, e, _) in &named {
        let l = e.base().layer_id.as_str();
        if !layer_ids.contains(&l) {
            layer_ids.push(l);
        }
    }
    let mut out = Vec::new();
    let mut moved_others = Vec::new();
    for layer_id in layer_ids {
        // The layer's objects in the document's order, with their places.
        let mut drawn: Vec<(i64, usize, Slot, &Entity)> = doc
            .entities()
            .enumerate()
            .filter(|(_, e)| e.base().layer_id == layer_id)
            .map(|(i, e)| {
                let (o, at) = place(i, e);
                (o, at, Slot(e.base().id), e)
            })
            .collect();
        drawn.sort_by_key(|(o, at, _, _)| (*o, *at));
        let mine: HashMap<Slot, &String> = named
            .iter()
            .filter(|(_, _, e, _)| e.base().layer_id == layer_id)
            .map(|(_, s, _, u)| (*s, *u))
            .collect();
        let n = mine.len();
        if drawn.len() == n {
            continue;
        }
        // The objects moved, in their own drawing order, and the layer's others.
        let ours: Vec<usize> = (0..drawn.len())
            .filter(|k| mine.contains_key(&drawn[*k].2))
            .collect();
        let next = match (input.to, reference) {
            (OrderTo::Front | OrderTo::Back, _) => {
                let already = match input.to {
                    OrderTo::Front => ours
                        .iter()
                        .enumerate()
                        .all(|(j, k)| *k == drawn.len() - n + j),
                    _ => ours.iter().enumerate().all(|(j, k)| *k == j),
                };
                if already {
                    continue;
                }
                let orders = drawn
                    .iter()
                    .filter(|d| !mine.contains_key(&d.2))
                    .map(|d| d.0);
                let start = match input.to {
                    OrderTo::Front => orders.max().unwrap_or(0) + 1,
                    _ => orders.min().unwrap_or(0) - n as i64,
                };
                Placed {
                    start,
                    shift: Vec::new(),
                }
            }
            (_, Some((ref_slot, _))) => {
                let Some(r) = drawn.iter().position(|d| d.2 == ref_slot) else {
                    continue;
                };
                let above = input.to == OrderTo::Above;
                let already = if above {
                    ours.iter().enumerate().all(|(j, k)| *k == r + 1 + j)
                } else {
                    ours.iter().enumerate().all(|(j, k)| *k + n == r + j)
                };
                if already {
                    continue;
                }
                let ro = drawn[r].0;
                // The others on the far side of the reference, in their drawing order.
                let far: Vec<usize> = if above {
                    (r + 1..drawn.len())
                        .filter(|k| !mine.contains_key(&drawn[*k].2))
                        .collect()
                } else {
                    (0..r)
                        .filter(|k| !mine.contains_key(&drawn[*k].2))
                        .collect()
                };
                if above {
                    // Ours take ro + 1 … ro + n; the far side starts at ro + n + 1 at least, all moved by one step.
                    let need = ro + n as i64 + 1;
                    let low = far.iter().map(|k| drawn[*k].0).min();
                    let step = low.map_or(0, |l| (need - l).max(0));
                    Placed {
                        start: ro + 1,
                        shift: far
                            .into_iter()
                            .map(|k| (k, step))
                            .filter(|(_, d)| *d != 0)
                            .collect(),
                    }
                } else {
                    // Ours take ro − n … ro − 1; the far side ends at ro − n − 1 at most.
                    let need = ro - n as i64 - 1;
                    let high = far.iter().map(|k| drawn[*k].0).max();
                    let step = high.map_or(0, |h| (need - h).min(0));
                    Placed {
                        start: ro - n as i64,
                        shift: far
                            .into_iter()
                            .map(|k| (k, step))
                            .filter(|(_, d)| *d != 0)
                            .collect(),
                    }
                }
            }
            (_, None) => continue,
        };
        let limit = |o: i64| !(i64::from(i32::MIN)..=i64::from(i32::MAX)).contains(&o);
        if limit(next.start)
            || limit(next.start + n as i64 - 1)
            || next.shift.iter().any(|(k, d)| limit(drawn[*k].0 + d))
        {
            let name = layers.get(layer_id).map_or(layer_id, |l| l.name.as_str());
            let way = match input.to {
                OrderTo::Front | OrderTo::Above => {
                    "daha öne alınamaz. Öndeki nesneleri arkaya göndererek"
                }
                _ => "daha arkaya gönderilemez. Arkadaki nesneleri öne getirerek",
            };
            return Err(Stop::failed(checks::error(
                codes::ORDER_LIMIT,
                format!("“{name}” katmanında çizim sırası sınıra geldi; nesne {way} yer açın."),
                Some("to".into()),
            )));
        }
        let with_order = |e: &Entity, order: i64| {
            let mut next = e.clone();
            next.base_mut().look.order = (order != 0).then_some(order as i32);
            next
        };
        for (j, k) in ours.into_iter().enumerate() {
            let (_, _, slot, e) = drawn[k];
            let entity = with_order(e, next.start + j as i64);
            if entity.base() == e.base() {
                continue;
            }
            out.push(Change {
                slot,
                uid: mine[&slot].clone(),
                entity,
            });
        }
        for (k, d) in next.shift {
            let (o, _, slot, e) = drawn[k];
            let Some(uid) = doc.uid(slot) else {
                continue;
            };
            moved_others.push(Change {
                slot,
                uid: uid.to_string(),
                entity: with_order(e, o + d),
            });
        }
    }
    // In the input's order, then the others moved to make room.
    let at: HashMap<&str, usize> = input
        .uids
        .iter()
        .enumerate()
        .map(|(i, u)| (u.as_str(), i))
        .collect();
    out.sort_by_key(|c| at.get(c.uid.as_str()).copied().unwrap_or(usize::MAX));
    out.extend(moved_others);
    Ok((out, locked.into_iter().collect()))
}

/// Where a layer's moved objects go: the first order they take, and the far side's others moved by a step.
struct Placed {
    start: i64,
    shift: Vec<(usize, i64)>,
}

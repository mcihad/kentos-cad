//! `cad.entities.order` v1 (docs/adr/0260 §5): Öne getir and Arkaya gönder,
//! as one undo step. The shared cases are
//! `fixtures/commands/v1/cad.entities.order.json`.
//!
//! A layer draws its objects by (order, place in the document). The objects
//! moved take, in their own drawing order, the orders after the largest of
//! their layer's other objects' (to the front) or before the smallest (to the
//! back); a layer whose objects already are the last (first) it draws, or
//! that has no other object, does not change.
//!
//! The checks, in order: the ids (`checks::uids`), the revision, the objects
//! (`entity_not_found`), their layers (`layer_locked`), the bound (`order_limit`).

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

fn check(
    doc: &Document,
    input: &EntitiesOrder,
) -> Result<(Vec<Change>, Vec<kentos_contracts::CommandWarning>), Stop> {
    checks::uids(&input.uids, "Sırası değişecek nesne verilmedi.")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let named = checks::named(doc, &input.uids)?;
    // What is on a locked layer is left out, said by its count (docs/adr/0278).
    let (named, locked) = checks::skip_locked(doc, named)?;
    let layers = doc.layers();
    // Every object's place in the document, by its slot.
    let all: Vec<(Slot, &Entity)> = doc.entities().map(|e| (Slot(e.base().id), e)).collect();
    let index = |slot: Slot| {
        all.iter()
            .position(|(s, _)| *s == slot)
            .unwrap_or(usize::MAX)
    };
    let mut layer_ids: Vec<&str> = Vec::new();
    for (_, _, e, _) in &named {
        let l = e.base().layer_id.as_str();
        if !layer_ids.contains(&l) {
            layer_ids.push(l);
        }
    }
    let mut out = Vec::new();
    for layer_id in layer_ids {
        let mut mine: Vec<(Slot, &Entity, &String)> = named
            .iter()
            .filter(|(_, _, e, _)| e.base().layer_id == layer_id)
            .map(|(_, s, e, u)| (*s, *e, *u))
            .collect();
        mine.sort_by_key(|(s, e, _)| place(index(*s), e));
        let others: Vec<(usize, &Entity)> = all
            .iter()
            .enumerate()
            .filter(|(_, (s, e))| {
                e.base().layer_id == layer_id && !mine.iter().any(|(m, _, _)| m == s)
            })
            .map(|(i, (_, e))| (i, *e))
            .collect();
        if others.is_empty() {
            continue;
        }
        let mut drawn: Vec<(i64, usize)> = all
            .iter()
            .enumerate()
            .filter(|(_, (_, e))| e.base().layer_id == layer_id)
            .map(|(i, (_, e))| place(i, e))
            .collect();
        drawn.sort();
        let ours: Vec<(i64, usize)> = mine.iter().map(|(s, e, _)| place(index(*s), e)).collect();
        let n = ours.len();
        let already = match input.to {
            OrderTo::Front => drawn[drawn.len() - n..] == ours[..],
            OrderTo::Back => drawn[..n] == ours[..],
        };
        if already {
            continue;
        }
        let orders = others
            .iter()
            .map(|(_, e)| i64::from(e.base().look.order.unwrap_or(0)));
        let start = match input.to {
            OrderTo::Front => orders.max().unwrap_or(0) + 1,
            OrderTo::Back => orders.min().unwrap_or(0) - n as i64,
        };
        if start < i64::from(i32::MIN) || start + n as i64 - 1 > i64::from(i32::MAX) {
            let name = layers.get(layer_id).map_or(layer_id, |l| l.name.as_str());
            let way = match input.to {
                OrderTo::Front => "daha öne alınamaz. Öndeki nesneleri arkaya göndererek",
                OrderTo::Back => "daha arkaya gönderilemez. Arkadaki nesneleri öne getirerek",
            };
            return Err(Stop::failed(checks::error(
                codes::ORDER_LIMIT,
                format!("“{name}” katmanında çizim sırası sınıra geldi; nesne {way} yer açın."),
                Some("to".into()),
            )));
        }
        for (k, (slot, e, uid)) in mine.into_iter().enumerate() {
            let order = (start + k as i64) as i32;
            let mut next = e.clone();
            next.base_mut().look.order = (order != 0).then_some(order);
            if next.base() == e.base() {
                continue;
            }
            out.push(Change {
                slot,
                uid: uid.clone(),
                entity: next,
            });
        }
    }
    // In the input's order.
    let at = |uid: &str| {
        input
            .uids
            .iter()
            .position(|u| u == uid)
            .unwrap_or(usize::MAX)
    };
    out.sort_by_key(|c| at(&c.uid));
    Ok((out, locked.into_iter().collect()))
}

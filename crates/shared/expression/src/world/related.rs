//! `ilişkili_toplam` (docs/adr/0761): an aggregate over the related records of the record being
//! evaluated, the ones a project relation's child field ties to it. The engine knows no relation; the
//! host (a form's rules) finds the related records and gives each as a scope, and this works out the
//! call's value over them by the same rules as `katman_toplamı` (`kentos.statistics/1`).

use super::WorldCall;
use super::aggregate::{self, Aggregate};
use crate::Expr;
use crate::compound::Item;
use crate::library::Func;
use crate::scalar::{R, truthy};

/// The call's operation; none when it is not `ilişkili_toplam` or names none.
pub fn operation(c: &WorldCall) -> Option<Aggregate> {
    (c.func == Func::RelatedAggregate)
        .then(|| c.text.as_deref().and_then(Aggregate::from_name))
        .flatten()
}

/// The call's value over `records`, in the child layer's order: those that meet its condition, its
/// expression on each (`say` without one counts them). `item` works an inner expression out on a
/// record ([`Expr::item`]).
pub fn related_value<T>(
    c: &WorldCall,
    records: &[T],
    item: impl Fn(&T, &Expr) -> Item,
    out: &mut String,
) -> R<'static> {
    let Some(op) = operation(c) else {
        return R::V(crate::scalar::V::Null);
    };
    let mut items: Vec<Item> = Vec::new();
    for t in records {
        if let Some(cond) = &c.condition
            && !truthy(item(t, cond).view())
        {
            continue;
        }
        items.push(match &c.value {
            Some(e) => item(t, e),
            // `say` alone: each record is one.
            None => Item::Bool(true),
        });
    }
    let refs: Vec<&Item> = items.iter().collect();
    super::give(
        Some(&aggregate::over(op, &refs, aggregate::separator(c))),
        out,
    )
}

//! A library block's parameter a regulation's fix sets (docs/adr/8045 §5, §6): an elevator's cabin, a parking place's
//! size. The value is the least its rule allows that reaches the limit, on its step from its least.

use kentos_commands::Entity;
use kentos_commands::block_dynamic::number_problem;
use kentos_domain::{Document, Slot};

/// The command such a fix runs: the values set on the insert (`Fix.operation` holds them).
pub const PARAMETERS: &str = "cad.blocks.parameters";

/// The least value of the dynamic parameter `name` of the insert at `slot` that is at least `want`, in its block's own
/// lengths: on its rule's step from its least; none past its most, or when the block has no such parameter.
pub fn least_value(doc: &Document, slot: Slot, name: &str, want: f64) -> Option<f64> {
    let Some(Entity::Insert(i)) = doc.get(slot) else {
        return None;
    };
    let p = doc
        .block(i.block)?
        .dynamic
        .as_ref()?
        .parameters
        .iter()
        .find(|p| p.name() == name)?;
    let rule = p.rule()?;
    if !want.is_finite() {
        return None;
    }
    let v = match (rule.min, rule.step) {
        (min, Some(step)) if step > 0.0 => {
            let from = min.unwrap_or(0.0);
            from + ((want - from) / step - 1e-6).ceil().max(0.0) * step
        }
        _ => want,
    };
    let v = (v * 1e6).round() / 1e6;
    number_problem(rule, v).is_none().then_some(v)
}

/// A length as a button says it: 1,20.
pub fn button_metres(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

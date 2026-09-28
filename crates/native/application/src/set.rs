//! `cad.entities.set` v1 (docs/adr/0066): the layer, colour, line weight
//! (docs/adr/0139), symbol, attributes or label of objects named by their persistent ids, as one
//! undo step named after the operation. The desktop's handler over the
//! native document; the web's is `apps/web/src/product/entitiesSet.ts`. Both
//! pass the shared cases in `fixtures/commands/v1/cad.entities.set.json`.
//!
//! Öznitelikler's Katman, Renk, Kalınlık and attribute rows write here. They put the
//! objects and the value in the input (TODOS.md CMD-07): the command reads
//! no selection and no library.
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one id; every id lowercase UUID text with hyphens (in order);
//! 2. something to set;
//! 3. no attribute name empty or only white space (Unicode's `White_Space`);
//!    the line weight given a number from 0 to 100 mm;
//! 4. the expected revision (every command's, `checks.rs`);
//! 5. every id names an object of the document (in order);
//! 6. the layer given is one, not a group;
//! 7. no object on a locked layer (in the input's order), then the layer
//!    given not locked.
//!
//! An object already as asked is left alone; when none changes nothing is
//! written: no undo step, and the revision stays.

use kentos_contracts::{
    CommandResult, CommandWarning, EntitiesPropertiesSet, EntitiesSetProperties,
    EntitiesSetPropertiesPlan, Entity, LayerNodeType, PropertiesOperation,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_blank};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesSetProperties) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// The objects execute would change, each as it would be written, and the
/// revision to expect for exactly that; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &EntitiesSetProperties,
) -> CommandResult<EntitiesSetPropertiesPlan> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: EntitiesSetPropertiesPlan {
                changed: checked.changes.into_iter().map(|c| c.entity).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the objects that change as one undo step
/// named after the operation, through the document's own `update_many`:
/// every object keeps its slot and persistent id. Nothing when no object
/// changes. Into the open transaction or group, if one is.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesSetProperties,
) -> CommandResult<EntitiesPropertiesSet> {
    let checked = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let (changed, ids) = checked
        .changes
        .iter()
        .map(|c| (c.uid.clone(), c.slot.0))
        .unzip();
    if !checked.changes.is_empty() {
        let writes = checked
            .changes
            .into_iter()
            .map(|c| (c.slot, c.entity))
            .collect();
        cx.doc.update_many(writes, label(&input));
    }
    CommandResult::Completed {
        output: EntitiesPropertiesSet {
            changed,
            ids,
            revision: cx.doc.revision().to_string(),
        },
        warnings: checked.warnings,
    }
}

/// The undo step's name, the one Öznitelikler and the symbol commands wrote
/// before the command; a symbol's is whether it is given or taken away.
pub fn label(input: &EntitiesSetProperties) -> &'static str {
    match input.operation {
        PropertiesOperation::Layer => "Katman değiştir",
        PropertiesOperation::Color => "Renk değiştir",
        PropertiesOperation::LineWeight => "Kalınlık değiştir",
        PropertiesOperation::Symbol if input.symbol == Some(None) => "Sembolü kaldır",
        PropertiesOperation::Symbol => "Sembol ata",
        PropertiesOperation::Attributes => "Değiştir",
        PropertiesOperation::Label => "Etiket değiştir",
    }
}

/// An object that changes: its slot, its id and itself as it will be.
struct Change {
    slot: Slot,
    uid: String,
    entity: Entity,
    /// Whether it moves to another layer.
    moves: bool,
}

struct Checked {
    changes: Vec<Change>,
    warnings: Vec<CommandWarning>,
}

/// `e` as the input asks, or `None` when it already is: only the fields
/// every object has change, so only they are compared.
fn changed(e: &Entity, input: &EntitiesSetProperties) -> Option<Entity> {
    let mut next = e.clone();
    let base = next.base_mut();
    if let Some(layer) = &input.layer_id {
        base.layer_id.clone_from(layer);
    }
    if let Some(color) = &input.color {
        base.color.clone_from(color);
    }
    if let Some(weight) = input.line_weight {
        base.line_weight = weight;
    }
    if let Some(symbol) = &input.symbol {
        base.symbol.clone_from(symbol);
    }
    if let Some(label) = &input.label {
        base.label.clone_from(label);
    }
    for (key, value) in input.attrs.iter().flatten() {
        match value {
            Some(value) => {
                base.attrs.insert(key.clone(), value.clone());
            }
            None => {
                base.attrs.remove(key);
            }
        }
    }
    (next.base() != e.base()).then_some(next)
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &EntitiesSetProperties) -> Result<Checked, Stop> {
    checks::uids(&input.uids, "Özellikleri değişecek nesne verilmedi.")?;
    let attrs = input.attrs.as_ref().filter(|a| !a.is_empty());
    if input.layer_id.is_none()
        && input.color.is_none()
        && input.line_weight.is_none()
        && input.symbol.is_none()
        && attrs.is_none()
        && input.label.is_none()
    {
        return Err(Stop::Failed(error(
            codes::NOTHING_TO_SET,
            "Değişecek özellik verilmedi. Katman, renk, kalınlık, sembol, öznitelik ya da etiket verin."
                .into(),
            None,
        )));
    }
    if attrs.is_some_and(|a| a.keys().any(|k| is_blank(k))) {
        return Err(Stop::Failed(error(
            codes::INVALID_ATTRIBUTE,
            "Öznitelik adı boş olamaz; yalnız boşluktan oluşan ad da boştur. Özniteliğe bir ad verin."
                .into(),
            Some("attrs".into()),
        )));
    }
    checks::line_weight(input.line_weight.flatten(), "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let found = checks::named(doc, &input.uids)?;
    let layers = doc.layers();
    let at_layer = || Some("layerId".to_owned());
    let target = match input.layer_id.as_deref() {
        None => None,
        Some(id) => {
            let Some(node) = layers.get(id) else {
                return Err(Stop::Failed(error(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    at_layer(),
                )));
            };
            if node.kind != LayerNodeType::Layer {
                return Err(Stop::Failed(error(
                    codes::NOT_A_LAYER,
                    format!(
                        "“{}” bir katman grubu; nesneler yalnız bir katmana taşınır. Grubun içinden bir katman seçin.",
                        node.name
                    ),
                    at_layer(),
                )));
            }
            Some((id, node.name.as_str()))
        }
    };
    // Nothing on a locked layer changes, and nothing moves onto one.
    for (at, _, e, _) in &found {
        let layer = &e.base().layer_id;
        if layers.is_locked(layer) {
            let name = layers.get(layer).map_or(layer.as_str(), |n| n.name.as_str());
            return Err(Stop::Failed(error(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."
                ),
                Some(format!("uids[{at}]")),
            )));
        }
    }
    if let Some((id, name)) = target
        && layers.is_locked(id)
    {
        return Err(Stop::Failed(error(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli; nesneler ona taşınamaz. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin."
            ),
            at_layer(),
        )));
    }
    let changes: Vec<Change> = found
        .iter()
        .filter_map(|&(_, slot, e, uid)| {
            let entity = changed(e, input)?;
            Some(Change {
                slot,
                uid: uid.clone(),
                moves: entity.base().layer_id != e.base().layer_id,
                entity,
            })
        })
        .collect();
    // On a hidden layer they vanish from the drawing: said when at least one moves there.
    let mut warnings = Vec::new();
    if let Some((id, name)) = target
        && !layers.is_visible(id)
        && changes.iter().any(|c| c.moves)
    {
        warnings.push(CommandWarning {
            code: codes::LAYER_HIDDEN.into(),
            message: format!("“{name}” katmanı gizli; taşınan nesneler görünmeyecek."),
            path: at_layer(),
        });
    }
    Ok(Checked { changes, warnings })
}

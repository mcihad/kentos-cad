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
//! 2. something to set (Bağı kopar's `unlink` too, docs/adr/0175 §4);
//! 3. no attribute name empty or only white space (Unicode's `White_Space`);
//!    the line weight given a number from 0 to 100 mm;
//! 4. the expected revision (every command's, `checks.rs`);
//! 5. every id names an object of the document (in order);
//! 6. the layer given is one, not a group;
//! 7. no object on a locked layer (in the input's order), then the layer
//!    given not locked;
//! 8. the attributes given, by the fields of each object's layer (the one
//!    it moves to, else its own; docs/adr/0199 §2), objects in the input's
//!    order, names in theirs: written in their canonical text.
//!
//! An object already as asked is left alone; when none changes nothing is
//! written: no undo step, and the revision stays.

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, CommandWarning, EntitiesPropertiesSet, EntitiesSetProperties,
    EntitiesSetPropertiesPlan, Entity, LayerNodeType, PropertiesOperation, attachments_problem,
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
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ENTITIES_SET,
        kentos_commands::CAD_ENTITIES_SET_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(
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
        // Öznitelikler's Bağı kopar (docs/adr/0175 §4).
        PropertiesOperation::Unlink => "Bağı kopar",
        // An object template given to objects (docs/adr/0176 §6).
        PropertiesOperation::Template => "Şablonu uygula",
        // Öznitelikler's Ekler (docs/adr/0225 §2).
        PropertiesOperation::Attachments => "Ekler",
        // The look's parts (docs/adr/0260 §6).
        PropertiesOperation::LineType => "Çizgi tipi değiştir",
        PropertiesOperation::LineTypeScale => "Çizgi tipi ölçeği",
        PropertiesOperation::Transparency => "Saydamlık değiştir",
        // An area's mask (docs/adr/0310 §1).
        PropertiesOperation::Wipeout => "Maske",
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

/// `e` as the input asks, its attributes `attrs` (the input's in their
/// canonical text), or `None` when it already is: only the fields every
/// object has change, so only they are compared, and a linked text's link
/// (docs/adr/0175 §4) and an associative hatch's tie (docs/adr/0186 §6).
fn changed(
    e: &Entity,
    input: &EntitiesSetProperties,
    attrs: Option<&BTreeMap<String, Option<String>>>,
    nearest: &kentos_commands::LineType,
) -> Option<Entity> {
    let mut next = e.clone();
    let mut unlinked = false;
    if input.unlink {
        match &mut next {
            Entity::Text(text) if text.label_of.is_some() => {
                text.label_of = None;
                text.label_scale = None;
                unlinked = true;
            }
            // An associative hatch follows its objects no more (docs/adr/0186 §6).
            Entity::Hatch(hatch) if hatch.assoc.is_some() => {
                hatch.assoc = None;
                unlinked = true;
            }
            _ => {}
        }
    }
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
    if let Some(t) = input.line_type {
        base.look.line_type = t;
        // A built-in type chosen alone takes the pattern away (docs/adr/0313 §1).
        if input.line_pattern.is_none() {
            base.look.line_pattern = None;
        }
    }
    if let Some(p) = &input.line_pattern {
        base.look.line_pattern.clone_from(p);
        if input.line_type.is_none() && p.is_some() {
            base.look.line_type = Some(*nearest);
        }
    }
    // A scale of 1 is no scale of its own: absent is 1 (docs/adr/0260 §2).
    if let Some(x) = input.line_type_scale {
        base.look.line_type_scale = x.filter(|x| *x != 1.0);
    }
    if let Some(x) = input.transparency {
        base.look.transparency = x;
    }
    if let Some(w) = input.wipeout {
        base.look.wipeout = w;
    }
    if let Some(symbol) = &input.symbol {
        base.symbol.clone_from(symbol);
    }
    if let Some(label) = &input.label {
        base.label.clone_from(label);
    }
    if let Some(list) = &input.attachments {
        base.attachments.clone_from(list);
    }
    for (key, value) in attrs.into_iter().flatten() {
        match value {
            Some(value) => {
                base.attrs.insert(key.clone(), value.clone());
            }
            None => {
                base.attrs.remove(key);
            }
        }
    }
    (unlinked || next.base() != e.base()).then_some(next)
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &EntitiesSetProperties) -> Result<Checked, Stop> {
    checks::uids(&input.uids, "Özellikleri değişecek nesne verilmedi.")?;
    let attrs = input.attrs.as_ref().filter(|a| !a.is_empty());
    if input.layer_id.is_none()
        && input.color.is_none()
        && input.line_weight.is_none()
        && input.line_type.is_none()
        && input.line_pattern.is_none()
        && input.line_type_scale.is_none()
        && input.transparency.is_none()
        && input.wipeout.is_none()
        && input.symbol.is_none()
        && attrs.is_none()
        && input.label.is_none()
        && input.attachments.is_none()
        && !input.unlink
    {
        return Err(Stop::failed(error(
            codes::NOTHING_TO_SET,
            "Değişecek özellik verilmedi. Katman, renk, kalınlık, sembol, öznitelik ya da etiket verin."
                .into(),
            None,
        )));
    }
    if attrs.is_some_and(|a| a.keys().any(|k| is_blank(k))) {
        return Err(Stop::failed(error(
            codes::INVALID_ATTRIBUTE,
            "Öznitelik adı boş olamaz; yalnız boşluktan oluşan ad da boştur. Özniteliğe bir ad verin."
                .into(),
            Some("attrs".into()),
        )));
    }
    checks::line_weight(input.line_weight.flatten(), "lineWeight")?;
    if let Some(x) = input.line_type_scale.flatten()
        && !(x.is_finite() && x > 0.0 && x <= kentos_commands::MAX_LINE_TYPE_SCALE)
    {
        return Err(Stop::failed(error(
            codes::INVALID_LINE_TYPE_SCALE,
            "Çizgi tipi ölçeği 0'dan büyük, en çok 1000 olmalı. Bir ölçek yazın ya da 1'e döndürmek için null verin.".into(),
            Some("lineTypeScale".into()),
        )));
    }
    if let Some(x) = input.transparency.flatten()
        && !(x.is_finite() && (0.0..=kentos_commands::MAX_TRANSPARENCY).contains(&x))
    {
        return Err(Stop::failed(error(
            codes::INVALID_TRANSPARENCY,
            "Saydamlık 0 ile 0,9 arasında olmalı (%0–90). Bir saydamlık yazın ya da kaynağına döndürmek için null verin.".into(),
            Some("transparency".into()),
        )));
    }
    if let Some(problem) = input.attachments.as_deref().and_then(attachments_problem) {
        return Err(Stop::failed(error(
            codes::INVALID_ATTACHMENTS,
            format!(
                "Ekler: {problem}. Her eke bir ad ve tek bir kaynak (gömülü fotoğraf, dosya yolu ya da http/https adresi) verin."
            ),
            Some("attachments".into()),
        )));
    }
    // A project line type by its name (docs/adr/0313 §1).
    if let Some(Some(name)) = &input.line_pattern
        && !doc
            .settings()
            .line_types
            .iter()
            .any(|t| kentos_commands::same_line_type_name(&t.name, name))
    {
        return Err(Stop::failed(error(
            codes::LINE_TYPE_NOT_FOUND,
            format!(
                "“{name}” adlı çizgi tipi projede yok. Çizgi tipleri penceresinden yükleyin ya da var olan bir adı verin."
            ),
            Some("linePattern".into()),
        )));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let found = checks::named(doc, &input.uids)?;
    // A mask is an area's only (docs/adr/0310 §1).
    if input.wipeout.is_some_and(|w| w.is_some())
        && let Some((i, _, e, _)) = found
            .iter()
            .find(|(_, _, e, _)| !matches!(e, Entity::Polygon(_)))
    {
        return Err(Stop::failed(error(
            codes::INVALID_WIPEOUT,
            format!(
                "Maske yalnız alanda olur; {}. nesne ({}) alan değil. Yalnız kapalı alanları seçin ya da çizgiyi önce Alana çevir ile alan yapın.",
                i + 1,
                e.kind()
            ),
            Some(format!("uids[{i}]")),
        )));
    }
    // An embedded photo is an image of the project's library (docs/adr/0225 §1).
    for (k, a) in input.attachments.iter().flatten().enumerate() {
        if let Some(asset) = &a.asset
            && !crate::edit::has_picture(doc, asset)
        {
            return Err(Stop::failed(error(
                codes::UNKNOWN_ASSET,
                format!(
                    "“{asset}” kimlikli fotoğraf projenin kitaplığında yok: silinmiş ya da başka bir çizimin olabilir. Projenin kitaplığındaki bir PNG ya da JPEG görüntünün kimliğini verin."
                ),
                Some(format!("attachments[{k}].asset")),
            )));
        }
    }
    let layers = doc.layers();
    let at_layer = || Some("layerId".to_owned());
    let target = match input.layer_id.as_deref() {
        None => None,
        Some(id) => {
            let Some(node) = layers.get(id) else {
                return Err(Stop::failed(error(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    at_layer(),
                )));
            };
            if node.kind != LayerNodeType::Layer {
                return Err(Stop::failed(error(
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
    // What is on a locked layer is left out, said by its count (docs/adr/0278); nothing moves onto one.
    let (found, locked) = checks::skip_locked(doc, found)?;
    if let Some((id, name)) = target
        && layers.is_locked(id)
    {
        return Err(Stop::failed(error(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli; nesneler ona taşınamaz. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin."
            ),
            at_layer(),
        )));
    }
    // A constant attribute's value is its block's (docs/adr/0317 §1): an insert takes none of its own.
    if let Some(attrs) = attrs {
        for (_, _, e, _) in &found {
            let kentos_commands::Entity::Insert(i) = e else {
                continue;
            };
            let Some(block) = doc.block(i.block) else {
                continue;
            };
            if let Some(a) = block.attributes.iter().find(|a| {
                a.options.constant && attrs.get(&a.tag).is_some_and(Option::is_some)
            }) {
                return Err(Stop::failed(error(
                    codes::CONSTANT_ATTRIBUTE,
                    format!(
                        "“{}” bloğunun “{}” özniteliği sabit: değeri tanımındakidir, yerleştirmede değiştirilmez.",
                        block.name, a.tag
                    ),
                    Some(format!("attrs.{}", a.tag)),
                )));
            }
        }
    }
    // The values given to a layer's fields, in their canonical text (docs/adr/0199 §2).
    let mut given = Vec::with_capacity(found.len());
    for (_, _, e, _) in &found {
        given.push(match attrs {
            Some(attrs) => {
                let layer = input.layer_id.as_deref().unwrap_or(&e.base().layer_id);
                Some(checks::field_values(doc, layer, attrs, "attrs")?)
            }
            None => None,
        });
    }
    // The pattern's nearest built-in type, what a reader without patterns draws (docs/adr/0313 §1).
    let nearest = input
        .line_pattern
        .as_ref()
        .and_then(Option::as_deref)
        .and_then(|name| {
            doc.settings()
                .line_types
                .iter()
                .find(|t| kentos_commands::same_line_type_name(&t.name, name))
        })
        .map_or(kentos_commands::LineType::Continuous, |t| t.nearest());
    let changes: Vec<Change> = found
        .iter()
        .zip(&given)
        .filter_map(|(&(_, slot, e, uid), attrs)| {
            let entity = changed(e, input, attrs.as_ref(), &nearest)?;
            Some(Change {
                slot,
                uid: uid.clone(),
                moves: entity.base().layer_id != e.base().layer_id,
                entity,
            })
        })
        .collect();
    // A layer's enforcing forms are its write profile (docs/adr/0401 §8): an attribute change that
    // breaks one is refused.
    let mut misfits: Vec<CommandWarning> = Vec::new();
    let mut rules = checks::LayerRules::new(doc);
    let mut writes: Vec<checks::UniqueWrite<'_>> = Vec::new();
    for c in &changes {
        if let Some(old) = doc.get(c.slot) {
            let after = c.entity.base();
            crate::forms::enforced(doc, c.slot, old, &after.layer_id, &after.attrs, "attrs")?;
            // The record against its subtype (docs/adr/0422 §2); one warning for all of them.
            let before = (old.base().layer_id == after.layer_id).then_some(&old.base().attrs);
            if let Some(w) =
                checks::subtype_fit(doc, &after.layer_id, before, &after.attrs, "attrs")?
                && !misfits.iter().any(|m| m.code == w.code)
            {
                misfits.push(w);
            }
            for w in checks::combination_fit(doc, &after.layer_id, before, &after.attrs, "attrs")? {
                if !misfits.iter().any(|m| m.code == w.code) {
                    misfits.push(w);
                }
            }
            // The layer's record rules and unique fields (docs/adr/0438 §1, §3).
            let object = crate::forms::object(doc, c.slot, &c.entity);
            for w in rules.fit(&after.layer_id, before, &after.attrs, Some(&object), "attrs")? {
                if !misfits.iter().any(|m| m.code == w.code && m.path == w.path) {
                    misfits.push(w);
                }
            }
            writes.push(checks::UniqueWrite {
                slot: Some(c.slot),
                layer: &after.layer_id,
                before,
                after: &after.attrs,
                at: "attrs".to_owned(),
            });
        }
    }
    checks::unique_fit(doc, &writes)?;
    // On a hidden layer they vanish from the drawing: said when at least one moves there.
    let mut warnings: Vec<CommandWarning> = locked.into_iter().collect();
    warnings.extend(misfits);
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

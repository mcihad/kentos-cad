//! `cad.blocks.edit` v1 (docs/adr/0144 §4): one change of the drawing's
//! block definitions as one undo step named after it: a new name, new
//! objects, a new base point or new attribute definitions (“Blok değiştir”),
//! a definition deleted (“Blok sil”) or every unused one (“Blokları
//! temizle”). Every insert shows a changed definition. The desktop's handler over the native document; the
//! web's is `apps/web/src/product/blocksEdit.ts`. Both pass the shared cases
//! in `fixtures/commands/v1/cad.blocks.edit.json`.
//!
//! What would change nothing writes nothing and completes with nothing in
//! `changed` and `removed`.
//!
//! The checks, in order (the first that fails answers):
//! 1. the block given where the operation needs one; what the operation
//!    needs from the input (a name, objects and a base point, a layer, the
//!    attribute definitions);
//! 2. the expected revision;
//! 3. the block is the drawing's;
//! 4. the operation's own: the block rules with the definition as it would
//!    be, the objects (`redefine`), a definition an insert uses (`remove`).

use std::collections::HashSet;

use kentos_contracts::{
    AttributeDefinition, BlockDefinition, BlockEditOperation, BlockId, BlocksEdit, BlocksEditPlan,
    BlocksEdited, CommandResult, CommandWarning, Entity,
};
use kentos_domain::{Document, Slot, labels};

use crate::ExecutionContext;
use crate::blocks_define::{as_drawn, block_rules, copies, name, no_layer, replaced, write};
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &BlocksEdit) -> CommandResult<()> {
    if crate::blocks_library::is_library(input.operation) {
        return crate::blocks_library::validate(cx, input);
    }
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &BlocksEdit) -> CommandResult<BlocksEditPlan> {
    if crate::blocks_library::is_library(input.operation) {
        return crate::blocks_library::plan(cx, input);
    }
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: BlocksEditPlan {
                added: Vec::new(),
                removed: in_drawing_order(cx.doc, &checked.removed),
                changed: checked.changed.into_iter().collect(),
                insert: checked.insert,
                deleted: checked.deleted.into_iter().map(|(_, uid)| uid).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes it as one undo step named after the
/// operation, into the open transaction or group, if one is.
pub fn execute(cx: &mut ExecutionContext<'_>, input: BlocksEdit) -> CommandResult<BlocksEdited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_BLOCKS_EDIT,
        kentos_contracts::CAD_BLOCKS_EDIT_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(cx: &mut ExecutionContext<'_>, input: BlocksEdit) -> CommandResult<BlocksEdited> {
    if crate::blocks_library::is_library(input.operation) {
        return crate::blocks_library::write(cx, &input);
    }
    let checked = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let changed: Vec<BlockId> = checked.changed.iter().map(|b| b.id).collect();
    let removed = in_drawing_order(cx.doc, &checked.removed);
    let mut slot = None;
    let mut warnings = checked.warnings.clone();
    let report = checked.sync.as_ref().map(|s| s.report.clone());
    if let Some(s) = &checked.sync
        && s.locked > 0
    {
        warnings.push(CommandWarning {
            code: codes::LAYER_LOCKED.into(),
            message: format!(
                "{} yerleştirme kilitli katmanda olduğu için değerleri eşitlenmedi. Katmanın kilidini Katmanlar panelinden açıp Öznitelikleri eşitle'yi yineleyin.",
                s.locked
            ),
            path: Some("block".into()),
        });
    }
    if !checked.removed.is_empty() {
        if let Err(stop) = remove(cx.doc, label(input.operation), &checked.removed) {
            return stop.into();
        }
    } else if checked.changed.is_some() || checked.insert.is_some() {
        let block = checked.changed.clone().or_else(|| {
            // The objects replaced by an insert of an unchanged definition.
            input.block.and_then(|id| cx.doc.block(id).cloned())
        });
        let Some(block) = block else {
            return CommandResult::Completed {
                output: done(cx.doc, Vec::new(), Vec::new(), None, Vec::new()),
                warnings,
            };
        };
        let gone: Vec<Slot> = checked.deleted.iter().map(|(s, _)| *s).collect();
        match write(cx.doc, labels::BLOCK_CHANGE, block, &gone, checked.insert) {
            Ok(written) => slot = written,
            Err(stop) => return stop.into(),
        }
    }
    // The inserts' values, in the same step (docs/adr/0317 §2).
    if let Some(s) = checked.sync
        && !s.changes.is_empty()
    {
        cx.doc.update_many(s.changes, label(input.operation));
    }
    let deleted = checked.deleted.into_iter().map(|(_, uid)| uid).collect();
    let mut output = done(cx.doc, changed, removed, slot, deleted);
    output.sync = report;
    CommandResult::Completed { output, warnings }
}

/// `ids` in the order the drawing lists its definitions.
fn in_drawing_order(doc: &Document, ids: &[BlockId]) -> Vec<BlockId> {
    doc.blocks()
        .iter()
        .map(|b| b.id)
        .filter(|id| ids.contains(id))
        .collect()
}

/// The undo step's name (docs/adr/0144 §4).
pub fn label(operation: BlockEditOperation) -> &'static str {
    match operation {
        BlockEditOperation::Rename
        | BlockEditOperation::Redefine
        | BlockEditOperation::Rebase
        | BlockEditOperation::Attributes
        | BlockEditOperation::Units
        | BlockEditOperation::Update => labels::BLOCK_CHANGE,
        BlockEditOperation::Remove => labels::BLOCK_REMOVE,
        BlockEditOperation::Purge => labels::BLOCK_PURGE,
        BlockEditOperation::Duplicate => crate::blocks_library::DUPLICATE,
        BlockEditOperation::Import | BlockEditOperation::Export => crate::blocks_library::IMPORT,
        BlockEditOperation::Sync => crate::blocks_sync::SYNC,
        BlockEditOperation::Fields => crate::blocks_sync::FIELDS,
        BlockEditOperation::Place => crate::blocks_sync::PLACE,
        BlockEditOperation::Card => labels::BLOCK_CHANGE,
    }
}

fn done(
    doc: &Document,
    changed: Vec<BlockId>,
    removed: Vec<BlockId>,
    slot: Option<Slot>,
    deleted: Vec<String>,
) -> BlocksEdited {
    BlocksEdited {
        sync: None,
        changed,
        removed,
        insert: slot.and_then(|s| doc.uid(s)).map(|u| u.to_string()),
        id: slot.map(|s| s.0),
        deleted,
        added: Vec::new(),
        skipped: Vec::new(),
        library: None,
        revision: doc.revision().to_string(),
    }
}

/// What may be written: the definition as it will be (when it changes), the
/// definitions to delete (the order they go in), and with `replace` the
/// objects to delete and the insert.
struct Checked {
    changed: Option<BlockDefinition>,
    removed: Vec<BlockId>,
    deleted: Vec<(Slot, String)>,
    insert: Option<Entity>,
    warnings: Vec<CommandWarning>,
    /// The inserts' values fitted to the definition (docs/adr/0317 §2).
    sync: Option<crate::blocks_sync::Synced>,
}

impl Checked {
    fn nothing() -> Checked {
        Checked {
            changed: None,
            removed: Vec::new(),
            deleted: Vec::new(),
            insert: None,
            warnings: Vec::new(),
            sync: None,
        }
    }
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &BlocksEdit) -> Result<Checked, Stop> {
    // Every operation names its block but these: `purge` and `fields` take every one, `place` its inserts'.
    let needs_block = !matches!(
        input.operation,
        BlockEditOperation::Purge | BlockEditOperation::Fields | BlockEditOperation::Place
    );
    if needs_block && input.block.is_none() {
        return Err(Stop::failed(error(
            codes::NO_BLOCK,
            "Değiştirilecek blok verilmedi. Bloğun kimliğini verin.".into(),
            Some("block".into()),
        )));
    }
    let empty = Vec::new();
    let uids = input.uids.as_ref().unwrap_or(&empty);
    match input.operation {
        BlockEditOperation::Rename => name(input.name.as_deref().unwrap_or(""))?,
        BlockEditOperation::Redefine => {
            checks::uids(uids, "Bloğa girecek nesne verilmedi.")?;
            if let Some(base) = input.base {
                checks::point(base, "Taban noktasının", "base")?;
            }
            if input.replace == Some(true) && input.layer_id.is_none() {
                return Err(no_layer());
            }
        }
        BlockEditOperation::Rebase => {
            let Some(base) = input.base else {
                return Err(Stop::failed(error(
                    codes::NO_BASE,
                    "Yeni taban noktası verilmedi. Bloğun taban noktasını verin.".into(),
                    Some("base".into()),
                )));
            };
            checks::point(base, "Taban noktasının", "base")?;
        }
        BlockEditOperation::Attributes => attributes(input.attributes.as_deref())?,
        BlockEditOperation::Units if input.units.is_none() => {
            return Err(Stop::failed(error(
                codes::NO_UNITS,
                "Bloğun birimi verilmedi. units ile mm, cm, m, in, ft ya da unitless verin.".into(),
                Some("units".into()),
            )));
        }
        BlockEditOperation::Units => {}
        BlockEditOperation::Update => update_input(input)?,
        BlockEditOperation::Place => {
            checks::uids(uids, "Konumu değişecek yerleştirme verilmedi.")?;
            if input.tag.as_deref().is_none_or(checks::is_blank) {
                return Err(Stop::failed(error(
                    codes::UNKNOWN_TAG,
                    "Taşınacak özniteliğin etiketi verilmedi. tag ile bloğun taşınabilir bir özniteliğini verin.".into(),
                    Some("tag".into()),
                )));
            }
            if let Some(at) = input.at {
                checks::point(at, "Özniteliğin yerinin", "at")?;
            }
        }
        BlockEditOperation::Remove
        | BlockEditOperation::Purge
        | BlockEditOperation::Duplicate
        | BlockEditOperation::Export
        | BlockEditOperation::Import
        | BlockEditOperation::Sync
        | BlockEditOperation::Fields
        | BlockEditOperation::Card => {}
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    if input.operation == BlockEditOperation::Purge {
        return Ok(Checked {
            removed: unused(doc),
            ..Checked::nothing()
        });
    }
    if input.operation == BlockEditOperation::Place {
        return place(doc, input, uids);
    }
    if input.operation == BlockEditOperation::Fields && input.block.is_none() {
        return fields_all(doc);
    }
    let Some(old) = input.block.and_then(|id| doc.block(id)) else {
        let id = input.block.map(|b| b.to_text()).unwrap_or_default();
        return Err(Stop::failed(error(
            codes::UNKNOWN_BLOCK,
            format!(
                "“{id}” kimlikli blok çizimde tanımlı değil: silinmiş ya da başka bir çizimin olabilir. Çizimde tanımlı bir bloğun kimliğini verin."
            ),
            Some("block".into()),
        )));
    };
    let mut checked = Checked::nothing();
    let mut next = old.clone();
    match input.operation {
        BlockEditOperation::Rename => {
            next.name = input.name.clone().unwrap_or_default();
            rules_with(doc, &next)?;
        }
        BlockEditOperation::Rebase => {
            next.base = input.base.unwrap_or(old.base);
            rules_with(doc, &next)?;
        }
        BlockEditOperation::Attributes => {
            next.attributes = input.attributes.clone().unwrap_or_default();
            rules_with(doc, &next)?;
            if let Some(r) = &input.renames {
                crate::blocks_sync::renames_ok(&next, r)?;
            }
            crate::blocks_sync::fields_of(doc, &next.attributes, &|i| {
                format!("attributes[{i}].field")
            })?;
        }
        BlockEditOperation::Redefine => {
            let named = checks::named(doc, uids)?;
            crate::blocks_define::no_tables(&named)?;
            next.entities = as_drawn(copies(named.iter().map(|(_, _, e, _)| *e)));
            next.base = input.base.unwrap_or(old.base);
            rules_with(doc, &next)?;
            if let (true, Some(layer)) = (input.replace == Some(true), &input.layer_id) {
                let mut r = replaced(doc, &named, layer, next.base)?;
                if let Entity::Insert(i) = &mut r.insert {
                    i.block = old.id;
                }
                checked.deleted = r.removed;
                checked.insert = Some(r.insert);
                checked.warnings = r.warnings;
            }
        }
        BlockEditOperation::Units => units(doc, old, &mut next, input)?,
        BlockEditOperation::Update => {
            next.entities = numbered(old, input.entities.clone().unwrap_or_default());
            next.base = input.base.unwrap_or(old.base);
            if let Some(a) = &input.attributes {
                next.attributes = a.clone();
            }
            rules_with(doc, &next)?;
        }
        BlockEditOperation::Remove => {
            if let Some(reason) = doc.block_removal_refused(old.id) {
                return Err(Stop::failed(error(
                    codes::BLOCK_IN_USE,
                    reason,
                    Some("block".into()),
                )));
            }
            checked.removed = vec![old.id];
            return Ok(checked);
        }
        BlockEditOperation::Sync => {
            let renames = input.renames.clone().unwrap_or_default();
            crate::blocks_sync::renames_ok(old, &renames)?;
            let remove = input.remove_tags.clone().unwrap_or_default();
            checked.sync = Some(crate::blocks_sync::sync(
                doc, old, old, &renames, &remove, true, false,
            )?);
            return Ok(checked);
        }
        BlockEditOperation::Fields => {
            checked.sync = Some(crate::blocks_sync::sync(
                doc,
                old,
                old,
                &Default::default(),
                &[],
                true,
                true,
            )?);
            return Ok(checked);
        }
        BlockEditOperation::Card => {
            let version = old.card.as_ref().map_or(1, |c| c.version);
            let category = input.category.clone().or_else(|| old.card.as_ref().and_then(|c| c.category.clone()));
            let tags = input
                .tags
                .clone()
                .unwrap_or_else(|| old.card.as_ref().map(|c| c.tags.clone()).unwrap_or_default());
            next.card = Some(kentos_contracts::BlockCard {
                category: category.filter(|c| !checks::is_blank(c)),
                tags,
                version,
            });
            if let Some(d) = &input.description {
                next.description = (!checks::is_blank(d)).then(|| d.clone());
            }
            if let Some(problem) = next.card.as_ref().and_then(kentos_contracts::blocks::card_problem) {
                return Err(Stop::failed(error(
                    codes::INVALID_BLOCK_CARD,
                    format!("“{}” bloğunun kartı: {problem}", old.name),
                    Some(if input.tags.is_some() { "tags" } else { "category" }.into()),
                )));
            }
        }
        BlockEditOperation::Place
        | BlockEditOperation::Purge
        | BlockEditOperation::Duplicate
        | BlockEditOperation::Export
        | BlockEditOperation::Import => {}
    }
    // A change of what it draws counts on its card (docs/adr/0317 §4): its name and card do not.
    fn draws(b: &BlockDefinition) -> (&Vec<Entity>, kentos_contracts::Vec2, &Vec<AttributeDefinition>, Option<kentos_contracts::BlockUnits>) {
        (&b.entities, b.base, &b.attributes, b.units)
    }
    let redrawn = draws(&next) != draws(old);
    if redrawn
        && let Some(card) = next.card.as_mut()
    {
        card.version = card.version.saturating_add(1);
    }
    // `attributes` and `update` sync the inserts unless asked not to (docs/adr/0317 §2).
    if matches!(
        input.operation,
        BlockEditOperation::Attributes | BlockEditOperation::Update
    ) && input.sync != Some(false)
        && next.attributes != old.attributes
    {
        let renames = input.renames.clone().unwrap_or_default();
        checked.sync = Some(crate::blocks_sync::sync(
            doc, old, &next, &renames, &[], true, false,
        )?);
    }
    if next != *old {
        checked.changed = Some(next);
    }
    Ok(checked)
}

/// `place` (docs/adr/0317 §1): each insert named shows `tag` at `at` (a drawing point, kept in its definition's
/// coordinates) or where the definition puts it; inserts on a locked layer left out and said.
fn place(doc: &Document, input: &BlocksEdit, uids: &[String]) -> Result<Checked, Stop> {
    let tag = input.tag.clone().unwrap_or_default();
    let named = checks::named(doc, uids)?;
    let (open, warning) = checks::skip_locked(doc, named)?;
    let mut changes = Vec::with_capacity(open.len());
    for (i, slot, e, _) in open {
        let Entity::Insert(insert) = e else {
            return Err(Stop::failed(error(
                codes::NOT_INSERT,
                "Verilen nesne bir blok yerleştirmesi değil. Bir bloğun yerleştirmesini verin.".into(),
                Some(format!("uids[{i}]")),
            )));
        };
        let Some(block) = doc.block(insert.block) else {
            continue;
        };
        if !block
            .attributes
            .iter()
            .any(|a| a.tag == tag && a.options.movable)
        {
            return Err(Stop::failed(error(
                codes::ATTRIBUTE_NOT_MOVABLE,
                format!(
                    "“{}” bloğunda “{tag}” etiketli taşınabilir bir öznitelik yok. Blok öznitelikleri penceresinde özniteliği taşınabilir yapın.",
                    block.name
                ),
                Some("tag".into()),
            )));
        }
        let mut next = insert.clone();
        match input.at {
            Some(at) => {
                let Some(q) = crate::blocks_sync::definition_point(block, insert, at) else {
                    continue;
                };
                next.attribute_places.insert(tag.clone(), q);
            }
            None => {
                next.attribute_places.remove(&tag);
            }
        }
        if next != *insert {
            changes.push((slot, Entity::Insert(next)));
        }
    }
    let report = kentos_contracts::AttributeSync {
        inserts: u32::try_from(changes.len()).unwrap_or(u32::MAX),
        ..Default::default()
    };
    Ok(Checked {
        warnings: warning.into_iter().collect(),
        sync: Some(crate::blocks_sync::Synced {
            changes,
            report,
            locked: 0,
        }),
        ..Checked::nothing()
    })
}

/// `fields` without a block: every block's inserts with a field filled again (docs/adr/0317 §1).
fn fields_all(doc: &Document) -> Result<Checked, Stop> {
    let mut changes = Vec::new();
    let mut report = kentos_contracts::AttributeSync::default();
    let mut locked = 0;
    for id in crate::blocks_sync::field_blocks(doc, None) {
        let Some(b) = doc.block(id) else { continue };
        let s = crate::blocks_sync::sync(doc, b, b, &Default::default(), &[], true, true)?;
        report.inserts += s.report.inserts;
        report.fields += s.report.fields;
        locked += s.locked;
        changes.extend(s.changes);
    }
    Ok(Checked {
        sync: Some(crate::blocks_sync::Synced {
            changes,
            report,
            locked,
        }),
        ..Checked::nothing()
    })
}

/// The attribute definitions `attributes` writes (docs/adr/0144 §7): the
/// list given, then each in its order: a tag, not one an earlier one has
/// (exactly), a finite point and turn, a height above zero.
fn attributes(list: Option<&[AttributeDefinition]>) -> Result<(), Stop> {
    let Some(list) = list else {
        return Err(Stop::failed(error(
            codes::NO_ATTRIBUTES,
            "Öznitelik listesi verilmedi. Bloğun bütün özniteliklerini verin; boş liste hepsini kaldırır.".into(),
            Some("attributes".into()),
        )));
    };
    for (i, a) in list.iter().enumerate() {
        let at = format!("attributes[{i}]");
        if checks::is_blank(&a.tag) {
            return Err(Stop::failed(error(
                codes::EMPTY_TAG,
                "Öznitelik etiketi boş olamaz; her özniteliğe bir etiket verin.".into(),
                Some(format!("{at}.tag")),
            )));
        }
        if list[..i].iter().any(|b| b.tag == a.tag) {
            return Err(Stop::failed(error(
                codes::DUPLICATE_TAG,
                format!(
                    "“{}” etiketi listede iki kez var; her etiket bir kez olmalı.",
                    a.tag
                ),
                Some(format!("{at}.tag")),
            )));
        }
        let whose = format!("“{}” özniteliğinin yerinin", a.tag);
        checks::point(a.p, &whose, &format!("{at}.p"))?;
        checks::finite(
            a.rotation,
            &format!("“{}” özniteliğinin açısı", a.tag),
            "Açıyı sonlu bir sayıyla verin.",
            &format!("{at}.rotation"),
        )?;
        if !(a.height.is_finite() && a.height > 0.0) {
            return Err(Stop::failed(error(
                codes::INVALID_HEIGHT,
                format!(
                    "“{}” özniteliğinin yazı yüksekliği sıfırdan büyük, sonlu bir sayı olmalı (metre, bloğun kendi ölçüsünde).",
                    a.tag
                ),
                Some(format!("{at}.height")),
            )));
        }
    }
    Ok(())
}

/// The block rules with `block` in its definition's place.
fn rules_with(doc: &Document, block: &BlockDefinition) -> Result<(), Stop> {
    let next: Vec<&BlockDefinition> = doc
        .blocks()
        .iter()
        .map(|b| if b.id == block.id { block } else { &**b })
        .collect();
    block_rules(&next)
}

/// The definitions no insert uses, in the order they can go: first those no
/// object or other definition places, then those only they placed, and so on.
fn unused(doc: &Document) -> Vec<BlockId> {
    let mut gone: Vec<BlockId> = Vec::new();
    let mut left: Vec<&BlockDefinition> = doc.blocks().iter().map(|b| &**b).collect();
    loop {
        let mut used: HashSet<BlockId> = HashSet::new();
        for e in doc.entities() {
            if let Entity::Insert(i) = e {
                used.insert(i.block);
            }
        }
        for b in &left {
            for e in &b.entities {
                if let Entity::Insert(i) = e {
                    used.insert(i.block);
                }
            }
        }
        let round: Vec<BlockId> = left
            .iter()
            .map(|b| b.id)
            .filter(|id| !used.contains(id))
            .collect();
        if round.is_empty() {
            return gone;
        }
        left.retain(|b| !round.contains(&b.id));
        gone.extend(round);
    }
}

/// Deletes the definitions in their order as one undo step named `label`.
fn remove(doc: &mut Document, label: &str, ids: &[BlockId]) -> Result<(), Stop> {
    let written = doc.transact(label, |doc| {
        for id in ids {
            doc.remove_block(*id).map_err(|r| r.to_string())?;
        }
        Ok::<(), String>(())
    });
    written.map_err(|message| Stop::failed(error(codes::BLOCK_REFUSED, message, None)))
}

/// `units` (docs/adr/0315 §4): the unit named; between two units with `rescale` the numbers kept (the geometry
/// scaled by their ratio); to or from unitless the numbers made the drawing's units or metres again, refused while
/// an insert places the block (it would change size).
fn units(
    doc: &Document,
    old: &BlockDefinition,
    next: &mut BlockDefinition,
    input: &BlocksEdit,
) -> Result<(), Stop> {
    use kentos_contracts::BlockUnits;
    use kentos_contracts::units::Scale;
    let Some(to) = input.units else {
        return Ok(());
    };
    let drawing = crate::units_change::unit_of(doc);
    let (was_free, now_free) = (
        old.units == Some(BlockUnits::Unitless),
        to == BlockUnits::Unitless,
    );
    if was_free != now_free {
        if doc.block_removal_refused(old.id).is_some() {
            return Err(Stop::failed(error(
                codes::BLOCK_IN_USE,
                format!(
                    "“{}” bloğu yerleştirilmiş; birimsiz yapılan ya da birimsizlikten çıkan blok yerleştirmelerinde boyunu değiştirirdi. Önce yerleştirmelerini kaldırın ya da bloğun kopyasında değiştirin.",
                    old.name
                ),
                Some("block".into()),
            )));
        }
        rescaled(
            next,
            if now_free {
                Scale::to_unit(drawing)
            } else {
                Scale::from_unit(drawing)
            },
        );
    } else if !now_free && input.rescale == Some(true) {
        let from = old.units.and_then(BlockUnits::unit).unwrap_or(drawing);
        let to = to.unit().unwrap_or(drawing);
        rescaled(next, Scale::keeping_numbers(from, to));
    }
    next.units = Some(to);
    rules_with(doc, next)
}

/// A definition alone scaled (docs/adr/0315 §4): its points and sizes, and its nested inserts' factors, since the
/// definitions they place stay as they are.
pub(crate) fn rescaled(block: &mut BlockDefinition, s: kentos_contracts::units::Scale) {
    kentos_contracts::units::scale_block(block, s);
    for e in &mut block.entities {
        if let Entity::Insert(i) = e {
            i.scale = s.apply(i.scale);
            i.y_scale = i.y_scale.map(|y| s.apply(y));
        }
    }
}

/// `update`'s input on its own (docs/adr/0316 §1): objects given, none a table, a picture, a raster or a point cloud,
/// each a geometry `cad.entities.create` would take, no two with one local id but 0 (new); a finite base point; the
/// attribute definitions as `attributes` checks them.
fn update_input(input: &BlocksEdit) -> Result<(), Stop> {
    let list = input.entities.as_deref().unwrap_or_default();
    if list.is_empty() {
        return Err(Stop::failed(error(
            codes::NO_ENTITIES,
            "Bloğun nesneleri verilmedi. Tanımın bütün nesnelerini yerel kimlikleriyle verin.".into(),
            Some("entities".into()),
        )));
    }
    for (i, e) in list.iter().enumerate() {
        let refused = match e {
            Entity::Table(_) => Some((codes::TABLE_IN_BLOCK, "tablo")),
            Entity::Image(_) => Some((codes::IMAGE_IN_BLOCK, "resim")),
            Entity::Raster(_) => Some((codes::RASTER_IN_BLOCK, "raster")),
            Entity::PointCloud(_) => Some((codes::POINTCLOUD_IN_BLOCK, "nokta bulutu")),
            _ => None,
        };
        if let Some((code, what)) = refused {
            return Err(Stop::failed(error(
                code,
                format!("{}. nesne bir {what}; {what} bloğa konamaz.", i + 1),
                Some(format!("entities[{i}]")),
            )));
        }
        if let Some(g) = crate::geometry::edit_geometry(crate::geometry::shape(e)) {
            crate::edit::check_geometry(&g, "entities", i, "nesnenin")?;
        }
    }
    let mut seen = HashSet::new();
    for (i, e) in list.iter().enumerate() {
        let id = e.base().id;
        if id != 0 && !seen.insert(id) {
            return Err(Stop::failed(error(
                codes::DUPLICATE_PIECE,
                format!(
                    "{}. nesnenin yerel kimliği {id} daha önce verildi; her nesnenin kimliği bir kez olur, yeni nesneye 0 verin.",
                    i + 1
                ),
                Some(format!("entities[{i}].id")),
            )));
        }
    }
    if let Some(base) = input.base {
        checks::point(base, "Taban noktasının", "base")?;
    }
    if input.attributes.is_some() {
        attributes(input.attributes.as_deref())?;
    }
    Ok(())
}

/// The objects as the definition keeps them (docs/adr/0316 §1): a given local id kept, a 0 the next id the
/// definition never had (past its own and the given ones), in the list's order; linked texts unlinked as a copy's.
fn numbered(old: &BlockDefinition, list: Vec<Entity>) -> Vec<Entity> {
    let most = old
        .entities
        .iter()
        .chain(&list)
        .map(|e| e.base().id)
        .max()
        .unwrap_or(0);
    let mut next = most;
    list.into_iter()
        .map(|e| {
            let mut e = crate::geometry::unlinked(e);
            if e.base().id == 0 {
                next = next.saturating_add(1);
                crate::blocks_define::base_mut(&mut e).id = next;
            }
            e
        })
        .collect()
}

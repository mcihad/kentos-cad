//! `cad.elevations.edit` v1 (docs/adr/0799, ADV-06f): a room's interior elevations derived from the model on a layer
//! of their own in the group İç görünüşler, with the layer's derivation record (docs/adr/0633 §2) naming the space,
//! the sides, the origin, each object by its role and the digest of what was drawn (`derive`); drawn again from the
//! model as it is now, the same objects kept by their roles (`refresh`); or the record removed, the drawing kept
//! (`release`). Each is one undo step “Oda iç görünüşü”; the plan is the tool's preview. The shared cases are
//! `fixtures/commands/v1/cad.elevations.edit.json` (`scripts/fixtures/room_elevation_cases.py`).
//!
//! Checks, in order: the operation's own (the space and its method, the sides, the gap, the origin, the name; the layer
//! and its record), the expected revision, then a locked layer (`layer_locked`).

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, CommandWarning, ELEVATION_GROUP, ELEVATION_STEP, ELEVATION_TOOL, ElevationRecord,
    ElevationsEdit, ElevationsEdited, ElevationsOperation, Entity, LayerDerivation, LayerNodeType,
};
use kentos_domain::{Document, NewLayer, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::elevations::{self, Drawn, bare};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// Where a derived elevation goes: the group (made when the drawing has none) and the new layer.
#[derive(Clone, Debug)]
struct Target {
    group: String,
    new_group: bool,
    layer: String,
    name: String,
}

/// What a refresh does to the layer's objects.
#[derive(Clone, Debug, Default)]
struct Changes {
    /// (role, the object as drawn, its id when it had one and it is free).
    add: Vec<(String, Entity, Option<Uuid>)>,
    update: Vec<(Slot, Entity)>,
    remove: Vec<Slot>,
}

enum Checked {
    Derive {
        target: Target,
        drawn: Drawn,
        sides: Vec<u32>,
    },
    Refresh {
        layer: String,
        name: String,
        record: ElevationRecord,
        drawn: Drawn,
        changes: Changes,
    },
    Release {
        layer: String,
        name: String,
    },
}

/// The first id of the form `{base}`, `{base}-2` … no node has.
fn free_id(doc: &Document, base: &str, numbered: bool) -> String {
    let layers = doc.layers();
    if numbered {
        (1u32..)
            .map(|k| format!("{base}-{k}"))
            .find(|id| layers.get(id).is_none())
            .unwrap_or_default()
    } else if layers.get(base).is_none() {
        base.to_owned()
    } else {
        (2u32..)
            .map(|k| format!("{base}-{k}"))
            .find(|id| layers.get(id).is_none())
            .unwrap_or_default()
    }
}

fn target(doc: &Document, name: Option<&str>, label: &str) -> Result<Target, Stop> {
    let layers = doc.layers();
    let existing = layers
        .nodes()
        .iter()
        .find(|n| n.kind == LayerNodeType::Group && n.name == ELEVATION_GROUP);
    let (group, new_group) = match existing {
        Some(g) => {
            if layers.is_locked(&g.id) {
                return Err(fail(
                    codes::LAYER_LOCKED,
                    format!("“{ELEVATION_GROUP}” grubu kilitli; iç görünüş katmanı açılamaz. Kilidi Katmanlar panelinden açın."),
                    "operation.space",
                ));
            }
            (g.id.clone(), false)
        }
        None => (free_id(doc, "ic-gorunusler", false), true),
    };
    let base = name.map_or_else(|| format!("{label} iç görünüşü"), str::to_owned);
    Ok(Target {
        group,
        new_group,
        layer: free_id(doc, "ic-gorunus", true),
        name: layers.unique_name(&base),
    })
}

fn locked_member(doc: &Document, r: &ElevationRecord) -> bool {
    r.members.values().any(|u| {
        Uuid::parse_str(u)
            .ok()
            .and_then(|x| doc.slot_of(x))
            .and_then(|s| doc.get(s))
            .is_some_and(|e| doc.layers().is_locked(&e.base().layer_id))
    })
}

/// The layer named by a refresh or a release, and its elevation record.
fn elevation_layer(doc: &Document, layer: &str) -> Result<(String, ElevationRecord), Stop> {
    let Some(node) = doc.layers().get(layer) else {
        return Err(fail(
            codes::LAYER_NOT_FOUND,
            format!("“{layer}” kimlikli katman çizimde yok. Bir iç görünüş katmanının kimliğini verin."),
            "operation.layer",
        ));
    };
    let Some((_, r)) = elevations::on_layer(doc, layer) else {
        return Err(fail(
            codes::ELEVATION_NOT_FOUND,
            format!("“{}” katmanı bir oda iç görünüşü değil (türetim kaydı yok).", node.name),
            "operation.layer",
        ));
    };
    Ok((node.name.clone(), r))
}

fn locked_layer(doc: &Document, layer: &str, name: &str, r: &ElevationRecord) -> Result<(), Stop> {
    if doc.layers().is_locked(layer) || locked_member(doc, r) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!("“{name}” iç görünüşünün katmanı ya da bir nesnesinin katmanı kilitli. Kilidi Katmanlar panelinden açın."),
            "operation.layer",
        ));
    }
    Ok(())
}

fn check(doc: &Document, input: &ElevationsEdit) -> Result<Checked, Stop> {
    match &input.operation {
        ElevationsOperation::Derive {
            space,
            sides,
            at,
            gap,
            name,
        } => {
            if let Some(n) = name
                && (n.trim().is_empty()
                    || n.trim() != n
                    || n.chars().count() > 200
                    || n.chars().any(char::is_control))
            {
                return Err(fail(
                    codes::INVALID_INPUT,
                    "Katmanın adı boş, baş ya da sonda boşluklu, 200 karakterden uzun ya da denetim karakterli olamaz.".into(),
                    "operation.name",
                ));
            }
            // The layer's id is needed for the objects; the space is checked first.
            let probe = elevations::draw(doc, space, sides, *at, *gap, "")?;
            let target = target(doc, name.as_deref(), &probe.label)?;
            let drawn = elevations::draw(doc, space, sides, *at, *gap, &target.layer)?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            Ok(Checked::Derive {
                target,
                drawn,
                sides: sides.clone(),
            })
        }
        ElevationsOperation::Refresh { layer } => {
            let (name, record) = elevation_layer(doc, layer)?;
            let drawn = match elevations::draw(doc, &record.space, &record.sides, Some(record.at), Some(record.gap), layer) {
                Ok(d) => d,
                Err(stop) => {
                    let e = stop.into_error();
                    if e.code == codes::ENTITY_NOT_FOUND || e.code == codes::NOT_A_SPACE {
                        return Err(fail(
                            codes::ELEVATION_SOURCE_GONE,
                            format!("“{name}” iç görünüşünün mahali çizimde yok; yeniden üretilemez. Bağı koparıp çizimi koruyabilir ya da katmanı silebilirsiniz."),
                            "operation.layer",
                        ));
                    }
                    return Err(Stop::failed(e));
                }
            };
            checks::revision(doc, input.expected_revision.as_deref())?;
            locked_layer(doc, layer, &name, &record)?;
            let mut changes = Changes::default();
            for (role, want) in &drawn.items {
                let had = record.members.get(role).and_then(|u| Uuid::parse_str(u).ok());
                match had.and_then(|u| doc.slot_of(u).map(|s| (u, s))) {
                    Some((_, slot)) => {
                        if doc.get(slot).is_some_and(|now| &bare(now) != want) {
                            changes.update.push((slot, want.clone()));
                        }
                    }
                    None => changes.add.push((role.clone(), want.clone(), had)),
                }
            }
            for (role, u) in &record.members {
                if drawn.items.iter().any(|(r, _)| r == role) {
                    continue;
                }
                if let Some(slot) = Uuid::parse_str(u).ok().and_then(|x| doc.slot_of(x)) {
                    changes.remove.push(slot);
                }
            }
            let same = elevations::digest(&drawn.items) == record.digest;
            if same && changes.add.is_empty() && changes.update.is_empty() && changes.remove.is_empty() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    format!("“{name}” iç görünüşü güncel; yeniden üretilecek bir şey yok."),
                    "operation.layer",
                ));
            }
            Ok(Checked::Refresh {
                layer: layer.clone(),
                name,
                record,
                drawn,
                changes,
            })
        }
        ElevationsOperation::Release { layer } => {
            let (name, record) = elevation_layer(doc, layer)?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            if doc.layers().is_locked(layer) {
                locked_layer(doc, layer, &name, &record)?;
            }
            Ok(Checked::Release {
                layer: layer.clone(),
                name,
            })
        }
    }
}

/// The warnings of a refresh: the drawing's, and the objects changed by hand written back.
fn refresh_warnings(doc: &Document, c: &Checked) -> Vec<CommandWarning> {
    let Checked::Refresh {
        layer,
        record,
        drawn,
        ..
    } = c
    else {
        return Vec::new();
    };
    let mut out = drawn.warnings.clone();
    let edited = elevations::standing(doc, layer, record).edited;
    if !edited.is_empty() {
        out.push(CommandWarning {
            code: codes::ELEVATION_EDITED_OVERWRITTEN.into(),
            message: format!(
                "İç görünüşün elle değiştirilmiş {} nesnesi modelin çizdiğiyle yeniden yazıldı.",
                edited.len()
            ),
            path: Some("operation.layer".into()),
        });
    }
    out
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ElevationsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: warnings_of(cx.doc, &c),
        },
        Err(stop) => stop.into(),
    }
}

fn warnings_of(doc: &Document, c: &Checked) -> Vec<CommandWarning> {
    match c {
        Checked::Derive { drawn, .. } => drawn.warnings.clone(),
        Checked::Refresh { .. } => refresh_warnings(doc, c),
        Checked::Release { .. } => Vec::new(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ElevationsEdit) -> CommandResult<ElevationsEdited> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the Oda iç görünüşü tool draws as its preview.
pub fn plan_in(doc: &Document, input: &ElevationsEdit) -> CommandResult<ElevationsEdited> {
    let c = match check(doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let warnings = warnings_of(doc, &c);
    let revision = doc.revision().to_string();
    let output = match c {
        Checked::Derive { target, drawn, .. } => ElevationsEdited {
            layer: target.layer,
            name: target.name,
            made: drawn.items.len() as u32,
            changed: 0,
            removed: 0,
            sides: drawn.sides,
            drawn: drawn.items.into_iter().map(|(_, e)| e).collect(),
            left_out: drawn.left_out,
            revision,
        },
        Checked::Refresh {
            layer,
            name,
            record,
            drawn,
            changes,
            ..
        } => {
            let shown = drawn
                .items
                .iter()
                .map(|(role, e)| {
                    let mut e = e.clone();
                    if let Some(slot) = record
                        .members
                        .get(role)
                        .and_then(|u| Uuid::parse_str(u).ok())
                        .and_then(|u| doc.slot_of(u))
                    {
                        e.base_mut().id = slot.0;
                    }
                    e
                })
                .collect();
            ElevationsEdited {
                layer,
                name,
                made: changes.add.len() as u32,
                changed: changes.update.len() as u32,
                removed: changes.remove.len() as u32,
                sides: drawn.sides,
                drawn: shown,
                left_out: drawn.left_out,
                revision,
            }
        }
        Checked::Release { layer, name, .. } => ElevationsEdited {
            layer,
            name,
            sides: Vec::new(),
            drawn: Vec::new(),
            made: 0,
            changed: 0,
            removed: 0,
            left_out: Vec::new(),
            revision,
        },
    };
    CommandResult::Completed { output, warnings }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ElevationsEdit) -> CommandResult<ElevationsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ELEVATIONS_EDIT,
        kentos_commands::CAD_ELEVATIONS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn refused(doc: &mut Document, group: kentos_domain::Group, why: String) -> CommandResult<ElevationsEdited> {
    doc.cancel_group(group);
    Stop::failed(error(codes::LAYER_REFUSED, why, Some("operation".into()))).into()
}

fn stamp(
    doc: &mut Document,
    layer: &str,
    record: &ElevationRecord,
    revision: String,
) -> Result<(), String> {
    let derivation = LayerDerivation {
        tool: ELEVATION_TOOL.into(),
        label: ELEVATION_STEP.into(),
        tool_version: kentos_commands::CAD_ELEVATIONS_EDIT_VERSION,
        parameters: elevations::parameters(record),
        inputs: Vec::new(),
        document_revision: revision,
        engine: format!("KentOS {}", env!("CARGO_PKG_VERSION")),
        at: elevations::utc_now(),
    };
    let Some(mut info) = doc.layers().info_of(layer) else {
        return Err("İç görünüşün katmanı bulunamadı.".into());
    };
    info.derivation = Some(derivation);
    doc.set_layer_info(layer, info, ELEVATION_STEP)
        .map(|_| ())
        .map_err(|r| r.to_string())
}

fn members_of(doc: &Document, roles: &[String], slots: &[Slot]) -> BTreeMap<String, String> {
    roles
        .iter()
        .zip(slots)
        .filter_map(|(r, s)| Some((r.clone(), doc.uid(*s)?.to_string())))
        .collect()
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ElevationsEdit) -> CommandResult<ElevationsEdited> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let warnings = warnings_of(cx.doc, &c);
    let doc = &mut *cx.doc;
    let before = doc.revision().to_string();
    let group = doc.begin_group(ELEVATION_STEP);
    match c {
        Checked::Derive {
            target,
            drawn,
            sides,
        } => {
            if target.new_group {
                let mut g = NewLayer::group(ELEVATION_GROUP);
                g.id = Some(target.group.clone());
                if let Err(r) = doc.add_layer(g, None, false) {
                    return refused(doc, group, r.to_string());
                }
            }
            let mut l = NewLayer::layer(target.name.clone());
            l.id = Some(target.layer.clone());
            if let Err(r) = doc.add_layer(l, Some(&target.group), false) {
                return refused(doc, group, r.to_string());
            }
            let roles: Vec<String> = drawn.items.iter().map(|(r, _)| r.clone()).collect();
            let entities: Vec<Entity> = drawn.items.iter().map(|(_, e)| e.clone()).collect();
            let slots = match doc.add_many(entities, ELEVATION_STEP) {
                Ok(s) => s,
                Err(full) => {
                    doc.cancel_group(group);
                    return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
                }
            };
            let record = ElevationRecord {
                space: match &input.operation {
                    ElevationsOperation::Derive { space, .. } => space.clone(),
                    _ => String::new(),
                },
                sides,
                at: drawn.at,
                gap: drawn.gap,
                members: members_of(doc, &roles, &slots),
                digest: elevations::digest(&drawn.items),
            };
            if let Err(why) = stamp(doc, &target.layer, &record, before) {
                return refused(doc, group, why);
            }
            doc.end_group(group);
            let made = slots.iter().filter_map(|s| doc.get(*s).cloned()).collect::<Vec<_>>();
            CommandResult::Completed {
                output: ElevationsEdited {
                    layer: target.layer,
                    name: target.name,
                    made: made.len() as u32,
                    changed: 0,
                    removed: 0,
                    sides: drawn.sides,
                    drawn: made,
                    left_out: drawn.left_out,
                    revision: doc.revision().to_string(),
                },
                warnings,
            }
        }
        Checked::Refresh {
            layer,
            name,
            mut record,
            drawn,
            changes,
            ..
        } => {
            if !changes.remove.is_empty() {
                doc.remove(&changes.remove);
            }
            let updates: Vec<(Slot, Entity)> = changes
                .update
                .iter()
                .map(|(slot, e)| {
                    let mut e = e.clone();
                    e.base_mut().id = slot.0;
                    (*slot, e)
                })
                .collect();
            let changed = updates.len() as u32;
            if !updates.is_empty() {
                doc.update_many(updates, ELEVATION_STEP);
            }
            let (keep, fresh): (Vec<_>, Vec<_>) = changes
                .add
                .iter()
                .partition(|(_, _, u)| u.is_some_and(|u| doc.slot_of(u).is_none()));
            let mut added: Vec<(String, Slot)> = Vec::new();
            if !keep.is_empty() {
                let with: Vec<(Uuid, Entity)> = keep
                    .iter()
                    .map(|(_, e, u)| (u.expect("partitioned on an id"), e.clone()))
                    .collect();
                match doc.add_many_with_uids(with, ELEVATION_STEP) {
                    Ok(slots) => added.extend(keep.iter().map(|(r, _, _)| r.clone()).zip(slots)),
                    Err(r) => return refused(doc, group, r.to_string()),
                }
            }
            if !fresh.is_empty() {
                match doc.add_many(fresh.iter().map(|(_, e, _)| e.clone()).collect(), ELEVATION_STEP) {
                    Ok(slots) => added.extend(fresh.iter().map(|(r, _, _)| r.clone()).zip(slots)),
                    Err(full) => {
                        doc.cancel_group(group);
                        return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
                    }
                }
            }
            let mut members = BTreeMap::new();
            for (role, _) in &drawn.items {
                if let Some((_, s)) = added.iter().find(|(r, _)| r == role) {
                    if let Some(u) = doc.uid(*s) {
                        members.insert(role.clone(), u.to_string());
                    }
                } else if let Some(u) = record.members.get(role) {
                    members.insert(role.clone(), u.clone());
                }
            }
            record.members = members;
            record.digest = elevations::digest(&drawn.items);
            if let Err(why) = stamp(doc, &layer, &record, before) {
                return refused(doc, group, why);
            }
            doc.end_group(group);
            let shown: Vec<Entity> = drawn
                .items
                .iter()
                .filter_map(|(role, _)| {
                    let u = Uuid::parse_str(record.members.get(role)?).ok()?;
                    doc.get(doc.slot_of(u)?).cloned()
                })
                .collect();
            CommandResult::Completed {
                output: ElevationsEdited {
                    layer,
                    name,
                    made: added.len() as u32,
                    changed,
                    removed: changes.remove.len() as u32,
                    sides: drawn.sides,
                    drawn: shown,
                    left_out: drawn.left_out,
                    revision: doc.revision().to_string(),
                },
                warnings,
            }
        }
        Checked::Release { layer, name, .. } => {
            let Some(mut info) = doc.layers().info_of(&layer) else {
                doc.cancel_group(group);
                return fail(codes::LAYER_NOT_FOUND, "Katman bulunamadı.".into(), "operation.layer").into();
            };
            info.derivation = None;
            if let Err(r) = doc.set_layer_info(&layer, info, ELEVATION_STEP) {
                return refused(doc, group, r.to_string());
            }
            doc.end_group(group);
            CommandResult::Completed {
                output: ElevationsEdited {
                    layer,
                    name,
                    sides: Vec::new(),
                    drawn: Vec::new(),
                    made: 0,
                    changed: 0,
                    removed: 0,
                    left_out: Vec::new(),
                    revision: doc.revision().to_string(),
                },
                warnings,
            }
        }
    }
}

//! `cad.time.version` v1 (docs/adr/0210 §7, 0622 §9): Yeni sürüm oluştur and Sona erdir. Objects of ranged temporal
//! layers end at a date; Yeni sürüm oluştur also writes beside each a copy with every property, starting at the date
//! and ending where the object ended. The checks are the tools' (Every object on a layer with a start and an end
//! field, not locked, its dates readable, its time holding the date); the writes are `cad.entities.set`'s (the end)
//! and `cad.entities.create`'s (the copies), one undo step named after the tool. A layer whose start or end field is
//! of the date kind (docs/adr/0199) takes the day alone. The shared cases are `fixtures/commands/v1/cad.time.version.json`
//! (`scripts/fixtures/time_version_command_cases.py`).

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, CommandWarning, EntitiesCreate, EntitiesEditPlan, EntitiesSetProperties,
    LayerFieldKind, PropertiesOperation, TimeVersion, TimeVersionKind, TimeVersionLayer,
    TimeVersioned, check_value,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::time::{self, Read, Unit};

use crate::checks::{self, Stop, error};
use crate::layers_edit::new_object;
use crate::{ExecutionContext, codes, create, set};

/// The undo steps' names, the tools'.
pub const VERSION_STEP: &str = "Yeni sürüm oluştur";
pub const END_STEP: &str = "Sona erdir";

fn refuse(code: &str, message: String, path: String) -> Stop {
    Stop::failed(error(code, message, Some(path)))
}

/// A layer's objects and what is written on them.
struct Group {
    layer: String,
    start: String,
    end: String,
    /// The date on the layer: the one given, or its day's midnight.
    at: f64,
    objects: Vec<(Slot, String)>,
    /// Where the input first names one of them.
    first: usize,
}

struct Checked {
    groups: Vec<Group>,
    /// The objects in the input's order, a repeat once.
    order: Vec<(Slot, String)>,
    /// The new versions a hidden layer will not show.
    warnings: Vec<CommandWarning>,
}

/// The date given, as the layers' attributes are read.
fn moment(text: &str) -> Result<f64, Stop> {
    match time::read(text) {
        Read::Moment(t) => Ok(t),
        _ => Err(refuse(
            codes::INVALID_MOMENT,
            format!(
                "“{text}” bir tarih olarak okunamadı; 05.03.2024, 2024-03-05 ya da 2024-03-05T10:30 gibi yazın."
            ),
            "at".into(),
        )),
    }
}

fn check(doc: &Document, input: &TimeVersion) -> Result<Checked, Stop> {
    checks::uids(&input.uids, "Sürümü yazılacak nesne verilmedi.")?;
    let given = moment(&input.at)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let named = checks::named(doc, &input.uids)?;
    let layers = doc.layers();
    let mut groups: Vec<Group> = Vec::new();
    let mut order = Vec::with_capacity(named.len());
    // The objects whose time does not hold the date: the first one's place, and how many.
    let mut outside: Option<(usize, String)> = None;
    let mut outside_count = 0usize;
    for (i, slot, entity, uid) in named {
        let layer = entity.base().layer_id.as_str();
        let node = layers.get(layer);
        let name = node.map_or(layer, |n| n.name.as_str());
        let Some((start, end)) = node
            .and_then(|n| n.time.as_ref())
            .and_then(|t| Some((t.start.clone(), t.end.clone()?)))
        else {
            return Err(refuse(
                codes::NOT_RANGED,
                format!(
                    "“{name}” katmanı başlangıç ve bitiş alanı olan bir zamansal katman değil; sürüm yalnız aralıklı katmanda yapılır. Katmanın zaman ayarlarına bakın. Hiçbir nesne yazılmadı."
                ),
                format!("uids[{i}]"),
            ));
        };
        if layers.is_locked(layer) {
            return Err(refuse(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; hiçbir nesne yazılmadı. Kilidi Katmanlar panelinden açın."
                ),
                format!("uids[{i}]"),
            ));
        }
        let attrs = &entity.base().attrs;
        let read = |k: &str| attrs.get(k).map_or(Read::Empty, |v| time::read(v));
        let (s, t) = (read(&start), read(&end));
        if s == Read::Unreadable || t == Read::Unreadable {
            return Err(refuse(
                codes::TIME_UNREADABLE,
                format!(
                    "“{name}” katmanındaki bir nesnenin başlangıcı ya da bitişi tarih olarak okunamıyor; önce Öznitelikler’den düzeltin. Hiçbir nesne yazılmadı."
                ),
                format!("uids[{i}]"),
            ));
        }
        if input.kind == TimeVersionKind::Version && new_object(entity).is_none() {
            return Err(refuse(
                codes::NOT_VERSIONABLE,
                format!(
                    "“{name}” katmanındaki nesnenin yeni sürümü yazılamaz: kayıtların ve mimari nesnelerin kopyası kendi komutlarıyladır. Nesneyi yalnız sona erdirebilirsiniz."
                ),
                format!("uids[{i}]"),
            ));
        }
        // A date field of the layer's takes the day alone: the date is that day's midnight.
        let days = node.is_some_and(|n| {
            n.fields
                .iter()
                .any(|f| f.kind == LayerFieldKind::Date && (f.name == start || f.name == end))
        });
        let at = if days {
            time::floor_to(given, Unit::Day)
        } else {
            given
        };
        let from = s.moment().unwrap_or(f64::NEG_INFINITY);
        let to = t.moment().unwrap_or(f64::INFINITY);
        if !(from < at && at < to) {
            outside_count += 1;
            outside.get_or_insert_with(|| (i, time::show(at, Unit::Day)));
        }
        order.push((slot, uid.clone()));
        match groups.iter_mut().find(|g| g.layer == layer) {
            Some(g) => g.objects.push((slot, uid.clone())),
            None => groups.push(Group {
                layer: layer.to_owned(),
                start,
                end,
                at,
                objects: vec![(slot, uid.clone())],
                first: i,
            }),
        }
    }
    if let Some((i, when)) = outside {
        return Err(refuse(
            codes::NOT_WITHIN,
            format!(
                "{outside_count} nesnenin zamanı {when} anını içermiyor: başlangıcı bu tarihten önce, bitişi sonra olmalı. Hiçbir nesne yazılmadı."
            ),
            format!("uids[{i}]"),
        ));
    }
    // A typed end field (and, for the copies, start field) must take the date as its text: refused before writing.
    for g in &groups {
        let text = time::write(g.at, false);
        let fields = layers
            .get(&g.layer)
            .map(|n| n.fields.as_slice())
            .unwrap_or_default();
        let named = [
            Some(&g.end),
            (input.kind == TimeVersionKind::Version).then_some(&g.start),
        ];
        for name in named.into_iter().flatten() {
            if let Some(field) = fields.iter().find(|f| f.name == *name)
                && let Err(refusal) = check_value(field, &text)
            {
                return Err(refuse(
                    codes::INVALID_ATTRIBUTE,
                    refusal.message,
                    "at".into(),
                ));
            }
        }
    }
    let warnings = groups
        .iter()
        .filter(|g| input.kind == TimeVersionKind::Version && !layers.is_visible(&g.layer))
        .map(|g| {
            let name = layers
                .get(&g.layer)
                .map_or(g.layer.as_str(), |n| n.name.as_str());
            CommandWarning {
                code: codes::LAYER_HIDDEN.into(),
                message: format!("“{name}” katmanı gizli; yeni sürümler görünmeyecek."),
                path: Some(format!("uids[{}]", g.first)),
            }
        })
        .collect();
    Ok(Checked {
        groups,
        order,
        warnings,
    })
}

/// A write's warnings but the new versions' hidden layer, which the command says itself.
fn others(warnings: Vec<CommandWarning>) -> impl Iterator<Item = CommandWarning> {
    warnings
        .into_iter()
        .filter(|w| w.code != codes::LAYER_HIDDEN)
}

/// The end written on a layer's objects.
fn set_input(group: &Group) -> EntitiesSetProperties {
    let text = time::write(group.at, false);
    EntitiesSetProperties {
        line_pattern: None,
        line_type: None,
        line_type_scale: None,
        transparency: None,
        wipeout: None,
        uids: group.objects.iter().map(|(_, u)| u.clone()).collect(),
        layer_id: None,
        color: None,
        line_weight: None,
        symbol: None,
        attrs: Some(BTreeMap::from([(group.end.clone(), Some(text))])),
        label: None,
        operation: PropertiesOperation::Attributes,
        expected_revision: None,
        unlink: false,
        attachments: None,
    }
}

/// A layer's copies as its objects are before their end changes: each starts at the date and ends where the object
/// ended (an object without an end makes one without).
fn create_input(doc: &Document, group: &Group) -> EntitiesCreate {
    let text = time::write(group.at, false);
    let objects = group
        .objects
        .iter()
        .filter_map(|(slot, _)| doc.get(*slot))
        .filter_map(|e| {
            let mut o = new_object(e)?;
            let mut attrs = e.base().attrs.clone();
            attrs.insert(group.start.clone(), text.clone());
            if !e.base().attrs.contains_key(&group.end) {
                attrs.remove(&group.end);
            }
            o.attrs = Some(attrs);
            Some(o)
        })
        .collect();
    EntitiesCreate {
        layer_id: group.layer.clone(),
        objects,
        operation: None,
        expected_revision: None,
    }
}

/// The checks, and those of the writes.
pub fn validate(cx: &ExecutionContext<'_>, input: &TimeVersion) -> CommandResult<()> {
    match plan_in(cx.doc, input) {
        CommandResult::Completed { warnings, .. } => CommandResult::Completed {
            output: (),
            warnings,
        },
        other => crate::trim::relay(other),
    }
}

/// The objects as their end would be written and their new versions as they would be made.
pub fn plan(cx: &ExecutionContext<'_>, input: &TimeVersion) -> CommandResult<EntitiesEditPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document (the tool's).
pub fn plan_in(doc: &Document, input: &TimeVersion) -> CommandResult<EntitiesEditPlan> {
    let checked = match check(doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut plan = EntitiesEditPlan {
        changed: Vec::new(),
        created: Vec::new(),
        removed: Vec::new(),
        revision: doc.revision().to_string(),
    };
    let Checked {
        groups,
        mut warnings,
        ..
    } = checked;
    for group in &groups {
        match set::plan_in(doc, &set_input(group)) {
            CommandResult::Completed {
                output,
                warnings: w,
            } => {
                plan.changed.extend(output.changed);
                warnings.extend(others(w));
            }
            other => return crate::trim::relay(other),
        }
        if input.kind == TimeVersionKind::Version {
            match create::plan_in(doc, &create_input(doc, group)) {
                CommandResult::Completed {
                    output,
                    warnings: w,
                } => {
                    plan.created.extend(output.entities);
                    warnings.extend(others(w));
                }
                other => return crate::trim::relay(other),
            }
        }
    }
    CommandResult::Completed {
        output: plan,
        warnings,
    }
}

/// Ends the objects at the date (and writes their new versions), one undo step named after the tool.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TimeVersion) -> CommandResult<TimeVersioned> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_TIME_VERSION,
        kentos_commands::CAD_TIME_VERSION_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: TimeVersion,
) -> CommandResult<TimeVersioned> {
    let checked = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let label = match input.kind {
        TimeVersionKind::Version => VERSION_STEP,
        TimeVersionKind::End => END_STEP,
    };
    let Checked {
        groups,
        order,
        mut warnings,
    } = checked;
    let group = cx.doc.begin_group(label);
    // Each object's copy: its id and slot.
    let mut copies: BTreeMap<u32, (String, u32)> = BTreeMap::new();
    let mut layers = Vec::with_capacity(groups.len());
    for g in &groups {
        let made = (input.kind == TimeVersionKind::Version).then(|| create_input(cx.doc, g));
        match set::execute(cx, set_input(g)) {
            CommandResult::Completed { warnings: w, .. } => warnings.extend(others(w)),
            other => {
                cx.doc.cancel_group(group);
                return crate::trim::relay(other);
            }
        }
        // What the layer's end field holds now: the date in the field's own text.
        let written = g
            .objects
            .first()
            .and_then(|(slot, _)| cx.doc.get(*slot))
            .and_then(|e| e.base().attrs.get(&g.end).cloned())
            .unwrap_or_else(|| time::write(g.at, false));
        layers.push(TimeVersionLayer {
            layer: g.layer.clone(),
            at: written,
        });
        let Some(made) = made else {
            continue;
        };
        match create::execute(cx, made) {
            CommandResult::Completed {
                output,
                warnings: w,
            } => {
                warnings.extend(others(w));
                for ((slot, _), (uid, id)) in g
                    .objects
                    .iter()
                    .zip(output.created.into_iter().zip(output.ids))
                {
                    copies.insert(slot.0, (uid, id));
                }
            }
            other => {
                cx.doc.cancel_group(group);
                return crate::trim::relay(other);
            }
        }
    }
    cx.doc.end_group(group);
    let (created, ids) = order
        .iter()
        .filter_map(|(slot, _)| copies.get(&slot.0).cloned())
        .unzip();
    CommandResult::Completed {
        output: TimeVersioned {
            kind: input.kind,
            ended: order.into_iter().map(|(_, u)| u).collect(),
            created,
            ids,
            layers,
            revision: cx.doc.revision().to_string(),
        },
        warnings,
    }
}

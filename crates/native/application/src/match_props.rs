//! `cad.entities.match` v1 (docs/adr/0304; AutoCAD's MATCHPROP, Özellik kopyala): the properties chosen of one object
//! given to others, as one undo step “Özellik kopyala”. The shared cases are
//! `fixtures/commands/v1/cad.entities.match.json`.
//!
//! The object's own properties (layer, colour, line type and its factor, line weight, transparency, symbol) go through
//! `cad.entities.set` ([`crate::set`]), so moving to a layer keeps its rules (locked, hidden, fields); the kind's
//! (a text's face, height and width factor; a dimension's look, value height and background; a hatch's pattern; a
//! leader's arrowhead, its size, the note's height and background) are written on the objects as they are after that,
//! everything else kept (a text's link to the object it labels too).

use kentos_contracts::{
    CommandResult, CommandWarning, EntitiesEditPlan, EntitiesEdited, EntitiesMatch,
    EntitiesSetProperties, Entity, MatchProperty, PropertiesOperation,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
use crate::codes;
use crate::set;
use crate::trim::relay;

/// The undo step's name.
pub const LABEL: &str = "Özellik kopyala";

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn bad_uid(uid: &str, path: String) -> Stop {
    Stop::failed(error(
        codes::INVALID_UID,
        format!(
            "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
        ),
        Some(path),
    ))
}

/// What the checks give: the source, the targets (slot and id, the source and locked ones left out), the
/// properties, and the warnings.
struct Checked {
    source: Entity,
    targets: Vec<(Slot, String)>,
    properties: Vec<MatchProperty>,
    warnings: Vec<CommandWarning>,
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &EntitiesMatch) -> Result<Checked, Stop> {
    if !is_uid_text(&input.source) {
        return Err(bad_uid(&input.source, "source".into()));
    }
    checks::uids(&input.uids, "Özellik verilecek nesne verilmedi.")?;
    let properties: Vec<MatchProperty> = match &input.properties {
        None => MatchProperty::ALL.to_vec(),
        Some(list) => {
            let mut seen = Vec::new();
            for p in list {
                if !seen.contains(p) {
                    seen.push(*p);
                }
            }
            seen
        }
    };
    if properties.is_empty() {
        return Err(refuse(
            codes::NO_PROPERTIES,
            "Kopyalanacak özellik seçilmedi. En az bir özellik verin ya da listeyi vermeyin (hepsi).",
            "properties",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let source_slot = checks::named(doc, std::slice::from_ref(&input.source))
        .map_err(|stop| {
            let mut e = stop.into_error();
            e.path = Some("source".into());
            Stop::failed(e)
        })?
        .first()
        .map(|(_, slot, _, _)| *slot);
    let Some(source) = source_slot.and_then(|s| doc.get(s)).cloned() else {
        return Err(refuse(
            codes::ENTITY_NOT_FOUND,
            "Kaynak nesne çizimde yok.",
            "source",
        ));
    };
    let found: Vec<_> = checks::named(doc, &input.uids)?
        .into_iter()
        .filter(|(_, slot, _, _)| Some(*slot) != source_slot)
        .collect();
    if found.is_empty() {
        return Err(refuse(
            codes::NO_ENTITIES,
            "Kaynağın kendisi dışında özellik verilecek nesne yok. Başka nesneler verin.",
            "uids",
        ));
    }
    let (open, locked) = checks::skip_locked(doc, found)?;
    Ok(Checked {
        source,
        targets: open
            .into_iter()
            .map(|(_, slot, _, uid)| (slot, uid.clone()))
            .collect(),
        properties,
        warnings: locked.into_iter().collect(),
    })
}

/// The kind's properties of `source` written on `target` when they are of one kind and the property is asked for;
/// none when nothing changes.
fn kind_props(source: &Entity, target: &Entity, asked: &[MatchProperty]) -> Option<Entity> {
    let mut out = target.clone();
    match (source, &mut out) {
        (Entity::Text(s), Entity::Text(t)) if asked.contains(&MatchProperty::Text) => {
            t.face = s.face.clone();
            t.height = s.height;
            t.width_factor = s.width_factor;
        }
        (Entity::Dimension(s), Entity::Dimension(t))
            if asked.contains(&MatchProperty::Dimension) =>
        {
            t.look = s.look.clone();
            t.height = s.height;
            t.mask = s.mask;
        }
        (Entity::Hatch(s), Entity::Hatch(t)) if asked.contains(&MatchProperty::Hatch) => {
            t.pattern = s.pattern.clone();
        }
        (Entity::Leader(s), Entity::Leader(t)) if asked.contains(&MatchProperty::Leader) => {
            t.arrow = s.arrow;
            t.arrow_size = s.arrow_size;
            t.height = s.height;
            t.mask = s.mask;
        }
        _ => return None,
    }
    (&out != target).then_some(out)
}

/// The writes: the object's own properties through `cad.entities.set` (a line weight only between objects drawn with
/// lines), then the kind's on the objects as they are now. The ids that changed, in the input's order.
fn write(
    cx: &mut ExecutionContext<'_>,
    checked: Checked,
) -> CommandResult<(Vec<String>, Vec<CommandWarning>)> {
    let Checked {
        source,
        targets,
        properties,
        mut warnings,
    } = checked;
    cx.doc.name_step(LABEL);
    let has = |p: MatchProperty| properties.contains(&p);
    let base = source.base();
    let mut changed: Vec<String> = Vec::new();
    let own = [
        MatchProperty::Layer,
        MatchProperty::Color,
        MatchProperty::LineType,
        MatchProperty::LineTypeScale,
        MatchProperty::LineWeight,
        MatchProperty::Transparency,
        MatchProperty::Symbol,
    ];
    if own.iter().any(|p| has(*p)) {
        // A weight goes only to what is drawn with lines, and only from what is.
        let (lined, plain): (Vec<_>, Vec<_>) = targets
            .iter()
            .partition(|(slot, _)| cx.doc.get(*slot).is_some_and(Entity::draws_lines));
        for (group, weight) in [(lined, source.draws_lines()), (plain, false)] {
            let weight = weight && has(MatchProperty::LineWeight);
            let anything = weight
                || own
                    .iter()
                    .any(|p| *p != MatchProperty::LineWeight && has(*p));
            if group.is_empty() || !anything {
                continue;
            }
            let input = EntitiesSetProperties {
                // The source's pattern goes with its line type (docs/adr/0313 §1).
                line_pattern: has(MatchProperty::LineType).then(|| base.look.line_pattern.clone()),
                uids: group.iter().map(|(_, uid)| uid.clone()).collect(),
                layer_id: has(MatchProperty::Layer).then(|| base.layer_id.clone()),
                color: has(MatchProperty::Color).then(|| base.color.clone()),
                line_weight: weight.then_some(base.line_weight),
                line_type: has(MatchProperty::LineType).then_some(base.look.line_type),
                line_type_scale: has(MatchProperty::LineTypeScale)
                    .then_some(base.look.line_type_scale),
                transparency: has(MatchProperty::Transparency).then_some(base.look.transparency),
                wipeout: None,
                symbol: has(MatchProperty::Symbol).then(|| base.symbol.clone()),
                attrs: None,
                label: None,
                attachments: None,
                unlink: false,
                operation: PropertiesOperation::Layer,
                expected_revision: None,
            };
            // The layer given is the source's: its words are said at `source`.
            let at_source = |path: &mut Option<String>| {
                if path.as_deref() == Some("layerId") {
                    *path = Some("source".into());
                }
            };
            match set::execute(cx, input) {
                CommandResult::Completed {
                    output,
                    warnings: w,
                } => {
                    changed.extend(output.changed);
                    warnings.extend(w.into_iter().map(|mut w| {
                        at_source(&mut w.path);
                        w
                    }));
                }
                CommandResult::Failed { mut error } => {
                    at_source(&mut error.path);
                    return CommandResult::Failed { error };
                }
                other => return relay(other),
            }
        }
    }
    let kind: Vec<(Slot, Entity)> = targets
        .iter()
        .filter_map(|(slot, _)| {
            let now = cx.doc.get(*slot)?;
            kind_props(&source, now, &properties).map(|e| (*slot, e))
        })
        .collect();
    for (slot, _) in &kind {
        if let Some(uid) = targets.iter().find(|(s, _)| s == slot).map(|(_, u)| u)
            && !changed.contains(uid)
        {
            changed.push(uid.clone());
        }
    }
    if !kind.is_empty() {
        cx.doc.update_many(kind, LABEL);
    }
    // Each warning once: the two writes can say the same.
    let mut said: Vec<CommandWarning> = Vec::new();
    for w in warnings {
        if !said.contains(&w) {
            said.push(w);
        }
    }
    let warnings = said;
    // In the input's order.
    let order: Vec<String> = targets
        .iter()
        .filter(|(_, uid)| changed.contains(uid))
        .map(|(_, uid)| uid.clone())
        .collect();
    CommandResult::Completed {
        output: (order, Vec::new()),
        warnings,
    }
}

/// `cad.entities.match` v1: Özellik kopyala.
pub mod entities_match {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesMatch) -> CommandResult<()> {
        match plan_in(cx.doc, input) {
            CommandResult::Completed { warnings, .. } => CommandResult::Completed {
                output: (),
                warnings,
            },
            other => relay(other),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &EntitiesMatch,
    ) -> CommandResult<EntitiesEditPlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over the document alone: the write on a copy, the objects as it leaves them.
    pub fn plan_in(doc: &Document, input: &EntitiesMatch) -> CommandResult<EntitiesEditPlan> {
        if let Err(stop) = check(doc, input) {
            return stop.into();
        }
        let mut copy = doc.clone();
        let mut cx = ExecutionContext::new(&mut copy);
        match execute(&mut cx, input.clone()) {
            CommandResult::Completed { output, warnings } => {
                let changed = output
                    .changed
                    .iter()
                    .filter_map(|uid| {
                        let slot = kentos_domain::Uuid::parse_str(uid)
                            .ok()
                            .and_then(|u| copy.slot_of(u))?;
                        copy.get(slot).cloned()
                    })
                    .collect();
                CommandResult::Completed {
                    output: EntitiesEditPlan {
                        changed,
                        created: Vec::new(),
                        removed: Vec::new(),
                        revision: doc.revision().to_string(),
                    },
                    warnings,
                }
            }
            other => relay(other),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: EntitiesMatch,
    ) -> CommandResult<EntitiesEdited> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_ENTITIES_MATCH,
            kentos_contracts::CAD_ENTITIES_MATCH_VERSION,
            input,
            |cx, input| {
                let checked = match check(cx.doc, &input) {
                    Ok(checked) => checked,
                    Err(stop) => return stop.into(),
                };
                match write(cx, checked) {
                    CommandResult::Completed {
                        output: (changed, _),
                        warnings,
                    } => CommandResult::Completed {
                        output: EntitiesEdited {
                            changed,
                            created: Vec::new(),
                            removed: Vec::new(),
                            revision: cx.doc.revision().to_string(),
                        },
                        warnings,
                    },
                    other => relay(other),
                }
            },
        )
    }
}

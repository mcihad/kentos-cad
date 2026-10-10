//! `cad.labels.pin` v1 (docs/adr/0212 §5): labels moved, turned or hidden by
//! hand, or freed, as one undo step “Etiket”. A label is an object's and a
//! class's (a rule-based layer's name; none, the object's first label); its
//! pin is kept on the object (`EntityBase.label_pins`), so it goes with the
//! object when it is moved or copied. The desktop's handler over the native
//! document; the web's is `apps/web/src/product/labelsPin.ts`. Both pass the
//! shared cases in `fixtures/commands/v1/cad.labels.pin.json`.
//!
//! The checks, in the contract's order: at least one change; each id is one;
//! each pin by its rules and its change's class; no label twice; the
//! expected revision; each object is the drawing's, on an unlocked layer,
//! its class one its layer labels with; the pins each object would have by
//! their rules. A change that leaves a pin as it is changes nothing.

use kentos_contracts::{
    CommandResult, Entity, LabelPin, LabelsMode, LabelsPin, LabelsPinPlan, LabelsPinned,
    PinnedObject, pins_problem,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
/// The stable codes of the answers (`CommandError.code`).
pub use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Etiket";

fn fail(code: &str, message: String, path: String) -> Stop {
    Stop::Failed(error(code, message, Some(path)))
}

/// The objects whose pins change, each with its pins as they would be, in the input's order.
fn check(doc: &Document, input: &LabelsPin) -> Result<Vec<(Slot, Entity, String)>, Stop> {
    if input.pins.is_empty() {
        return Err(fail(
            codes::NO_ENTITIES,
            "Değişecek etiket yok. En az bir nesnenin etiketini verin.".into(),
            "pins".into(),
        ));
    }
    for (i, c) in input.pins.iter().enumerate() {
        if !is_uid_text(&c.uid) {
            return Err(fail(
                codes::INVALID_UID,
                format!(
                    "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).",
                    c.uid
                ),
                format!("pins[{i}]/uid"),
            ));
        }
        if let Some(p) = &c.pin {
            if p.class.is_some() && p.class != c.class {
                return Err(fail(
                    codes::INVALID_PIN,
                    "İğnenin sınıfı değişikliğin sınıfıyla aynı olmalı (ya da verilmemeli).".into(),
                    format!("pins[{i}]/pin/class"),
                ));
            }
            let pin = LabelPin {
                class: c.class.clone(),
                ..p.clone()
            };
            if let Some(problem) = pins_problem(std::slice::from_ref(&pin)) {
                return Err(fail(
                    codes::INVALID_PIN,
                    format!("Etiket iğnesi: {problem}."),
                    format!("pins[{i}]/pin"),
                ));
            }
        }
        if input.pins[..i]
            .iter()
            .any(|d| d.uid == c.uid && d.class == c.class)
        {
            return Err(fail(
                codes::INVALID_PIN,
                "Aynı nesnenin aynı etiketi iki kez değişiyor; her etiketi bir kez verin.".into(),
                format!("pins[{i}]"),
            ));
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let layers = doc.layers();
    // Each object once, at its first change, its pins changed in the input's order.
    let mut objects: Vec<(Slot, Entity, String, usize)> = Vec::new();
    for (i, c) in input.pins.iter().enumerate() {
        let k = match objects.iter().position(|(_, _, uid, _)| *uid == c.uid) {
            Some(k) => k,
            None => {
                let found = Uuid::parse_str(&c.uid)
                    .ok()
                    .and_then(|u| doc.slot_of(u))
                    .and_then(|slot| Some((slot, doc.get(slot)?)));
                let Some((slot, entity)) = found else {
                    return Err(fail(
                        codes::ENTITY_NOT_FOUND,
                        format!(
                            "“{}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                            c.uid
                        ),
                        format!("pins[{i}]/uid"),
                    ));
                };
                let layer = &entity.base().layer_id;
                let node = layers.get(layer);
                let name = node.map_or(layer.as_str(), |n| n.name.as_str());
                if layers.is_locked(layer) {
                    return Err(fail(
                        codes::LAYER_LOCKED,
                        format!(
                            "“{name}” katmanı kilitli; üzerindeki nesnenin etiketi değişmez. Kilidi Katmanlar panelinden açın."
                        ),
                        format!("pins[{i}]/uid"),
                    ));
                }
                objects.push((slot, entity.clone(), c.uid.clone(), i));
                objects.len() - 1
            }
        };
        let (_, entity, _, _) = &mut objects[k];
        let layer = entity.base().layer_id.clone();
        let node = layers.get(&layer);
        let name = node.map_or(layer.as_str(), |n| n.name.as_str());
        let rules = node
            .and_then(|n| n.style.labels.as_ref())
            .filter(|l| l.mode == LabelsMode::Rules);
        if let Some(class) = &c.class {
            let known = rules.is_some_and(|l| l.classes.iter().any(|k| &k.name == class));
            if !known {
                let why = match rules {
                    Some(_) => format!("“{name}” katmanının “{class}” adlı etiket sınıfı yok."),
                    None => format!("“{name}” katmanının tek etiketi var; sınıfı verilmez."),
                };
                return Err(fail(codes::UNKNOWN_CLASS, why, format!("pins[{i}]/class")));
            }
        }
        let pins = &mut entity.base_mut().label_pins;
        let at = pins.iter().position(|p| p.class == c.class);
        match (&c.pin, at) {
            (Some(p), Some(at)) => {
                pins[at] = LabelPin {
                    class: c.class.clone(),
                    ..p.clone()
                };
            }
            (Some(p), None) => pins.push(LabelPin {
                class: c.class.clone(),
                ..p.clone()
            }),
            (None, Some(at)) => {
                pins.remove(at);
            }
            (None, None) => {}
        }
    }
    let mut out = Vec::with_capacity(objects.len());
    for (slot, entity, uid, first) in objects {
        if let Some(problem) = pins_problem(&entity.base().label_pins) {
            return Err(fail(
                codes::INVALID_PIN,
                format!("Etiket iğneleri: {problem}."),
                format!("pins[{first}]"),
            ));
        }
        let before = doc.get(slot).map(|e| &e.base().label_pins);
        if before != Some(&entity.base().label_pins) {
            out.push((slot, entity, uid));
        }
    }
    Ok(out)
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LabelsPin) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The objects whose pins would change, with their pins; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LabelsPin) -> CommandResult<LabelsPinPlan> {
    match check(cx.doc, input) {
        Ok(changes) => CommandResult::Completed {
            output: LabelsPinPlan {
                objects: changes
                    .into_iter()
                    .map(|(_, e, uid)| PinnedObject {
                        uid,
                        label_pins: e.base().label_pins.clone(),
                    })
                    .collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the pins as one undo step “Etiket” when any changes.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LabelsPin) -> CommandResult<LabelsPinned> {
    let changes = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let changed = doc.update_many(
        changes.into_iter().map(|(slot, e, _)| (slot, e)).collect(),
        LABEL,
    ) as u32;
    CommandResult::Completed {
        output: LabelsPinned {
            changed,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

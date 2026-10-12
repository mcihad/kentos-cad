//! `cad.layers.states` v1 (docs/adr/0248 §3): the project's named layer
//! states (docs/adr/0177 §4) saved, updated, renamed, removed and applied,
//! by the shared rules (`kentos_domain::layer_states`). The states are the
//! project's settings: changing them is an edit, no undo step. Applying one
//! changes the visibility and locks as the eye and the lock do (the tree's
//! own changes) and the styles in one undo step “Katman durumu: <ad>”. The
//! Katman durumları window and Katmanlar' Katman durumları ▾ write through
//! it; the shared cases are in `fixtures/commands/v1/cad.layers.states.json`.

use kentos_contracts::{
    CommandResult, LayerState, LayerStatesDone, LayerStatesOperation, LayersStates,
};
use kentos_domain::Document;
use kentos_domain::layer_states::{self, LayerStateParts};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The step applying a state's styles: “Katman durumu: <ad>”.
pub fn apply_label(name: &str) -> String {
    format!("Katman durumu: {name}")
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &LayersStates) -> Result<(), Stop> {
    use LayerStatesOperation as Op;
    let op = input.operation;
    if op != Op::Save && input.state.is_none() {
        return Err(fail(
            codes::NO_STATE,
            "Katman durumunu kimliğiyle verin (state).".into(),
            "state",
        ));
    }
    let blank = input.name.as_deref().map(checks::is_blank);
    if (op == Op::Rename && blank != Some(false)) || (op == Op::Save && blank == Some(true)) {
        return Err(fail(
            codes::EMPTY_NAME,
            "Durumun adını yazın.".into(),
            "name",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let states = &doc.settings().layer_states;
    if let Some(id) = input.state.as_deref().filter(|_| op != Op::Save)
        && !states.iter().any(|s| s.id == id)
    {
        return Err(fail(
            codes::STATE_NOT_FOUND,
            format!("“{id}” kimlikli katman durumu projede yok."),
            "state",
        ));
    }
    if matches!(op, Op::Save | Op::Rename)
        && let Some(name) = input.name.as_deref()
        && let Some(refused) = layer_states::name_refused(
            states,
            name.trim(),
            input.state.as_deref().filter(|_| op == Op::Rename),
        )
    {
        return Err(fail(codes::DUPLICATE_NAME, refused, "name"));
    }
    Ok(())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersStates) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do, on a reading copy; the drawing is left as it is.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersStates) -> CommandResult<LayerStatesDone> {
    if let Err(stop) = check(cx.doc, input) {
        return stop.into();
    }
    let mut copy = cx.doc.reading_copy();
    let mut output = write(&mut copy, input);
    output.revision = cx.doc.revision().to_string();
    CommandResult::Completed {
        output,
        warnings: Vec::new(),
    }
}

/// Saves, updates, renames, removes or applies the state.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersStates,
) -> CommandResult<LayerStatesDone> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    CommandResult::Completed {
        output: write(cx.doc, &input),
        warnings: Vec::new(),
    }
}

/// The project's states replaced by `states`: an edit, not an undo step.
fn write_states(doc: &mut Document, states: Vec<LayerState>) {
    let mut settings = doc.settings().clone();
    settings.layer_states = states;
    doc.set_settings(settings);
}

fn write(doc: &mut Document, input: &LayersStates) -> LayerStatesDone {
    use LayerStatesOperation as Op;
    let mut states = doc.settings().layer_states.clone();
    let id = input.state.clone().unwrap_or_default();
    let at = states.iter().position(|s| s.id == id);
    let mut out = LayerStatesDone {
        state: id.clone(),
        changed: false,
        visible: 0,
        locked: 0,
        styles: 0,
        missing: 0,
        revision: String::new(),
    };
    match (input.operation, at) {
        (Op::Save, _) => {
            let name = input.name.as_deref().map_or_else(
                || layer_states::next_state_name(&states),
                |n| n.trim().to_owned(),
            );
            let parts = LayerStateParts {
                locks: input.locks.unwrap_or(false),
                styles: input.styles.unwrap_or(true),
            };
            let state = layer_states::capture(
                doc.layers().nodes(),
                &layer_states::next_state_id(&states),
                &name,
                parts,
            );
            out.state.clone_from(&state.id);
            states.push(state);
            write_states(doc, states);
            out.changed = true;
        }
        (Op::Update, Some(at)) => {
            let kept = layer_states::parts_of(&states[at]);
            let parts = LayerStateParts {
                locks: input.locks.unwrap_or(kept.locks),
                styles: input.styles.unwrap_or(kept.styles),
            };
            let name = states[at].name.clone();
            let next = layer_states::capture(doc.layers().nodes(), &id, &name, parts);
            out.changed = states[at] != next;
            states[at] = next;
            write_states(doc, states);
        }
        (Op::Rename, Some(at)) => {
            let name = input.name.as_deref().unwrap_or_default().trim().to_owned();
            out.changed = states[at].name != name;
            states[at].name = name;
            write_states(doc, states);
        }
        (Op::Remove, Some(at)) => {
            states.remove(at);
            write_states(doc, states);
            out.changed = true;
        }
        (Op::Apply, Some(at)) => {
            let state = states[at].clone();
            let c = layer_states::changes(doc.layers().nodes(), &state);
            for (node, visible) in &c.visible {
                doc.set_layer_visible(node, *visible);
            }
            for (node, locked) in &c.locked {
                doc.set_layer_locked(node, *locked);
            }
            if !c.styles.is_empty() {
                let label = apply_label(&state.name);
                let group = doc.begin_group(&label);
                for (node, style) in &c.styles {
                    doc.set_layer_style(node, style.clone(), &label);
                }
                doc.end_group(group);
            }
            out.changed = !c.is_empty();
            out.visible = count(c.visible.len());
            out.locked = count(c.locked.len());
            out.styles = count(c.styles.len());
            out.missing = count(c.missing);
        }
        _ => {}
    }
    out.revision = doc.revision().to_string();
    out
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

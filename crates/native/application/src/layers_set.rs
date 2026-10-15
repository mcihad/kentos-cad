//! `cad.layers.set` v1 (docs/adr/0248 §2, 0311 §1–§2): the layer tree's own
//! changes, as the Katmanlar panel's eye, lock, freeze, name and Etkin katman
//! yap make them: an
//! edit of the drawing (dirty, a new revision), never an undo step
//! (docs/adr/0020, the shared document cases); the active layer is kept in
//! the file but is no edit. The shared cases are in
//! `fixtures/commands/v1/cad.layers.set.json`.

use kentos_contracts::{CommandResult, LayerNodeType, LayerSetOperation, LayersChanged, LayersSet};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &LayersSet) -> Result<(), Stop> {
    use LayerSetOperation as Op;
    let op = input.operation;
    if matches!(
        op,
        Op::Rename | Op::Visible | Op::Locked | Op::Plot | Op::Frozen | Op::Active
    ) && input.layer.is_none()
    {
        return Err(fail(
            codes::NO_LAYER,
            "Katmanı ya da grubu kimliğiyle verin (layer).".into(),
            "layer",
        ));
    }
    if op == Op::Isolate && input.layers.as_ref().is_none_or(Vec::is_empty) {
        return Err(fail(
            codes::NO_LAYERS,
            "Görünecek katmanları ya da grupları kimlikleriyle verin (layers).".into(),
            "layers",
        ));
    }
    if op == Op::Rename && input.name.as_deref().is_none_or(checks::is_blank) {
        return Err(fail(
            codes::EMPTY_NAME,
            "Katmanın adı boş olamaz; bir ad yazın.".into(),
            "name",
        ));
    }
    if matches!(op, Op::Visible | Op::Locked | Op::Plot | Op::Frozen) && input.value.is_none() {
        return Err(fail(
            codes::NO_VALUE,
            "Açık mı kapalı mı olduğunu value ile verin (true ya da false).".into(),
            "value",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let known = |id: &str, path: &str| {
        doc.layers().get(id).ok_or_else(|| {
            fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{id}” kimlikli katman ya da grup çizimde yok. Var olan bir düğümün kimliğini verin."
                ),
                path,
            )
        })
    };
    if let Some(id) = input
        .layer
        .as_deref()
        .filter(|_| !matches!(op, Op::Isolate | Op::ShowAll | Op::ThawAll | Op::Unisolate))
    {
        let n = known(id, "layer")?;
        if op == Op::Active {
            if n.kind == LayerNodeType::Group {
                return Err(fail(
                    codes::NOT_A_LAYER,
                    format!(
                        "“{}” bir katman grubu; etkin katman yalnız bir katman olur. Grubun bir katmanını verin.",
                        n.name
                    ),
                    "layer",
                ));
            }
            if n.service.is_some() {
                return Err(fail(
                    codes::SERVICE_LAYER,
                    format!(
                        "“{}” servisten çizilir ve nesne tutmaz; etkin katman nesneleri alan bir katman olur.",
                        n.name
                    ),
                    "layer",
                ));
            }
            if doc.layers().is_frozen(id) {
                return Err(fail(
                    codes::LAYER_FROZEN,
                    format!(
                        "“{}” dondurulmuş; etkin katman olamaz. Önce katmanı ya da grubunu çözün.",
                        n.name
                    ),
                    "layer",
                ));
            }
        }
        if op == Op::Frozen && input.value == Some(true) {
            let active = doc.layers().active();
            let holds = doc.layers().leaves_of(id).iter().any(|l| l.id == active);
            if holds {
                let what = match n.kind {
                    LayerNodeType::Group => format!("“{}” grubu etkin katmanı içeriyor", n.name),
                    LayerNodeType::Layer => format!("“{}” etkin katman", n.name),
                };
                return Err(fail(
                    codes::ACTIVE_FROZEN,
                    format!(
                        "{what}; etkin katman dondurulamaz. Önce başka bir katmanı etkin yapın."
                    ),
                    "layer",
                ));
            }
        }
    }
    if op == Op::Isolate {
        for (i, id) in input.layers.iter().flatten().enumerate() {
            known(id, &format!("layers[{i}]"))?;
        }
    }
    Ok(())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersSet) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do, on a reading copy; the drawing is left as it is.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersSet) -> CommandResult<LayersChanged> {
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

/// Makes the change: an edit when it changes the tree, never an undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersSet) -> CommandResult<LayersChanged> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_LAYERS_SET,
        kentos_contracts::CAD_LAYERS_SET_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(cx: &mut ExecutionContext<'_>, input: LayersSet) -> CommandResult<LayersChanged> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    CommandResult::Completed {
        output: write(cx.doc, &input),
        warnings: Vec::new(),
    }
}

fn write(doc: &mut Document, input: &LayersSet) -> LayersChanged {
    use LayerSetOperation as Op;
    let id = input.layer.as_deref().unwrap_or_default();
    let value = input.value.unwrap_or(false);
    let changed = match input.operation {
        Op::Rename => {
            let before = doc.revision();
            doc.rename_layer(id, input.name.as_deref().unwrap_or_default());
            doc.revision() != before
        }
        Op::Visible => {
            let before = doc.revision();
            doc.set_layer_visible(id, value);
            doc.revision() != before
        }
        Op::Locked => doc.set_layer_locked(id, value),
        Op::Plot => doc.set_layer_plot(id, value),
        Op::Frozen => doc.set_layer_frozen(id, value),
        Op::ThawAll => doc.thaw_all_layers(),
        Op::Unisolate => doc.unisolate_layers(),
        Op::Active => {
            let before = doc.layers().active() != id;
            doc.set_active_layer(id) && before
        }
        Op::Isolate => doc.isolate_layers(input.layers.as_deref().unwrap_or_default()),
        Op::ShowAll => {
            let before = doc.revision();
            doc.show_all_layers();
            doc.revision() != before
        }
    };
    LayersChanged {
        changed,
        active: doc.layers().active().to_owned(),
        revision: doc.revision().to_string(),
    }
}
